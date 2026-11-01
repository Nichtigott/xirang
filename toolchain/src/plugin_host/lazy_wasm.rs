//! Lazy, slot-scoped activation of verified Wasm plugins.
//! 已验证 Wasm 插件的懒加载、按槽激活。

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::run_method::{FlowContract, FrameworkId, PluginMode, VerifiedPluginArtifact};
use arc_swap::ArcSwapOption;

use crate::plugin_host::{HostError, PluginInstance, WasmBackend};

#[path = "lazy_wasm/slot_state.rs"]
mod slot_state;
use slot_state::SlotState;

/// Trust lane enabled for one runtime plugin slot.
/// 运行时插件槽允许使用的信任通道。
///
/// The definition lives in the kernel `plugin` module; this alias keeps the
/// historical `crate::plugin_host::ValidationChannel` path.
/// 定义本体在 kernel 的 `plugin` 模块；本别名保留
/// `crate::plugin_host::ValidationChannel` 历史路径。
pub use crate::run_method::PluginChannel as ValidationChannel;

/// A release-time opening for one Wasm extension or replacement.
/// 正式发布时保留的一个 Wasm 扩展或替换入口。
#[derive(Clone, Copy, Debug)]
pub struct WasmPluginSlot {
    /// Slot name, unique within one table and used to address installs and calls.
    /// 槽名，在单个表内唯一，用于寻址安装与调用。
    pub name: &'static str,
    /// Framework whose artifacts this slot admits.
    /// 该槽接纳的 framework。
    pub framework: FrameworkId,
    /// Whether the plugin extends or replaces the framework.
    /// 插件是扩展还是替换该 framework。
    pub mode: PluginMode,
    /// Flow contract the artifact must match before activation.
    /// 激活前工件必须匹配的数据流合同。
    pub contract: FlowContract,
    /// Trust lanes allowed to install into this slot; must be non-empty.
    /// 允许安装进该槽的信任通道；不得为空。
    pub channels: &'static [ValidationChannel],
}

impl WasmPluginSlot {
    /// Build a slot definition; `const` so tables can live in static storage.
    /// 构造槽定义；为 `const`，便于把表放在静态存储中。
    pub const fn new(
        name: &'static str,
        framework: FrameworkId,
        mode: PluginMode,
        contract: FlowContract,
        channels: &'static [ValidationChannel],
    ) -> Self {
        Self {
            name,
            framework,
            mode,
            contract,
            channels,
        }
    }
}

/// Lazily activates verified Wasm plugins in explicitly retained slots.
/// 仅在显式保留的槽中懒加载已验证 Wasm 插件。
pub struct WasmPluginTable {
    backend: WasmBackend,
    slots: BTreeMap<&'static str, SlotState>,
}

impl WasmPluginTable {
    /// Build a table over the default backend, rejecting malformed slot definitions.
    /// 在默认后端上建表，并拒绝非法的槽定义。
    pub fn new(slots: &'static [WasmPluginSlot]) -> Result<Self, HostError> {
        Self::with_backend(slots, WasmBackend::default())
    }

    /// Build a table with an explicit backend so callers can supply their own limits.
    /// 用显式后端建表，便于调用方提供自己的限制。
    pub fn with_backend(
        slots: &'static [WasmPluginSlot],
        backend: WasmBackend,
    ) -> Result<Self, HostError> {
        let mut states = BTreeMap::new();
        for definition in slots {
            if definition.name.trim().is_empty() {
                return Err(HostError::Slot("plugin slot name is empty".to_owned()));
            }
            if definition.channels.is_empty() {
                return Err(HostError::Slot(format!(
                    "plugin slot `{}` has no validation channel",
                    definition.name
                )));
            }
            if states
                .insert(
                    definition.name,
                    SlotState {
                        definition: *definition,
                        active: ArcSwapOption::empty(),
                        pending: Mutex::new(None),
                        has_pending: AtomicBool::new(false),
                        next_generation: AtomicU64::new(0),
                        activation_error: Mutex::new(None),
                    },
                )
                .is_some()
            {
                return Err(HostError::Slot(format!(
                    "duplicate plugin slot `{}`",
                    definition.name
                )));
            }
        }
        Ok(Self {
            backend,
            slots: states,
        })
    }

    /// Queue verified bytes without compiling or instantiating them.
    /// 挂起已验证字节，不在安装阶段编译或实例化。
    pub fn install(
        &self,
        slot: &str,
        channel: ValidationChannel,
        artifact: VerifiedPluginArtifact,
    ) -> Result<u64, HostError> {
        self.slot(slot)?.install(channel, artifact)
    }

