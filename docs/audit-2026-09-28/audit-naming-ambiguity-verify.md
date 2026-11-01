# t35 独立复核：命名歧义与约定冲突专项（`audit-naming-ambiguity.md`，t34）

> 复核者 logic-adversary。本轮只出报告，源码一行不改；写权限只落在本文件。
> 复核对象：`docs/audit-2026-09-28/audit-naming-ambiguity.md`（530 行，作者 bridge-auditor）及其对合并方案模块名的判定。
> **双重身份声明**：我是 `docs/audit-2026-09-28/audit-naming-review.md`（§D D-1…D-11）的作者。因此本轮**额外**检查两份报告的重复计数与口径一致性（§9），但**不因此放松对 t34 的证伪标准**——事实上本轮把 t34 的 R-4 基线判为不合格，理由与 t27 当年打回我的那份完全同型（谓词与计数）。
> 工作树 `HEAD=cf0c378`（120 项未提交改动，与 t34 同一批）。

## 0. 我用什么手段（与作者不同）

| 项 | 作者（t34） | 我（t35） |
| --- | --- | --- |
| 枚举 | 六个命名空间各给一个总数（顶层目录 14 / 包 12 / lib 12 / bin 5 / 模块 271 / feature 8 / 公开名 831），词表比对 + 抽读 | **从零重算**：`cargo metadata --no-deps --offline` 取包/lib/bin/feature；自写正则扫模块声明（**同时数"去重名字数"与"声明处数"**）；公开名用独立正则重数（含/不含测试文件两档） |
| 「生态保留名」依据 | 多为印象式（如"`core` 是标准库 crate 名""`toolchain` 在生态里默认指 rustup/rustc 工具链"） | **只认可核对来源**：`rustc --print sysroot` 下的 std rlib 清单、`cargo --help` 的子命令表、`cargo +<name>` 的报错原文、`~/.cargo/registry` 的存在、写进清单的 feature 声明 |
| 合并方案 | 引 "方案 `:40`/`:62-72`/`:167`/`:89-93`/`:122-126`/`:194`"，**未点名文件** | 逐个核对到 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（并说明为什么不可能是 `audit-boundary-refactor-plan.md`） |
| 严重度 | 4 判据（生态保留名 / 跨生态 / 同族易混 / 一名多义） | 四判据 + **反过火问**："读者真的会先入为主理解错，并据此去**错的地方**找代码吗？" |
| 判定用词 | 是/否 + 建议名 | 证实 / 部分证实 / 证伪 / 无法判定 |

**总体判定：`needs_revision`（R-4 基线不可用作门禁），但 §3 的七个模块名判定已复核通过、可先行用于改合并方案。** 6 条 MAJOR 的**现象**全部成立（其中 AMB-06 应降 MINOR），§4 的「不是缺陷」清单判据基本成立（补 1 条），合并方案的 3 换 4 保留**全部维持**；真正要修的是 **R-4 的现状基线与计数口径**（缺 4 个名字、`build` 谓词 hack、名字数与站点数未区分）以及**与 §D 的一处重复计数**。

---

## 1. 我的独立枚举与差集

### 1.1 我算出来的数（命令与来源都可核对）

| 命名空间 | 我的结果 | 与 t34 的差 |
| --- | --- | --- |
| 顶层目录（不含隐藏/`target/`） | **14**：`build_method cli conventions core debug_method docs examples macro mcp picture plugin-host run_method studio tools` | 一致 |
| 包名 | **12** | 一致 |
| lib 名 | **12**（含两个示例宿主的 `control_button`、`control_button_graft`） | 一致 |
| bin 名 | **5**：`cargo-xirang xirang xirang-dev xirang-mcp xirang-studio` | 一致 |
| feature 名 | **8**：`authoring default dev-supervisor node-graph process-tools prototype-fixtures syntax wasm` | 一致（但它 §6 又写"13 个 feature 名"——那是**合并方案的** 13 个，两处未标对象） |
| 模块名 | 去重 **271** / **声明处 339** | 去重一致；**声明处数它没给**（见 §7 的计数口径问题） |
| 公开名（`pub fn/struct/enum/trait/const/type/mod`） | 含测试文件 **904** / 不含 `*_tests.rs`+`tests/` **902** | 它 **831**（差 8.5%，谓词未写明——见 §10 的 V35-08） |
| 内联 `mod tests {` | **76** | 一致 |

