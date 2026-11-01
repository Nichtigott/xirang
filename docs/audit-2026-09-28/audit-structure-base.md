# t7 结构基线：目录、模块挂载图、crate 依赖图、清单与特性、体量对账

> 任务 t7（boundary-architect，职责 3「代码边界是否合适」的事实底座）。本轮只出报告，源码一行不改。
> 审计对象是**工作树**：HEAD `cf0c378c68307927589c69d628a2a327ab91b680`；审计开始时 `git status --porcelain | wc -l` = **121**（任务书写 120，实测 121；见 §1.3）。
> 机器可读原始数据在同目录 `docs/audit-2026-09-28/audit-inventory.json`（executor/t1 产出）。本文件每个数字都在 §0 给出口径与命令，可独立复算；与 `audit-inventory.md` 的核对结果见 §1.3。
> 文件名带 `audit-` 前缀是队长 2026-09-28 的裁决：`conventions/src/doc_blocks.rs:69` 的豁免只按**文件**名前缀，`docs/audit-2026-09-28/` 这个目录名不提供豁免（§2.4 记录为结构发现）。

---

## 0. 口径（先定死，后文所有数字按它读）

| 口径项 | 定义 | 复算命令 |
| --- | --- | --- |
| **行数** | 全文行数（含 `///`、空行、`#[cfg(test)]` 模块），与 `wc -l` 同口径（以换行符计） | `find . -name '*.rs' -not -path './target/*' -not -path '*/target/*' -print0 \| xargs -0 cat \| wc -l` |
| **文件集合** | 工作树里每个 `*.rs`，**排除任何名为 `target` 的目录**（含 `examples/control-button/target/xirang/out/generated_lib.rs` 这类构建产物） | 同上 `find` |
| **成员目录** | 根清单 `Cargo.toml:2` 的 `members` 数组（12 项） | `grep -n 'members = ' Cargo.toml` |
| **crate 依赖** | `cargo metadata --no-deps --format-version 1 --offline`，只取 `dependencies` 里名字以 `xirang-` 开头的边 | `cargo metadata --no-deps --format-version 1 --offline > /tmp/meta.json` |
| **模块声明** | 去掉注释与字符串字面量（"屏蔽"）之后，文本里的 `mod <name>;`；`#[path = "…"]` 的**取值**回原文读取（屏蔽会把路径抹白） | 见 §2.1 的脚本口径说明 |
| **`pub` 条目** | 行首（允许缩进）为 `pub fn/struct/enum/trait/union/const/static/type/mod` 的行数 | `grep -rhE '^[ \t]*pub (fn\|struct\|enum\|trait\|union\|const\|static\|type\|mod)\b' <crate>/src --include='*.rs' \| wc -l` |
| **发布面** | `publish` 未设为 `false` 的成员（9 个） | `cargo metadata --no-deps --format-version 1 --offline` 的 `packages[].publish` |

两条与"约 84k 行"的口径差：本工作树的 `.rs` 是 **78,951** 行；`git ls-files '*.rs'` 是 **76,599** 行 / 380 文件（工作树另有 14 个未跟踪 `.rs`）。若把仓库**所有**被跟踪文本文件相加是 98,516 行。84k 与本报告的 78,951 相差约 6%，最可能来自口径（是否含 markdown/toml、或数在别的提交上），**请 t17 报告对账时以本条口径为准**，不要两个数混用。

---

## 1. 目录树与体量

### 1.1 总账

```
工作树 .rs（排除 target/）：394 文件 / 78,951 行
  ├─ 十个 crate 的 src/        73,224 行
  ├─ 十个 crate 的 tests+examples/  3,958 行
  └─ examples/（两个示例宿主）      1,769 行
工作区成员目录内 .rs：387 文件 / 78,727 行
非成员夹具包 studio/tests/fixtures/node-editor/：7 文件 / 224 行（自带 [workspace]，故意不编译）
tools/：6 个可执行脚本，0 行 Rust
```

命令：`find . -name '*.rs' -not -path './target/*' -not -path '*/target/*' | wc -l`

### 1.2 每个目录的文件数 / 行数 / 一句话职责

数字＝该目录（含子目录）下的 `.rs` 文件数与行数，排除 `target/`。命令模板：
`find <dir> -name '*.rs' -not -path '*/target/*' -print0 | xargs -0 cat | wc -l`

