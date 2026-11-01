//! Registration declarations and compile-time construction contracts.
//! 注册声明与编译期构造合同。
//!
//! The vocabulary is split by concept, and the file tree mirrors that split.
//! Plugin records are defined in the plugin module and re-exported here, so a
//! declaration still names them (`declaration::PluginManifest`) without this
//! module owning a second copy.
//! 词表按概念拆分，文件树与之一致。插件记录定义在 plugin 模块并在此再导出，因此声明
//! 仍以原名引用它们（`declaration::PluginManifest`），而本模块不持有第二份副本。

#[path = "soil_builder.rs"]
mod soil_builder;
pub use soil_builder::*;

use std::fmt;

use crate::registry_core::identity::{NodeId, StableFaceId};

#[path = "source_location.rs"]
mod source_location;
pub use source_location::*;
#[path = "contract.rs"]
mod contract;
pub use contract::*;
#[path = "registration.rs"]
mod registration;
pub use registration::*;
#[path = "owned.rs"]
mod owned;
pub use owned::*;
#[path = "runtime_checks.rs"]
mod runtime_checks;
pub use runtime_checks::*;
#[path = "call_evidence.rs"]
mod call_evidence;
pub use call_evidence::*;

pub use crate::registry_core::plugin::contracts::*;

/// Whether a provided-name list contains one required name.
/// 提供名列表中是否含有一个要求名。
///
/// The compiled and owned twins store the same names as `&str` and `String`
/// respectively. Comparing them by hand is where a twin could quietly switch to
/// `starts_with`, case folding, or a positional index and still compile. This is
/// the only comparison either side performs, so a change to the rule is a change
/// to one function. `registration_rule_twins_report_identical_failures` and
/// `object_contract_twins_report_identical_failures` pin both callers.
/// 编译期与 owned 孪生分别把同一批名字存为 `&str` 与 `String`。手写比较正是某一侧可能
/// 悄悄改成 `starts_with`、大小写折叠或按下标取值却仍能编译的地方。这是两侧唯一的
/// 比较方式，因此规则改动只改一个函数。
/// `registration_rule_twins_report_identical_failures` 与
/// `object_contract_twins_report_identical_failures` 钉住两个调用方。
fn provided_contains<S: AsRef<str>>(provided: &[S], required: &S) -> bool {
    provided
        .iter()
        .any(|candidate| candidate.as_ref() == required.as_ref())
}

/// The two storage shapes one structural-rule check has to accept.
/// 结构规则校验必须同时接受的两种存储形态。
///
/// [`RegistrationRule::validate`] and [`OwnedRegistrationRule::validate`] used to
/// carry this check line by line. The exact failure wording was the only thing
/// either side promised, so a message edited in one twin left the compiled and
/// reloaded paths reporting different text for the same declaration while both
/// still compiled. This core holds the five checks and their wording once; each
/// twin only adapts its own storage. `RegistrationRule` supplies `&str` slices
/// and the owned rule `String` slices, so the lists are generic and neither side
/// has to allocate an adapter vector.
/// [`RegistrationRule::validate`] 与 [`OwnedRegistrationRule::validate`] 过去逐行各写
/// 一份。两侧唯一承诺一致的就是失败措辞，因此只改一侧的消息会让编译期与热重载两条路径
/// 对同一份声明报出不同文本，而两边都仍能编译。该核只保留这五项检查及其措辞，两个孪生
/// 各自只做存储适配。`RegistrationRule` 提供 `&str` 切片，owned 规则提供 `String`
/// 切片，因此列表用泛型表达，两侧都不需要为适配分配向量。
pub(crate) struct RegistrationRequirementCheck<'a, S> {
    required_preset: Option<&'a str>,
    required_parts: &'a [S],
    required_exports: &'a [S],
    required_handle_traits: &'a [S],
    required_part_traits: &'a [S],
    preset: &'a str,
    provided_parts: &'a [S],
    exports: &'a [S],
    handle: &'a str,
    handle_traits: &'a [S],
    parts: &'a str,
    part_traits: &'a [S],
}

