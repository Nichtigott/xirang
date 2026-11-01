//! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
//! XiRang example: the README Control/Button two-level tree as a real host
//! library.
//!
//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
//! 注册面代码保持普通 Rust，父级不维护子对象清单。
//! The crate has one build wiring point. `host!()` pulls in the plan the build
//! step generated; face code stays ordinary Rust and no parent keeps a child
//! roster.

xirang_toolchain::run_method::host!();

// 这个 crate 自己调用 `host!()`，所以类型化 graft 计划里的 `crate::...` 与生成
// 树解析到同一个 crate。宿主如果把库和二进制分开，计划必须写在调用 `host!()`
// 的那一个里；写在另一个 crate 里的 Rust 路径无法在这里解析。
// This crate calls `host!()` itself, so `crate::...` in a typed graft plan
// resolves in the same crate as the generated tree. A host that splits a library
// and a binary must keep the plan in whichever one calls `host!()`.

// `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
// 会用私有项遮蔽那个公开重导出。
// `host!()` re-exports the kernel at the crate root, so the protocol nouns are
// already in scope; importing them again would shadow that public re-export.

/// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
/// The example's host identity. A graft requires both sides to share it.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");

// `cut` selects a logical host identity; `graft` names the linked replacement.
// `cut` 选择宿主逻辑身份；`graft` 指定静态链接的替换件。
// The catalog remains available when implementations move to another crate.
// 实现移入其它 crate 后，逻辑身份目录仍然可用。
// The array wrapper (`&[Graft]`) and the string selectors stay supported; this example uses the
// readable wrapper so that "a real host carries a plan" is a compiled fact, not a convention.
// 数组包装（`&[Graft]`）与字符串选择器仍然受支持；本示例用可读包装，好让"真宿主携带计划"是一条**被编译
// 过**的事实，而不是一条约定。
/// Static grafts against logical identities, independent of crate ownership.
/// 面向逻辑身份的静态 graft，与 crate 归属无关。
pub const GRAFTS: &[Graft] = &[
    graft(crate::catalog::control::object::BUTTON).with(control_button_graft::button_fast::NODE_ID),
    graft(crate::catalog::control::object::SLIDER).with(control_button_graft::slider_fast::NODE_ID),
];

/// 按框架和包命名空间装配这个示例的注册机。
/// Assemble the example's registry from its framework and package namespace.
///
/// It goes through the toolchain's named entry, so this example is also the end-to-end consumer of
/// `host_registry!()` — and because the crate is a workspace member, the workspace gate compiles and
/// tests it: the pin for "a host can reach its own registry" is this function running.
/// 它走工具链的具名入口，因此这个示例同时是 `host_registry!()` 的端到端消费者——而由于本 crate 是工作区
/// 成员，工作区门禁会编译并测试它："宿主拿得到自己的注册机"这条钉子，就是这个函数跑起来。
pub fn base_registry() -> Registry {
    xirang_toolchain::host_registry!().expect("example faces register")
}

/// 打印注册树的逻辑路径，供示例二进制和集成测试共用。
/// Print the registry tree's logical paths; shared by the example binary and the
/// integration tests.
pub fn outline() -> Vec<String> {
    let registry = base_registry();
    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{}  kind={}  source={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.file
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