| 目录 | 文件 | 行数 | 一句话职责 |
| --- | ---: | ---: | --- |
| `core/` | 93 | 21,787 | 内核：协议名词 + 纯方法（身份、声明、树、解析、策略、渲染），无 I/O、不读 `std::env` |
| `core/src/registry_core/tree/` | 21 | 4,746 | 注册树操作：连接器、事务、graft 覆盖/记录、查询、端口、入口页 |
| `core/src/registry_core/plugin/` | 11 | 3,557 | 插件协议：工件、目录、合同/签名、策略、槽位、信任 |
| `core/src/registry_core/syntax/` | 11 | 2,575 | 注册面语法与入口解析（`syntax` 特性门控，`syn`/`proc-macro2` 可选） |
| `core/src/registry_core/declaration/` | 9 | 2,468 | 注册面/对象词表、合同、运行期校验、调用证据 |
| `core/src/registry_core/mir/` | 9 | 1,855 | MIR 文本/JSONL 解析、增量、合并、模型、渲染 |
| `core/src/registry_core/authoring/` | 10 | 1,836 | 创作面解析/校验/快照（文件系统执行器在 `run_method`） |
| `core/src/registry_core/source/` | 5 | 1,275 | 词法级源码扫描（函数、跨度、调用） |
| `core/src/registry_core/identity/` | 4 | 974 | 稳定 id、SHA-256、路径文本、命名空间 |
| `core/src/registry_core/diagnostic/` | 4 | 940 | `RegistryError`、构建诊断、拓扑检查 |
| `core/src/registry_core/lexicon/` | 2 | 479 | 共享文本契约（生成文件名、运行期 crate 名、环境变量、`.xirang` 路径） |
| `core/src/registry_core/release/` | 1 | 409 | 发布期剪枝与计划词表 |
| `core/src/registry_core/requirements/` | 1 | 255 | 能力声明与要求 |
| `core/src/registry_core/json/` | 1 | 98 | 本工作区所有工件共用的 JSON 字符串编码 |
| `macro/` | 3 | 917 | 编译期注册面字段前端：字段顺序容错、容错分隔符、带跨度的诊断、编辑器镜像 |
| `build_method/` | 48 | 10,110 | 构建期执行面：源码发现、清单解析、缓存、scope、`StaticPlan` 生成、脚手架模板 |
| `build_method/graft_view/` | 7 | 1,398 | 构建期 graft 视图（已声明/匹配/覆盖行/计划行/查询） |
| `build_method/scaffold/` | 3 | 1,148 | `new` 与 `snippets` 的模板渲染（project/install/snippets） |
| `build_method/renderer/` | 4 | 929 | `generated_lib` 渲染：别名、IDE 视图、通道、树 |
| `run_method/` | 61 | 9,584 | 运行期执行面：注册机实例、`host!`/`trace_call!` 宏、trace 绑定、authoring 执行器 |
| `run_method/authoring/` | 23 | 3,597 | authoring 执行器（`authoring` 特性门控）：文件系统、清单、操作、校验、快照 |
| `run_method/runtime/` | 17 | 3,216 | 运行期状态与 trace：帧栈、数据边、工件 IO/解析、局部值 |
| `run_method/macros/` | 8 | 1,164 | 声明式宏：`host!`、注册面对象、trace、字段辅助 |
| `debug_method/` | 5 | 481 | 观测证据面：MIR 合并、`UnifiedCallGraph`、tracing/petgraph 适配、inventory 收集器 |
| `plugin-host/` | 15 | 3,888 | 插件宿主执行面：Wasm/进程沙箱、代际部署、惰性激活槽表、准入 |
| `plugin-host/tests/` | 4 | 1,901 | 端到端准入、故障矩阵、Wasm 表开销、进程加载开销（占总行数 49%） |
| `studio/` | 79 | 12,532 | Ratatui 创作/检视 TUI：app 状态机、ui 渲染、键盘/鼠标、图与搜索 |
| `studio/studio/app/` | 46 | 8,718 | Studio 应用层（含其 `tests/`）：状态、导航、变更、键盘、热区、查询、测试 |
| `studio/studio/app/tests/` | 11 | 3,373 | Studio 应用层测试（`#[cfg(test)]` 挂载），占 `app/` 的 39% |
| `studio/studio/ui/` | 20 | 2,709 | Studio 渲染层：面板、表单、图、搜索、状态栏、遮罩 |
| `studio/tests/fixtures/` | 7 | 224 | **非工作区成员**的仅源码夹具宿主包（`[workspace]` 自成一界，从不编译） |
| `mcp/` | 39 | 8,682 | AI 代理 stdio 桥：JSON-RPC 循环、工具分发、路径守卫、预览式写入 |
| `cli/` | 14 | 2,802 | 进程粘合：argv 分发、cargo 子进程、子命令转发 |
| `conventions/` | 20 | 6,399 | 仓库约定门禁（`publish = false`）：内核纯净性、模块挂载、尺寸棘轮、文档块/锚点、特性、命名 |
| `examples/control-button/` | 13 | 1,635 | 示例宿主：README 的 Control/Button 树 + `host!()` 生成的注册计划 |
| `examples/control-button-graft/` | 4 | 134 | 项目外实现：用 `external_object!` 显式声明来源、在自己的命名空间建注册机 |
| `tools/` | 0 | 0 | 6 个 shell 工具（publish / package-audit / external-rehearsal / release-audit / scale-audit / visual） |

### 1.3 与 executor `audit-inventory.md` 的交叉核对

逐项对账（我的独立测量 vs `docs/audit-2026-09-28/audit-inventory.md`）：

| 项 | 我的值 | audit-inventory.md | 判定 |
| --- | --- | --- | --- |
| 工作树 `.rs` 文件总数 | 394 | 394 | ✅ 一致 |
| 工作树 `.rs` 行数（排除 `target/`） | 78,951 | 78,951 | ✅ 一致 |
| `mod` 声明数 / 挂载边 | 339 | 339 | ✅ 一致 |
| `core` | 93 / 21,787 | 93 / 21,787 | ✅ 一致 |
| `macro` | 3 / 917 | 3 / 917 | ✅ 一致 |
| `run_method` | 61 / 9,584 | 61 / 9,584 | ✅ 一致 |
| `build_method` | 48 / 10,110 | 48 / 10,110 | ✅ 一致 |
| `debug_method` | 5 / 481 | 5 / 481 | ✅ 一致 |
| `plugin-host` | 15 / 3,888 | 15 / 3,888 | ✅ 一致 |
| `mcp` | 39 / 8,682 | 39 / 8,682 | ✅ 一致 |
| `cli` | 14 / 2,802 | 14 / 2,802 | ✅ 一致 |
| `conventions` | 20 / 6,399 | 20 / 6,399 | ✅ 一致 |
| `examples/control-button` | 13 / 1,635 | 13 / 1,635 | ✅ 一致 |
| `examples/control-button-graft` | 4 / 134 | 4 / 134 | ✅ 一致 |
| `studio` | **79 / 12,532** | **72 / 12,308** | ⚠️ **口径差**，不是分歧 |
| 600 行以上文件数 | 5 | 5 | ✅ 一致 |
| 未提交改动数 | 121 | 121 | ⚠️ 与任务书的 120 不符（两边实测都是 121） |

**`studio` 的差是可解释的**：`audit-inventory.md:70` 把非成员夹具包 `studio/tests/fixtures/node-editor/`（7 文件 / 224 行）单列一行；我把它算在 `studio/` 里。72 + 7 = 79，12,308 + 224 = 12,532，**两个口径都自洽**。建议 t17 对账时统一采用"成员目录内全部 `.rs`，嵌套非成员包单列"的 executor 口径。

**结论：两份清点在文件/行数层面零冲突**，唯一差异是嵌套非成员包的归属口径。

---

## 2. 模块挂载图

### 2.1 数字

| 项 | 数字 |
| --- | ---: |
| 真实模块声明（目标文件存在） | **339** |
| 其中 `#[path = "…"]` 形式 | **306** |
| 其中裸 `mod x;` | **33** |
| 声明指向不存在的文件（dangling） | **0** |
| 同一文件被两条声明指名（duplicated） | **0** |
| `mod.rs` | **0** |
| 语句位置的 `include!` | **1** |
| 文本形式出现 `include!` 的总处数（含文档注释/字符串） | 7 |
| 从 `lib.rs`/`main.rs`/`build.rs` 出发可达的模块文件 | 全部（除 3 个 `src/bin/*.rs`，它们是 cargo 自动发现的二进制源根） |
| 孤儿模块文件 | **0** |

命令（口径：屏蔽注释与字符串后取 `mod <name>;`，`#[path]` 取值回原文读；用 `path.parent()` 作为解析基准，裸 `mod x;` 先试 `<dir>/x/x.rs` 再试 `<dir>/x.rs`——这正是 `conventions/src/mounting.rs` 的 `target_of`）：

```sh
# 复算按 §0 的脚本口径；关键断言用现成的门禁命令即可
cargo test -p xirang-conventions --offline   # mounting 门禁断言 core 无未挂载/dangling/重复
find . -name 'mod.rs' -not -path '*/target/*' | wc -l
```

### 2.2 形态：父目录收纳子模块（主流）与 crate 根平铺（两个 crate）并存

