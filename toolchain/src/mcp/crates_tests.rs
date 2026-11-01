//! Pins for `xirang.crates`: the preview discipline and the three writers (audit `M7`, P4).
//! `xirang.crates` 的钉子：预览纪律与三个写入方（审计 `M7`，P4）。

use crate::build_method::crate_plan::crates_dir;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::crates;

/// A host package with a declaration, a lib target and the records a plan reads.
/// 一个宿主包：带声明、lib target，以及规划要读的记录。
fn host(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let area = std::env::temp_dir().join(format!(
        "xirang-mcp-crates-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&area);
    // One level down: a ghost is the host's sibling, so a host directly in the temp directory would
    // plan its packages into `/tmp`.
    // 在下一层：幽灵是宿主的同级，因此直接坐在临时目录里的宿主会把包规划进 `/tmp`。
    let root = area.join("host");
    fs::create_dir_all(root.join("src/panel/frame")).expect("the host's sources");
    fs::write(
        root.join("src/lib.rs"),
        "xirang_toolchain::run_method::host!();\n",
    )
    .expect("the entry");
    fs::write(
        root.join("src/panel/panel.rs"),
        "crate::root_object! { kind: Panel, parent: crate::ROOT_NODE_ID, }\n",
    )
    .expect("the root face");
    fs::write(
        root.join("src/panel/frame/frame.rs"),
        "crate::panel_object! { kind: Frame, parent: crate::panel::NODE_ID, }\n",
    )
    .expect("the fragment's face");
    // A second subtree, so a test can declare a crate that claims something real.
    // 第二棵子树，好让测试能声明一个认领了真实东西的 crate。
    fs::create_dir_all(root.join("src/panel/gauge")).expect("the second subtree");
    fs::write(
        root.join("src/panel/gauge/gauge.rs"),
        "crate::panel_object! { kind: Gauge, parent: crate::panel::NODE_ID, }\n",
    )
    .expect("the second fragment's face");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nxirang-toolchain = {{ path = {:?} }}\n",
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .expect("the host manifest");
    fs::write(
        root.join("add_crates.rs"),
        "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
         pub const SHAPE: Shape = Shape {\n    package_prefix: \"app\",\n\
         crates: &[Crate::named(\"widgets\").at(&[crate::panel::frame::SUBTREE])],\n};\n",
    )
    .expect("the declaration");
    let out = root.join("target/xirang/out");
    fs::create_dir_all(&out).expect("the records directory");
    let mut manifest = String::from(
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls\t\
         parent_node\towns_registry\tlogical_path\n",
    );
    for (node, source, kind) in [
        (
            "00000000000000000000000000000001",
            "panel/panel.rs",
            "Panel",
        ),
        (
            "00000000000000000000000000000002",
            "panel/frame/frame.rs",
            "Frame",
        ),
        (
            "00000000000000000000000000000003",
            "panel/gauge/gauge.rs",
            "Gauge",
        ),
    ] {
        manifest.push_str(&format!(
            "{node}\t{source}\t{kind}\t-\t{kind}\t{kind}\t-\t-\t-\t-\t-\tfalse\t-\n"
        ));
    }
    fs::write(out.join("pruning_manifest.tsv"), manifest).expect("the pruning manifest");
    root
}

/// A host that declares nothing is told what a declaration looks like, not handed an empty table.
/// 没有声明的宿主会被告诉声明长什么样，而不是拿到一张空表。
#[test]
fn a_host_without_a_declaration_is_told_where_one_goes() {
    let root = host("none");
    fs::remove_file(root.join("add_crates.rs")).expect("remove the declaration");
    let answer = crates(&root, &json!({})).expect("answers");
    assert!(answer.contains("declares no crates"), "{answer}");
    assert!(answer.contains("add_crates.rs"), "{answer}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `plan` writes nothing, and says what the two writers would do.
/// `plan` 不写任何东西，并说明两个写入方会做什么。
#[test]
fn plan_is_read_only_and_names_the_writers() {
    let root = host("plan");
    let answer = crates(&root, &json!({"action":"plan"})).expect("answers");
    assert!(answer.contains("partition of `app`"), "{answer}");
    assert!(answer.contains("app-widgets"), "{answer}");
    assert!(answer.contains("app-facade"), "{answer}");
    assert!(
        answer.contains("no action taken"),
        "plan says it took none: {answer}"
    );
    assert!(
        !crates_dir(&root).join("app-widgets").exists(),
        "plan wrote nothing"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A write without `apply` is a preview: the partition is printed and nothing is on disk.
/// 不带 `apply` 的写入是预览：打印拆分，磁盘上什么都不变。
#[test]
fn an_action_without_apply_changes_nothing() {
    let root = host("preview");
    let answer = crates(&root, &json!({"action":"release"})).expect("answers");
    assert!(answer.contains("preview"), "{answer}");
    assert!(answer.contains("apply: true"), "{answer}");
    assert!(
        !crates_dir(&root).join("app-widgets").exists(),
        "the preview wrote nothing"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `apply: true` writes the release shape, reads it back off the disk, and `revert` takes it back.
/// `apply: true` 写下发布形状、从磁盘读回它，而 `revert` 把它收回去。
#[test]
fn applying_writes_the_release_shape_and_reverting_takes_it_back() {
    let root = host("apply");
    let area = root.clone();
    let answer = crates(&root, &json!({"action":"release","apply":true})).expect("writes");
    assert!(answer.contains("wrote"), "{answer}");
    assert!(
        answer.contains("release"),
        "the answer carries the tree as read back: {answer}"
    );
    let manifest = fs::read_to_string(crates_dir(&area).join("app-widgets/Cargo.toml"))
        .expect("the release manifest");
    assert!(manifest.contains("shape = \"release\""), "{manifest}");
    assert!(
        crates_dir(&area)
            .join("app-widgets/src/panel/frame/frame.rs")
            .is_file(),
        "the release shape carries the fragment's own sources"
    );
    assert!(
        crates_dir(&area).join("app-facade/build.rs").is_file(),
        "and the facade exists"
    );

    let answer = crates(&root, &json!({"action":"revert","apply":true})).expect("reverts");
    assert!(answer.contains("wrote"), "{answer}");
    assert!(
        !crates_dir(&area).join("app-widgets").exists(),
        "the ghost is gone"
    );
    assert!(
        !crates_dir(&area).join("app-facade").exists(),
        "the facade is gone"
    );
    let _ = std::fs::remove_dir_all(area);
}

/// An action this tool does not have is refused by name, with the four that exist.
/// 本工具没有的动作按名字被拒，并列出存在的四个。
#[test]
fn an_unknown_action_is_refused_with_the_list() {
    let root = host("unknown");
    let refusal = crates(&root, &json!({"action":"publish"})).expect_err("refuses");
    assert!(refusal.contains("`publish`"), "{refusal}");
    assert!(refusal.contains("`plan`"), "{refusal}");
    assert!(refusal.contains("`revert`"), "{refusal}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The declaration layer is operable: `declare` adds a crate, `undeclare` takes it out, and both
/// preview the exact text before writing it.
/// 声明层可以操作：`declare` 加一个 crate，`undeclare` 把它去掉，两者在写入之前都预览那段确切的文本。
#[test]
fn the_declaration_layer_is_declarable_and_undeclarable() {
    let root = host("declaration");
    let area = root.clone();
    let file = root.join("add_crates.rs");

    // `declare` previews and writes nothing.
    let preview = crates(
        &root,
        &json!({"action":"declare","crate":"gauge","subtree":"crate::panel::gauge::SUBTREE"}),
    )
    .expect("previews");
    assert!(preview.contains("preview:"), "{preview}");
    assert!(
        preview.contains(r#"+        Crate::named("gauge")"#),
        "the preview is the text diff: {preview}"
    );
    let before = fs::read_to_string(&file).expect("the declaration");
    assert!(
        !before.contains("gauge"),
        "the preview left the author's file alone"
    );

    // Applying it writes the entry, and `plan` then sees two crates.
    let applied = crates(
        &root,
        &json!({"action":"declare","crate":"gauge","subtree":"crate::panel::gauge::SUBTREE","apply":true}),
    )
    .expect("writes");
    assert!(applied.contains("gauge"), "{applied}");
    let after = fs::read_to_string(&file).expect("the declaration");
    assert!(after.contains(r#"Crate::named("gauge")"#), "{after}");
    assert!(
        after.contains(r#"Crate::named("widgets")"#),
        "the crate that was already there is untouched: {after}"
    );

    // `undeclare` refuses while the crate's package is on disk, because after the entry is gone
    // nothing knows that directory.
    fs::create_dir_all(crates_dir(&area).join("app-gauge")).expect("the package directory");
    let refusal =
        crates(&root, &json!({"action":"undeclare","crate":"gauge"})).expect_err("refused");
    assert!(refusal.contains("revert"), "{refusal}");
    std::fs::remove_dir_all(crates_dir(&area).join("app-gauge")).expect("take it back");

    // With nothing on disk it removes exactly that entry, and the file keeps the other crate.
    let removed = crates(
        &root,
        &json!({"action":"undeclare","crate":"gauge","apply":true}),
    )
    .expect("writes");
    assert!(
        removed.contains(r#"-        Crate::named("gauge")"#),
        "the reply shows the line that went: {removed}"
    );
    let after = fs::read_to_string(&file).expect("the declaration");
    assert!(!after.contains("gauge"), "{after}");
    assert!(
        after.contains(r#"Crate::named("widgets")"#),
        "and the crate that was not named is untouched: {after}"
    );
    let _ = std::fs::remove_dir_all(area);
}

/// A host that declares nothing gets its first crate declared, file and all — that is what "add a
/// crate layout" means when there is no layout yet.
/// 没有声明的宿主会把第一个 crate 声明出来，连文件一起——那就是"还没有布局时加一个 crate 布局"的含义。
#[test]
fn declaring_the_first_crate_creates_the_declaration() {
    let root = host("first");
    let area = root.clone();
    fs::remove_file(root.join("add_crates.rs")).expect("start with no declaration");
    let answer = crates(&root, &json!({})).expect("answers");
    assert!(answer.contains("declares no crates"), "{answer}");

    crates(
        &root,
        &json!({"action":"declare","crate":"widgets","subtree":"crate::panel::frame::SUBTREE","apply":true}),
    )
    .expect("creates it");
    let file = fs::read_to_string(root.join("add_crates.rs")).expect("the new declaration");
    assert!(
        file.contains(r#"Shape::of("app""#) || file.contains(r#"package_prefix: "app""#),
        "the prefix comes from the host package: {file}"
    );
    let answer = crates(&root, &json!({"action":"plan"})).expect("plans");
    assert!(answer.contains("app-widgets"), "{answer}");
    let _ = std::fs::remove_dir_all(area);
}

/// A missing or empty argument is refused by name, with the shape the caller has to pass.
/// 缺失或为空的参数按名字被拒，并给出调用方必须传的形状。
#[test]
fn a_missing_argument_is_refused_with_its_shape() {
    let root = host("args");
    for (arguments, expected) in [
        (json!({"action":"declare"}), "`crate` is required"),
        (
            json!({"action":"declare","crate":"gauge"}),
            "`subtree` is required",
        ),
        (json!({"action":"undeclare"}), "`crate` is required"),
    ] {
        let refusal = crates(&root, &arguments).expect_err("refused");
        assert!(refusal.contains(expected), "{refusal}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
