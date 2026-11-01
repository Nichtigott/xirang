//! Registration declarations and the compile-time construction contracts.
//! 注册声明与编译期构造合同。
//!
//! The vocabulary is split by concept: the compile-time declaration types stay
//! here, the construction-contract traits and records live in [`contract`], and
//! the owned snapshot forms live in [`owned`]. The module tree mirrors that
//! split, and every public path is preserved by re-export.
//! 词表按概念拆分：编译期声明类型留在本页，构造合同 trait 与记录位于 [`contract`]，
//! 拥有型快照形式位于 [`owned`]。文件树与之一致，所有公开路径都通过再导出保留。

use super::*;
use crate::registry_core::lexicon::path_is_under;

/// The **author-facing** declaration of a face (H/A2e): the value-like fields, plus a `DEFAULT` to spread.
/// 面的**作者形**声明（H/A2e）：值类字段，外加一个供展开的 `DEFAULT`。
///
/// The four **type-like** fields a face declares — `preset`, `parts`, `params`, `handle` — are *not* here,
/// because a struct literal cannot carry a type: they live as associated types on [`Face`]. That split is
/// what keeps "a wrong type is a compile error" after the move away from the macro (measured: written as a
/// literal, the preset could only have been a string).
/// 面声明的四个**类型类**字段——`preset`、`parts`、`params`、`handle`——**不在这里**，因为结构体字面量装不下
/// 类型：它们住在 [`Face`] 的关联类型上。这条分界正是"离开宏之后，写错类型仍然是编译错误"的保证（实测：写成
/// 字面量时，preset 只能退化成字符串）。
///
/// Everything is `const`-constructible, so a face can write one declaration and spread the rest:
/// 每个字段都能在 `const` 里构造，因此一个面可以只写它要改的那几个字段、其余展开：
///
/// ```ignore
/// impl Card {
///     pub const REGISTRATION: Registration = Registration {
///         parent: crate::board::Board::REGISTRATION.parent,
///         ..Registration::DEFAULT
///     };
/// }
/// ```
/// # Who supplies each field
/// # 每个字段由谁供给
///
/// The audit that decides which half a field belongs to — asked of every field when the declaration
/// spelling changed (H/A2e), because a value the **build** derives cannot be written by an author and must
/// cross through [`FaceInputs`] instead:
/// 决定一个字段属于哪一半的那次审计——在声明拼法改变时（H/A2e）对每个字段问过一遍，因为**构建**派生出来的值
/// 作者写不出来，必须改经 [`FaceInputs`] 过境：
///
/// - **The author writes it** (sixteen of them): `parent`, `stable_name`, `name`, `summary`, `exports`,
///   `needs_registry`, `getting_from_other_registry`, `registry_rule_path`, `admission`, `requires`,
///   `provides`, `flow`, `flow_provider`, `runtime_checks`, `plugin` — everything a face declares about
///   itself.
/// - **The build supplies it** (two of them):
///   - `registry_name` — the name a face owns among its siblings; the declaration macros derived it as
///     `last_path_segment(module_path!())`, and it is not part of the declaration vocabulary, so a face
///     written as a literal cannot state it. **Moved to `FaceInputs`** ✓.
///   - `registry_rule` — the rule loaded from `registry_rule_path`; the macro spelling carried the loaded
///     value in its declaration, so a literal-form face has nothing to carry and falls back to
///     `RegistrationRule::ANY`, which silently stops the parent rule from rejecting anything.
///     **Not moved yet** ✗ — the renderer has to expose the loaded rule first.
/// - **作者的字段（十六个）**：`parent`、`stable_name`、`name`、`summary`、`exports`、`needs_registry`、
///   `getting_from_other_registry`、`registry_rule_path`、`admission`、`requires`、`provides`、`flow`、
///   `flow_provider`、`runtime_checks`、`plugin`——一个面对面自己声明的一切。
/// - **构建供给的（两个）**：`registry_name`（**已搬进 `FaceInputs`** ✓）；`registry_rule`（宏式拼法把
///   **加载后的规则值**带在声明里 ✓，字面量形没得可带 ⇒ 回落 `ANY` ⇒ 父规则静默失效 ✗。**尚未搬** ✗——
///   需要渲染器先把加载后的规则暴露出来）。
pub struct Registration {
    /// Node identity of the face this one is registered under.
    /// 本注册面所挂载到的父节点身份。
    pub parent: NodeId,
    /// Opt-in identity that survives a source-file move.
    /// 可选的、能跨源码文件移动保持不变的逻辑身份。
    pub stable_name: Option<&'static str>,
    /// Human-readable name, in both languages.
    /// 双语的人类可读名字。
    pub name: LocalizedText,
    /// Human-readable summary, in both languages.
    /// 双语的人类可读摘要。
    pub summary: LocalizedText,
    /// Names this face exports to its children.
    /// 本面导出给子面的名字。
    pub exports: &'static [&'static str],
    /// Whether this face owns a registry (its children must satisfy its rule).
    /// 本面是否拥有注册机（其子对象必须满足它旁边的规则）。
    pub needs_registry: bool,
    /// Registry name this face is keyed by, when it is not the module's own name.
    /// 本面据以索引的注册名（当它不是模块自身的名字时）。
    pub registry_name: &'static str,
    /// Object this face is taken from, when it comes from another registry.
    /// 本面取自另一个注册机时的来源对象。
    pub getting_from_other_registry: Option<&'static str>,
    /// Sibling rule this face's children are checked against.
    /// 本面的子对象据以校验的同目录规则。
    pub registry_rule_path: &'static str,
    /// The rule itself, when it is written inline.
    /// 规则本身（内联书写时）。
    pub registry_rule: RegistrationRule,
    /// Which external paths this face admits.
    /// 本面放行哪些外部路径。
    pub admission: Admission,
    /// Capabilities this face requires.
    /// 本面需要的capability。
    pub requires: &'static [RequirementSpec],
    /// Capabilities this face provides.
    /// 本面提供的 capability。
    pub provides: &'static [&'static str],
    /// The flow contract this face fills, when it fills one.
    /// 本面所填的流合同（若填了）。
    pub flow: Option<FlowContract>,
    /// The object whose flow this face provides, when it provides instead of filling.
    /// 本面提供其流量的对象（当它是提供方而非填写方时）。
    pub flow_provider: Option<&'static str>,
    /// Runtime checks this face asks for.
    /// 本面要求的运行期检查。
    pub runtime_checks: &'static [RuntimeCheckSpec],
    /// The plugin this face can be replaced by, when it is replaceable surface.
    /// 本面可被哪个插件替换（当它是可替换表面时）。
    pub plugin: Option<PluginManifest>,
}

