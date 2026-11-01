//! Dependency-free, compile-time SHA-256 node identities.
//! 零依赖、编译期计算的 SHA-256 节点身份。

use std::fmt;
use std::str::FromStr;

/// Version of the identity input and persisted catalog formats.
/// 身份输入与持久化目录格式的版本。
pub const IDENTITY_SCHEMA: &str = "3";

#[path = "node_id.rs"]
mod node_id;
pub use node_id::*;
#[path = "sha256.rs"]
mod sha256;
pub use sha256::*;
#[path = "path_text.rs"]
mod path_text;
pub use path_text::*;

#[cfg(test)]
mod tests {
    use super::{
        NodeId, ROOT_NODE_ID, last_path_segment, manifest_relative_source, root_node_id,
        sha256_hex, strip_path_prefix, strip_prefix,
    };

    const ABC: NodeId = NodeId::from_bytes(b"abc");
    const MULTI_BLOCK: NodeId =
        NodeId::from_bytes(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");

    /// The derived source must equal what the build step used to inject, so
    /// identities survive the switch from injection to derivation.
    /// 推导出的 source 必须等于构建步骤原先注入的值，身份才不受切换影响。
    #[test]
    fn manifest_relative_source_matches_the_injected_form() {
        assert_eq!(
            manifest_relative_source(
                "/home/me/proj",
                "/home/me/proj/src/control/object/button/button.rs"
            ),
            "control/object/button/button.rs"
        );
        // A manifest path that itself contains `src` must not confuse the strip.
        // manifest 路径自身含 `src` 时不能被剥错。
        assert_eq!(
            manifest_relative_source("/home/me/src/proj", "/home/me/src/proj/src/a/b.rs"),
            "a/b.rs"
        );
    }

    /// Declarations outside `src/` keep a usable identity instead of panicking.
    /// 不在 `src/` 下的声明保留一个可用的身份，而不是 panic。
    #[test]
    fn manifest_relative_source_falls_back_outside_src() {
        assert_eq!(
            manifest_relative_source("/home/me/proj", "/home/me/proj/tests/probe.rs"),
            "tests/probe.rs"
        );
        // Not under the manifest at all: keep the original value.
        // 完全不在 manifest 下：保留原值。
        assert_eq!(
            manifest_relative_source("/home/me/proj", "/elsewhere/face.rs"),
            "/elsewhere/face.rs"
        );
        assert_eq!(manifest_relative_source("/home/me/proj", ""), "");
        // A face compiled by another package arrives as an absolute path (a partition fragment mounts
        // the host's file that way), and the plan's identity is the owning package's `src/` tail — so
        // the tail is what has to come back, or the fragment's face hashes where the checkout happens
        // to live. Measured: a fragment's `widget` face hashed `/tmp/…/app/src/…` and disagreed with
        // the plan, failing `assert_static_identity` with `E0080`.
        // 由另一个包编译的面是以绝对路径到来的（分区碎片就是这样挂宿主的文件的），而计划里的身份是
        // **属主包** `src/` 之后那一段——所以必须取回那一段，否则碎片的那个面哈希的是检出恰好在哪。
        // 实测：碎片里那个 `widget` 面哈希了 `/tmp/…/app/src/…`，与计划不一致，`assert_static_identity`
        // 以 `E0080` 失败。
        assert_eq!(
            manifest_relative_source(
                "/tmp/build/crates/app-widgets",
                "/tmp/build/app/src/panel/object/frame/object/widget/widget.rs"
            ),
            "panel/object/frame/object/widget/widget.rs"
        );
        // A **relative** spelling is not a mount: rustc hands one to a crate compiled inside this
        // workspace, and rewriting it would change identities that were never in question (measured:
        // `toolchain/src/call_evidence/tests/collector_integration.rs` must stay itself).
        // **相对**拼法不是挂载：它是本工作区内编译的 crate 从 rustc 那里拿到的拼法，改写它会动到本来没有
        // 疑问的身份（实测：`toolchain/src/call_evidence/tests/collector_integration.rs` 必须原样）。
        assert_eq!(
            manifest_relative_source(
                "/tmp/build/crates/app-widgets",
                "toolchain/src/call_evidence/tests/collector_integration.rs"
            ),
            "toolchain/src/call_evidence/tests/collector_integration.rs"
        );
        // The host itself may live under a directory named `src`; the nearest `src/` is the owner's.
        // 宿主自己可能住在叫 `src` 的目录下；**最近**的那个 `src/` 才是属主的。
        assert_eq!(
            manifest_relative_source(
                "/tmp/build/crates/app-widgets",
                "/home/me/src/proj/src/panel/frame.rs"
            ),
            "panel/frame.rs"
        );
    }

    /// Windows reports the manifest directory with backslashes while the build
    /// step writes `#[path]` with forward slashes. An exact comparison would
    /// leave the absolute path as the identity there, so the two forms must
    /// agree on one relative value.
    /// Windows 以反斜杠给出 manifest 目录，而构建步骤用正斜杠写 `#[path]`。
    /// 精确比较会让身份退化成绝对路径，因此两种形式必须归一到同一个相对值。
    #[test]
    fn manifest_relative_source_normalizes_windows_separators() {
        assert_eq!(
            manifest_relative_source(r"C:\proj", "C:/proj/src/control/object/button/button.rs"),
            "control/object/button/button.rs"
        );
        // A backslash-separated file path behaves the same.
        // 反斜杠分隔的文件路径同样处理。
        assert_eq!(
            manifest_relative_source(r"C:\proj", r"C:\proj\src\control\control.rs"),
            r"control\control.rs"
        );
        // A trailing separator on the manifest does not change the result.
        // manifest 末尾多一个分隔符不影响结果。
        assert_eq!(
            manifest_relative_source("C:/proj/", "C:/proj/src/a.rs"),
            "a.rs"
        );
    }

    #[test]
    fn strip_path_prefix_stops_at_component_boundaries() {
        assert_eq!(
            strip_path_prefix("/work/app/x.rs", "/work/app"),
            Some("/x.rs")
        );
        assert_eq!(
            strip_path_prefix("/work/application/x.rs", "/work/app"),
            None
        );
        assert_eq!(strip_path_prefix("C:/app2/a.rs", "C:/app"), None);
        assert_eq!(strip_path_prefix("/work/app", "/work/app"), Some(""));
    }

    #[test]
    fn strip_path_prefix_treats_both_separators_alike() {
        assert_eq!(strip_path_prefix("C:/a/b", r"C:\a"), Some("/b"));
        assert_eq!(strip_path_prefix(r"C:\a\b", "C:/a"), Some(r"\b"));
        assert_eq!(strip_path_prefix("C:/a/b", "D:/a"), None);
    }

    #[test]
    fn last_path_segment_reads_the_module_name() {
        assert_eq!(last_path_segment("myproj::control::control"), "control");
        assert_eq!(
            last_path_segment("myproj::control::object::button"),
            "button"
        );
        // No separator at all is the whole value; a trailing separator is empty.
        // 没有分隔符时返回原值；尾随分隔符返回空串。
        assert_eq!(last_path_segment("button"), "button");
        assert_eq!(last_path_segment("myproj::"), "");
    }

    #[test]
    fn strip_prefix_reports_matches_honestly() {
        assert_eq!(strip_prefix("/a/b/c", "/a"), Some("/b/c"));
        assert_eq!(strip_prefix("/a/b/c", "/a/b/c"), Some(""));
        assert_eq!(strip_prefix("/a/b/c", "/x"), None);
        // A longer prefix than the value is not a match.
        // 前缀比原值更长时不算匹配。
        assert_eq!(strip_prefix("/a", "/a/b"), None);
    }

    #[test]
    fn matches_the_standard_vector() {
        assert_eq!(ABC.to_string(), "ba7816bf8f01cfea414140de5dae2223");
        assert_eq!(ABC.to_string().parse(), Ok(ABC));
        assert_eq!(MULTI_BLOCK.to_string(), "248d6a61d20638b8e5c026930c3e6039");
    }

    #[test]
    fn path_is_part_of_the_identity() {
        assert_ne!(
            NodeId::from_path("control/object/button/button.rs", "Button"),
            NodeId::from_path(
                "engine/role/style/object/composite_style/object/button/button.rs",
                "Button",
            )
        );
    }

    /// A face must keep one identity on every platform, so the two path
    /// separators hash to the same bytes.
    /// 注册面在每个平台上必须保持同一个身份，因此两种路径分隔符哈希为相同字节。
    #[test]
    fn path_identities_fold_platform_separators() {
        assert_eq!(
            NodeId::from_namespaced_path("app", "control\\control.rs", "Control"),
            NodeId::from_namespaced_path("app", "control/control.rs", "Control")
        );
        assert_eq!(
            NodeId::from_path("a\\b.rs", "A"),
            NodeId::from_path("a/b.rs", "A")
        );
        // Folding must not merge genuinely different paths.
        // 折叠不能把真正不同的路径混为一谈。
        assert_ne!(
            NodeId::from_path("a/b.rs", "A"),
            NodeId::from_path("ab.rs", "A")
        );
    }

    #[test]
    fn path_components_are_unambiguous() {
        assert_ne!(
            NodeId::from_path("ab", "c"),
            NodeId::from_path("a", "bc"),
            "the path/name separator must be part of the identity input"
        );
    }

    #[test]
    fn namespace_changes_node_identity() {
        assert_ne!(
            NodeId::from_namespaced_path("library-a", "src/button.rs", "Button"),
            NodeId::from_namespaced_path("library-b", "src/button.rs", "Button")
        );
    }

    #[test]
    fn namespaced_roots_are_distinct_and_legacy_root_remains_stable() {
        assert_ne!(root_node_id("library-a"), root_node_id("library-b"));
        assert_ne!(ROOT_NODE_ID, root_node_id("library-a"));
    }

    /// Identities are persisted: `.xirang/external-grafts/<selector>/graft.plan`
    /// stores a `NodeId` and parses it back, and a mismatch only warns. These
    /// literals are therefore a COMPATIBILITY PIN, not a snapshot. Relational
    /// assertions cannot replace them — swapping the two digest inputs in
    /// `from_path`, or toggling its `separator` flag, moves both sides of an
    /// `assert_eq!`/`assert_ne!` together and passes. Only a literal catches it.
    /// Do not edit a literal to make this test green unless the identity format
    /// is being deliberately migrated and every recorded plan is re-derived.
    /// 身份会被持久化：`.xirang/external-grafts/<selector>/graft.plan` 存有
    /// `NodeId` 并在读回时解析，不匹配只会给出警告。因此这些字面量是兼容性钉，而不是
    /// 快照。关系型断言无法取代它们——交换 `from_path` 的两个摘要输入，或翻转它的
    /// `separator` 标志，会让 `assert_eq!`/`assert_ne!` 的两侧一起移动并通过。只有
    /// 字面量才能发现。除非是有意迁移身份格式并重新推导每一份已记录的 plan，否则不要
    /// 为了让本测试变绿而改字面量。
    #[test]
    fn pinned_identities_are_an_on_disk_compatibility_contract() {
        let path_id = NodeId::from_path("control/control.rs", "Control");
        let namespaced_id = NodeId::from_namespaced_path("app", "control/control.rs", "Control");
        // The Windows spelling must fold onto the same identity as its slash
        // twin, so it is pinned to the same literal rather than a second value.
        // Windows 写法必须折叠到与其正斜杠孪生相同的身份，因此钉的是同一个字面量，
        // 而不是第二个值。
        let namespaced_windows_id =
            NodeId::from_namespaced_path("app", "control\\control.rs", "Control");
        assert_eq!(namespaced_windows_id, namespaced_id);

        assert_eq!(
            [
                path_id.to_string(),
                namespaced_id.to_string(),
                namespaced_windows_id.to_string(),
                ROOT_NODE_ID.to_string(),
            ],
            [
                // `NodeId::from_path("control/control.rs", "Control")`
                "b7f058b83908f5b3603201bf030674c7",
                // `NodeId::from_namespaced_path("app", "control/control.rs", "Control")`
                "3ba6f15f11b62ea433f6f16acea3512a",
                // the same call with `control\control.rs`
                "3ba6f15f11b62ea433f6f16acea3512a",
                // `ROOT_NODE_ID` = `from_path("<root>", "root")`
                "33515c0bbbf5082cb760eca7c48ff9b6",
            ]
        );
    }

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
