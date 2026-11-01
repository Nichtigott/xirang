# t46 全量重跑：七项对账检查在最终产物上的整体复核

- 对象：`docs/audit-2026-09-28/audit-report.md`（1704 行）、`audit-findings.json`（212 条）、`audit-structure-map.html`、`audit-naming-review.md`（543 行）、`audit-naming-ambiguity.md`（788 行）、`audit-publish-surface-merge-plan.md`（587 行）。
- 工作目录 `/home/nich/Moirai_N3/nichlink`；HEAD `cf0c378`。本轮只出报告：**源码一行未改、任何人的产物一行未改**；写权限只用在本文件；cargo 未运行（不需要），所有脚本落在 `/tmp/t46_*`。
- 装置说明：七项检查全部由**我自己写的脚本**重跑（Python，见文中"装置"行）；作者贴出的脚本只在**第 7 项**被用作"实现物"本身（把报告里的 `SITE_ORDER` 脚本抽到 `/tmp/t46_r4_script.sh`，只改 `ROOT` 默认值指向我自建夹具，其余逐字节保留），其余各项不采信任何自检结论。
- 静态比对（只读、未重跑谓词）的范围：第 1 项里的"id/old_ids 是否凭空出现"、第 4 项里的文案与清单、第 6 项的逐名一致性——这些用脚本在**文本/JSON 事实**上比对，不涉及运行期探针。

## 已核范围（先说清楚我亲手重跑了什么）

| # | 检查 | 我重跑的方式 | 结论 |
| --- | --- | --- | --- |
| 1 | t18 第一轮对账（7 个子项） | 自写脚本读 JSON + 报告，重算五处计数、id 集合双向差集、traceability、`":None"`、`title==id`、批次、falsified | 7/7 通过（2 条非阻塞记账备注） |
| 2 | t20 K1/K2/K3 | 自写脚本断言 `severity ≥ max(severity_sources)`、逐条重读来源行、比对 `meta.note` 声明集与数据、扫 `163` | 3/3 通过 |
| 3 | t22 K4/K5/K6 | 自写占位符扫描 + **全量锚点扫描器**（含区间两端与配对 token，按 §1.5 的排除项）+ 类别集比对 | 3/3 通过（1 条非阻塞备注） |
| 4 | t27 的 9 条命名复核 | 逐条定位当前文本 + **独立复算 394/105/34/28**（自写计数器） | 9/9 通过 |
| 5 | K-NAM1 配对锚点 | 读 `examples/control-button/src/lib.rs:44-58` 逐行核对 + 锚点扫描器 | 通过（宏调用在 `:48`，现引 `:48-54`） |
| 6 | 七个模块名判定 | 报告 §7.8 与方案 `:17-20`/`:62-65` 逐名比对 + 旧名残留全文扫描 + 各表同步检查 | 通过（旧名残留 0） |
| 7 | R-4 基线（V35-02 + V40-01/02/03） | 自建多站点夹具 + 抽出的谓词脚本两序各跑一次 + 真树三跑 + 自写解析核对 | 4/4 通过（2 条非阻塞残留） |

---

## 0. 必须先说的一件事：产物冻结之后，工作树动了（与七项检查的结论强相关）

这不是"发现"，是**环境事实**，它决定了下文若干处"为什么我复算的数字与报告不完全一样"：

- 五个**源码**文件在产物冻结之后被修改（mtime 17:43–17:53；`audit-findings.json` / `audit-report.md` / `audit-structure-map.html` 的 mtime 是 17:12，`audit-inventory.json` 是 12:43）：
  `build_method/node_id.rs`（117→181 行）、`build_method/identity_cache.rs`（101→306）、`core/src/registry_core/authoring/parse/admission.rs`（235→366）、`core/src/registry_core/plugin/catalog/catalog.rs`（455→535）、`studio/studio/app/mutations.rs`（395→586）；`git diff --stat` 合计 +845/−94。
