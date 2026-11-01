# 交付前终局复扫（t38）：命名歧义轴折入 + K-NAM1 修复 + 合并方案连带同步

对账人：gates-auditor。工作树 `HEAD=cf0c378`；**源码一行未改、未改任何人的产物**；本文件是本轮唯一写入。
判定：**可以交付**（最后一句复述）。

## 已核范围（这份签字的边界）

**本轮复核（终局复扫：新增轴 + 三处修复 + 回归，不重跑整轮）**
1. 新轴「命名歧义与约定冲突」的 *findings*（§7.6 逐条表）与 `audit-findings.json` 的 `AMB-*` **id 集合双向比对**（含重复）；§7.9「不是缺陷」清单（10 行）与 JSON 的关系。
2. `AMB-*` 的 `verify_status` 与 `audit-naming-ambiguity-verify.md`（t35）逐条判定对照；证伪/降级项在 §4 的说明；`unverified` 的标注与严重度。
3. `AMB-*` 的严重度不变量：**我自己的解析器**从 `audit-naming-ambiguity.md`（严重度行）与 `-verify.md`（判定表）重取声明严重度，逐条断言 `severity ≥ max(来源)` 或带 `severity_note`。
4. 报告 §7.8 的七个模块名判定与 `audit-publish-surface-merge-plan.md` §2/§1.1 表**逐名比对**；方案里 `§2.1/§2.2`（E0428 消失/仍存在）、`:178` 撞名表、feature/bin/`required-features` 与安装命令的同步抽查；**旧名残留计数**。
5. K-NAM1：报告 `:1421` 的锚点是否可解析、配对 token 是否在所指行/区间（我自己读了 `examples/control-button/src/lib.rs:46-56`）。
6. 两条非阻塞：`NAM-43` 的 `verifier`；§1.5 排除项③ 是否补了真实例子（我独立数了它声称的 21 条）。
7. 回归：计数五处一致 + `counts` 六组聚合重算；批次/id 前缀映射表含 AMB↔B8；§3.2 映射覆盖；模板残留；正文全量锚点扫描（含配对 token 规则）与 §1.5 排除项逐条对应；HTML 无外链 + 新页签 `amb` 数字与 JSON/inventory；随机 10 条 `file`/`line`；`check_report.py` 真实产物 exit 0。

**口径说明（不是缺陷）**：我自己的锚点扫描器只把**源文件**（394 个 `.rs`/配置文件源）当可解析对象，本轮得 **555 条**正文锚点；交付消息里的「1021 条」应是"所有 `<path>:<line>` 形状（含 `.md`/`.json`/`.toml`/`.py`/`.html`，区间计两端）"的口径。报告正文**没有**声称任何锚点总数（只声明范围与排除项），因此两个数字不冲突。

**未在本轮复跑（沿用 t18→t29 的结论）**：非命名/歧义条目的出处核验与独立严重度重取（162 条时已做）、§8.2 与 `batch` 的逐行比对、严重度不变量的"修前为红"实验、语言比例与目录前缀（本轮顺带复查，见 §6）。

**明确不在签字范围内**：任何技术结论（AMB 各条的命名建议是否恰当、合并方案的取舍、`xirang-toolchain` 保留/改名的裁定）；`meta.unresolved_quoted_anchors`（63）与 `meta.nam_pattern_tokens`（17）声明 token 的**内容**；`/tmp` 探针与工具产物；`docs/audit-2026-09-28/` 之外的文件。

---

## 1. 新轴 ↔ JSON —— **通过**

- §7.6（逐条表）= **20 行、20 个不同 id**；与 JSON 的 20 个 `AMB-*`：**missing=[] extra=[] dups=[]**（双向相等、无重复）。§7.6 表里的严重度列与 JSON 逐条**无 mismatch**（我逐行比对）。
- `verify_status` 与 t35 判定逐条一致（我从 `audit-naming-ambiguity-verify.md` 的表行解析出 16 条判定）：
  - `证实` → `confirmed`（AMB-02/03/04/05/07/08/09/10/11/15/17/19/20）；`部分证实（方向对，主理由要换）` → `partial`（AMB-01）；`部分证伪（机制错，现象可留）` → `partial`（AMB-06）——**无一 mismatch**；
  - t35 未涉及的四条（`AMB-12/13/14/18`）＋ 表里有行但无判定的一条（`AMB-16`）= JSON 的 5 条 `unverified` ✓。
