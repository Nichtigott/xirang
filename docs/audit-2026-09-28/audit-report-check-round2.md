# 交付前对账（第二轮 t20）：核 t19 修复后的三份产物

对账人：gates-auditor。工作树 `HEAD=cf0c378`；本轮源码一行未改，唯一写入是本文件。
对象：`audit-report.md`（1289 行）、`audit-findings.json`（162 条明细）、`audit-structure-map.html`；对照上一轮我自己的 `audit-report-check.md`。
纪律：只对账「报告 ↔ 证据」，不重审任何技术结论；所有结论用**我自己写的脚本**重算（未采信 check_report.py，也未把它当断言清单，只当格式参考）。

**判定：不可交付 —— 3 项阻塞**（见 §5）：三条严重度与来源证据相矛盾且无说明、`meta.note` 与数据不一致、两处过时总数 163。
**上一轮 7 项阻塞：6 项已关闭、1 项部分关闭**（B-6）。

---

## 0. 我跑的命令（全部只读，脚本内联在 python3 里）

```sh
python3 <<'PY'   # ① 从逐条字段重算 total/severity/verify/evidence/batch + 与 counts 全等？
PY
python3 <<'PY'   # ② 从五份片区报告的「严重度」列/标题独立重取每条严重度，与 JSON 对比
PY
python3 <<'PY'   # ③ §8.2 表格逐行：声明条数/CRITICAL+MAJOR 列/代表 id 归属 vs JSON 重算
PY
python3 <<'PY'   # ④ §2 三个队列 vs JSON id 集合（双向）+ 重复 id
PY
python3 <<'PY'   # ⑤ 全文扫描旧数字（163/17/12/111/126…）与统计句；随机 10 条 file/line + 10 条正文锚点
PY
python3 <<'PY'   # ⑥ HTML：外链、内嵌数据 vs audit-inventory.json、动作数
PY
```

---

## 1. 上一轮 7 项阻塞（B-1…B-7）

| id | 结论 | 证据 |
| --- | --- | --- |
| B-1 `LGC-LG-21` 静默降级 | **已关闭** | JSON：`severity=MAJOR`、`severity_original=MAJOR`、`batch=B4-MAJOR 逻辑与实现缺陷`、`verify_status=confirmed`；`audit-report.md:382` 的 §2.2 详写行是 `#### LGC-LG-21 · MAJOR · …`；`:832` 的 §3 映射表也是 `MAJOR`；§8.2 的 B4 行（47 条、其中 47 CRITICAL/MAJOR）含它所在批次。与总账 `audit-logic-safety-report.md:34`、复核 `audit-verify-logic.md:53`「维持（症状升级）」一致。 |
| B-2 未复核条数 12 vs 17 | **已关闭** | 逐条字段 `confirmed 131 / partial 21 / unverified 10`（合计 162）；`audit-report.md` 的 §0（`复核覆盖…: unverified 10`）、`:26`、§1.6 表、§9.3、§10.2 标题（`未复核条目（10 条）`）**全部是 10**，并逐条列出 10 个 id。全文只有 1 处出现「17」（`audit-report.md:82`），出现在 §1.4(b) 的**修正记录**表里，明写「`counts.by_verify_status` 写 confirmed 126 / unverified 17，而逐条字段是 confirmed 131 / unverified 10」——是"旧值"的留档，不是断言。 |
| B-3 LG-51/52 与 LH-10 重复 | **已关闭** | `LGC-LH-10` **不再是行**（JSON 无该 id、§2.3 表无该 id）；`LGC-LG-51` 的 `old_ids=['LH-10b','LH-10']`、`LGC-LG-52` 的 `old_ids=['LH-10a','LH-10']`，两条都有真标题与真 `fix_hint`；总数因此 162（§0 合计行写 162）。 |
| B-4 §0 说 MAJOR 有 1 条未复核 | **已关闭** | §0 表 MAJOR 行现写「53 条**全部**有独立复核结论（45 证实 / 8 部分证实、0 未复核）」；我从逐条字段重算 MAJOR 的 verify 分布 = `{confirmed: 45, partial: 8}`，与 `counts.major_by_status` 一致（`{confirmed: 45, partial: 8}`），unverified=0。 |
| B-5 §8.2 批次与 JSON 不符 | **已关闭** | §8.2 改成表格，标题明写「从 `audit-findings.json` 的 `batch` 字段生成」。**脚本逐行比对 6 个批次**：声明条数 = 我按 `batch` 重算的条数（3/23/14/47/21/54，合计 162 ✓）；「其中 CRITICAL/MAJOR」列 = 重算值（3/3/3/47/0/0 ✓）；33 个代表 id 的 `batch` 全部等于所在行（0 例外）。 |
| B-6 `evidence_kind` 三套数字 + `meta.note` | **部分关闭（→ 阻塞 K2）** | `counts.notes.evidence_kind` 已写对（`{runtime, read, reviewed, none}`，= 我从逐条字段重算的 41/18/93/10），§1.6 的 `evidence_kind` 说明也已一致；**但 `meta.note` 没改**，仍写「`verify_status ∈ {confirmed, partial, falsified, unverified}`，`evidence_kind ∈ {runtime, read}`」——数据里没有 `falsified`，evidence_kind 也不是该二值集。同一份 JSON 的 `meta.note` 与 `counts.notes` 互相矛盾。 |
| B-7 占位标题 + `:None` 坏指针 | **已关闭** | 全产物扫描 `':None'` = **0 次**；`title == id` 的行 = **0**；`source_line is None` 的行 = **0**（LG-51 = 319、LG-52 = 466）；`old_id_sources` 无空值。 |