工作区的挂载约定是 **每个模块文件都由父文件显式声明**——`#[path = "<dir>/<name>.rs"] pub mod <name>;` 或（父文件是 crate 根／本身经 `#[path]` 载入时）裸 `mod <name>;`。模块**本体**要么平铺在父模块的目录里（`<dir>/<name>.rs`），要么落在以自己命名的目录里（`<name>/<name>.rs`）；后者是它还要收纳子模块时该在的位置。现状是**两种风格并存**：

| crate | `src/` 下 `.rs` 总数 | 直属 `src/` | 在子目录里 | 形态 |
| --- | ---: | ---: | ---: | --- |
| `core` | 91 | 2 | 89 | 子目录收纳，最深 4 层 |
| `run_method` | 52 | 1 | 51 | 子目录收纳，最深 3 层 |
| `studio` | 71 | 2 | 69 | 子目录收纳，最深 3 层 |
| `build_method` | 46 | 32 | 14 | 混合：主层平铺，三个成组子目录 |
| `plugin-host` | 11 | 9 | 2 | 混合：主层平铺，两个模块带自己的目录 |
| `cli` | 14 | 7 | 7 | 混合：`commands/` 平铺 + `explain_*` |
| `mcp` | 39 | 39 | 0 | **完全平铺** |
| `conventions` | 20 | 20 | 0 | **完全平铺** |
| `macro` | 3 | 3 | 0 | 完全平铺 |
| `debug_method` | 4 | 4 | 0 | 完全平铺 |

命令：`find <crate>/src -name '*.rs' | awk -F/ '{print NF}' | sort | uniq -c`（或 §2.1 脚本的目录聚合）。

最深的一处是 `core/src/registry_core/plugin/contracts/signing/signing.rs`（模块路径 `xirang::plugin::contracts::signing`，4 层目录）。

**精确形状（339 条声明的分解）**：**58 条**把模块本体挂在 `<name>/<name>.rs`（即它拥有自己的目录），其余 **281 条**以 `<dir>/<name>.rs` 平铺在父模块的目录里。"自己的目录"不是风格偏好，而是**有子模块时的必要条件**：子模块用裸 `mod x;` 时按 `<父文件所在目录>/x/x.rs` 解析（`conventions/src/mounting.rs` 的 `target_of`），所以父模块的本体必须落在以自己命名的目录里，子模块的目录才会落在它旁边。

**结构观察（不是缺陷，但值得 t14 决定要不要统一）**：这 58 个"有自己目录"的模块里，**28 个的目录里除了模块本体没有任何子模块文件**——目录此刻只为装那一个文件而存在（一旦它长出子模块，这个目录就是必须的）。其中最小的是 `run_method/plugin/plugin.rs`（7 行）与 `run_method/registry/registry.rs`（7 行）：

```text
core/src/registry_core/{json,release,requirements},
core/src/registry_core/authoring/{snapshot,validation},
core/src/registry_core/plugin/{catalog,plugin_policy,slot,trust}, plugin/contracts/signing,
core/src/registry_core/tree/{connector,entry_pages,header,index,inspection,metadata,query,transaction},
run_method/{authoring/filesystem,authoring/parse,authoring/snapshot,authoring/validation,
                authoring/manifest/parse,call_report,plugin,registry,runtime/trace/edges,runtime/trace/frames}
```

这 28 个目录是"统一形状"的代价：`plugin/plugin.rs` 只有 7 行（一条 `pub use` 式 shim），但仍占一个目录、一条 `#[path]` 声明、一次 nav 跳转。反过来，`mcp/`（39 文件 / 8,682 行，全平铺）与 `conventions/`（20 / 6,399，全平铺）证明平铺在本仓库的规模下完全可用。**两种风格并存本身没有失败模式**（门禁只管"没有 `mod.rs`、没有第二个 `include!`"），所以这是风格/阅读顺序问题，归 ① 与 ②，不归结构红线。

### 2.3 孤儿 / `mod.rs` / `include!`

- **孤儿模块：0**。`conventions/src/mounting.rs` 的 `mounts()` 专门盯内核"没有任何声明指名的模块文件"，今天为绿；我把同一算法套到全部 12 个成员目录，也只有一个预期内的类别例外——`src/bin/*.rs`（`cli/bin/xirang.rs`、`cli/bin/cargo-xirang.rs`、`studio/bin/xirang-dev.rs`）由 cargo 作为二进制源根自动发现，不在 `mod` 图里。
- **`mod.rs`：0**（`find` 计数）。门禁覆盖。（`core/src/registry_core/diagnostic/build.rs` 这类**叫** `build.rs` 的文件是普通模块，不是 cargo 构建脚本；crate 根的 `build.rs` 才是——`mounting.rs` 的注释专门记了这个坑。）
- **`include!`：语句位置 1 处**，即 `run_method/macros/entry.rs:55` 的 `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`（`host!()` 展开物），与 AGENTS.md 的"唯一一处"一致。另外 6 处文本命中都在**文档注释或字符串字面量**里（`run_method/macros/entry.rs` 的 `///` 说明、`build_method/renderer/pass.rs` 的断言字符串、`conventions/src/lib.rs` 的夹具字符串、`conventions/src/mounting.rs` 的模块文档），门禁屏蔽后不计入。

### 2.4 已确认的结构债务与数字漂移

1. **`conventions/src/mounting.rs:10` 的"29 处裸声明"已经过期**。按 §2.1 口径实测是 **33**（`#[path]` 306 + 裸 33 = 339）。分布：

   | 父文件 | 裸 `mod` 数 | 位置是否合法 |
   | --- | ---: | --- |
   | `studio/studio/app/app.rs` | 16 | ✅ 该文件经 `#[path]` 载入 |
   | `plugin-host/src/lib.rs` | 7 | ✅ crate 根 |
   | `debug_method/src/lib.rs` | 3 | ✅ crate 根 |
   | `examples/control-button-graft/src/lib.rs` | 3 | ✅ crate 根 |
   | `studio/studio/ui/ui.rs` | 3 | ✅ 经 `#[path]` 载入 |
   | `studio/studio/studio.rs` | 1 | ✅ 经 `#[path]` 载入 |

   33 处**全部合法**（父文件要么是 crate 根，要么本身经 `#[path]` 载入），门禁的合法位置判定为 0 例外。但 `mounting.rs` 把数字写死在散文里，而 `AGENTS.md` 已经学乖删掉了数字——**同一门禁的两份散文一个记数一个不记数，记数的那份先烂**。建议（t14 落地项）：把该句改成不含数字的表述，或在门禁里断言该计数（`AGENTS.md` 已选择后者不做的理由）。

