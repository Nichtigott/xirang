//! Where a partition writes: the host's own workspace root, or its enclosing workspace.
//! 分区写到哪里：宿主自己的工作区根，或它所在的外层工作区。

use std::path::{Path, PathBuf};

pub(crate) fn partition_roots(package_root: &Path) -> (PathBuf, Option<PathBuf>) {
    let workspace = workspace_root_of(package_root);
    if workspace == package_root {
        // A standalone package is its own workspace root. Writing the generated packages **beside**
        // it (in the parent directory) made cargo refuse them: a member must be hierarchically below
        // its workspace root, and the parent's implicit root cannot take a second root either. The
        // split therefore lands **under** the host (`<host>/crates/…`) and the host manifest gains
        // the `[workspace]` section that admits them — a package may be a workspace root.
        // 独立包自己就是工作区根。把生成包写在它**旁边**（父目录）会被 cargo 拒绝：成员必须位于工作区根
        // 之下，而父目录的隐式根也不能再接纳第二个根。因此拆分落在宿主**之内**（`<host>/crates/…`），
        // 宿主清单获得接纳它们的 `[workspace]` 段——包可以同时是工作区根。
        (package_root.to_path_buf(), Some(package_root.to_path_buf()))
    } else {
        (workspace.clone(), Some(workspace))
    }
}

/// The outer-most ancestor that is a workspace, or the package itself when there is none.
/// 最外层的、是工作区的祖先目录；没有时就是包自己。
///
/// `rustflags` belong to the workspace root because they are per **invocation** and cargo finds config
/// from the current directory: a config inside the package would only apply when cargo is run from
/// there, which is not where a workspace builds from.
/// `rustflags` 属于工作区根，因为它们是每次调用生效的，而 cargo 从当前目录找 config：放在包里的 config 只在
/// 从那个目录跑 cargo 时才生效，而工作区并不是从那里构建的。
fn workspace_root_of(package_root: &Path) -> PathBuf {
    let mut root = package_root.to_path_buf();
    let mut current = package_root.parent();
    while let Some(parent) = current {
        let manifest = parent.join("Cargo.toml");
        let is_workspace = std::fs::read_to_string(&manifest)
            .map(|text| text.lines().any(|line| line.trim() == "[workspace]"))
            .unwrap_or(false);
        if is_workspace {
            root = parent.to_path_buf();
            current = parent.parent();
        } else {
            break;
        }
    }
    root
}

/// Resolve `.` and `..` segments textually, without touching the filesystem.
/// 纯文本地消去 `.` 与 `..` 段，不碰文件系统。
///
/// Without this the walk below answers `../../host/../../../toolchain`: it resolves to the right
/// package, but a dependency path a reader cannot check by eye is a path that will be misread. The
/// segments cancel because the host's own spelling is relative to the host.
/// 没有它，下面的走法会答出 `../../host/../../../toolchain`：它确实解析到正确的包，但一条读者用眼睛核不了的
/// 依赖路径就是一条会被误读的路径。那些段能相互抵消，是因为宿主自己的拼写是相对宿主写的。
pub(crate) fn compact_path(path: &Path) -> PathBuf {
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => match parts.last() {
                Some(last) if last != ".." => {
                    parts.pop();
                }
                _ => parts.push("..".into()),
            },
            other => parts.push(other.as_os_str().to_owned()),
        }
    }
    parts.iter().collect()
}

/// One `key = "value"` from a manifest's top-level lines, without quotes.
/// 清单顶层某行 `key = "value"` 的值，不含引号。
pub(crate) fn toml_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&format!("{key} = ")))
        .map(|value| value.trim().trim_matches('"').to_owned())
}

/// One manifest section's body, header excluded and trailing blank lines trimmed.
/// 清单里某一节的正文，不含表头、去掉尾部空行。
pub(crate) fn toml_section(text: &str, section: &str) -> Option<String> {
    let header = format!("[{section}]");
    let start = text.lines().position(|line| line.trim() == header)? + 1;
    let body: Vec<&str> = text
        .lines()
        .skip(start)
        .take_while(|line| !line.trim_start().starts_with('['))
        .collect();
    let body = body.join("\n");
    let trimmed = body.trim_end();
    (!trimmed.is_empty()).then(|| format!("{trimmed}\n"))
}
