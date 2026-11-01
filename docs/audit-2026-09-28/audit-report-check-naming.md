# 交付前复扫（t29）：命名专项折入后的报告与 findings

对账人：gates-auditor。工作树 `HEAD=cf0c378`；**源码一行未改、未改任何人的产物**；本文件是本轮唯一写入。
判定：**不可交付 —— 1 项阻塞**（一处配对锚点 token 不在所引行上，一词级修复；其余全部通过）。

## 已核范围（这份签字的边界）

**本轮复核（只扫新增部分 + 回归）**
1. 「命名抽象与可读性」节的 *findings*（§7.1 A/B/C 三张表）与 `audit-findings.json` 的 `NAM-*` **id 集合双向比对**（含重复检测）；每条 NAM 的 `source_report`/`old_id_sources`（文件 + 行）存在性与行号上界。
2. `NAM-*` 的 `verify_status` 与 `audit-naming-verify.md`（t27）逐条判定对照（28 条可解析判定 + 2 条 t27 未涉及者），t27 证伪项的去向，`unverified` 的显式标注与严重度。
3. `NAM-*` 的严重度不变量：**我自己的解析器**从 `audit-naming-review.md`（t25/t30 修订版）与 `audit-naming-verify.md`（t27）重取声明严重度，逐条比对 `severity ≥ max(来源)`、降级是否带 `severity_note`。
4. 计数五处一致（§0 表 / §1.4 / §2.2+§2.2.1+§2.3 / JSON 逐条 / `counts`）+ `counts` 六组聚合重算（含新类别 `naming-report` 与批次 `B7`）。
5. 模板残留扫描；**我自己的正文全量锚点扫描**（条数与 t24 的 450 条对比，识别新增锚点）；§1.5 的扫描范围/排除项**是否被扩大**及扩大是否合理；随机 10 条 `file`/`line`。
6. `audit-structure-map.html`：外链、新视图 `nam` 的数字与 JSON/`audit-inventory.json` 比对。

**未在本轮复跑（沿用 t18→t24 的结论）**：163→192 的历次记账修正历史、严重度不变量的"修前为红"实验、非命名条目的出处核验与独立严重度重取、§8.2 与 `batch` 的逐行比对、§3.2 映射覆盖、语言比例与目录前缀（最后两项本轮顺带复查见 §6）。

**明确不在签字范围内**：任何技术结论（命名建议是否恰当、D-x 落地方案是否可行）；`meta.unresolved_quoted_anchors`（63 个）与 `meta.nam_pattern_tokens`（17 个）声明 token 的**内容**；`/tmp` 探针产物；`docs/audit-2026-09-28/` 之外的文件。

---

## 1. 新增节与 JSON 一致 —— **通过**

- §7.1 三张表（A 文件名与目录名 19 条 / B 函数名与方法 8 条 / C 模块名 3 条）= **30 行、30 个不同 id**；与 JSON 的 30 个 `NAM-*`：**missing=[] extra=[] dups=[]**（双向相等、无重复）。
- 每条 NAM 的出处：`source_report` 全部 = `audit-naming-review.md`；`old_id_sources` 30/30 都带 `(report, line)`，我逐条验证**文件存在、行号 ≤ 文件总行数**（0 问题）。
- 说明一处**设计上的非对称**（不是缺陷）：§7（命名节）里还有 §7.4「查了、干净（命名轴）」，其中的 `NAM-42` 等是**"结论=干净"**条目（`audit-naming-review.md:380`、`audit-naming-verify.md:59`/`:142` 对应），**本来就不该有 JSON 行**——它们不在 JSON、也不在 §2 队列里（§2 的 NAM id 集合 = JSON 的 30 个，见 §4）。因此"节的 id 集合与 JSON 双向相等"这个要求应按「**findings**（§7.1）↔ JSON"读，§7.4 的干净项不参与。

## 2. 复核状态诚实 —— **通过**

