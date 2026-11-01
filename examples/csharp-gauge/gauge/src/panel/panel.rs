//! Panel：C# 声明的 Gauge 所挂的 Rust 父面。
//! Panel: the Rust parent face the C#-declared Gauge hangs under.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Panel>()
    .needs_registry(true)
    .under(crate::root_node_id(crate::XIRANG_NAMESPACE))
    .registry_rule_path("src/panel/registry_rule/registry_rule.rs");

/// 父面的业务类型。
/// The parent face's business type.
pub struct Panel;
