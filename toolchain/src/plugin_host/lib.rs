//! Isolated execution and atomic deployment for XiRang plugins.
//! XiRang 插件的隔离执行与原子部署。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

mod admission;
mod deployment;
mod error;
#[cfg(feature = "wasm")]
mod lazy_wasm;
#[cfg(feature = "process-tools")]
mod process;
mod verifier;
#[cfg(feature = "wasm")]
mod wasm;

pub use admission::{
    OFFICIAL_LOCK, PLUGIN_LOCK_DIRECTORY, PluginAdmission, USER_LOCK, lane_for, plugin_catalog,
};
pub use deployment::{Deployment, HotDeployment};
pub use error::HostError;
#[cfg(feature = "wasm")]
pub use lazy_wasm::{ValidationChannel, WasmPluginSlot, WasmPluginTable};
#[cfg(feature = "process-tools")]
pub use process::{ProcessBackend, ProcessInstance, ProcessLimits, ProcessProgram};
pub use verifier::{Ed25519Verifier, TrustedPublicKey};
#[cfg(feature = "wasm")]
pub use wasm::{WasmBackend, WasmInstance, WasmLimits};

use crate::run_method::PluginAdapter;

/// One callable plugin implementation.
/// 一个可调用的插件实现。
pub trait PluginInstance: Send + Sync + 'static {
    /// The execution adapter that backs this instance.
    /// 该实例所依托的执行适配器。
    fn adapter(&self) -> PluginAdapter;

    /// Run one operation against the plugin and return its raw response bytes.
    /// The adapter enforces its own limits and reports every failure as `HostError`.
    /// 对插件执行一次操作并返回原始响应字节；适配器自行实施限制，并把所有失败报告为
    /// `HostError`。
    fn call(&self, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError>;

    /// Confirm the instance answers a `health` call with exactly `ok`.
    /// 确认实例对 `health` 调用给出的回答正好是 `ok`。
    fn health_check(&self) -> Result<(), HostError> {
        let response = self.call("health", &[])?;
        if response == b"ok" {
            Ok(())
        } else {
            Err(HostError::Health(format!(
                "expected `ok`, received {:?}",
                String::from_utf8_lossy(&response)
            )))
        }
    }
}
