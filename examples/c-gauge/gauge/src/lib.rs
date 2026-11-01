//! 一个 C 声明的面，经构建进树之后由运行期注册树看见。
//! A C-declared face, visible in the runtime registration tree once the build has folded it in.

xirang_toolchain::run_method::host!();

/// 这个示例的宿主身份。
/// The example's host identity.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.c-gauge");

/// 装配注册树。
/// Assemble the registration tree.
pub fn base_registry() -> Registry {
    xirang_toolchain::host_registry!().expect("the C face registers")
}

/// 打印逻辑路径、种类与来源，供测试断言。
/// Print logical paths, kinds and sources for the tests to assert on.
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
