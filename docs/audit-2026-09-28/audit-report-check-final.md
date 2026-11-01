# 交付前终局复扫（t24）：K4/K5/K6 + 计数回归

对账人：gates-auditor。工作树 `HEAD=cf0c378`；**源码一行未改，未改任何人的产物**（所有篡改实验都在 `/tmp` 的副本上做）。本文件是本轮唯一写入。
判定：**可以交付**（最后一句复述）。

## 已核范围（这份签字的边界）

**本轮复核（终局复扫，只扫三处 + 回归）**
1. **K4**：自己写脚本扫 `audit-report.md` 的未展开模板占位符（排除 ``` 代码块与 `` ` `` 行内代码），并另扫 Python 表达式形状（`{len(`、`{counts`、`{c[`、`{total}`、`{n}`、`str(`）——含代码块在内。另把作者新加的"模板残留"断言用 `/tmp` 副本**弄红**，确认它不是恒真闸门。
2. **K5**：定位 `:572` 那条锚点、解析它并核对配对 token 落在所引行/区间内；**重跑我自己的正文锚点全量扫描**（`/tmp/anchor_check.py`，与 `conventions/src/doc_anchors.rs` 同规则：解析、行号上界、配对 token）；并核 §1.5（`:133`）的自检口径是否已写成「扫描范围 + 排除项」。
3. **K6**：`severity_source` 的类别清单（`audit-report.md:113` 与 `audit-findings.json` 的 `meta.note`）与实际取值分布是否一致；全文与 JSON 是否还有 `inline-heading`。
4. **歧义锚点**：验证队长对 `control/control.rs`（加行号 43）的裁定——工作树里两个同名候选是否真的都不含第 43 行。
5. **回归**：总数/严重度/批次（162 = 3/56/103；3/23/14/50/21/51）；`counts` 六组聚合与逐条字段是否仍全等；`check_report.py` 在真实产物上 exit 0。
6. 顺带重跑三项廉价检查：HTML 外链与 `audit-inventory.json` 比对、目录前缀、语言比例。

**未在本轮复跑（沿用前几轮结论，见 `audit-report-check.md` / `audit-report-check-round2.md` / `audit-report-check-round3.md`）**
- 163 条 `id`/`old_ids` 的出处核验（第一轮：163/163 可溯源）；
- 机械不变量的"修前为红"（第二轮我用自造违规副本复现：exit 1、11 条失败、点名 id 与来源）；
- 我自己的严重度重取（第三轮：覆盖 162/162、0 违规、分层抽样 20 条）；
- §8.2 与 `batch` 字段的逐行比对、§3.2 映射 162/162、§0/§1.4/§2.2/§2.3 的计数一致性（第三轮）；
- "约 84k"口径对账（第三轮列为非阻塞 N2，本轮见 §1.5 已补 `audit-report.md:128`）。

**明确不在签字范围内**
- 任何**技术结论**（缺陷机制是否成立、严重度是否恰当、修复方向是否可行）——那是 6 份 verify 报告 + 总账 + 5 份片区报告的活，本报告只对账「报告 ↔ 证据」；
- `meta.unresolved_quoted_anchors` 声明的 21 个示意/夹具/探针包路径 token 的**内容**（只核它们被正确排除与标注）；
- `/tmp` 下的探针与工具产物、`docs/audit-2026-09-28/` 之外的任何文件。

---

## 1. K4 未展开的模板占位符 —— **已关闭**