impl Registration {
    /// Every field at its empty value: what `..` spreads when the author writes only what matters.
    /// 每个字段的空值：作者只写要改的那几个字段时，`..` 展开的就是它。
    pub const DEFAULT: Registration = Registration {
        parent: NodeId::from_raw([0; 16]),
        stable_name: None,
        name: LocalizedText { zh: "", en: "" },
        summary: LocalizedText { zh: "", en: "" },
        exports: &[],
        needs_registry: false,
        registry_name: "",
        getting_from_other_registry: None,
        registry_rule_path: "",
        registry_rule: RegistrationRule::ANY,
        admission: Admission::ANY,
        requires: &[],
        provides: &[],
        flow: None,
        flow_provider: None,
        runtime_checks: &[],
        plugin: None,
    };
}

/// One capability requirement and the object expected to provide it.
/// 一条能力需求，以及本应提供它的对象。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequirementSpec {
    /// Capability name that must be resolvable in the registry tree.
    /// 必须在注册树中可解析的能力名称。
    pub capability: &'static str,
    /// Object expected to provide that capability; any other provider fails the check.
    /// 本应提供该能力的对象；由其他对象提供即判定失败。
    pub provider: &'static str,
}

/// The admission rule: a denial overrides any allowance, and an empty allow list admits every path
/// that is not denied.
/// 门禁规则：拒绝优先于允许；允许列表为空时，接受所有未被拒绝的路径。
///
/// The static declaration and its owned twin are two spellings of one gate, so the rule is read
/// once here: a copy per spelling is how "denied wins" and "empty admits everything" drift apart,
/// and either drift silently changes which external registries a tree accepts.
/// 静态声明与它的 owned 孪生是同一道门禁的两种拼法，因此规则在这里只读一次：每种拼法各存一份副本，正是
/// "拒绝优先"与"空表全放行"逐渐分歧的原因，而任一处分歧都会静默改变这棵树接受哪些外部注册机。
pub(super) fn admits<A: AsRef<str>, D: AsRef<str>>(
    allowed: &[A],
    denied: &[D],
    path: &str,
) -> bool {
    if denied
        .iter()
        .any(|prefix| path_is_under(path, prefix.as_ref()))
    {
        return false;
    }
    allowed.is_empty()
        || allowed
            .iter()
            .any(|prefix| path_is_under(path, prefix.as_ref()))
}

