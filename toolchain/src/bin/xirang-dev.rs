//! Resident rebuild supervisor for XiRang Studio.
//! XiRang Studio 的常驻重建监督器。
//!
//! This binary is **workspace-only** (feature `dev-supervisor`): it rebuilds
//! Studio from a checkout and launches that checkout's `target/debug` binary, so
//! an installed copy has neither a source tree to rebuild nor a workspace build
//! to launch. It is therefore not installed by `cargo install xirang-toolchain`.
//! 本二进制**仅限工作区**（特性 `dev-supervisor`）：它从检出重建 Studio 并启动该检出的
//! `target/debug` 产物，而安装副本既没有可重建的源码树，也没有可启动的工作区构建，因此
//! `cargo install xirang-toolchain` 不会安装它。

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use notify::Watcher as _;

fn main() -> ExitCode {
    match supervise(std::env::args().nth(1).as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("XiRang Studio: {error}");
            ExitCode::FAILURE
        }
    }
}

fn supervise(query: Option<&str>) -> Result<(), String> {
    let studio_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // The baked manifest path is the reason this tool cannot be installed: it
    // names the machine that compiled it. Failing here, with the fix in the
    // message, beats letting Cargo report "manifest not found" as if the user's
    // project were broken.
    // 编译期烧进来的清单路径正是本工具无法安装的原因：它指的是编译它的那台机器。在这里
    // 失败、并把修法写进消息，好过让 Cargo 报"找不到清单"、看起来像用户的项目坏了。
    if !studio_root.join("Cargo.toml").is_file() {
        return Err(format!(
            "this `xirang-dev` was built from `{}`, which no longer exists; it rebuilds Studio \
             from that checkout, so run it from one instead of an installed copy: \
             `cargo run -p xirang-toolchain --features dev-supervisor --bin xirang-dev -- watch`",
            studio_root.display()
        ));
    }
    let workspace_root = studio_root
        .parent()
        .ok_or_else(|| "Studio manifest has no package parent".to_owned())?;
    let package_root = std::env::var_os("XIRANG_PACKAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.to_owned());
    let current_exe = std::env::current_exe().ok();
    let path_env = std::env::var_os("PATH");
    let executable = resolve_studio(current_exe.as_deref(), path_env.as_deref(), workspace_root);
    let mut watcher = SourceWatcher::new(&package_root).map_err(|error| error.to_string())?;

    rebuild(&studio_root)?;
    watcher.drain();
    let mut child = spawn_child(&executable, query)?;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return exited(status),
            Ok(None) => {}
            Err(error) => return Err(format!("cannot inspect Studio process: {error}")),
        }
        match watcher.changed().map_err(|error| error.to_string())? {
            false => thread::sleep(Duration::from_millis(80)),
            true => {
                while watcher.changed().unwrap_or(false) {}
                match rebuild(&studio_root) {
                    Ok(()) => {
                        stop_child(&mut child).map_err(|error| error.to_string())?;
                        child = spawn_child(&executable, query)?;
                    }
                    Err(error) => eprintln!(
                        "XiRang Studio: build failed; keeping the last good process\n{error}"
                    ),
                }
            }
        }
    }
}

/// The child Studio binary, in the order a checkout and an installed sibling both
/// work.
/// 子 Studio 二进制，按"检出与已安装同级副本都能用"的顺序解析。
///
/// The sibling next to the running executable is checked first because that is
/// where Cargo puts both binaries — in the workspace's `target/debug` and in an
/// installed `~/.cargo/bin` — so a normal build is found without trusting `PATH`.
/// `PATH` is the second answer for a supervisor started from somewhere else, and
/// the workspace path stays last because older builds of this tool only ever
/// looked there (and it is still the right answer when the sibling binary has not
/// been built yet but `cargo build` has been run for another target).
/// 先检查与当前可执行文件同级的目录，因为 Cargo 正是把两个二进制放在一起的——工作区的
/// `target/debug` 与安装后的 `~/.cargo/bin` 都是如此——因此正常构建无需相信 `PATH` 就能
/// 找到。`PATH` 是"监督器从别处启动"时的第二个答案；工作区路径留在最后，因为本工具早期
/// 的构建只看那里（而且当同级二进制尚未构建、但已为其他 target 跑过 `cargo build` 时，
/// 它仍然是正确答案）。
fn resolve_studio(exe: Option<&Path>, path_env: Option<&OsStr>, workspace_root: &Path) -> PathBuf {
    let name = format!("xirang-toolchain{}", std::env::consts::EXE_SUFFIX);
    if let Some(candidate) = exe.and_then(Path::parent).map(|dir| dir.join(&name))
        && candidate.is_file()
    {
        return candidate;
    }
    if let Some(path_env) = path_env {
        for dir in std::env::split_paths(path_env) {
            let candidate = dir.join(&name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    workspace_root.join("target/debug").join(name)
}

/// The supervisor's outcome for a Studio process that has exited.
/// Studio 进程退出后监督器的结论。
///
/// A child that exited non-zero is not a successful session: an editor that died
/// on startup and one the reader quit look identical from here, and only the exit
/// status tells them apart. Reporting success for both is what let a crashing
/// Studio look like a normal end.
/// 以非零退出的子进程不是一次成功的会话：在这里看，启动即崩的编辑器与读者主动退出的编辑器
/// 一模一样，只有退出状态能区分它们。两者都报成功，正是崩溃的 Studio 看起来像正常结束的原因。
fn exited(status: ExitStatus) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("Studio exited with {status}"))
    }
}

