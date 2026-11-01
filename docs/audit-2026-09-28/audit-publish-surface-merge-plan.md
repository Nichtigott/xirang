# t26 发布面合并方案：9 → 3（`xirang-kernel` / `xirang-macro` / `xirang-toolchain`）

> 任务 t26 / t31 / t36（boundary-architect）。**本轮只出方案，源码一行不改**：主产物是本文；另按队长指示在同一轮修另一份**我自己的**产物 `audit-boundary-refactor-plan.md` 的 R-05（跨报告冲突，见「修订记录（t36）」第 10 条）。
> **维护者已拍板，本文不再讨论「要不要合」**，只回答「怎么合、怎么分批、怎么不弄坏身份」。
> 命名说明：任务书写的文件名 `audit-publish-surface-merge-plan.md` 本身就满足队长 2026-09-28 的硬规则（本目录下每个文件名以 `audit-` 开头，`conventions/src/doc_blocks.rs:69` 的 `RECORD_PREFIXES` 据此豁免），因此本文直接以该名落盘，没有产生旧名文件。
> 口径：本文每个数字都由本人在工作树（HEAD `cf0c378` + 未提交改动）复算，复算命令随数字给出；队长转达的计数只作对照，不直接引用（见 §3.1 的差异说明）。

---

## 修订记录（t36：按命名歧义专项 t34 + 其独立复核 t35 同步模块名）

> 依据：`docs/audit-2026-09-28/audit-naming-ambiguity.md`（t34）与 `docs/audit-2026-09-28/audit-naming-ambiguity-verify.md`（t35，**以复核裁定为准**）。下面**每一处都点名文件 + 行号**（t35 §4.1 的教训：不点名文件的行号会把人引到另一份方案上）。
> 规则：**只改名字不改分析不算合格**——凡某个约束因改名消失、或某段分析因此失效，在原文里写明"已消失 / 仍存在"（见 §2.1、§2.2 的改写）。

| # | 旧名 | 新名 | 依据（点名文件 + 行号） | 连带改动 |
| --- | --- | --- | --- | --- |
| 1 | 模块 **`build`** | **`build_time`** | `audit-naming-ambiguity.md:363`（建议 `build_time`，与 `AGENTS.md` 该 crate 自述 "Build-time filesystem / `OUT_DIR` orchestration" 同词）；`audit-naming-ambiguity-verify.md:101`（crate/目录层必换、模块层可换，单独看属 MINOR） | §1.1 表、§1.2 目录树、§2.1 `build_time::run()`、§2.2 碰撞表与根代码、§3.1/§3.6 替换目标、§4.2 `git mv`、§4.4 脚手架、§5.1 `shims` 路径、§5.5、§10.1 |
| 2 | 模块 **`run`** | **`runtime`** | `audit-naming-ambiguity.md:364`；`audit-naming-ambiguity-verify.md:102`（**理由须改成"删掉一条只为躲重名而设的规则"，不是"否则 E0428"**） | 同上；**另**：§2.2 整段改写（E0428 那条约束**已消失**，见该节的"消失/仍存在"表），§2.1 的构建入口解释同步 |
| 3 | 模块 **`plugins`** | **`plugin_host`** | `audit-naming-ambiguity.md:365`（与 `.xirang/plugins` = `PLUGIN_LOCK_DIRECTORY`（`plugin-host/src/admission.rs:38`）**同词反义**）；`audit-naming-ambiguity-verify.md:103`（**必须换**，MAJOR 维持） | §1.1、§1.2、§2.2、§3.1/§3.6、§4.2、§5.1、§10.1 |
| 4 | 模块 `call_evidence` / `studio` / `mcp` / `cli` | **保留** | `audit-naming-ambiguity.md:366-369`（四条全判"是"）；`audit-naming-ambiguity-verify.md:104-107`（复核通过） | §1.1 的「处置」列与 §10.1 写明**保留判据**（不默默不动） |
| 5 | crate `xirang-kernel` / `kernel/` | **保留** | `audit-naming-ambiguity.md:373-375`（"方案里最值钱的一次改名"）；t35 未否 | §1.1 加一句：README 首段需写"kernel here = 纯协议词汇与纯方法"（t34 `:375`） |
| 6 | crate `xirang-toolchain` / `toolchain/` | **已拍板：保留** | `audit-naming-ambiguity-verify.md:109-118`（"`toolchain` 被 rustup/rustc 占用"**成立但判 MINOR**；建议保留 + README 一行；备选 `xirang-surfaces`） | §1.1/§1.2/§5.5 各加一句，并落到文书：**README/AGENTS 加一行"本仓的 toolchain = 执行面集合，不是 Rust 工具链"**。备选 `xirang-surfaces` 不再采用 |
| 7 | feature `build` / `run` / `evidence` / `plugins` / …（13 个） | **保留原名** | `audit-naming-ambiguity.md:382`（"建议配一张 README 对照表；`default = []` 与 `required-features` 的设计本身是对的"）；`audit-naming-ambiguity-verify.md:124`（"它建议保留 `evidence` 与 feature 名 ✓"） | §1.3 的注释改成新模块名 + 新增一段"feature 名与模块名不同名是**有意**的" + README 对照表要求 |
| 8 | bin 名（5 个） | **保留** | `audit-naming-ambiguity-verify.md:125`（五个 bin 全部保留 + `required-features` + `autobins = false` ✓） | §1.4 加注：`audit-naming-ambiguity.md:160` 的 AMB-08（命名建议在 `:168`）建议 `xirang-dev` → `xirang-supervisor`（MINOR），**已拍板：保留 `xirang-dev`**——bin 名是脚本/CI 的既有契约，改名成本大于收益 |
| 9 | 文档锚点代价 | **新增一段** | `audit-naming-ambiguity-verify.md:126`（t34 的四个数复算**逐字一致**） | §3.2 新增「文档锚点代价」段，带**范围声明**（是否含本轮审计报告） |
| 10 | 跨报告冲突（另一份产物） | `audit-boundary-refactor-plan.md` 的 **R-05** | `audit-naming-ambiguity-verify.md:219`（挂账）＋ `audit-naming-review.md:404-412`（NAM-06 的最终裁定） | 该文件的 R-05 已由"把 `writers.rs` 并入 `mutations.rs`"改为"改名 `write_guard.rs`、保留守卫缝"，并注明原修法已被复核否决 |

**本方案的模块名最终清单**：`build_time` / `runtime` / `call_evidence` / `plugin_host` / `studio` / `mcp` / `cli`（7 个）；**feature 名不随之变**，仍是 `build` / `run` / `evidence` / `plugins` / `wasm` / `process-tools` / `node-graph` / `studio` / `mcp` / `cli` / `authoring` / `prototype-fixtures` / `dev-supervisor`。

---

## 0. 结论速览

| 项 | 前 | 后 |
| --- | --- | --- |
| 工作区成员 | 12（9 发布 + `conventions` + 2 examples） | 6（3 发布 + `conventions` + 2 examples） |
| 发布 crate | 9 | **3** |
| crate 目录 | `core/ macro/ build_method/ run_method/ debug_method/ plugin-host/ studio/ mcp/ cli/` | `kernel/ macro/ toolchain/` |
| lib 名 | `xirang`（内核）等 9 个 | `xirang_kernel` / `xirang_macro` / `xirang_toolchain` |
| 裸名 `xirang` | 内核的 lib 名 | **退役**（没有任何 crate 再叫它） |
| `[[bin]]` | 5 个，分布在 3 个包 | 5 个，全在 `xirang-toolchain`，靠 `required-features` 分别门控 |
| 发布顺序 | 7 层 9 个名字 | **3 层 3 个名字** |
| 内部版本要求 | 20 处 / 11 个清单 | 发布面 **3 处 / 2 个清单**（`macro→kernel` 1、`toolchain→kernel` 1、`toolchain→macro` 1）；连示例宿主算 **7 处 / 5 个清单** |
| 一次版本推进要改的文件 | 13（11 个 `Cargo.toml` + `Cargo.lock` + `CHANGELOG.md`） | 发布面 **5**（根 + `macro` + `toolchain` + `Cargo.lock` + `CHANGELOG.md`）；含示例宿主 7 |

**为什么合（写进方案开头，全部为实测）：**
1. **没有「只发一个 crate」的路径**：`tools/xirang-publish:462` 的 `metadata_requirement_problems()` 强制每条内部要求 `== ^<workspace version>`，而 `--check-table` 在 CI（`.github/workflows/ci.yml:134`）与发布工作流里都是必过步骤。当前工作区版本 `0.1.6` 在清单里出现 **20 处 / 11 个文件**（`grep -rn 'version = "0.1.6"' --include='Cargo.toml' . | grep -v /target/ | wc -l`，逐文件分布见下）。合并后**发布面只剩 3 处 / 2 个清单**（`macro → kernel`、`toolchain → kernel`、`toolchain → macro`），连两个示例宿主算上共 7 处 / 5 个清单（示例那 3 处按 t14 R-28 的建议可以直接删掉版本号）。也就是说：**"独立发版"从来不存在，而 9 个包要求每次版本推进同步改动 20 处——合并后发布面只需 3 处（**−85%**），连示例宿主算 7 处（−65%）**。
2. **没有用户**：0.1.0–0.1.5 已发布（`CHANGELOG.md:14`），但维护者确认无外部使用者；`tools/xirang-publish --verify-consumers` 至今只验证过工作流自己建的一次性消费者。
3. **合并在身份上安全**：十个发布 crate 的 `src/` 里**生产注册面声明为 0**（所有 `*_object!` 命中都落在文档注释、夹具字符串、`#[cfg(test)]` 模块或宏定义自身四类里，见 §7）。
4. **7 个执行面本来就在一条链上**：`cli → mcp/studio → debug_method → run_method → core`，且 `cli` 的依赖闭包是 9 个发布 crate 里的 **8 个**；真正被独立消费的只有 `core`（内核）与 `macro`（proc-macro）。

