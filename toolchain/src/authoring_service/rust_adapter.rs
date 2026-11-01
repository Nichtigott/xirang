//! The Rust side of the shared authoring service: it reuses the existing generator and executor
//! instead of growing a second Rust writer.
//! 共享创作服务的 Rust 一侧：复用现有生成器与执行器，而不是再长出一个 Rust 写入器。

use std::path::Path;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FieldUpdate, ObligationKind,
    PlanDiagnostic, PlannedEdit, PlannedFile, RegistrationField, RegistrationUpdate,
    ValidationObligation,
};
use xirang_kernel::identity::NodeId;
use xirang_kernel::interchange::LanguageId;
use xirang_kernel::tree::Registry;

use crate::run_method::authoring::operations::{
    NewModuleFace, NewModulePlan, RewriteModulePlan, RuleChange, plan_add_module_from_face,
    plan_new_face_from_spec, plan_rewrite_module_face,
};
use crate::run_method::{AuthoredFace, ModuleFacePatch};

use super::resolve::{self, ParentFace};
use crate::source_layout::SourceLayout;

/// One registration face's field values, owned so the borrowed request structs can point at them.
/// 一个注册面的字段取值，拥有所有权，使借用的请求结构体可以指向它们。
#[derive(Clone, Debug, Default)]
pub(crate) struct FaceFields {
    pub(crate) module: String,
    pub(crate) kind: String,
    pub(crate) preset: String,
    pub(crate) parts: String,
    pub(crate) name_zh: String,
    pub(crate) name_en: String,
    pub(crate) summary_zh: String,
    pub(crate) summary_en: String,
    pub(crate) exports: String,
    pub(crate) stable_name: String,
    pub(crate) needs_registry: bool,
    pub(crate) getting_from_other_registry: String,
    pub(crate) registration_rule: String,
    pub(crate) admission: String,
    pub(crate) handle_traits: String,
    pub(crate) handle_contracts: String,
    pub(crate) part_traits: String,
    pub(crate) part_contracts: String,
    pub(crate) requires: String,
    pub(crate) provides: String,
    pub(crate) runtime_checks: String,
    pub(crate) flow: String,
    pub(crate) flow_provider: String,
}

impl FaceFields {
    /// Build the create request from a sparse update: unnamed fields keep the generator's defaults.
    /// 从稀疏更新构建新建请求：未点名字段保留生成器默认值。
    pub(crate) fn for_create(update: &RegistrationUpdate, module: &str, kind: &str) -> Self {
        let mut fields = Self {
            module: module.to_owned(),
            kind: kind.to_owned(),
            registration_rule: String::new(),
            admission: String::new(),
            ..Self::default()
        };
        fields.apply(update);
        fields
    }

    /// Build the edit patch from a read-back face plus a sparse overlay: unnamed fields keep their
    /// current values, `Clear` erases one, `Set` replaces one.
    /// 从读回的面加稀疏覆盖构建编辑补丁：未点名字段保留当前值，`Clear` 清空，`Set` 替换。
    pub(crate) fn for_edit(current: &AuthoredFace, update: &RegistrationUpdate) -> Self {
        let mut fields = Self {
            module: current.module.clone(),
            kind: current.kind.clone(),
            preset: current.preset.clone(),
            parts: current.parts.clone(),
            name_zh: current.name_zh.clone(),
            name_en: current.name_en.clone(),
            summary_zh: current.summary_zh.clone(),
            summary_en: current.summary_en.clone(),
            exports: current.exports.clone(),
            stable_name: current.stable_name.clone(),
            needs_registry: current.needs_registry,
            getting_from_other_registry: current.getting_from_other_registry.clone(),
            registration_rule: current.registration_rule.clone(),
            admission: current.admission.clone(),
            handle_traits: current.handle_traits.clone(),
            handle_contracts: current.handle_contracts.clone(),
            part_traits: current.part_traits.clone(),
            part_contracts: current.part_contracts.clone(),
            requires: current.requires.clone(),
            provides: current.provides.clone(),
            runtime_checks: current.runtime_checks.clone(),
            flow: current.flow.clone(),
            flow_provider: current.flow_provider.clone(),
        };
        fields.apply(update);
        fields
    }

