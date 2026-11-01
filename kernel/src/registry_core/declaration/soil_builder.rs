//! Native Rust registration declarations consumed by the face attribute.
//! 由 face 属性消费的原生 Rust 注册声明。

use super::*;
use crate::registry_core::{FlowContract, FlowContractProvider};

/// A registration declaration; methods configure the actual compiled metadata.
/// 注册声明；各方法配置实际参与编译的元数据。
#[derive(Clone, Copy, Debug)]
pub struct Soil {
    info: RegistrationInfo,
    previous: Option<&'static Soil>,
    requirement: Option<RequirementSpec>,
}

/// Compatibility name for the original `xirang_kernel::declaration::Face` builder.
/// 原 `xirang_kernel::declaration::Face` 构造器的兼容名称。
#[doc(hidden)]
pub type Face = Soil;

impl Soil {
    /// Declare the business type; the attribute records its source spelling.
    /// 声明业务类型；属性记录该类型的源码拼写。
    pub const fn of<T>() -> Self {
        let _ = core::mem::size_of::<*const T>();
        Self {
            info: RegistrationInfo {
                namespace: "",
                id: crate::ROOT_NODE_ID,
                parent: crate::ROOT_NODE_ID,
                kind: "",
                preset: "NoPreset",
                parts: "NoParts",
                params: "",
                handle: "",
                stable_name: None,
                name: LocalizedText { en: "", zh: "" },
                summary: LocalizedText { en: "", zh: "" },
                exports: &[],
                needs_registry: false,
                registry_name: "",
                getting_from_other_registry: None,
                registry_rule_path: "",
                registry_rule: RegistrationRule::ANY,
                admission: Admission::ANY,
                requires: &[],
                provides: &[],
                contract: ObjectContract {
                    required_parts: &[],
                    provided_parts: &[],
                },
                flow: FlowContract::NONE,
                flow_provider: None,
                handle_traits: &[],
                part_traits: &[],
                runtime_checks: &[],
                plugin: None,
                source: SourceLocation {
                    file: "",
                    line: 0,
                    column: 0,
                    function: "",
                },
            },
            previous: None,
            requirement: None,
        }
    }

    /// Set the preset and retain its construction requirements.
    /// 设置 preset 并保留它的构造需求。
    pub const fn preset<T: PresetContract>(mut self) -> Self {
        self.info.contract.required_parts = T::REQUIRED_PARTS;
        self
    }

    /// Set the parts and retain their provided construction parts.
    /// 设置 parts 并保留它提供的构造部件。
    pub const fn parts<T: PartsContract>(mut self) -> Self {
        self.info.contract.provided_parts = T::PROVIDED_PARTS;
        self
    }

    /// Set the English and Chinese display names, in that order.
    /// 按英文、中文的顺序设置显示名称。
    pub const fn name(mut self, en: &'static str, zh: &'static str) -> Self {
        self.info.name = LocalizedText { en, zh };
        self
    }

    /// Set the English and Chinese summaries, in that order.
    /// 按英文、中文的顺序设置摘要。
    pub const fn summary(mut self, en: &'static str, zh: &'static str) -> Self {
        self.info.summary = LocalizedText { en, zh };
        self
    }

    /// Append a capability requirement without an allocation or fixed count limit.
    /// 追加能力需求，不分配内存，也不设置固定条数上限。
    pub const fn requires(&'static self, capability: &'static str, provider: &'static str) -> Self {
        Self {
            previous: Some(self),
            requirement: Some(RequirementSpec {
                capability,
                provider,
            }),
            ..*self
        }
    }

    /// Set a flow provider; an explicit flow expression takes precedence in the attribute.
    /// 设置数据流提供者；属性中显式的数据流表达式优先。
    pub const fn flow_provider<T: FlowContractProvider>(mut self) -> Self {
        self.info.flow = T::FLOW_CONTRACT;
        self
    }

    /// Apply compiler-checked handle contract labels from `contracts!`.
    /// 应用 `contracts!` 检查后的 handle 合同标签。
    pub const fn handle_contracts(mut self, labels: &'static [&'static str]) -> Self {
        self.info.handle_traits = labels;
        self
    }

    /// Apply compiler-checked parts contract labels from `contracts!`.
    /// 应用 `contracts!` 检查后的 parts 合同标签。
    pub const fn part_contracts(mut self, labels: &'static [&'static str]) -> Self {
        self.info.part_traits = labels;
        self
    }

    /// Return the data the declaration actually constructed.
    /// 返回声明实际构造的数据。
    #[doc(hidden)]
    pub const fn metadata(self) -> RegistrationInfo {
        self.info
    }

    /// Materialize the requirement chain at compile time, preserving source order.
    /// 编译期展开需求链，保持源码顺序。
    #[doc(hidden)]
    pub const fn requirement_array<const N: usize>(&self) -> [RequirementSpec; N] {
        let mut result = [RequirementSpec {
            capability: "",
            provider: "",
        }; N];
        let mut cursor = self;
        let mut index = N;
        loop {
            if let Some(requirement) = cursor.requirement {
                assert!(index > 0, "requirement count mismatch");
                index -= 1;
                result[index] = requirement;
            }
            match cursor.previous {
                Some(previous) => cursor = previous,
                None => break,
            }
        }
        assert!(index == 0, "requirement count mismatch");
        result
    }
}

macro_rules! setters {
    ($(($method:ident, $field:ident, $ty:ty, $en:literal, $zh:literal)),* $(,)?) => {
        impl Soil { $(
            #[doc = $en]
            #[doc = $zh]
            pub const fn $method(mut self, value: $ty) -> Self { self.info.$field = value; self }
        )* }
    };
}

setters! {
    (under, parent, NodeId, "Register under this parent.", "注册到此父级。"),
    (exports, exports, &'static [&'static str], "Declare exports.", "声明导出项。"),
    (needs_registry, needs_registry, bool, "Declare registry ownership.", "声明注册机所有权。"),
    (getting_from_other_registry, getting_from_other_registry, Option<&'static str>, "Declare an external registry source.", "声明外部注册机来源。"),
    (registry_rule_path, registry_rule_path, &'static str, "Name the rule source.", "指定规则源码。"),
    (registry_rule, registry_rule, RegistrationRule, "Set the registration rule.", "设置注册规则。"),
    (admission, admission, Admission, "Set dependency admission.", "设置依赖准入。"),
    (handle_traits, handle_traits, &'static [&'static str], "Declare handle labels.", "声明 handle 标签。"),
    (part_traits, part_traits, &'static [&'static str], "Declare parts labels.", "声明 parts 标签。"),
    (provides, provides, &'static [&'static str], "Declare provided capabilities.", "声明提供的能力。"),
    (flow, flow, FlowContract, "Set the flow contract.", "设置数据流合同。"),
    (runtime_checks, runtime_checks, &'static [RuntimeCheckSpec], "Set runtime checks.", "设置运行期检查。"),
}

impl Soil {
    /// Set the stable author-owned name.
    /// 设置作者持有的稳定名称。
    pub const fn stable_name(mut self, value: &'static str) -> Self {
        self.info.stable_name = Some(value);
        self
    }
    /// Declare plugin provenance.
    /// 声明插件来源。
    pub const fn plugin(mut self, value: crate::PluginManifest) -> Self {
        self.info.plugin = Some(value);
        self
    }
    /// Override the source for an external declaration.
    /// 覆盖外部声明的源码位置。
    pub const fn source(mut self, value: &'static str) -> Self {
        self.info.source.file = value;
        self
    }
}
