//! Cut-to-module matching shared by the build and the authoring surfaces.
//! 构建与创作界面共用的"切口到模块"匹配规则。
//!
//! A string cut names a registry path and a typed cut names a Rust expression
//! that points at a face's `NODE_ID`; both map onto the same module vocabulary
//! here, together with the plugin-surface rule pruning follows.
//! 字符串切口命名注册路径，类型化切口命名指向注册面 `NODE_ID` 的 Rust 表达式；
//! 两者在这里都映射到同一套模块词汇，修剪所遵循的插件面规则也在此处。

use std::path::Path;

use xirang_kernel::lexicon;

use crate::build_method::{FaceSource, relative_display};

/// The two endpoints of a string cut, as modules.
/// 字符串切口两个端点对应的模块。
///
/// The far endpoint arrives as data (`cut_end`), so this never inspects the
/// path text: a path that literally contains `" to "` is one target, and reading
/// it as a range here is exactly the bug this signature removes. `cut_end` is
/// `None` for a single target and for a cut at `root`, which forces the whole
/// tree.
/// 远端端点以数据（`cut_end`）传入，因此这里绝不检查路径文本：字面含有 `" to "`
/// 的路径是单个目标，把它当区间读正是本次签名改动消除的缺陷。`cut_end` 为 `None`
/// 表示单个目标或落在 `root` 上、强制保留全树的切口。
pub(crate) fn string_cut_modules(
    cut: &str,
    cut_end: Option<&str>,
) -> (Option<String>, Option<String>) {
    match cut_end {
        Some(end) => (graft_cut_module(cut), graft_cut_module(end)),
        None => (graft_cut_module(cut), None),
    }
}

fn graft_cut_module(cut: &str) -> Option<String> {
    if cut == "root" {
        return None;
    }
    let module = cut.trim_start_matches("root/").replace('/', "::");
    (!module.is_empty()).then_some(module)
}

/// Derive the module a typed graft cut targets from its Rust expression.
/// 从类型化 graft 切口的 Rust 表达式推导它指向的模块。
///
/// The expression is a path to a face's `NODE_ID`, so matching it against the
/// discovered face modules is exact: it does not depend on the registry path
/// agreeing with the module path.
/// 表达式是指向某个注册面 `NODE_ID` 的路径，因此与已发现的注册面模块逐一匹配
/// 是精确的：它不依赖注册路径与模块路径一致。
pub(crate) fn graft_expression_module(expression: &str, faces: &[FaceSource]) -> Option<String> {
    let expression = expression.trim();
    // Hosts spell typed cuts from the crate root, while a face's module name is
    // relative: without stripping the prefix the compare missed every real cut.
    // 宿主从 crate 根书写类型化切口，而注册面的模块名是相对的：不去掉前缀就永远匹配不到
    // 真实切口。
    let expression = expression
        .strip_prefix("crate::")
        .or_else(|| expression.strip_prefix("self::"))
        .unwrap_or(expression);
    faces
        .iter()
        .find(|face| expression == format!("{}::NODE_ID", face.module))
        .map(|face| face.module.clone())
}

/// A face that declares a `plugin:` field is replaceable plugin surface and
/// must survive pruning regardless of reachability.
/// 声明了 `plugin:` 字段的注册面是可替换的插件面，无论可达性都必须存活。
pub(crate) fn face_declares_plugin(src: &Path, face: &FaceSource) -> bool {
    let relative = relative_display(src, &face.source);
    // Verbatim text is what this asks for, and both `field` on a parsed face and the unit's `field` row
    // answer it — so the facts path is the same question, not an approximation (`stem 2`).
    // 它问的是**逐字原文**，而"解析出的面上的 `field`"与"unit 的 `field` 行"回答的是同一件事——因此事实路径
    // 是同一个问题，不是近似（`stem 2`）。
    crate::build_method::discovery_cache::face_raw_field(
        &face.source,
        &relative,
        lexicon::FACE_FIELD_PLUGIN,
    )
    .is_some()
}

#[cfg(test)]
mod tests {
    use super::{face_declares_plugin, graft_cut_module};
    use crate::build_method::FaceSource;
    use std::path::PathBuf;

    /// A range cut has two endpoints and neither may be dropped; a single path
    /// that happens to contain the range word is still one endpoint.
    /// 区间切口有两个端点，哪个都不能丢；恰好含有区间词的单条路径仍然只是一个端点。
    #[test]
    fn a_range_cut_keeps_both_endpoints() {
        assert_eq!(
            super::string_cut_modules("root/a", Some("root/b")),
            (Some("a".to_owned()), Some("b".to_owned()))
        );
        assert_eq!(
            super::string_cut_modules("root/a", None),
            (Some("a".to_owned()), None)
        );
        assert_eq!(super::string_cut_modules("root", None), (None, None));
        assert_eq!(
            super::string_cut_modules("root/a to b", None),
            (Some("a to b".to_owned()), None)
        );
    }

    fn face(module: &str, source: PathBuf) -> FaceSource {
        FaceSource {
            id: crate::build_method::registry_identity::package_node_id(
                &source.to_string_lossy(),
                "Kind",
            ),
            source,
            module: module.to_owned(),
        }
    }

    #[test]
    fn a_typed_cut_is_recognized_from_the_crate_root() {
        let faces = vec![face(
            "control::object::button",
            PathBuf::from("src/button.rs"),
        )];
        assert_eq!(
            super::graft_expression_module("control::object::button::NODE_ID", &faces).as_deref(),
            Some("control::object::button")
        );
        assert_eq!(
            super::graft_expression_module("crate::control::object::button::NODE_ID", &faces)
                .as_deref(),
            Some("control::object::button")
        );
    }

    #[test]
    fn graft_cut_module_maps_registry_path_to_module() {
        assert_eq!(graft_cut_module("root/b").as_deref(), Some("b"));
        assert_eq!(graft_cut_module("root/a/b").as_deref(), Some("a::b"));
        assert_eq!(graft_cut_module("root"), None);
    }

    #[test]
    fn plugin_declaring_face_is_detected_from_source_text() {
        let root = std::env::temp_dir().join(format!(
            "xirang-forced-roots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let plain = root.join("a/a.rs");
        let plugin = root.join("c/c.rs");
        std::fs::create_dir_all(plain.parent().expect("parent")).expect("mkdir");
        std::fs::create_dir_all(plugin.parent().expect("parent")).expect("mkdir");
        std::fs::write(
            &plain,
            "crate::root_object! {\n    kind: A,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("write plain face");
        std::fs::write(
            &plugin,
            "crate::root_object! {\n    kind: C,\n    plugin: manifest,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("write plugin face");

        assert!(!face_declares_plugin(&root, &face("a", plain.clone())));
        assert!(face_declares_plugin(&root, &face("c", plugin.clone())));

        let _ = std::fs::remove_dir_all(&root);
    }
}