---

## 1. 目标结构

### 1.1 crate 清单前后对照

| 前 | 目录 | 后 | 目录 | 处置 |
| --- | --- | --- | --- | --- |
| `xirang-core`（lib `xirang`） | `core/` | **`xirang-kernel`**（lib `xirang_kernel`） | **`kernel/`** | 改名 + 目录改名（**保留**，t34 `:373-375` 判"方案里最值钱的一次改名"）；`LIB_NAME_EXCEPTIONS` 里那条例外随之删除（§6）。**README 首段需加一句**"kernel here = 纯协议词汇与纯方法"，因为 Rust 生态里 `kernel` 强指 OS/no_std（t34 `:375`） |
| `xirang-macro` | `macro/` | `xirang-macro`（lib `xirang_macro`） | `macro/` | **包与目录不动**（proc-macro 物理约束）；只把它 `[dependencies]` 里的 `xirang-core` 改成 `xirang-kernel`（`macro/Cargo.toml:20` 的 `features = ["syntax"]` 不变） |
| `xirang-build-method` | `build_method/` | **`xirang-toolchain`**（lib `xirang_toolchain`）模块 **`build_time`** | **`toolchain/`** | 并入（模块名见「修订记录（t36）」#1；合并后的 crate 名 `xirang-toolchain` **已拍板：保留**，理由见 #6——本仓的 toolchain = 执行面集合，不是 Rust 工具链） |
| `xirang-run-method` | `run_method/` | 同上，模块 **`runtime`** | | 并入（#2；**同名嵌套代价见 §2.2**） |
| `xirang-debug-method` | `debug_method/` | 同上，模块 **`call_evidence`**（t30 的 D-9 要求保留词加限定词，见 §10.1） | | 并入 |
| `xirang-plugin-host` | `plugin-host/` | 同上，模块 **`plugin_host`** | | 并入（#3：`plugins` 与 `.xirang/plugins` 同词反义） |
| `xirang-studio` | `studio/` | 同上，模块 `studio` | | 并入（**保留**：产品名，包/bin/README 三处一致 —— t34 `:367`、t35 `:105`） |
| `xirang-mcp` | `mcp/` | 同上，模块 `mcp` | | 并入（**保留**：协议缩写，唯一含义 —— t34 `:368`、t35 `:106`） |
| `xirang-cli` | `cli/` | 同上，模块 `cli` | | 并入（**保留**：约定缩写，`cargo --help` 里没有 `cli` 子命令 —— t34 `:369`、t35 `:107`） |
| `xirang-conventions`（`publish = false`） | `conventions/` | 不动 | `conventions/` | 只改门禁里的名字与路径（§6） |
| 两个 example（`publish = false`） | `examples/` | 不动 | `examples/` | 只改依赖声明与入口路径（§3） |

### 1.2 目录树（合并后）

```text
kernel/                       xirang-kernel, lib xirang_kernel
  src/lib.rs                  #[path = "registry_core.rs"] pub mod registry_core;（内部 339 条挂载全部不变）
  src/registry_core/{identity,declaration,tree,syntax,authoring,diagnostic,
                     requirements,plugin,mir,source,release,json,lexicon}
  tests/{nesting_budget.rs,ungated_authoring_data.rs}
  README.md / README.zh-CN.md

macro/                        xirang-macro, lib xirang_macro（不动）

toolchain/                    xirang-toolchain, lib xirang_toolchain, autobins = false
                              ← 名字保留（t35 `:109-118`）：本仓的 toolchain = 执行面集合，不是 Rust 工具链（**已拍板：保留**，备选 xirang-surfaces 不采用）
  Cargo.toml                  5 个 [[bin]] + 13 个 feature（§1.3 / §1.4）
  src/lib.rs                  7 条 #[path] 挂载 + pub use runtime::*（§2.2 的碰撞分析）
  src/build_time/     ← build_method/   （build_time.rs ← lib.rs，其余相对路径全部不变）
  src/runtime/        ← run_method/     （runtime.rs ← lib.rs；**内含同名子模块** `runtime/runtime.rs`，见 §2.2 的同名嵌套记账）
  src/call_evidence/  ← debug_method/src/   （call_evidence.rs ← lib.rs）
  src/plugin_host/    ← plugin-host/src/    （plugin_host.rs ← lib.rs）
  src/studio/         ← studio/         （studio.rs ← lib.rs；studio/ ← studio/studio/）
  src/mcp/            ← mcp/            （mcp.rs ← lib.rs）
  src/cli/            ← cli/            （cli.rs ← lib.rs；commands/、explain*.rs）
  src/bin/xirang.rs  xirang-studio.rs  xirang-dev.rs  xirang-mcp.rs  cargo-xirang.rs
  tests/              17 个测试/示例文件并拢（实测 0 个同名冲突，§5.2）
  examples/scale_audit.rs
  README.md / README.zh-CN.md

conventions/                  publish = false（12 道门禁；只改名字与路径）
examples/control-button/      publish = false（宿主示例；依赖与入口路径改，**面文件不移动**）
examples/control-button-graft/ publish = false（项目外实现；同上）
```

### 1.3 feature 设计（完整）

```toml
[features]
default = []                       # 关键决定：宿主面向的 crate 不能默认拖 TUI

# ── 宿主面向（两半，分别对应 build-dependencies 与 dependencies）
build          = ["xirang-kernel/syntax", "dep:syn", "dep:proc-macro2", "dep:serde_json"]  # 模块 build_time
run            = ["dep:xirang-macro"]                                                      # 模块 runtime

# ── 执行面（今天各自的公开 API 门控，名字不变）
authoring      = ["run", "xirang-kernel/syntax", "dep:syn"]          # 原 run_method/authoring
evidence       = ["run", "dep:inventory", "dep:petgraph", "dep:tracing"] # 模块 call_evidence（原 debug_method，无特性整包）
plugins        = ["run", "dep:arc-swap", "dep:ed25519-dalek"]          # 模块 plugin_host（原 plugin-host 的"常驻内核"）
wasm           = ["plugins", "dep:wasmi"]                              # 原 plugin-host/wasm（原默认开）
process-tools  = ["plugins", "dep:tempfile"]                           # 原 plugin-host/process-tools
node-graph     = ["dep:rataflow"]                                      # 原 studio/node-graph（原默认开）
studio         = ["build", "run", "authoring", "evidence",
                  "dep:crossterm", "dep:notify", "dep:ratatui", "dep:serde_json"]
mcp            = ["build", "run", "authoring", "evidence", "dep:serde_json"]
cli            = ["build", "run", "studio", "mcp", "dep:serde_json"]   # 伞：原 xirang-cli 的闭包

# ── 仅工作区（不进发布面：一个测试夹具、一个只在本检出能跑的监督器）
prototype-fixtures = []            # 原 studio/prototype-fixtures（保持默认关）
dev-supervisor     = []            # 原 studio/dev-supervisor（保持默认关）
```

**三条设计依据（都可复算）**：
1. **feature `build` / `run`（模块 `build_time` / `runtime`）必须分开**：宿主把同一个包写进 `[dependencies]` 与 `[build-dependencies]` 是合法的，而 `resolver = "2"`（`Cargo.toml:3`）**不会把 build-dependency 与普通依赖的 feature 合并**——因此 `build` 那一遍只编构建侧代码（`syn`/`proc-macro2`/`serde_json`），目标那一遍只编运行期代码（`macro`），**依赖与编译量都不比今天多**。
2. **`build` 不需要 `run`**：`build_method` 今天只依赖 `core`（t7 §3.1 的依赖图），它对 `xirang_run_method` 的 29 处引用全在测试/夹具字符串里（今天能编译就是证明：清单里没有 run_method 依赖）。所以宿主的 `build.rs` 那一遍拿不到运行期代码。
3. **`cli` 必须蕴含 `studio`+`mcp`**：`cli/lib.rs:150` 直接调 `xirang_mcp::run()`、`cli/commands/studio.rs:42` 直接调 `xirang_studio::launch_with()`。这正是今天 `cargo install xirang-cli` 会编 8 个 crate 的原因；合并后它变成一条 `--features cli`。

**feature 名与模块名不同名是**有意**的（t34 `:382` + t35 `:124`）**：本表 13 个 feature 名**全部保留**（`build`/`run`/`evidence`/`plugins`/`wasm`/`process-tools`/`node-graph`/`studio`/`mcp`/`cli`/`authoring`/`prototype-fixtures`/`dev-supervisor`），而模块名按「修订记录（t36）」改了三个 —— 于是出现 `feature build` 门控模块 `build_time`、`feature run` 门控模块 `runtime`、`feature plugins` 门控模块 `plugin_host`、`feature evidence` 门控模块 `call_evidence` 这四组**同名不同域**。t34 的裁定是：feature 是"可选能力的开关"，名字应当是能力名，**不要求**与模块异名；`--all-features`、`required-features` 与 CI 因此**零改动**。代价是读者需要在 README 里看到一张对照表。

