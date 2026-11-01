# 交付前对账（第三轮 t22）：核 t21 修复后的三份产物

对账人：gates-auditor。工作树 `HEAD=cf0c378`；源码一行未改，唯一写入是本文件。
对象：`audit-report.md`（190,496 字节）、`audit-findings.json`（162 条）、`audit-structure-map.html`；对照前两轮我自己的 `audit-report-check.md` / `audit-report-check-round2.md`。
纪律：只对账「报告 ↔ 证据」；不重审技术结论；不改任何人的产物（所有违规实验都在 `/tmp` 的副本上做）。
**判定：不可交付 —— 3 项阻塞**（§5），全部是**一词级**修复；实质内容已全部核过。

---

## 0. 我跑的命令

```sh
python3 /tmp/xirang-audit-logs/check_report.py                       # 真实产物：exit 0
python3 /tmp/xirang-audit-logs/check_report.py /tmp/vs-round3/violating-findings.json   # 我造的违规副本：exit 1
python3 /tmp/vs-round3/my_sev_check.py                                 # 我自己的严重度重取（全 162 条）
python3 <<'PY'  # 六组计数重算 / §2↔JSON id 集合 / §3.2 覆盖 / meta.note 取值集 / 全文 163 扫描
PY
python3 /tmp/anchor_check.py                    # audit-report.md 全量 444 条正文锚点（含配对 token 规则）
python3 <<'PY'  # 随机 10 条 file/line + HTML 外链与 inventory 比对 + 语言比例
PY
```

---

## 1. K1 / K2 / K3 逐项

### K1 三条静默降级 —— **已关闭**
| id | severity | batch | severity_source | severity_note |
| --- | --- | --- | --- | --- |
| `LGC-LG-38`（=S6） | **MAJOR** | B4-MAJOR 逻辑与实现缺陷 | `max-of-sources` | 「严重度取源报告的较高者：lane 声明 MAJOR（`audit-lane-surfaces.md:125`）」 |
| `LGC-LG-43`（=BR-3） | **MAJOR** | B4 | `max-of-sources` | 同上（`audit-lane-bridges.md:21`） |
| `LGC-LG-53`（=G-06） | **MAJOR** | B4 | `max-of-sources` | 同上（`audit-lane-gates.md:72`） |

连带记账**逐项重算一致**（我从逐条字段重算的 `rec` 与 `counts` 全等）：
- 总数 **162**；CRITICAL **3** / MAJOR **56** / MINOR **103**（与 t21 声明、与船长期待一致）；
- 批次 B1 3 / B2 23 / B3 14 / **B4 50** / B5 21 / **B6 51**（合 162）；
- §0 表（`:14-17`）= 3/56/103/162、MAJOR 行「56 条全部有独立复核结论（48 证实 / 8 部分证实 / 0 未复核）」与我重算的 `major_by_status={confirmed:48, partial:8}` 相等；
- §1.4（`:76`）「CRITICAL 3 / MAJOR 56 / MINOR 103，合计 162」；§2.2（`:180`）= 56、§2.2.1（`:182`）= 56、§2.3（`:632`）= 103；
- §2 与 JSON 的 id 集合**双向相等**：CRITICAL 3/3、MAJOR 56/56、MINOR 103/103，`missing=[] extra=[] dups=[]`；
- §2.1+§2.2.1 详情块 **59 = 3 + 56**；§3.2 映射表 **162 行**、覆盖全部 162 个 finding id（无缺、无多）；
- §8.2 六个批次表：条数与 CRITICAL/MAJOR 列再次与 `batch` 字段重算相等（3/23/14/50/21/51）。

### K2 `meta.note` 取值集 —— **已关闭（余一处类别冗余，见 §5 K6）**
`audit-findings.json` 的 `meta.note` 现写：`verify_status ∈ {confirmed, partial, unverified}`（**不存在 falsified**）、`mechanism_falsified + falsified_mechanism` 是独立标记且这类条目 `verify_status=partial`、`evidence_kind ∈ {runtime, read, reviewed, none}`。
- 逐条字段实测：`verify_status` 取值集 = `{confirmed, partial, unverified}` ✓；`evidence_kind` = `{none, read, reviewed, runtime}` ✓；`mechanism_falsified` 3 条（`LGC-LG-20/33/34`）且全部是 `partial` ✓；全文没有 `verify_status=falsified` 的行 ✓；
- 与同一文件 `counts.notes` 的两句一致 ✓。