fn rebuild(studio_root: &Path) -> Result<(), String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(studio_root.join("Cargo.toml"))
        .args(["--bin", "xirang-toolchain"])
        .output()
        .map_err(|error| format!("cannot invoke Cargo: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn spawn_child(executable: &Path, query: Option<&str>) -> Result<Child, String> {
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(query) = query.filter(|query| !query.is_empty() && *query != "watch") {
        command.arg(query);
    }
    command.spawn().map_err(|error| {
        format!(
            "cannot start `{}`: {error}; build it first with `cargo build -p xirang-toolchain --bin xirang-toolchain`",
            executable.display()
        )
    })
}

fn stop_child(child: &mut Child) -> io::Result<()> {
    request_stop(child)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill()?;
    let _ = child.wait();
    Ok(())
}

#[cfg(unix)]
fn request_stop(child: &Child) -> io::Result<()> {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    if unsafe { kill(child.id() as i32, 15) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn request_stop(child: &Child) -> io::Result<()> {
    let _ = child.id();
    Ok(())
}

struct SourceWatcher {
    _watcher: notify::RecommendedWatcher,
    events: std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
}

impl SourceWatcher {
    fn new(package_root: &Path) -> io::Result<Self> {
        let (sender, events) = std::sync::mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(sender, notify::Config::default())
            .map_err(io::Error::other)?;
        watcher
            .watch(package_root, notify::RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
        for root in [
            package_root.join("src"),
            package_root.join("toolchain/src/studio"),
            // Plugin selection is source too. Watching only the extension
            // crate leaves catalog changes invisible until restart.
            // 插件选择文件同样属于源码。只监听 extension crate 会让目录修改
            // 在重启前不可见。
            package_root.join(".xirang/plugins"),
        ] {
            if root.is_dir() {
                watcher
                    .watch(&root, notify::RecursiveMode::Recursive)
                    .map_err(io::Error::other)?;
            }
        }
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }

    fn changed(&mut self) -> io::Result<bool> {
        use std::sync::mpsc::RecvTimeoutError;

        let first = match self.events.recv_timeout(Duration::from_millis(120)) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => return Ok(false),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "watch channel closed",
                ));
            }
        };
        let mut relevant = first.is_ok_and(|event| relevant_event(&event));
        while let Ok(event) = self.events.try_recv() {
            relevant |= event.is_ok_and(|event| relevant_event(&event));
        }
        Ok(relevant)
    }

    fn drain(&mut self) {
        while self.events.try_recv().is_ok() {}
    }
}

fn relevant_event(event: &notify::Event) -> bool {
    use notify::event::{EventKind, ModifyKind};

    matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Name(_))
    ) && event.paths.iter().any(|path| {
        !path
            .components()
            .any(|part| matches!(part.as_os_str().to_str(), Some("target" | ".git")))
            && (path.extension().and_then(|extension| extension.to_str()) == Some("rs")
                // The same five names the studio library's stamp accepts
                // (`studio::app::source_stamp::RELEVANT_FILE_NAMES`). This is a mirror,
                // not an import: a `pub(crate)` predicate is invisible to this binary.
                // 与 studio 库的戳接受的同一份五个名字
                // （`studio::app::source_stamp::RELEVANT_FILE_NAMES`）。这是镜像而不是 import：
                // 本二进制看不到 `pub(crate)` 谓词。
                || matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("Cargo.toml" | "Cargo.lock" | "build.rs" | "official.lock" | "user.lock")
                ))
    })
}

#[cfg(test)]
mod tests {
    use notify::{Event, EventKind, event::CreateKind};

    use super::{exited, relevant_event, resolve_studio};

    /// A throwaway directory for the resolution fixtures, unique per call.
    /// 解析夹具使用的一次性目录，每次调用唯一。
    fn fixture_dir(label: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "xirang-dev-{label}-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("fixture dir");
        dir
    }

    fn studio_name() -> String {
        format!("xirang-toolchain{}", std::env::consts::EXE_SUFFIX)
    }

    fn dev_name() -> String {
        format!("xirang-dev{}", std::env::consts::EXE_SUFFIX)
    }