    fn apply(&mut self, update: &RegistrationUpdate) {
        for (field, value) in update.named() {
            let target = match field {
                RegistrationField::Module => &mut self.module,
                RegistrationField::Kind => &mut self.kind,
                RegistrationField::Preset => &mut self.preset,
                RegistrationField::Parts => &mut self.parts,
                RegistrationField::NameZh => &mut self.name_zh,
                RegistrationField::NameEn => &mut self.name_en,
                RegistrationField::SummaryZh => &mut self.summary_zh,
                RegistrationField::SummaryEn => &mut self.summary_en,
                RegistrationField::Exports => &mut self.exports,
                RegistrationField::StableName => &mut self.stable_name,
                RegistrationField::GettingFromOtherRegistry => {
                    &mut self.getting_from_other_registry
                }
                RegistrationField::RegistrationRule => &mut self.registration_rule,
                RegistrationField::Admission => &mut self.admission,
                RegistrationField::HandleTraits => &mut self.handle_traits,
                RegistrationField::HandleContracts => &mut self.handle_contracts,
                RegistrationField::PartTraits => &mut self.part_traits,
                RegistrationField::PartContracts => &mut self.part_contracts,
                RegistrationField::Requires => &mut self.requires,
                RegistrationField::Provides => &mut self.provides,
                RegistrationField::RuntimeChecks => &mut self.runtime_checks,
                RegistrationField::Flow => &mut self.flow,
                RegistrationField::FlowProvider => &mut self.flow_provider,
                RegistrationField::NeedsRegistry => {
                    self.needs_registry = value == "true";
                    continue;
                }
            };
            *target = value.to_owned();
        }
    }

    /// The create request struct, borrowing this field set.
    /// 新建请求结构体，借用这组字段。
    pub(crate) fn as_new(&self, parent: NodeId) -> NewModuleFace<'_> {
        NewModuleFace {
            module: &self.module,
            kind: &self.kind,
            preset: &self.preset,
            parts: &self.parts,
            name_zh: &self.name_zh,
            name_en: &self.name_en,
            summary_zh: &self.summary_zh,
            summary_en: &self.summary_en,
            exports: &self.exports,
            stable_name: &self.stable_name,
            parent,
            needs_registry: self.needs_registry,
            getting_from_other_registry: &self.getting_from_other_registry,
            registration_rule: &self.registration_rule,
            admission: &self.admission,
            handle_traits: &self.handle_traits,
            handle_contracts: &self.handle_contracts,
            part_traits: &self.part_traits,
            part_contracts: &self.part_contracts,
            requires: &self.requires,
            provides: &self.provides,
            runtime_checks: &self.runtime_checks,
            flow: &self.flow,
            flow_provider: &self.flow_provider,
        }
    }

    /// The edit request struct, borrowing this field set.
    /// 编辑请求结构体，借用这组字段。
    pub(crate) fn as_patch(&self) -> ModuleFacePatch<'_> {
        ModuleFacePatch {
            module: &self.module,
            kind: &self.kind,
            preset: &self.preset,
            parts: &self.parts,
            name_zh: &self.name_zh,
            name_en: &self.name_en,
            summary_zh: &self.summary_zh,
            summary_en: &self.summary_en,
            exports: &self.exports,
            stable_name: &self.stable_name,
            needs_registry: self.needs_registry,
            getting_from_other_registry: &self.getting_from_other_registry,
            registration_rule: &self.registration_rule,
            admission: &self.admission,
            handle_traits: &self.handle_traits,
            handle_contracts: &self.handle_contracts,
            part_traits: &self.part_traits,
            part_contracts: &self.part_contracts,
            requires: &self.requires,
            provides: &self.provides,
            runtime_checks: &self.runtime_checks,
            flow: &self.flow,
            flow_provider: &self.flow_provider,
        }
    }
}

