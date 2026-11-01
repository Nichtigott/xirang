//! The move fixture the graft pins share: a host whose plan the pin rewrites, and nothing else.
//! 嫁接钉子共用的搬迁夹具：一个计划由钉子改写的主机，别无其他。
//!
//! Test-free on purpose: `test_shape` judges a test-only file that *carries* a test, so the pins keep
//! the test budget and the builders live here (the same shape as `check_fixtures.rs`).
//! 有意不带测试：`test_shape` 判定的是**自带测试**的仅测试文件，因此测试额度留给钉子，构造函数住在这里
//! （与 `check_fixtures.rs` 同一种形状）。

use std::path::PathBuf;

use super::*;
use serde_json::json;

pub(super) struct Tree(pub(super) PathBuf);

impl Tree {
    pub(super) fn fixture() -> Self {
        let root = super::apply_move_tests::scratch_for_grafts();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("Cargo.toml"),
            "[package]\nname=\"move-graft-test\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[workspace]\n").unwrap();
        let tree = Self(root);
        tree.plan("");
        for owner in ["panel", "shelf"] {
            tree.face(owner, "root", true);
        }
        for leaf in ["a", "b", "c"] {
            tree.face(&format!("panel/{leaf}"), "panel", false);
        }
        tree.face("loose", "root", false);
        tree
    }

    pub(super) fn face(&self, module: &str, parent: &str, owns: bool) {
        let leaf = module.rsplit('/').next().unwrap();
        let directory = self.0.join("src").join(module);
        std::fs::create_dir_all(&directory).unwrap();
        let parent = if parent == "root" {
            ROOT_PARENT_SPELLING.to_owned()
        } else {
            format!("crate::{}::NODE_ID", parent.replace('/', "::"))
        };
        let alias = if parent == ROOT_PARENT_SPELLING {
            "root"
        } else {
            parent
                .trim_end_matches("::NODE_ID")
                .rsplit("::")
                .next()
                .unwrap()
        };
        std::fs::write(directory.join(format!("{leaf}.rs")), format!(
            "crate::{alias}_object! {{\n    kind: Kind_{leaf},\n    parent: {parent},\n    needs_registry: {owns},\n    flow: crate::FlowContract::new(crate::ContractId::new(\"move.v1\"), 1, \"Input\", \"Output\"),\n}}\npub fn behavior(n: i32) -> i32 {{ n + 7 }}\n"
        )).unwrap();
    }

    pub(super) fn plan(&self, cuts: &str) {
        std::fs::write(
            self.0.join("src/lib.rs"),
            format!("crate::static_graft_plan!(FRAMEWORK,\n{cuts}\n);\n"),
        )
        .unwrap();
    }

    pub(super) fn applied(&self, node: &str, to: &str, name: Option<&str>, rebind: bool) -> String {
        let mut request = json!({"node": node, "to": to, "name": name});
        if rebind {
            request["rebind_grafts"] = json!(true);
        }
        run_move(&self.0, &self.0, true, &request)
            .expect("the move lands")
            .message
    }

    pub(super) fn refused_with_rebind(&self, node: &str, to: &str) -> String {
        let result = run_move(
            &self.0,
            &self.0,
            true,
            &serde_json::json!({"node": node, "to": to, "rebind_grafts": true}),
        );
        match result {
            Ok(outcome) => panic!("the move was accepted: {}", outcome.message),
            Err(error) => error,
        }
    }

    pub(super) fn refused(&self, node: &str, to: &str, name: Option<&str>) -> String {
        let source = self.0.join("src/panel/b/b.rs");
        let before = std::fs::read(&source).unwrap();
        let result = run_move(
            &self.0,
            &self.0,
            true,
            &json!({"node":node,"to":to,"name":name}),
        );
        let error = result
            .err()
            .expect("graft impact must be refused before moving");
        assert_eq!(std::fs::read(source).unwrap(), before);
        assert!(!self.0.join(".xirang/moves").exists());
        error
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
