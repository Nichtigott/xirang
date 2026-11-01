# 合并批次 2 开工前现算与可执行清单（七合一为 `xirang-toolchain`）

- 现算时间：批 1（`core/` → `kernel/`、`xirang-core` → `xirang-kernel`、lib `xirang_kernel`）**已落地**之后
- 只读引用：`audit-publish-surface-merge-plan.md`（下称 **plan**）§1.3/§1.4/§2.2/§4.2/§4.3/§5.1/§5.3/§8
- 本文件状态：**有阻塞缺口**（见 §7）——下列表格标了「实测」的按当前树重算；标了「引 plan」的是方案原文，**执行前必须按给出的命令重算**。

## 0. 结论速览（实测）

| 项 | 值 |
| --- | --- |
| 待合并 crate（包名） | `xirang-build-method` / `xirang-run-method` / `xirang-debug-method` / `xirang-plugin-host` / `xirang-studio` / `xirang-mcp` / `xirang-cli` |
| `*.rs` 文件数（实测，含 `tests/`） | build_method **50**、run_method **67**、debug_method **5**、plugin-host **16**、studio **92**、mcp **42**、cli **14**（合计 **286**） |
| 目录同名（`<x>/<x>.rs` 且目录递归只有一个 `.rs`） | 七个 crate 内只剩 **studio 3 处**（其余 0） |
| 现存 bin（实测 5 个） | `xirang`(cli)、`cargo-xirang`(cli)、`xirang-mcp`(mcp)、`xirang-studio`(studio)、`xirang-dev`(studio，`required-features = ["dev-supervisor"]`) |
| 现存 feature（实测） | plugin-host `default = ["wasm"]` + `wasm`、`process-tools`；studio `default = ["node-graph"]` + `node-graph`、`prototype-fixtures`、`dev-supervisor`；run_method `authoring`（依赖 `xirang-kernel/syntax`、`dep:syn`）；build_method / debug_method / mcp / cli **无 feature** |
| plan 表格规模（引 plan，执行前重算） | §4.2 「依赖搬家」**9 行**；§1.4 「bin 表」**7 行**；§5.1 「门禁路径」**14 行**，其中 SHIMS 一行为 **16 条钉子** |

## 1. 目标结构

- 目录 `toolchain/`，包名 `xirang-toolchain`，库名 `xirang_toolchain`（与目录/包名一致，`naming` 门禁要求 `xirang-<dir>`）。
- 模块名映射（plan §1.2 + 实测目录）：`build_method/` → `build_time`、`run_method/` → `runtime`、`debug_method/` → `call_evidence`、`plugin-host/` → `plugin_host`、`studio/` → `studio`、`mcp/` → `mcp`、`cli/` → `cli`。
- 迁入 `graft-document`（`docs/ROADMAP.md` 与 `docs/roadmap-1.0.md` 两个名字**都保留、都不得改名**，大小写不敏感文件系统上会互相覆盖）。

## 2. `git mv` 级步骤（重算后的动作，plan §4.2/§4.3）

1. `git mv build_method toolchain/src/build_time`（同理 runtime / call_evidence / plugin_host / studio / mcp / cli —— 七个目录一次搬完，**不留中间态**）。
2. 每个 `toolchain/src/<module>/Cargo.toml` 删除；依赖并入唯一的 `toolchain/Cargo.toml`，按 plan §4.2 的 9 行搬家表（内核依赖改成 `xirang-kernel[syntax]` 形态；`xirang-macro`、`syn` 走 optional + 特性）。
3. 五个 bin：`toolchain/src/bin/{xirang,cargo-xirang,xirang-mcp,xirang-studio,xirang-dev}.rs`，`[[bin]]` 的 `path` 逐个改；`xirang-dev` 保留 `required-features = ["dev-supervisor"]`（实测确认）。
4. 模块挂载：`toolchain/src/lib.rs` 用 `#[path = "<module>/<module>.rs"] pub mod <module>;` 七条（**各模块内部原有的 `#[path]` 只改前缀**）。
5. `[lib] name = "xirang_toolchain"`、`autobins = false`、`default = []`（feature 名全部保留，见 §4）。

## 3. E0428 陷阱（plan §2.2）

`build`、`mcp`、`cli` 各自导出 `run`/`main` 一类的裸动词，而新 crate 有 `run` 模块 ⇒ **根只 glob `run` 模块**（`pub use self::run::*;` 只出现一次），构建期入口写成 **`xirang_toolchain::build::run()`**，不要 `pub use build_time::run`。复核：`grep -rn 'pub use .*::run;' toolchain/src/lib.rs` 只应命中一处 glob。