    /// The binary Cargo built next to the supervisor is the one it should start:
    /// that is true in `target/debug` and in an installed `bin` directory, and it
    /// needs no `PATH` at all.
    /// Cargo 放在监督器旁边的那份二进制才是它该启动的：`target/debug` 与安装后的 `bin`
    /// 目录都是如此，而且完全不需要 `PATH`。
    #[test]
    fn the_sibling_binary_wins_over_path_and_the_workspace() {
        let sibling = fixture_dir("sibling");
        std::fs::write(sibling.join(studio_name()), "").expect("sibling binary");
        let on_path = fixture_dir("path");
        std::fs::write(on_path.join(studio_name()), "").expect("path binary");
        let workspace = fixture_dir("workspace");
        let path_env = std::env::join_paths([on_path.as_path()]).expect("join PATH");

        assert_eq!(
            resolve_studio(Some(&sibling.join(dev_name())), Some(&path_env), &workspace),
            sibling.join(studio_name())
        );

        let _ = std::fs::remove_dir_all(&sibling);
        let _ = std::fs::remove_dir_all(&on_path);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// With no sibling built, `PATH` decides before the workspace path is guessed.
    /// 同级目录没有构建产物时，先由 `PATH` 决定，再去猜工作区路径。
    #[test]
    fn path_is_searched_when_there_is_no_sibling() {
        let empty = fixture_dir("empty");
        let on_path = fixture_dir("path-only");
        std::fs::write(on_path.join(studio_name()), "").expect("path binary");
        let workspace = fixture_dir("workspace-fallback");
        let path_env = std::env::join_paths([on_path.as_path()]).expect("join PATH");

        assert_eq!(
            resolve_studio(Some(&empty.join(dev_name())), Some(&path_env), &workspace),
            on_path.join(studio_name())
        );

        let _ = std::fs::remove_dir_all(&empty);
        let _ = std::fs::remove_dir_all(&on_path);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// The workspace path stays the last answer, so a supervisor started from a
    /// build directory that has no sibling yet keeps working.
    /// 工作区路径仍是最后一个答案，因此从"尚无同级产物"的构建目录启动的监督器依然可用。
    #[test]
    fn the_workspace_target_is_the_last_resort() {
        let exe_dir = fixture_dir("no-sibling");
        let empty_path = fixture_dir("empty-path");
        let workspace = fixture_dir("workspace-last");
        let path_env = std::env::join_paths([empty_path.as_path()]).expect("join PATH");

        assert_eq!(
            resolve_studio(Some(&exe_dir.join(dev_name())), Some(&path_env), &workspace),
            workspace.join("target/debug").join(studio_name())
        );

        let _ = std::fs::remove_dir_all(&exe_dir);
        let _ = std::fs::remove_dir_all(&empty_path);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn filters_generated_files_and_accepts_sources() {
        let source =
            Event::new(EventKind::Create(CreateKind::File)).add_path("src/control.rs".into());
        let target = Event::new(EventKind::Create(CreateKind::File))
            .add_path("target/debug/generated.rs".into());
        assert!(relevant_event(&source));
        assert!(!relevant_event(&target));
    }

    #[test]
    fn accepts_plugin_catalog_changes() {
        let catalog = Event::new(EventKind::Modify(notify::event::ModifyKind::Any))
            .add_path(".xirang/plugins/official.rs".into());
        assert!(relevant_event(&catalog));
    }

    #[test]
    fn accepts_remove_rename_and_cargo_changes() {
        let removed = Event::new(EventKind::Remove(notify::event::RemoveKind::File))
            .add_path("toolchain/src/studio/old.rs".into());
        let renamed = Event::new(EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )))
        .add_path(".xirang/plugins/user.lock".into());
        let cargo = Event::new(EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Content,
        )))
        .add_path("Cargo.toml".into());
        assert!(relevant_event(&removed));
        assert!(relevant_event(&renamed));
        assert!(relevant_event(&cargo));
    }

    /// Run a child that exits with `code` and fold its status the way the loop
    /// does.
    /// 跑一个以 `code` 退出的子进程，并按循环的方式折叠它的状态。
    fn outcome(code: i32) -> Result<(), String> {
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("exit {code}"))
            .status()
            .expect("run a shell that exits");
        exited(status)
    }

    /// A clean quit is a successful session; a crash is not. Reporting success
    /// for both is what let a Studio that died on startup look like a normal end.
    /// 正常退出是一次成功会话；崩溃不是。两者都报成功，正是启动即崩的 Studio 看起来像正常
    /// 结束的原因。
    #[test]
    fn a_studio_that_exits_non_zero_is_reported() {
        assert!(outcome(0).is_ok(), "a clean exit stays successful");
        let error = outcome(7).expect_err("a non-zero exit must be reported");
        assert!(error.contains("Studio exited"), "{error}");
    }
}
