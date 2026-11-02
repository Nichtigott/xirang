# 使用息壤 / Using XiRang

[中文首页](../README.zh-CN.md) · [English overview](../README.md) · [Language extensions / 语言扩展](extending-languages.md)

这份指南围绕当前检出的 Rust 宿主工作流。命令在宿主根目录运行，除非明确指定 `--project` 或 `--path`。较早的发布版可能不包含这里的全部入口。

This guide follows the Rust-host workflow in the current checkout. Run commands from the host root unless a project path is supplied. Older releases may not contain every entry shown here.

从 [六类 AI 工程问题](ai-engineering-workflow.md)理解规则、记忆、候选范围、行为验证与独立审查怎样组合。这里给出实际操作；生产项目与语言后端见[第 9 节](#production)。

See [the six-problem workflow](ai-engineering-workflow.md) for memory, rules, scoped candidates, behavioral verification and independent review. This page supplies operations; [section 9](#production) covers production projects.

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

以下是支持 MCP stdio 的 AI 客户端配置：由客户端启动 `xirang-mcp` 服务进程。它不是 Studio 配置；Studio 是通过 `xirang studio` 打开的人机 TUI。替换绝对路径，配置位置与外层字段依 AI 客户端而定。
This configures an AI client that supports MCP over stdio to launch `xirang-mcp`. It is not Studio configuration: Studio is the human-facing TUI opened with `xirang studio`. Replace absolute paths and follow the AI client’s configuration location and wrapper format.

客户端启动的进程还必须能在 PATH 中找到 Cargo 和需要的语言工具链；只填 MCP 二进制的绝对路径不代表继承了交互式终端环境。包身份查询可能需要 cargo metadata，生产计划也会探测编译器。环境缺失时应读回复中的 unresolvable/unknown，不能只看进程 exit 0。

The client's process must also find Cargo and required language tools on PATH. An absolute MCP executable path does not reproduce your interactive shell environment. Package identity may require cargo metadata; production plans probe compilers. Read unresolvable/unknown verdicts rather than treating exit 0 as success.

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

改名的字段是 `fields.module`。下面是一条独立示例，前提是对象仍在 `root/panel/reading`；先预览，再加 `apply: true` 执行。改名后后续请求使用回复中的新路径。

Rename uses `fields.module`. This separate example assumes the object is still at `root/panel/reading`. Preview, then add `apply: true`; use the returned new path thereafter.

```json
{"action":"rename","node":"root/panel/reading","fields":{"module":"sensor"}}
```

需要完整注册行时，显式请求 `full: true`；默认 registry 回复可以只有层级摘要。

Request `full: true` when you need actual registration rows; the default response may be a level summary.

```sh
xirang-mcp --call registry --json '{"full":true,"freshness":"verify"}'
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

<a id="production"></a>
## 9. 生产项目：依赖到交付 / Production: dependencies through delivery

生产项目使用 `xirang.toml`；注册宿主使用 Cargo + host 接线。两者可以协作，但 `face add`（注册创作）与 `add`（生产依赖）不是同一个入口。

Production projects use `xirang.toml`; registered Rust hosts use Cargo and host wiring. They can cooperate, but registration authoring and production dependency management are distinct.

从一个可编译的 C++ 库开始 / Start with a C++ library:

```sh
xirang new math-kit --lang cpp --lib
cd math-kit
xirang plan --json       # 只预览动作 / inspect actions
xirang build            # 首次生成锁与构建证据 / establish lock and build evidence
xirang test             # 实际运行测试 / execute tests
xirang build --locked   # 依赖图不能悄悄变化 / refuse lock drift
xirang report
```

添加依赖时先准备名称相符、后端支持的真实库。以下是假定旁边有 util 库的示例，不应在没有该项目时直接粘贴：

Dependencies must name actual compatible library units. The following assumes a sibling util library:

```sh
xirang add util --path ../util
xirang update --check
xirang build
xirang test
```

生产计划会给出实际命令与输入输出；编译器、ABI、导出类型与依赖组合按后端能力验证。支持记录生产，不等于支持该语言任意包管理器或整个生态的构建。

Plans expose actual commands and inputs/outputs. Compiler, ABI, export types and dependency combinations are backend-specific; record production does not imply arbitrary ecosystem builds.

在已完成构建的 math-kit 中打包与安装 / Package the already-built library:

```sh
xirang package --output ../math-kit-package
xirang verify ../math-kit-package
xirang install ../math-kit-package --prefix ../math-kit-install
xirang history --prefix ../math-kit-install
xirang log verify --prefix ../math-kit-install
```

package 不代替 build；它需要已有产物。install 将经核验的交付物放入安装前缀的一代。安装了后续一代后，`xirang rollback --prefix ../math-kit-install` 才有前一代可切回；它不是 Git 回滚，也不会回滚你的源码编辑。

Packaging consumes build products. Installation creates a verified generation. Rollback needs a previous generation and changes the installed generation, not Git history or edited source.

远程、队列与自定义后端是可选路径。先在本地跑通同一计划，再通过 `xirang --help` 与后端握手确认支持范围。MCP 的生产入口只读，不执行这些构建：

Remote workers, queues and custom backends are optional. Establish a local plan first and inspect current capabilities. MCP's production entry is read-only:

```sh
xirang-mcp --call multilang --json '{"action":"inventory"}'
xirang-mcp --call multilang --json '{"action":"plan"}'
xirang-mcp --call multilang --json '{"action":"report"}'
```

<a id="remote-workers"></a>
### 远程 worker 与团队验证 / Remote workers and team verification

先在上面的 `math-kit` 生产项目中用本机 worker 验证协议。`xirang` 需在 PATH 中；这会实际执行构建与测试，不是远程主机验证：

First exercise the protocol with a local worker in the production `math-kit` project above. Put `xirang` on PATH. These commands execute builds and tests locally; they do not demonstrate a remote host:

```sh
xirang remote --worker 'xirang worker' --locked .
xirang remote --worker 'xirang worker' --test --locked .
xirang remote --worker 'xirang worker' --evidence .
```

团队准备好远程环境后，可换成 SSH 预设。以下 `build-host` 是需替换的主机名；预先配置非交互 SSH 身份认证及可信主机密钥，worker 机器上需有 `xirang`、对应编译器与依赖。

Once the team has provisioned a remote environment, use the SSH preset. Replace `build-host` with your worker hostname. Configure noninteractive SSH authentication and trusted host keys beforehand; install XiRang, compilers and dependencies on the worker.

```sh
xirang remote --ssh build-host --locked --timeout 300 .
xirang remote --ssh build-host --test --locked --timeout 300 .
xirang remote --ssh build-host --evidence .
xirang remote --ssh build-host --list .
xirang remote --ssh build-host --fetch-all . --into ../worker-artifacts
```

客户端发送本地解析后的项目绝对路径，当前协议不会上传源码。两端应在同一路径准备同一版本的源码、锁文件与所需依赖；路径不同需自行提供转换任务目录的命令传输包装。默认取回根为项目的 `target/xirang/build`，`--root` 可指定 `target/xirang` 下另一个受限根。先列出产物，再决定取回范围。

The client sends the locally resolved absolute project path, not a source upload. Prepare the same revision, lockfile and required dependencies at the same path on both machines, or provide a command-transport wrapper that maps the job directory. Retrieval defaults to the project's `target/xirang/build`; `--root` selects another restricted root under `target/xirang`. List artifacts before choosing what to retrieve.

孤岛解决源码落点与逻辑关系的分离，graft 选择团队提供的兼容实现，add_crates 组织编译与交付包，worker 执行构建与验证。版本同步、访问权限与团队审批由所使用的仓库和 CI 流程承担；这些能力不会自动合并冲突或证明未运行的测试。

Islands separate source placement from logical relationships; graft selects compatible team implementations; add_crates organizes compilation and delivery; workers execute builds and verification. Repositories and CI retain responsibility for revision synchronization, access control and team approval. These capabilities do not merge conflicts automatically or prove unexecuted tests.

<a id="knowledge"></a>
## 10. 规则、记忆与任务 / Rules, memory and tasks

先发现具体 schema，避免把一段自然语言直接当可执行规则：

Discover the schema before treating a natural-language policy as an executable rule:

```sh
xirang-mcp --list rules --full
xirang-mcp --list adopted --full
xirang-mcp --list task --full
xirang-mcp --call rules --json '{"action":"list"}'
xirang-mcp --call adopted
xirang-mcp --call task --json '{"action":"list"}'
```

| 步骤 / Step | 入口 / Entry | 意义 / Meaning |
| --- | --- | --- |
| 提议规范 / Propose | rules: propose | 候选不自动约束 / A proposal is not accepted authority |
| 采信依据 / Adopt | adopted + confirm + apply | 保存依据文件和指纹；属暂时采信 / Record evidence and fingerprints provisionally |
| 接受规范 / Accept | rules: accept + authority | 要求有效采信依据 / Require in-force adoption authority |
| 读回或重查 / Recheck | rules: list/check；conformance | 输出过期、冲突、例外或未知 / Report stale, conflicting, exempt or unknown |
| 验证改动 / Verify | check 的计划 / check plan | 执行约定的配置、测试与义务 / Run declared configurations and obligations |
| 建立任务 / Record task | task: open | 关联对象与已存在的验证计划 / Bind object and existing verification plan |
| 恢复 / Resume | task: status/resume | 重算义务，必要时恢复验证 / Recompute and resume checks |
| 关闭 / Close | task: close + confirm + apply | 五项义务成立后记录结论 / Record closure only when five obligations hold |

task open 的示例参数如下；这里的 verification 必须换成 check 实际输出的计划 id，不能自己编一个。写任务记录不会替你执行之前未跑的验证。

Replace verification with the plan id actually produced by check. Recording a task does not run missing verification.

```json
{
  "action": "open",
  "task": "panel-change",
  "object": "root/panel",
  "verification": "REPLACE_WITH_CHECK_PLAN_ID",
  "apply": true
}
```

交给 `xirang.task`，或通过 `xirang_tools` 携带 tool 与 arguments。随后查询 status；只有义务满足才 close。后续任务可用 from 引用已关闭任务，但会重查其依据，不能把旧结论当永久授权。

Invoke via `xirang.task` or `xirang_tools` with tool and arguments. Inspect status before close. A later task can inherit from a closed one, subject to current evidence.

维护规则目前只对支持的字段和操作执行相应检查。命名、注释或业务语义需要具体检查器或审查；一个任务的 verified 范围也仅限它关联的计划。完整团队路径见[六类问题与执行流程](ai-engineering-workflow.md)。

Rules enforce supported predicates and operations. Naming, documentation and business semantics need concrete checks or review. Verification is bounded by the attached plan.

<a id="vocabulary"></a>
## 11. 名字与模型 / Vocabulary

| 名字 / Name | 在这里的含义 / Meaning here |
| --- | --- |
| Soil / 注册面 | 一个实现进入工程结构的声明 / An implementation's registration declaration |
| Registry / 注册树 | 对象的父子关系与可查询注册事实 / Parent-child relationships and registration facts |
| catalog / 对象目录 | 逻辑对象到身份常量的公开路径 / Logical object paths to identity constants |
| NodeId | 源码实例身份；不等于永久业务对象 id / A source-instance identity |
| StableFaceId | 台账中的持续对象身份，可关联多个源码实例 / Persistent ledger identity across source instances |
| graft / 嫁接 | 指定兼容实现承担某个槽位 / Assign a compatible implementation to a slot |
| island / 孤岛 | 逻辑归属与默认源码落点分离的放置方式 / Explicit placement outside the default source location |
| fragment / 碎片 crate | 分区后负责编译某些子树的生成包 / Generated package compiling claimed subtrees |
| facade / 门面 crate | 汇合跨包目录、注册与计划的生成入口 / Generated entry joining cross-package catalogs, registrations and plans |
| adoption / 采信 | 带依据与失效条件的暂时接受 / Provisional acceptance with evidence and invalidation |
| candidate / 候选 | 带作用域和基线的改动副本 / Working copy bound to scope and baseline |