    /// Activate a pending generation on demand, then invoke it.
    /// 首次使用时激活待发布代，再执行调用。
    pub fn call(&self, slot: &str, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        let state = self.slot(slot)?;
        state.activate(self.backend)?;
        let active = state
            .active
            .load_full()
            .ok_or_else(|| HostError::Slot(format!("plugin slot `{slot}` is not installed")))?;
        active.instance.call(operation, input)
    }

    /// Activate a pending generation on demand, without calling an operation.
    /// 按需激活待发布代，且不调用任何操作。
    ///
    /// Activation is lazy — it happens inside [`call`](Self::call) — so a host that installs an
    /// artifact and then polls [`is_loaded`](Self::is_loaded) learns nothing until it performs an
    /// operation. When the pending generation *cannot* activate, that gap is a dead end: no
    /// operation exists that would ever make `is_loaded` true, and the slot stays unloaded with no
    /// way to ask for the attempt (audit `PH-4`). This is that missing step, reachable on its own;
    /// `Ok(false)` means nothing was pending, so a caller may poll it without arming an unnecessary
    /// activation.
    /// 激活是惰性的——它发生在 [`call`](Self::call) 里——因此"装好工件再轮询
    /// [`is_loaded`](Self::is_loaded) 的宿主"在执行一次操作之前什么也学不到。当待定代际**无法**
    /// 激活时，这个缺口是死路：没有任何操作能让 `is_loaded` 变成 true，槽会一直未加载，且无从请求
    /// 那次尝试（审计 `PH-4`）。这就是缺的那一步，可以单独调用；`Ok(false)` 表示本来就没有待定项，
    /// 于是调用方可以放心轮询而不必触发一次不必要的激活。
    pub fn activate_pending(&self, slot: &str) -> Result<bool, HostError> {
        let state = self.slot(slot)?;
        if !state.has_pending.load(Ordering::Acquire) {
            return Ok(false);
        }
        state.activate(self.backend)?;
        Ok(true)
    }

    /// Report whether the slot has an active generation and nothing pending.
    /// 报告该槽是否已有激活代际且没有待处理代际。
    pub fn is_loaded(&self, slot: &str) -> Result<bool, HostError> {
        let state = self.slot(slot)?;
        Ok(!state.has_pending.load(Ordering::Acquire) && state.active.load().is_some())
    }

    /// Return the active generation, or `None` while the slot is not yet activated.
    /// 返回激活代际；槽尚未激活时返回 `None`。
    pub fn generation(&self, slot: &str) -> Result<Option<u64>, HostError> {
        Ok(self
            .slot(slot)?
            .active
            .load_full()
            .map(|active| active.generation))
    }

    /// Report why the last activation attempt failed.
    /// 报告上一次激活尝试失败的原因。
    ///
    /// `None` means the last attempt succeeded or no pending generation has been
    /// activated yet. The report exists because a failed activation changes
    /// nothing a poller can already see: `active` keeps the previous generation,
    /// so `is_loaded` stays `true` and `generation` keeps returning the old
    /// number while every later call fails. Silence was the one answer a
    /// readiness poll could not act on.
    /// `None` 表示上次尝试成功，或还没有待发布代被激活过。之所以需要这份报告：激活失败不会
    /// 改变轮询方已经能看到的东西——`active` 仍是上一代，因此 `is_loaded` 保持 `true`、
    /// `generation` 继续返回旧编号，而此后每次调用都失败。沉默正是就绪轮询唯一无法据以行动的
    /// 回报。
    pub fn activation_error(&self, slot: &str) -> Result<Option<String>, HostError> {
        self.slot(slot)?
            .activation_error
            .lock()
            .map(|error| error.clone())
            .map_err(|_| HostError::State("plugin slot lock was poisoned".to_owned()))
    }

    fn slot(&self, name: &str) -> Result<&SlotState, HostError> {
        self.slots
            .get(name)
            .ok_or_else(|| HostError::Slot(format!("unknown plugin slot `{name}`")))
    }
}

fn validate_artifact(
    slot: WasmPluginSlot,
    channel: ValidationChannel,
    artifact: &VerifiedPluginArtifact,
) -> Result<(), HostError> {
    crate::run_method::validate_artifact(
        slot.name,
        slot.framework,
        slot.mode,
        slot.contract,
        slot.channels,
        channel,
        artifact,
    )
    .map_err(|error| match error {
        crate::run_method::SlotValidationError::Policy(message) => HostError::Policy(message),
        crate::run_method::SlotValidationError::Contract(message) => HostError::Contract(message),
    })
}