**README 必须加一张对照表**（feature ⇄ 模块 ⇄ 原 crate 名，4 行）：`build`→`build_time`→`build_method`；`run`→`runtime`→`run_method`；`plugins`→`plugin_host`→`plugin-host`；`evidence`→`call_evidence`→`debug_method`。这张表是**唯一**把两套名字钉在一起的地方，`AGENTS.md` 与此处都应指向它。

**与今天的行为差异（必须写进 README）**：`plugin-host` 今天 `default = ["wasm"]`、`studio` 今天 `default = ["node-graph"]`。合并后 `default = []`，因此
- 想要 wasm 后端的用户写 `features = ["wasm"]`（不再"默认自带"）；
- Studio 的 `node-graph` 需要显式写出（见 §1.4 的两个选项）。

### 1.4 `[[bin]]` 设计（bin 名全部保留）

| bin | 今天属于 | 新 `path` | 新 `required-features` | 安装命令 |
| --- | --- | --- | --- | --- |
| `xirang` | `xirang-cli` | `src/bin/xirang.rs` | `["cli"]` | `cargo install xirang-toolchain --features cli` |
| `cargo-xirang` | `xirang-cli` | `src/bin/cargo-xirang.rs` | `["cli"]` | 同上（`cargo xirang …` 不变） |
| `xirang-studio` | `xirang-studio` | `src/bin/xirang-studio.rs` | `["studio"]` | `cargo install xirang-toolchain --features studio,node-graph` |
| `xirang-dev` | `xirang-studio` | `src/bin/xirang-dev.rs` | `["dev-supervisor", "studio"]` | `cargo run -p xirang-toolchain --features dev-supervisor,studio,node-graph --bin xirang-dev -- watch` |
| `xirang-mcp` | `xirang-mcp` | `src/bin/xirang-mcp.rs` | `["mcp"]` | `cargo install xirang-toolchain --features mcp` |

**bin 名保留的理由**：bin 名是用户界面（PATH 上的命令、`cargo xirang` 的插件名、脚本与 README 里的调用），包名才是这次要动的；`required-features` 让"装哪个"继续由 feature 决定。**t35 `:125` 复核确认**：五个 bin 名全部保留 + `required-features` + `autobins = false` 的设计本身是对的。

**一处已拍板的 bin 命名**：`audit-naming-ambiguity.md:160` 的 AMB-08（MINOR；命名建议句在 `:168`）指出 `xirang-dev` 这个 bin 名与它门控的 feature `dev-supervisor` 不同词（生态里 `x-dev` 通常读成"x 的开发构建"），建议改 `xirang-supervisor`。**已拍板：保留 `xirang-dev`**——bin 名是脚本/CI 的既有契约，改名成本大于收益。这条与 §1.1 的 `xirang-toolchain`（同样**已拍板：保留**）属于同一类"名字有歧义但成本大于收益"的决定。

**`node-graph` 的取舍（两个选项，推荐 A）**：
- **A（推荐）**：`studio` **不**蕴含 `node-graph`；文档安装行写 `--features studio,node-graph`。好处：`studio` 在没有 `rataflow` 时仍能编译（今天 `studio/README.md` 承诺"没有该特性时调用树面板画边框并说明"这条性质保住了）；代价：安装命令多一个 feature。
- **B**：`studio = [..., "node-graph"]`。安装命令最短，但**那条降级性质永久失去**（Cargo 的 feature 只能加不能减，隐含了就无法关闭）。
- 注意：今天 `cargo install xirang-cli` 会**隐式**带上 `node-graph`（`cli → studio` 用的是 studio 的默认特性）。选 A 后，等价命令是 `--features cli,node-graph`；这条差异必须写进 README，否则用户会拿到一个"少了调用树"的 Studio。

**`autobins = false` 是必须的**：五个 bin 都在 `src/bin/` 下，若让 cargo 自动发现，它们会带上**空的** `required-features`，于是 `cargo install xirang-toolchain`（无 feature）会装出全部五个 bin。设 `autobins = false` 并逐条声明 `[[bin]]`（沿用今天 `cli`/`core`/`plugin-host`/`conventions` 的写法）后，`required-features` 才真正生效。

**无 feature 时的行为**：`default = []` 意味着 `cargo install xirang-toolchain`（不加 `--features`）会报"没有可安装的 bin"。这是**刻意的**：同一个包既服务宿主（`features = ["run"]`/`["build"]`）又出厂工具，默认值必须偏向不拖累宿主。README 的三条安装命令因此是唯一入口，`cargo install` 的错误信息本身就是提示。

---

## 2. 宿主用法与 API 路径

### 2.1 合并后的宿主

```toml
# 宿主的 Cargo.toml
[dependencies]
xirang-toolchain = { version = "0.2", default-features = false, features = ["run"] }

[build-dependencies]
xirang-toolchain = { version = "0.2", default-features = false, features = ["build"] }
```

```rust
// build.rs —— 唯一一行（今天是 xirang_build_method::run()）
fn main() { xirang_toolchain::build_time::run(); }
```

```rust
// src/lib.rs —— 今天写法：xirang_run_method::host!();
xirang_toolchain::host!();

// 可选：源码范围发现提示（今天写法 xirang_run_method::application!(entry = …)）
xirang_toolchain::application!(entry = crate::app::run);
```

**为什么构建入口写 `xirang_toolchain::build_time::run()`**（t36 改写，旧叙述已失效）：`build_time` 模块导出一个 `run` 函数（今天 `build_method/lib.rs:137`）。**原文说"否则 `pub mod run;` 与 `pub use build::run;` 会在根上重名（E0428）"——这条约束已因改名消失**：模块名现在是 `runtime`，根上不再有名为 `run` 的模块（`audit-naming-ambiguity.md:364`、`audit-naming-ambiguity-verify.md:102`）。**仍然存在的是另一条约束**：本 crate 里有三个不同的函数叫 `run`（`build_time::run` / `mcp::run` / `cli::run`），根上不能同时 re-export 三个同名项，因此工具侧入口一律按模块路径寻址（§2.2 的"消失/仍存在"表）。
同理，`mcp::run()`（`mcp/lib.rs:58`）与 `cli::run(argv)`（`cli/lib.rs:124`）也留在各自模块里——根上**只** glob 宿主面向的 `runtime` 模块。

**宏的位置**：`host!` / `application!` / `static_graft_plan!` / `trace_call!` / `*_object!` 都是 `#[macro_export]`，合并后自动落在 `xirang_toolchain` 的 crate 根，所以 `xirang_toolchain::host!()` 成立；它们只在 **feature `run`** 打开时存在（宿主本来就开着 `run`；注意 feature 名保留 `run`、模块名叫 `runtime`，见 §1.3 的对照表）。

**生成计划（`OUT_DIR/generated_lib.rs`）**：渲染器用内核词表里的**一个常量**寻址运行期 crate——`core/src/registry_core/lexicon/lexicon.rs:28`（批 1 后为 `kernel/src/…`）的 `pub const RUN_METHOD_CRATE: &str = "xirang_run_method";`（被 `build_method/renderer/aliases.rs:27,34`、`macro/src/mirror.rs:172,321`、`macro/src/lib.rs:367` 使用）。改成 `"xirang_toolchain"` 之后，生成代码里的 `::xirang_toolchain::registry_core` 与 `::xirang_toolchain::__xirang_object!` 自动成立，因为 `toolchain` 根重导出了内核的 `registry_core`（今天的 shim，见 §2.2）。**这是"一次改名、全树生效"的那个点。**

### 2.2 新 crate 根的重导出策略（含一次真实的命名碰撞）

**t36 重写：先分清"哪条约束已因改名消失、哪条仍然存在"**（这是本节唯一正确的读法；旧版把两者混在一句里，会让人以为 E0428 是当前阻塞）。

| 约束 | 状态 | 依据 |
| --- | --- | --- |
| `pub mod run;` 与 `pub use build::run;` 在 crate 根重名（**E0428**） | **已消失** | 模块名改为 `runtime` 后，根上不再有名为 `run` 的模块（`audit-naming-ambiguity.md:364`、`audit-naming-ambiguity-verify.md:102`） |
| 三个不同的函数同名 `run`，不能同时 re-export 到根 | **仍存在** | `build_time::run()`（今天 `build_method/lib.rs:137`）、`mcp::run()`（`mcp/lib.rs:58`）、`cli::run(argv)`（`cli/lib.rs:124`） |
| 7 个模块名本身撞到某个根导出 | **不存在**（实测） | 见下表：三个 `run` 是**函数**，与模块名 `runtime` 无关 |

| 模块 | 它携带的、会与根上同名项相撞的公开名 |
| --- | --- |
| `build_time` | **`run`**（函数：构建脚本入口） |
| `mcp` | **`run`**（函数：桥入口） |
| `cli` | **`run`**（函数：argv 分发） |
| `runtime` / `call_evidence` / `plugin_host` / `studio` | 无 |

因此 `toolchain/src/lib.rs` 的规则是：**只 glob 宿主面向的 `runtime` 模块**（宿主面向词汇：`registry_core`、`tree`、`lexicon`、`CallTrace`、`EvidenceKind`、`face_fields` 等），其余六个模块一律用模块路径寻址。**改名带来的收益是"删掉一条只为躲重名而设的规则"**（t35 `:102` 的原话），不是修掉一个编译错误。复算命令：

```sh
grep -rnE '^pub fn run\b' --include='*.rs' build_method/src cli/src mcp/src
```

