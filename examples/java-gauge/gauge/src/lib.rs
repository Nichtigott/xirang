//! 一个由 Java 注解声明、经构建期记录文件并进树、再由运行期注册树看见的面。
//! A face declared with the Java annotation, folded in through the build-time record file, visible
//! in the runtime registration tree.

xirang_toolchain::run_method::host!();

/// 这个示例的宿主身份。
/// The example's host identity.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.java-gauge");

/// 装配注册树。
/// Assemble the registration tree.
pub fn base_registry() -> Registry {
    xirang_toolchain::host_registry!().expect("the Java-declared face registers")
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
