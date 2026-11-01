# t31 复判：t30 修订后的命名报告 + 合并方案 §10 同步

> 复核者 boundary-architect。**只出报告 + 只改我自己写的那份方案**：本文（新）+ `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（我的产物）。源码一行未改，`cargo` 均 `--offline`。
> 复核对象：`docs/audit-2026-09-28/audit-naming-review.md`（**525 行**，第一版 374 行）——t30 已按我 t27 的 9 条 findings 逐条修订。
> 我的前一轮：`docs/audit-2026-09-28/audit-naming-verify.md`（9 条 findings 的原始证据）。

## 0. 这一轮怎么核（与 t27 相同的手段，独立于作者）

1. **不采信"已修"的自述**：每条都回到**原始谓词**上用我自己的脚本重算（`audit-inventory.json` 的 `files[]`/`functions[]` + 文件系统 `sed`/`grep`/`wc`）。
2. **逐行跑作者写在 §G 的脚本**，把它注释里的"→ 输出"与实际输出逐项对照（这是"谓词 + 命令 + 输出"是否诚实的唯一硬证据）。
3. **对 D-5/D-7 这类"清单"逐行核存在性与签名**（不是只数行数）。
4. 复核对象版本：`audit-naming-review.md` = 525 行；我在本轮读过的行段见各处引用。

---

## A. 9 条 findings 的关闭判定

| # | t27 finding | 判定 | 证据（我独立复算的命令/输出/行号） | 备注 |
| --- | --- | --- | --- | --- |
| ① | NAM-40「394/394 一致、0 违规」被证伪 | **已关闭** | 修订写在 `audit-naming-review.md:326`（结论级更正）与 `:328`/`:331-340`：谓词写成"`module_path` 非空 ∧ 末段 ≠ 文件 stem"，输出 **14 处四族**，并承认原两个"例外"是**重导出别名**（`:342`）。我用同一输入独立复算：**P1 = 14**，与它逐族相等（`registry_identity`/`registry_syntax` 2、`*_command` 5、`explain::*` 3、`::tests` 4）；它补的"P1' 含空 = 23"我也复算为 **23**（14 + 9 个 `module_path` 为空的 example/夹具面文件，逐个列出与它一致）。`grep -n '394/394'` 现在只剩 5 处，**全部在"原写…是错的/不要引用"的语境**（`:15`、`:23`、`:328`、`:344`、`:519`），没有任何一处把它当前提使用 | 它还把"文件**名**镜像（43/44）"与"**模块名**不镜像（14 处有意）"拆成两句（`:344`、§F F-2 `:424`、D-10 `:384`）——这正是我 t27 §3.2 要求的收口方式 |
| ② | NAM-07 / D-3 的「39」配错谓词 | **已关闭**（数字与谓词已统一）**＋ 一条落地范围条件**（见 §C.1） | `audit-naming-review.md:62` 的标题已改为"**34 处**…该目录**递归**只有这一个 `.rs`"；`:64-72` 给出三档谓词 A/B/C 与各自结果（34 / 39 / 67）并说明 39 的来源、把 39 与 67 标为不再使用；D-3 行 `:377` = **34（发布面 28）**。**我逐行跑了 §G-1 的 P2 段**（`audit-naming-review.md:461-482`）：`P2b direct-single-entry: 34`、`P2d recursive: 34`、`P2d published: 28`、`P2b-P2d: []`、`P3 dirs: 105` —— 与它注释里的"→ 34 / 34 / 28 / [] / 105"**逐项相符**。我另用自己的谓词独立算得 34/28（t27 §3.1），一致 | 验收措辞已改成"按 **A** 收平后为 **0**"（`:72`），**在发布面 28 内确实可达成**（裸 `mod x;` 在本仓的挂载约定下合法：`conventions/src/mounting.rs:8` 明确"父是 crate root 或本身经 `#[path]` 载入时同样正确"）；**范围条件见 §C.1** |
| ③ | NAM-05 降 MINOR 并在两个候选名间二选一 | **已关闭** | 标题 `:90` 写明"MINOR（原 **MAJOR**，t27 复核后降级）"；`:92` 的修订块给出正确计数 **12/17 同名 + 5 个例外**，且例外行号与我独立复核一致（`inspect` `mcp/tools.rs:331`、`read_source` `:356`、`status` `:393`、`unified` `mcp/mir.rs:125`、`explain` `mcp/evidence.rs:65`）；`:94` 承认"名字与内容相符"；`:98` 明确"**采用 t27 倾向**：→ `mcp/build_evidence.rs`"，并保留拆分方案为备选（"二选一，不要同时"）。D-9 行 `:383` 同步为 `build_evidence.rs` | 与它自己的 D-9（保留词加限定）**不再冲突** ✓ |
| ④ | NAM-06 降 MINOR + 改修法 + 撤销"并入 mutations.rs" | **已关闭** | 条目内（`:404-413`）：`:406` 引用首行 `Write guards` + 第二段"没有回落"，`:408` 明确"**同时撤回我原来的修法**…改为改名"，`:412` 定为 `studio/studio/app/write_guard.rs` 并写明"保留两个函数与那道缝，不动 `mutations.rs`"；§E 的 `STU-S-14` 行（`:399`）同步为"维持 MINOR…修法改为改名 `write_guard.rs`，不合并"。我实测首行确为 `Write guards`（t27 §1） | 这是我 t27 的"守卫缝"反证被接受；作者没有保留任何"合并"的残留表述（`grep -n '并入 mutations' audit-naming-review.md` 只命中撤回那句话本身） |
| ⑤ | D-5 ≥14、D-7 ~8、NAM-02 的 `CallRefRing`、§0 的目录数 | **已关闭** | **D-5**：`:196-234` 给出全量 **24 行**表并把白名单逐行标出，`:230` 给出算术"24 − 白名单 9 = **非入口位 15**"；我**逐行核了表中的 18 行**（`sed -n` 打行）——`build_method/pipeline.rs:11`、`cli/lib.rs:124`、`core/src/registry_core/declaration/runtime_checks.rs:345`、`plugin-host/src/deployment.rs:49`、`process.rs:146`、`wasm.rs:149`、`studio/studio/app/lifecycle.rs:79`、`mcp/mir.rs:217`、`core/src/registry_core/release/release.rs:243`、`studio/studio/ui/graph/node_graph.rs:36`、`mcp/apply.rs:60`、`tree_delta.rs:71`、`studio/studio/app/interaction.rs:12`、`entry_pages.rs:39`、`:43`、`plugin-host/src/lazy_wasm.rs:137`、`plugin_policy.rs:55`、`plugin-host/src/lib.rs:48` —— **全部存在且签名与表中一致**；我另跑了 §G-2 P5：`22` 行 + trait 2 行 = 24 ✓。**D-7**：`:274-297` 给出"全量 25 行、计入 **8** 行 + 17 行排除理由"，8 行的 `file:line` 我逐个打行核对（`core/src/registry_core/authoring/parse/parse.rs:258`、`core/src/registry_core/authoring/parse/admission.rs:128`、`core/src/registry_core/authoring/parse/rules.rs:108`、`:140`、`core/src/registry_core/authoring/parse/admission.rs:53`、`build_method/contracts.rs:193`、`cli/lib.rs:107`、`run_method/authoring/operations/paths.rs:41`）——**全部命中**；§G-2 P6 我也跑了：`25` ✓。**NAM-02**：`:51-58` 已把 `CallRefRing` 换成实测存在的 `Overlay`（`studio/studio/app/state/misc.rs:28`）、`CallRef`（`:55`）、`CallTreeView`（`:76`）——我打行确认三者都在。**§0**：`:31` 改成"**105 个目录**（谓词：394 个文件所在的不同父目录）"，我独立复算 **105** ✓ | D-5 的 15 与我 t27 的"≥14"自洽（我当时的清单漏了 `find` 两行与 `get_mut` 的分类） |
| ⑥ | 四条漏项 | **已关闭** | （a）**NAM-03 的公开路径钉子**：`:81-82` 新增 ⚠️ 段，点名 `conventions/src/shims.rs:50` 并列出同表其余被钉文件（`:27-80`）；我核对 `shims.rs:50` 确实是 `run_method/authoring/validation/validation.rs` ✓。（b）**模块挂载改名家族**：新增 **NAM-43**（`:345-354`），四族 + "全仓 44 个 `*_tests.rs` 里 40 个直接挂载、4 个挂 `mod tests`"——我独立复算：**44 = 4 + 40 + 0**（`mod tests` 4 个：`cli/lib_tests.rs`、`lexicon_tests.rs`、`source_tests.rs`、`face_tests.rs`；其余 40 个为 `mod <stem>`）✓。（c）**`entry_pages::get_mut`**：`:43` 已进 NAM-37（`:318`）与 NAM-32 表（`:225`）✓。（d）模块挂载家族即 (b) ✓ | 它给 NAM-43 的处置（"写进 `AGENTS.md` 的挂载约定 + 统一第 4 族，不动第 1 族"）与 `shims.rs` 棘轮不冲突 ✓ |
| ⑦ | "干净/一致"类结论必须写成"谓词 + 命令 + 输出" | **已关闭** | `:417-431` 的 §F 表 8 行，每行都有"谓词（§G 编号）+ 命令 + 输出"三列；`:440-523` 的 §G 给出 P1–P8 的脚本与实测输出，`:517-522` 另列"三处口径必须在引用时保持一致"（无载荷目录=P2d 34/28、模块名=P1 14、镜像=P2c 43/44、裸动词=P5 15、实现词根=P6 8），并逐条标注"**不要**引用"的旧数 | **我把 §G 的 P1/P2/P3/P5/P6 逐行跑过**，输出与它注释里的期望值**逐项相等**（14 / 23 / 34 / 34 / 28 / [] / 105 / 22 / 2 / 25）——这是本节关闭的硬证据 |
| ⑧ | D-4/D-6/D-11 的不可复算项 | **已关闭** | **D-4**（`:378`）只留已确证的 3 处（`core/src/registry_core/tree/query/query.rs:105`、`:164`、`:183`，我在 t27 已核实这三行都是私有 `fn collect_*`）并明确标"**待清单**"；**D-6**（`:380`）改成**纯前缀白名单**口径，并注明"落地后违规数会升到含 `skips`/`targets` 一级"；**D-11**（`:385`）改成"**无法判定（缺清单）**…已确证的只有 NAM-08/09/11/17 → 已确证 **4** 组"，不再写"12 组里 6 组" | 三条从"数"降为"清单/不可判定"，符合 t27 的要求（不可机械判定的不要写进棘轮基线） |
| ⑨ | §F/F-2 与 D-10/D-11 不得再引用"394/394" | **已关闭** | §F F-2（`:424`）拆成"文件名镜像 43/44"与"模块名不镜像 14 处有意"两句；D-10（`:384`）的判定改成"文件 stem 比对（**不是 `module_path`**）"；D-11（`:385`）不再有任何"一致"前提；`:519` 明写"**不要**引用 '394/394 一致'" | 与 ① 同源，一并关闭 |