- 这五个文件正是 **LGC-LG-01 / LGC-LG-02 / LGC-LG-03（三条 CRITICAL）与 LGC-LG-04（MAJOR）** 的所在文件，diff 的形态是**修复 + 回归测试**（例：`admission.rs` 新增 `both_lists_survive_the_read_and_write_round_trip`；`mutations.rs` 新增 `mod lock_writes` 与"锁不可解析时绝不允许成功"的断言；`node_id.rs` 把缓存键改成 `(命名空间, 相对路径)`，注释直接复述 LGC-LG-01 的机制）。
- 后果（我已逐条量化）：
  1. **§1.1 的代码规模基准已不再复现**：报告写 394 文件 / 78,951 行；我按同一口径实测 **394 文件 / 79,622 行**，差 **+671 行**，正好等于上述 5 个文件的行数增量（+205+64+131+80+191）。
  2. **R-4 的枚举数已变**：报告贴出的输出是"模块名 **296** / 声明处 **440**"，我今天跑同一脚本得 **297 / 442**；差额由修复引入的两处 inline 声明解释——`identity_cache.rs` 新增 `mod tests {`（+1 站点，名字 `tests` 已存在）与 `mutations.rs` 新增 `mod lock_writes {`（+1 站点且 +1 新名字）。**R-4a/R-4b/R-4c 的裁决数不受影响**（见第 7 项）。
  3. **4 条"权威定位"（标题 `file:line`）指向的内容已移位**：`LGC-LG-01` 的 `build_method/node_id.rs:36` 冻结时是那句 `CACHED_NODE_IDS.get()…` 的缓存读取，现在是修复后的一行文档注释；`LGC-LG-02` 的 `core/src/registry_core/authoring/parse/admission.rs:40` 由 `return match (allow.is_empty(), deny.is_empty())` 变成文档注释；`LGC-LG-40` 的 `core/src/registry_core/plugin/catalog/catalog.rs:281`、`STU-S-08` 的 `studio/studio/app/mutations.rs:320`、`STU-V-01` 的 `studio/studio/app/mutations.rs:332` 同样移位。按报告 §1.5 自己声明的口径（文件存在 + 行号在界内）这些锚点**仍全部为 0 违规**（文件变长了），但"标题行就是这个缺陷发生处"的承诺在当前树上**对这 5 个文件已不成立**。
- 我的处理：七项检查里凡涉及"当前树"的量（锚点扫描、R-4 枚举、394/78,951）都**按今天的树实测**并在此标注差异；凡涉及"冻结时的事实"（212 条字段、计数、命名报告文本）按产物自身比对。**这一条是否阻塞交付，留给队长**——它不在七项检查的判据内，但会让读者按报告去 `build_method/node_id.rs:36` 时看不到它描述的代码。

---

## 1. 第一轮对账（t18）

**装置**：Python 脚本读 `audit-findings.json` + `audit-report.md` 原文；traceability 用 `old_id_sources` 与 `source_report` 的**文件全文**比对；计数五处分别从 §0 表、§1.4 文本、§2 标题/表行、逐条字段、`counts` 取值。

| 子项 | 当前状态 | 证据 |
| --- | --- | --- |
| id/old_ids 是否都能在 `source_report` 原文里找到（0 条凭空出现） | **通过（0 凭空出现）**，附两条记账备注 | 212 条的每个 `old_ids`（239 项）与 `old_id_sources[].old_id`（243 项，去重后共 223 个不同 token）**全部**能在**语料**里找到；其中 26 个不在 `old_id_sources[].report` 指名的那份文件里（例：`LGC-LG-05` 的 `K-22` 记在 `audit-lane-surfaces.md`，实际出现在 `audit-lane-kernel.md`），落在总账/复核报告里——即该字段的语义是"这一行去重自哪份片区报告"，不是"这个旧 id 的出处文件"；5 条（`STU-V-01/02`、`GTE-N-1/2/3`）的 `old_ids` 为空而 `old_id_sources` 有原 id，报告 §2.3/§3.2 标"（复核新增）"+ 原 id，读者仍可回溯 |
| 严重度计数五处一致 | **通过** | 逐条字段 `{CRITICAL:3, MAJOR:66, MINOR:143}`；`counts.by_severity` 同；§0 表同；§1.4 正文 "CRITICAL 3 / MAJOR 66 / MINOR 143" 同；§2 明细 CRITICAL 标题 3、MAJOR 标题 66、§2.3 表行 143 |
| §2 与 JSON 的 id 集合双向相等 | **通过** | 212 个 id 全部出现在 `audit-report.md`（0 缺失）；分严重度各自相等（3/66/143），无重复 |
| `":None"` 坏指针 | **0** | JSON 与报告里 `":None"`/`:None` 均为 0 |
| `title == id` 占位标题 | **0**（且 0 条空标题） | 212 条逐条比对 |
| 批次与 JSON `batch` 逐条相符 | **通过** | §0 的 8 行批次表与 `counts.by_batch` 逐项相等（3/23/14/50/21/51/30/20）；§2.3 的 143 行"批次"列与 JSON `batch` 逐条 0 mismatch；§9.2 的 8 行"条数"列与 `counts` 相等，其"代表条目"列抽样的 45 个 id **全部属于所标批次**（该表自述是抽样，全量以 JSON `batch` 为准） |
| 已被证伪的条目不得以原结论留在队列 | **通过** | 5 条 `mechanism_falsified`（`LGC-LG-20/33/34`、`NAM-40`、`AMB-06`）各有非空 `falsified_mechanism`、`verify_status=partial`，且 §4.1 有专表逐条写明"被证伪的原机制 / 保留下来的结论"；`verify_status` 取值集里不存在 `falsified` |