/// Read a Rust face back into the shared update vocabulary.
/// 把 Rust 面读回共享的更新词汇。
pub(crate) fn read_update(current: &AuthoredFace) -> RegistrationUpdate {
    let mut update = RegistrationUpdate::new();
    fn text(update: &mut RegistrationUpdate, field: RegistrationField, value: &str) {
        if value.is_empty() {
            update.set_normalized(field, FieldUpdate::Clear);
        } else {
            update.set_normalized(field, FieldUpdate::Set(value.to_owned()));
        }
    }
    text(&mut update, RegistrationField::Module, &current.module);
    text(&mut update, RegistrationField::Kind, &current.kind);
    text(&mut update, RegistrationField::Preset, &current.preset);
    text(&mut update, RegistrationField::Parts, &current.parts);
    text(&mut update, RegistrationField::NameZh, &current.name_zh);
    text(&mut update, RegistrationField::NameEn, &current.name_en);
    text(
        &mut update,
        RegistrationField::SummaryZh,
        &current.summary_zh,
    );
    text(
        &mut update,
        RegistrationField::SummaryEn,
        &current.summary_en,
    );
    text(&mut update, RegistrationField::Exports, &current.exports);
    text(
        &mut update,
        RegistrationField::StableName,
        &current.stable_name,
    );
    update.set_normalized(
        RegistrationField::NeedsRegistry,
        FieldUpdate::Set(current.needs_registry.to_string()),
    );
    text(
        &mut update,
        RegistrationField::GettingFromOtherRegistry,
        &current.getting_from_other_registry,
    );
    text(
        &mut update,
        RegistrationField::RegistrationRule,
        &current.registration_rule,
    );
    text(
        &mut update,
        RegistrationField::Admission,
        &current.admission,
    );
    text(
        &mut update,
        RegistrationField::HandleTraits,
        &current.handle_traits,
    );
    text(
        &mut update,
        RegistrationField::HandleContracts,
        &current.handle_contracts,
    );
    text(
        &mut update,
        RegistrationField::PartTraits,
        &current.part_traits,
    );
    text(
        &mut update,
        RegistrationField::PartContracts,
        &current.part_contracts,
    );
    text(&mut update, RegistrationField::Requires, &current.requires);
    text(&mut update, RegistrationField::Provides, &current.provides);
    text(
        &mut update,
        RegistrationField::RuntimeChecks,
        &current.runtime_checks,
    );
    text(&mut update, RegistrationField::Flow, &current.flow);
    text(
        &mut update,
        RegistrationField::FlowProvider,
        &current.flow_provider,
    );
    update
}

/// Plan a Rust create through the existing generator.
/// 经现有生成器计划一次 Rust 新建。
pub(crate) fn plan_create(
    root: &Path,
    layout: &SourceLayout,
    registry: &Registry,
    parent: &ParentFace,
    fields: &FaceFields,
    explicit: Option<(&Path, xirang_multilang::source_ref::SourceBase)>,
) -> Result<FaceAuthoringPlan, String> {
    let parent_id = parent.id().unwrap_or_else(|| registry.id());
    let plan = match parent {
        // An explicit destination wins over the derived one, for every parent kind: the request
        // named where the file goes, and the generator's job is to write it there.
        // 显式目的地在任何父级下都胜过推导出的那一个：请求点名了文件去哪，生成器的工作就是写到那里。
        _ if explicit.is_some() => {
            let parent_source = match parent {
                ParentFace::Root => "<root>",
                ParentFace::Rust { source, .. } | ParentFace::Native { source, .. } => source,
            };
            let parent_kind = match parent {
                ParentFace::Root => "root",
                ParentFace::Rust { kind, .. } => kind,
                ParentFace::Native { name, .. } => name,
            };
            let parent_reference = match parent {
                ParentFace::Native { .. } => Some(parent.rust_reference()?),
                ParentFace::Root | ParentFace::Rust { .. } => None,
            };
            let parent_module = match &parent {
                ParentFace::Rust { .. } => Some(parent.module()?),
                _ => None,
            };
            let (directory, source_base) = explicit.expect("the branch tested it");
            plan_new_face_from_spec(
                registry,
                &fields.module,
                parent_id,
                parent_source,
                parent_kind,
                parent_reference.as_deref(),
                parent_module.as_deref(),
                directory.to_path_buf(),
                source_base,
                &fields.as_new(parent_id),
            )?
        }
        ParentFace::Native { source, name, .. } => {
            let destination = resolve::destination(
                layout,
                parent,
                &fields.module,
                xirang_multilang::registration::SourcePlacement::UnderScanRoot,
                "rs",
            )?;
            let relative_directory = destination
                .parent()
                .ok_or_else(|| "a destination always has a parent".to_owned())?;
            let identity_relative = relative_directory
                .strip_prefix(
                    layout
                        .scan_root
                        .strip_prefix(&layout.package_root)
                        .unwrap_or(Path::new("")),
                )
                .unwrap_or(relative_directory)
                .to_path_buf();
            plan_new_face_from_spec(
                registry,
                &fields.module,
                parent_id,
                source,
                name,
                Some(&parent.rust_reference()?),
                None,
                identity_relative,
                xirang_multilang::source_ref::SourceBase::Identity,
                &fields.as_new(parent_id),
            )?
        }
        ParentFace::Root | ParentFace::Rust { .. } => {
            let spec = fields.as_new(parent_id);
            let parent_module = match &parent {
                ParentFace::Rust { .. } => Some(parent.module()?),
                _ => None,
            };
            let parent_base = match &parent {
                ParentFace::Rust { base, .. } => *base,
                _ => xirang_multilang::source_ref::SourceBase::Identity,
            };
            plan_add_module_from_face(registry, &spec, parent_module.as_deref(), parent_base)?
        }
    };
    Ok(create_plan(root, &plan))
}

