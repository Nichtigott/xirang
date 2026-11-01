# 命名歧义与约定冲突专项（audit-naming-ambiguity）

- 范围：**目录名 / crate 名 / lib 名 / 模块名 / feature 名 / bin 名 / 公开 API 名**，外加
  `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（下称**方案**）§2 的 7 个执行面模块名复核。
- 本轮按维护者给的**总方法**组织（见 §0）：每条先写**行为清单**（带 `file:line`），再写**角色一句话**，
  最后判**现名是不是这个角色**并给建议名。四问判据（生态保留名 / 跨生态 / 同族易混 / 本仓一名多义）
  保留为**辅助判据**（「读者会先当成什么」）。
- 与上一轮的分工（不重复计数）：`audit-naming-review.md` 查「名字说少了」「名字说谎」（§D 在 `:387`，
  D-1…D-11 管**文件 stem 与函数名**）；本轮查**「名字说成了别的」**，对象是**命名空间单位**
  （目录/包/lib/模块/feature/bin）。凡与 §D 同源的在正文标 `= D-n`，**不计入对方违规数**。
- 本轮只出报告，源码一行未改；所有计数都由 §5 的脚本复算。
- **引用约定**：凡引用合并方案，一律写全名 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（下称也用它），不单写行号。

## 修订记录

### 本轮（t39，按 t35 复核结论修 R-4 及其连带项；§3/§4 未改）

| 复核项 | 处置 | 位置 |
| --- | --- | --- |
| **V35-01（阻塞）R-4 漏名** | 补齐 `data`、`face_helpers`、`misc`、`support`，并追加谓词同样命中的 `test_support`；新增 **R-4c 近名对**（`mir`↔`mirror` 等 7 对）。**根因**：上一版的末-token 谓词只写在 §5 的脚本里，而贴出的"本次输出"取自更早那次**只判首 token** 的运行——规则从未真正执行过。本轮重跑并把脚本输出原样贴入 | §5 R-4a/R-4c |
| **V35-02（阻塞）`build` 豁免判据自相矛盾** | 撤销"名字全等"豁免，改为**父路径限定对** `ALLOWED_QUALIFIED = [("diagnostic","build")]`（可机械判定，含自测 4 例）；新判据下 crate 级 `build`（合并方案 `audit-publish-surface-merge-plan.md:194` 引入）被**报出**，`diagnostic::build` 仍豁免 | §5 R-4「豁免」 |
| **V35-03（阻塞）名字数 vs 站点数** | R-4 拆成三个可判定指标：**R-4a 名字数 12 / 声明处数 12**、**R-4b 名字数 15 / 声明处数 26**（命中 16/27，扣除 `build` 的父路径豁免 1 处）、**R-4c 7 对 / 13 名**；删除"净违规 17"这类把指标混起来的单一数字 | §5 R-4 现值 |
| ① 与 §D 重复计数 | `support`/`misc` 标注 **= D-1 的 NAM-01/NAM-02**（同一对象），R-4a 只算一次并给双口径 | §5 R-4a |
| ② 两张词表合成一张 | D-9（文件 stem 对象）与 R-2（命名空间名对象）合并为**一张保留词表**，逐行标出两个对象各自的现状与处置 | §5 R-2 |
| ③ 引用点名文件 | 全文 **14 处**合并方案引用改为"文件全名＋行号"（其中 10 处原有 `方案` 前缀，4 处只写了裸行号） | §1/§3/§5 等 |
| ④ `AMB-06` 降级 | MAJOR → **MINOR**（顶层 `examples/` 是成员不是 target：不误导"去哪找"，只与 Cargo 目录约定冲突） | §1 AMB-06 |
| ⑤ 对象未标清 | 枚举口径写明：模块名 **296**（含 inline `mod x {`；上一版 271 只扫 `;`）＋ **440 声明处**；feature **8** 是"全部 12 个清单去重并集（含 `default`）"；831 的谓词与范围写明（10 crate 的 `src/**/*.rs`，**含** `*_tests.rs`；排除后为 829） | §0 表、§6 |

`[机器]` 自检（本轮，t39）：锚点 **0 violation**（116 处 `file:line` + 16 处配对 token）；`cargo test -p xirang-conventions --offline` = **99 passed / 0 failed**（见 §6）。

### 本轮（t41，按 t40 复判修 V35-02 残余 + 3 条 low）

| 复核项 | 处置 | 位置 |
| --- | --- | --- |
| **V35-02（硬阻塞）豁免按名字判、未按站点判** | 豁免**下沉到逐站点**（`for (p,i,parent) in ordered`），违规列表存 `(名字, file:line, parent)`，"名字数"改为"有 ≥1 个违规站点的名字数"，与 R-4b 的验收词（声明处数 = 0）一致；新增**多站点自测**：同一份代码在 `SITE_ORDER=walk` 与 `SITE_ORDER=reverse` 下各跑一次，两跑裁决相同（报出 1 处 / 豁免 1 处），并附"旧逻辑只看首站点"在 reverse 序下整体豁免的对照 | §5 R-4b + 自测输出 |
| **V40-01（low）R-4c 会误伤好名字** | 复核人**撤回**上一版"R-4c 不会误伤"（`app`↔`application` 两侧都是好名字）→ 新增 `ALLOWED_PAIRS = {("app","application")}` 登记豁免并附一行理由；验收词同时改写为"改名落在更短/更泛的一侧 **或** 已登记 + 一行理由"；R-4c 现值 7 对 → **6 对 / 12 名** | §5 R-4c |
| **V40-02（low）33→27 的差额归因** | 删掉"与**声明形式**"那半句：差额**全部来自动词表**（`graft` −7、`walk` +1 = −6），inline `mod X {` 对 R-4b 贡献 **0 处** | §5 R-4b 对照段 |
| **V40-03（low）算术不自洽** | §6 的"296 个模块名里的 270 个"改为 **275**（296−275=21 = 被点名的模块名数，与复核人复算一致） | §6 |
| **记账：复核人撤回的两处** | ① 上一版"漏 4 名"更正为 **5 名**（t40 自己承认它的扫描漏了 inline 模块与 `pub(crate)` 前缀——本轮追加的正是 `test_support`）；② 撤回"R-4c 不会误伤好名字"。它认可的那条根因披露**保留**：上一版贴出的输出来自**只判首 token** 的运行，规则从未真正执行 | 本节与 §5 |

## 0. 方法（维护者的三段式）与枚举规模

**三段式（每条缺一段即不合格）**：
1. **行为清单**：它**实际干什么**，3–6 条，每条带 `file:line`；
2. **角色一句话**：`它是「……的东西」`（名词短语，说明职能与组织方式）；
3. **命名**：现名**是否就是**这个角色（是/否 + 差在哪）→ 建议名（角色名；`build`/`run` 这类**无宾语动词不是角色名**）。

**枚举规模**（`[机器]`，谓词与命令见 §5）：

| 命名空间 | 全量数 | 对象与来源 |
| --- | --- | --- |
| 顶层目录（tracked，排除 `target/`） | 14 | `ls -d */` 减去构建产物目录 |
| 包名 / lib 名 / bin 名 | 12 / 12 / 5 | `cargo metadata --no-deps --offline` 的 `packages[].name` 与 `targets[].kind` |
| 模块名 | **296 个名字 / 440 个声明处** | 全树正则扫 `mod X;` **与** `mod X {`（含 inline 模块；上一版只扫 `;` 得 271，本轮改口径，见修订记录 V35-03） |
| feature 名 | **8** | 全部 12 个清单 `[features]` 去重后的名字**并集**（含 Cargo 保留名 `default`）；合并方案为**新 crate** 设计的 13 个（不含 `default`；含则为 14）是**另一个对象**，见 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md` §1.3 |
| 公开 API 名 | **831** | 谓词 `^\s*pub (?:async )?(fn\|struct\|enum\|trait\|type\|const\|static) NAME`；范围 = 10 个 crate 的 `src/**/*.rs`（**含** `*_tests.rs`；不含 `tests/`、`examples/`、`benches/`）。逐 crate：core 479 / run_method 124 / build_method 62 / plugin-host 50 / conventions 49 / studio 44 / debug_method 13 / macro 4 / cli 4 / mcp 2；**排除** `*_tests.rs` 后为 829（只少 conventions 的 2 个） |

六类命名空间**逐个**与 §5 的**一张**保留词表、以及 R-4 三个角色谓词比对；831 个公开名**机器筛 + 抽读**（见 §6）。

---

## 1. 发现（AMB-01 … AMB-20：MAJOR 5 条 / MINOR 15 条——AMB-06 已于本轮按 t35 降为 MINOR）

### AMB-01 —— `core/`（+ lib `xirang`）

- **严重度**：MAJOR　**类别**：1（生态保留名）
- **行为清单**：
  1. 定义身份：`core/src/registry_core/identity/node_id.rs:12`（`pub struct NodeId`）、`:42`/`:52`（`from_path`/`from_namespaced_path`，纯 SHA-256，无 I/O）。
  2. 定义注册面词汇与合同：`core/src/registry_core/declaration/declaration.rs:1`（"Registration declarations and compile-time construction contracts"）、`declaration/registration.rs`。
  3. 持有注册树与事务：`core/src/registry_core/tree/registry.rs:18`（`pub struct Registry`）、`:35-55`（`root`/`root_for`/`root_for_namespace`）、`tree/transaction/transaction.rs:98-179`（批次校验）。
  4. 解析注册面声明并守卫病态嵌套：`core/src/registry_core/syntax/syntax.rs:1`、`syntax/nesting.rs:342`（`guard_nesting`）。
  5. 从源码文本抽函数/调用点：`core/src/registry_core/source/source.rs:120`（`function_symbols`）、`:434`（`registration_kinds`）。
  6. 共享文本契约：`core/src/registry_core/lexicon/lexicon.rs:23-44`（`GENERATED_LIB_FILE`、`RUN_METHOD_CRATE`、`SCOPE_ENV`…）。
- **角色一句话**：它是「XiRang 的协议词汇与纯方法内核——所有不读盘、不读环境、不涉进程生命周期的定义都住在这里的那个组织单位」。
- **命名**：**否**。`core` 说不出职能（「核心」是位置词，任何东西都能叫核心），而且它是 Rust **标准库 crate 名**（`use core::…` 有确定含义）；lib 名 `xirang` 又等于项目名与 CLI 二进制名（一名三角色）。建议 **目录 `kernel/` + 包 `xirang-kernel` + lib `xirang_kernel`**——AGENTS.md:3-6 已经立了 "pure kernel" 这个词，改完名字与自述同词（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:38`` 正是这样）。
- **辅助判据**：读者把 `core/` 当成 no_std 标准库或「被 `use core::` 指向的东西」。判据/命令：`grep -n 'name = "xirang"' core/Cargo.toml`（→`core/Cargo.toml:32` 的 lib 名）；`cargo metadata … | python3 -c '…"xirang" in names…'`（→ `False`，没有叫 `xirang` 的包）。
- **影响面**：lib 改名是**公开路径**（`use xirang::…`、docs.rs 链接、63 处 `core/…rs:NN` 文档锚点、`conventions/src/shims.rs` 棘轮）；**身份不受影响**（方案 §6 已证 `NodeId` 不含 crate 名）。

### AMB-02 —— `build_method/`

- **严重度**：MAJOR　**类别**：1（Cargo 惯用名）
- **行为清单**：
  1. 发现源码树并预热身份缓存：`build_method/pipeline.rs:55`（`prime_node_id_cache`）、`build_method/identity_cache.rs:13-65`（读 `<target>/xirang/cache/units/*.tsv`）。
  2. 推断构建作用域：`build_method/scope.rs`，读取侧 `build_method/scope_view.rs:109-118`（`keeps`，按 `::` 段判子树）。
  3. 校验注册面/合同/静态计划：`build_method/registration_check.rs`、`contracts.rs:15-111`、`static_plan.rs:110-125`。
  4. 渲染生成的计划：`build_method/renderer/tree.rs:68-70`、`renderer/pass.rs`。
  5. 把清单发布到 `OUT_DIR`：`build_method/pipeline.rs:155-184`（先指纹后五份载荷）、`manifests.rs:14-22`。
  6. 入口（被宿主 `build.rs` 调用）：`build_method/lib.rs:137 pub fn run()`、`:153 run_for`、`:179 check_for`。
- **角色一句话**：它是「构建期把宿主源码变成生成的注册计划与清单的那一半——构建脚本背后的组织单位」。
- **命名**：**方向对、词不对**。`build_method` 确实在说「谁」（维护者认可的「说的是这个是谁」），但 `method` 在 Rust 里首先读成「类型上的函数」（R-4b），而 `build` 与 Cargo 的 build 家族（`build.rs`/`[build-dependencies]`/`cargo build`）同词。建议 **`build_time`**（AGENTS.md 对本 crate 的自述就是 "Build-time filesystem / `OUT_DIR` orchestration"），入口写成 `build_time::run()`。
- **辅助判据**：读者把 `build_method/` 当成「构建脚本相关的东西」或「一个构建方法」；`grep -rn build.rs` 全仓 4 个文件 3 种角色（真构建脚本 2、CLI 命令实现 1、模块文件 1）。判据/命令：`find . -name build.rs -not -path '*/target/*' | sort`。
- **影响面**：`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:65``（`src/build/build.rs` 的模块文件与构建脚本同名）、`:89`/`:151` 的 feature `build`、37 处 `build_method/…rs:NN` 锚点。

### AMB-03 —— `run_method/`

- **严重度**：MAJOR　**类别**：2（跨生态阅读）
- **行为清单**：
  1. 承载运行期注册状态：`run_method/lib.rs:27`（`pub mod runtime`）、`run_method/runtime/runtime.rs`。
  2. 记录并绑定调用 trace：`run_method/runtime/trace/`，`runtime/trace/call_trace.rs:100`（`pub fn runtime()`）。
  3. 执行创作写盘：`run_method/authoring/operations/operations.rs:320-334`（`edit_module_face` 入口 + `apply_module_face_values`）、`operations/face_write.rs:58-79`（字段序）。
  4. 提供宿主宏：`run_method/macros/face_registration.rs:160-171`（`development`/`debug`/`linked` 三个 collector 臂）、`host!`/`application!`。
  5. 保留历史路径：`run_method/plugin/plugin.rs:1-7`（6 行 shim）、`run_method/lib.rs:29-38`（`pub use xirang::registry_core::*`）。
- **角色一句话**：它是「运行期承载注册状态、记录已发生的调用、并执行创作写盘的那一半」。
- **命名**：**否**。`run`/`run_method` 会被读成前端 runtime 目录（维护者原话）、`cargo run`、程序入口 `run()`；而它实际是「运行期那一半」。建议 **`runtime`**（与 AGENTS.md 自述 "Runtime state instance + trace binding" 同词）——顺带消掉方案里 `pub mod run` 与 `pub use build::run` 的 **E0428**（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:167``）。
- **辅助判据**：全仓 5 个 `run` 系公开名（`build_method::run`、`cli::run`、`mcp::run`、`RuntimeCheckSpec::run`、`CallTrace::runtime`，见 AMB-20）。判据/命令：`grep -rnE '^pub fn run\b' --include=*.rs build_method/src cli/src mcp/src`。
- **影响面**：50 处 `run_method/…rs:NN` 锚点；`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:90``/`:148` 的 feature `run`；`:175-178` 的 `RUN_METHOD_CRATE` 字符串常量（改名必须连带改值）。

### AMB-04 —— `plugin-host/`（crate/目录）与方案模块名 `plugins`

- **严重度**：MAJOR　**类别**：3（同族易混）
- **行为清单**（完整版与逐文件证据见 **§2.1**）：
  1. 验签（Ed25519，指纹必须与公钥字节相符）：`plugin-host/src/verifier.rs:10`、`:52-71`。
  2. 从锁准入（先策略后签名，官方通道必须有信任根）：`plugin-host/src/admission.rs:57-82`、`:165-185`。
  3. 隔离执行（wasm 限额 / 进程隔离 + 硬超时）：`plugin-host/src/wasm.rs:149-233`、`process.rs:146-184`、`:246-436`。
  4. 懒激活（按槽代际状态机）：`plugin-host/src/lazy_wasm.rs:126-176`、`lazy_wasm/slot_state.rs:74-107`。
  5. 原子部署（校验通过才换快照）：`plugin-host/src/deployment.rs:35-78`。
- **角色一句话**：它是「**把锁里写下的插件变成正在运行的插件**的那个运行期宿主」——**不创建插件**。
- **命名**：**两层要分开判**。① 现状 crate/目录名 `plugin-host` **是**角色名（`host` = 宿主，标准插件架构用词；README/threat-model 也这么写）→ **保留**；改 `plugin-runtime` 只强化「运行期」那一半、丢掉准入与验签，且要 yank 已发布名 + 改 10 处锚点，不划算。② 合并方案把它的**模块名**写成 `plugins`——模板块与挂载行见 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:40`、`:194`——**不是**角色名，而且 `.xirang/plugins/`（`plugin-host/src/admission.rs:38`）真的是用户第三方插件锁的所在地——同名反义 → **必须换成 `plugin_host`**。
- **辅助判据**：读者把 `plugins/` 当成放插件本体的目录（VS Code `extensions/`、WordPress `plugins/` 都是这个意思）。判据/命令：`grep -n "PLUGIN_LOCK_DIRECTORY" plugin-host/src/admission.rs`（→ `:38`）；`sed -n '40p;194p' docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`。
- **影响面**：只影响**未执行的**合并方案（模块 `plugins` + feature `plugins`，见 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:91`）；现状 crate 名不动，因此已发布面、文档锚点、threat-model 全都不用改。

### AMB-05 —— 公开类型 `Registry`

- **严重度**：MAJOR　**类别**：1（Cargo 生态词）　**= D-9 的保留词 `registry`**（D-9 计文件 stem，本条计类型名，不计入 D-9 违规数）
- **行为清单**：
  1. 注册一批注册面（带事务与校验）：`core/src/registry_core/tree/registry.rs:18`、`tree/transaction/transaction.rs:98-179`。
  2. 查询：`tree/query/query.rs:52`（`registry(id)` 返回**包含**该节点的子注册机）、`:86`（`path_for(id)`）、`tree/index/index.rs:79`（`index()` 扁平索引）。
  3. 嫁接/覆盖：`core/src/registry_core/tree/graft_ops/` 下的 `overlay.rs:256-309` 与 `graft_ops.rs:255-270`。
  4. 导出视图与身份：`core/src/registry_core/tree/inspection.rs`、`identity/node_id.rs:52`。
- **角色一句话**：它是「已注册的注册面组成的**树**——可注册、可查询、可嫁接的那一个数据结构」。
- **命名**：**否（但改名代价大）**。`Registry` 是「登记处/登记机构」这个机构名词，而它是**树**（文件就住在 `tree/` 下）；更糟的是 Cargo/Cargo.toml 语境里 "the registry" 默认指 **crates.io**，本仓的 `tools/xirang-publish:80,189,192,722` 正是这个含义。建议：**保留 `Registry`**（公开类型 + docs.rs + 大量派生词），但要求两件事都做——① 模块 `registry_core` 与类型文档首行点明「注册树 ≠ crates.io」；② 写进 §5 词表。若允许公开面变更，角色名是 `RegistrationTree`。
- **辅助判据**：读者/agent 把 `Registry`/`registry_core`/MCP 工具 `xirang.registry`（`mcp/tools.rs:114`）当成包注册表。判据/命令：`grep -n "pub struct Registry" core/src/registry_core/tree/registry.rs`；`grep -n "index.crates.io\|the registry" tools/xirang-publish`。
- **影响面**：公开类型（docs.rs 页面）与 MCP 工具名（agent 的调用契约）；`registry` 是 D-9 的保留词，本条的登记行与 D-9 共用一份词表；63 处 `core/…rs:NN` 锚点中与该类型相关者。

### AMB-06 —— 顶层 `examples/`

- **严重度**：MINOR（t35 复核后由 MAJOR 降级：它不误导"去哪找"，只与 Cargo 的目录约定冲突）　**类别**：1（Cargo 自动发现目录）
- **行为清单**：
  1. 它是一个**宿主库**（不是 example target）：`examples/control-button/src/lib.rs` + 四个面文件（`src/control/control.rs`、`src/control/object/{button,slider}/*.rs`、`src/control/registry_rule/registry_rule.rs`）。
  2. 它有自己的真构建脚本：`examples/control-button/build.rs`（调 `xirang_build_method::run()`）。
  3. 它有自己的 example target 与测试：`[机器]` cargo metadata → `example: [graft_record, health_check, tree]`、`test: [health_check, ide_mirror, registry, static_plan_allocations]`。
  4. 第二个成员提供外部实现：`examples/control-button-graft/src/{control_fast,button_fast,slider_fast}.rs` + `external_registry()`。
- **角色一句话**：它是「两个可直接编译的**演示宿主工程**——用来演示宿主该怎么声明注册面与外部实现」。
- **命名**：**否**。Cargo 把 `examples/` 当**自动发现 target 的目录**（与 `tests/`/`benches/` 同族），于是 `cargo run --example control-button` 会失败（实测 exit 101）；而顶层 `examples/` 里还有一个真正的 `examples/`。建议 **`example-hosts/`**（或 `hosts/`）——它就是「宿主示例」，与 Cargo target 不再混淆。
- **辅助判据**：读者按 `cargo run --example <dir>` 找。判据/命令：`sed -n '2p' Cargo.toml`（members 含 `examples/control-button`）；`cargo run --offline --example control-button; echo $?`（→ exit 101）。
- **影响面**：根 `Cargo.toml:2`、`tools/xirang-external-rehearsal`、13 处 `examples/` 锚点与 README 措辞。

### AMB-07 —— `debug_method/`

- **严重度**：MINOR　**类别**：1（Cargo 惯用名）+ 3（同族）
- **行为清单**：
  1. 提供编译期收集器入口：`debug_method/src/lib.rs:26`（`pub use inventory`）、`:32`（`pub struct CollectedRegistration`）。
  2. 收集器由宏臂发出，且只在 debug 构建里生效：`run_method/macros/face_registration.rs:164-167`（`debug` 臂 + `#[cfg(debug_assertions)]`）。
  3. 聚合静态调用候选与真实调用：`debug_method/src/lib.rs:37`（`UnifiedCallGraph`、`CallEvidence`、`MirGraph` 等重导出）。
- **角色一句话**：它是「把编译期登记与运行期观测的调用证据汇成可查询证据的那个组织单位」。
- **命名**：**否**。`debug_method` 的两个词都不指向角色：`debug` 在 Cargo 里是 profile/`target/debug`（而它确实只在 debug 断言下收集，这层因果让歧义更难讲清），`method` 不是角色词。建议 **`call_evidence`**（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:40`` 已如此，t31 在 `audit-naming-verify2.md:42-47` 已裁定）。
- **辅助判据**：读者以为它与「debug 构建配置」有关或它是一个函数方法。`[机器]` 全仓**没有任何标识符含 `Method`**（`grep -rnE '\w*Method\w*' <8 crates>/src` → 0 行）→ 歧义只在目录/crate 名这一层。
- **影响面**：3 处 `debug_method/…` 锚点；合并后目录消失，只剩 feature `evidence`。

### AMB-08 —— bin `xirang-dev` 与 feature `dev-supervisor`

- **严重度**：MINOR　**类别**：3（同族易混）
- **行为清单**：
  1. 常驻并重建 Studio：`studio/bin/xirang-dev.rs:1-7`（"Resident rebuild supervisor for XiRang Studio"）。
  2. 只在本检出可用，故被门控：`studio/Cargo.toml:95-99`（`required-features = ["dev-supervisor"]`）。
  3. 启动检出自己的 `target/debug`：同文件 `:4-7`。
- **角色一句话**：它是「开发期重建并启动 Studio 的**监督器**」。
- **命名**：**否**。bin 叫 `-dev`（生态里 `x-dev` = 「x 的开发构建」），feature 叫 `supervisor`——同一件东西两个词。建议 bin 改 **`xirang-supervisor`** 或 `xirang-dev-supervisor`，与 feature 同词。
- **辅助判据**：读者以为 `xirang-dev` 是 studio 的开发版二进制。判据/命令：`ls studio/bin/`；`grep -n "required-features" -B 4 studio/Cargo.toml`。
- **影响面**：bin 名（README/CI/`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:125``）。

### AMB-09 —— 顶层 `picture/`

- **严重度**：MINOR　**类别**：2（跨生态阅读）
- **行为清单**：
  1. 放品牌字标：`picture/XiRang_wordmark.svg`（被 `README.md:3`、`README.zh-CN.md:3` 以内联 `<img src="./picture/…">` 引用）。
  2. 放界面截图：`picture/XiRang_studio.png`（被 `README.md:699`、`README.zh-CN.md:624` 以 raw.githubusercontent 链接引用）。
  3. 被一份设计文档当配色来源引用：`docs/design-call-tree-attribution.html:9,634`。
- **角色一句话**：它是「本仓的**品牌与界面截图资源**目录（给 README/文档用，不参与构建）」。
- **命名**：**否**。生态惯例是 `assets/`／`images/`／`static/`；`picture`（单数）在顶层看不出是资源目录还是某个 crate 的输出。建议 **`assets/`**（最小、最常见）或 `docs/images/`。
- **辅助判据**：读者按 `assets/`/`images/` 找图。判据/命令：`ls -R picture`；`grep -rn "picture/" --include=*.md --include=*.html . | grep -v /target/`（4 处引用）。
- **影响面**：只触及根 README（不打包）与一份 docs 的注释；不动任何公开面。

### AMB-10 —— `docs/ROADMAP.md` 与 `docs/roadmap-1.0.md` 只差大小写

- **严重度**：MINOR　**类别**：1（文件系统/平台约定；略超本轮六类命名空间，但同一失效模式）
- **行为清单**：
  1. `docs/ROADMAP.md` 是路线图索引（批次表，带中英两份 `ROADMAP.zh-CN.md`）。
  2. `docs/roadmap-1.0.md` 是 1.0 里程碑登记（490 行，含进度快照与批次）。
  3. 两者被本仓其它文档相互引用（`docs/` 内多处）。
- **角色一句话**：两者是「同一件事的两个版本/层级（路线图索引 与 1.0 登记）」，因此名字必须在**字母层面**就可区分。
- **命名**：**否**。名字只差 `ROADMAP`/`roadmap` 的大小写，在大小写不敏感的文件系统（macOS 默认 APFS、Windows NTFS）上无法共存，checkout/切分支会冲突。建议 `roadmap-1.0.md` → **`roadmap-to-1.0.md`**（或把索引改成 `ROADMAP-INDEX.md`）。
- **辅助判据**：读者/文件系统分不开。判据/命令：`ls docs | grep -i roadmap`（→ 三个名字）。
- **影响面**：`docs/` 内部引用与 `doc_anchors` 门禁；与 `audit-naming-review.md` **无重叠**（那篇 0 次提到 ROADMAP/大小写）。

### AMB-11 —— 内核模块 `syntax`（+ feature `syntax`）

- **严重度**：MINOR　**类别**：4（本仓一名多义）
- **行为清单**：
  1. 解析注册面宏声明：`core/src/registry_core/syntax/face.rs`（`FaceSyntax`/`parse_faces`）、`syntax/entries.rs`（`graft_entries`/`application_entries`）。
  2. 守卫病态嵌套：`core/src/registry_core/syntax/nesting.rs:342`（`guard_nesting`）+ `syntax/deep_input_tests.rs`。
  3. 提供字段/记号契约：`core/src/registry_core/syntax/fields.rs`、`syntax/tokens.rs`、`syntax/reference_scan.rs`。
- **角色一句话**：它是「解析 XiRang **注册面声明**的语法层（并负责拒绝病态嵌套）」。
- **命名**：**否**。`syntax` 在 Rust 语境里默认指 Rust 语法/`syn`，而它解析的是本仓的宏声明；另外 feature 也叫 `syntax`（`macro/Cargo.toml:20`、`build_method/Cargo.toml:16`、`conventions/Cargo.toml:29`），`pkg/syntax` 这种写法与模块路径 `pkg::syntax` 形近。建议模块 **`face_syntax`**，feature **`parse`**（或 `face-syntax`）。
- **辅助判据**：读者以为它解析 `.rs` 的文法。判据/命令：`head -1 core/src/registry_core/syntax/syntax.rs`（→ "Parser shared by build-time checks and live authoring."）；`grep -rn 'features = \["syntax"\]' --include=Cargo.toml .`（→ 3 处）。
- **影响面**：内核公开路径（shims 棘轮）；3 个清单的 feature；`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:39`` 明说该 feature 不变（若采纳需同步）。

### AMB-12 —— 内核模块 `json`

- **严重度**：MINOR　**类别**：1（生态名）
- **行为清单**：
  1. 把字符串转义进工件文本：`core/src/registry_core/json/json.rs:44`（`push_json_string`）、`:67`（`json_string`）。
  2. 供本仓所有 JSON 工件共用（模块首行：`//! JSON string encoding, shared by every artifact this workspace writes.`）。
- **角色一句话**：它是「**本仓工件文本的 JSON 字符串编码器**」。
- **命名**：**否**。`json` 是**格式名**，读者会以为这里是 JSON 解析/序列化层（真正的 JSON 处理全仓走 `serde_json`）。建议 **`json_text`**/**`json_encoding`**（保留词加限定，D-9 的手法）。
- **辅助判据**：读者按格式名找一个格式实现。判定/命令：`head -1 core/src/registry_core/json/json.rs`；`grep -rn "json::" core/src build_method/src | head`。
- **影响面**：内核公开路径（4 个公开函数被 diagnostic/mir/渲染器使用）。

### AMB-13 —— 内核模块 `release`

- **严重度**：MINOR　**类别**：1（Cargo 惯用名）
- **行为清单**：
  1. 描述发布态保留的紧凑拓扑：`core/src/registry_core/release/release.rs:9`（`pub struct StaticFace`）、`:49`（`pub struct StaticPlan`）。
  2. 由构建期生成静态计划时消费（`build_method/static_plan.rs`、`renderer/` 生成 `StaticFace::new` 调用）。
- **角色一句话**：它是「发布态保留的注册拓扑与静态计划——被剪枝后仍然存在的那一份」。
- **命名**：**否（轻度）**。`release` 在 Cargo 语境默认指 `[profile.release]`/`--release`，本仓另有 `tools/xirang-release-audit` 与发布工作流。建议保留 + 词表登记；若加限定词，**`release_plan`** 比 `release` 更贴职责。
- **辅助判据**：读者以为与 release 构建配置有关。判据/命令：`head -1 core/src/registry_core/release/release.rs`；`ls core/src/registry_core/release/`（只有 `release.rs`，D-3 的「无载荷目录」另计）。
- **影响面**：内核公开路径 + `StaticPlan` 被生成代码引用（字符串常量在 `lexicon`）。

### AMB-14 —— 内核模块 `requirements`

- **严重度**：MINOR　**类别**：2（跨生态阅读）
- **行为清单**：
  1. 声明能力与需求的数据：`core/src/registry_core/requirements/requirements.rs:17`（`CapabilityDeclaration`）、`:35`（`CapabilityRequirement`）。
  2. 找出没有被满足的需求并产出诊断：`requirements/requirements.rs:68`（`missing_capabilities` → `Vec<BuildDiagnostic>`）。
  3. 被构建期校验与注册校验消费（`build_method` 的 requirements 相位诊断）。
- **角色一句话**：它是「检查每条能力需求是否被某个提供者满足的分析器」。
- **命名**：**否（轻度）**。Python/DevOps 语境里 `requirements` = 依赖清单（`requirements.txt`）。建议 **`capabilities`**（与字段 `requires`/`provides` 同族，也更贴它分析的东西）；字段名 `requires` 不动。
- **辅助判据**：读者按 pip/依赖清单理解。判据/命令：`head -1 core/src/registry_core/requirements/requirements.rs`；`grep -n "requires:" core/src/registry_core/declaration/registration.rs | head -3`。
- **影响面**：内核公开路径；`impact`/`converge` 两个 MCP 工具消费其结论。

### AMB-15 —— 内核模块 `source`（一名四义之一）

- **严重度**：MINOR　**类别**：4（本仓一名多义）
- **行为清单**：
  1. 词法扫描 Rust 源码取函数与调用点：`core/src/registry_core/source/source.rs:23`（`SourceFunction`）、`:120`（`function_symbols`）、`:434`（`registration_kinds`）。
  2. 提供掩码/切片工具：`source/source.rs:267`（`mask_non_code`）、`:43`（`function_source_range`）、`source/lex.rs`、`source/calls.rs`、`source/walk.rs`。
  3. 被 search/callgraph/索引类工具共用（模块首行自述）。
- **角色一句话**：它是「从 Rust 源码文本里抽出函数与调用点的**词法扫描器**」。
- **命名**：**否**。`source` 在本仓还表示：面的文件路径（`FaceView.source`）、位置（`SourceLocation`，`core/src/registry_core/declaration/source_location.rs:9`）、插件来源（`PluginSource`）、诊断来源（`Diagnostic::source`，`diagnostic/error.rs:148`）——共 4 义。建议模块 **`source_lexer`**（或 `lex`），并把 `source` 写进词表；`PluginSource` 若要更强可改 `PluginOrigin`。
- **辅助判据**：Rust/Cargo 里 `source` 同时是「源码」与「依赖来源」（`[source]` 配置），两个含义都在生态里。判据/命令：`head -1 core/src/registry_core/source/source.rs`；`grep -rn "pub enum PluginSource" core/src`；`grep -rn "pub source:" core/src/registry_core/declaration/*.rs | head -3`。
- **影响面**：模块是内核公开路径；`PluginSource` 是插件公开面（改名属公开 API 变更）。

### AMB-16 —— 类型层 `artifact` 一名四义

- **严重度**：MINOR　**类别**：4　**= D-9 的保留词 `artifact`**（D-9 计文件 stem；本条计类型名，不重复计数）
- **行为清单**：
  1. 插件工件（字节 + 注册声明 + 校验状态）：`core/src/registry_core/plugin/artifact/artifact.rs`（`PluginArtifact`、`verify_artifact`/`verify_signed`）。
  2. MIR 工件（本仓自造的 JSONL 快照）：`mcp/mir.rs:20-26`（快照表头 = 来源凭据）。
  3. trace 工件（`.xirang/traces/xirang.trace`）：`run_method/runtime/trace/artifact/`。
  4. Cargo/CI 语里的构建产物（`target/`）。
- **角色一句话**：在本仓它一律是「**本工具写出并读回的自描述文件**（自带来源/校验信息）」。
- **命名**：**否（轻度）**。CI 的 `actions/upload-artifact` 让读者先当成「上传的产物包」。建议保留 `PluginArtifact`（公开面），把定义写进词表，并要求新类型一律带限定（`MirSnapshot`/`TraceArtifact`/`PluginArtifact` 已是这个形状）。
- **辅助判据**：判据/命令：`grep -rn "artifact" mcp/mir.rs run_method/runtime/trace/artifact/*.rs | head -6`。
- **影响面**：`PluginArtifact`/`VerifiedPluginArtifact` 是插件公开面；MIR/trace 工件是 CLI/MCP 的格式契约（改类型名不动格式）。

### AMB-17 —— `index` 一名三义

- **严重度**：MINOR　**类别**：4　**= D-9 的保留词 `index`**（同上，不计入 D-9 违规数）
- **行为清单**：
  1. 注册树的扁平查找索引：`core/src/registry_core/tree/index/index.rs:13`（`pub struct RegistryIndex`）、`:79`（`Registry::index()` 建表 `by_id`/`by_path`/`by_kind`）。
  2. MCP 的源码索引（列文件/函数）：`mcp/index.rs:35`（`load_sources`）、`:129`（`load_one`）、`:167`（`is_safe_child`）。
  3. crates.io 的 index：`tools/xirang-publish:80`（`index_url="https://index.crates.io"`）。
- **角色一句话**：(1) 是「注册树的查找表」、(2) 是「源码文本的索引器」，两者都不是 crates.io 的 index。
- **命名**：**否（轻度）**。建议 `RegistryIndex` 保留（限定词已在前）、`mcp/index.rs` 按 D-9 方向改 **`source_index.rs`**、词表登记。
- **辅助判据**：JS 的 `index.{js,ts}` 与 Cargo 的 registry index 都在读者预期里。判据/命令：`grep -n "pub struct RegistryIndex\|pub fn index" core/src/registry_core/tree/index/index.rs`；`grep -n "index.crates.io" tools/xirang-publish`。
- **影响面**：`RegistryIndex` 是内核公开类型（改名需 shim）；`mcp/index.rs` 为内部模块。

### AMB-18 —— `host` 一名两义

- **严重度**：MINOR　**类别**：4（本仓一名多义）
- **行为清单**：
  1. 宿主应用的入口声明：`run_method` 的 `host!()`/`application!()` 宏（`run_method/macros/`），`build_method` 解析 `HostEntry`（宿主入口文件）。
  2. 插件宿主：`plugin-host/` 提供「运行插件的宿主进程」（见 §2.1 的行为清单）。
- **角色一句话**：`host` 在本仓有两个层级——「被 XiRang 托管的**宿主应用**」与「托管插件的**插件宿主**」。
- **命名**：**否（轻度）**。通用计算语境里 `host` = 主机/容器。建议写法进词表（`host project`/`host!()` = 用户应用；`plugin host` = 运行插件的进程），并要求**不要**再出现单独以 `host` 命名的目录（现状 `plugin-host` 已带限定词，本条是防回归）。
- **辅助判据**：判据/命令：`head -1 plugin-host/src/lib.rs`；`grep -rn "host project" README.md | head -2`。
- **影响面**：`host!()`/`application!()` 是宿主公开宏（不改名，只做文档义务）。

### AMB-19 —— feature 名与模块/包名重名的成批现象

- **严重度**：MINOR　**类别**：3/4（同族易混）
- **行为清单**：
  1. 现状 feature 开的是可选能力：`core/syntax`（`macro/Cargo.toml:20`）、`run_method/authoring`（`run_method/Cargo.toml`）、`plugin-host/wasm`+`process-tools`（`plugin-host/Cargo.toml:21-23`）、`studio` 的 `node-graph`+`prototype-fixtures`+`dev-supervisor`（定义在 `studio/Cargo.toml:25-26`、`:35`、`:45`）。
  2. 合并方案新增 7 个 feature 与模块同名并分别门控目标面（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:89-93`：`build`/`run`/`evidence`/`plugins`/…）。
  3. 用 `required-features` 决定装哪个 bin：`:122-126`、`:135-137`（`autobins = false` 之后它才生效）。
- **角色一句话**：feature 是「可选能力的开关」，因此它的名字应当是**能力名**（名词或领域词），不是模块名或动词。
- **命名**：**否（轻度）**。`--features build`/`run`/`cli` 会被读成「构建相关/跑一下/装 cli」。建议：① 现状 `syntax` → `parse`（AMB-11）；② 方案那七个 feature 至少配一张 README 对照表（feature ⇄ 模块 ⇄ 原 crate 名）；③ 门禁**只**要求「feature 名不得与其他命名空间的保留词冲突」，不要求 feature 与模块异名（同名便于记忆是合理诉求）。
- **辅助判据**：判据/命令：`grep -rn '^\[features\]' -A 20 */Cargo.toml | head -40`；`sed -n '89,93p;122,126p' docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`。
- **影响面**：宿主清单（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:148-151``）、`cargo install` 的 `required-features`、CI 的 `--all-features`。

### AMB-20 —— 函数/类型层的同一轴：13 个公开名的**整名**就是别处的既有词

- **严重度**：MINOR　**类别**：4（维护者原话「同理函数名也是」）
- **行为清单**（`[机器]`：831 个公开名的整名命中词表者 13 个，逐个读定义处）：
  | 公开名 | 它实际干什么（file:line） | 读者会先当成 |
  | --- | --- | --- |
  | `cli::main()` | 读 `std::env::args_os()` → `argv_strings` → 调 `run`；定义在 `cli/lib.rs:94`（真入口是 `cli/bin/xirang.rs:10`） | 二进制的进程入口 |
  | `build_method::run()` | 建 `out_dir` + `check_for`；定义在 `build_method/lib.rs:137`（`check_for` 在 `:179`） | 「运行构建方法」 |
  | `mcp::run()` | 解析包根 + 逐帧跑 stdio 桥（`mcp/protocol.rs:71`） | 程序入口 |
  | `cli::run(argv)` | argv 分发（`cli/lib.rs:124`） | 同上 |
  | `RuntimeCheckSpec::run()` | 拿 `RuntimeValue` 评估一条运行期检查（`core/src/registry_core/declaration/runtime_checks.rs:345`） | 「跑这个检查」（应为 `check`） |
  | `CallTrace::runtime()` | 造一个运行期模式的 trace（`run_method/runtime/trace/call_trace.rs:100`） | 「运行期的那个 trace」 |
  | `Registry::registry(id)` | 返回**包含**该节点的子注册机（`core/src/registry_core/tree/query/query.rs:52`） | 整棵树 |
  | `Registry::index()` | 构建扁平查找表（`core/src/registry_core/tree/index/index.rs:79`） | crates.io index |
  | `FaceField::requirements(name)` | 取该面的 requires 列表（`core/src/registry_core/syntax/fields.rs:159`） | 需求清单 |
  | `Diagnostic::source()` | 取诊断来源（`core/src/registry_core/diagnostic/error.rs:148`） | 源码文本 |
  | `GraftPlan::target()`/`::graft()` | 取计划的目标身份/替换名（`run_method/authoring/external_graft/plan.rs:60`、`:72`） | Cargo 的 target |
- **角色一句话**：函数名应当是「**动作 + 宾语**」的角色（`run_from_env`、`owning_registry`、`build_index`、`evaluate`），
  只有**入口位**才允许裸动词（与 §D 的 D-5 同规则）。
- **命名**：**否（4 个该改、7 个登记即可）**。最值：`cli::main` → **`run_from_env`**；`RuntimeCheckSpec::run` → **`check`**；
  `Registry::registry(id)` → **`owning_registry(id)`**；`Registry::index()` → **`build_index()`**（或用 `RegistryIndex::from(&registry)`）。
  其余（`target`/`graft`/`source`/`requirements` 访问器、`CallTrace::runtime`）进词表。
- **辅助判据**：判据/命令见 §5 的脚本（输出 13 行）；**影响的 831 个名字里 212 个按分词命中词表但都带了限定词**（`GraftPlan`/`FaceView`/`PluginArtifact`…）——它们是正确形状，不动。
- **影响面**：`cli::main`、`RuntimeCheckSpec::run`、`Registry::registry`、`Registry::index` 都是公开 API（改名属公开面变更；`cli` 已有 `run` 可作别名）。

---

## 2. 维护者点名的三个案例（完整三段式）

### 2.1 `plugin-host/` —— 它是不是「host」那个角色？

- **行为清单**（我逐文件核过，结论与维护者给的行为链一致）：
  1. **签名验证**：`plugin-host/src/verifier.rs:10`（`TrustedPublicKey`：指纹必须与公钥字节的 SHA-256 相符）、`:52-71`（`Ed25519Verifier::verify` 用内核构造的 payload 验签）。
  2. **从锁准入**：`plugin-host/src/admission.rs:57-82`（`plugin_catalog` 读 `<pkg>/.xirang/plugins/{official,user}.lock`）、`:165-185`（`admit`：先 `PluginPolicy::decision` 查框架/来源/摘要/签名/锁，再 `verify_signed` 或 `verify_artifact`）。
  3. **隔离执行**：`plugin-host/src/wasm.rs:149-233`（`WasmBackend::load`：模块字节/元素段/内存/表上限 + ABI + health 导出），`plugin-host/src/process.rs:146-184`（暂存私有副本并要求与已验证字节逐字节相等）、`:246-436`（每次调用一个新子进程 + 硬超时）。
  4. **懒激活**：`plugin-host/src/lazy_wasm.rs:126-176`（`install`/`call`/`activate_pending`/`is_loaded`）、`lazy_wasm/slot_state.rs:74-107`（代际状态机）。
  5. **原子部署**：`plugin-host/src/deployment.rs:35-78`（`HotDeployment::new`/`replace`/`load`，校验通过才换快照）。
- **角色一句话**：它是「**把锁里写下的插件变成正在运行的插件**的那个运行期宿主——负责验签、准入、隔离执行与原子换代的组织单位」。它**不创建插件**（没有生成/编译插件的代码：全 crate 无 `compile`/`build` 生产路径）。
- **命名**：**是**。`plugin-host` 里的 `host` 正是「宿主」这个角色名词（为插件提供运行环境的那个东西），
  且本仓的 README、`docs/threat-model*.md`、`PluginAdmission` 的文档都这么说——**建议保留 `plugin-host`（crate/目录名）**。
  若要更强，`plugin-runtime` 会把重心收在「运行期」（懒激活 + 隔离执行 + 后端），但会**丢掉准入与验签那一半**，
  而且代价明确：已发布的 `xirang-plugin-host` 已在 crates.io（0.1.0–0.1.5 无法撤回，只能 yank），
  10 处 `plugin-host/…rs:NN` 文档锚点、`docs/threat-model*`、示例与 README 的依赖写法都要改，收益只是措辞更锐。
  **真正该改的是方案里的模块名 `plugins`**（§3）：那才是把「用户插件所在地」与「运行插件的代码」搞成同名的那个改动。

### 2.2 内核 `core/src/registry_core/plugin/` —— 家族名与子模块角色名

- **行为清单**：
  1. 定义插件工件与「已校验」状态：`plugin/artifact/artifact.rs:1`（"Verified plugin artifacts and selection decisions"）、`verify_artifact`/`verify_signed`。
  2. 解析锁记录并做目录匹配：`plugin/catalog/catalog.rs:122-215`（`PluginCatalog::parse`：7/10 字段、schema 门禁、身份五元组查重）。
  3. 定义信任策略与签名抽象：`plugin/trust/trust.rs:37-132`（`PluginSignatureVerifier` trait、`PluginTrustPolicy::verify`：摘要/撤销/官方密钥）。
  4. 定义槽位与准入通道：`plugin/slot/slot.rs:16-108`（`PluginChannel`、`validate_artifact`：通道/framework/mode/flow）。
  5. 定义宿主筛选策略与适配器契约：`plugin/plugin_policy/plugin_policy.rs:124-162`（`decision`：框架/来源/摘要/签名/锁）。
  6. 定义随插件字节传输的合同与签名载荷：`plugin/contracts/contracts.rs:488-495`（`signing_payload`）。
- **角色一句话**：它是「**定义『什么叫一个可准入的插件』的词汇层**——工件、锁、信任、槽位、策略、合同的定义集合」，
  即一个**家族**（不是单一职能），与 `support`/`misc` 同类，只是这个家族的名字恰好是一个生态常用词。
- **命名**：**否（家族名要带限定）**。建议家族改 **`plugin_protocol`**（模块首行自己就说 "Plugin protocol: …"），
  子模块角色名（现状 → 角色，**建议不动**，因为父路径已经限定）：
  `artifact/`=「一个插件工件及其校验状态」；`catalog/`=「锁里登记的插件清单」；`trust/`=「信任策略」；
  `slot/`=「插件落位与准入通道」；`plugin_policy/`=「宿主筛选策略」（已带限定）；`contracts/`=「数据流合同与签名载荷」。
  唯一**不属于该家族**的是 `graft/`（"Atomic graft commands and validation errors"——它是注册树的嫁接命令解析，被 `tree/graft_ops` 消费，只是**外部插件的计划**恰好走它）：**它的归属是结构轴的问题，本报告不下结论**，只记一条判据供结构轴用——
  「`graft` 的消费者里有多少是插件路径？若多数不是，它不该挂在 `plugin_*` 下」。
- **辅助判据**：`grep -rn "use crate::registry_core::plugin" core/src | grep -v plugin/ | head`（消费者分布）；
  `head -1 core/src/registry_core/plugin/plugin.rs`。
- **影响面**：家族改名触及内核公开路径（`xirang::plugin::…` 被 `plugin-host`、`run_method` 与文档引用）+ shims 棘轮；
  子模块不动则影响面为零。

### 2.3 `run_method/plugin/plugin.rs` —— shim 的角色与合并时的处置

- **行为清单**（全文 7 行）：
  1. 声明自己的角色：`run_method/plugin/plugin.rs:1-4`（"The implementation lives in the kernel `plugin` module; this shim keeps the historical `xirang_run_method::plugin` path."）。
  2. 转发两条重导出：`run_method/plugin/plugin.rs:6`（`pub use xirang::plugin;`）、`:7`（`pub use xirang::plugin::*;`）。
  3. 被门禁钉住：`conventions/src/shims.rs` 的 `SHIMS` 表逐条记录执行面 shim（`AGENTS.md` 规则 2：执行面用 shim 保留历史路径）。
- **角色一句话**：它是「**保留历史公开路径的转发层**（shim）——本身不实现任何插件逻辑」。
- **命名**：**是**。shim 的命名惯例就是**与被转发目标同名**（同族的 `build_method/identity.rs` → 挂成 `registry_identity`；见 `audit-naming-review.md` §C 的 NAM-43 第 1 族），所以 `plugin/plugin.rs` 这个名字是**对的**。
- **合并时怎么处理**（角色决定动作）：
  1. **合并期**：随 `run_method` 一起进 `toolchain/src/run/plugin.rs`（或等价位置），**保留** `pub use` 两行 —— shims 门禁要求的历史路径在合并后再活一个版本线。
  2. **一次性删除的时机**：与所有 `run_method::*` shim 一起，在**允许公开面变更**的那次版本（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:308-310`` 已列出名字/readme/documentation 的替换清单）统一删除，并同步删 `conventions/src/shims.rs` 的对应条目（棘轮只许变短，删除即是一步）。
  3. **判据**：`grep -n "run_method/plugin/plugin.rs" conventions/src/shims.rs`（若在该表里，说明它是被承诺的公开路径，必须先保留）。

---

## 3. 方案 §2 七个模块名：三段式逐名判定

合并方案的模块表在 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:40`（§1.1）与 `:62-72`（§1.2），
根导出碰撞在 `:167`/`:176-185`，挂载形态在 `:194`，逐 bin 安装在 `:122-126`，feature 在 `:89-93`。

| 方案模块名 | 行为清单（它搬进去的是什么，file:line 见对应 AMB） | 角色一句话 | 命名判定 | 建议 |
| --- | --- | --- | --- | --- |
| `build` | = AMB-02 的六条（发现/作用域/校验/渲染/发布/入口） | 「构建期把宿主源码变成计划与清单的那一半」 | **否**：`build` 是无宾语动词（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:65`` 的 `src/build/build.rs` 还让模块文件与构建脚本同名；feature `build` 见 `:89`） | **`build_time`** |
| `run` | = AMB-03 的五条（运行期状态/trace/创作执行/宏/shim） | 「运行期承载注册状态、记录调用并执行创作的那一半」 | **否**：`run` 是无宾语动词；它正是 E0428 的成因（`:167`），并逼出「只 glob `run`」的规则（`:185`） | **`runtime`**（顺带消掉 E0428） |
| `plugins` | = §2.1 的五条（验签/准入/隔离执行/懒激活/部署） | 「把锁里写下的插件变成正在运行的插件的那一半（**插件宿主**）」 | **否**：`plugins/` 在生态里放插件**本体**，而本仓 `.xirang/plugins/`（`plugin-host/src/admission.rs:38`）真的是用户插件所在地 | **`plugin_host`** |
| `call_evidence` | = AMB-07 的三条（收集器/宏臂/聚合） | 「把编译期登记与运行期观测汇成证据的那一半」 | **是**（限定词 + 与 `CallEvidence` 同词；t31 已裁定） | 保留；feature 仍叫 `evidence`，README 加一行对照 |
| `studio` | = Studio TUI（`studio/{lib,main}.rs`，创作/检视面） | 「面向人的 Ratatui 创作与检视面」 | **是**（产品名，本仓三处一致） | 保留 |
| `mcp` | = MCP 桥（`mcp/protocol.rs:71` 起，17 个工具） | 「面向 AI 客户端的 stdio 桥」 | **是**（协议缩写） | 保留 |
| `cli` | = argv 分发 + cargo 子进程（`cli/lib.rs:124`、`commands/`） | 「面向命令行的分发面」 | **是（轻度）**：约定的缩写，无更优词 | 保留 + 对照表 |

**同时判定方案的两个新 crate 名与两个 feature 事实**：

- `xirang-kernel` / `kernel/`（`:38`、`:53`）——**是**：AGENTS.md:3-6 已立 "pure kernel" 这个词，
  lib 改 `xirang_kernel` 顺带消掉 AMB-01 的 lib 歧义（**方案里最值钱的一次改名**）。残留：Rust 生态里 `kernel`
  强指 OS/no_std，README 首段需一句「kernel here = 纯协议词汇与纯方法」。
- `xirang-toolchain` / `toolchain/`（`:40`、`:62`）——**否（需要限定或登记）**："toolchain" 在生态里默认指
  **rustup/rustc 工具链**（`rust-toolchain.toml`、`rustup toolchain list`；`[机器]` 本仓无该文件，冲突只在读者预期层）。
  一个装着 build+run+studio+mcp+cli 的 crate 叫 `toolchain`，读者会先入为主以为它对接 Rust 工具链。
  备选：**`xirang-surfaces`**（AGENTS.md 的词 "execution surfaces"）、`xirang-host`；
  若保留 `toolchain`，在 README 首段写明「指本仓的执行面集合」。
- `xirang-macro` / `macro/`（`:39`）——**是**：它就是 proc-macro crate，名字准确；`macro` 是关键字但对目录/包名无影响。
- feature `build`/`run`/`plugins`/`studio`/`mcp`/`cli` 与模块/旧 crate 同名（`:89-93`、`:122-126`）——见 AMB-19：
  建议配一张 README 对照表；`default = []` 与 `required-features` 的设计（`:135-137`）本身是对的。

---

## 4. 「不是缺陷」清单（同样给出角色，判据是四问 + 角色谓词）

| 名字 | 角色一句话 | 为什么不判歧义 |
| --- | --- | --- |
| `studio`、`mcp`、`cli` | 产品名/协议缩写/约定缩写 | 三处（包、bin、README）一致；生态里没有更强的既有含义；不看代码也知道去哪找 |
| `macro`（目录） | 「proc-macro 前端 crate」 | 它**就是** proc-macro（`[lib] proc-macro = true`）；关键字形对目录名无影响 |
| `conventions` | 「仓库规则的可执行门禁」 | 该 crate 的 README 首段与全部 12 个模块都只说这件事；不在任何生态保留名里 |
| `tools/`、`docs/`、`target/` | 「脚本」「文档」「Cargo 产物」 | 三个通用惯例；`target/` 是 Cargo 固定产物目录（构建产物，R-1 豁免） |
| 内核 `mir` | 「读 rustc MIR 转储并与真实调用合并的证据层」 | **它确实是** rustc 的 Mid-level IR（`mir/mir.rs:1` "Static MIR candidates…"，`MirGraph::from_mir_text` 读 `-Zunpretty=mir`）→ 读者默认含义**正确**；只需词表一行（JSONL 是本仓包装形态） |
| `face`、`graft`、`trace`、`tree`、`identity`、`declaration`、`diagnostic`、`lexicon`、`authoring` | 各自领域词 | 本仓领域词，README/设计文档给了定义；生态里无竞争含义（`trace` 与 `tracing` crate 只是邻近） |
| `mod tests`（76 处内联）与 `*_tests` | 单元测试模块 | Rust 惯用写法；若硬禁一次产生 76 条噪声——「一次把什么都点成缺陷」的反面教材 |
| 私有模块 `mod build;`（`core/src/registry_core/diagnostic/diagnostic.rs:14`） | 「构建诊断」 | 命中 `build`，但**父路径已消歧**（`conventions/src/size.rs:22` 也这么引用） |
| `cargo-xirang` | Cargo 子命令 | `cargo-<x>` 是 Cargo 的**唯一合法**命名方式（不能改） |
| `wasm`、`process-tools`、`node-graph`、`prototype-fixtures` | 后端/控件/夹具开关 | 描述性词，不指错地方（`wasm` 只是与引擎名邻近） |

---

## 5. 命名歧义共识（可落 `conventions` 门禁）

与 `audit-naming-review.md` §D（`:387`）**正交**：D-1…D-11 管**文件 stem 与函数名**，本节管**命名空间单位**
（目录 / 包 / lib / 模块 / feature / bin），R-4 是维护者本轮新增的**角色命名**规则；同名概念在两层各有一条规则，
**违规计数不重复**（正文标 `= D-n` 的条目不计入对方）。

### R-1 保留名黑名单（硬禁，精确匹配整个名字段）

```text
core std alloc proc_macro build target dist out vendor tests benches examples src bin main lib
```
**谓词**：命名空间单位的名字**完整段**等于黑名单任一词即违规；三条豁免：
① 该目录是构建产物（`git check-ignore target` 命中）；② 父路径已把它限定（`diagnostic::build`、`commands/build.rs` 挂成 `build_command`）；③ `mod tests`/`*_tests`。
`[机器]` 现状：顶层目录命中 `core`、`examples`（2）；成员目录 `core`（与上同一物，不重复计）；模块 `build`(②)/`tests`(③) 豁免 → **净违规 2**。

### R-2 需限定词（保留词表：与 `audit-naming-review.md` 的 D-9 **合成一张**）

> **合成说明（t35 ②）**：D-9 与本节的 R-2 是同一件事（保留词 × 加限定词或登记豁免），只是**谓词对象不同**：
> D-9 的对象是**文件 stem**，R-2 的对象是**命名空间名**（目录/包/lib/模块/feature/bin）。两表合成**一张**，
> **同一对象只计一次**；词表本体建议落成 `conventions` 能读的一份常量（`lexicon` crate 已有"文本契约集中一处"的先例）。

| 保留词 | 本仓语义（读者会先当成什么） | D-9 对象：文件 stem 现状 | R-2 对象：命名空间名现状 | 处置 |
| --- | --- | --- | --- | --- |
| `registry` | 注册树（≠ Cargo/crates.io 的 registry） | `tree/registry/registry.rs` 等（D-9 已计） | 模块 `registry`（`core/src/registry_core/tree/registry.rs` 所在目录）、`registry_core`；类型 `Registry`；MCP 工具 `xirang.registry`（AMB-05） | 保留 + 文档首行消歧 + 登记 |
| `evidence` | 证据（构建产物证据 / 已观测调用证据） | `runtime/evidence.rs`、`mcp/evidence.rs`（D-9 已计；NAM-11 已裁定 `build_evidence.rs`） | 模块 `evidence`；feature `evidence` | 已带限定/已裁定 |
| `index` | 查找索引（树索引 / 源码索引 / crates.io index） | `tree/index/index.rs`（D-9 已计） | 模块 `index`、`RegistryIndex`、`Registry::index()`（AMB-17） | 加限定（`source_index`） |
| `artifact` | 本工具写出并读回的自描述文件 | `plugin/artifact/*`（D-9 已计） | 类型 `PluginArtifact`；MIR/trace artifact（AMB-16） | 保留 + 登记 |
| `face` `graft` `trace` | 注册面 / 嫁接记录与切口 / 已记录调用 | D-9 已计（`face*`/`graft*`/`trace*` 文件） | 模块 `face`、`graft`、`trace`（§4 已判"领域词、不是缺陷"） | 登记豁免 |
| `release` | 发布态保留的注册拓扑（≠ Cargo `release` profile） | — | 模块 `release`（AMB-13） | 保留 + 登记（或 `release_plan`） |
| `debug` | 观测/证据那一半（≠ debug profile 与 `target/debug`） | — | 目录/包/lib `debug_method`（AMB-07；R-4a 也命中） | 合并后消失 → `call_evidence` |
| `json` | 工件文本的 JSON 字符串编码器 | `json/json.rs`（D-9 已计） | 模块 `json`（AMB-12） | 加限定（`json_text`） |
| `source` | 源码词法扫描器 / 字段"文件路径" / 插件来源 | `source/source.rs` 等（D-9 已计） | 模块 `source`、`PluginSource`、`FaceView.source`（AMB-15） | 模块加限定（`source_lexer`） |
| `plugin` `plugins` `host` | 插件协议 / 插件宿主 / 宿主应用 | `plugin*` 文件（D-9 已计） | 模块 `plugin`、合并方案的模块 `plugins`、crate `plugin-host`（AMB-04/18） | 方案 `plugins` → `plugin_host` |
| `run` `runtime` | 运行期那一半（≠ 前端 runtime 目录、`cargo run`） | — | 目录/包/lib `run_method`、feature `run`、合并方案的模块 `run`（AMB-03） | 改 `runtime` |
| `method` | 「执行面」的组织单位 | — | `*_method` 三个目录/包/lib（AMB-02/03/07）；**由 R-4a 计** | R-4a 的改法表 |
| `syntax` | 注册面声明语法（≠ Rust syntax/`syn`） | — | 模块 `syntax` + feature `syntax`（AMB-11） | 模块 `face_syntax`、feature `parse` |
| `requirements` | 能力需求分析（≠ pip `requirements.txt`） | — | 模块 `requirements`（AMB-14） | 改 `capabilities` |
| `mir` | rustc 的 Mid-level IR（读者默认含义**正确**） | `mir/mir.rs`（D-9 已计） | 模块 `mir`（§4 已判不是缺陷）+ 近名对 `mir`↔`mirror`（**R-4c**） | 登记；近名对见 R-4c |

`[机器]` R-2 现状：**模块 15 + feature 1 = 16 个名字**命中保留词表
（`artifact evidence face graft index json mir plugin registry release requirements runtime source syntax trace` + feature `syntax`），
其中 5 个已带限定词或已裁定（走"登记"），其余按 AMB-11/12/13/14/15/17 的建议改名。

### R-3 命名空间内部一致性（防同名不同物）

1. **feature 名不得与它门控的模块名相同**（现状 `syntax`；方案 `build`/`run`/`plugins` 采纳建议后自然满足）。
2. **一个词不得同时命名「容器」与「被容器装的东西」**（`plugins` 模块 vs `.xirang/plugins/`；`host` vs `plugin-host`）。
3. **同一概念在目录/crate/模块/bin 四层用同一个词**（反例：`plugin-host` crate vs 方案 `plugins` 模块 vs `.xirang/plugins` 目录；`xirang-dev` bin vs `dev-supervisor` feature）。
4. **模块文件名不得与 Cargo 保留文件名同形却不同职责**（现状 `build.rs` 一词四用：真构建脚本 2、CLI 命令实现 1、模块文件 1 → 模块名不要取 `build`）。

### R-4 角色命名（拆成三个可判定指标；维护者的总方法）

**规则**：命名空间单位（目录 / 包 / lib / 模块 / feature / bin）的名字必须是**名词短语角色名**。
- **R-4a · 名字的最后一个 token 必须是角色名词**：不得是动作动词，也不得是
  `method`/`misc`/`support`/`utils`/`common`/`helper`/`helpers`/`data`/`base`/`general`/`stuff`；
- **R-4b · 单 token 名字不得是动作动词**（无宾语动词不是角色名）；
- **R-4c · 近名对**：`A` 是 `B` 的字符串前缀 ∧ `A` 是单 token ∧ `B` 去掉 `A` 后不含 `_` ∧ 剩余部分不是 `s`/`es`
  （排除"集合模块用复数"这一本仓约定）；
- **函数名不受 R-4a/b 限制**（动作名天然是动词），但裸动词只允许出现在**入口位**（crate 根入口 / CLI 命令实现 / trait 实现 / `main`）——与 `audit-naming-review.md` 的 **D-5** 同规则，本轮不重复计数。

**豁免（可机械判定；按 t35 V35-02 重写）**：
1. `*_command`、`*_tests`（挂载约定）；
2. 动名词（`*-ing`：`authoring`/`rendering`/`migration`/`observation`/`mounting`…）与 R-2 表里标"登记豁免"的领域词；
3. **父路径限定对**：**"名字全等即豁免"已撤销**；改为允许表里的 `(父模块, 名字)` 对：

   ```text
   ALLOWED_QUALIFIED = { ("diagnostic", "build") }   # diagnostic::build = 「构建诊断」，conventions/src/size.rs:22 也这么引用
   ```

   谓词：`(声明所在的父模块名, 名字) ∈ ALLOWED_QUALIFIED` → 豁免；否则**报出**。自测（`[机器]`，脚本见下）：

   ```text
   ("build", parent="<crate root>")  -> 违规    # 合并方案 toolchain/src/lib.rs 的 pub mod build; 会被报出
   ("build", parent="diagnostic")    -> 豁免    # core/src/registry_core/diagnostic/diagnostic.rs:14
   ("build_command", parent="lib")   -> 豁免    # 后缀规则
   ("support", parent="app")         -> 违规    # 该对象已由 D-1 计数，见 R-4a
   ```

   **这条口径修掉了一个真实漏洞**：上一版按名字全等豁免 `build`，而合并方案正要引入 **crate 级 `build` 模块**
   （`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:194` 的 `pub mod build;`）——旧判据会静默放过它；
   新判据把它**正确报出**，同时 `diagnostic::build` 仍被豁免。

**谓词脚本**（`mod X;` 与 `mod X {` 两种声明都计入；本轮实跑输出贴在后面）：

```sh
cd /home/nich/Moirai_N3/nichlink
python3 - <<'PY'
import re,os,sys,json,subprocess
ROOT=sys.argv[1] if len(sys.argv)>1 else "."
FULL=os.path.abspath(ROOT)==os.path.abspath(".")
SITE_ORDER=os.environ.get("SITE_ORDER","walk")   # walk | reverse：把"站点访问序"做成显式输入，便于顺序无关性自测
BAD_TAIL={"method","misc","support","utils","common","helper","helpers","data","base","general","stuff"}
VERBS={"add","apply","build","converge","create","delete","diff","edit","explain","install","parse","preview","render","search","verify","walk"}
TWO_WAY={"graft","plan","record","pass"}          # 本仓也用作领域名词，放宽口径才收
ALLOWED_QUALIFIED={("diagnostic","build")}        # 父路径限定对（逐站点生效）
ALLOWED_PAIRS={("app","application")}             # V40-01：两侧都是好名字，登记豁免（理由见报告）
def toks(n): return [t for t in re.split(r'[-_]+', re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', n).lower()) if t]
def rel(p): return p.replace('./','') if FULL else p
sites={}
for root,dirs,files in os.walk(ROOT):
    if 'target' in root.split(os.sep) or '.git' in root.split(os.sep): continue
    for f in files:
        if not f.endswith('.rs'): continue
        p=os.path.join(root,f)
        parent=f[:-3] if f not in ('lib.rs','main.rs') else '<crate root>'
        for i,l in enumerate(open(p,errors='replace').read().split('\n'),1):
            m=re.match(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*[;{]',l)
            if m: sites.setdefault(m.group(1),[]).append((rel(p),i,parent))
other={}   # 非模块命名空间（仅在整仓模式枚举）
if FULL:
    other["顶层目录"]=[d for d in os.listdir('.') if os.path.isdir(d) and not d.startswith('.') and d!="target"]
    meta=json.loads(subprocess.run(["cargo","metadata","--no-deps","--format-version","1","--offline"],capture_output=True,text=True).stdout)
    other["包名"]=[p["name"] for p in meta["packages"]]
    other["lib名"]=[t["name"] for p in meta["packages"] for t in p["targets"] if "lib" in t["kind"] or "proc-macro" in t["kind"]]
    other["bin名"]=[t["name"] for p in meta["packages"] for t in p["targets"] if "bin" in t["kind"]]
    feats=set()
    for root,dirs,files in os.walk('.'):
        if 'target' in root.split(os.sep) or '.git' in root.split(os.sep): continue
        if 'Cargo.toml' in files:
            t=open(os.path.join(root,'Cargo.toml'),errors='replace').read()
            m=re.search(r'^\[features\](.*?)(^\[|\Z)',t,re.S|re.M)
            if m: feats|={x.group(1) for x in re.finditer(r'^([a-zA-Z_][a-zA-Z0-9_-]*)\s*=',m.group(1),re.M)}
    other["feature名"]=sorted(feats)
def exempt_site(name,parent):
    return name.endswith("_command") or name.endswith("_tests") or (parent,name) in ALLOWED_QUALIFIED
print(f"[ROOT={ROOT}] 枚举: 模块名 {len(sites)} 声明处 {sum(len(v) for v in sites.values())}"
      + (f" | feature {len(other['feature名'])} | 顶层目录 {len(other['顶层目录'])}" if FULL else ""))
# ---- R-4a（名字 / 声明处）
a_names=[]; a_sites=0
for k,vals in list(other.items())+[("模块名",sorted(sites))]:
    for x in vals:
        if toks(x) and toks(x)[-1] in BAD_TAIL:
            a_names.append((k,x)); a_sites+= len(sites[x]) if k=="模块名" else 1
print("\n[R-4a] 末 token ∈ BAD_TAIL：名字",len(a_names),"声明处",a_sites)
for k,x in a_names: print(f"    {k:8} {x}" + (f"  -> {sites[x]}" if k=="模块名" else ""))
# ---- R-4b（逐站点豁免；违规列表存 (名字, file:line)）
vb=[]; eb=[]
cand=[("模块名",x) for x in sorted(sites)]+[(k,x) for k,vals in other.items() for x in vals]
for k,x in cand:
    if len(toks(x))==1 and toks(x)[0] in VERBS:
        if k=="模块名":
            ordered=list(sites[x])
            if SITE_ORDER=="reverse": ordered.reverse()
            old_verdict="豁免" if ordered and (ordered[0][2],x) in ALLOWED_QUALIFIED else "报出"
            print(f"    [顺序={SITE_ORDER}] {x}: 访问序 {[(p,i) for p,i,_ in ordered]} | 新逻辑(逐站点) 报出 {sum(1 for (p,i,par) in ordered if not exempt_site(x,par))} 处, 豁免 {sum(1 for (p,i,par) in ordered if exempt_site(x,par))} 处 | 旧逻辑(只看首站点) {old_verdict}")
            for (p,i,parent) in ordered:
                (eb if exempt_site(x,parent) else vb).append((x,p,i,parent))
        else:
            vb.append((x,f"<{k}>",0,"<n/a>"))
if not FULL:   # 自测：把访问序打出来，证明两种夹具的 walk 顺序确实不同
    for x in sorted(sites):
        if len(sites[x])>1: print("    访问序:", x, sites[x])
print("\n[R-4b] 严格口径：违规声明处",len(vb),"违规名字",len({v[0] for v in vb}),"豁免声明处",len(eb))
for v in vb: print("    ",v)
print("    豁免:",eb)
relaxed=[(x,p,i,parent) for x in sorted(sites) if len(toks(x))==1 and toks(x)[0] in (VERBS|TWO_WAY)
         for (p,i,parent) in sites[x] if not exempt_site(x,parent)]
print("    放宽口径(含 graft/plan/record/pass): 名字",len({v[0] for v in relaxed}),"声明处",len(relaxed))
# ---- R-4c（对级；ALLOWED_PAIRS 登记豁免）
pairs=[]; posted=[]; plural=[]
names=sorted(sites)
for a in names:
    for b in names:
        if a!=b and b.startswith(a) and len(toks(a))==1 and b[len(a):] and '_' not in b[len(a):]:
            if b[len(a):] in ("s","es"): plural.append((a,b))
            elif (a,b) in ALLOWED_PAIRS or (b,a) in ALLOWED_PAIRS: posted.append((a,b))
            else: pairs.append((a,b))
print("\n[R-4c] 违规对",len(pairs),[f"{a}↔{b}" for a,b in pairs])
print("    登记豁免",len(posted),[f"{a}↔{b}" for a,b in posted])
print("    单复数族豁免",len(plural),[f"{a}↔{b}" for a,b in plural])
PY
```

`[机器]` **本轮实跑输出（原样）**（整仓：`python3 r4.py`，`ROOT=.`）：

```text
[ROOT=.] 枚举: 模块名 296 声明处 440 | feature 8 | 顶层目录 14

[R-4a] 末 token ∈ BAD_TAIL：名字 14 声明处 14
    顶层目录     run_method
    顶层目录     debug_method
    顶层目录     build_method
    包名       xirang-run-method
    包名       xirang-build-method
    包名       xirang-debug-method
    lib名     xirang_run_method
    lib名     xirang_build_method
    lib名     xirang_debug_method
    模块名      data  -> [('studio/studio/ui/graph.rs', 20, 'graph')]
    模块名      face_helpers  -> [('run_method/macros/face.rs', 7, 'face')]
    模块名      misc  -> [('studio/studio/app/state/state.rs', 14, 'state')]
    模块名      support  -> [('studio/studio/app/app.rs', 34, 'app')]
    模块名      test_support  -> [('build_method/renderer.rs', 32, 'renderer')]
    [顺序=walk] add: 访问序 [('studio/studio/app/keyboard_overlay.rs', 10)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] apply: 访问序 [('mcp/lib.rs', 67), ('core/src/registry_core/tree/graft_ops/record.rs', 40)] | 新逻辑(逐站点) 报出 2 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] build: 访问序 [('core/src/registry_core/diagnostic/diagnostic.rs', 14)] | 新逻辑(逐站点) 报出 0 处, 豁免 1 处 | 旧逻辑(只看首站点) 豁免
    [顺序=walk] converge: 访问序 [('mcp/lib.rs', 91)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] create: 访问序 [('run_method/authoring/operations/operations.rs', 9)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] delete: 访问序 [('studio/studio/app/keyboard_overlay.rs', 12), ('studio/studio/ui/forms.rs', 16), ('run_method/authoring/operations/operations.rs', 12)] | 新逻辑(逐站点) 报出 3 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] diff: 访问序 [('mcp/lib.rs', 85)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] edit: 访问序 [('studio/studio/app/tests.rs', 59), ('studio/studio/app/keyboard_overlay.rs', 14)] | 新逻辑(逐站点) 报出 2 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] explain: 访问序 [('cli/lib.rs', 19)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] install: 访问序 [('build_method/scaffold.rs', 14)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] parse: 访问序 [('core/src/registry_core/authoring/authoring.rs', 21), ('run_method/authoring/authoring.rs', 29), ('run_method/authoring/manifest/manifest.rs', 9), ('run_method/runtime/trace/artifact/artifact.rs', 28)] | 新逻辑(逐站点) 报出 4 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] preview: 访问序 [('mcp/lib.rs', 73)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] render: 访问序 [('core/src/registry_core/mir/mir.rs', 29), ('run_method/authoring/manifest/face/face.rs', 18)] | 新逻辑(逐站点) 报出 2 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] search: 访问序 [('mcp/lib.rs', 115), ('studio/studio/app/keyboard_overlay.rs', 22), ('studio/studio/app/state/state.rs', 22), ('studio/studio/ui/ui.rs', 33)] | 新逻辑(逐站点) 报出 4 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] verify: 访问序 [('mcp/lib.rs', 100)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出
    [顺序=walk] walk: 访问序 [('core/src/registry_core/source/source.rs', 17)] | 新逻辑(逐站点) 报出 1 处, 豁免 0 处 | 旧逻辑(只看首站点) 报出

[R-4b] 严格口径：违规声明处 26 违规名字 15 豁免声明处 1
     ('add', 'studio/studio/app/keyboard_overlay.rs', 10, 'keyboard_overlay')
     ('apply', 'mcp/lib.rs', 67, '<crate root>')
     ('apply', 'core/src/registry_core/tree/graft_ops/record.rs', 40, 'record')
     ('converge', 'mcp/lib.rs', 91, '<crate root>')
     ('create', 'run_method/authoring/operations/operations.rs', 9, 'operations')
     ('delete', 'studio/studio/app/keyboard_overlay.rs', 12, 'keyboard_overlay')
     ('delete', 'studio/studio/ui/forms.rs', 16, 'forms')
     ('delete', 'run_method/authoring/operations/operations.rs', 12, 'operations')
     ('diff', 'mcp/lib.rs', 85, '<crate root>')
     ('edit', 'studio/studio/app/tests.rs', 59, 'tests')
     ('edit', 'studio/studio/app/keyboard_overlay.rs', 14, 'keyboard_overlay')
     ('explain', 'cli/lib.rs', 19, '<crate root>')
     ('install', 'build_method/scaffold.rs', 14, 'scaffold')
     ('parse', 'core/src/registry_core/authoring/authoring.rs', 21, 'authoring')
     ('parse', 'run_method/authoring/authoring.rs', 29, 'authoring')
     ('parse', 'run_method/authoring/manifest/manifest.rs', 9, 'manifest')
     ('parse', 'run_method/runtime/trace/artifact/artifact.rs', 28, 'artifact')
     ('preview', 'mcp/lib.rs', 73, '<crate root>')
     ('render', 'core/src/registry_core/mir/mir.rs', 29, 'mir')
     ('render', 'run_method/authoring/manifest/face/face.rs', 18, 'face')
     ('search', 'mcp/lib.rs', 115, '<crate root>')
     ('search', 'studio/studio/app/keyboard_overlay.rs', 22, 'keyboard_overlay')
     ('search', 'studio/studio/app/state/state.rs', 22, 'state')
     ('search', 'studio/studio/ui/ui.rs', 33, 'ui')
     ('verify', 'mcp/lib.rs', 100, '<crate root>')
     ('walk', 'core/src/registry_core/source/source.rs', 17, 'source')
    豁免: [('build', 'core/src/registry_core/diagnostic/diagnostic.rs', 14, 'diagnostic')]
    放宽口径(含 graft/plan/record/pass): 名字 19 声明处 36

[R-4c] 违规对 6 ['app↔apply', 'ide↔identity', 'json↔jsonl', 'lex↔lexicon', 'mir↔mirror', 'render↔renderer']
    登记豁免 1 ['app↔application']
    单复数族豁免 7 ['contract↔contracts', 'diagnostic↔diagnostics', 'face↔faces', 'graft↔grafts', 'manifest↔manifests', 'node↔nodes', 'report↔reports']
```

**多站点自测（V35-02 的修法验证）**：夹具 `/tmp/t41probe/plan_lib` 正是合并方案 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:194` 落地后的形态
（`src/lib.rs` 的 crate 级 `pub mod build;` **与** `src/diagnostic/diagnostic.rs` 的 `mod build;` 并存）。
`SITE_ORDER` 把"站点访问序"做成显式输入，**同一份代码跑两遍**：

```text
[ROOT=/tmp/t41probe/plan_lib] 枚举: 模块名 2 声明处 3

[R-4a] 末 token ∈ BAD_TAIL：名字 0 声明处 0
    [顺序=walk] build: 访问序 [('/tmp/t41probe/plan_lib/src/lib.rs', 1), ('/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1)] | 新逻辑(逐站点) 报出 1 处, 豁免 1 处 | 旧逻辑(只看首站点) 报出
    访问序: build [('/tmp/t41probe/plan_lib/src/lib.rs', 1, '<crate root>'), ('/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1, 'diagnostic')]

[R-4b] 严格口径：违规声明处 1 违规名字 1 豁免声明处 1
     ('build', '/tmp/t41probe/plan_lib/src/lib.rs', 1, '<crate root>')
    豁免: [('build', '/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1, 'diagnostic')]
    放宽口径(含 graft/plan/record/pass): 名字 1 声明处 1

[R-4c] 违规对 0 []
    登记豁免 0 []
    单复数族豁免 0 []
```

```text
[ROOT=/tmp/t41probe/plan_lib] 枚举: 模块名 2 声明处 3

[R-4a] 末 token ∈ BAD_TAIL：名字 0 声明处 0
    [顺序=reverse] build: 访问序 [('/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1), ('/tmp/t41probe/plan_lib/src/lib.rs', 1)] | 新逻辑(逐站点) 报出 1 处, 豁免 1 处 | 旧逻辑(只看首站点) 豁免
    访问序: build [('/tmp/t41probe/plan_lib/src/lib.rs', 1, '<crate root>'), ('/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1, 'diagnostic')]

[R-4b] 严格口径：违规声明处 1 违规名字 1 豁免声明处 1
     ('build', '/tmp/t41probe/plan_lib/src/lib.rs', 1, '<crate root>')
    豁免: [('build', '/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1, 'diagnostic')]
    放宽口径(含 graft/plan/record/pass): 名字 1 声明处 1

[R-4c] 违规对 0 []
    登记豁免 0 []
    单复数族豁免 0 []
```

两跑的裁决**完全相同**（违规 **1 处** / 豁免 **1 处**，与访问序无关）；同一段输出里的"旧逻辑（只看首站点）"
在 `reverse` 序下会把整个名字**豁免**——这正是 t40 抓到的"门禁不确定"，现已下沉到逐站点。
另有同目录夹具 `/tmp/t41probe/libtree`（两处声明在同一目录）与另一序夹具 `/tmp/t41probe/diagfirst`，结论同样都是 1 处 / 1 处。

**三个指标的现值与验收口径**：

| 指标 | 谓词对象 | 命中（名字 / 声明处） | 扣除 | **现值（净）** | 验收 |
| --- | --- | --- | --- | --- | --- |
| **R-4a** | 命名空间名 | 14 / 14 | `support`、`misc` **= `audit-naming-review.md` D-1 的 NAM-01/NAM-02**（同一对象，文件层已计，此处只算一次） | **12 / 12**（目录 3 + 包 3 + lib 3 + 模块 3：`data`、`face_helpers`、`test_support`） | **名字数 = 0** |
| **R-4b** | 单 token 名字的**声明处**（豁免**逐站点**判，不按名字整体放过） | 16 / 27（严格动词表，**含** `build`） | 父路径限定对只豁免 `build` 的**那 1 个站点**（`diagnostic::build`）；"名字数"= 有 ≥1 个违规站点的名字数 | **15 名 / 26 处** | **声明处数 = 0**（改名，或把该 `(父, 名)` 对写进允许表） |
| **R-4c** | 名字对 | 7 对 / 13 名 | 登记豁免 1 对（`app`↔`application`，见 `ALLOWED_PAIRS`）+ 单复数族 7 对（`contract`↔`contracts` 等） | **6 对 / 12 名** | **违规对数 = 0**（改名落在更短/更泛的一侧，**或** 已登记且附一行理由） |

对照 t35/t40 的复核数：R-4b **名字数与本轮严格口径一致（16，含 `build`）**；声明处数 t35 报 33、本文按上述谓词为 27，
差额**全部来自动词表**——`graft` 少计 **7** 处（本仓把它当领域名词）、`walk` 多计 **1** 处（上一版的动词表漏了它），净 **−6** → 33−6=27；
inline `mod X {` 对 R-4b 的贡献是 **0 处**（R-4b 只收单 token 名字，27 处已在上面逐条列出）—— 口径与逐处清单可由 t42 复算。上一版的"净违规 17"把三个指标混在一起，**已删除**。

**改法表（现名 → 角色 → 建议）**：

| 现名 | 角色（它是「……的东西」） | 建议 |
| --- | --- | --- |
| `build_method` / `run_method` / `debug_method` | 构建期那一半 / 运行期那一半 / 证据那一半 | `build_time` / `runtime` / `call_evidence` |
| `data`（`studio/studio/ui/graph.rs:20`，父 `graph`） | 图面板的数据装载 | `graph_data`（保留词加限定） |
| `face_helpers`（`run_method/macros/face.rs:7`，父 `face`） | 面宏阶梯的辅助函数集 | `face_field_helpers` 或并入 `face_fields`（D-1 的同类建议） |
| `test_support`（`build_method/renderer.rs:32`，父 `renderer`） | 渲染器的测试夹具 | `test_fixtures`（`fixtures` 在本仓已是同类模块名） |
| `support` / `misc`（studio） | 见 NAM-01/NAM-02 | **不在本轮重复给建议**（= D-1/Nam-01/02 的既有结论） |
| `parse`（`core/.../authoring/parse/`） | 把清单文本变成结构的解析器 | `parser`（`renderer/` 已经是这个形状） |
| `render`（`core/.../mir/mir.rs:29`、`run_method/.../face/face.rs:18`） | 把结构变成文本的渲染器 | `renderer`（`build_method` 已有 `renderer.rs`，重名则 `render_pass`） |
| `walk`（`core/.../source/source.rs:17`） | 遍历源码目录的遍历器 | `walker`（或并入 `source_lexer`） |
| `install`（`build_method/scaffold.rs:14`） | 装插件的动作 | `installer`（或 `installation`） |
| `verify`（`mcp/lib.rs:100`） | 跑内核校验并报差异的工具 | `verification`（`plugin-host/src/verifier.rs` 已占用 `verifier`） |
| `preview`（`mcp/lib.rs:73`） | 预览副本与 diff | `preview_copy` / `preview_report` |
| `apply`（`mcp/lib.rs:67`、`core/.../graft_ops/record.rs:40`） | 带预览的创作写入 / 应用一条记录 | `authoring_write` / `apply_record`（工具名仍叫 `xirang.apply`） |
| `search` / `explain` / `diff` / `converge`（mcp） | 各工具的查询实现 | `search_tool` / `explain_tool` / `diff_tool` / `converge_tool`；若维护者更看重"文件名 = 工具动词"的镜像，整族写进允许表并登记 |
| `add`/`edit`/`delete`/`create`（`run_method/.../operations.rs:9/12`、studio 的键盘层） | 事务与表单的操作 | 父路径限定（`operations`/`transaction`）可写进允许表；若要改名：`addition`/`editing`/`deletion`/`creation` |
| R-4c 的 7 对 | 见下 | `app`↔`apply`/`application`：strudio 的 `app` 与 mcp 的 `apply` 至少一个加限定；`json`↔`jsonl`：`json` 改 `json_text`（AMB-12）后自然消解；`lex`↔`lexicon`：`lex` → `lexer`；`mir`↔`mirror`：`mirror` 是宏镜像，建议 `face_mirror`；`ide`↔`identity`：`ide` → `ide_mirror`；`render`↔`renderer`：随 R-4b 的 `render` 一起消解 |

### 落地顺序

1. **R-4b（声明处 15 名 / 26 处，必须降到 0）**：先改三件事——`build_method`/`run_method`/`debug_method` 三个目录
   （与合并方案同步）、`parse/` → `parser/`、`install` → `installer`；其余 12 个名字给一次整族表决
   （镜像 or 加 `_tool`/`_report` 等后缀），**或**把父路径限定写入 `ALLOWED_QUALIFIED`（棘轮式：只许变短）。
2. **R-4a（名字 12 / 声明处 12，必须降到 0）**：`data`/`face_helpers`/`test_support` 三个模块加限定词；
   三个 `*_method` 目录随合并消失（与第 1 条同一次改动）。
3. **R-4c（7 对，必须降到 0）**：`json`↔`jsonl` 与 `render`↔`renderer` 随第 1/2 条自然消解；`mir`↔`mirror`、`ide`↔`identity`、
   `lex`↔`lexicon`、`app`↔`apply`/`application` 各改一侧；单复数族（7 对）写进豁免表。
4. **R-1**：纯文本门禁，先钉现状 2 处（`core`、`examples`），棘轮式收口（合并方案批 1 之后应降到 1）。
5. **R-3**：四条都是「同名不同物」的交叉表判定，**在合并方案落地前先跑一遍**，避免把新歧义焊进新结构。
6. **R-2**：与合并方案的模块名同一次表决（`build`/`run`/`plugins` 正是 R-2 命中项）；词表（R-2 的登记行）
   与 `audit-naming-review.md` 的 D-9 **共用一份**（建议落成 `conventions` 能读的常量，`lexicon` 已有先例），避免两份漂移。

## 6. 查了、干净（覆盖口径）

- **全量枚举（机器）**：顶层目录 14、成员目录 12、包名 12、lib 名 12、bin 名 5、**模块名 296（名字）/ 440（声明处）**、
  feature 名 8（跨 12 个清单去重并集，含 `default`；合并方案为新 crate 设计的 13 个是另一对象）。
  六类**逐个**与 §5 的**一张**保留词表（D-9+R-2）及 R-4a/b/c 三个谓词比对（脚本可复算）。
- **全量人读**：内核 13 个模块的**文档首行**与主要公开 API（`authoring declaration diagnostic identity json lexicon mir
  plugin release requirements source syntax tree`）；方案 §2 的 7 个模块名 + 3 个 crate 名 + 13 个 feature 名
  （`:38-40`、`:62-72`、`:89-93`、`:122-126`、`:194`）；§1/§2 每条现状名的原文与行为证据。
- **抽样人读**：831 个公开名里**整名**命中词表的 13 个（AMB-20 的表，逐个读定义处）；按分词命中词表的 212 个
  （`graft` 36 / `face` 35 / `plugin` 28 / `source` 25 / `trace` 22 / …）——抽读了 `RegistryIndex`、`SourceLocation`、
  `PluginSource`、`PluginArtifact`、`HostEntry`/`host!()`、`EvidenceKind`/`CallEvidence` 等，其余按「已带限定词」放过。
- **只做机器比对、没逐个人读**：296 个模块名里的 275 个（只与谓词比对；被点名的 21 个逐个读过，296−275=21）、
  ≈800 个公开名、76 处 `mod tests`（按豁免处理）。
- **维护者三个点名案例**都给了完整三段式（§2.1/2.2/2.3），其中 `plugin-host` 的行为清单是我**逐文件核实**的
  （verifier → admission → wasm/process → lazy_wasm → deployment 五段，与维护者给的行为链一致）。
- **与既有报告的交叉引用**：`audit-naming-review.md` §D（`:387`）的文件/函数层规则不重复计数；
  D-9 的 `evidence`/`index`/`artifact`/`registry` 四个词在本报告只补**类型/模块名层**的实例；
  `audit-naming-verify2.md:42-47` 已裁定的 `call_evidence` 只引用不重开；
  `graft/` 挂在内核 `plugin/` 下的归属问题**只给判据、不下结论**（交结构轴）。

- **门禁与锚点自检（t39 首次、t41 复跑）**：`cargo test -p xirang-conventions --offline` = **99 passed / 0 failed**；
  本报告锚点脚本 = **116 处 `file:line` 全部为真**（0 violation）、**16 处配对 token 0 suspect**；
  R-4 的多站点自测（`SITE_ORDER=walk|reverse`，夹具 `/tmp/t41probe/{plan_lib,libtree,diagfirst}`）两跑裁决相同（报出 1 处 / 豁免 1 处）；
  源码一行未改（`git status` 中本目录外无新增改动）。
