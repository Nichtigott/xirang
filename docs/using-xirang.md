# 使用息壤 / Using XiRang

[中文首页](../README.zh-CN.md) · [English overview](../README.md) · [Language extensions / 语言扩展](extending-languages.md)

这份指南围绕当前检出的 Rust 宿主工作流。命令在宿主根目录运行，除非明确指定 `--project` 或 `--path`。较早的发布版可能不包含这里的全部入口。

This guide follows the Rust-host workflow in the current checkout. Run commands from the host root unless a project path is supplied. Older releases may not contain every entry shown here.

## 1. 安装与新建 / Install and create

```sh
git clone https://github.com/Nichtigott/xirang
cd xirang
cargo install --path toolchain --features cli,lang-cpp
xirang new my-app --lib --path "$PWD"
cd my-app
```

`--path` 是 **XiRang 检出位置**，不是新项目的目的地。新项目放在当前目录的 `my-app/`。不带 `--lib` 创建二进制宿主。只需要纯 Rust 时，安装使用 `--features cli`；不要省掉 `cli`，否则没有满足条件的 CLI 二进制。

`--path` points at the **XiRang checkout**, not the new project's destination. The project is created in `./my-app`. Omit `--lib` for a binary host. Use `--features cli` for Rust-only authoring. Without `cli`, installation does not select the CLI binaries.

直接从检出运行 / Run without installing:

```sh
cargo run -p xirang-toolchain --features cli,lang-cpp --bin xirang -- --help
```

发布版 / Released CLI:

```sh
cargo install xirang-toolchain --features cli
```

同一宿主的普通依赖和构建依赖应使用相同来源与版本。源码模式下提交前检查生成的绝对 path 依赖，团队共享时改成约定的相对路径或固定版本来源。

Keep the host's regular and build dependencies on the same source and version. Before sharing a scaffold created with local dependencies, replace machine-specific absolute paths with agreed relative paths or pinned sources.

## 2. 接入已有 Rust 项目 / Add an existing Rust project

在项目根运行，替换两处路径 / Run in your project and replace both paths:

```sh
cargo add xirang-toolchain --path /absolute/path/to/xirang/toolchain
cargo add xirang-toolchain --build --path /absolute/path/to/xirang/toolchain
```

`build.rs` 中接入生成器；已有构建脚本应合并调用，保留原有工作。
Add the generator to `build.rs`, preserving any existing build work:

```rust
fn main() {
    xirang_toolchain::build_method::run();
}
```

在拥有注册树的 crate 根部接线一次 / Wire it once in the crate root that owns the tree:

```rust
xirang_toolchain::run_method::host!();
```

`host!()` 引入生成计划和模块接线。不要再为同一批生成模块手写重复挂载。库和二进制是不同的 Rust crate；graft 声明放在调用 `host!()` 的那个入口中。自定义 `[lib] path` 属于已覆盖布局，但不能因此省略构建入口。

`host!()` mounts the generated plan and modules. Do not duplicate those mounts manually. A library and a binary are different Rust crates: put graft declarations in the one that calls `host!()`. Custom `[lib] path` layouts are supported; they still need the build entry.

## 3. 创建、读取与编辑 / Create, inspect, edit

```sh
xirang face languages
xirang face add panel --lang rust --fields '{"needs_registry":true}'
xirang face add panel --lang rust --fields '{"needs_registry":true}' --write
xirang face show src/panel/panel.rs
xirang check
cargo build
xirang explain root/panel --json
```

第一条 add 是预览，第二条执行。多个语言可用时，显式传 `--lang`，或让 `--file` 的扩展名选择语言。`show` 读取声明字段；`explain` 给出身份、父级和构建投影，并可能明确报告构建已过期。

The first add previews, the second applies. With several enabled languages, pass `--lang` or select one with a destination extension. `show` reads declaration fields; `explain` reports identity, parent, and build projection, including stale build evidence.

编辑默认目录里的工具生成面 / Edit a generated face in the default layout:

```sh
xirang face edit src/panel/panel.rs --lang rust \
  --fields '{"summary_en":"A panel","summary_zh":"面板"}'
xirang face edit src/panel/panel.rs --lang rust \
  --fields '{"summary_en":"A panel","summary_zh":"面板"}' --write
```

仅改请求点名的字段。手写声明的接管是独立政策，不能假定工具能安全重写任何文件。`xirang add` 是多语言生产项目的**依赖添加**命令；注册面创建用 `xirang face add`，两者不是别名。