/// Dependency admission for objects produced outside the current registry tree.
/// 当前注册树对外部注册机产物的依赖门禁。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    /// External path prefixes admitted; empty admits every path not denied.
    /// 允许进入的外部路径前缀；为空时允许所有未被拒绝的路径。
    pub allowed_paths: &'static [&'static str],
    /// External path prefixes refused; a denial overrides any allowance.
    /// 拒绝进入的外部路径前缀；拒绝优先于允许。
    pub denied_paths: &'static [&'static str],
}

impl Admission {
    /// Admission that accepts every external path.
    /// 接受任何外部路径的门禁。
    pub const ANY: Self = Self::new(&[], &[]);

    /// Build an admission from explicit allow and deny path prefixes.
    /// 用显式的允许与拒绝路径前缀构造门禁。
    pub const fn new(
        allowed_paths: &'static [&'static str],
        denied_paths: &'static [&'static str],
    ) -> Self {
        Self {
            allowed_paths,
            denied_paths,
        }
    }

    /// Admission allowing only paths under the listed prefixes.
    /// 只允许列出的前缀之下的路径进入的门禁。
    pub const fn allow_paths(paths: &'static [&'static str]) -> Self {
        Self::new(paths, &[])
    }

    /// Whether an external path may enter: denied prefixes win, and an empty
    /// allow list admits every path not denied.
    /// 外部路径是否可以进入：拒绝前缀优先，允许列表为空时接受所有未被拒绝的路径。
    pub fn accepts(self, path: &str) -> bool {
        admits(self.allowed_paths, self.denied_paths, path)
    }

    /// Copy this admission into the owned form a snapshot can retain.
    /// 将该门禁复制为快照可长期持有的拥有所有权形式。
    pub fn into_owned(self) -> OwnedAdmission {
        OwnedAdmission {
            allowed_paths: self
                .allowed_paths
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            denied_paths: self
                .denied_paths
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }
}

/// Structural rule for faces entering a Registry.
/// 注册面进入 Registry 时必须满足的结构规范。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistrationRule {
    /// Preset name every admitted face must declare; `None` leaves it unconstrained.
    /// 每个被接纳的注册面都必须声明的 preset 名；`None` 表示不作要求。
    pub required_preset: Option<&'static str>,
    /// Part names every admitted face must provide.
    /// 每个被接纳的注册面都必须提供的 part 名称。
    pub required_parts: &'static [&'static str],
    /// Export names every admitted face must declare.
    /// 每个被接纳的注册面都必须声明的导出名称。
    pub required_exports: &'static [&'static str],
    /// Interfaces the handle type must implement, for example `ControlHandle`.
    /// handle 类型必须实现的接口，例如 `ControlHandle`。
    pub required_handle_traits: &'static [&'static str],
    /// Interfaces the parts type must implement, for example `ActionParts`.
    /// parts 类型必须实现的接口，例如 `ActionParts`。
    pub required_part_traits: &'static [&'static str],
}

impl RegistrationRule {
    /// Rule with no structural requirements; it admits any face.
    /// 不含任何结构要求的规则，接纳任意注册面。
    pub const ANY: Self = Self::new();

    /// Whether this rule constrains nothing — that is, whether it equals [`Self::ANY`].
    /// 本规则是否不含任何约束——也就是它是否等于 [`Self::ANY`]。
    ///
    /// A face's rule has two possible suppliers: the author's declaration, and the build. The build
    /// supplies it only when the declaration does not, and "does not" needs a `const` test, because a
    /// face's declaration is assembled in a `const` context.
    /// 一个面的规则有两个可能的供给方：作者的声明与构建。构建只在声明**没有**给的时候供给，而"没有给"需要一个
    /// `const` 判定，因为面的声明是在 `const` 上下文里拼装的。
    pub const fn is_any(&self) -> bool {
        self.required_preset.is_none()
            && self.required_parts.is_empty()
            && self.required_exports.is_empty()
            && self.required_handle_traits.is_empty()
            && self.required_part_traits.is_empty()
    }

