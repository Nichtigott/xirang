//! Fuel-metered, import-free Wasm plugin loading and invocation.
//! 启用燃料计量、无导入的 Wasm 插件加载与调用。

use std::sync::Mutex;

use crate::run_method::{PluginAdapter, VerifiedPluginArtifact};
use wasmi::{
    Config, EnforcedLimits, Engine, Instance, Linker, Memory, Module, Store, StoreLimits,
    StoreLimitsBuilder,
};

use crate::plugin_host::{HostError, PluginInstance};

/// Resource budget applied to every Wasm call.
/// 每次 Wasm 调用使用的资源预算。
#[derive(Clone, Copy, Debug)]
pub struct WasmLimits {
    /// Linear-memory ceiling per instance, in bytes; growth beyond it traps.
    /// 每个实例的线性内存上限（字节）；超出即触发 trap。
    pub memory_bytes: usize,
    /// Fuel granted to each call; an exhausted call traps.
    /// 每次调用授予的燃料；燃料耗尽时调用触发 trap。
    ///
    /// The unit belongs to the engine, not to this host: it is a bound on how
    /// much work a call may do, not a promise that a given number buys a given
    /// amount of work. A major engine upgrade can re-scale it, so a host that
    /// tuned this value against an older engine should re-measure rather than
    /// assume the same number still fits.
    /// 该单位属于引擎而不属于本宿主：它是"一次调用最多做多少工作"的上限，而不是"某个数值
    /// 能买到多少工作"的承诺。引擎大版本升级可能重新标定它，因此针对旧引擎调过这个值的宿主
    /// 应当重新实测，而不是假设同一个数字仍然够用。
    pub fuel_per_call: u64,
    /// Largest request payload accepted, in bytes — but the host writes the input at
    /// offset 0 of the plugin's *initial* memory before calling the handler, so the real
    /// ceiling is the smaller of this and that memory. A one-page plugin therefore
    /// refuses every input above 64 KiB however large this number is; a plugin that needs
    /// more declares it in its initial memory (or grows it from `start`).
    /// 接受的最大请求负载字节数——但宿主在调用处理函数之前把输入写在插件**初始**内存的偏移 0
    /// 处，因此真正的上限是这个值与那块内存中较小的一个。于一页内存的插件无论这里多大都会拒绝
    /// 超过 64 KiB 的输入；需要更多输入的插件应在初始内存里声明（或在 `start` 里增长）。
    pub max_input_bytes: usize,
    /// Largest response payload accepted, in bytes.
    /// 接受的最大响应负载字节数。
    pub max_output_bytes: usize,
    /// Largest number of table elements a module may instantiate.
    /// 模块可实例化的表元素上限。
    ///
    /// The memory ceiling does not bound a table: a table is a separate array of
    /// function references, instantiated eagerly, so a module that declares
    /// `(table 100000000 funcref)` costs hundreds of megabytes of host memory
    /// without touching a single memory page. This is the ceiling for that array,
    /// and a module over it fails to instantiate rather than being honoured.
    /// 内存上限并不约束表：表是一块独立的函数引用数组，会即时实例化，因此声明
    /// `(table 100000000 funcref)` 的模块会花掉宿主数百兆内存，而一页线性内存都没碰。
    /// 这里是那块数组的上限，超过它的模块实例化失败，而不是被照办。
    ///
    /// Measured rather than estimated: a function reference costs the host 8 bytes
    /// (`plugin-host/tests/wasm_table_cost.rs`), so the default ceiling of 4096 is
    /// 32 KiB and the hundred-million-entry module above would be 762 MiB. The
    /// limiter denies the allocation before the table exists, so refusing it was
    /// measured at 5 KiB of peak allocation, not 762 MiB. This field is the only
    /// bound on a single table's size: wasmi's `EnforcedLimits::strict()`, which
    /// `WasmBackend::load` also applies, caps how many tables a module may declare
    /// (`max_tables`) and how many element segments it may carry
    /// (`max_element_segments`), but not how large one table may grow.
    /// 实测而非估计：一个函数引用在宿主一侧占 8 字节（`plugin-host/tests/wasm_table_cost.rs`），
    /// 因此默认上限 4096 是 32 KiB，而上面那个一亿条目的模块本来会是 762 MiB。限制器在表存在
    /// 之前就拒绝这次分配，因此拒绝它的实测峰值是 5 KiB，而不是 762 MiB。本字段是单张表大小的
    /// 唯一约束：wasmi 的 `EnforcedLimits::strict()`（`WasmBackend::load` 也会施加）限制的是一个
    /// 模块可以声明多少张表（`max_tables`）与多少个元素段（`max_element_segments`），而不是
    /// 单张表能长到多大。
    pub table_elements: usize,
    /// Largest total element-section payload a module may carry, in bytes.
    /// 模块可携带的元素段负载总量上限，以字节计。
    ///
    /// A passive element segment is invisible to the two ceilings that look like
    /// they cover it: `table_elements` bounds a table's *growth* and a passive
    /// segment never grows one, while wasmi's `EnforcedLimits::strict()` caps how
    /// many element *segments* a module may declare, not how many entries they
    /// carry. wasmi materializes every entry at instantiation, measured at about
    /// 32 bytes per entry against about one byte per entry in the compact
    /// encoding — a 2 000 103-byte module carrying two million entries cost
    /// 64 070 402 bytes of host memory, with `(table 1 funcref)` and default
    /// limits. This budget is therefore also an allocation budget of roughly
    /// thirty-two times it, which is why the default caps that shape at about
    /// 8 MiB. The payload is measured from the binary's section headers before
    /// compilation, the same hand-checked seam `max_module_bytes` uses, because no
    /// engine limit applies before the engine runs.
    /// 被动元素段对两道看起来覆盖它的上限都不可见：`table_elements` 约束的是表的**增长**，
    /// 而被动段从不增长表；wasmi 的 `EnforcedLimits::strict()` 限制的是一个模块可以声明多少个
    /// 元素**段**，而不是它们携带多少条目。wasmi 在实例化时为每个条目物化约 32 字节，而紧凑编码
    /// 下每条约一字节——一个 2 000 103 字节、带两百万条目的模块，在 `(table 1 funcref)` 与默认
    /// 上限下花掉 64 070 402 字节宿主内存。因此本预算同时也是约三十二倍的分配预算，这正是默认值
    /// 把那种形状压在约 8 MiB 的原因。负载在编译前从二进制的段头量出，与 `max_module_bytes`
    /// 用的是同一处手工检查接缝，因为在引擎运行之前没有任何引擎限制生效。
    pub max_element_bytes: usize,
    /// Largest artifact the backend will compile, in bytes.
    /// 后端愿意编译的最大工件字节数。
    ///
    /// Compilation happens before any limit below can apply, so this is the one
    /// bound that has to be checked by hand; it is what keeps a huge artifact from
    /// spending the host's memory and time before the sandbox is even entered.
    /// 编译发生在下面任何限制生效之前，因此这是唯一必须手工检查的上限；正是它阻止一个巨大
    /// 工件在沙箱都没进入之前就花掉宿主的内存与时间。
    pub max_module_bytes: usize,
}