Only named fields are changed. Adopting handwritten declarations is a separate policy; do not assume every file can be rewritten. `xirang add` adds a **dependency** to a production project. Registration authoring uses `xirang face add`.

## 4. 独立目录 / Source islands

先通过 `xirang explain root/panel --json` 读取 `node.id`，将结果填入 `PANEL_ID`。CLI 的 `--parent` 接受 32 位十六进制 NodeId 或 `root`；MCP 还接受逻辑路径。

Read `node.id` from `xirang explain root/panel --json` and place it in `PANEL_ID`. CLI `--parent` accepts a 32-digit hexadecimal NodeId or `root`; MCP also accepts logical paths.

```sh
PANEL_ID=replace_with_the_node_id_from_explain
xirang face add reading --lang rust --parent "$PANEL_ID" \
  --file components/reading/reading.rs --fields '{"needs_registry":true}'
xirang face add reading --lang rust --parent "$PANEL_ID" \
  --file components/reading/reading.rs --fields '{"needs_registry":true}' --write
xirang check
cargo build
xirang explain root/panel/reading --json
```

结果 / Result:

```text
注册关系 / registration       文件位置 / source
root                         src/panel/panel.rs
└── panel                    components/reading/reading.rs
    └── reading              xirang.objects.tsv  ← 显式落点 / explicit placement
```

给 reading 添加默认子面时，文件落在 `components/reading/object/<name>/<name>.rs`。新的 `--file` 可建立嵌套覆盖。工具维护台账和生成挂载；不要通过手改台账来“修复”身份。

Default children of reading live under `components/reading/object/<name>/<name>.rs`. Another `--file` can establish a nested override. The tools maintain the ledger and generated mounts; manually changing ledger identities is not a migration.

**CLI 孤岛编辑（2026-10-25 修复）**：`face add --file` 能创建 `src` 外孤岛，`face edit` 现在同时接受**包内路径**（如 `components/reading/reading.rs`）和 **NodeId**；身份拼写按布局的 `identity_base` 计算，`src` 外按包内相对，NodeId 分支按身份匹配候选文件而不是按路径存在性猜。回归在 `toolchain/tests/face_island_cli.rs`（`--features cli` 面），覆盖路径与 NodeId 两种入口。

**CLI island edit (fixed 2026-10-25):** `face add --file` creates an island outside `src`, and `face edit` now accepts both its package-relative **path** and its **NodeId**. Identity spelling follows the layout's `identity_base` (package-relative outside it), and the NodeId branch matches candidates by identity rather than guessing from path existence. The regression is `toolchain/tests/face_island_cli.rs` (the `--features cli` face), covering both entry forms.

### 项目之外的目录 / Directories outside the project

源码声明与本机绑定分开：`xirang.sources.tsv` 保存来源 id、种类和包内物化位置；`.xirang/source-bindings.tsv` 保存本机绝对根。共享前者，本机维护后者。当前来源映射支持 `local`；来源未绑定、越界或不明时应明确失败，不能靠“哪个路径存在”选择另一份源码。

Source declarations and machine bindings are separate: `xirang.sources.tsv` records an id, kind, and package-relative materialization location; `.xirang/source-bindings.tsv` records this machine's absolute root. Share declarations, keep bindings local. This map currently supports `local`; missing or invalid sources must fail explicitly.

它首先解决读取与定位，不自动授予包外写权限，也不是任意远程仓库的同步服务。需要编辑第三方代码时，应使用有权限的工作副本并明确版本策略。

This solves reading and location. It does not grant writes outside the package or synchronize arbitrary remote repositories. Editing third-party code requires an authorized working copy and an explicit version policy.

## 5. 嫁接与分区 / Grafts and partitions

从可运行例子学习 / From the XiRang checkout:

```sh
cargo test -p xirang-example-control-button --test registry
xirang grafts examples/control-button --json
xirang explain --overlay --path examples/control-button --json
```

真实类型化声明在 [示例入口](../examples/control-button/src/lib.rs)，第三方实现在 [control-button-graft](../examples/control-button-graft)。`--overlay` 显示静态投影，实际有效树与业务调用还需运行测试。

The [example entry](../examples/control-button/src/lib.rs) declares typed grafts against [external implementations](../examples/control-button-graft). `--overlay` displays a static projection; tests must still exercise the effective tree and application calls.