    /// Start an unconstrained structural rule.
    /// 创建一个尚未添加结构要求的规则。
    pub const fn new() -> Self {
        Self {
            required_preset: None,
            required_parts: &[],
            required_exports: &[],
            required_handle_traits: &[],
            required_part_traits: &[],
        }
    }

    /// Set the required preset name and return the updated rule.
    /// 设置所需 preset 名并返回更新后的规则。
    pub const fn require_preset(mut self, preset: &'static str) -> Self {
        self.required_preset = Some(preset);
        self
    }

    /// Set the required part names and return the updated rule.
    /// 设置所需 part 名称并返回更新后的规则。
    pub const fn require_parts(mut self, parts: &'static [&'static str]) -> Self {
        self.required_parts = parts;
        self
    }

    /// Set the required export names and return the updated rule.
    /// 设置所需导出名称并返回更新后的规则。
    pub const fn require_exports(mut self, exports: &'static [&'static str]) -> Self {
        self.required_exports = exports;
        self
    }

    /// Set the required handle trait names and return the updated rule.
    /// 设置所需 handle trait 名称并返回更新后的规则。
    pub const fn require_handle_traits(mut self, traits: &'static [&'static str]) -> Self {
        self.required_handle_traits = traits;
        self
    }

    /// Set the required parts trait names and return the updated rule.
    /// 设置所需 parts trait 名称并返回更新后的规则。
    pub const fn require_part_traits(mut self, traits: &'static [&'static str]) -> Self {
        self.required_part_traits = traits;
        self
    }

    /// Return one message per unmet structural requirement for `info`; an empty
    /// result means the declaration may enter the registry.
    /// 针对 `info` 的每项未满足结构要求各返回一条消息；结果为空表示该声明可进入注册机。
    pub fn validate_registration_rule(&self, info: &RegistrationInfo) -> Vec<String> {
        validate_registration_requirements(RegistrationRequirementCheck {
            required_preset: self.required_preset,
            required_parts: self.required_parts,
            required_exports: self.required_exports,
            required_handle_traits: self.required_handle_traits,
            required_part_traits: self.required_part_traits,
            preset: info.preset,
            provided_parts: info.contract.provided_parts,
            exports: info.exports,
            handle: info.handle,
            handle_traits: info.handle_traits,
            parts: info.parts,
            part_traits: info.part_traits,
        })
    }

    /// Copy this rule into the owned form a snapshot can retain.
    /// 将该规则复制为快照可长期持有的拥有所有权形式。
    pub fn into_owned(self) -> OwnedRegistrationRule {
        OwnedRegistrationRule {
            required_preset: self.required_preset.map(str::to_owned),
            required_parts: self
                .required_parts
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            required_exports: self
                .required_exports
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            required_handle_traits: self
                .required_handle_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            required_part_traits: self
                .required_part_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }
}

impl Default for RegistrationRule {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistrationInfo {
    /// Every field at its empty value: what a builder spreads when the author writes only what matters.
    /// 每个字段的空值：作者只写要改的那些字段时，构造器展开的就是它。
    pub const DEFAULT: RegistrationInfo = RegistrationInfo {
        namespace: "",
        id: NodeId::from_raw([0; 16]),
        parent: NodeId::from_raw([0; 16]),
        kind: "",
        preset: "",
        parts: "",
        params: "",
        handle: "",
        stable_name: None,
        name: LocalizedText { zh: "", en: "" },
        summary: LocalizedText { zh: "", en: "" },
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
        flow: crate::FlowContract::NONE,
        flow_provider: None,
        handle_traits: &[],
        part_traits: &[],
        runtime_checks: &[],
        plugin: None,
        // A synthetic location, by the rule `SourceLocation::is_synthetic` states.
        // 一个合成位置，按 `SourceLocation::is_synthetic` 写明的规则。
        source: SourceLocation {
            file: "<unset>",
            line: 0,
            column: 0,
            function: "<unset>",
        },
    };

