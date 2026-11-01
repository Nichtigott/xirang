//! A face declared without a `handle:` must keep the preset and parts its
//! author wrote.
//! 未写 `handle:` 的注册面必须保留作者写下的 preset 与 parts。

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

/// A custom preset and parts pair, distinct from the permissive defaults so the
/// assertion can tell "forwarded" from "silently replaced".
/// 一对自定义 preset/parts，刻意不同于宽松默认值，断言才能区分"被转发"与"被静默替换"。
struct ProbePreset;
struct ProbeParts;
impl xirang_toolchain::run_method::PresetContract for ProbePreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &["probe"];
}

impl xirang_toolchain::run_method::PartsContract for ProbeParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &["probe"];
}

/// No `handle:` field: this is the arm that defaults the handle to `kind`.
/// 没有 `handle:` 字段：这正是把 handle 默认为 `kind` 的那个 arm。
mod probe {

    use crate::{ProbeParts, ProbePreset};

    xirang_toolchain::__control_object! {
        collector: development,
        kind: ProbeFace,
        preset: ProbePreset,
        parts: ProbeParts,
        name: { zh: "探针面", en: "Probe face" },
    }
}

/// The record must name the author's types, not the macro's defaults.
/// 记录必须写出作者的类型，而不是宏的默认值。
#[test]
fn a_custom_preset_and_parts_survive_the_defaulting_arm() {
    assert_eq!(probe::REGISTRATION.preset, "ProbePreset");
    assert_eq!(probe::REGISTRATION.parts, "ProbeParts");
    // The contract reads the same types, so this is not just a label.
    // 合同读的是同一批类型，因此这不只是标签。
    assert_eq!(probe::REGISTRATION.contract.required_parts, ["probe"]);
    assert_eq!(probe::REGISTRATION.contract.provided_parts, ["probe"]);
}
