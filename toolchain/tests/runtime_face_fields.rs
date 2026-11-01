//! A hand-written face may order its fields freely and separate them with `,`
//! or `;`.
//! 手写注册面可以自由排列字段，并用 `,` 或 `;` 分隔。

/// The identity namespace this test crate's faces are compiled under.
/// 本测试 crate 的注册面编译时所用的身份命名空间。
///
/// A crate that declares faces without `host!()` owns this constant itself: the declaration macros
/// read it (audit `M7`, P3.3), so the test crate has to provide the same one-line value a host gets
/// from `host!()`.
/// 一个声明注册面却不调用 `host!()` 的 crate 自己拥有这个常量：声明宏读它（审计 `M7`，P3.3），因此测试
/// crate 必须提供宿主从 `host!()` 得到的那一行同样的值。
pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");

mod canonical {

    xirang_toolchain::__xirang_object! {
        kind: Ordered,
        name: { zh: "有序", en: "Ordered" },
        needs_registry: true,
        parent: xirang_toolchain::run_method::registry_core::root_node_id("face-fields-test"),
        registry_rule: xirang_toolchain::run_method::registry_core::RegistrationRule::ANY,
    }
}

mod shuffled {

    xirang_toolchain::__xirang_object! {
        registry_rule: xirang_toolchain::run_method::registry_core::RegistrationRule::ANY;
        kind: Ordered;
        parent: xirang_toolchain::run_method::registry_core::root_node_id("face-fields-test");
        needs_registry: true;
        name: { zh: "有序", en: "Ordered" };
    }
}

/// Both declarations name the same face, so both must produce the same
/// identity: `source` comes from `file!()` and both live in this file.
/// 两份声明指的是同一个注册面，因此身份必须一致：`source` 来自 `file!()`，而两者
/// 都在本文件里。
#[test]
fn any_order_and_separator_declares_the_same_face() {
    assert_eq!(canonical::NODE_ID, shuffled::NODE_ID);
    assert_eq!(
        canonical::REGISTRATION.needs_registry,
        shuffled::REGISTRATION.needs_registry
    );
    assert_eq!(
        canonical::REGISTRATION.parent,
        shuffled::REGISTRATION.parent
    );
    // The slot now follows the declaring module, so two spellings in two modules
    // agree on everything *except* the slot they derive from where they live;
    // for a generated face that module is the file, which is why the author no
    // longer states it.
    // 槽位现在跟随声明所在的模块，因此两种写法在两个模块里除了"各自从所在位置推导出的
    // 槽位"之外全都一致；对生成注册面而言那个模块就是文件本身，这也是作者不再声明它的原因。
    assert_eq!(canonical::REGISTRATION.registry_name, "canonical");
    assert_eq!(shuffled::REGISTRATION.registry_name, "shuffled");
    assert_eq!(
        canonical::REGISTRATION.name.zh,
        shuffled::REGISTRATION.name.zh
    );
    assert_eq!(
        canonical::REGISTRATION.registry_rule_path,
        shuffled::REGISTRATION.registry_rule_path
    );
}

/// Observation evidence must still point at the author's own lines, so the
/// front end must not move `line!()`/`column!()` into the generated alias. The
/// shuffled declaration stands later in this file, and that is what it reports.
/// 观测证据仍须指向作者自己的行，因此前端不能把 `line!()`/`column!()` 挪到生成的
/// 别名里。乱序声明在本文件中位置更靠后，它报出的就是那一行。
#[test]
fn a_reordered_face_reports_the_authors_lines() {
    assert_eq!(
        canonical::REGISTRATION.source.file,
        shuffled::REGISTRATION.source.file
    );
    // Read the lines through a collection so the comparison is made on values
    // rather than on two constants, which a lint would fold away.
    // 通过集合读取行号，让比较发生在值上而不是两个常量上——否则会被 lint 折叠掉。
    let lines = [
        canonical::REGISTRATION.source.line,
        shuffled::REGISTRATION.source.line,
    ];
    assert_ne!(
        lines[0], lines[1],
        "each declaration reports its own line: {lines:?}"
    );
    assert!(lines.iter().all(|line| *line > 0), "lines: {lines:?}");
    assert_eq!(canonical::REGISTRATION.source.column, 5);
    assert_eq!(shuffled::REGISTRATION.source.column, 5);
}

/// An editor that inserts a macro call writes `name!(…)`, so the parenthesised
/// form has to declare exactly the same face as the braced one — and it must
/// accept `;` separators and any order just as well.
///
/// One `;` after the closing bracket is rustc's rule, not the front end's: a
/// macro invocation in item position is only an item when it is brace-delimited,
/// so `name!(…)` and `name![…]` must be followed by `;`. The alias matcher
/// itself accepts any delimiter.
/// 编辑器插入宏调用时写的是 `name!(…)`，因此括号形式必须声明与花括号形式完全相同的
/// 注册面，并且同样接受 `;` 分隔与任意顺序。
///
/// 收尾的那一个 `;` 是 rustc 的规则、不是前端的：位于 item 位置的宏调用只有在花括号
/// 分隔时才算 item，所以 `name!(…)` 与 `name![…]` 后面必须跟 `;`。别名匹配器本身
/// 接受任何分隔符。
mod parens {

    xirang_toolchain::__xirang_object!(
        kind: Ordered;
        registry_rule: xirang_toolchain::run_method::registry_core::RegistrationRule::ANY;
        needs_registry: true;
        name: { zh: "有序", en: "Ordered" };
        parent: xirang_toolchain::run_method::registry_core::root_node_id("face-fields-test")
    );
}

