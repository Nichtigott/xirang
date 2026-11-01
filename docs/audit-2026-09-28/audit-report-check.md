# 交付前对账（t18）：`audit-report.md` / `audit-findings.json` vs 各路原始证据

对账人：gates-auditor。工作树 `HEAD=cf0c378` + 未提交改动；本轮源码一行未改，唯一写入是本文件。
纪律：**不复核任何技术结论**（那是 10 份复核报告的活）；本文件只对账「报告 ↔ 证据」的一致性。
判定：见文末 §6。**结论是「不可交付」**，7 项阻塞全部是机械可修的对账问题，不动任何 finding 的技术内容。

---

## 0. 我实际比对的文件与命令

比对对象（我自己逐个打开并计数，不采信任何人的自检）：

| 文件 | 行数 | 我用它回答哪个问题 |
| --- | ---: | --- |
| `docs/audit-2026-09-28/audit-report.md` | 1204 | 第 1/2/3/4/5/6/8/9/10 项 |
| `docs/audit-2026-09-28/audit-findings.json` | 8509（163 行明细 + `counts`/`id_map`/`retracted_or_downgraded`…） | 第 1/3/4/5/9/10 项的主数据 |
| `docs/audit-2026-09-28/audit-structure-map.html` | 185 | 第 7 项 |
| `docs/audit-2026-09-28/audit-inventory.json` | 67741 | 第 7/9 项的比对基准 |
| `audit-lane-{kernel,surfaces,studio,bridges,gates}.md` | 386/431/355/630/394 | 第 1 项出处核验 |
| `audit-verify-{kernel,surfaces,studio,bridges,gates,logic}.md` | 316/284/204/240/220/582 | 第 2/3/5 项 |
| `audit-logic-safety-report.md` / `audit-logic-hunt.md` / `audit-structure-base.md` / `audit-boundary-refactor-plan.md` / `audit-inventory.md` | 618/398/501/686/783 | 第 2/3/9 项 |

主要命令（全部只读；脚本用 python3 内联，未写工作区）：

```sh
python3 - <<'PY'   # 计数：severity / verify_status / evidence_kind / lane / batch，逐条与 §2 明细对 id
PY
python3 - <<'PY'   # 出处核验：每条的 id/old_ids 是否在其 source_report 原文里出现
PY
python3 - <<'PY'   # 严格锚点：163 条的 file/line；正文随机 10 条 .rs:NNN（seed=20260928）
PY
python3 - <<'PY'   # HTML：解出内嵌 JSON，与 audit-inventory.json 逐 crate 逐字段比对
PY
```

---

## 1. 逐项结论（任务书 10 项 + 队长 3 项）