- 被证伪项的去向：`AMB-06`（机制被证伪、现象可留）在 **§4.1** 有专条，`mechanism_falsified=true` + `falsified_mechanism` 写明了被证伪的机制与保留下来的结论 ✓。
- `unverified` 不得是 CRITICAL/MAJOR：5 条（`AMB-12/13/14/16/18`）**全部 MINOR** ✓；§2.3 的行里带 `unverified` 字样，且都在 §10.2「未复核条目（17 条）」里。
- **§7.9「不是缺陷」清单与 JSON 的关系**：该节 **10 行且不引用任何 id**（列的是名字），因此与 JSON 的 `AMB-*` 集合**无交集、无对应行** ✓。抽查两条它的"为什么不判"依据：`mir` 一行称它**确实**是 rustc 的 MIR —— `core/src/registry_core/mir/mir.rs:1`（第 1 行）实测 "Static MIR candidates and evidence-aware call relation merging." ✓；私有模块 `mod build;` 一行称父路径已消歧 —— `core/src/registry_core/diagnostic/diagnostic.rs:14` 实测就是 `mod build;`（`conventions/src/size.rs:22` 也这么引用）✓。

## 2. 严重度不变量覆盖到 AMB —— **通过**

- 我自己的解析器（从 `audit-naming-ambiguity.md` 的 `- **严重度**：X` 行取，20/20 条都有；再用 `-verify.md` 的判定表交叉）：**20/20 条都取到来源**，`severity ≥ max(来源)`，**0 违规、0 条需要 `severity_note`**。
- 具体：`AMB-01…05` = MAJOR（来源 `audit-naming-ambiguity.md:66/81/96/110/124`），`AMB-06…20` = MINOR（来源行 137/150/162/174/186/198/210/221/232/244/256/269/281/292/304）；`AMB-06` 的 MAJOR→MINOR 已由来源报告自己改写为 MINOR（§1 头写明「AMB-06 已于本轮按 t35 降为 MINOR」）并在 §4.1 记账 ✓。
- JSON 侧：`severity_source` 20/20 = `ambiguity-report`；`mechanism_falsified` 仅 `AMB-06`；`severity_note` 为空是**正确**的（没有任何一条低于来源）。

## 3. 与合并方案一致 —— **通过**

- **七个模块名逐名一致**：报告 §7.8 与 `audit-publish-surface-merge-plan.md` §2 表（`:17-20`）都是 `build`→**`build_time`**、`run`→**`runtime`**、`plugins`→**`plugin_host`**，`call_evidence`/`studio`/`mcp`/`cli` **保留**；`:62-65` 的并入表把四个源 crate 的模块名一一对上（`build_method`→`build_time`、`run_method`→`runtime`、`debug_method`→`call_evidence`、`plugin-host`→`plugin_host`）✓。
- **`call_evidence` 与 mcp 侧 `build_evidence.rs` 不混用**：报告 `:1345`/`:1351`/`:1406`/`:1468` 与方案 `:569` 都写明 `debug_method` 的模块名 = `call_evidence`（feature 仍叫 `evidence`），而 `build_evidence` 只指 `mcp/build_evidence.rs`（NAM-05 的修法）✓。
- **同步抽查（≥3 处）**：
  1. `§2.1 :196` 与 `§2.2` 的"消失/仍存在"表（`:209-211`、`:215`、`:218`）：E0428 记为**已消失**（模块名不再是 `run`），而**三个 `run` 函数仍在**（`build_time::run()` / `mcp::run()` / `cli::run(argv)`）——与改名后的事实自洽 ✓；
  2. `:178` 起的撞名表 + `:267-270` 的依赖替换表：根 glob 改为 `runtime`、其余按模块路径寻址（`xirang_toolchain::build_time::` 等）✓；
  3. feature/bin/`required-features` 与安装命令：`:137-139` 的 feature⇄模块对照表（13 个 feature 名**全部保留**，另给 README 的四行对照表 `build`→`build_time`、`run`→`runtime`、`plugins`→`plugin_host`、`evidence`→`call_evidence`），`:149/151/153` 三条 `cargo install xirang-toolchain --features cli|studio,node-graph|mcp`，`:162/164/166` 的 `autobins=false` 与 `default=[]` 说明 ✓。