## 4. feature / bin 红线（实测后要守的）

- feature 名**一个都不许改**（`wasm`/`process-tools`/`authoring`/`node-graph`/`prototype-fixtures`/`dev-supervisor`）；`kernel.syntax` 的引用改成 `xirang-kernel/syntax`。
- `default = []`；`plugin-host` 原来的 `default = ["wasm"]`、studio 的 `default = ["node-graph"]` 必须原样搬到新清单（否则默认行为变了）。
- `autobins = false` + 5 个显式 `[[bin]]`，`xirang-dev` 的 `required-features` 保留。

## 5. 发布工具与 CI 的落点

- `tools/xirang-publish` 两张表（crate 顺序表、依赖表）需把七个包名换成一行 `xirang-toolchain`，并保持**发布顺序 core→…→run_method→studio**（见 §8）；复核 `tools/xirang-publish --check-table`。
- CI（`.github/workflows/*`）：`-p` 参数、`--all-features` 作业、`tools/xirang-external-rehearsal` 的宿主路径、脚手架模板里的 crate 名逐处改；`release_workflow` 门禁会检查上传步骤的 tag ref，别顺手改坏。

## 6. 判别性验收谓词（>0→0）与仅计数项

| 种类 | 谓词 | 复核命令 | 期望 |
| --- | --- | --- | --- |
| **判别性** | 旧包名在活树中的引用 | `grep -rn --exclude-dir=target 'xirang-build-method\|xirang-run-method\|xirang-debug-method\|xirang-plugin-host\|xirang-studio\|xirang-mcp\|xirang-cli' . \| grep -v docs/audit` | 搬迁后 **0** |
| **判别性** | 旧目录名作 crate 目录 | `ls -d build_method run_method debug_method plugin-host studio mcp cli 2>/dev/null \| wc -l` | **0** |
| **判别性** | 旧 `[lib] name` | `grep -rn '^name = "xirang_' */Cargo.toml toolchain/Cargo.toml` | 只剩 `xirang_kernel` / `xirang_toolchain` / `xirang_macro` / `xirang_conventions` |
| **判别性** | 五个 bin 名仍在 | `grep -rn '^name = "\(xirang\|cargo-xirang\|xirang-mcp\|xirang-studio\|xirang-dev\)"$' toolchain/Cargo.toml \| wc -l` | **5** |
| **判别性** | shims 表路径全部仍可读（`missing_shims` 是 panic） | `cargo test -p xirang-conventions --offline shims` | 绿 |
| 仅计数 | `*.rs` 文件数（搬迁前后应守恒） | `find build_method run_method … -name '*.rs' \| wc -l` vs `find toolchain/src -name '*.rs' \| wc -l` | 前后相等（**286**，§0） |
| 仅计数 | 叶子测试名逐名不变（前缀会变） | `cargo test -p xirang-toolchain --offline -- --list \| sort` 前后 `diff` | 只差模块前缀 |
| 红线段 | 身份不可变 | `git status --porcelain -- examples studio/tests/fixtures` | **0 行**（`file!()`→`NodeId` 是宏注入，动了就静默改身份） |

## 7. 三处硬输入（本轮的现算结果，原「阻塞缺口」已全部补完）

### 7.1 SHIMS 表 —— 批 2 的**第 0 步**（现树实测 **17 条**，plan 记 16）

`missing_shims`（`conventions/src/shims.rs:125`）对表里每个文件做 `read_to_string(..).expect(..)`：**文件被搬走而表没同步 ⇒ 门禁 panic，不是 finding**。因此这张表必须在**任何 `git mv` 之前**改好（七个目录搬进 `toolchain/src/<module>/` 时，路径前缀同步换掉；语句本身一字不动）。