- 我从 `audit-naming-verify.md` 的表行解析出 28 条逐条判定，与 JSON 对照（映射：证实→`confirmed`；部分证实/部分证伪→`partial`）：
  - `confirmed` 21 条、`partial` 7 条与判定逐条吻合；唯一"口径差"是 **NAM-36**（t27 写「证实（部分）」、JSON 记 `partial`）——两种读法都站得住，不算不一致；
  - **NAM-40**：t27 判「证伪」、JSON 记 `partial` + `mechanism_falsified=true` + `falsified_mechanism`，且 `audit-report.md` §4.1 有专条（「原结论『394/394 一致、0 违规』被 t27 证伪；改写为 14 处四族不一致」）✓；
  - **NAM-05 / NAM-06**：t27 判「部分证伪 / 部分证实，降级（MAJOR→MINOR）」，JSON 记 `partial` + `severity_note`（note 全文写明"t27 复核后从 MAJOR 降为 MINOR"+ 理由）✓；
  - t27 未涉及的两条（`NAM-20`、`NAM-21`）在 JSON 里都是 **`unverified`**，且在 §2.3 的行里**显式带 `unverified` 字样**、并在 §10.2「未复核条目（12 条）」里逐一列出（`NAM-20`/`NAM-21` 均在）✓；
  - `unverified` 的两条都是 **MINOR**（无 CRITICAL/MAJOR 未复核）✓。
- **NAM-43** 是 t30 修订新增、t27 未见过：t27 报告里 `NAM-43` 出现 **0 次**，`audit-naming-verify2.md`（t31）出现 2 次并写明「新增 **NAM-43…」（见 t31 的 ⑥(b)）。JSON 记 `verify_status=confirmed`、`verifier="t27,t31"` —— 状态有 t31 的依据，但 **verifier 里的 `t27` 是过度归属**（见 §6 非阻塞 N1）。

## 3. 严重度不变量（我自己的脚本）—— **通过**

- 我用自写解析器从两份命名报告重取"声明的严重度"：`audit-naming-review.md` 的 `**NAM-NN / SEV …**` 行（29 条）+ `audit-naming-verify.md` 表的严重度列（26 条，含 `MAJOR → MINOR` 这种取箭头后的目标值）。
- 结果：**0 条 `severity < max(来源)` 且无 `severity_note`**；30 条 NAM 中 29 条能取到来源（`NAM-40` 本身是"干净结论被推翻"，两份来源都没给它严重度 → 我的解析器记为无来源，而 JSON 的 `severity_note` + `mechanism_falsified` 已说明来龙去脉）。
- 抽查（全部与来源相符）：`NAM-01/03/04/30/31` = MAJOR（来源 `audit-naming-review.md:49/95/102/278/284` 均 MAJOR）；`NAM-05/06` = MINOR（来源 `:108`/`:422`，且 note=Y）；其余 MINOR 与来源一致。

## 4. 计数一致 —— **通过**

| 位置 | CRITICAL | MAJOR | MINOR | 合计 |
| --- | ---: | ---: | ---: | ---: |
| §0 表 | 3 | 61 | 128 | 192 |
| §1.4 | 3 | 61 | 128 | 192 |
| §2 明细（§2.1 = 3、§2.2.1 = 61、§2.3 表 = 128） | 3 | 61 | 128 | 192 |
| JSON 逐条（我重算） | 3 | 61 | 128 | 192 |
| `counts.by_severity` | 3 | 61 | 128 | 192 |

- `counts` 六组聚合全部与逐条字段**全等**：`total` 192、`by_severity`、`by_verify_status` 151/29/12、`by_evidence_kind` 41/18/121/12、`by_batch`（B1 3 / B2 23 / B3 14 / B4 50 / B5 21 / B6 51 / **B7 30** = 192）、`by_severity_source`（ledger-table 44 / declared-table 110 / verified-override 5 / max-of-sources 3 / **naming-report 30** = 192）；`mechanism_falsified.ids` = 4 条（含 `NAM-40`）、`unverified_ids` = 12 条，均与逐条字段相等。
- §2 与 JSON 的 id 集合双向相等：CRITICAL 3/3、MAJOR 61/61、MINOR 128/128，`missing=[] extra=[] dups=[]`。