分区在**自己的宿主副本**里执行。以下假设已有 `control` 子树 / Partition your **own host copy**; this example assumes a `control` subtree:

```sh
xirang check
xirang crates --declare objects --subtree crate::control::SUBTREE
xirang crates --declare objects --subtree crate::control::SUBTREE --write
xirang crates
xirang crates --write
cargo build --workspace
cargo test --workspace
```

声明预览和生成包预览是两步。独立宿主的生成包落在 `<host>/crates/`，已有 workspace 使用规划得到的工作区根。工具负责成员、依赖与 re-export 接线。release 形状使用 `xirang crates --write --release`；查看当前帮助与 [分区演练](../tools/xirang-partition-rehearsal) 后再操作。

Declaration preview and package preview are separate steps. Standalone hosts place generated packages under `<host>/crates/`; workspace hosts use the planned workspace root. The tool wires members, dependencies, and re-exports. `xirang crates --write --release` produces the release shape; consult current help and the [rehearsal](../tools/xirang-partition-rehearsal).

生成 crate 不是权威源码。不要在那里独立编辑后期待 release 保留它；在宿主修改，再重生成。注册对象 move 经 MCP 的 `action: "move"`；CLI 的 `xirang move <unit> <destination>` 属于生产单元目录移动，不能混用。

Generated crates are not authoritative source. Edit the host and regenerate. Registration-object moves use MCP `action: "move"`; CLI `xirang move <unit> <destination>` moves a production unit directory. They are different operations.

<a id="mcp"></a>
## 6. MCP：把工程上下文交给 Agent / Engineering context for agents

通用 stdio 客户端配置形状如下，替换绝对路径；客户端可能采用不同配置位置或外层字段。
A typical stdio client configuration follows; replace absolute paths. Clients may use different configuration locations or wrapper fields.

```json
{
  "mcpServers": {
    "xirang": {
      "command": "/absolute/path/to/xirang",
      "args": ["mcp"],
      "env": {"XIRANG_PACKAGE_ROOT": "/absolute/path/to/my-app"}
    }
  }
}
```

默认工具入口为 `xirang.explore`、`xirang.apply`、`xirang_tools`。以下为工具名与参数，不是 shell 命令。
The default entry points are `xirang.explore`, `xirang.apply`, and `xirang_tools`. These are tool names and arguments, not shell commands.

```text
xirang.explore
{"query":"root/panel"}

xirang_tools
{"tool":"converge","arguments":{"node":"root/panel"}}

xirang_tools
{"tool":"consistency","arguments":{"parent":"root/panel"}}

xirang_tools
{"tool":"status","arguments":{}}
```

获取 schema 时只给 `tool`，执行时再带 `arguments`。`status` 帮你选择 `check` 的测试面；对象路径不是测试面名。
Use `tool` alone to inspect a schema; add `arguments` to execute. Use `status` to select a test face for `check`; an object path is not a test-face name.

新增独立目录子面预览 / Preview an island child:

```json
{
  "action": "add",
  "language": "rust",
  "parent": "root/panel",
  "file": "components/reading/reading.rs",
  "fields": {"module": "reading", "kind": "Reading", "needs_registry": true}
}
```

交给 `xirang.apply`。不要在前一节已创建 reading 的同一宿主重复添加。阅读预览后，同一请求加上 `"apply": true` 才执行；若返回绑定基准的 `plan_id`，后续执行应携带该计划，基准改变后重新预览。真实故障、陈旧计划与回滚示例在 [island-loop](../tools/xirang-island-loop)。

Pass this to `xirang.apply` on a host where reading has not already been created. Inspect the preview, then add `"apply": true` to execute the same request. When a baseline-bound `plan_id` is returned, carry it into execution and re-preview after a baseline change. See [island-loop](../tools/xirang-island-loop) for actual stale-plan and rollback scenarios.

对象搬动预览 / Preview an object move:

```json
{"action":"move","node":"root/panel/reading","to":"root","rebind_grafts":true}
```

`apply` 的普通模式默认预览。预览成功也不代表测试通过；执行之后仍需检查实际树、构建、消费者与行为。候选模式的范围、授权和落地流程由 `xirang_tools {"tool":"candidate"}` 查询当前 schema；不能把调用者填写的 reviewer 名称当成真实独立审查。

