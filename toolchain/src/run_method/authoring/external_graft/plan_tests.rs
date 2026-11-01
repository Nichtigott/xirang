//! External-graft plan authoring tests.
//! 外部 graft 计划创作测试。
//!
//! A separate page so the module under test stays inside the size ratchet: a test
//! module is excluded from it, and this one was the larger half of the file.
//! 独立一页，使被测模块留在尺寸棘轮之内：测试模块不受棘轮约束，而这一份是文件里更大的
//! 那一半。

use std::path::Path;

use super::*;
use crate::run_method::AuthoringContext;

/// A record whose text does not parse can still be removed. It has a
/// directory, and removing it is the repair; reading it stays an error, and
/// the trash keeps the bytes so a mistaken removal is reversible by hand.
/// 文本解析不了的记录仍然可以被删除。它有目录，删掉它就是修复；读取它仍然是错误，而
/// 回收目录保留字节，因此删错时仍可手工恢复。
#[test]
fn a_broken_record_can_still_be_removed() {
    with_temp_root(|_root| {
        let selector = "broken_graft";
        let directory = external_graft_root().join(selector);
        fs::create_dir_all(&directory).expect("record directory");
        fs::write(
            directory.join(lexicon::GRAFT_PLAN_FILE),
            "graft = [unclosed\n",
        )
        .expect("broken plan text");

        assert!(
            read_external_graft(selector).is_err(),
            "the text really is unreadable"
        );
        let trash = remove_external_graft(selector).expect("a broken record must be removable");
        assert!(trash.is_dir(), "{trash:?}");
        assert!(!directory.exists(), "the record directory is gone");
        assert!(
            fs::read_to_string(trash.join(lexicon::GRAFT_PLAN_FILE))
                .expect("the bytes are kept")
                .contains("unclosed"),
            "the trash keeps the bytes so a mistaken removal stays reversible"
        );

        // A selector names one directory; it is not a path into the tree.
        // 选择器命名的是一个目录，不是通往树里的路径。
        assert!(
            external_graft_directory("../escape").is_err(),
            "a selector must not walk out of the record directory"
        );
        assert!(
            remove_external_graft(selector).is_err(),
            "removing a record twice is an error"
        );
    });
}

/// Run one plan test against a throwaway package root.
/// 在一个临时包根上运行一条计划测试。
///
/// The name carries a counter as well as the clock: two tests can start in
/// the same nanosecond on a platform with a coarse clock, and sharing one
/// root made them overwrite each other's plan.
/// 名字里除了时钟还有一个计数器：在时钟精度较粗的平台上两个测试可能落在同一纳秒，
/// 共用一个根目录就会互相覆盖对方的计划。
fn with_temp_root<T>(operation: impl FnOnce(&Path) -> T) -> T {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-external-graft-{}-{stamp}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create root");
    let result = AuthoringContext::new(root.clone(), "xirang.test").scope(|| operation(&root));
    let _ = std::fs::remove_dir_all(&root);
    result
}

/// A registry carrying one registered face, so a plan has a real target.
/// 带一个已注册面的注册机，使计划有真实目标。
fn registry_with_button() -> Registry {
    use crate::run_method::registry_core::{
        Admission, NodeId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
        OwnedSourceLocation, RegistrationRule, RegistrationSnapshot, root_node_id,
    };

    let namespace = "xirang.test";
    let kind = "Button";
    let mut registry = Registry::root_for_namespace(
        crate::run_method::FrameworkId::new("xirang.test"),
        namespace,
    );
    registry
        .register_snapshot_batch([RegistrationSnapshot {
            namespace: namespace.to_owned(),
            id: NodeId::from_namespaced_path(namespace, "control/object/button/button.rs", kind),
            parent: root_node_id(namespace),
            kind: kind.to_owned(),
            preset: "NoPreset".to_owned(),
            parts: "NoParts".to_owned(),
            params: kind.to_owned(),
            handle: kind.to_owned(),
            stable_name: None,
            name: OwnedLocalizedText {
                zh: kind.to_owned(),
                en: kind.to_owned(),
            },
            summary: OwnedLocalizedText {
                zh: String::new(),
                en: String::new(),
            },
            exports: Vec::new(),
            needs_registry: false,
            registry_name: "button".to_owned(),
            getting_from_other_registry: None,
            registry_rule_path: "<test>".to_owned(),
            registry_rule: RegistrationRule::ANY.into_owned(),
            admission: Admission::ANY.into_owned(),
            requires: Vec::new(),
            provides: Vec::new(),
            contract: OwnedObjectContract {
                required_parts: Vec::new(),
                provided_parts: Vec::new(),
            },
            flow: OwnedFlowContract::none(),
            flow_provider: None,
            handle_traits: Vec::new(),
            part_traits: Vec::new(),
            runtime_checks: Vec::new(),
            plugin: None,
            source: OwnedSourceLocation {
                file: "control/object/button/button.rs".to_owned(),
                line: 1,
                column: 1,
                function: kind.to_owned(),
            },
        }])
        .expect("button registers");
    registry
}