---

## 2. 任务书其余检查项（2–7）

### 2 未复核条数只有一个数字 —— 通过
见 B-2：全文 10，唯一「17」在修正记录里（`:82`，明确标注为旧值）；`10 条` 出现 3 处（§0、§9.3、§10.2 标题）且都指同一集合。

### 3 §8.2 与 `batch` 逐条一致 + `counts` 可由逐条字段重算 —— 通过
- §8.2：见 B-5。**用脚本比对，不是目测**：声明条数与 CRITICAL/MAJOR 列逐行等于我从 JSON 重算的值，代表 id 无一条错批。
- `counts` 重算：`total` 162 ✓；`by_severity` `{CRITICAL:3, MAJOR:53, MINOR:106}` ✓；`by_verify_status` `{confirmed:131, partial:21, unverified:10}` ✓；`by_batch` 六项逐项 ✓；`by_evidence_kind` `{runtime:41, read:18, reviewed:93, none:10}` ✓；`mechanism_falsified.ids` = 逐条带该标记的 3 条 ✓；`unverified_ids` 与逐条集合相等（10 条）✓；`major_by_status` ✓；`by_severity_source` ✓（47/5/43/67 = 162）。**9 组聚合全部与逐条字段全等**。

### 4 `LGC-LG-21` 与 `LGC-LG-20` —— 通过
- `LGC-LG-21`：**MAJOR**（见 B-1）。它没有任何 `severity_note`，因为最终产物里它的严重度**没有被改过**（`severity_original` 也是 MAJOR）——即上一轮那个"MINOR 在队列、MAJOR 在证据"的矛盾已从数据里消失，不需要降级/维持说明；§4 的 `LGC-LG-34` 条目（`audit-report.md:969`）仍指认它是真缺陷。
- `LGC-LG-20`：`mechanism_falsified=true` + `falsified_mechanism` 有值、`severity=MAJOR`、`verify_status=partial`；它与 `LGC-LG-33`、`LGC-LG-34` 一起构成 `counts.mechanism_falsified`（3 条），**没有**「被证伪却按原结论留在队列」的条目（那三条的"被证伪"是原机制，保留的是改写后的结论，§1.4 判定规则一写明了）。

### 5 严重度计数五处一致 —— 通过
| 位置 | CRITICAL | MAJOR | MINOR | 合计 |
| --- | ---: | ---: | ---: | ---: |
| §0 表（`audit-report.md:14-17`） | 3 | 53 | 106 | 162 |
| §1.4（`:76`） | 3 | 53 | 106 | 162 |
| §2 明细（§2.1 = 3、§2.2.1 = 53、§2.3 表 = 106） | 3 | 53 | 106 | 162 |
| JSON 逐条字段（我重算） | 3 | 53 | 106 | 162 |
| `counts.by_severity` | 3 | 53 | 106 | 162 |