## 5. 锚点与模板 —— **一处阻塞（K-NAM1）**

- **模板残留 0**（我自写扫描：排除 ``` 与行内代码后，`{…}` 形状里没有 `(`/`[` 或纯标识符形状的命中）。
- **正文全量锚点扫描 525 条**（t24 时 450 条；新增 75 条来自命名节与 §7.5）：
  - 文件不存在 **0**、行号越界 **0**；
  - `AMBIGUOUS` 2 次命中同一条：原文写作裸路径 `control/control.rs` 并带行号 43（出现在两处）——§1.5 的排除项 ① 已点名它、正文也标注为 `/tmp` 探针包内相对路径，且我已核过两个工作树候选分别只有 39/32 行、**不含第 43 行**（t24 结论沿用）→ 门禁规则（多候选→跳过）下不算违规 ✓；
  - **配对 token 不符 1 条**（`audit-report.md:1339`）：`` 类型化 `static_graft_plan!`（`examples/control-button/src/lib.rs:50`、`:52`） `` —— 该宏的调用在**第 48 行**（`examples/control-button/src/lib.rs:48` `xirang_run_method::static_graft_plan!(`），50/52 行是它的两个 `cut(...)` 实参。按 `conventions/src/doc_anchors.rs` 的配对规则（token 必须出现在所引行/区间内），这是一处不成立的引用；它与 t22 报的 K5、t23 修掉的 `CallTreeMemo` 属**同一类**（这次是新引入的）。
    - 注：本版 §1.5 的自我声明只承诺"**可解析**"（文件存在 + 行号在界内），因此它**没有**被这句自述直接证伪；但配对写法本身就是"这个 token 在这几行上"的断言，读者按它去核会落空。
    - 最小修法：把该处引用改成 `examples/control-button/src/lib.rs:48`（宏调用所在行）或 `:48-54`（整段调用）；一词级改动。
- **§1.5 的排除项确实被扩大**，我逐类判定：
  - ① 声明 token（`meta.unresolved_quoted_anchors` 现 63 个）+/tmp/target/ + 探针包路径：**合理**（都是非工作树定位；原文那处带行号 43 的裸路径已被我独立核实为"任何工作树候选都不含第 43 行"）。
  - ② **新增**「命名专项里的模式化文件名」（`meta.nam_pattern_tokens` 17 条，如 `<x>/<x>.rs`、`app/<module>.rs`、`*_tests.rs`）：**合理且已声明**——它们是谓词 P1–P8 的输入**模式**，不是路径；而且这类 token 多数没有行号，本来就不会被门禁当锚点。抽查该清单与命名节里的用法一致。
  - ③ **新增**「§8 条目里少数文件级定位（`path:1`）」：**目前是空集**——我在正文里没有找到"指向目录的 `:1` 锚点"这一类形状（我的全量扫描里没有这类失败）；也就是说 ③ 只是预防性声明，既没有隐藏真锚点、也没有可指认的实例。建议要么删掉 ③，要么给出它实际覆盖的那几条 id（否则读者无法核）。
  - **结论**：扩大**没有掩盖本轮那条真缺陷**（我的扫描在排除项之外仍然抓到了 `:1339`），因此不属于"用排除项洗白"。
- **随机 10 条 `file`/`line`**（seed=20260928）：`NAM-19`→`studio/studio/app/tests/project.rs:1`、`GTE-G-09`→`tools/xirang-publish:101`、`KRN-C-10`→`core/src/registry_core/mir/model.rs:28`、`NAM-20`→`mcp/index.rs:1`、`LGC-LG-09`→`build_method/pipeline.rs:155`、`KRN-K-15`→`core/src/registry_core/mir/jsonl.rs:111`、`LGC-LG-25`→`conventions/src/release_workflow.rs:96`、`STU-C-02`→`studio/studio/app/writers.rs:1`、`STU-S-06`→`studio/studio/app/search_queries.rs:74`、`NAM-01`→`studio/studio/app/support.rs:1`：**10/10 命中且行号在界内**。

## 6. HTML —— **通过**

- `audit-structure-map.html`：`https?://` **0** 个、无外部 `<script src>`/`<link>` → 可离线打开 ✓。
- 新视图 **`nam`（30 条）**：id 集合与 JSON 的 30 个 `NAM-*` **相等**，severity 逐条**无 mismatch**；另有 `nam_d3_note` 引号块，内容是 D-3 的三口径（「递归谓词 A1 ∧ 路径不以 `examples/` 或 `studio/tests/` 开头；三口径 = 仓库 34 / 发布面 28 / 豁免…」），与报告 §7.5、JSON 的 NAM-07 叙述一致。
- 回归：内嵌 12 个 crate 的 `files/lines/fn/pubfn` 与 `audit-inventory.json` **0 mismatch**；`actions` 28、`batches` 5 未变。
- 顺带（廉价复查）：`docs/audit-2026-09-28/` 无非法前缀文件；中文比与前几轮同口径。

---

## 7. 非阻塞

- **N1（t29 新发现）** `NAM-43` 的 `verifier` 写作 `"t27,t31"`，但 `audit-naming-verify.md`（t27）**全篇 0 次**提到 `NAM-43`（该条是 t30 修订新增、由 `audit-naming-verify2.md`（t31）在 ⑥(b) 判定）。建议改成 `"t31"`（或 `"t30,t31"`）。
- **N2** §1.5 的排除项 ③（§8 的 `path:1` 文件级定位）在本轮正文里找不到实例（见 §5）；建议删掉或补上它实际覆盖的 id。
- **N3** 沿用 t18→t24 的观察：`unverified` 条目的"未被复核"信息在 §2.3 与 §10.2 都标了 ✓；`NAM-20`/`NAM-21` 的标题里也自带 `unverified`，无需再改。

---

## 8. 判定

**不可交付 —— 1 项阻塞**：

1. **K-NAM1** `audit-report.md:1339` 的配对写法 `` static_graft_plan!（`examples/control-button/src/lib.rs:50`、`:52`） `` 中，token 不在所引行上（宏调用在 `:48`，50/52 是它的实参行）。这是 t22-K5/t23 已修过的同一类问题的新实例；按 `conventions/src/doc_anchors.rs` 的配对规则不成立。最小修法：改成 `:48` 或 `:48-54`。

**其余全部通过**：§7.1 的 30 个 `NAM-*` 与 JSON 双向相等且出处（文件 + 行）30/30 有效；`verify_status` 与 t27 判定逐条一致（`NAM-40` 的证伪有 §4.1 专条 + `mechanism_falsified`，`NAM-05/06` 的降级有 `severity_note`），`unverified` 只 2 条且都是 MINOR 并在 §2.3/§10.2 显式标注；我用自写解析器从两份命名报告重取的严重度不变量 **0 违规**；计数五处一致（192 = 3/61/128，批次 B1 3 / B2 23 / B3 14 / B4 50 / B5 21 / B6 51 / B7 30），`counts` 六组聚合可由逐条字段重算；模板残留 0；正文 525 条锚点除这一条配对失败外 0 违规（唯一歧义项经核实为已声明排除的探针包路径）；§1.5 的排除项扩大**合理且没有掩盖该缺陷**（③ 目前是空集，建议删或补例）；随机 10 条 `file`/`line` 10/10；HTML 无外链、新视图 `nam` 与 JSON 0 mismatch、crate 表与 inventory 0 mismatch。