- **旧名残留计数**（在方案里）：`pub use run::*` **0**、`pub use build::*` **0**、`src/run/` **0**、`src/build/` **0**、`src/plugins/` **0**、「模块名 `build`/`run`/`plugins`」**0**、`xirang_toolchain::run` **0**、`toolchain/src/run/` **0**（我最初 grep 到的 4 处是 `toolchain/src/runtime…` 的前缀误命中，逐条看过：`:242` 同名嵌套记账、`:359-360` 的 `git mv` 目标、`:583` 的 `git mv` 示例）。唯一保留的旧形态是 `:196`/`:209` 的 E0428 **对照引用**（明写"已消失"）✓。

## 4. K-NAM1 —— **已关闭**

- 报告 `:1421` 现写 `` 类型化 `static_graft_plan!`（`examples/control-button/src/lib.rs:48-54`，宏调用在 `:48`、两个 `cut(...)` 实参在 `:50`/`:52`） ``；我自己读了该文件 **46-56 行**：`xirang_run_method::static_graft_plan!(` 正在 **48 行**，`cut(crate::control::object::button::NODE_ID)` 在 50、`cut(...slider...)` 在 52 —— 配对 token 落在所引区间内 ✓；旧的 `:50`/`:52` 单点配对写法全文 **0 处**。

## 5. 两条非阻塞 —— **都已修**

- `NAM-43` 的 `verifier` = **`t31`** ✓（不再是 `t27,t31`）。
- §1.5 排除项③ 已从"空集"改为**带真实例子的声明**：写明「§7 条目里的文件级定位（`path:1`）……例如 `NAM-01` 的 `studio/studio/app/support.rs:1`，全文共 21 条」。我独立数了：§7（§7.1–§7.10）里以 `:1` 结尾的锚点共 **22 处 / 21 个不同路径**，其中带「（文件级定位）」标注的**正好 21 条**（第 22 处是同一路径的重复出现；唯一未标注的是 §7.9 干净清单里的 `core/src/registry_core/mir/mir.rs:1`，它按后缀唯一解析、且不是命名条目）→ 声明与正文**一致**，且该排除项**不掩盖真缺陷**（K-NAM1 就是在排除项之外被我抓到并已修的）。

## 6. 回归 —— **全部一致**