### K3 全文 `163` —— **已关闭**
- `audit-report.md:132` 现写「这两份产物里的全部 **162** 条都跑过脚本自检」；
- **全文扫描只剩一处 `163`**：`audit-report.md:571`，它是引文里的行号 `studio/studio/ui/search/detail.rs:15/163`（锚点行号，不是条数）✓；
- 但见 §5 K4：同一行为的渲染在 `:4` 留了一个未展开的表达式。

---

## 2. 机械不变量「修前为红」（我没重跑队长的实验，另造了一份违规副本）

- 真实产物：`python3 /tmp/xirang-audit-logs/check_report.py` → **exit 0**，输出 `rows with no severity source in the reports: 0 []`、`recomputed: 162 {CRITICAL:3, MAJOR:56, MINOR:103}…`、`claims checked: 37`、`OK: all counts and report claims agree`。
- 我的违规副本（`/tmp/vs-round3/violating-findings.json`，把 `KRN-C-01` 由 MAJOR 改成 MINOR 并清掉 `severity_note`）→ **exit 1，11 条失败**，其中不变量那一条逐字点名：
  ```text
  severity below every source it comes from: KRN-C-01 is MINOR but audit-lane-kernel.md:280 (KRN)
  declares MAJOR; all sources: MAJOR@audit-lane-kernel.md:280(KRN), MAJOR@audit-verify-kernel.md:35(V-KRN)
  -- downgrade needs severity_note
  ```
  → 打印了违规 id、两处/多处来源的 `file:line` 与各自 lane，以及修复提示。**闸门不是恒真的**：同一脚本在真实产物上 exit 0、在违规副本上 exit 1，且 `rows with no severity source = 0` 说明它真的查到了来源（不是空集合上的真命题）。
- 与队长独立复现的结论一致（他用 `LGC-LG-04`，11 条失败；我用 `KRN-C-01`，同样 11 条）✓。

---

## 3. 我自己的严重度重取（队长指定的重点，独立于作者脚本）

我另写了一份解析器（`/tmp/vs-round3/my_sev_check.py`）：**按 (文件, id) 建索引**（避免 kernel/studio 都有 `C-xx` 时跨车道串号），从 13 份原始报告里按我自己的规则取"声明的严重度"（表行取 id 后第一个纯严重度格；标题行取 `### <id> … <SEV>`；verdict 格里带「维持/不变/下调/上调/升为/降为」的严重度也算），再把每条的 `old_id_sources`（含对应 lane 的 verify 报告）与 `ledger_id` 三处合起来取最高。

- 覆盖率：**162/162** 条都能在我的索引里找到来源（`rows with no source found by MY parser: 0`）；
- 全量结论：**0 条 `severity < max(三处来源)` 且无 `severity_note`**；
- 唯二"低于我算出的最高者"且**有**说明的行（允许）：`LGC-LG-33`（我算 MAJOR@`audit-lane-kernel.md:37`，行 MINOR，note = 机制证伪）、`STU-S-04`（我算 MAJOR@`audit-lane-studio.md:47`，行 MINOR，note = t11 建议下调）；0 条"高于来源"的行；
- 分层抽样 **20 条**（覆盖四类 `severity_source`；`max-of-sources` 仅 3 条故全取）：

| id | 行严重度 | 我算出的最高来源 | 来源文件:行 | `severity_source` | note |
| --- | --- | --- | --- | --- | --- |
| `LGC-LG-52` | MINOR | MINOR | `audit-logic-safety-report.md:465` | ledger-table | — |
| `LGC-LG-42` | MINOR | MINOR | `audit-lane-surfaces.md:276` | ledger-table | — |
| `LGC-LG-20` | MAJOR | MAJOR | `audit-logic-hunt.md:262` | ledger-table | — |
| `LGC-LG-03` | CRITICAL | CRITICAL | `audit-lane-studio.md:22` | ledger-table | — |
| `LGC-LG-15` | MAJOR | MAJOR | `audit-lane-surfaces.md:62` | ledger-table | — |
| `LGC-LG-08` | MAJOR | MAJOR | `audit-lane-surfaces.md:13` | ledger-table | — |
| `LGC-LG-34` | MINOR | MINOR | `audit-logic-safety-report.md:47` | verified-override | 有 |
| `LGC-LG-33` | MINOR | MAJOR | `audit-lane-kernel.md:37` | verified-override | 有（机制证伪） |
| `LGC-LG-50` | MAJOR | MAJOR | `audit-lane-studio.md:71` | verified-override | 有 |
| `LGC-LG-05` | MAJOR | MAJOR | `audit-lane-surfaces.md:306` | verified-override | 有 |
| `LGC-LG-16` | MAJOR | MAJOR | `audit-lane-surfaces.md:150` | verified-override | — |
| `LGC-LG-53` | MAJOR | MAJOR | `audit-lane-gates.md:72` | max-of-sources | 有 |
| `LGC-LG-38` | MAJOR | MAJOR | `audit-lane-surfaces.md:125` | max-of-sources | 有 |
| `LGC-LG-43` | MAJOR | MAJOR | `audit-lane-bridges.md:21` | max-of-sources | 有 |
| `KRN-K-16` | MINOR | MINOR | `audit-lane-kernel.md:204` | declared-table | — |
| `SUR-C9` | MINOR | MINOR | `audit-lane-surfaces.md:388` | declared-table | — |
| `GTE-G-05` | MINOR | MINOR | `audit-lane-gates.md:60` | declared-table | — |
| `GTE-G-09` | MINOR | MINOR | `audit-lane-gates.md:100` | declared-table | — |
| `STU-S-20` | MINOR | MINOR | `audit-lane-studio.md:164` | declared-table | — |
| `GTE-G-03` | MINOR | MINOR | `audit-lane-gates.md:44` | declared-table | — |

