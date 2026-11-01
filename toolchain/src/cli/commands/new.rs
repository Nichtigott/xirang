//! `xirang new`: scaffold a host project in `./<name>`.
//! `xirang new`：在 `./<name>` 中搭建宿主项目。
//!
//! Split out of `lib.rs` because scaffolding is a self-contained execution
//! surface: it reads the current directory and the running executable to decide
//! where the `xirang-kernel`/`xirang-toolchain` dependency comes from, and
//! touches no other command's state.
//! 从 `lib.rs` 拆出，因为脚手架是一块自包含的执行面：它读当前目录与正在运行的可执行
//! 文件来决定 `xirang-kernel`/`xirang-toolchain` 依赖来自哪里，不触碰其他命令的
//! 状态。

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::build_method::scaffold::{self, DependencySource, ProjectKind};

pub(crate) fn new(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut name: Option<String> = None;
    let mut lib = false;
    let mut path: Option<PathBuf> = None;
    let mut git: Option<String> = None;
    let mut language: Option<String> = None;
    let mut args = args.by_ref().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return crate::cli::usage(out),
            "--lib" => lib = true,
            "--path" => {
                let value = args.next().ok_or("--path requires a directory")?;
                // The explicit path is a claim about where a XiRang checkout is, and
                // nothing checked it: `--path /tmp` wrote `path = "/tmp/run_method"` and
                // exited 0. The claim is resolved where the source is chosen, below —
                // not here, because a request that names two sources has to be refused
                // for *that* reason rather than for whichever path it mentioned.
                // 显式路径是关于"XiRang 检出在哪"的主张，而过去没有任何检查：`--path /tmp` 写下
                // `path = "/tmp/run_method"` 并退出 0。这条主张在下面**选定来源处**解析，而不是在这里
                // ——因为一次点了两个来源的请求必须按"这处矛盾"被拒，而不是按它顺手提到的某个路径。
                path = Some(PathBuf::from(value));
            }
            "--git" => git = Some(args.next().ok_or("--git requires a URL")?),
            "--lang" => language = Some(args.next().ok_or("--lang requires a language")?),
            // An option-shaped token is not a package name. This subcommand has no
            // `--help` branch, so `xirang new --help` used to take `--help` as the name
            // and scaffold `./--help` — it was the only subcommand that did not refuse the
            // input, and the mistake wrote into whatever directory the shell was in.
            // 以选项形状出现的 token 不是包名。本子命令没有 `--help` 分支，因此
            // `xirang new --help` 过去把 `--help` 当名字并在 `./--help` 里搭起脚手架——它是唯一
            // 不拒绝这种输入的子命令，而这个错误会写进 shell 当时所在的目录。
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unexpected option '{arg}'; usage: xirang new <name> [--lib] \
                     [--lang <language>] [--path <workspace> | --git <url>]"
                ));
            }
            _ if name.is_none() => name = Some(arg),
            _ => return Err(format!("unexpected argument '{arg}'")),
        }
    }
    let name = name.ok_or("new requires a package name")?;
    if let Some(language) = &language
        && !language.eq_ignore_ascii_case("rust")
    {
        if path.is_some() || git.is_some() {
            return Err(format!(
                "--lang {language} and --path/--git name different things: a non-Rust project                  has no XiRang dependency to point at"
            ));
        }
        let root = std::env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?
            .join(&name);
        let summary = crate::multilang::new_project(&root, &name, language, lib)?;
        println!("{summary}");
        return Ok(());
    }
    let source = match (path, git) {
        // Two sources at once is refused rather than resolved: the arm used to be
        // `(Some(workspace), _)`, so naming both silently ignored the URL and
        // scaffolded against a local workspace the caller had also pointed away from
        // (the failure that surfaced was about the local path, never about the
        // conflict). `USAGE` spells the two as alternatives, and `build` refuses the
        // same shape of contradiction ("pass one of them"), so this is the promise the
        // command line already made (audit `LGC-LG-45`).
        // 一次给两个来源会被拒绝而不是被择一解决：旧臂是 `(Some(workspace), _)`，因此两者都给会静默
        // 忽略 URL，并对着调用方同时明确排除过的本地工作区搭脚手架（浮现出来的失败是关于本地路径的，
        // 从来不是关于这处矛盾）。`USAGE` 把两者写成互斥，而 `build` 对同形状的矛盾也拒绝
        //（"pass one of them"），因此这正是命令行本就做出的承诺（审计 `LGC-LG-45`）。
        (Some(_), Some(_)) => {
            return Err("--path and --git name two different sources; pass one of them".to_owned());
        }
        (Some(workspace), None) => DependencySource::Local {
            workspace: checkout_root(&workspace)?,
        },
        (None, Some(url)) => DependencySource::Git { url },
        (None, None) => scaffold::detected_source(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &std::env::current_exe()
                .map_err(|error| format!("cannot locate current executable: {error}"))?,
        ),
    };
    let root = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?
        .join(&name);
    let kind = if lib {
        ProjectKind::Library
    } else {
        ProjectKind::Binary
    };
    scaffold::create_project(&root, &name, kind, &source)?;
    println!(
        "created {} project at {}",
        if lib { "library" } else { "binary" },
        root.display()
    );
    Ok(())
}

