//! Atomic deployment of registration metadata plus executable code.
//! 注册元数据与可执行代码的原子部署。

use std::sync::{Arc, Mutex};

use crate::run_method::{GraftPlan, Registry};
use arc_swap::ArcSwap;

use crate::plugin_host::{HostError, PluginInstance};

/// One consistent view of registration metadata and executable code.
/// 注册元数据与可执行代码的一致视图。
pub struct Deployment<I> {
    /// Monotonic counter; compare successive snapshots to notice a swap. Starts at 0.
    /// 单调递增的计数器；比较前后快照即可察觉替换。初始为 0。
    pub generation: u64,
    /// Registry snapshot that matches `plugin`.
    /// 与 `plugin` 相匹配的注册树快照。
    pub registry: Registry,
    /// Plugin instance serving this snapshot; the `Arc` keeps it alive for readers.
    /// 服务该快照的插件实例；`Arc` 让读者持有期间其保持存活。
    pub plugin: Arc<I>,
}

/// Lock-free reads with serialized, validated replacement.
/// 读取无锁，替换串行校验。
pub struct HotDeployment<I> {
    current: ArcSwap<Deployment<I>>,
    writer: Mutex<()>,
}

impl<I: PluginInstance> HotDeployment<I> {
    /// Install generation 0 after the plugin passes its health probe.
    /// 在插件通过健康探针后安装第 0 代。
    pub fn new(registry: Registry, plugin: I) -> Result<Self, HostError> {
        plugin.health_check()?;
        Ok(Self {
            current: ArcSwap::from_pointee(Deployment {
                generation: 0,
                registry,
                plugin: Arc::new(plugin),
            }),
            writer: Mutex::new(()),
        })
    }

    /// Return the current snapshot; readers never block a writer.
    /// 返回当前快照；读者不会阻塞写者。
    pub fn load_hot_deployment(&self) -> Arc<Deployment<I>> {
        self.current.load_full()
    }

    /// Validate code and registry graft before publishing one new generation.
    /// 发布新代之前，先完成代码体检和注册树嫁接校验。
    pub fn replace(
        &self,
        plugin: I,
        plan: &GraftPlan,
        external: &Registry,
    ) -> Result<u64, HostError> {
        plugin.health_check()?;
        let _writer = self
            .writer
            .lock()
            .map_err(|_| HostError::Process("deployment writer lock was poisoned".to_owned()))?;
        let live = self.current.load_full();
        let registry = live
            .registry
            .overlay(plan, external)
            .map_err(|error| HostError::Registry(error.to_string()))?;
        let generation = live.generation.saturating_add(1);
        self.current.store(Arc::new(Deployment {
            generation,
            registry,
            plugin: Arc::new(plugin),
        }));
        Ok(generation)
    }
}
