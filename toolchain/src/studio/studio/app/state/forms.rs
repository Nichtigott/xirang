//! New-project, add/edit, and plugin form state.
//! 新建项目、添加/编辑与插件表单状态。

use std::collections::{BTreeMap, BTreeSet};

use super::{new_project_field, plugin_field};
use crate::run_method::authoring::face_field;
use crate::run_method::face_field_presentation;
use crate::run_method::{FACE_FIELD_COUNT, NodeId};

/// Fields used by the New Project wizard.
/// New Project 向导使用的字段。
#[derive(Clone, Debug)]
pub struct NewProjectState {
    /// Raw wizard input, one value per [`new_project_field`] row.
    /// 向导原始输入，每个 [`new_project_field`] 行一个值。
    pub values: [String; new_project_field::COUNT],
    /// Index of the focused row, matching `values`.
    /// 当前聚焦行在 `values` 中的下标。
    pub field: usize,
    /// Whether the focused row is accepting typed input.
    /// 当前行是否正在接受键入。
    pub editing: bool,
}

impl NewProjectState {
    pub(crate) fn new() -> Self {
        let mut values: [String; new_project_field::COUNT] = std::array::from_fn(|_| String::new());
        values[new_project_field::DIRECTORY] = "./xirang-app".to_owned();
        values[new_project_field::PACKAGE] = "xirang-app".to_owned();
        values[new_project_field::KIND] = "binary".to_owned();
        Self {
            values,
            field: new_project_field::DIRECTORY,
            editing: false,
        }
    }
}

/// Add/edit form state for one registration face.
/// 单个注册面的添加/编辑表单状态。
#[derive(Clone, Debug)]
pub struct AddState {
    /// One text value per canonical face field index.
    /// 每个规范注册面字段下标对应一个文本值。
    pub values: [String; FACE_FIELD_COUNT],
    /// Index of the focused field, as a canonical field index.
    /// 当前聚焦字段的规范字段下标。
    pub field: usize,
    /// Whether the focused field is accepting typed input.
    /// 当前字段是否正在接受键入。
    pub editing: bool,
    /// Fields made mandatory by the selected parent Registry contract.
    /// 由所选父 Registry 合同提升为必填的字段及其具体要求。
    pub parent_requirements: BTreeMap<usize, String>,
    /// Derived fields shown for context but not accepted as free-form input.
    /// 仅用于说明、不能自由输入的派生字段。
    pub locked_fields: BTreeSet<usize>,
    /// The language this face will be authored in.
    /// 本面将用哪种语言创作。
    pub language: String,
    /// The languages this build can author, in the order `l` cycles them.
    /// 本构建可以创作的语言，按 `l` 循环的顺序。
    pub languages: Vec<String>,
    /// The selected language's index in [`Self::languages`].
    /// 选中语言在 [`Self::languages`] 中的下标。
    pub language_index: usize,
    /// Whether the form offers a language choice at all (add does, edit does not).
    /// 表单是否提供语言选择（新增提供，编辑不提供）。
    pub allow_language: bool,
    /// The form rows the selected language carries.
    /// 选中语言携带的表单行。
    pub carried: BTreeSet<usize>,
}

/// Stable, task-oriented order used by both Add and Edit.
/// The author's rows come first; the read-only machine values close the list, so
/// the form can print them as one strip and `move_face_field` still reaches them.
/// Add 与 Edit 共用的稳定任务顺序。作者的输入行在前，只读的机器取值收尾，因此表单可以把
/// 它们打印成一条，而 `move_face_field` 仍然能到达它们。
pub(crate) const FACE_FORM_ORDER: [usize; FACE_FIELD_COUNT] = [
    face_field::PARENT,
    face_field::MODULE,
    face_field::KIND,
    face_field::NAME_ZH,
    face_field::NAME_EN,
    face_field::SUMMARY_ZH,
    face_field::SUMMARY_EN,
    face_field::STABLE_NAME,
    face_field::NEEDS_REGISTRY,
    face_field::REGISTRY_RULE,
    face_field::ADMISSION,
    face_field::GETTING_FROM_OTHER_REGISTRY,
    face_field::PRESET,
    face_field::PARTS,
    face_field::HANDLE_CONTRACTS,
    face_field::PART_CONTRACTS,
    face_field::EXPORTS,
    face_field::REQUIRES,
    face_field::PROVIDES,
    face_field::FLOW,
    face_field::FLOW_PROVIDER,
    face_field::RUNTIME_CHECKS,
    // Machine values: derived from the module, the face's location, and the
    // contract paths above, so they are shown for review and never typed.
    // 机器取值：由模块、注册面位置与上面的契约路径推导，因此只供查看，从不键入。
    face_field::TREE_SLOT,
    face_field::REGISTRY_RULE_PATH,
    face_field::HANDLE_TRAITS,
    face_field::PART_TRAITS,
];

