//! Field-write policy and the one applier both add and edit reach.
//! 字段写入策略，以及 add 与 edit 共用的唯一应用器。

use super::*;

/// How the shared applier writes values into a manifest.
/// 共享应用器如何把字段写入清单。
#[derive(Clone, Copy)]
pub(super) enum FaceWrite {
    /// Creating a face: trim each value and keep the manifest default when blank.
    /// 创建注册面：裁剪每个值；为空时保留清单默认值。
    Create,
    /// Editing a face: write every value verbatim, blank included.
    /// 编辑注册面：逐字写入每个值，包括空值。
    Edit,
}

impl FaceWrite {
    /// The field order this path has always used, kept verbatim so the first
    /// validation error it reports does not move.
    /// 本路径一直使用的字段顺序，逐字保留，避免它报告的首个校验错误发生变化。
    pub(super) fn field_order(self) -> &'static [&'static str] {
        match self {
            Self::Create => CREATE_FIELD_ORDER,
            Self::Edit => EDIT_FIELD_ORDER,
        }
    }
}

/// The add path's field order.
/// add 路径的字段顺序。
const CREATE_FIELD_ORDER: &[&str] = &[
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "exports",
    "stable_name",
    "getting_from_other_registry",
    "requires",
    "provides",
    "runtime_checks",
    "flow",
    "flow_provider",
    "handle_traits",
    "handle_contracts",
    "part_traits",
    "part_contracts",
    "needs_registry",
    "registration_rule",
    "admission",
];

/// The edit path's field order.
/// edit 路径的字段顺序。
const EDIT_FIELD_ORDER: &[&str] = &[
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "exports",
    "stable_name",
    "needs_registry",
    "getting_from_other_registry",
    "registration_rule",
    "admission",
    "handle_traits",
    "part_traits",
    "requires",
    "provides",
    "runtime_checks",
    "flow",
    "flow_provider",
];

/// Write every shared field the path's field order names into `face`. Both public
/// entry points reach the field-by-field handling below through this one loop, so
/// a new field is wired once instead of once per struct. The two tables below are
/// the count — this comment deliberately carries no number, because the three
/// that used to sit in `face_write.rs`, `face_values.rs` and `macro/src/mirror.rs`
/// disagreed with each other *and* with the tables (audit `SUR-C6`).
/// 把本路径字段顺序点名的每个共用字段写入 `face`。两个公开入口都经由这一个循环抵达下面的
/// 逐字段处理，因此新增字段只需接一次线，而不是每个结构体各接一次。数量就是下面那两张表——
/// 这条注释有意不写数字，因为过去分散在 `face_write.rs`、`face_values.rs` 与 `macro/src/mirror.rs`
/// 的那三个数字彼此不一致，也都不等于表长（审计 `SUR-C6`）。
pub(super) fn apply_module_face_values(
    face: &mut FaceManifest,
    values: &ModuleFaceValues<'_>,
    write: FaceWrite,
) -> Result<(), String> {
    for field in write.field_order() {
        apply_face_value(face, values, field, write)?;
    }
    Ok(())
}

/// One field's conversion, validation, and application.
/// 单个字段的转换、校验与应用。
pub(super) fn apply_face_value(
    face: &mut FaceManifest,
    values: &ModuleFaceValues<'_>,
    field: &str,
    write: FaceWrite,
) -> Result<(), String> {
    match field {
        "kind" => edit_kind(face, values.kind, write),
        "needs_registry" => face.edit("needs_registry", &values.needs_registry.to_string()),
        // Trait labels follow the compiler-checked paths when there are any, and
        // stand alone when there are none — the same rule the macro applies
        // through `__face_trait_labels_or!`. Both paths therefore keep the same
        // editing rule, and the helper is the whole handling.
        // trait 标签在有参与编译检查的路径时跟随路径，没有时独立成立——与宏经
        // `__face_trait_labels_or!` 施加的规则相同。两条路径的编辑规则因此一致，辅助函数
        // 就是全部处理。
        "handle_traits" => apply_trait_label(
            face,
            "handle_traits",
            values.handle_traits,
            values.handle_contracts,
        ),
        "part_traits" => apply_trait_label(
            face,
            "part_traits",
            values.part_traits,
            values.part_contracts,
        ),
        // These are validated and written on every edit, blank included.
        // 这些字段每次编辑都会校验并写入，包括空值。
        "registration_rule" | "admission" => edit_required(face, field, values.value(field), write),
        _ => edit_optional(face, field, values.value(field), write),
    }
}