**保留名来源（我的依据，全部本地可复算）**：
- Rust std crate 名：`ls "$(rustc --print sysroot)"/lib/rustlib/x86_64-unknown-linux-gnu/lib/` → `libcore` / `liballoc` / `libstd` / `libproc_macro`（+ `libtest`）——`core` 名列其中。
- Cargo 约定路径名：`src` / `build` / `target` / `tests` / `benches` / `examples` / `bin` / `lib` / `main` / `out`。
- Cargo 子命令（`cargo --help` 原表）：`build check clean doc new init add remove run test bench update search publish install uninstall`。
- **`toolchain` 的直接依据**：`cargo +definitely-not-a-toolchain --version` → `error: no such command: '+definitely-not-a-toolchain'` 且 `help: invoke 'cargo' through 'rustup' to handle '+toolchain' directives`——**`+<name>` 就是 Cargo/rustup 的工具链选择器**，这是"toolchain 在 Rust 生态里已被占用"的硬证据（本机未装 rustup，故无法跑 `rustup toolchain list`，这是本次核验的一个已知限制）。
- `registry` 的生态义：`cargo --help` 的 `search  Search registry for crates` 与 `publish  Package and upload this package to the registry`；本机 `~/.cargo/registry/` 存在。

**按"整名或任一 token ∈ 保留名"相交的结果**：顶层目录 `build_method` / `core` / `examples`；包 `xirang-build-method` / `xirang-core`；lib `xirang_build_method`；bin 与 feature **零命中**。

### 1.2 差集

**它漏的（我补，见 §7/§10）**：
1. **4 个末 token 落在它自己 `bad_tail` 表里的模块名**：`support`（`studio/studio/app/app.rs:34`）、`misc`（`studio/studio/app/state/state.rs:14`）、`data`（`studio/studio/ui/graph.rs:20`）、`face_helpers`（`run_method/macros/face.rs:7`）——它的 R-4a 模块输出是**空的**，这 4 个必须进基线。
2. **`mir` ↔ `mirror` 近名对**：`core/src/registry_core/mir/mir.rs:1`（rustc MIR）vs `macro/src/lib.rs:59`→`macro/src/mirror.rs:1`（"Editor-only field mirror"）。它 §4 判 `mir` 不算歧义（我同意），但**漏了这一对**。
3. **模块名的"声明处数"**（339；裸动词那 16 个名字占 33 处）——门禁若按"名字"钉，改起来是 33 个站点。
4. **方案行号的归属文件**：它只写"方案 `:NN`"。

**它多报的（我降级/移出，见 §3 AMB-06）**：`examples/` 的 MAJOR（Cargo 自动发现机制在**工作区根**不成立）。

---

## 2. MAJOR 6 条逐条复核（全复核）