**非阻塞备注**：① 26 处 `old_id_sources[].report` 不是该 token 的所在文件（跨片区合并的旧 id 一律沿用本行的 `source_report`）；② 5 条 `old_ids` 为空的新增行（原 id 只在 `old_id_sources` 与 §3.2）。两条都不影响"0 凭空出现"，但会让只读 `old_ids` 的下游脚本看不到这 5 条的出处。

---

## 2. 第二轮（t20 的 K1/K2/K3）

**装置**：自写脚本按 `severity_sources`（三处来源：lane / verify / ledger）取最大值，与最终 `severity` 比较；另**逐条重读每个来源行**核对记录的严重度是否真在那一行上（不是"字段自洽"就算过）；`meta.note` 的声明集逐项对数据；全文扫 `163`。

| 子项 | 当前状态 | 证据 |
| --- | --- | --- |
| K1 是否存在没有 `severity_note` 的降级 | **通过（0 违规）** | 212 条里 `severity < max(severity_sources)` 只有 2 条：`LGC-LG-33`（MAJOR→MINOR）与 `STU-S-04`（MAJOR→MINOR），**都带 `severity_note`**；`severity > max` 0 条；`severity_required` 与逐条来源最大值一致；`counts.severity_changed_with_note` 列出 3 条（含 `LGC-LG-50` 的 MINOR→MAJOR）。**独立重读**：我读了 432 个来源行，412 行在行内直接带有所记录的严重度 token，20 行是"该行不带严重度词"的表头/叙述行，**0 处记录与来源行矛盾** |
| K2 `meta.note` 的取值集与逐条字段一致 | **通过** | 逐条字段实际集：`verify_status ∈ {confirmed, partial, unverified}`（**无 `falsified`**）、`evidence_kind ∈ {runtime, read, reviewed, none}`；`meta.note` 声明的两个集**逐字相同**；`meta.note` 声明的 `severity_source` 分布（ambiguity-report 20、declared-table 110、ledger-table 44、max-of-sources 3、naming-report 30、verified-override 5）与 `counts.by_severity_source`、与逐条重算**三处逐项相等** |
| K3 全文旧总数 163 | **通过** | 报告里 2 处、JSON 里 4 处，全部是**行号/区间引用**（`studio/studio/app/navigation.rs:155-163`、`studio/studio/ui/search/detail.rs:15/163`、`build_method/graft_view/declared.rs:163`、JSON 的 `"line": 163` 字段），没有一处是旧总数 |

---

## 3. 第三轮（t22 的 K4/K5/K6）

**装置**：① 占位符扫描：正则 `\{[A-Za-z_][A-Za-z0-9_.\[\]()'"]{0,40}\}` 全文扫（含代码块），并单独 grep `{len(`/`{counts`/`{total}`/`{n}`/`{c[`/`str(`；② **自写全量锚点扫描器**：212 个 `file`/`line` 字段 + 正文每个 `<path>:<line>[-<line>]`（区间两端都查），排除 `meta.unresolved_quoted_anchors`、`meta.nam_pattern_tokens`、`/tmp`、`target/`、裸 basename、§7 的 21 处"文件级定位"，并按 `conventions/src/doc_anchors.rs` 的配对规则校验 `` `token`（`path:line`） ``；③ 类别集比对。

