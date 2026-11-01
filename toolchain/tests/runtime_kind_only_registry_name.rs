//! The kind-only face shape and the `registry_name` it derives.
//! 只有 kind 的注册面形态，以及它推导出的 `registry_name`。

/// The identity namespace this test crate's faces are compiled under.
/// 本测试 crate 的注册面编译时所用的身份命名空间。
///
/// A crate that declares faces without `host!()` owns this constant itself — the declaration macros
/// read it (audit `M7`, P3.3) — so a test crate provides the same one-line value a host gets from
/// `host!()`. It lives at this file's top level because the invocations sit inside `mod` blocks and
/// `crate::` in their expansion resolves to the crate root.
/// 一个声明注册面却不调用 `host!()` 的 crate 自己拥有这个常量——声明宏读它（审计 `M7`，P3.3）——因此测试
/// crate 要提供宿主从 `host!()` 得到的那一行同样的值。它放在本文件的顶层，因为那些调用位于 `mod` 块里，
/// 而展开里的 `crate::` 解析到 crate 根。
pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");

/// A face declared with `collector` and `kind` and nothing else: no `handle`,
/// no explicit `registry_name`, no `needs_registry`.
/// 只写 `collector` 与 `kind`、别无其它字段的注册面：没有 `handle`，没有显式
/// `registry_name`，也没有 `needs_registry`。
mod kind_only {
    // type named in `kind:`/`preset:`/`parts:` must be public too: private in a public interface
    // is `E0446`.
    // `parts:` 点名的类型也必须公开——公开接口里的私有类型是 `E0446`。

    xirang_toolchain::__control_object! {
        collector: development,
        kind: KindOnlyFace,
    }
}

/// `{ collector, kind }` is the smallest reachable face spelling. B1 deleted the
/// arm that used to catch this shape, so it now falls through to the arm that
/// defaults every post-`kind` field; this test pins the `registry_name` that
/// surviving path produces. It is derived from the declaration's own module path
/// (`last_path_segment(module_path!())`), so it is the *module* name here, not
/// the kind. That is the pre-1.0 behaviour and is intentionally unchanged: the
/// point of the test is equivalence, so if a later batch makes the value
/// kind-derived, this assertion is the one that must be updated deliberately.
/// `{ collector, kind }` 是可达的最小注册面写法。B1 删除了过去接住这一形态的 arm，
/// 它现在落到把 `kind` 之后每个字段取默认值的那个 arm；本测试钉住这条存活路径产出的
/// `registry_name`。它由声明自身所在的模块路径推导
/// （`last_path_segment(module_path!())`），因此这里是**模块**名而不是 kind。这是
/// 1.0 之前的行为，有意保持不变：本测试的目的就是等价性，若后续批次改成由 kind 推导，
/// 需要显式修改的正是这条断言。
#[test]
fn a_kind_only_face_names_its_registry_after_its_module() {
    assert_eq!(kind_only::REGISTRATION.kind, "KindOnlyFace");
    assert_eq!(kind_only::REGISTRATION.registry_name, "kind_only");
    assert_ne!(
        kind_only::REGISTRATION.registry_name,
        kind_only::REGISTRATION.kind,
        "the derived name is the module segment, not the kind"
    );
}