| # | 现路径 | 钉住的语句（截断） | 搬后路径 |
| --- | --- | --- | --- |
| 1 | `run_method/lib.rs` | `pub use xirang_kernel::registry_core;` | `toolchain/src/runtime/lib.rs` |
| 2 | `run_method/lib.rs` | `pub use xirang_kernel::registry_core::*;` | 同上 |
| 3 | `run_method/registry.rs` | `pub use xirang_kernel::tree;` | `toolchain/src/runtime/registry.rs` |
| 4 | `run_method/registry.rs` | `pub use xirang_kernel::tree::*;` | 同上 |
| 5 | `run_method/authoring/face_file.rs` | `pub use xirang_kernel::authoring::{FACE_FIELD_COUNT, face_field};` | `toolchain/src/runtime/authoring/face_file.rs` |
| 6 | `run_method/authoring/parse/parse.rs` | `pub use xirang_kernel::authoring::parse::*;` | 同前缀 |
| 7 | `run_method/authoring/context.rs` | `pub use xirang_kernel::authoring::validation::*;` | 同前缀 |
| 8 | `run_method/runtime/runtime.rs` | `pub use xirang_kernel::{COORDINATES_IN_VIEWPORT, Coordinates, …};` | 同前缀 |
| 9 | `run_method/runtime/trace/trace.rs` | `pub use xirang_kernel::CallSite;` | 同前缀 |
| 10 | `run_method/runtime/trace/trace.rs` | `pub use xirang_kernel::declaration::source_file_matches;` | 同前缀 |
| 11 | `run_method/runtime/trace/trace.rs` | `pub use xirang_kernel::TraceMode;` | 同前缀 |
| 12 | `run_method/runtime/evidence.rs` | `pub use xirang_kernel::{CallEdge, EvidenceKind, LogicalCallEdge…};` | 同前缀 |
| 13 | `run_method/plugin.rs` | `pub use xirang_kernel::plugin;` | 同前缀 |
| 14 | `run_method/plugin.rs` | `pub use xirang_kernel::plugin::*;` | 同前缀 |
| 15 | `build_method/syntax.rs` | `pub use xirang_kernel::registry_core::syntax::{…};` | `toolchain/src/build_time/syntax.rs` |
| 16 | `build_method/identity.rs` | `pub use xirang_kernel::registry_core::identity::NodeId;` | 同前缀 |
| 17 | `build_method/identity.rs` | `pub use xirang_kernel::registry_core::identity::IDENTITY_SCHEMA…;` | 同前缀 |

（7/13/14 的路径与语句已按**命名批次 G4a** 后的现树记：`face_file.rs`、`context.rs`、`run_method/plugin.rs`——plan §5.1 写作 `face_manifest.rs` / `validation/validation.rs` / `plugin/plugin.rs`，是旧值。）**复核**：`cargo test -p xirang-conventions --offline shims` 必须绿，且 `grep -c '("' conventions/src/shims.rs` 与搬后路径逐条 `test -f`。

### 7.2 plan §4.2 依赖搬家表（原文抄录 + 按现树重算）

| 来源（旧 crate） | plan 原文 | 现树实测 / 重算 | 性质 | 复核 |
| --- | --- | --- | --- | --- |
| `build_method` | `xirang-core→xirang-kernel[syntax]`、`proc-macro2`、`serde_json`、`syn`（全 optional，挂 `build`） | 内核依赖现已是 `xirang-kernel`（批 1 已落地），只需确认 `[syntax]` 与 optional 特性名随新 crate 保留 | 判别性（旧名引用须 0） | `cargo metadata --offline --format-version 1 \| grep -c xirang-core` = **0** |
| `run_method` | `xirang-macro`（optional，挂 `run`）、`syn`（optional，挂 `authoring`）、`[lints.rust] unexpected_cfgs` | 现树 feature 实测为 `authoring = ["xirang-kernel/syntax", "dep:syn"]`（**没有** `run` 特性）⇒ 合并时按现树写，别照抄旧的 `run` | 仅计数/结构 | `cargo metadata … \| grep authoring` |
| `debug_method` | `petgraph`、`tracing`、`inventory`（optional，挂特性 `evidence`） | 现树 debug_method **无 feature**（实测）⇒ 要么把 `evidence` 特性补进新 crate，要么按现树把这三个依赖改为非 optional；**执行前须裁定**（列此便于裁决） | 结构 | `grep -n 'petgraph\|inventory' debug_method/Cargo.toml` |
| `plugin-host` | `arc-swap`、`ed25519-dalek`（挂 `plugins`）、`wasmi`（挂 `wasm`）、`tempfile`（挂 `process-tools`）、`wat`（dev） | 现树 feature 实测 `default = ["wasm"]` + `wasm = ["dep:wasmi"]` + `process-tools = ["dep:tempfile"]`（**无** `plugins` 特性）⇒ 按现树写；`default` 必须原样带走 | 判别性+结构 | `cargo metadata … \| grep -c 'wasm'` |
| `studio` | `crossterm`、`notify`、`ratatui`、`rataflow`（挂 `node-graph`）、`serde_json`、`[package.metadata.docs.rs] all-features`、`default-run` | 现树 feature 实测 `default = ["node-graph"]` + `node-graph = ["dep:rataflow"]` + `prototype-fixtures` + `dev-supervisor`；**两个 bin 的 `required-features` 必须保留** | 判别性 | `grep -n 'required-features' studio/Cargo.toml` |
| `mcp` | `serde_json`、`[[bin]]`（→ `src/bin/xirang-mcp.rs`） | 现树 bin 名 `xirang-mcp`（实测）；`default-run` 无 | 仅计数 | `grep -n '\[\[bin\]\]' -A 3 mcp/Cargo.toml` |
| `cli` | `serde_json`、两个 `[[bin]]`、`default-run = "xirang"` | 现树两个 bin `xirang` / `cargo-xirang`（实测）；`cargo-xirang` 无 `required-features` | 判别性 | `grep -n 'default-run' cli/Cargo.toml` |

