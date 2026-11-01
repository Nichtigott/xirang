//! Pins for the feature faces: the list names the non-default features and the targets that
//! require them, and it says what it cannot answer.
//! 特性面的钉子：清单点名非默认特性与需要它们的 target，并说出它答不了什么。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::faces_lines;
use super::registered_faces_line;
use serde_json::json;

/// A throwaway package with one extra feature and a target that requires it.
/// 一个一次性包：多一个特性，以及一个需要它的 target。
fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-faces-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [features]\nextra = []\n\n[[bin]]\nname = \"tool\"\npath = \"src/tool.rs\"\n\
         required-features = [\"extra\"]\n"
        ),
    );
    write_fixture(root.join("src/lib.rs"), "// host entry\n");
    write_fixture(root.join("src/tool.rs"), "fn main() {}\n");
    root
}

fn write_fixture(path: PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

/// The list names the non-default feature, the target that requires it, and the rule that makes
/// the default face blind to files compiled only under it.
/// 清单点名非默认特性、需要它的 target，以及让默认面对"只在它之下编译的文件"失明的那条规则。
#[test]
fn the_faces_name_the_non_default_feature_and_its_required_target() {
    let root = package("faces");
    let lines = faces_lines(&root);
    let joined = lines.join("\n");
    assert!(joined.contains("default=[]"), "{joined}");
    assert!(joined.contains("all=[extra]"), "{joined}");
    assert!(joined.contains("required=[tool -> [extra]]"), "{joined}");
    assert!(
        joined.contains("cannot fail on the default face"),
        "{joined}"
    );
    assert!(
        joined.contains("before believing a green default run"),
        "the note says what to do about it: {joined}"
    );
    assert!(
        !joined.contains("is red"),
        "it must not claim to know which face is red: {joined}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A root Cargo cannot describe says so, rather than reading as a tree with no faces.
/// Cargo 描述不了的根会说出来，而不是被读成一棵没有面的树。
#[test]
fn an_undescribable_root_says_why_instead_of_saying_nothing() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-faces-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("empty root");
    let lines = faces_lines(&root);
    assert_eq!(lines.len(), 1, "{lines:?}");
    // The label is `features`: `face` in this repository means the registration face, and the
    // 2026-10-10 queue item ⑥ is what labelling this line `faces` cost (report §7h).
    // 标签是 `features`：本仓库里 `face` 指注册面，而把这一行标成 `faces` 的代价就是 2026-10-10 队列
    // 第 ⑥ 条（报告 §7h）。
    assert!(lines[0].starts_with("features  unavailable ("), "{lines:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The 2026-10-10 defect queue, item ⑥ (4 reproductions): `status` and `registry` answered two
/// different questions with the same word.
/// 2026-10-10 缺陷队列第 ⑥ 条（4 次复现）：`status` 与 `registry` 用同一个词回答了两个不同的问题。
///
/// Measured on the shipped example through the real bridge: `status` printed
/// `faces  xirang-example-control-button: default=[] all=[]` — true of the **Cargo feature** table —
/// while `registry` listed `faces 3` **registration faces** on the same tree, and `status`'s `next`
/// line then sent the reader down the symbol route "on a tree with no registered face". Nothing in
/// either reply said the two `faces` were different words (report §7h). The fix keeps both answers
/// and names them: the feature line is `features`, and the registration count is derived the same way
/// `registry` derives it, so the two cannot disagree.
/// 用真桥在出厂示例上实测：`status` 印 `faces  xirang-example-control-button: default=[] all=[]`
/// ——对 **Cargo 特性**表而言为真——而 `registry` 在同一棵树上列出 `faces 3` 个**注册面**，接着
/// `status` 的 `next` 行还把读者推向"没有任何注册面的树"那条逐符号路线。两份回复都没有一个字说这两个
/// `faces` 是不同的词（报告 §7h）。修法是两个答案都留、各自点名：特性那行写作 `features`，注册面数用
/// `registry` 同一套推导，因此两者不可能不一致。
#[test]
fn the_two_meanings_of_face_are_labelled_and_agree_with_registry() {
    let root = package("face-meanings");
    // One registration face, in the shape the `apply` path itself writes, so this is the tree both
    // tools see in production rather than a hand-made approximation.
    // 一个注册面，用 `apply` 路径自己写出的形状，因此这就是两个工具在生产里看到的那种树，而不是手搓的近似。
    crate::mcp::apply::apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is added");

    let features = faces_lines(&root).join("\n");
    assert!(
        features.contains("features  face-meanings:"),
        "the Cargo-feature line is labelled with the word that means features: {features}"
    );
    assert!(
        !features.contains("faces  face-meanings"),
        "and nothing is labelled `faces` while meaning features: {features}"
    );
    let (line, counted) = registered_faces_line(&root);
    assert_eq!(counted, Some(1), "{line}");
    assert!(
        line.starts_with("registered faces 1"),
        "the registration faces are counted too: {line}"
    );
    let registry =
        crate::mcp::tools::registry_tool(&root, &json!({})).expect("the registry renders");
    assert!(
        registry.contains("faces 1"),
        "and `registry` agrees with it: {registry}"
    );
    // The wiring, not only the pieces: `status`'s whole reply carries both labelled lines, and it
    // emits its own `next` — which is what keeps the catalogue's fallback hint (the sentence about a
    // tree with **no registered face**) out of a reply about a tree that has one.
    // 接线而不只是零件：`status` 的整份回复同时带着两条各自标名的行，而且自己给一条 `next`——正是这一点
    // 把目录里那条兜底提示（"没有任何注册面的树"那句）挡在"有注册面的树"的回复之外。
    let reply = crate::mcp::tools::status(&root).expect("the status renders");
    assert!(
        reply.contains("features  face-meanings: default=[] all=[extra]"),
        "the feature line is there, labelled: {reply}"
    );
    assert!(
        reply.contains("registered faces 1"),
        "the registration count is in the reply: {reply}"
    );
    assert!(
        reply.lines().any(|line| line.starts_with("next   ")),
        "and the reply has its own next line: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declared Python face that the build does not register is named beside the count, not hidden.
/// 构建未登记的 Python 声明会写在计数旁边，而不是被藏起来。
#[cfg(feature = "lang-python")]
#[test]
fn an_unregistered_python_face_is_named_beside_the_count() {
    let root = package("python-gap");
    crate::mcp::apply::apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the Rust face is added");
    let widgets = root.join("widgets");
    std::fs::create_dir_all(&widgets).expect("python directory");
    std::fs::write(
        widgets.join("gauge.py"),
        "@soil.face(under=faces.label.NODE_ID, named=\"Gauge\")\nclass Gauge:\n    pass\n",
    )
    .expect("python face");

    let (line, counted) = registered_faces_line(&root);
    assert_eq!(counted, Some(1), "{line}");
    assert!(
        line.contains("declared Python face(s) are not registered yet"),
        "the gap is visible where the count is read: {line}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declared JavaScript face is named beside the count with its own language word.
/// 构建未登记的 JavaScript 声明会连同语言名写在计数旁边。
#[cfg(all(feature = "lang-js", feature = "mcp"))]
#[test]
fn an_unregistered_javascript_face_is_named_beside_the_count() {
    let root = package("javascript-gap");
    crate::mcp::apply::apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the Rust face is added");
    let widgets = root.join("widgets");
    std::fs::create_dir_all(&widgets).expect("javascript directory");
    std::fs::write(
        widgets.join("gauge.js"),
        "import { faces, soil } from \"xirang\";\n\n\
         export class Gauge {}\n\n\
         soil.face({\n  under: faces.label.NODE_ID,\n  named: \"Gauge\",\n});\n",
    )
    .expect("javascript face");

    let (line, counted) = registered_faces_line(&root);
    assert_eq!(counted, Some(1), "{line}");
    assert!(
        line.contains("declared JavaScript face(s) are not registered yet"),
        "the gap is visible where the count is read: {line}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declared TypeScript face is named beside the count with its own language word.
/// 构建未登记的 TypeScript 声明会连同语言名写在计数旁边。
#[cfg(all(feature = "lang-ts", feature = "mcp"))]
#[test]
fn an_unregistered_typescript_face_is_named_beside_the_count() {
    let root = package("typescript-gap");
    crate::mcp::apply::apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the Rust face is added");
    let widgets = root.join("widgets");
    std::fs::create_dir_all(&widgets).expect("typescript directory");
    std::fs::write(
        widgets.join("gauge.ts"),
        "import { faces, soil } from \"xirang\";\n\n\
         export class Gauge {\n    reading(): number {\n        return 42;\n    }\n}\n\n\
         soil.face({\n  under: faces.label.NODE_ID,\n  named: \"Gauge\",\n});\n",
    )
    .expect("typescript face");

    let (line, counted) = registered_faces_line(&root);
    assert_eq!(counted, Some(1), "{line}");
    assert!(
        line.contains("declared TypeScript face(s) are not registered yet"),
        "the gap is visible where the count is read: {line}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