| 子项 | 当前状态 | 证据 |
| --- | --- | --- |
| K4 未展开的模板占位符 | **0** | `{len(`、`{counts`、`{total}`、`{n}`、`{c[`、`str(` 全为 0；全文 brace-identifier 候选 12 处，全部是引文里的 **Rust 格式串**（`format!("use {crate_name} as _;")` 一类）或宏占位符（`{source}`/`{framework}` 是插件锁字段名），不是渲染残留 |
| K5 正文锚点可解析 + 配对 token 在所指行 | **通过（0 违规）** | 212 个字段：**0 违规**。正文按 §1.5 声明口径（排除引文行）**检查 366 条**：0 不存在、0 越界、0 配对 token 不符；我另做**严格变体**（连 §1.5 排除的"现象与判据/最小修复方向"两行也扫）：**559 条**，仍是 **0 违规**、0 配对不符。§1.5 的"扫描范围 + 三类排除项"已写成文 |
| K6 `severity_source` 类别清单与数据一致 | **通过**，附 1 条非阻塞备注 | 分布清单 6 个类别与数据**逐项相等**（报告 §1.4 / `meta.note` / `counts` / 逐条四处）；`inline-heading` 在两个文件里**各 0 次**（t22 的原缺陷已关闭）。**但**：§1.4 的"定义"句里仍解释了一个数据中 0 条的类别 `verifier-section`（报告出现 1 次、`meta.note` 0 次、数据 0 条）。这与 t22-K6 判定"声明了 0 条的类别"是同一形状，只是位置从"分布清单"挪到了"定义句"；分布清单本身与数据逐项相等，故不阻塞 |

---

## 4. 命名报告复核（t27 的 9 条）

**装置**：定位当前文本逐条读；另**自写计数器**独立复算四个数（394、105、34/28/6）。

| # | 子项 | 当前状态 | 证据 |
| --- | --- | --- | --- |
| 1 | `NAM-40`「394/394 一致」改成正确口径（14 处四族） | **已改** | `docs/audit-2026-09-28/audit-naming-review.md:346`（结论级更正："这条是错的"）→ `:349` 给出 14 处分四族；`:15` 修订表①"照改"；`:344` 条目标题已改为"结论已修订（原'干净'被证伪）" |
| 2 | D-10/D-11 不再引用它 | **已不再引用该结论** | D-10（`:402`）判定列改为"文件 stem 比对（不是 `module_path`）"，现状列把"文件名 43/44 成对"与"模块名自由（14 处有意）"分开写并指向**更正后的** `NAM-40`；D-11（`:403`）通篇不提 `NAM-40`。全文 5 处 `394/394` 全部在"被证伪/是错的/不要引用"语境（`:15`、`:23`、`:346`、`:362`、`:538`） |
| 3 | `D-3` 统一为递归谓词 34（仓库）/ 28（发布面）+ 豁免 | **已统一且我独立复算一致** | `:395` 的 D-3 行写明谓词（递归 `.rs` 集合恰为 `{<dir>.rs}`）∧ 路径不以 `examples/`/`studio/tests/` 开头，三口径"仓库 34 / 发布面 28 / 豁免后 0"，并声明不再用 39/67；**我的自写计数器**（os.walk 递归集合比对，排除 `target/`）实测 **34 / 28 / 6**，与豁免清单（`examples/control-button/...` 3 处 + `studio/tests/fixtures/node-editor/...` 3 处）逐条一致 |
| 4 | `NAM-05` 降 MINOR + 修法改对（`build_evidence.rs`） | **已改** | `:108` 标题即"MINOR（原 MAJOR，t27 复核后降级）"；正文承认计数应为 **12/17**，修法采用 `mcp/evidence.rs` → **`build_evidence.rs`**，并写明"`build_evidence` 只指 mcp 侧文件，`debug_method` 模块名是 `call_evidence`"（`:108` 的注 + `:20` 的表） |
| 5 | `NAM-06` 降 MINOR + 修法改对（`write_guard.rs`） | **已改** | `:422` 标题"MINOR（原 MAJOR，t27 复核后降级并改修法）"；正文**撤回**"并入 `mutations.rs`"，改为改名 **`write_guard.rs`**；两个目标文件在工作树里都还不存在（`ls` 均 No such file），与"只出报告、源码不改"一致 ✓ |
| 6 | `D-5`/`D-7` 计数与逐行清单一致 | **一致** | D-5（`:397`）：全量 **24 行**、白名单 9、非入口位 **15 处**；我逐行数 `NAM-32` 的表：**24 行 / 15 个"违规" / 9 个 WL**，24−9=15 ✓。D-7（`:399`）：计入 **8**、全量 **25**；我数 `NAM-34`：计入表 **8 行**，"不计入的 17 行"两类合计 **17**（领域词 14 + 惯用语/实现分界 3），8+17=25 ✓ |
| 7 | `NAM-02` 引的类型真实存在 | **已改且存在** | `NAM-02`（`:1` 起）现写 `CallRef`/`CallTreeView`，并明确"原写的 `CallRefRing` 在本仓不存在"；我 grep 实测 `studio/studio/app/state/misc.rs:55 pub struct CallRef`、`:76 pub struct CallTreeView` ✓ |
| 8 | §0 的目录数 105 | **一致且我独立复算** | `:35` 写"394 个 `.rs` + 全部 105 个目录（谓词：394 个文件所在的不同父目录）"；我的计数器实测 **105** ✓（同一谓词） |
| 9 | 四条漏项补上 | **全部补上** | ① `NAM-03`（`:99`）有 ⚠️"这是公开路径变更（t27 补的漏项）"，点名 `conventions/src/shims.rs:50` 与同表其余条目；② `NAM-43`（`:363`）新增，记录四家族；③ `*_tests → mod tests` 的不统一写在同一处（44 个 `*_tests.rs` 里 40 个直接挂载、4 个挂 `mod tests`）；④ `entry_pages::get_mut` 出现在 `NAM-37`（`:336`/`:337`，明标"不计"）、`NAM-32` 第 21 行与 D-4 行 |
| 10 | 「干净」类结论写成「谓词 + 命令 + 输出」 | **已改** | §F 标题即"每条写成 谓词 + 命令 + 输出"，修订注说明"原来把结论写成 X/N 一致而不给谓词"；表头为 `结论 / 谓词（§G 编号）/ 命令 / 输出`，并指向 §G 的 P1–P8 可复算脚本 |