```rust
// toolchain/src/lib.rs（形态示意）
#[cfg(feature = "build")]    #[path = "build_time/build_time.rs"]       pub mod build_time;
#[cfg(feature = "run")]      #[path = "runtime/runtime.rs"]             pub mod runtime;
#[cfg(feature = "evidence")] #[path = "call_evidence/call_evidence.rs"] pub mod call_evidence;
#[cfg(feature = "plugins")]  #[path = "plugin_host/plugin_host.rs"]     pub mod plugin_host;
#[cfg(feature = "studio")]   #[path = "studio/studio.rs"]       pub mod studio;
#[cfg(feature = "mcp")]      #[path = "mcp/mcp.rs"]             pub mod mcp;
#[cfg(feature = "cli")]      #[path = "cli/cli.rs"]             pub mod cli;

#[cfg(feature = "run")]
pub use runtime::*;    // 宿主面向的唯一 glob（含 registry_core / tree / lexicon shim）
```

> 上面的 `#[cfg(feature = …)]` 用的是**feature 名**（保留：`build`/`run`/`evidence`/`plugins`/…），而 `pub mod` 用的是**模块名**（新：`build_time`/`runtime`/`call_evidence`/`plugin_host`/…）——两套名字在这里第一次并排出现，不是笔误（§1.3 的对照表）。

**改名带来的一处新代价（记账，不是缺陷）**：`run_method` 自己也有一个 `runtime` 模块（`run_method/lib.rs:27` 的 `pub mod runtime` → `run_method/runtime/runtime.rs`），而它被 `conventions/src/shims.rs:54` 的 `SHIMS` 棘轮钉住（下游可写的公开路径）。并进来之后它将落在 `toolchain/src/runtime/runtime/`，于是出现 **`toolchain::runtime::runtime::trace`** 这种同名嵌套；同时移动后的 lib 体里那句 `pub use runtime::*;` 会把名字 `runtime` 带到 crate 根，而根上已有一个**显式**的 `pub mod runtime` —— Rust 的 glob 优先级低于显式项，**不会报错**，但那个内层模块在根上**不会被 re-export**（仍可经 `toolchain::runtime::runtime` 到达）。**本轮不改内层名**（它是 `shims.rs:54` 钉住的公开路径，改名要动棘轮 + 下游路径），把它列为命名专项/维护者的后续项；README 的模块表必须写清这两层的区别。

**每个被并入的 `lib.rs` 变成模块文件后要做三件小事**（§5.1 有逐项步骤）：① 顶部的 `//!` crate 文档变成该模块的文档（内容不变，仍然双语）；② `#![warn(missing_docs)]` 只在 `toolchain/src/lib.rs` 保留一份，其余 6 处删掉（它们不再是 crate 根）；③ 各 crate 根里的 `pub use xirang::...` shim 改成 `pub use xirang_kernel::...`（§6 的 `shims` 门禁按新路径与新品针钉住）。

---

## 3. 机械改名清单与计数

### 3.1 lib 路径引用（本人复算）

口径：`.rs` 文件里形如 `<lib名>::` 的引用，使用词边界 `(?<![A-Za-z0-9_])`，因此 `registry_core::`、`xirang_run_method::` 不会污染 `xirang::`。命令：

```sh
python3 - <<'EOF'   # 逐文件统计 <lib>:: 出现次数
import os,re
pats=['xirang','xirang_run_method','xirang_build_method','xirang_debug_method',
      'xirang_plugin_host','xirang_macro','xirang_studio','xirang_mcp','xirang_cli']
for f in ...:   # 遍历 .rs（排除 target/）
    for n in pats: len(re.findall(r'(?<![A-Za-z0-9_])'+n+r'::', text))
EOF
```

| 旧 lib 名 | 出现次数 | 文件数 | 新写法 |
| --- | ---: | ---: | --- |
| `xirang::` | **237** | 88 | `xirang_kernel::` |
| `xirang_run_method::` | **275** | 93 | `xirang_toolchain::`（根 glob **`runtime`**） |
| `xirang_build_method::` | **83** | 45 | `xirang_toolchain::build_time::` |
| `xirang_debug_method::` | **8** | 7 | `xirang_toolchain::call_evidence::` |
| `xirang_plugin_host::` | **15** | 5 | `xirang_toolchain::plugin_host::` |
| `xirang_cli::` | **5** | 2 | `xirang_toolchain::cli::` |
| `xirang_macro::` | **4** | 1 | **不变**（`macro` 保持独立） |
| `xirang_studio::` | **2** | 2 | `xirang_toolchain::studio::` |
| `xirang_mcp::` | **2** | 2 | `xirang_toolchain::mcp::` |
| **合计** | **631** | **192** | — |

**与队长转达数的差异（已复算）**：`xirang::` 237/88、`xirang_build_method::` 83/45、`plugin_host` 15/5、`debug_method` 8/7、`macro` 4/1、`mcp` 2/2、`studio` 2/2 **完全一致**；`xirang_run_method::` 我量到 **275/93**，转达数是 298/98——差异来自是否把 `::` 前缀写法（`::xirang_run_method::…`）与文档注释里的裸名计入。我的口径是"任何 `<lib>::` 形态"，包含 `::` 前缀；请以本节命令为准。

### 3.2 按目录分布（改动量的落点）

| 目录 | 该目录里 `<lib>::` 引用总数 |
| --- | ---: |
| `run_method/` | 157 |
| `studio/` | 104 |
| `mcp/` | 102 |
| `build_method/` | 82 |
| `plugin-host/` | 46 |
| `cli/` | 40 |
| `conventions/` | 32 |
| `examples/` | 27 |
| `core/` | 26 |
| `debug_method/` | 9 |
| `macro/` | 6 |

### 3.3 反向视图：每个 crate 被谁引用（决定"哪一批要动哪些文件"）

| 被引用的旧 lib | 合计 | 引用者分布 |
| --- | ---: | --- |
| `xirang::` | 237 | `run_method` 73、`build_method` 49、`mcp` 44、`conventions` 32、`core` 25、`cli` 8、`macro` 6 |
| `xirang_run_method::` | 275 | `studio` 82、`run_method` 79、`plugin-host` 31、`build_method` 29、`examples` 25、`mcp` 20、`debug_method` 8、`core` 1 |
| `xirang_build_method::` | 83 | `mcp` 36、`cli` 25、`studio` 16、`build_method` 4、`examples` 2 |
| `xirang_debug_method::` | 8 | `studio` 5、`mcp` 1、`run_method` 1、`debug_method` 1 |
| `xirang_plugin_host::` | 15 | `plugin-host` 15 |
| `xirang_macro::` | 4 | `run_method` 4 |
| `xirang_studio::` | 2 | `studio` 1、`cli` 1 |
| `xirang_mcp::` | 2 | `mcp` 1、`cli` 1 |
| `xirang_cli::` | 5 | `cli` 5 |

（`run_method` 自己引用 `xirang_run_method::` 79 次、`plugin-host` 引用自己 15 次、`cli` 引用自己 5 次——都是**自引用**，合并后应改为 `crate::…`：这是"改路径"与"顺手去自引用"的区别，建议分两步做，见 §4.4。）

### 3.4 清单与包名

| 项 | 数字 | 命令 |
| --- | ---: | --- |
| 带连字符的旧包名出现（**可编辑文件**，已排除 `docs/audit-*.md` 记录、`CHANGELOG.md`、`Cargo.lock`、`docs/architecture-map.*`） | `xirang-studio` 98 / 32 文件、`xirang-core` 80 / 38、`xirang-build-method` 69 / 33、`xirang-run-method` 66 / 32、`xirang-cli` 48 / 24、`xirang-mcp` 47 / 21、`xirang-debug-method` 32 / 21、`xirang-plugin-host` 28 / 22、`xirang-macro` 22 / 14、`xirang-conventions` 10 / 8 | `grep -rc '<name>' <file>` |
| 需要改动的文件（三个模式的并集） | **280**（其中 `.rs` 214、`Cargo.toml` 13） | 见 §3.1 脚本 |
| 需要改动的 `Cargo.toml` | **13**：根 + `kernel` + `macro` + `toolchain` + `conventions` + 2 examples + 夹具 `studio/tests/fixtures/node-editor`（改名前是 9 个成员清单） | 同上 |
| 旧目录名字面量 | `studio/` 1231、`core/` 408、`run_method/` 315、`build_method/` 240、`mcp/` 234、`plugin-host/` 73、`cli/` 61、`macro/` 36、`debug_method/` 30（含散文提及；**功能性**的那些在 §6 逐条点名） | `grep -rn 'core/src' conventions/src` 等 |

### 3.5 不需要改的东西（同样是结论，能省大量担心）

| 项 | 结论 | 证据 |
| --- | --- | --- |
| `#[path]` 挂载声明 | **0 处需要改** | 339 条声明全部是相对路径（t7 §2.1）；`grep -rn '#\[path' --include='*.rs' . \| grep -E '#\[path\s*=\s*"(\.\.\|/)'` → **0**。合并只把整棵子树搬到 `toolchain/src/<mod>/`，相对解析不变 |
| `#[cfg(feature = …)]` | **0 处语义变化** | 逐 crate 实测：每个 crate 用到的 `cfg(feature)` 都是**自己声明**的（`run_method`:`authoring`×3；`plugin-host`:`wasm`×12、`process-tools`×5；`studio`:`prototype-fixtures`×20、`node-graph`×29；其余 4 个 crate 一处都没用）→ 合并后同一批名字仍然是同一个开关 |
| 测试/示例文件名冲突 | **0** | 7 个 crate 的 `tests/`+`examples/` 共 17 个文件、17 个不同名（`for f in */tests/*.rs */examples/*.rs; do basename $f; done \| sort \| uniq -d` → 空） |
| `mod`/目录挂载约定 | 不变 | `<dir>/<name>.rs` 形态照旧；新根的 7 条挂载按同一约定写 |