> 行数口径：plan §4.2 的表按 `|` 行数是 **7 行**（不是任务书写的 9 行；9 是我上次按“含表头/分隔行”的口径估的——**以实测 7 行为准**）。

### 7.3 plan §5.1 里仍写 `core` 的三行（已按批 1 后的现树重算）

| 门禁 | plan 旧值 | **现树实测** | 批 2 后应为 |
| --- | --- | --- | --- |
| `purity` | `root.join("core").join("src")` | `conventions/src/purity.rs:291` = `root.join("kernel").join("src")`（**批 1 已改**） | `root.join("toolchain").join("src")`（内核对 I/O 的豁免扫描不变，只换目录名） |
| `mounting` | `root.join("core/src")`（`mounting.rs:25`） | `conventions/src/mounting.rs:258` = `root.join("kernel/src")` | 内核一半保持 `kernel/src`；**新增** `toolchain/src` 一半（`#[path]` 挂载规则不覆盖其它 crate，`toolchain` 需自带同类规则或明示豁免） |
| `naming` | 期望包名 `xirang-<dir>` + 库名形状 | `naming.rs:35/:379` 现写 `xirang-kernel (lib xirang)` 已改为 `xirang-kernel`/`xirang_kernel`；`naming_tests.rs:67-76` 断言 `kernel/`+`xirang-kernel`+`xirang_kernel` 就是规则本身 | 新 crate 必须 `toolchain/` + `xirang-toolchain` + `xirang_toolchain`（无特例） |
| 其余 `core` 残留 | — | 实测 `grep -rn '"core"' conventions/src` 只剩上表与两处测试夹具（`lib.rs:555` 的 `root.join("kernel/src")` 已改） | 搬 `toolchain/` 后重跑 `cargo test -p xirang-conventions --offline` 全绿 |

## 8. 回滚与不可回滚点

**批 2 可回滚（纯机械）**：还原七个目录（`git mv toolchain/src/<module>` 各自搬回 `build_method/` 等）、还原七个旧 `Cargo.toml`（从 HEAD 取回）、还原根 `Cargo.toml` 的 members、还原 `tools/xirang-publish` 两张表与 CI 的 `-p`。命令面：`git status --porcelain` 逐条回退 + `cargo metadata --offline` 复核；**不做 `git checkout/restore/stash`**（本批纪律），用文件级手工还原。

**批 3（发布面切换）不可回滚点**（plan §8）：
1. `0.1.6` 一旦发布（`cargo publish` 到 crates.io），**包名与版本即公开事实**；`xirang-toolchain` 0.2.0 发布后旧 9 名 **yank** 也不可撤销（yank 只影响解析，历史版本仍在）。
2. 前置：发布顺序 **core→…→run_method→studio**（即 `xirang-kernel` → … → `xirang-toolchain`）；`0.1.6` 发布后必须重跑 `tools/xirang-publish --verify-publish --verify-consumers`（需要 index），再执行旧 9 名 yank。
3. 旧 9 名 yank **之前**必须确认新包已能被 `--verify-consumers` 消费；一旦 yank，用户侧的旧 `path`/版本解析会变，属对外行为。

## 9. 给 `audit-merge-batch1-spec.md` 的校正（同轮追加）

在批 1 spec 的 §2 谓词处追加一行：**§2 的宽松谓词 `core/…rs:NN` 需要词边界** —— 应写成 `(^|[^A-Za-z0-9_])core/…rs:NN`，否则那 25 条命中是 `kernel/src/registry_core/…` 里的 **子串假阳性**；加词边界后真锚点是 **24 条 → 0**（已实测）。该校正已写入 `audit-merge-batch1-spec.md` 末尾的「校正（t112）」小节。
