# 迁移说明

[English](migration.md) | 简体中文

## 从旧的多功能布局迁移

注册协议位于 `xirang-kernel`；构建发现、身份缓存、scope 和 `StaticPlan` 位于 `xirang-toolchain`；MIR 和数据流证据位于 `xirang-toolchain`；TUI 位于 `xirang-toolchain`；隔离插件执行位于 `xirang-toolchain`；统一命令行入口（`xirang new/check/build/snippets/explain/grafts/studio/mcp`）位于 `xirang-toolchain`。

应用通过 `Registry::root_for_namespace` 提供自己的根 Registry。release 使用宿主生成的 `StaticPlan`，本 workspace 不提供固定的 `root_registry`。

## 外部声明

跨 crate 声明使用 `xirang_toolchain::run_method::external_object!`。B3b 之后只有 `kind` 必填，其余字段全部可选，并按生成的紧凑形式取默认值：`source` 取声明文件、`registry_name` 取模块名末段、`parent` 取包根、`handle` 取 `kind`、`preset`/`parts` 取 `NoPreset`/`NoParts`、`plugin` 取 `None`、`exports`/`requires`/`provides`/`runtime_checks` 取空。没有版本字段，也没有目标框架字段：框架属于注册机而不属于注册面。`registry_rule` 默认取 `RegistrationRule::ANY`，而不是宿主的兄弟规则解析器——该解析器（`super::registry_rule::REGISTRATION_RULE`）是构建只会在宿主自己的生成树里、注册面旁边生成的相对路径；外部 crate 没有这个兄弟模块，因此拥有注册机的外部面得到宽松的 `ANY`，要收窄必须显式写出规则。开发构建只有在宿主显式启用 `collector: debug` 时才发现这些声明；core 本身不依赖 inventory。release 应通过已验证插件 artifact 或应用自有输入保留外部注册面。

## 注册面按真实路径载入

构建期渲染以前会把每个注册面复制到 `OUT_DIR/registration_sources`，再 `include!` 那份副本，于是编译器和编辑器解析到的模块是构建产物，而不是正在编辑的文件。现在注册面以真实模块载入：

- **叶子面**（不拥有子注册机）写成 `#[path = "..."] pub mod <name>;`，公开模块路径与 `type_name` 完全不变；
- **拥有子注册机的面**必须同时充当容器，因此载入同名子模块再重导出（`mod <name>; pub use <name>::*;`）。只有这一种情况会多一段模块路径，例如 `app::control::control::Control`。

两个后果：

- 面文件仍是普通模块文件，因此可以继续以 `//!` 或任何 inner attribute 开头。`include!` 展开无法引入它们——这正是旧设计必须在副本里把 `//!` 改写成 `//` 的原因。
- 声明宏改为从 `file!()` 推导 `source`、从 `module_path!()` 推导注册机名，生成树因此不再注入 `__REGISTRATION_SOURCE` / `__REGISTRATION_MODULE_NAME`。不在 `src/` 下的声明（例如集成测试）保留 cargo 记录的路径，而不是让剥离失败。

只有 `#[path]` 对 IDE 还不够。rust-analyzer 只在 `mod` 声明位于文件或展开的顶层时才应用该属性，因此声明在生成内联模块里的注册面——根以下的每一个面——永远进不了 crate：没有补全、没有跳转、没有悬停，尽管 `cargo` 与 `rustc` 都能看到它。所以每个这样的面还会得到一份仅供 IDE 的视角：

- 真实声明由 `cfg(not(rust_analyzer))` 把关；
- 顶层再加一条 `#[cfg(rust_analyzer)] #[path = "..."] mod __xirang_ra_<路径>;`，在 rust-analyzer 真正应用 `#[path]` 的位置载入同一个文件；
- 在注册面的真实位置加 `#[cfg(rust_analyzer)] use crate::__xirang_ra_<路径> as <名字>;`，重建它的模块路径，可见性与真实声明一致（叶子面是 `pub`，容器面是 `pub(crate)`）。

两种工具各自只看到其中一份声明，因此 `rustc` 读到的树与改动前完全一致，用户可见行为不变。`build_method` 会输出 `cargo::rustc-check-cfg=cfg(rust_analyzer)`，让多出来的 cfg 保持安静。

## 注册面字段可以任意顺序书写

手写声明过去会因为一个分隔符而失败：`kind: Tool;` 报 `no rules expected ';'`，字段顺序写反同样失败。现在两个读取者都宽容，并且共用同一个切分器（`split_face_fields`），因此构建步骤与编译器读到的字段永远一致：

- `,` 与 `;` 都算分隔符，漏写分隔符会在下一个 `name:` 处收尾，末尾多余的分隔符被忽略，顺序任意；
- 词表里没有的字段、或重复给出的字段，由 `xirang-macro` 前端报在该 token 上，并列出可接受的字段；
- 合法声明永远不会走到该前端：它是宏阶梯的最后一条 arm，展开与从前逐字节一致。

有一条限制要说清：前端的诊断挂到转发 token 的那个生成别名上，面文件以"宏调用处"的形式附注。`macro_rules!` 这一跳会丢掉作者 token 的 span，因此手写注册面无法把脱字符精确落在出错的那个 token 上。

## 编辑器能补全面字段

宏的 token 树对编辑器是不透明的，因此 `crate::control_object! { … }` 的大括号里过去什么都补不出来——既没有字段名，连值位置的路径补全也没有。两处改动在不改变作者语法的前提下解决了它：

