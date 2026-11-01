//! Private generation state machine for one lazily activated Wasm slot.
//! 单个懒激活 Wasm 槽的私有代际状态机。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::run_method::VerifiedPluginArtifact;
use arc_swap::ArcSwapOption;

use crate::plugin_host::{HostError, PluginInstance, WasmBackend, WasmInstance};

use super::{ValidationChannel, WasmPluginSlot, validate_artifact};

/// One verified generation waiting for lazy compilation.
/// 一个等待懒编译的已验证代际。
pub(super) struct PendingPlugin {
    generation: u64,
    artifact: VerifiedPluginArtifact,
}

/// One activated generation paired with its live instance.
/// 一个已激活的代际及其存活实例。
pub(super) struct LoadedPlugin {
    pub(super) generation: u64,
    pub(super) instance: WasmInstance,
}

/// Generation bookkeeping for one named slot.
/// 单个具名槽的代际簿记。
pub(super) struct SlotState {
    pub(super) definition: WasmPluginSlot,
    pub(super) active: ArcSwapOption<LoadedPlugin>,
    pub(super) pending: Mutex<Option<PendingPlugin>>,
    pub(super) has_pending: AtomicBool,
    pub(super) next_generation: AtomicU64,
    /// Why the last activation of a pending generation failed, if it did.
    /// 上一次待发布代激活失败的原因（如果有）。
    ///
    /// A failed activation leaves `active` untouched, so the previous generation
    /// keeps answering and `is_loaded` keeps saying `true`; without this record
    /// the failure was invisible to every poller. The failed generation is dropped
    /// rather than re-queued, so this record is also the poller's signal that a
    /// retry means installing the artifact again (audit `LGC-LG-47`).
    /// 激活失败不会改动 `active`，上一代继续作答、`is_loaded` 继续说 `true`；没有这份记录，
    /// 失败对每个轮询方都不可见。失败的代际被丢弃而不是重新排队，因此这份记录也是轮询方"想重试就得
    /// 重新安装工件"的信号（审计 `LGC-LG-47`）。
    pub(super) activation_error: Mutex<Option<String>>,
}

