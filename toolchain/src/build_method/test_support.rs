//! Fixtures the build method's test modules share.
//! 构建方法各测试模块共享的夹具。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A fresh directory under the system temp dir, unique per process and call.
/// 系统临时目录下一个新目录，按进程与调用唯一。
///
/// Every build-method test that lays sources down needs one, and the first three copies drifted
/// into three files (one of them the renderer, which is not where a build-wide fixture belongs).
/// 每个把源码铺下来的构建方法测试都需要一个，而最初的三份副本散在三个文件里（其中一份在渲染器里——那并不是
/// 一个行星级的夹具该住的地方）。
pub(crate) fn temporary_directory(label: &str) -> PathBuf {
    crate::build_method::registry_identity::freeze_test_namespace();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "xirang-build-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary fixture root");
    path
}

/// Write one registry-owning face, creating its directory.
/// 写一个自持注册机的面，并建好它的目录。
pub(crate) fn write_registry(path: &Path, macro_name: &str, kind: &str, parent: &str) {
    fs::create_dir_all(path.parent().expect("fixture parent"))
        .expect("temporary fixture directory");
    fs::write(
        path,
        format!("crate::{macro_name}! {{ kind: {kind}, needs_registry: true, parent: {parent}, }}"),
    )
    .expect("temporary registry face");
}