---

## 5. K-NAM1（t29）配对锚点

**装置**：直接读 `examples/control-button/src/lib.rs:44-58` 并逐行核对宏调用的真实行号；再用第 3 项的全量锚点扫描器验配对规则。

- 报告里这条现在写作（`docs/audit-2026-09-28/audit-report.md:1421`）：`` 类型化 `static_graft_plan!`（`examples/control-button/src/lib.rs:48-54`，宏调用在 `:48`、两个 `cut(...)` 实参在 `:50`/`:52`） ``。
- 文件实测：`:48` = `xirang_run_method::static_graft_plan!(`；`:49` = `FRAMEWORK,`；`:50` = `cut(crate::control::object::button::NODE_ID)`；`:52` = `cut(crate::control::object::slider::NODE_ID)`；`:54` = `);`。
- **判定：通过**——锚点区间 `48-54` 覆盖整个宏调用，配对 token `static_graft_plan!` **落在区间内（第 48 行）**；t29 报的"引 `:50`/`:52` 而 token 在 `:48`"已修掉。全量扫描器对此处也判 0 违规。

---

## 6. 七个模块名判定（t35 的 §3）

**装置**：报告 §7.8 与方案 `:17-20`、`:62-65` 逐名比对；全文扫旧名残留；逐节检查随改名同步的表格与示例。

| 模块名 | 报告 §7.8（`:1489-1497`） | 方案 `:17-20`（修订记录）+ `:62-65`（并入表） | 一致？ |
| --- | --- | --- | --- |
| `build` → **`build_time`** | 建议 `build_time` | #1 行 `build` → `build_time`；`:62` 并入行写"模块 **`build_time`**" | ✓ |
| `run` → **`runtime`** | 建议 `runtime`（顺带消掉 E0428） | #2 行 `run` → `runtime`；`:63` 写"模块 **`runtime`**" | ✓ |
| `plugins` → **`plugin_host`** | 建议 `plugin_host` | #3 行 `plugins` → `plugin_host`；`:65` 写"模块 **`plugin_host`**" | ✓ |
| `call_evidence` 保留 | 保留（t31 裁定；feature 仍 `evidence`） | #4 保留；`:64` 写"模块 **`call_evidence`**" | ✓ |
| `studio` / `mcp` / `cli` 保留 | 保留（各自理由） | #4 保留；`:66-68` 三行"保留" | ✓ |