/// Run the five structural checks and return every failure in declaration order.
/// 依声明顺序执行五项结构检查，返回全部失败项。
///
/// The order is part of the observable result: callers join the vector into one
/// diagnostic, so moving a check changes the message a host prints.
/// `parent_rule_aggregates_every_missing_structural_requirement` depends on all
/// five being present at once.
/// 顺序是可观察结果的一部分：调用方把该向量拼成一条诊断，移动检查就改变了宿主打印的
/// 消息。`parent_rule_aggregates_every_missing_structural_requirement` 依赖五项同时
/// 出现。
pub(crate) fn validate_registration_requirements<S: AsRef<str>>(
    check: RegistrationRequirementCheck<'_, S>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if let Some(expected) = check.required_preset
        && check.preset != expected
    {
        failures.push(format!(
            "preset `{expected}` is required, received `{}`",
            check.preset
        ));
    }
    for required in check.required_parts {
        if !provided_contains(check.provided_parts, required) {
            failures.push(format!(
                "required structural part `{}` is missing",
                required.as_ref()
            ));
        }
    }
    for required in check.required_exports {
        if !provided_contains(check.exports, required) {
            failures.push(format!(
                "required export `{}` is missing",
                required.as_ref()
            ));
        }
    }
    for required in check.required_handle_traits {
        if !provided_contains(check.handle_traits, required) {
            failures.push(format!(
                "handle `{}` must implement interface `{}`",
                check.handle,
                required.as_ref()
            ));
        }
    }
    for required in check.required_part_traits {
        if !provided_contains(check.part_traits, required) {
            failures.push(format!(
                "parts `{}` must implement interface `{}`",
                check.parts,
                required.as_ref()
            ));
        }
    }
    failures
}