### 3.6 批内替换顺序（必须同批改完才编译得过）

改名不是"一个 sed 就完事"，因为 5 个旧 lib 名里有两个会 **collapse 到同一目标**（`xirang_run_method::` 与 `xirang_studio::`/`_mcp::`/`_cli::` 都变 `xirang_toolchain::*`），而 `_build_method::`/`_debug_method::`/`_plugin_host::` 还要**补上模块前缀**。安全顺序（每一步之后全树仍可编译）：

1. **先 `xirang::` → `xirang_kernel::`**（内核改名批；与其他名字互不干扰，因为词边界挡住了 `_`）。
2. **再 `xirang_run_method::` → `xirang_toolchain::`**（唯一"平移到根"的一个）。
3. **然后 `xirang_build_method::`→`xirang_toolchain::build_time::`、`xirang_debug_method::`→`xirang_toolchain::call_evidence::`、`xirang_plugin_host::`→`xirang_toolchain::plugin_host::`、`xirang_studio::`→`xirang_toolchain::studio::`、`xirang_mcp::`→`xirang_toolchain::mcp::`、`xirang_cli::`→`xirang_toolchain::cli::`。**顺序无关**（目标串互不为前缀），但必须与新根的模块名同时落盘。
4. **最后**处理自引用（§3.3 的三个自引用组）与生成侧字符串（`lexicon::RUN_METHOD_CRATE`、脚手架模板、`macro/src/mirror.rs` 的镜像文本）。
5. 清单里的包名（`xirang-run-method = { … }` 等）与 `[[bin]]`/feature 一起改——**清单与代码必须在同一个提交里**，否则 `cargo metadata` 就先失败。

---

## 4. 物理合并的机械步骤（`git mv` 级）

### 4.1 内核改名（批 1）

```text
git mv core kernel
kernel/Cargo.toml    name "xirang-core" → "xirang-kernel"
                     [lib] name "xirang" → "xirang_kernel"
                     readme/documentation 里的 "xirang-core" → "xirang-kernel"
Cargo.toml:2         members: "core" → "kernel"
（内部 339 条 #[path] 与 tests/ 挂载：不动）
```

### 4.2 七个 crate → `toolchain/`（批 2）

每个 crate 的步骤完全同型（以 `run_method` 为例）：

```text
git mv run_method/src            toolchain/src/runtime
git mv toolchain/src/runtime/lib.rs  toolchain/src/runtime/runtime.rs   # 符合 <dir>/<name>.rs 约定；内层 runtime/ 会随之变成 runtime/runtime/（§2.2 的同名嵌套记账）
git mv run_method/tests/*  toolchain/tests/                    # 无同名冲突（§3.5）
git mv run_method/examples/scale_audit.rs toolchain/examples/
git rm run_method/README.md run_method/README.zh-CN.md         # 内容并入 toolchain/README*
git rm run_method/Cargo.toml                                   # 包身份并进 toolchain/Cargo.toml
# 其余 6 个 crate 同理：build_method→src/build_time, debug_method→src/call_evidence,
# plugin-host→src/plugin_host, studio→src/studio（含 studio/studio → src/studio/studio）,
# mcp→src/mcp, cli→src/cli
```

**每个被并入的 `lib.rs` 的三件小事**（§2.2）：`//!` 顶部文档变模块文档；删掉本文件的 `#![warn(missing_docs)]`（只在 `toolchain/src/lib.rs` 留一份）；`pub use xirang::…` → `pub use xirang_kernel::…`。

**每个被并入的 crate 清单里要搬进 `toolchain/Cargo.toml` 的东西**：

| 来源 | 搬到新清单的 |
| --- | --- |
| `build_method` | `[dependencies] xirang-core→xirang-kernel[syntax]`、`proc-macro2`、`serde_json`、`syn`（全部标 optional，挂 `build`） |
| `run_method` | `xirang-macro`（optional，挂 `run`）、`syn`（optional，挂 `authoring`）、`[lints.rust] unexpected_cfgs`（`cfg(rust_analyzer)`） |
| `debug_method` | `petgraph`、`tracing`、`inventory`（optional，挂特性 `evidence`；模块名叫 `call_evidence`） |
| `plugin-host` | `arc-swap`、`ed25519-dalek`（挂 `plugins`）、`wasmi`（挂 `wasm`）、`tempfile`（挂 `process-tools`）、`wat`（dev） |
| `studio` | `crossterm`、`notify`、`ratatui`、`rataflow`（挂 `node-graph`）、`serde_json`、`[package.metadata.docs.rs] all-features`、`default-run` |
| `mcp` | `serde_json`、`[[bin]]`（→ `src/bin/xirang-mcp.rs`） |
| `cli` | `serde_json`、两个 `[[bin]]`、`default-run = "xirang"` |

同时：`autobins = false`、`[package.metadata.docs.rs] all-features = true`、`[lib] name = "xirang_toolchain"`。

### 4.3 五个 bin 的路径

```text
git mv cli/bin/xirang.rs        toolchain/src/bin/xirang.rs
git mv cli/bin/cargo-xirang.rs  toolchain/src/bin/cargo-xirang.rs
git mv studio/main.rs             toolchain/src/bin/xirang-studio.rs
git mv studio/bin/xirang-dev.rs toolchain/src/bin/xirang-dev.rs
git mv mcp/main.rs                toolchain/src/bin/xirang-mcp.rs
```

bin 文件里的调用要改（实测 3 处）：`mcp/main.rs:16 xirang_mcp::run()` → `xirang_toolchain::mcp::run()`；`studio/main.rs:33 xirang_studio::launch_with(project)` → `xirang_toolchain::studio::launch_with(project)`；`cli/bin/{xirang,cargo-xirang}.rs` 的 `xirang_cli::{main,argv_strings,run}` → `xirang_toolchain::cli::{…}`。

### 4.4 与 `git mv` 无关但必须同批的动作

| 动作 | 位置 |
| --- | --- |
| `RUN_METHOD_CRATE` 常量 → `"xirang_toolchain"`（含它的钉住测试） | `core/src/registry_core/lexicon/lexicon.rs:28`（批 1 后 `kernel/…`）、`core/src/registry_core/lexicon/lexicon_tests.rs:15` |
| 宏里的绝对路径：`::xirang_run_method::face_fields!` → `$crate::face_fields!`；`::xirang_debug_method::submit!` → `$crate::call_evidence::submit!`；`::xirang::lexicon::…` → `::xirang_kernel::lexicon::…` | `run_method/macros/face_objects.rs:169`、`run_method/macros/face_registration.rs:166`、`run_method/macros/entry.rs:37-38` |
| 自引用改 `crate::`（79+15+5 处） | `run_method/**`、`plugin-host/src/**`、`cli/**` |
| 脚手架模板：生成两份 `xirang-toolchain`（`[dependencies] features=["run"]` / `[build-dependencies] features=["build"]` —— **feature 名不变**）、`build.rs` 写 `xirang_toolchain::build_time::run()`、生成源码写 `xirang_toolchain::{host!,…}` 与 `xirang_kernel::lexicon::…` | `build_method/scaffold/project.rs:109-131`（`dependency_specs`）、`:154`（prelude）、`:192`（build.rs 行） |
| 清掉可能被复用的构建产物：`examples/*/target/xirang/out/generated_lib.rs` 与 `discovery.fingerprint` 里是旧路径；`build_output_is_current`（`build_method/scope_view.rs:39`）的指纹只覆盖 **`root/src` 的源码发现结果**，**不含渲染器输出**，因此改名后旧产物可能被判"仍是最新" | 交付前 `rm -rf examples/*/target/xirang`（或在批 2 验收前 `cargo clean -p`） |

---

## 5. 门禁、工具、CI、脚手架、文档的重指向

### 5.1 `conventions` 的 12 道门禁（逐条）