### 6 没有引入新的不一致 —— 一处新不一致（K1）
- **§2 与 JSON 的 id 集合双向相等**：CRITICAL 3/3、MAJOR 53/53、MINOR 106/106，`missing=[] extra=[] dups=[]`（脚本逐类比对）✓。
- `evidence_kind` 口径：数据唯一（`{runtime, read, reviewed, none}`），`counts.notes` 已说明；**`meta.note` 仍不符**（K2）。
- `title != id` ✓、`:None` 归零 ✓、`source_line` 无空值 ✓。
- **新不一致（K1）**：我用脚本把五份片区报告的「严重度」列/标题与 JSON 逐条对照（157 条能匹配上），发现 **3 条队列严重度低于其来源片区的声明**，且既无 `severity_note`、也不在 §4/`retracted_or_downgraded`：
  | id | 旧 id | 片区报告（来源） | 独立复核 | JSON 最终 | `severity_source` |
  | --- | --- | --- | --- | --- | --- |
  | `LGC-LG-38` | S6 | `audit-lane-surfaces.md:125`「### S6 · [MAJOR]」 | `audit-verify-surfaces.md:35`「S6 \| MAJOR \| 证实 \| **不变**」 | **MINOR**（B6） | ledger-table |
  | `LGC-LG-43` | BR-3 | `audit-lane-bridges.md:21`「BR-3 \| MAJOR」 | `audit-verify-bridges.md:21`「BR-3 \| MAJOR \| 证实 \| **不变（偏上限的 MAJOR）**」 | **MINOR**（B6） | ledger-table |
  | `LGC-LG-53` | G-06 | `audit-lane-gates.md:72`「### G-06 MAJOR」 | `audit-verify-gates.md:19`「G-06 \| 证实 \| **维持 MAJOR**」 | **MINOR**（B6） | ledger-table |
  这三条的总账行（`audit-logic-safety-report.md:452`、`:457`、`:467`）也只写 MINOR，没有给理由。按报告 §1.4(c) 自己新立的规则「严重度从**源报告的严重度表**重取」（正是这条规则把 `BRG-BR-1/BR-2/BRG-BR-C2` 从 MINOR 改成 MAJOR），以及 t16 指出、§1.4 也已采用的「合并取最高」，这三条应为 MAJOR。
- 顺带确认队长要求核的**系统性修正**：我**独立**从 `audit-lane-bridges.md` 的严重度列重取，`BR-1` = MAJOR（`:19`）、`BR-2` = MAJOR（`:20`）、`BR-C2` = MAJOR（`:45`）——这三条由 MINOR 改 MAJOR **是修正而不是回退**（它们此前是按"标题里没有内联严重度"的默认值记的，与作者自己的表不符）；MAJOR 由 50→53 中另外 1 条是 `LGC-LG-21` 恢复 MAJOR，总数 163→162 来自 B-3 去重，两个变化都成立。

### 7 锚点与 HTML —— 通过
- 随机 10 条 `file`/`line`（`seed=20260928`）：`GTE-G-09`→`tools/xirang-publish:101`、`KRN-C-10`→`core/src/registry_core/mir/model.rs:28`、`LGC-LG-09`→`build_method/pipeline.rs:155`、`KRN-K-15`→`core/src/registry_core/mir/jsonl.rs:111`、`LGC-LG-25`→`conventions/src/release_workflow.rs:96`、`STU-C-02`→`studio/studio/app/writers.rs:1`、`STU-S-06`→`studio/studio/app/search_queries.rs:74`、`GTE-N-1`→`.github/workflows/release.yml:66`、`LGC-LG-52`→`plugin-host/src/process.rs:392`、`SUR-C9`→`build_method/pipeline.rs:105`：**10/10 命中真实文件且行号在界内（0 违规）**。
- 随机 10 条正文锚点（同一 seed，排除 `meta.unresolved_quoted_anchors` 的 21 个声明 token）：`cli/lib.rs:169`、`studio/studio/app/graft.rs:129`、`core/src/registry_core/declaration/declaration.rs:344`、`build_method/manifests.rs:127`、`run_method/authoring/manifest/face/render.rs:79`、`core/src/registry_core/authoring/parse/parse.rs:211`、`studio/studio/app/support.rs:1`、`studio/studio/app/tests/call_tree.rs:25`、`studio/studio/app/search_queries.rs:82`、`studio/studio/ui/graph/nodes.rs:183`：**10/10 OK（0 违规）**。全文共 444 条正文锚点。
- HTML：外部 URL **0** 个（无 `http(s)://`、无外部 `<script src>`/`<link>`）；内嵌数据的 12 个 crate × `files/lines/fn/pubfn` 与 `audit-inventory.json` **0 mismatch**；`actions` 28 项。
- **中文为主**：CJK 与拉丁字母比 0.34（与上一轮同口径；无 CJK 的行全部是 §3 映射表的元数据列与 `file:line` 行）。
- **目录命名**：`docs/audit-2026-09-28/` 内全部 `audit-` 前缀（含本文件 `audit-report-check-round2.md`），无残留非前缀文件。

---

## 3. 阻塞项