/// The form rows one language's registration record can carry.
/// 一门语言的注册记录能携带的表单行。
///
/// The mapping is data, not a guess: a language whose capability table names `kind`, `exports` and
/// `needs_registry` carries exactly those rows, and the form prints the rest muted instead of
/// letting an author type a field the adapter will refuse.
/// 这份映射是数据而不是猜测：能力表点名 `kind`、`exports` 与 `needs_registry` 的语言就只携带这几行，
/// 表单把其余行画成暗色，而不是让作者键入一个适配器会拒绝的字段。
fn carried_fields(language: &str) -> BTreeSet<usize> {
    use xirang_kernel::authoring::RegistrationField;
    let Some(id) = xirang_kernel::interchange::LanguageId::parse_language(language) else {
        return BTreeSet::new();
    };
    let Some(capabilities) = crate::authoring_service::language_capabilities(&id) else {
        return BTreeSet::new();
    };
    if language == "rust" {
        return (0..FACE_FIELD_COUNT).collect();
    }
    let mut rows = BTreeSet::new();
    for field in capabilities.fields {
        let row = match field {
            RegistrationField::Module => Some(face_field::MODULE),
            RegistrationField::Kind => Some(face_field::KIND),
            RegistrationField::Exports => Some(face_field::EXPORTS),
            RegistrationField::NeedsRegistry => Some(face_field::NEEDS_REGISTRY),
            _ => None,
        };
        if let Some(row) = row {
            rows.insert(row);
        }
    }
    rows
}

/// The form's rows, in display order.
/// 表单各行的显示顺序。
pub(crate) fn face_field_indices() -> &'static [usize] {
    &FACE_FORM_ORDER
}

pub(crate) fn move_face_field(add: &mut AddState, step: isize) {
    let position = FACE_FORM_ORDER
        .iter()
        .position(|field| *field == add.field)
        .unwrap_or_default();
    let next = position
        .saturating_add_signed(step)
        .min(FACE_FORM_ORDER.len().saturating_sub(1));
    add.field = FACE_FORM_ORDER[next];
}

/// Plugin selection form. The UI writes one crate anchor, never an object list.
/// 插件选择表单。界面只写入一个 crate 锚点，不维护对象清单。
#[derive(Clone, Debug)]
pub struct PluginState {
    /// Raw plugin input, one value per [`plugin_field`] row.
    /// 插件原始输入，每个 [`plugin_field`] 行一个值。
    pub values: [String; plugin_field::COUNT],
    /// Index of the focused row, matching `values`.
    /// 当前聚焦行在 `values` 中的下标。
    pub field: usize,
    /// Whether the focused row is accepting typed input.
    /// 当前行是否正在接受键入。
    pub editing: bool,
    /// Whether a first `s` armed this form for writing.
    /// 第一次按 `s` 是否已让本表单进入待写状态。
    ///
    /// Writing appends a `use` line to the host's plugin entry and a line to the
    /// lock, so a mistyped crate name could break the host build. The first press
    /// arms and any other key clears it, which is what makes the write two
    /// deliberate presses on one form.
    /// 写入会向宿主的插件入口追加一行 `use`、并向锁追加一行，因此一个打错的 crate 名可能
    /// 破坏宿主构建。第一次按只进入待写状态，任何其他键都会解除，这就是"同一次表单上两次
    /// 有意的按键"的来源。
    pub pending_submit: bool,
}

impl PluginState {
    pub(crate) fn new() -> Self {
        let mut values: [String; plugin_field::COUNT] = std::array::from_fn(|_| String::new());
        values[plugin_field::SOURCE] = "official".to_owned();
        values[plugin_field::FRAMEWORK] = "xirang.default".to_owned();
        values[plugin_field::MODE] = "extension".to_owned();
        Self {
            values,
            field: plugin_field::SOURCE,
            editing: false,
            pending_submit: false,
        }
    }
}

