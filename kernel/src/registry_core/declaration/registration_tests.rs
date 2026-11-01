//! The author-facing declaration shape (H/A2e) and the associated types that carry its type-like half.
//! 作者形声明（H/A2e），以及承载它类型类那一半的关联类型。

use super::Registration;

/// The author-facing shape is writable **today**, and what it promises about its defaults is pinned:
/// only the fields the author cares about, the rest spread from `DEFAULT`, and the whole thing `const`.
/// 作者形**今天**就能写，而它对默认值的承诺被钉住：只写作者关心的那几个字段、其余从 `DEFAULT` 展开，
/// 而且整份是 `const`。
///
/// The type-like half (`preset`/`parts`/`params`/`handle`) is **not** here yet: as associated types it
/// needs a real implementor, and the only one that counts — the build naming a face's type to read them
/// back — arrives with the generated tree's assembly. A trait nothing implements outside `cfg(test)` is
/// dead code under `-D warnings`, which is the same rule that keeps a field from landing before its
/// consumer.
/// 类型类的那一半（`preset`/`parts`/`params`/`handle`）**还不在**：作为关联类型它需要一个真正的实现者，而唯一
/// 算数的那个——构建点名面的类型把它们读回来——要等生成树的拼装。在 `cfg(test)` 之外无人实现的 trait，在
/// `-D warnings` 下就是死代码；这与"字段不能先于消费者落地"是同一条规矩。
#[test]
fn the_author_facing_shape_is_writable_with_const_defaults() {
    // Only what the author cares about, with the rest spread from `DEFAULT` — and all of it `const`.
    // 只写作者关心的那几个字段，其余从 `DEFAULT` 展开——而且整份都是 `const`。
    const CARD: Registration = Registration {
        needs_registry: true,
        registry_name: "card",
        ..Registration::DEFAULT
    };

    assert_eq!(CARD.registry_name, "card", "the literal's field survives");
    assert_eq!(
        Registration::DEFAULT.registry_name,
        "",
        "and the spread fills the rest with the empty value"
    );
    assert_eq!(
        Registration::DEFAULT.parent,
        crate::registry_core::identity::NodeId::from_raw([0; 16]),
        "including the parent, whose default is the unset identity"
    );
    // Spreading is what carries the *unwritten* fields: compare a written one against the default.
    // 展开正是承载**未写字段**的东西：拿一个写了的字段与默认值比。
    assert_ne!(
        CARD.needs_registry,
        Registration::DEFAULT.needs_registry,
        "the default owns no registry"
    );
}
