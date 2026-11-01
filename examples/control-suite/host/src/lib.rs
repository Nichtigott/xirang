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
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-suite");

// `graft` selects a logical host identity; `with` names the linked replacement.
// `graft` 选择宿主逻辑身份；`with` 指定静态链接的替换件。
// The catalog remains available when implementations move to another crate.
// 实现移入其它 crate 后，逻辑身份目录仍然可用。
// String selectors remain supported: graft("root/control/button").with("button_fast").
// 字符串选择器仍然受支持，但字符串内容不具备 Rust 路径补全。
/// Static grafts against logical identities, independent of crate ownership.
/// 面向逻辑身份的静态 graft，与 crate 归属无关。
pub const GRAFTS: &[Graft] = &[
    graft(crate::catalog::control::object::BUTTON)
        .with(control_button_graft::button_fast::NODE_ID),
    graft(crate::catalog::control::object::SLIDER)
        .with(control_button_graft::slider_fast::NODE_ID),
];

/// 按框架和包命名空间装配这个示例的注册机。
/// Assemble the example's registry from its framework and package namespace.
///
/// The same named entry `examples/control-button` uses: two hosts, one incantation — and this one is
/// **not a workspace member**, so it is the half that proves the entry works from a crate whose
/// `build.rs` ran outside the workspace build. `tools/xirang-daily-behaviors` is what builds it
/// (measured: it was the only device naming this example, and `tools/xirang-external-rehearsal`
/// does not touch it).
/// 与 `examples/control-button` 用的是同一个具名入口：两个宿主、一句咒语——而这一份**不是工作区成员**，
/// 因此它是"该入口能从'不在工作区构建里'的 crate 用起来"的那一半。构建它的是
/// `tools/xirang-daily-behaviors`（实测：只有它点名这个示例，而 `tools/xirang-external-rehearsal`
/// 根本不碰它）。
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

#[cfg(test)]
mod tests {
    /// What the host still compiles: `panel` stays with it, and its own plan is non-empty. The
    /// subtrees it handed away are **not** nameable here — by design, and the reason the deep paths
    /// are checked through the tool's records instead (`xirang explain …`).
    /// 宿主还编译什么：`panel` 留在它这里，它自己的计划非空。它交出去的那些子树在这里**叫不出来**——这是设计，
    /// 也正是深路径改由工具的记录来核对的原因（`xirang explain …`）。
    #[test]
    fn the_host_still_owns_what_it_did_not_hand_away() {
        assert!(!crate::panel::NODE_ID.to_string().is_empty());
        assert!(!crate::XIRANG_NAMESPACE.is_empty(), "the host keeps a namespace");
    }
}
