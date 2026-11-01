//! File-backed authoring for XiRang registration faces.
//! XiRang 注册面的文件化创作支持。

use std::collections::BTreeMap;
use std::path::Path;

use crate::run_method::NodeId;

use super::context::{authoring_namespace, rule_path_for_source};
use super::manifest::FaceManifest;

/// A source-tree mutation that needs one rebuild before it becomes executable.
/// 一次源码树变更；它需要经过一次重建才会成为可执行注册面。
impl FaceManifest {
    /// The face's fields, in two classes that must not be confused.
    /// 注册面的字段，分两类，不可混淆。
    ///
    /// * **Renderable fields** — everything the template writes back. They are
    ///   exactly what [`FaceManifest::edit`] accepts, and what the two field-order
    ///   tables in `operations/face_write.rs` carry.
    /// * **Derived index keys** — `namespace`, `module`, `parent_node`,
    ///   `parent_source`, `parent_kind`, `provided_parts`, `required_parts`. The
    ///   renderer *reads* them to decide what to write; a caller cannot set them,
    ///   and `edit` refuses their names.
    ///
    /// `registry_name`, `handle`, and `params` were a third, dishonest class:
    /// accepted by `edit` (and seeded here) while no template emitted them, so an
    /// edit to one returned `Ok` and was dropped at the next save. They are gone
    /// (audit `LG-38`); a derived `registry_name` is `module`, and a derived
    /// `handle` is `kind`, both computed by the declaration macro at the
    /// declaration site rather than stored in a file.
    /// 字段分两类，不可混淆。
    ///
    /// * **可落盘字段**——模板会写回的全部字段。它们恰好是 [`FaceManifest::edit`] 接受的集合，
    ///   也是 `operations/face_write.rs` 里那两张字段顺序表的成员。
    /// * **派生索引键**——`namespace`、`module`、`parent_node`、`parent_source`、
    ///   `parent_kind`、`provided_parts`、`required_parts`。渲染器**读**它们来决定写什么；
    ///   调用方设不了它们，`edit` 也会拒绝这些名字。
    ///
    /// `registry_name`、`handle` 与 `params` 曾是第三类、也是不诚实的一类：`edit` 接受（并在此
    /// 播种）它们，而没有任何模板发射它们，因此对其中一个的编辑会返回 `Ok`、并在下次保存时被丢掉。
    /// 它们已被删除（审计 `LG-38`）；派生的 `registry_name` 就是 `module`、派生的 `handle` 就是
    /// `kind`，两者都在声明点由声明宏算出，而不是存在文件里。文件形式的面不存 `handle`，但
    /// `to_snapshot` 会把同一个派生值补给内核——内核用原始 `handle` 填 `source.function`。见
    /// `authoring/snapshot.rs`。
    pub(crate) fn new(
        name: &str,
        kind: &str,
        parent: NodeId,
        parent_source: &str,
        parent_kind: &str,
        source: &str,
    ) -> Self {
        let mut values = BTreeMap::new();
        let namespace = authoring_namespace();
        for (key, value) in [
            ("namespace", namespace.as_str()),
            ("module", name),
            ("kind", kind),
            ("preset", "NoPreset"),
            ("parts", "NoParts"),
            ("registration_rule", "ANY"),
            ("admission", "ANY"),
            ("source", source),
            ("parent_node", &parent.to_string()),
            ("parent_source", parent_source),
            ("parent_kind", parent_kind),
            ("name_zh", ""),
            ("name_en", ""),
            ("summary_zh", ""),
            ("summary_en", ""),
            ("stable_name", ""),
            ("exports", ""),
            ("provides", ""),
            ("needs_registry", "false"),
            ("getting_from_other_registry", ""),
            ("handle_traits", ""),
            ("handle_contracts", ""),
            ("part_traits", ""),
            ("part_contracts", ""),
            ("requires", ""),
            ("runtime_checks", ""),
            ("flow", ""),
            ("flow_provider", ""),
        ] {
            values.insert(key.to_owned(), value.to_owned());
        }
        values.insert(
            "registry_rule_path".to_owned(),
            rule_path_for_source(source),
        );
        values.insert("provided_parts".to_owned(), String::new());
        values.insert("required_parts".to_owned(), String::new());
        Self { values }
    }

    pub(super) fn parse_source(
        path: &Path,
        identity_prefix: &str,
        parents: Option<&crate::run_method::authoring::parse::ParentMap>,
    ) -> Result<Self, String> {
        super::manifest::parse::source(path, identity_prefix, parents)
    }
}

pub use super::context::AuthoringContext;
pub use super::external_graft::{
    ExternalGraftPlanEntry, ExternalGraftPlanFile, create_external_graft, external_graft_root,
    list_external_grafts, read_external_graft, remove_external_graft, rewrite_external_graft,
};
pub use super::operations::{
    AuthoredFace, AuthoringChange, ModuleFacePatch, NewModuleFace, add_module,
    add_module_from_face, add_module_with_registration, authored_face, delete_module,
    edit_module_face, generated_snapshots, generated_snapshots_from, generated_snapshots_in,
};

// Field dictionary and slot names shared by the Studio form and the file
// authoring API. The definitions live in the kernel `authoring` module.
// Studio 表单与文件创作 API 共用的字段词典与槽位名；定义位于 kernel 的
// `authoring` 模块。
pub use xirang_kernel::authoring::{FACE_FIELD_COUNT, face_field};