`severity_source` 的四类取值我也重算过分布：`ledger-table` 44 / `declared-table` 110 / `verified-override` 5 / `max-of-sources` 3 = 162，与 `counts.by_severity_source` 全等。（队长给的 `47/43/67/5` 是 t19 版的口径；t21 已把 `inline-heading` 并入 `declared-table`，并新增 `max-of-sources`。）

---

## 4. 六组计数 + id 集合 + 锚点 + HTML + 语言

- **六组聚合**（我从逐条字段重算 vs `counts`）：`total` 162 ✓、`by_severity` 3/56/103 ✓、`by_verify_status` 131/21/10 ✓、`by_batch` 3/23/14/50/21/51 ✓、`by_evidence_kind` 41/18/93/10 ✓、`by_severity_source` 44/110/5/3 ✓；`mechanism_falsified.ids` = 逐条 3 条 ✓；`unverified_ids` = 逐条 10 条 ✓；`major_by_status` = 48/8 ✓。
- **§2 ↔ JSON id 集合**：三类双向相等、无重复（见 §1 K1）。
- **随机 10 条 `file`/`line`**（seed=20260928）：`GTE-G-09`→`tools/xirang-publish:101`、`KRN-C-10`→`core/src/registry_core/mir/model.rs:28`、`LGC-LG-09`→`build_method/pipeline.rs:155`、`KRN-K-15`→`core/src/registry_core/mir/jsonl.rs:111`、`LGC-LG-25`→`conventions/src/release_workflow.rs:96`、`STU-C-02`→`studio/studio/app/writers.rs:1`、`STU-S-06`→`studio/studio/app/search_queries.rs:74`、`GTE-N-1`→`.github/workflows/release.yml:66`、`LGC-LG-52`→`plugin-host/src/process.rs:392`、`SUR-C9`→`build_method/pipeline.rs:105`：**10/10 命中且行号在界内**。
- **正文锚点全量**（不止随机 10 条）：444 条 `.rs:<line>` 我全扫了一遍——文件不存在 0、行号越界 0；**但配对 token 规则下有 1 处不符、另有 1 处同名歧义**，见 §5 K5 与 §6 N1。
- **HTML**：`https?://` 计数 **0**（无外链、无外部 `<script src>`/`<link>`）；内嵌 12 个 crate 的 `files/lines/fn/pubfn` 与 `audit-inventory.json` **0 mismatch**。
- **目录命名**：`docs/audit-2026-09-28/` 内全部 `audit-` 前缀（含本文件），无残留。
- **中文为主**：CJK/拉丁比 **0.35**，与上两轮同口径。

---

## 5. 阻塞项（全部是一词级修复）

### K4 `audit-report.md:4` 留着一个未展开的渲染表达式 `{len(F)}`
- 现象：`> **本报告的三个产物**：…`audit-findings.json`（**{len(F)}** 条结构化明细）…`——读者看到的是 Python 表达式而不是数字。全文只此一处（JSON 里 0 处）；同一段的其它统计都已渲染成 162。
- 最小修法：把 `{len(F)}` 写成 `162`（或让它与 §0 的合计同源渲染）。