**结论：9/9 已关闭**（其中 ② 附一条**落地范围条件**，见 §C.1；它不改变任何已修正的数字，也不属于我 t27 的 9 条）。

**我撤回的 finding：无。** 我 t27 的 9 条经复判**全部成立**；作者对③④的处理方式（降级 + 改名方向取我倾向 + 撤回合并修法）我完全接受，不需要我让半步。**我接受的证伪**：无（作者也没有提出反证——它在 `:25` 明确写"我明确不同意的：无"）。

---

## B. 合并方案的同步（我的产物）

`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（548 → **549 行**）按 t30 的裁定改了 13 处：

| # | 位置 | 改动 |
| --- | --- | --- |
| 1 | §1.1 crate 表 | `debug_method` 行的模块名 `evidence` → **`call_evidence`**（注明是 t30 的 D-9 要求） |
| 2 | §1.2 目录树 | `src/evidence/` → `src/call_evidence/`，`evidence.rs ← lib.rs` → `call_evidence.rs ← lib.rs` |
| 3 | §1.3 feature 表 | 特性名**保持 `evidence`**（D-9 管文件/模块名，不管特性名）；在该行注明"模块名叫 `call_evidence`" |
| 4 | §2.2 碰撞表 | 七个模块名列表与"无相撞"行同步为 `call_evidence` |
| 5 | §2.2 根代码片段 | `#[path = "call_evidence/call_evidence.rs"] pub mod call_evidence;`（仍在 `feature = "evidence"` 之下） |
| 6 | §3.1 引用表 | `xirang_debug_method::` 的目标写法 → `xirang_toolchain::call_evidence::` |
| 7 | §3.6 替换顺序 | 同上 |
| 8 | §4.2 `git mv` 列表 | `debug_method→src/call_evidence` |
| 9 | §4.4 宏路径 | `$crate::evidence::submit!` → `$crate::call_evidence::submit!` |
| 10 | §5.5 README 归并段 | 七段清单同步为 `build`/`run`/`call_evidence`/… |
| 11 | §10.1 `evidence` 行 | 从"⚠️ 需要 t25 裁定"改成"✅ 已裁定并同步：定名 `call_evidence`（中心类型是内核既有的 `CallEvidence`/`EvidenceKind`）；**`build_evidence` 是 mcp 侧那个文件的名字，用在这里不对**（`debug_method` 的内容不是构建产物证据）；备选 `observation`" |
| 12 | §10.1 `D-3` 行 + §10.2 | `39 处` → **34 处（发布面 28）**，并补上 examples/fixture 的 6 处豁免与身份理由（见 §C.1） |
| 13 | §10.1 `NAM-40` 行 | 改成引用 t30 修订后的 `NAM-40`/`NAM-43`（14 处四族），并明写"第一版的『394/394 一致、0 违规』已被证伪并撤回，本文任何句子都不以它为据" |