/// The two storage shapes one construction-contract check has to accept.
/// 构造合同校验必须同时接受的两种存储形态。
///
/// [`ObjectContract::validate`] and [`OwnedObjectContract::validate`] were a
/// byte-identical pair, which is exactly the kind of copy that stays identical
/// only until someone edits one of them. The two failure texts and the
/// required/provided relation live here once; the twins pass their own slices.
/// [`ObjectContract::validate`] 与 [`OwnedObjectContract::validate`] 是一对逐字节相同
/// 的副本——这种副本只有在没人改动其中之一时才保持相同。两条失败文本与"要求/提供"
/// 关系在此只写一份，两个孪生各自传入自己的切片。
pub(crate) fn validate_object_contract<S: AsRef<str>>(
    required_parts: &[S],
    provided_parts: &[S],
    object: &str,
) -> Vec<String> {
    let mut failures = Vec::new();
    for required in required_parts {
        if !provided_contains(provided_parts, required) {
            failures.push(format!(
                "`{object}` is missing construction part `{}`",
                required.as_ref()
            ));
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::identity::root_node_id;

    /// A compiled declaration whose every list is empty and whose preset, parts,
    /// and handle are fixed, so a failing message has a known text.
    /// 一份各列表皆空、preset/parts/handle 固定的编译期声明，使失败消息文本可知。
    fn info(namespace: &'static str, kind: &'static str) -> RegistrationInfo {
        RegistrationInfo {
            namespace,
            id: NodeId::from_namespaced_path(namespace, "src/item.rs", kind),
            parent: root_node_id(namespace),
            kind,
            preset: "NoPreset",
            parts: "NoParts",
            params: "Params",
            handle: "Handle",
            stable_name: None,
            name: LocalizedText {
                zh: "名称",
                en: "Name",
            },
            summary: LocalizedText { zh: "", en: "" },
            exports: &[],
            needs_registry: false,
            registry_name: kind,
            getting_from_other_registry: None,
            registry_rule_path: "<test>",
            registry_rule: RegistrationRule::ANY,
            admission: Admission::ANY,
            requires: &[],
            provides: &[],
            contract: ObjectContract {
                required_parts: &["paint"],
                provided_parts: &[],
            },
            flow: FlowContract::NONE,
            flow_provider: None,
            handle_traits: &[],
            part_traits: &[],
            runtime_checks: &[],
            plugin: None,
            source: SourceLocation {
                file: "src/item.rs",
                line: 1,
                column: 1,
                function: kind,
            },
        }
    }

    /// The owned twin of [`info`], carrying exactly the same declaration data.
    /// [`info`] 的 owned 孪生，承载完全相同的声明数据。
    fn owned_snapshot(namespace: &str, kind: &str) -> RegistrationSnapshot {
        RegistrationSnapshot {
            namespace: namespace.to_owned(),
            id: NodeId::from_namespaced_path(namespace, "src/item.rs", kind),
            parent: root_node_id(namespace),
            kind: kind.to_owned(),
            preset: "NoPreset".to_owned(),
            parts: "NoParts".to_owned(),
            params: "Params".to_owned(),
            handle: "Handle".to_owned(),
            stable_name: None,
            name: OwnedLocalizedText {
                zh: "名称".to_owned(),
                en: "Name".to_owned(),
            },
            summary: OwnedLocalizedText {
                zh: String::new(),
                en: String::new(),
            },
            exports: Vec::new(),
            needs_registry: false,
            registry_name: kind.to_owned(),
            getting_from_other_registry: None,
            registry_rule_path: "<test>".to_owned(),
            registry_rule: OwnedRegistrationRule {
                required_preset: None,
                required_parts: Vec::new(),
                required_exports: Vec::new(),
                required_handle_traits: Vec::new(),
                required_part_traits: Vec::new(),
            },
            admission: Admission::ANY.into_owned(),
            requires: Vec::new(),
            provides: Vec::new(),
            contract: OwnedObjectContract {
                required_parts: vec!["paint".to_owned()],
                provided_parts: Vec::new(),
            },
            flow: OwnedFlowContract::none(),
            flow_provider: None,
            handle_traits: Vec::new(),
            part_traits: Vec::new(),
            runtime_checks: Vec::new(),
            plugin: None,
            source: OwnedSourceLocation {
                file: "src/item.rs".to_owned(),
                line: 1,
                column: 1,
                function: kind.to_owned(),
            },
        }
    }

    /// A missing requirement must produce the same text through the compiled and
    /// the reloaded entry point; the two validation bodies now share one core.
    /// 同一条缺失要求经编译期与热重载入口必须产出相同文本；两份校验体现在共用一个核。
    #[test]
    fn registration_rule_twins_report_identical_failures() {
        let namespace = "declaration-twins";
        let rule = RegistrationRule::new()
            .require_preset("ActionParts")
            .require_parts(&["paint"])
            .require_exports(&["control.render"])
            .require_handle_traits(&["ControlHandle"])
            .require_part_traits(&["ActionParts"]);
        let compiled_info = info(namespace, "Button");
        let mut reloaded = owned_snapshot(namespace, "Button");
        reloaded.registry_rule = rule.into_owned();

        let compiled = rule.validate_registration_rule(&compiled_info);
        let owned = reloaded
            .registry_rule
            .validate_owned_registration_rule(&reloaded);
        assert_eq!(compiled, owned, "compiled and reloaded disagree");
        for message in [
            "preset `ActionParts` is required, received `NoPreset`",
            "required structural part `paint` is missing",
            "required export `control.render` is missing",
            "handle `Handle` must implement interface `ControlHandle`",
            "parts `NoParts` must implement interface `ActionParts`",
        ] {
            assert!(
                compiled.iter().any(|failure| failure == message),
                "missing diagnostic: {message}\n{compiled:?}"
            );
        }
    }

    /// The same construction-contract failure must read identically on both
    /// storage shapes. Only the part relation is left to report: the output half
    /// compared two author-written names with each other and said nothing about
    /// the types, so it was replaced by `assert_contract` at compile time.
    /// 同一条构造合同失败在两种存储形态上必须读起来完全一致。现在只剩 part 关系可报告：
    /// 输出那一半只是把作者写下的两个名字互相比较、对类型什么都没说，因此已由编译期的
    /// `assert_contract` 取代。
    #[test]
    fn object_contract_twins_report_identical_failures() {
        let compiled_contract = ObjectContract {
            required_parts: &["paint", "layout"],
            provided_parts: &["layout"],
        };
        let owned_contract = OwnedObjectContract {
            required_parts: vec!["paint".to_owned(), "layout".to_owned()],
            provided_parts: vec!["layout".to_owned()],
        };

        let compiled = compiled_contract.validate_object_contract("Button");
        let owned = owned_contract.validate_owned_object_contract("Button");
        assert_eq!(compiled, owned, "compiled and owned disagree");
        assert_eq!(
            compiled,
            ["`Button` is missing construction part `paint`".to_owned()]
        );
    }

    /// Both admission gates must read the shared prefix predicate the same way.
    /// 两道准入闸门必须按同一种方式读取共享前缀谓词。
    #[test]
    fn admission_twins_accept_and_reject_identical_paths() {
        let compiled = Admission::new(&["ui", "ui/controls"], &["ui/experimental"]);
        let reloaded = compiled.into_owned();
        for path in [
            "ui",
            "ui/controls",
            "ui/controls/Button",
            "ui/experimental",
            "ui/experiment",
            "ui2",
            "graphics",
            "",
        ] {
            assert_eq!(
                compiled.accepts(path),
                reloaded.accepts(path),
                "admission twins disagree on `{path}`"
            );
        }
    }
}