### K5 `audit-report.md:572` 的配对 token 与所引行不符，且与 §1.5 的自检声明冲突
- 现象：`…它和 `CallTreeMemo`（`studio/studio/app/call_tree_queries.rs:74` 注释明确写了…）是同一类问题…`——按 `conventions/src/doc_anchors.rs` 的配对规则（token 必须出现在所引行区间内），`CallTreeMemo` 出现在该文件 **第 93 行**，而引用点的是 **74**；74 行附近是关于备忘的注释、没有这个标识符。
- 冲突点：`audit-report.md:133`（§1.5）自称「全文扫描锚点时请排除这 21 个 token 与上述两个引文行」，§9.3（`audit-report.md:1286`）与 t21 的交付说明写「逐条自检 0 violation」。我全量扫 444 条后，按同一条规则有 **1 条**不符（就是这一条）。原始片区报告写的是 `studio/studio/app/call_tree_queries.rs:74-101`（`audit-lane-studio.md:57`），区间内确实含第 93 行；是汇总时把区间收窄成单行造成的。
- 最小修法：把该引用改回 `:74-101`（或 `:93`），或在 §1.5/§9.3 的声明里写明这一条例外；改完「0 违规」才成立。

### K6 `inline-heading` 仍作为一个 `severity_source` 类别被声明，而数据里已没有这种取值
- 现象：`audit-report.md:113` 与 `audit-findings.json:156`（`meta.note`）都写「`severity_source` 记录…（`ledger-table` / `declared-table` / **`inline-heading`** / `verified-override`）」；实测取值分布是 `ledger-table` 44 / `declared-table` 110 / `verified-override` 5 / `max-of-sources` 3，**没有任何 `inline-heading`**；而 `max-of-sources`（3 条，正是 K1 修正引入的新类别）**没有**出现在这两处的列表里。
- 最小修法：把这两处的类别列表改成实际取值集（把 `inline-heading` 换成 `max-of-sources`，或写成「历史类别 `inline-heading` 已并入 `declared-table`」）。

---

## 6. 非阻塞

- **N1** `audit-report.md` 正文里有一个同名歧义锚点（原文写作裸路径 `control/control.rs` 加行号 43，完整相对路径应是下面两条之一）（`examples/control-button/src/control/control.rs` 与 `studio/tests/fixtures/node-editor/src/control/control.rs` 两条候选）。按 `conventions/src/doc_anchors.rs` 的规则，多于一个候选时门禁**跳过**（所以它不算违规、也不会让 `cargo test` 变红），但读者据此无法确定是哪一个文件；建议写成完整相对路径。
- **N2** 我抽样的 20 条与全量 162 条的独立重取见 §3 —— 除 `LGC-LG-33`（机制证伪，已有 note）外没有"低于来源"的条目，也没有"高于来源"的条目；`severity_note` 强制规则在数据上成立。
- **N3** 上一轮我报的 `LGC-LG-38/43/53` 现在不仅升为 MAJOR，还各自带了「取来源较高者」的 note；建议下一轮把这条规则写进 `§1.4` 的判定规则三，避免"取最高"只体现在数据里。

---

## 7. 判定

**不可交付 —— 3 项阻塞**：

1. **K4** `audit-report.md:4` 留着未展开的 `{len(F)}`（渲染残留，读者看到的是表达式）。
2. **K5** `audit-report.md:572` 的配对写法 `` `CallTreeMemo`（`studio/studio/app/call_tree_queries.rs:74`） `` 里的 token 不在所引行上（token 在第 93 行），与报告 §1.5/§9.3 的「正文编号锚点 0 违规」声明冲突（我全量 444 条扫出这 1 条）。
3. **K6** `severity_source` 的类别清单在 `audit-report.md:113` 与 `audit-findings.json:156` 里仍含 `inline-heading`（数据里 0 条），且漏了实际的 `max-of-sources`（3 条）。

**其余全部通过**：K1 三条已为 MAJOR/B4、连带记账（162 = 3/56/103、批次 3/23/14/50/21/51、§0/§1.4/§2.2/§2.3/§8.2）逐项重算一致；K2 的 `meta.note` 取值集与逐条字段一致（`falsified` 已不存在、`mechanism_falsified` 是独立标记）；K3 全文 `163` 只剩一处锚点行号；六组计数与 `counts` 全等；§2 ↔ JSON 三类 id 集合双向相等无重复；§3.2 映射 162/162、详情 59 = 3+56；机械不变量**实测修前为红**（我的违规副本 exit 1、11 条失败、点名 id 与来源）、真实产物 exit 0 且非恒真；我自己的严重度重取覆盖 162/162、0 违规；随机 10 条 `file`/`line` 全过；HTML 无外链且与 inventory 0 mismatch；目录命名合规；中文为主。
三项阻塞都是**一词级**修改（一处数字、一处行号、一处类别词），改完无需重跑整轮对账——我可以在同一 attempt 上复扫这三处并给出最终判定。