**自检**：该文件 `file:line` 引用 **48 处 / 0 violation**（我自己的锚点脚本）；`cargo test -p xirang-conventions --offline` = **99 passed / 0 failed**；文件内 3 个 ```rust 片段用仓库同款 `syn` 复验仍 `file=true stmts=true`。

---

## C. 这一轮**新发现**的（不属于 t27 的 9 条）

### C.1 D-3 的"34 处"里有 2 个目录装着**身份承载**文件 → 收平会改 `NodeId`

**事实**：谓词 A 的 34 处里，有 6 处在两个 example 宿主与 Studio 夹具内：

```
examples/control-button/src/control/object/button      ← examples/control-button/src/control/object/button/button.rs
examples/control-button/src/control/object/slider      ← .../slider/slider.rs
examples/control-button/src/control/registry_rule      ← .../registry_rule.rs
studio/tests/fixtures/node-editor/src/control/object/node_editor/object
studio/tests/fixtures/node-editor/src/control/object/node_editor/registry_rule
studio/tests/fixtures/node-editor/src/control/registry_rule
```

其中前两个目录里的 `button.rs`/`slider.rs` 是**注册面声明**（`crate::*_object!`），`file!()` 派生 `NodeId`，而 `NodeId` 被写进落盘 graft 记录、并被宿主 `examples/control-button/src/lib.rs:50`/`:52` 的类型化 `static_graft_plan!` 以常量引用。**收平 = 移动文件 = 改 `NodeId` = 旧记录与那条类型化计划失效**。

**结论**：D-3 的验收"按 A 收平后为 0"**只在发布面 28 内可达成**；对 examples/fixture 的 6 处必须显式豁免（或走一次记录迁移，那是 t26 §6 的流程）。这不是作者的数字错误（34/28 都对），而是**规则范围**缺一句限定——我在自己的合并方案 §10.1/§10.2 里已写明这条豁免，并把两个宿主整体放进 §9 的「不做」清单。

**建议给 t30 的一句话**（非阻塞）：D-3 的"机械判定"加上"∧ 目录路径不以 `examples/` 或 `studio/tests/` 开头"，或在该行注明"examples/fixture 的 6 处为豁免（身份承载）"。

### C.2 §G 的标号 `P2b` 与 NAM-07 表的谓词 `B` 是同名不同义（文档小混淆）

- NAM-07 的修订表（`audit-naming-review.md:69`）把"**直接 `.rs`（来自 inventory）**恰为 `{<dir>.rs}`"记为 **B**，结果 39；
- §G-1 的脚本（`:473`）把 `sorted(os.listdir(dp)) == [stem + '.rs']`（**目录内只有一个条目**）也打印成 `P2b 直接子项`，结果 **34**。

两者都自洽（我跑过：`listdir==1个条目` → 34；`直接 .rs 子项 == {dir.rs}` → 39），但**同一个标签 `P2b` 指了两个不同谓词**，而 39 恰恰是"不要引用"的那个数。建议把 §G 的 `P2b` 改名（如 `P2b′ 单条目`）并在两处交叉引用，否则下一个读者会以为 39 与 34 是同一个谓词的两次矛盾结果。**不影响任何采用中的数字。**

---

## 最终的判定

**命名报告可以作权威使用。**

- 9/9 findings 已关闭，且我是**用独立手段**（自己的谓词 + 逐行跑它 §G 的脚本核对输出 + 逐行核清单的存在性与签名）确认的，不是采信"已修"的自述。
- 修订后的五个数（`module_path` 14、无载荷目录 34/28、目录 105、裸动词 15、实现词根 8）与我的独立复算**逐项相等**。
- 唯一需要补的是 §C.1 的一句**范围豁免**（D-3 不改 examples/fixture 的 6 处，其中 2 处身份承载）——我已把它写进自己的合并方案，**不阻塞**这份命名报告作为命名权威用于下一轮的逐条批准；§C.2 是标签混淆的文档小修。

**我这一轮没有覆盖的**：没有复核 t30 修订中**未被我 t27 点到的段落**是否被顺手改动（我只逐条核了 9 条 findings 的落点 + 我引用的行段）；没有重跑 §G 的 `P7`（别名 grep）与 `P4`（`-> bool`）的**全量**输出（我核了它们的结果数 4 与 3，与第一版一致）。