/// The identity of the one face `registry_with_button` registers.
/// `registry_with_button` 注册的那个面的身份。
fn button_target(registry: &Registry) -> NodeId {
    registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "button")
        .expect("button is registered")
        .id
}

/// Creating a plan whose selector would land outside the record directory is an
/// error, and it creates nothing on the way: `..` used to be accepted, written to
/// `<pkg>/.xirang/graft.plan`, and then never listed — a record the runtime
/// never applied and the author could not see.
/// 用会落到记录目录之外的选择器创建计划是错误，而且过程中不创建任何东西：`..` 过去会被接受、
/// 写到 `<pkg>/.xirang/graft.plan`，然后又永远不会被列出——一份运行期从不应用、作者也看不到
/// 的记录。
#[test]
fn a_selector_that_escapes_the_record_directory_creates_nothing() {
    with_temp_root(|_root| {
        let registry = registry_with_button();
        let target = button_target(&registry);
        for escaped in ["..", ".", ".hidden"] {
            let error = create_external_graft(&registry, target, escaped, false)
                .expect_err("a selector must not leave the record directory");
            assert!(
                error.contains("selector") || error.contains("name"),
                "the refusal must explain itself: {error}"
            );
        }
        // Nothing was created for any of them, and the plan directory is untouched.
        // 它们一个都没有创建任何东西，计划目录保持原样。
        assert!(
            !external_graft_root()
                .join("..")
                .join(lexicon::GRAFT_PLAN_FILE)
                .exists(),
            "no escapee plan may be written"
        );
    });
}

#[test]
fn a_created_plan_round_trips_is_listed_and_moves_to_trash() {
    with_temp_root(|root| {
        let registry = registry_with_button();
        let created =
            create_external_graft(&registry, button_target(&registry), "button_graft", false)
                .expect("plan is created");
        assert_eq!(created.selector, "button_graft");
        assert_eq!(created.target_path(), "root/button");
        assert_eq!(created.graft(), "button_graft");
        assert!(!created.full());

        let text = fs::read_to_string(created.plan_path()).expect("plan text");
        assert_eq!(
            GraftPlanDocument::parse_graft_plan_document(&text).expect("plan parses"),
            created.document
        );
        assert!(text.contains("target_path=root/button\n"), "{text}");
        assert!(text.contains("full=false\n"), "{text}");

        assert_eq!(
            read_external_graft("button_graft").expect("read back"),
            created
        );
        let listed = list_external_grafts().expect("list plans");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].selector, "button_graft");
        assert_eq!(listed[0].document(), Ok(&created.document));

        let rewritten = rewrite_external_graft("button_graft", true).expect("toggle full");
        assert!(rewritten.full());
        assert!(
            read_external_graft("button_graft")
                .expect("read back")
                .full()
        );
        assert_eq!(
            rewrite_external_graft("button_graft", true).expect("idempotent"),
            rewritten
        );

        let trash = remove_external_graft("button_graft").expect("remove");
        assert!(
            trash.starts_with(
                root.join(lexicon::XIRANG_DIR)
                    .join("trash")
                    .join(lexicon::EXTERNAL_GRAFT_DIR),
            )
        );
        assert!(trash.join(lexicon::GRAFT_PLAN_FILE).is_file());
        assert!(list_external_grafts().expect("list plans").is_empty());
    });
}

#[test]
fn a_duplicate_selector_is_refused_with_its_path() {
    with_temp_root(|_| {
        let registry = registry_with_button();
        let target = button_target(&registry);
        create_external_graft(&registry, target, "button_graft", false).expect("first plan");
        let error = create_external_graft(&registry, target, "button_graft", true)
            .expect_err("duplicate selector is refused");
        assert!(error.contains("already exists"), "{error}");
        assert!(error.contains(lexicon::GRAFT_PLAN_FILE), "{error}");
    });
}

#[test]
fn selectors_that_escape_or_confuse_a_plan_are_refused() {
    with_temp_root(|_| {
        let registry = registry_with_button();
        let target = button_target(&registry);
        for selector in ["", "   ", "a/b", "a\\b"] {
            assert!(
                create_external_graft(&registry, target, selector, false).is_err(),
                "selector `{selector}` must be refused"
            );
        }
        assert!(read_external_graft("a/b").is_err());
    });
}

#[test]
fn a_broken_plan_is_listed_with_its_reason() {
    with_temp_root(|_| {
        let broken = external_graft_root().join("broken");
        fs::create_dir_all(&broken).expect("create directory");
        fs::write(
            broken.join(lexicon::GRAFT_PLAN_FILE),
            "version=9\ntarget_path=root\ngraft=x\nfull=false\n",
        )
        .expect("write");
        let listed = list_external_grafts().expect("list plans");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].selector, "broken");
        let reason = listed[0].document().expect_err("a broken plan is reported");
        assert!(reason.contains("version `9`"), "{reason}");
    });
}

#[test]
fn listing_a_package_without_plans_is_empty() {
    with_temp_root(|_| {
        assert!(list_external_grafts().expect("list plans").is_empty());
    });
}