    /// The same assembly, for a caller that has the face's **type** and not its two type parameters.
    /// 同一次拼装，给手里只有面的**类型**、而没有它那两个类型参数的调用方。
    ///
    /// The two signatures exist because their callers differ: the declaration macros hold `preset` and
    /// `parts` as types and must not add an `impl FaceContract for $kind` (a trait impl is keyed on the
    /// type, so a crate that declares the same kind twice could not have two), while the build reading a
    /// face written in the new spelling has only the face's name. Both delegate to the same body, so the
    /// derivation of the contract records stays in one place.
    /// 两个签名并存，是因为调用方不同：声明宏手里 `preset`/`parts` 就是类型，而它们**不能**加
    /// `impl FaceContract for $kind`（trait impl 以类型为键，一个 crate 里同一 kind 声明两次就不可能有两条）；
    /// 而构建读到用新拼法写的面时，手里只有那个面的名字。两者都委派给同一段函数体，因此合同记录的推导
    /// 仍然只有一处。
    pub const fn assemble_face<F: FaceContract>(
        inputs: FaceInputs,
        declaration: Registration,
    ) -> RegistrationInfo {
        RegistrationInfo::assemble::<F::Preset, F::Parts>(
            FaceInputs {
                handle_traits: F::HANDLE_TRAITS,
                part_traits: F::PART_TRAITS,
                ..inputs
            },
            declaration,
        )
    }

    /// Assemble the owned snapshot both spellings of a declaration must produce (H/A2e).
    /// 拼出两种声明拼法都必须产出的那份拥有型快照（H/A2e）。
    ///
    /// Two arguments, because a declaration has exactly two halves: the one **the build knows** and the
    /// one **the author writes**. The type parameters carry the type-like half and are checked here
    /// exactly as [`assert_contract`] checks them.
    /// 两个参数，因为一份声明恰好有两半：**构建知道的**那一半，和**作者写的**那一半。类型类的那一半由类型
    /// 参数承载，并在这里按 [`assert_contract`] 的同一套检查校验。
    ///
    /// Type parameters rather than a trait bound on the face: a trait impl is keyed on the **type**, so a
    /// crate that declares the same kind twice (which the fixtures do on purpose) could not have two.
    /// 用类型参数而不是给面加 trait 约束：trait impl 以**类型**为键，而一个 crate 里同一 kind 声明两次
    /// （夹具就是这样用的）就会撞车。
    pub const fn assemble<P, T>(inputs: FaceInputs, declaration: Registration) -> RegistrationInfo
    where
        P: PresetContract,
        T: PartsContract<Output = P::Output>,
    {
        RegistrationInfo {
            namespace: inputs.namespace,
            id: inputs.id,
            parent: declaration.parent,
            kind: inputs.kind,
            preset: inputs.preset,
            parts: inputs.parts,
            params: inputs.params,
            handle: inputs.handle,
            stable_name: declaration.stable_name,
            name: declaration.name,
            summary: declaration.summary,
            exports: declaration.exports,
            needs_registry: declaration.needs_registry,
            registry_name: if declaration.registry_name.is_empty() {
                inputs.registry_name
            } else {
                declaration.registry_name
            },
            getting_from_other_registry: declaration.getting_from_other_registry,
            registry_rule_path: declaration.registry_rule_path,
            registry_rule: if declaration.registry_rule.is_any() {
                inputs.registry_rule
            } else {
                declaration.registry_rule
            },
            admission: declaration.admission,
            requires: declaration.requires,
            provides: declaration.provides,
            contract: ObjectContract {
                required_parts: P::REQUIRED_PARTS,
                provided_parts: T::PROVIDED_PARTS,
            },
            flow: match declaration.flow {
                Some(flow) => flow,
                None => crate::FlowContract::NONE,
            },
            flow_provider: declaration.flow_provider,
            handle_traits: inputs.handle_traits,
            part_traits: inputs.part_traits,
            runtime_checks: declaration.runtime_checks,
            plugin: declaration.plugin,
            source: inputs.source,
        }
    }
}

/// The half of a face declaration **only the build can supply** (H/A2e): namespace, identity, kind, the
/// four type names, the derived trait labels and the source location.
/// 面声明里**只有构建能供给**的那一半（H/A2e）：命名空间、身份、kind、四个类型名、推导出的 trait 标签与
/// 源码位置。
///
/// The other half is the [`Registration`] the author writes; [`RegistrationInfo::assemble`] joins them.
/// 另一半是作者写的 [`Registration`]；[`RegistrationInfo::assemble`] 把两半合起来。
pub struct FaceInputs {
    /// Package namespace that owns the declaration.
    /// 拥有本声明的包命名空间。
    pub namespace: &'static str,
    /// Stable node identity the build assigned to this face.
    /// 构建分配给本面的稳定节点身份。
    pub id: NodeId,
    /// Face kind name, which is the name of the type the face constructs.
    /// 注册面种类名，也就是本面所构造的类型的名字。
    pub kind: &'static str,
    /// The name this face owns among its siblings, which is what decides whether two of them collide.
    /// 本面在兄弟之间拥有的那个名字——两个兄弟面会不会撞名，由它决定。
    ///
    /// The declaration macros fill it with `last_path_segment(module_path!())`, and it is **not** part of
    /// the declaration vocabulary: a face cannot write it. A face written as a `Registration` literal
    /// therefore gets it from the build, which is the side that knows the module.
    /// 声明宏把它填成 `last_path_segment(module_path!())`，而它**不在**声明词汇表里：面写不出它。因此写成
    /// `Registration` 字面量的面从**构建**取它——知道模块的是构建那一侧。
    pub registry_name: &'static str,
    /// The structural rule this face imposes on the children it admits, as the build resolves it.
    /// 本面对它接纳的子面所施加的结构规则，由构建解析得出。
    ///
    /// The declaration macros derived it from the face's own `registry_rule` module whenever the face
    /// owns a registry (`needs_registry`), by naming that module's constant as a Rust path — so the value
    /// is one only the compiler can evaluate, and a face written as a `Registration` literal has nothing
    /// to carry. The build therefore supplies it, and the declaration wins whenever the author wrote one.
    /// 只要面拥有注册机（`needs_registry`），声明宏就从面自己的 `registry_rule` 模块推导它——办法是把那个模块的
    /// 常量写成一个 Rust 路径 —— 因此这个值只有**编译器**能求出来，而写成 `Registration` 字面量的面没得可带。
    /// 于是由构建供给；而作者若自己写了，则以声明为准。
    pub registry_rule: RegistrationRule,
    /// Preset type path, as the build writes it.
    /// preset 类型路径，由构建写出。
    pub preset: &'static str,
    /// Parts type path, as the build writes it.
    /// parts 类型路径，由构建写出。
    pub parts: &'static str,
    /// Parameter type path this face declares for its construction input.
    /// 本面为其构造输入声明的参数类型路径。
    pub params: &'static str,
    /// Handle type path that exposes this face to its owner.
    /// 向其拥有者暴露本面的 handle 类型路径。
    pub handle: &'static str,
    /// Source location of the declaration site.
    /// 声明点的源码位置。
    pub source: SourceLocation,
    /// Labels of the handle contracts this face asserts, derived from their types.
    /// 本面断言的 handle 合同标签，由它们的类型推导而来。
    pub handle_traits: &'static [&'static str],
    /// Labels of the part contracts this face asserts, derived from their types.
    /// 本面断言的 part 合同标签，由它们的类型推导而来。
    pub part_traits: &'static [&'static str],
}

