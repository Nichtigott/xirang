//! Conversion from authored fields to a registration snapshot.
//! 将创作字段转换成注册快照。
//!
//! The pure conversion lives in the kernel `authoring` module; this shim
//! keeps the `FaceManifest::to_snapshot` method and injects the
//! process-bound namespace fallback.
//! 纯转换位于 kernel 的 `authoring` 模块；本 shim 保留
//! `FaceManifest::to_snapshot` 方法，并注入绑定进程状态的命名空间回落值。

use super::context::authoring_namespace;
use super::manifest::FaceManifest;
use crate::run_method::RegistrationSnapshot;

impl FaceManifest {
    pub(crate) fn to_snapshot(&self) -> Result<RegistrationSnapshot, String> {
        // No injection of a derived `handle` here: the kernel's conversion owns
        // that derivation and uses it for **both** the `handle` field and
        // `source.function` (`core/.../authoring/snapshot.rs`), so an
        // authored face that stores no `handle` still carries its function name.
        // A patch here that supplied the key to the kernel was how the two sides
        // drifted apart in the first place (audit `N-4`); one derivation, in the
        // kernel, is what the pin below checks.
        // 这里不再注入派生的 `handle`：内核的转换自己拥有那份推导，并把它同时用于 `handle`
        // 字段与 `source.function`（`core/.../authoring/snapshot.rs`），因此不存
        // `handle` 的创作面仍带着自己的函数名。过去在这里给内核补上该键，正是两侧漂移的来路
        // （审计 `N-4`）；推导只有一份、在内核，下面的钉子钉的就是这件事。
        xirang_kernel::authoring::snapshot::snapshot_from_values(
            &self.values,
            &authoring_namespace(),
        )
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
