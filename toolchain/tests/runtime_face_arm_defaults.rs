//! Every shape of `preset:`/`parts:` must record the same defaults: neither
//! binding, either one alone, or both. Since `handle` is derived from `kind`,
//! the shape that used to distinguish the arms no longer exists, and these cases
//! now pin the one arm's defaulting instead.
//! `preset:`/`parts:` 的每一种形态都必须记录相同的默认值：都不写、只写其一、或都写。
//! 由于 `handle` 由 `kind` 派生，过去用来区分 arm 的形态已不存在，这些用例现在钉住
//! 那唯一一条 arm 的默认行为。

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

/// A custom preset whose recorded name is distinguishable from the default.
/// 一个自定义 preset，其记录名可与默认值区分。
struct ProbePreset;
struct ProbeParts;
impl xirang_toolchain::run_method::PresetContract for ProbePreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &["probe"];
}

/// A custom parts whose recorded name is distinguishable from the default.
/// 一个自定义 parts，其记录名可与默认值区分。
impl xirang_toolchain::run_method::PartsContract for ProbeParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &["probe"];
}

/// No `preset:` and no `parts:`. This is the shape all three example faces take,
/// and the one that used to record `"$crate :: NoPreset"` and
/// `"$crate :: NoParts"`: the defaulting arm re-dispatched those literal tokens
/// and the arm that received them `stringify!`-ed the tokens instead of naming
/// the default.
/// 既没写 `preset:` 也没写 `parts:`。三个例子面都是这个形态，也是
/// 过去记录成 `"$crate :: NoPreset"` 与 `"$crate :: NoParts"` 的形态：取默认的 arm
/// 把这两个字面 token 回派出去，接住它们的 arm 对 token 做了 `stringify!`，而不是命名
/// 默认值。
mod handle_without_preset_or_parts {

    xirang_toolchain::__control_object! {
        collector: development,
        kind: HandleDefaults,
    }
}

/// The no-`handle` counterpart of the shape above. Both arms must land on the
/// same default names, so the test compares them instead of trusting each in
/// isolation.
/// 上面形态的无 `handle` 版本。两个 arm 必须落到同一组默认名字上，因此测试直接比较
/// 二者，而不是各自单独相信。
mod without_handle_defaults {

    xirang_toolchain::__control_object! {
        collector: development,
        kind: NoHandleDefaults,
    }
}

/// `preset:` only: the written name survives and `parts` falls
/// back to the default.
/// 只写 `preset:`：写下的名字保留，`parts` 回退到默认值。
mod handle_with_preset_only {

    use crate::ProbePreset;

    xirang_toolchain::__control_object! {
        collector: development,
        kind: HandlePresetOnly,
        preset: ProbePreset,
    }
}

/// `parts:` only, the mirror of the shape above.
/// 只写 `parts:`，是上一形态的镜像。
mod handle_with_parts_only {

    use crate::ProbeParts;

    xirang_toolchain::__control_object! {
        collector: development,
        kind: HandlePartsOnly,
        parts: ProbeParts,
    }
}

/// Both bindings written.
/// 两个绑定都写下。
mod handle_with_preset_and_parts {

    use crate::{ProbeParts, ProbePreset};

    xirang_toolchain::__control_object! {
        collector: development,
        kind: HandleBoth,
        preset: ProbePreset,
        parts: ProbeParts,
    }
}

/// F1 regression pin: an omitted binding is named `NoPreset`/`NoParts`, never
/// the expanded tokens the default type is spelled with.
/// F1 回归钉：省略的绑定命名为 `NoPreset`/`NoParts`，绝不是默认类型写出来时那串
/// 展开后的 token。
#[test]
fn a_handle_face_records_the_plain_default_names() {
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.preset,
        "NoPreset"
    );
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.parts,
        "NoParts"
    );
    // Both handle arms must agree with the no-handle arm on the default names:
    // that equivalence, not either value alone, is the regression.
    // 两个带 handle 的 arm 都必须在默认名字上与无 handle 的 arm 一致：这条等价关系——
    // 而不是任一单独取值——才是回归点。
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.preset,
        without_handle_defaults::REGISTRATION.preset
    );
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.parts,
        without_handle_defaults::REGISTRATION.parts
    );
    assert_eq!(without_handle_defaults::REGISTRATION.preset, "NoPreset");
    assert_eq!(without_handle_defaults::REGISTRATION.parts, "NoParts");
    // The contract must read the default types too, so the fix is not only a
    // label change.
    // 合同也必须读取默认类型，因此这处修复不只是标签变化。
    assert!(
        handle_without_preset_or_parts::REGISTRATION
            .contract
            .required_parts
            .is_empty()
    );
    assert!(
        handle_without_preset_or_parts::REGISTRATION
            .contract
            .provided_parts
            .is_empty()
    );
}

/// F4: either binding may be written alone beside a custom `handle`; the
/// omitted one still takes the default.
/// F4：两个绑定都可以在自定义 `handle` 旁单独书写；省略的那个仍然取默认值。
#[test]
fn either_binding_may_be_written_alone_beside_a_handle() {
    assert_eq!(handle_with_preset_only::REGISTRATION.preset, "ProbePreset");
    assert_eq!(handle_with_preset_only::REGISTRATION.parts, "NoParts");
    assert_eq!(handle_with_parts_only::REGISTRATION.preset, "NoPreset");
    assert_eq!(handle_with_parts_only::REGISTRATION.parts, "ProbeParts");
}

/// Both bindings written beside a `handle`: the explicit names and their
/// contracts survive.
/// 在 `handle` 旁把两个绑定都写下：显式名字及其合同都保留。
#[test]
fn both_bindings_written_beside_a_handle_survive() {
    assert_eq!(
        handle_with_preset_and_parts::REGISTRATION.preset,
        "ProbePreset"
    );
    assert_eq!(
        handle_with_preset_and_parts::REGISTRATION.parts,
        "ProbeParts"
    );
    assert_eq!(
        handle_with_preset_and_parts::REGISTRATION
            .contract
            .required_parts,
        ["probe"]
    );
}