| 门禁 | 功能性的名字/路径 | 改动 |
| --- | --- | --- |
| `purity` | `root.join("core").join("src")`（`conventions/src/purity.rs:291`） | → `join("kernel")` |
| `mounting` | `root.join("core/src")`（`conventions/src/mounting.rs:252`，内核挂载树的范围） | → `root.join("kernel/src")` |
| `shims` | `SHIMS` 表里 16 条钉子，**路径**分布在 `run_method/**`、`build_method/{identity,syntax}.rs`（`conventions/src/shims.rs:27` 起）；检测器 `xirang_reexports()` 找的是**字面串** `"pub use xirang::"`（`conventions/src/shims.rs:108`） | 路径 → `toolchain/src/{run,build}/**`；针串 → `"pub use xirang_kernel::"`。语义仍是"宿主书写过的路径不能消失"，只是现在是 `xirang_toolchain::…` 的模块路径 |
| `naming` | 期望包名 `xirang-<dir>`（`conventions/src/naming.rs:89`）+ 库名例外表 `LIB_NAME_EXCEPTIONS = &[("core","xirang")]`（`:39`） | **删掉例外表**：`kernel/`→`xirang-kernel`/`xirang_kernel`、`toolchain/`→`xirang-toolchain`/`xirang_toolchain` 都自动满足规则（这是合并带来的净收益：少一条 carve-out） |
| `size` | 棘轮基线一条（`conventions/src/size.rs:67`）：`("core/src/registry_core/plugin/contracts/contracts.rs", 639)`；`crate_directories` 自动跟随 members | 路径 → `kernel/src/...`；`toolchain/` 合并后要**重测**所有越线文件（`toolchain/src/**` 的测试形态文件变多，但测试按位置/挂载形态豁免，逻辑不变） |
| `features` | `WORKSPACE_ONLY_TARGETS = &[("studio","xirang-dev","dev-supervisor")]`（`conventions/src/features.rs:31`）、`OFF_BY_DEFAULT = &[("studio","prototype-fixtures")]`（`:40`） | 目录名 `studio` →`toolchain`；target 名与 feature 名**不变**（`xirang-dev`、`dev-supervisor`、`prototype-fixtures`） |
| `lint` | `required_roots()`：每个成员的 `src/lib.rs` + 一条特例 `mcp/main.rs`（`conventions/src/lint.rs:57`） | 特例 → `toolchain/src/bin/xirang-mcp.rs`（实测：五个 bin 根里只有它带 `#![warn(missing_docs)]`） |
| `bilingual` | 每个 crate 的 README 对 | 9 对 → 3 对（`kernel`、`macro`、`toolchain`）+ `conventions` 无 README（现状即如此） |
| `release_version` | 从清单里读所有 `xirang-` 前缀的内部要求 | **无需改**（自动跟随新名字；合并后只剩 2 条） |
| `release_workflow` | 只查 `.github/workflows/release.yml` 的上传步骤与 `tools/xirang-publish` | **无需改**（不点 crate 名） |
| `doc_blocks` / `doc_anchors` | 只扫 markdown | 随文档更新自动生效；**记录类文件（`docs/audit-*.md`）不动** |
| 门禁自身的文档串 | `purity.rs`、`mounting.rs`、`doc_blocks.rs`、`doc_anchors.rs`、`lib.rs` 的模块文档里写着 `core/src`、`run_method/src`、`build_method` 等 | 随批 2 一起改（属于 t14 R-27「散文里的名字/数字会漂移」同一类） |

### 5.2 工具

| 工具 | 改动 |
| --- | --- |
| `tools/xirang-publish` | `levels`（`:86` 起，7 层 9 名）→ **3 行**：`xirang-kernel` / `xirang-macro` / `xirang-toolchain`；依赖表（`:323` 起，9 行）→ **3 行**（`xirang-macro:xirang-kernel`、`xirang-toolchain:xirang-kernel xirang-macro`、`xirang-kernel:`）；`--verify-consumers` 与 `--publish` 都从这张表枚举（`:394 for crate in $all_crates`），因此**只改表**；头部 prose（"nine"/顺序说明）一并更新 |
| `tools/xirang-package-audit` | 包集合**自动派生**（`:227-244` 遍历 members、跳过 `publish=false`、由 cargo 解析内部依赖）→ 无需改逻辑；只改头部那段描述层级顺序与"目录不是规则"的注释（它描述的 9 条表已不存在，lane-gates G-21 已记过这笔账） |
| `tools/xirang-external-rehearsal` | 依赖重指向是通用 `sed`（`path = "../../` → `$ROOT/`），对 `../../toolchain` 同样成立；`BIN="$ROOT/target/debug/xirang"` 不变（bin 名保留）；`--note` 里的 `cargo build -p xirang-cli`（`:95`）→ `cargo build -p xirang-toolchain --features cli` |
| `tools/xirang-release-audit` | `-p xirang-run-method --example scale_audit`（`:71`）→ `-p xirang-toolchain --example scale_audit --features run`（示例在 `run` 特性下） |
| `tools/xirang-scale-audit` / `xirang-visual` | 不点发布 crate 名（实测）；`xirang-visual` 产出 `docs/architecture-map.*` → 改名后**重新生成**，不要手改生成物 |
| `deny.toml` | `[licenses.private]` 段落里写着"三个 `publish = false` 成员（`conventions` 与两个 example）"——**不变**；其余不点 crate 名 |

### 5.3 CI 与发布工作流

| 位置 | 改动 |
| --- | --- |
| `.github/workflows/ci.yml:55` | `cargo package -p xirang-core --locked --allow-dirty` → `-p xirang-kernel` |
| `.github/workflows/ci.yml:180` | `cargo test -p xirang-plugin-host --features process-tools` → `-p xirang-toolchain --features plugins,process-tools` |
| `.github/workflows/ci.yml:153` | `-p xirang-example-control-button --test ide_mirror` **不变** |
| `.github/workflows/ci.yml` 其它 job | 全是 `--workspace`，自动覆盖 3 个包；`--all-features` 仍会打开 `prototype-fixtures`（夹具包不在成员里，所以只跑测试不编译它） |
| `.github/workflows/release.yml` | 不点 crate 名（版本来自 `tools/xirang-publish --workspace-version`，tables 来自工具）→ **无需改** |
| `.github/workflows/ci.yml` 的注释 | 若干处点名 `xirang-cli`/`xirang-build-method`/`xirang-core`（如 `:51-53`、`:127-132`）→ 同步改文（属 t14 R-27 同类） |

### 5.4 脚手架模板

生成器必须产生**合并后的宿主形态**（§2.1）：两份同名的 `xirang-toolchain` 分别落在 `[dependencies]`（`features = ["run"]`）与 `[build-dependencies]`（`features = ["build"]`），两者都带 `default-features = false` 与 `version = "{RELEASE_REQUIREMENT}"`（`build_method/scaffold/project.rs:103` 的 `const RELEASE_REQUIREMENT: &str = env!("CARGO_PKG_VERSION");` 自动跟随版本线）；`build.rs` 写 `xirang_toolchain::build_time::run()`；生成源码把 `xirang_run_method::` 全部换成 `xirang_toolchain::`（含 `lexicon` 那条：`xirang_kernel::lexicon::TRACE_FILE_ENV`，或者走 `toolchain` 根重导出的 `lexicon`）。
`DependencySource::{Local,Git}` 两条路径都要改（`:109-131`）；`Local` 的 `workspace.join("run_method")`/`join("build_method")` → `join("toolchain")`。

### 5.5 文档与 README 的去留

| 文件 | 处置 |
| --- | --- |
| `AGENTS.md:15-24` 的 crate 表（10 行） | → **4 行**：`xirang-kernel`（lib `xirang_kernel`）、`xirang-macro`、`xirang-toolchain`、`xirang-conventions`；建议同时补上 t14 R-03 的"为什么必须独立"列。`AGENTS.md` 的"变更规则 1/2/4"、宿主用法段、门禁清单段都要按新名字/新路径重写（规则 4 的 `xirang-<x>` / `<x>/` / `xirang_<x>` 对新三件仍然成立） |
| `README.md:855-901`（crate 表 7 行 + `Workspace layout` 代码块 + "`xirang-core`（library name `xirang`）是内核"段）、`README.zh-CN.md` 对应段 | 表的 9 行 → 3 行；布局块 12 行 → 6 行；内核段的 lib 名改成 `xirang_kernel` |
| 17 个 crate README（实测：`core` 2、`macro` 1、其余 7 个各 2） | **保留 5 个**：`kernel/README{,.zh-CN}.md`、`macro/README.md`、`toolchain/README{,.zh-CN}.md`；**删除 12 个**。内容按职责合并：`toolchain` 的 README 承接 `build_time`/`runtime`/`call_evidence`/`plugin_host`/`studio`/`mcp`/`cli` 七段的要点（**并加两处**：① §1.3 要求的 feature ⇄ 模块 ⇄ 原 crate 名对照表；② §1.1/§5.5 要求的"本仓的 toolchain = 执行面集合，不是 Rust 工具链"一句）（每段保留"这一块为什么这样切"的段落），而不是七份 README 拼贴 |
| `docs/` 的活文档 | 需改：`docs/ROADMAP.md`、`docs/ROADMAP.zh-CN.md`、`docs/roadmap-1.0.md`、`docs/graft.md{,.zh-CN}`、`docs/migration.md{,.zh-CN}`、`docs/threat-model.md{,.zh-CN}`、`docs/discussion-introduction{,.en}.md`、`docs/design-graft-record-and-health-check.md`、`docs/design-trace-ingest.md`、`docs/performance-baseline.md`、`docs/architecture-map.*`（重新生成） |
| **不动** | `docs/audit-*.md`、`docs/audit-3p-*.md`、`docs/audit-2026-09-28/**`、`CHANGELOG.md` 的历史段——**它们是记录**（`conventions` 的 `RECORD_PREFIXES` 正因为此才按文件名豁免）。`CHANGELOG.md` 只**新增**一个 `[0.2.0] — unreleased` 段，写明"9 → 3 合并、旧 9 名 yank、包名与 lib 名迁移" |

---

## 6. 身份复核（合并为什么不动 `NodeId`）

**机制**（`core/src/registry_core/identity/node_id.rs`，批 1 后为 `kernel/…`）：
- `NodeId = sha256(relative_path, declared_name)`；命名空间版本 `NodeId::from_namespaced_path(namespace, relative_path, declared_name)` 把**命名空间**混进哈希域（`:52`）。
- 命名空间的来源是**宿主自己的包名**：`root_node_id(env!("CARGO_PKG_NAME"))`（脚手架生成的宿主源码就是这么写的，`build_method/scaffold/project.rs:167`），或 `XIRANG_NAMESPACE` 覆盖值。**没有任何 xirang crate 的名字进入身份**。

**6 个承载身份的生产文件（一个都不能动）**：

