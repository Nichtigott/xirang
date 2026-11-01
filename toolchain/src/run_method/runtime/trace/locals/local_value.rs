//! A value captured or inferred for a single invocation.
//! 单次调用中捕获或推断的值。

use crate::run_method::registry_core::declaration::SourceLocation;

use super::local_kind::LocalKind;
use super::observation::Observation;

/// A value captured or inferred for a single invocation.
/// 单次调用中捕获或推断的值。
///
/// `observation` is `Unobserved` for MIR-only locals. Such values describe a
/// possible binding and never claim that a runtime value was seen.
/// `observation` 为 `Unobserved` 时表示仅由 MIR 推断，不能当作运行时实值。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalValue {
    /// The trace-local id; equal ids mean the same captured value.
    /// 追踪内的局部 id；id 相等即同一个已捕获值。
    pub id: u64,
    /// The name used in output, `function::name` for inferred locals.
    /// 输出中使用的名称；推断局部值为 `function::name`。
    pub name: String,
    /// The rendered type name reported by the recorder.
    /// 记录方给出的渲染后类型名。
    pub type_name: String,
    /// The rendered value, `<not observed>` for MIR-only locals.
    /// 渲染后的值；仅由 MIR 得出的局部值为 `<not observed>`。
    pub value: String,
    /// The role this value plays in its invocation.
    /// 该值在其调用中扮演的角色。
    pub kind: LocalKind,
    /// Where the value was recorded or inferred.
    /// 该值被记录或推断的位置。
    pub source: SourceLocation,
    /// The enclosing frame, `None` when recorded outside every traced call.
    /// 所属调用帧；在任何被追踪调用之外记录时为 `None`。
    pub frame_id: Option<u64>,
    /// Whether the value was observed at runtime or inferred statically.
    /// 该值是运行时观测到的还是静态推断出来的。
    pub observation: Observation,
}