Ordinary `apply` defaults to preview. A successful preview does not establish test success; inspect the resulting tree and verify builds, consumers, and behavior. Inspect `xirang_tools {"tool":"candidate"}` for current candidate scope and landing requirements. A caller-supplied reviewer name is not evidence of independent review.

### 上下文与证据 / Context and evidence

`explore` 可接受函数名、类型名、逻辑路径与错误关键词。`scope` 限定查询时，范围外调用方和测试没有被检查。静态调用是候选；实际值来自宿主记录的 trace，没有记录就没有值。

`explore` accepts function/type names, logical paths, and symptom words. A restricted `scope` leaves outside callers and tests unexamined. Static calls are candidates; runtime values require a host-recorded trace.

`known_context: []` 请求内容回执；后续仅传回仍留在模型上下文里的回执。源码变化会重发，责任与运行证据独立刷新。会话切换或上下文压缩后清空回执。减少回复字节不等于已经证明模型账单下降。

`known_context: []` requests content receipts. Return only receipts whose source remains in the model context. Changed source is resent; responsibility and runtime evidence refresh separately. Clear receipts after compaction or a new session. Fewer reply bytes do not establish a billing reduction.

## 7. Studio / TUI

```sh
xirang studio
# 或者 / or:
XIRANG_PACKAGE_ROOT=/absolute/path/to/my-app xirang studio
```

| 键 / Key | 行为 / Action |
| --- | --- |
| `n` | 新建项目 / New project |
| `a`, `e`, `d` | 添加、编辑、删除注册面 / Add, edit, delete a face |
| `g` | 嫁接计划 / Graft plan |
| `/` | 搜索 / Search |
| `Tab` | 切换焦点 / Change focus |
| `j`, `k`, 方向键 / arrows | 导航 / Navigate |
| `r`, `F5` | 重载 / Reload |
| `b`, `F9` | 构建检查 / Build check |
| `q`, `Ctrl-C` | 退出 / Quit |

Studio 使用原生终端的备用屏幕。页面与表单会改变快捷键的上下文，以当前界面的帮助为准。运行证据需要宿主显式记录，空 trace 不是空运行。

Studio uses the terminal's alternate screen. Keys depend on the current page or form; use its visible help. Runtime evidence requires host instrumentation. An empty trace does not prove nothing ran.

## 8. 团队与验证 / Teams and verification

共享源码、合同、项目来源声明、对象台账、版本锁和被接受的规则；本机来源绑定不进共享配置。让任务声明可以修改的对象、公共接口、测试判据和预算。不同任务使用独立工作副本；审查与测试应绑定同一份内容。

Share source, contracts, project source declarations, object ledgers, locks, and accepted rules. Keep machine bindings local. State the objects, APIs, test criteria, and budget a task may change. Use independent working copies and bind review and testing to the same content.

验证分层 / Verification layers:

| 检查 / Check | 能说明什么 / Evidence |
| --- | --- |
| `xirang check` | 当前源码的注册发现与校验 / Discovery and registration validation |
| `cargo build --workspace` | 真实类型和生成接线能编译 / Actual types and generated wiring compile |
| `cargo test --workspace` | 已运行断言通过；注意 0 tests、ignored / Executed assertions passed; inspect zero/ignored tests |
| 运行树、消费者、graft 测试 / Runtime and consumer tests | 实际连接符合预期 / Actual wiring behaves as expected |
| 反例、故障注入 / Counterexamples and faults | 错误会被抓住，失败语义成立 / Failures are detected and handled |
| 独立审查 / Independent review | 重新核对需求、实现与遗漏 / Recheck requirements, implementation, omissions |

开发息壤本身时 / When contributing to XiRang:

```sh
cargo fmt --all -- --check
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo clippy --workspace --all-targets --offline --all-features -- -D warnings
cargo test --workspace --all-features --doc --offline
tools/xirang-publish --check-table
```

离线选项适用于已有依赖缓存的环境；冷环境需联网取依赖。全特性与目标矩阵由 `tools/xirang-test` 承担，真实结构组合另跑 `island-loop` / `partition-rehearsal`；不要用一个测试总数替代这些覆盖。

Offline commands assume a populated dependency cache. A cold environment needs dependency access. `tools/xirang-test` covers the feature/target matrix; `island-loop` and `partition-rehearsal` cover real structural combinations. One aggregate count is not a substitute for these different checks.