/// Parens and braces are the same token tree, so they must declare one face.
/// 括号与花括号是同一个 token 树，因此必须声明同一个注册面。
#[test]
fn the_parenthesised_form_declares_the_same_face() {
    assert_eq!(canonical::NODE_ID, parens::NODE_ID);
    assert_eq!(canonical::REGISTRATION.kind, parens::REGISTRATION.kind);
    assert_eq!(
        canonical::REGISTRATION.needs_registry,
        parens::REGISTRATION.needs_registry
    );
    assert_eq!(canonical::REGISTRATION.registry_name, "canonical");
    assert_eq!(parens::REGISTRATION.registry_name, "parens");
    assert_eq!(
        canonical::REGISTRATION.name.en,
        parens::REGISTRATION.name.en
    );
    assert_eq!(canonical::REGISTRATION.parent, parens::REGISTRATION.parent);
    // The declaration still reports the author's own line and column.
    // 声明仍然报告作者自己的行与列。
    let columns = [
        canonical::REGISTRATION.source.column,
        parens::REGISTRATION.source.column,
    ];
    assert_eq!(columns, [5, 5]);
    let lines = [
        canonical::REGISTRATION.source.line,
        parens::REGISTRATION.source.line,
    ];
    assert!(lines.iter().all(|line| *line > 0), "lines: {lines:?}");
}

/// The canonical rule this test file keeps for its own module, exactly where the
/// authoring layout puts it: beside the face, in `registry_rule`.
/// 本测试文件为自己模块保留的规范规则，位置正是创作布局规定的地方：注册面旁边的
/// `registry_rule`。
mod registry_rule {
    use xirang_toolchain::run_method::registry_core::RegistrationRule;

    pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
        .require_exports(&["control.render"])
        .require_handle_traits(&["ControlHandle"]);
}

/// A directory face that owns a registry may omit `registry_rule:` outright: it
/// then takes the canonical sibling rule rather than the permissive default.
/// 拥有注册机的目录面可以直接省略 `registry_rule:`：此时取同目录的规范规则，
/// 而不是宽松默认值。
mod omitted_rule {

    xirang_toolchain::__xirang_object! {
        kind: RuleOmitted,
        name: { zh: "省略规则", en: "Rule omitted" },
        needs_registry: true,
        parent: xirang_toolchain::run_method::registry_core::root_node_id("face-fields-test"),
    }
}

/// A face that owns no registry keeps the permissive default when it omits the
/// field, because the rule that governs it belongs to its parent.
/// 不拥有注册机的面省略该字段时保留宽松默认值：管它的规则属于它的父级。
mod leaf_no_rule {

    xirang_toolchain::__xirang_object! {
        kind: LeafNoRule,
    }
}

/// Omitting the field is not the same as writing `ANY`: a registry-owning face
/// takes the rule beside it.
/// 省略字段不等于写 `ANY`：拥有注册机的面取它旁边那份规则。
#[test]
fn an_omitted_rule_resolves_to_the_canonical_sibling() {
    let derived = omitted_rule::REGISTRATION.registry_rule;
    let canonical = registry_rule::REGISTRATION_RULE;
    assert_eq!(derived.required_exports, canonical.required_exports);
    assert_eq!(
        derived.required_handle_traits,
        canonical.required_handle_traits
    );
    assert_eq!(derived.required_exports, ["control.render"]);
}

/// A face that owns no registry keeps `ANY` when it omits the field.
/// 不拥有注册机的面省略该字段时保留 `ANY`。
#[test]
fn a_face_without_a_registry_keeps_the_permissive_default() {
    let rule = leaf_no_rule::REGISTRATION.registry_rule;
    assert!(rule.required_exports.is_empty());
    assert!(rule.required_handle_traits.is_empty());
    assert_eq!(rule.required_preset, None);
}

mod external_shuffled {

    use xirang_toolchain::run_method::registry_core::{
        ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
    };

    // Same field set as any external face, but `;`-separated and in a different
    // order: the front end must sort it and send it to `__external_object!`.
    // 与任何外部面相同的字段集合，但用 `;` 分隔且顺序不同：前端必须把它排序后送到
    // `__external_object!`。
    xirang_toolchain::run_method::external_object! {
        kind: ExternalFast;
        flow: FlowContract::new(ContractId::new("t.v1"), 1, "In", "Out");
        source: "face_fields/external_fast.rs";
        parts: NoParts;
        preset: NoPreset;
        name: { zh: "外部", en: "External" };
        summary: { zh: "外部实现", en: "External implementation" };
        exports: ["t.out"];
        needs_registry: false;
        parent: root_node_id(env!("CARGO_PKG_NAME"));
        getting_from_other_registry: None;
        registry_rule_path: "face_fields/external_fast.rs";
        registry_rule: RegistrationRule::ANY;
        requires: [];
        provides: [];
        runtime_checks: [];
    }
}

/// An external face is written by hand too, so it gets the same tolerance.
/// 外部面同样是手写的，因此享有同样的宽容。
#[test]
fn an_external_face_accepts_semicolons_and_any_order() {
    assert_eq!(external_shuffled::REGISTRATION.kind, "ExternalFast");
    assert_eq!(external_shuffled::REGISTRATION.name.en, "External");
    let registered = [external_shuffled::REGISTRATION.needs_registry];
    assert_eq!(registered, [false]);
    assert_eq!(
        external_shuffled::REGISTRATION.source.file,
        "face_fields/external_fast.rs"
    );
}
