//! File half of the trace artifact: the default path, reader, and writer.
//! trace artifact 的文件一半：默认路径、读取方与写入方。
//!
//! This is the execution surface's half: it touches the environment for the
//! namespace anchor and the filesystem for the document. The document itself is
//! pure and lives in the parent module.
//! 这是执行面的一半：为命名空间锚点读取环境，为文档读写文件系统。文档本身是纯的，住在父模块。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::run_method::registry_core::identity::root_node_id;
use crate::run_method::registry_core::lexicon::{
    TRACE_DIR, TRACE_FILE, TRACE_FILE_ENV, XIRANG_DIR,
};

use super::{CallTrace, TraceArtifact};

/// The path a host writes a trace artifact to, and a reader looks in.
/// 宿主写入 trace artifact、读取方查找它的路径。
///
/// `XIRANG_TRACE_FILE` wins when it names a non-empty path — absolute, or
/// relative to `package_root` — and otherwise the shared
/// `XIRANG_DIR`/`TRACE_DIR`/`TRACE_FILE` contracts give
/// `package_root/.xirang/traces/xirang.trace`. Both the writer and the reader
/// ask this one function, so an override cannot move the file for one of them and
/// not the other.
/// `XIRANG_TRACE_FILE` 给出非空路径时以它为准——绝对路径，或相对 `package_root` 的路径——
/// 否则由共享的 `XIRANG_DIR`/`TRACE_DIR`/`TRACE_FILE` 契约给出
/// `package_root/.xirang/traces/xirang.trace`。写入方与读取方问的是同一个函数，因此覆盖不会
/// 只挪动其中一方的文件。
pub fn trace_artifact_path(package_root: &Path) -> PathBuf {
    resolve_artifact_path(package_root, std::env::var_os(TRACE_FILE_ENV).as_deref())
}

/// The path decision itself, separated so it can be tested without the process
/// environment.
/// 路径判断本身；单独拆出，以便不依赖进程环境地测试它。
pub(super) fn resolve_artifact_path(package_root: &Path, configured: Option<&OsStr>) -> PathBuf {
    if let Some(configured) = configured {
        let configured = Path::new(configured);
        if !configured.as_os_str().is_empty() {
            return if configured.is_absolute() {
                configured.to_path_buf()
            } else {
                package_root.join(configured)
            };
        }
    }
    package_root
        .join(XIRANG_DIR)
        .join(TRACE_DIR)
        .join(TRACE_FILE)
}

/// Write a recorded trace as an artifact, replacing `path` atomically.
/// 把已记录的追踪写成 artifact，并以原子方式替换 `path`。
///
/// `namespace` is the identity the host compiled under — the value its
/// declaration macros read, which for a host crate is `crate::XIRANG_NAMESPACE` — the constant
/// `host!()` defines from `env!("CARGO_PKG_NAME")` (audit `M7`, P3.3). It is a parameter because the process cannot read it
/// back: Cargo sets `CARGO_PKG_NAME` for the build script and for `env!`, not for
/// an installed binary, and `XIRANG_NAMESPACE` is the *reader's* override — a
/// writer that stamped from it would publish a namespace its own compiled node ids
/// do not live in, and every reader would refuse the artifact. The root anchor is
/// derived from the same name, because a registry root is `root_node_id(namespace)`.
/// `namespace` 是宿主编译时所用的身份——它的声明宏读取的那个值，对宿主 crate 就是
/// `crate::XIRANG_NAMESPACE`，即 `host!()` 用 `env!("CARGO_PKG_NAME")` 定义的那个常量
/// （审计 `M7`，P3.3）。它作为参数传入，因为进程读不回来：Cargo 只为构建脚本与
/// `env!` 设置 `CARGO_PKG_NAME`，不为已安装的二进制设置；而 `XIRANG_NAMESPACE` 是
/// **读取方**的覆盖——写入方若按它盖戳，就会发布一个自己编译出的节点 id 并不居住的命名空间，
/// 每个读取方都会拒绝该 artifact。root 锚点由同一个名字推出，因为注册树根就是
/// `root_node_id(namespace)`。
pub fn write_trace_artifact(trace: &CallTrace, path: &Path, namespace: &str) -> Result<(), String> {
    // The directory is this writer's own output directory, so it creates it: the
    // documented host usage pairs `write_trace_artifact` with
    // `trace_artifact_path`, and on a fresh project that path's
    // `.xirang/traces/` does not exist yet — the write failed with
    // "No such file or directory", and the message named the writer's temporary
    // file rather than the missing directory. Creating it is what the authoring
    // executor's artifact writer does for `.xirang/external-grafts/`.
    // 目录是本写入方自己的输出目录，因此由它创建：文档化的宿主用法把 `write_trace_artifact` 与
    // `trace_artifact_path` 配成一对，而在一个全新的项目里，那条路径的 `.xirang/traces/` 还
    // 不存在——写入以 "No such file or directory" 失败，而且消息点名的是写入方自己的临时文件，
    // 而不是缺失的目录。创作执行器自己的产物写入方对 `.xirang/external-grafts/` 也是这么做的。
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut artifact = TraceArtifact::from_trace(trace);
    artifact.namespace = namespace.to_owned();
    artifact.root = root_node_id(namespace);
    atomic_write(path, &artifact.render_trace_artifact())
}

/// Read an artifact and rebuild the trace it records.
/// 读取 artifact 并重建它记录的追踪。
pub fn read_trace_artifact(path: &Path) -> Result<(TraceArtifact, CallTrace), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let artifact =
        TraceArtifact::parse_trace_artifact(&source).map_err(|error| error.to_string())?;
    let trace = artifact
        .clone()
        .into_trace()
        .map_err(|error| error.to_string())?;
    Ok((artifact, trace))
}

/// Write `contents` to `path` through a unique sibling and one rename.
/// 经由唯一的同级文件与一次重命名把 `contents` 写到 `path`。
///
/// The same pattern as the authoring executor's writer, and for the same reason:
/// a fixed temporary name must be deleted before it can be reused, and that
/// deletion could destroy a sibling this writer never created.
/// 与 authoring 执行器的写入方同一模式，理由也相同：固定临时名必须先删除才能复用，而那次删除
/// 可能毁掉一个本写入方从未创建的同级文件。
fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    use std::io::Write as _;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let temporary = path.with_file_name(format!(
        ".{name}.xirang-{}-{sequence}.tmp",
        std::process::id()
    ));
    let outcome = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("cannot create {}: {error}", temporary.display()))?;
        file.write_all(contents.as_bytes())
            .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
        std::fs::rename(&temporary, path)
            .map_err(|error| format!("cannot replace {}: {error}", path.display()))
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    outcome
}
