//! Pins for the reference judgment: contract first, kind only as a signal, and a reason with every verdict.
//! 参考判定的钉子：先看合同，kind 只是信号，而每个判定都带理由。
//!
//! The rule this replaces was measured wrong (`docs/managed-maintenance/independent-review-2026-10-17.md` §2, P1),
//! so these pins are written against the ruling's four sentences rather than against the old behaviour: kind is a
//! signal · a different type with the same contract is a candidate · the same type is not a reliable specimen ·
//! insufficient information is said instead of being read as an exclusion.
//! 被替换的那条规则被量出错（`docs/managed-maintenance/independent-review-2026-10-17.md` §2 P1），因此这些钉子是照着裁定的
//! 四句话写的，而不是照着旧行为写的：kind 只是信号 · 类型不同但合同相同是候选 · 同类型不是可靠标本 · 信息不足要说出来，而不是
//! 被读成排除。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::section;
use crate::mcp::source_index::SourceFile;

/// A package whose faces live under one parent, with the declarations a test wants to plant.
/// 一个包，面都在同一个父级之下，带着测试想植进去的那些声明。
///
/// The parent face owns the registry, so the leaves under `object/<name>/<name>.rs` resolve their parent through
/// `crate::control::NODE_ID` exactly as the shipped example does.
/// 父面拥有注册机，因此 `object/<name>/<name>.rs` 下的叶子经 `crate::control::NODE_ID` 解析父级，与出厂示例完全一致。
fn package(label: &str, leaves: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-references-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/control")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"references-{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A host whose faces live under one parent.\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/control/control.rs"),
        "#[xirang_toolchain::soil]\n\
         pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Control>()\n\
         \x20\x20\x20\x20.needs_registry(true)\n\
         \x20\x20\x20\x20.under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\n\
         pub struct Control;\n",
    )
    .unwrap();
    for (name, declaration) in leaves {
        let path = root.join(format!("src/control/object/{name}/{name}.rs"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, declaration).unwrap();
    }
    root
}

/// One leaf's declaration: a kind, an optional flow contract and optional capability labels.
/// 一个叶子的声明：一个 kind、可选的 flow 合同与可选的能力标签。
fn leaf(kind: &str, contract: Option<(&str, u32)>, exports: &[&str]) -> String {
    let mut declaration = format!(
        "#[xirang_toolchain::soil]\n\
         pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<{kind}>()\n"
    );
    if !exports.is_empty() {
        declaration.push_str(&format!(
            "    .exports(&[{}])\n",
            exports
                .iter()
                .map(|label| format!("\"{label}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    declaration.push_str("    .under(crate::control::NODE_ID)\n");
    if let Some((id, version)) = contract {
        declaration.push_str(&format!(
            "    .flow(FlowContract::new(ContractId::new(\"{id}\"), {version}, \"In\", \"Out\"))\n"
        ));
    }
    declaration.push_str(";\n\n");
    declaration.push_str(&format!("pub struct {kind};\n"));
    declaration
}

/// The section for one face, from the same inputs the projection gives it.
/// 一个面的那一段，用的输入与投影给的一样。
fn section_for(root: &Path, target: &str) -> String {
    let namespace = crate::mcp::registry::namespace(root).unwrap();
    let views = crate::build_method::face_views(root, &namespace).unwrap();
    let view = views
        .iter()
        .find(|view| view.path.ends_with(&format!("/{target}")))
        .unwrap_or_else(|| panic!("no face `{target}` in {views:?}"));
    let source = format!("src/control/object/{target}/{target}.rs");
    let mut faces = BTreeMap::new();
    faces.insert(source.clone(), view.path.clone());
    let sources = vec![SourceFile {
        relative: source,
        source: String::new(),
        functions: Vec::new(),
        branches: Default::default(),
    }];
    let seeds = [(0usize, 0usize)].into_iter().collect();
    section(root, &sources, &seeds, &faces)
}

fn line_with<'a>(reply: &'a str, needle: &str) -> &'a str {
    reply
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line names {needle:?} in:\n{reply}"))
}

/// The review's own reproduction, now a positive control: a Button and a Slider declare the **same** flow
/// contract, so the different type is a candidate with a reason — not an exclusion.
/// 复核自己的复现，现在是一条正控制：Button 与 Slider 声明了**同一个** flow 合同，因此类型不同的那一个是带理由的候选——
/// 不是排除。
#[test]
fn a_different_kind_with_the_same_contract_is_a_candidate_with_a_reason() {
    let root = package(
        "same-contract",
        &[
            (
                "button",
                &leaf("Button", Some(("control.render", 1)), &["control.render"]),
            ),
            (
                "slider",
                &leaf("Slider", Some(("control.render", 1)), &["control.render"]),
            ),
        ],
    );
    let reply = section_for(&root, "button");
    let candidate = line_with(&reply, "/slider");
    assert!(
        candidate.contains("candidate: same contract"),
        "a different type with the same contract is a candidate: {reply}"
    );
    assert!(
        candidate.contains("control.render@1"),
        "and the reason names the contract it shares: {candidate}"
    );
    assert!(
        candidate.contains("control.render"),
        "and the capability labels it shares: {candidate}"
    );
    assert!(
        !reply.contains("not a candidate"),
        "nothing here may be excluded: {reply}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The ruling's other half: the **same** kind is a signal, not a proof — when neither declaration names a
/// contract the verdict says so, and several faces sharing one kind does not make any of them the model.
/// 裁定的另一半：**同** kind 是信号不是证明——两边都没点名合同时判定会说出来，而多个面共用一个 kind 不会让其中任何
/// 一个成为样板。
#[test]
fn the_same_kind_alone_is_a_weak_signal_and_a_count_is_not_a_role() {
    let root = package(
        "same-kind",
        &[
            ("left", &leaf("Button", None, &[])),
            ("right", &leaf("Button", None, &[])),
            ("note", &leaf("Button", None, &[])),
        ],
    );
    let reply = section_for(&root, "left");
    for sibling in ["/right", "/note"] {
        let line = line_with(&reply, sibling);
        assert!(
            line.contains("candidate (weak)"),
            "the same kind with no contract is a weak candidate: {line}"
        );
        assert!(
            line.contains("not a proof of role"),
            "and the reason says what the signal is not: {line}"
        );
    }
    assert!(
        reply.contains("a count is not a role"),
        "three faces sharing a kind is a pattern, not evidence: {reply}"
    );
    assert!(
        reply.contains("the flow contract was read on 0 of 2 sibling(s)"),
        "the section states how much it actually read: {reply}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A contract mismatch is named with **both** contracts, and the kind is explicitly not what decided it.
/// 合同不一致会点名**两个**合同，并明说不是 kind 决定的。
#[test]
fn a_contract_mismatch_is_named_rather_than_being_called_a_different_role() {
    let root = package(
        "mismatch",
        &[
            (
                "button",
                &leaf("Button", Some(("control.render", 1)), &["control.render"]),
            ),
            (
                "gauge",
                &leaf("Button", Some(("control.measure", 2)), &["control.measure"]),
            ),
        ],
    );
    let reply = section_for(&root, "button");
    let line = line_with(&reply, "/gauge");
    assert!(
        line.contains("not a candidate as written"),
        "a mismatch is stated as a verdict: {line}"
    );
    assert!(
        line.contains("control.render@1") && line.contains("control.measure@2"),
        "both contracts are named: {line}"
    );
    assert!(
        line.contains("the kind is the same, and a kind is not a role"),
        "and the same kind is not what decided it: {line}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The adoption ledger is a signal **with its lease**: a specimen whose bytes moved is reported as lapsed, not as
/// an adopted model.
/// 采信台账是带**租约**的信号：字节动过的标本按失效报，不按已采信的样板报。
#[test]
fn an_adoption_entry_is_a_specimen_only_while_its_lease_holds() {
    let root = package(
        "specimen",
        &[
            ("button", &leaf("Button", None, &[])),
            ("pad", &leaf("Button", None, &[])),
        ],
    );
    let fingerprint = xirang_kernel::adoption::adoption_fingerprint(&[(
        "src/control/object/pad/pad.rs".to_owned(),
        std::fs::read_to_string(root.join("src/control/object/pad/pad.rs")).unwrap(),
    )]);
    std::fs::create_dir_all(root.join(".xirang/adopted")).unwrap();
    std::fs::write(
        root.join(".xirang/adopted/entries"),
        format!(
            "the-pad-is-the-shape|the sibling leaf|read it|maintainer|2026-10-18|src/control/object/pad/pad.rs|{fingerprint}|because\n"
        ),
    )
    .unwrap();
    let held = section_for(&root, "button");
    assert!(
        line_with(&held, "/pad").contains("an adopted specimen (the-pad-is-the-shape)"),
        "a holding lease is named as one: {held}"
    );
    // Move the bytes the decision rested on: the same entry is no longer a specimen.
    // 移动决定所依托的字节：同一条条目不再是标本。
    let changed = format!(
        "{}// moved\n",
        std::fs::read_to_string(root.join("src/control/object/pad/pad.rs")).unwrap()
    );
    std::fs::write(root.join("src/control/object/pad/pad.rs"), changed).unwrap();
    let lapsed = section_for(&root, "button");
    assert!(
        line_with(&lapsed, "/pad").contains("that lease has lapsed, so it is not a specimen"),
        "a lapsed lease is reported as lapsed: {lapsed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// An unreadable registration surface is still `unknown`, never an empty list: an empty list would be read as a
/// finding about the tree.
/// 读不到注册面时仍然是 `unknown`，绝不是空清单：空清单会被读成关于这棵树的一个结论。
#[test]
fn a_section_that_cannot_read_the_faces_says_unknown_instead_of_nothing() {
    let root =
        std::env::temp_dir().join(format!("xirang-references-unknown-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    let sources = vec![SourceFile {
        relative: "src/lib.rs".to_owned(),
        source: String::new(),
        functions: Vec::new(),
        branches: Default::default(),
    }];
    let mut faces = BTreeMap::new();
    faces.insert("src/lib.rs".to_owned(), "root/control/button".to_owned());
    let seeds: BTreeSet<(usize, usize)> = [(0usize, 0usize)].into_iter().collect();
    let reply = section(&root, &sources, &seeds, &faces);
    assert!(
        reply.starts_with("references unknown:") && reply.contains("could not be read"),
        "an unreadable registry is not an empty list, and it says what it could not read: {reply}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