/// Declarative information for an ordinary object or a registry owner.
/// 普通 object 或注册机拥有者的声明式注册信息。
#[derive(Clone, Copy, Debug)]
pub struct RegistrationInfo {
    /// Package namespace that owns this declaration.
    /// 拥有本声明的包命名空间。
    pub namespace: &'static str,
    /// Stable node identity assigned to this face.
    /// 分配给本注册面的稳定节点身份。
    pub id: NodeId,
    /// Node identity of the face this one is registered under.
    /// 本注册面所挂载到的父节点身份。
    pub parent: NodeId,
    /// Face kind name, for example `Button`.
    /// 注册面种类名，例如 `Button`。
    pub kind: &'static str,
    /// Preset type path that constructs this face.
    /// 构造本注册面的 preset 类型路径。
    pub preset: &'static str,
    /// Parts type path that supplies this face's construction parts.
    /// 提供本注册面构造 parts 的 parts 类型路径。
    pub parts: &'static str,
    /// Parameter type path this face declares for its construction input.
    /// 本注册面为其构造输入声明的参数类型路径。
    pub params: &'static str,
    /// Handle type path that exposes this face to its owner.
    /// 向其拥有者暴露本注册面的 handle 类型路径。
    pub handle: &'static str,
    /// Optional author-owned identity that survives source moves.
    /// 可选的作者逻辑身份，可跨源码文件移动保持不变。
    pub stable_name: Option<&'static str>,
    /// Localized display name shown to authors.
    /// 向作者显示的本地化名称。
    pub name: LocalizedText,
    /// Localized one-line description shown alongside the name.
    /// 与名称一同显示的本地化单行描述。
    pub summary: LocalizedText,
    /// Export names this face declares.
    /// 本注册面声明的导出名称。
    pub exports: &'static [&'static str],
    /// Whether this face owns a child Registry.
    /// 本注册面是否拥有一个子注册机。
    pub needs_registry: bool,
    /// Name of that child Registry, used to build its path; ignored otherwise.
    /// 子注册机的名称，用于生成其路径；不需要子注册机时忽略。
    pub registry_name: &'static str,
    /// Name of an external registry this face is provisioned from, if any.
    /// 本注册面从其获取内容的外部注册机名（如果有）。
    pub getting_from_other_registry: Option<&'static str>,
    /// The location a rule for this face **would** be read from: beside the face, at
    /// `registry_rule/registry_rule.rs`, whether or not that file exists. This field names a
    /// position, not the existence of a file, which is why it is filled for a face whose rule
    /// file is absent (audit `t75` §6.1).
    /// 本注册面的规则**会**被读取的位置：与注册面并列的 `registry_rule/registry_rule.rs`，无论
    /// 那里有没有文件。本字段名的是一个位置，而不是某份文件的存在，因此规则文件缺席的注册面同样会
    /// 填上它（审计 `t75` §6.1）。
    pub registry_rule_path: &'static str,
    /// Rule for faces entering the Registry owned by this face.
    /// 该注册面拥有的 Registry 所使用的注册规范。
    pub registry_rule: RegistrationRule,
    /// External dependency gate for this face's registry.
    /// 该注册面所属注册机对外部依赖的门禁。
    pub admission: Admission,
    /// Capability requirements this face declares.
    /// 本注册面声明的能力需求。
    pub requires: &'static [RequirementSpec],
    /// Capability names this face makes available to other faces.
    /// 本注册面向其他注册面提供的能力名称。
    pub provides: &'static [&'static str],
    /// Construction contract comparing required and supplied parts.
    /// 比较所需 parts 与实际提供 parts 的构造合同。
    pub contract: ObjectContract,
    /// Automatically comparable input/output contract for grafting.
    /// 供嫁接自动比较的输入/输出合同。
    pub flow: crate::FlowContract,
    /// Type that supplied the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果有）。
    pub flow_provider: Option<&'static str>,
    /// Interface names declared by the handle type on this registration face.
    /// 此注册面的 handle 类型声明实现的接口名称。
    pub handle_traits: &'static [&'static str],
    /// Interface names declared by the parts type on this registration face.
    /// 此注册面的 parts 类型声明实现的接口名称。
    pub part_traits: &'static [&'static str],
    /// Runtime value checks the host applies to this face; empty accepts any value.
    /// 宿主对本注册面取值执行的运行期校验；为空时接受任何取值。
    pub runtime_checks: &'static [RuntimeCheckSpec],
    /// Optional provenance for a face supplied by an external plugin crate.
    /// 外部插件 crate 提供注册面时，可附带插件来源元数据。
    pub plugin: Option<crate::PluginManifest>,
    /// Declaration site this metadata was captured from.
    /// 捕获这份元数据时所在的声明位置。
    pub source: SourceLocation,
}

