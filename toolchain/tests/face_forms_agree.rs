//! Both spellings of one declaration must produce **one** `RegistrationInfo` (H/A2e's acceptance).
//! 同一份声明的两种拼法必须产出**同一份** `RegistrationInfo`（H/A2e 的验收）。

/// The namespace constant every declaration macro reads.
/// 每个声明宏都会读的命名空间常量。
pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");

/// Declared with the field macro: the spelling every existing face uses today.
/// 用字段宏声明：今天每个面都在用的拼法。
mod by_macro {
    xirang_toolchain::run_method::external_object! {
        kind: Pair,
        preset: PairPreset,
        parts: PairParts,
        name: { zh: "配对", en: "Pair" },
        summary: { zh: "两边必须一致", en: "Both sides must agree" },
        exports: ["pair"],
        stable_name: "pair",
        needs_registry: true,
        parent: xirang_toolchain::run_method::registry_core::root_node_id("forms"),
    }

    /// The kind the declaration above names.
    /// 上面那条声明点名的 kind。
    pub struct Pair;

    /// A preset whose record is distinguishable from the defaults.
    /// 记录可与默认值区分的 preset。
    pub struct PairPreset;

    impl xirang_toolchain::run_method::PresetContract for PairPreset {
        type Output = ();
        const REQUIRED_PARTS: &'static [&'static str] = &["pair-left"];
    }

    /// The matching parts type.
    /// 与之相配的 parts 类型。
    pub struct PairParts;

    impl xirang_toolchain::run_method::PartsContract for PairParts {
        type Output = ();
        const PROVIDED_PARTS: &'static [&'static str] = &["pair-right"];
    }
}

/// The same declaration assembled from a hand-written `Registration` — the half the literal spelling
/// supplies — plus the arguments only the build knows.
/// 同一份声明，改由手写的 `Registration`（字面量拼法提供的那一半）加上只有构建才知道的参数拼出。
mod by_assembly {
    use super::by_macro::{PairParts, PairPreset};
    use super::*;
    use xirang_toolchain::run_method::registry_core::{NodeId, RegistrationRule};
    use xirang_toolchain::run_method::{
        FaceInputs, LocalizedText, Registration, RegistrationInfo, SourceLocation,
    };

    /// The declaration, written the way a literal-form face writes it.
    /// 这份声明，按字面量形的写法。
    pub const DECLARATION: Registration = Registration {
        parent: xirang_toolchain::run_method::registry_core::root_node_id("forms"),
        stable_name: Some("pair"),
        name: LocalizedText {
            zh: "配对",
            en: "Pair",
        },
        summary: LocalizedText {
            zh: "两边必须一致",
            en: "Both sides must agree",
        },
        exports: &["pair"],
        needs_registry: true,
        // The build's side of the contract: the macro derives this from `module_path!()`, so a
        // literal-form face receives the same value from whoever renders it.
        // 构建那一侧的输入：宏从 `module_path!()` 推导它，因此字面量形的面由渲染它的那一方收到同一个值。
        registry_name: "by_macro",
        registry_rule_path: "toolchain/tests/face_forms_agree.rs",
        registry_rule: RegistrationRule::ANY,
        ..Registration::DEFAULT
    };

    /// The assembled snapshot, given exactly what the macro derives for the same declaration.
    /// 拼装出的快照，喂给它的正是宏为同一份声明推导出的那些东西。
    pub const ASSEMBLED: RegistrationInfo = RegistrationInfo::assemble::<PairPreset, PairParts>(
        FaceInputs {
            namespace: XIRANG_NAMESPACE,
            id: NodeId::from_namespaced_path(XIRANG_NAMESPACE, file!(), "Pair"),
            kind: "Pair",
            registry_rule: xirang_toolchain::RegistrationRule::ANY,
            // The name a face owns among its siblings, which the macros derive as the module's last
            // segment; spelled here so the two forms are compared field for field.
            // 面在兄弟之间拥有的名字，宏按模块末段推导；这里写出来，两形才能逐字段对比。
            registry_name: "pair",
            preset: "PairPreset",
            parts: "PairParts",
            params: "Pair",
            handle: "Pair",
            // `file!()` here is this file, exactly what the declaration macro records for a face written
            // in it; the line is the caller's, which is why only the file is asserted below.
            // 这里的 `file!()` 就是这个文件，与声明宏为写在其中的面所记录的完全一致；行号由调用方给，
            // 因此下面只断言文件。
            source: SourceLocation {
                file: file!(),
                line: 0,
                column: 0,
                function: "Pair",
            },
            handle_traits: &[],
            part_traits: &[],
        },
        DECLARATION,
    );
}