impl AddState {
    pub(crate) fn new(parent: NodeId) -> Self {
        // Only the slots whose initial value is not the empty string are named;
        // every other slot starts empty, and `face_field_default` supplies what
        // the guide shows for it.
        // 只给初值不是空串的槽位命名；其余槽位初始为空，其指南取值由
        // `face_field_default` 提供。
        let mut values: [String; FACE_FIELD_COUNT] = std::array::from_fn(|_| String::new());
        values[face_field::PARENT] = parent.to_string();
        values[face_field::NEEDS_REGISTRY] = "false".to_owned();
        values[face_field::REGISTRY_RULE] = "ANY".to_owned();
        values[face_field::ADMISSION] = "ANY".to_owned();
        values[face_field::PARTS] = "NoParts".to_owned();
        values[face_field::PRESET] = "NoPreset".to_owned();
        let languages: Vec<String> = crate::authoring_service::available_languages()
            .iter()
            .map(|language| language.as_str().to_owned())
            .collect();
        let language = languages
            .first()
            .cloned()
            .unwrap_or_else(|| "rust".to_owned());
        let carried = carried_fields(&language);
        Self {
            values,
            field: face_field::PARENT,
            editing: false,
            language,
            languages,
            language_index: 0,
            allow_language: true,
            carried,
            parent_requirements: BTreeMap::new(),
            // Derived from the module, from the face's own location, and from the
            // contract paths: shown so the author can check them, not authored.
            // 由模块、注册面自身位置与契约路径推导：供作者核对，不由作者书写。
            locked_fields: BTreeSet::from([
                face_field::TREE_SLOT,
                face_field::REGISTRY_RULE_PATH,
                face_field::HANDLE_TRAITS,
                face_field::PART_TRAITS,
            ]),
        }
    }

    pub(crate) fn is_editable(&self, field: usize) -> bool {
        !self.locked_fields.contains(&field)
    }

    /// Whether the selected language carries one form row.
    /// 选中语言是否携带某个表单行。
    pub(crate) fn is_carried(&self, field: usize) -> bool {
        self.carried.contains(&field)
    }

    /// The language's label for the footer: its name and the rows it carries.
    /// 给页脚用的语言标签：名字与它携带的行。
    pub(crate) fn language_summary(&self) -> String {
        let labels = FACE_FORM_ORDER
            .iter()
            .filter(|field| self.carried.contains(field))
            .map(|field| face_field_presentation(*field).label)
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}: {labels}", self.language)
    }

    /// Cycle to the next language this build can author.
    /// 循环到本构建可以创作的下一个语言。
    pub(crate) fn cycle_language(&mut self) {
        if !self.allow_language || self.languages.is_empty() {
            return;
        }
        self.language_index = (self.language_index + 1) % self.languages.len();
        self.language = self.languages[self.language_index].clone();
        self.carried = carried_fields(&self.language);
    }

    pub(crate) fn apply_parent_rule(&mut self, rule: &crate::run_method::OwnedRegistrationRule) {
        self.parent_requirements.clear();
        if let Some(preset) = &rule.required_preset {
            self.values[face_field::PRESET] = preset.clone();
            self.parent_requirements
                .insert(face_field::PRESET, format!("preset `{preset}`"));
        }
        if !rule.required_parts.is_empty() {
            self.parent_requirements.insert(
                face_field::PARTS,
                format!("parts providing {}", rule.required_parts.join(", ")),
            );
        }
        if !rule.required_exports.is_empty() {
            self.values[face_field::EXPORTS] = rule.required_exports.join(",");
            self.parent_requirements.insert(
                face_field::EXPORTS,
                format!("exports {}", rule.required_exports.join(", ")),
            );
        }
        if !rule.required_handle_traits.is_empty() {
            self.parent_requirements.insert(
                face_field::HANDLE_CONTRACTS,
                format!(
                    "handle implements {}",
                    rule.required_handle_traits.join(", ")
                ),
            );
        }
        if !rule.required_part_traits.is_empty() {
            self.parent_requirements.insert(
                face_field::PART_CONTRACTS,
                format!("parts implement {}", rule.required_part_traits.join(", ")),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The form's last rows are exactly the read-only ones, which is what lets
    /// the renderer draw one machine-value strip instead of marking rows apart.
    /// 表单最后几行正好是只读行，这正是渲染器能画成一条机器取值、而不必逐行标记的原因。
    #[test]
    fn the_read_only_rows_close_the_form_order() {
        let state = AddState::new(NodeId::from_path("forms", "test"));
        let machine = FACE_FORM_ORDER
            .iter()
            .position(|slot| !state.is_editable(*slot))
            .expect("the form keeps at least one read-only row");
        assert!(
            FACE_FORM_ORDER[machine..]
                .iter()
                .all(|slot| !state.is_editable(*slot)),
            "{FACE_FORM_ORDER:?}"
        );
        assert_eq!(machine, FACE_FIELD_COUNT - state.locked_fields.len());
    }

    /// Every slot is either an author row or a read-only machine value, and the
    /// two sets together cover the whole layout: no slot can be dropped from the
    /// form without failing here.
    /// 每个槽位要么是作者输入行，要么是只读机器取值，两者合起来覆盖整个布局：任何槽位若被
    /// 表单漏掉，都会在这里失败。
    #[test]
    fn every_slot_appears_once_in_the_form_order() {
        let mut seen = FACE_FORM_ORDER.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen, (0..FACE_FIELD_COUNT).collect::<Vec<_>>());
    }
}