| # | 检查项 | 结论 |
| --- | --- | --- |
| 1 | 凭空出现的条目 | **通过**：163 条的 `id`/`old_ids` **全部**能在其 `source_report` 原文里找到（0 条无出处）。 |
| 2 | 已被证伪却仍在队列 | **通过（队长点名的 5 项逐一核过）**：V-03→`LGC-LG-34`（§4）、幻影依赖（§4 明写**撤销**）、K-03→`LGC-LG-33`（§4）、`STU-S-04`（§4 降 MINOR）、`STU-S-07`（§4 明写**维持 MAJOR**，§2.2.1 也确实按 MAJOR 列）。无「§4 撤销、§2 仍以原严重度保留」的情形。**但** `LGC-LG-20` 的 `verify_status=falsified` 与它仍在 §2.2 作 MAJOR 行动项并存（见 §3-N5，非阻塞）。 |
| 3 | 严重度被上调/下调而无说明 | **不通过 → 阻塞 B-1**：`LGC-LG-21` 在总账（`audit-logic-safety-report.md:34`、`:318`）是 **MAJOR**、独立复核（`audit-verify-logic.md:53`）写「**维持（症状升级）**」，而在 `audit-findings.json` 里 `severity=MINOR`、`severity_original=null`，§2.3 的 MINOR 表（`audit-report.md:596`）与 §3 映射表（`:762`）都按 MINOR 列，§4 没有任何降级说明。 |
| 4 | 严重度计数与明细一致 | **通过**：`§0` 表（3/48/112=163）、`§1.4` 声明、§2 明细条目数（§2.1 = 3、§2.2.1 = 48、§2.3 表 = 112）、`audit-findings.json` 的行计数与 `counts.by_severity` **五处完全一致**；三张表与 JSON 的 id 集合**双向相等**（无缺、无多、无重）。**唯一缺陷是两处小节标题的数字过时**（§2.2 写 49、§2.3 写 111，见 §3-N1），明细本身正确。 |
| 5 | 「代码阅读佐证」被写成「已实测」 | **通过**：48 条 MAJOR + 3 条 CRITICAL 的「复核状态」行逐条与 JSON 的 `evidence_kind`/`verify_status` 对照，**0 条**把 `read`/`none` 写成「实测/探针」；反方向也没有（`runtime` 的没被写成只读码）。 |
| 6 | 「中文为主」 | **通过**：无 CJK 且含 ≥8 个英文词的 24 行**全部落在 §3 的 id 映射表（行 642–847）**，是元数据列（旧 id、出处、严重度），不是散文；正文散文逐段中文。 |
| 7 | HTML 数字可追溯 + 离线自包含 | **通过**：`audit-structure-map.html` 无任何 `http(s)://`、无 `<link>`/外部 `<script src>`（`<style>`/`<script>`/内嵌 `application/json` 数据块全内联）；内嵌 12 个 crate 的 `files/lines/fn/pubfn/src_lines/test_lines` 与 `audit-inventory.json` **逐项相等（0 mismatch）**，合计 387/78,727 = inventory 的成员合计；28 项动作与方案的 28 个 `R-01…R-28` **一一对应**（无一方多/少）。 |
| 8 | 目录内非 `audit-` 前缀文件 | **通过**：`docs/audit-2026-09-28/` 现有 20 个文件，全部 `audit-` 前缀，无残留（若残留，`conventions/src/doc_blocks.rs:133` 的 `is_record` 不豁免、`cargo test --workspace` 会红）。 |
| 9 | 关键数字可追溯 + 四处口径差 | **部分通过**：394/78,951 可追溯到 `audit-structure-base.md`（4 处）与 `audit-inventory.md`（1 处）、报告自己也写了 3 处；387/78,727 可追溯到 `audit-inventory.md`（2 处）+ 报告 1 处；121 vs 120 在 §1.2（`audit-report.md:51-57`）显式说明三口径；严重度计数在 §1.4（`:69-82`）显式说明。**「约 84k」这一口径在总报告里没有对账**（只在 `audit-structure-base.md:22` 与 `:497` 提出，且那两处明确点名「请 t17 报告对账时以本条口径为准」）→ §3-N2。 |
| 10 | 随机 10 条 `.rs:NNN` 锚点 | **通过（我自己跑的脚本）**：163 条的 `file`/`line` 字段全部命中真实文件且 `1 ≤ line ≤ 总行数`（0 违规）；正文随机 10 条（`seed=20260928`）全部命中且无歧义：`studio/studio/ui/forms/project.rs:47`、`run_method/authoring/operations/face_write.rs:85`、`studio/studio/app/writers.rs:1`、`studio/studio/app/tests/call_tree.rs:25`、`core/src/registry_core/release/release.rs:259`、`run_method/macros/face.rs:139`、`build_method/contracts.rs:193`、`core/src/registry_core/declaration/declaration.rs:344`、`core/src/registry_core/tree/graft_ops/overlay.rs:111`、`core/src/registry_core/tree/graft_ops/resolution.rs:22`。 |
| 队长 1 | 计数独立重数 | 见第 4 项：**严重度计数五处一致**（3/48/112/163）；**但 `verify_status` 与 `evidence_kind` 的计数有三套数字互不相同** → 阻塞 B-2/B-3/B-6。 |
| 队长 2 | 被证伪项只出现在「撤销/降级」 | 见第 2 项：点名的 5 项全部合规（`LGC-LG-50` 按总账「取最高」维持 MAJOR，§4 与 §2.2.1 一致）。 |
| 队长 3 | `unverified` 不许冒充「已确认」 | **通过**：12 条 `unverified` 在 §2.3 表里**每条都带 `unverified` 字样**（我逐行核过）；CRITICAL/MAJOR 的 `unverified` = **0**，作者「无 CRITICAL/MAJOR 未复核」的声明成立。**但**未复核的**条数**自相矛盾 → 阻塞 B-2。 |

---

## 2. 阻塞项（必须改后才可交付）

### B-1（最重）`LGC-LG-21` 被静默从 MAJOR 降为 MINOR，行动队列与批次随之错位