/// The checkout `--path` names, resolved, or the reason it cannot be one.
/// `--path` 点名的检出（已解析），或它不成其为检出的原因。
fn checkout_root(value: &Path) -> Result<PathBuf, String> {
    let directory = std::fs::canonicalize(value)
        .map_err(|error| format!("--path {}: {error}", value.display()))?;
    if !is_checkout(&directory) {
        return Err(format!(
            "--path {} is not a XiRang checkout: it has no kernel/ and toolchain/",
            value.display()
        ));
    }
    Ok(directory)
}

/// Whether a directory is a XiRang checkout this scaffold can point at.
/// 某个目录是否是脚手架可以指向的 XiRang 检出。
///
/// The check lives here rather than in `build_method` because the scaffold writes a
/// *dependency* into someone else's manifest: the predicate has to be enforced by the
/// caller that is about to write it, and keeping it local means the packaged CLI does
/// not need a symbol newer than the published `xirang-toolchain`.
/// 这个判断放在这里而不是 `build_method`，因为脚手架是把一条**依赖**写进别人的清单：判断必须
/// 由即将写下它的调用方执行，而放在本地意味着打包后的 CLI 不需要一个比已发布
/// `xirang-toolchain` 更新的符号。
///
/// The two directories are the two halves a generated host depends on. They replaced
/// `core/`, `build_method/` and `run_method/` in batches 1 and 2, and this predicate
/// stayed on the old names — which is why `--path` refused this very checkout for as
/// long as nothing compiled a generated project. `tools/xirang-external-rehearsal`
/// now does, so the names here cannot drift again unnoticed.
/// 这两个目录是生成的宿主所依赖的两半。它们在批 1 与批 2 里取代了 `core/`、`build_method/`
/// 与 `run_method/`，而这个判断留在了旧名字上——只要没有任何东西去编译一个生成出来的项目，
/// `--path` 就会一直拒绝本检出自己。如今 `tools/xirang-external-rehearsal` 会去编译，
/// 因此这里的名字不会再无声漂移。
fn is_checkout(workspace: &Path) -> bool {
    workspace.join("kernel").is_dir() && workspace.join("toolchain").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory is a checkout only when both crates a scaffold points at are there;
    /// an empty or unrelated directory is refused before anything is written.
    /// 只有当脚手架要指向的两个 crate 都在时目录才算检出；空目录或无关目录会在写下任何东西之前
    /// 被拒绝。
    #[test]
    fn only_a_real_checkout_is_accepted() {
        let root = std::env::temp_dir().join(format!("xirang-new-checkout-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture directory");
        assert!(!is_checkout(&root), "an empty directory is not a checkout");
        // The stale half is refused too: `kernel/` alone is what batch 1 left the old
        // spellings looking like, and accepting it would point the generated manifest
        // at a directory the merge removed.
        // 陈旧的那一半同样被拒：只有 `kernel/` 正是批 1 之后旧拼法看到的样子，接受它就会让
        // 生成的清单指向一个在合并里被移除的目录。
        std::fs::create_dir_all(root.join("kernel")).expect("fixture directory");
        assert!(!is_checkout(&root), "one half is not a checkout: {root:?}");
        std::fs::create_dir_all(root.join("toolchain")).expect("fixture directory");
        assert!(is_checkout(&root), "both halves make it a checkout");
        let _ = std::fs::remove_dir_all(&root);
    }
}