| 文件 | 声明方式 | 引用它 `NODE_ID` 的地方 |
| --- | --- | --- |
| `examples/control-button/src/control/control.rs` | `crate::control_object!` | — |
| `examples/control-button/src/control/object/button/button.rs` | `crate::*_object!` | `examples/control-button/src/lib.rs:50` 的类型化 `static_graft_plan!` |
| `examples/control-button/src/control/object/slider/slider.rs` | `crate::*_object!` | 同上（`:52`） |
| `examples/control-button-graft/src/control_fast.rs` | `external_object!` | `examples/control-button-graft/src/lib.rs` 的 `external_registry()` |
| `examples/control-button-graft/src/button_fast.rs` | `external_object!` | 同上 |
| `examples/control-button-graft/src/slider_fast.rs` | `external_object!` | 同上 |

复算：

```sh
grep -rlnE '[A-Za-z_][A-Za-z0-9_]*_object!' examples/control-button/src examples/control-button-graft/src
# → 6 个生产面文件（+ examples/control-button-graft/src/lib.rs 只出现在文档注释里）
grep -rn 'NODE_ID' examples/control-button/src/lib.rs   # 类型化计划引用的 4 处
```

**本方案对它们的处置：不动。** 合并只改 crate 边界、清单、引用路径、目录名与门禁；`examples/` 的两个宿主**保留目录与文件名**，只改它们的依赖声明与入口路径（`xirang_run_method::` → `xirang_toolchain::`）。因此：
- 宿主源码里的 `file!()` 不变 → `relative_path` 不变 → `NodeId` 不变 → **已落盘的 graft 记录与计划继续解析**；
- 示例宿主的包名不变 → 命名空间不变；
- 生成计划重新渲染后**内容逐字节等价**（只有运行期 crate 名从 `xirang_run_method` 变成 `xirang_toolchain`，那是路径不是身份）。

**反例检查（本次逐条做过，结论：无阻塞）**：
1. **十个发布 crate 的 `src/` 生产注册面声明 = 0**：`*_object!`/`*_face!` 的每处命中都落在四类里——文档注释、夹具字符串（含 `studio/studio/app/tests/*` 里的临时源码文本）、`#[cfg(test)]` 模块、宏定义自身（`run_method/macros/*` 是定义不是声明）。因此**移动任何一个 crate 内部文件都不会改身份**。
2. **命名空间不来自 crate 名**：见上；`RUN_METHOD_CRATE` 只用于**发射**生成代码的路径文本。
3. **`lexicon` 的钉住测试**会跟着改（`lexicon_tests.rs:15` 断言 `RUN_METHOD_CRATE == "xirang_run_method"`），这是**测试**对名字的断言，不是身份数据。
4. **落盘产物的缓存**：`examples/*/target/xirang/{out,cache}` 里是旧路径文本。`build_output_is_current`（`build_method/scope_view.rs:39`）的指纹只覆盖 `root/src` 的**源码发现结果**，不覆盖渲染器输出，所以改名后**必须清掉这些目录**（§4.4 最后一行），否则可能读到旧的 `generated_lib.rs`。这是**运维步骤**，不是身份问题。
5. **一条需要留意的历史事实**：`CHANGELOG.md` 记录过身份变更会以 `[re-identified (daca0f7b… → bc2df33a…)]` 的形式出现。本方案**不触发**它——如果批 1/批 2 之后测试里出现这类输出，说明有人移动了那 6 个文件之一，应当**立即停下**并按阻塞处理。

---

## 7. 分批执行与验收

固定验收命令（每批都跑，全部 `--offline`）：

```sh
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
tools/xirang-publish --check-table
cargo test --workspace --offline --all-features          # 覆盖 conventions 门禁与 prototype-fixtures
tools/xirang-external-rehearsal                        # 两个示例宿主在检出外重建
```

| 批 | 前置 | 动作 | 回滚 | 验收 |
| --- | --- | --- | --- | --- |
| **批 0：冻结与预备**（建议，可跳过） | 无 | ① 冻结 feature 名、bin 名、模块名（本文 §1.3/§1.4/§2.2）；② 决定 §1.4 的 A/B（推荐 A）；③ 若同时做 t14 R-01（`[workspace.dependencies]`），**先做它**——这样 §4.2 的清单改动只发生一次 | 纯文档 | 决策写进 `AGENTS.md`；`--check-table` 绿 |
| **批 1：内核改名** | 批 0 的决定 | `git mv core kernel`；`kernel/Cargo.toml` 的包名与 lib 名；根 `Cargo.toml:2` 的 members；全树 `xirang::` → `xirang_kernel::`（237 处/88 文件）；`conventions` 的 `purity`/`mounting`/`size` 路径与 `LIB_NAME_EXCEPTIONS`；`tools`/CI/脚手架/文档里的 `xirang-core`、`core/`、`core/src` | `git revert`（改名与文本替换，无身份后果） | 五条固定命令全绿；`grep -rn 'lib name.*"xirang"' */Cargo.toml` 为空；`--check-table` 仍报 9 个包（只换了名字） |
| **批 2：物理合并** | 批 1 合入 | §4.2 的七个 `git mv`；§4.3 的五个 bin；`toolchain/Cargo.toml`（feature + 5 个 `[[bin]]` + 依赖 + `autobins=false`）；`toolchain/src/lib.rs` 的 7 条挂载与 `pub use runtime::*`；§3.6 的替换顺序；`lexicon::RUN_METHOD_CRATE`；宏里的绝对路径；脚手架模板；`conventions` 的 `shims`/`features`/`lint`/`bilingual`；`tools` 三处；CI 两处；`AGENTS.md` 与 README（含 12 个 README 删除）；`rm -rf examples/*/target/xirang` | `git revert`（仍是纯改名 + 目录搬运；**未发布前**没有外部后果） | 五条固定命令全绿；`ls */Cargo.toml` 只剩 3 个发布包 + `conventions` + 2 examples；`tools/xirang-publish --check-table` 报 **3 个包**；`tools/xirang-package-audit` 内容那一半对 3 个包全绿；`grep -rn 'xirang_run_method\|xirang_build_method\|xirang_debug_method\|xirang_plugin_host\|xirang_studio::\|xirang_mcp::\|xirang_cli::' --include='*.rs' .` 只剩记录文件 |
| **批 3：发布面切换（不可回滚点）** | 批 2 合入 + 维护者授权 | `Cargo.toml:7` 版本线 → `0.2.0`；跑 `tools/xirang-publish --publish --yes`（按 `kernel → macro → toolchain` 三层）；发布后跑 `--verify-consumers`（3 个新名从 index 解析）与 `tools/xirang-package-audit`（3 个 tarball）；最后 `cargo yank --version 0.1.x <9 个旧名>` | **不可回滚的是每个包名下那一个版本号的内容**（crates.io 版本不可替换）；`yank` 本身可用 `cargo yank --undo` 撤销 | 发布工作流全绿；`--verify-consumers` 绿；旧 9 名 yank 后 `cargo search`/index 状态符合预期 |
| **批 4：收尾** | 批 3 发布后 | 清理 `docs/architecture-map.*` 重新生成、`README` 的安装段与新 `cargo install` 用法、`AGENTS.md` 的门禁清单与 crate 边界表、`CHANGELOG.md` 的 0.2.0 段定稿 | 文档级 | 文档锚点自查（`conventions/src/doc_anchors.rs` 同规则的脚本）0 违规；`cargo test -p xirang-conventions --offline` 绿 |

**批 1 与批 2 之间不允许并行**：批次 2 的全部替换都以"内核已经叫 `xirang_kernel`"为前提；反过来做会让 237 处引用与 275 处引用在同一提交里交错，评审不可读。
**批 2 内部**的可并行部分：7 个 crate 的 `git mv` 与各自 `lib.rs` 的三件小事互不依赖（可以一个 crate 一个提交），但 §3.6 的全局替换与根清单必须最后一次性完成。

---

## 8. 回滚与不可回滚点

| 阶段 | 可回滚性 | 说明 |
| --- | --- | --- |
| 批 0 / 批 1 / 批 2 | **完全可回滚** | 全是改名、目录搬运与文本替换；`git revert` 即恢复。没有任何外部副作用（未发布） |
| 批 3 的**上传** | **不可回滚** | 每个包名下的那一个版本号内容永久固定（只能 yank，不能替换）。因此批 3 之前必须验证：① 五条固定命令全绿；② `--check-table` 报 3 个包；③ `tools/xirang-package-audit` 对 3 个包的内容检查全绿；④ `tools/xirang-external-rehearsal` 在检出外用新依赖重建两个示例宿主；⑤ 冻结了 feature 名与 bin 名（发布后改 feature 名是新的破坏） |
| 旧 9 名 yank | **可撤销**（`cargo yank --undo`） | 但要接受一个事实：**crates.io 上会永久留下这 9 个名字（0.1.x，yanked）**，与 3 个新名字并存。"9 → 3"因此是"旧 9 名冻结在 0.1.x + 新 3 名从 0.2.0 起"，不是"删除 9 个包" |
| 身份 | **不受影响** | §6；唯一需要运维动作的是清掉 `examples/*/target/xirang` 缓存 |

---

## 9. 「不做」清单

