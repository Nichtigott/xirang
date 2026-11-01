//! Runtime locals and local-value queries.
//! 运行时局部值与局部值查询。

#[path = "local_id.rs"]
mod local_id;
#[path = "local_kind.rs"]
mod local_kind;
#[path = "local_value.rs"]
mod local_value;
#[path = "observation.rs"]
mod observation;
#[path = "recording.rs"]
mod recording;

pub use self::local_id::LocalId;
pub use self::local_kind::LocalKind;
pub use self::local_value::LocalValue;
pub use self::observation::Observation;
