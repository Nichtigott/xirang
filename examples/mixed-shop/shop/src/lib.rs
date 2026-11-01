xirang_toolchain::run_method::host!();

// A library host has no `main` to write at the end of: record around the work you
// actually run (`CallTrace::runtime`, `trace_call!`), then write it with
// `trace_artifact_path` + `write_trace_artifact`, or scaffold a binary for the demo.

/// 这个示例的宿主身份。
/// The example's host identity.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.mixed-shop");

/// 装配注册树。
/// Assemble the registration tree.
pub fn base_registry() -> Registry {
    xirang_toolchain::host_registry!().expect("example faces register")
}

/// 打印逻辑路径，供测试断言。
/// Print the logical paths for the tests to assert on.
pub fn outline() -> Vec<String> {
    let registry = base_registry();
    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{} kind={} source={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.file
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

/// The shared catalog names the C++ Area through the same object relation the Rust tree uses.
/// 共享目录用与 Rust 树相同的对象关系点名 C++ Area。
#[cfg(test)]
mod catalog_contract {
    /// The exact reference a Rust declaration writes; the C++ source writes the same string.
    /// Rust 声明写下的精确引用；C++ 源码写下同一个字符串。
    const AREA: xirang_toolchain::NodeId = crate::catalog::shop::panel::Area::NODE_ID;

    #[test]
    fn the_catalog_area_is_a_compile_time_node() {
        assert_eq!(AREA.to_string().len(), 32);
    }
}