| 不做 | 理由 |
| --- | --- |
| 把 `xirang-macro` 并进 `xirang-toolchain` | `proc-macro = true` 的 crate 只能导出宏，物理上不能同时作为普通库；且它依赖内核的 `syntax`（`macro/Cargo.toml:20`），把它并进 toolchain 会让 `toolchain → macro → kernel` 与 `toolchain → kernel` 并存（可以编译，但把"编译期前端"与"运行期框架"混成一个包，违背本次"内核 / 前端 / 工具链"三分） |
| 改动 `conventions` 的 `publish = false` 或它的位置 | 它遍历检出；放进已发布包的 `tests/` 会让解包者失败（`conventions/src/lib.rs` 模块文档已论证） |
| 改动两个 example 的目录布局、包名或那 6 个面文件 | §6：动它们就是改 `NodeId`，需要一次记录迁移且不可回滚 |
| 为合并去重写宏、内核解析器或字段词表 | 合并只做"边界与路径"。`run_method` 宏里那处绝对路径必须改（否则不编译），但改成 `$crate::` 是**同一处最小改动**，不是重写；lane 各片区的逻辑发现（S\*、K\*、BR\*）走各自的修复批 |
| 重命名任何 `pub` 模块路径或公开项（除本次明确的三件套改名） | 公开 API 破坏要与版本推进同批；本次 0.2.0 只承担"发布面合并"这一件事 |
| 把 `docs/audit-*.md` 里的旧 crate 名"修好" | 它们是**记录**（`RECORD_PREFIXES` 的立意）；改了就不是记录 |
| 手工修改 `docs/architecture-map.*` 与 `Cargo.lock` | 两者都是生成物；重新生成即可 |
| 顺手做 t14 的其余 27 项（文件级归属、唯一来源收口、门禁散文数字） | 那是 `audit-boundary-refactor-plan.md` 的批次；本文只在与之重叠处（`[workspace.dependencies]`、门禁路径、README 归并）标注接口 |

---

## 10. 与命名专项（t25 `audit-naming-review.md`）的接口

写本文时 t25 的报告已在同目录；本节**不替它下结论**，只做事实性对照：把它已写下的规则（`NAM-xx` / `D-x`）与本文的名字选择逐条比对，标出**如果那些规则被批准落地，本文哪一处映射要跟着改**。

### 10.1 本文选的名字 vs t25 已写下的规则

| 本文的选择 | t25 的相关规则 | 对照结论 |
| --- | --- | --- |
| 新 crate 目录 `kernel/`、`toolchain/`；`macro/` 不变 | 命名规则（`conventions/src/naming.rs:89` 的 `xirang-<dir>` + `<dir>/` + `xirang_<dir>` 三件套） | ✅ 三件套自动成立。t30 修订后的 `NAM-40` 明确：**挂载改名是常态**（实测 14 处 → `registry_identity`/`registry_syntax`、`*_command`×5、`explain::*`×3、`::tests`×4），`NAM-43` 记录了这四个家族；`toolchain/src/<mod>/<mod>.rs` 属同一约定的常规形态，与新目录名无关。**注意**：第一版 `NAM-40` 写的"394/394 一致、0 违规"已被 t27 证伪并由 t30 撤回，本文任何句子都不以它为据 |
| 模块名 `build_time` / `runtime` / `plugin_host` / `studio` / `mcp` / `cli`（原拟 `build` / `run` / `plugins`） | `D-1`（文件名不得是类别名，黑名单 `support|misc|helpers|…`）；`D-5`（裸动词只许出现在**入口位**——针对函数名）；t34/t35 的 **R-4**（末 token 必须是**角色名词**） | ✅ 三个新名都是角色名词（满足 R-4）；`D-5` 管函数名，`build_time::run`/`mcp::run`/`cli::run` 都落在"crate 入口位"的允许范围内 |
| 模块名 `plugin_host`（原拟 `plugins`） | `D-10`（类型单数、模块复数） | ⚠️ **风格冲突，已裁定**：改名后不再满足"模块复数"，但 `.xirang/plugins` 是同词反义（t35 判 **MAJOR 必换**），语义正确优先于复数习惯。请在命名共识里把 `D-10` 的适用范围收紧为"不与其他既有词构成反义时"，与 `R-4` 一并落地 |
| **模块名 `call_evidence`**（原拟 `evidence`） | **`D-9`（保留词表：`evidence`/`index`/`artifact`/`registry`/`face`/`graft`/`trace` 已被载义，同一词出现第二个含义时必须加限定词）** | ✅ **已裁定并同步**（t30 的 D-9 保留、NAM-05 采用 `build_evidence.rs`）：本文把该模块定名 **`call_evidence`**——它的中心类型就是内核既有的 `CallEvidence`/`EvidenceKind` 与 `UnifiedCallGraph`，所以限定词取"调用证据"；**`build_evidence` 是 mcp 侧那个文件的正确名字，用在这里不对**（`debug_method` 的内容不是构建产物证据）。备选 `observation`（`debug_method/README.md:5` 的自述是 "observation evidence surface"）。**特性名保持 `evidence`**（D-9 管的是文件/模块名，不管特性名）。改动面：§1.2/§1.3 的目录与模块名、§2.2 的碰撞表、§3.6 的替换目标、§4.2 的 `git mv`、`conventions/src/shims.rs` 里 `run_method/runtime/evidence.rs` 那条 shim 的新路径 |
| 7 个模块本体文件 `build/build.rs`…`cli/cli.rs` | `NAM-07` / `D-3`（"无载荷目录收平"：目录内只有 `<dir>.rs` 的应收平成 `<dir>.rs` 并改裸 `mod`） | ✅ **不冲突**：这 7 个新目录各自装着 4–70 个子文件/子目录，正是 `D-3` 要保护的"有载荷"情形。`D-3` 要收平的那 **34 处（发布面 28）**（如 `core/src/registry_core/tree/header/header.rs`、`run_method/{plugin,registry,call_report}/…`）都在被合并的子树**内部**；**但其中 6 处在两个 example 宿主与 Studio 夹具里，且 `examples/control-button/src/control/object/{button,slider}` 这两个目录装着声明注册面的文件**——收平会把 `object/button/button.rs` 搬到 `object/button.rs`，`file!()` 一变 `NodeId` 就变（宿主 `static_graft_plan!` 的类型化引用与落盘 graft 记录随之失效）。因此 `D-3` 落地时必须**把 examples/fixture 的 6 处排除在谓词之外**（我已在 §9 的「不做」清单里把这两个宿主整体划为不动） |
| 并入时保留各 crate 内部的同名文件（`parse.rs`、`validation.rs`、`snapshot.rs`…） | `D-11`（跨 crate 同名词必须同义，否则加限定）；`NAM-03`/`NAM-04`（`run_method/authoring/validation/validation.rs` → `…/context.rs`、`build_method/cache.rs` 拆分） | ✅ 合并**既不制造也不消除**这些冲突：每个旧 crate 的子树整棵搬走，同名文件仍在不同的模块目录下。这些改名发生在被合并的子树内部，**不改变本文的目录级映射**（只改变被搬运文件的名字） |
| `[[bin]]` 名保留（`xirang`/`cargo-xirang`/`xirang-studio`/`xirang-dev`/`xirang-mcp`） | t25 覆盖范围是文件名/目录名/模块名/函数名，**不含 bin 名** | ✅ 无冲突；本文仍建议不改（bin 名是用户界面） |
| `RUN_METHOD_CRATE` 常量名（`core/src/registry_core/lexicon/lexicon.rs:28`） | t25 的 A/B/C 三轴都不含常量名 | 若日后要改名（如 `TOOLCHAIN_CRATE`），影响 1 处定义 + 3 处使用 + 1 个钉住测试 |

### 10.2 两件事共用一个工作树时的顺序（本文的建议）

`D-3`（收平 34 处无载荷目录，发布面 28）与本文的批 2（七合一）**都会大规模 `git mv` 同一批子树**。两者同时做会让 `git status` 里"移动 + 改名"混成一片，评审与 `git log --follow` 都读不出历史。建议：

1. **先 `D-3` 收平，再批 2 合并**（推荐）。理由：收平是"同层内文件移动 + `#[path]` 改裸 `mod`"，纯结构、可一次机械完成；做完之后被合并的子树内部形状稳定，批 2 的 `git mv` 只做一次目录级搬运。
2. 或者**先批 2，再 `D-3`**。代价：收平要在新的 `toolchain/src/<mod>/…` 路径下重做一遍定位（脚本要改前缀），收益是合并先落地。
3. **不要交错**：本文 §3.5 的"0 处 `#[path]` 需要改"这一结论以"子树内部形状不变"为前提；若 `D-3` 在同一批里改了子树内的 `#[path]`，则该结论要按 `D-3` 的清单重算（收平本身会把 28 条 `#[path = "x/x.rs"]` 改成裸 `mod x;`；另有 6 处在 examples/fixture，按上文应排除）。

`NAM-03`（`validation`→`context`）、`NAM-04`（`cache.rs` 拆分）这类**子树内单文件改名**同理：先做改名（一个可读的提交），再做合并搬运（另一个提交），或者在 `git mv` 里一次写成 `git mv run_method/authoring/validation/validation.rs toolchain/src/runtime/authoring/context.rs`（一次移动 + 改名，`git log --follow` 仍能追）。

### 10.3 接口约定

本方案的**批次边界与验收命令与命名无关**：批 1＝内核三件套改名，批 2＝七合一；无论 t25 最终定什么文件名/模块名，**批 2 的物理搬运步骤与 §3.6 的替换顺序都不变，只有"目标名字"这一层会变**。因此建议在**批 0** 里把 t25 的结论（`D-9` 对 `evidence` 的裁定已落为模块名 `call_evidence`；`D-3` 的先后顺序与它的 examples/fixture 豁免见 §10.2）一起冻结，避免批 2 之后再改一次名字——那会让 `shims` 门禁、文档锚点与 README 的工作做两遍。