- **计数五处**：§0 表（3 / 66 / 143 / 212）、§1.4（「CRITICAL 3 / MAJOR 66 / MINOR 143，合计 212」）、§2 明细（§2.1 3 + §2.2.1 66 + §2.3 143）、JSON 逐条、`counts.by_severity` —— 全等；§2 与 JSON 的 id 集合双向相等（3/3、66/66、143/143，无缺无多无重）。
- **`counts` 六组重算全等**：`total` 212；verify 164/31/17；evidence 41/18/136/17；批次 B1 3 / B2 23 / B3 14 / B4 50 / B5 21 / B6 51 / B7 30 / **B8 20** = 212；`by_severity_source` 含 `naming-report` 30 与 **`ambiguity-report` 20**；`mechanism_falsified.ids` 5 条（`LGC-LG-20/33/34`、`NAM-40`、`AMB-06`）与逐条字段相等；`unverified_ids` 17 条相等。
- **批次与 id 前缀映射**：`:127` 的来源片区表含 `AMB`（0/5/15）；§2.3 的 AMB 行都带 `B8`；§8.2 的 `B8-命名歧义与约定冲突 | 20 | 5` 行与 `counts.by_batch` 一致；§3.2 映射表 **212 行 / 212 个不同 id，覆盖全部 finding，无多余**。
- **模板残留 0**（排除 ``` 与行内代码后的 `{…}` 形状里无 `(`/`[`、无纯标识符形状）。
- **正文全量锚点**：我扫出 **555 条**（源文件口径，见开头说明）：文件不存在 **0**、行号越界 **0**、**配对 token 不符 0**；唯二被标 `AMBIGUOUS` 的是同一个已声明排除项① 里的探针包路径（`control/control.rs` +43，两处出现；两个工作树候选 39/32 行不含第 43 行的结论沿用 t24）→ 门禁规则（多候选→跳过）下 **0 violation**，且排除项与 §1.5 的三类声明逐条对应、未掩盖缺陷。
- **HTML**：`https?://` **0**、无外部 `<script src>`/`<link>`；新增第 6 页签 **`amb`（20 条）** 的 id 集合与 severity 与 JSON **逐条一致**；`nam`(30) 与 JSON 一致；12 个 crate × `files/lines/fn/pubfn` 与 `audit-inventory.json` **0 mismatch**；`nam_d3_note` 仍在。
- **随机 10 条 `file`/`line`**（seed=20260928）：`NAM-19`→`studio/studio/app/tests/project.rs:1`、`GTE-G-09`→`tools/xirang-publish:101`、`KRN-C-10`→`core/src/registry_core/mir/model.rs:28`、`NAM-20`→`mcp/index.rs:1`、`AMB-13`→`core/src/registry_core/release/release.rs:9`、`AMB-05`→`core/src/registry_core/tree/registry.rs:18`、`LGC-LG-09`→`build_method/pipeline.rs:155`、`KRN-K-15`→`core/src/registry_core/mir/jsonl.rs:111`、`LGC-LG-25`→`conventions/src/release_workflow.rs:96`、`STU-C-02`→`studio/studio/app/writers.rs:1`：**10/10 命中且行号在界内**。
- **作者闸门**：`check_report.py` 在真实产物上 **exit 0**（`claims checked: 43`、`recomputed: 212 {3,66,143} {164,31,17}`、`OK`）。
- 顺带：`docs/audit-2026-09-28/` 无非法前缀文件；中文比 0.34（与前几轮同口径）。

---

## 7. 判定

**可以交付。**

本轮要求核的六项全部通过：新轴 §7.6 的 20 条 `AMB-*` 与 JSON **双向相等**、复核状态与 t35 判定逐条一致（`AMB-06` 的证伪在 §4.1 有专条、5 条 `unverified` 全是 MINOR 并已标注）、§7.9 干净清单与队列无交集且抽查依据为真；AMB 的严重度不变量在我自己的解析器下 **0 违规**；报告 §7.8 与合并方案的七个模块名**逐名一致**且方案内的并入表/根 glob/E0428 撞名表/feature 与安装命令均已同步（旧名残留 0，仅保留 `:196`/`:209` 的"已消失"对照）；K-NAM1 的配对 token 已落进 `examples/control-button/src/lib.rs:48-54`（我读了那几行）；`NAM-43` 的 verifier 已是 `t31`、§1.5 排除项③ 已补真实例子且我独立复核了它声称的 21 条；回归全绿（212 = 3/66/143、批次含 B8 20、`counts` 六组可由逐条字段重算、模板残留 0、555 条正文锚点 0 violation 且排除项与声明逐条对应、HTML 无外链且新页签与 JSON/inventory 0 mismatch、随机 10 条全过、作者闸门 exit 0）。

签字的边界见开头的「已核范围」：本轮是**终局复扫**，只核新增轴、三处修复与回归；前几轮的实质核验结论沿用未重跑；**技术结论与声明排除 token 的内容不在本报告签字范围内**。