2. **豁免规则按"文件**名**前缀"，不按路径**。`conventions/src/doc_blocks.rs:69` 的 `RECORD_PREFIXES = &["audit", "design"]` 由 `is_record`（`conventions/src/doc_blocks.rs:133`）实现，`markdown_files`（`conventions/src/doc_blocks.rs:73`）用它 `retain`。于是：

   - `docs/audit-3p-2026-09-27.md` ✅ 豁免（文件名以 `audit` 开头）；
   - `docs/audit-2026-09-28/lane-kernel.md` ❌ **不豁免**（目录名带 `audit` 不算）。

   当时的实测（HEAD + 三份片区报告，文件名尚未加前缀）：`cargo test -p xirang-conventions --offline` → `97 passed; 2 failed`，两个失败分别来自 `doc_blocks` 的 Rust 围栏解析与 `doc_anchors` 的锚点解析，共命中 3 个文件 13 处锚点 + 3 处围栏。按队长裁决把报告改名为 `audit-*.md` 之后，同一条命令复绿：`99 passed; 0 failed`（本文件也在其中，零 finding）。这是**门禁边界**的真实形状：把报告放进以 `audit` 命名的目录，是一个看起来很自然、但并不豁免的做法。队长已裁决：报告文件名统一加 `audit-` 前缀（本文件即按此命名），**这不是一条待修缺陷，是一条结构约定**。

---

## 3. crate 依赖图

### 3.1 边与层

来源：`cargo metadata --no-deps --format-version 1 --offline`（只列 `xirang-*` 边；`normal` / `build` / `dev` 已标注）。

```text
core                     -> (无内部依赖)
macro                    -> core            [core 开 syntax]
build_method             -> core            [core 开 syntax]
run_method               -> core, macro
debug_method             -> run_method
plugin-host              -> run_method
studio                   -> run_method[authoring], debug_method, build_method
mcp                      -> build_method, run_method[authoring], debug_method, core
cli                      -> build_method, mcp, studio, core
conventions              -> core [syntax]（path-only，无 version）
examples/control-button  -> run_method[authoring], control-button-graft ; build-dep: build_method ; dev-dep: build_method
examples/control-button-graft -> run_method
```

层级（`tools/xirang-publish:86` 的 `levels` 与之一致）：

```text
core → {macro, build_method} → run_method → {debug_method, plugin-host} → {studio, mcp} → cli
conventions 与两个 examples 不在发布链上
```

**环：0**（对 10 个发布 crate + conventions 做 DFS 三色标定，无回边）。

### 3.2 传递闭包＝"一次安装要编译多少 crate"

| 入口（被发布/被安装者） | 闭包大小 | 闭包含 |
| --- | ---: | --- |
| `cargo install xirang-cli` | **8** | cli, build_method, core, mcp, debug_method, macro, run_method, studio |
| `cargo install xirang-studio` | 6 | studio, build_method, core, debug_method, macro, run_method |
| `cargo install xirang-mcp` | 6 | mcp, build_method, core, debug_method, macro, run_method |
| 宿主最小集（`[dependencies] run_method` + `[build-dependencies] build_method`） | 4 | core, macro, run_method, build_method |
| 只 `cargo add xirang-plugin-host` | 4 | core, macro, run_method, plugin-host |

这是后文 §6 的关键数字：**9 个发布名里，一个 `cargo install xirang-cli` 就会编译 8 个**；只有 `plugin-host` 在闭包之外。

### 3.3 `examples/` 与 `conventions/` 的位置：合理，且各有硬理由

- **`examples/*` 是工作区成员**：这样才能被 `cargo test --workspace` 覆盖、共享 `Cargo.lock` 与 `target/`、并被 `tools/xirang-external-rehearsal` 在检出外重指向。两者都是 `publish = false`，无版本要求，因而 `deny.toml` 的 `allow-wildcard-paths = true` 正是为它们开的。
- **`studio/tests/fixtures/node-editor/` 不是成员**：它自带 `[workspace]` 表，`description` 明确"仅源码、从不编译"。这不是漏列，是必须——一旦成为成员，它会进入所有门禁的遍历范围。
- **`conventions/` 是成员但不是发布面**：它遍历整个检出，若放在某个已发布 crate 的 `tests/` 里，`.crate` 会带上 `tests/`，解包者会跑一个找不到兄弟目录的门禁。`publish = false` 是让这个归属诚实的手段（`conventions/src/lib.rs` 的模块文档已论证）。位置正确。

### 3.4 跨 crate 的隐性耦合：一处"幻影依赖"

`run_method/macros/face_registration.rs:166` 的宏体展开为 `::xirang_debug_method::submit! { … }`，但 `run_method` **不依赖** `debug_method`（§3.1）。也就是说 `run_method` 的公开宏可以发射一个指向"调用方必须自己另外引入的 crate"的绝对路径：

- 触发条件：宿主显式写 `collector: debug`（`debug_method/README.md:12` 有记录，`debug_method/tests/collector_integration.rs:7` 是唯一树内用例）；
- 后果：宿主必须**手工**往自己的 `Cargo.toml` 加 `xirang-debug-method`，否则报"找不到 crate"，且没有任何清单或门禁提示它；
- 现状：默认模式是 `development`（`run_method/macros/face_objects.rs:13`）与 `linked`（`face_external.rs:51`），都不发射该路径，所以仓库自己不会踩到。

这是**真实存在的边界倒挂**（一个 crate 的宏知道另一个 crate 的路径），也是 §6 里 `debug_method` 合并案最硬的技术理由：合并后 `$crate::` 就能表达同一件事，幻影依赖自然消失。

---

## 4. 清单与包管理

### 4.1 workspace 成员与共享版本线

- 成员 12 个：`core`, `macro`, `run_method`, `build_method`, `cli`, `debug_method`, `studio`, `plugin-host`, `mcp`, `examples/control-button`, `examples/control-button-graft`, `conventions`（`Cargo.toml:2`）。
- 共享版本线：`Cargo.toml:7` 的 `[workspace.package] version = "0.1.6"`，10 个成员用 `version.workspace = true` 继承；两个示例宿主是独立的 `0.1.0`（不发布，无所谓）。
- 内部要求（`version = "0.1.6"`，caret）共 **20 处**，分布在 **11 个** `Cargo.toml`（root 1 处 + 10 个成员清单 19 处）：

  ```sh
  grep -rn 'version = "0.1.6"' --include='Cargo.toml' . | grep -v '^./target/' | wc -l   # -> 20
  ```

- **版本线是硬约束，不是习惯**：`tools/xirang-publish` 的 `metadata_requirement_problems()`（`tools/xirang-publish:462`）要求**每一条**内部要求都等于 `^<workspace version>`，否则 `--check-table` 失败，而 `--check-table` 在 CI（`.github/workflows/ci.yml:134`）与发布工作流里都是必过步骤。当前实测：

  ```sh
  tools/xirang-publish --check-table   # -> dependency table matches the manifests (9 crates)   exit 0
  tools/xirang-publish --workspace-version   # -> 0.1.6
  ```

  **推论（§6.2 会再量化一次）**：只要工作区版本动一位，20 处内部要求必须同步动，也就是**整条线一起进版本、9 个 crate 一起重发**。`docs/roadmap-1.0.md:74` 记下的"保留独立发版、只有真正用到新行为时才抬下界"在当前实现里**不成立**——工具不允许下界低于当前版本线。这是"已记录决策"与"已实施门禁"之间的一处真实矛盾，建议 t14 单列：要么改文档，要么放宽工具（允许 `>=0.1.0,<0.2` 这类"只有在用到新符号时才抬下界"的写法）。

