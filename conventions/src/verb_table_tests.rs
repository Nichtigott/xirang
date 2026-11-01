//! Pins for the NAM-33 verb-table gate: counter-proofs first, then the workspace ratchet.
//! NAM-33 动词表门禁的钉子：先反证，再是工作区棘轮。

use super::verb_table::{Violation, violations, violations_in};

fn names(found: &[Violation]) -> Vec<String> {
    found.iter().map(|item| item.name.clone()).collect()
}

/// A `get_` that does not return an `Option` is the table's `find_`.
/// 不返回 `Option` 的 `get_` 就是表里的 `find_`。
#[test]
fn a_get_that_is_not_an_option_is_reported() {
    let source = "pub fn get_face(&self) -> Face {\n    todo!()\n}\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["get_face".to_owned()]);
    assert!(found[0].why.contains("find_"));
}

/// The same name returning an `Option` is exactly what the table asks for.
/// 同名但返回 `Option` 的，正是表里要求的样子。
#[test]
fn a_get_that_is_an_option_is_exempt() {
    let source = "pub fn get_face(&self) -> Option<Face> {\n    todo!()\n}\n";
    assert!(violations_in("x/src/lib.rs", source).is_empty());
}

/// A bare verb is an entry position's privilege: `run` in `build.rs` is fine,
/// `run` in a library file is not.
/// 裸动词是入口位的特权：`build.rs` 里的 `run` 可以，库文件里的 `run` 不行。
#[test]
fn a_bare_verb_is_only_allowed_at_an_entry_position() {
    assert!(violations_in("toolchain/src/build_method/build.rs", "fn run() {}\n").is_empty());
    assert!(violations_in("toolchain/src/bin/xirang.rs", "fn main() {}\n").is_empty());
    let found = violations_in("toolchain/src/runtime/lib.rs", "pub fn run() {}\n");
    assert_eq!(names(&found), vec!["run".to_owned()]);
}

/// An unknown bare verb is reported wherever it sits.
/// 不在清单里的裸动词在任何位置都报。
#[test]
fn a_bare_table_verb_outside_an_entry_position_is_reported() {
    let found = violations_in("x/src/lib.rs", "pub fn handle() {}\n");
    assert_eq!(names(&found), vec!["handle".to_owned()]);
    // A single-word *noun* is not a bare verb: the table has nothing to say about it.
    // 单词**名词**不是裸动词：表对它没有话可说。
    assert!(violations_in("x/src/lib.rs", "pub fn dispatch() {}\n").is_empty());
}

/// `collect_` on a private function is the recursive helper the table calls `visit_`.
/// 私有函数上的 `collect_` 正是表里叫作 `visit_` 的递归 helper。
#[test]
fn a_private_collect_is_reported() {
    let source = "fn collect_leaves(node: &Node, out: &mut Vec<Id>) {\n    todo!()\n}\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["collect_leaves".to_owned()]);
    assert!(
        violations_in(
            "x/src/lib.rs",
            "pub fn collect_leaves() -> Vec<Id> { todo!() }\n"
        )
        .is_empty()
    );
}

/// A doc comment that merely mentions a violating name is not a declaration.
/// 仅仅**提到**违规名字的文档注释不是声明。
#[test]
fn a_mention_is_not_a_declaration() {
    let source = "/// Calls `get_face` under the hood.\n/// 底层会调用 `get_face`。\npub fn face_name(&self) -> Face {\n    todo!()\n}\n";
    assert!(violations_in("x/src/lib.rs", source).is_empty());
}

/// The workspace itself: the count is pinned so the gate has a ratchet, and the
/// list is printed so a new violation is visible in the failure output.
/// 工作区本身：数量被钉住，使门禁成为棘轮；违规清单也会打印，因此新增违规一看即知。
#[test]
fn the_shipped_workspace_verb_table_holds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    let found = violations(root);
    println!("verb-table violations: {}", found.len());
    for item in &found {
        println!("  {}:{} {}", item.file, item.line, item.name);
    }
    // The ratchet reached zero on 2026-09-29: the table is now a **must-be-empty** gate rather
    // than a ceiling, so a new bare verb (or a private `collect_`) fails here outright.
    // 棘轮在 2026-09-29 收到 0：这张表现在是**必须为空**的门禁，而不是上限——新增一个裸动词
    // （或一个私有 `collect_`）会直接在这里失败。
    assert!(
        found.is_empty(),
        "the verb table has {} violations (it must be empty); see the list above",
        found.len()
    );
}