随改名同步的四处（我逐处核对）：
- **根导出/glob**：方案 §2.1/§2.2 的 `toolchain/src/lib.rs` 形态（`:228-237`）已是 `build_time`/`runtime`/`call_evidence`/`plugin_host`，并明写"只 glob 宿主面向的 `runtime` 模块"（`:220`、`:237`）；
- **E0428 分析**：§2.2 的"消失/仍存在"表（`:205-220`）把"`pub mod run;` 与 `pub use build::run;` 重名"标为**已消失**（并给了理由：模块名改成 `runtime`），把"三个函数同名 `run`"标为**仍存在**，与 t35 要求的改法（理由改成"删掉一条只为躲重名而设的规则"）一致；
- **撞名表**：`:214-219` 的模块表已用新名，逐行给出各自会撞的公开名（`build_time`/`mcp`/`cli` 各带一个 `run`，其余四个"无"）；
- **feature/bin/`required-features` 表与安装命令**：§1.4（`:145-155`）用新包名 `xirang-toolchain` 给出五条 bin 的 `required-features` 与 `cargo install xirang-toolchain --features cli|studio,node-graph|mcp`；§1.3 保留 feature 名并给对照表（`:166` 说明 `features = ["run"]`/`["build"]` 与模块名不同的理由）。
- **旧名残留计数 = 0**：全文扫 `pub mod build;`/`pub mod run;`/`pub mod plugins;`/`build::`/`run::`/`plugins::`，只命中 `:196` 与 `:209` 两处，且都在"**已消失**/旧叙述已失效"的对照语境里；`§10.1` 也明写"第一版 NAM-40 的 394/394 已被证伪并撤回，本文任何句子都不以它为据"。

---

## 7. R-4 基线（t40 的 V35-02 + V40-01/02/03）

**装置**：① 把报告 §5 的 `SITE_ORDER` 脚本抽到 `/tmp/t46_r4_script.sh`（逐字节保留，只把 `ROOT` 默认值改成我的夹具路径另存为 `/tmp/t46_r4_fixture.sh`）；② 自建夹具 `/tmp/t46fix`：`src/lib.rs` 有 crate 级 `pub mod build;`，`src/diagnostic/diagnostic.rs` 有 `mod build`（即 `diagnostic::build`），**两站点并存**；③ 夹具两序各跑一次、真树三跑（无 env / walk / reverse）；④ 自写解析核对手工复算的数。

| 子项 | 当前状态 | 证据 |
| --- | --- | --- |
| V35-02-1 `build` 豁免**逐站点**判定 | **通过** | 夹具两序输出：`新逻辑(逐站点) 报出 1 处, 豁免 1 处`；报出的是夹具根那处 `build`（`/tmp/t46fix` 下的 `src/lib.rs` 第 2 行，parent=`<crate root>`），豁免的是 `diagnostic::build`（同夹具下 `src/diagnostic/diagnostic.rs` 第 2 行，parent=`diagnostic`）；两序**裁决完全相同**（只有 echo 访问序的那一行不同） |
| V35-02-2 与文件顺序无关 | **通过（裁决/计数无关）** | 真树三跑 `违规声明处 26 违规名字 15 豁免声明处 1` **三次逐字相同**；夹具两序同为 1 报 1 免。**精确化**：违规**清单的打印顺序**随 `SITE_ORDER` 变化（reverse 下各名字的站点次序颠倒），因此若把 t42 的"逐字相同"理解为全文比对则不成立，理解为**裁决与计数**则成立——谓词把访问序做成显式输入，正是为了让这一点可测 |
| V40-01 `app`↔`application` 已登记豁免 | **已登记**（残留见下） | 谓词里 `ALLOWED_PAIRS={("app","application")}`（报告 `:527`），模型行 `:725` 写"登记豁免 1 对"、现值 **6 对 / 12 名**；实跑输出 `登记豁免 1 ['app↔application']`、`违规对 6`、`单复数族豁免 7`，与模型行逐项相符 |
| 反过火 0 误报 | **通过** | 真树 R-4a 命中的 14 处全是**真命中**：3 目录（`run_method`/`debug_method`/`build_method`）+ 3 包名 + 3 lib 名 + 5 个模块名（`data`、`face_helpers`、`misc`、`support`、`test_support`），全部在 BAD_TAIL/动作词表内；`token` 精确匹配避开了 `metadata` 一类子串陷阱（`metadata` 未出现）；R-4b 的 15 名/26 处与报告 §5 模型行一致 |
| V40-02 "声明形式"半句已删 | **已删** | 全文 grep `声明形式` 只命中 `:37`（修订表里"删掉'与声明形式'那半句"这句**描述删除动作**的文本）；`:728` 的对照段只归因于动词表（`graft` −7、`walk` +1 = −6、→ 33−6=27），并明写 inline `mod X {` 对 R-4b 贡献 **0 处** |
| V40-03 `296−275=21` 自洽 | **自洽** | `:776` 写"296 个模块名里的 275 个（只与谓词比对；被点名的 21 个逐个读过，296−275=21）"；全文再无 `270`（另一处 `270` 是 `core/src/registry_core/tree/graft_ops/graft_ops.rs:255-270` 的行号区间，无关） |