- 证据（我逐个文件读的行）：
  - `audit-logic-safety-report.md:34`：「`LG-21` | **MAJOR** | `apply edit` 静默忽略 `handle_contracts`/`part_contracts`…」
  - `audit-logic-safety-report.md:318`：「### LG-21 **MAJOR** — …」
  - `audit-verify-logic.md:53`：「`LG-21` | MAJOR | **证实**（症状比报告更重：traits/contracts 互相矛盾） | … | **维持**（症状升级）」
  - 而 `audit-findings.json` 的该条：`severity=MINOR`、`severity_original=null`、`batch=B6-MINOR 一致性与清扫`
  - `audit-report.md:596` 把它排在 **MINOR 表**；`:762` 的 §3 映射表标 MINOR；`:1121` 的 §8.2 又把它列进「**B3（14 条桥与宿主）**」——同一 id 在交付物里同时是 MINOR/B6 与 B3。
  - §4（`audit-report.md:848-899`）与 `retracted_or_downgraded`（7 条）**都没有** LG-21 的降级说明；§4 反而在 `LGC-LG-34` 条里把它称作「真缺陷…（MAJOR）」（`:861`）。
- 必须修：在 `audit-findings.json` 把 `LGC-LG-21` 的 `severity` 改回 `MAJOR`（`severity_original` 填 `MAJOR`、`batch` 改 B3/B4 与 §8.2 对齐），**或**在 §4 补一条「LG-21 降 MINOR + 理由」并把 §4/§8.2 里称它为 MAJOR 的措辞改一致。二者选一，不能两头都在。

### B-2 未复核条数：交付物给出 12 与 17 两个数

- `audit-report.md:25`：「confirmed 126、falsified 1、partial 19、**unverified 17**」
- `audit-report.md:26`：紧接着写「**12 条未获独立复核**（其中 CRITICAL/MAJOR 0 条）」
- `audit-report.md:97`（§1.6）：「`unverified` | 没有任何独立复核（**17 条**）」
- `audit-report.md:1170`（§9.3）：「没有对 **17 条** `unverified` 条目做独立复核」
- `audit-report.md:1180`（§10.2 标题）：「未复核条目（**17 条**）」，但该标题下的表**只有 12 行**——我逐条与 JSON 比对，这 12 行与 JSON 里 `verify_status=unverified` 的 12 条**完全相等**。
- `audit-findings.json` 的 `counts.by_verify_status` 也写 `unverified: 17`（`confirmed: 126`），而它自己的明细行是 `confirmed 131 / partial 19 / falsified 1 / unverified 12`。
- 必须修：以明细为准（**131 / 19 / 1 / 12**），改 `audit-report.md:25`、`:97`、`:1170`、`:1180` 与 `audit-findings.json` 的 `counts.by_verify_status`；`:26` 已经是 12，不动。

### B-3 同一对缺陷被算了两次：`LGC-LH-10` 与 `LGC-LG-51`+`LGC-LG-52`

- `audit-report.md:1178`（§10.1）：「**`LGC-LG-51+52`**（MINOR）：成员 `LGC-LH-10`。…拆得比原报告更细，予以采纳；**LH-10 作为组不再单独成行**。」
- 但 `LGC-LH-10` **仍是独立行**：`audit-findings.json` 里它有 `severity=MINOR`、`old_ids=["LH-10","LG-51+52"]`、evidence 里同时含 a（`plugin-host/src/process.rs:392` 坏帧上下文）与 b（`mcp/mir.rs:217` 存在性 oracle）两个子项；`audit-report.md:615` 的 §2.3 表里也有它。
- 而 `LGC-LG-51`（= 子项 b）与 `LGC-LG-52`（= 子项 a）同样是独立行（`:612`、`:613`）。于是 MINOR 的 112 里至少有 1 行是作者自己宣布「不再单独成行」的组行。
- 必须修：删掉 `LGC-LH-10` 行（MINOR 112→111、合计 163→162），**或**撤回 §10.1 那句话并把 LH-10 保留为组行、同时删掉 LG-51/52 两行（同样 111）。无论哪种，§10.2、`counts`、§2.3 表、`id_map` 都要同步重算。

### B-4 `§0` 说 MAJOR 有 1 条「未获独立复核」，与明细和 §10.2 自相矛盾

- `audit-report.md:15`：「**MAJOR** | **48** | 47 条有独立复核（其中 7 条部分证实、细节被修正），**1 条未获独立复核（一律标 unverified）**」
- JSON 明细的 MAJOR：`confirmed 40 / partial 7 / falsified 1 / unverified **0**`（我逐条数的）。
- `audit-report.md:1197` 自己写：「清单里**没有任何 CRITICAL 或 MAJOR**（唯一的 MAJOR 级别未复核项 `GTE-N-1` 已由 t13 夹具实测，归为 `confirmed`）」——`GTE-N-1` 在 JSON 里确实是 `confirmed`。所以 §0 那句的「1 条」应是 `falsified` 的 `LGC-LG-20` 被误写成了 unverified。
- 必须修：改成「48 条全部有复核结论（40 证实 / 7 部分证实 / 1 条被证伪，见 §4）」。