| id | 复核结论 | 我的依据 | 严重度 |
| --- | --- | --- | --- |
| **AMB-01** `core/` + lib `xirang` | **部分证实（方向对，主理由要换）** | ① lib `xirang` = 项目名（`Cargo.toml:1` 起的 workspace）**= 其中一个 bin 名**（我的 bin 枚举：`xirang`）**= 内核 lib 名**——一个名字三种角色，这是**实害**（`use xirang::…` 分不清"内核"还是"整仓"；docs.rs 链接混淆）。② `core` 确在 std crate 名里（sysroot rlib 实测），但本仓不是 `no_std`，工作区里看到 `core/` 的读者多半读成"本项目的核心 crate"——**这半不足以支撑 MAJOR，只作旁证**。③ 建议 `kernel/`+`xirang-kernel` 与 `AGENTS.md:3` 的 "pure kernel" 同词、与合并方案 `:38`/`:53` 一致 | **MAJOR 维持**（换主理由） |
| **AMB-02** `build_method/` | **证实** | `method` 在 Rust 首读"类型上的函数"（`build_method/` 里没有 `Method` 类型，读者预期落空）；`build` 与 Cargo 家族同词（`cargo --help` 的 `build`、`[build-dependencies]`、`build.rs`）。**`find . -name build.rs` 实测 4 个文件 3 种角色**：`cli/commands/build.rs`（CLI 命令实现）、`core/src/registry_core/diagnostic/build.rs`（私有模块文件）、`examples/control-button/build.rs`、`studio/tests/fixtures/node-editor/build.rs`（真构建脚本）——它的这条证据**逐字成立** | **MAJOR 维持** |
| **AMB-03** `run_method/` | **证实** | `run` 与 `cargo run`（`cargo --help`）、程序入口 `run()` 同词；crate 实际是"运行期那一半"（`AGENTS.md` 该行自述 "Runtime state instance + trace binding"）。另：改名要连带改 `core/src/registry_core/lexicon/lexicon.rs:28` 的 `RUN_METHOD_CRATE = "xirang_run_method"`（全仓 4 处引用）——它的影响面提到该常量 ✓ | **MAJOR 维持** |
| **AMB-04** `plugin-host/` + 方案模块名 `plugins` | **证实（且维护者的猜测被证伪）** | ① `.xirang/plugins` 是真的用户插件锁目录：`plugin-host/src/admission.rs:38` `pub const PLUGIN_LOCK_DIRECTORY: &str = ".xirang/plugins";`——方案的模块名 `plugins` 与它**同词反义** ✓。② `plugin-host` 不是"只跑插件"：`plugin-host/src/verifier.rs:10`+`:53`（`Ed25519Verifier::admit`）、`plugin-host/src/admission.rs:167`（`admit`）、`plugin-host/src/wasm.rs:149`/`plugin-host/src/process.rs:146`（`load`）、`plugin-host/src/lazy_wasm.rs:127`（`install`）、`plugin-host/src/deployment.rs:36`（`new` + `health_check`）——**验签与准入是这 crate 的一半**。→ **队长的猜测"该叫 `plugin-runtime`"证伪**：`runtime` 只覆盖执行半，丢掉验签/准入/部署；保留 `plugin-host` 正确 | **MAJOR 维持**（针对方案模块名 `plugins`） |
| **AMB-05** 公开类型 `Registry` | **证实** | `registry` 在 Cargo 生态有确定义（上文 `cargo --help` 两条 + `~/.cargo/registry/`）；本仓 `Registry` 是注册树根（`core/src/registry_core/tree/registry.rs:18`）。它自己标了 `= D-9 的保留词 registry，计文件 stem vs 计类型名，不重复计数` ✓ 记账正确 | **MAJOR 维持**（修法是限定/词表登记，不必强改公开类型） |
| **AMB-06** 顶层 `examples/` | **部分证伪（机制错，现象可留）** | 根清单是**虚拟清单**且成员是**显式列出**的：`Cargo.toml:1-2` `[workspace] members = [..., "examples/control-button", …]`。Cargo 的 `examples/` 自动发现是**按包**找 `examples/*.rs` / `examples/*/main.rs`，工作区根没有包，**不会**自动发现这两个宿主。"读者会以为这是示例 target"这半成立（现象），"Cargo 自动发现目录"这半不成立（机制） | **MAJOR → MINOR** |

**补充（关于队长的另一处猜测 `mir`）**：见 §8。`mir` 的"撞名"不是与外部生态撞，而是与本仓 `mirror` 撞——**部分成立，但撞的对象与理由都要换**。

---

## 3. MINOR 抽样（14 条抽 7 条，≥1/3）