**非阻塞残留 R1（MINOR）**：R-4c 的**改法表与落地顺序**没有跟着 V40-01 的豁免同步——`:749` 仍写"R-4c 的 7 对"并把 `app`↔`apply`/`application` 并在一格里说"至少一个加限定"，`:758` 仍写"R-4c（7 对，必须降到 0）"、"`app`↔`apply`/`application` 各改一侧"；而权威的模型行（`:725`）与谓词已是 **6 对 / 12 名**且 `app`↔`application` 登记豁免。同一格里的 `strudio` 是 `studio` 的笔误。判据：同一报告内"现值"与"改法清单"对同一指标给出 6 与 7 两个数，读者按改法清单执行会把已豁免的一对也改掉。

**非阻塞残留 R2（MINOR）**：报告贴出的 R-4 输出是 `296 / 440`，今天的同一脚本在今天的树上给 `297 / 442`（原因见第 0 节：修复引入 `mod tests {` 与 `mod lock_writes {`）。R-4a/R-4b/R-4c 的裁决数**未变**（14/14、26/15/1、6/1/7 与我实测逐项相同），所以这只是"贴出的枚举行已过期"。

---

## 8. 判定

七项检查**全部通过**，无阻塞项；下列 5 条为非阻塞残留或环境提示，建议在下一版或下一轮处理：

| # | 内容 | 严重度 | 归属 |
| --- | --- | --- | --- |
| R1 | R-4c 改法表/落地顺序仍写"7 对"与"各改一侧"（未同步 V40-01 的豁免；含 `strudio` 笔误） | MINOR | 第 7 项 |
| R2 | 报告贴出的 R-4 枚举行 `296/440` 已被修复轮改成 `297/442`（裁决数不变） | MINOR | 第 7 项 + 第 0 节 |
| R3 | `old_id_sources[].report` 对 26 个跨片区旧 id 不是该 token 的所在文件；5 条新增行的 `old_ids` 为空（原 id 只在 `old_id_sources`/§3.2） | MINOR | 第 1 项 |
| R4 | §1.4 的"定义"句仍解释一个 0 条类别 `verifier-section`（分布清单本身与数据逐项相等） | MINOR | 第 3 项 |
| R5 | **环境**：5 个源码文件在产物冻结后被就地修复（3 条 CRITICAL + 1 条 MAJOR 所在文件），导致 §1.1 的 `394/78,951` 与 4 条标题锚点的**指向内容**过期（行号仍在界内，故不违反 §1.5 的声明口径） | 需队长裁定 | 第 0 节 |

**在以上七项检查下，最终产物可以交付**（0 阻塞项；上表 R1–R5 为非阻塞残留或环境提示）。唯一需要队长决定的是 R5：产物记录的"冻结状态"已不再等于今天的工作树——若要把它作为对当前树的权威清单发布，建议重跑一次 §1.1 的数字与那 4 条锚点（或在本报告旁附一份"修复轮 delta"）。