### B-5 §8.2 的批次列表与 JSON 的 `batch` 字段矛盾（并自称「与 §2 一致」）

- `audit-report.md:1117`（§8.2 标题）：「修复批次（**与 §2 的建议批次一致**）」
- `audit-report.md:1121`：「**B3（14 条桥与宿主）**：`LGC-LG-21`、`LGC-LG-19`/`LGC-LG-20`、`LGC-LG-22`、`LGC-LG-23` 为主」
- JSON 的 `batch`：`LGC-LG-19` → B4、`LGC-LG-20` → B4、`LGC-LG-21` → B6、`LGC-LG-22` → B6、`LGC-LG-23` → B5。五个被点名的 id **没有一个**是 B3。
- 必须修：§8.2 的批次列表改为从 `audit-findings.json` 的 `batch` 字段生成（或反过来改 JSON），并删掉「与 §2 一致」这句在改完前不成立的表述。

### B-6 `evidence_kind` 的计数有三套数字，且 `meta.note` 与数据不符

- `audit-report.md:99`（§1.6）：`runtime` **29**、`read` **21**、`reviewed` **96**
- `audit-findings.json` 的 `counts.by_evidence_kind`：`runtime` **39**、`read` **15**、`reviewed` **93**、`none` **16**
- JSON 明细行实际：`runtime` **41**、`read` **18**、`reviewed` **93**、`null` **11**（合计 163）
- `audit-findings.json` 的 `meta.note` 写「`evidence_kind` ∈ {runtime, read}」，而数据里有 `reviewed` 与 `null` 两种额外取值。
- 必须修：以明细为准（41/18/93/11）重算 §1.6 与 `counts.by_evidence_kind`，并把 `meta.note` 的取值集改为 `{runtime, read, reviewed, null}`（或说明 null = 未复核、无手段）。

### B-7 两行只有占位标题，且指针是坏的

