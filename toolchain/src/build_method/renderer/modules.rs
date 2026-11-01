//! One real module tree shared by rustc and rust-analyzer.
//! rustc 与 rust-analyzer 共用的一棵真实模块树。

use std::fmt::Write as _;
use std::path::Path;

use crate::build_method::{Node, discovery_cache::write_if_changed};

pub(super) fn emit_face_declaration(output: &mut String, node: &Node, source: &str, depth: usize) {
    let indent = "    ".repeat(depth);
    writeln!(output, "{indent}#[path = {source:?}]").unwrap();
    if node.children.is_empty() {
        writeln!(output, "{indent}pub mod {};", node.name).unwrap();
    } else {
        writeln!(output, "{indent}mod {};", node.name).unwrap();
        writeln!(output, "{indent}pub use {}::*;", node.name).unwrap();
    }
}

/// Materialize generated containers without copying or including business source files.
/// 将生成容器落盘，不复制或 include 业务源码文件。
///
/// The containers arrive **already split**: the render walk emitted each one into its own buffer as it
/// descended (`stem 2`, step 3). This used to receive one big source and call `syn::parse_file` on it to
/// find the module boundaries again — a round trip that measured **3.710 s** on a 1,000-face host (and
/// whose per-module offset conversion was itself O(N × L) until the line index landed).
/// 容器是**已经切好的**：渲染遍历在下行时就把每一个发射进它自己的缓冲区（`stem 2` 第③步）。这里过去收到
/// 一大份源码，再对它调用 `syn::parse_file` 重新找出模块边界——那趟往返在 1,000 面的宿主上实测 **3.710 s**
/// （而它逐模块的偏移换算在行索引落地之前本身还是 O(N × L)）。
pub(crate) fn write_modules(
    out_dir: &Path,
    source: &str,
    modules: &[(String, String)],
) -> Result<(), String> {
    let directory = out_dir.join("xirang_modules");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    for (key, body) in modules {
        write_if_changed(&directory.join(format!("{key}.rs")), body)?;
    }
    write_if_changed(
        &out_dir.join(xirang_kernel::lexicon::GENERATED_LIB_FILE),
        source,
    )
}