### K1 三条严重度与来源证据相矛盾且无说明（`LGC-LG-38` / `LGC-LG-43` / `LGC-LG-53`）
- 现象：来源片区报告声明 MAJOR（`:125` / `:21` / `:72`），独立复核确认 MAJOR（`:35` / `:21` / `:19`，其中两条明写「不变」「维持」），而交付队列是 **MINOR/B6**，`severity_note` 为空、§4 与 `retracted_or_downgraded` 里没有它们，总账的对应行也没写理由。
- 为什么是阻塞：这三条会进「B6 一致性与清扫」而不是 B4/B3——与上一轮 B-1 同一类（严重度被下调而无说明），且**违反报告 §1.4 自己刚立的"从源报告严重度表重取"规则**（同一条规则已用于把 BR-1/2/C2 升为 MAJOR），也违反总账自己"取最高"的规则（§1.4 规则二已为 LG-50 采用）。
- 最小修法：把这三条的 `severity` 改为 MAJOR（`severity_source` 记 `declared-table` 或 `verified-override`、`batch` 随之进 B3/B4），**或**在三处各补一条 `severity_note` 说明"为什么总账的 MINOR 覆盖片区的 MAJOR 且复核的维持"，并把 §1.4 的规则写成"总账行除外"。

### K2 `audit-findings.json` 的 `meta.note` 与同一文件的 `counts.notes`/数据不一致（B-6 未完全关闭）
- 证据：`meta.note` = 「`verify_status ∈ {confirmed, partial, falsified, unverified}`，`evidence_kind ∈ {runtime, read}`」，而逐条字段里 `verify_status` 的取值集是 `{confirmed, partial, unverified}`（无 `falsified`）、`evidence_kind` 是 `{runtime, read, reviewed, none}`；`counts.notes.verify_status` 与 `counts.notes.evidence_kind` 写的才是正确集合。
- 最小修法：把 `meta.note` 的这两句同步成与 `counts.notes` 一致（或直接删掉这两句、指向 `counts.notes`）。

### K3 两处过时总数「163」（应为 162）
- 证据（我全文扫过，只此两处）：`audit-report.md:122`「这两份产物里的全部 **163** 条都跑过脚本自检」、`:713`「**所有** 163 条的映射都在下表」。`id_map` 实测 **162** 项、§3 表按行统计也覆盖 162 条 finding（另有 30 行是 STR- 结构项的映射），§0 合计与 `counts.total` 都是 162。
- 最小修法：两处改成 162（或改成「`len(id_map)` 条」由脚本渲染）。

---

## 4. 非阻塞（建议一并处理）

- **N1** §1.4 规则二写「交付脚本对『最终严重度 ≠ **来源**严重度』的行强制 `severity_note`」，但脚本用的"来源"是 **`severity_original` 字段**（Ledger 行的原值），不是**片区报告的严重度表**——这正是 K1 的根因。建议把断言改成"最终严重度必须等于『片区表 / 总账表 / 复核结论』三者的多数或最高，若取低值则强制 `severity_note`"。
- **N2** `severity_original` 对 162 行**全部非空**（含从未变动的行）。若按 t20 任务书第 6 项字面理解（"`severity_original` 非空的每条都有 `severity_note`"），有 154 行不满足；实际语义显然是"变了才要说明"。建议把该字段改成"仅在与来源不同时填写"，或在字段说明里写明"非空 ≠ 变更"。
- **N3** §3 的映射表标题写「**所有** 163 条的映射都在下表」（K3 同一处），且表内混有 30 行 `STR-` 结构项——建议在表头写明"本表另含 STR- 结构项 N 行"，避免读者以为 finding 数更多。

---

## 5. 判定

**不可交付 —— 3 项阻塞**：

1. **K1** `LGC-LG-38` / `LGC-LG-43` / `LGC-LG-53` 的队列严重度（MINOR/B6）低于其来源片区声明与独立复核结论（MAJOR，两条明写"不变/维持"），既无 `severity_note` 也不在撤销/降级节里，违反报告 §1.4 自己新立的"从源报告严重度表重取"规则。
2. **K2** `audit-findings.json` 的 `meta.note` 仍声明 `verify_status ∈ {…, falsified, …}` 与 `evidence_kind ∈ {runtime, read}`，与同文件的逐条字段及 `counts.notes` 矛盾（上一轮 B-6 只改了一半）。
3. **K3** `audit-report.md:122` 与 `:713` 两处仍写总数 **163**（应为 162）。

其余全部通过：上一轮 7 项阻塞 6 项关闭、1 项部分关闭；严重度计数五处一致（3/53/106=162）；§2 与 JSON 的 id 集合双向相等、无重复；未复核条数全文唯一（10）；§8.2 与 `batch` 脚本比对 0 例外且 `counts` 九组聚合都能由逐条字段重算；`LG-21` = MAJOR/B4、`LG-20` 带 `mechanism_falsified` 且仍 MAJOR；BR-1/BR-2/BR-C2 的 MAJOR 是**修正**（我从 `audit-lane-bridges.md:19/20/45` 独立重取确认）；随机 10 条 file/line 与 10 条正文锚点 0 违规；HTML 无外链、与 inventory 0 mismatch；目录命名合规。
三处都是机器可判、改动量极小的修复（两条字段同步、两处数字、三条严重度或说明）——修完即达成交付条件。