impl SlotState {
    pub(super) fn install(
        &self,
        channel: ValidationChannel,
        artifact: VerifiedPluginArtifact,
    ) -> Result<u64, HostError> {
        validate_artifact(self.definition, channel, &artifact)?;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| HostError::State("plugin slot lock was poisoned".to_owned()))?;
        // The generation is allocated while the lock is held. Allocating it
        // first let two racing installs number themselves out of order: the one
        // that took the lock second stored a *lower* generation last, so the
        // number a caller received could be superseded by an older one and the
        // higher generation was silently dropped.
        // 代际在持锁期间分配。先分配会让两个并发安装把编号排反：后拿到锁的那个最后写入一个
        // **更小**的代际，于是调用方收到的编号可能被更老的编号取代，更大的那个被静默丢弃。
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
        *pending = Some(PendingPlugin {
            generation,
            artifact,
        });
        self.has_pending.store(true, Ordering::Release);
        Ok(generation)
    }

    /// Activate the pending generation, compiling it outside the slot lock.
    /// 激活待发布代际，并在槽锁之外编译它。
    ///
    /// The lock covers the hand-off and the publish, never the load: `backend.load`
    /// compiles and instantiates a module, and holding the slot's lock across it made
    /// every concurrent `call`, `install` and `activate` wait for that compile (audit
    /// `LGC-LG-47`). Because an `install` may now land *during* a load, the flag is
    /// published from the queue's own state under the lock rather than cleared
    /// blindly: a generation queued while this one compiled must stay pending
    /// instead of being mistaken for one that has been handled.
    /// 锁只覆盖交接与发布，绝不覆盖加载：`backend.load` 会编译并实例化模块，把槽锁跨它持有会让
    /// 每一次并发 `call`/`install`/`activate` 都等这次编译（审计 `LGC-LG-47`）。由于一次
    /// `install` 现在可能落在加载**期间**，标志改为在锁下从队列自身状态发布，而不是盲目清零：
    /// 本次编译期间排队的代际必须保持待定，而不是被误当成已处理的代际。
    pub(super) fn activate(&self, backend: WasmBackend) -> Result<(), HostError> {
        self.activate_with(|artifact| backend.load_wasm_backend(artifact))
    }

    /// The same activation with the load itself injected.
    /// 同一套激活流程，但加载本身可注入。
    ///
    /// This seam exists to make the timing claim above **measurable**. A real
    /// `backend.load` on this machine finishes in microseconds, so a test that wants
    /// to observe the lock being free *while a load runs* has no window: the load is
    /// over before a second thread gets scheduled. Injecting the load lets a test
    /// hold it open (a barrier or a `sleep` on the far side of the seam) and then ask
    /// the slot, from another thread, whether `install` or the queue is still
    /// reachable — which is exactly the property audit `LGC-LG-47` is about. The
    /// verdict is not this module's to give: the pin is here for the reviewer who
    /// re-runs the fix (`slot_state_tests`), including its red side.
    /// 这个接缝是为了让上面那条**时序**主张**可测**。本机真实的 `backend.load` 微秒级完成，因此想观察
    /// \"加载**期间**锁是空的\"的测试根本没有窗口：第二个线程还没被调度，加载已经结束了。注入加载让测试
    /// 能在接缝另一边把它拖住（barrier 或 `sleep`），再从另一个线程问这个槽：`install` 或队列是否仍然
    /// 可达——这正是审计 `LGC-LG-47` 说的性质。结论不由本模块给出：这条钉子留给重跑修复的复核者
    /// （`slot_state_tests`），红侧一起给。
    ///
    /// Two constraints on the injected closure, both learned the hard way:
    /// 对注入闭包的两条约束，都是踩过才写下的：
    ///
    /// * **It must not re-enter this slot.** The closure runs on the calling thread
    ///   with `pending` released, and every entry into the slot (`activate`,
    ///   `activate_pending`, `install`, `take_pending`, `publish_pending`) takes that
    ///   same non-reentrant `std::sync::Mutex`. A load that calls back into this slot —
    ///   for example by going through the public table API while a test still holds the
    ///   hand-off — would take a lock this thread cannot have and **deadlock the test
    ///   instead of failing it**. Verification hit exactly that while mutating the
    ///   hand-off back to the pre-fix shape, so a load that needs to inspect the slot
    ///   does it from *another* thread.
    /// * **它不得重入本槽。** 闭包在调用线程上跑，此时 `pending` 是放开的，而进入这个槽的每一个
    ///   入口（`activate`、`activate_pending`、`install`、`take_pending`、`publish_pending`）
    ///   都要拿同一把**不可重入**的 `std::sync::Mutex`。一次加载若回调进本槽——比如在测试仍握着
    ///   交接时经公开表 API 绕回来——会去拿这把线程已经不可能拿到的锁，把测试**吊死**而不是让它
    ///   失败。复核者在把交接改回修前形状时就踩到过这一点，因此需要观察槽的加载请**从另一个线程**做。
    /// * **It is `FnOnce`.** One activation drives exactly one load, so a seam cannot
    ///   count attempts by calling the closure twice; a test that wants two loads calls
    ///   `activate_with` twice. The closure returns the `WasmInstance` and the shared
    ///   body still runs `health_check`, so a closure can refuse a load (`Err`) without
    ///   inventing an instance — that is how the timing pins keep a real `WasmInstance`
    ///   out of their scaffolding.
    /// * **它是 `FnOnce`。** 一次激活恰好驱动一次加载，因此接缝不能靠把闭包调两次来计数尝试次数；
    ///   想要两次加载的测试就调两次 `activate_with`。闭包交回 `WasmInstance`，且 `health_check`
    ///   仍在共享的函数体里跑，因此闭包可以只**拒绝**一次加载（`Err`）而不必造出实例——时序钉子
    ///   正是靠这一点把真正的 `WasmInstance` 挡在脚手架之外。
    fn activate_with(
        &self,
        load: impl FnOnce(VerifiedPluginArtifact) -> Result<WasmInstance, HostError>,
    ) -> Result<(), HostError> {
        if !self.has_pending.load(Ordering::Acquire) {
            return Ok(());
        }
        let Some(candidate) = self.take_pending()? else {
            self.publish_pending();
            return Ok(());
        };

        let loaded = load(candidate.artifact).and_then(|instance| {
            instance.health_check()?;
            Ok(LoadedPlugin {
                generation: candidate.generation,
                instance,
            })
        });
        match loaded {
            Ok(loaded) => {
                self.active.store(Some(Arc::new(loaded)));
                self.publish_pending();
                self.record_activation(None);
                Ok(())
            }
            Err(error) => {
                // The failed generation is dropped and the slot reports it through
                // `activation_error`: `is_loaded` keeps describing the generation
                // that is still serving, which is the contract `fault_matrix` pins.
                // 失败的代际被丢弃，槽通过 `activation_error` 报告它：`is_loaded` 继续描述仍在
                // 服务的那个代际，这正是 `fault_matrix` 钉住的契约。
                self.publish_pending();
                self.record_activation(Some(error.to_string()));
                Err(error)
            }
        }
    }

    /// Take the pending generation out under the slot lock, releasing the lock
    /// before the caller compiles it.
    /// 在槽锁下取出待发布代际，并在调用方编译它之前释放锁。
    fn take_pending(&self) -> Result<Option<PendingPlugin>, HostError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| HostError::State("plugin slot lock was poisoned".to_owned()))?;
        Ok(pending.take())
    }

    /// Publish whether a generation is queued, read under the same lock the
    /// installs write the queue with.
    /// 在与安装写队列的同一把锁下读取并发布"是否还有代际排队"。
    fn publish_pending(&self) {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        self.has_pending.store(pending.is_some(), Ordering::Release);
    }

    /// Remember, or clear, the outcome of the last activation.
    /// 记下或清除上一次激活的结果。
    fn record_activation(&self, error: Option<String>) {
        if let Ok(mut slot) = self.activation_error.lock() {
            *slot = error;
        }
    }
}

#[cfg(test)]
#[path = "slot_state_tests.rs"]
mod slot_state_tests;