impl Default for WasmLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 16 * 1024 * 1024,
            fuel_per_call: 1_000_000,
            max_input_bytes: 1024 * 1024,
            max_output_bytes: 1024 * 1024,
            table_elements: 4096,
            max_element_bytes: 256 * 1024,
            max_module_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Loads Wasm without WASI or host imports.
/// 在没有 WASI 和宿主导入的环境中加载 Wasm。
#[derive(Clone, Copy, Debug, Default)]
pub struct WasmBackend {
    limits: WasmLimits,
}

impl WasmBackend {
    /// Build a backend that applies `limits` to every loaded instance.
    /// 构造一个对每个已加载实例施加 `limits` 的后端。
    pub const fn new(limits: WasmLimits) -> Self {
        Self { limits }
    }

    /// Compile and instantiate the artifact, checking the limits, the host ABI version
    /// *when the plugin exports one* (a missing `xirang_abi_version` is a legacy
    /// plugin, as `README.md` says), and the `xirang_health` export.
    /// 编译并实例化工件，检查各项上限、宿主 ABI 版本（**当插件导出它时**；缺少
    /// `xirang_abi_version` 的是旧插件，见 `README.md`）以及 `xirang_health` 导出。
    ///
    /// This performs **no slot policy**: channel, framework, mode and flow are checked by
    /// [`WasmPluginTable::install`](crate::plugin_host::WasmPluginTable::install) and
    /// `validate_artifact`. A host that calls the backend directly gets the wasm limits
    /// and nothing else.
    /// 本方法**不做槽位策略**：通道、框架、模式与 flow 由
    /// [`WasmPluginTable::install`](crate::plugin_host::WasmPluginTable::install) 与 `validate_artifact`
    /// 检查。直接调用后端的宿主只得到 wasm 上限，别的什么都没有。
    pub fn load_wasm_backend(
        &self,
        artifact: VerifiedPluginArtifact,
    ) -> Result<WasmInstance, HostError> {
        let (_, bytes) = artifact.into_parts();
        if bytes.len() > self.limits.max_module_bytes {
            return Err(HostError::Limit(format!(
                "artifact is {} bytes; limit is {}",
                bytes.len(),
                self.limits.max_module_bytes
            )));
        }
        // A passive element segment never reaches `table_growing`, so the store's
        // element ceiling does not see it: this is the only bound on what a
        // module can make wasmi materialize at instantiation.
        // 被动元素段永远到不了 `table_growing`，因此 store 的元素上限看不到它：这是对"模块能
        // 让 wasmi 在实例化时物化多少"的唯一边界。
        let element_bytes = element_section_bytes(&bytes);
        if element_bytes > self.limits.max_element_bytes {
            return Err(HostError::Limit(format!(
                "module declares {element_bytes} bytes of element segments; limit is {}",
                self.limits.max_element_bytes
            )));
        }
        let mut config = Config::default();
        config.consume_fuel(true);
        // wasmi's own strict limits bound what a module may contain and refuse one
        // whose functions could be compiled lazily; its defaults leave all of that
        // unlimited, so a small artifact could otherwise buy unbounded compile-time
        // work. `strict` is wasmi's number, not one invented here.
        // wasmi 自带的 strict 限制约束模块可以包含什么，并拒绝那些函数可能被惰性编译的模块；
        // 它的默认值把这一切都留成无限，因此一个很小的工件本来可以换来无界的编译期工作量。
        // `strict` 是 wasmi 自己的数值，不是这里编的。
        config.enforced_limits(EnforcedLimits::strict());
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &bytes)
            .map_err(|error| HostError::InvalidArtifact(error.to_string()))?;
        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.memory_bytes)
            .table_elements(self.limits.table_elements)
            .instances(1)
            .memories(1)
            .tables(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&engine, store_limits);
        store.limiter(|limits| limits);
        store
            .set_fuel(self.limits.fuel_per_call)
            .map_err(|error| HostError::Limit(error.to_string()))?;
        let linker = Linker::new(&engine);
        // `instantiate_and_start` is what `instantiate` plus `PreInstance::start`
        // became in wasmi 1.x; it runs the module's `start` function, which is
        // why the fuel above is set before it. A plugin cannot dodge its budget
        // by doing work during instantiation.
        // `instantiate_and_start` 就是 `instantiate` 加 `PreInstance::start` 在 wasmi 1.x
        // 中的形态；它会运行模块的 `start` 函数，这也正是上面那笔燃料必须在此之前设定好的
        // 原因。插件无法靠把工作放进实例化阶段来逃避预算。
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|error| HostError::InvalidArtifact(error.to_string()))?;
        if let Ok(abi) = instance.get_typed_func::<(), i32>(&store, "xirang_abi_version") {
            let version = abi
                .call(&mut store, ())
                .map_err(|error| HostError::Abi(error.to_string()))?;
            if version != crate::run_method::PLUGIN_ABI_VERSION as i32 {
                return Err(HostError::Abi(format!(
                    "plugin ABI version {version} is incompatible with host ABI {}",
                    crate::run_method::PLUGIN_ABI_VERSION
                )));
            }
        }
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| HostError::Abi("missing exported memory `memory`".to_owned()))?;
        instance
            .get_typed_func::<(i32, i32), i64>(&store, "xirang_health")
            .map_err(|_| HostError::Abi("missing `xirang_health(i32, i32) -> i64`".to_owned()))?;

        Ok(WasmInstance {
            state: Mutex::new(WasmState {
                store,
                instance,
                memory,
            }),
            limits: self.limits,
        })
    }
}

