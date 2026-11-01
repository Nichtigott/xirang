//! The conventional host entry, chosen from the layout.
//! 按布局选择的约定宿主入口。
//!
//! Split out of `entry.rs` when that page reached the size ceiling, and the
//! boundary is the concern rather than the count: `entry.rs` resolves *declared*
//! and *configured* entries and reports what it had to refuse, while this page
//! answers the one question that needs no configuration — which file Cargo's
//! convention names — and it is the only part of entry resolution that needs the
//! package's source layout rather than the identity base.
//! 在那一页触到尺寸上限时从 `entry.rs` 拆出，界线是关注点而不是行数：`entry.rs` 解析**声明的**
//! 与**配置的**入口并报告它不得不拒绝的东西，而本页回答的是唯一不需要配置的问题——Cargo 约定命名
//! 了哪个文件——它也是入口解析里唯一需要包的源码布局、而不是身份基准的部分。

use std::fs;
use std::path::{Path, PathBuf};

use super::SourceLayout;

/// Pick the ordinary Cargo entry when no `application!` declaration exists.
/// 没有 `application!` 声明时，按 Cargo 约定选择默认入口。
///
/// The candidates come from the layout, not from a hardcoded `src/`: a package
/// whose library target is `host/lib.rs` has its crate root — the file that calls
/// `host!()` — at that path, and looking for `main.rs`/`lib.rs` under `src/` would
/// find neither. The declared target leads the list, and the conventional names
/// under the scan root follow it, so a package with no declared target behaves
/// exactly as before.
/// 候选来自布局而不是写死的 `src/`：库目标是 `host/lib.rs` 的包，其 crate 根——调用 `host!()` 的
/// 那个文件——就在那条路径上，而在 `src/` 下找 `main.rs`/`lib.rs` 两个都找不到。声明的目标排在
/// 最前，紧随其后的是遍历根下的约定名，因此没有声明目标的包行为与从前完全一致。
pub(crate) fn default_entry_source(layout: &SourceLayout) -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(target) = &layout.target {
        candidates.push(target.clone());
    }
    candidates.push(layout.scan_root.join("main.rs"));
    candidates.push(layout.scan_root.join("lib.rs"));
    // Cargo prefers `main.rs`, but a lib+bin host calls `host!()` in whichever
    // file owns the generated tree — often `lib.rs`, with `main.rs` left as a
    // stub. An `application!` declaration is not the only thing that can live
    // there: the declared graft plan does too, and picking the stub silently
    // dropped it. The entry is the file that calls `host!()`; Cargo's order only
    // breaks the tie.
    // Cargo 偏好 `main.rs`，但库+二进制宿主会在拥有生成树的那个文件里调用
    // `host!()`——常见情形是 `lib.rs`，而 `main.rs` 只是个空壳。那里不只可能放
    // `application!` 声明，声明的 graft 计划也在其中，选中空壳会把它静默丢掉。
    // 入口应当是调用 `host!()` 的文件；Cargo 的顺序只用来打破平局。
    if let Some(entry) = candidates.iter().find(|path| calls_host(path)) {
        return entry.clone();
    }
    let main = candidates[0].clone();
    if main.is_file() {
        return main;
    }
    let lib = candidates[1].clone();
    if lib.is_file() {
        return lib;
    }
    // Keep the existing missing-entry diagnostic and conservative fallback.
    // 保留原有 missing-entry 诊断，并继续使用保守的全树回退。
    main
}

/// Whether a source file declares this crate as a host.
/// 源文件是否把本 crate 声明为宿主。
///
/// The decision is lexical, not line-based: the kernel's source scanner masks
/// comments and string literals, so the `host!()` a doc comment documents never
/// counts, while the real invocation at the crate root does. `host!()` is a
/// macro invocation and the kernel's `body_calls` scanner deliberately skips
/// macro names, so the bang is folded to a space before the scan: that keeps the
/// scanner's masking — the part this needs — and turns the invocation into the
/// call shape it matches.
/// 判断是词法层面的，而不是逐行匹配：内核源码扫描器会屏蔽注释与字符串字面量，
/// 因此文档注释里提到的 `host!()` 不算数，而 crate 根上的真实调用算数。
/// `host!()` 是宏调用，内核的 `body_calls` 扫描器刻意跳过宏名，因此扫描前把感叹号
/// 折成空格：这样既保留了这里需要的屏蔽语义，又把该调用变成扫描器能匹配的调用形状。
pub(crate) fn calls_host(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|source| host_macro_call(&source))
}

/// Whether `source` contains a real `host!()` invocation outside comments and
/// string literals.
/// `source` 是否在注释与字符串字面量之外包含真实的 `host!()` 调用。
fn host_macro_call(source: &str) -> bool {
    // Only the bang right after `host` is rewritten; comments and string
    // literals are still masked by the kernel scanner itself.
    // 只改写紧跟在 `host` 之后的感叹号；注释与字符串字面量仍由内核扫描器自己屏蔽。
    let probe = source.replace("host!", "host ");
    xirang_kernel::source::body_calls(&probe, "host")
}