### 4.2 feature 门控与 `required-features`

| 成员 | 特性 | 默认 | 门控对象 | 类型 |
| --- | --- | --- | --- | --- |
| `core` | `syntax`（`core/Cargo.toml:17`） | 否 | 注册面解析器 + 应用/graft 入口发现（公开 API） | 公开 API 门控 |
| `run_method` | `authoring`（`run_method/Cargo.toml:16`） | 否 | authoring 执行器（公开 API） | 公开 API 门控 |
| `studio` | `node-graph`（`studio/Cargo.toml:26`） | **是** | `rataflow` 节点图控件（调用树绘制） | 公开 API 门控（默认开） |
| `studio` | `prototype-fixtures`（`studio/Cargo.toml:35`） | 否 | 仅测试用的检出夹具（非发布面） | 测试夹具 |
| `studio` | `dev-supervisor`（`studio/Cargo.toml:45`） | 否 | `xirang-dev` 重建监督器 bin | 仅工作区 target |
| `plugin-host` | `wasm`（`plugin-host/Cargo.toml:22`） | **是** | `wasmi` 沙箱后端 | 公开 API 门控（默认开） |
| `plugin-host` | `process-tools`（`plugin-host/Cargo.toml:23`） | 否 | 进程后端（`tempfile`） | 公开 API 门控 |

**`required-features` 全仓只有 1 个 target**：`studio/Cargo.toml:99` 的 `xirang-dev` → `["dev-supervisor"]`。这符合"按 target 的 `publish = false` 等价物"的立意，且 `conventions/src/features.rs` 已把它变成门禁。

**向下游的连带**：谁开了什么（`cargo metadata` 的依赖边可见）——

- `core/syntax` 被 `macro`、`build_method`、`conventions` **直接**开启。关键是前者：`run_method` **强制依赖 `macro`**，因此**任何链接 `run_method` 的宿主都绕不开 `core/syntax`**，也就绕不开 `syn` + `proc-macro2`——`syntax` 特性省下的只有"只用 `core` 本身"的消费者。工作区内部则更进一步：`studio`/`mcp`/示例宿主都开 `run_method/authoring`，特性合并后 `run_method` 总是带 `authoring`。
- `plugin-host` 的 `wasm` 默认开：`cargo add xirang-plugin-host` 会拉 `wasmi`；只有显式 `default-features = false` 才能避开。
- **特性名分散在 4 个 crate**（`syntax`/`authoring`/`prototype-fixtures`+`dev-supervisor`/`wasm`+`process-tools`），AGENTS.md 明确"不要为对齐而改名，它们是公开 API"。这是分 crate 的**直接代价**：读者要记住 4 套词汇、7 个特性名、其中 2 个默认开，且没有任何一处集中描述。

### 4.3 publish 与发布面

- `publish = false` 的成员 **3 个**：`conventions`（`conventions/Cargo.toml:10`）、`examples/control-button`（`examples/control-button/Cargo.toml:5`）、`examples/control-button-graft`（`examples/control-button-graft/Cargo.toml:5`）。与 `AGENTS.md` 一致。
- 发布面 **9 个**；**0.1.0–0.1.5 已经全部上过 crates.io**（`CHANGELOG.md:14`、`docs/roadmap-1.0.md:21`）。这一点决定了 §6 的"可减空间"：包名一旦发布就不能删除，只能 yank。
- 发布顺序由 `tools/xirang-publish` 的两张手维护表表达：层表（`tools/xirang-publish:86` 起，7 层 9 个名字）与依赖表（`tools/xirang-publish:323` 起，9 行）。

### 4.4 依赖声明位置与共享情况

根清单没有 `[workspace.dependencies]` 段（`Cargo.toml` 全文 14 行）。被多个 crate 重复声明的外部依赖：

| 依赖 | 声明它的 crate | 版本/特性 |
| --- | --- | --- |
| `serde_json` | `build_method`, `cli`, `mcp`, `studio`（4 处） | 全部 `"1"` |
| `syn` | `core`(optional), `run_method`(optional), `build_method`, `macro`(dev), `conventions`（5 处） | `full,parsing,visit` ×3；`full,parsing` ×2 |
| `proc-macro2` | `core`(optional), `macro`, `build_method`（3 处） | `span-locations` ×2；无特性 ×1 |
| `tempfile` | `plugin-host`(optional, `3.22`), `conventions`(dev, `3`)（2 处） | 下限不同 |
| `xirang-*` 内部要求 | 10 个成员清单，19 处 | 全部 `0.1.6` |

**该共享的没有共享**：`serde_json`/`syn`/`proc-macro2` 的重复声明共 12 处，可以用 `[workspace.dependencies]` 收敛到 1 处一份。**而且工具已经准备好了**：`tools/xirang-publish:523` 会检查 `[workspace.dependencies]` 里的内部版本，`tools/xirang-publish:602` 已经能读 `xirang-x.workspace = true` 这种继承拼法（注释明确写了"经 `[workspace.dependencies]` 读出它，才不会让表显得不对并诱使人删掉它"）。也就是说，**从 20 处内部版本要求降到 1 处（`[workspace.dependencies]` 里一份）不需要改任何工具**。

### 4.5 逐条判定（薄/厚/混职能/该共享未共享）

| 判定对象 | 事实 | 判定 |
| --- | --- | --- |
| `debug_method` 太薄 | 4 文件 / 444 src 行 / 10 pub fn；消费者只有 `studio`、`mcp`；无人 `cargo install` | ⚠️ **唯一的合并候选**（§5.1） |
| `macro` 太薄 | 3 文件 / 917 行 / 4 个 `#[proc_macro]` 入口 | ✅ **不该并**：proc-macro crate 物理上不能并入普通 lib |
| `cli` 太薄？ | 2,802 行 / 4 个 pub fn / 2 个 bin | ✅ 不该并：它是 umbrella，闭包 8 个 crate（§3.2） |
| `mcp` 公开面极小 | 8,682 行但只有 1 个 `pub use protocol::run`（`mcp/lib.rs:58`）；库只为 bin 与测试存在 | ✅ 不该并：它有自己的安装面与协议边界 |
| `core` 太厚？ | 21,787 行 / 93 文件 / 12 个顶层模块 | ✅ 不该拆：所有 crate 依赖它，拆出子 crate 只会造出 core→core 的第二条线；`syntax` 已经是特性门控（等效隔离） |
| `build_method` 太厚？ | 10,110 行 / 46 文件，其中 `scaffold` 1,148 行是模板渲染、`graft_view` 1,398 行是视图 | ✅ 暂不该拆：三块都是"给构建步骤准备输入/输出"的同一职能；`scaffold` 是唯一像是在做第二件事的（模板 → 用户项目），但它被 `cli new/snippets` 使用且与 `build_method` 共享 `Cargo.toml` 模板常量，拆出会复制一份模板知识 |
| `studio` 太厚？ | 12,532 行 / 79 文件，`app` + `ui` + `tests` | ✅ 不该拆：TUI 是一个状态机 + 它的渲染，拆开会让 `ui` 反向依赖 `app` 的私有状态 |
| 该共享未共享 | 见 §4.4，12 处可收敛 | ⚠️ **建议改**（零风险，不改公开面） |
| 有没有该拆的 crate | 无 | 见 §5.3 |