/// Field by field, because the snapshot is not `PartialEq` and a silent difference here is exactly the
/// kind that would surface later as "one spelling ships a different face".
/// 逐字段比：快照没有实现 `PartialEq`，而这里静默的差别正是那种"某一种拼法发布了另一个面"的形态。
#[test]
fn both_spellings_produce_one_registration_info() {
    // Naming the kind is what the declaration macro's text does; here it keeps the type alive.
    // 点名那个 kind 正是声明宏的文本所做的事；这里它让那个类型不至于成为死代码。
    let _ = by_macro::Pair;

    let macro_side = by_macro::REGISTRATION;
    let assembled = by_assembly::ASSEMBLED;

    assert_eq!(macro_side.namespace, assembled.namespace);
    assert_eq!(
        macro_side.kind, assembled.kind,
        "kind comes from the type name"
    );
    assert_eq!(macro_side.preset, assembled.preset);
    assert_eq!(macro_side.parts, assembled.parts);
    assert_eq!(macro_side.params, assembled.params);
    assert_eq!(macro_side.handle, assembled.handle);
    assert_eq!(
        macro_side.parent, assembled.parent,
        "the declaration's parent wins"
    );
    assert_eq!(macro_side.stable_name, assembled.stable_name);
    assert_eq!(macro_side.name, assembled.name);
    assert_eq!(macro_side.summary, assembled.summary);
    assert_eq!(macro_side.exports, assembled.exports);
    assert_eq!(macro_side.needs_registry, assembled.needs_registry);
    assert_eq!(macro_side.registry_name, assembled.registry_name);
    assert_eq!(
        macro_side.getting_from_other_registry,
        assembled.getting_from_other_registry
    );
    assert_eq!(macro_side.registry_rule_path, assembled.registry_rule_path);
    assert_eq!(macro_side.admission, assembled.admission);
    assert_eq!(macro_side.requires, assembled.requires);
    assert_eq!(macro_side.provides, assembled.provides);
    assert_eq!(
        macro_side.contract.required_parts, assembled.contract.required_parts,
        "the preset's record is read off the type, not written"
    );
    assert_eq!(
        macro_side.contract.provided_parts, assembled.contract.provided_parts,
        "and the parts type's record likewise"
    );
    assert_eq!(macro_side.flow, assembled.flow);
    assert_eq!(macro_side.flow_provider, assembled.flow_provider);
    assert_eq!(macro_side.handle_traits, assembled.handle_traits);
    assert_eq!(macro_side.part_traits, assembled.part_traits);
    assert_eq!(macro_side.runtime_checks, assembled.runtime_checks);
    assert_eq!(macro_side.plugin, assembled.plugin);
    assert_eq!(
        macro_side.source.file, assembled.source.file,
        "both spellings record the same file; the line is the caller's to supply"
    );
    assert_eq!(
        macro_side.id, assembled.id,
        "one identity, whichever spelling wrote the declaration"
    );
}

/// The labels a literal-form face carries come off **the face**, not from its consumer. The default is
/// "no contracts", and a face that has them says so once, on its own `FaceContract` impl.
/// 字面量形携带的标签来自**那个面**，而不是它的消费者。"没有合同"是默认值，而有合同的面对着它自己的
/// `FaceContract` 实现说一次。
#[test]
fn a_literal_form_face_owns_its_contract_labels() {
    use xirang_toolchain::run_method::registry_core::{NoParts, NoPreset, NodeId};
    use xirang_toolchain::run_method::{
        FaceContract, FaceInputs, Registration, RegistrationInfo, SourceLocation,
    };

    struct Labeled;

    impl FaceContract for Labeled {
        type Preset = NoPreset;
        type Parts = NoParts;
        const HANDLE_TRAITS: &'static [&'static str] = &["GenericContract"];
        const PART_TRAITS: &'static [&'static str] = &["PartsContract"];
    }

    const INFO: RegistrationInfo = RegistrationInfo::assemble_face::<Labeled>(
        FaceInputs {
            namespace: "labels",
            id: NodeId::from_raw([7; 16]),
            kind: "Labeled",
            registry_rule: xirang_toolchain::RegistrationRule::ANY,
            registry_name: "labeled",
            preset: "",
            parts: "",
            params: "Labeled",
            handle: "Labeled",
            source: SourceLocation {
                file: file!(),
                line: 0,
                column: 0,
                function: "Labeled",
            },
            // Deliberately *not* the face's own labels: the assembly must read the face's, so a consumer
            // cannot quietly hand in its own idea of what this face claims.
            // 有意**不是**这个面自己的标签：拼装必须读这个面的，消费者不能悄悄塞进它自己以为的声明。
            handle_traits: &["consumer-invented"],
            part_traits: &["consumer-invented"],
        },
        Registration::DEFAULT,
    );

    assert_eq!(
        INFO.handle_traits,
        &["GenericContract"],
        "the face's own handle labels win"
    );
    assert_eq!(
        INFO.part_traits,
        &["PartsContract"],
        "and so do its part labels"
    );
}

/// The check a hand-written face makes about its own contracts. A trait the face implements passes; the
/// doctest on `assert_contracts!` covers the other direction, and this side is what makes the two
/// statements of one fact — the labels on `FaceContract` and the traits the macro names — live together.
/// 手写的面对自己那些合同所做的检查。面实现了的 trait 通过；另一方向由 `assert_contracts!` 上的 doctest
/// 覆盖，而这一侧让"同一件事的两种陈述"——`FaceContract` 上的标签与宏点名的 trait——住在一起。
#[test]
fn a_hand_written_face_checks_its_own_contracts() {
    use xirang_toolchain::run_method::FaceContract;
    use xirang_toolchain::run_method::registry_core::{NoParts, NoPreset};

    trait ControlHandle {
        fn label() -> &'static str;
    }

    struct Card;

    impl ControlHandle for Card {
        fn label() -> &'static str {
            "ControlHandle"
        }
    }

    impl FaceContract for Card {
        type Preset = NoPreset;
        type Parts = NoParts;
        const HANDLE_TRAITS: &'static [&'static str] = &["ControlHandle"];
    }

    // The check the new spelling writes beside the face; it compiles only because the impl above exists.
    // 新拼法写在面旁边的检查；它编译得过，只因为上面那个 impl 存在。
    xirang_toolchain::assert_contracts!(Card: ControlHandle);

    assert_eq!(Card::label(), "ControlHandle");
    assert_eq!(
        <Card as FaceContract>::HANDLE_TRAITS,
        &["ControlHandle"],
        "the label and the checked trait are two statements of one fact"
    );
}
