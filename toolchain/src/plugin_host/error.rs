//! Failure vocabulary returned at the plugin-host seam.
//! 插件宿主接口返回的失败词汇。

use std::{fmt, io};

/// Failure returned at the plugin-host seam.
/// 插件宿主接口返回的失败。
#[derive(Debug)]
pub enum HostError {
    /// The artifact failed structural checks such as checksum or Wasm validation.
    /// 工件未通过校验和、Wasm 格式等结构检查。
    InvalidArtifact(String),
    /// The operation name is not a valid plugin identifier.
    /// 操作名不是合法的插件标识符。
    InvalidOperation(String),
    /// The plugin's exports or ABI version contradict the host contract.
    /// 插件的导出或 ABI 版本与宿主契约不符。
    Abi(String),
    /// A declared resource budget (fuel, memory, input, or output) was exceeded.
    /// 超出声明的资源预算（燃料、内存、输入或输出）。
    Limit(String),
    /// The child process could not start, exited non-zero, or broke its pipes.
    /// 子进程无法启动、非零退出或管道中断。
    Process(String),
    /// The call exceeded its hard deadline and the host killed it.
    /// 调用超出硬超时，宿主已将其中止。
    Timeout,
    /// The health probe answered something other than `ok`.
    /// 健康探针的回答不是 `ok`。
    Health(String),
    /// The slot is unknown, malformed, or carries no installed generation.
    /// 插件槽未知、定义非法，或没有已安装的代际。
    Slot(String),
    /// The slot's trust lane or manifest policy rejected the artifact.
    /// 插件槽的信任通道或清单策略拒绝了该工件。
    Policy(String),
    /// The artifact's flow contract does not match the slot's contract.
    /// 工件的数据流合同与插件槽的合同不匹配。
    Contract(String),
    /// Host bookkeeping state is unusable, typically a poisoned lock.
    /// 宿主簿记状态不可用，通常是锁被毒化。
    State(String),
    /// The registry graft needed to publish a deployment failed.
    /// 发布部署所需的注册树嫁接失败。
    Registry(String),
    /// Underlying I/O failure; the only variant that exposes a source error.
    /// 底层 I/O 失败；唯一会暴露源错误的变体。
    Io(io::Error),
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArtifact(message) => {
                write!(formatter, "invalid plugin artifact: {message}")
            }
            Self::InvalidOperation(message) => {
                write!(formatter, "invalid plugin operation: {message}")
            }
            Self::Abi(message) => write!(formatter, "plugin ABI error: {message}"),
            Self::Limit(message) => write!(formatter, "plugin limit exceeded: {message}"),
            Self::Process(message) => write!(formatter, "plugin process failed: {message}"),
            Self::Timeout => formatter.write_str("plugin call timed out"),
            Self::Health(message) => write!(formatter, "plugin health check failed: {message}"),
            Self::Slot(message) => write!(formatter, "plugin slot error: {message}"),
            Self::Policy(message) => {
                write!(formatter, "plugin policy rejected artifact: {message}")
            }
            Self::Contract(message) => write!(formatter, "plugin contract mismatch: {message}"),
            Self::State(message) => write!(formatter, "plugin host state failed: {message}"),
            Self::Registry(message) => write!(formatter, "registry graft failed: {message}"),
            Self::Io(error) => write!(formatter, "plugin I/O failed: {error}"),
        }
    }
}

impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for HostError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