---

## 5. 体量与职能对账

**结论先行：用职能看，需要动的只有 1 处（`debug_method` 并入 `run_method`），没有需要拆的 crate。**

### 5.1 该合的：`debug_method` → `run_method`（非默认 `evidence` 特性）

**它今天是"数据模型在 A、这一份证据视图在 B"**：`CallTrace`/`LocalKind`/`SourceLocation`/`EvidenceKind` 这些类型**已经**在 `run_method`（并经它从 `core` 再导出），`debug_method/src/lib.rs` 做的只是再导出 + 3 个模块：`adapters.rs`（214 行，tracing/petgraph）、`collector.rs`（34 行，inventory `submit!`）、`mir.rs`（155 行，MIR 解析合并 + `UnifiedCallGraph`）。全 crate 444 行。

**为什么这样切更内聚**：
1. 它给 `run_method` 的数据模型提供的正是"图形状的视图"——把视图放在模型旁边，读者不必跨 crate 才能看懂 `UnifiedCallGraph::new(&MirGraph, &CallTrace)` 在消费谁。
2. 它**顺手消灭一处真实倒挂**（§3.4）：`::xirang_debug_method::submit!` 变成 `$crate::…`，"宿主要手工加一个清单里没写的依赖"这条隐形要求消失。
3. 消除 1 份清单、2 份 README（`debug_method/README.md` + `README.zh-CN.md`）、1 个 docs.rs 页、1 条发布表层条目、3 条内部版本要求、1 个 `#![warn(missing_docs)]` 面。

**反对合并的理由（我也列出来，这是判定要付的价）**：
- `run_method` 是**每个宿主都会链接**的 crate。合并后它多 3 个依赖（`petgraph`、`tracing`、`inventory`），必须放在非默认 `evidence` 特性后面，否则每个宿主的依赖树都被撑大。
- 合并后想用 `collector: debug` 的宿主，要开的是 `xirang-run-method/evidence` 而不是加一个 crate——**这一条其实更好**，但它是**行为变更**，要在 CHANGELOG 里写清。
- 30 处（26 个文件）树外引用要点名改（§6.2 表），含两份 README、AGENTS.md、migration 文档。

### 5.2 不该合的（用职能论证，不是口味）

| crate | 硬理由 |
| --- | --- |
| `macro` | `[lib] proc-macro = true` 的 crate **不能**作为普通库被同一个 crate 同时用；且 `core` 不能反过来依赖它（成环）。物理不可合并。 |
| `build_method` ↔ `run_method` | 一个是宿主的 **build-dependency**（为 host triple 编译），一个是**普通依赖**（为 target triple 编译）。合并即等于让宿主的 `build.rs` 那一遍把运行期注册机、trace、authoring 全编一遍，也会破坏交叉编译时的 triple 语义。 |
| `plugin-host` | 唯一在 `cli` 闭包**之外**的发布 crate（§3.2），把 `wasmi` 挡在其余 8 个 crate 的安装树之外。它是发布图里的**叶子**：树内无人依赖它（只有 `conventions/src/naming.rs` 的一句注释提到名字），消费者是树外宿主。合并会把它 1,987 行 + `wasmi` + `ed25519-dalek` 塞进某个必然被打包的工具。 |
| `conventions` | 见 §3.3：遍历检出，且 `publish = false`。 |
| 两个 examples | 见 §3.3：必须是成员（CI/锁文件/排练工具），必须 `publish = false`。 |
| `studio` / `mcp` / `cli` | 三个各自独立的**安装面**（三个 bin、三种用途）；`cli` 是伞，且已有的依赖方向（cli → mcp/studio）是正确方向。 |
| `core` | 内核；12 个成员里 **6 个直接依赖**它（`macro`、`build_method`、`run_method`、`mcp`、`cli`、`conventions`），其余经 `run_method`/`build_method` 传递依赖。 |

### 5.3 该拆的：无

按"混了两种职能"逐 crate 检查（§1.2 的职责列 + §5.2），唯一"像在做第二件事"的是 `build_method/scaffold/`（1,148 行，给用户项目生成模板），但它：① 与 `build_method` 共享 `Cargo.toml` 模板常量（`RELEASE_REQUIREMENT`，`build_method/scaffold/project.rs:103`）；② 只被 `cli new/snippets` 使用；③ 拆出后 `cli` 要多一条依赖、发布表要多一行。**收益 < 成本，不建议拆**。

超过 600 行棘轮上限的文件只有 5 个，其中**生产代码只有 1 个**且已被显式钉在棘轮基线里（`core/src/registry_core/plugin/contracts/contracts.rs`，639 行，`conventions/src/size.rs:56` 的 `CEILING = 600`）；另 4 个是测试形态文件（`cli/lib_tests.rs` 1015、`plugin-host/tests/fault_matrix.rs` 968、`examples/control-button/tests/registry.rs` 743、`studio/studio/app/tests/call_tree.rs` 702），按 `size.rs` 的边界豁免。**"太厚"在这个仓库里不是结构问题，是 1 个已知欠账。**

---

## 6. crate 顶层边界：目标 workspace 清单

> 队长 2026-09-28 追加：crate 顶层本身是一等审计对象（维护者原话："不仅仅是 crate 内部，crate 顶层本身也是如此，我现在觉得 crate 太多了，作为一个这么小的项目"）。

### 6.1 正面回答：12 个成员 / 9 个发布 crate，对一个 7.9 万行的项目是不是太多

**不是"太多"，但有一处该减。** 判定不用口味，用"独立 crate 的必要性"分类：