/// Plan a Rust field rewrite through the existing executor.
/// 经现有执行器计划一次 Rust 字段重写。
pub(crate) fn plan_edit(
    root: &Path,
    layout: &SourceLayout,
    registry: &Registry,
    id: NodeId,
    current: &AuthoredFace,
    update: &RegistrationUpdate,
) -> Result<FaceAuthoringPlan, String> {
    let fields = FaceFields::for_edit(current, update);
    let patch = fields.as_patch();
    let plan = plan_rewrite_module_face(registry, id, &patch)?;
    Ok(rewrite_plan(root, layout, plan))
}

/// Convert the generator's create plan into the shared plan vocabulary.
/// 把生成器的新建计划转换成共享计划词汇。
fn create_plan(root: &Path, plan: &NewModulePlan) -> FaceAuthoringPlan {
    let mut files = vec![PlannedFile {
        path: resolve::package_relative(root, &plan.source)
            .unwrap_or_else(|_| plan.source.display().to_string()),
        original: None,
        updated: plan.source_text.clone(),
        edits: Vec::new(),
        remove: false,
    }];
    if let Some((rule, text)) = &plan.rule {
        files.push(PlannedFile {
            path: resolve::package_relative(root, rule)
                .unwrap_or_else(|_| rule.display().to_string()),
            original: None,
            updated: text.clone(),
            edits: Vec::new(),
            remove: false,
        });
    }
    FaceAuthoringPlan {
        language: rust(),
        operation: AuthoringOperation::Create,
        files,
        diagnostics: Vec::new(),
        obligations: obligations("the new face"),
    }
}

/// Convert the executor's rewrite plan into the shared plan vocabulary.
/// 把执行器的重写计划转换成共享计划词汇。
fn rewrite_plan(root: &Path, layout: &SourceLayout, plan: RewriteModulePlan) -> FaceAuthoringPlan {
    let _ = layout;
    let edits = precise_edits(&plan.old_source, &plan.rendered);
    let mut files = vec![PlannedFile {
        path: resolve::package_relative(root, &plan.source)
            .unwrap_or_else(|_| plan.source.display().to_string()),
        original: Some(plan.old_source.clone()),
        updated: plan.rendered.clone(),
        edits,
        remove: false,
    }];
    let mut diagnostics = Vec::new();
    match plan.rule {
        RuleChange::Keep => {}
        RuleChange::Write(path, text) => files.push(PlannedFile {
            path: resolve::package_relative(root, &path)
                .unwrap_or_else(|_| path.display().to_string()),
            original: std::fs::read_to_string(&path).ok(),
            updated: text,
            edits: Vec::new(),
            remove: false,
        }),
        RuleChange::Remove(path) => {
            if let Ok(text) = std::fs::read_to_string(&path) {
                files.push(PlannedFile {
                    path: resolve::package_relative(root, &path)
                        .unwrap_or_else(|_| path.display().to_string()),
                    original: Some(text),
                    updated: String::new(),
                    edits: Vec::new(),
                    remove: true,
                });
                diagnostics.push(PlanDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: format!(
                        "{} is removed because the face no longer owns a registry rule",
                        path.display()
                    ),
                });
            }
        }
    }
    FaceAuthoringPlan {
        language: rust(),
        operation: AuthoringOperation::Modify,
        files,
        diagnostics,
        obligations: obligations("the edited face"),
    }
}

/// One precise edit spanning the changed region, or none when the text is unchanged.
/// 一处覆盖变化区间的精确编辑；文本未变时为空。
pub(crate) fn precise_edits(old: &str, new: &str) -> Vec<PlannedEdit> {
    if old == new {
        return Vec::new();
    }
    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    let mut prefix = 0;
    while prefix < old_bytes.len()
        && prefix < new_bytes.len()
        && old_bytes[prefix] == new_bytes[prefix]
        && old.is_char_boundary(prefix + 1)
        && new.is_char_boundary(prefix + 1)
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_bytes.len() - prefix
        && suffix < new_bytes.len() - prefix
        && old_bytes[old_bytes.len() - 1 - suffix] == new_bytes[new_bytes.len() - 1 - suffix]
        && old.is_char_boundary(old_bytes.len() - 1 - suffix)
        && new.is_char_boundary(new_bytes.len() - 1 - suffix)
    {
        suffix += 1;
    }
    vec![PlannedEdit {
        start: prefix,
        end: old_bytes.len() - suffix,
        replacement: new[prefix..new_bytes.len() - suffix].to_owned(),
    }]
}