/// A string literal that spells a declaration is **data**: a generator writing a `build.rs` emits
/// `fn main` inside a literal, and the gate has to read the workspace's one lexical rule rather than
/// the raw text (audit `M7`, P3.2).
/// 恰好拼出声明的字符串字面量是**数据**：一个生成 `build.rs` 的生成器会在字面量里发出 `fn main`，而门禁必须
/// 读本工作区唯一的词法规则而不是原文（审计 `M7`，P3.2）。
#[test]
fn a_declaration_inside_a_string_literal_is_not_a_declaration() {
    let source =
        "fn ghost_build_rs() -> String {\n    format!(\"fn main() {{\\n    work();\\n}}\\n\")\n}\n";
    let violations = violations_in("toolchain/src/build_method/generator.rs", source);
    assert!(
        violations.is_empty(),
        "a literal is not a declaration: {violations:?}"
    );
    // The control: the same declaration **as code** is still judged, so the masking did not simply
    // stop the gate from working.
    // 对照：同一句声明**作为代码**仍然被判定，因此掩码没有把门禁变成不干活。
    let real = "fn main() {\n    work();\n}\n";
    let violations = violations_in("toolchain/src/build_method/generator.rs", real);
    assert_eq!(violations.len(), 1, "{violations:?}");
}

/// A verb in a frozen evaluation archive is history; the same verb in live sources still fails.
/// 冻结评测归档里的动词是历史；活源码里的同一个动词仍然失败。
#[test]
fn archived_evaluation_sources_are_not_live_code() {
    let root = std::env::temp_dir().join(format!(
        "xirang-verb-archive-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let live = root.join("src/live.rs");
    let archived =
        root.join("docs/managed-maintenance/evaluation-2026-01-01/round-1/tree/src/eval.rs");
    for path in [&live, &archived] {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, "pub fn run() {}\n").expect("file");
    }
    let found = violations(&root);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].file, "src/live.rs");
    let _ = std::fs::remove_dir_all(&root);
}

/// A trait method's name belongs to the trait contract, not to this gate.
/// trait 方法的名字属于 trait 契约，而不是本门禁。
#[test]
fn a_trait_method_is_named_by_its_trait() {
    let source = "pub trait Reader {\n    fn read(&self, source: &str) -> String;\n}\n";
    assert!(violations_in("multilang/src/registration.rs", source).is_empty());
}

/// A deprecated declaration is a compatibility entry; the same name without the marker is a finding.
/// 被 `#[deprecated]` 标记的声明是兼容入口；同一名字去掉标记就是发现。
#[test]
fn a_deprecated_compatibility_entry_is_exempt() {
    let with = "/// Use `read_declaration`.\n#[deprecated(note = \"compatibility\")]\npub fn read(source: &str) -> String { source.to_owned() }\n";
    assert!(violations_in("kernel/src/registry_core/interchange.rs", with).is_empty());
    let without = "pub fn read(source: &str) -> String { source.to_owned() }\n";
    let found = violations_in("kernel/src/registry_core/interchange.rs", without);
    assert_eq!(names(&found), vec!["read".to_owned()]);
}

/// The trait exemption stops at the trait's closing brace: an ordinary function after it is live code.
/// trait 豁免在 trait 的闭合括号处结束：它之后的普通函数是活代码。
#[test]
fn a_function_after_a_closed_trait_is_still_checked() {
    let source = "pub trait Reader {\n    fn read(&self) -> String;\n}\n\npub fn read() -> String { String::new() }\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["read".to_owned()], "{found:?}");
}

/// Likewise, a trait implementation's exemption does not leak past its closing brace.
/// 同样，trait 实现的豁免不会漏过它的闭合括号。
#[test]
fn a_function_after_a_closed_trait_impl_is_still_checked() {
    let source = "struct X;\npub trait Reader { fn read(&self); }\nimpl Reader for X {\n    fn read(&self) {}\n}\n\npub fn read() -> String { String::new() }\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["read".to_owned()], "{found:?}");
}

/// An inherent impl is not a trait implementation: its methods are named by this crate and checked.
/// inherent impl 不是 trait 实现：它的方法由本 crate 命名，必须检查。
#[test]
fn an_inherent_impl_method_is_still_checked() {
    let source = "struct X;\nimpl X {\n    pub fn read(&self) -> String { String::new() }\n}\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["read".to_owned()], "{found:?}");
}

/// A foreign declaration inside an `extern` block is not ours to rename.
/// `extern` 块里的外部函数声明不归我们改名。
#[test]
fn a_foreign_declaration_is_not_checked() {
    let source =
        "unsafe extern \"C\" {\n    fn open(path: *const u8, flags: i32, ...) -> i32;\n}\n";
    assert!(violations_in("toolchain/src/plugin_host/process/child.rs", source).is_empty());
}

/// A generic trait impl (`impl<T> Trait for Type`) is still a trait implementation.
/// 泛型 trait 实现（`impl<T> Trait for Type`）仍然是 trait 实现。
#[test]
fn a_generic_trait_impl_method_is_exempt() {
    let source = "struct X<T>(T);\nimpl<T> Reader for X<T> {\n    fn read(&self, source: &str) -> String { source.to_owned() }\n}\n";
    assert!(violations_in("toolchain/src/plugin_host/process/child.rs", source).is_empty());
}