/// Total element-section payload in a wasm binary, in bytes.
/// wasm 二进制里元素段负载的总字节数。
///
/// Only the section headers are walked — `id`, LEB128 size, payload — which is
/// enough to bound the entries, because every entry costs at least one byte of
/// payload. A binary this walk cannot make sense of reports `0` and is left to the
/// engine, whose diagnostic for a malformed module is better than a budget
/// refusal would be.
/// 只走段头——`id`、LEB128 长度、负载——这已足以约束条目数，因为每个条目至少占一字节负载。
/// 本遍历读不懂的二进制报 `0` 并留给引擎：它对"模块损坏"的诊断比一条预算拒绝更有用。
fn element_section_bytes(bytes: &[u8]) -> usize {
    /// Magic (4) plus version (4).
    /// 魔数（4）加版本（4）。
    const HEADER: usize = 8;
    /// Section id of the element section.
    /// 元素段的段 id。
    const ELEMENT_SECTION: u8 = 9;
    if bytes.len() < HEADER || &bytes[..4] != b"\0asm" {
        return 0;
    }
    let mut index = HEADER;
    let mut total = 0usize;
    while index < bytes.len() {
        let id = bytes[index];
        index += 1;
        let Some((size, next)) = read_uleb(bytes, index) else {
            return 0;
        };
        let size = size as usize;
        if next + size > bytes.len() {
            return 0;
        }
        if id == ELEMENT_SECTION {
            total += size;
        }
        index = next + size;
    }
    total
}

