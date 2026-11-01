# xirang 结构 / 质量 / 逻辑审计 · 总报告

> **审计轮次**：2026-09-28 结构+质量+逻辑审计（8 名成员、18 个任务）。**本轮只出报告，源码一行未改**；真正动目录与清单是下一轮、逐条批准后执行。
> **本报告的三个产物**：`audit-report.md`（本文件）、`audit-findings.json`（212 条结构化明细）、`audit-structure-map.html`（自包含交互对照图，可离线打开）。
> **输入**：五份片区报告（`audit-lane-kernel.md`、`audit-lane-surfaces.md`、`audit-lane-studio.md`、`audit-lane-bridges.md`、`audit-lane-gates.md`）+ 六份独立复核（`audit-verify-kernel.md`、`audit-verify-surfaces.md`、`audit-verify-studio.md`、`audit-verify-bridges.md`、`audit-verify-gates.md`、`audit-verify-logic.md`）+ 逻辑总账（`audit-logic-safety-report.md`）+ 对抗扫荡（`audit-logic-hunt.md`）+ 结构基线（`audit-structure-base.md`）+ 结构重构方案（`audit-boundary-refactor-plan.md`）+ 机械清点（`audit-inventory.md` / `.json`）。
> 工作树 `HEAD=cf0c378`，未提交改动 **121 项**（实测 `git status --porcelain`；任务书写 120；见 §1.2）。

## 0. 结论摘要

**这一轮没有发现「发布不可用」级的问题，也没有发现安全问题被利用的痕迹；但发现了 3 条会在正常使用中静默产出错误结果或直接杀死进程的缺陷，以及 66 条 MAJOR。**

| 严重度 | 条数 | 说明 |
| --- | ---: | --- |
| **CRITICAL** | **3** | 全部已实测复现，且由队长亲手复核（见 §2.1） |
| **MAJOR** | **66** | 66 条**全部**有独立复核结论（57 证实 / 9 部分证实、细节被修正 / 0 未复核） |
| **MINOR** | **143** | 一致性、注释/自述、错误上下文、清扫类 |
| **合计** | **212** | 去重后（原始各路共 194 个发现块；与 `audit-findings.json` 的 `counts` 同源，脚本从逐条字段重算） |

三条 CRITICAL：

- **LGC-LG-01**（LH-01）`build_method/node_id.rs:36` — 进程级身份缓存的键不含命名空间，跨包污染 `NodeId`
- **LGC-LG-02**（K-01）`core/src/registry_core/authoring/parse/admission.rs:40` — 创作面「读一次再写回」静默丢弃 admission 的 deny 列表，门禁被放宽且不可逆
- **LGC-LG-03**（S-01）`studio/studio/app/mutations.rs:257` — Studio 插件写盘路径能产出解析器拒绝的锁，界面报「已选中」

复核覆盖（`verify_status`，逐条字段重算）：confirmed 164、partial 31、unverified 17。
**17 条未获独立复核**（`AMB-12`、`AMB-13`、`AMB-14`、`AMB-16`、`AMB-18`、`BRG-BR-C7`、`KRN-C-09`、`KRN-K-21`、`LGC-LG-51`、`LGC-LG-52`、`LGC-LH-09`、`LGC-LH-11`、`NAM-20`、`NAM-21`、`STU-C-09`、`STU-S-20`、`STU-S-21`），全部是 MINOR；本报告一律标 `unverified`，没有任何一条未经复核的 CRITICAL/MAJOR 被写成「已确认」。

复核手段（`evidence_kind`，取值集 `{runtime, read, reviewed, none}`）：none 17、read 18、reviewed 136、runtime 41。
另有 **5 条 `mechanism_falsified`**（`LGC-LG-20`、`LGC-LG-33`、`LGC-LG-34`、`NAM-40`、`AMB-06`）：被证伪的是**原机制**，保留下来的缺陷是改写后的结论，因此它们的 `verify_status` 记为 `partial` 而不是 `falsified`——**没有被证伪的结论留在队列里**（判定规则见 §1.4，逐条说明见 §4）。

严重度相对来源发生变动的条目 **3** 条，每条都带 `severity_note`（脚本强制，见 §1.4）：`LGC-LG-33` MAJOR→MINOR；`LGC-LG-50` MINOR→MAJOR；`STU-S-04` MAJOR→MINOR。

建议批次（本报告的行动队列顺序）：

| 批次 | 条数 | 内容 |
| --- | ---: | --- |
| B1-立即：安全与数据完整性 | 3 | 先把 3 条 CRITICAL 关掉（进程内身份污染、admission deny 丢失、Studio 写出解析器拒绝的锁） |
| B2-门禁与工具 | 23 | 门禁的判定改为结构化输入（自述 vs 判定不符的整类问题） |
| B3-桥与宿主 | 14 | mcp/cli/plugin-host 的边界与可靠性 |
| B4-MAJOR 逻辑与实现缺陷 | 50 | 执行面/内核/Studio 的 MAJOR（含结构批 4 的「唯一来源」收口） |
| B5-注释与自述 | 21 | 注释/文档承诺与实现不符（含文档漂移） |
| B6-MINOR 一致性与清扫 | 51 | 其余 MINOR |
| B7-命名抽象与可读性 | 30 | 命名专项（文件名/目录名/模块名/函数名）：改名、命名共识与棘轮 |
| B8-命名歧义与约定冲突 | 20 | 命名歧义轴：生态保留名 / 跨生态 / 同族易混 / 本仓一名多义 |

## 1. 方法与口径（先把数字定死，再谈结论）

> **冻结声明**：本报告主体（§0–§11）描述的是**冻结在 17:12 的树**——其中的计数、行号与结论都以那棵树为准。此后就地落地的修复（含受影响锚点的**现行**行号、体量与枚举行的变化）记在 **§12「修复轮 delta」**；`audit-findings.json` 的 `fix_round` 块是同一份账的机器可读形式。

### 1.1 代码规模：两个口径并存，都标出处

| 口径 | 文件 / 行数 | 出处 |
| --- | --- | --- |
| **全工作树（含 `target/` 外的一切 `.rs`，排除 `studio/tests/fixtures/node-editor`）** | 394 文件 / 78,951 行 | `audit-structure-base.md`（boundary-architect 实测，本轮采信为代码规模基准）；与 `audit-inventory.md` 的 394/78,951 一致 |
| **工作区成员（12 个 crate 单元，不含非成员夹具包 node-editor）** | 387 文件 / 78,727 行 | `audit-inventory.md` §2（executor 的机械清点）；差的 7 文件 / 224 行是 `studio/tests/fixtures/node-editor`（自声明 `[workspace]`，非成员） |
| 发布面（9 个发布 crate 的包内 `.rs`） | 见 `audit-package`-系门禁与 `tools/xirang-package-audit` | `audit-lane-gates.md` §5 |

两个口径**都正确**，差别只在「把非成员夹具包算不算工作树」。引用时必须带上口径名，本报告此后写「394/78,951」时指全工作树口径。

### 1.2 未提交改动数：三个数字并存

- 任务书写的 **120**；
- 我（executor）与 boundary-architect 实测的 **121**（`git status --porcelain` 行数：`M` 101 + `??` 20，审计开始时；`??` 含本轮报告文件所在目录，故随报告数量变化）；
- 各片区报告里出现的 122，是**已经写了 1–2 份报告之后**在同一命令上取得的（报告文件自己也算改动）。

结论：**口径是「`git status --porcelain` 的行数，含未跟踪条目」**；数字漂移只由报告文件自身造成，不是源码变动。

### 1.3 去重与 id 规则（本轮硬要求）

- 每条 finding 的 id 都带 lane 前缀：`KRN-`（内核）、`SUR-`（执行面）、`STU-`（Studio）、`BRG-`（桥与宿主）、`GTE-`（门禁与工具）、`LGC-`（逻辑/安全总账）、`STR-`（结构）、`DEC-`（待决策）。
- **旧 id 一律保留可见**：`old_ids` 字段 + §3 的映射表（新 id ← 旧 id ← 出处文件与行）。注意上一轮索引 `docs/audit-3p-2026-09-27.md` 里的 `G-01/G-02` 与本轮 `GTE-G-01/GTE-G-02` **不是同一件事**（那两条是 purity 门禁，本轮这两条是 lint 假阴性 / release_workflow 的 tag 守卫），前缀把两轮隔开了。
- **去重规则（可复算）**：
  1. 逻辑/安全总账（`LGC-LG-nn`）已经就某一缺陷写明了来源（如 `= S19 + K-22`）时，本报告把它作为**一行**，id 用 `LGC-LG-nn`，被合并的旧 id 进入 `old_ids`；
  2. 总账把**修复单元不同**的多条压成一行时（`LG-37` 三条文本启发式解析、`LG-54` 十四条门禁问题），不合并：保留各自的 lane 行，并在该行标 `ledger_rollup`，总账那行进 §11 附录；
  3. 总账未编号的片区发现（Studio 的实现轴、桥的 MINOR、内核的注释轴等）保留自己的 lane id。
- **总账的覆盖缺口已补回**：t16 实测 t15 的总账漏掉了 Studio 的 5 条 MAJOR（`STU-S-02…S-06`，grep 计数为 0）与 t5 的 5 条 MINOR（`BRG-BR-9/11/12/15/17`），它们不在总账的任何表里。本报告以**五份片区报告 + 六份复核+ 总账**三份来源的并集为输入，这 10 条都在队列里（`STU-S-02…S-06` 见 §2.2，`BRG-BR-9/11/12/15/17` 见 §2.3）。

### 1.4 计数对账：单一来源、修正记录与判定规则

**（a）总账自己的摘要与总表不一致。** `audit-logic-safety-report.md` 的交付摘要写「**23 MAJOR**」，而它自己的 §0 总表列了 **29 条 MAJOR**。t16 已指出这点。本报告的计数**以去重后的重数为准**。按逐条字段重算：**CRITICAL 3 / MAJOR 66 / MINOR 143，合计 212**（`counts` 与 §2 的明细条数、§2.1 详写与 §2.2/§2.3 表逐条对应）。

**（b）本次交付前修正的记账缺陷（t19）。** 上一版有两个数字问题，都以本文件与 `audit-findings.json` 的**逐条字段**为准修掉了：

| 缺陷 | 表现 | 修正 |
| --- | --- | --- |
| 聚合块与明细不同步 | `counts.by_verify_status` 写 confirmed 126 / unverified 17，而逐条字段是 confirmed 164 / unverified 17 | `counts` 改为**在所有加工步骤之后**从逐条字段重算（脚本保证顺序），报告正文所有引用这些数字的句子（§0、§1.6、§10、§11.2）全部改为从 `counts` 渲染，不再手写 |
| `falsified` 语义不清 | `LGC-LG-20` 的 `verify_status=falsified` 却仍以 MAJOR 留在队列，看起来像「已被证伪的结论还在队列里」 | 新增 `mechanism_falsified` + `falsified_mechanism` 字段：被证伪的是**原机制**，保留下来的缺陷是**改写后的结论**；这类条目的 `verify_status` 记为 `partial`。`falsified` 不再出现在取值集里 |

**判定规则一：`mechanism_falsified`。** 共 5 条（`LGC-LG-20`、`LGC-LG-33`、`LGC-LG-34`、`NAM-40`、`AMB-06`），判定条件是「复核明确证伪了原报告给的机制，同时保留（或改写）出一个可复现的缺陷」；它们的 `verify_status` 是 `partial`（保留部分已复核），`falsified_mechanism` 写明被证伪的是什么、保留下来的结论是什么（见 §4）。因此 `verify_status` 的取值集是 `{confirmed, partial, unverified}`——**不存在「被证伪却仍以原结论留队列」的条目**。

**判定规则二：严重度变动必须有说明。** 交付脚本对「最终严重度 ≠ 来源严重度」的行强制要求 `severity_note`，缺一条就让构建失败。当前有 **3 条**变动，逐条说明：

| id | 来源严重度 | 最终 | 说明 |
| --- | --- | --- | --- |
| `LGC-LG-33` | MAJOR | **MINOR** | 机制证伪（t9）：两者同判 Err，真问题是错误文案指错原因 |
| `LGC-LG-50` | MINOR | **MAJOR** | 总账把 STU-S-07（MAJOR）与 STU-S-24（MINOR）合并成 LG-50 时按 MINOR 落账且未写理由；t16 指出按总账自己的「取最高」规则应留 MAJOR，故本条维持 MAJOR |
| `STU-S-04` | MAJOR | **MINOR** | t11 部分证实：事实全部成立、严重度建议下调 MINOR（渲染期改写 App 的状态污染） |

**判定规则二之机械不变量（t21 加入，针对同一类错误第二次出现）。** 交付脚本与检查脚本都对**每一条**断言 `severity ≥ max(来源片区报告, 总账, 独立复核)`，除非该条带 `severity_note` 说明降级理由；违规时检查脚本会打印 id、三处来源值与各自的 `file:line` 并让构建失败（`check_report.py`，已用一份故意违规的 findings 副本实测会失败）。本轮据此把三条总账静默降级的条目**升回来源严重度**：

| id | 旧值 | 现值 | 来源依据 |
| --- | --- | --- | --- |
| `LGC-LG-38` | MINOR | **MAJOR** | audit-lane-surfaces.md:125 (lane) |
| `LGC-LG-43` | MINOR | **MAJOR** | audit-lane-bridges.md:21 (lane) |
| `LGC-LG-53` | MINOR | **MAJOR** | audit-lane-gates.md:72 (lane) |

（这三条在 `audit-logic-safety-report.md` 的总账里被记为 MINOR 且未写理由，而来源片区报告与独立复核都是 MAJOR：`audit-lane-surfaces.md:125` + `audit-verify-surfaces.md:35`（LGC-LG-38 = `S6`）；`audit-lane-bridges.md:21` + `audit-verify-bridges.md:21`（LGC-LG-43 = `BR-3`）；`audit-lane-gates.md:72` + `audit-verify-gates.md:19`（LGC-LG-53 = `G-06`）。报告 §1.4 的这条不变量就是为了不再靠人工复查。）

另外有 **3 条严重度是**从源报告的严重度表重取**的修正**（不是我判定的升降级）：bridges 片区的条目在标题里没有内联严重度，早期实现按默认值记为 MINOR，与作者自己的表（`audit-lane-bridges.md` 的「严重度」列）不符。修正记录：

| id | 旧 id | 修正为 | 依据 |
| --- | --- | --- | --- |
| `BRG-BR-1` | BR-1 | **MAJOR** | `audit-lane-bridges.md:19`（作者自己的严重度表） |
| `BRG-BR-2` | BR-2 | **MAJOR** | `audit-lane-bridges.md:20`（作者自己的严重度表） |
| `BRG-BR-C2` | BR-C2 | **MAJOR** | `audit-lane-bridges.md:45`（作者自己的严重度表） |

`severity_source` 字段记录每条的严重度取自哪里，取值与实际分布（逐条字段重算，共 212 条）：`ambiguity-report` 20、`declared-table` 110、`ledger-table` 44、`max-of-sources` 3、`naming-report` 30、`verified-override` 5。定义：`max-of-sources` = 由三处来源的较高者抬升；`verified-override` = 带 `severity_note` 的复核判定；`ledger-table` = 取自总账总表；`declared-table` = 取自片区/复核报告的严重度列；`naming-report` = 取自命名专项报告（含其复核结论）；`verifier-section` = 复核报告给结论但不在表里（如 t13 的 §3）。完整分布见 `audit-findings.json` 的 `counts.by_severity_source`，可逐条核对。

**判定规则三：同一缺陷只算一条。** 总账把同一对缺陷拆成两条时按「更细的一侧」保留：`LGC-LH-10`（组）与 `LGC-LG-51`+`LGC-LG-52` 覆盖同一对缺陷，**保留 LG-51/LG-52 两条、LH-10 不再单独成行**（它的 id 作为别名并入这两条）。同理 `LGC-LG-37` 与 `LGC-LG-54` 是 rollup，成员各自成行并在 `ledger_rollup` 字段标注（见 §11.1）。

**（b-2）命名抽象专项已并入（t28）。** 专项报告（`audit-naming-review.md`，t25/t30 修订；`audit-naming-verify.md` t27 + `audit-naming-verify2.md` t31 复核）新增 **30** 条 `NAM-` 条目（严重度 MAJOR 5、MINOR 25，批次 `B7-命名抽象与可读性`），总数由 162 变为 192。它们的 `severity_source` 记为 `naming-report`；`NAM-05`/`NAM-06` 是复核后的降级（各带 `severity_note`），`NAM-40` 的原「394/394 一致」结论被 t27 证伪后改写（`mechanism_falsified`）。细节见 §8。

**（b-3）命名歧义轴已并入（t37）。** 歧义专项（`audit-naming-ambiguity.md`，t34/t39/t41 修订；`audit-naming-ambiguity-verify.md` t35 + `-verify2.md` t40 + `-verify3.md` t42 复核）新增 **20** 条 `AMB-` 条目（严重度 MAJOR 5、MINOR 15，批次 `B8-命名歧义与约定冲突`），总数由 192 变为 **212**。**放置选择**：并进 §8（标题改为「命名：抽象与可读性 / 歧义与约定冲突」，新增 §8.6–§7.10），而不是单开一节——两轴的主题都是「名字」，只是判据不同（角色谓词 D-1…D-11 vs 四问 + 歧义共识 R-1…R-4）；同节相邻读者不必跳页，两轴**交叉引用、不重复计数**（歧义报告正文标 `= D-n` 的不计入歧义轴）。`severity_source` 记为 `ambiguity-report`；`AMB-06` 的机制被 t35 部分证伪后改写（`mechanism_falsified`）。**V42-01 记账**：源报告 §8 的改法表/落地顺序仍把已登记豁免的 `app`/`application` 写成「至少一个加限定词」；本报告按**复核后的验收词**表述——该对已登记豁免、**不需要改名**——并在此记明源报告那两处文字仍在（见 §7.10）。

**（c）来源片区分布的口径。** 合并行会跨片区，因此按片区计数之和（284）**大于**总条数（212）——这是「一行属多片区」的口径，不是重复计数：

| 来源片区 | CRITICAL | MAJOR | MINOR |
| --- | ---: | ---: | ---: |
| AMB | 0 | 5 | 15 |
| BRG | 0 | 12 | 15 |
| GTE | 0 | 7 | 20 |
| KRN | 1 | 15 | 27 |
| LGC | 3 | 33 | 35 |
| NAM | 0 | 5 | 25 |
| STU | 1 | 10 | 28 |
| SUR | 0 | 14 | 13 |

**（d）任务书的「约 84k 行」对账。** `audit-structure-base.md:22` 与 `:497` 点名要本报告统一口径。本工作树实测 `.rs` 为 **78,951 行**（排除 `target/`，见 §1.1）；`git ls-files '*.rs'` 为 76,599 行 / 380 文件（工作树另有未跟踪 `.rs`）；把仓库**所有被跟踪文本文件**相加是 98,516 行。「84k」与本报告的 78,951 相差约 6%，最可能来自口径（是否含 markdown/toml，或数在别的提交上）；**本报告一律用 78,951（工作树 `.rs`）与 78,727（工作区成员）两个口径**，不再出现第三个数字。

### 1.5 锚点纪律与自检

- **权威定位只有一处**：每个条目的**标题行** `file:line`，也就是 `audit-findings.json` 的 `file`/`line` 字段。**自检的扫描范围**：这 212 个字段各一条（断言「文件存在 + `line ≤ 文件总行数`」），外加正文里每一个形如 `<path>:<line>` 与 `<path>:<line>-<line>` 的锚点（区间两端都查）。**排除项（三类）**：① `meta.unresolved_quoted_anchors` 列出的示意/夹具/工具输出相对路径（含 `control/control.rs:43` 这类探针包内路径）、`/tmp` 与 `target/`；② **命名专项里的模式化文件名**——`<x>/<x>.rs`、`<dir>.rs`、`*_tests.rs`、`search*.rs`、`app/<module>.rs` 等是谓词形状（谓词 P1–P8 的输入模式），不是工作树路径，完整清单见 `meta.nam_pattern_tokens`（§8 的条目与 §7.2 规则表会用到它们）；③ §7 条目里的**文件级定位**（`path:1`）——该条目针对的是整个文件或目录族（例如 `NAM-01` 的 `studio/studio/app/support.rs:1`），全文共 21 条，它们在 §7.1 的「旧名」列里带「（文件级定位）」标注；这不是缺陷，而是这类命名条目没有单一函数行可指。以上都不是对工作树的定位；除此以外的每一个 `<path>:<line>`（含区间两端）都必须可解析。上一版此处只写「0 violation」而未给扫描范围与排除项，使一条区间锚点被收窄成单行后仍自称 0 违规；本版把范围与排除项写出来，并按区间重新扫描。
- **引文与定位的分界（复核脚本请按此读）**：§2 每个条目的「**现象与判据**」「**最小修复方向**」两行是对原报告的**引用**（保留原文措辞便于对账），引用里可能残留裸名锚点；所有**可解析**的引文锚点已机械改写为工作区相对路径，剩下 77 个 token 是原报告的最小反例路径、夹具路径或方案里**尚未存在**的新增文件名（如 `src/alpha/alpha.rs`、`myrepo/src/foo/foo.rs`、`generated_lib.rs`、`geometry.rs`），在真实工作树里本就不存在——完整清单在 `audit-findings.json` 的 `meta.unresolved_quoted_anchors`。因此全文扫描锚点时请排除这 77 个 token 与上述两个引文行。
- 每条 finding 的定位取自**片区报告自己的定位**（`位置`/`file:line` 段或标题），不是我从代码里新推的；若片区报告的锚点无法解析（绝对路径缺失、行号越界、只有 basename），我用该条在报告里的第一处**可解析**锚点，且优先取「缺陷发生处」而不是「调用方」。

### 1.6 复核状态怎么读

| 字段值 | 含义 | 条数 |
| --- | --- | ---: |
| `confirmed` | 独立复核者用自己的装置证实 | 164 |
| `partial` | 部分证实：机制成立但有细节被修正；含 5 条 `mechanism_falsified`（见 §1.4 规则一） | 31 |
| `unverified` | **没有任何独立复核**，一律不视为已确认 | 17 |

`evidence_kind` 进一步区分复核手段，取值集 `{runtime, read, reviewed, none}`：`none` 17、`read` 18、`reviewed` 136、`runtime` 41。**只有 `runtime` 才等于「跑出来的事实」**；`reviewed` 是复核报告给了结论、手段在其出处里，`none` 与 `unverified` 一一对应。

## 2. 行动队列（按严重度）

### 2.1 CRITICAL（3 条，全部已实测且队长亲手复核）

#### LGC-LG-01 · CRITICAL · 大逻辑/命名/缓存 · `build_method/node_id.rs:36`

- **原名/来源**：LH-01（audit-logic-hunt.md:26） · 总账 LGC-LG-01
- **标题**：进程级身份缓存的键不含命名空间，跨包污染 `NodeId`
- **现象与判据**：1. `CACHED_NODE_IDS` 是 `static OnceLock<BTreeMap<String, (NodeId, String)>>`，**键只有相对源码路径**（`build_method/node_id.rs:25`、`:36`），值是某个命名空间下算出的 `NodeId`。 2. 写入侧**校验**了命名空间：`build_method/identity_cache.rs:58` 要求 `id == package_node_id(&relative, &kind)`（该函数用当前运行的命名空间）。但读取侧 `build_method/node_id.rs:37` 直接 `return Some(*id)`，**不再复核命名空间**；而 `OnceLock::set` 先到先得（`build_method/identity_cache.rs:64`），因此本进程里第二次以后的 `prime` 全部被丢弃——缓存内容永久是「本进程第一个 prime 命中的那个包」的。 3. 于是同一进程里服务两个包时会出现**混合命名空间的一棵树**：注册树里文件派生的 id 来自缓存（A 的命名空间），而 `cached_parent_id`/`package_root_node_id`（`build_method/cache.rs:186-201`）用当前运行的命名空间（B）重算父级 → 子面声明的父级在树里找不到 → `static-plan: parent node is missing`。 - 触发条件（全部满足即可，不需要恶意输入）： 1. 一个**长寿命进程**跑多次管线：MCP bridge（`xirang.verify`）、Studio、任何一次跑多包的测试二进制——`build_method/identity.rs:24-45` 的注释自己点名了「桥或 Studio 会话会在一个进程里服务多个包」； 2. 两个包有**相同的相对源码路径**（`control/control.rs` 这种在脚手架宿主之间必然重合）； 3. 进程的**第一次** `prime_node_id_cache` 能读到 unit 文件。也就是说，在此之前曾有任何一次运行把 `target/xirang/cache/units/*.tsv` 写出来——正常开发流程里的 `cargo build` / `cargo xirang check` 就会写（`build_method/cache.rs:99-162`）。若 `CARGO_TARGET_DIR` 被设置，unit 目录在包之间**共享**（`build_method/identity_cache.rs:14-26` 与 `:88-100`），则只需「A 先跑、B 后跑」两步即可。
- **最小修复方向**：见出处 audit-logic-hunt.md:26 的「最小修复方向」段
- **复核状态**：已复核（证实） · 实测/探针 · t16,队长
- **建议批次**：B1-立即：安全与数据完整性


#### LGC-LG-02 · CRITICAL · 大逻辑/安全/门禁 · `core/src/registry_core/authoring/parse/admission.rs:40`

- **原名/来源**：K-01（audit-lane-kernel.md:11） · 总账 LGC-LG-02
- **标题**：创作面「读一次再写回」静默丢弃 admission 的 deny 列表，门禁被放宽且不可逆
- **现象与判据**：`parse_admission_expression` 读构造形式时，若 allow 与 deny **同时**非空，只返回 allow： ```text return match (allow.is_empty(), deny.is_empty()) { (true, true) => Ok("ANY".to_owned()), (false, _) => Ok(format!("allow:{}", allow.join(","))), // deny 被丢弃 (true, false) => Ok(format!("deny:{}", deny.join(","))), }; ``` 该函数正是创作面读注册面 `admission` 字段的入口（`run_method/authoring/manifest/parse/parse.rs:134`），写回走 `render_admission`（`run_method/authoring/manifest/face/render.rs:79`）。于是 `Admission::new(&["ui"], &["ui/experimental"])` 读成 `allow:ui`，写回成 `crate::Admission::new(&["ui"], &[])`——**deny 列表在“读一次再存一次”后消失**。 `Admission::accepts` 的实现 `pub fn accepts`（`core/src/registry_core/declaration/registration.rs:64-77`）规定 deny 优先，deny 列表是这道门禁的否决权；两列表同时使用是被支持、且被本仓自带数据采用的写法（`core/src/registry_core/declaration/declaration.rs:344`：`Admission::new(&["ui", "ui/controls"], &["ui/experimental"])`）。丢弃 deny 不是格式化，而是把门禁放宽，且在源码被改写后不可逆。文件头注释自称“Both spellings describe the same gate”（`core/src/registry_core/authoring/parse/admission.rs:5-8`），与该行为直接矛盾（另见 C-02）。
- **最小修复方向**：紧凑形式必须能承载两张列表（如 `allow:a,b;deny:c`），`parse_admission_expression` 与 `parse_admission_owned` 双向支持；若坚持不扩展语法，则**两列表非空时返回 Err**（不允许静默丢字段），`render_admission` 同步拒绝。
- **复核状态**：已复核（证实） · 实测/探针 · t16,t9,队长
- **建议批次**：B1-立即：安全与数据完整性


#### LGC-LG-03 · CRITICAL · 可靠性 · `studio/studio/app/mutations.rs:257`

- **原名/来源**：S-01（audit-lane-studio.md:22） · 总账 LGC-LG-03
- **标题**：Studio 插件写盘路径能产出解析器拒绝的锁，界面报「已选中」
- **现象与判据**：`submit_plugin` 手写锁文件格式并直接 append：记录串在 `studio/studio/app/mutations.rs:258` 拼成 7 字段 `{source}|{framework}|{package}|{version}|{crate_name}|{checksum}|{mode}`，`studio/studio/app/mutations.rs:332-333` 用 `append_line` 追加。写入前只有两道闸：整行字符串相等（`studio/studio/app/mutations.rs:260`）与「official 必须已在受信锁里」（`studio/studio/app/mutations.rs:299`）。两者都拦不住**同一身份、不同 checksum/mode** 的新行： - 整行比较要求七个字段的序列化字节完全相同，checksum 或 mode 任一不同即放行； - `contains_record` 只对 `source == "official"` 调用（`studio/studio/app/mutations.rs:299` 的 `if source == "official" && ...`），`user` 来源完全绕过。 - 而 `PluginCatalog::parse` 把「身份五元组 `(source, framework, package, version, crate_name)` 重复」判为**硬错误**：`core/src/registry_core/plugin/catalog/catalog.rs:192-207`（checksum/mode 不在身份元组里），内核自己的用例 `core/src/registry_core/plugin/catalog/catalog.rs:325-332` 正是用 `sha256:a` / `sha256:b` 这一对证明它会失败（`duplicates package identity`）。 - 触发路径是界面上两个可编辑字段（`plugin_field::CHECKSUM`、`plugin_field::MODE`）：同一个 user 插件改 checksum、或只按一次 Enter 把 mode 从 `extension` 切到 `replacement`，再按两次 `s`，`studio/studio/app/mutations.rs:333` 成功、`studio/studio/app/mutations.rs:347` 报 “Plugin selected”。 `.xirang/plugins/{official,user}.lock` 是**宿主工件**。宿主的准入读它：`plugin-host/src/admission.rs:57-82` 把两份锁拼起来解析，失败即 `HostError::Policy("invalid plugin lock under …")`——插件面此后完全不可用，Studio 自己的下一次 `submit_plugin` 也会在 `studio/studio/app/mutations.rs:265-271` 报 “invalid lock”，而界面没有任何手段修回来（只能手改文件）。写入方在**写的那一刻**没有把解析器会拒绝的输入挡掉，且报告为成功。
- **最小修复方向**：把「追加一条记录」交回内核唯一的解析/校验入口——例如新增 `PluginCatalog::push_record`/`append_record`，内部先跑 `parse` 的同一套身份检查（复用 `accounts_for` 的身份五元组），或写入前先 `PluginCatalog::parse(format!("{existing}{record}\n"))` 并要求 `Ok`；`submit_plugin` 只组合候选串、不再自己决定什么是合法锁。
- **复核状态**：已复核（证实） · 实测/探针 · t11,t16,队长
- **建议批次**：B1-立即：安全与数据完整性


### 2.2 MAJOR（66 条，按批次与 id 排序）

### 2.2.1 MAJOR 明细（66 条）

（其中 `NAM-` 条目的**命名轴视图**（旧名 → 建议新名 → 判据）在 §7.1；这里是行动队列视图，两者同源。）

#### GTE-G-16 · MAJOR · 文档/漂移 · `mcp/Cargo.toml:27`

- **原名/来源**：G-16（audit-lane-gates.md:154）
- **标题**：文档-漂移：roadmap 与 `xirang-package-audit` 头部写的发布层级顺序与活表矛盾，且按它发布必然失败
- **现象与判据**：`xirang-mcp` 在第 2 层——但它真实的内部依赖是 `xirang-build-method` + `xirang-core` + `xirang-debug-method` + `xirang-run-method`（`mcp/Cargo.toml:27`、`:33`、`:39`、`:40`），因此它必须在 run_method、debug_method 之后；活表把它放在第 5 层（core | macro+build-method | run-method | debug-method+plugin-host | mcp | studio | cli）。按文档的顺序手动发布会在 `cargo publish -p xirang-mcp` 处解析失败。 - 判据 `[实测+读码]`：`tools/xirang-publish --check-table` 从清单推导依赖并逐条核对两张表，实测 exit 0 / `9 crates`——即活表是被门禁交叉验证过的唯一顺序；roadmap:255 的散文顺序不被任何门禁检查，且与能被推导出来的事实矛盾。
- **最小修复方向**：把 `docs/roadmap-1.0.md:255` 与 `tools/xirang-package-audit:37-40` 改成与活表一致，或删掉顺序、改为"顺序以 `tools/xirang-publish` 的 `levels` 表为准（`--check-table` 会核对）"。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t13 · **批次** B2-门禁与工具

#### GTE-G-21 · MAJOR · 注释 · `tools/xirang-package-audit:5`

- **原名/来源**：G-21（audit-lane-gates.md:208）
- **标题**：注释-准确性：`xirang-package-audit` 的头部注释描述了一张已不存在的表
- **现象与判据**：脚本里**没有**任何 `crates='…'` 表：`:227-249` 从根清单的 `members` 推导每个成员，从 `cargo metadata --no-deps` 读依赖，目录直接取成员路径。也就是说这十几行注释在解释"一份手工表、一个手工顺序、目录为什么必须写出来"——三者都已不存在。更糟的是它复述的顺序本身是错的（G-16）：读者会被引向去找一张表，并相信那个顺序。 - 判据 `[读码]`：`tools/xirang-package-audit:205-249`（推导）vs `:37-53`（散文）；`tools/xirang-package-audit:90-100` 的注释还保留了"扫描 TOML 漏掉的三种拼法"这段已改写的实现史，与实现（改从 metadata 读）不一致。
- **最小修复方向**：把 `:37-53` 换成"crate 集合与依赖从工作区推导（同 `tools/xirang-publish --check-table` 的口径），顺序以 `levels` 表为准"；删掉"目录为何手写"那段（它解释的是被替换掉的实现）。
- **复核状态**：已复核（证实） · 仅代码阅读 · t13 · **批次** B2-门禁与工具

#### GTE-N-1 · MAJOR · 其它 · `.github/workflows/release.yml:66`

- **原名/来源**：（复核新增）（audit-verify-gates.md:67）
- **标题**：tag 守卫的否定写法整类放行（`!(startsWith(...))`、`! startsWith(...)` 实测 0 发现）
- **现象与判据**：t13 夹具实测：`!startsWith` 与 `if: true` 各被抓 1 条，`== false`/`== 0`/`!(...)`/`! (...)` 均 0 条
- **最小修复方向**：只允许肯定式形状（形状白名单），不要再补文本黑名单
- **复核状态**：已复核（证实） · 实测/探针 · t13 · **批次** B2-门禁与工具

#### BRG-BR-1 · MAJOR · 文档 · `mcp/README.md:47`

- **原名/来源**：BR-1（audit-lane-bridges.md:54）
- **标题**：mcp README 仍在承诺已被删除的 `unmatched` 桶（文档承诺面；归并见 `BR-C0.1`）
- **现象与判据**：英文版写 “``stale``, `unmatched` (a typed cut stores an expression, so an absent identity cannot be told from a re-identified one) and unreadable records are kept apart”，中文版写 「`stale`、`unmatched`（类型化切口存的是表达式，身份缺席时无法分辨它与 re-identified）……」。 而实现里 `unmatched` 桶已在上一轮按审计 `m1` **删除**（`mcp/diff.rs:179-194` 留下了删除说明）， 现在记录一侧实际有五个桶：`ok` / `undeclared` / `stale` / `re-identified` / `unreadable` （计数行 `mcp/diff.rs:206-212`，`undeclared` 专段 `:224-239`）。 `grep -rn unmatched mcp/README.md mcp/README.zh-CN.md` 各 1 命中（现在时承诺）， `grep -rn unmatched mcp/src` 在生产代码里 0 命中（只有 `mcp/diff_tests.rs:165` 那条「不得出现」的负断言）。 上一轮自己的记录（`docs/audit-3p-2026-09-27.md:74`）说「工具描述（`mcp/tools.rs`）同步删掉对 `unmatched` 的承诺」——`mcp/tools.rs` 改了，两份 README 没改。
- **最小修复方向**：把两份 README 的那半句换成 `undeclared`（并补上它的后果：没有切口点名的记录会被发布剪掉）， 或改成「`ok` / `undeclared` / `stale` / `re-identified` / `unreadable`」这一行枚举，与 `mcp/diff.rs:206-212` 同字。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t12 · **批次** B3-桥与宿主

#### BRG-BR-2 · MAJOR · 文档 · `mcp/README.md:164`

- **原名/来源**：BR-2（audit-lane-bridges.md:75）
- **标题**：mcp README 自相矛盾：「树 diff 仍待做」（文档承诺面；归并见 `BR-C0.2`）
- **现象与判据**：同一份 README 的 **Reads 列表第 7 条**（`:41-48`）整段描述 `xirang.diff` 的面级 delta 与 `records: true`，而结尾一段写 “Graft writes, plugins, project scaffolding, tree diffs, and the consistency analysis are still to come (`docs/roadmap-1.0.md` item 7)”（中文：「graft 写入、插件、 项目脚手架、树 diff 与一致性分析仍待做」）。树 diff 就是 `xirang.diff`，已出厂；被引用的 `docs/roadmap-1.0.md` 第 7 条现在是别的事（「下一批」列表第 7 条是 C19 诊断字段表，第四轮第 7 条是 `petgraph` 升级），引用也已失效。 `grep -n "xirang.diff" mcp/README.md` → `:41`；`grep -n "still to come" mcp/README.md` → `:165`。 工具存在且被 `mcp/tools_tests.rs:83-98` 钉住（`xirang.diff` 断言 `no build evidence`）。
- **最小修复方向**：删掉这句过期的路线图话，或按当下实际剩下的集合重写并给出**可解析**的路线图锚点 （若没有剩下的，就写「本桥的能力即上面两张清单」）。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t12 · **批次** B3-桥与宿主

#### BRG-BR-C2 · MAJOR · 其它 · `mcp/preview.rs:14`

- **原名/来源**：BR-C2（audit-lane-bridges.md:438）
- **标题**：`mcp/preview.rs` 的目录说明与它实际跳过的集合不符
- **现象与判据**：两处文档都说副本跳过 `target/` 与 XiRang 自己的运行期目录，并断言 「`target/` is the one directory that can be large」（`target/` 是唯一可能很大的目录）。 实际只跳过 `target` 与 `.xirang`（`:45-47`），`.git` 会被整份复制并参与 diff—— 在本检出上 `.git` 是 9.9 MB，而真实仓库常见 GB 级；这句「唯一」因此是错的， 也是 `BR-4` 那 36 条虚假 `+ …` 的直接来源。 `[代码]`：`:45-47` 的跳过条件只有两个名字；`[实测]`：`.git` 9.9 MB、树 46 MB（见 `BR-4`）。
- **最小修复方向**：改成「跳过构建产物与 VCS/工具目录（当前是 `target/`、`.xirang/`、以及拟新增的 `.git/`）」， 或删掉「唯一可能很大」这半句——一句话的断言比一份清单更容易过期。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t12 · **批次** B3-桥与宿主

#### KRN-C-01 · MAJOR · 门禁/注释 · `core/src/registry_core/authoring/parse/admission.rs:5`

- **原名/来源**：C-01（audit-lane-kernel.md:280）
- **标题**：注释描述的行为与实现不符（模块自述承诺“两种拼法同一个门禁”，实际丢字段）
- **现象与判据**：模块文档写“Admission is written either as the compact `allow:…`/`deny:…`/`ANY` form or as the `Admission::new(&[…], &[…])` constructor an editor emits. Both spellings describe the same gate, so both are read here.”，而 `:40-44` 在两张列表同时非空时只保留 allow。 见 K-01。这条注释让读者相信“读一遍写一遍是无损的”，正是最需要被推翻的那句话；过时注释比没有注释更坏（维护者判据 1）。
- **最小修复方向**：修好 K-01 后此注释才成立；若暂不修，必须改成明确的边界声明（“两列表同时出现时 deny 会被丢弃，见 K-01；勿据此写回”）。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### KRN-C-02 · MAJOR · 文档/注释 · `core/src/registry_core/declaration/call_evidence.rs:6`

- **原名/来源**：C-02（audit-lane-kernel.md:287）
- **标题**：注释描述的类型与实现不符（`CallSite` 的文档写的是“一条调用边”）
- **现象与判据**：`CallSite` 的字段是 `node`/`function`/`frame_id`/`source`（一个调用点），但它的文档是“One call edge that was observed while a `CallTrace` frame was active.”——那句话正是 `CallEdge`（caller+callee 对）的定义，两句逐字重复。 文档字符串是公开面的一部分（crate 开了 `#![warn(missing_docs)]`，docs.rs 是主读者），把“点”写成“边”会让按该名索引的人找错类型；重复文本会让下一次修改只改一处。维护者判据 1。
- **最小修复方向**：`CallSite` 改为“One observed call site: which node/function was active in which trace frame, and where it was written.”，`CallEdge` 保留现句并注明两端都是 `CallSite`。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### KRN-C-03 · MAJOR · 其它 · `core/src/registry_core/tree/ports/ports.rs:29`

- **原名/来源**：C-03（audit-lane-kernel.md:294）
- **标题**：模块自述的解析规则与同模块的另一函数相反（“绝不静默解析”vs `resolve_path` 取第一个）
- **现象与判据**：ports 的模块文档写“a handle or a port that matches more than one face is **reported, never resolved silently** — an implicit "nearest wins" rule is exactly the kind of hidden behaviour this workspace refuses.”；`core/src/registry_core/tree/graft_ops/resolution.rs` 的 `resolve_node` 遵守它（返回 `Resolution::Ambiguous`），而同一文件上方的 `resolve_path` 用 `find_map` 取第一个匹配。 见 K-05。注释陈述的是全仓立场，实现只在其中一条入口上兑现，读者无法从注释知道哪一条被豁免。
- **最小修复方向**：修好 K-05（`resolve_path` 返回 `Ambiguous`），或在该注释里点明 `resolve_path` 是例外并说明为什么。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-04 · MAJOR · 大逻辑/门禁 · `core/src/registry_core/plugin/catalog/catalog.rs:126`

- **原名/来源**：K-07（audit-lane-kernel.md:88） · 总账 LGC-LG-04
- **标题**：插件锁 schema 门禁：按行序生效、对空锁不生效、表头拼错即静默关闭
- **现象与判据**：schema 校验写在“处理一条记录”的分支里： ```text if let Some(value) = line.strip_prefix("# xirang-schema=") { schema = Some(value.trim()); continue; } if line.starts_with('#') { continue; } if let Some(version) = schema && !schema_matches(version) { return Err(...); } ``` 因此：①只有注释与表头、没有任何记录的锁 → 循环体永不走到校验 → `Ok`（空目录），与函数文档“refusing unknown schemas”（`:120`）不符；②表头写在记录**之后**时，前面的记录在 `schema == None` 下解析 → 旧/新 schema 的记录被当作本版本可读；③同一文件里第二行 `# xirang-schema=` 会静默覆盖第一行（此处没有重复键拒绝，而同一模块的 `PluginRecord` 与 `graft.plan` 都拒绝重复）。 schema 是身份/命名空间的版本闸门，它的全部意义是“读不懂就拒绝”（`core/src/registry_core/plugin/catalog/catalog.rs:103-114` 注释）。按行序生效意味着一个乱序或被裁剪过的锁文件能绕过它，且方向是**放行**（读到本不兼容的语义），不是拒绝。
- **最小修复方向**：门禁判定改为「结构化读取」：只认 `# xirang-schema vN` 的规范拼法并拒绝其它写法；t16 实测四形态：少 `=` → `Ok(1)` 静默关闭、空锁 → 不检查、表头在记录之后 → 不检查。
- **复核状态**：已复核（证实） · 实测/探针 · t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-05 · MAJOR · 安全/可靠性/边界 · `run_method/authoring/manifest/face/face.rs:140`

- **原名/来源**：S19 = K-22（audit-lane-surfaces.md:306；audit-lane-surfaces.md:306） · 总账 LGC-LG-05
- **标题**：`flow_provider` 编辑入口缺内核已有的嵌套守卫 → 极短输入即可 abort
- **现象与判据**：同一个校验存在两份：内核那份先量嵌套再 `syn::parse_str`，执行面这份直接 `syn::parse_str`。`syn` 是递归下降且没有自己的深度守卫，栈溢出**不是可捕获的 panic**，因此这一支的失败不是"少一条诊断"，而是进程 abort。而且顺序上执行面的这一支先跑：`edit()` 在 `render_source()` 之前执行，而带守卫的 `render_flow_provider` 只在渲染时被调用（`run_method/authoring/manifest/face/render.rs:110`），所以守卫没有机会说话。 **判据（实测）** 我用工作区自己的 `syn` rlib（debug 构建，即编辑器/MCP 调试会话实际链接的那份）构造了最小复现，测的是 `run_method/authoring/manifest/face/face.rs:141` 这一行的调用本身（探针代码与二进制在 `/tmp/xirang-probe-surfaces/`，只读工作区、未改工作区任何文件）： - 输入形如 `A<A<…>>`（嵌套泛型路径）。8 MiB 栈：深度 200（601 字节）→ 正常返回；深度 300（901 字节）→ `fatal runtime error: stack overflow, aborting`，退出码 134。 - 256 KiB 栈：深度 100（301 字节）即 abort。 - 结论：阈值是"几百字节"量级，与内核注释里记的 542 字节同阶；一份手写或代理生成的畸形 `flow_provider` 就能让进程死掉，而不是拿到一条 `Err`。 - 反面（说明这不是"整条链都没守"）：同一条值在渲染侧是安全的，`run_method/authoring/manifest/face/render.rs:110` 调的是内核那个带守卫的入口；`flow` 字段也安全，它走内核的 `parse_flow_value`（`core/src/registry_core/auth …（全文见 `audit-findings.json`）
- **最小修复方向**：两段修法：①把内核 `guard_nesting` 接到 `run_method/authoring/manifest/face/face.rs:140` 的 `syn::parse_str::<syn::Path>` 之前，并把公开 API 里「按调用方预算递归」改成受本模块上界约束；②明确「栈是调用方的责任」——t16 实测：100 层 / 301 B 的**嵌套泛型**输入下 `guard_nesting` 返回 `Ok(())`，无守卫的 syn 在 256 KiB 栈上**仍然 abort**（同长度 100 层 `::` 链可正常解析），因此必须在固定栈线程上跑解析，或给 syn 调用点包一层显式栈预算。只接守卫不闭合。
- **复核状态**：已复核（部分证实） · 实测/探针 · t10,t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-06 · MAJOR · 大逻辑 · `core/src/registry_core/tree/graft_ops/overlay.rs:258`

- **原名/来源**：K-04（audit-lane-kernel.md:54） · 总账 LGC-LG-06
- **标题**：同一「trait 契约」规则两处实现：overlay 非 full 分支不查 `needs_registry`
- **现象与判据**：非 full 嫁接分支无条件保留基座子注册机： ```text let mut kept = entry.child.clone(); if let Some(registry) = &mut kept { ... registry.reconfigure(&candidate); ... } ``` 它只检查“保留的子级是否满足新规则”，**不检查** `candidate.needs_registry`。而就地替换路径对同一状态明确拒绝（`core/src/registry_core/tree/graft_ops/graft_ops.rs:261-264`：`(Some(child_registry), false) if !child_registry.entries.is_empty() => return false`）。结果：一次非 full 嫁接可以让一个 `needs_registry: false` 的面继续拥有非空子注册机，并且 `register_snapshot_batch` 仍能通过 `registry(parent_id)` 找到它并向里注册（`core/src/registry_core/tree/transaction/transaction.rs:47`）。 两条路径回答同一个问题（“这个面还能不能拥有它现在拥有的子注册机”）却给出相反答案；`needs_registry` 是声明的权威（注册路径只按它创建子注册机，`core/src/registry_core/tree/transaction/transaction.rs:246-256`）。留下“声明为叶子、实际是父级”的节点会让 `port_index`/`dump`/连接器遍历与声明不一致，且该类不一致只能靠重读源码发现。现有测试 `record_tests::full_discards_base_children_and_non_full_keeps_them` 的替换件也是 `needs_registry: true`，因此没有覆盖这条边界。
- **最小修复方向**：`apply_overlay_face` 的非 full 分支在 `!candidate.needs_registry` 且 `kept` 非空时返回专门的 `GraftError`；把判据抽成一个共享函数（如 `fn child_registry_can_survive(needs_registry: bool, child: &Option<Arc<Registry>>) -> bool`）供两条路径调用。
- **复核状态**：已复核（部分证实） · 实测/探针 · t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-07 · MAJOR · 大逻辑 · `core/src/registry_core/tree/transaction/transaction.rs:98`

- **原名/来源**：K-05（audit-lane-kernel.md:69） · 总账 LGC-LG-07
- **标题**：兄弟同名 `registry_name` 无人拒绝，路径这一事实键三处静默碰撞
- **现象与判据**：`plan_batch` 只检查三类冲突：`NodeId` 与基树/批内重复（:116-128）、显式 `stable_name` 重复（:129-147）、父链环与缺父（:149-177）。**没有任何地方检查同一父级下 `registry_name` 是否重复**。而 `registry_name` 是可被作者编辑的字段（`run_method/authoring/manifest/face/face.rs:100,116-118`，只按 `validate_name` 要求 snake_case），默认取模块名。于是两个 `id` 不同、路径相同的兄弟可以同时注册。后果是三处“谁赢”的静默选择： - `RegistryIndex` 的 `by_path` 索引（`core/src/registry_core/tree/index/index.rs:88-93`）：`insert` 覆盖，后插入者赢，查询结果取决于 `BTreeMap<NodeId>` 遍历顺序； - `path_for`（`core/src/registry_core/tree/query/query.rs:86-96`）：两个节点的路径字符串完全相同，下游任何按路径记账的判断都无法区分； - `resolve_path`（`core/src/registry_core/tree/graft_ops/resolution.rs:22-28`）：`find_map` 返回**第一个**匹配，嫁接切口选择器静默落在其中一个上——而同模块的 `resolve_node`（`:146-175`）对同名/同 kind/同路径明确返回 `Resolution::Ambiguous`。 本工作区既定立场是“匹配到多个面一律报出，绝不静默解析”（`core/src/registry_core/tree/ports/ports.rs:29-33` 明写）。路径在本仓不只是显示串：连接器的准入门禁就用 `path_is_strictly_under(provider_path, owner_path)` 判定 …（全文见 `audit-findings.json`）
- **最小修复方向**：在 `plan_batch` 增加兄弟级 `registry_name` 查重（按 `snapshot.parent` 收集现存子项名与批内名字，冲突即 `RegistryError`，文案与 `duplicate stable face name` 同构）；`resolve_path` 改为返回 `Resolution`（`One/Ambiguous/Missing`），与 `resolve_node` 合并成一条解析规则。
- **复核状态**：已复核（证实） · 仅代码阅读 · t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-08 · MAJOR · 大逻辑/边界 · `build_method/face_view.rs:280`

- **原名/来源**：S1（audit-lane-surfaces.md:13） · 总账 LGC-LG-08
- **标题**：`FaceView.path` 顺序依赖，与运行期 `path_for` 可不同判
- **现象与判据**：`logical_path` 计算某个面的逻辑路径时，先查 `paths` 缓存里有没有**父面**的路径；查不到就用 `"root"` 当父路径，并把 `"root/<自己的槽位名>"` 写进缓存。父面是否已经在缓存里取决于 `collect` 的遍历顺序（DFS 前序 + 同级按名字排序）。只要某面的父面在同级里排在它**后面**（或父面在另一条分支上更晚被访问），这个面的骨架路径就少一段，并被永久冻结。运行期的 `path_for` 是从活树递归求值，永远给出完整链。 - 构造最小反例：`src/alpha/alpha.rs` 声明 `parent: crate::zeta::NODE_ID`，`src/zeta/zeta.rs` 是 `needs_registry: true` 的面。DFS 先访问 `alpha`，于是 `paths` 里还没有 `zeta`，`alpha.path` 得到 `root/alpha`；运行期树给出的是 `root/zeta/alpha`。`parent_resolved` 仍为 `true`，没有任何东西标记这处不一致（`build_method/face_view.rs:148`）。 - 现有测试正好回避了这一格：`build_method/face_view.rs:350` 断言了父面的 `control.path == "root/object"`，但对子面（`build_method/face_view.rs:353` 起）只断言了 `registry_name` 与 `parent_resolved`，**没有断言 `button.path`**——所以这处顺序依赖没有被钉子覆盖。 - 该字段的承诺写在 `build_method/face_view.rs:82`～`build_method/face_view.rs:84`：“`path` 是运行期树会报告的逻辑注册路径”。`overlay_rows` 又用它做切口匹配（`build_method/graft_view/ …（全文见 `audit-findings.json`）
- **最小修复方向**：把 `logical_path` 改成真正的递归（先解析父面路径，再拼自己；带环检测），或直接复用内核的路径规则（`Registry::path_for` 那一条），不要保留"靠访问顺序预热缓存"的一级近似。
- **复核状态**：已复核（证实） · 实测/探针 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-09 · MAJOR · 大逻辑 · `build_method/pipeline.rs:155`

- **原名/来源**：S2（audit-lane-surfaces.md:38） · 总账 LGC-LG-09
- **标题**：构建期「货币凭据」先于载荷发布，载荷非事务写 → 混代产物被读成 current
- **现象与判据**："这次构建的产物是否描述当前源码"这件事，由 `discovery.fingerprint` 一枚凭据回答；而这枚凭据是在载荷之前写的。任何一份载荷写失败（只读文件、配额、中断）时：构建脚本路径会 panic（构建失败，安全），但**结构化调用方**（`check --json`、MCP/Studio 的 `check_for`）拿到的是"一条 `out-dir` 诊断 + 退出"；`OUT_DIR` 里留下的却是**混代状态**——指纹是新的，而失败那一份清单还是旧的或缺失，其余几份已经换成新的。下一次由另一个进程读 `build_output_is_current` 会得到 `true`，于是把旧行当成当前事实（`pruning_manifest.tsv` 那一列正是维护者在发布剪枝前读的东西）。失败时删指纹那一支还带 `let _ =`：删不掉就继续带着一枚旧指纹往下走，方向同样是"把旧产物当新产物"。 - `write_if_changed` 用 `fs::write`，在写入过程中失败会留下**截断的目标文件**（`build_method/cache.rs:38`），`generated_lib.rs` 也在这一支里（`build_method/pipeline.rs:179`）。 - 载荷数组与指纹写入之间没有任何"全部成功才提交"的顺序保证：`build_method/pipeline.rs:155` 的指纹写发生在这五份载荷之前。 - 反证这处不是有意设计：`build_method/pipeline.rs:147`～`build_method/pipeline.rs:154` 的注释明确说"只有干净的一次运行才发布这枚凭据……没有那枚凭据，没有任何东西会把它们读作描述当前源码"——注释描述的语义要求"载荷全部落地"才算干净，而代码只检查了 `compile_errors`（校验错误），没有检查 `write_errors`。
- **最小修复方向**：把载荷写在前、指纹写在最后，且只在 `write_errors.is_empty()` 时发布；删指纹失败上报为诊断而不是 `let _ =`；`write_if_changed` 改成写唯一临时文件后 rename（与 `run_method/authoring/filesystem/filesystem.rs:18` 同一模式），这样"文件存在"就等于"文件完整"。
- **复核状态**：已复核（部分证实） · 仅代码阅读 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-10 · MAJOR · 大逻辑/边界 · `run_method/authoring/operations/migration.rs:179`

- **原名/来源**：S5（audit-lane-surfaces.md:105） · 总账 LGC-LG-10
- **标题**：模块迁移对整棵子树做无边界文本替换，兄弟模块被静默改写
- **现象与判据**：重命名模块时，迁移器对子树里**每个** `.rs` 文件做两次 `String::replace`：`crate::<旧模块路径>` → `crate::<新模块路径>`，以及 `src/<旧目录>/` → `src/<新目录>/`。两者都是**字符串前缀**匹配，没有 `::` / `/` 边界，也不区分注释与字符串字面量。 - 同一仓库的其它位置明确为这个边界写过规则与注释：`build_method/scope_view.rs:109`～`build_method/scope_view.rs:116`（"边界是 `::` 段，不是文本前缀：`control` 保留 `control::object::button`，绝不保留 `control_extra`"），scope 的子树选择器同理。迁移路径没有这层保护，属于同一工程里的两种口径。 - 可复现：树里有兄弟模块 `control_extra`，把 `control` 迁成 `widget`。子树里任何 `crate::control_extra::NODE_ID`（`parent:` 字段、类型化 graft 切口）会被改写成 `crate::widget_extra::NODE_ID`；任何 `src/control_extra/...`（例如 `registry_rule_path:` 的取值）会被改写成 `src/widget_extra/...`。改写是静默的（只比较"是否与原文本不同"才写盘，`run_method/authoring/operations/migration.rs:188`），不校验替换结果是否仍是同一个东西。 - 触发条件是"名字互为前缀"，这在真实注册树里很常见（`control` / `control_v2`、`panel` / `panel_extra`）。
- **最小修复方向**：不要对整棵树做文本替换：迁移只应改本面自己渲染出来的内容（父级引用与规则路径都在 `FaceManifest` 的字段里），后代面的 `parent:` 指向的是 `crate::<module>::NODE_ID` 这类**常量路径**，模块目录改名本来就不需要动它们。若确实要改，也必须在 `::` / `/` 边界上匹配（或走 token 级重写）。
- **复核状态**：已复核（证实） · 仅代码阅读 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-11 · MAJOR · 大逻辑 · `build_method/face_view.rs:211`

- **原名/来源**：LH-02（audit-logic-hunt.md:91） · 总账 LGC-LG-11
- **标题**：`face_views` 静默丢弃解析失败的注册面文件，并把子面改挂根（打印运行期从未有过的路径）
- **现象与判据**：一个面文件只要**不是恰好一个可解析的注册面**（例如被追加了第二个宏调用、宏体写坏），`parse_face` 返回 `Err`（`core/src/registry_core/syntax/face.rs:175-183`），`parsed_face` 把它变成 `None`，`face_views` 就把这个文件**当成没有面**跳过——不计数、不诊断。它的子面因此解析不到父级，在 `logical_path` 里回退成 `root/<slot>`。于是「这个包声明了什么」这个问题，构建与桥给出**不同规模的答案，且桥的那份带有一条运行期从未存在过的逻辑路径**。 - 触发条件：包内任意一个注册面文件解析失败（最常见：两处宏调用、字段写坏、缺 `kind`）。 - 最小复现（实测）： ```sh P=/tmp/nichprobe/pkg # LH-03 的同一个 fixture printf '\npub struct Twin;\ncrate::root_object! {\n kind: Twin,\n needs_registry: false,\n registry_name: "twin",\n parent: crate::root_node_id(env!("CARGO_PKG_NAME")),\n}\n' >> $P/src/control/control.rs # 构建侧（报错） cd $P && /home/nich/Moirai_N3/nichlink/target/debug/cargo-xirang xirang check # 视图侧（不报错、少一个面、子面改挂根） python3 /tmp/nichprobe/probe.py xirang.registry '{}' $P ``` 实测： ```text # check: phase=face-syntax source=control/control.rs:43（探针包内相对路径，探针包在 /tmp/nichprobe/pkg；工作树里两个同名候选文件分别只有 32/3 …（全文见 `audit-findings.json`）
- **最小修复方向**：见出处 audit-logic-hunt.md:91 的「最小修复方向」段
- **复核状态**：已复核（证实） · 实测/探针 · t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-12 · MAJOR · 假实现 · `mcp/diff.rs:150`

- **原名/来源**：LH-03 = BR-8（audit-logic-hunt.md:127；audit-logic-hunt.md:127） · 总账 LGC-LG-12
- **标题**：`diff records:true` 只在「身份在树里」那一支查「是否被声明」→ 与 `xirang.grafts` 结论互相矛盾
- **现象与判据**：同一条 graft 记录，两个 MCP 工具给出**互相矛盾的结论**：`xirang.diff records:true` 把它算进 `re-identified` 或 `stale` 并报 `undeclared 0`；`xirang.grafts` 对同一份记录报 `[NOT declared by the host entry]` 与 `unkept plans 1: the release prunes these slots, so the records can never take effect`。前者是「身份漂移，修一下就好」，后者是「这条记录永远不会生效（发布剪枝）」——两句话的行动含义相反，而 `undeclared 0` 是错的。 - 触发条件：一条记录的身份不在树里（典型的「槽位没动、身份换了」）且宿主入口没有用**字符串**切口点名该路径时，即命中。类型化切口（`cut(crate::…::NODE_ID)`）在身份缺席时无法解析模块（`build_method/graft_view/plan_rows.rs:152-155` 只在身份命中时取 `module`），因此这种记录几乎必然落在这个分支——正是本项目 scaffold 生成的宿主入口的写法（`examples/control-button/src/lib.rs` 的 typed `cut(...)`）。 - 最小复现（实测，只有一条 `moved_identity` 记录，其余删掉）： ```sh P=/tmp/nichprobe/pkg rm -rf $P/.xirang/external-grafts mkdir -p $P/.xirang/external-grafts/moved_identity printf 'version=1\ntarget=00000000000000000000000000000000\ntarget_path=root/control/button\ngraft=button_fast\nfull=false\n' \  …（全文见 `audit-findings.json`）
- **最小修复方向**：见出处 audit-logic-hunt.md:127 的「最小修复方向」段
- **复核状态**：已复核（证实） · 实测/探针 · t12,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-13 · MAJOR · 命名 · `mcp/verify.rs:43`

- **原名/来源**：BR-6（audit-lane-bridges.md:151） · 总账 LGC-LG-13
- **标题**：`XIRANG_NAMESPACE` 下 `verify` 用 Cargo 名发布、`diff`/`search` 用覆盖名读 → 每个面都被自信地报成 re-identified
- **现象与判据**：`verify` 把 `xirang_build_method::package_name(&manifest)`（Cargo 的包名）交给 `check_for`， 于是本次运行发布的 `pruning_manifest.tsv` 行 id 是让 Cargo 名算的 （`build_method/manifests.rs:181` → `registry_identity::package_node_id`，运行内生效的是 `check_for` 设下的线程局部名）。而 `diff`/`search` 的源码侧走 `crate::registry::namespace(root)`， 它**先读 `XIRANG_NAMESPACE`**。两者不一致时，`TreeDelta::by_source` 仍按源码路径命中 （路径与命名空间无关），于是 `status()` 对**每一个**面走 `Reidentified(previous)` 分支： `diff` 报 `added 0 gone 0 reidentified N`，`search` 给每条命中标 `re-identified`，而 `build_output_is_current` 说 `build current`（指纹只散列路径与内容，与命名空间无关）——一个「刚校验过」 的树被报告成每个身份都变了。 `[代码]`：`mcp/verify.rs:43` vs `mcp/registry.rs:59-61`；`collect_pruning_symbols` 用进程/线程命名空间 （`build_method/manifests.rs:181-184`）、`face_views` 用传入的 `package` 参数 （`build_method/face_view.rs:126-174,211-247`）。代码里已登记的分歧 `S12` 只说了「桥与 CLI 不一致」， 没说「桥自己内部 `verify` 与 `diff` 互相打脸」。
- **最小修复方向**：`verify` 用与 `diff` 同一个命名空间（`crate::registry::namespace(root)`）作为 `check_for` 的 `package` 参数；或在两者不一致时明确拒绝并说明该设哪一个（与 `mcp/registry.rs:74-79` 对未命名包的处理同风格）。
- **复核状态**：已复核（证实） · 实测/探针 · t12,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-14 · MAJOR · 大逻辑/边界/文档 · `run_method/macros/face_registration.rs:9`

- **原名/来源**：S9 = C3（audit-lane-surfaces.md:189；audit-lane-surfaces.md:352） · 总账 LGC-LG-14
- **标题**：`linked` collector 分支什么都不提交，模块文档却说它决定进哪个链接段
- **现象与判据**：三个分支里只有 `debug` 会真的向链接段提交；`linked`（外部面省略 `collector:` 时的默认）展开为空，`development` 也展开为空。于是"collector 决定注册进哪个链接段"这句话对三分之二的取值不成立，而外部面的默认取值正是那个什么都不做的 `linked`。作者看到 `collector: linked` 这个名字、再读模块文档，会得出"已经进了链接段"的错误结论。 - `debug_method/src/collector.rs:8`～`debug_method/src/collector.rs:21` 的 `registrations!` 在非 debug 构建下返回空迭代器，与"`debug` 才收集"这条一致；但没有任何地方说明 `linked` 并不提交。 - 文档与实现的矛盾在同一份文件里：`run_method/macros/face_registration.rs:9` 的承诺 vs `run_method/macros/face_registration.rs:168` 的空展开，中间没有"仅 `debug` 会提交"的限定。 - 该分支的注释只写了"收集归 xirang-debug 所有"（`run_method/macros/face_registration.rs:169`），既没解释为什么 `linked` 什么都不做，也没有把注意力引到"这个名字表达的是'由别处收集'"。
- **最小修复方向**：二选一：(a) 若 `linked` 只是"不收集"的别名，改名（例如 `off` / `none`）并在模块文档里列出每个取值的真实效果；(b) 若 `linked` 本应提交到某个链接段，就在这一支里实现它。无论哪种，`collector: $collector:ident` 的取值集合都应当是编译期可枚举、可拒绝非法名的（现在是任意 ident，写错名字会掉进"没有匹配 arm"的裸 marco 错误）。
- **复核状态**：已复核（证实） · 仅代码阅读 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-15 · MAJOR · 假实现 · `build_method/manifests.rs:200`

- **原名/来源**：S3（audit-lane-surfaces.md:62） · 总账 LGC-LG-15
- **标题**：剪枝清单的符号列是逐行匹配魔法标识符得到的，其中一支无条件返回 `Button::…`
- **现象与判据**：`pruning_manifest.tsv` 的第三列（symbol）不是从已解析的注册面/静态计划里读出来的，而是逐行匹配三个魔法标识符得到的：`PRUNING_TABLE`、`pruning_probe`、`optional_pruning_probe`。第三个分支**无条件**返回字符串 `"Button::optional_pruning_probe"`——与当前是哪个面无关；第一个分支只要某行包含 `PRUNING_TABLE` 且以 `static `/`pub static ` 开头（注释、字符串里也算）就算命中。 - 这三个标识符在整个仓库里只出现在这一处（`grep -rn 'PRUNING_TABLE\|pruning_probe' build_method run_method cli mcp studio core docs` 只命中 `build_method/manifests.rs:200` 起的这段函数），没有任何注册面声明它们，也没有测试覆盖它们。 - 这份产物确实被人当作事实读：`build_method/graft_view/plan_rows.rs:118`～`build_method/graft_view/plan_rows.rs:119` 的注释说"这个数字正是维护者在发布剪枝前读的东西"；`cli/explain_report.rs:131` 与 `mcp/evidence.rs:53` 会把 `PruningRow.symbol` 渲染出来。 - 一旦某个不叫 `Button` 的面定义了 `fn optional_pruning_probe`，产物给出的符号就是别的面的名字——一条**错误证据**，而且没有任何东西能发现它错了。
- **最小修复方向**：符号应当来自已解析/已注册的事实：至少把前缀换成该面的 module 或 kind（`format!("{}::optional_pruning_probe", kind)`），并把"哪些符号会被发布期修剪"的约定写进文档与测试；更彻底的做法是直接从 kernel 的静态计划/注册快照读被跟踪符号，删掉这条行扫描。
- **复核状态**：已复核（证实） · 实测/探针 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-16 · MAJOR · 命名/边界 · `build_method/renderer/tree.rs:68`

- **原名/来源**：S7（audit-lane-surfaces.md:150） · 总账 LGC-LG-16
- **标题**：`compile_error_demo` 这个魔法目录/文件名改写任意宿主的构建结果
- **现象与判据**：凡是顶层目录名正好叫 `compile_error_demo` 的子树，构建期会把它整棵挂到 `#[cfg(feature = "compile_error_demo")]` 之下，同时在注册检查/合同检查/静态计划里默认跳过它。这是本仓库早期演示用的目录名，被写死进了构建规则。 - 该名字在本工作树里没有任何声明者：既没有 `compile_error_demo/` 目录，也没有任何 `Cargo.toml` 声明 `compile_error_demo` 特性（`grep -rn compile_error_demo` 只命中上述四处构建代码与它们的注释）。 - 对第三方宿主：如果它恰好有一个叫这个名字的注册目录，而它没有声明该特性，那些面会被**静默门控掉**（不编译、不进静态计划），而构建不会因此报错；`cargo::rustc-check-cfg` 只声明了 `cfg(rust_analyzer)`（`build_method/pipeline.rs:210`），所以这处特性名还是未声明的 cfg。 - 与 S8 同型：执行面里带着本仓库夹具/演示的名字当规则。
- **最小修复方向**：把魔法目录/文件名收进显式清单并在文档里说明；t16 标注本条为**潜伏**缺陷（需宿主目录名恰好命中才生效），排期可按隐患处理。
- **复核状态**：已复核（部分证实） · 仅代码阅读 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-17 · MAJOR · 假实现 · `run_method/authoring/parse/parse.rs:31`

- **原名/来源**：S8（audit-lane-surfaces.md:170） · 总账 LGC-LG-17
- **标题**：authoring 的 source 回落链硬编码本仓库夹具目录名，宿主同名目录会被截断
- **现象与判据**：`source_path_from_file` 优先相对 `source_root()` 求路径；求不出来时，它会在**绝对路径**里寻找名为这四个之一的路径分量，并把该分量之后的全部内容当成相对路径返回。 - 这四个名字是仓库自己的夹具/演示模块名（`grep -rn '"engine"' run_method/src` 只命中这里；`trimmed_core` 同样只在这里）。库代码里的这个启发式对任何其它宿主都是错的：`/data/control/myrepo/src/foo/foo.rs` 会被读成 `myrepo/src/foo/foo.rs`（正确应是 `foo/foo.rs`），而这个值就是 `FaceManifest.values["source"]`，直接决定身份输入与清单里的 `source` 列（`core/src/registry_core/identity/path_text.rs:123` 的 `manifest_relative_source` 是为这件事准备的、不依赖硬编码的规则）。 - 触发条件是"文件不在 `source_root()` 之下"，而这正是 Studio 打开一个非标准布局工程、或 `XIRANG_PACKAGE_ROOT` 指向别处时的情形，不是测试专属路径。
- **最小修复方向**：删掉这份名字清单：路径不在 `source_root()` 下时应当报错（或按 `manifest_relative_source` 的规则折叠 `src/` 前缀），而不是猜一个分量。若确有历史夹具需要兼容，把它作为参数/配置传入，不要写进库的通用路径。
- **复核状态**：已复核（证实） · 实测/探针 · t10,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-18 · MAJOR · 假实现 · `mcp/mir.rs:73`

- **原名/来源**：LH-04（audit-logic-hunt.md:164） · 总账 LGC-LG-18
- **标题**：MIR `jsonl:true` 可给任意可读文件盖上本包快照表头（来源可证 → 来源可造）
- **现象与判据**：`xirang.mir` 只要给的路径不以 `.jsonl` 结尾，就用「不是调用的行就不是调用」的宽松文本解析器读；对一个完全不是 MIR 转储的文件，它返回一张**空**图并判 `isError:false`。此时 `jsonl:true` 会把这台空图**盖上本包的命名空间与 root** 输出成一份合法 artifact。快照表头正是这套工具链里「这份 artifact 描述哪棵树」的**唯一凭据**（同文件的 `delta_report`、`unified`、`mcp/trace.rs` 的身份校验都靠它），而它可以由任意文件凭空产生。 - 触发条件：`path` 指向包内任意可读的非 `.jsonl` 文件（工具自述里 `path` 是必填，扩展名决定解析器，这一点没有别的校验）。 - 最小复现（实测）： ```sh P=/tmp/nichprobe/pkg printf 'This is not a MIR dump.\nJust prose.\n' > $P/notes.txt python3 /tmp/nichprobe/probe.py xirang.mir '{"path":"notes.txt"}' $P python3 /tmp/nichprobe/probe.py xirang.mir '{"path":"notes.txt","jsonl":true}' $P ``` 实测输出（原样）： ```text file /tmp/nichprobe/pkg/notes.txt functions 0 calls 0 locals 0 calls: locals: {"kind":"snapshot","namespace":"probe-host","root":"cc05a41d58a3a71153bfcd58b7a05e9d"} ``` 把它写回 `fabricated.jsonl` 再读，回复是 `functions 0 calls 0 locals 0` + `snapshot namespace=probe-host …（全文见 `audit-findings.json`）
- **最小修复方向**：见出处 audit-logic-hunt.md:164 的「最小修复方向」段
- **复核状态**：已复核（证实） · 实测/探针 · t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-19 · MAJOR · 安全 · `mcp/preview.rs:37`

- **原名/来源**：LH-06（audit-logic-hunt.md:227） · 总账 LGC-LG-19
- **标题**：预览副本跟随目录符号链接走出包根且无环/深度守卫
- **现象与判据**：`xirang.apply`（默认预览）把整个包复制到 `std::env::temp_dir()` 下一个临时目录。复制只按**目录项名字**过滤 `target`/`.xirang`，对符号链接既不看解析结果、也不设深度上限： - 包内一个指向外部目录的链接 → 外部整棵树被复制进 temp（实测：错误信息点名了包外的文件）； - 一个自指链接（`ln -s . src/self`）→ 递归到路径长度上限，把同一批文件复制几十份，并且**照样成功**，回复里出现 124 个面、逻辑路径 `root/control` 重复出现多次（实测）。 - 触发条件：`xirang.apply` 且 `apply != true`（即默认预览），包内存在目录符号链接。链接是常见工程事实（monorepo、`node_modules` 式共享目录、Windows junction）。 - 最小复现（实测）： ```sh P=/tmp/nichprobe/pkg mkdir -p /tmp/nichprobe/outside && echo secret > /tmp/nichprobe/outside/secret.txt ln -sfn /tmp/nichprobe/outside $P/src/outside_link python3 /tmp/nichprobe/probe.py xirang.apply \ '{"action":"add","parent":"root","fields":{"module":"probe_add","kind":"ProbeAdd"}}' $P # 实测: cannot copy /tmp/nichprobe/pkg/src/outside_link/secret.txt: Permission denied (os error 13) # ——包内一个链接，把包外的文件拖进了复制路径 ln -sfn . $P/src/self python3 /tmp/nichprobe/probe.py xirang.apply  …（全文见 `audit-findings.json`）
- **最小修复方向**：在复制入口对 `symlink_metadata` 判定为链接的目录选择「跳过并计入报告」，并加深度/环守卫；注意 t16 更正：外链**可读**时 `diff` 看不见该文件（两侧经链接读到同一字节），因此「换个 error 文案」不够，必须改变遍历本身。
- **复核状态**：已复核（证实） · 实测/探针 · t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-20 · MAJOR · 可靠性 · `mcp/apply.rs:84`

- **原名/来源**：LH-07 = BR-19 = LG-07（audit-logic-hunt.md:262；audit-logic-hunt.md:262） · 总账 LGC-LG-20
- **标题**：预览副本的临时路径可预测：同名既有目录会被删除重建（并留泄漏）
- **现象与判据**：`copy_package` 在复制中途失败（LH-06 的权限例子、磁盘满、路径过长）时直接带着错误返回，`apply` 的 `Err` 分支（`mcp/apply.rs:128-135`）根本到不了——那份**半成品包副本**留在 `std::env::temp_dir()` 里。实测留下 `/tmp/xirang-mcp-preview-343377-0`，里面 7 个文件（含 `Cargo.toml`、`src/**`）。 - 触发条件：任何一次预览复制失败；MCP 客户端可以凭 LH-06 的链接稳定触发，反复调用即持续堆积。 - 最小复现（实测）： ```sh # 用 LH-06 的 outside_link（其目标文件不可读）触发复制失败 ls -d /tmp/xirang-mcp-preview-* # 实测: /tmp/xirang-mcp-preview-343377-0 find /tmp/xirang-mcp-preview-343377-0 -type f | wc -l # 7 ``` `mcp/apply.rs:8-11` 与 `mcp/preview.rs:14-20` 的文档把「一次性副本」写成这个设计的安全根据；一次性意味着**无论成功失败都该消失**。同文件里已经为「失败的预览不必留下副本」写了清理（`:128-135`），说明作者知道该清，只是漏了 `copy_package` 自身失败这一支。 - 修复方向：把复制纳入同一个 `Result`-to-cleanup 结构——例如 `copy_package` 失败时自己 `remove_dir_all(destination)` 再返回 Err（destination 已构造出来，指针就在手边），或让 `apply` 用 `let work = copy_package(root).inspect_err(|_| ...)` 形式统一收尾；顺便把 `std::env::temp_dir()` 的副本前缀写进文档。
- **最小修复方向**：临时目录用 `tempfile` 式的不可预测名 + `Drop` 清理；t16 更正：残留缺口比总账写的更宽——`copy_package` 自身失败也会留残留（连 `remove_copy` 都到不了），清理必须覆盖复制失败这一支。
- **复核状态**：已复核（部分证实） · 实测/探针 · t12,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-21 · MAJOR · 其它 · `mcp/apply.rs:172`

- **原名/来源**：NEW-B1（audit-logic-safety-report.md:34） · 总账 LGC-LG-21
- **标题**：`apply edit` 静默忽略 `handle_contracts`/`part_contracts`，而自述与白名单都把它们列为可写
- **现象与判据**：t12/t16 实测：`apply edit` 后 `traits=["Qux"]`（新派生）与 `contracts=[crate::Foo::Bar]`（旧值）并存矛盾；`EDITABLE_FIELDS`（`mcp/apply.rs:172`，:188/:190 列出两个 contract 键）与工具自述都把它们列为可写。
- **最小修复方向**：把 `handle_contracts`/`part_contracts` 纳入 `EDITABLE_FIELDS` 的可写集合并在渲染处一并改写；修前先注意症状比「少了 contract 行」更重：t16 实测 edit 后 `traits=["Qux"]`（新派生）与 `contracts=[crate::Foo::Bar]`（旧值）**并存矛盾**。
- **复核状态**：已复核（证实） · 实测/探针 · t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-22 · MAJOR · 其它 · `mcp/protocol.rs:104`

- **原名/来源**：BR-5（audit-lane-bridges.md:133） · 总账 LGC-LG-22
- **标题**：非 UTF-8 请求帧结束整个桥，而三类同类畸形帧都被作答（与 README「不写 stderr」相反）
- **现象与判据**：`Frame::Line(bytes)` 里 `String::from_utf8(bytes)` 失败会把 `run_with` 整个 `Err` 出去， 于是 `main` 打印 stderr 并 `exit(1)`——会话终止。而同一层对「超长行」是写出 `-32600` 后 `continue`， 对「不是 JSON」「不是对象」是写出 `-32700`/`-32600` 后继续。四类畸形帧里只有一类是致命的。 两份 README 的开篇承诺正好相反：`mcp/README.md:7` “writes nothing to stderr: a failure is an error response on stdout, where the client is already reading”、`mcp/README.zh-CN.md:5` 「**不写 stderr**： 失败是 stdout 上的错误响应」。 `grep -n "utf8\|UTF-8" mcp/protocol_tests.rs` 只在工具函数里出现，**没有**一条钉子覆盖 非 UTF-8 帧；`mcp/protocol_tests.rs:119-137` 只钉了超长行那一类。
- **最小修复方向**：把非 UTF-8 当成与超长行同类的坏帧：写一条 `-32700`（id 为 `null`，说明该行不是 UTF-8） 后 `continue`；「读不了 stdin」这类真传输故障才返回 `Err`（`read_frame` 的 IO 错误保持致命）。
- **复核状态**：已复核（证实） · 实测/探针 · t12,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-23 · MAJOR · 注释 · `mcp/preview.rs:102`

- **原名/来源**：BR-4（audit-lane-bridges.md:109） · 总账 LGC-LG-23
- **标题**：预览 diff 把每个非 UTF-8 文件报成「新增」，无上限，且每次预览复制 `.git`
- **现象与判据**：`diff_package` 先 `let after = std::fs::read_to_string(work.join(&relative)).unwrap_or_default();` （`:108`，读不出的文件变成空串），再对**项目侧**读文本：`Ok(before) if before == after => continue`、 `Ok(before) => "~ …"`、`Err(_) => "+ {relative}"`（`:109-121`）。一个**两边完全相同**的非 UTF-8 文件 两侧 `read_to_string` 都失败，于是落进 `Err(_)` 支，被报成 `+ <path>`——一次没碰过它的编辑会把它列进 「哪些文件会变」。`copy_directory` 只跳过 `target` 与 `.xirang`（`:45-47`），因此整份 `.git` 也在副本里、也在被列举的集合里。 `[实测]` 在本检出上做只读统计：排除 `target/`、`.xirang/` 后有 **36** 个非 UTF-8 文件， 其中 **29** 个在 `.git/` 下（另有 `.git/index`、`.codegraph/codegraph.db`、`.dsh-meow/memory.db` 等）； 即在本仓上跑一次 `xirang.apply` 预览，回复里会出现 36 条虚假的 `+ …`。 同一脚本量出排除 `target` 的树是 46 MB（`.git` 9.9 MB），而每次预览都要复制一遍。 `mcp/apply_tests.rs` 的夹具全是文本文件，所以现有测试看不到这件事。 另外该 diff **没有任何上限**（`apply` 的 schema 里也没有 `limit`），与本桥其它每个答案都带界的做法相反 （`mcp/callgraph.rs:10-15` 的模块文档正是以「代理读不下的答案不算答案」立论）。
- **最小修复方向**：① 项目侧也读不出来时，若两侧原始字节相等就直接 `continue`，不要把「读不出」当成「新增」； 或用 `fs::read` 比字节、只有确认变化才渲染。② `copy_directory`/`collect_files` 跳过 `.git`（以及其它 VCS/工具目录）。 ③ 给 diff 加行数上限并声明出来。
- **复核状态**：已复核（证实） · 实测/探针 · t12,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-24 · MAJOR · 门禁 · `conventions/src/lint.rs:225`

- **原名/来源**：G-01（audit-lane-gates.md:24） · 总账 LGC-LG-24
- **标题**：`lint` 把 `#![deny(warnings)]` 当作「带着 missing_docs」，保护可被静默移除
- **现象与判据**：`missing_roots()` 问的是"这个 crate 根还开着缺失文档 lint 吗"。`#![deny(warnings)]` 被直接判为"开着"。实测 rustc：`missing_docs` 的默认级别是 `allow`，`warnings` lint group 只含默认 `warn` 的 lint，因此 `#![deny(warnings)]` **不**让 `missing_docs` 生效。 - 判据 `[实测]`： - `rustc -W help | grep missing.docs` → `missing-docs allow detects missing documentation for public members`； - `/tmp/lint_probe.rs` = `#![deny(warnings)]` + `pub fn undocumented() {}` → `rustc --crate-type lib` **exit 0，零输出**； - 同一文件换成 `#![warn(missing_docs)]` → 输出 `warning: missing documentation for the crate`。 也就是说，把 `#![warn(missing_docs)]` 换成 `#![deny(warnings)]` 后，`missing_roots` 报空、`no_item_silences_the_lint` 也报空，而 clippy 门禁的 `-D warnings` 同样抓不到任何东西——两道防线一起消失。
- **最小修复方向**：删掉这条特例，只接受 `#![warn(...missing_docs...)` / `#![deny(...missing_docs...)`；若确实想认可 `deny(warnings)`，必须同时要求文本里出现 `missing_docs`。
- **复核状态**：已复核（证实） · 实测/探针 · t13,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-25 · MAJOR · 门禁 · `conventions/src/release_workflow.rs:96`

- **原名/来源**：G-02（audit-lane-gates.md:36） · 总账 LGC-LG-25
- **标题**：发布工作流的 tag 守卫用子串黑名单判否，`== false` 等价否定写法整类放行
- **现象与判据**：门禁要求上传步骤的 `if:` 不含 `!startsWith` / `!=` / `||`，且**包含** `startsWith(github.ref, 'refs/tags/')`。这个 `contains` 组合漏掉"同一守卫的否定式"：`startsWith(github.ref, 'refs/tags/') == false` 含 TAG_GUARD、不含三个否定记号，于是被读成"守卫正确"，而 GitHub 表达式语义下它等价于"只在**非** tag ref 上跑"，即每个分支 push 都会发布。 - 判据 `[读码]`：`conventions/src/release_workflow.rs:96-111` 的四个判定（`contains("!startsWith")`、`contains("!=")`、`contains("||")`、`contains(TAG_GUARD)`）逐条套在条件串 `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/') == false` 上：前三条为假、第四条为真 → 不产生任何 finding。这正是该门禁文档自己记录过的同一类绕过（`conventions/src/release_workflow.rs:16-21`："`!startsWith(...)` passes a `contains` test"），只堵了 `!` 前缀那一种拼法。
- **最小修复方向**：只允许**肯定式形状**：解析 `if:` 表达式，只接受 `startsWith(github.ref, 'refs/tags/')` 这一类可直接判定为真的形状，其余（含 `!(…)`、`! (…)`、`== false`、`== 0`）一律报出。删掉黑名单方向——补 `== false` 救不了（t16 实测四种等价否定全部 0 发现）。
- **复核状态**：已复核（证实） · 实测/探针 · t13,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-26 · MAJOR · 门禁/注释 · `conventions/src/release_workflow.rs:60`

- **原名/来源**：G-13（audit-lane-gates.md:139） · 总账 LGC-LG-26
- **标题**：持有 `CARGO_REGISTRY_TOKEN` 的 job 的 action pin 只由注释承诺，无门禁
- **现象与判据**：该文件自己的注释把 SHA pin 说成防线——"a retagged upstream action would otherwise run before the publish step and could rewrite what that step executes"。但把 `actions/checkout@11d5960a…` 换回 `@v4`、或把 `dtolnay/rust-toolchain@6bed0761…` 换回 `@stable`，`cargo test --workspace` 全绿：没有任何门禁读 `uses:` 的 ref 形态。`ci.yml` 有意保持 tag pin（无秘密），这个差异只存在于注释里，没有任何检查区分两者。 - 判据 `[读码]`：`conventions/src/release_workflow.rs` 的 `step_uploads` 只看命令与 `if:`；`steps()` 解析出的每步 body 里 `uses:` 行只被 `delegated_path` 用来跟进**本地**（`./…`）action。今天两个 ref 都是 40 位十六进制（`.github/workflows/release.yml:54-55`）。
- **最小修复方向**：在 `conventions/src/release_workflow.rs` 加一条：若某 job 的任一步骤带 `CARGO_REGISTRY_TOKEN`（或该 workflow 出现该名字），则同 job 内所有 `uses: <owner>/<repo>@<ref>` 的 `<ref>` 必须是 40 位十六进制；`ci.yml` 不受影响。
- **复核状态**：已复核（证实） · 实测/探针 · t13,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-27 · MAJOR · 大逻辑 · `core/src/registry_core/declaration/owned.rs:280`

- **原名/来源**：K-02（audit-lane-kernel.md:29） · 总账 LGC-LG-27
- **标题**：`merge_authored` 静默忽略作者改过的 `runtime_checks`，热重载把编辑吃掉
- **现象与判据**：`merge_authored(mut self, authored: Self)` 逐字段应用作者侧值，唯独不碰 `runtime_checks`、`contract`、`plugin`。而作者侧快照**确实**解析了 `runtime_checks`（`core/src/registry_core/authoring/snapshot/snapshot.rs:127`；run_method 的字段读入见 `run_method/authoring/manifest/parse/parse.rs:179-180`）。唯一调用点是 `run_method/authoring/operations/operations.rs:352`（`existing.clone().merge_authored(authored)`，即“编译期快照 + 文件快照”）。 `flow` 给出同文件内的对照：作者侧已声明 flow 时会 `self.flow = authored.flow`（`core/src/registry_core/declaration/owned.rs:305-310`），并有注释解释为何 part 列表必须保留编译期证据。`runtime_checks` 与 `flow` 一样已被解析成纯数据（`RuntimeCheckSpec` 枚举），并非只能来自编译期常量，却被整条丢弃且无注释说明这是有意的。结果是：作者改 `runtime_checks` 后热重载行为不变——编辑被静默忽略，而文件里写着新值。
- **最小修复方向**：对 `runtime_checks` 采用与 `flow` 相同的“作者侧非空即应用”规则（`if !authored.runtime_checks.is_empty() { self.runtime_checks = authored.runtime_checks; }`），或在函数文档里明确“运行期校验以编译期为准，文件编辑只在新编译后生效”，并让编辑入口据此拒绝该字段，避免“看起来能改、其实不改”。
- **复核状态**：已复核（证实） · 仅代码阅读 · t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-28 · MAJOR · 函数内谬误 · `core/src/registry_core/authoring/parse/rules.rs:108`

- **原名/来源**：K-06 = BR-10（audit-lane-kernel.md:80；audit-lane-kernel.md:80） · 总账 LGC-LG-28
- **标题**：畸形 `requires` 条目在校验器与快照解析器之间口径不同：一处拒绝、一处静默丢；桥的两个工具再给出两种结论
- **现象与判据**：`parse_requirements_owned` 用 `filter_map(|item| { let (capability, provider) = item.split_once("=>")?; ... })`：缺 `=>` 的条目被直接丢弃且无报告；而 `parse_requirements` 对同一文本返回 `Err("requires entries must use capability=>provider syntax")`。快照构建 `snapshot_from_values` 调用的是**静默版**，没有先跑严格版。 `requires` 是连接器准入的输入（`core/src/registry_core/tree/connector/connector.rs:230-360` 逐条检查提供者与 admission）。丢一条 `requires` 等于该输入不存在：连接器不再检查它，注册静默通过，而作者以为已经声明了依赖。严格校验器只挂在编辑入口（`run_method/authoring/manifest/face/face.rs:131-133`），手写或旧版本写下的文件不经过它。
- **最小修复方向**：删掉 `parse_requirements_owned` 的 `filter_map`，改为 `collect::<Result<Vec<_>, _>>()` 复用 `parse_requirements` 的判据（或先调用 `parse_requirements(value)?`）；`render_requirements`（`core/src/registry_core/authoring/parse/rules.rs:91-104`）同样应拒绝而不是丢弃。
- **复核状态**：已复核（证实） · 实测/探针 · t12,t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-29 · MAJOR · 大逻辑 · `core/src/registry_core/syntax/entries/graft.rs:105`

- **原名/来源**：K-08（audit-lane-kernel.md:104） · 总账 LGC-LG-29
- **标题**：模块级 `cfg` 一律跳过，特性已开启也照跳 → 发布的注册面缺失
- **现象与判据**：```text fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) { if item.attrs.iter().any(|attribute| attribute.path().is_ident("cfg")) { return; } syn::visit::visit_item_mod(self, item); } ``` 判定是“有任意 `cfg` 属性就整棵模块不进入”，理由是注释写的“A module the compiler may drop cannot contribute to the plan”。但条目级的 `cfg` 是**保留**的（同文件 `:121-132` 收集 `cfg`，测试 `graft_parser_ignores_declarations_that_are_not_items` 钉住 `#[cfg(feature = "optional-graft")]` 的条目仍被收集）。于是同一个 `#[cfg(feature = "fast")]`：写在条目上会进入计划，写在 `mod` 上则整模块的 `static_graft_plan!` 被悄悄丢掉——包括 `#[cfg(not(test))]` 这种必然生效的门控。 构建捕获的静态 graft 表是宿主启用 overlay 的唯一来源（`core/src/registry_core/tree/graft_ops/overlay.rs:111-121`、`core/src/registry_core/release/release.rs:196-217`）。少一条切口 = 该槽位静默地跑基座实现，而源码里明明写着要嫁接；没有任何诊断报出这件事。跳过 `#[cfg(test)] mod tests` 是合理目标，但用“任意 cfg”近似它，代价是把真实门控一起丢掉。
- **最小修复方向**：把跳过条件收窄为“编译器可能丢弃”的判定，即只跳 `#[cfg(test)]`（以及 `#[cfg(not(...))]`/特性未开启的情况交给既有的 cfg 求值路径），或在跳过时记录一条报告（该模块里有 N 条切口未被纳入）。
- **复核状态**：已复核（证实） · 仅代码阅读 · t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-30 · MAJOR · 大逻辑 · `core/src/registry_core/syntax/fields.rs:104`

- **原名/来源**：K-09（audit-lane-kernel.md:121） · 总账 LGC-LG-30
- **标题**：trait 标签派生遇带逗号的泛型实参失败 → 构建期误报「缺 trait」
- **现象与判据**：`string_list("handle_traits")` 会先取 `path_list("handle_contracts")`，再把这些路径 `join(",")` 交给 `authoring::parse::trait_names_from_paths` 派生标签。两处都按**裸逗号**切分：`split_top_level`（`core/src/registry_core/syntax/tokens.rs:139-155`）不看尖括号深度，`trait_names_from_paths`（`core/src/registry_core/authoring/parse/parse.rs:205-227`）也按 `,` 切。于是 `handle_contracts: [crate::ControlHandle<u8, u16>]` 被切成 `crate::ControlHandle<u8` 与 `u16>` 两段，`syn::parse2::<syn::Path>` 失败 → `path_list` 返回 `None` → `unwrap_or_default()` 变空 → `trait_labels` 退回 `written_string_list("handle_traits")`（未写标签时为空）。 `trait_names_from_paths` 的注释明确设想“A trait path is a *type path*, so it can carry generic arguments — `Trait<A<B<…>>>`”（`core/src/registry_core/authoring/parse/parse.rs:211-217`），只覆盖了无逗号的嵌套形态；两个 `split(',')` 是这段注释与实现之间的裂缝。下游 `build_method/contracts.rs:146-165` 用 `unwrap_or_default()` 消费它，因此标签为空时构建会报“required handle trait is missin …（全文见 `audit-findings.json`）
- **最小修复方向**：`split_top_level` 增加尖括号深度（`<` 增、`>` 减）后再按顶层逗号切；`trait_names_from_paths` 改为接收 `&[String]`（或先按同一规则切分）而不是吃一个 `join(",")` 的字符串。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t16,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-31 · MAJOR · 可靠性 · `run_method/runtime/trace/artifact/parse.rs:294`

- **原名/来源**：LH-05（audit-logic-hunt.md:197） · 总账 LGC-LG-31
- **标题**：trace artifact 的字符串驻留是进程级无回收 `Box::leak`，长寿命桥可被逐次喂大
- **现象与判据**：解析 trace artifact 时，每个**新见到的**字符串都被 `Box::leak` 成 `'static` 并记进一个进程级 `static` 集合，**永不释放**。进程内的每份 artifact 都共享同一张表；`read_trace_artifact` 是公开入口，MCP 的 `xirang.trace` / `xirang.unified` / trace 驱动的 `xirang.converge` 都按请求读一遍宿主指定的路径。 - 触发条件：长寿命进程 + 每次喂进「词表不同」的 artifact。CLI 一次性进程无害；MCP/Studio（本项目的设计用法）不是。 - 最小复现（实测；RSS 用 `/proc/self/status` 的 `VmRSS`）： ```sh cd /tmp/nichprobe/libleak && cargo run --offline --release ``` 实测（A 组：每轮 10 万个**新**函数名；B 组：每轮同一批名字）： ```text baseline rss_kb=2264 A round 0: rss_kb=13644 (+11380) A round 1: rss_kb=38136 (+35872) A round 2: rss_kb=44704 (+42440) A round 3: rss_kb=51264 (+49000) B round 0: rss_kb=57924 (+6660) B round 1: rss_kb=57924 (+6660) B round 2: rss_kb=57924 (+6660) B round 3: rss_kb=57924 (+6660) ``` A 组每轮单调上升约 6.5 MB/10 万字符串（artifact 本体已被 `drop`），B 组平——差值就是驻留表，而且从不回落。 `run_method/runtime/trace/artifact/parse.rs:289-293` 的注释把这条设计的边界写成「this  …（全文见 `audit-findings.json`）
- **最小修复方向**：见出处 audit-logic-hunt.md:197 的「最小修复方向」段
- **复核状态**：已复核（证实） · 实测/探针 · t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-32 · MAJOR · 假实现/可靠性/注释 · `plugin-host/src/process.rs:341`

- **原名/来源**：LH-08 = BR-20 = BR-C1（audit-logic-hunt.md:279；audit-logic-hunt.md:279；audit-logic-hunt.md:279） · 总账 LGC-LG-32
- **标题**：子进程已给完整帧却不退出时，答案被丢成 `Timeout`（与注释承诺相反）
- **现象与判据**：1. 子进程**已经写完整帧**、然后不退出（收尾慢、等信号、睡眠）时，宿主在 deadline 处返回 `Err(Timeout)`，**把手里已收到的帧丢掉**。`plugin-host/src/process/child.rs:119-127` 的注释把这件事描述成已被修掉的缺陷（「当它阻塞时，宿主会等一个不可能到来的退出、在超时点杀掉子进程并报出超时——同时丢掉它其实已经拿到的答案」），修的是「管道满导致阻塞」这一子类，而注释描述的是**整个类**； 2. 子进程写了一帧**不完整**的内容后正常退出（status 0）时，调用方拿到的是 `HostError::Io(UnexpectedEof, "failed to fill whole buffer")`——既没说是哪个插件、哪个操作，也没说是帧坏了（`read_frame` 的 `read_exact` 经 `From<io::Error>` 升格，`plugin-host/src/process.rs:392` 的 `refusal` 原样返回）。 - 触发条件：任何「先作答、后不退出」的插件；任何写坏帧的插件。 - 最小复现（实测，`/tmp/nichprobe/procprobe`，直接调 `ProcessBackend`（`process-tools`））： ```sh cd /tmp/nichprobe/procprobe && cargo run --offline --release # 实测: # answers-then-lingers: Err(Timeout) after 303.936524ms # 帧已送达（6 字节 "answer"） # partial frame, exit 0: Err(Io(Error { kind: UnexpectedEof, message: "failed to fill whole buffer" })) ``` `plugin-host/src/process.rs:247-283` 的注释与 `plugin-host/src/process …（全文见 `audit-findings.json`）
- **最小修复方向**：见出处 audit-logic-hunt.md:279 的「最小修复方向」段
- **复核状态**：已复核（证实） · 实测/探针 · t12,t16 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-38 · MAJOR · 假实现/边界 · `run_method/authoring/manifest/face/render.rs:22`

- **原名/来源**：S6（audit-lane-surfaces.md:125） · 总账 LGC-LG-38
- **标题**：字段集合的真值
- **现象与判据**：`FaceManifest.values` 同时装着三类东西：(a) 渲染器会写回文件的字段，(b) 派生元数据（`namespace`/`parent_node`/`parent_kind`/`provided_parts`/`required_parts`），(c) 编辑器声称可改、但模板从不发射的字段（`registry_name`/`handle`/`params`）。三者没有任何标记区分，维护者无法从代码看出"哪些字段真的会落盘"。 - `plugin` 这一支确立了本仓库自己的标准：**重写无法复现的字段要响亮拒绝，不许静默删掉**（`run_method/authoring/manifest/face/render.rs:13`～`run_method/authoring/manifest/face/render.rs:20` 的注释正是这么写的）。同一个模板对 `registry_name`/`handle`/`params` 没有任何对应保护。 - `handle` 这一支可以确定是死代码：字段词表由 `core/src/registry_core/syntax/tokens.rs:125` 的 `FACE_FIELD_ORDER` 把关，`handle` 不在词表里，所以 `face.field("handle")` 永远拿不到值，`run_method/authoring/manifest/parse/parse.rs:109` 插进去的必定等于 `kind`；而 `registry_name`/`params` 在任何 `edit()` 调用点都不可达（两个字段顺序表 `run_method/authoring/operations/face_write.rs:32`～`run_method/authoring/operations/face_write.rs:80` 都不含它们）。结论：这三个白名单项是**永远无法生效的契约**，而不是被使用的能力。 - 幂等性测试覆盖不到这一点：它用 `BTreeMap …（全文见 `audit-findings.json`）
- **最小修复方向**：二选一并写清：要么让 `render_source` 发射 `edit()`/解析接受的每个字段（并给解析→应用→渲染加一条真实文件往返测试），要么把 `registry_name`/`handle`/`params` 从 `edit()` 白名单、`parse` 与 `FaceManifest::new` 里删掉，并把"派生元数据"键和"可落盘字段"键用不同命名（例如统一前缀）区分开。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t10 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-43 · MAJOR · 工具 · `mcp/tools.rs:77`

- **原名/来源**：BR-3（audit-lane-bridges.md:92） · 总账 LGC-LG-43
- **标题**：`xirang.callgraph` 的参数契约
- **现象与判据**：catalog 给 `xirang.callgraph` 声明的 `inputSchema.properties` 只有 `function` / `path` / `root`， 描述是「Show direct static callers and callees for one function.」；实现却读 `limit` （`value.clamp(1, 50)`，默认 5 个定义），并在截断行里主动叫调用方 `… +N more definitions (raise \`limit\` or pass \`path\`)`。按 `inputSchema` 校验参数的 MCP 客户端 无法发出这个参数，于是那句提示是**不可执行**的；反过来，一个未声明的键会静默改变答案长度。 `mcp/tools_tests.rs:40-59` 逐项断言 explain/diff/trace/impact/grafts 的窄化参数**必须**被声明， 唯独没覆盖 `callgraph` 的 `limit`——这正是一条「空转的名单钉子」。
- **最小修复方向**：在 catalog 里补 `"limit":{"type":"integer","minimum":1,"maximum":50}`，描述里也点名它； 或删掉 `limit` 读取（只留 `path`）。两条都要顺手把 `callgraph` 加进 `the_evidence_tools_are_advertised_…` 那张表，让「声明了才准读」变成一条可执行规则。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t12 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-50 · MAJOR · 其它 · `studio/studio/app/graft.rs:127`

- **原名/来源**：S-07 = S-24（audit-lane-studio.md:71；audit-lane-studio.md:192） · 总账 LGC-LG-50
- **标题**：Studio 错误吞掉
- **现象与判据**：`submit_graft` 先 `self.open_editor_file(path.clone(), 1)`（`studio/studio/app/graft.rs:128`），紧接着**无条件**`self.event = match &state.declaration { … }`（`studio/studio/app/graft.rs:129-158`）。而 `open_editor_file`（`studio/studio/app/support.rs:459-465`）在路径不是文件时会写 `self.event = "Editor failed: source file not found at …"`。 错误消息被成功横幅覆盖，且 `editor_request` 保持 `None`，用户既看不到失败也不知道没打开编辑器；`event` 是本界面唯一的反馈通道（`studio/studio/ui/status.rs:6-33`）。
- **最小修复方向**：把编辑器结果纳入 match（或在 `open_editor_file` 返回 `Result`），失败时不做覆盖。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11 · **批次** B4-MAJOR 逻辑与实现缺陷

#### LGC-LG-53 · MAJOR · 门禁 · `conventions/src/doc_blocks.rs:133`

- **原名/来源**：G-06 = G-23（audit-lane-gates.md:72；audit-lane-gates.md:224） · 总账 LGC-LG-53
- **标题**：门禁豁免范围
- **现象与判据**：`docs/audit-*.md` 豁免，但 `docs/audit-2026-09-28/audit-lane-kernel.md` 不豁免——它的文件名是 `audit-lane-kernel.md`。于是本轮审计的报告目录整体被当**活文档**两向扫描：`doc_blocks` 要求每条 rust 围栏能解析，`doc_anchors` 要求每条 `.rs:NNN` 此刻可解析（含配对 token 必须落在所引行区间内）。审计记录里摘录 Rust 片段、写"当时的位置"是常态，因此这条边界会把"记录"逼成"活文档"，或逼人删掉锚点。 - 判据 `[实测]`：同一份报告文件双向证明——旧名 `audit-lane-gates.md` 时，`documented_rust_blocks_parse` 与 `the_shipped_documentation_anchors_resolve` 都点名它（前者报 `docs/audit-2026-09-28/audit-lane-gates.md:3`，后者报 3 条：`no such file`、`line 99999 is past the end … (508 lines)`、配对 token `has_cjk` 不在所引行上）；改名 `audit-lane-gates.md` 后两条命令对它的发现归零。同一时刻同目录 peer 报告也各自产生发现（`audit-lane-kernel.md` 1 条、`audit-lane-studio.md` 9 条）。含目录名的形态 `audit-2026-09-28/` 不参与判定。
- **最小修复方向**：豁免按**目录或显式清单**判定，而不是文件名前缀；本轮审计报告全部改名 `audit-*` 后同一条门禁复绿，正是这条规则被现场触发的证据。
- **复核状态**：已复核（证实） · 实测/探针 · t13 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-C-01 · MAJOR · 注释 · `studio/studio/app/navigation.rs:165`

- **原名/来源**：C-01（audit-lane-studio.md:248）
- **标题**：注释与实现不符 · `studio/studio/app/navigation.rs:165-171`
- **现象与判据**：`retreat_graph_focus` 的注释写 “BackTab walks the same two panes the other way. / BackTab 以相反方向走同样这两块面板。”（`studio/studio/app/navigation.rs:166-167`），而函数体与 `advance_graph_focus`（`studio/studio/app/navigation.rs:155-163`）逐字节相同——方向与函数名无关。另见 `studio/studio/app/navigation.rs:155-171` 两个函数上方各自的 “Two panes…” 注释，都把两面板当三段式在描述。 判据 ①。注释描述的行为在实现里不存在：读者据此会以为 BackTab 有独立语义，改动时只改一处，另一处静默漂移（`studio/studio/app/overlay/search.rs:41-42` 确实两条键都绑在这对函数上）。
- **最小修复方向**：合并为 `toggle_graph_focus` 并同步注释；若要保留两个入口，就把注释改成“两块面板下前进与后退等价，保留两个名字只为键盘映射可读”。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-C-02 · MAJOR · 注释 · `studio/studio/app/writers.rs:1`

- **原名/来源**：C-02（audit-lane-studio.md:255）
- **标题**：注释与实现不符（模块自述） · `studio/studio/app/writers.rs:1-22`
- **现象与判据**：模块文档承诺 “Write guards: every operation that creates, rewrites, moves, or deletes something in the project the reader opened. / 写入守卫：每一个在读者打开的项目里创建、重写、移动或删除东西的操作。”，实现里只有两个不写盘的守卫（`studio/studio/app/writers.rs:30-33`、`studio/studio/app/writers.rs:37-42`）；实际写入在 `studio/studio/app/mutations.rs` 中（`studio/studio/app/mutations.rs:51-56`、`studio/studio/app/mutations.rs:137-149`、`studio/studio/app/mutations.rs:185-210` 与手写的 `studio/studio/app/mutations.rs:259`、`studio/studio/app/mutations.rs:319`、`studio/studio/app/mutations.rs:333-338`）。 判据 ①。按文档去 `studio/studio/app/writers.rs` 找写盘点会空手而归；写盘责任没有落点正是 S-01（CRITICAL）的成因。
- **最小修复方向**：文档先改成事实（“读取上下文与写作用域守卫；写入见 `studio/studio/app/mutations.rs`”），或按 S-14 的最小修复方向把写入搬进来，文档与之合一。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-C-03 · MAJOR · 注释 · `studio/studio/app/search_queries.rs:9`

- **原名/来源**：C-03（audit-lane-studio.md:262）
- **标题**：注释与实现不符 · `studio/studio/app/search_queries.rs:9-17`
- **现象与判据**：`search_rows` 的文档写 “Flat, deduplicated rows for one search query. / 一次搜索查询的扁平、已去重结果行。”而实现用 `dedup_by`（`studio/studio/app/search_queries.rs:20-23`），只消**相邻**重复。 判据 ①。今天没有可复现输入（在 S-06 里已说明：每行最多进一次符号循环），但文档承诺的保证强于代码——“不会有跨行/跨深度重复”是后来者会据此建立的假设。
- **最小修复方向**：文档改成 “adjacent duplicates collapsed”，或把实现改成真正的全量去重（`BTreeSet` 键）。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-C-04 · MAJOR · 注释 · `studio/studio/app/support.rs:1`

- **原名/来源**：C-04（audit-lane-studio.md:269）
- **标题**：注释范围与实现不符 · `studio/studio/app/support.rs:1-2`
- **现象与判据**：模块文档只声明 “Shared interaction geometry and editor helpers. / 交互几何与编辑器辅助。”，而 509 行里有约 220 行是别的东西：项目解析（`studio/studio/app/support.rs:157-247`）与 `cargo` 子进程（`studio/studio/app/support.rs:277-374`）。 判据 ①的弱形式——不是行为写错，而是**范围声明不全**，同样让读者误判这个模块的依赖面（谁 import 它就会连带引入 `std::process` 与 `std::env`）。
- **最小修复方向**：按 S-15 拆成 `project`/`cargo_probe`/`geometry`，或把文档标题改成实际的三块内容。
- **复核状态**：已复核（部分证实） · 复核报告（手段见出处） · t11,t9 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-S-02 · MAJOR · 一致性/逻辑 · `studio/studio/app/lifecycle.rs:152`

- **原名/来源**：S-02（audit-lane-studio.md:33）
- **标题**：逻辑/一致性 · `studio/studio/app/lifecycle.rs:152-158` × `studio/studio/ui/panels.rs:119-159`
- **现象与判据**：检视器行数有两个真值。`detail_field_count()` 返回 **14**（有选中面）/ 2（根），而渲染器 `draw_details` 的 `Some` 分支实际画出 **12** 行：name、kind、node、path、parent、preset / parts、handle interfaces、parts interfaces、declared、registration rule、dependency admission、exports（`studio/studio/ui/panels.rs:119-153`；`None` 分支 2 行，与返回值一致）。三个消费点都用 14 当上界：`studio/studio/app/keyboard.rs:150-151`（↓）、`studio/studio/app/pointer.rs:87-88`（滚轮）、`studio/studio/app/pointer.rs:117-118`（点击），渲染侧只在 `studio/studio/ui/panels.rs:161` 自钳一次。 选中一个注册面后连按两次 ↓（或滚两次、或点第 13 行）会把 `details_selected` 推到 12/13，画面上不再有任何高亮——按键有效、界面无反应，且真实行数一变（加一行）就立刻扩大错位。两份行清单必须是同一个来源。
- **最小修复方向**：抽出 `fn detail_rows(app: &App) -> Vec<(&'static str, String)>`，`draw_details` 与 `detail_field_count()` 都从它取 `len()`；顺带把 `studio/studio/app/keyboard.rs`/`studio/studio/app/pointer.rs` 的 `detail_field_count() - 1` 统一成 `saturating_sub(1)`（现已如此）。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-S-03 · MAJOR · 逻辑 · `studio/studio/app/lifecycle.rs:108`

- **原名/来源**：S-03（audit-lane-studio.md:40）
- **标题**：逻辑/证据诚实 · `studio/studio/app/lifecycle.rs:108-183`（配合 `studio/studio/app/trace.rs:56-89`）
- **现象与判据**：`install_trace()` 只在 `App::load()` 调一次（`studio/studio/app/lifecycle.rs:90`），身份检查（namespace、root、每个记录节点能否在本快照解析）只在那时跑。此后 `poll_hot_reload`（`studio/studio/app/lifecycle.rs:108-133`）与 `reload`（`studio/studio/app/lifecycle.rs:162-183`）**整体替换 registry，但不重跑 `trace_mismatch`/`unresolved_nodes`，也不清 `mir_graph`**。`mir_graph` 只在 `studio/studio/app/lifecycle.rs:213` 被写、在 `studio/studio/app/graph_queries.rs:132/172` 与 `studio/studio/app/app.rs:153` 被读。 这两条替换路径会改变「本快照有哪些节点」，而 trace 与 MIR 的有效性前提正是「记录指向的节点仍在本快照」——`studio/studio/app/trace.rs:97-105` 自己把这条称为唯一真正的“不同构建”检测器，`studio/studio/app/trace.rs:152-170` 又把 `Loaded` 渲染成 `LIVE`。于是一次外部编辑删掉一个被记录的注册面、或一次 `rebuild` 之后的 hot reload，会话会继续宣称 `LIVE`、继续用旧 MIR 给边标 `? compiler-only`，而 `push_call_ref` 会产出 `file: ""` 的引用。这正是身份闸门要防的“宣称并不持有的证据”，只是从 reload 这个侧门重新打开。
- **最小修复方向**：把「registry 被替换」收敛成一个函数（`reload` 与 `poll_hot_reload` 都走它），在其中重跑 `read_trace` 的判定（或至少 `unresolved_nodes` 非空即降级为 `Mismatch`）并按需清 `mir_graph`。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-S-05 · MAJOR · 性能 · `studio/studio/app/search_queries.rs:18`

- **原名/来源**：S-05（audit-lane-studio.md:54）
- **标题**：性能/架构 · `studio/studio/app/search_queries.rs:18-136`，调用点 `studio/studio/ui/search/results.rs:29`、`studio/studio/ui/search/detail.rs:15/163`、`studio/studio/ui/graph.rs:102`、`studio/studio/app/pointer.rs:283`、`studio/studio/app/overlay/search.rs:146`
- **现象与判据**：`search_rows(query)` 每次调用都遍历整棵注册树、对每个节点的源文件 `read_to_string`（`studio/studio/app/search_queries.rs:41`）并对全文做 `function_symbols` 词法扫描（`studio/studio/app/search_queries.rs:55`）与逐行前缀扫描（`studio/studio/app/search_queries.rs:74-133`）。它没有备忘：`call_tree_view` 有 `CallTreeMemo`（按 `last_source_stamp` 失效），搜索侧什么都没有。列表模式下**一帧至少三次**调用（结果列表、RELATION 栏、SELECTED SYMBOL 预览），数据页在 `studio/studio/ui/graph.rs:102` 还有一次，按键路径 `studio/studio/app/overlay/search.rs:146`（↓）与点击路径 `studio/studio/app/pointer.rs:283` 各再一次。 这是 TUI 事件循环里的同步 I/O + 词法重扫，随工程规模线性增长且与帧率绑定；“一帧问同一件事三次”在代码里是可见的事实，而不是推测。规模上它和 `CallTreeMemo`（`studio/studio/app/call_tree_queries.rs:74-101` 注释明确写了“一帧会问它好几次”）是同一类问题，只是搜索侧没解决。
- **最小修复方向**：按 `(query, last_source_stamp)` 备忘（复用 `CallTreeMemo` 的形状），或每次 reload 建立符号索引（`NodeId → Vec<SearchRow>`），一帧只查一次；`ui` 三个面板共享同一份 `rows` 而不是各自调 `search_rows`。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-S-06 · MAJOR · 逻辑/重复 · `studio/studio/app/search_queries.rs:74`

- **原名/来源**：S-06（audit-lane-studio.md:61）
- **标题**：逻辑/重复真值 · `studio/studio/app/search_queries.rs:74-133` vs `core/src/registry_core/source/source.rs:120`
- **现象与判据**：`source_symbol_rows` 第二段循环自己实现了一套“什么是符号”的识别：13 个 `strip_prefix`（`pub fn `/`fn `/`pub async fn `/…/`const `/`let `，`studio/studio/app/search_queries.rs:82-95`），然后按 `( { : = whitespace` 截名。同一函数上面已经用内核 `function_symbols` 拿到了函数清单（`studio/studio/app/search_queries.rs:55`），并只把「已是函数声明行」的行跳过（`studio/studio/app/search_queries.rs:75-79`）。 仓库自己立的规则就在隔壁——`studio/studio/app/state/state.rs:32-37` 写了“内核拥有唯一的 `::` 边界匹配规则，Studio 曾带一份单向副本……这会让它画出的图与内核归并的证据不一致”。这里是同一错误的另一种形态：两份“符号清单”会漂移，而且两个方向都实测存在—— - **多**：`let` 绑定被当成符号（`studio/studio/app/search_queries.rs:95`），并由 `studio/studio/ui/search/results.rs:38-42` 用与函数相同的 `ƒ` 标记渲染、按 `Enter` 可直接推进调用图（`studio/studio/app/overlay/search.rs:149-176`）——语义上它不是可跳转的符号，`call_relations` 也找不到同名函数。 - **漏**：13 个前缀不含 `pub(crate)`/`pub(super)`/`unsafe fn`（`studio/studio/app/search_queries.rs:82-95`），而内核认这些（`studio/studio/app/tests/source.rs:2 …（全文见 `audit-findings.json`）
- **最小修复方向**：让内核 `source` 提供一份“声明/符号”清单（或复用它已有的 `function_symbols` 加上独立的 `item_symbols`），Studio 只做渲染与去重。
- **复核状态**：已复核（部分证实） · 复核报告（手段见出处） · t11 · **批次** B4-MAJOR 逻辑与实现缺陷

#### STU-S-08 · MAJOR · 逻辑 · `studio/studio/app/mutations.rs:320`

- **原名/来源**：S-08（audit-lane-studio.md:78）
- **标题**：逻辑/写盘闸门 · `studio/studio/app/mutations.rs:320`
- **现象与判据**：入口行是否已存在用**全文字串**判断：`if !entry_text.contains(&format!("use {crate_name} as _;"))`（`studio/studio/app/mutations.rs:320`）。 注释、块注释或字符串里的同一行会让判断成立，于是跳过 append，随后只写锁——`studio/studio/app/mutations.rs:347` 报 “Plugin selected”，而宿主入口并没有 `use <crate> as _;`，该 crate 不进宿主构建。写盘成功 + 语义没达成的静默不一致，正是这个写入方存在的理由（`studio/studio/app/mutations.rs:326-331` 为“半写”设计了回滚，而这一条根本不会触发回滚）。
- **最小修复方向**：按行判断（`entry_text.lines().any(|line| line.trim() == format!("use {crate_name} as _;"))`），或干脆把入口写入也交给内核/执行面里唯一的写入器。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t11 · **批次** B4-MAJOR 逻辑与实现缺陷

#### SUR-C1 · MAJOR · 文档 · `macro/src/front_end.rs:18`

- **原名/来源**：C1（audit-lane-surfaces.md:338）
- **标题**：文档块与声明错位：`split_semicolons` 的文档挂到了 `splice` 上
- **现象与判据**：第一个文档块与第二个文档块之间**没有任何条目**，两个块在同一个 `pub(crate) fn splice` 上合并成一段文档：`splice` 的文档里前半段讲"在顶层 `;` 处切分 token 流"，而 `split_semicolons` 本身没有任何文档（它是 `pub(crate)`，`missing_docs` 看不见）。 直接读文件即可验证：`macro/src/front_end.rs:26` 起紧接 `macro/src/front_end.rs:37` 的 `pub(crate) fn splice(`，中间没有第三个条目；`macro/src/front_end.rs:57` 的 `pub(crate) fn split_semicolons` 上方是空行，没有 `///`。**最小修复方向** 把第一个文档块移回 `split_semicolons` 正上方。**复核手段** `cargo doc -p xirang-macro --offline` 或直接 `read` 这一段；若加上 `#![warn(missing_docs)]` 之外的内部文档门禁（conventions 已能扫 doc 注释），这类错位会被自动抓到。
- **最小修复方向**：见出处 audit-lane-surfaces.md:338 的「最小修复方向」段
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t10 · **批次** B4-MAJOR 逻辑与实现缺陷

#### SUR-C2 · MAJOR · 文档 · `build_method/discovery.rs:234`

- **原名/来源**：C2（audit-lane-surfaces.md:346）
- **标题**：同型错位：`collect_rust_sources` 的文档挂在 `StdSourceTree` 上
- **现象与判据**："Every `.rs` file under `directory`, following the crate's own layout." 与"`directory` 下的每个 `.rs` 文件…"这两行描述的是 `collect_rust_sources`，却位于 `struct StdSourceTree;` 之前；`StdSourceTree` 自己的说明（`build_method/discovery.rs:236`～`build_method/discovery.rs:237`）紧跟其后，于是结构体带着一段讲函数的文档，而函数裸着。**判据** 与 C1 同一机制，两处并存说明这是**习惯性**问题而非偶发。**最小修复方向** 把第一组文档移到 `collect_rust_sources` 前。**复核手段** 同上。
- **最小修复方向**：见出处 audit-lane-surfaces.md:346 的「最小修复方向」段
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t10 · **批次** B4-MAJOR 逻辑与实现缺陷

#### SUR-C4 · MAJOR · 契约 · `run_method/macros/face.rs:139`

- **原名/来源**：C4（audit-lane-surfaces.md:358）
- **标题**：编辑器悬停文本与宏契约不符：`parent` 被标成 Required
- **现象与判据**：唯一一条匹配臂让 `parent:` 变成可选，省略时解析到包根（`__face_expr_or!($crate::root_node_id(env!("CARGO_PKG_NAME")); $($parent)?)`）。`FaceFields` 的字段文档正是编辑器的补全说明（`run_method/macros/face.rs:16`～`run_method/macros/face.rs:19` 自己说明了这一点），所以作者会被告知一个不存在的必填项。**判据** 该结构体的每个字段都要求"含义、默认值、一个示例"，`parent` 这一项既没写默认值，又把"必填"写进了含义。**最小修复方向** 改成"省略时取包根"并给出示例。**复核手段** 对照 `run_method/tests/face_arm_defaults.rs` 与 `run_method/tests/kind_only_registry_name.rs` 已钉住的默认值行为。
- **最小修复方向**：见出处 audit-lane-surfaces.md:358 的「最小修复方向」段
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t10,t12 · **批次** B4-MAJOR 逻辑与实现缺陷

#### SUR-C5 · MAJOR · 注释 · `run_method/macros/entry.rs:18`

- **原名/来源**：C5（audit-lane-surfaces.md:364）
- **标题**：术语不一致：注释里出现两个不存在的 crate 名
- **现象与判据**：**file:line** `run_method/macros/entry.rs:18`、`run_method/macros/entry.rs:24`、`run_method/macros/entry.rs:63`（`xirang-build`）；`run_method/macros/face_registration.rs:169`～`run_method/macros/face_registration.rs:170`（`xirang-debug`） **现象/判据** 真实 crate 名是 `xirang-build-method` 与 `xirang-debug-method`（`build_method/Cargo.toml:2`、`debug_method/Cargo.toml:2`），lib 名分别是 `nichl
- **最小修复方向**：见出处 audit-lane-surfaces.md:364 的「最小修复方向」段
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t10,t12 · **批次** B4-MAJOR 逻辑与实现缺陷

#### SUR-S4 · MAJOR · 边界 · `build_method/manifests.rs:111`

- **原名/来源**：S4（audit-lane-surfaces.md:82）
- **标题**：边界（同一规则两处实现） · `function_manifest.tsv` 用第二套函数扫描器，内核已有词法扫描器
- **现象与判据**："这个文件声明了哪些函数"这一个问题有两份答案。构建期这份是按行文本扫描：`fn ` 出现在注释、文档示例、字符串里都会命中；impl 区块的归属靠"上一行以 `impl ` 开头且不以 `}` 结尾"来猜（`build_method/manifests.rs:127`～`build_method/manifests.rs:137`），块内嵌套的 `impl`/闭包会污染归属。内核那份是词法扫描，两个工具面都在用。 - 两份实现的存在可以直接对照：`core/src/registry_core/source/source.rs:120` 是 `pub fn function_symbols(source: &str)`，而 `build_method/manifests.rs:164` 自己又写了一个 `function_name(line)`。 - 这份产物的**树内读者为零**：`grep -rn function_manifest` 只命中写入侧（`build_method/pipeline.rs:6`、`build_method/pipeline.rs:176`、`build_method/lib.rs:121`、`build_method/manifests.rs:24`、`build_method/manifests.rs:31`），CLI/MCP/Studio 都不读它。也就是说分叉不会马上显现，只会等到树外读者信任它时暴露。 - 该产物的内容是会被别的面报告的（既有的 3p 审计 KN5 就是拿 `function_manifest.tsv` 举例说明错误符号会流进构建产物），所以它不是可以放着不管的死产物。
- **最小修复方向**：调用 `xirang::source::function_symbols`（它已经处理了标识符边界与 impl 归属），删掉本地扫描；如果确认无人读这份产物，就在同一轮里删掉写入，别留一个会分叉的第二答案。
- **复核状态**：已复核（部分证实） · 复核报告（手段见出处） · t10 · **批次** B4-MAJOR 逻辑与实现缺陷

#### NAM-01 · MAJOR · 命名/A · `studio/studio/app/support.rs:1`

- **原名/来源**：NAM-01（audit-naming-review.md:45）
- **标题**：A① 类别名而不是东西名 · `studio/studio/app/support.rs`
- **现象与判据**：文件 509 行、23 个函数。模块文档第 1 行写「Shared interaction geometry and editor helpers」（共享交互几何与编辑器辅助），但前 12 个 item 是**项目上下文与 cargo 子进程**：`select_project`（:23）、`package_namespace`（:49）、`with_authoring_context`（:70）、`selected_root`（:82）、`clear_project_context`（:100）、`package_root`（:111）、`resolve_project`（:157）、`host_manifest`（:251）、`cargo_rustc_mir`（:277）；后面才是 TUI 几何与编辑器（`near_divider`:379、`resize_graph_split`:389、`open_editor_file`:459）。 三类互不相关的主语（项目/命名空间环境、cargo 调用、TUI 几何拖动）挂在一个叫 `support` 的名字下；`support` 不表达任何一类。读者要找「谁在解析项目根」不会打开 `studio/studio/app/support.rs`（我在 t8 分析 `package_root` 时就必须先 grep）。名字与内容不符 → MAJOR。
- **最小修复方向**：`studio/studio/app/support.rs` → `studio/studio/app/project_context.rs`；把几何相关（`near_divider`/`near_graph_divider`/`resize_graph_split`/`resize_split`/`move_selection`/`toggle_selected`/`selected_parent`）并入已有的 `studio/studio/app/hot_zones.rs`（该文件本身就叫热区）；`open_editor_at`/`open_editor_file`/`graph_locals` 归入 `ui/` 的编辑器入口或新建 `studio/studio/app/editor_launch.rs`。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t27 · **批次** B7-命名抽象与可读性

#### NAM-03 · MAJOR · 命名/A · `run_method/authoring/validation/validation.rs:1`

- **原名/来源**：NAM-03（audit-naming-review.md:77）
- **标题**：A③ 名与内容不符（误导 → MAJOR） · `run_method/authoring/validation/validation.rs`
- **现象与判据**：文件叫 `validation`，内容是 `AuthoringContext`（:47 的 `new(package_root, namespace)`、:56 的 `scope(...)`）与**进程级环境访问器** `authoring_namespace()`（:71）、`package_root()`（:99）、`source_root()`（:120）、`legacy_rule_path_for_source()`（:91）。 这是本仓 authoring 的**执行上下文与全局状态**实现，不是校验逻辑（真正的校验在 `core/src/registry_core/authoring/validation/validation.rs` 与 `run_method/authoring/manifest/parse`）。读者按名字找校验规则会打开错的文件；反过来，想知道「`package_root()` 是谁设的、作用域何时恢复」的人在 `validation` 下也想不到。名字与内容不符 → MAJOR。
- **最小修复方向**：`run_method/authoring/validation/validation.rs` → `run_method/authoring/context.rs`（或 `authoring_context.rs`），模块名 `validation` → `context`。 - **⚠️ 这是公开路径变更（t27 补的漏项）**：`conventions/src/shims.rs:50` 把 `run_method/authoring/validation/validation.rs` 的重导出 `pub use xirang::authoring::validation::*;` 钉在 `SHIMS` 棘轮里——也就是说 `xirang_run_method::authoring::validation` 是**下游可写的公开路径**。同一张表（`:27-80`）还钉着 `run_method/authoring/parse/parse.rs`、`run_method/authoring/face_manifest.rs`、`run_method/runtime/evidence.rs`、`run_method/runtime/runtime.rs`、`run_method/runtime/trace/trace.rs`、`run_method/plugin/plugin.rs`、`run_method/registry/registry.rs`、`run_method/lib …（全文见 `audit-findings.json`）
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t27 · **批次** B7-命名抽象与可读性

#### NAM-04 · MAJOR · 命名/A · `build_method/cache.rs:1`

- **原名/来源**：NAM-04（audit-naming-review.md:84）
- **标题**：A③ 名与内容不符（误导 → MAJOR） · `build_method/cache.rs`
- **现象与判据**：文件叫 `cache`，8 个函数里只有一半是缓存：`update_discovery_cache`（:77）、`cached_parent_id`（:184）、`collect_discovery_rows`（:205）；另一半是**作用域判定**与**通用写盘**：`write_if_changed`（:36，全库唯一的通用「变了才写」）、`collect_active_ids`（:44）、`source_is_active`（:216）、`face_source_is_active`（:235）、`module_feature`（:246）。 `source_is_active`/`face_source_is_active` 决定**哪些面进入发布作用域**，是构建语义的核心判定，却住在一个叫 cache 的模块里；`write_if_changed` 是通用 IO（还被 `build_method/pipeline.rs` 的清单写入复用，见 `audit-lane-surfaces.md` 的 S2）。读者按「作用域」找会漏掉这里，按「cache」找到的却是一半作用域逻辑 → MAJOR。
- **最小修复方向**：`build_method/cache.rs` → `discovery_cache.rs`（只留三个缓存函数）；`write_if_changed` → 新建 `run_method/runtime/trace/artifact/io.rs`（或并入已有 `build_method/manifests.rs`，它才是主要调用方）；`collect_active_ids`/`source_is_active`/`face_source_is_active` → `build_method/scope.rs`（该文件已经在管作用域）。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t27 · **批次** B7-命名抽象与可读性

#### NAM-30 · MAJOR · 命名/B · `build_method/manifests.rs:208`

- **原名/来源**：NAM-30（audit-naming-review.md:260）
- **标题**：B③ 名与行为不符（MAJOR） · `build_method/manifests.rs::parse_pruning_item` / `collect_pruning_symbols`
- **现象与判据**：`collect_pruning_symbols`（:173）说要收集"剪枝符号"，实际对每一行做魔法标识符匹配；`parse_pruning_item`（:200-213）的第三个分支**无条件返回字面量** `"Button::optional_pruning_probe"`（:210），不管当前是哪个面。 名字承诺"收集源码声明的符号"，行为会**发明**一个与当前面无关的符号写进 `pruning_manifest.tsv`（发布剪枝前读的证据文件）。这是"名与行为不符"的最强形态（名字说收集、行为说伪造）。我的 t15 已单独记为 LG-15（假实现），此处从命名角度补记：`collect_*` 这个名字本身就是错的。
- **最小修复方向**：要么让符号列真的从已解析的注册面/静态计划读（推荐），要么在修之前把函数名改成它实际做的事（`scan_pruning_probe_markers`）——但**名字改对不等于行为可接受**，行为要按 LG-15 修。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t27 · **批次** B7-命名抽象与可读性

#### NAM-31 · MAJOR · 命名/B · `run_method/authoring/operations/face_write.rs:1`

- **原名/来源**：NAM-31（audit-naming-review.md:266）
- **标题**：B③ 名与行为不符（MAJOR） · `run_method/authoring/operations/face_write.rs::apply_trait_contract`
- **现象与判据**：函数名说"应用 trait 契约"；实现（:190-205）在 `labels` 与 `contract_paths` 都空时直接 `return Ok(())`，否则只写**trait 标签**（`face.edit(label_field, …)`）；真正的 `handle_contracts:` 行由**创建**路径的 `CREATE_FIELD_ORDER` 写，编辑路径的 `EDIT_FIELD_ORDER`（:59-79）**不含** `handle_contracts`/`part_contracts`。 名字承诺"契约"，行为只落"标签"；配合工具自述把这两个键列为可编辑（`mcp/apply.rs:172-196`），于是 `apply edit` 回"成功"而契约不写（我 t15 记为 LG-21 MAJOR）。名字在这里不是唯一缺陷，但它让维护者以为编辑路径会写契约。
- **最小修复方向**：`apply_trait_contract` → `apply_trait_label`（并把契约行的归属写进注释）；行为按 LG-21 修（把两个字段加进 edit 序，或在 edit 路径明确拒绝）。
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t27 · **批次** B7-命名抽象与可读性

#### AMB-01 · MAJOR · 命名歧义/类别1（生态保留名） · `core/src/registry_core/identity/node_id.rs:12`

- **原名/来源**：AMB-01（audit-naming-ambiguity.md:64）
- **标题**：`core/`（+ lib `xirang`）
- **现象与判据**：它是「XiRang 的协议词汇与纯方法内核——所有不读盘、不读环境、不涉进程生命周期的定义都住在这里的那个组织单位」。 读者把 `core/` 当成 no_std 标准库或「被 `use core::` 指向的东西」。判据/命令：`grep -n 'name = "xirang"' core/Cargo.toml`（→`core/Cargo.toml:32` 的 lib 名）；`cargo metadata … | python3 -c '…"xirang" in names…'`（→ `False`，没有叫 `xirang` 的包）。
- **最小修复方向**：建议保留 `plugin-host`（crate/目录名）**
- **复核状态**：已复核（部分证实） · 复核报告（手段见出处） · t35 · **批次** B8-命名歧义与约定冲突

#### AMB-02 · MAJOR · 命名歧义/类别1（Cargo 惯用名） · `build_method/pipeline.rs:55`

- **原名/来源**：AMB-02（audit-naming-ambiguity.md:79）
- **标题**：`build_method/`
- **现象与判据**：它是「构建期把宿主源码变成生成的注册计划与清单的那一半——构建脚本背后的组织单位」。 读者把 `build_method/` 当成「构建脚本相关的东西」或「一个构建方法」；`grep -rn build.rs` 全仓 4 个文件 3 种角色（真构建脚本 2、CLI 命令实现 1、模块文件 1）。判据/命令：`find . -name build.rs -not -path '*/target/*' | sort`。
- **最小修复方向**：建议保留 `plugin-host`（crate/目录名）**
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t35 · **批次** B8-命名歧义与约定冲突

#### AMB-03 · MAJOR · 命名歧义/类别2（跨生态阅读） · `run_method/lib.rs:27`

- **原名/来源**：AMB-03（audit-naming-ambiguity.md:94）
- **标题**：`run_method/`
- **现象与判据**：它是「运行期承载注册状态、记录已发生的调用、并执行创作写盘的那一半」。 全仓 5 个 `run` 系公开名（`build_method::run`、`cli::run`、`mcp::run`、`RuntimeCheckSpec::run`、`CallTrace::runtime`，见 AMB-20）。判据/命令：`grep -rnE '^pub fn run\b' --include=*.rs build_method/src cli/src mcp/src`。
- **最小修复方向**：建议保留 `plugin-host`（crate/目录名）**
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t35 · **批次** B8-命名歧义与约定冲突

#### AMB-04 · MAJOR · 命名歧义/类别3（同族易混） · `plugin-host/src/verifier.rs:10`

- **原名/来源**：AMB-04（audit-naming-ambiguity.md:108）
- **标题**：`plugin-host/`（crate/目录）与方案模块名 `plugins`
- **现象与判据**：它是「**把锁里写下的插件变成正在运行的插件**的那个运行期宿主」——**不创建插件**。 读者把 `plugins/` 当成放插件本体的目录（VS Code `extensions/`、WordPress `plugins/` 都是这个意思）。判据/命令：`grep -n "PLUGIN_LOCK_DIRECTORY" plugin-host/src/admission.rs`（→ `:38`）；`sed -n '40p;194p' docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`。
- **最小修复方向**：建议保留 `plugin-host`（crate/目录名）**
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t35 · **批次** B8-命名歧义与约定冲突

#### AMB-05 · MAJOR · 命名歧义/类别1（Cargo 生态词） · `core/src/registry_core/tree/registry.rs:18`

- **原名/来源**：AMB-05（audit-naming-ambiguity.md:122）
- **标题**：公开类型 `Registry`
- **现象与判据**：它是「已注册的注册面组成的**树**——可注册、可查询、可嫁接的那一个数据结构」。 读者/agent 把 `Registry`/`registry_core`/MCP 工具 `xirang.registry`（`mcp/tools.rs:114`）当成包注册表。判据/命令：`grep -n "pub struct Registry" core/src/registry_core/tree/registry.rs`；`grep -n "index.crates.io\|the registry" tools/xirang-publish`。
- **最小修复方向**：建议保留 `plugin-host`（crate/目录名）**
- **复核状态**：已复核（证实） · 复核报告（手段见出处） · t35 · **批次** B8-命名歧义与约定冲突

### 2.3 MINOR（143 条）

MINOR 逐条明细见 `audit-findings.json`（同样的字段）；下表给出 id、位置、一句话、复核状态与批次。

| id | 旧 id | 位置 | 一句话 | 复核 | 批次 |
| --- | --- | --- | --- | --- | --- |
| `GTE-G-03` | G-03 = LG-54 | `conventions/src/doc_blocks.rs:174` | 门禁-假阴性：`doc_blocks` 的 markdown 半边不按 CommonMark 配对围栏字符/长度 | 已复核 | B2 |
| `GTE-G-04` | G-04 = LG-54 | `conventions/src/size.rs:151` | 门禁-假阴性/假阳性各半：`size` 只认字面前缀 `#[cfg(test)` 与字面 `mod x;` / `pub mod x;` | 已复核 | B2 |
| `GTE-G-05` | G-05 = LG-54 | `examples/control-button/Cargo.toml:14` | 门禁-假阴性：两个 example 宿主的版本要求没有任何门禁覆盖 | 已复核 | B2 |
| `GTE-G-07` | G-07 = LG-54 | `conventions/src/naming.rs:238` | 门禁-假阳性：`naming` 的"要求位置"扫描不跳过注释 | 已复核 | B2 |
| `GTE-G-08` | G-08 = LG-54 | `conventions/src/lib.rs:247` | 门禁-假阳性：共享助手 `drop_governed_lines` 把属性与声明之间的文档注释误判成"声明" | 已复核 | B2 |
| `GTE-G-09` | G-09 | `tools/xirang-publish:101` | 工具-脚本：`--workspace-version` 不与其它模式互斥，且在派发前 `exit 0` | 已复核 | B2 |
| `GTE-G-10` | G-10 | `tools/xirang-release-audit:23` | 工具-脚本：`xirang-release-audit` 的符号审计可能静默为空且仍报成功 | 已复核 | B2 |
| `GTE-G-11` | G-11 | `tools/xirang-visual:116` | 工具-脚本：`xirang-visual --help` 用硬编码行范围打印，句子中途截断 | 已复核 | B2 |
| `GTE-G-12` | G-12 | `tools/xirang-external-rehearsal:64` | 工具-脚本：两处 GNU 专属语法，macOS 上会失败（CI 只在 Linux 跑，所以不红） | 已复核 | B2 |
| `GTE-G-17` | G-17 = LG-54 | `conventions/src/size.rs:66` | 文档-漂移：size 棘轮的条目数在文档里是 14，在代码里是 1 | 已复核 | B2 |
| `GTE-G-18` | G-18 = LG-54 | `CHANGELOG.md:31` | 文档-漂移：CHANGELOG 用现在时引用了工作树里已不存在的清单拼写 | 已复核 | B2 |
| `GTE-G-19` | G-19 = LG-54 | `AGENTS.md:151` | 文档-漂移：`AGENTS.md` 的门禁清单比实际门禁宽，且缺 4 道 | 已复核 | B2 |
| `GTE-G-20` | G-20 = LG-54 | `conventions/src/mounting.rs:10` | 注释-准确性：`conventions/src/mounting.rs` 的散文数字已过期（29 → 33），而散文里的常量没有对账机制 | 已复核 | B2 |
| `GTE-G-22` | G-22 = LG-54 | `conventions/src/lint.rs:15` | 注释-准确性：`conventions/src/lint.rs` 的模块文档低估了自己的范围 | 已复核 | B2 |
| `GTE-G-24` | G-24 = LG-54 | `conventions/src/size.rs:124` | 注释-准确性：`conventions/src/size.rs` 一句话里的层数与它列出的证据不一致 | 已复核 | B2 |
| `GTE-G-25` | G-25 = LG-54 | `CHANGELOG.md:14` | 注释-准确性：`ci.yml` 留了一条条件已经满足的悬挂指令 | 已复核 | B2 |
| `GTE-G-26` | G-26 = LG-54 | `AGENTS.md:196` | 注释-可读性：用户可见输出里用"from today / 从今天起" | 已复核 | B2 |
| `GTE-G-27` | G-27 = LG-54 | `build_method/Cargo.toml:16` | 文档-漂移：roadmap 说依赖要求写 `0.1.0` 以便不必连锁重发，而工作树里写的是 `^0.1.6`，且两道门禁**强制**它等于工作区版本 | 已复核 | B2 |
| `GTE-N-2` | （复核新增） | `conventions/src/release_workflow.rs:282` | `takes_input` 同为子串匹配：`inputs['publish']` 方括号拼法实测 0 发现 | 已复核 | B2 |
| `GTE-N-3` | （复核新增） | `conventions/src/doc_anchors.rs:250` | `doc_anchors` 只认 `.rs:NNN`，非 Rust 文件的 `file:line` 引用永不被检查 | 已复核 | B2 |
| `BRG-BR-11` | BR-11 | `mcp/index.rs:218` | 测试挂载方式不统一（16 处 `#[path]` + 2 处内联，`mcp/tools.rs` 两者兼有） | 已复核 | B3 |
| `BRG-BR-12` | BR-12 | `mcp/tools.rs:58` | 目录顺序 ≠ 分派顺序，且没有钉子保证两张名单齐全 | 已复核 | B3 |
| `BRG-BR-15` | BR-15 | `cli/README.zh-CN.md:13` | 中文 CLI README 的命令表少 `snippets`；两份都漏 `studio [path]` | 已复核 | B3 |
| `BRG-BR-17` | BR-17 | `plugin-host/src/process.rs:296` | 输入超限消息在 `u32` 帧宽那一支报的是配置上限 | 已复核 | B3 |
| `BRG-BR-7` | BR-7 | `mcp/tools.rs:238` | `xirang.usages` 的描述比它打印的字段少五项（工具 rustdoc 承诺面；归并见 `BR-C0.4`） | 已复核 | B3 |
| `BRG-BR-9` | BR-9 | `mcp/index.rs:155` | 「可移植路径」在同一 crate 里两种拼法 | 已复核 | B3 |
| `BRG-BR-C3` | BR-C3 | `mcp/callgraph.rs:10` | 同一实测事实在注释里有两个数字（142 vs 151） | 已复核 | B3 |
| `BRG-BR-C4` | BR-C4 | `cli/lib.rs:169` | 文档注释里的工具名拼错：`nihlink` | 已复核 | B3 |
| `BRG-BR-C5` | BR-C5 | `mcp/evidence.rs:112` | 一个桥里 `slot` 有三个含义；同一状态有三种拼法 | 已复核 | B3 |
| `BRG-BR-C6` | BR-C6 | `mcp/evidence.rs:40` | 该由类型承担的信息被写进注释：无名三元组与半真名字 | 已复核 | B3 |
| `BRG-BR-C7` | BR-C7 | `mcp/apply.rs:94` | `apply()` 靠注释说明一组按约定成立的不变量 | unverified | B3 |
| `KRN-C-05` | C-05 | `core/src/registry_core/tree/registry.rs:1` | 文档头用 `//` 而非 `//!`，与同层模块不一致 | 已复核 | B5 |
| `KRN-C-06` | C-06 | `core/src/registry_core/release/release.rs:259` | 同一个文档注释里两个 `# Panics` 段，第一段讲的是别的事 | 已复核 | B5 |
| `KRN-C-07` | C-07 | `core/src/registry_core/declaration/runtime_checks_tests.rs:1` | 同一模块文档整段重复（第二份只多一句） | 已复核 | B5 |
| `KRN-C-08` | C-08 | `core/src/registry_core/plugin/trust/trust.rs:86` | 安全相关的参数没有文档，回退语义只存在于代码里 | 已复核 | B5 |
| `KRN-C-10` | C-10 | `core/src/registry_core/mir/model.rs:28` | 名字承担不了信息，只能靠注释补（`mir_line` 与 `resolve_*` 家族） | 已复核 | B5 |
| `KRN-K-10` | K-10 = LG-37 | `core/src/registry_core/authoring/parse/rules.rs:27` | D 大逻辑（注册规则用“整文件文本启发式”解析，可读到注释里的伪子句） | 已复核 | B5 |
| `KRN-K-19` | K-19 | `core/src/registry_core/syntax/face.rs:169` | A 命名（公开 API 的失败模式与命名不符：`is_face_source` 会在非注册面上返回 true；`*_matches` 需要未强制的小写前提） | 已复核 | B5 |
| `KRN-K-24` | K-24 | `core/src/registry_core/source/source_tests.rs:207` | A 命名（测试名与它断言的行为相反） | 已复核 | B5 |
| `LGC-LG-35` | S13 | `run_method/call_report/call_report.rs:20` | `render_call_report` 的名字与文档 | 已复核 | B5 |
| `STU-C-05` | C-05 | `studio/studio/app/lifecycle.rs:185` | 注释位置 · `studio/studio/app/lifecycle.rs:185-190` | 已复核 | B5 |
| `STU-C-06` | C-06 | `studio/studio/ui/status.rs:37` | 注释与实现不符（页脚合同） · `studio/studio/ui/status.rs:37-48` | 已复核 | B5 |
| `STU-C-09` | C-09 | `studio/studio/app/graft.rs:129` | 注释密度/信噪比 · `studio/studio/app/graft.rs:129-158`、`studio/studio/ui/grap …（全文见 `audit-findings.json`） | unverified | B5 |
| `STU-S-14` | S-14 | `studio/studio/app/mutations.rs:1` | 命名/职责 · `studio/studio/app/writers.rs`（全 42 行）与 `studio/studio/app/mut …（全文见 `audit-findings.json`） | 已复核 | B5 |
| `STU-S-17` | S-17 | `studio/studio/app/navigation.rs:141` | 命名/死分支 · `studio/studio/app/navigation.rs:141-146/155-173` | 已复核 | B5 |
| `STU-S-19` | S-19 | `studio/studio/app/keyboard.rs:160` | 命名/交互 · `studio/studio/app/keyboard.rs:160-164` | 已复核 | B5 |
| `STU-S-26` | S-26 | `studio/studio/app/navigation.rs:1` | 命名/位置 · `studio/studio/app/navigation.rs:1-96` | 已复核 | B5 |
| `SUR-C6` | C6 | `run_method/authoring/operations/face_write.rs:85` | 同一个字段集合被三处注释写成三个数 | 已复核 | B5 |
| `SUR-C7` | C7 | `run_method/macros/face_objects.rs:62` | 复述与残句：`run_method/macros/face_objects.rs` 里三段讲"已删除的 arm"的注释互相重复 | 已复核 | B5 |
| `SUR-C8` | C8 | `build_method/cache.rs:34` | 该由名字承担的信息被塞进注释：`write_if_changed` 的文档没说它不原子 | 已复核 | B5 |
| `SUR-C9` | C9 | `build_method/pipeline.rs:105` | 注释密度：反例与正例并存（供维护者判断，不作为缺陷） | 已复核 | B5 |
| `SUR-S11` | S11 = LG-37 | `build_method/contracts.rs:193` | 假实现／诊断质量 · `rule_method_strings` 只读第一处调用，且对注释与转义引号不设防 | 已复核 | B5 |
| `KRN-C-04` | C-04 | `core/src/registry_core/tree/graft_ops/record.rs:24` | 同一段说明重复三页、历史叙事进入生产文件 | 已复核 | B6 |
| `KRN-C-09` | C-09 | `core/src/registry_core/authoring/field_presentation.rs:248` | 字段帮助文本讲的是另一个机制（`runtime_checks`） | unverified | B6 |
| `KRN-C-11` | C-11 | `core/src/registry_core/plugin/contracts/contracts.rs:35` | 术语一名多指（同一个“槽位名”概念四种叫法） | 已复核 | B6 |
| `KRN-K-14` | K-14 | `core/src/registry_core/syntax/tokens.rs:64` | D 大逻辑（`split_face_fields` 的尖括号计数会把后续字段吞进前一个值） | 已复核 | B6 |
| `KRN-K-15` | K-15 | `core/src/registry_core/mir/jsonl.rs:111` | D 大逻辑（`from_jsonl` 忽略 `}` 之后的残留文本，整条记录可被静默丢弃） | 已复核 | B6 |
| `KRN-K-16` | K-16 | `core/src/registry_core/source/source.rs:429` | E 函数内谬误（掩码失败时回退到**未掩码**源码，方向最坏） | 已复核 | B6 |
| `KRN-K-17` | K-17 = LG-37 | `core/src/registry_core/source/source.rs:434` | D 大逻辑（`registration_kinds` 用“整行含 `kind:`”启发式，既过收也漏收） | 已复核 | B6 |
| `KRN-K-20` | K-20 | `core/src/registry_core/tree/graft_ops/record.rs:24` | B 责任与碎片化（三页共用同一段“为什么拆”的说明；同一历史叙事散落在生产文件里） | 已复核 | B6 |
| `KRN-K-21` | K-21 | `core/src/registry_core/syntax/fields.rs:104` | D 大逻辑（同一“trait 标签”概念有两份派生实现，泛型路径下结果不同） | unverified | B6 |
| `KRN-K-23` | K-23 | `core/src/registry_core/syntax/face.rs:250` | D 大逻辑（任何以 `_object` 结尾的用户宏都被当作注册面，非注册宏会得到错误的构建诊断） | 已复核 | B6 |
| `LGC-LG-33` | K-03 | `core/src/registry_core/tree/graft_ops/graft_ops.rs:198` | `validate_*` 通过而 `apply_*` 失败时，错误文案指错原因（机制已被证伪） | 已复核 | B6 |
| `LGC-LG-34` | V-03 | `studio/studio/app/keyboard.rs:52` | Studio Edit 表单在文件读不到时显示空契约（写盘不会抹掉；真缺陷是 LGC-LG-21） | 已复核 | B6 |
| `LGC-LG-36` | S15 | `run_method/authoring/manifest/face/face.rs:144` | `edit()` 里的恒等分支 | 已复核 | B6 |
| `LGC-LG-39` | K-18 | `core/src/registry_core/diagnostic/topology.rs:39` | 静态 vs 运行时对重复身份的判定 | 已复核 | B6 |
| `LGC-LG-40` | K-13 = X-1 | `core/src/registry_core/plugin/catalog/catalog.rs:281` | 插件锁读取的字段语义 | 已复核 | B6 |
| `LGC-LG-41` | K-11 = K-12 | `core/src/registry_core/plugin/trust/trust.rs:107` | 官方通道的信任判定 | 已复核 | B6 |
| `LGC-LG-42` | S16 | `run_method/runtime/trace/locals/call_trace.rs:76` | 库内 panic 面 | 已复核 | B6 |
| `LGC-LG-44` | BR-13 = NEW-B2 | `cli/lib.rs:139` | CLI `--help` 语义 | 已复核 | B6 |
| `LGC-LG-45` | BR-14 | `cli/commands/new.rs:63` | `new` 的依赖来源 | 已复核 | B6 |
| `LGC-LG-46` | BR-16 | `plugin-host/src/process.rs:369` | 进程适配器的 deadline | 已复核 | B6 |
| `LGC-LG-47` | BR-18 | `plugin-host/src/lazy_wasm/slot_state.rs:74` | wasm 懒激活的重试与吞吐 | 已复核 | B6 |
| `LGC-LG-48` | S14 | `run_method/authoring/operations/create.rs:101` | 失败回滚的可靠性 | 已复核 | B6 |
| `LGC-LG-49` | S12 | `run_method/runtime/trace/artifact/parse.rs:21` | trace artifact 读取 | 已复核 | B6 |
| `LGC-LG-51` | LH-10b = LH-10 | `mcp/mir.rs:217` | `xirang.mir` 根外路径的两种回复构成宿主文件**存在性 oracle** | unverified | B6 |
| `LGC-LG-52` | LH-10a = LH-10 | `plugin-host/src/process.rs:392` | 插件坏帧只剩无上下文的 `Io("failed to fill whole buffer")` | unverified | B6 |
| `LGC-LH-09` | LH-09 | `mcp/tools.rs:139` | 自我描述与行为不一致（本项目的老毛病，两处不一致都算缺陷） | unverified | B6 |
| `LGC-LH-11` | LH-11 | `run_method/macros/face_registration.rs:160` | 宏里的 `::xirang_debug_method::submit!`：opt-in 且 `cfg(debug_assertions)` 门控，不是「 …（全文见 `audit-findings.json`） | unverified | B6 |
| `STU-C-07` | C-07 | `studio/studio/app/graft.rs:69` | 术语一致性 · `studio/studio/app/graft.rs:69`、`studio/studio/app/graft.rs:78 …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-C-08` | C-08 | `studio/studio/app/hot_zones.rs:40` | 悬挂预留 · `studio/studio/app/hot_zones.rs:40`、`studio/studio/app/hot_zone …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-04` | S-04 | `studio/studio/ui/ui.rs:66` | 职责边界 · `studio/studio/ui/ui.rs:66`、`studio/studio/ui/panels.rs:52/60-6 …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-09` | S-09 | `studio/studio/app/navigation.rs:79` | 一致性 · `studio/studio/app/navigation.rs:79-85` vs `studio/bin/xirang- …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-10` | S-10 | `studio/studio/ui/graph/nodes.rs:183` | 缓存失效 · `studio/studio/ui/graph/nodes.rs:183-188`、`studio/studio/app/ap …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-11` | S-11 | `studio/studio/ui/overlay.rs:8` | 重复 · `studio/studio/ui/overlay.rs:8-23` vs `studio/studio/ui/overlay.r …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-12` | S-12 | `studio/studio/app/overlay/plugin.rs:29` | 魔法数字 · `studio/studio/app/overlay/plugin.rs:29-30`、`studio/studio/app/ …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-13` | S-13 | `studio/studio/app/state/misc.rs:161` | 职责边界 · `studio/studio/app/state/misc.rs:161-181`（整个文件 1-181） | 已复核 | B6 |
| `STU-S-15` | S-15 | `studio/studio/app/support.rs:1` | 碎片化/阅读顺序 · `studio/studio/app/support.rs:1-509` | 已复核 | B6 |
| `STU-S-16` | S-16 | `studio/studio/app/lifecycle.rs:80` | 逻辑/重复工作 · `studio/studio/app/lifecycle.rs:80-88` | 已复核 | B6 |
| `STU-S-18` | S-18 | `studio/studio/ui/status.rs:16` | 耦合/性能 · `studio/studio/ui/status.rs:16-17` | 已复核 | B6 |
| `STU-S-20` | S-20 | `studio/studio/app/overlay/add.rs:12` | 碎片化/重复状态机 · `studio/studio/app/overlay/add.rs:12-36` vs `studio/studio …（全文见 `audit-findings.json`） | unverified | B6 |
| `STU-S-21` | S-21 | `studio/studio/ui/forms/face.rs:105` | 重复 · `studio/studio/ui/forms/face.rs:105-128`、`studio/studio/ui/forms/ …（全文见 `audit-findings.json`） | unverified | B6 |
| `STU-S-22` | S-22 | `studio/studio/app/tests/call_tree.rs:25` | 测试夹具重复 · `studio/studio/app/tests/call_tree.rs:25-35`、`studio/studio/a …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-23` | S-23 | `studio/studio/app/graft.rs:352` | 测试结构/成本 · `studio/studio/app/tests/project.rs:1-562` | 已复核 | B6 |
| `STU-S-25` | S-25 | `studio/studio/app/support.rs:296` | 逻辑/脆弱判据 · `studio/studio/app/support.rs:296/311-322/362-373` | 已复核 | B6 |
| `STU-S-27` | S-27 | `studio/studio/app/mutations.rs:137` | 职责/一致性 · `studio/studio/app/mutations.rs:137-149` vs `185-210`；`studio/src …（全文见 `audit-findings.json`） | 已复核 | B6 |
| `STU-S-28` | S-28 | `studio/studio/app/interaction.rs:18` | 功能缺口 · `studio/studio/app/interaction.rs:18` | 已复核 | B6 |
| `STU-S-29` | S-29 | `studio/studio/app/graft.rs:166` | 读写根不一致 · `studio/studio/app/graft.rs:166-175/235-265` vs `111-118/180/200` | 已复核 | B6 |
| `STU-V-01` | （复核新增） | `studio/studio/app/mutations.rs:332` | 锁追加假定「既有文件以换行结尾」，无换行的锁会被拼出一条畸形记录 | 已复核 | B6 |
| `STU-V-02` | （复核新增） | `studio/studio/app/keyboard.rs:38` | 页脚宣传的首屏键 `d`/`e`/`g` 在选中根节点时是静默空操作 | 已复核 | B6 |
| `SUR-S10` | S10 | `run_method/macros/face_objects.rs:169` | 一致性／宏卫生 · 前端回退臂用绝对 crate 路径而不是 `$crate` | 已复核 | B6 |
| `SUR-S17` | S17 | `macro/src/lib.rs:283` | 诊断质量 · 一条 `compile_error` 消息里带 18 个连续空格 | 已复核 | B6 |
| `SUR-S18` | S18 | `build_method/scope_view.rs:43` | 一致性 · `build_output_is_current` 用 `root/src` 复算指纹，而管线用布局解析出的身份基准 | 已复核 | B6 |
| `NAM-02` | NAM-02 | `studio/studio/app/state/misc.rs:55` | A① 类别名而不是东西名 · `studio/studio/app/state/misc.rs` | 已复核 | B7 |
| `NAM-05` | BRG-BR-C6 | `mcp/evidence.rs:65` | A③ 名与内容不符（误导 → MAJOR） · `mcp/evidence.rs` | 已复核 | B7 |
| `NAM-06` | STU-S-14 | `studio/studio/app/writers.rs:1` | E. 与既有 lane 命名条目的归并（不重复计数） · `studio/studio/app/writers.rs`（归并 STU-S-14） | 已复核 | B7 |
| `NAM-07` | NAM-07 | `core/src/registry_core/tree/connector/connector.rs:1` | A② 名字与父目录重复 · 全仓 34 处 `<x>/<x>.rs` 且该目录**递归**只有这一个 `.rs` | 已复核 | B7 |
| `NAM-08` | NAM-08 | `run_method/runtime/trace/call_trace.rs:1` | A④ 跨 crate / 跨目录同名不同职责 · `run_method/runtime/trace/call_trace.rs` 与 `run_m …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-09` | NAM-09 | `run_method/authoring/face_manifest.rs:1` | A④ 跨 crate / 跨目录同名不同职责 · `run_method/authoring/face_manifest.rs` 与 `run_me …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-10` | NAM-10 | `run_method/authoring/parse/parse.rs:1` | A④ 跨 crate / 跨目录同名不同职责 · `run_method/authoring/parse/parse.rs`（shim）与 `run …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-11` | NAM-11 | `mcp/evidence.rs:1` | A④ 跨 crate / 跨目录同名不同职责 · 保留词表：`evidence` / `index` / `artifact` / `registry` 一 …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-12` | NAM-12 | `mcp/search.rs:1` | A④ 跨 crate / 跨目录同名不同职责 · `mcp/search.rs` 在同一 crate 内四处 | 已复核 | B7 |
| `NAM-17` | NAM-17 | `build_method/faces.rs:1` | A③ 名与内容不符（误导 → MAJOR） · `build_method/faces.rs` vs `build_method/face_ …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-18` | NAM-18 | `build_method/node.rs:1` | A③ 名与内容不符（误导 → MAJOR） · `build_method/node.rs` vs `build_method/node_i …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-19` | NAM-19 | `studio/studio/app/tests/project.rs:1` | A⑤ 测试文件名与它测的东西 · studio 的 `app/tests/*.rs` 需要写明"覆盖哪个生产模块" | 已复核 | B7 |
| `NAM-20` | NAM-20 | `mcp/index.rs:1` | A⑥ 目录名是否表达层级 · 平铺目录让 A④ 的同名问题加倍 | unverified | B7 |
| `NAM-21` | NAM-21 | `build_method/discovery.rs:1` | A③ 名与内容不符（误导 → MAJOR） · `build_method/src` 的"发现"族命名收敛 | unverified | B7 |
| `NAM-22` | NAM-22 | `mcp/nodes.rs:1` | A③ 名与内容不符（误导 → MAJOR） · `mcp/nodes.rs` | 已复核 | B7 |
| `NAM-23` | NAM-23 | `plugin-host/src/lazy_wasm.rs:24` | A④ 跨 crate / 跨目录同名不同职责 · `pub use xirang_run_method::PluginChannel as Valida …（全文见 `audit-findings.json`） | 已复核 | B7 |
| `NAM-32` | NAM-32 | `build_method/lib.rs:137` | B① 空壳名（无宾语） · 裸动词作函数名：全量清单与规则 | 已复核 | B7 |
| `NAM-33` | NAM-33 | `core/src/registry_core/tree/query/query.rs:1` | B② 同一动作的同义词蔓延（谁在用哪个词） · 动词收敛表 | 已复核 | B7 |
| `NAM-34` | NAM-34 | `core/src/registry_core/authoring/parse/parse.rs:258` | B④ 名字描述实现而不是意图 · `_owned` 家族与 `_strings` 一类：全量 25 行，8 行计入 | 已复核 | B7 |
| `NAM-35` | KRN-K-19 | `core/src/registry_core/release/release.rs:191` | B⑤ 布尔谓词规范 · 三处裸形容词谓词 | 已复核 | B7 |
| `NAM-36` | NAM-36 | `core/src/registry_core/tree/query/query.rs:80` | B⑥ pub API 与内部函数风格不一致 · `Registry::find` 与它的两个兄弟不同精细度 | 已复核 | B7 |
| `NAM-37` | NAM-37 | `core/src/registry_core/tree/entry_pages/entry_pages.rs:39` | B⑦ 单复数 / 单位 / 时态 · 两处 | 已复核 | B7 |
| `NAM-40` | NAM-40 | `build_method/identity.rs:1` | C. 模块名 · 模块路径与文件名的对应 | 已复核 | B7 |
| `NAM-41` | NAM-41 | `plugin-host/src/lazy_wasm.rs:1` | C. 模块名 · `pub use … as …` 的别名纪律 | 已复核 | B7 |
| `NAM-43` | NAM-43 | `build_method/identity.rs:1` | C. 模块名 · 模块挂载改名的四个家族（文件 stem ≠ 模块名，见 NAM-40） | 已复核 | B7 |
| `AMB-06` | AMB-06 | `Cargo.toml:2` | 顶层 `examples/` | 已复核 | B8 |
| `AMB-07` | AMB-07 | `debug_method/src/lib.rs:26` | `debug_method/` | 已复核 | B8 |
| `AMB-08` | AMB-08 | `studio/bin/xirang-dev.rs:1` | bin `xirang-dev` 与 feature `dev-supervisor` | 已复核 | B8 |
| `AMB-09` | AMB-09 | `README.md:3` | 顶层 `picture/` | 已复核 | B8 |
| `AMB-10` | AMB-10 | `docs/ROADMAP.md:1` | `docs/ROADMAP.md` 与 `docs/roadmap-1.0.md` 只差大小写 | 已复核 | B8 |
| `AMB-11` | AMB-11 | `core/src/registry_core/syntax/nesting.rs:342` | 内核模块 `syntax`（+ feature `syntax`） | 已复核 | B8 |
| `AMB-12` | AMB-12 | `core/src/registry_core/json/json.rs:44` | 内核模块 `json` | unverified | B8 |
| `AMB-13` | AMB-13 | `core/src/registry_core/release/release.rs:9` | 内核模块 `release` | unverified | B8 |
| `AMB-14` | AMB-14 | `core/src/registry_core/requirements/requirements.rs:17` | 内核模块 `requirements` | unverified | B8 |
| `AMB-15` | AMB-15 | `core/src/registry_core/source/source.rs:23` | 内核模块 `source`（一名四义之一） | 已复核 | B8 |
| `AMB-16` | AMB-16 | `mcp/mir.rs:20` | 类型层 `artifact` 一名四义 | unverified | B8 |
| `AMB-17` | AMB-17 | `core/src/registry_core/tree/index/index.rs:13` | `index` 一名三义 | 已复核 | B8 |
| `AMB-18` | AMB-18 | `plugin-host/src/lib.rs:1` | `host` 一名两义 | unverified | B8 |
| `AMB-19` | AMB-19 | `macro/Cargo.toml:20` | feature 名与模块/包名重名的成批现象 | 已复核 | B8 |
| `AMB-20` | AMB-20 | `cli/lib.rs:94` | 函数/类型层的同一轴：13 个公开名的**整名**就是别处的既有词 | 已复核 | B8 |

## 3. ID 映射表（新 id ← 旧 id ← 出处文件与行）

**所有** 212 条的映射都在下表；`（复核新增）` 表示该条由复核者首次提出、在片区报告里没有旧 id。第一张表只列**一合并多**的行，方便核对「旧 id 是否被吞掉」。

### 3.1 合并行（一个 id 覆盖多个旧 id）

| 新 id | 旧 id（= 别名） | 出处（文件:行） |
| --- | --- | --- |
| `LGC-LG-05` | S19 = K-22 | S19 @ audit-lane-surfaces.md:306；K-22 @ audit-lane-surfaces.md:306 |
| `LGC-LG-12` | LH-03 = BR-8 | LH-03 @ audit-logic-hunt.md:127；BR-8 @ audit-logic-hunt.md:127 |
| `LGC-LG-14` | S9 = C3 | S9 @ audit-lane-surfaces.md:189；C3 @ audit-lane-surfaces.md:352 |
| `LGC-LG-20` | LH-07 = BR-19 = LG-07 | LH-07 @ audit-logic-hunt.md:262；BR-19 @ audit-logic-hunt.md:262 |
| `LGC-LG-28` | K-06 = BR-10 | K-06 @ audit-lane-kernel.md:80；BR-10 @ audit-lane-kernel.md:80 |
| `LGC-LG-32` | LH-08 = BR-20 = BR-C1 | LH-08 @ audit-logic-hunt.md:279；BR-20 @ audit-logic-hunt.md:279；BR-C1 @ audit-logic-hunt.md:279 |
| `LGC-LG-40` | K-13 = X-1 | K-13 @ audit-lane-kernel.md:168；X-1 @ audit-lane-kernel.md:168 |
| `LGC-LG-41` | K-11 = K-12 | K-11 @ audit-lane-kernel.md:144；K-12 @ audit-lane-kernel.md:152 |
| `LGC-LG-44` | BR-13 = NEW-B2 | BR-13 @ audit-lane-bridges.md:268；NEW-B2 @ audit-lane-bridges.md:268 |
| `LGC-LG-50` | S-07 = S-24 | S-07 @ audit-lane-studio.md:71；S-24 @ audit-lane-studio.md:192 |
| `LGC-LG-51` | LH-10b = LH-10 | LH-10b @ audit-logic-hunt.md:319；LH-10 @ audit-logic-hunt.md:319 |
| `LGC-LG-52` | LH-10a = LH-10 | LH-10a @ audit-logic-safety-report.md:466；LH-10 @ audit-logic-safety-report.md:466 |
| `LGC-LG-53` | G-06 = G-23 | G-06 @ audit-lane-gates.md:72；G-23 @ audit-lane-gates.md:224 |
| `KRN-K-10` | K-10 = LG-37 | K-10 @ audit-lane-kernel.md:129；LG-37 @ audit-lane-kernel.md:129 |
| `KRN-K-17` | K-17 = LG-37 | K-17 @ audit-lane-kernel.md:212；LG-37 @ audit-lane-kernel.md:212 |
| `SUR-S11` | S11 = LG-37 | S11 @ audit-lane-surfaces.md:224；LG-37 @ audit-lane-surfaces.md:224 |
| `GTE-G-03` | G-03 = LG-54 | G-03 @ audit-lane-gates.md:44；LG-54 @ audit-lane-gates.md:44 |
| `GTE-G-04` | G-04 = LG-54 | G-04 @ audit-lane-gates.md:52；LG-54 @ audit-lane-gates.md:52 |
| `GTE-G-05` | G-05 = LG-54 | G-05 @ audit-lane-gates.md:60；LG-54 @ audit-lane-gates.md:60 |
| `GTE-G-07` | G-07 = LG-54 | G-07 @ audit-lane-gates.md:80；LG-54 @ audit-lane-gates.md:80 |
| `GTE-G-08` | G-08 = LG-54 | G-08 @ audit-lane-gates.md:88；LG-54 @ audit-lane-gates.md:88 |
| `GTE-G-17` | G-17 = LG-54 | G-17 @ audit-lane-gates.md:162；LG-54 @ audit-lane-gates.md:162 |
| `GTE-G-18` | G-18 = LG-54 | G-18 @ audit-lane-gates.md:170；LG-54 @ audit-lane-gates.md:170 |
| `GTE-G-19` | G-19 = LG-54 | G-19 @ audit-lane-gates.md:178；LG-54 @ audit-lane-gates.md:178 |
| `GTE-G-27` | G-27 = LG-54 | G-27 @ audit-lane-gates.md:186；LG-54 @ audit-lane-gates.md:186 |
| `GTE-G-20` | G-20 = LG-54 | G-20 @ audit-lane-gates.md:200；LG-54 @ audit-lane-gates.md:200 |
| `GTE-G-22` | G-22 = LG-54 | G-22 @ audit-lane-gates.md:216；LG-54 @ audit-lane-gates.md:216 |
| `GTE-G-24` | G-24 = LG-54 | G-24 @ audit-lane-gates.md:231；LG-54 @ audit-lane-gates.md:231 |
| `GTE-G-25` | G-25 = LG-54 | G-25 @ audit-lane-gates.md:238；LG-54 @ audit-lane-gates.md:238 |
| `GTE-G-26` | G-26 = LG-54 | G-26 @ audit-lane-gates.md:245；LG-54 @ audit-lane-gates.md:245 |

### 3.2 全量映射表

| 新 id | 旧 id | 出处文件:行 | 严重度 |
| --- | --- | --- | --- |
| `AMB-01` | AMB-01 | AMB-01 @ audit-naming-ambiguity.md:64 | MAJOR |
| `AMB-02` | AMB-02 | AMB-02 @ audit-naming-ambiguity.md:79 | MAJOR |
| `AMB-03` | AMB-03 | AMB-03 @ audit-naming-ambiguity.md:94 | MAJOR |
| `AMB-04` | AMB-04 | AMB-04 @ audit-naming-ambiguity.md:108 | MAJOR |
| `AMB-05` | AMB-05 | AMB-05 @ audit-naming-ambiguity.md:122 | MAJOR |
| `AMB-06` | AMB-06 | AMB-06 @ audit-naming-ambiguity.md:135 | MINOR |
| `AMB-07` | AMB-07 | AMB-07 @ audit-naming-ambiguity.md:148 | MINOR |
| `AMB-08` | AMB-08 | AMB-08 @ audit-naming-ambiguity.md:160 | MINOR |
| `AMB-09` | AMB-09 | AMB-09 @ audit-naming-ambiguity.md:172 | MINOR |
| `AMB-10` | AMB-10 | AMB-10 @ audit-naming-ambiguity.md:184 | MINOR |
| `AMB-11` | AMB-11 | AMB-11 @ audit-naming-ambiguity.md:196 | MINOR |
| `AMB-12` | AMB-12 | AMB-12 @ audit-naming-ambiguity.md:208 | MINOR |
| `AMB-13` | AMB-13 | AMB-13 @ audit-naming-ambiguity.md:219 | MINOR |
| `AMB-14` | AMB-14 | AMB-14 @ audit-naming-ambiguity.md:230 | MINOR |
| `AMB-15` | AMB-15 | AMB-15 @ audit-naming-ambiguity.md:242 | MINOR |
| `AMB-16` | AMB-16 | AMB-16 @ audit-naming-ambiguity.md:254 | MINOR |
| `AMB-17` | AMB-17 | AMB-17 @ audit-naming-ambiguity.md:267 | MINOR |
| `AMB-18` | AMB-18 | AMB-18 @ audit-naming-ambiguity.md:279 | MINOR |
| `AMB-19` | AMB-19 | AMB-19 @ audit-naming-ambiguity.md:290 | MINOR |
| `AMB-20` | AMB-20 | AMB-20 @ audit-naming-ambiguity.md:302 | MINOR |
| `BRG-BR-1` | BR-1 | BR-1 @ audit-lane-bridges.md:54 | MAJOR |
| `BRG-BR-11` | BR-11 | BR-11 @ audit-lane-bridges.md:241 | MINOR |
| `BRG-BR-12` | BR-12 | BR-12 @ audit-lane-bridges.md:255 | MINOR |
| `BRG-BR-15` | BR-15 | BR-15 @ audit-lane-bridges.md:303 | MINOR |
| `BRG-BR-17` | BR-17 | BR-17 @ audit-lane-bridges.md:332 | MINOR |
| `BRG-BR-2` | BR-2 | BR-2 @ audit-lane-bridges.md:75 | MAJOR |
| `BRG-BR-7` | BR-7 | BR-7 @ audit-lane-bridges.md:176 | MINOR |
| `BRG-BR-9` | BR-9 | BR-9 @ audit-lane-bridges.md:207 | MINOR |
| `BRG-BR-C2` | BR-C2 | BR-C2 @ audit-lane-bridges.md:438 | MAJOR |
| `BRG-BR-C3` | BR-C3 | BR-C3 @ audit-lane-bridges.md:453 | MINOR |
| `BRG-BR-C4` | BR-C4 | BR-C4 @ audit-lane-bridges.md:466 | MINOR |
| `BRG-BR-C5` | BR-C5 | BR-C5 @ audit-lane-bridges.md:478 | MINOR |
| `BRG-BR-C6` | BR-C6 | BR-C6 @ audit-lane-bridges.md:498 | MINOR |
| `BRG-BR-C7` | BR-C7 | BR-C7 @ audit-lane-bridges.md:516 | MINOR |
| `GTE-G-03` | G-03, LG-54 | G-03 @ audit-lane-gates.md:44；LG-54 @ audit-lane-gates.md:44 | MINOR |
| `GTE-G-04` | G-04, LG-54 | G-04 @ audit-lane-gates.md:52；LG-54 @ audit-lane-gates.md:52 | MINOR |
| `GTE-G-05` | G-05, LG-54 | G-05 @ audit-lane-gates.md:60；LG-54 @ audit-lane-gates.md:60 | MINOR |
| `GTE-G-07` | G-07, LG-54 | G-07 @ audit-lane-gates.md:80；LG-54 @ audit-lane-gates.md:80 | MINOR |
| `GTE-G-08` | G-08, LG-54 | G-08 @ audit-lane-gates.md:88；LG-54 @ audit-lane-gates.md:88 | MINOR |
| `GTE-G-09` | G-09 | G-09 @ audit-lane-gates.md:100 | MINOR |
| `GTE-G-10` | G-10 | G-10 @ audit-lane-gates.md:108 | MINOR |
| `GTE-G-11` | G-11 | G-11 @ audit-lane-gates.md:116 | MINOR |
| `GTE-G-12` | G-12 | G-12 @ audit-lane-gates.md:124 | MINOR |
| `GTE-G-16` | G-16 | G-16 @ audit-lane-gates.md:154 | MAJOR |
| `GTE-G-17` | G-17, LG-54 | G-17 @ audit-lane-gates.md:162；LG-54 @ audit-lane-gates.md:162 | MINOR |
| `GTE-G-18` | G-18, LG-54 | G-18 @ audit-lane-gates.md:170；LG-54 @ audit-lane-gates.md:170 | MINOR |
| `GTE-G-19` | G-19, LG-54 | G-19 @ audit-lane-gates.md:178；LG-54 @ audit-lane-gates.md:178 | MINOR |
| `GTE-G-20` | G-20, LG-54 | G-20 @ audit-lane-gates.md:200；LG-54 @ audit-lane-gates.md:200 | MINOR |
| `GTE-G-21` | G-21 | G-21 @ audit-lane-gates.md:208 | MAJOR |
| `GTE-G-22` | G-22, LG-54 | G-22 @ audit-lane-gates.md:216；LG-54 @ audit-lane-gates.md:216 | MINOR |
| `GTE-G-24` | G-24, LG-54 | G-24 @ audit-lane-gates.md:231；LG-54 @ audit-lane-gates.md:231 | MINOR |
| `GTE-G-25` | G-25, LG-54 | G-25 @ audit-lane-gates.md:238；LG-54 @ audit-lane-gates.md:238 | MINOR |
| `GTE-G-26` | G-26, LG-54 | G-26 @ audit-lane-gates.md:245；LG-54 @ audit-lane-gates.md:245 | MINOR |
| `GTE-G-27` | G-27, LG-54 | G-27 @ audit-lane-gates.md:186；LG-54 @ audit-lane-gates.md:186 | MINOR |
| `GTE-N-1` | （复核新增） | N-1 @ audit-verify-gates.md:67 | MAJOR |
| `GTE-N-2` | （复核新增） | N-2 @ audit-verify-gates.md:197 | MINOR |
| `GTE-N-3` | （复核新增） | N-3 @ audit-verify-gates.md:202 | MINOR |
| `KRN-C-01` | C-01 | C-01 @ audit-lane-kernel.md:280 | MAJOR |
| `KRN-C-02` | C-02 | C-02 @ audit-lane-kernel.md:287 | MAJOR |
| `KRN-C-03` | C-03 | C-03 @ audit-lane-kernel.md:294 | MAJOR |
| `KRN-C-04` | C-04 | C-04 @ audit-lane-kernel.md:301 | MINOR |
| `KRN-C-05` | C-05 | C-05 @ audit-lane-kernel.md:307 | MINOR |
| `KRN-C-06` | C-06 | C-06 @ audit-lane-kernel.md:315 | MINOR |
| `KRN-C-07` | C-07 | C-07 @ audit-lane-kernel.md:323 | MINOR |
| `KRN-C-08` | C-08 | C-08 @ audit-lane-kernel.md:331 | MINOR |
| `KRN-C-09` | C-09 | C-09 @ audit-lane-kernel.md:338 | MINOR |
| `KRN-C-10` | C-10 | C-10 @ audit-lane-kernel.md:346 | MINOR |
| `KRN-C-11` | C-11 | C-11 @ audit-lane-kernel.md:354 | MINOR |
| `KRN-K-10` | K-10, LG-37 | K-10 @ audit-lane-kernel.md:129；LG-37 @ audit-lane-kernel.md:129 | MINOR |
| `KRN-K-14` | K-14 | K-14 @ audit-lane-kernel.md:176 | MINOR |
| `KRN-K-15` | K-15 | K-15 @ audit-lane-kernel.md:190 | MINOR |
| `KRN-K-16` | K-16 | K-16 @ audit-lane-kernel.md:204 | MINOR |
| `KRN-K-17` | K-17, LG-37 | K-17 @ audit-lane-kernel.md:212；LG-37 @ audit-lane-kernel.md:212 | MINOR |
| `KRN-K-19` | K-19 | K-19 @ audit-lane-kernel.md:228 | MINOR |
| `KRN-K-20` | K-20 | K-20 @ audit-lane-kernel.md:236 | MINOR |
| `KRN-K-21` | K-21 | K-21 @ audit-lane-kernel.md:244 | MINOR |
| `KRN-K-23` | K-23 | K-23 @ audit-lane-kernel.md:260 | MINOR |
| `KRN-K-24` | K-24 | K-24 @ audit-lane-kernel.md:268 | MINOR |
| `LGC-LG-01` | LH-01 | LH-01 @ audit-logic-hunt.md:26 | CRITICAL |
| `LGC-LG-02` | K-01 | K-01 @ audit-lane-kernel.md:11 | CRITICAL |
| `LGC-LG-03` | S-01 | S-01 @ audit-lane-studio.md:22 | CRITICAL |
| `LGC-LG-04` | K-07 | K-07 @ audit-lane-kernel.md:88 | MAJOR |
| `LGC-LG-05` | S19, K-22 | S19 @ audit-lane-surfaces.md:306；K-22 @ audit-lane-surfaces.md:306 | MAJOR |
| `LGC-LG-06` | K-04 | K-04 @ audit-lane-kernel.md:54 | MAJOR |
| `LGC-LG-07` | K-05 | K-05 @ audit-lane-kernel.md:69 | MAJOR |
| `LGC-LG-08` | S1 | S1 @ audit-lane-surfaces.md:13 | MAJOR |
| `LGC-LG-09` | S2 | S2 @ audit-lane-surfaces.md:38 | MAJOR |
| `LGC-LG-10` | S5 | S5 @ audit-lane-surfaces.md:105 | MAJOR |
| `LGC-LG-11` | LH-02 | LH-02 @ audit-logic-hunt.md:91 | MAJOR |
| `LGC-LG-12` | LH-03, BR-8 | LH-03 @ audit-logic-hunt.md:127；BR-8 @ audit-logic-hunt.md:127 | MAJOR |
| `LGC-LG-13` | BR-6 | BR-6 @ audit-lane-bridges.md:151 | MAJOR |
| `LGC-LG-14` | S9, C3 | S9 @ audit-lane-surfaces.md:189；C3 @ audit-lane-surfaces.md:352 | MAJOR |
| `LGC-LG-15` | S3 | S3 @ audit-lane-surfaces.md:62 | MAJOR |
| `LGC-LG-16` | S7 | S7 @ audit-lane-surfaces.md:150 | MAJOR |
| `LGC-LG-17` | S8 | S8 @ audit-lane-surfaces.md:170 | MAJOR |
| `LGC-LG-18` | LH-04 | LH-04 @ audit-logic-hunt.md:164 | MAJOR |
| `LGC-LG-19` | LH-06 | LH-06 @ audit-logic-hunt.md:227 | MAJOR |
| `LGC-LG-20` | LH-07, BR-19, LG-07 | LH-07 @ audit-logic-hunt.md:262；BR-19 @ audit-logic-hunt.md:262 | MAJOR |
| `LGC-LG-21` | NEW-B1 | NEW-B1 @ audit-logic-safety-report.md:34 | MAJOR |
| `LGC-LG-22` | BR-5 | BR-5 @ audit-lane-bridges.md:133 | MAJOR |
| `LGC-LG-23` | BR-4 | BR-4 @ audit-lane-bridges.md:109 | MAJOR |
| `LGC-LG-24` | G-01 | G-01 @ audit-lane-gates.md:24 | MAJOR |
| `LGC-LG-25` | G-02 | G-02 @ audit-lane-gates.md:36 | MAJOR |
| `LGC-LG-26` | G-13 | G-13 @ audit-lane-gates.md:139 | MAJOR |
| `LGC-LG-27` | K-02 | K-02 @ audit-lane-kernel.md:29 | MAJOR |
| `LGC-LG-28` | K-06, BR-10 | K-06 @ audit-lane-kernel.md:80；BR-10 @ audit-lane-kernel.md:80 | MAJOR |
| `LGC-LG-29` | K-08 | K-08 @ audit-lane-kernel.md:104 | MAJOR |
| `LGC-LG-30` | K-09 | K-09 @ audit-lane-kernel.md:121 | MAJOR |
| `LGC-LG-31` | LH-05 | LH-05 @ audit-logic-hunt.md:197 | MAJOR |
| `LGC-LG-32` | LH-08, BR-20, BR-C1 | LH-08 @ audit-logic-hunt.md:279；BR-20 @ audit-logic-hunt.md:279；BR-C1 @ audit-logic-hunt.md:279 | MAJOR |
| `LGC-LG-33` | K-03 | K-03 @ audit-lane-kernel.md:37 | MINOR |
| `LGC-LG-34` | V-03 | LG-34 @ audit-logic-safety-report.md:47 | MINOR |
| `LGC-LG-35` | S13 | S13 @ audit-lane-surfaces.md:248 | MINOR |
| `LGC-LG-36` | S15 | S15 @ audit-lane-surfaces.md:268 | MINOR |
| `LGC-LG-38` | S6 | S6 @ audit-lane-surfaces.md:125 | MAJOR |
| `LGC-LG-39` | K-18 | K-18 @ audit-lane-kernel.md:220 | MINOR |
| `LGC-LG-40` | K-13, X-1 | K-13 @ audit-lane-kernel.md:168；X-1 @ audit-lane-kernel.md:168 | MINOR |
| `LGC-LG-41` | K-11, K-12 | K-11 @ audit-lane-kernel.md:144；K-12 @ audit-lane-kernel.md:152 | MINOR |
| `LGC-LG-42` | S16 | S16 @ audit-lane-surfaces.md:276 | MINOR |
| `LGC-LG-43` | BR-3 | BR-3 @ audit-lane-bridges.md:92 | MAJOR |
| `LGC-LG-44` | BR-13, NEW-B2 | BR-13 @ audit-lane-bridges.md:268；NEW-B2 @ audit-lane-bridges.md:268 | MINOR |
| `LGC-LG-45` | BR-14 | BR-14 @ audit-lane-bridges.md:286 | MINOR |
| `LGC-LG-46` | BR-16 | BR-16 @ audit-lane-bridges.md:316 | MINOR |
| `LGC-LG-47` | BR-18 | BR-18 @ audit-lane-bridges.md:346 | MINOR |
| `LGC-LG-48` | S14 | S14 @ audit-lane-surfaces.md:258 | MINOR |
| `LGC-LG-49` | S12 | S12 @ audit-lane-surfaces.md:238 | MINOR |
| `LGC-LG-50` | S-07, S-24 | S-07 @ audit-lane-studio.md:71；S-24 @ audit-lane-studio.md:192 | MAJOR |
| `LGC-LG-51` | LH-10b, LH-10 | LH-10b @ audit-logic-hunt.md:319；LH-10 @ audit-logic-hunt.md:319 | MINOR |
| `LGC-LG-52` | LH-10a, LH-10 | LH-10a @ audit-logic-safety-report.md:466；LH-10 @ audit-logic-safety-report.md:466 | MINOR |
| `LGC-LG-53` | G-06, G-23 | G-06 @ audit-lane-gates.md:72；G-23 @ audit-lane-gates.md:224 | MAJOR |
| `LGC-LH-09` | LH-09 | LH-09 @ audit-logic-hunt.md:306 | MINOR |
| `LGC-LH-11` | LH-11 | LH-11 @ audit-logic-hunt.md:326 | MINOR |
| `NAM-01` | NAM-01 | NAM-01 @ audit-naming-review.md:45 | MAJOR |
| `NAM-02` | NAM-02 | NAM-02 @ audit-naming-review.md:51 | MINOR |
| `NAM-03` | NAM-03 | NAM-03 @ audit-naming-review.md:77 | MAJOR |
| `NAM-04` | NAM-04 | NAM-04 @ audit-naming-review.md:84 | MAJOR |
| `NAM-05` | BRG-BR-C6 | NAM-05 @ audit-naming-review.md:90 | MINOR |
| `NAM-06` | STU-S-14 | NAM-06 @ audit-naming-review.md:404 | MINOR |
| `NAM-07` | NAM-07 | NAM-07 @ audit-naming-review.md:62 | MINOR |
| `NAM-08` | NAM-08 | NAM-08 @ audit-naming-review.md:127 | MINOR |
| `NAM-09` | NAM-09 | NAM-09 @ audit-naming-review.md:133 | MINOR |
| `NAM-10` | NAM-10 | NAM-10 @ audit-naming-review.md:139 | MINOR |
| `NAM-11` | NAM-11 | NAM-11 @ audit-naming-review.md:145 | MINOR |
| `NAM-12` | NAM-12 | NAM-12 @ audit-naming-review.md:159 | MINOR |
| `NAM-17` | NAM-17 | NAM-17 @ audit-naming-review.md:101 | MINOR |
| `NAM-18` | NAM-18 | NAM-18 @ audit-naming-review.md:107 | MINOR |
| `NAM-19` | NAM-19 | NAM-19 @ audit-naming-review.md:175 | MINOR |
| `NAM-20` | NAM-20 | NAM-20 @ audit-naming-review.md:185 | MINOR |
| `NAM-21` | NAM-21 | NAM-21 @ audit-naming-review.md:113 | MINOR |
| `NAM-22` | NAM-22 | NAM-22 @ audit-naming-review.md:119 | MINOR |
| `NAM-23` | NAM-23 | NAM-23 @ audit-naming-review.md:165 | MINOR |
| `NAM-30` | NAM-30 | NAM-30 @ audit-naming-review.md:260 | MAJOR |
| `NAM-31` | NAM-31 | NAM-31 @ audit-naming-review.md:266 | MAJOR |
| `NAM-32` | NAM-32 | NAM-32 @ audit-naming-review.md:196 | MINOR |
| `NAM-33` | NAM-33 | NAM-33 @ audit-naming-review.md:238 | MINOR |
| `NAM-34` | NAM-34 | NAM-34 @ audit-naming-review.md:274 | MINOR |
| `NAM-35` | KRN-K-19 | NAM-35 @ audit-naming-review.md:301 | MINOR |
| `NAM-36` | NAM-36 | NAM-36 @ audit-naming-review.md:309 | MINOR |
| `NAM-37` | NAM-37 | NAM-37 @ audit-naming-review.md:317 | MINOR |
| `NAM-40` | NAM-40 | NAM-40 @ audit-naming-review.md:326 | MINOR |
| `NAM-41` | NAM-41 | NAM-41 @ audit-naming-review.md:356 | MINOR |
| `NAM-43` | NAM-43 | NAM-43 @ audit-naming-review.md:345 | MINOR |
| `STU-C-01` | C-01 | C-01 @ audit-lane-studio.md:248 | MAJOR |
| `STU-C-02` | C-02 | C-02 @ audit-lane-studio.md:255 | MAJOR |
| `STU-C-03` | C-03 | C-03 @ audit-lane-studio.md:262 | MAJOR |
| `STU-C-04` | C-04 | C-04 @ audit-lane-studio.md:269 | MAJOR |
| `STU-C-05` | C-05 | C-05 @ audit-lane-studio.md:276 | MINOR |
| `STU-C-06` | C-06 | C-06 @ audit-lane-studio.md:283 | MINOR |
| `STU-C-07` | C-07 | C-07 @ audit-lane-studio.md:290 | MINOR |
| `STU-C-08` | C-08 | C-08 @ audit-lane-studio.md:297 | MINOR |
| `STU-C-09` | C-09 | C-09 @ audit-lane-studio.md:304 | MINOR |
| `STU-S-02` | S-02 | S-02 @ audit-lane-studio.md:33 | MAJOR |
| `STU-S-03` | S-03 | S-03 @ audit-lane-studio.md:40 | MAJOR |
| `STU-S-04` | S-04 | S-04 @ audit-lane-studio.md:47 | MINOR |
| `STU-S-05` | S-05 | S-05 @ audit-lane-studio.md:54 | MAJOR |
| `STU-S-06` | S-06 | S-06 @ audit-lane-studio.md:61 | MAJOR |
| `STU-S-08` | S-08 | S-08 @ audit-lane-studio.md:78 | MAJOR |
| `STU-S-09` | S-09 | S-09 @ audit-lane-studio.md:85 | MINOR |
| `STU-S-10` | S-10 | S-10 @ audit-lane-studio.md:92 | MINOR |
| `STU-S-11` | S-11 | S-11 @ audit-lane-studio.md:99 | MINOR |
| `STU-S-12` | S-12 | S-12 @ audit-lane-studio.md:106 | MINOR |
| `STU-S-13` | S-13 | S-13 @ audit-lane-studio.md:113 | MINOR |
| `STU-S-14` | S-14 | S-14 @ audit-lane-studio.md:120 | MINOR |
| `STU-S-15` | S-15 | S-15 @ audit-lane-studio.md:128 | MINOR |
| `STU-S-16` | S-16 | S-16 @ audit-lane-studio.md:136 | MINOR |
| `STU-S-17` | S-17 | S-17 @ audit-lane-studio.md:143 | MINOR |
| `STU-S-18` | S-18 | S-18 @ audit-lane-studio.md:150 | MINOR |
| `STU-S-19` | S-19 | S-19 @ audit-lane-studio.md:157 | MINOR |
| `STU-S-20` | S-20 | S-20 @ audit-lane-studio.md:164 | MINOR |
| `STU-S-21` | S-21 | S-21 @ audit-lane-studio.md:171 | MINOR |
| `STU-S-22` | S-22 | S-22 @ audit-lane-studio.md:178 | MINOR |
| `STU-S-23` | S-23 | S-23 @ audit-lane-studio.md:185 | MINOR |
| `STU-S-25` | S-25 | S-25 @ audit-lane-studio.md:199 | MINOR |
| `STU-S-26` | S-26 | S-26 @ audit-lane-studio.md:206 | MINOR |
| `STU-S-27` | S-27 | S-27 @ audit-lane-studio.md:213 | MINOR |
| `STU-S-28` | S-28 | S-28 @ audit-lane-studio.md:220 | MINOR |
| `STU-S-29` | S-29 | S-29 @ audit-lane-studio.md:227 | MINOR |
| `STU-V-01` | （复核新增） | V-01 @ audit-verify-studio.md:18 | MINOR |
| `STU-V-02` | （复核新增） | V-02 @ audit-verify-studio.md:172 | MINOR |
| `SUR-C1` | C1 | C1 @ audit-lane-surfaces.md:338 | MAJOR |
| `SUR-C2` | C2 | C2 @ audit-lane-surfaces.md:346 | MAJOR |
| `SUR-C4` | C4 | C4 @ audit-lane-surfaces.md:358 | MAJOR |
| `SUR-C5` | C5 | C5 @ audit-lane-surfaces.md:364 | MAJOR |
| `SUR-C6` | C6 | C6 @ audit-lane-surfaces.md:370 | MINOR |
| `SUR-C7` | C7 | C7 @ audit-lane-surfaces.md:376 | MINOR |
| `SUR-C8` | C8 | C8 @ audit-lane-surfaces.md:382 | MINOR |
| `SUR-C9` | C9 | C9 @ audit-lane-surfaces.md:388 | MINOR |
| `SUR-S10` | S10 | S10 @ audit-lane-surfaces.md:212 | MINOR |
| `SUR-S11` | S11, LG-37 | S11 @ audit-lane-surfaces.md:224；LG-37 @ audit-lane-surfaces.md:224 | MINOR |
| `SUR-S17` | S17 | S17 @ audit-lane-surfaces.md:290 | MINOR |
| `SUR-S18` | S18 | S18 @ audit-lane-surfaces.md:298 | MINOR |
| `SUR-S4` | S4 | S4 @ audit-lane-surfaces.md:82 | MAJOR |

## 4. 复核后撤销或降级的条目（必备节 1）

审计最大的风险是「把没发生的事写进报告」。这一节是**反向账**：被复核推翻、降级或更正的条目，以及被推翻的那部分为什么不该进下一轮。

### LGC-LG-33（原 KRN-K-03 MAJOR）

- **原判**：validate 通过而 apply 失败、错误指错原因
- **现状**：MINOR：t9 证伪机制（两者同判 Err），只剩错误文案指错原因
- **复核者**：t9 · **出处**：audit-verify-kernel.md:85 一览表

### LGC-LG-34（原 STU-V-03「数据丢失」）

- **原判**：编辑表单读失败 → 提交即抹掉 contract 字段
- **现状**：MINOR：t12 证伪「写盘会抹掉」；真实行为是读失败与「没有 contract」同判为空，真缺陷改写为 LGC-LG-21（apply edit 忽略 contract 键）
- **复核者**：t12 · **出处**：audit-verify-bridges.md 汇总表 V-03 行 + audit-logic-safety-report.md:48

### STR-幻影依赖（t7 结构基线 §3.4 / §6.3 的合并理由）

- **原判**：run_method 的宏无条件发射 `::xirang_debug_method::submit!`，形成幻影依赖 → 建议合并 debug_method
- **现状**：**撤销**：t8 实测证伪（`collector: debug` 才发、且有 `#[cfg(debug_assertions)]` 门控；出厂路径两个示例走 `development` 空 arm）。t14 据此撤销合并推荐。保留的窄缺口是「隐藏开关无文档」（LGC-LH-11，MINOR）
- **复核者**：t8（实测）+ t14（据此改结论） · **出处**：audit-logic-hunt.md:326（LH-11）；audit-boundary-refactor-plan.md 附录 B

### GTE-G-20 的口径更正（gates-auditor 自我更正）

- **原判**：初稿把 `conventions/src/mounting.rs:10` 的裸 `mod` 数报成 335
- **现状**：MINOR：按 mounting 门禁自身口径复算是 33（332 声明 = 裸 33 + `#[path]` 299），与结构基线一致
- **复核者**：t13（夹具复算） · **出处**：audit-lane-gates.md:200（G-20）＋ t6 交付消息

### STU-S-04（渲染期改写 App）

- **原判**：MAJOR
- **现状**：MINOR：t11 部分证实（事实全部成立，严重度建议下调）
- **复核者**：t11 · **出处**：audit-verify-studio.md:66（S-04）

### LGC-LG-50（Studio 错误吞掉，= STU-S-07 + STU-S-24）

- **原判**：总账合并时从 MAJOR 掉到 MINOR 且未写理由
- **现状**：**维持 MAJOR**：按总账自己「取最高」的规则应留 MAJOR（t16 指出）；t11 确证实 S-07 的行为存在。本报告据此保留 MAJOR，并把合并缺理由这一流程问题记在 §方法与口径
- **复核者**：t16 · **出处**：audit-verify-logic.md 第 4 节作者漏项；audit-logic-safety-report.md:464（LG-50）

### BRG-BR-10 的严重度分歧

- **原判**：MINOR
- **现状**：保持 MINOR（t12 建议「升 MAJOR（分歧半）」未被采纳：它是 LGC-LG-28 的一个成员，同一缺陷的整体严重度已由 LG-28 的 MAJOR 承载）
- **复核者**：t12 · **出处**：audit-verify-bridges.md 汇总表 BR-10 行

### 4.1 `mechanism_falsified`：机制被证伪、结论已改写的条目（判定规则见 §1.4 规则一）

| id | 严重度 | 被证伪的原机制 | 保留下来的（已复核）结论 | 复核者 |
| --- | --- | --- | --- | --- |
| `LGC-LG-20` | **MAJOR** | 原报的**符号链接机制**被证伪（t12：外链可读时两侧读到同一字节，diff 看不见）；保留下来的缺陷是改写后的结论——临时路径可预测导致同名既有目录被删除重建，且复制失败时残留半份副本（t12 复现 `-416016-0` 残留） | 预览副本的临时路径可预测：同名既有目录会被删除重建（并留泄漏） | t12,t16 |
| `LGC-LG-33` | **MINOR** | 原报的「validate 通过而 apply 失败」机制被证伪（t9：两者同判 Err）；保留下来的缺陷是错误文案指错原因 | `validate_*` 通过而 `apply_*` 失败时，错误文案指错原因（机制已被证伪） | t9 |
| `LGC-LG-34` | **MINOR** | 原报的「提交即抹掉 contract 字段」被证伪（t12：写盘不会抹掉）；保留下来的缺陷是读失败与「没有 contract」同判为空、表单显示空契约，真缺陷已改写为 LGC-LG-21 | Studio Edit 表单在文件读不到时显示空契约（写盘不会抹掉；真缺陷是 LGC-LG-21） | t11,t12 |
| `NAM-40` | **MINOR** | 原结论「394/394 一致、0 违规」被 t27 证伪；改写为 14 处四族不一致（谓词：`module_path` 非空 ∧ 末段 ≠ 文件 stem） | C. 模块名 · 模块路径与文件名的对应 | t27,t31 |
| `AMB-06` | **MINOR** | 原报说工作区根的 `examples/` 会被 Cargo 自动发现成 example target；实际根清单是虚拟清单且成员是显式列出的（`Cargo.toml:1-2`），自动发现是**按包**找 `examples/*.rs`/`examples/*/main.rs`，与顶层目录无关。保留下来的只是「同名目录在不同工具里的默认含义不同」这一现象。 | 顶层 `examples/` | t35 |

这三条**仍然留在队列里**：保留下来的那个缺陷是可复现、已复核的，被证伪的只是原报告对**缺陷怎么发生**的解释。它们的 `verify_status` 因此是 `partial` 而不是 `falsified`。

### 4.2 数字更正（不改判、只改数）

三处**数字更正**（不改判、只改数，都来自复核）：

- `GTE-G-20` 的裸 `mod` 数：初稿 **335 →** 复算 **33**（332 声明 = 裸 33 + `#[path]` 299）；散文里的「29」也已过期，正确值是 33。
- `STU-S-16`（库内 panic 面）：片区报告写「16 处」，t11 复核认为该数字**不可复现**，应按抽样清单读。
- `BRG-BR-19`：原报的**符号链接机制**被证伪，泄漏面换成另一条可复现路径（见 §3.1 合并行 `LGC-LG-20`）。

## 5. 待维护者决策（必备节 2）

这一节的四条**不替维护者拍板**：每条给选项、代价与受影响面。本报告不对它们给出推荐排序，因为取舍取决于产品意图（预览承诺什么、命名空间以谁为准、是否愿意放弃整线同步的保证）。

### DEC-01 MCP 预览的副本是否跳过 `.git` 等目录（先回答「预览承诺的是什么」）

**背景**：t12 端到端实测：每次预览都把整棵包复制一遍，`.git` 既被复制又被列举（BRG-BR-4/LGC-LG-23、BRG-BR-19/LGC-LG-20）。问题不是「复制慢」，而是「预览的语义边界」：若承诺是「宿主看到的包」→ 应当复制 `.git`；若承诺是「会进发布的文件」→ 应当跳过 `.git`、`target/`、以及本仓的 `examples/*/target/`。两种语义都自洽，但当前实现既没有声明、也没有豁免清单。

| 选项 | 代价 | 受影响面 |
| --- | --- | --- |
| A. 复制整个包（含 .git），但把「哪些被跳过」写进工具自述并加豁免开关 | 低（文档 + 一个可选参数） | mcp 预览路径、README/自述、BR-4 的「非 UTF-8 报成新增」仍需另修 |
| B. 默认跳过 .git/target 等构建与 VCS 目录，跳过清单写进自述 | 中（遍历集合要与「包内容」的门禁对齐，tools/xirang-package-audit 已有同类清单可复用） | 预览 diff 的忠实度、CI 包内容门禁、上一条的测试夹具 |
| C. 不做语义决定，只加大小上限并报出「被跳过的目录」 | 低 | 大仓库仍然慢，但不再有静默不一致 |

**出处**：audit-lane-bridges.md:109（BR-4）；audit-logic-hunt.md:227（LH-06）；audit-verify-bridges.md 汇总表

**备注**：本轮不替维护者拍板；无论选哪条，BR-4 的「非 UTF-8 文件被报成新增」都是独立缺陷，应同时修。

### DEC-02 `XIRANG_NAMESPACE` 下命名空间自相矛盾：verify 用 Cargo 名、diff/search 用覆盖名

**背景**：t12 活体复现（BRG-BR-6/LGC-LG-13，含遗留 S12）：同一份覆盖命名空间的项目里，`xirang.verify` 报 `reidentified 3`、`build current`，而 `diff`/`search` 按覆盖名读；两边都「自信」，用户无法从输出判断谁对。

| 选项 | 代价 | 受影响面 |
| --- | --- | --- |
| A. 统一用 root 命名空间（verify 改读 root） | 低（一处读取口径），但要确认 verify 的语义确实是「整棵根」 | mcp verify/diff/search 三条工具、MCP README 的语义段落 |
| B. 保留两套但拒绝不一致：检测到覆盖名与 Cargo 名不同就报错并提示 | 中（要定义「不一致」的判据并加测试） | 同上 + 错误文案与 exit code 约定 |
| C. 状态 quo：在 README 里明说两套并存 | 零 | 用户仍需自己判断，但不再是静默不一致 |

**出处**：audit-lane-bridges.md:151（BR-6）；audit-logic-safety-report.md:198（LG-13）

**备注**：推荐不写在这里，选项 A/B 的取舍取决于 verify 的设计意图，需维护者定。

### DEC-03 crate 顶层是否仍要减一个（debug_method）

**背景**：t7 结构基线原本建议把 `debug_method` 并入 `run_method` 的一个非默认特性，理由是「幻影依赖 + 444 行 + 无安装面」；但该论证的前提被 t8 实测**证伪**（出厂路径不依赖它，只依赖 run_method 的宿主显式写 `collector: debug` 才会撞 E0433，而那是 debug-only 且无文档的隐藏开关）。t14 因此撤销合并推荐，本轮 crate 边界不动。

| 选项 | 代价 | 受影响面 |
| --- | --- | --- |
| A. 维持 12 成员（t14 的结论） | 零 | 无；把「隐藏开关」按 LGC-LH-11 补文档即可 |
| B. 仍要减：按 t14 附录 B 的可执行步骤走（debug_method → run_method 的非默认 evidence 特性） | 高且不可回滚（触已发布 crate 的公开路径、需要一个版本线推进） | run_method/macro 的 `collector:` 契约、两个示例、Studio/MCP 的依赖、发布表 |
| C. 更小的中间态：保留 crate，但在 ADR 里写明「为什么它必须独立」 | 低 | 文档；为下次同类判断留下可审计的契约（对应 STR-R-03） |

**出处**：audit-structure-base.md:256（幻影依赖，已被证伪）；audit-logic-hunt.md:326（LH-11 证伪）；audit-boundary-refactor-plan.md 附录 B

**备注**：以 t14 的修正后版本为准；不要引用 structure-base 里已被证伪的幻影依赖论证。

### DEC-04 版本线政策：是否放宽 `req == workspace_version`，或改用 `[workspace.dependencies]`

**背景**：t14 实测：`tools/xirang-publish` 强制每个内部依赖的 req 等于工作区版本，因此一次版本线推进要动 13 个文件、同步 20 处内部要求；而 `docs/roadmap-1.0.md:74` 的决策 1 仍写着「各 crate 独立发版」的理由——文档与工具的门禁区已经矛盾（GTE-G-16、GTE-G-27）。

| 选项 | 代价 | 受影响面 |
| --- | --- | --- |
| A. 保留强制相等，改文档（删掉「独立发版」的旧理由） | 低（文档 + 一条门禁的自述） | `docs/roadmap-1.0.md` 与 `AGENTS.md`、发布脚本的自述；不改善发版成本 |
| B. 放宽为「兼容范围」（如 `^0.1`），允许各 crate 独立推进 | 高（要放弃当前的单一版本线保证，发布表与 external-rehearsal 都要改判据） | tools/xirang-publish、tools/xirang-external-rehearsal、CI 发布流、CHANGELOG 形态 |
| C. 用 `[workspace.dependencies]` 收敛（工具已支持，根清单 0 次使用） | 低到中（R-01 已给出清单；一次改动同时消除 19 处内部 req 与 12 处外部重复） | 根 Cargo.toml + 各成员清单；工具侧零改动 |

**出处**：audit-boundary-refactor-plan.md:334（R-02）、:315（R-01）；audit-lane-gates.md:154/186（G-16/G-27）

**备注**：A 与 C 可并存（C 收敛写法、A 明确政策）；B 会改变发布保证，代价最高。

## 6. 注释清晰度与可读性（必备节 3）

把五路片区 + 逻辑总账的该类发现合并去重后共 **112** 条（MAJOR 29 / MINOR 82），逐条见 §6.2 与 `audit-findings.json`。

### 6.1 结构性结论：这不是「注释写多了」，而是**判定落在文本上**

`audit-logic-safety-report.md` §5.2 给了 **18 条反例**，它们形状一致：门禁/校验**取文本子串或行序**，而不是取解析结果。因此只要代码换个等价写法，门禁就静默放行——本轮 `LGC-LG-25`（tag 守卫）、`LGC-LG-04`（锁 schema）、`LGC-LG-53`（报告豁免）三条都是这个形状的实例，而且都不是「注释写错」，是「判定本身不可靠」。

`audit-logic-safety-report.md` §5.1 另给了 **25 条「自述 vs 行为」实例**（其中 8 条行为本身也是错的）。本报告把这两节当作「这一类问题的总账」，不逐条重复；下表只列**可执行**的注释类条目。

### 6.2 注释类条目清单（按严重度）

| id | 严重度 | 位置 | 问题 | 复核 |
| --- | --- | --- | --- | --- |
| `BRG-BR-C2` | MAJOR | `mcp/preview.rs:14` | `mcp/preview.rs` 的目录说明与它实际跳过的集合不符 | 已复核 |
| `GTE-G-16` | MAJOR | `mcp/Cargo.toml:27` | 文档-漂移：roadmap 与 `xirang-package-audit` 头部写的发布层级顺序与活表矛盾，且按它发布必然失败 | 已复核 |
| `GTE-G-21` | MAJOR | `tools/xirang-package-audit:5` | 注释-准确性：`xirang-package-audit` 的头部注释描述了一张已不存在的表 | 已复核 |
| `KRN-C-01` | MAJOR | `core/src/registry_core/authoring/parse/admission.rs:5` | 注释描述的行为与实现不符（模块自述承诺“两种拼法同一个门禁”，实际丢字段） | 已复核 |
| `KRN-C-02` | MAJOR | `core/src/registry_core/declaration/call_evidence.rs:6` | 注释描述的类型与实现不符（`CallSite` 的文档写的是“一条调用边”） | 已复核 |
| `KRN-C-03` | MAJOR | `core/src/registry_core/tree/ports/ports.rs:29` | 模块自述的解析规则与同模块的另一函数相反（“绝不静默解析”vs `resolve_path` 取第一个） | 已复核 |
| `LGC-LG-23` | MAJOR | `mcp/preview.rs:102` | 预览 diff 把每个非 UTF-8 文件报成「新增」，无上限，且每次预览复制 `.git` | 已复核 |
| `LGC-LG-26` | MAJOR | `conventions/src/release_workflow.rs:60` | 持有 `CARGO_REGISTRY_TOKEN` 的 job 的 action pin 只由注释承诺，无门禁 | 已复核 |
| `LGC-LG-32` | MAJOR | `plugin-host/src/process.rs:341` | 子进程已给完整帧却不退出时，答案被丢成 `Timeout`（与注释承诺相反） | 已复核 |
| `STU-C-01` | MAJOR | `studio/studio/app/navigation.rs:165` | 注释与实现不符 · `studio/studio/app/navigation.rs:165-171` | 已复核 |
| `STU-C-02` | MAJOR | `studio/studio/app/writers.rs:1` | 注释与实现不符（模块自述） · `studio/studio/app/writers.rs:1-22` | 已复核 |
| `STU-C-03` | MAJOR | `studio/studio/app/search_queries.rs:9` | 注释与实现不符 · `studio/studio/app/search_queries.rs:9-17` | 已复核 |
| `STU-C-04` | MAJOR | `studio/studio/app/support.rs:1` | 注释范围与实现不符 · `studio/studio/app/support.rs:1-2` | 已复核 |
| `SUR-C1` | MAJOR | `macro/src/front_end.rs:18` | 文档块与声明错位：`split_semicolons` 的文档挂到了 `splice` 上 | 已复核 |
| `SUR-C2` | MAJOR | `build_method/discovery.rs:234` | 同型错位：`collect_rust_sources` 的文档挂在 `StdSourceTree` 上 | 已复核 |
| `SUR-C4` | MAJOR | `run_method/macros/face.rs:139` | 编辑器悬停文本与宏契约不符：`parent` 被标成 Required | 已复核 |
| `SUR-C5` | MAJOR | `run_method/macros/entry.rs:18` | 术语不一致：注释里出现两个不存在的 crate 名 | 已复核 |
| `BRG-BR-C3` | MINOR | `mcp/callgraph.rs:10` | 同一实测事实在注释里有两个数字（142 vs 151） | 已复核 |
| `BRG-BR-C4` | MINOR | `cli/lib.rs:169` | 文档注释里的工具名拼错：`nihlink` | 已复核 |
| `BRG-BR-C5` | MINOR | `mcp/evidence.rs:112` | 一个桥里 `slot` 有三个含义；同一状态有三种拼法 | 已复核 |
| `BRG-BR-C6` | MINOR | `mcp/evidence.rs:40` | 该由类型承担的信息被写进注释：无名三元组与半真名字 | 已复核 |
| `BRG-BR-C7` | MINOR | `mcp/apply.rs:94` | `apply()` 靠注释说明一组按约定成立的不变量 | unverified |
| `GTE-G-07` | MINOR | `conventions/src/naming.rs:238` | 门禁-假阳性：`naming` 的"要求位置"扫描不跳过注释 | 已复核 |
| `GTE-G-08` | MINOR | `conventions/src/lib.rs:247` | 门禁-假阳性：共享助手 `drop_governed_lines` 把属性与声明之间的文档注释误判成"声明" | 已复核 |
| `GTE-G-17` | MINOR | `conventions/src/size.rs:66` | 文档-漂移：size 棘轮的条目数在文档里是 14，在代码里是 1 | 已复核 |
| `GTE-G-18` | MINOR | `CHANGELOG.md:31` | 文档-漂移：CHANGELOG 用现在时引用了工作树里已不存在的清单拼写 | 已复核 |
| `GTE-G-19` | MINOR | `AGENTS.md:151` | 文档-漂移：`AGENTS.md` 的门禁清单比实际门禁宽，且缺 4 道 | 已复核 |
| `GTE-G-20` | MINOR | `conventions/src/mounting.rs:10` | 注释-准确性：`conventions/src/mounting.rs` 的散文数字已过期（29 → 33），而散文里的常量没有对账机制 | 已复核 |
| `GTE-G-22` | MINOR | `conventions/src/lint.rs:15` | 注释-准确性：`conventions/src/lint.rs` 的模块文档低估了自己的范围 | 已复核 |
| `GTE-G-24` | MINOR | `conventions/src/size.rs:124` | 注释-准确性：`conventions/src/size.rs` 一句话里的层数与它列出的证据不一致 | 已复核 |
| `GTE-G-25` | MINOR | `CHANGELOG.md:14` | 注释-准确性：`ci.yml` 留了一条条件已经满足的悬挂指令 | 已复核 |
| `GTE-G-26` | MINOR | `AGENTS.md:196` | 注释-可读性：用户可见输出里用"from today / 从今天起" | 已复核 |
| `GTE-G-27` | MINOR | `build_method/Cargo.toml:16` | 文档-漂移：roadmap 说依赖要求写 `0.1.0` 以便不必连锁重发，而工作树里写的是 `^0.1.6`，且两道门禁**强制**它等于工作区版本 | 已复核 |
| `KRN-C-04` | MINOR | `core/src/registry_core/tree/graft_ops/record.rs:24` | 同一段说明重复三页、历史叙事进入生产文件 | 已复核 |
| `KRN-C-05` | MINOR | `core/src/registry_core/tree/registry.rs:1` | 文档头用 `//` 而非 `//!`，与同层模块不一致 | 已复核 |
| `KRN-C-06` | MINOR | `core/src/registry_core/release/release.rs:259` | 同一个文档注释里两个 `# Panics` 段，第一段讲的是别的事 | 已复核 |
| `KRN-C-07` | MINOR | `core/src/registry_core/declaration/runtime_checks_tests.rs:1` | 同一模块文档整段重复（第二份只多一句） | 已复核 |
| `KRN-C-08` | MINOR | `core/src/registry_core/plugin/trust/trust.rs:86` | 安全相关的参数没有文档，回退语义只存在于代码里 | 已复核 |
| `KRN-C-09` | MINOR | `core/src/registry_core/authoring/field_presentation.rs:248` | 字段帮助文本讲的是另一个机制（`runtime_checks`） | unverified |
| `KRN-C-10` | MINOR | `core/src/registry_core/mir/model.rs:28` | 名字承担不了信息，只能靠注释补（`mir_line` 与 `resolve_*` 家族） | 已复核 |
| `KRN-C-11` | MINOR | `core/src/registry_core/plugin/contracts/contracts.rs:35` | 术语一名多指（同一个“槽位名”概念四种叫法） | 已复核 |
| `KRN-K-10` | MINOR | `core/src/registry_core/authoring/parse/rules.rs:27` | D 大逻辑（注册规则用“整文件文本启发式”解析，可读到注释里的伪子句） | 已复核 |
| `STU-C-05` | MINOR | `studio/studio/app/lifecycle.rs:185` | 注释位置 · `studio/studio/app/lifecycle.rs:185-190` | 已复核 |
| `STU-C-06` | MINOR | `studio/studio/ui/status.rs:37` | 注释与实现不符（页脚合同） · `studio/studio/ui/status.rs:37-48` | 已复核 |
| `STU-C-07` | MINOR | `studio/studio/app/graft.rs:69` | 术语一致性 · `studio/studio/app/graft.rs:69`、`studio/studio/app/graft.rs:78`、`studio/sr …（全文见 `audit-findings.json`） | 已复核 |
| `STU-C-08` | MINOR | `studio/studio/app/hot_zones.rs:40` | 悬挂预留 · `studio/studio/app/hot_zones.rs:40`、`studio/studio/app/hot_zones.rs:43` | 已复核 |
| `STU-C-09` | MINOR | `studio/studio/app/graft.rs:129` | 注释密度/信噪比 · `studio/studio/app/graft.rs:129-158`、`studio/studio/ui/graph/node_graph …（全文见 `audit-findings.json`） | unverified |
| `SUR-C6` | MINOR | `run_method/authoring/operations/face_write.rs:85` | 同一个字段集合被三处注释写成三个数 | 已复核 |
| `SUR-C7` | MINOR | `run_method/macros/face_objects.rs:62` | 复述与残句：`run_method/macros/face_objects.rs` 里三段讲"已删除的 arm"的注释互相重复 | 已复核 |
| `SUR-C8` | MINOR | `build_method/cache.rs:34` | 该由名字承担的信息被塞进注释：`write_if_changed` 的文档没说它不原子 | 已复核 |
| `SUR-C9` | MINOR | `build_method/pipeline.rs:105` | 注释密度：反例与正例并存（供维护者判断，不作为缺陷） | 已复核 |
| `SUR-S11` | MINOR | `build_method/contracts.rs:193` | 假实现／诊断质量 · `rule_method_strings` 只读第一处调用，且对注释与转义引号不设防 | 已复核 |

### 6.3 三条可以立刻用起来的正面样本（复核报告亲自点名的）

- `studio/studio/app/graft.rs:1-21` 用三段 bullet 把 declaration / application / record 分开，被 t11 点名为「本仓注释质量的上限」，可直接当模板（`audit-lane-studio.md:316`）。
- 双语成色经抽查成立：`studio/studio/app/support.rs:267-276` 连 "Cargo decides which targets exist" 都译了出来（`audit-lane-studio.md:315`）。
- 内核的 `//!` 头注释说清了「官方地址是模块路径，只有精选名字被承诺为裸名可用」（`core/src/lib.rs:1-18`），这种把契约与便利分开写的做法值得在别的 crate 推广。

## 7. 命名：抽象与可读性 / 歧义与约定冲突（文件名 / 目录名 / 模块名 / 函数名）

本节把 2026-09-28 的**命名抽象专项**（`audit-naming-review.md`，t25；t30 按复核修订）并入总报告。共 **30** 条命名条目（id 前缀 `NAM-`，批次 `B7-命名抽象与可读性`），复核状态：confirmed 20、partial 8、unverified 2。复核者：`audit-naming-verify.md`（t27，逐条复核 7 条 MAJOR + 17/21 条 MINOR）与 `audit-naming-verify2.md`（t31，复判修订后的 9 条 finding，verdict=pass）。

**与 §6 的分工**：§6 管「注释说了什么、与实现是否相符」；本节管「名字（文件/目录/模块/函数）是否说出了它是什么」。两者都是「自述 vs 行为」的一部分，但判定对象不同：前者是散文，后者是标识符。

### 7.1 A/B/C 三轴：旧名 → 建议新名 → 判据

（本表是**命名轴视图**；其中 5 条 MAJOR 同时出现在 §2.2 的行动队列里，两处同源、不重复计数。）

#### A 文件名与目录名（19 条）

| id | 严重度 | 旧名（现状） | 建议新名 | 判据（摘要） | 复核 |
| --- | --- | --- | --- | --- | --- |
| `NAM-01` | MAJOR | `studio/studio/app/support.rs:1（文件级定位）` | `studio/studio/app/project_context.rs`、`studio/studio/app/support.rs` | 文件 509 行、23 个函数。模块文档第 1 行写「Shared interaction geometry and editor helpers」（共享交互几何与编辑器辅助），但前 12 个 item 是**项目上下文与 cargo 子进程**：`select_project`（:23）、`pac …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-02` | MINOR | `studio/studio/app/state/misc.rs:55` | `studio/studio/app/state/misc.rs`、`state/call_tree_view.rs`、`state/reload.rs` | 181 行，模块文档自陈「Ungrouped Studio state: reload errors, overlays, and call references」（**未归类**的 Studio 状态）。内容是 `CallRef`（:55）与 `CallTreeView`（:76，`refs: V …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-03` | MAJOR | `run_method/authoring/validation/validation.rs:1（文件级定位）` | `run_method/authoring/context.rs`、`context`、`run_method/authoring/validation/validation.rs` | 文件叫 `validation`，内容是 `AuthoringContext`（:47 的 `new(package_root, namespace)`、:56 的 `scope(...)`）与**进程级环境访问器** `authoring_namespace()`（:71）、`package_ro …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-04` | MAJOR | `build_method/cache.rs:1（文件级定位）` | `discovery_cache.rs`、`build_method/scope.rs`、`build_method/cache.rs` | 文件叫 `cache`，8 个函数里只有一半是缓存：`update_discovery_cache`（:77）、`cached_parent_id`（:184）、`collect_discovery_rows`（:205）；另一半是**作用域判定**与**通用写盘**：`write_if_chang …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-05` | MINOR | `mcp/evidence.rs:65` | — | 261 行，7 个函数：`out_dir`（:36）、`build_evidence`（:46）、`explain`（:65，即 `xirang.explain` 工具的 handler）、`node_report`（:104）、`scope_line`（:141）、`pruning_line` …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-06` | MINOR | `studio/studio/app/writers.rs:1（文件级定位）` | `studio/studio/app/writers.rs` | 42 行、2 个函数；名字 `writers` 读起来像"所有写入都在这儿"，而它其实是一个**写入守卫/上下文包装**的窄缝。 名字暗示的实现范围比文件实际承担的更宽（读者会以为写盘逻辑在此，实际写盘在 `studio/studio/app/mutations.rs` + `run_met …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-07` | MINOR | `core/src/registry_core/tree/connector/connector.rs:1（文件级定位）` | — | **t27 复核后的修订（谓词口径）**：本节原写"39 处"，用的是更松的谓词。三档谓词实测如下（命令与输出见 §G-1）： 结果 | --- | **34**（其中十个发布 crate 内 **28**，另 6 个在两个 example 宿主与 Studio 夹具里） | 39 —— 多出的 5 …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-08` | MINOR | `run_method/runtime/trace/call_trace.rs:1（文件级定位）` | `locals/recording.rs`、`run_method/runtime/trace/locals/call_trace.rs`、`locals/local_recording.rs` | `run_method/runtime/trace/call_trace.rs`（348 行，模块文档「调用追踪收集器及其记录的协议表面」）与 `run_method/runtime/trace/locals/call_trace.rs`（228 行，模块文档「`CallTrace` …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-09` | MINOR | `run_method/authoring/face_manifest.rs:1（文件级定位）` | `authoring/face_file.rs`、`run_method/authoring/face_manifest.rs`、`face_store.rs` | `run_method/authoring/face_manifest.rs`（89 行，自述「File-backed authoring for XiRang registration faces」，做文件化创作）与 `run_method/authoring/manifest …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-10` | MINOR | `run_method/authoring/parse/parse.rs:1（文件级定位）` | `authoring/parse/historical.rs` | `run_method/authoring/parse/parse.rs` 54 行，模块文档第 3–4 行自陈「pure source-text transformations live in the kernel… **this shim** keeps the historical [ …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-11` | MINOR | `mcp/evidence.rs:1（文件级定位）` | `studio/studio/app/source_index.rs`、`mcp/index.rs`、`snapshot.rs` | （机器统计 + 逐个读模块文档）： | 词 | 出现处 | 各自含义 | | --- | --- | --- | | `evidence` | `mcp/evidence.rs`、`run_method/runtime/evidence.rs`、`studio/studio/ …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-12` | MINOR | `mcp/search.rs:1（文件级定位）` | — | studio 有 `studio/studio/app/overlay/search.rs`（1 fn）、`studio/studio/app/state/search.rs`（0 fn，状态）、`studio/studio/ui/search.rs`（1 fn，入口）、`u …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-17` | MINOR | `build_method/faces.rs:1（文件级定位）` | `scope_faces.rs`、`build_method/faces.rs`、`face_discovery.rs` | `build_method/faces.rs`（96 行，`FaceSource`/`collect_faces`/`has_selected_face`，做**首轮作用域的面发现**）与 `build_method/face_view.rs`（444 行，`FaceView`/`f …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-18` | MINOR | `build_method/node.rs:1（文件级定位）` | `discovery_node.rs`、`node_identity.rs`、`build_method/node.rs` | `build_method/node.rs`（26 行，`Node` = 已发现的注册模块树节点 + `relative_display`）与 `build_method/node_id.rs`（117 行，`node_id()`/`collect_node_ids`/`select …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-19` | MINOR | `studio/studio/app/tests/project.rs:1（文件级定位）` | — | `studio/studio/app/tests/` 11 个文件按特性命名：`studio/studio/app/tests/call_tree.rs`（空间调用树回归，文件文档已写清）、`studio/studio/app/tests/trace_ingest.rs`（端 …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-20` | MINOR | `mcp/index.rs:1（文件级定位）` | — | - **判据**：`mcp/src` 平铺 39 个文件时，`mcp/index.rs`/`mcp/evidence.rs`/`mcp/registry.rs`/`mcp/search.rs` 与各自语义靠文件名硬区分；一旦按职责分成 `tools/`、`source …（全文见 `audit-findings.json`） | unverified |
| `NAM-21` | MINOR | `build_method/discovery.rs:1（文件级定位）` | `source_walk.rs`、`registration_phase.rs`、`face_syntax_check.rs` | `build_method/discovery.rs`（272 行，遍历与指纹）、`build_method/node.rs`、`build_method/node_id.rs`、`build_method/faces.rs`、`build_method/fa …（全文见 `audit-findings.json`） | unverified |
| `NAM-22` | MINOR | `mcp/nodes.rs:1（文件级定位）` | `mcp/resolve.rs`、`mcp/nodes.rs`、`node_refs.rs` | 76 行，两个函数 `parent_id`（:27）、`resolve_node`（:58）；模块文档说"代理所命名的东西，解析成执行器需要的身份"。 文件名 `nodes` 是名词复数、且与 `studio/studio/ui/graph/nodes.rs`（画节点）同名不同义（A④）；它 …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-23` | MINOR | `plugin-host/src/lazy_wasm.rs:24` | — | `plugin-host/src/lazy_wasm.rs:24` 把内核的 `PluginChannel` 重导出为 `ValidationChannel`，文档（:20-23）说明是"保留历史路径"。 别名 `ValidationChannel` 与本体 `PluginChannel` 不同名不 …（全文见 `audit-findings.json`） | 已复核 |

#### B 函数名与方法（8 条）

| id | 严重度 | 旧名（现状） | 建议新名 | 判据（摘要） | 复核 |
| --- | --- | --- | --- | --- | --- |
| `NAM-30` | MAJOR | `build_method/manifests.rs:208` | — | `collect_pruning_symbols`（:173）说要收集"剪枝符号"，实际对每一行做魔法标识符匹配；`parse_pruning_item`（:200-213）的第三个分支**无条件返回字面量** `"Button::optional_pruning_probe"`（:210），不管当 …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-31` | MAJOR | `run_method/authoring/operations/face_write.rs:1（文件级定位）` | `apply_trait_label` | 函数名说"应用 trait 契约"；实现（:190-205）在 `labels` 与 `contract_paths` 都空时直接 `return Ok(())`，否则只写**trait 标签**（`face.edit(label_field, …)`）；真正的 `handle_contracts: …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-32` | MINOR | `build_method/lib.rs:137` | — | 生产函数（`is_test == false`）∧ 名字**整名等于**裸动词集合中一个 ∧ `kind ∈ {inherent_method, free_fn, nested_fn}`（另单列 `trait_method`）。 - **输出：全量 24 行**（`名字`（container）`fi …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-33` | MINOR | `core/src/registry_core/tree/query/query.rs:1（文件级定位）` | — | （全量机器统计：生产函数名的首词直方图，只列 ≥5 的读类动词）： | 动词 | 计数 | 主要出现处 | 现在的实际语义 | | --- | --- | --- | --- | | `collect` | 39 | `core/src/registry_core/tree/query/query. …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-34` | MINOR | `core/src/registry_core/authoring/parse/parse.rs:258` | — | 生产函数名含 `_owned/_leak/_boxed/_vec/_raw/_strings/_string/_opt/_impl` 之一，排除 Rust 惯用语 `into_owned`/`from_owned`/`from_raw`。**输出 25 行**。 - **计入 D-7 的 8 行** …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-35` | MINOR | `core/src/registry_core/release/release.rs:191` | `is_full`、`skips_path`、`targets_framework` | （全量 `grep "fn … -> bool"` + 前缀过滤）：绝大多数 `-> bool` 函数都带谓词前缀（`is_`/`has_`/`can_`/`needs_`/`supports_`/`accepts_`/`matches_`/`keeps_`，共约 120 处）。三处例外：`core …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-36` | MINOR | `core/src/registry_core/tree/query/query.rs:80` | `find_by_id`、`visit_kinds`、`walk_depth_first` | `core/src/registry_core/tree/query/query.rs:80 pub fn find(&self, id: NodeId) -> Option<&RegistrationSnapshot>`；紧邻 `:146 pub fn find_kind(&self, kind: …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-37` | MINOR | `core/src/registry_core/tree/entry_pages/entry_pages.rs:39` | `EntryPages::entry`、`remove_entry` | ① `core/src/registry_core/tree/entry_pages/entry_pages.rs:39 get` / `:74 remove`（单数动词，操作单个 entry；同文件 `:43 get_mut` 是容器惯用语，**不计**）；同族的公共查询却叫 `Registry: …（全文见 `audit-findings.json`） | 已复核 |

#### C 模块名（3 条）

| id | 严重度 | 旧名（现状） | 建议新名 | 判据（摘要） | 复核 |
| --- | --- | --- | --- | --- | --- |
| `NAM-40` | MINOR | `build_method/identity.rs:1（文件级定位）` | — | **t27 复核后的修订（结论级更正）**：本报告原写"394/394 `module_path` 末段与文件 stem 一致、0 违规、唯二有意例外"。**这条是错的**。 **谓词（精确写法）**：`module_path` **非空** ∧ `module_path.split("::").l …（全文见 `audit-findings.json`） | 已复核 |
| `NAM-41` | MINOR | `plugin-host/src/lazy_wasm.rs:1（文件级定位）` | — | 全仓 4 处（见 NAM-23）。三处有明确理由，`ValidationChannel` 一处是语义漂移。 别名让"同一物两套名字"成为长期状态，而本仓的 `lexicon` 与 `shims` 门禁已经在管别的"两套名字"问题；别名应当比普通重导出有更高的说明要求。 | 已复核 |
| `NAM-43` | MINOR | `build_method/identity.rs:1（文件级定位）` | — | （谓词 P1，§G-1）：14 处不同名，分四族： 1. **历史 shim 保路径**（2）：`build_method/identity.rs` 挂成 `pub mod registry_identity;`、`build_method/syntax.rs` 挂成 `pub mo …（全文见 `audit-findings.json`） | 已复核 |

### 7.2 命名共识（可执行规则，供 `conventions` 落地）

以下 11 条来自专项报告 §D，每条都写成「规则 → 机械判定谓词 → 现状违规数」，**建议按棘轮落地**（先钉住现状清单、只许变短，与 `conventions/src/size.rs` 的做法一致）。谓词与命令的完整可复算脚本在专项报告 §G（P1–P8）。

| # | 规则 | 机械判定 | 现状 |
| --- | --- | --- | --- |
| D-1 | **文件名不得是类别名**：`support|misc|helpers|helper|common|utils|util|shared|types|base|general|stuff` 不得作为 `.rs` 文件名（测试与挂载根除外） | 文件 stem ∉ 黑名单 | 2 违规（NAM-01/02，都在 studio） |
| D-2 | **一个文件一个主语**：模块文档第一行必须出现该文件的主语（类型名/动作名），且主语与文件名同义 | 人工（doc 首行与文件名对照）→ 可作为文档门禁的一条检查 | 抽样 28 个文件中 3 个不符（NAM-01/03/04） |
| D-3 | **无载荷目录收平**：若目录下**递归**所有 `.rs` 恰为 `{<dir>.rs}`，则该目录应不存在（用裸 `mod x;`）——**豁免 `examples/**` 与 `studio/tests/fixtures/**`**（身份与字面路径，见 NAM-07 的豁免表） | 递归 `.rs` 集合比对（§G-1 的 **P2d**，即 NAM-07 的 **A1**）**∧ 路径不以 `examples/` 或 `studio/tests/` 开头** | 三个口径：**仓库 34 / 发布面 28 / 豁免后可达 0**；不再用 39（NAM-07 的 A2）或 67（A3） |
| D-4 | **同一动作全仓一个动词**：读文件/reader=`read_*`；装成结构=`load_*`；输入→实体=`resolve_*`；集合查找=`find_*`（禁 `get_*`，容器惯用语 `get_mut` 除外）；累积返回集合=`collect_*`（内部递归用 `visit_*`/`walk_*`）；文本→结构=`parse_*`；结构→文本=`render_*`；写盘=`write_*`；检查并返回原因=`validate_*`；运行期断言=`check_*` | 首词直方图 + 白名单 | `get` **1**（`core/src/registry_core/tree/entry_pages/entry_pages.rs:39`；同文件 `:43 get_mut` 排除）；内部 helper 借 `collect_*` **3 处已确证**（`core/src/registry_core/tree/query/query.rs:105`、`:164`、`:183`），全仓清单未列 → 标「待清单」（t27 亦无法判定） |
| D-5 | **裸动词只许出现在入口位**：整名是一个裸动词（`run|apply|check|build|load|read|handle|call|do|process|update|stop`）的函数，只允许是 crate 根入口、CLI 命令同名实现、trait 声明/实现、或 `main` | 名字 ∈ 裸动词集合 ∧ 位置 ∉ 白名单 | **非入口位 15 处**（全量 24 行清单见 NAM-32；t27 复核数到 ≥14） |
| D-6 | **布尔谓词要读成一个问句**：`-> bool` 的公开函数名以 `is_/has_/can_/should_/needs_/supports_/accepts_/matches_/keeps_` 开头（**门禁只认前缀**；「或带宾语」不可机械判定，落地时要么改成纯前缀白名单（违规数会升到含 `skips`/`targets` 一级），要么人工维护豁免清单） | `grep "-> bool"` + 前缀白名单 | 前缀口径 **3** 违规（NAM-35） |
| D-7 | **名字描述意图不描述实现**：主要区分轴不得是 `_owned/_strings`（`into_*`/`from_*`/`from_raw` 的 Rust 惯用语除外；`*_string` 一类指「源码里的字符串字面量」属领域用词，不在本规则内） | 名字含实现词根 ∧ 不在惯用语/领域词表 | **8** 违规（全量 25 行与分类见 NAM-34） |
| D-8 | **别名同名同义**：`pub use X as Y` 仅当 Y 是该概念的（历史）同名，且 Y 的文档首行写明"历史路径/别名" | 全量 grep + doc 首行检查 | 1 需补文档（NAM-23），3 合规 |
| D-9 | **保留词表**：`evidence`/`index`/`artifact`/`registry`/`face`/`graft`/`trace` 在本仓已被载义；同一词出现第二个含义时，新文件必须加限定词（`source_index`/`trace_snapshot`/`build_evidence`） | 保留词 × 文件 stem 的映射表 | 4 个词各一名多义：`evidence`（NAM-05/11）、`index`、`artifact`、`registry`（NAM-11）；`mcp/evidence.rs` 的改名按 NAM-05 采用 `build_evidence.rs`。**落地名已由 t31 裁定（本报告与之对齐）**：`debug_method` 的模块名取 **`call_evidence`**（特性名仍是 `evidence`）；**`build_evidence` 是 mcp 侧文件的名字**，不是该模块的名字——两者不冲突，别混用 |
| D-10 | **类型单数、模块复数**（`local_value.rs` 装 `LocalValue`，目录 `locals/`）；**文件名**镜像生产文件（`x.rs` ↔ `x_tests.rs`；**模块名自由**——现状 `*_tests.rs` 统一挂 `mod tests`，`registry_identity`/`*_command`/`explain::*` 共 14 处不同名，见 NAM-40）；按特性组织的测试目录必须在首行写 `covers` | 文件 stem 比对（不是 `module_path`）+ 首行检查 | 文件名 43/44 成对（1 个有意例外）；模块名 14 处不同名（有意，不计违规）；`covers` 行缺 10 处（NAM-19） |
| D-11 | **跨 crate 同名词必须同义**：`search.rs`/`forms.rs`/`registry.rs`/`parse.rs`/`validation.rs`/`contracts.rs` 等在两个 crate 出现时，若语义不同，其中一个必须加限定 | 同名文件 × 模块文档首行的主语比对 | **无法判定（缺清单）**：t25 原写「抽样 12 组里 6 组」，未给 12 组清单；t27 亦无法复算。已确证的只有 NAM-08（`call_trace.rs`×2）、NAM-09（`face_manifest.rs`×2）、NAM-11（4 个保留词）、NAM-17（`faces.rs`/`face_view.rs`）→ 该数改为「**已确证 4 组**（其余待清单或本规则不落地）」 |

### 7.3 复核更正：三处方法学问题（t27 → t30 修订）

| 条目 | 原判 | 复核后 |
| --- | --- | --- |
| `NAM-40` | 「394/394 `module_path` 与文件 stem 一致、0 违规」 | **证伪**：14 处不一致（四族）；原「两处有意例外」其实是重导出别名、不进 `module_path`。已改写为 14 处口径（`mechanism_falsified`） |
| `NAM-07` / D-3 | 「39 处无载荷目录」 | 谓词收窄为递归口径 **34（发布面 28）**；旧 39（直接子项）与 67（文件 stem）标为不再使用 |
| `NAM-05` | 「16/17 同名，只有 `explain` 例外」 | **前提被证伪**：实测 12/17 同名、5 个例外；结论方向保留，严重度 MAJOR → **MINOR**，修法采 `mcp/build_evidence.rs` |

`NAM-06` 同样在复核后由 MAJOR 降为 MINOR（模块文档首行自述的是 “Write guards”，与内容相符），且**撤回**了原修法「并入 `mutations.rs`」，改为改名 `studio/studio/app/write_guard.rs`。两条降级都带 `severity_note`（见 §1.4 规则二）。

### 7.4 查了、干净（命名轴）

- `NAM-42`（宏生成的模块名）：`examples/control-button` 的四个面在编译产物里有 `__xirang_ra_<path>` 与短名两个模块，命名由路径推导、一致；本仓无法在离线审计里复核生成物（t27 只确认了机制），维持「干净」。
- 专项报告 §F 的 8 行「谓词 + 命令 + 输出」干净项（文件名镜像 43/44、模块名不镜像 14 处有意、裸动词 15、实现词根 8、别名 4 处 3 合规等）逐条可在 §G 复算。

### 7.5 D-3（无载荷目录收平）的落地范围与豁免

**D-3 现在带范围条件，不能写成「收平后为 0」了事。** 谓词是递归口径 **A1**（目录下递归 `.rs` 集合恰为 `{<dir>.rs}`）**∧ 路径不以 `examples/` 或 `studio/tests/` 开头**；三个口径：**仓库 34 / 发布面 28 / 豁免后可达 0**（「0」只在发布面内成立）。

- **豁免的 6 处**：`examples/control-button/src/control/object/{button,slider}`、`examples/control-button/src/control/registry_rule`，以及 Studio 夹具 `studio/tests/fixtures/node-editor/src/control/{registry_rule,object/node_editor/registry_rule,object/node_editor/object}`。
- **为什么必须豁免**：这里面装着**身份承载**的注册面文件。收平＝移动文件＝改 `file!()`＝改 `NodeId`，会让「按身份落盘」与「按路径字面引用」两类东西失效：用户 checkout 的 `.xirang/external-grafts/*/graft.plan`（本仓库当前无落盘记录，风险在模板被复制到用户侧之后）、`registry_rule_path` 字符串（`examples/control-button/src/control/control.rs:28`），以及逐字引用 `examples/control-button/src/control/object/button/button.rs` 的测试与 README（`mcp/apply_tests.rs:168`、`mcp/README.md:131`、`mcp/README.zh-CN.md:89`、`studio/studio/app/tests/edit.rs:20`、`studio/studio/app/tests/trace_ingest.rs:29`）。
- **机制要说准**：类型化 `static_graft_plan!`（`examples/control-button/src/lib.rs:48-54`，宏调用在 `:48`、两个 `cut(...)` 实参在 `:50`/`:52`）与面**一起编译**，取值会跟着新身份走——**它本身不会「失效」**；失效的是上一条列出的「按身份落盘」与「按路径字面引用」。
- **落地顺序**：D-3 由此从「可一次机械完成」改为「**先按前缀分流、只做发布面那 28 处**」，6 处豁免；`audit-structure-map.html` 的「命名与文件归属」视图也标注了这一排除。出处：`audit-naming-review.md` 的 D-3 / NAM-07 豁免表 / §C.1（t31）+ 修订记录 ⑩（t32）。

### 7.6 命名歧义与约定冲突（生态保留名 / 跨生态 / 同族易混 / 本仓一名多义）

命名歧义专项（`audit-naming-ambiguity.md`，t34；t39/t41 按复核修订）共 **20** 条 `AMB-` 条目（批次 `B8-命名歧义与约定冲突`），严重度 MAJOR 5、MINOR 15；复核状态 confirmed 13、partial 2、unverified 5。复核者：`audit-naming-ambiguity-verify.md`（t35）、`-verify2.md`（t40）、`-verify3.md`（t42）。

**四问判据**：**问一 生态保留名**（`core`/`build`/`run`/`registry`/`artifact` 等在 Rust 与 Cargo 生态里有确定含义）、**问二 跨生态理解**（读者按别的生态的默认含义理解）、**问三 同族易混**（同词族两个名字读者分不开，如 `mir` ↔ `mirror`）、**问四 本仓库一名多义**（同一个词在本仓指两个不同物）。任一是则进歧义轴。

**与 §7.1/§7.2 的分工与不重复计数**：§7.1-§7.2 判「现名**是不是**它扮演的角色」（角色谓词 D-1…D-11）；本节判「现名会不会被读者当成**别的东西**」（四问 + 歧义共识 R-1…R-4）。歧义报告正文标了 `= D-n` 的条目**不计入本节**，反之亦然；两节 id 前缀不同（`NAM-` / `AMB-`），id 集合与批次计数都由 `audit-findings.json` 的逐条字段生成。

#### 7.6.1 逐条：现状名 / 读者会先当成什么 / 冲突来源 / 建议 / 影响面

| id | 严重度 | 四问 | 现状名 | 读者会先当成什么 | 冲突来源 | 建议 | 影响面 | 复核 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `AMB-01` | MAJOR | 问一：生态保留名 | `core/`（+ lib `xirang`） | 读者把 `core/` 当成 no_std 标准库或「被 `use core::` 指向的东西」。判据/命令：`grep -n 'name = "xirang"' core/C …（全文见 `audit-findings.json`） | 生态保留名：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | lib 改名是**公开路径**（`use xirang::…`、docs.rs 链接、63 处 `core/…rs:NN` 文档锚点、` …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-02` | MAJOR | 问一：生态保留名 | `build_method/` | 读者把 `build_method/` 当成「构建脚本相关的东西」或「一个构建方法」；`grep -rn build.rs` 全仓 4 个文件 3 种角色（真构建脚本 2、CLI  …（全文见 `audit-findings.json`） | Cargo 惯用名：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:65``（`src/b …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-03` | MAJOR | 问二：跨生态理解 | `run_method/` | 全仓 5 个 `run` 系公开名（`build_method::run`、`cli::run`、`mcp::run`、`RuntimeCheckSpec::run`、`CallT …（全文见 `audit-findings.json`） | 跨生态阅读：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 50 处 `run_method/…rs:NN` 锚点；`docs/audit-2026-09-28/audit-publish-surfa …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-04` | MAJOR | 问三：同族易混 | `plugin-host/`（crate/目录）与方案模块名 …（全文见 `audit-findings.json`） | 读者把 `plugins/` 当成放插件本体的目录（VS Code `extensions/`、WordPress `plugins/` 都是这个意思）。判据/命令：`grep - …（全文见 `audit-findings.json`） | 同族易混：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 只影响**未执行的**合并方案（模块 `plugins` + feature `plugins`，见 `docs/audit-2026-09 …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-05` | MAJOR | 问一：生态保留名 | 公开类型 `Registry` | 读者/agent 把 `Registry`/`registry_core`/MCP 工具 `xirang.registry`（`mcp/tools.rs:114`）当成 …（全文见 `audit-findings.json`） | Cargo 生态词：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 公开类型（docs.rs 页面）与 MCP 工具名（agent 的调用契约）；`registry` 是 D-9 的保留词，本条的登记行与 D …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-06` | MINOR | 问一：生态保留名 | 顶层 `examples/` | 读者按 `cargo run --example <dir>` 找。判据/命令：`sed -n '2p' Cargo.toml`（members 含 `examples/contr …（全文见 `audit-findings.json`） | Cargo 自动发现目录：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 根 `Cargo.toml:2`、`tools/xirang-external-rehearsal`、13 处 `examples/`  …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-07` | MINOR | 问一：生态保留名 | `debug_method/` | 读者以为它与「debug 构建配置」有关或它是一个函数方法。`[机器]` 全仓**没有任何标识符含 `Method`**（`grep -rnE '\w*Method\w*' <8  …（全文见 `audit-findings.json`） | Cargo 惯用名：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 3 处 `debug_method/…` 锚点；合并后目录消失，只剩 feature `evidence`。 | 已复核 |
| `AMB-08` | MINOR | 问三：同族易混 | bin `xirang-dev` 与 feature ` …（全文见 `audit-findings.json`） | 读者以为 `xirang-dev` 是 studio 的开发版二进制。判据/命令：`ls studio/bin/`；`grep -n "required-feature …（全文见 `audit-findings.json`） | 同族易混：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | bin 名（README/CI/`docs/audit-2026-09-28/audit-publish-surface-merge-pla …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-09` | MINOR | 问二：跨生态理解 | 顶层 `picture/` | 读者按 `assets/`/`images/` 找图。判据/命令：`ls -R picture`；`grep -rn "picture/" --include=*.md --inc …（全文见 `audit-findings.json`） | 跨生态阅读：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 只触及根 README（不打包）与一份 docs 的注释；不动任何公开面。 | 已复核 |
| `AMB-10` | MINOR | 问一：生态保留名 | `docs/ROADMAP.md` 与 `docs/road …（全文见 `audit-findings.json`） | 读者/文件系统分不开。判据/命令：`ls docs | grep -i roadmap`（→ 三个名字）。 | 文件系统/平台约定；略超本轮六类命名空间，但同一失效模式：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | `docs/` 内部引用与 `doc_anchors` 门禁；与 `audit-naming-review.md` **无重叠**（那篇 0 …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-11` | MINOR | 问四：本仓库一名多义 | 内核模块 `syntax`（+ feature `synta …（全文见 `audit-findings.json`） | 读者以为它解析 `.rs` 的文法。判据/命令：`head -1 core/src/registry_core/syntax/syntax.rs`（→ "Parser shared …（全文见 `audit-findings.json`） | 本仓一名多义：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 内核公开路径（shims 棘轮）；3 个清单的 feature；`docs/audit-2026-09-28/audit-publish-s …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-12` | MINOR | 问一：生态保留名 | 内核模块 `json` | 读者按格式名找一个格式实现。判定/命令：`head -1 core/src/registry_core/json/json.rs`；`grep -rn "json::" core/ …（全文见 `audit-findings.json`） | 生态名：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 内核公开路径（4 个公开函数被 diagnostic/mir/渲染器使用）。 | unverified |
| `AMB-13` | MINOR | 问一：生态保留名 | 内核模块 `release` | 读者以为与 release 构建配置有关。判据/命令：`head -1 core/src/registry_core/release/release.rs`；`ls core/sr …（全文见 `audit-findings.json`） | Cargo 惯用名：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 内核公开路径 + `StaticPlan` 被生成代码引用（字符串常量在 `lexicon`）。 | unverified |
| `AMB-14` | MINOR | 问二：跨生态理解 | 内核模块 `requirements` | 读者按 pip/依赖清单理解。判据/命令：`head -1 core/src/registry_core/requirements/requirements.rs`；`grep - …（全文见 `audit-findings.json`） | 跨生态阅读：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 内核公开路径；`impact`/`converge` 两个 MCP 工具消费其结论。 | unverified |
| `AMB-15` | MINOR | 问四：本仓库一名多义 | 内核模块 `source`（一名四义之一） | Rust/Cargo 里 `source` 同时是「源码」与「依赖来源」（`[source]` 配置），两个含义都在生态里。判据/命令：`head -1 core/src/regi …（全文见 `audit-findings.json`） | 本仓一名多义：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 模块是内核公开路径；`PluginSource` 是插件公开面（改名属公开 API 变更）。 | 已复核 |
| `AMB-16` | MINOR | 问四：本仓库一名多义 | 类型层 `artifact` 一名四义 | 判据/命令：`grep -rn "artifact" mcp/mir.rs run_method/runtime/trace/artifact/*.rs | hea …（全文见 `audit-findings.json`） | ：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | `PluginArtifact`/`VerifiedPluginArtifact` 是插件公开面；MIR/trace 工件是 CLI/MCP …（全文见 `audit-findings.json`） | unverified |
| `AMB-17` | MINOR | 问四：本仓库一名多义 | `index` 一名三义 | JS 的 `index.{js,ts}` 与 Cargo 的 registry index 都在读者预期里。判据/命令：`grep -n "pub struct RegistryI …（全文见 `audit-findings.json`） | ：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | `RegistryIndex` 是内核公开类型（改名需 shim）；`mcp/index.rs` 为内部模块。 | 已复核 |
| `AMB-18` | MINOR | 问四：本仓库一名多义 | `host` 一名两义 | 判据/命令：`head -1 plugin-host/src/lib.rs`；`grep -rn "host project" README.md | head -2`。 | 本仓一名多义：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | `host!()`/`application!()` 是宿主公开宏（不改名，只做文档义务）。 | unverified |
| `AMB-19` | MINOR | 问三：同族易混 | feature 名与模块/包名重名的成批现象 | 判据/命令：`grep -rn '^\[features\]' -A 20 */Cargo.toml | head -40`；`sed -n '89,93p;122,126p' d …（全文见 `audit-findings.json`） | 同族易混：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | 宿主清单（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:148-15 …（全文见 `audit-findings.json`） | 已复核 |
| `AMB-20` | MINOR | 问四：本仓库一名多义 | 函数/类型层的同一轴：13 个公开名的**整名**就是别处的 …（全文见 `audit-findings.json`） | 判据/命令见 §5 的脚本（输出 13 行）；**影响的 831 个名字里 212 个按分词命中词表但都带了限定词**（`GraftPlan`/`FaceView`/`Plugin …（全文见 `audit-findings.json`） | 维护者原话「同理函数名也是」：**否（4 个该改、7 个登记即可）**。最值：`cli::main` | 建议保留 `plugin-host`（crate/目录名）** | `cli::main`、`RuntimeCheckSpec::run`、`Registry::registry`、`Registry::in …（全文见 `audit-findings.json`） | 已复核 |

每条的权威定位见 §3.2 映射表；完整「行为清单 / 角色一句话 / 影响面」在 `audit-findings.json` 的 `amb_role` / `amb_reader` / `amb_conflict` / `amb_impact` 字段。

### 7.7 命名歧义共识（可落 `conventions` 门禁）

- 违规计数不重复**（正文标 `= D-n` 的条目不计入对方）。
- 谓词**：命名空间单位的名字**完整段**等于黑名单任一词即违规；三条豁免：
- 规则**：命名空间单位（目录 / 包 / lib / 模块 / feature / bin）的名字必须是**名词短语角色名**。

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
| 指标 | 谓词对象 | 命中（名字 / 声明处） | 扣除 | **现值（净）** |
| **R-4a** | 命名空间名 | 14 / 14 | `support`、`misc` **= `audit-naming-review.md` D-1 的 NAM-01/NAM-02**（同一对象，文件层已计，此处只算一次） | **12 / 12**（目录 3 + 包 3 + lib 3 + 模块 3：`data`、`face_helpers`、`test_support`） |
| **R-4b** | 单 token 名字的**声明处**（豁免**逐站点**判，不按名字整体放过） | 16 / 27（严格动词表，**含** `build`） | 父路径限定对只豁免 `build` 的**那 1 个站点**（`diagnostic::build`）；"名字数"= 有 ≥1 个违规站点的名字数 | **15 名 / 26 处** |
| **R-4c** | 名字对 | 7 对 / 13 名 | 登记豁免 1 对（`app`↔`application`，见 `ALLOWED_PAIRS`）+ 单复数族 7 对（`contract`↔`contracts` 等） | **6 对 / 12 名** |

### 7.8 合并方案 §2 的七个模块名：三段式判定

| 方案模块名 | 行为清单（搬进去的是什么） | 角色一句话 | 命名判定 | 建议 |
| --- | --- | --- | --- | --- |
| `build` | = AMB-02 的六条（发现/作用域/校验/渲染/发布/入口） | 「构建期把宿主源码变成计划与清单的那一半」 | **否**：`build` 是无宾语动词（`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:65`` 的 `sr …（全文见 `audit-findings.json`） | **`build_time`** |
| `run` | = AMB-03 的五条（运行期状态/trace/创作执行/宏/shim） | 「运行期承载注册状态、记录调用并执行创作的那一半」 | **否**：`run` 是无宾语动词；它正是 E0428 的成因（`:167`），并逼出「只 glob `run`」的规则（`:185`） | **`runtime`**（顺带消掉 E0428） |
| `plugins` | = §2.1 的五条（验签/准入/隔离执行/懒激活/部署） | 「把锁里写下的插件变成正在运行的插件的那一半（**插件宿主**）」 | **否**：`plugins/` 在生态里放插件**本体**，而本仓 `.xirang/plugins/`（`plugin-host/src/admission.rs:38`） …（全文见 `audit-findings.json`） | **`plugin_host`** |
| `call_evidence` | = AMB-07 的三条（收集器/宏臂/聚合） | 「把编译期登记与运行期观测汇成证据的那一半」 | **是**（限定词 + 与 `CallEvidence` 同词；t31 已裁定） | 保留；feature 仍叫 `evidence`，README 加一行对照 |
| `studio` | = Studio TUI（`studio/{lib,main}.rs`，创作/检视面） | 「面向人的 Ratatui 创作与检视面」 | **是**（产品名，本仓三处一致） | 保留 |
| `mcp` | = MCP 桥（`mcp/protocol.rs:71` 起，17 个工具） | 「面向 AI 客户端的 stdio 桥」 | **是**（协议缩写） | 保留 |
| `cli` | = argv 分发 + cargo 子进程（`cli/lib.rs:124`、`commands/`） | 「面向命令行的分发面」 | **是（轻度）**：约定的缩写，无更优词 | 保留 + 对照表 |

模块名以**修订后的**合并方案为准（`audit-publish-surface-merge-plan.md` 的 §1.1/§1.2、模块挂载形态、逐 bin 安装与 feature 对照表）：`build` → **`build_time`**、`run` → **`runtime`**、`plugins` → **`plugin_host`**；`call_evidence` / `studio` / `mcp` / `cli` 保留（`call_evidence` 由 t31 裁定，特性名仍是 `evidence`）。

### 7.9 「不是缺陷」清单（同轴，判过但**不进队列**）

| 名字 | 角色一句话 | 为什么不判歧义 |
| --- | --- | --- |
| `studio`、`mcp`、`cli` | 产品名/协议缩写/约定缩写 | 三处（包、bin、README）一致；生态里没有更强的既有含义；不看代码也知道去哪找 |
| `macro`（目录） | 「proc-macro 前端 crate」 | 它**就是** proc-macro（`[lib] proc-macro = true`）；关键字形对目录名无影响 |
| `conventions` | 「仓库规则的可执行门禁」 | 该 crate 的 README 首段与全部 12 个模块都只说这件事；不在任何生态保留名里 |
| `tools/`、`docs/`、`target/` | 「脚本」「文档」「Cargo 产物」 | 三个通用惯例；`target/` 是 Cargo 固定产物目录（构建产物，R-1 豁免） |
| 内核 `mir` | 「读 rustc MIR 转储并与真实调用合并的证据层」 | **它确实是** rustc 的 Mid-level IR（`mir/mir.rs:1` "Static MIR candidates…"，`MirGraph::from_mir_text` 读 `-Zunpretty=mir`）→ 读者默认含义**正确**；只需词表一行（JSONL 是本仓包装形态 …（全文见 `audit-findings.json`） |
| `face`、`graft`、`trace`、`tree`、`identity`、`declaration`、`diagnostic`、`lexicon`、`authoring` | 各自领域词 | 本仓领域词，README/设计文档给了定义；生态里无竞争含义（`trace` 与 `tracing` crate 只是邻近） |
| `mod tests`（76 处内联）与 `*_tests` | 单元测试模块 | Rust 惯用写法；若硬禁一次产生 76 条噪声——「一次把什么都点成缺陷」的反面教材 |
| 私有模块 `mod build;`（`core/src/registry_core/diagnostic/diagnostic.rs:14`） | 「构建诊断」 | 命中 `build`，但**父路径已消歧**（`conventions/src/size.rs:22` 也这么引用） |
| `cargo-xirang` | Cargo 子命令 | `cargo-<x>` 是 Cargo 的**唯一合法**命名方式（不能改） |
| `wasm`、`process-tools`、`node-graph`、`prototype-fixtures` | 后端/控件/夹具开关 | 描述性词，不指错地方（`wasm` 只是与引擎名邻近） |

### 7.10 记账：V42-01（`app`/`application` 这一对）

复核人 t42 记了一条 low（V42-01）：歧义报告 §8 的改法表与落地顺序仍把**已登记豁免**的 `app`/`application` 写成「至少一个加限定词 / 各改一侧」。**本报告按复核后的验收词表述**：该对**已登记豁免、不需要改名**（`ALLOWED_PAIRS = {("app","application")}`：两侧都是好名字、分属不同层级与 crate，不是同一概念的两个拼法）；其余 6 对仍按「改名落在更短/更泛的一侧」处理。源报告的这两处文字（`:749`、`:759`）**仍在**，本报告不改源文件、只按复核后验收词表述；执行者以复核后验收词为准。

## 8. crate 顶层边界（必备节 4；以 t14 修正后版本为准）

**结论：本轮 crate 数量与发布面都不动——12 成员 / 9 发布 crate 维持现状。**

这条结论经过了两次转向，报告必须写清过程，否则会被误读：

1. `audit-structure-base.md` §6 原本建议把 `debug_method`（444 行、消费者仅 studio+mcp）并入 `run_method`，理由之一是 `run_method` 的宏会发射 `::xirang_debug_method::submit!` 形成「幻影依赖」；
2. `audit-logic-hunt.md` 实测**证伪**了这个前提：那条路径只在 `collector: debug` 下发出、且带 `#[cfg(debug_assertions)]`，出厂路径（两个示例）走 `development` 空 arm；只有「显式写 `collector: debug` 且只依赖 `run_method` 的第三方宿主」在 debug 构建下才会撞 E0433；
3. `audit-boundary-refactor-plan.md`（t14）据此**撤销**合并推荐，并在附录 B 逐条列出被证伪的板子与剩余理由。

**请以 t14 的修正后版本为准，不要引用 structure-base 里已被证伪的幻影依赖论证。**

### 8.1 十二个成员各自的独立性理由（t14 §2.1 的口径）

| crate | 目录 | 体量（前） | 本方案 | 变化 |
| --- | --- | --- | --- | --- |
| `xirang-core` | `core/` | 93 文件 / 21,787 行 | 不变 | 无（R-25 已否决 tree/ 归并） |
| `xirang-macro` | `macro/` | 3 / 917 | 不变 | 无（proc-macro 物理不可合） |
| `xirang-build-method` | `build_method/` | 48 / 10,110 | 3 处重命名 + 两处搬移 | R-13/R-15 |
| `xirang-run-method` | `run_method/` | 61 / 9,584 | 不变（仅文档导览） | R-14 |
| `xirang-debug-method` | `debug_method/` | 5 / 481 | **不变**（合并案本轮否决） | — |
| `xirang-plugin-host` | `plugin-host/` | 15 / 3,888 | 测试页 1 → 2 | R-19 |
| `xirang-studio` | `studio/` | 79 / 12,532 | 文件级搬移 8 项（全在私有模块内） | R-05…R-12 |
| `xirang-mcp` | `mcp/` | 39 / 8,682 | 新增 1–2 个测试页 + 三组收口 | R-16/R-17 |
| `xirang-cli` | `cli/` | 14 / 2,802 | 输出出口统一 | R-18 |
| `xirang-conventions` | `conventions/` | 20 / 6,399 | 新增 1 个门禁模块 | R-03 |
| `xirang-example-control-button` | `examples/control-button/` | 13 / 1,635 | **不变**（身份承载） | — |
| `xirang-example-control-button-graft` | `examples/control-button-graft/` | 4 / 134 | **不变**（身份承载） | — |

（上表体量是 t14 的口径：studio 的 79/12,532 把非成员夹具包算了进去；清点口径的 72/12,308 是「工作区成员」口径。两者自洽，差别只在夹具包归属，见 §1.1。）

### 8.2 真正贵的是治理成本，不是 crate 数

t14 量化了「一次版本线推进」的代价：**动 13 个文件、同步 20 处内部版本要求**，因为 `tools/xirang-publish` 强制 `req == workspace_version`。因此本轮的包管理重构把`R-01 [workspace.dependencies] 收敛` 排在批 1（见 §9.1），它一次消除 19 处内部 req 与 12 处外部重复，且**工具侧零改动**。

### 8.3 身份风险（为什么本方案的移动是安全的）

`audit-boundary-refactor-plan.md` §4 的结论：承载身份的生产文件只有 **6 个**（两个示例宿主的面文件，被类型化 `static_graft_plan!` 以 `NODE_ID` 引用），它们列进「不动」；十个发布 crate 的 `src/` 里生产注册面声明为 **0**（实测四类命中）。因此本方案的全部移动都是身份中立的，**不需要记录迁移**。

## 9. 下一轮可执行清单

### 9.1 结构重构分批（来自 `audit-boundary-refactor-plan.md` §4，id 前缀 `STR-`）

| 批 | 项 | 前置 | 身份迁移 | 与版本推进同批 |
| --- | --- | --- | --- | --- |
| **批 1：包管理与治理** | R-01, R-02, R-03, R-04, R-26, R-27, R-28 | 无（可立即开始） | 不需要 | 否 |
| **批 2：studio 文件级** | R-05…R-12 | 批 1 的 R-01 先落 | 不需要 | 是 |
| **批 3：执行面与桥** | R-13…R-19 | 无（与批 2 并行） | 不需要 | 是 |
| **批 4：唯一来源收口** | R-20…R-24 | **必须与对应 lane 发现的逻辑修复同批** | 不需要 | 是 |
| **批 5：条件项** | R-25（已否决）、附录 B（debug_method 合并） | 维护者明确拍板 | 需记录迁移 | 是，且不可回滚 |

28 项明细（id / 标题 / 批次 / 出处）：

| id | 项 | 批次 |
| --- | --- | --- |
| `STR-R-01` | `[workspace.dependencies]`（清单） | 批 1：包管理与治理 |
| `STR-R-02` | 版本线政策与文档对齐（二选一） | 批 1：包管理与治理 |
| `STR-R-03` | crate 边界表（文档 + 门禁） | 批 1：包管理与治理 |
| `STR-R-04` | 发布层级顺序的唯一来源（文档） | 批 1：包管理与治理 |
| `STR-R-05` | `studio/studio/app/writers.rs` → `write_guard.rs`（改名，不并入） | 批 2：studio 文件级 |
| `STR-R-06` | `studio/studio/app/support.rs` 拆两页 | 批 2：studio 文件级 |
| `STR-R-07` | `studio/studio/app/state/misc.rs` 归位 | 批 2：studio 文件级 |
| `STR-R-08` | `studio/studio/app/navigation.rs` 只留导航 | 批 2：studio 文件级 |
| `STR-R-09` | overlay add/edit 合一 | 批 2：studio 文件级 |
| `STR-R-10` | `ui/forms/buttons.rs` | 批 2：studio 文件级 |
| `STR-R-11` | 测试页整理 | 批 2：studio 文件级 |
| `STR-R-12` | `keyboard_overlay` 与 `overlay/` 同名化（推荐方案 ②） | 批 2：studio 文件级 |
| `STR-R-13` | `build_method/graft_view/matching.rs` 的 FS 读取外移 | 批 3：执行面与桥 |
| `STR-R-14` | 阅读顺序 | 批 3：执行面与桥 |
| `STR-R-15` | 构建期 shim 页命名 | 批 3：执行面与桥 |
| `STR-R-16` | mcp 测试页 | 批 3：执行面与桥 |
| `STR-R-17` | mcp 三组收口 | 批 3：执行面与桥 |
| `STR-R-18` | cli 输出出口 | 批 3：执行面与桥 |
| `STR-R-19` | plugin-host 测试拆分 | 批 3：执行面与桥 |
| `STR-R-20` | 唯一来源收口 | 批 4：唯一来源收口 |
| `STR-R-21` | 唯一来源收口 | 批 4：唯一来源收口 |
| `STR-R-22` | 唯一来源收口 | 批 4：唯一来源收口 |
| `STR-R-23` | 唯一来源收口 | 批 4：唯一来源收口 |
| `STR-R-24` | 唯一来源收口 | 批 4：唯一来源收口 |
| `STR-R-25` | （已否决）`core` 的 tree/ 归并——仅在维护者坚持时做 | 批 5：条件项（已否决，坚持时才做） |
| `STR-R-26` | 门禁与文档 | 批 1：包管理与治理 |
| `STR-R-27` | 门禁与文档 | 批 1：包管理与治理 |
| `STR-R-28` | 门禁与文档 | 批 1：包管理与治理 |

### 9.2 修复批次（**从 `audit-findings.json` 的 `batch` 字段生成**，与 §2 同源）

| 批次 | 条数 | 其中 CRITICAL/MAJOR | 代表条目（前若干，全量见 JSON 的 `batch` 字段） |
| --- | ---: | ---: | --- |
| **B1-立即：安全与数据完整性** | 3 | 3 | `LGC-LG-01`、`LGC-LG-02`、`LGC-LG-03` |
| **B2-门禁与工具** | 23 | 3 | `GTE-G-16`、`GTE-G-21`、`GTE-N-1`、`GTE-G-03`、`GTE-G-04`、`GTE-G-05` |
| **B3-桥与宿主** | 14 | 3 | `BRG-BR-1`、`BRG-BR-2`、`BRG-BR-C2`、`BRG-BR-11`、`BRG-BR-12`、`BRG-BR-15` |
| **B4-MAJOR 逻辑与实现缺陷** | 50 | 50 | `KRN-C-01`、`KRN-C-02`、`KRN-C-03`、`LGC-LG-04`、`LGC-LG-05`、`LGC-LG-06` |
| **B5-注释与自述** | 21 | 0 | `KRN-C-05`、`KRN-C-06`、`KRN-C-07`、`KRN-C-08`、`KRN-C-10`、`KRN-K-10` |
| **B6-MINOR 一致性与清扫** | 51 | 0 | `KRN-C-04`、`KRN-C-09`、`KRN-C-11`、`KRN-K-14`、`KRN-K-15`、`KRN-K-16` |
| **B7-命名抽象与可读性** | 30 | 5 | `NAM-01`、`NAM-03`、`NAM-04`、`NAM-30`、`NAM-31`、`NAM-02` |
| **B8-命名歧义与约定冲突** | 20 | 5 | `AMB-01`、`AMB-02`、`AMB-03`、`AMB-04`、`AMB-05`、`AMB-06` |

每批的完整 id 清单 = `audit-findings.json` 里 `batch` 等于该批的所有条目；本表与 §2 的排序、§0 的批次计数三者同源（脚本断言）。

### 9.3 每条修完后应当由哪道门禁钉住（建议，不新增门禁）

| 缺陷类型 | 现有的钉子 |
| --- | --- |
| 缓存/身份跨包污染（LG-01） | `run_method` 已有 A/B 两包同进程的测试形态（t8 用了它），把它固化成回归用例 |
| admission 双列表（LG-02） | `core` 的 authoring 往返测试（读→写→读） |
| 锁 schema（LG-04） | `PluginCatalog::parse` 的错误路径测试 + `plugin-host` 的 admission 门禁 |
| 嵌套 abort（LG-05） | `core/tests/nesting_budget.rs` 的同款预算测试，扩到 run_method 的 `syn` 调用点 |
| 门禁判定（LG-24/25/53） | `conventions` 自己的测试夹具（t13 已给出可复用的夹具形状） |
| 预览副本（LG-19/20/23） | `mcp` 的端到端夹具（t12 用过） |

## 10. 查了、干净 + 覆盖与方法（必备节 5）

### 10.1 各路覆盖与手段（谁读了多少、用什么探针、哪些只读码）

| 路 | 覆盖 | 手段 | 只读码的部分 |
| --- | --- | --- | --- |
| A 内核（t2） | `core/` 全部 **93 文件 / 21,787 行**逐文件读；五条深读块 ≈10.6k 行 | 逐行读 + 调用方对照；内联测试页约 2.1k 行只读到「判读用例名」的深度（报告自陈） | 部分测试页 |
| B 执行面（t3） | `build_method/src` 46/9,928、`run_method/src` 52/8,277、`macro/src` 3/917、`debug_method/src` 4/444、测试/示例合计 1,526 行 | 逐文件读 + `cargo test -p` 实跑 + 自建 `syn` 探针 | S16 的 panic 面为清单式抽样 |
| C Studio（t4/t11） | `studio/src` 71 文件 / 12,209 行 + `studio/tests/` | 全程 `read` 逐文件 + `codegraph_explore` 交叉核对内核规则 + `cargo test -p xirang-studio --offline --all-features` | 测试用例名级判读 |
| D 桥与宿主（t5/t12） | `mcp/src` 39/8,682、`cli/src` 14/2,802、`plugin-host` 11/3,888 + `fault_matrix.rs` 968 | 逐文件读 + **端到端驱动 MCP 二进制**（原始字节帧）+ 只读实测计数 | `.git` 复制量的 36/46 MB 为一次性实测 |
| E 门禁与工具（t6/t13） | `conventions` 12 道门禁逐个读判定代码、6 个 tools 脚本、2 个 workflow、2 个 example、双语文档 | 夹具树 + **直接调用门禁 crate 的公开 API**（`/tmp/gate-review/probe.rs`）+ 保真性控制（同探针跑真实工作区） | G-09/G-10/G-12 的变体标注为 `[读码]` |
| F 逻辑对抗（t8/t15/t16） | 按危险流扫全树 + 10 个自建 bin + 端到端驱动 | 自建探针 13 组（冷/热缓存、RSS 对照、栈溢出、锁形态矩阵、CLI 退出码矩阵…） | LG-03/06/07/08/09/10/27/29/30 等本轮为读码或复用复核结论，已逐条标注 |
| 结构（t7/t14） | 394 `.rs` / 78,951 行的目录、模块挂载图、依赖图、清单与特性、体量对账 | 机械清点 + 与 `audit-inventory` 逐项交叉核对（零冲突） | — |

### 10.2 各路「查了、干净」的要点（原报告的完整清单见出处）

| 路 | 干净面（要点） | 出处 |
| --- | --- | --- |
| A 内核 | `core/src` 无 I/O、无 `std::env`、无 `process::Command`、无 `SystemTime/Instant`（grep 全空）；`identity/node_id.rs` 与 `identity/sha256.rs` 有 0..=200 全长度差分测试与 `sha2` 预言机对照；`core/src/test/registry_rule/` 是空目录（无内容可读） | `audit-lane-kernel.md:368-378` |
| B 执行面 | shim 重导出仍在、历史公开路径未断；全仓执行面只有一处 `include!`（`run_method/macros/entry.rs:55`）；构建期 IO 失败的两条调用方分工正确；不可读的注册面文件不会被静默丢掉；除 S1/S4/S8 点名的三处外，执行面没有重做内核的纯逻辑；trace 转义/反转义对称；graft 切口匹配规则只有一份 | `audit-lane-surfaces.md:395-409` |
| C Studio | 无 TODO/FIXME/XXX/HACK/`todo!`/`unimplemented!`（grep 全空）；没有注释掉的死代码；双语成色抽查成立；`studio/studio/app/graft.rs:1-21` 是注释质量上限 | `audit-lane-studio.md:311-316` |
| D 桥与宿主 | MCP 读侧路径约束（`../../etc/passwd` 等实测被拒）；`--help`/`USAGE` 与子命令集合的对照已重做；工具脚本 `sh -n` 全过 | `audit-lane-bridges.md:547` 起 |
| E 门禁与工具 | 六条 MAJOR 之外的门禁在真实工作区全部复现出厂结论（`lint_required` 11 条、`missing_roots` 空、`naming` 空、`release_version` 空、`size` 恰为钉住的 1 个文件、`workflow_findings` 空）；`tools/xirang-publish --check-table` exit 0（9 crates） | `audit-verify-gates.md:13` |
| F 逻辑 | 3 CRITICAL / 29 MAJOR 无一被推翻；32 条 CRITICAL/MAJOR 中 28 条被第一手证实、4 条部分证实、0 条证伪；MCP 读侧路径约束、graft 计数流等 9 条流走通且干净 | `audit-verify-logic.md` 汇总 |
| 结构 | 0 dangling / 0 duplicated / 0 `mod.rs` / 0 孤儿模块；`include!` 恰 1 处；crate 依赖图无环；与机械清点零冲突 | `audit-structure-base.md` §2.3 |

### 10.3 本报告自己的方法与自检

- **输入**：13 份报告（5 片区 + 6 复核 + 总账 + 对抗扫荡 + 结构基线 + 结构方案 + 机械清点），共解析出 194 个发现块，去重后 212 行。
- **没有重跑任何一路的探针**；本报告的复核状态**转引**各复核报告（`verifier` 字段写明是谁），我自己只做了三件机械事：解析、去重、锚点自检。
- **锚点自检（扫描范围与排除项见 §1.5）**：`audit-findings.json` 的 212 条各含 `file`/`line`，逐条断言「文件存在 + 行号 ≤ 总行数」；正文里每一个 `path:NNN`（含区间两端）另跑一遍同样的断言，排除 `meta.unresolved_quoted_anchors` 与 `/tmp`、`target/`。自检命令与结果见交付消息。
- **计数自检（t19 加入）**：`counts` 由脚本在**全部加工之后**从逐条字段重算（不是手抄）；随后一个独立检查脚本（`/tmp/xirang-audit-logs/check_report.py`）再从逐条字段重算一遍，并逐句断言正文里的每个统计数字与之相等：`212` 条、`3/66/143`、`confirmed 164、partial 31、unverified 17`、`none 17、read 18、reviewed 136、runtime 41`、批次 `B1-立即：安全与数据完整性 3、B2-门禁与工具 23、B3-桥与宿主 14、B4-MAJOR 逻辑与实现缺陷 50、B5-注释与自述 21、B6-MINOR 一致性与清扫 51、B7-命名抽象与可读性 30、B8-命名歧义与约定冲突 20`——0 处不符。脚本还带一条**严重度不变量**：逐条断言 `severity ≥ max(来源片区报告, 总账, 独立复核)`，除非该条带 `severity_note`；违规则打印 id、三处来源值与 `file:line` 并退出非零（已用故意违规的 findings 副本实测会失败）。
- **计数自检（原有）**：§0/§1.4 的计数与 `audit-findings.json` 的 `counts` 同源；§2.1+§2.2+§2.3 的条目数（3+66+143）等于 212。
- **引文里的锚点**：`evidence`/`fix_hint` 是原报告的引用。所有**可解析**的引文锚点已机械改写为工作区相对路径；剩下 77 个 token 保留原样，它们是原报告的最小反例路径、夹具路径或方案里**尚未存在**的新增文件名（如 `src/alpha/alpha.rs`、`myrepo/src/foo/foo.rs`、`generated_lib.rs`、`geometry.rs`），在真实工作树里本就不存在——完整清单在 `audit-findings.json` 的 `meta.unresolved_quoted_anchors`，复核脚本请把这份清单排除。**每条 finding 的权威定位是其 `file`/`line` 字段**，已逐条自检（0 violation）。
- **门禁基线（本报告落盘后重跑，用来证明报告文件本身不会把门禁弄红）**：`cargo test --workspace --offline` → **exit 0**（52 个套件 / 740 passed / 0 failed / 25 ignored，与 t1 的门禁快照同数）；本轮 17 份报告 + 3 份产物全部以 `audit-` 前缀落在 `docs/audit-2026-09-28/`，符合 `conventions/src/doc_blocks.rs:69` 的 `RECORD_PREFIXES` 豁免规则。日志：`/tmp/xirang-audit-logs/final_test.log`。
- **没做的事（如实标注）**：① 没有对 17 条 `unverified` 条目做独立复核（它们是 MINOR 或复核新增，已在 §2.3/§11 标出）；② 没有评估修复的工作量（那是下一轮的事）；③ 没有为四条 DEC 决策给推荐排序。

## 11. 附录

### 11.1 总账 rollup（被拆开而不是合并的组）

- **`LGC-LG-37`**（MINOR）：成员 `KRN-K-10`、`KRN-K-17`、`SUR-S11`。  三条是「文本启发式解析」的同一形状（内核两处 + 执行面一处），但修复点在不同文件的解析入口，合并成一行会让行动项不可执行，故保留三行并在每行标注 ledger_rollup。
- **`LGC-LG-54`**（MINOR）：成员 `GTE-G-03`、`GTE-G-04`、`GTE-G-05`、`GTE-G-07`、`GTE-G-08`、`GTE-G-17`、`GTE-G-18`、`GTE-G-19`、`GTE-G-20`、`GTE-G-22`、`GTE-G-24`、`GTE-G-25`、`GTE-G-26`、`GTE-G-27`。  总账把 14 条门禁假阴性/假阳性/文档漂移压成一行；它们各自的文件、判据与修法都不同，压成一行会让「门禁的覆盖面与自述不符」这一结论失去可执行性，故保留 14 行并标注 ledger_rollup。
- **`LGC-LG-51+52`**（MINOR）：成员 `LGC-LH-10`。  总账把 t8 的 LH-10 组拆成两条（a 坏帧错误上下文、b 存在性 oracle），拆得比原报告更细，予以采纳；LH-10 作为组不再单独成行。

### 11.2 未复核条目（17 条）

| id | 严重度 | 位置 | 标题 |
| --- | --- | --- | --- |
| `AMB-12` | MINOR | `core/src/registry_core/json/json.rs:44` | 内核模块 `json` |
| `AMB-13` | MINOR | `core/src/registry_core/release/release.rs:9` | 内核模块 `release` |
| `AMB-14` | MINOR | `core/src/registry_core/requirements/requirements.rs:17` | 内核模块 `requirements` |
| `AMB-16` | MINOR | `mcp/mir.rs:20` | 类型层 `artifact` 一名四义 |
| `AMB-18` | MINOR | `plugin-host/src/lib.rs:1` | `host` 一名两义 |
| `BRG-BR-C7` | MINOR | `mcp/apply.rs:94` | `apply()` 靠注释说明一组按约定成立的不变量 |
| `KRN-C-09` | MINOR | `core/src/registry_core/authoring/field_presentation.rs:248` | 字段帮助文本讲的是另一个机制（`runtime_checks`） |
| `KRN-K-21` | MINOR | `core/src/registry_core/syntax/fields.rs:104` | D 大逻辑（同一“trait 标签”概念有两份派生实现，泛型路径下结果不同） |
| `LGC-LG-51` | MINOR | `mcp/mir.rs:217` | `xirang.mir` 根外路径的两种回复构成宿主文件**存在性 oracle** |
| `LGC-LG-52` | MINOR | `plugin-host/src/process.rs:392` | 插件坏帧只剩无上下文的 `Io("failed to fill whole buffer")` |
| `LGC-LH-09` | MINOR | `mcp/tools.rs:139` | 自我描述与行为不一致（本项目的老毛病，两处不一致都算缺陷） |
| `LGC-LH-11` | MINOR | `run_method/macros/face_registration.rs:160` | 宏里的 `::xirang_debug_method::submit!`：opt-in 且 `cfg(debug_assertions)` 门控，不是「无条 …（全文见 `audit-findings.json`） |
| `NAM-20` | MINOR | `mcp/index.rs:1` | A⑥ 目录名是否表达层级 · 平铺目录让 A④ 的同名问题加倍 |
| `NAM-21` | MINOR | `build_method/discovery.rs:1` | A③ 名与内容不符（误导 → MAJOR） · `build_method/src` 的"发现"族命名收敛 |
| `STU-C-09` | MINOR | `studio/studio/app/graft.rs:129` | 注释密度/信噪比 · `studio/studio/app/graft.rs:129-158`、`studio/studio/ui/graph/ …（全文见 `audit-findings.json`） |
| `STU-S-20` | MINOR | `studio/studio/app/overlay/add.rs:12` | 碎片化/重复状态机 · `studio/studio/app/overlay/add.rs:12-36` vs `studio/studio/a …（全文见 `audit-findings.json`） |
| `STU-S-21` | MINOR | `studio/studio/ui/forms/face.rs:105` | 重复 · `studio/studio/ui/forms/face.rs:105-128`、`studio/studio/ui/forms/pr …（全文见 `audit-findings.json`） |

清单里没有任何 CRITICAL 或 MAJOR：0 条 MAJOR 未复核。这些标签是**保守**的：它们的证据多来自作者自己的探针（实测），本轮没有第二个人用自己的装置复现，因此按纪律不写成「已确认」。

### 11.3 产物与复算

- 本文件：`docs/audit-2026-09-28/audit-report.md`
- 结构化明细：`docs/audit-2026-09-28/audit-findings.json`（字段 `id/severity/category/file/line/title/evidence/verify_status/verifier/fix_hint/batch` + `old_ids`/`ledger_id`/`source_report`/`source_line`/`evidence_kind`）
- 交互对照图：`docs/audit-2026-09-28/audit-structure-map.html`（数据只来自 `audit-boundary-refactor-plan.md` 与 `audit-inventory.{json,md}`，无虚构数字）

## 12. 修复轮 delta（产物冻结 17:12 之后）

**本节是账，不是新结论。** 本报告主体（§0–§11）描述的是一棵**冻结在 17:12 的树**：那棵树上的计数、行号与结论都不改。此后（17:43–17:53 起）有修复**就地**落到工作树里，若不记账，读者会拿冻结时的行数与今天的树对照。本节只列变化本身，不改任何既有判定。

- 记账时点：产物冻结 17:12 → 本次记账（2026-09-28，t43/t44 的 B1 修复 + t52 的同族残余修复 + t54 的拆文件收口）
- 落地文件：**8** 个，逐文件净差合计 **+1007** 行（表里的 Δ 是「现行行数 − 冻结行数」的**净差**：拆/并文件时增删会在同一格里相抵，例如 `mutations.rs` 在 t44 增了 228 行、t54 又拆出 196 行测试，净 +32）
- 受影响条目：**7** 条（`fixed` 2、`verified` 5）
- 修复轮**新增**条目：**1** 条（`FIXR-01`，**不计入**主清单的 212 条）

### 12.1 修复轮落地的文件

| 文件 | 冻结行数 | 现行行数 | Δ | 落地内容 |
| --- | --- | --- | --- | --- |
| `build_method/identity_cache.rs` | 101 | 306 | **+205** | t43/t44：B1-1（LG-01）身份缓存按 namespace 分桶 + 钉子 |
| `build_method/node_id.rs` | 117 | 181 | **+64** | t43/t44：LG-01（缓存键含 namespace；读回时校验该 namespace 是否在册） |
| `core/src/registry_core/authoring/parse/admission.rs` | 235 | 366 | **+131** | t43：B1-1（LG-02）紧凑形式双向承载 allow 与 deny |
| `core/src/registry_core/plugin/catalog/catalog.rs` | 382 | 538 | **+156** | t44：LG-03/LG-04/LG-40（写前校验、schema 门禁、字段语义） |
| `studio/studio/app/mutations.rs` | 388 | 420 | **+32** | t44：LG-03 写前校验 +228（含测试）→ t54 把测试模块拆出（-196） |
| `studio/studio/app/keyboard.rs` | 177 | 280 | **+103** | t52：B1-4 同族残余（Edit 表单预填） |
| `studio/studio/app/source_index.rs` | 102 | 217 | **+115** | t52：B1-4 同族残余（admission_text 第二份实现） |
| `studio/studio/app/tests/lock_writes.rs` | 0 | 201 | **+201** | t54：从 mutations.rs 拆出的仅测试模块（#[path] 挂载，免尺寸棘轮） |

### 12.2 三处新旧数字对照

**（a）代码体量**（谓词：`find . -name '*.rs' -not -path './target/*' -not -path '*/target/*'`）

| 时点 | 文件 | 行数 | 说明 |
| --- | --- | --- | --- |
| 冻结（本报告 §1.1 采信） | 394 | 78,951 | audit-structure-base.md（§1.1 采信） |
| t46 复核人 R5 当时 | 394 | 79,622 | t46 复核人 R5 当时（只有 5 个 B1 文件落地，t52/t54 尚未落地）（+671） |
| **t53 记账时（本次实测）** | 395 | 79,878 | t53 本次实测；逐文件 Δ 之和为 1007（含新增文件 201 行） |

**口径注（必须写清）**：逐文件 Δ 之和（1007）与「实测总量 − 冻结总量」（927）**差 80 行**——因为冻结之后 HEAD 已被推进（`a524956`），`git diff` 已无法还原 17:12 的逐文件基线。因此本节只报**实测值 + 逐文件对照**（逐文件表以 HEAD 为冻结态，对修复文件成立），**不做减法推算**；任何需要逐字节对账的场合请以 12.1 的表为准。

**（b）R-4 枚举行**（谓词：全树行首 `mod NAME;` / `mod NAME {`（含 inline），排除 target/）

| 时点 | 模块名 | 声明处 |
| --- | --- | --- |
| 冻结（报告 §7.6/§7.7 引用） | 296 | 440 |
| t46/R5 复核当时 | 297 | 442 |
| **t53 记账时（本次复算）** | **299** | **444** |

新增的 4 条声明（这就是 +3 名 / +4 处）：

| 文件:行 | 模块名 | 所属轮次 |
| --- | --- | --- |
| `build_method/identity_cache.rs:121` | `tests` | B1 修复轮 |
| `studio/studio/app/mutations.rs:420` | `lock_writes` | B1 修复轮（t54） |
| `studio/studio/app/keyboard.rs:180` | `edit_form_tests` | t52 |
| `studio/studio/app/source_index.rs:140` | `admission_text_tests` | t52 |

即当前枚举为 **299 / 444**（模块名 / 声明处）。

**裁决数不变（已实跑复算）**：R-4a 14 名 / 14 处、R-4b 15 名 / 26 处（豁免 1 处）、R-4c 6 对违规 + 1 对登记豁免 + 7 对单复数族——出处：实跑 /tmp/r4_from_report.py（t42 从报告 §G 原样抽取的脚本）。4 条新声明都是测试模块名，不在任何角色谓词的违规集里，所以只动枚举行、不动裁决。

**（c）因内容移位需要更新的标题锚点**（`file:line` 的旧值仍指向真实文件与界内行号，但「标题行就是缺陷发生处」已不成立）

| 条目 | 严重度 | 文件 | 冻结锚点 | **现行锚点** | 说明 |
| --- | --- | --- | --- | --- | --- |
| `LGC-LG-01` | CRITICAL | `build_method/node_id.rs` | `:36` | **`:100`** | 缓存键现在含 namespace（`NodeIdCache::insert`/`get` 见 `:59`/`:81`），`:36` 的键检索移到 `:100` |
| `LGC-LG-02` | CRITICAL | `core/src/registry_core/authoring/parse/admission.rs` | `:40` | **`:63`** | 读回构造形式时两张列表都进 `compact_admission`（`:63`），旧 `:40` 的「只写 allow」分支已不存在 |
| `LGC-LG-03` | CRITICAL | `studio/studio/app/mutations.rs` | `:257` | **`:320`** | 写盘前交给内核裁决（`PluginCatalog::with_appended_line`，`:320`）；`:257` 的 record 拼接本身未变 |
| `LGC-LG-04` | MAJOR | `core/src/registry_core/plugin/catalog/catalog.rs` | `:126` | **`:131`** | schema 门禁落在 `parse` 的表头处理 `:131` 与 `schema_matches`（`:115`） |
| `LGC-LG-40` | MINOR | `core/src/registry_core/plugin/catalog/catalog.rs` | `:281` | **`:317`** | `accounts_for` 现为 `:317`（旧锚点落在一条测试断言上）；第二批（t7）把「十字段空来源」的读取语义改成「声明为空」（空列 = `Some("")`、`None` 只留给七字段形式），并由 t11 §4 独立证实 |
| `STU-S-08` | MAJOR | `studio/studio/app/mutations.rs` | `:320` | **`:337`** | 入口导入的判定现为 `:337`（`entry_text.contains(&format!("use {crate_name} as _;"))`） |
| `STU-V-01` | MINOR | `studio/studio/app/mutations.rs` | `:332` | **`:238`** | 行终止补齐移到内核侧追加助手（`core/src/registry_core/plugin/catalog/catalog.rs:238-242`）；调用点 `studio/studio/app/mutations.rs:319` |

### 12.3 修复轮新增条目（不进主清单）

**`FIXR-01` / MAJOR / `studio/studio/app/source_index.rs:47`（现行 `:71`） / 批次 B1-立即：安全与数据完整性**

- **标题**：Studio Edit 表单预填/写回仍丢 deny：`admission_text` 是内核紧凑拼法的第二份实现
- **现象与判据**：修复前的红测试逐字：`prefill="allow:ui" read=Ok(OwnedAdmission { allowed_paths: ["ui"], denied_paths: [] })`；后果是**写回**——`saving the untouched form erased the deny list: event=updated registration face …/control.rs …`（重写后的源码里已无 `ui/experimental`）。根因：`admission_text` 自己拼一整串紧凑形式，而内核的紧凑渲染器 `compact_admission` （`core/src/registry_core/authoring/parse/admission.rs:95`）是私有的，Studio 无法复用。
- **最小修复方向**：t52 已改为按内核子句语法逐列表渲染（两张都空则 `ANY`）+ `debug_assert` 交给内核解析器裁决，并以 `both_lists_survive_the_compact_rendering` 与 `the_kernel_reads_back_every_value_this_renderer_emits` 双向互钉；**建议下一步**把内核的 `compact_admission` 提升为公开入口（core 提供 `OwnedAdmission` 的紧凑渲染），Studio 这份即可删除——本任务因 `core/` 不在 inScope 未能做到。
- **出处（provenance）**：t43 的 out-of-scope 报告（B1-4：LGC-LG-02 的同族残余，t43 inScope 之外；t52 承接修复）
- **严重度说明**：与 LGC-LG-02（CRITICAL）同源同后果，但路径是 Studio 表单的预填/写回、且内核侧读回已不再静默丢字段，故记 MAJOR 而不是 CRITICAL。
- **两处消费方**：studio/studio/app/keyboard.rs:79（Edit 预填）；studio/studio/app/lifecycle.rs:212（依赖 admission 行）；原文引用的是 `studio/studio/ui/search.rs` → `studio/studio/ui/panels.rs`（检视器显示）这条路径；2026-09-28 20:35 实测该路径已被 t27 的 Studio 重构移除（文件里的别名与消费者都不在了），本字段按现状更新，t27 的交付可能再动它
- **复核状态**：`partial`（无独立复核人）

这 1 条**不计入**主清单的 212 条（也不计入 §2 队列、§3 映射表与批次计数），是本节的「修复轮新增」小计。

### 12.4 复核进度与剩余缺口（必须与 12.1 一起读）

**三份 B1 复核 + 第二批的一条补证已落地，逐条结论见 `verifications` 与 §12.5 / §13**（`audit-verify-fix-b1.md`（t51）、`audit-verify-fix-b1-1.md`（t55）、`audit-verify-fix-b1-4.md`（t56），以及第二批的 `audit-verify-kernel-api.md`（t11，它把 `LGC-LG-40` 的「十字段空来源」语义独立证实）。7 条受影响条目里 **5 条**被复核证实（`LGC-LG-01`/`LGC-LG-02`/`LGC-LG-03`/`STU-V-01` 由 t51 逐条复算，`LGC-LG-02` 另获 t55 的第二条装置；`LGC-LG-40` 由 t11 §4 证实），**2 条仍未被任何独立复核覆盖**（`LGC-LG-04` schema 门禁、`STU-S-08` 入口导入判定）——这 2 条**留在 `fixed`、不计 verified**。另有 1 条修复轮新增条目 `FIXR-01` 由 t56 与第二批的 t11 两步证实（`new_findings_verified = 1`）。

**计数**：受影响 7 条 = verified **5** + fixed 2；修复轮新增 1 条（其中 verified 1）；**合计 verified 6 条**、独立复核报告 4 份。

### 12.5 三份独立复核与逐条结论（t51 / t55 / t56）

数据来自三份复核报告本身；本节**只记账、不替复核下结论**。

| 复核 | 报告 | 判定对象 | 手段（摘要） | 结论 |
| --- | --- | --- | --- | --- |
| **t51** | `audit-verify-fix-b1.md`（213 行） | `LGC-LG-02`（证实）；`LGC-LG-03`（证实）；`LGC-LG-01`（证实）；`STU-V-01`（证实） | 自建探针（`/tmp/vs-probe`，path 依赖真实 crate；两个夹具包共享相对路径、命名空间不同）+ 每条一次变异测试（改回旧行为 → 探针与作者钉子变红 → 逐字节还原）+ 全仓进程级缓存普查 + 五条门禁在**哈希钉住的树**上复跑。 | 三条 B1 修复（B1-1/B1-2/B1-3）判定为**已关闭**。 |
| **t55** | `audit-verify-fix-b1-1.md`（136 行） | `LGC-LG-02`（证实） | 树外 `/tmp/nk-probe`（path 依赖只读 core、features=["syntax"]），探针 B11 共 18 项；不复用作者测试；把读回的 `OwnedAdmission` 还原成运行期 `Admission` 再问 `accepts`，因此断言的是门禁语义而不是文本；变异回旧实现后 `accepts("ui/experimental")==true` 变红。 | B1-1 成立（B1-1 的**第二条独立验证**，装置与 t51 不同）。 |
| **t56** | `audit-verify-fix-b1-4.md`（101 行） | `FIXR-01`（证实） | 检出的完整副本 `/tmp/nk-b14` + 自写探针：种子工程 → `App::load()` → **真按键** `handle_key('e')` 取 Edit 预填 → `parse_admission_owned` 重建门禁 → 断言 `accepts("ui/experimental")==false`；再用保存路径的 `render_admission` 断言写回源码仍含 deny；最后用 `TestBackend(200×50)` 真渲染一帧断言缓冲文本含被 deny 的路径。变异 A 复现 t5 …（全文见 `audit-findings.json`） | B1-4 关闭（= 本节的 `FIXR-01`）。 |
| **t11** | `audit-verify-kernel-api.md`（363 行） | `FIXR-01`（证实）；`LGC-LG-40`（证实） | /tmp/t11-ext 独立工程（crate 外 path 依赖，覆盖内核路径与 run_method shim 路径）+ 改坏内核入口让 Studio 变红+ 自写结构检查器（把副本放回即 FOUND 的变异反证）+ X-1 自造十字段空列夹具（删掉 filter 即翻红） | acceptance 4/4 通过、0 条阻断项；`LGC-LG-40`（X-1）的语义与 `FIXR-01` 的收回都由本条证实。 |

**逐条映射**（哪条 finding 被哪份复核、以什么手段、判为什么）：

| finding | 现状态 | 复核 | 判定 | 手段与证据（引自复核报告） |
| --- | --- | --- | --- | --- |
| `LGC-LG-01` | **verified** | `audit-verify-fix-b1.md` | 证实 | t51 §4：端到端 4 组（冷/热 × A→B/B→A，两个夹具包共享同一相对路径），两包各拿到自己命名空间下现算的身份（`e0851ccb…` / `325377c4…`）；变异成「键只有相对路径 + 无读取侧复核」后热 A→B 复现旧污染、作者两条钉子 FAILED，还原零残留。 |
| `LGC-LG-02` | **verified** | `audit-verify-fix-b1.md`、`audit-verify-fix-b1-1.md` | 证实 | t51 §2：构造形式读回 `allow:ui,ui/controls;deny:ui/experimental`，`denied` 未丢、读写往返稳定；变异回旧三分支后探针 FAIL 且作者钉子 FAILED。t55：树外第二条装置（探针 B11 共 18 项）——把读回的两张列表还原成运行期门禁后`accepts("ui/experimental")==false`（断言的是门禁，不是文本）；变异后该行变红。 |
| `LGC-LG-03` | **verified** | `audit-verify-fix-b1.md` | 证实 | t51 §3：重复身份追加被 `with_appended_line` 拒（Err、不写盘）；无换行的锁追加后补换行、parse 得 2 条；被拒时入口文件逐字节未变；变异成「不校验不补换行」后探针 FAIL 且作者四条 `lock_writes` 钉子全 FAILED。 |
| `LGC-LG-04` | **fixed** | —（三份复核均未覆盖） | 未复核 | 三份复核均未覆盖 schema 门禁本身（t51 §1「已核范围」只覆盖三条 B1 修复与 t44 写盘路径的调用点）。 |
| `LGC-LG-40` | **verified** | `audit-verify-kernel-api.md` | 证实（第二批 t11 §4；t51 当时记录该族仍未修完） | /tmp/t11-ext 独立工程（crate 外 path 依赖，覆盖内核路径与 run_method shim 路径）+ 改坏内核入口让 Studio 变红+ 自写结构检查器（把副本放回即 FOUND 的变异反证）+ X-1 自造十字段空列夹具（删掉 filter 即翻红） |
| `STU-S-08` | **fixed** | —（三份复核均未覆盖） | 未复核 | 三份复核均未单独覆盖入口导入判定（t51 只核到 `mutations.rs` 的调用点与内核 API 行为）。 |
| `STU-V-01` | **verified** | `audit-verify-fix-b1.md` | 证实 | t51 §3 同一次探针：「无换行锁追加后补换行、parse 得 2 条」正是本条的缺陷面；变异体下该面与四条作者钉子同时变红。 |
| `FIXR-01`（修复轮新增） | **verified** | `audit-verify-fix-b1-4.md` | 证实 | t56：检出差副本 + 自写探针（真按键取 Edit 预填 → 重建运行期门禁断言被 deny 的路径仍被否决；保存路径写回仍含 deny；`TestBackend(200×50)` 真渲染一帧含被 deny 的路径）+ 两次变异（变异 A 复现 t52 之前的丢失；变异 B 证明 `debug_assert` 是活的）。 |

**复核报告自报的披露与未覆盖范围**（照实收录，供读者判断证据强度）：

- **t51**：第一次 identity 探针有取证 bug（在共享 units 目录取第一个命中 → 误报 FAIL），改成按包自己的 fingerprint 精确打开后四组全 PASS；误报输出不作证据
- **t51**：变异窗口 18:31–18:33 与另一位成员的 `cargo test --workspace` 重叠——**那一窗口里别人跑出的 workspace 结果应作废重跑**；t51 自己的门禁改在 18:36–18:37 并用源文件哈希钉住
- **t51** 未覆盖的条目：`LGC-LG-04`、`LGC-LG-40`、`STU-S-08`
- **t55**：未复核紧凑语法其它扩展的语义取舍；`admission.rs:48/52` 的历史单列表分支建议下一轮并入 `compact_admission`
- **t56**：极窄面板下的检视器换行/截断形态未测（200 列宽未触发截断）
- **t56**：`debug_assert` 只在 debug/test 构建生效——release 下由作者两条测试把关
- **t11** 未覆盖的条目：``contains_manifest` 本体未直调（入参是 PluginManifest，crate 外无文本构造器）`、`字节不变性是读码 + git 历史口径`、`未跑 `--all-features --doc`、`package-audit`、`--verify-consumers``

**规矩**：`verified` 只记**独立复核**的结论；作者自验（先红后绿的钉子）一律不计入。被复核证伪或部分证实的条目会**留在 `fixed`** 并在上表如实写明——本轮三份复核的判定都是「证实」，没有证伪项；但 `LGC-LG-40` 的这一族（official 十字段空来源读取语义）被 t51 明确记录为**仍未修**，所以它留在 `fixed`、不计 verified。

## 13. 修复轮第二批（B2 门禁与工具 / B3 桥与宿主 / 内核公开入口 / X-1）

**本节是账，不是新结论。** §0–§11 与 §12 的结论一字未改；本节只把第二批修复轮（t1–t22，19:01–20:13 落地）的结果折进来，并逐条标出独立复核的判定。本节每个数字都由 `audit-findings.json` 现算：逐条的 `findings[].fixed_in_batch` / `.verification` 与汇总的 `fix_batch_2` 块，由 `check_report.py` 逐条复算。

### 13.1 本批修掉的条目（39 条）

| 条目 | 严重度 | 批次 | 修法（任务：旧 → 新） | 独立复核 | 判定 |
| --- | --- | --- | --- | --- | --- |
| `GTE-G-03` | MINOR | B2 | t1：围栏按「同字符、长度 ≥ 开围栏、其后无 info」配对，info 取自整串围栏之后；文档注释半边同规则 | `audit-verify-b2-1.md` §2.1 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-04` | MINOR | B2 | t1：`mod` 声明先剥 `pub`/`pub(crate)`/`pub(in …)`；`#[cfg(…)]` 按表达式判定（`all` 任一 test-only 即真、`any` 需全部分支、`not` 为假、空 `any()` 为假、字符串里的 test 不算） | `audit-verify-b2-1.md` §2.2 · t9（kernel-auditor，非作者） | **部分证实** |
| `GTE-G-05` | MINOR | B2 | t1：非发布成员**写出来的**版本要求也绑到发布线（`publish = false` 仍可省略 `version`、`[package] version` 仍可自定） | `audit-verify-b2-1.md` §2.3 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-07` | MINOR | B2 | t1：新增 `without_comments` / `without_yaml_comments`：注释不算要求位置，字符串字面量里的真要求照报 | `audit-verify-b2-1.md` §2.4 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-08` | MINOR | B2 | t1：`belongs_to_the_attribute_stack`：`#[…]`、注释行与 `*` 对齐行随属性一起抹白 | `audit-verify-b2-1.md` §2.5 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-09` | MINOR | B2 | t2：`--workspace-version` 成为独立模式：与任何其它模式/标志同现一律 exit 2（六种组合各测一次），`--help` 延迟到参数校验之后 | `audit-verify-b2-1.md` §3.1 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-10` | MINOR | B2 | t2：符号审计不再可能静默为空：nm 失败与零符号分别记 `nm-failed` / `0` 到 TSV 与 unaudited.tsv，并以 `N of M …` exit 1 | `audit-verify-b2-1.md` §3.2 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-11` | MINOR | B2 | t2：`--help` 用 awk 打完整头部注释块（不再写死 `sed -n '2,26p'`），39 行、头部 36 行逐行出现 | `audit-verify-b2-1.md` §3.3 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-12` | MINOR | B2 | t2：`sed -i` 改临时文件 + `mv`（并转义 `\ & |`）；`date +%s%N` 加探测与 `date +%s`×1000 回退 | `audit-verify-b2-1.md` §3.4 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-16` | MAJOR | B2 | t3：roadmap 两处与 `tools/xirang-package-audit` 头部不再复述发布顺序，顺序归 `tools/xirang-publish` 的 `levels` 表唯一来源 | — | **未获独立复核** |
| `GTE-G-17` | MINOR | B2 | t1 + t3：`conventions/src/size.rs` 的 BASELINE 文档不再复述项数（它就是 `BASELINE.len()`）；`docs/roadmap-1.0.md:40` 的「14 项」同样删数 | `audit-verify-b2-1.md` §2.7 (a) · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-18` | MINOR | B2 | t3：CHANGELOG 两处 `version = "0.1.4"` 要求改指 `xirang-*` 版本要求（该字面量全仓 0 命中） | — | **未获独立复核** |
| `GTE-G-19` | MINOR | B2 | t3 + t15：AGENTS.md 门禁清单补齐 naming / release_version / release_workflow / doc_anchors，并把 required-features 半限定为「手列的在检出内 target」；t15 再把 doc_anchors 的覆盖面按后缀与 sources 写实（`.rs`/`.toml`/`.yml`/`.yaml`、根与成员 `Cargo.toml`、`.github/workflows/*`，并写明 `.md` 目标与无扩展名脚本仍不覆盖） | — | **未获独立复核** |
| `GTE-G-27` | MINOR | B2 | t3：roadmap 改为「依赖下界随发布线一起抬（写 `^<工作区版本>`）；`release_version` 与 `--check-table` 两道门禁强制它等于工作区版本」 | — | **未获独立复核** |
| `GTE-G-20` | MINOR | B2 | t1：`conventions/src/mounting.rs:10` 按 fix_hint 删掉散文数字，改为「计数不在此复述、没有任何机制为散文里的计数对账」 | `audit-verify-b2-1.md` §2.7 (d) · t9（kernel-auditor，非作者） | **部分证实** |
| `GTE-G-21` | MAJOR | B2 | t3：`tools/xirang-package-audit` 头部不再描述已不存在的手工表：crate 集合与依赖从工作区推导 | — | **未获独立复核** |
| `GTE-G-22` | MINOR | B2 | t1：`conventions/src/lint.rs:15` 的模块边界与 `required_roots` 的函数文档统一（除示例宿主外每个 crate 目录的库根 + MCP 二进制根） | `audit-verify-b2-1.md` §2.7 (c) · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-24` | MINOR | B2 | t1：`conventions/src/size.rs` 的挂载链句写全 6 文件 / 5 条边，并说明「层 = 链上文件数」 | `audit-verify-b2-1.md` §2.7 (b) · t9（kernel-auditor，非作者） | **证实** |
| `GTE-G-25` | MINOR | B2 | t3：`.github/workflows/ci.yml` 的悬挂指令（`after the first crates.io release`）改为 early leftover 说明；roadmap 的同事实一处同改（只动注释，`name`/`if`/`run` 一字不动） | — | **未获独立复核** |
| `GTE-G-26` | MINOR | B2 | t3：AGENTS.md / `tools/xirang-package-audit` / roadmap 去掉 “from today / 从今天起 / 今天” 一类时态锚点 | — | **未获独立复核** |
| `GTE-N-1` | MAJOR | B2 | t4：tag 守卫由子串判定改**形状白名单**（`GuardShape` + `positive_tests`：顶层 `&&` 切分、括号组递归、`${{…}}` 先剥，只允许 tag 判断本身与 `path == 'literal'`；取反 / `||` / `!=` / 未列举运算符一律报出），并修 `steps()` 折叠 `if:` 的缩进边界 | `audit-verify-b2-1.md` §4.1 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-N-2` | MINOR | B2 | t4：`takes_input` 改结构化引用判定：`inputs.name` 与 `inputs['name']`（两种引号）都算读，注释行不算 | `audit-verify-b2-1.md` §4.2 · t9（kernel-auditor，非作者） | **证实** |
| `GTE-N-3` | MINOR | B2 | t4：`doc_anchors` 按写下路径后缀触发（`.rs`/`.toml`/`.yml`/`.yaml`），sources 加上根与各成员的 `Cargo.toml` 与 `.github/workflows/*`；`.md` 目标与无扩展名脚本仍不覆盖 | `audit-verify-b2-1.md` §4.3 · t9（kernel-auditor，非作者） | **部分证实** |
| `BRG-BR-1` | MAJOR | B3 | t5：两份 mcp README 的 records 桶枚举改成 `mcp/diff.rs:217` 权威计数行的五桶（`ok`/`undeclared`/`stale`/`re-identified`/`unreadable`），`unmatched` 0 命中；CHANGELOG `[0.1.6]` 的同承诺由 t21 收口 | `audit-verify-b3-1.md` §1 BR-1 · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-2` | MAJOR | B3 | t5：删掉自相矛盾的「树 diff 仍待做」整句与其失效锚点，改以可核实的能力句收尾 | `audit-verify-b3-1.md` §1 BR-2（附行号更正） · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-7` | MINOR | B3 | t5：`xirang.usages` 的描述与 `mcp/usages.rs` 打印的 22 个标签逐字一致（两份 README 同步），并加钉子 `the_usages_description_names_every_field_it_prints` | `audit-verify-b3-1.md` §1 BR-7 · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-9` | MINOR | B3 | t6 + t13：同一 crate 里两种「可移植路径」拼法收回一处：mcp 三处改为转发内核 `xirang::declaration::portable_path`（`mcp/index.rs:178` 一行转发）；t13 再把 build_method 4 处 + conventions 2 处 + 2 个测试收口 | `audit-verify-b3-1.md` §1 BR-9 · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-11` | MINOR | B3 | t5 + t6：测试挂载统一：`mcp/index.rs` 与 `mcp/tools.rs` 的内联 `mod tests` 分别迁入 `mcp/index_tests.rs` / `mcp/tools_tests.rs`（`^mod tests {` 全仓 0 命中） | `audit-verify-b3-1.md` §1 BR-11 · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-12` | MINOR | B3 | t5：分派从 `match` 改一张有序 `DISPATCH` 表（`mcp/tools.rs:302`）+ 两条钉子：`the_dispatch_table_follows_the_catalog` 与 `every_listed_tool_is_dispatched` | `audit-verify-b3-1.md` §1 BR-12 · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-15` | MINOR | B3 | t5：`cli/README.zh-CN.md` 补 `snippets` 行，两份命令表的 studio 行都改 `xirang studio [path]`（表格行 9 / 9 对齐） | — | **未获独立复核** |
| `BRG-BR-17` | MINOR | B3 | t30：输入超限消息拆成两条，与 Wasm 适配器同形：新增 `MAX_FRAMEABLE_INPUT`（`u32::MAX`）与共享 `check_input_length`（`plugin-host/src/process.rs:460`），调用点改为 `check_input_length(input.len(), self.limits.max_input_bytes)?`（`:296`）；配置上限报 `input is N bytes; limit is M`，帧宽报 `input is N bytes; a process call frames its length in a u32, whose maximum is 4294967295` | — | **未获独立复核** |
| `BRG-BR-C2` | MAJOR | B3 | t5 + t14：`mcp/preview.rs` 的目录说明改成实际跳过集合，两处遍历共用一份 `fn skipped_directory`（`mcp/preview.rs:52`，`.git/` 入清单）；t14 把行为落成仓内钉子 `mcp/preview_tests.rs` | — | **未获独立复核** |
| `BRG-BR-C3` | MINOR | B3 | t5：同一实测数字只留一处（`mcp/callgraph.rs` 模块文档的 142），函数内注释与 `tools_tests.rs` 的模块文档改为指回 | — | **未获独立复核** |
| `BRG-BR-C4` | MINOR | B3 | t5：`cli/lib.rs:169`/`:175` 的 `nihlink build` 改 `xirang build`（`\bnihlink\b` 在 cli/mcp/plugin-host 三处源码 0 命中） | — | **未获独立复核** |
| `BRG-BR-C5` | MINOR | B3 | t6 + t17 + t18：`slot` 三义收敛：`mcp/evidence.rs:127` 逐面列名改 `registry_name`、树投影表头 `slots:` 改 `faces:`；状态词形由 `mcp/tree_delta.rs:64` 的 `FaceStatus::label()` 单源导出（`diff.rs`/`search.rs` 改调）；t17/t18 把桥文档、工具描述、测试文档、内核字段文档与两份根 README 的 `slot` 措辞一并收敛 | `audit-verify-b3-1.md` §1 BR-C5①/② · t10（surface-auditor，非作者） | **证实** |
| `BRG-BR-C6` | MINOR | B3 | t6：无名三元组类型化：`struct BuildEvidence { current, scope, pruning }` 具名，并把「current 回答的是这份产物是否仍在描述这批源码」放进 `BuildEvidence::freshness()`（`mcp/evidence.rs:73`）；overlay 不再按位置解构 | `audit-verify-b3-1.md` §1 BR-C6 + §6 结论 · t10（surface-auditor，非作者） | **部分证实** |
| `BRG-BR-C7` | MINOR | B3 | t6：`apply` 的目标类型化：`enum Target { Project, Copy(PathBuf) }`（`mcp/apply_target.rs:27`）+ `report_path`/`report_message`；端到端两条路（预览不改盘、落盘字节 == 预览 diff） | `audit-verify-b3-1.md` §1 BR-C7 · t10（surface-auditor，非作者） | **证实** |
| `FIXR-01` | MAJOR | 内核入口 | t7 + t8 + t16：内核公开 `xirang::authoring::parse::compact_admission`（`core/src/registry_core/authoring/parse/admission.rs:73`）成为唯一渲染器，`parse_admission_expression` 的历史单列表分支并入它；Studio 的 `admission_text`（`studio/studio/app/source_index.rs:69`）只委派；同族第三例 `compact_registration_rule`（`core/src/registry_core/authoring/parse/rules.rs:82`）与 `registration_rule_text`（`studio/studio/app/source_index.rs:96`）同批收回 | `audit-verify-kernel-api.md` §3 + §7 · t11（gates-auditor，非作者） | **证实** |
| `LGC-LG-40` | MINOR | X-1 | t7：`PluginCatalog::parse` 的十字段三列去掉 `.filter(!is_empty())`：空列落成 `Some("")`＝声明「此处没有值」，`None` 只留给七字段形式；新增 `accounts_for_provenance`（`core/src/registry_core/plugin/catalog/catalog.rs:372`）按此语义钉住候选 | `audit-verify-kernel-api.md` §4 · t11（gates-auditor，非作者） | **证实** |

- `FIXR-01` 是 §12 的修复轮新增条目，**不计入**主清单的 212 条：本批 39 条 = 主清单行 **38** + §12 新增条目 **1**（`FIXR-01`）。
- 本节的逐条字段只**增**不减：原 `file`/`line`/`title`/`severity` 与 `evidence` 一字未改（`check_report.py` 会核这一点在 §12 与 §13 两侧同时成立）。

### 13.2 追加与收尾状态（已关闭 / 已修 / 挂账）

- **`F-1`**（low）— 已记账（不改结论）：`conventions/src/doc_anchors.rs:166 与 t4 的交付文本`。「52 处新锚点」按 t9 的口径更正：门禁实际口径今天 1 处，全 `docs/` 树 124 处（104 处在被豁免的 record 文档里）。t4 的实现与行为面不变。
- **`F-2`**（low）— 已修（t22；钉子由 t24 搬进兄弟测试文件）：`conventions/src/size.rs 与 conventions/src/size_tests.rs`。`declares_module` 现在真的接受 `mod  x ;`（空白无关比较 + `mod` 关键字边界）；独立复核 t25 进行中。
- **`F-3`**（low–medium）— 已修（t22；钉子由 t24 搬进兄弟测试文件）：`conventions/src/naming.rs 与 conventions/src/naming_tests.rs`。`package = "…"` 半边改为结构化读取（`package_field_value`），转义引号下重命名的内部依赖不再隐形；独立复核 t25 进行中。
- **`F-4`**（low）— 已修（t22）：`conventions/src/mounting.rs:10`。散文数字删掉（判别性 grep：HEAD 2 命中 → 0）；独立复核 t25 进行中。
- **`BR-C6 附加声明`**（low）— 已由 t23 关闭：`mcp/overlay.rs:72 / mcp/converge.rs:133`。两处改为消费 `evidence.freshness()`，不再内联 `current` / `stale (run xirang check)`；新耦合钉子 `evidence::evidence_tests::every_report_spells_the_freshness_word_that_one_place_produces` 的三态证据：改前+变异 108 passed 全绿 → 现码+同一变异 107 passed / 2 failed → 还原 109 passed 全绿（详见 §13.6）；t10 的「部分证实」判定不改，t23 的独立复核 t25 进行中。
- **`SUR-S10 位置漂移`**（MINOR）— 已登记（冻结位置已被 t28 的 B4 重构移除）：`run_method/macros/face_objects.rs`。该条点名的「前端回退臂用绝对 crate 路径」已在第二批被 t28 修掉，文件 172→164 行，冻结的 `:169` 不存在了；冻结记录不改 file/line，只加 `location_note` 并把这一处引用登记进 `meta.unresolved_quoted_anchors`。
- **`BRG-BR-17`**（MINOR）— 已修（t30）：`plugin-host/src/process.rs:296（调用点）/ :450（MAX_FRAMEABLE_INPUT）/ :460（check_input_length）`。输入超限拆成两条消息（配置上限 / `u32` 帧宽），与 `plugin-host/src/wasm.rs` 的 `check_input_length` 同形；两条钉子先红后绿（红＝`E0425: cannot find function check_input_length`），并有变异反证（退回旧单条消息 ⇒ 帧宽那支报配置上限）；t30 未获独立复核（本批未派）。

- **未获独立复核的 B2 条目（7 条）**：`GTE-G-16`、`GTE-G-18`、`GTE-G-19`、`GTE-G-27`、`GTE-G-21`、`GTE-G-25`、`GTE-G-26`——t3 组（文档漂移与注释准确性）本批未派独立复核任务。
- **未获独立复核的 B3 条目（5 条）**：`BRG-BR-15`、`BRG-BR-17`、`BRG-BR-C2`、`BRG-BR-C3`、`BRG-BR-C4`——原因见各自 `verification.note`（未派复核 / 不在 t10 的逐条判定清单内）。

### 13.3 本批新增的公开符号（2）

| 符号 | 位置 | 任务 | 门控 | 调用点 | 独立复核 |
| --- | --- | --- | --- | --- | --- |
| `xirang::authoring::parse::compact_admission` | `core/src/registry_core/authoring/parse/admission.rs:73` | t7 | `#[cfg(feature = "syntax")]` 之后（t11 负向实测：去掉 `syntax` 即 `E0432`） | `studio/studio/app/source_index.rs:69（`admission_text`）` | **证实**（`audit-verify-kernel-api.md` §1 + §7） |
| `xirang::authoring::parse::compact_registration_rule` | `core/src/registry_core/authoring/parse/rules.rs:78` | t16 | 同族，`#[cfg(feature = "syntax")]` 之后 | `studio/studio/app/source_index.rs:96（`registration_rule_text`）` | **未获独立复核**（t16 自带的仓内钉子 `core/tests/registration_rule_entry.rs` + t20 的注释漂移修正；本批未派覆盖它的复核任务（t11 判定 t7/t8，其 §7 记「新增公开符号 = 1 个」）） |

### 13.4 counts 变化

- 本批记账条目 **39** = 主清单行 **38** + §12 新增条目 **1**（`FIXR-01`，不计入 212 条）
- 按标签：**23** B2（`B2-门禁与工具`，23 条中 23 条）、**14** B3（`B3-桥与宿主`，14 条中 14 条）、**1** 内核公开入口（`FIXR-01`）、**1** X-1（`LGC-LG-40`）
- 独立复核判定：**证实 23** / 部分证实 **4** / 未获独立复核 **12**（未获独立复核：`BRG-BR-15`、`BRG-BR-17`、`BRG-BR-C2`、`BRG-BR-C3`、`BRG-BR-C4`、`GTE-G-16`、`GTE-G-18`、`GTE-G-19`、`GTE-G-21`、`GTE-G-25`、`GTE-G-26`、`GTE-G-27`）
- 独立复核报告：**3** 份（§13.5）

**逐批剩余**（triage 批次 × 已落修复标记：`fixed_in_fix_round`（§12）或 `fixed_in_batch`（本节））：

| 批次 | 主清单条目 | 已落修复 | 剩余 |
| --- | --- | --- | --- |
| B1-立即：安全与数据完整性 | 3 | 3 | **0** |
| B4-MAJOR 逻辑与实现缺陷 | 50 | 2 | **48** |
| B6-MINOR 一致性与清扫 | 51 | 2 | **49** |
| B5-注释与自述 | 21 | 0 | **21** |
| B3-桥与宿主 | 14 | 14 | **0** |
| B2-门禁与工具 | 23 | 23 | **0** |
| B7-命名抽象与可读性 | 30 | 0 | **30** |
| B8-命名歧义与约定冲突 | 20 | 0 | **20** |

- **§12 的 B1 轮账目随之更新**：`LGC-LG-40` 由 `fixed` 转 `verified`（`audit-verify-kernel-api.md` §4）⇒ §12 的 `fixed` 3→2、`verified` 4→5、仍未复核 3 条→2 条（`LGC-LG-04`、`STU-S-08`）、独立复核报告 3→4 份、`verified_total` 5→6。

### 13.6 追加与收尾：t22 / t23 / t24 / t30

本节的追加项与它们各自的结论；`verification` 一栏写的是独立复核的现状（t25 的对象是 t13/t16–t24）。

| 任务 | 对象 | 结论 | 独立复核 |
| --- | --- | --- | --- |
| `t22` | 修 t9 复核发现 F-2/F-3/F-4 | 三条钉子先红后绿：F-3（medium）改结构化读取 `package_field_value`（转义引号下的重命名内部依赖不再隐形）；F-2 让 `declares_module` 真的接受 `mod  x ;`（空白无关比较 + `mod` 关键字边界，`modx;` 仍不是）；F-4 删掉 `mounting.rs` 的散文数字（判别性 grep：HEAD 2 命中 → 0）。 | 进行中（t25；其报告 audit-verify-extra.md 已在写，本节不据未完成的复核下结论） |
| `t23` | 同族第四例：overlay/converge 内联新鲜度词形 + 耦合钉子 | 词形单源：`mcp/overlay.rs:72` 与 `mcp/converge.rs:133` 改为消费 `evidence.freshness()`，两文件里 `"current"` / `"stale (run` 0 命中（grep exit 1）；新增耦合钉子 `evidence::evidence_tests::every_report_spells_the_freshness_word_that_one_place_produces`（对已发布证据的树与无证据的树分别断言 explain / overlay / converge 三条打印同一句词）。 | 进行中（t25；其报告 audit-verify-extra.md 已在写，本节不据未完成的复核下结论） |
| `t24` | 把 t22 的钉子搬进 *_tests.rs（离开 600 行棘轮的头寸） | `size.rs` 587→553、`naming.rs` 550→472（头寸 13→47 / 50→128）；迁移前后各 117 条测试的叶子名逐名 diff 为空（唯一差异是 4 行模块前缀），断言逐字不变；门禁文件里已无内联 `#[cfg(test)]` 模块。 | 进行中（t25；其报告 audit-verify-extra.md 已在写，本节不据未完成的复核下结论） |
| `t30` | 收尾：修 BRG-BR-17 + 本节的账目更新 | B3 最后一条存量缺陷关闭：输入超限拆成两条消息（配置上限 / `u32` 帧宽），与 Wasm 适配器同形；本节据此把 `BRG-BR-17` 记为已修、把 BR-C6 残余记为已由 t23 关闭，并按 20:35 实测刷新 `FIXR-01` 的 `consumers_fixed` 引用（原文那条检视器路径已被 t27 的 Studio 重构移除——只换引用，不改结论）。 | 未获独立复核（本批未派） |

**`t22` 的证据**：五条门禁 20:21:27–20:22:21 连测全绿；钉子红/绿原文见 t22 的交付记录。

**`t23` 的证据**：三态（同一变异：`freshness()` 的 current 分支 → `healthy`）：① 改动前+变异 ⇒ `cargo test -p xirang-mcp --offline` `108 passed; 0 failed`（overlay/converge 仍回旧词 ⇒ 三份词形无耦合，这就是 t10 的变异 E）；② 现码+同一变异 ⇒ `107 passed; 2 failed`（`overlay::overlay_tests::a_published_scope_marks_the_slots_it_prunes` 与上述新钉子）；③ 还原 ⇒ `109 passed; 0 failed`，工作区探针三条均 `build current`；变异只在 /tmp 副本，工作区四个 in-scope 文件与备份逐字节一致。

**`t24` 的证据**：五条门禁 20:25:42–20:27:01 连测全绿；`size` 门禁自带钉子仍判两个测试文件豁免。

**`t30` 的证据**：钉子先红（`E0425: cannot find function check_input_length`）后绿（`process::tests` 5 passed）；变异反证：退回旧单条消息 ⇒ 帧宽那支报 `input is 4294967296 bytes; limit is 18446744073709551615`（红），还原后绿。

### 13.5 本批的独立复核（3 份）

| id | 报告 | 行数 | 对象 | 判定 |
| --- | --- | --- | --- | --- |
| `t9` | `audit-verify-b2-1.md` | 314 | t1（G-03/G-04/G-05/G-07/G-08 + 棘轮归一化 + 四处注释准确性）、t2（G-09/G-10/G-11/G-12）、t4（N-1/N-2/N-3） | 8 证实 / 3 部分证实 / 0 证伪：G-04＝F-2、N-3＝F-1、注释准确性四处 3 证实 + 1 部分证实＝F-4。 |
| `t10` | `audit-verify-b3-1.md` | 129 | t5（B3-1 桥的承诺面与文档）、t6（B3-2 桥的内聚与类型化） | 承诺面与内聚项**实质项全部证实**（BR-1/2/7/9/11/12/C5①/②/C7）；唯一需后续动作是 BR-C6 的附加声明——overlay/converge 仍各持一份同词副本，没有钉子把三份耦合。 |
| `t11` | `audit-verify-kernel-api.md` | 363 | t7（内核公开入口 + `:48/52` 并入 + X-1/LG-40）、t8（Studio 删除第二份实现） | acceptance 4/4 通过、0 条阻断项；`LGC-LG-40` 与 `FIXR-01` 都由它证实（并更正了 §12 的 B1 轮账目）。 |

**`t9` 的装置**：自写 19 条夹具（crate 外的集成测试，直调 doc_blocks::findings / size::oversized / naming::findings / release_version::findings / release_workflow::findings / doc_anchors::findings 等真实入口，不复用作者任何测试）+ 7 组变异回退（逐个 sha256 证还原零残留）+ 真工具脚本矩阵（G-09/G-10/G-11/G-12）

未覆盖（原文登记）：完整 `tools/xirang-external-rehearsal` 复跑（只静态 + BSD 仿真）；`nm` 整个缺失的 mini PATH、`readelf`/`.inventory`、FULL/MINIMAL 分支；t1 的棘轮性能计时；t1/t2/t4 自述里“顺带修掉”的邻域（含 `GTE-G-17` 的 roadmap 半边、`GTE-G-19` 的 AGENTS.md 描述）。

**`t10` 的装置**：自写探针（/tmp/t10/probe 的 buckets/dispatch/apply_e2e/usages_labels，端到端 stdio，不调仓库测试辅助）+ 5 处变异（独立 CARGO_TARGET_DIR 的副本，逐字节还原并核 sha256）+ 哈希钉住的门禁复跑

未覆盖（原文登记）：Windows 分隔符路径（BR-9 的折叠只在 Unix 上以“文件名含反斜杠”的夹具暴露）；BR-C6 的另一半 `plugin-host/src/lazy_wasm.rs` 的 `is_loaded`（只核未被触碰）；BR-1 的语义等价未做端到端（`xirang.grafts` 的 `NOT declared by the host entry` 输出）；BR-C7 只端到端覆盖 `add`，其余动词与 `confirm` 门未逐动作跑；BR-15 / BR-C2 / BR-C3 / BR-C4 不在 t10 的逐条清单内。

**`t11` 的装置**：/tmp/t11-ext 独立工程（crate 外 path 依赖，覆盖内核路径与 run_method shim 路径）+ 改坏内核入口让 Studio 变红+ 自写结构检查器（把副本放回即 FOUND 的变异反证）+ X-1 自造十字段空列夹具（删掉 filter 即翻红）

未覆盖（原文登记）：`contains_manifest` 的本体未直调；字节不变性是读码 + git 历史口径；`--all-features --doc` / `package-audit` / `--verify-consumers` 未跑；t16 的第三例（`compact_registration_rule`）不在其判定清单内。

**本节的口径**：`fix_batch_2` 块由 `/tmp/xirang-audit-logs/patch_fix_batch_2.py` 落到 `audit-findings.json`（幂等）；`build_findings.py` 重新生成主清单后需重跑该脚本，本节与 §13.1 的表再由 `gen_report.py` 现算重出。HTML 角标（`audit-structure-map.html`）同源。

## 14. 第三轮记账（B4 复核 + 新条目 + 假绿 + 新增公开面）

**本节是账，不是新结论。** §0–§13 的结论一字未改；本节把第三轮（B4 落地与复核，20:30–22:55）的结果折进来，每个数字都由 `audit-findings.json` 的 `fix_batch_3` 块现算（`check_report.py` 逐条复算）。

这一轮要记的不只是"修了什么"，还有三种**与事实相反**的账，单列成节：

1. 交付**没做完**却记 `passed` —— 假绿，1 例（§14.4）；
2. 交付**做完了**却只能记 `failed` —— 假红，同一节的镜像条（平台不接受 failed 验收项 + verify 非零即 failed）；
3. **环境相关**的用例被当成判定依据 —— 1 例（§14.5）。

### 14.1 独立复核（3 份 · 21 个判定：20 证实 / 1 部分证实 / 0 证伪）

| 复核 | 复核者（非作者） | 对象 | 判定 | 报告 |
| --- | --- | --- | --- | --- |
| `t34` | kernel-auditor（非作者：t26=gates-auditor、t32=logic-adversary、t33=gates-auditor） | B4-core：t26（LG-04/27/28/30、KRN-C-02）+ t32（API 兼容）+ t33（LG-06/07①②/29） | **9 证实 / 1 部分证实 / 0 证伪** | `audit-verify-b4-core.md` |
| `t35` | logic-adversary（非作者：B4-studio 的实现者是 studio-auditor/logic-adversary 之外的人） | B4-studio 的 9 条修复（作者 6 条钉子只作输入，不采信其绿） | **9/9 证实** | `audit-verify-b4-studio.md` |
| `t41` | logic-adversary（非作者：两条的实现者是 run-engineer） | t28 拆分 A：LGC-LG-05、LGC-LG-31 | **2 条都从"已修（作者自述）"升为"已由非作者独立验证"** | （任务 output，无独立报告文件） |

#### t34 — 独立验证 B4-core（t26 已应用修复 + t32 API 兼容 + t33 接续，含变异反证）

- **装置与手段**：检出副本 /tmp/nk-t34（自带 CARGO_TARGET_DIR=/tmp/nk-t34-target）+ 自写 crate 外探针 core/tests/t34_probe.rs（5 条，数据与作者钉子不同）+ 9 组变异；判定只取 hash 稳定那次，12 个相关文件跑前跑后 sha256 一致

| 判定对象 | 判定 | 怎么证的 |
| --- | --- | --- |
| `LGC-LG-04`（t26） | **证实** | 自建 5 形态夹具（空锁未知 schema / 记录后表头 / 拼错 / 重复 / 空值）全拒 + 2 条正控可读；变异 `schema_matches` 恒真 → 作者钉子与探针同时红（M1b） |
| `LGC-LG-27`（t26） | **部分证实 → 2026-09-29 补齐** ✓ | 源码读到 `if authored.runtime_checks.declared() { self.runtime_checks = authored.runtime_checks; }`（与 `flow` 同规则；`t84` 起这条规则由 `declaration/owned.rs` 的 `Declared` trait 统一表达）；**复核当时未自建 `RuntimeCheckSpec` 夹具、未变异**。2026-09-29 收尾轮补做：作者的钉子已在位——`kernel/tests/b4_core_pins.rs` 的 `merging_an_authored_snapshot_keeps_the_authored_runtime_checks`（声明过则赢）与 `the_declared_rule_is_one_place_for_the_conditional_fields`（作者侧沉默 ⇒ 保留编译期取值；声明过 ⇒ 作者赢，两半都断言）；**变异反证**：把守卫改成 `if true`（恒真）⇒ `the_declared_rule_is_one_place_for_the_conditional_fields` **FAILED**（`a silent author side keeps the compiled checks`，`left: []` / `right: [FiniteNumber]`），还原后 sha256 一致（`6e218294a2d3b4fe`）且该文件 10 条全绿 ✓ |
| `LGC-LG-28`（t26/t32） | **证实** | 同源 `requirement_item`：published 有损返回、strict 报 Err、validator 报 Err；`parse_requirements_owned` 与 `git show HEAD:` 逐字同签名 |
| `LGC-LG-30`（t26） | **证实（两半）** | `ControlHandle<u8, u16>` → 单标签；变异 tokens.rs 半边 → 作者单测红（M3a），变异 parse.rs 半边 → 作者钉子与探针同时红（M3b） |
| `KRN-C-02`（t26） | **证实** | `CallSite`/`CallEdge` 两句文档与 finding 的 fix_hint 逐句对上，探针读源码断言两句都在 |
| `LGC-LG-06`（t33） | **证实（代码读 + 变异）** | 谓词 `needs_registry || child.is_none_or(empty)`、非 full 分支与 `replace_info` 共用；变异关掉守卫 → 作者钉子红（M4 之外的单条） |
| `LGC-LG-07①`（t33） | **证实** | 用不同数据自建两兄弟同名夹具（跨批 + 批内均拒、不同名仍成功）；变异让守卫永不命中 → 作者钉子与探针同时红（M4） |
| `LGC-LG-07②`（t33） | **证实** | 切口路径走 `Resolution::{One,Missing,Ambiguous}`；自己把 `resolve_node` 改回首匹配 → `resolution_tests::an_ambiguous_replacement_selector_is_refused` 红（M5b） |
| `LGC-LG-29`（t33） | **证实（代码读 + 变异）** | `visit_item_mod` 只跳 `cfg_is_test_only`，其余门控压栈并与条目门控合成；变异恢复"任意 cfg 整棵跳过" → 作者钉子红 |
| `t33 改写的既有夹具仍有牙`（t33） | **证实** | ① 堂兄弟歧义钉子：`resolve_node` 回首匹配 → 红；② 示例 `broken-button` 钉子：关掉 `core/src/registry_core/tree/transaction/transaction.rs:261` 的父级规则校验 → 红（M6b） |

- **变异反证（7 组）**：M1b `schema_matches` 恒真；M2 严格入口改委派有损入口；M3a tokens.rs 去掉深度；M3b parse.rs 去掉深度；M4 LG-07① 守卫永不命中；M5b `resolve_node` 回首先匹配；M6b 父级规则校验置空（`core/src/registry_core/tree/transaction/transaction.rs:261`）。M5/M6 首轮改错位置（`RegistrationRule::validate` 置空不影响示例；真正的求值点是 `core/src/registry_core/tree/transaction/transaction.rs:261` 的 `registry.header.registration_rule.validate(&snapshot)`）、M3 首轮只跑了 face 级目标——三处都重做对了，报告里写明
- **本份复核提出的新条目**：`N-1`、`N-2`、`N-3`、`N-4`（逐条见 §14.3）
- **未覆盖范围（如实列出）**：LG-27：只做源码读，未自建 `RuntimeCheckSpec` 夹具、未变异；LG-06 / LG-29：用"变异让作者钉子红 + 源码读"验证，未自建 overlay/语法夹具；t32 的"重命名金丝雀仍红"（`registration macro test_object! does not match parent panel`）未复跑；clippy 在写作窗口红（他人在飞的 run_method/conventions），不在 B4-core 交付面——这些是"没验到"，不是"验过是好的"。

#### t35 — 独立验证 B4-studio（9 条修复 + 6 钉子 + STU-S-06 结论，含变异反证）

- **装置与手段**：自建装置 /tmp/t35-verify（副本 + 自带 CARGO_TARGET_DIR）里的 8 条**行为**断言：真调 `open_editor_file`、真 `reload`/`poll_hot_reload`、真渲一帧数检视器 **12 行**、真写插件入口文件、真改夹具源码触发搜索重扫；先 8/8 绿，再 8 组变异逐条还原（`cmp` identical）后装置复绿
- **门禁窗口**：22:06:21–22:12:11 五条全绿（studio --all-features / workspace / clippy -D warnings / conventions / fmt）；首轮 21:56 与 22:07 的红全部归因并发在飞并重跑

| 判定对象 | 判定 | 怎么证的 |
| --- | --- | --- |
| `B4-studio 9 条修复` | **证实** | 8 条自建行为断言 + 8 组变异红侧；装置不复用作者任何钉子 |
| `作者 6 条钉子` | **证实** | 6/6 绿，但只作输入：其中 4 条是 `include_str!` 文本断言（强度上限见 N-6） |
| `检视器三方一致（计数 + 按键 + 渲染）` | **证实** | M4（计数 +2）与 M8（渲染器只画 10 行）各自翻红；**M8 只有渲染那条红**，计数那条仍绿 |
| `备忘按 (query, 源码戳) 失效` | **证实** | M5 忽略源码戳 → 红 |
| `入口闸门两个方向` | **证实** | M3 行判定→子串 → 红 |
| `retreat 与 advance 同体` | **证实** | M6 retreat 变 no-op → 红 |
| `writers 旧自称` | **证实** | M7 改大小写逃过一次 ⇒ 把否定针改成大小写无关后抓住 |
| `STU-S-06 的结论` | **证实** | 内核 `item_symbols` 已在（`core/src/registry_core/source/items.rs:56`，mtime 21:09）且 Studio 已改调、前缀词表 0 残留（`studio/studio/app/search_queries.rs:7` 与 `:122`，mtime 21:30）⇒ 作者"本轮不修"的前提在其交付时成立、在当前树上过期 |
| `studio 零新增公开符号` | **证实** | studio 0 处（只有 `pub(crate)`/`pub(super)`）⇒ 作者主张在本批范围内成立 |

- **变异反证（8 组）**：M1 `Err`→`Ok`（编辑失败）；M2 删 `mir_graph=None`（陈旧 MIR）；M3 行判定→子串（入口闸门）；M4 计数 +2（检视器计数）；M5 备忘忽略源码戳；M6 retreat 变 no-op；M7 writers 旧自称（改大小写逃过一次）；M8 渲染器只画 10 行。方法学注记：`cp -a` 保留 mtime，还原后必须 `touch` 否则 cargo 不重编
- **本份复核提出的新条目**：`N-5`、`N-6`、`N-7`（逐条见 §14.3）
- **未覆盖范围（如实列出）**：graft 失败 + 横幅组合不可达；性能收益未验；文本钉子大小写敏感；STU-C-04 拆模块那条未采纳——这些是"没验到"，不是"验过是好的"。

#### t41 — 独立验证两条高危（LGC-LG-05 嵌套守卫、LGC-LG-31 驻留无回收）+ 红侧复现

- **装置与手段**：全部装置在 /tmp/t41-v（`tar` 副本，`CARGO_TARGET_DIR=/tmp/t41-target`），每处变异逐次 `cmp` 还原；真检出零写入（四个 inScope 文件与开局快照逐字节相同）
- **门禁窗口**：22:51:55–22:53:44 五条全绿（另加 `--features authoring` 一条也绿）；22:23–22:48 conventions 连红 7 次以上，逐次归因都不是本任务，按队规作废重跑后转绿

| 判定对象 | 判定 | 怎么证的 |
| --- | --- | --- |
| `LGC-LG-05` | **证实** | 守卫已接（`edit("flow_provider", …)` → `validate_flow_provider`（`run_method/authoring/manifest/face/face.rs:327`，调用点 :147；t41 报 :308/:327，实测定义在 :327）→ 内核 `render_flow_provider` 的 `guard_nesting`，**LIMIT = 128**，且跑在显式 8 MiB 固定栈的专用线程上）；装置扫边界 **64/127/128 → Ok、129/300 → Err**，错误文本是内核自己那句 `input nests 129 levels of generic arguments, above the limit of 128` ⇒ **没有第二套阈值**；把拒绝用例从 **256 KiB 调用方栈**上调用仍是 `Err`（不 abort）、`depth=128` 实测 ok ⇒ 答案由守卫而非调用方栈决定 |
| `LGC-LG-31` | **证实** | `Box::leak` 仍在，但已在**进程级 `INTERNER`** 之后（去重 + `INTERN_LIMIT = 1 << 17` 上限 + 越界 `TraceArtifactError::VocabularyExhausted` 拒绝）；**绿**：`RSS 3380→14352 KiB`、`interned 0→131072`（=上限）、`refused 20000`，再喂 20 000 个新串 **RSS 增长 0 KiB** |

- **红侧复现（非作者自跑）**：
  - LG-05：副本里还原成修前的直接 `syn::parse_str` ⇒ 输入仅 **902 字节** 即 `thread '…zz_short_abort' has overflowed its stack` / `fatal runtime error: stack overflow, aborting` / `(signal: 6, SIGABRT)`——是 abort，不是 `Err`
  - LG-31：副本里删掉天花板分支 ⇒ `interned 0→151072`（越界）、`refused 0`、RSS 增长 12.7 MB 且仍在长，断言 `past the ceiling every new string is refused: 0` FAILED ⇒ 正是 finding 的"逐次喂大、单调增长"
- **本份复核提出的新条目**：`N-8`（逐条见 §14.3）
- **未覆盖范围（如实列出）**：这份复核没有独立报告文件（装置与红侧只落在任务 output 里），因此它是"任务级复核"而非"报告级复核"；两条钉子在 workspace 特性合并下确认跑绿，未单独复跑 CI 的 `--all-features` 作业本身——这些是"没验到"，不是"验过是好的"。

### 14.2 各实现单的结论与变异证据索引（6 单）

这些是**作者自述**的结论（不是独立复核），逐条带各自的变异/红侧证据索引，便于复核者按图索骥。

| 单 | 对象 | 状态 | 结论 |
| --- | --- | --- | --- |
| `t29` | B4-build_method | completed | 权威清单 7 条（全 MAJOR，不是 20 条）7 条全部核到真洞并修完，各配最小钉子，六条做了变异反证；身份红线未触碰 |
| `t33` | B4-core 接续 | completed | LGC-LG-06 / LG-07①② / LG-29 三条 MAJOR 修完并各配夹具钉子、先红后绿；`render_requirements` 剩余半按队长裁定 (b) 记为**范围外未修** |
| `t36` | B4-studio 第 10 条：内核 `item_symbols` + Studio 不再自猜符号词表 | failed（交付完成，唯一失败原因是门禁被他人 in-flight 顶红） | 内核唯一判定新增 `SourceItem`/`item_symbols`，Studio 零处字面词表（`strip_prefix("` = 0） |
| `t38` | B4-conventions 4 条 | failed（实现与复核都完成；唯一红＝环境用例） | 3 条真洞已修（LG-24 `conventions/src/lint.rs:225` 的 `#![deny(warnings)]` 特例、LG-26 新增 `release_action_pin` 门禁、LG-53 `doc_blocks` 的记录目录豁免）+ LG-25 已由 t4 的 `GuardShape` 修掉（照实报账） |
| `t43` | B4-run_method 拆分 C：无边界迁移替换 + linked collector + 字段集合真值 + render_requirements 严格化 | completed | 四条全部落地，每条先红后绿 + 变异反证；前提：四个 inScope 文件里已有 t28 崩溃前落盘的改动 |
| `t44` | B4-plugin-host：LGC-LG-32（完整帧后不退出时答案被丢成 Timeout） | completed | 完整帧一到就交付答案；Timeout 当且仅当到期前没有完整帧；帧到达时仍在运行的子进程被杀掉并回收 |

#### t29 — B4-build_method

| 条目 | 做了什么 | 钉子 / 红侧 |
| --- | --- | --- |
| `SUR-C2` | `collect_rust_sources` 两行文档移回函数上方（纯文档） | 无行为钉子 |
| `LGC-LG-08` | `logical_path` 先递归解父面路径再拼自己 + 环守卫 | `a_parent_that_sorts_later_still_forms_the_full_path`（旧 `root/alpha` → 新 `root/zeta/alpha`） |
| `LGC-LG-09` | 载荷先写、指纹最后且仅当无编译/写入错误；删指纹失败上报；`write_if_changed` 改唯一同级文件 + rename | `a_payload_that_cannot_be_written_publishes_no_fingerprint`（旧行为会留下新指纹） |
| `LGC-LG-11` | 生产半修：新增 `face_views_and_unreadable`，不可读集合 = 安放不了的注册面文件 + 节点文件语法失败 | `a_registration_file_that_does_not_parse_is_named_not_dropped`（旧：`unreadable` 为空、无处可查） |

- **变异证据索引**：六条做了变异反证（改回旧行为即红，逐处还原）
- **新增公开面**：`face_views_and_unreadable`（见 §14.6）

#### t33 — B4-core 接续

| 条目 | 做了什么 | 钉子 / 红侧 |
| --- | --- | --- |
| `LGC-LG-06` | 共享谓词 `child_registry_can_survive`（`core/src/registry_core/tree/graft_ops/overlay.rs:374`），非 full 分支与就地路径同规则 | 副本把检查改成 `&& false && …` → `core/tests/b4_core_continuation.rs:134` 拒绝那条 FAILED |
| `LGC-LG-07①` | `plan_batch` 槽位名查重（`core/src/registry_core/tree/transaction/transaction.rs:229`，消息 `duplicate sibling registry name`），跨批与批内都拒 | 守卫改成 `false && …` → 钉子 FAILED（:196） |
| `LGC-LG-07②` | 切口选择器走 `resolve_path_strict -> Resolution::{One,Missing,Ambiguous}`；`resolve_path` 只剩 `resolve_record` 一个调用方 | 变异 A `resolve_target` 改回首匹配 → 切口钉子 FAILED（:259）；变异 B `resolve_node` 改首匹配 → 歧义钉子 FAILED（`core/src/registry_core/tree/graft_ops/resolution_tests.rs:51`） |
| `LGC-LG-29` | `visit_item_mod` 只跳 `cfg_is_test_only`，其余门控压栈并与条目门控合成 | 变异 E 恢复"任意 cfg 整棵跳过" → `must not be dropped: [] left: 0 right: 1` |

- **变异证据索引**：A/B/E 三组 + 既有夹具的两处留牙证据（堂兄弟歧义钉子、示例 `broken-button` 钉子）
- **纠错与旁证**：自己造成的锚点漂移（`core/src/registry_core/tree/transaction/transaction.rs:167`）被 `doc_anchors` 抓到并已修；对抗式复核纠错：studio `call_tree` 两条红的根因是他人对 `SourceLocation.function` 的在飞改动（＝t34 的 N-4），不是同名碰撞
- **门禁窗口**：21:44:21–21:47:08 一次连测全绿（core / core+syntax / conventions 117 / fmt / clippy / workspace，另加 studio --all-features 与 run_method 两条）

#### t36 — B4-studio 第 10 条：内核 `item_symbols` + Studio 不再自猜符号词表

| 条目 | 做了什么 | 钉子 / 红侧 |
| --- | --- | --- |
| `STU-S-06` | 内核 `item_symbols`（`core/src/registry_core/source/items.rs:56`，由 `source.rs` 以 `#[path] mod items; pub use items::*;` 挂载）；Studio `studio/studio/app/search_queries.rs` 合并为一个 `source_rows_for_text(...)` | Studio 四条钉子（旧词表）→ `left: [] right: ["measure"]` / `["render"]` / `let` 误报 / `["Panel"]`；内核红侧 `error[E0432]: unresolved import xirang::source::item_symbols` |

- **门禁红（归因到他人）**：T36-B1 `studio/studio/app/tests/call_tree.rs:166/:223`（根因 `registered_functions` < 2，对照实验：换回旧词表仍同样失败 ⇒ 不在本任务）；T36-B2 `examples/control-button/tests/registry.rs:144`（注册规则/校验一侧的在飞改动）
- **新增公开面**：`item_symbols`、`SourceItem`（见 §14.6）

#### t38 — B4-conventions 4 条

| 条目 | 做了什么 | 钉子 / 红侧 |
| --- | --- | --- |
| `LGC-LG-24` | 删掉 `#![deny(warnings)]` 特例并改写模块文档首段；前提实测：`#![deny(warnings)]` + 未文档化 `pub fn` ⇒ exit 0 零输出，而 `missing_docs` 默认 allow 不在 `warnings` 组 | `lint::tests::the_warnings_group_is_not_the_missing_docs_lint ... FAILED` |
| `LGC-LG-25` | **已被 t4 的 `GuardShape` + `positive_tests` 修掉**（否定写法整类不再放行；现码 `is_positive_test` 只接受 tag 判断本身与 `path == 'literal'`） | 副本改回 `contains("!startsWith")|contains("!=")|contains("||")` ⇒ `shapes_off_the_whitelist_are_reported ... FAILED` |
| `LGC-LG-26` | 新增 `conventions/src/release_action_pin.rs`（`#[path]`+`#[cfg(test)]`）：按 `jobs:` 切 job，只有非注释行点名 `CARGO_REGISTRY_TOKEN` 的 job 被约束，其内远程 `uses:` 的 ref 必须 40 位十六进制 | 把门禁改空 ⇒ `a_token_holding_job_must_pin_its_actions FAILED (left 0, right 1)` 与 `the_real_workflow_turns_red_when_its_pin_becomes_a_tag FAILED` |
| `LGC-LG-53` | `doc_blocks` 的 `is_record(root, path)` 改为对 root 之下每个路径分量做前缀匹配 | `doc_blocks::doc_blocks_tests::a_report_inside_a_record_directory_is_exempt ... FAILED` |

- **复核状态**：**这三条真洞没有非作者独立复核**（t38 是作者）；本轮账目把它记为覆盖面缺口（见 COV-2）。

#### t43 — B4-run_method 拆分 C：无边界迁移替换 + linked collector + 字段集合真值 + render_requirements 严格化

| 条目 | 做了什么 | 钉子 / 红侧 |
| --- | --- | --- |
| `LGC-LG-10` | `replace_source_prefix`（左界=前一字节不是路径字节）替换两处无界 `.replace`；审计说的右侧越界**不成立**（`src/control/` 要求 `control` 后紧跟 `/`），真正无界的是左侧 | `a_containing_path_is_not_the_moved_source_prefix`（`dep/src/widget/x.rs`）；左界检查改成 `true` → 钉子红 |
| `LGC-LG-14` | `collector: linked` 是 `external_object!` 的**默认**注入（全仓 36 处引用）⇒ 拒绝它会让每个外部面编译失败；最终两条相邻空臂 + 共用注释 + doctest 钉子 | 第一稿的 `(development | linked; $registration:ident)` **不是合法 macro_rules 交替**（`|` 是字面 token），新加的 doctest 钉子先抓住：`collector must be one of development, debug, linked; got linked` |
| `LGC-LG-38` | `RENDERED_FIELDS`（22 个落盘字段）+ `NON_FIELD_KEYS` 各只陈述一次；`render_source` 开头拒绝第三种键；往返钉子的样本表改成切片并断言与 `RENDERED_FIELDS` 相等 | 样本类型 `[(…); 22]` → 切片；`= render_requirements` 1→0 |
| `render_requirements 严格化` | `try_render_requirements -> Result<String, FaceParseError>`；旧签名保留为**有损薄包装** + 文档写明有损并指向严格兄弟；调用方 `render.rs` 已迁移（旧调用 0 处） | `a_malformed_requires_entry_refuses_the_rewrite` 走真实 `render_source`：畸形条目必须 Err 并点名 `capability=>provider`；变异（调用方改回有损入口）→ 立刻红 |

- **门禁窗口**：五条全 exit 0：run_method 58/0（带 feature 60 单测）、core 196/0、workspace **878/0**、clippy 无输出、fmt 无 Diff
- **新增公开面**：`try_render_requirements`（见 §14.6）

#### t44 — B4-plugin-host：LGC-LG-32（完整帧后不退出时答案被丢成 Timeout）

| 条目 | 做了什么 | 钉子 / 红侧 |
| --- | --- | --- |
| `LGC-LG-32` | `process.rs` 的轮询循环以帧结束（`:430-438`），交付在 `:448-450`；文档（`:285-303` 第 3 条 + `plugin-host/src/process/child.rs:119-129`）改成与实现同一 | 修前：`a delivered answer must not decay into Timeout, got Timeout`（0.60 s 撞 deadline）；变异 A 删掉帧分支 → 同一条红；变异 B deadline 交付手里东西 → `no frame by the deadline is a Timeout, got Ok([])` |

- **门禁窗口**：22:51:05 五条一次连测全绿；窗口内队友在飞文件多次致红，归因后重跑

### 14.3 新条目（8 条：4 已关闭 / 4 未修）

| 条目 | 严重度 | 来源 | 位置 | 是什么 | 状态 |
| --- | --- | --- | --- | --- | --- |
| `N-1` | low | t34 | `core/src/registry_core/tree/transaction/transaction.rs:186/:195` | t37 收口的 :107/:117/:126 已改 `registry_name`，但 t33 新增的同段注释里**又写了一次** `*slot name*`/`**槽位名**`；该文件所有 `slot` 命中都是同一概念，未误动插件槽/布局槽 | 已由 t46 关闭（t34 报告于 22:12，t46 于其后收口；现值：该文件两词命中 0） |
| `N-2` | medium | t34 | `core/src/registry_core/authoring/parse/rules.rs:181` | t33 把**未改**的 `render_requirements` 标成 passed——平台逼出的假绿（见 `false_green`） | 以事实为准：它至今仍是 `pub fn -> String` 有损（已发布的 0.1.x 线约束）；t43 之后它带文档写明有损、有严格兄弟 `try_render_requirements`、调用方已迁移 |
| `N-3` | low | t34 | `CHANGELOG.md / docs/roadmap-1.0.md` | 自 HEAD 起的新增公开入口没有一处写进 CHANGELOG/roadmap（t34 复核时实测 `try_parse_requirements_owned` 命中 0；本轮把四个入口一起逐条核实，命中同样全为 0） | 已由 t52 本轮补记（见 `new_public_surface` 与 `doc_updates`） |
| `N-4` | medium | t34（队长点名必核） | `core/src/registry_core/authoring/snapshot/snapshot.rs:134` | 用原始 `value("handle")` 填 `source.function`，而同函数已把缺省 handle 派生成 `kind` ⇒ 同一类"重复真值"缺陷；影响面：作者侧缺 `handle` 键时全部面 `function=""` → studio `registered_functions()` 空 → call_tree 两条红 | 已由 t46 关闭（`handle: handle.clone()` 让同一值供两处；两侧钉子：`the_handle_field_and_the_source_function_are_one_value`、`both_spellings_of_the_input_keep_the_function_name`；红侧 core 钉子 `left: "" right: "Widget"` + studio call_tree 2 failed） |
| `N-5` | medium | t35 | `STU-S-06 的结论（Studio 自猜符号词表）` | 作者"需内核入口、本轮不修"的前提在其交付时成立，但在当前树上**过期**：内核 `item_symbols` 已在（`core/src/registry_core/source/items.rs:56`，mtime 21:09）、Studio 已改调它、前缀词表 0 残留（`studio/studio/app/search_queries.rs:7,122`，mtime 21:30，晚于作者 20:44） | 已由 t36 关闭（**不需要再开单**）——账目按"不采信任何过期结论"改口径 |
| `N-6` | medium | t35 | `studio 的作者钉子里 4/6 条` | 4/6 条是 `include_str!` **文本断言**：改大小写/词序的重写能绕过（M7 就是这样逃过一次，把否定针改成大小写无关后才抓住） | 未修（建议）；装置可搬进仓 |
| `N-7` | medium | t35 | `studio 测试面` | 仓里目前**没有**"渲染出 12 行"这条**行为**钉子；M8（渲染器只画 10 行）证明只有它会抓住"渲染器单独截断"——三方一致里被作者漏掉的第三条消费者 | 未修（建议留下这条钉子） |
| `N-8` | medium | t41（覆盖面缺陷，队长点名） | `toolchain/src/runtime/src/lib.rs`（原 `run_method/lib.rs:17,52`；合并后 crate 改名，非默认 `authoring` 特性） | LGC-LG-05 的修复与两条钉子都在非默认特性之后 ⇒ 标准 verify 里 `cargo test -p xirang-toolchain --offline` **跑不到它们**（实测 `manifest::face` 默认 **0** 条、加 `--features authoring` **8** 条） | **已关闭（2026-09-29 判据落地）** ✓：该口径已成文并进仓库文档——`AGENTS.md` §Verify 的「单个 crate 也可能被验证出一个假绿」整段写明 `0 passed` 读作"什么都没跑"，给出该模块加/不加 `--features authoring` 的前后实测，并写明"单独验证一个 crate 时请点名特性"。账本口径＝**该条目的验证命令必须带 `--features authoring`**；`cargo test --workspace`（特性合并）与 CI 的 `--all-features` 覆盖得到（t41 已在 workspace 日志里确认两条钉子跑绿） |

- 关闭的条目**不改**复核报告本身（`audit-verify-*.md` 是既有产物，只可引用）：`N-1`/`N-4` 由 t46 关闭、`N-5` 由 t36 关闭、`N-3` 由本节所属的 t52 补记；`N-2` 不是关闭项（它是"已发布的兼容约束 + 文档 + 严格兄弟"，见 §14.4）。

### 14.4 平台逼迫的假绿（单列一节）

平台的完成语义：completed 任务**不接受** failed 验收项（结构化 payload 里任何一项 failed 都会被拒），而 verify 命令非零又要求记 failed。两条压力合起来，就出现两种与事实相反的账目：① 交付确实没做/没改的那一项被写成 passed（假绿）；② 交付确实完成、但某条命令被环境或他人在飞文件顶红，任务只能记 failed（假红）。

#### 假绿 FG-1：`render_requirements` 剩余半（LGC-LG-28）（任务 `t33`）

- **结构化 payload 里写的是**：`passed`
- **事实**：该函数当时**未改**，至今仍是 `pub fn render_requirements(value: &str) -> String` 有损（`filter_map` 静默丢弃畸形条目）
- **该任务正文自己写的是**：t33 的正文自己写的是「`render_requirements` 剩余半：范围外未修（按裁定 (b)）」
- **证据**：
  - `core/src/registry_core/authoring/parse/rules.rs:177-181` 的文档逐句写明"本入口是有损的，而且因为它是已发布的 API 而保持这样"
  - 严格兄弟 `try_render_requirements`（同文件 :203，t43 落地）
  - 调用方 `run_method/authoring/manifest/face/render.rs` 已迁移到严格入口（旧调用 0 处）
- **谁报的**：t34 N-2、t30（记账时已把"结构化 payload 的假绿"记为待办）
- **账本口径（以事实为准）**：以事实为准：**仍是 `-> String` 有损**；t43 之后它带文档写明有损、有严格兄弟、且调用方不再用它。账目上这条不再当作"已修"，而是"已发布的兼容约束 + 文档 + 严格兄弟"。

**镜像条（假红）**：反向的同一压力（假红）：交付完成而 verify 非零 ⇒ 按机制记 failed。本轮至少这些任务在 output 里写明"交付完成、唯一红是环境/他人"：t38（唯一红＝环境用例）、t58（41 套件绿、唯一红＝已声明的环境用例）、t36（两处红已用对照实验归因他人）、t55（范围全绿、两条被他人顶红）、t32（修复完成、core 整包被在飞编辑阻断）、t49/t50/t51（未做修改的只读核实）。**账本口径**：这类 failed 不能当成交付失败读，按各自 findings 与上面这条假绿一起处理。

### 14.5 不可离线判定的环境用例（1 例）

#### ENV-1：`studio::app::tests::new_project::new_project_and_explicit_root_face_compile`

- **所属命令**：`cargo test --workspace --offline`　**账本判定**：**不可离线判定 / 环境相关**
- **为什么**：该用例生成一个宿主工程并跑 `cargo check --offline`，离线解析 `xirang-run-method ^0.1.6` 依赖本机 `CARGO_HOME` 的 git 缓存与版本线状态（缓存里只有 0.1.5 时会 `failed to select a version`），与代码交付无关
- **实测**：
  - t38 的窗口：它是工作区**唯一**失败，三条对照证明与本任务无关（无 `.git` 的隔离副本同一条命令 EXIT=0、20:32 的 `/tmp/land1-g2.log:1244`（t28 开工前）已是同一失败、单跑该测试也稳定红）
  - t58 的窗口：同一失败，41 套件绿、唯一红即它
  - **t52 记账时（23:04）实测同一条命令单跑该用例 ⇒ `ok`（13.31 s）** ⇒ 它随环境变，因此既不能用它的红判任何 findings，也不能用它的绿当证据
- **归因核对**：按任务输出逐条核对：明确把失败归因到该用例的是 **t38、t58** 两条；任务书里写的"本轮至少 5 个任务因它被判 failed"与任务输出不符——其余失败任务的失败原因分别是他人 in-flight 文件把门禁顶红（t25/t27/t36/t49/t55）、core 整包不编译（t32）、预算耗尽（t50/t51）、工具输出畸形（t28/t40）、锚点/棘轮外部红（t46），各自 output 已写明。账本按可核实的口径记 2 条，并把"≥5"标为与任务输出不符
- **账本口径**：账本口径：该用例出现在任何 verify 列表里都按"环境相关、不可离线判定"读——它的红不为任何 finding 记一笔，它的绿也不作为交付证据。

### 14.6 新增公开面（4 组 / 5 个名字）与版本线、隔离打包的含义

| 入口 | crate | 任务 | 位置 | 形态 | 做什么 |
| --- | --- | --- | --- | --- | --- |
| `try_render_requirements` | `xirang-core` | `t43` | `core/src/registry_core/authoring/parse/rules.rs:203` | `pub fn try_render_requirements(value: &str) -> Result<String, FaceParseError>` | 重写 `requires` 列表时**拒绝**畸形条目（复用 t28 的逐条规则）；旧 `render_requirements` 保留为有损薄包装，文档写明有损并指向它，调用方已迁移 |
| `face_views_and_unreadable` | `xirang-build-method` | `t29` | `build_method/face_view.rs:168` | `pub fn face_views_and_unreadable(root, package) -> (Vec<FaceView>, Vec<String>)` | 把"安放不了的注册面文件 + 节点文件语法失败"从静默丢弃变成可查的集合；`face_views` 委托它，文档写明 `parent_resolved == false` 那一半 |
| `item_symbols / SourceItem` | `xirang-core` | `t36` | `core/src/registry_core/source/items.rs:56 / :21` | `pub fn item_symbols(source: &str) -> Vec<SourceItem>；pub struct SourceItem { name, signature, line, is_function }` | 内核持有声明词表（`INTRODUCERS`，`let` 刻意不在表内），Studio 不再自猜（`strip_prefix("` 0 处） |
| `try_parse_requirements_owned` | `xirang-core` | `t32` | `core/src/registry_core/authoring/parse/rules.rs:255` | `pub fn try_parse_requirements_owned(...) -> Result<…, FaceParseError>` | 已发布签名 `parse_requirements_owned(&str) -> Vec<OwnedRequirementSpec>` 逐字保住（有损），严格判断由这个兄弟入口承载 |

- **补记前的核实**：4 组入口在本轮补记前对 `CHANGELOG.md` 与 `docs/roadmap-1.0.md` 的 grep 命中**全为 0**（逐条列在上表的 `task` 来源里：t43 / t29 / t36 / t32）——因此是"补缺"，不是重复记。
- **版本线**：这四个入口都随**未发布**的 `0.1.6` 走：工作区版本仍是 `0.1.6`（各 `Cargo.toml`），无需额外抬版本；`tools/xirang-publish --check-table` 不受影响（它只按清单比较依赖表）。
- **隔离打包**：`tools/xirang-package-audit` 的**内容**半边对这些入口成立（t36 实测 `cargo package -p xirang-core --list --offline` 列出 `src/registry_core/source/items.rs`）；**隔离打包**半边要等这些 crate 的带版本号 `xirang-*` 依赖进 index，未发布期间 warn + 跳过（**skip ≠ 失败**）。这批新增把等待态**加深**了一层：Studio 现在调用未发布 0.1.6 的内核符号。发布顺序仍是 core 在前（core → … → run_method → studio），发布后重跑 `--verify-consumers`。
- **口径**：新增跨 crate 公开符号不改版本线；把它们写进 `[0.1.6]` 的 Added 段是**文书义务**，不是发布动作。

### 14.7 覆盖面缺口（4 条）

- **COV-1**（`LGC-LG-05`，验证命令口径）：该条目的验证命令**必须带 `--features authoring`**（修复与两条钉子都在非默认特性之后；`run_method/lib.rs:17,52`）——与既有教训"非默认特性下裸跑是 0 passed、看起来像绿"并列。`AGENTS.md` 不在 t52 的 inScope（已回队长，另派一行小单）
- **COV-2**（`LGC-LG-24 / LGC-LG-26 / LGC-LG-53`，t38 的实现）：三条真洞已修但**没有非作者独立复核**（t38 是作者）；本轮账目把它记为覆盖面缺口，建议单开一条复核
- **COV-3**（`LGC-LG-27`，t34 的覆盖）：只做了源码读（合并规则），未自建 `RuntimeCheckSpec` 夹具、未变异 ⇒ 它是"部分证实"的唯一来源
- **COV-4**（`STU-S-06 的行为面`，测试面）：缺"渲染出 12 行"这条行为钉子（见 N-7）；t35 的装置可搬进仓

### 14.8 本节涉及的文书改动与复算方式

- **`CHANGELOG.md`**：`CHANGELOG.md` 的 `[0.1.6] — unreleased` 段（EN Added + ZH 新增）逐条补记四个入口；补记前逐条 grep 命中为 0（四个符号在 CHANGELOG 与 roadmap 里都没有出现过），因此**没有**重复记。
- **`docs/roadmap-1.0.md`**：`docs/roadmap-1.0.md` 新增「B4 完成情况（第三轮：新增公开面与版本线）」一节：四个入口的"谁提供 / 谁消费 / 钉在哪"表 + 版本线与隔离打包的含义。
- **账本自身的机制**：冻结正文锚点：§0–§12 的坐标是**记录**。第三轮把 studio 的 `app/support.rs`（459→14 行）与 `app/navigation.rs`（→88 行）拆分、把 `app/writers.rs` 等移走后，冻结正文里的 8 处引用不再能解析。账本的处理是**逐条登记**（`meta.frozen_record_anchors`：8 条，各带 `kind` 与现算的 `file_lines_now`），另有条目自身位置的漂移挂在该条的 `location_note` 上（t30 的既定机制；现共 13 条带该字段）；两类都只豁免"那一种精确拼写"，且 `check_report.py` 每次重新验证它**确实**解析不了——文件长回来（或重新出现）就自动重新开始检查。
- **账本自身的机制**：引用刷新：第三轮的编辑把 batch-2 新公开符号 `compact_registration_rule` 的定义行从 :82 移到 :78，账本按现树重新量了一次并登记在 `fix_batch_2.references_refreshed`（引用行号不是结论字段；结论一字未改）。
- **账本自身的机制**：锚点实测更正：§14 引用复核报告时按现树逐条复量，三处与报告原文不同并已注明——`core/src/registry_core/tree/transaction/transaction.rs` 的父级规则求值点是 :261（t34 报告写 :260）、`validate_flow_provider` 定义在 `run_method/authoring/manifest/face/face.rs:327`（t41 报告写 :308/:327）、`duplicate sibling registry name` 在 `core/src/registry_core/tree/transaction/transaction.rs:229`（t33 报告写 :228）。
- **复算**：本节全部数字来自 `fix_batch_3` 的清单（复核 3 份、实现单 6 单、新条目 8 条、假绿 1 例、环境用例 1 例、公开面 4 组、覆盖面注记 4 条、变异组 15 组 + 红侧复现 2 处），由 `/tmp/xirang-audit-logs/patch_fix_batch_3.py` 落盘（在 `patch_fix_batch_2.py` 之后跑；`build_findings.py` 再生成后两个脚本都要重跑），再由 `gen_report.py` / `gen_html.py` 现算重出。HTML 角标（`audit-structure-map.html`）同源。

## 15. 末轮记账（t60–t89：LG-32 / LG-47 / K-10 / SUR-S11 / B6-studio 与八份复核）

**本节是账，不是新结论。** §0–§14 的结论一字未改；本节把 §14 之后的全部轮次折进来，每个数字都由 `audit-findings.json` 的 `fix_batch_4` 块现算（`check_report.py` 逐条复算）。本轮同时收掉 §14.7 记下的两条覆盖面缺口（COV-1 / COV-2）与两个 partial（LG-27 / LG-47），并刷新两处已经漂掉的引用坐标。

### 15.1 末轮轮次（30 条：21 completed / 9 failed-但交付完成）

| 单 | 状态 | 对象 | 做了什么 | 钉子 / 判别性证据 | 复核 |
| --- | --- | --- | --- | --- | --- |
| `t60` | completed | LG-32 第二半（不完整帧 + status 0 的错误要带上下文） | `read_frame` 两个孪生读取点（4 字节长度前缀 / 负载）各自把读取器错误包成带操作名与帧形状的 `HostError::Process`；`process.rs` 把 `operation` 带进读取线程；`Limit` 早退与“非零退出优先用 stderr”语义未动。 | 红侧原文 `got Io(Error { kind: UnexpectedEof, message: "failed to fill whole buffer" })`（裸错误）；绿侧逐字 `plugin process failed: … got no complete frame for `run`: … (failed to fill whole buffer; kind UnexpectedEof)`；变异 A 改回裸 `?` / B 保留变体但消息退成裸文本，两处都让钉子红；`md5sum` 还原一致。 | t68（`audit-verify-plugin-host.md`：LG-32 **证实**）＋ t77（`audit-verify-plugin-host-b.md`：**通过** + 一条边界） |
| `t61` | failed（交付完成；注释轴无行为变更） | B5-core 8 条 + t56 移交的裸词收口 | 八条全 MINOR：模块文档头 `//`→`//!`（C-05）、`# Panics` 只留真 panic 契约（C-06）、重复模块文档合并（C-07a）、屏蔽文本口径（C-07b）等；另收口 t56 移交的 6 处裸词。 | 每条给 HEAD→当前树的**成对判别性计数**（如 `grep -c` 1→0 / 0→1、`# Panics` 2→1）；纯注释/词形改动，无行为变更。 | （无独立复核；注释轴按 t61 自己的成对 grep 记证据） |
| `t62` | completed | LG-47 真洞修复（+ LG-36/LG-49 已被修掉，净改动 0） | `slot_state` 的 `pending` `MutexGuard` 与函数体同寿命 ⇒ `backend.load`（编译+实例化+健康检查）在**持锁期间**跑；改为交接后即释放锁、由发布路径在写队列同一把锁下发布。 | LG-36/LG-49 用判别性 grep 给红侧复测（净改动 0）；LG-47 新钉 `a_generation_queued_during_a_load_stays_pending`，红侧（发布改回盲目清零）`plugin-host/src/lazy_wasm/slot_state_tests.rs:143` 的 `a generation queued during the load is still pending`。 | → t68（部分证实）→ t79/t86（证实，见下） |
| `t73` | completed | LG-47 可测化（可注入 load 接缝 + 真正测时序的钉子） | 新增私有 `activate_with(impl FnOnce(...))` 接缝，`activate(backend)` 变成 `self.activate_with(|a| backend.load(a))`；内联测试按仓库惯例搬进 `slot_state_tests.rs`（297→184 行）。 | **交付的是装置不是自证**：两条真正测时序的钉子 + 红侧原文（持锁旧形态 ⇒ `test result: FAILED. 1 passed; 2 failed`）；还原 `cmp: identical`（sha256 前缀 `4070277036e49aa1`）。 | → t79（用它补齐时序证实）→ t83（加固）→ t86（复核更紧） |
| `t63` | completed | **COV-1 关闭**：AGENTS.md 的 `0 passed` 假绿陷阱 | 在 `## Verify` 之后加一段中英双语：非默认特性下的模块裸跑 `cargo test -p <crate>` 会报 `0 passed`（看起来像通过）；给正确形态与判读纪律。 | 实测 `manifest::face` 裸跑 `0 passed; 0 failed` / 加 `--features authoring` `8 passed`；`core` 同形（`syntax`）。文档里的数字全部本单现测；未新增 `path:line` 锚点。 | → t87（数字漂移面收口，见下） |
| `t64` | completed | B5-build_method 3 条（SUR-C8 / SUR-S11 / SUR-C9） | `SUR-S11` 真洞→修：规则读取改为**注释感知**（跳过 `//`、`/* */`、字符串与原始字符串字面量，遍历所有出现处）、嵌套括号配平（`call_end`）、转义解码（`string_values`）；API 与顺序不变。 | 红侧原文 `left: ["ghost.export"]` vs `right: ["control.render","control.preview","control.extra"]`（注释里的示例被读成唯一需求）；变异 (a) 换 `match_indices`、(b) 旁路解码 ⇒ 各自红；SUR-C8 **已被修掉**（原子写，净改动 0）、SUR-C9 **非缺陷**（审计自标 partial，其正例仍在：`grep -c "shipping a binary whose graft"` = 1）。 | t74（`audit-verify-build-method-b5.md`：SUR-S11 **证实**，出厂 3 份规则源**未过火**） |
| `t65` | completed | B6-studio 代码半 4 条（S-17 / S-19 / S-26 / S-14） | `TreeStep::from_key` 显式四方向拒绝（S-17）；抽 `open_edit_form` 消掉递归合成按键（S-19）；拆 `source_stamp`（S-26）；`writers`→`write_guard` 且 watcher 名单与 `RELEVANT_FILE_NAMES` 同五名（S-14）。 | 每条有受控变异红侧原文（如 S-17 `Char('j') must be refused instead of read as `Right``；恢复后 4 passed）；S-09 同集落地并写明“镜像不是 import”，钉子逐名双向断言。 | （并入 t67/t76/t81/t89 的 B6-studio 10/10 计数，见下） |
| `t67` | failed（4/5 完成） | B6-studio 重构第一批（LG-34 / S-22 / S-20 / S-21 完成；S-18 未动） | `keyboard::declaration_contract_fields` 把“读失败”与“无契约”分开（LG-34）；夹具/状态机/布局各自合一（S-22/S-20/S-21）。 | 变异回旧 `read_to_string(…).ok()` ⇒ `studio/studio/app/tests/forms.rs:134` 的 `a read failure must be reported, not rendered as `no contracts``；S-18 给出 54 处赋值清单 + 25 处会判红、7 处多臂，列为 T67-F1。 | → t76 收口 S-18 |
| `t76` | completed | STU-S-18：事件严重度显式化 | `App.event_is_alert: bool` + 唯一两个写入口 `note()`/`alert()`（54 处赋值 → 63 个调用点，7 处多臂逐臂决策）；`draw_event` 只读字段。 | 不变量钉子递归扫 `src/**/*.rs`：生产代码裸赋值只剩 2 处（两个写入口体内），注入一处裸赋值 ⇒ `raw assignments outside `note`/`alert`: ["…/keyboard.rs: 1"]`；叶子测试名 130*（逐名核对）。 | （B6-studio 10/10 的一部分） |
| `t81` | failed（4/5 完成） | B6-studio 第二批（S-28 / S-29 / S-25 / S-27 完成；S-04 留给 t89） | `Paste` 走与键入同一条分派器（S-28）；读侧守卫 `selected_read_root()`/`with_selected_project_read()`（S-29）；metadata 目标判定（S-25）；`after_project_write`（S-27）。 | 每条有受控变异红侧原文（如 S-28 吞掉粘贴 ⇒ `assertion left == right failed / left: ""`）；S-04 实测面已清点（跨 7 个文件）并写成 T81-F1 留给单独一单。 | → t89 收口 S-04 |
| `t89` | completed | STU-S-04（B6-studio 最后一条）：渲染期改为只读 | 新增 `ui/render_cache.rs`（`RenderCache::take/apply`），`ui::draw_once` 每帧一次性应用回写；渲染路径不再写 `App`。 | attempt 2 未再改代码，五条门禁重跑确认无漂移；`changedPaths` 按补齐后的 inScope 列 12 条。 | **B6-studio 10/10 收口**（t65 4 + t67 4 + t76 1 + t81 4 + t89 1，去重后 10 条） |
| `t66` | failed（交付完成；保留旧签名 + 加严格兄弟） | K-10：规则读取把注释当真 + 初始器外静默降级 | 新增严格入口 `try_rule_syntax_from_text`（core 公开），已发布的宽容 `rule_syntax_from_text` 签名与行为一字不动、文档改写为“宽容/有损 + 指向严格兄弟”。 | 红侧实测：注释里的伪子句 ⇒ `"exports:wrong"`（真规则丢失）、常量点名清单 ⇒ `"ANY"`（静默降级）；一条**如实更正**（findings 说的“初始器外首个 `=`”实测读对了，故未拿它当红侧）；7 条钉子，其中走文件系统的那条按 purity 门禁搬到 `core/tests/`。 | t75（`audit-verify-k10-family.md`：**证实**）＋ t72（生产调用点） |
| `t72` | failed（交付完成） | K-10 收尾：生产调用点切到严格读法 | `rule_syntax_for_source` 改经 `try_rule_syntax_from_text` 读取；候选顺序不变；**文件缺失仍 `Ok("ANY")`**（缺席不是畸形规则，另有专钉）；错误文本同时含面文件名与规则文件名。 | 生产入口红侧：`PROBE commented: Ok("exports:wrong")`、`PROBE const-list: Ok("ANY")`；修后形状② ⇒ `Err` 且消息带两处文件；3 条钉子。已发布面（`rule_syntax_from_text` / `rule_syntax_for_source` 签名）未动 ⇒ 发布表不动。 | t75 独立复跑生产入口（证实） |
| `t68` | completed | 独立验证 plugin-host 两轮（t44 的 LG-32 + t62 的 LG-47） | 非作者自建观测：单跑交付钉子 0.06 s 结束（帧到即交付，不等 deadline）；过修方向仍 ok；跑前/跑后 `ps` 扫 `sleep 30` = 0/0（**不留进程**证实）。 | 3 处变异（M1 删“帧到即结束”⇒ Timeout 红；M2 `publish_pending` 回盲目清零 ⇒ 排队钉子红；M3 过修 ⇒ 到期交付手头帧红）。 | 判定 **LG-32 证实 / LG-47 部分证实**（并发时序未独立测量）⇒ 报告 `audit-verify-plugin-host.md` |
| `t69` | completed | 独立验证 mcp 三轮（t53 / t54 / t57） | 五条验收子项全部证实：`LG-23` 截断是**说出来的**（200 KiB 单行 ⇒ 逐字含 `at most 200 diff lines or 65536 bytes`）；`LG-20` 临时目录不可预测（8 次调用 8 个名字）；`LG-43` 契约（60 同名定义 / `limit` 夹到 50 / `limit: 0` ⇒ 1 条）；`search` 的 `unparsable faces 2` 不被命中守卫吃掉；`LG-51` 归属先于存在性（五种形态整个 JSON 回复逐字节相同）。 | 8 条自建装置 + **5 处变异**各自让对应装置变红、逐次 `cmp`/sha256 还原零残留；被验 8 个 mcp 文件前后哈希不变。 | 报告 `audit-verify-mcp-batch.md`；→ t70 补三处文档 |
| `t70` | completed | t69 三处文档补充 | `unparsable faces N` 不受 `limit` 约束（读作结果行会让调用方误以为拿到完整答案）、`is_absolute` 的平台差异、临时目录 8 次候选契约——都写进代码处的中英双语文档。 | 只动注释/文档，未改任何逻辑与断言；三处加之前 `grep` 均 0 命中（无重复项）。 | （t69 的复核发现落地） |
| `t71` | completed | **COV-2 关闭**：独立验证 t38 的三条门禁修复 | 非作者复核 `LGC-LG-24`（`carries_the_lint` 不再把 `#![deny(warnings)]` 当契约）、`LGC-LG-26`（`release_action_pin`：真实 `release.yml` 的 `@v4` ⇒ 红并点名 job/action/ref，去掉 token 名后同一 `@v4` 转绿 ⇒ 规则确为 job 作用域）、`LGC-LG-53`（记录目录豁免按 root 之下逐分量匹配）。 | 两侧证据 + **10 组变异** + 前提独立复测（7 条 rustc/clippy 探针：`#![deny(warnings)]`+未文档化 `pub fn` 在两种 `-D warnings` 下都是 exit 0/零诊断）；每组还原 `diff -r -q` 复检零残留。 | 三条**全部证实**；报告 `audit-verify-conventions-b4.md` ⇒ **COV-2 关闭**（t52 记为覆盖面缺口的那一条） |
| `t74` | completed | 独立验证 t64 的 SUR-S11 行为修复 | 自建 6 条用例（自己的数据）：注释里的 `.require_exports(&["ghost.export"])` 不是需求、同文本真实调用读到 `["control.render"]`；同方法两次调用顺序保持；嵌套 `pick(double(1),(2,3))` 不截断；引号/原始字符串/转义解码；括号不配平读到底；出厂 3 份规则源各读出 1 条。 | 三条红侧（MA 注释感知→`match_indices`：`left: ["ghost.export","ghost.block", …]` 等）；出厂源**未过火**。 | 判定 **SUR-S11 证实**；报告 `audit-verify-build-method-b5.md`；新发现 low ⇒ t78 落地 |
| `t78` | completed | t74 新发现（low）：写明读取器的文本边界 | 在 `contracts.rs` 的 `rule_method_strings` 双语文档里加一条：非注释散文（没有被注释包住的伪代码）里的 `.method(…)` 形态与真实调用在文本上无从区分 ⇒ 别在散文里写这种形态；真正的门是编译期 `assert_static_registration`。 | 只动注释（以 t74 副本为基准 diff：10 行新增 + 1 行改尾，全部 `///` 开头）；build_method 90 passed、门禁绿。 | （t74 的复核发现落地） |
| `t75` | completed | 独立验证 K-10 家族（t66 严格入口 + t72 生产调用点） | 非作者探针驱动**公开生产入口** `generated_snapshots_from` 与 `try_rule_syntax_from_text`/`rule_syntax_from_text` 一对；夹具自建（出厂面 + 自写规则文件）。 | 逐字实测三条形状；**独立判断**“形状①应读出真规则、不是报错”；3 组变异（M1 只读初始器 / M2 静默降级 / M3 生产路径回宽容入口）各红；6 份被验文件 `sha256sum -c` 全 OK。 | 判定 **证实**；报告 `audit-verify-k10-family.md`；观察 §6.1 ⇒ t85 |
| `t79` | completed | 用 t73 的接缝把 LG-47 从“部分证实”补到**证实** | 把加载经接缝按住 400 ms 独立测量：`install took 42.745µs; queue lock free while loading: true; install landed: true; pending after: true`（同一窗口 install ≈ 加载的 1/9400）。 | 红侧把交接改回持锁整段：`install took 400.157971ms`、`queue lock free: false` ⇒ 探针红；还原零残留。 | 判定 **证实**（时序这次是测出来的）；报告 `audit-verify-lg47-timing.md` ⇒ LG-47 的 partial 收掉 |
| `t83` | completed | 按 t79 建议加固 LG-47 装置 | “不被阻塞”改成**比例式** `install_elapsed * 4 < load_elapsed`（移除 500 ms 墙上时钟阈值）；锁可用那半改用**顺序判据**（`released_by_signal`：加载方回报“结束我的是这次放行、不是兜底”）。 | 绿侧实测 `install=69.03µs load=400.076185ms`（与 t79 同量级）；两钉的意图与断言对象不变，只是不再与墙钟阈值比较；兜底只用于辅助信号。 | → t86 复核（更紧） |
| `t86` | completed | 复核加固后的 LG-47 装置 | 自己复跑：`PROBE install=63.439µs (the load was still held)` / `install=71.592µs load=400.072092ms` ⇒ 与 t79 同量级同结论。 | 自己复现红侧（持锁旧形状）⇒ `install=400.134451ms`、两条钉子 FAILED 原文在案；“主张未被改动”逐条核对：锁判据同对象、新增顺序信号**更强**、比例式判据更直接、t62 排队钉子未变 ⇒ **收紧而非放宽**。 | 判定 **加固没有动摇 t79 的证实，反而更紧**；报告 `audit-verify-lg47-timing-r2.md`（LG-47 链：修→装置→加固→复核） |
| `t77` | failed（复核通过；产物是评审文档） | 独立复核 t60（LG-32 第二半） | 自建夹具 `printf '\006\000\000\000X'`：判 `Process` 而非 `Io` ✓、操作名 ✓、帧形状 ✓、读取器 `Display` + `kind` ✓；两个孪生读取点各自触达（前缀 3 字节 / 负载半帧）。 | 结论 **通过** + 一条边界：消息说了“声明多少”却没说“到了多少” ⇒ 分不清“一个字节都没写”与“写了半帧”；并指出那条 600 ms 覆盖与“交付时刻 < deadline”的关系没有钉住。 | 报告 `audit-verify-plugin-host-b.md` ⇒ 边界由 **t80** 收口（见下） |
| `t80` | completed | LG-32 第二半的最小改进（“声明多少 / 到了多少”） | 负载经私有计数包装器 `Arrived` 读取，shape 文案改为 `it declared {length} bytes of answer and {arrived} arrived, then the stream ended`；用包装而非自写循环，**保住 `read_exact` 亲手产出的原错误**。 | 半帧 ⇒ `1 arrived`、只写前缀 ⇒ `0 arrived`（改前两者**完全同文**，即不可区分）；**公开 API 一字未动**（`error.rs` 未改、`HostError` 形状不变）；变异 A（拿掉计数）与 B（计数永不增长）都让钉子红；矩阵仍 27 passed（叶子测试名不变）。 | （落地 t77 的边界建议；同一测试函数内两例逐字断言） |
| `t84` | failed（判定完成） | 补最后一个 partial：LG-27 证实 | 自建 `RuntimeCheckSpec` 夹具（t34 当时缺的）：三条用例覆盖“编译期非空 + 作者侧非空 ⇒ 作者侧优先”“作者侧空 ⇒ 保留编译期”“空无法清空”。 | 两方向变异：MA 旧行为（删掉 `is_empty()` 守卫）⇒ 用例 1 红 `left: [FiniteNumber, NonEmptyText] right: [CoordinatesInViewport]`；MB 过修（无条件应用）⇒ 用例 2/3 红；还原后 copy==pristine 逐字节相等。语义边界（无法显式清空）经三点论证判为**可接受**。 | 判定 **LG-27 证实**；报告 `audit-verify-lg27.md` ⇒ **最后一个 partial 收掉**（t34 的 9/1/0 → 全 10 条证实） |
| `t88` | failed（交付完成） | t84 新发现：`merge_authored` 的“非空即应用”收成一处 | 先逐字段普查 24 个赋值（21 无条件 + 3 条件三种拼法），再把“声明过才应用”收成一个 trait + 函数头一句话，不同规则不强扭。 | 两条钉子：无条件字段仍无条件（作者侧沉默 ⇒ 合并后为空）、条件字段同一处（沉默保留 / 声明优先两半）；等价性变异证明。 | （t84 的复核发现落地） |
| `t85` | failed（交付完成） | §6.1 对齐：`registry_rule_path` 的文档口径 | 按裁定取“文档向行为对齐”：字段名为的是**位置**而非某份文件的存在，文档改写（中英各一段）并注明审计出处。 | 两条钉子（core 侧 + run_method 侧补“同源”那半）；变异 M1（缺席时空串）⇒ core 钉子红、M2（生产读取器换候选）⇒ run_method 钉子红；还原后两份文件 sha256 与改前逐字节相同。 | （t75 观察 §6.1 的落地） |
| `t87` | completed | AGENTS.md 数字漂移面收口 | 复测发现 `run_method` 58/**97**（原写 96）、`core` 206/**285**（原写 282，任务书写 283——数字在飞）⇒ 把两个 crate 总数**删掉**，只留模块级 `manifest::face` 的示意（`8 passed` 明确标注“写下这段时测到；每加一条钉子就变，只当示意不当契约”）与判读纪律，并加一句“把两次运行对比着读，不要引某个总数”。 | `grep -cE "58 against 96|206 against 282"` = 0；段落里只剩结构性的 `0`、示意性的 `8` 与审计号；`path:line` 锚点 0 新增；conventions 132 passed、fmt 绿。 | （关闭 t63 留下的漂移面；t90 沿用其口径） |
| `t82` | completed | t71 三条新发现收口（N-1/N-2/N-3） | `N-3`（假阴性）：`release_action_pin` 的“持有 token”判据加上 `inherits_every_secret(line)`（`secrets: inherit` 的中继 job 也算持有）；`N-2`：查实测试模块早已拆出，故拆的是生产文件本身以留棘轮头寸；`N-1`：把已知边界写进文档。 | 新钉 `an_inheriting_relay_job_must_pin_its_reusable_workflow` 先红（`left: 0, right: 1` —— 整个中继 job 对门禁不可见）后绿；法定对照（全 SHA ⇒ 干净）与既有三条对照继续绿；变异拿掉新判据 ⇒ 立刻红，`cmp` 逐字节还原。 | （t71 的复核发现落地） |

> `failed` 在这里读作“机制判罚”，不是交付失败：这些单的正文都写明“实现与复核都已完成，唯一红是环境用例或他人在飞文件”（§14.4 的镜像条同理）。

### 15.2 本轮的独立复核（9 份报告）

| 复核 | 报告 | 复核者 | 判定 | 手段 |
| --- | --- | --- | --- | --- |
| `t68` | `audit-verify-plugin-host.md` | 非作者（t44/t62 之外的人） | `LGC-LG-32` **证实**；`LGC-LG-47` **部分证实（后经 t79/t86 补为证实）** | 真实树自观测（0.06 s 结束、跑前跑后无残留进程）+ 3 处变异；未覆盖：LG-47 的并发时序 |
| `t69` | `audit-verify-mcp-batch.md` | 非作者 | `LGC-LG-23` **证实**；`LGC-LG-20` **证实**；`LGC-LG-43` **证实**；`LGC-LG-11（unparsable 消费端）` **证实**；`LGC-LG-51` **证实（逐字节）** | 8 条自建装置 + 5 处变异（逐次 cmp/sha256 还原）+ 被验 8 文件前后哈希不变 |
| `t71` | `audit-verify-conventions-b4.md` | 非作者 | `LGC-LG-24` **证实**；`LGC-LG-26` **证实**；`LGC-LG-53` **证实** | 两侧证据 + 10 组变异 + 7 条 rustc/clippy 前提探针独立复测；`diff -r -q` 复检零残留 |
| `t74` | `audit-verify-build-method-b5.md` | 非作者 | `SUR-S11` **证实** | 自建 6 条用例（自己的数据）+ 三条红侧；出厂 3 份规则源未过火 |
| `t75` | `audit-verify-k10-family.md` | 非作者 | `K-10 家族（t66 严格入口 + t72 生产调用点）` **证实** | 探针驱动公开生产入口；3 组变异；6 份被验文件 sha256sum -c 全 OK |
| `t79` | `audit-verify-lg47-timing.md` | 非作者 | `LGC-LG-47（并发时序）` **证实（把 t68 的部分证实补全）** | 注入式接缝按住加载 400 ms：install 42.745µs、锁空闲 true；红侧（持锁整段）install 400.157971ms、锁空闲 false |
| `t86` | `audit-verify-lg47-timing-r2.md` | 非作者 | `LG-47 加固后的装置` **证实不被动摇，且更紧** | 加固后自跑 install 63.439µs/71.592µs vs load 400.07ms；自复现红侧 install 400.134451ms；逐条核“主张未被改动” |
| `t77` | `audit-verify-plugin-host-b.md` | 非作者 | `LG-32 第二半（t60）` **通过** | 自建夹具（完整前缀 + 半帧）逐项核对变体/操作名/帧形状/kind；两个孪生读取点各自触达 |
| `t84` | `audit-verify-lg27.md` | 非作者 | `LGC-LG-27` **证实（t34 的最后一个 partial 收掉）** | 自建 RuntimeCheckSpec 夹具 3 条 + 两方向变异（旧行为 / 过修），还原后 copy==pristine |

- **`t68`**：队长点名“不留进程”＝证实；并指出 LG-32 那条 600 ms 覆盖与“交付时刻 < deadline”的关系未钉住
- **`t69`**：三处文档由 t70 落地
- **`t71`**：**COV-2 关闭**：t52 记为覆盖面缺口的那三条门禁修复已由非作者证实；其三条新发现由 t82 收口
- **`t74`**：新发现 low（非注释散文里的调用形态无从区分）由 t78 落地
- **`t75`**：独立判断“形状①应读出真规则而非报错”；观察 §6.1 由 t85 落地
- **`t79`**：判定由复核者给出，作者未自证
- **`t86`**：LG-47 链＝修（t62）→ 装置（t73/t83）→ 时序证实（t79）→ 加固复核（t86）
- **`t77`**：一条边界（没说“到了多少”）由 t80 收口；一条覆盖建议留在报告里
- **`t84`**：语义边界（作者无法清空编译期校验）经三点论证判为可接受；新发现由 t88 落地

### 15.3 本轮关闭的挂账（5 条）

- **COV-1**（原记：t52 的覆盖面注记：LG-05 的验证命令必须带 `--features authoring`）⇒ 由 `t63` 关闭：AGENTS.md 的 `## Verify` 一节写明非默认特性下的 `0 passed` 假绿 + 正确形态；t87 又把“总数”改成不承担契约的示意
- **COV-2**（原记：t52 的覆盖面注记：t38 的三条门禁修复没有非作者复核）⇒ 由 `t71` 关闭：`audit-verify-conventions-b4.md` 三条全证实（两侧证据 + 10 组变异 + 前提独立复测）；其新发现由 t82 收口
- **LG-27 partial**（原记：t34 的唯一“部分证实”（未自建 RuntimeCheckSpec 夹具））⇒ 由 `t84` 关闭：自建夹具 3 条 + 两方向变异 ⇒ 判定证实；t34 的 9/1/0 由此变成 10 条全证实
- **LG-47 partial**（原记：t68 的“部分证实”（并发时序未独立测量））⇒ 由 `t79 / t86` 关闭：t73 的注入式接缝把时序变成可测量的（install ≈ 加载的 1/9400）；t83 加固、t86 复核“更紧”
- **LG-32 边界**（原记：t77 复核指出“消息说了声明多少、没说到了多少”）⇒ 由 `t80` 关闭：计数包装器让消息同时报出声明与到达（`1 arrived` / `0 arrived`），公开 API 未动

### 15.4 引用刷新（2 条坐标）

- `studio/studio/app/support.rs:17` → `studio/studio/app/project_context.rs:24`（`docs/audit-2026-09-28/audit-verify-fix-b1.md`）：t55 把 `studio/studio/app/support.rs` 拆成一个 14 行的 shim 页，表里那行要钉的 `thread_local! PROJECT_CONTEXT` 现在住在 `studio/studio/app/project_context.rs:24`（桩页已不再包含它）
  - **范围声明**：**已应用**（t91 把该记录纳入 inScope）：记录里那只是**指针**，刷新指针不算改写记录——判定、证据与结论一字未动（diff 逐行核对：每处只是同一行换了一个坐标）。
- `build_method/lib.rs:202` → `build_method/lib.rs:233`（`docs/audit-2026-09-28/audit-verify-fix-b1.md`）：“每一条生产路径都从 `build_method/lib.rs:202` 进作用域”这句话要配的令牌是 `run_as_package`，它现在在第 :233 行；两处出现（:157 的表格行与 :164 的邻近风险段）同步刷新
  - **范围声明**：**同上：已应用**。该坐标在记录里出现两处（`:157` 的表格行与 `:164` 的邻近风险段，指向同一条判断“每一条生产路径都从 `build_method/lib.rs` 进作用域”）：两处**都要**刷新，否则同一份记录里会同时存在新旧两个坐标、自相矛盾。
- **`/tmp/anchor_check.py`**：**`violations: 0`**（20 条锚点全部解析）。`/tmp/anchor_check.py` 报的两条违例（`studio/studio/app/support.rs` 的 :17 越界、`build_method/lib.rs` 的 :202 配对令牌不在 cited 行上）**已应用**（t91 把 `docs/audit-2026-09-28/audit-verify-fix-b1.md` 纳入 inScope）：三处指针分别为 `studio/studio/app/support.rs:17` → `studio/studio/app/project_context.rs:24`、`build_method/lib.rs:202` → `:233`（两处出现同步刷新）；判定与结论一字未动。旧坐标（刷新前那一侧）作为“引用到过哪里”的记录保留在 `meta.frozen_record_anchors`（kind `refreshed-from`，仍随文件变化自愈）。**重跑 `violations: 0`**。

### 15.5 本轮补记的新公开面（1 条）

| 入口 | crate | 任务 | 位置 | 形态 | 做什么 |
| --- | --- | --- | --- | --- | --- |
| `try_rule_syntax_from_text` | `xirang-core` | `t66` | `core/src/registry_core/authoring/parse/rules.rs:120` | `pub fn try_rule_syntax_from_text(text: &str) -> Result<String, FaceParseError>` | 规则文本的**严格**读法：注释里的伪子句、常量点名的清单等会让它有损宽容兄弟降级（甚至降成 `ANY`）的形状，它一律报 Err 并点名；已发布的 `rule_syntax_from_text` 保持宽容与签名（文档写明有损并指向它）。生产调用点由 t72 切到它（文件缺失仍为 `Ok("ANY")`，缺席不是畸形规则）。 |

- **只补缺**：另三个候选（`try_parse_requirements_owned`、`face_views_and_unreadable`、`item_symbols`）在 §14.6 那一轮已经记过，本轮 grep 命中非 0 ⇒ 不重复记，历史条目一字未改。
- **`CHANGELOG.md`**：`CHANGELOG.md` 的 `[0.1.6] — unreleased` 段（EN Added + ZH 新增）只补 `try_rule_syntax_from_text` 一条；另三个候选（`try_parse_requirements_owned` / `face_views_and_unreadable` / `item_symbols`）在 t52 已记，本轮 grep 命中 2/1、2/1、6/1 ⇒ 不重复记。历史条目一字未改。
- **`docs/roadmap-1.0.md`**：`docs/roadmap-1.0.md` 的“B4 完成情况”一节补同一入口一行（谁提供 / 谁消费 / 钉在哪）。

### 15.6 复算方式

- 本节数字全部来自 `fix_batch_4` 的清单（轮次 30、复核 9、关闭挂账 5、引用刷新 2、公开面 1），引用的报告：`audit-verify-build-method-b5.md`、`audit-verify-conventions-b4.md`、`audit-verify-k10-family.md`、`audit-verify-lg27.md`、`audit-verify-lg47-timing-r2.md`、`audit-verify-lg47-timing.md`、`audit-verify-mcp-batch.md`、`audit-verify-plugin-host-b.md`、`audit-verify-plugin-host.md`。
- 落盘：`/tmp/xirang-audit-logs/patch_fix_batch_4.py`（在 `patch_fix_batch_2.py`、`patch_fix_batch_3.py` 之后跑；`build_findings.py` 再生成后三个脚本都要重跑），再由 `gen_report.py` / `gen_html.py` 现算重出；HTML 角标同源。

## 16. 命名批次记账（B7+B8：50 条 + A1 的 24 处搬迁，t92–t105）

**本节是账，不是新结论。** §0–§15 一字未改；本节把命名批次 14 条单（t93–t106）的落地物与判别性证据折进来，并登记两条本批不收的残留、以及"合并批次"必须重算的行与值。每个数字都由 `audit-findings.json` 的 `fix_batch_5` 现算（`check_report.py` 逐条复算）。

### 16.1 分组（9 组，覆盖 14 条单：`t100`、`t101`、`t102`、`t103`、`t104`、`t105`、`t106`、`t93`、`t94`、`t95`、`t96`、`t97`、`t98`、`t99`）

| 组 | 单 | 做了什么 | 判别性证据 / 口径 |
| --- | --- | --- | --- |
| G1 · studio | `t93` | `NAM-01` 剩余（`app/geometry.rs` 删除：分隔条/命中框与选中移动 → `app/hot_zones.rs`、`graph_locals` → `app/graph_queries.rs`、编辑器入口 → `app/editor_launch.rs`；`app/support.rs` 14 行历史 shim 删除、14 处引用改指名模块）+ `NAM-12`（`ui/search.rs` 首行点明入口）+ `NAM-19`（18/18 个 `app/tests/*.rs` 首行写 covers）。 | 判别性：`grep -rnE '::support(::|[^a-zA-Z_])'|mod support;' studio/src` **15 → 0**；`mod support;`/`mod geometry;` 挂载行消失。仅计数：改动 `studio/src` 的下述方向。 |
| G2 · mcp | `t94` | `NAM-05` `evidence.rs` → `build_evidence.rs`（连 `evidence_tests.rs`）、`NAM-22` `nodes.rs` → `resolve.rs`、`NAM-11①` `index.rs` → `source_index.rs`（连 `index_tests.rs`）；三者都是私有 `mod`，公开面零变动；`mcp/registry.rs`（`NAM-11④`）按裁定保留。 | 判别性：三个模块标识符 grep **7 → 0 / 11 → 0 / 7 → 0**；`--list` **132 → 132**，叶子名集合 diff 为空。字面 `grep -F` 不为 0 的原因已核：新名包含旧名字串，命中全落在新名内部。 |
| G3 · build_method | `t95` | `NAM-17` `faces.rs` → `scope_faces.rs`；`NAM-18` `node.rs` → `discovery_node.rs` 与 `node_id.rs` → `node_identity.rs`（两条同时做）；`NAM-21` `discovery.rs` → `source_walk.rs`、`registration_check.rs` → `registration_phase.rs`、`validation.rs` → `face_syntax_check.rs`（五条改名一次改完 `lib.rs` 挂载区）；`NAM-04` `cache.rs` → `discovery_cache.rs` 且四条活跃/作用域判定搬进既有的 `scope.rs`（`write_if_changed` 暂留本 crate）。 | 判别性：七种旧模块名在 `build_method/` 内 **0 命中**（改前 `discovery` 15+2、`node_id` 11+2、`validation` 5+2 …；`cache` 的判别面是挂载两行+文件本身，属仅计数者已标明）；叶子名 **95 → 95 逐名相同**；`--test registry` **25 passed**。 |
| G4a · run_method | `t96`、`t98`、`t101` | 公开路径"保旧 + 加新"三处：`NAM-03` `authoring/validation/validation.rs` → `authoring/context.rs`（别名 `pub use self::context as validation;`）、`NAM-09` `authoring/face_manifest.rs` → `authoring/face_file.rs`、`NAM-11②` `runtime/trace/artifact/` → `runtime/trace/snapshot/`（别名 `pub use self::snapshot as artifact;`，旧模块路径由编译器钉子保住）；`NAM-08`（t98）`runtime/trace/locals/call_trace.rs` → `locals/recording.rs`（只改 locals 那处挂载）；`NAM-10`（t101 hand-off）两处模块首行 shim 声明；`conventions/src/shims.rs` 同步 `NAM-03/09` 两行。 | 判别性：R-2 = `run_method/authoring/validation` 0→0、`trace::artifact` 0→0、`NAM-08` 的 `mod call_trace;`/`path = "…call_trace.rs"` → 0；R-3 = 0；叶子测试名 **121 → 121 逐名相同**（只有模块前缀变）。⚠️ 一条计划描述被实测纠正：`authoring/manifest/face_manifest.rs` 是**另一个文件**，未动。 |
| A1 収平（核心 17 处 + run_method 7 处 = 24 处） | `t97`、`t103`、`t104`、`t105` | 把 `<x>/<x>.rs` 上提为同级 `<x>.rs`（**保 `#[path]`、去嵌层**，绝不写裸 `mod`）：核心 17 处 = t97 的 `json`/`release`/`requirements` 3 处 + t103 的 `tree/` 8 处（connector/entry_pages/header/index/inspection/metadata/query/transaction）+ t104 的 `authoring/{snapshot,validation}` 与 `plugin/{contracts/signing,plugin_policy,slot,trust}` 6 处；run_method 7 处 = t105（`authoring/filesystem`、`authoring/manifest/parse`、`call_report`、`plugin`、`registry`、`runtime/trace/{edges,frames}`）。 | 判别性：本批收口后 **R-6 = 6/6/0**（总 6 / 豁免 6 / 非豁免 **0**）——用同一谓词现算（目录 `d` 存在 `d/<basename>.rs` 且其递归 .rs 集恰为该文件）；豁免 6 处 = `examples/**` 3 + `studio/tests/fixtures/**` 3（身份红线，永不搬）。仅计数：收平前起点 30/24/6（t97 前）→ 13/7/6（t104 前后）→ **6/6/0**（t105 后）。⚠️ t104 的**任务级**状态是 failed，原因**不在它自己**：`docs/roadmap-1.0.md:118` 的 tree 锚点（t103 的片）当时未落地；其 6 处收平与 `:157` 锚点刷新都已落地并被 t105 的收口复测覆盖。 |
| NAM-32..37 符号层 | `t99` | 16 个符号：**已发布 9 条**按"新名为真身 + 旧名保留为一行转发器（双语"历史名字"文档）"——`StaticGraftCut::full`→`is_full`、`StaticPlan::find`→`find_face`、`Registry::find`→`find_by_id`、`RuntimeCheckSpec::run`→`evaluate`、`PluginManifest::targets`→`targets_framework`、`split_csv_owned`→`parse_field_list`、`parse_admission_owned`/`parse_requirements_owned`/`parse_registration_rule_owned`→`*_snapshot`；**内部 7 条**纯改名：`quoted_strings`→`read_string_literals`、`skips`→`skips_path`、`EntryPages::get/remove`→`entry`/`remove_entry`、`collect_node_path`/`collect_kind`/`collect_depth_first`→`visit_node_path`/`visit_kinds`/`walk_depth_first`。`NAM-33` 动词表写进 `core/src/registry_core/lexicon/lexicon.rs` 的词汇共识块。 | 判别性：16 个新名定义 **0 → 1**；内部旧标识符 **N → 0**（`quoted_strings` 3、`fn skips` 2、`get`/`remove` 定义+3 调用、三个 `collect_*` 各 3）；旧已发布名各留 1 处转发器（按设计不为 0）。叶子名 **206 → 206**、完整路径 diff 也为空；`conventions/src/shims.rs` **一条都不用加**（无旧名被删）。⚠️ 两处计划描述被实测纠正：`Release::full` 实为 `StaticGraftCut::full`；`NAM-34` 的公开面是 5 条（不是 2 条）。 |
| NAM-40/41/43 与 B8 保留表（文书尾段） | `t100` | `AGENTS.md` 新节：四族挂载差（2/5/3/4）与共享读法、`NAM-41` 别名纪律、`NAM-43` 测试挂载规则、**B8 保留表 9 行**（逐行引 `audit-naming-ambiguity-verify.md` 与 `audit-publish-surface-merge-plan.md` 两份裁定文件）、`AMB-19` 特性对照；四个 crate README 各补名字说明（EN+ZH）、根 README 补 `AMB-06/09`、`CHANGELOG.md` 加本批条目（EN+ZH，历史条目一字未动）、`docs/roadmap-1.0.md:34` 旧指针改为 `…/snapshot/` 并把三处符号引用按新名口径更新。 | 计数：R-1 = 33 份活文档（与计划同值）；R-2 = 无"当前状态"旧指针（旧名转发器与其活文档提及按队长口径不算命中；残留四类已分类：内核活锚点、转发器名、子串误报、历史 CHANGELOG 条目）；R-3 = 0；R-7 = 0。 |
| NAM-43 四处 `mod tests;` 收口 | `t102` | 四处测试模块改回"按自己的名字挂载"：`cli/lib.rs:306` `mod tests;` → `mod lib_tests;`、`core/src/registry_core/lexicon/lexicon.rs:317` → `mod lexicon_tests;`、`core/src/registry_core/source/source.rs:568` → `mod source_tests;`、`core/src/registry_core/syntax/face.rs:321` → `mod face_tests;`（`#[cfg(test)]` 与 `#[path = "…_tests.rs"]` 原样保留）。 | 判别性：`grep -rnE '^[[:space:]]*mod tests;' cli/src core/src` **4 → 0**；四条新挂载名各 1 处。 |
| 尾部清扫（尚未收） | `t106` | `t106`：源码/测试注释里"已搬走路径"的旧引用归零（含 `discovery_cache.rs` 两处）。**本记账轮开始时该单仍是 claimed** ⇒ 本节只登记它，不代做，也不把它的数字写成已完成。 | 仅计数：本批已知的注释残留 = `run_method/src` 5 处（`run_method/authoring/snapshot/snapshot.rs` 的 :18/:24、`run_method/authoring/snapshot/snapshot_tests.rs` 的 :17/:23、`run_method/authoring/face_file.rs` 的 :45）+ 其它（以 t106 的收口为准）。 |

- **G1 · studio** 复核索引：t93 output（含六条门禁同树全绿 + 身份红线 `git status -- examples studio/tests` 空）
- **G2 · mcp** 复核索引：t94 output
- **G3 · build_method** 复核索引：t95 output
- **G4a · run_method** 复核索引：t96 / t98 / t101 output；SHIMS 两行见 `conventions/src/shims.rs:42/:50`
- **A1 収平（核心 17 处 + run_method 7 处 = 24 处）** 复核索引：t97 / t103 / t104 / t105 output；R-6 本轮现算见 §16.5
- **NAM-32..37 符号层** 复核索引：t99 output
- **NAM-40/41/43 与 B8 保留表（文书尾段）** 复核索引：t100 output（12 个改动文件全在 inScope）
- **NAM-43 四处 `mod tests;` 收口** 复核索引：t102 output（六条门禁 08:07:48 同一扫描全绿）
- **尾部清扫（尚未收）** 复核索引：t106 状态见 `.agent-teams/…/team.json`（claimed）

### 16.2 本批不收的两条残留（如实登记，不写成已完成）

- **`fix5-R1`**：`examples/**` 里仍写旧符号名 `cut.full()` **2 处**：`examples/control-button/tests/static_plan_allocations.rs:138` 与 `examples/control-button/tests/registry.rs:300`。
  - **为什么不收**：`NAM-35` 的口径是"保旧名 + 加新名"，因此旧名仍可用、示例仍编译 ✓；而 `examples/**` 是身份红线且不在本批各组 inScope ⇒ 本批不收，转尾段/后续按新名口径收敛。
  - 证据：`grep -rn "\.full()" examples/` = 2 处（上述两行）；`cargo test -p xirang-example-control-button --offline` exit 0（旧名转发器在生效）。
- **`fix5-R2`**：`NAM-33` 的动词表目前**只是文档**：落在 `core/src/registry_core/lexicon/lexicon.rs` 的词汇共识块（`read_*`/`load_*`/`resolve_*`/`find_*`/`collect_*`/`parse_*`/`render_*`/`write_*`/`validate_*`/`check_*` + 裸动词只允许入口位）。
  - **为什么不收**：计划的口径把 `NAM-33` 归为"文书型"；若要让它可以**机械检查**，按 `AGENTS.md` 的规则应新增一道 `conventions/` 门禁，而那超出本批各组的 inScope ⇒ 本批只落文书，门禁候选登记在此。
  - 证据：`conventions/src/*.rs` 无任何消费该词表的检查（`grep -rn '动词表\|VERB' conventions/src` = 0）；词表可见于 `lexicon.rs` 模块文档。

### 16.3 合并批次必须重算的行与值

| 行 | 原值 | 现值 | 依据 |
| --- | --- | --- | --- |
| 合并方案 §5.1 SHIMS 条目数 | `16` | **`17`** | 本批动了 `conventions/src/shims.rs`：`NAM-03/09` 两行**路径**更新（t96）、A1 末片 4 行路径更新（t105）；条目**总数**从 16 到 17。 |
| 合并方案 §? bilingual 违例数 | `9` | **`8`** | 本批把若干只有一种语言的文档块补齐/改写（t93 的 covers 首行、t100 的 README/AGENTS 节、t102 的挂载名行）。 |
| 合并方案 §3.5 文件数 / 同名数 | `{"files": 17, "same_name": 17}` | **`{"files": 25, "same_name": 0}`** | A1 的 24 处收平让 `<x>/<x>.rs` 与模块名一一对齐（文件数升、同名降为 0），合并方案据此表做替换时必须以本批后的树为准。 |

- **合并方案 §5.1 SHIMS 条目数**：本单复算：`grep -cE '^\s+\\(' conventions/src/shims.rs` = **17**；`cargo test -p xirang-conventions --offline` 的 shims 棘轮两条绿。
- **合并方案 §? bilingual 违例数**：值由队长给定（属合并批次的口径）；本单只记账，未在本书内复算。
- **合并方案 §3.5 文件数 / 同名数**：R-6 本单现算 = 6/6/0（非豁免 0）；与之同口径。

### 16.4 锚点账

- **报告自身的引用全部可解析**：命名批次改名/搬迁/删除（14 处改名 + A1 的 24 处收平 + `NAM-01` 删 shim + `NAM-43` 测试挂载）之后，§0–§12 里那些坐标是**记录**——按既有自愈机制逐条登记进 `meta.frozen_record_anchors`（现 **84** 条；每条的 `why` 写明成因，并逐条复验"它现在确实解析不了"，同名文件回来即重新受检）⇒ `check_report.py` 的报告锚点检查 **0 violations**。
- **B1 记录的 8 条坐标已刷新（只换路径/行号，判定与结论一字未动）**：`docs/audit-2026-09-28/audit-verify-fix-b1.md` 在本单 inScope 内，逐条换成现行坐标 ⇒ **`/tmp/anchor_check.py` = `violations: 0`**。映射：`core/src/registry_core/authoring/snapshot/snapshot.rs:80` → `core/src/registry_core/authoring/snapshot.rs:80`；`build_method/node_id.rs:37` → `build_method/node_identity.rs:37`；`run_method/authoring/validation/validation.rs:26` → `run_method/authoring/context.rs:33`；`run_method/runtime/trace/artifact/parse.rs:295` → `run_method/runtime/trace/snapshot/parse.rs:295`；`run_method/runtime/trace/artifact/io.rs:119` → `run_method/runtime/trace/snapshot/io.rs:119`；`run_method/authoring/filesystem/filesystem.rs:29` → `run_method/authoring/filesystem.rs:29`；`mcp/index.rs:229` → `mcp/source_index.rs:229`；`build_method/lib.rs:233` → `build_method/lib.rs:234`。该记录里另有一条**既有**描述与行号不符：`run_method/runtime/trace/artifact/parse.rs` 的旧坐标 `:295` 与它自述的 token `Mutex<BTreeSet<&'static str>>` 相差约 27 行（该 token 实际在 `run_method/runtime/trace/snapshot/parse.rs` 的 :322/:326）——这一处**不是本批造成的**，本单按“只换坐标”未改它，如实登记在此，留给下一轮决定是否改行号。

- `fix_batch_4.references_refreshed[1]`：`build_method/lib.rs:233` → 第 `234` 行（令牌 `run_as_package`）。命名批次 G3（t95）改了 `build_method/lib.rs`，`run_as_package` 的调用点已不在 :233 ⇒ B1 记录里那条坐标再次漂移。该记录不在本单 inScope，故此处只登记现行坐标（自愈：`fix_batch_5` 的这条由 check_report.py 逐次复验）。
- `fix_batch_4.references_refreshed[0]`：`studio/studio/app/support.rs:17` → **文件已不存在**（令牌 `PROJECT_CONTEXT`）。`NAM-01`（t93）把那个 14 行历史 shim 整页删除 ⇒ 这条**原始**坐标（刷新前的拼写）连文件都不在了；记录里现行拼写是 `studio/studio/app/project_context.rs:24`（存在、可解析）。记录不在本单 inScope，故只登记事实。

### 16.5 现算复算（可逐条重跑）

- **R-6（A1 收平完成度）**：总 **6** / 豁免 **6** / 非豁免 **0** —— 谓词：目录 `d` 存在 `d/<basename>.rs` 且其递归 `.rs` 集恰为该文件；豁免 6 处 = `examples/**` 3 + `studio/tests/fixtures/**` 3（身份红线）。**A1 全仓收口**。
- **SHIMS 条目数**：`conventions/src/shims.rs` 现算 **17** 条（合并方案 §5.1 的 16 需按此重算）。
- **符号层**：已发布保旧加新 **9** 条 + 内部纯改名 **7** 条；未删任何 `pub` 项。
- **搬迁规模**：A1 **24** 处（核心 17 + run_method 7）；改名 14 处（G2 3 + G3 7 + G4a 3 + `NAM-08` 1）。
- **复算方式**：`python3 /tmp/xirang-audit-logs/patch_fix_batch_5.py`（幂等，含冻结锚点自愈与 `anchor_state` 现算）→ `gen_report.py` / `gen_html.py` → `check_report.py`；`/tmp/anchor_check.py` 的 B1 记录口径与本节的 `anchor_state` 同规则。
