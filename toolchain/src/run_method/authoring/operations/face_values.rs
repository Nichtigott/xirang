//! The registration-face field view shared by add and edit requests.
//! add 与 edit 请求共用的注册面字段视图。

use super::*;

/// The registration-face fields [`NewModuleFace`] and [`ModuleFacePatch`] share,
/// in one view either public struct can build. `parent` is the only field outside
/// it: the applier never reads it, so it stays an add-only argument. The field
/// list itself is the count — no number is written here, because the ones that
/// used to be here and in its two siblings disagreed with the struct (audit
/// `SUR-C6`).
/// [`NewModuleFace`] 与 [`ModuleFacePatch`] 共用的注册面字段；任一公开
/// 结构体都能构建出这同一份视图。`parent` 是唯一不在其中的字段：应用器从不读取
/// 它，因此它仍是 add 独有的参数。数量就是字段清单本身——这里不写数字，因为过去这里
/// 与它的两个兄弟文件里的数字都不等于本结构体的字段数（审计 `SUR-C6`）。
///
/// `registry_rule_path` is deliberately absent: it is derived from the face's own
/// location (`FaceManifest` writes it on render), so accepting it from a caller
/// only created a value the applier had to discard. The applier used to write
/// then drop it; B3a removed the field instead.
/// `registry_rule_path` 刻意不在其中：它由注册面自身位置派生（渲染时由
/// `FaceManifest` 写入），接受调用方传入只会造出一个应用器必须丢弃的值。应用器过去
/// 先写后丢；B3a 改为直接删除该字段。
pub(super) struct ModuleFaceValues<'a> {
    pub(super) module: &'a str,
    pub(super) kind: &'a str,
    pub(super) preset: &'a str,
    pub(super) parts: &'a str,
    pub(super) name_zh: &'a str,
    pub(super) name_en: &'a str,
    pub(super) summary_zh: &'a str,
    pub(super) summary_en: &'a str,
    pub(super) exports: &'a str,
    pub(super) stable_name: &'a str,
    pub(super) needs_registry: bool,
    pub(super) getting_from_other_registry: &'a str,
    pub(super) registration_rule: &'a str,
    pub(super) admission: &'a str,
    pub(super) handle_traits: &'a str,
    pub(super) handle_contracts: &'a str,
    pub(super) part_traits: &'a str,
    pub(super) part_contracts: &'a str,
    pub(super) requires: &'a str,
    pub(super) provides: &'a str,
    pub(super) runtime_checks: &'a str,
    pub(super) flow: &'a str,
    pub(super) flow_provider: &'a str,
}

impl<'a> ModuleFaceValues<'a> {
    /// View an add request's fields.
    /// 查看一次 add 请求的字段。
    pub(super) fn from_new(face: &NewModuleFace<'a>) -> Self {
        Self {
            module: face.module,
            kind: face.kind,
            preset: face.preset,
            parts: face.parts,
            name_zh: face.name_zh,
            name_en: face.name_en,
            summary_zh: face.summary_zh,
            summary_en: face.summary_en,
            exports: face.exports,
            stable_name: face.stable_name,
            needs_registry: face.needs_registry,
            getting_from_other_registry: face.getting_from_other_registry,
            registration_rule: face.registration_rule,
            admission: face.admission,
            handle_traits: face.handle_traits,
            handle_contracts: face.handle_contracts,
            part_traits: face.part_traits,
            part_contracts: face.part_contracts,
            requires: face.requires,
            provides: face.provides,
            runtime_checks: face.runtime_checks,
            flow: face.flow,
            flow_provider: face.flow_provider,
        }
    }

    /// View an edit patch's fields.
    /// 查看一次 edit 补丁的字段。
    pub(super) fn from_patch(patch: &ModuleFacePatch<'a>) -> Self {
        Self {
            module: patch.module,
            kind: patch.kind,
            preset: patch.preset,
            parts: patch.parts,
            name_zh: patch.name_zh,
            name_en: patch.name_en,
            summary_zh: patch.summary_zh,
            summary_en: patch.summary_en,
            exports: patch.exports,
            stable_name: patch.stable_name,
            needs_registry: patch.needs_registry,
            getting_from_other_registry: patch.getting_from_other_registry,
            registration_rule: patch.registration_rule,
            admission: patch.admission,
            handle_traits: patch.handle_traits,
            handle_contracts: patch.handle_contracts,
            part_traits: patch.part_traits,
            part_contracts: patch.part_contracts,
            requires: patch.requires,
            provides: patch.provides,
            runtime_checks: patch.runtime_checks,
            flow: patch.flow,
            flow_provider: patch.flow_provider,
        }
    }

    /// One shared field, keyed by the manifest field name the applier writes.
    /// 按应用器写入的清单字段名读取某个共用字段。
    pub(super) fn value(&self, field: &str) -> &'a str {
        match field {
            "kind" => self.kind,
            "preset" => self.preset,
            "parts" => self.parts,
            "name_zh" => self.name_zh,
            "name_en" => self.name_en,
            "summary_zh" => self.summary_zh,
            "summary_en" => self.summary_en,
            "exports" => self.exports,
            "stable_name" => self.stable_name,
            "getting_from_other_registry" => self.getting_from_other_registry,
            "registration_rule" => self.registration_rule,
            "admission" => self.admission,
            "handle_traits" => self.handle_traits,
            "handle_contracts" => self.handle_contracts,
            "part_traits" => self.part_traits,
            "part_contracts" => self.part_contracts,
            "requires" => self.requires,
            "provides" => self.provides,
            "runtime_checks" => self.runtime_checks,
            "flow" => self.flow,
            "flow_provider" => self.flow_provider,
            _ => "",
        }
    }
}