/// One unsigned LEB128 value at `start`, with the index after it.
/// `start` 处的一个无符号 LEB128 取值，以及它之后的索引。
fn read_uleb(bytes: &[u8], start: usize) -> Option<(u32, usize)> {
    let mut value = 0u32;
    let mut shift = 0u32;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        value |= u32::from(byte & 0x7f).checked_shl(shift)?;
        index += 1;
        if byte & 0x80 == 0 {
            return Some((value, index));
        }
        shift += 7;
        if shift >= 32 {
            return None;
        }
    }
    None
}

struct WasmState {
    store: Store<StoreLimits>,
    instance: Instance,
    memory: Memory,
}

/// A loaded, fuel-metered Wasm plugin.
/// 已加载并启用燃料计量的 Wasm 插件。
pub struct WasmInstance {
    state: Mutex<WasmState>,
    limits: WasmLimits,
}

/// The largest input this adapter can frame.
/// 本适配器能成帧的最大输入。
///
/// The Wasm ABI passes the length as an `i32`, so `i32::MAX` is the ceiling the *frame* imposes:
/// above it `input.len() as i32` wraps to a negative length instead of failing. The process adapter
/// has the same kind of ceiling, at `u32::MAX`, because its frames carry a u32 length — the two
/// adapters disagreed, and only the process side enforced its own (audit `PH-5`).
/// Wasm ABI 把长度作为 `i32` 传递，因此 `i32::MAX` 是**帧格式**给出的上限：超过它，
/// `input.len() as i32` 会回绕成负数而不是失败。进程适配器有同类的上限 `u32::MAX`，因为它的帧携带
/// u32 长度——两个适配器口径不一致，而只有进程那一边真的执行了自己的上限（审计 `PH-5`）。
const MAX_FRAMEABLE_INPUT: usize = i32::MAX as usize;

/// Reject an input this adapter cannot frame, naming whether the configured limit or the frame
/// width is what refused it.
/// 拒绝本适配器无法成帧的输入，并说明拒绝它的是配置上限还是帧宽。
///
/// Kept as one function so the frame-width half can be unit-tested without a two-gigabyte slice:
/// the branch is a value, not a filesystem or an allocation.
/// 收成一个函数，正是为了让帧宽那一半无需一个两吉字节的切片就能被单测：这个分支是一个值，而不是
/// 文件系统或一次分配。
fn check_input_length(length: usize, configured: usize) -> Result<(), HostError> {
    if length > configured {
        return Err(HostError::Limit(format!(
            "input is {length} bytes; limit is {configured}"
        )));
    }
    if length > MAX_FRAMEABLE_INPUT {
        return Err(HostError::Limit(format!(
            "input is {length} bytes; a Wasm call frames its length in an i32, whose maximum is \
             {MAX_FRAMEABLE_INPUT}"
        )));
    }
    Ok(())
}

impl PluginInstance for WasmInstance {
    fn adapter(&self) -> PluginAdapter {
        PluginAdapter::Wasm
    }

    fn call(&self, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        check_input_length(input.len(), self.limits.max_input_bytes)?;
        let export = operation_export(operation)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::Process("Wasm store lock was poisoned".to_owned()))?;
        state
            .store
            .set_fuel(self.limits.fuel_per_call)
            .map_err(|error| HostError::Limit(error.to_string()))?;
        let memory = state.memory;
        memory
            .write(&mut state.store, 0, input)
            .map_err(|error| HostError::Abi(error.to_string()))?;
        let instance = state.instance;
        let function = instance
            .get_typed_func::<(i32, i32), i64>(&state.store, &export)
            .map_err(|_| HostError::Abi(format!("missing `{export}(i32, i32) -> i64`")))?;
        let packed = function
            .call(&mut state.store, (0, input.len() as i32))
            .map_err(|error| HostError::Process(error.to_string()))? as u64;
        let offset = (packed >> 32) as usize;
        let length = (packed & u64::from(u32::MAX)) as usize;
        if length > self.limits.max_output_bytes {
            return Err(HostError::Limit(format!(
                "output is {length} bytes; limit is {}",
                self.limits.max_output_bytes
            )));
        }
        let end = offset
            .checked_add(length)
            .ok_or_else(|| HostError::Abi("output range overflowed".to_owned()))?;
        if end > memory.data_size(&state.store) {
            return Err(HostError::Abi(format!(
                "output range {offset}..{end} is outside linear memory"
            )));
        }
        let mut output = vec![0; length];
        memory
            .read(&state.store, offset, &mut output)
            .map_err(|error| HostError::Abi(error.to_string()))?;
        Ok(output)
    }
}

fn operation_export(operation: &str) -> Result<String, HostError> {
    if crate::run_method::validate_operation_name(operation).is_err() {
        return Err(HostError::InvalidOperation(operation.to_owned()));
    }
    Ok(format!("xirang_{operation}"))
}

#[cfg(test)]
#[path = "wasm_tests.rs"]
mod wasm_tests;