/// Write the kind after normalizing it. A blank create keeps the kind derived
/// from the module name; a blank edit is still validated and reported.
/// 规范化后写入 kind。创建时的空值保留由模块名派生的 kind；编辑时的空值仍会
/// 校验并报错。
pub(super) fn edit_kind(
    face: &mut FaceManifest,
    kind: &str,
    write: FaceWrite,
) -> Result<(), String> {
    if matches!(write, FaceWrite::Create) && kind.trim().is_empty() {
        return Ok(());
    }
    face.edit("kind", &normalize_kind_name(kind.trim()))
}

/// Write a field that may legitimately be blank. A create trims and skips a
/// blank value so the manifest default survives; an edit writes it through.
/// 写入允许为空的字段。创建裁剪并跳过空值，保留清单默认值；编辑则如实写入。
pub(super) fn edit_optional(
    face: &mut FaceManifest,
    field: &str,
    value: &str,
    write: FaceWrite,
) -> Result<(), String> {
    match write {
        FaceWrite::Create => {
            let value = value.trim();
            if value.is_empty() {
                Ok(())
            } else {
                face.edit(field, value)
            }
        }
        FaceWrite::Edit => face.edit(field, value),
    }
}

/// Write a field the face always carries. A create trims it; an edit keeps it
/// exactly as the patch delivered it.
/// 写入注册面始终携带的字段。创建裁剪它；编辑按补丁原样保留。
pub(super) fn edit_required(
    face: &mut FaceManifest,
    field: &str,
    value: &str,
    write: FaceWrite,
) -> Result<(), String> {
    match write {
        FaceWrite::Create => face.edit(field, value.trim()),
        FaceWrite::Edit => face.edit(field, value),
    }
}