| id | 复核结论 | 依据 |
| --- | --- | --- |
| AMB-07 `debug_method/` | 证实（现象成立） | 与 `debug_assertions`/`dbg!` 同词头；且合并方案已把它并成模块 `call_evidence`（`:42`），crate 名将消失——**MNOR 维持，无需额外动作** |
| AMB-08 bin `xirang-dev` / feature `dev-supervisor` | 证实 | `studio/Cargo.toml:36-45` 两个 feature 都在；README/注释解释了 `required-features` 即 per-target 的 `publish = false`；合并方案 `:122-126` 保留 bin 名 + `required-features` ✓ |
| AMB-09 顶层 `picture/` | 证实（存在，但**建议降为 MINOR/移出**） | `picture/` 实测只有 `XiRang_studio.png`、`XiRang_wordmark.svg`——**图片目录，没有歧义可言**；"`picture` 单数而装多张图"是优雅问题，不误导（读者不会因它找错地方）。建议移出发现清单或降 MINOR 并只保留"建议改名 `assets/`/`images/`"一句 |
| AMB-10 `docs/ROADMAP.md` 与 `docs/roadmap-1.0.md` 只差大小写 | 证实 | 两份都在（我的目录枚举里可见）；大小写只在 Case-sensitive FS 区分，这是**真实的可移植性风险**（Windows/macOS 默认 FS 上二者会互覆），不是纯命名优雅问题——**建议保留 MINOR 并加一句"跨平台 FS"** |
| AMB-11 内核模块 `syntax` + feature `syntax` | 证实 | `core` 的 feature `syntax` 与 `core/src/registry_core/syntax/syntax.rs` 同名同对象（feature 门控该模块的解析入口）——**同名同物不构成歧义**；它写的是 MINOR（可读性），判定成立 |
| AMB-15 内核 `source` 一名四义 | 证实（抽样核两处） | 内核 `core/src/registry_core/source/`（词法扫描）、`mcp` 的 source 查询、studio 的 `studio/studio/app/source_index.rs`（这一处命名是对的）——**它把 studio 那处列为"命名更好"的对照，判法正确** |
| AMB-17 `index` 一名三义 | 证实 | `core/src/registry_core/tree/tree.rs:19 pub mod index;`（注册树索引）vs `mcp/lib.rs:76 mod index;`（源码/符号索引）vs `Registry::index()`（扁平查找表，AMB-20 的表里有 file:line）——三处同词不同物 ✓ |
| AMB-19 feature 名与模块/包名重名 | 证实 | 我的 feature 枚举 8 个里 `syntax`/`wasm`/`authoring`/`node-graph` 都与模块同名；`AGENTS.md` 明确说"特性名是公开 API、不要为对齐改名"——它的建议（README 对照表而非改名）与 `AGENTS.md` **一致** ✓ |
| AMB-20 13 个"整名就是别处既有词"的公开名 | 证实（抽核 4 个） | `cli/lib.rs:94 pub fn main(args)`（真入口在 `cli/bin/xirang.rs`）、`build_method/lib.rs:137 pub fn run()`、`mcp/protocol.rs:71`、`RuntimeCheckSpec::run`（`core/src/registry_core/declaration/runtime_checks.rs:345`）——四处文件行号**全部命中**；它给的建议（`cli::main`→`run_from_env`、`RuntimeCheckSpec::run`→`check`）方向正确，且与 §D 的 D-5（裸动词只许入口位）**同规则** ✓ |

**抽样结论**：14 条里我核了 9 条，**8 条成立、1 条建议降为"优雅问题"（AMB-09）**；没有发现"把好名字判成缺陷"的严重过火。

---

## 4. 合并方案七个模块名：逐条复核（**这是本轮最要紧的一项**）

### 4.1 先纠一处引用缺陷

t34 全篇写"方案 `:40`/`:167`/`:89-93`/`:122-126`/`:194`"，**没有点名文件**。这些行号属于 **`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`**（我逐行核对命中，见下表）；而 `docs/audit-2026-09-28/audit-boundary-refactor-plan.md` 里**根本没有** `build_time`/`plugin_host`/`call_evidence`/`toolchain` 这些词，它的 §2.1 还写着"成员数 12 → 12、不挪 crate"。我最初按"方案"打开了后者，白查一轮——**这正是"引用必须点名文件"的实例**（V35-05）。

### 4.2 逐名复核

| 方案模块名 | 方案位置（实测命中） | 我的复核 | 结论 |
| --- | --- | --- | --- |
| `build` | `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:40`（crate 表："模块 `build`"）、`:89`（feature `build`） | 与 `cargo build`、`build.rs`、`[build-dependencies]` 同词；改 `build_time` 与 `AGENTS.md` 该行自述 "Build-time filesystem / `OUT_DIR` orchestration" 同词。**注意**：作为"crate 内六个模块之一"，`build` 的误读风险明显低于目录 `build_method/`（同 crate 还有 `run`/`plugins`/`studio`，上下文已限定） | **crate/目录层：必须换（MAJOR）**；模块层：可换（与 crate 同名一致性），单独看属 MINOR |
| `run` | `:41`（模块 `run`）、`:93`（feature `run`） | 与 `cargo run` 同词；改名 `runtime` 与 `AGENTS.md` 同词。它"顺带消掉 `:167` 的 E0428"**属实**（`:167` 原文正是那条碰撞的说明），但**方案自己已经用"只 glob `run` 模块、其余用模块路径"绕开了**（`:176-186`）——所以改名是"少一条只为躲重名而存在的规则"，不是"不改就编译不过"。这个区别必须写清，否则维护者会以为 E0428 是当前阻塞 | **建议换（MAJOR 可维持），但理由要改成"删除一条只为躲重名而设的规则"，不是"否则 E0428"** |
| `plugins` | `:43`（模块 `plugins`） | 与 `.xirang/plugins`（`plugin-host/src/admission.rs:38` 的 `PLUGIN_LOCK_DIRECTORY`）**同词反义**——那个目录放的是**插件锁**，模块放的是**插件宿主** | **必须换 → `plugin_host`（MAJOR 维持）** |
| `call_evidence` | `:42`、`:67`（目录树）、`:195`（挂载行） | 限定词 + 与 `CallEvidence` 同词；t31 已裁定（我 t32 已对齐）；feature 仍叫 `evidence`（`:91`）✓ | **保留（是）** |
| `studio` | `:44` 一带（crate 表） | 产品名；包/bin/README 三处一致（我的枚举：包 `xirang-studio`、bin `xirang-studio`） | **保留（是）** |
| `mcp` | 同上 | 协议缩写，唯一含义 | **保留（是）** |
| `cli` | 同上 | 约定缩写；`cargo --help` 里没有 `cli` 这个子命令，不撞 | **保留（是）** |