impl RegistrationInfo {
    /// Return the opt-in logical identity for this face.
    /// 返回该注册面的可选逻辑稳定身份。
    pub const fn stable_face_id(self) -> StableFaceId {
        let name = match self.stable_name {
            Some(name) => name,
            None => self.kind,
        };
        StableFaceId::from_name(name)
    }

    /// Return the stable identity only when the declaration opted into one.
    /// 只有声明显式选择稳定名称时才返回稳定身份。
    ///
    /// The fallback used by `stable_face_id` is useful for diagnostics, but it
    /// is not a uniqueness promise: several independent faces may all be
    /// named `Button`. Only an explicit name can be checked globally.
    /// `stable_face_id` 的回退值适合诊断，但不代表全局唯一；多个独立注册面
    /// 可以同名为 `Button`。只有显式名称才需要做全局冲突检查。
    pub const fn explicit_stable_face_id(self) -> Option<StableFaceId> {
        match self.stable_name {
            Some(name) => Some(StableFaceId::from_name(name)),
            None => None,
        }
    }

    /// Copy a compiled declaration into an independently owned snapshot.
    /// 将编译期声明复制为独立拥有所有权的快照。
    pub fn into_snapshot(self) -> RegistrationSnapshot {
        RegistrationSnapshot {
            namespace: self.namespace.to_owned(),
            id: self.id,
            parent: self.parent,
            kind: self.kind.to_owned(),
            preset: self.preset.to_owned(),
            parts: self.parts.to_owned(),
            params: self.params.to_owned(),
            handle: self.handle.to_owned(),
            stable_name: self.stable_name.map(str::to_owned),
            name: OwnedLocalizedText {
                zh: self.name.zh.to_owned(),
                en: self.name.en.to_owned(),
            },
            summary: OwnedLocalizedText {
                zh: self.summary.zh.to_owned(),
                en: self.summary.en.to_owned(),
            },
            exports: self
                .exports
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            needs_registry: self.needs_registry,
            registry_name: self.registry_name.to_owned(),
            getting_from_other_registry: self.getting_from_other_registry.map(str::to_owned),
            registry_rule_path: self.registry_rule_path.to_owned(),
            registry_rule: self.registry_rule.into_owned(),
            admission: self.admission.into_owned(),
            requires: self
                .requires
                .iter()
                .map(|requirement| OwnedRequirementSpec {
                    capability: requirement.capability.to_owned(),
                    provider: requirement.provider.to_owned(),
                })
                .collect(),
            provides: self
                .provides
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            contract: self.contract.into_owned(),
            flow: self.flow.into(),
            flow_provider: self.flow_provider.map(str::to_owned),
            handle_traits: self
                .handle_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            part_traits: self
                .part_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            runtime_checks: self.runtime_checks.to_vec(),
            plugin: self.plugin,
            source: OwnedSourceLocation {
                file: self.source.file.to_owned(),
                line: self.source.line,
                column: self.source.column,
                function: self.source.function.to_owned(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime check values and specifications.
// These types are declared here (rather than in the runtime crate) because
// `RegistrationInfo`/`RegistrationSnapshot` carry them; the runtime crate
// re-exports them at its historical paths.
// 以下类型因被 RegistrationInfo/RegistrationSnapshot 持有而定义在 kernel；
// runtime crate 会在原路径上重导出它们。

/// The face fields in the order the authoring macros accept them.
/// 作者侧宏接受的注册面字段顺序。
///
/// This is the one place the vocabulary is ordered. The face macros declare it
/// for an editor, and the macro front end sorts an author's fields into it, so
/// a face may be written in any order and still reach the same declaration.
/// 这是词表顺序的唯一来源。注册面宏据此为编辑器声明字段，宏前端据此把作者写的
/// 字段排成这个顺序——因此注册面可以用任意顺序书写，最终仍落到同一份声明。
pub const FACE_FIELD_ORDER: &[&str] = &[
    // `source` belongs to the external form only (`external_object!`), which
    // names the file explicitly because an external crate is not part of the
    // host's generated tree. It comes first because that matcher expects it
    // right after `collector`.
    // `source` 只属于外部形式（`external_object!`）：外部 crate 不在宿主的生成树里，
    // 因此要显式指出文件。它排在首位，因为那个 matcher 期望它紧跟 `collector`。
    "source",
    "kind",
    "preset",
    "parts",
    "name",
    "summary",
    "exports",
    "stable_name",
    "needs_registry",
    "parent",
    "getting_from_other_registry",
    "registry_rule_path",
    "registry_rule",
    "admission",
    "handle_traits",
    "handle_contracts",
    "part_traits",
    "part_contracts",
    "requires",
    "provides",
    "flow",
    "flow_provider",
    // The spelling lives in `lexicon` because the build step and the macro
    // front end look the field up by name; one text, one definition.
    // 该拼写住在 `lexicon`：构建步骤与宏前端都按名字查找这个字段，一份文本一个定义点。
    crate::registry_core::lexicon::FACE_FIELD_PLUGIN,
    "runtime_checks",
];

#[cfg(test)]
#[path = "registration_tests.rs"]
mod registration_tests;