- 表头行（`audit-report.md:4`）现为：`` audit-findings.json（162 条结构化明细） `` ✓。
- 我自己的扫描（排除 ``` 代码块与行内代码）：**placeholder 形状命中 0**。
- 补充扫描（**不**排除代码块）：`{len(` / `{counts` / `{c[` / `{total}` / `{n}` / `str(` → **0 命中**（即连引文里的 Rust 格式串也没有被误报，它们的形状是 `{source}|{framework}|…`，属于合法内容）。
- **闸门非恒真（我自己弄红）**：把作者的 `check_report.py` 复制到 `/tmp/vs-final/check_report_copy.py`（只把 `D` 指到我自己的副本树 `/tmp/vs-final/tree`），在副本报告里把 `（162 条结构化明细）` 改回 `（{len(F)} 条结构化明细）` →
  ```text
  FAILURES (1):
    - unexpanded template placeholder in the report: ['{len(']
  exit=1
  ```
  同一份脚本在**未篡改**的副本上 `exit=0`（`claims checked: 37`、`OK`）。→ 新加的这两条断言与严重度不变量一样，是"修前为红"的真闸门。

## 2. K5 那条锚点 + 正文锚点全量扫描 + §1.5 口径 —— **已关闭**

- 锚点现为**区间**：`studio/studio/app/call_tree_queries.rs:74-101`（`audit-report.md:572` 同一段），配对 token `CallTreeMemo` 实测在 `studio/studio/app/call_tree_queries.rs:93`，**落在区间内** ✓。
- 我自己的**全量正文锚点扫描**（`/tmp/anchor_check.py`，规则与 `conventions/src/doc_anchors.rs` 一致）：本文档共 **450 条** `<path>:<line>`（含区间）锚点，结果：
  - 文件不存在 **0**、行号越界 **0**、配对 token 不在所引行/区间 **0**；
  - 余下 **1 条**被我的脚本标为 `AMBIGUOUS`：`control/control.rs`（+43），两个同名候选；**按门禁规则"多于一个候选"是跳过而不是违规**（`conventions/src/doc_anchors.rs` 的 `resolve` 对多解直接放过），且它已在正文就地标注并在 §1.5 的排除项里（见 §4）。因此按门禁口径：**0 violation**。
- §1.5（`audit-report.md:133`）现写明：**扫描范围** = 162 个 `file`/`line` 字段各一条 + 正文每个 `<path>:<line>` 与 `<path>:<line>-<line>`（区间两端都查）；**排除项** = `meta.unresolved_quoted_anchors` 的示意/夹具/工具输出相对路径、`/tmp` 与 `target/` 下的路径、引文里标注为「探针包内相对路径」的 token；并写明**上一版只写「0 violation」而未给范围与排除项**，才使一条区间锚点被收窄成单行后仍自称 0 违规。→ 不再有"自检说 0、实际有 1"的口径缺口。

## 3. K6 `severity_source` 类别清单 —— **已关闭**

- `audit-report.md:113` 现写「取值与实际分布（逐条字段重算，共 162 条）：`declared-table` 110、`ledger-table` 44、`max-of-sources` 3、`verified-override` 5」，并给每个类别一句定义；
- `audit-findings.json` 的 `meta.note` 同一句同一组数字；
- 我从逐条字段重算的分布：`{ledger-table: 44, verified-override: 5, max-of-sources: 3, declared-table: 110}` —— 与两处**逐项相等**；
- `inline-heading` 在 `audit-report.md` 与 `audit-findings.json` 里的出现次数均为 **0** ✓（该类别在 t21 已并入 `declared-table`）。

## 4. 歧义锚点（`control/control.rs` +43）—— **已按队长裁定处理，我核实裁定成立**

- 我亲自数了工作树里两个同名候选：`examples/control-button/src/control/control.rs` = **39 行**、`studio/tests/fixtures/node-editor/src/control/control.rs` = **32 行**（`wc -l`）；两者都**不含第 43 行**。全树再找也没有第三个 `*/control/control.rs`。
- 因此把该 token 硬指到任一候选都会造出**虚构锚点**；作者的做法（就地标注「探针包内相对路径，探针包在 `/tmp/nichprobe/pkg`；工作树里两个同名候选文件分别只有 32/39 行…」+ 列入 §1.5 排除项）是正确的最小处理。**维持队长裁定，不建议改动**。

## 5. 回归 —— **全部不变**

- `check_report.py`（真实产物）：**exit 0**，输出 `rows with no severity source in the reports: 0 []`、`recomputed: 162 {CRITICAL:3, MAJOR:56, MINOR:103} {confirmed:131, partial:21, unverified:10} {runtime:41, read:18, reviewed:93, none:10}`、`batches: {B1:3, B4:50, B6:51, B5:21, B3:14, B2:23}`、`claims checked: 37`、`OK: all counts and report claims agree with the per-row fields (0 violations)`。
- 我从逐条字段重算六组聚合：`total` 162 ✓、`by_severity` **3/56/103** ✓、`by_verify_status` 131/21/10 ✓、`by_batch` **3/23/14/50/21/51** ✓、`by_evidence_kind` 41/18/93/10 ✓、`by_severity_source` 44/110/5/3 ✓ —— 与 `counts` **全等**。
- 顺带三项（本轮重跑）：`audit-structure-map.html` 无 `https?://`、无外部 `<script src>`/`<link>`；内嵌 12 个 crate 的 `files/lines/fn/pubfn` 与 `audit-inventory.json` **0 mismatch**（28 项动作未变）；`docs/audit-2026-09-28/` 无非法前缀文件；中文比 **0.35**（与前几轮同口径）。

---

## 6. 判定

**可以交付。**

本轮的三个修复点（K4 模板残留、K5 区间锚点与自检口径、K6 类别清单）全部关闭，且两条新闸门（严重度不变量、模板残留）都被我用 `/tmp` 副本**弄红**验证过、在真实产物上为绿；计数回归不变（162 = 3/56/103，批次 3/23/14/50/21/51，`counts` 与逐条字段全等）；我自己的 450 条正文锚点全量扫描按门禁规则 0 violation，唯一被标歧义的那条经核实**任何工作树候选都不含第 43 行**，就地标注 + 排除是正确处理。

签字的边界见开头的「已核范围」：本轮只复扫 K4/K5/K6 + 回归 + 三项廉价检查，前几轮的实质核验结论沿用（出处核验、严重度不变量修前为红、我的独立严重度重取、§8.2/§3.2/计数一致性）；**技术结论不在本报告签字范围内**。