### 4.3 `xirang-toolchain` 专项（维护者已选，我给出可核对依据）

- **事实**：`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:1` 的标题就写着 `9 → 3（xirang-kernel / xirang-macro / xirang-toolchain）`；`:16`/`:40` 的目录树用 `kernel/ macro/ toolchain/`。维护者**已经选定**这个名字。
- **我的依据（不是印象）**：`cargo +definitely-not-a-toolchain --version` 的报错原文是 `error: no such command: '+definitely-not-a-toolchain'` + `help: invoke 'cargo' through 'rustup' to handle '+toolchain' directives`——`+<name>` 即 **Cargo/rustup 的工具链选择器**；`rust-toolchain.toml`、`rustup toolchain list` 是同一词汇家族。**限制**：本机没有 rustup（`~/.rustup` 不存在），我无法跑 `rustup toolchain list` 做二次印证。
- **裁决**：t34 的"`toolchain` 在生态里已被 rustup/rustc 占用"**成立**；但**严重度我判 MINOR，不是 MAJOR**——① 它是本仓内的目录/crate 名，读者第一眼看到的是"XiRang 的工具面集合"而不是"一个 rustup 工具链"（`xirang-` 前缀已限定）；② 误读的代价是"以为它对接 Rust 工具链"，不会把人带到错的文件；③ 改名成本是 yank 已发布的 `xirang-toolchain`（若已发布）+ 方案全文重指。
- **给维护者的可执行建议（二选一，我倾向 ①）**：
  ① **保留 `xirang-toolchain`**，在 `README` 首段 + `AGENTS.md` 的 crate 表加一行"本仓的 toolchain = 执行面集合（build/run/call_evidence/plugins/studio/mcp/cli），不是 Rust 工具链"——成本一行，歧义消解；
  ② 若坚持避开：`xirang-surfaces`（用 `AGENTS.md` 已有的 "execution surfaces" 词），代价是方案 `:1`/`:16`/`:40`/`:53` 与工具/CI 全部重指。
  **这条留给维护者拍板**，两者都不阻塞合并方案（无论选哪个，方案其余部分不变）。

### 4.4 影响面核算（它算全了吗）

| 影响面 | t34 是否算到 | 我的核算 |
| --- | --- | --- |
| 已发布公开 API / lib 路径 | 部分 | 它给了 lib 改名是公开路径（AMB-01）✓，但 `toolchain` 合并是**9→3 的 yank 事件**（合并方案 `:1`），比"改一个 crate 名"大得多；它 §3 未提"已发布名会被 yank" |
| feature 名 | ✓ | `:89-93` 的 13 个 feature（我数列：`default build run authoring evidence plugins wasm process-tools node-graph studio cli mcp dev-supervisor`）；它建议保留 `evidence` 与 feature 名 ✓ |
| bin 名 | ✓ | `:122-126` 五个 bin **全部保留** + `required-features` + `autobins = false` ✓ |
| 文档锚点 | ✓（数精确） | **它的四个数我复算完全一致**：谓词 `<dir>/…rs:NN` 于 `docs/**/*.md`（**排除本轮 `audit-2026-09-28/`**）→ `core` **63**、`run_method` **50**、`build_method` **37**、`plugin-host` **10**。**但同一谓词若把本轮报告算进去会变成 `core` 1293 / `plugin-host` 259**（20 倍量级）——"锚点代价"必须写明算不算审计报告，否则数字没有意义 |
| 身份（`NodeId`） | ✓ | 它引合并方案 §6（`NodeId` 不含 crate 名）✓；与我 t32 给 `examples/` 的豁免口径一致（改**路径**才改身份） |

---

## 5. R-4 角色命名规则（**本轮判 needs_revision 的主因**）

### 5.1 谓词回放（我按它的规则重跑）

我把全仓 271 个去重模块名逐个回放它的两条谓词：

