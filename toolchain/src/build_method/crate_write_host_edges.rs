//! Whether a host needs a dependency edge into a generated crate.
//! 宿主是否需要一条指向生成 crate 的依赖边。

use std::fs;
use std::path::{Path, PathBuf};

/// Whether a host face outside the claimed subtrees still spells `crate::<subtree>::…`.
/// 宿主在认领子树之外是否仍写 `crate::<子树>::…`。
///
/// Shared by the development and release writers so both agree on whether a host dependency edge is
/// needed. The host **entry** is skipped: its `static_graft_plan!` text keeps cuts for faces it handed
/// away, and that text is data for the generator, not a type the host compiles.
/// 开发与发布写入共用它，二者对"是否需要宿主依赖边"给同一个答案。宿主**入口**被跳过：它的
/// `static_graft_plan!` 文本保留着交出去的面的切口，而那是给生成器的数据，不是宿主编译的类型。
pub(crate) fn host_needs_fragment(host_root: &Path, subtrees: &[String]) -> bool {
    if subtrees.is_empty() {
        return false;
    }
    let mut files = Vec::new();
    visit_rust_files(&host_root.join("src"), &mut files);
    visit_rust_files(&host_root.join("tests"), &mut files);
    for subtree in subtrees {
        let needle = format!("crate::{subtree}");
        let claimed = format!("/{}/", subtree.replace("::", "/"));
        for file in &files {
            let relative = file
                .strip_prefix(host_root)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");
            if relative.contains(&claimed) {
                continue;
            }
            if file
                .file_name()
                .is_some_and(|name| name == "lib.rs" || name == "main.rs")
            {
                continue;
            }
            if std::fs::read_to_string(file).is_ok_and(|text| text.contains(&needle)) {
                return true;
            }
        }
    }
    false
}

/// Every `.rs` below `directory`, skipping build outputs and generated packages.
/// `directory` 下每个 `.rs`，跳过构建产物与生成包。
fn visit_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == "crates" || name == ".xirang" {
            continue;
        }
        if path.is_dir() {
            visit_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// A path relative to the workspace root, with `/` separators.
/// 相对工作区根的路径，使用 `/` 分隔符。
pub(super) fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
