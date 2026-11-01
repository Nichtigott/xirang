//! Pins for the host's named registry entry.
//! 宿主注册机具名入口的钉子。

use super::host_registry;
use crate::run_method::registry_core::FrameworkId;

/// The pair the caller passes is the pair the registry answers under.
/// 调用方给的那一对，就是这台注册机作答时用的那一对。
///
/// An empty plan is the one shape this file can build without a host crate (a `RegistrationInfo` is
/// only ever produced by a generated `registrations()`), and it is enough for the half that was
/// actually missing: before this entry existed, there was no *name* to call at all. The other half —
/// a real host reaching its own faces — is exercised end to end by `examples/control-button`, which
/// the workspace gate compiles and tests.
/// 空计划是本文件在没有宿主 crate 的情况下唯一能造的形状（`RegistrationInfo` 只由生成的
/// `registrations()` 产出），而对真正缺掉的那一半来说它够用：在本入口存在之前，根本**没有名字**可调。
/// 另一半——真宿主拿到自己的面——由 `examples/control-button` 端到端跑到，工作区门禁会编译并测试它。
#[test]
fn the_framework_is_the_one_the_host_passed() {
    let framework = FrameworkId::new("xirang.pin.host-registry");
    let registry = host_registry(framework, "pin.namespace", &[]).expect("an empty plan assembles");
    assert_eq!(
        registry.framework(),
        framework,
        "the registry answers under the framework the host declared"
    );
    assert!(
        registry.depth_first().is_empty(),
        "an empty plan leaves the root alone: the root is the tree, not an entry in it"
    );
}