fn obligations(face: &str) -> Vec<ValidationObligation> {
    vec![
        ValidationObligation {
            kind: ObligationKind::RoundTrip,
            detail: format!("read {face} back and compare the normalized registration fields"),
        },
        ValidationObligation {
            kind: ObligationKind::Build,
            detail: "build the package so the declaration is compiled and registered".to_owned(),
        },
        ValidationObligation {
            kind: ObligationKind::Registration,
            detail: format!("confirm {face} appears in the built tree under its parent"),
        },
    ]
}

fn rust() -> LanguageId {
    LanguageId::parse_language("rust").expect("`rust` is a language id")
}

#[cfg(test)]
mod rust_adapter_tests {
    use super::*;

    /// A create request names only what it wants; the generator defaults fill the rest, and the
    /// read-back comparison only has to match the named fields.
    /// 新建请求只点名它想要的；其余由生成器默认值补齐，读回比较只需匹配点名字段。
    #[test]
    fn a_sparse_create_request_keeps_defaults_for_unnamed_fields() {
        let mut update = RegistrationUpdate::new();
        update
            .set_text(RegistrationField::Exports, FieldUpdate::Set("area"))
            .unwrap();
        let fields = FaceFields::for_create(&update, "area", "Area");
        assert_eq!(fields.module, "area");
        assert_eq!(fields.kind, "Area");
        assert_eq!(fields.exports, "area");
        assert_eq!(fields.summary_en, "");
        assert!(!fields.needs_registry);
    }

    /// An edit overlay keeps every field it does not name and erases one it explicitly clears.
    /// 编辑覆盖保留它没有点名的每个字段，并清空它显式清空的那一个。
    #[test]
    fn an_edit_overlay_keeps_unnamed_fields_and_clears_named_empty_ones() {
        let current = AuthoredFace {
            module: "area".to_owned(),
            kind: "Area".to_owned(),
            preset: "AreaPreset".to_owned(),
            parts: "AreaParts".to_owned(),
            name_zh: "区域".to_owned(),
            name_en: "Area".to_owned(),
            summary_zh: "summary".to_owned(),
            summary_en: "summary".to_owned(),
            exports: "area".to_owned(),
            stable_name: String::new(),
            needs_registry: true,
            getting_from_other_registry: String::new(),
            registration_rule: "ANY".to_owned(),
            admission: "ANY".to_owned(),
            handle_traits: String::new(),
            handle_contracts: String::new(),
            part_traits: String::new(),
            part_contracts: String::new(),
            requires: String::new(),
            provides: String::new(),
            runtime_checks: String::new(),
            flow: String::new(),
            flow_provider: String::new(),
        };
        let mut update = RegistrationUpdate::new();
        update
            .set_text(RegistrationField::SummaryEn, FieldUpdate::Clear)
            .unwrap();
        update
            .set_text(RegistrationField::Exports, FieldUpdate::Set("area,volume"))
            .unwrap();
        let fields = FaceFields::for_edit(&current, &update);
        assert_eq!(fields.exports, "area,volume");
        assert_eq!(fields.summary_en, "");
        assert_eq!(fields.name_zh, "区域", "an unnamed field survived");
        assert!(fields.needs_registry, "an unnamed flag survived");
    }

    /// The precise edit's range is the changed region, and splicing it reproduces the new text.
    /// 精确编辑的区间就是变化区间，拼接它能复现新文本。
    #[test]
    fn a_precise_edit_reproduces_the_new_text() {
        let old = "before\n  changed old\n after\n";
        let new = "before\n  changed new\n after\n";
        let edits = precise_edits(old, new);
        assert_eq!(edits.len(), 1);
        let edit = &edits[0];
        assert_eq!(
            format!(
                "{}{}{}",
                &old[..edit.start],
                edit.replacement,
                &old[edit.end..]
            ),
            new
        );
        assert!(precise_edits(old, old).is_empty());
    }
}