| 必要性来源 | crate | 可复算证据 |
| --- | --- | --- |
| **物理约束**（技术上不能合） | `macro`（proc-macro）、`build_method` vs `run_method`（build-dep / target-dep） | §5.2 |
| **打包/发布约束** | `conventions`（`publish = false`，遍历检出）、2 个 examples（`publish = false` 成员） | §3.3、§4.3 |
| **编译隔离**（合并会污染别人的依赖树） | `plugin-host`（`wasmi`；唯一不在 cli 闭包内的发布 crate）、`studio`（ratatui/crossterm/rataflow/notify）、`mcp`（独立安装面） | §3.2、§5.2 |
| **产品入口** | `cli`（伞；闭包 8 个 crate）、`core`（内核；12 个成员里 6 个直接依赖） | §3.1、§3.2 |
| **无以上任何一条** | **`debug_method`**（444 行、消费者 2 个、无安装面、且造成一处幻影依赖） | §3.4、§5.1 |

体量分布是**双峰**，不是"一堆微型 crate"：`core` 21.8k / `studio` 12.5k / `build_method` 10.1k / `mcp` 8.7k / `run_method` 8.3k / `conventions` 6.4k 六个在 6k 以上；`cli` 2.8k / `plugin-host` 3.9k 两个在中段；只有 `macro`(917) 与 `debug_method`(481) 在 1k 以下，而 `macro` 是物理不可合并的。**11/12 个成员能给出可复算的"为什么必须独立"，1 个不能。**

### 6.2 多 crate 的实际代价（量化）

| 代价 | 数字 | 复算命令 |
| --- | ---: | --- |
| **一次版本线推进要动的文件** | **13**：11 个 `Cargo.toml`（20 处内部要求 + 1 处 workspace 版本）+ `Cargo.lock`（其中 10 条 `xirang-*` 版本行，两个示例仍是 `0.1.0`）+ `CHANGELOG.md` | `grep -rn 'version = "0.1.6"' --include='Cargo.toml' . \| grep -v /target/` |
| 版本线是硬约束（不允许"用到才抬下界"） | 工具强制 `req == ^workspace_version`，否则 `--check-table` 红 | `tools/xirang-publish:462`、`tools/xirang-publish:602`、`--check-table` |
| **一次跨 crate API 改动**额外要动 | 发布顺序表（层表 7 行 / 依赖表 9 行）只在**增删 crate 或增删内部依赖边**时才动；改名则要动 | `tools/xirang-publish:86`、`tools/xirang-publish:323` |
| 发布单元 = 整个工作区 | 一次 `--publish --yes` 按 7 层发布 9 个 crate；**没有"只发一个 crate"的路径** | `tools/xirang-publish` 的 `--publish` 分支 |
| CI 里按 crate 名的引用 | 4 处 `-p`（`core`、`build_method`、`plugin-host`、`examples/control-button`） | `grep -rno '\-p xirang[a-z-]*' .github` |
| 文档里按 crate 名的引用（自身目录之外） | `core` 113 处/43 文件、`build_method` 72/38、`cli` 70/28、`run_method` 65/38、`studio` 55/28、`macro` 43/17、`plugin-host` 32/23、`debug_method` 30/26、`mcp` 28/15、`conventions` 13/9 | `grep -rn '<name>' --include='*.md' --include='*.yml' --include='*.rs' . \| grep -v /target/ \| grep -v '^./<dir>/'` |
| 脚手架模板里按 crate 名的引用 | 2 个（`xirang-run-method`、`xirang-build-method`），且模板会**生成**宿主的清单 | `grep -rn 'xirang-' build_method/scaffold/` |
| 依赖声明重复 | `serde_json` ×4、`syn` ×5、`proc-macro2` ×3、`tempfile` ×2 | §4.4 |
| 特性词汇分散 | 7 个特性名 / 4 个 crate / 2 个默认开 | §4.2 |
| 一次安装要编译的 crate | `cargo install xirang-cli` → 8 个 | §3.2 |

**这张表回答"共享版本线是否意味着任何小 crate 变更都要整条线推进"：是，而且是门禁强制的。** `docs/roadmap-1.0.md:74` 把"保留编译隔离与**独立发版**"列为保留 9 个 crate 的理由，但今天**没有独立发版**：版本线一份、内部要求必须等于当前线、发布脚本一次遍历 9 个。**保留 9 个 crate 的收益只剩"编译隔离"，"独立发版"这条收益并未兑现。**（建议 t14 二选一：改文档承认，或放宽工具实现真正的独立发版。）

### 6.3 目标清单与合并方案

**目标：11 个成员 / 8 个发布 crate。**

| 动作 | 成员 | 发布面 |
| --- | --- | --- |
| 保留 | `core`, `macro`, `build_method`, `run_method`(+`evidence`), `plugin-host`, `studio`, `mcp`, `cli`, `conventions`, `examples/control-button`, `examples/control-button-graft` | 8 个 |
| 合并 | `debug_method` → `run_method`（新非默认特性 `evidence`）；`debug_method` 从 `members` 移除；发布表两处删名 | −1 |

合并后的模块归属：

```text
run_method/evidence/evidence.rs     (原 debug_method/src/lib.rs 的重导出，feature = "evidence")
run_method/evidence/mir.rs          (原 debug_method/src/mir.rs)
run_method/evidence/adapters.rs     (原 debug_method/src/adapters.rs)
run_method/evidence/collector.rs    (原 debug_method/src/collector.rs)
run_method/tests/collector_integration.rs
```

`debug_method/Cargo.toml` 的三个依赖进 `run_method` 的 `[dependencies]`：`petgraph = "0.8"`、`tracing = "0.1"`、`inventory = "0.3.24"`，全部 `optional = true`，由 `evidence = ["dep:petgraph", "dep:tracing", "dep:inventory"]` 开启。`run_method/macros/face_registration.rs:166` 改成 `$crate::evidence::submit!`。`mcp`、`studio` 的清单各删一行，各加一处 `features = ["authoring", "evidence"]`；示例宿主默认**不开** `evidence`（它们今天也不依赖 `debug_method`）。

**迁移路径与判定标准**（t14 会把每一步展开）：

1. **不可回滚的那一步**：`debug_method` 从 `members` 移除、发布表删名。包名一旦发布就不能删除，已发布的版本只能 yank（yank 之后新的解析拿不到它，已有锁文件仍能用旧版构建），Cargo 也没有"包名重定向"机制。所以一旦这样发出：旧用户停在旧版本、新用户看不到这个名字，而仓库里再没有对应的 crate。因此这一步必须与 0.2.0 的版本推进**同一次**完成。
2. 可回滚的步骤：模块搬进 `run_method/evidence/`、特性加上、`mcp`/`studio` 改依赖——这些在任何时候都能搬回来。
3. **若维护者不愿付这次破坏**：保留 12/9，把 §6.2 里"零风险的替代"做掉（`[workspace.dependencies]` 收敛 + 版本线决策说清 + 边界表），成本几乎为零，收益（可读性/可维护性）涵盖多 crate 代价的大半。

**所以我给的推荐是分层的**：`debug_method` 合并是"该做且现在最便宜"（0.1.x、公开面未冻结、消费者只有 2 个树内 crate）；`[workspace.dependencies]` 与版本线决策是"无论合不合并都该做"。