| 谓词 | 我的结果 | t34 的结果 | 差 |
| --- | --- | --- | --- |
| **R-4a：末 token ∈ `bad_tail`** | **4 个模块名**：`support`、`misc`、`data`、`face_helpers`（声明处：`studio/studio/app/app.rs:34`、`studio/studio/app/state/state.rs:14`、`studio/studio/ui/graph.rs:20`、`run_method/macros/face.rs:7`） | **空**（它只报了 3 个 `*_method` 目录） | **缺 4** |
| **R-4b：单 token 且是裸动词** | **16 个**：`add apply build converge create delete diff edit explain graft install parse preview render search verify`（共 **33 个声明处**） | **14 个**（少了 `build`、`graft`） | 见下 |

**14 与 16 的差**：它用 `exempt()` 里的 `n == "build"` 与 `toks(n)[0] == "graft"` 排掉了这两个：
- `graft` 排除**有据**（它 §4 把 `graft` 列为领域名词化用法 ✓）；
- **`build` 的排除是"按名字全等豁免"，与它自己写的口径"父路径已消歧"不是一回事**：今天确实只有一处 `mod build;`（`core/src/registry_core/diagnostic/diagnostic.rs:14`，父路径已限定，**豁免是对的**），但**谓词按名字全等豁免，等于今后任何位置的 `build` 模块都自动放过**——而合并方案正要引入一个 crate 级 `build` 模块（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:40`）。正确写法是拿**限定名**（`diagnostic::build`）去豁免，或把豁免写成"父路径是 `diagnostic`"。

### 5.2 计数口径（名字数 vs 站点数）

- 它的"17 个组织单位"实为 **17 个不同的名字**（14 + 3 个 `*_method` 目录）。
- 真改起来是**站点**：16 个裸动词模块名占 **33 个声明处**（`search` 4、`parse` 4、`delete` 3、`apply`/`edit`/`render` 各 2…），再加 3 个 `*_method` 目录 = **36 个站点**（若含 4 个 bad_tail 模块名则 **40**）。
- 门禁基线必须写"单位"，否则"违规 17 → 0"的验收标准无法机械判定（这正是我 t30 修 D-3 时踩过的同一个坑）。

### 5.3 会不会误报好名字

- **会误报一类**：`parse`、`render`、`search`、`diff`、`verify`、`explain`、`preview` 这些模块，读者按名找文件的**命中率其实很高**（`mir/render.rs` 就是"渲染 MIR 的那页"）。按"末 token 必须是角色名词"逐字执行，会强制 33 个站点里的大部分改名为 `parser`/`renderer`/`searcher`/`verifier`——**收益是优雅，不是纠正误导**。
- **建议把 R-4b 拆两档**（可直接写成谓词）：
  - **必改（读成"动作入口"、会把人带错）**：`apply`、`delete`、`create`、`install`、`converge`、`edit`、`add`、`build`（crate 级）；
  - **可选（名词化更好）**：`parse`→`parser`、`render`→`renderer`、`verify`→`verification`、`explain`→`explanation`/保留、`preview`→`preview_copy`、`search`/`diff`（**二者本就是英文名词**，可不改）。
- 它给的具体改法（`parse`→`parser`、`install`→`installer`、`verify`→`verification`、`preview`→`preview_copy`、`cli::main`→`run_from_env`、`RuntimeCheckSpec::run`→`check`）**逐条我都同意方向** ✓，只是不该与"必改"混在同一个 17 里。

---

## 6. §4「不是缺陷」清单复核

| 它判"不是缺陷"的 | 我的复核 | 备注 |
| --- | --- | --- |
| `studio` / `mcp` / `cli` | 成立 | 三处（包/bin/README）一致，我的枚举可核对 |
| `macro` | 成立 | 它**就是** proc-macro crate（`macro/` 的 lib `proc_macro = true`；`cargo metadata` 里 `xirang-macro` 的 target kind 含 proc-macro）；关键字形不影响目录/包名 |
| `conventions` | 成立 | 不在生态保留名内；README 首段只说一件事 |
| `tools/` `docs/` `target/` | 成立 | `target/` 是 Cargo 固定产物目录 |
| **`mir`** | **基本成立，但漏一条** | `core/src/registry_core/mir/mir.rs:1-4` 自述 "Static MIR candidates… parsing rustc `-Zunpretty=mir` text and JSONL artifacts" ⇒ **它确实是 rustc MIR，读者默认含义正确** ✓（队长的"mir 是撞名"因此**部分证伪**）。**但它漏了本仓 `mir` ↔ `mirror`**：`macro/src/lib.rs:59` 挂 `mod mirror;`，`macro/src/mirror.rs:1` 是 "Editor-only field mirror"（编辑器字段镜像）。同一工作区里 `mir`（rustc 中层 IR）与 `mirror`（宏展开镜像）相邻，属**可预防的近名对** → 我补一条 MINOR：词表加一行（`mir`=rustc MIR）或把 `mirror.rs` 改成 `editor_mirror.rs` |
| `face`/`graft`/`trace`/`tree`/`identity`/`declaration`/`diagnostic`/`lexicon`/`authoring` | 成立（抽样核 3 个） | 都是本仓领域词，README/设计文档有定义；`trace` 与 `tracing` crate 只是邻近 |
| `mod tests`（76 处）+ `*_tests` | 成立 | 我实测内联 `mod tests {` = **76** ✓，与它一致；硬禁会产生 76 条噪声 |
| 私有 `mod build;`（`core/src/registry_core/diagnostic/diagnostic.rs:14`） | 成立（豁免对），但**谓词写法错** | 见 §5.1：豁免要按限定名，不能按名字全等 |
| `cargo-xirang` | 成立 | `cargo-<x>` 是 Cargo 插件的唯一合法命名；合并方案 `:122-126` 也保留 |
| `wasm` / `process-tools` / `node-graph` / `prototype-fixtures` | 成立 | 描述性词；`wasm` 只是与引擎名邻近（`wasmi` 是依赖名，不是命名冲突） |

---

## 7. 与 `audit-naming-review.md` §D 的重复计数与口径一致性（我的作者身份检查）

| 重叠点 | 我的 §D 条目 | t34 的条目 | 判定 |
| --- | --- | --- | --- |
| `support` / `misc` | `D-1` 黑名单 + **NAM-01/NAM-02**（**文件 stem**：`studio/studio/app/support.rs`、`studio/studio/app/state/misc.rs`） | `R-4a`（**模块名** `support`/`misc`，声明处 `studio/studio/app/app.rs:34`/`studio/studio/app/state/state.rs:14`） | **同一个对象被两份报告各计一次**（文件与模块同名，是同一个东西）。必须二选一：`support`/`misc` 归 **D-1/NAM-01/02**（我已有具体改法与豁免表），t34 的 R-4a 只做**交叉引用**「= NAM-01/02，不另计」；`data`/`face_helpers` 是新对象，归 t34 |
| `evidence`/`index`/`artifact`/`registry` | `D-9`（**文件 stem** 层：`mcp/evidence.rs` 等） | `AMB-16/17/AMB-05`（**类型/模块名**层，它自己在标题行标了"不重复计数"） | **记账正确** ✓（它主动标了 `= D-9`），但要在**门禁**层面明确：D-9 与 R-2 若各生成一条"违规数"，`mcp/evidence.rs` 会被数两次——**建议合并成一张登记表**，按对象去重（我 §D 的 D-9 行与它的 R-2 段互加交叉引用） |
| `face`/`graft`/`trace` | `D-9` 保留词表 | `R-2` 需限定词表（它写"与 D-9 的并集，不另计"） | 同上：**必须一张表**，否则同一词在两处各有一条"待判定" |
| 裸动词 | `D-5`（**函数名**层，非入口位 15 处） | `R-4b`（**模块名**层，16 个名字/33 站点） | **不重复** ✓（对象不同：函数 vs 模块），它标了"与 D-5 同规则" ✓ |
| 类别名 | `D-1`（文件 stem） | `R-4a`（模块/目录/包/lib 的末 token） | 层不同，但**命中同一批名字时要按上面第 1 行处理** |
| `writers.rs` | 我的 `NAM-06`（t30 修订后：**改名 `write_guard.rs`**，理由：它是"写入守卫"缝） | t34 §2.3 未提；**但 `audit-boundary-refactor-plan.md` 的 R-05 仍写"并入 `mutations.rs` 并删名"** | **两份方案的裁决互相矛盾**（合并/删除 vs 改名保留缝）。t27 的"守卫缝"反证成立，我的 t30 已撤回"合并"——**合并方案 R-05 需要同步修订**。这是**跨报告一致性问题**，不由 t34 负责，但必须挂账（V35-04 备注） |

**结论**：两份报告**没有**在"函数名 vs 模块名"这一维度上重复，但在**同名对象跨层**（`support`/`misc`、四个保留词）上有一处真重复计数与两处"同一词两张表"的风险。

---

## 8. 「查了、干净」口径的诚实性

它的 §6 披露（我逐条对照）：

| 它的声明 | 我的判断 |
| --- | --- |
| 全量机器比对：271 模块名、12 包/lib、5 bin、8 feature、831 公开名 | **诚实**（我复算：模块 271 ✓、包 12 ✓、lib 12 ✓、bin 5 ✓、feature 8 ✓；公开名 831 vs 我 902/904——差 8.5%，但**它把口径交给了读者判断**，且它标了"逐条正则计数"与每 crate 的分解，属可复算） |
| 抽样人读：公开名 13 个整名命中 + 212 个分词命中里抽读若干 | **诚实**（它写了"其余按已带限定词放过"） |
| 只做机器比对：250/271 模块名、≈800 公开名 | **诚实**（它主动列了"没逐个人读"的量） |
| 三个点名案例给了完整三段式，`plugin-host` 的行为链逐文件核实 | **属实**（我按 §2.1 的行号抽查 6 处，全部命中） |
| 与 `audit-naming-review.md` §D 正交、不重复计数 | **方向对，但有一处真重复**（`support`/`misc`，见 §7） |

**未标明的两处口径**（不影响诚实性，影响可复算性）：① 831 的正则没给；② §0 的"feature 名 8"与 §6 的"13 个 feature 名"分别指**现状**与**合并方案**，行文未标。

---

## 9. 作者漏项汇总（我补的）

1. **R-4a 缺 4 个模块名**：`support`、`misc`、`data`、`face_helpers`（§5.1，带声明处行号）——直接影响基线数。
2. **R-4b 的 `build` 豁免谓词错**（按名字全等 ≠ 父路径消歧）——未来 crate 级 `build` 模块会被静默放过（§5.1）。
3. **计数未区分名字数与站点数**（16/33；含 4 项则 40）——门禁基线单位缺失（§5.2）。
4. **`mir` ↔ `mirror` 近名对**（`core/src/registry_core/mir/mir.rs:1` vs `macro/src/mirror.rs:1`）——它把 `mir` 放进"不是缺陷"时漏了这一对（§6）。
5. **方案行号未点名文件**（实为 `audit-publish-surface-merge-plan.md`；`audit-boundary-refactor-plan.md` 无这些名字）（§4.1）。
6. **锚点数未说明范围**：它的 4 个数（63/50/37/10）**在"排除本轮审计报告"口径下逐字正确**；含本轮报告则放大到 1293/259（§4.4）。
7. **与 §D 的 `support`/`misc` 重复计数**（§7）。
8. **未提 `audit-boundary-refactor-plan.md` R-05 与 `NAM-06` 的裁决冲突**（跨报告一致性，非它责任，但应挂账）（§7）。

---

## 10. 复核结论汇总与最后一句

**可以立即据以改合并方案的部分（已复核通过）**：
- §3 的七个模块名判定：`plugins` **必须**换 `plugin_host`；`build`/`run` 在 **crate/目录层必须**换（`build_time`/`runtime`），在模块层可换（理由从"否则 E0428"改为"删掉一条只为躲重名而设的规则"）；`call_evidence`/`studio`/`mcp`/`cli` **保留** ✓；`xirang-kernel` ✓；`xirang-toolchain` 保留 + README 一行（或改 `xirang-surfaces`，交维护者）。
- 6 条 MAJOR 现象（AMB-06 降 MINOR）、§4 清单（补 `mir`/`mirror` 一条）、`plugin-host` 保留（**队长 `plugin-runtime` 猜测证伪**）、`mir` 不是跟外部生态撞（**队长猜测部分证伪**）。

**必须先修的部分（门禁基线）**：R-4 的现状数（缺 4 名、`build` 谓词 hack、名字/站点单位）、与 §D 的重复计数、方案引用点名。

**不可用（列阻塞项）**：**R-4 作为 `conventions` 门禁基线不可用**——① 漏 4 个模块名（`support`/`misc`/`data`/`face_helpers`）；② `build` 的豁免谓词与文档口径不一致，会把合并方案新引入的 crate 级 `build` 模块静默放过；③ "17 个组织单位"未区分名字数与站点数（实为 16 名/33 站点，含漏项 40），"违规 → 0"无法机械判定。**R-4 段落按上述三条修订后，本报告即可作为命名门禁与合并方案改动的权威依据；§3 与 §4 无需等待，可先行执行。**