/// Write the label field from the contract paths when there are paths, and from
/// the author's labels when there are none.
/// 有路径时由路径写标签字段，没有路径时由作者的标签写入。
///
/// It writes the *label*, never the contract line: the `handle_contracts:` /
/// `part_contracts:` rows are emitted by the **create** order
/// ([`CREATE_FIELD_ORDER`]), and the edit order does not carry them, so renaming
/// this helper `apply_trait_label` is what keeps its name equal to what it does.
/// Whether the edit path should also rewrite the contract rows is the MCP
/// `apply edit` finding (`LG-21`: it returned success while leaving the old
/// contract in place) and is decided where that path's field set is declared.
/// 它写的是**标签**，绝不是契约行：`handle_contracts:` / `part_contracts:` 两行由**创建**顺序
/// （[`CREATE_FIELD_ORDER`]）发射，编辑顺序里没有它们，因此这个辅助函数叫
/// `apply_trait_label` 才与它的行为相等。编辑路径是否也该改写契约行是 MCP `apply edit` 那条
/// 发现（`LG-21`：它返回成功却留着旧契约），由那条路径的字段集合的声明处裁定。
pub(super) fn apply_trait_label(
    face: &mut FaceManifest,
    label_field: &str,
    labels: &str,
    contract_paths: &str,
) -> Result<(), String> {
    if labels.trim().is_empty() && contract_paths.trim().is_empty() {
        return Ok(());
    }
    let labels = if contract_paths.trim().is_empty() {
        labels.trim().to_owned()
    } else {
        trait_names_from_paths(contract_paths)?
    };
    face.edit(label_field, &labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an add request and an edit patch carrying the same 28 values, so one
    /// fixture drives both public shapes and proves they stay in step.
    /// 构建携带同一组 28 个取值的 add 请求与 edit 补丁；一份夹具同时驱动两个公开
    /// 结构体，证明二者保持一致。
    macro_rules! shared_face_fields {
        ($($field:ident = $value:expr),* $(,)?) => {
            (
                NewModuleFace {
                    $($field: $value,)*
                    parent: ROOT_NODE_ID,
                },
                ModuleFacePatch {
                    $($field: $value,)*
                },
            )
        };
    }

    /// One fully populated pair of shared values.
    /// 一组完整填充的共用取值。
    fn fixture() -> (NewModuleFace<'static>, ModuleFacePatch<'static>) {
        shared_face_fields!(
            module = "widget",
            kind = "Widget",
            preset = "NoPreset",
            parts = "NoParts",
            name_zh = "部件",
            name_en = "Widget",
            summary_zh = "摘要",
            summary_en = "Summary",
            exports = "a,b",
            stable_name = "widget",
            needs_registry = false,
            getting_from_other_registry = "",
            registration_rule = "ANY",
            admission = "ANY",
            handle_traits = "",
            handle_contracts = "",
            part_traits = "",
            part_contracts = "",
            requires = "",
            provides = "widget",
            runtime_checks = "",
            flow = "",
            flow_provider = "",
        )
    }

    /// The manifest an add starts from, before any field is applied.
    /// add 在应用任何字段之前所依据的清单。
    fn blank_manifest() -> FaceManifest {
        FaceManifest::new(
            "widget",
            "Widget",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "widget/widget.rs",
        )
    }

    /// The shared fields are handled once: both public shapes flow through the
    /// same applier, so identical values produce identical faces.
    /// 共用字段只处理一次：两个公开结构体流经同一个应用器，相同取值产生相同
    /// 注册面。
    #[test]
    fn identical_values_reach_the_same_face_from_add_and_edit() {
        let (request, patch) = fixture();
        let new_values = ModuleFaceValues::from_new(&request);
        let patch_values = ModuleFaceValues::from_patch(&patch);
        for field in CREATE_FIELD_ORDER
            .iter()
            .copied()
            .chain(EDIT_FIELD_ORDER.iter().copied())
        {
            assert_eq!(
                new_values.value(field),
                patch_values.value(field),
                "shared field `{field}`"
            );
        }

        let mut created = blank_manifest();
        apply_module_face_values(&mut created, &new_values, FaceWrite::Create)
            .expect("create applies");
        let mut edited = blank_manifest();
        apply_module_face_values(&mut edited, &patch_values, FaceWrite::Edit)
            .expect("edit applies");
        assert_eq!(created.values, edited.values);
    }

    /// A blank add value keeps the manifest default, while a blank edit value is
    /// written through: the two rules the public structs used to carry separately.
    /// 创建时的空值保留清单默认值，编辑时的空值则如实写入：公开结构体过去各自
    /// 携带的两条规则。
    #[test]
    fn a_blank_create_keeps_defaults_while_a_blank_edit_writes_them_through() {
        let (request, patch) = fixture();
        let blank_request = NewModuleFace {
            preset: "",
            ..request
        };
        let blank_patch = ModuleFacePatch {
            preset: "",
            ..patch
        };

        let mut created = blank_manifest();
        apply_module_face_values(
            &mut created,
            &ModuleFaceValues::from_new(&blank_request),
            FaceWrite::Create,
        )
        .expect("create applies");
        assert_eq!(
            created.values.get("preset").map(String::as_str),
            Some("NoPreset")
        );

        let mut edited = blank_manifest();
        apply_module_face_values(
            &mut edited,
            &ModuleFaceValues::from_patch(&blank_patch),
            FaceWrite::Edit,
        )
        .expect("edit applies");
        assert_eq!(edited.values.get("preset").map(String::as_str), Some(""));
    }

    /// The two field orders are the truth about what each path writes, and the
    /// edit order is the create order minus the contract rows — the labels are the
    /// only trait key an edit rewrites (audit `NAM-31`).
    /// 两张字段顺序表就是两条路径写入内容的真值，而编辑顺序等于创建顺序减去两个契约行——
    /// 标签是编辑会改写的唯一 trait 键（审计 `NAM-31`）。
    ///
    /// Red before the fix: the numbers in the three comments (`27`, `28`, `29`)
    /// disagreed with each other and with the tables, so nothing named this rule.
    /// 修前为红：三处注释里的数字（`27`、`28`、`29`）彼此不一致、也不等于表长，因此这条规则
    /// 无处可查。
    #[test]
    fn the_edit_order_is_the_create_order_without_the_contract_rows() {
        let in_create_but_not_edit: Vec<&str> = CREATE_FIELD_ORDER
            .iter()
            .copied()
            .filter(|field| !EDIT_FIELD_ORDER.contains(field))
            .collect();
        assert_eq!(
            in_create_but_not_edit,
            ["handle_contracts", "part_contracts"],
            "the only rows an edit does not rewrite are the contract rows"
        );
        for field in EDIT_FIELD_ORDER {
            assert!(
                CREATE_FIELD_ORDER.contains(field),
                "edit's `{field}` is not in the create order"
            );
        }
        // Every row either table names is a key `edit` accepts: a field wired into
        // a table but not into `edit` would be dropped silently on the next save.
        // 两张表点名的每一行都是 `edit` 接受的键：接进表却没接进 `edit` 的字段会在下次保存时
        // 被静默丢掉。
        let (request, _) = fixture();
        let values = ModuleFaceValues::from_new(&request);
        for field in CREATE_FIELD_ORDER.iter().chain(EDIT_FIELD_ORDER.iter()) {
            // `needs_registry` is the one table member a caller spells as a Rust
            // bool; the applier is what turns it into the `true`/`false` text
            // `edit` validates, so the pin supplies that text itself.
            // `needs_registry` 是表里唯一由调用方写成 Rust bool 的成员；把它变成 `edit` 校验的
            // `true`/`false` 文本是应用器的活，因此这条钉子自己给出该文本。
            let value = if *field == "needs_registry" {
                "false"
            } else {
                values.value(field)
            };
            let mut face = blank_manifest();
            face.edit(field, value).unwrap_or_else(|error| {
                panic!("`{field}` is in a field order but not editable: {error}")
            });
        }
    }
}