### 6.4 合并对发布层面逐条的影响（落地清单）

| 面 | 影响 |
| --- | --- |
| `Cargo.toml` 的 `members` | 12 → 11（删 `"debug_method"`） |
| 共享版本线 | 不变（仍是 1 处 `[workspace.package] version`），但要随 0.2.0 推进 |
| 内部版本要求 | 20 处 → 17 处（删 `mcp`、`studio` 各 1 条 `xirang-debug-method`，删 `debug_method` 清单里的 1 条 `run_method`） |
| `[workspace.dependencies]` | 建议同批引入：内部要求 17 → 1 处（工具已支持，`tools/xirang-publish:523`/`:602`） |
| 发布层表（`tools/xirang-publish:86`） | `xirang-debug-method xirang-plugin-host` 这一层 → 只剩 `xirang-plugin-host`；层数 7 → 7（名字少 1 个，层数不变） |
| 发布依赖表（`tools/xirang-publish:323`） | 删 `xirang-debug-method:xirang-run-method` 一行；`xirang-studio`/`xirang-mcp` 两行各少一个名字 |
| `publish = false` 成员 | **不变**（仍是 `conventions` + 2 个 examples） |
| `tools/xirang-publish` 依赖顺序 | 若同时引入 `[workspace.dependencies]`，要跑一次 `--check-table` 确认继承拼法被读出（工具已有 `x.workspace = true` 分支） |
| `AGENTS.md` | crate 表删一行；"九个 crate"→"八个 crate"（出现多次）；`syntax`/`authoring`/`wasm` 的段落要加 `evidence`；特性名段落要说明新增非默认特性 |
| `README.md` / `README.zh-CN.md` | crate 表删一行（`README.md` 的 `xirang-debug-method` 行）、`debug_method/` 目录段删掉、`Workspace layout` 文本块删一行 |
| `CHANGELOG.md` | 0.2.0 段写明"`xirang-debug-method` 并入 `xirang-run-method`（`evidence`）"；**不要回改历史段**（那是记录） |
| `docs/migration*.md` | §3.4 的幻影依赖说明改成 `run_method/evidence` |
| `deny.toml` | 无变化（`petgraph`/`tracing`/`inventory` 已在图中；许可证已在 allow 列表内） |
| CI | 4 处 `-p` 都不点名 `debug_method`，无需改；`--all-features` 会覆盖 `evidence` |

### 6.5 身份与 API 风险（挂点，留给 t14 的专章）

crate 合并会同时改**包名、目录名、lib 名与模块路径**，比目录内移动危险一个量级：

- `NodeId` 由 `file!()` 派生（AGENTS.md 的挂载约定就是为了保护它）。**模块路径改变会改变模块内注册面记录的 `file!()`**，从而静默改变身份 → 落盘 graft 记录不再解析。因此 `debug_method → run_method` 这一步**只搬"不做注册面声明"的代码**，且要跑 `conventions` 的 mounting 门禁 + `run_method/tests/graft_record.rs` 做验证。
- 公开 API 路径：`xirang_debug_method::UnifiedCallGraph` 等 3 个 `pub use` 块（`debug_method/src/lib.rs`）会消失，`mcp`/`studio` 的引用必须同批改。这是"不可回滚"的第二个面。
- 版本推进的连带：任何内部版本 bump 都会把 9（→8）个 crate 一起推进，所以这次破坏**只能与一次版本推进同批**，不能单独发。

---

## 7. 我不建议动的部分

| 部分 | 为什么别动 |
| --- | --- |
| `macro/` 保持独立 crate | proc-macro 物理约束（§5.2）。它只有 917 行、4 个 `#[proc_macro]`，但**没有任何合并路径**。 |
| `build_method/` 与 `run_method/` 的边界 | build-dep / target-dep 的 triple 语义（§5.2）。动它等于动每个宿主的构建模型。 |
| `plugin-host/` 保持独立 crate | 唯一不在 cli 闭包内的发布 crate；把 `wasmi` 挡在 8 个 crate 之外（§3.2）。 |
| `conventions/` 的位置与 `publish = false` | 它遍历检出；放进已发布 crate 的 `tests/` 会让解包者失败（§3.3）。 |
| `examples/` 作为工作区成员、`studio/tests/fixtures/node-editor/` 作为**非**成员 | 前者要 CI/锁文件/排练覆盖，后者必须从不编译（§3.3）。 |
| `core/` 的模块划分与 `syntax` 特性形状 | `syntax` 已经用特性做到了"解析器隔离"，拆 crate 只会造出 core→core 的第二条依赖线。 |
| `cli` 对 `mcp`/`studio` 的依赖 | 这是"一次 `cargo install` 拿到全部命令"的实现方式（闭包 8 个，§3.2）；删掉它就要接受多包安装或多一次进程查找。 |
| 每 crate 自己的特性名（`syntax`/`authoring`/`wasm`/`process-tools`/…） | 它们是公开 API（AGENTS.md 明文）。要统一必须先抬大版本并写迁移说明。 |
| `tools/xirang-publish` 的两张手维护表 | 它们今天与清单一致（`--check-table` 绿）。要换成派生表是另一件事，不要顺手做。 |
| `studio/tests/fixtures/node-editor/` 的 `[workspace]` 表 | 那是"永不并入外层工作区"的机制本身。 |
| 4 个超过 600 行的**测试形态**文件 | 按 `size.rs` 的边界豁免；动它们买不到结构收益。 |

---

## 8. 待确认 / 交给下一环

1. **`conventions/src/mounting.rs:10` 的 "29" → 33**：请 gates-auditor 用他自己的口径复核一次（我给的口径在 §0/§2.1）。两边一致就进 t14 的"门禁散文去数字"清单。
2. **"约 84k 行"的口径**：本报告实测 78,951 行（工作树 `.rs`，排除 `target/`）。请 t17 对账时统一，避免两份数字混用。
3. **`debug_method` 合并案**：§6.3 给了目标清单与判定标准。这条与 t14 的"身份风险专章"直接相连；若维护者选"保留 12/9"，请明确否掉合并（我会在 t14 里改成"零风险替代"路线）。
4. **版本线决策**（§4.1/§6.2）：`docs/roadmap-1.0.md:74` 的"独立发版"理由与 `tools/xirang-publish` 的强制门禁不一致。要么改文档，要么放宽工具。这条**不影响**本轮任何其他审计，但会影响 1.0 的发布语义。
5. **`collector: debug` 的幻影依赖**（§3.4）：合并案会顺手解决；若不合并，建议至少在 `run_method` 的宿主文档里写清"用 `collector: debug` 需要自己加 `xirang-debug-method`"（现在只有 `debug_method/README.md:12` 提到）。
6. **`[workspace.dependencies]`**（§4.4）：建议做，且与合并案解耦（工具已支持继承拼法）。
