<div align="center">

<img src="picture/xirang-mark.svg" width="72" alt="息壤：从共同基础生长的注册树">

# XiRang · 息壤

**面向长期演进的 Rust 工程：让每个部件的身份、归属与承诺成为可读的事实，供人和 AI 一起扩展、替换与验证。**

[English](README.md) · [Issues](https://github.com/Nichtigott/xirang/issues) · [Discussions](https://github.com/Nichtigott/xirang/discussions)

[![CI](https://img.shields.io/github/actions/workflow/status/Nichtigott/xirang/ci.yml?label=CI)](.github/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/xirang-toolchain.svg)](https://crates.io/crates/xirang-toolchain)
[![License: MIT](https://img.shields.io/badge/license-MIT-d8bb82)](LICENSE)

<img src="picture/xirang-overview.svg" width="100%" alt="息壤将对象的注册关系、源码位置和编译归属分开，以共同事实连接人、AI、编译器与编辑器。">

</div>

息壤把工程的部件写成明确的事实：一个实现是什么（`Soil`）、它挂在哪里（父级 / `Registry`）、它承诺什么（`exports`、`FlowContract`）、以及某个槽位由哪一个兼容实现占据（`graft`）。同一批事实同时驱动构建、CLI、Studio 与 MCP——人和 Agent 读到的是同一套结构。

它面向长期维护的框架、组件库和多贡献者工程；单 crate 的一次性小程序通常用普通模块即可。

> **名字**：息壤是神话里自生长的土壤。这里表达的是一种工程抱负：新部件从既有结构里长出来，并给别人的实现留出位置。

## 安装

```sh
cargo install xirang-toolchain --features cli
```

会安装 `xirang`，以及 `cargo-xirang`、`xirang-studio`、`xirang-mcp`。需要 Rust 1.96.0 或更新。只在检出内工作的 `xirang-dev` 不会被安装。

## 五分钟上手

```sh
xirang new my-app --lib
cd my-app

# 先预览：不带 --write 时 face add 只打印计划。
xirang face add panel --lang rust --fields '{"needs_registry":true}'
xirang face add panel --lang rust --fields '{"needs_registry":true}' --write

xirang check        # 校验注册事实，不证明应用行为
cargo build         # 编译生成的接线
xirang explain root/panel --json
xirang studio       # 交互式浏览与创作
```

宿主的声明就是普通 Rust；可编译的宿主见 [`examples/control-button`](examples/control-button)。

## 一张表说清模型

| 名词 | 它回答的问题 |
| --- | --- |
| `Soil`（注册面） | 这个实现怎样进入共享结构？ |
| `Registry` | 它属于哪里，下一层由谁拥有？ |
| `registry_rule` / `admission` | 子级必须提供什么；这一支允许哪些外部注册路径？ |
| `exports` / `FlowContract` | 它承诺什么接口或替换合同？ |
| `catalog` | 如何把对象与它的编译归属分开引用？ |
| `graft` | 这个槽位由哪个兼容实现占据？ |
| `crates` 分区 | 哪个包编译树的哪一部分？ |

身份（`NodeId`、`StableFaceId`）、源码位置与编译归属是**三个分开的决定**，由共同事实连接。搬声明、搬文件、重新分区是三种不同的操作，各有各的检查。

## 仓库地图

| 路径 | 里面是什么 |
| --- | --- |
| [`kernel/`](kernel) | 协议名词与纯方法；不做文件系统或进程绑定 |
| [`toolchain/`](toolchain) | 执行面：构建、运行、调用证据、插件宿主、Studio、MCP、CLI；共享创作服务 |
| [`multilang/`](multilang) | 语言适配器、各语言 SDK、多语言生产管线 |
| [`macro/`](macro) | Rust 声明的编译期前端 |
| [`conventions/`](conventions) | 可执行的仓库规范（不是发布的运行时依赖） |
| [`examples/`](examples) | 真实宿主：`control-button`、`control-button-graft`、`mixed-shop`、`partitioned-button`、`record-gauge`、`java-gauge`、`go-gauge`、`csharp-gauge`、`c-gauge`、`zig-gauge` |
| [`tools/`](tools) | 演练与门禁：`xirang-test`、`island-loop`、`partition-rehearsal`、`mixed-shop`、`callgraph-matrix`、`external-rehearsal`、`publish` |

## 语言支持

| 语言 | 注册方式 | 构建 | 说明 |
| --- | --- | --- | --- |
| Rust | 原生 | 原生 | 参考实现 |
| C++ | 原生 | 原生 | 可与 Rust 父级组成混合宿主 |
| Python | 创作适配器 | — | 生产集成暂停 |
| C、Zig、JS、TS、Go、Java、C#、Python | 记录文件（`xirang.faces`） | 各自示例 | 各语言自己的构建写出声明记录，宿主读取 |

每种记录文件语言都在 `examples/` 下带 SDK 与真实宿主：构建时由该语言的工具链运行生产者、写出记录，Rust 构建只读记录。

## Agent 与 MCP

```sh
XIRANG_PACKAGE_ROOT=/absolute/path/to/my-app xirang mcp
```

入口包括 `explore`（当前源码与责任上下文）、`apply`（预览结构改动）、`check`、`converge`、`consistency` 与 `xirang_tools`。答案会区分声明、静态候选、构建投影和已记录的运行期证据；发现的关系不是因果证明。

MCP 不是操作系统沙箱：通过 shell 的普通写入不会被阻止，候选落地检查也不是隔离。把答案当作强制保证之前，先读执行模式。

## 校验本检出

```sh
cargo fmt --all -- --check
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
tools/xirang-test          # 每个特性面；灰测试就是缺陷
```

`tools/xirang-test` 会跑默认、all-features、no-default 以及逐特性面，并在某个写下的测试没有任何面跑到时失败。更长的演练：`tools/xirang-external-rehearsal`、`tools/xirang-partition-rehearsal`、`tools/xirang-island-loop`、`tools/xirang-mixed-shop`、`tools/xirang-callgraph-matrix`。

## 现状与边界

- 注册、构建接线、CLI、Studio 与 MCP 已实现并由本检出的门禁覆盖；`0.2.0` 已在 crates.io 上。
- 跨语言注册对 Rust 与 C++ 混合宿主是真实的；其他语言按后端、按能力分别支持，不是统一完备。
- 注册树不是完整调用图：动态派发、FFI 与运行期取值需要观测。没观测到不等于不存在。
- 带固定物化的远程孤岛、分布式编译与性能工作属于后续项，不是当前承诺。

## 参与

欢迎具体的使用场景、最小失败用例、语言适配器与兼容的独立实现。一次贡献应当包含成功与失败路径，并写明它验证了什么。仓库规则见 [`AGENTS.md`](AGENTS.md)，可机械检查的规则在 `conventions/`。

[MIT License](LICENSE) · [Issues](https://github.com/Nichtigott/xirang/issues) · [Discussions](https://github.com/Nichtigott/xirang/discussions)