- `LGC-LG-51` 的 `title` 就是 `"LG-51"`、`LGC-LG-52` 的 `title` 就是 `"LG-52"`；`audit-report.md:612`、`:613` 的「一句话」列同样只有 `LG-51` / `LG-52`。维护者在行动队列里看到两行无法判断要修什么（内容只在 `evidence` 字段里）。
- 同两行的 `fix_hint` 是「见出处 `audit-logic-safety-report.md:**None** 的「最小修复方向」段」——`None` 是空 `source_line` 渲染出来的字面量；另有 3 行（`LGC-LG-34`、`KRN-C-08`、`STU-S-10`）也带这个字面 `None`（合计 5 条坏指针）。
- 必须修：给 LP-51/52 补一句标题（从 evidence 首句取），并把 `fix_hint` 的 `None` 渲染成「该条无独立行号，见 §10.1」之类。

---

## 3. 非阻塞（建议同批改）

- **N1** 小节标题数字过时：`audit-report.md:135` 写「MAJOR（**49** 条）」而其明细 `:137` 是 48；`:523` 写「MINOR（**111** 条）」而其表是 112 行。严重度总数本身正确（§0/§1.4/明细三处一致）。
- **N2** 「约 84k 行」口径未对账：任务书给的 ~84k 与本轮实测 78,951 差约 6%，`audit-structure-base.md:22`、`:497` 两处**点名要 t17 统一口径**，但 `audit-report.md` 全文没有出现 84k（§1.1 只写了两套实测口径）。建议在 §1.1 加一句「任务书的 ~84k 系含 md/toml 或其它提交的口径，本轮以 78,951（全树）/78,727（成员）为准」。
- **N3** `counts.by_source_lane`（合计 235）与明细 `lane`（合计 163）不是同一口径：§1.4 正文已显式说明「一行可属多个片区…故合计大于 163」，**说明到位**；但 JSON 的该字段本身没有携带这条口径，容易被下一个读者误当成 163。建议改名（如 `by_source_lane_overlapping`）或加 `note`。
- **N4** `ledger_rollup` 标记缺失：§1.3（`:65`）与 §10.1（`:1176`、`:1177`）都承诺「保留各自的 lane 行，**并在该行标 `ledger_rollup`**」，而 17 行成员（`KRN-K-10/K-17`、`SUR-S11`、`GTE-G-03/04/05/07/08/17/18/19/20/22/24/25/26/27`）**没有一行带这个字符串**（该串在 JSON 里只出现 2 次，都是别的行的 `fix_hint` 文字）。链路本身可由 `old_ids` 恢复（每行都带 `LG-37`/`LG-54`），所以只是措辞与数据不一致。
- **N5** `LGC-LG-20` 的 `verify_status=falsified` 却仍是 §2.2 的 MAJOR 行动项：§4 只在「另外三处数字更正」里提它的**机制**被证伪（`:894-897`），没有单开条目。建议把该行改成 `partial`（finding 保留、机制更正，与 t16 §3 的表述一致）或为它单开一条 §4。
- **N6** 9 行 `source_line: null`（`LGC-LG-21`、`LGC-LG-34`、`LGC-LG-51`、`LGC-LG-52`、`STU-V-01`、`STU-V-02`、`GTE-N-1`、`GTE-N-2`、`GTE-N-3`）：这些行在 §3 的「旧 id ← 出处」列里显示成 `…:None`。不阻塞（`file`/`line` 字段都在），但交付物里出现 `:None` 不好看。

---

## 4. 我**没有**做的事（划界）

- 不复核任何技术结论（CRITICAL/MAJOR 的机制、严重度是否恰当、修复方向是否可行）——那是 6 份 verify 报告 + 总账的活；本文件只在**报告与证据不一致**时出手。
- 不评价 finding 的取舍（哪些该进队列、哪些该合并为一条）——只报告「作者自己声明的规则/数字」与「数据」的冲突。
- 未逐个打开 163 条引用的原始段落逐字比对措辞（只核了 id/old_ids 的出处与 `file`/`line` 的可解析性）；§2 的「现象与判据」是引文，按作者给 t18 的读法，引文里的示意图与夹具名（`meta.unresolved_quoted_anchors` 的 20 个 token）不算锚点。

---

## 5. 交付物另一面的核对（一并记录）

- `audit-report.md` 的 §2 三项**内容完整性**通过：§2.1 的 3 条 CRITICAL、§2.2.1 的 48 条 MAJOR、§2.3 表的 112 条 MINOR 与 `audit-findings.json` 的 id 集合**双向相等**，无缺、无多、无重复 id；48 条 MAJOR 的标题行都标着 `MAJOR`。
- §1.3 声明的「总账覆盖缺口已补回」经核对成立：`STU-S-02/03/05/06`（MAJOR）与 `STU-S-04`（MINOR，§4 已说明降级）都在队列；`BRG-BR-9/11/12/15/17` 都在 §2.3。
- 三条 CRITICAL 的 `verify_status` 都是 `confirmed`、`evidence_kind` 都是 `runtime`，与 §2.1 标题「全部已实测」一致。

---

## 6. 判定

**不可交付（7 项阻塞）**：

1. **B-1** `LGC-LG-21` 被从已验证的 MAJOR 静默降为 MINOR（`severity_original=null`、无 §4 说明、批次还分列 B3/B6）——直接影响下一轮排期。
2. **B-2** 未复核条数在同一份交付物里有 **12** 与 **17** 两个数（`audit-report.md:25/97/1170/1180` 与 `audit-findings.json` 的 `counts` 写 17；`:26` 与明细写 12）。
3. **B-3** `LGC-LH-10` 与 `LGC-LG-51`+`LGC-LG-52` 覆盖同一对缺陷，且 §10.1 明写「LH-10 不再单独成行」而它仍在队列 → MINOR 计数含 1 行冗余。
4. **B-4** `§0` 的「MAJOR 有 1 条未获独立复核」与明细（MAJOR `unverified=0`）及 §10.2 末句矛盾。
5. **B-5** §8.2 的 B3 列表点名 5 个 id，其 JSON `batch` 无一为 B3，而该段自称「与 §2 的建议批次一致」。
6. **B-6** `evidence_kind` 计数三套数字（29/21/96 vs 39/15/93/16 vs 41/18/93/11），且 `meta.note` 声明的取值集与数据不符。
7. **B-7** `LGC-LG-51`/`LGC-LG-52` 两行只有占位标题，且 5 条 `fix_hint` 含字面 `":None"` 坏指针。

修完以上 7 项（全部是重算数字、补一句说明、改一处措辞或删一行重复）后，**其余 9 项检查全部通过**，包括：163 条全部可溯源、严重度计数五处一致（3/48/112=163）、已证伪项处理合规、无「只读码冒充实测」、中文为主、HTML 自包含且与 inventory 数字零差异、目录无非法前缀文件、163 条 `file`/`line` 与随机 10 条正文锚点全部可解析。