- 每个生成的 `*_object!` 别名把作者的 token 交给 `face_fields_mirror!`，`external_object!` 则经 `face_fields!` 到达同一个发射器；两者都构造真实字段列表；
- 这次调用带 `#[cfg(rust_analyzer)]`，而它展开出的镜像本身是合法且类型正确的 Rust，因此会编译镜像的编辑器不会在作者的行上报错：写成表达式的字段保留 token、由推导定型；命名类型的字段（`kind`、`preset`、`parts`、`handle`、`flow_provider`）移入类型注解；镜像无法定型的字段（`name: { zh: "…", en: "…" }`、`requires: [a => b]`、`registry_name: slot`、`getting_from_other_registry: None`、空列表）保留字段名，值改为 `loop {}`。

`FaceFields` 是作者侧词表的隐藏镜像，按宏接受的顺序声明，每个字段各有一个类型参数。没有任何宿主代码构造它：镜像是一个 `fn`，编辑器在 `cfg(rust_analyzer)` 下编译它，`rustc` 从不展开它；列出还没写的字段名、回答值位的正是它。

该 cfg 属于**被编辑的那个 crate**。宿主通过自己的 build script 声明它——`build_method` 会输出 `rustc-check-cfg`；没有 build script 的外部实现则在自己的 manifest 里声明：

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(rust_analyzer)'] }
```

注意不能把这个 cfg 挪进 `run_method`：编辑器只对**自己直接分析的 crate** 置 `cfg(rust_analyzer)`，对从 git 解析来的依赖不置，于是依赖里定义的宏会走编译器分支、什么也补不出来。

身份不变：`source`、`registry_name`，以及由此而来的 `NodeId`、注册路径和全部 graft 选择器都保持原值。

用 `static_graft_plan!` 声明的 graft 计划现在无论宿主是否同时声明 `application!(entry = ...)`，都会从宿主入口被捕获——与 `SourceScope` 原有的入口解析保持一致。此前缺少 `application!` 时计划会被静默丢弃。

示例见 `examples/control-button/`（README 那棵 Control/Button 树作为真实宿主）与 `examples/control-button-graft/`（在其上做 graft 的项目外实现）。

## Studio 与插件宿主

使用 `cargo run --manifest-path studio/Cargo.toml` 启动 Studio；`1`～`3` 切换 Search、Inspect、Data。`watch` 由 `xirang-dev` 提供，它是非默认 `dev-supervisor` 特性下的工作区专用二进制：会在源码、Cargo 或插件目录变化时重建子 Studio 进程，因此需要本检出（`cargo run -p xirang-toolchain --features dev-supervisor --bin xirang-dev -- watch`），`cargo install xirang-toolchain --features cli` 也不会安装它。

插件宿主只接受 `VerifiedPluginArtifact`。Wasm slot 在编译期声明；进程适配器需显式启用 `process-tools`。现有 `PluginManifest` 流程合同和 lock 记录保持兼容。
# Graft overlay 迁移

当前 graft 模型是不可变覆盖层。宿主保留原始源码，只在入口声明外部实现：

```rust
let plan = xirang_toolchain::run_method::graft_plan!(framework,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/layout"] full graft "layout_v2",
);
let effective = base.overlay(&plan, &external)?;
```

`cut A graft X` 替换一个逻辑槽位，并继承 A 原来的子节点；`cut A full graft X`
替换 A 的整棵子树；范围 cut 则选择同一父注册机下连续的兄弟。原树和外部树都
不会被修改，只有在数据流合同、注册规范、准入和连接器全部通过后才发布有效视图。

旧的 `Registry::graft` API 和 Studio 源码复制流程已经移除。公开执行路径统一为
`GraftPlan`，再调用 `Registry::overlay`。

## `graft.plan` 记录

`.xirang/external-grafts/<selector>/graft.plan` 是创作记录，既不是编译器读取的声明，
也不是一次 `overlay` 调用。它的版式未变（`version=1`、`target`、`target_path`、`graft`、
`full`），但现在有了读取方：kernel 的 `GraftPlanDocument` 负责解析与渲染，遇到不认识的
版本或键就拒绝而不是猜，并且是这套版式唯一的定义处。记录现在也是覆盖的**输入**：
`xirang_toolchain::run_method::apply_recorded_grafts` 读取 `.xirang/external-grafts/`，
`Registry::overlay_recorded` 把每条记录与静态声明对账、施加，并报告每一处调整；优先级
策略与哪些报告是致命错误见 [`graft.zh-CN.md`](graft.zh-CN.md)。`xirang-toolchain` 新增
`declared_grafts`/`host_entry_source`，供创作界面查询构建会发布哪些槽位。

`xirang_toolchain::run_method::ExternalGraftPlanFile` 不再把 `target`、`graft`、`full` 暴露为
公开字段；它携带解析后的 `GraftPlanDocument` 与选择器，并通过 `target()`、
`target_path()`、`graft()`、`full()`、`plan_path()` 回答。`root` 仍是公开的 `PathBuf`
字段（没有 `root()` 方法），`selector` 与 `document` 同样是公开字段。新增读取、列出、
改范围和回收计划的函数：`read_external_graft`、`list_external_grafts`、
`rewrite_external_graft`、`remove_external_graft`。`create_external_graft` 签名不变，
但改为经 kernel 文档写入，因此写出的内容一定能读回。
