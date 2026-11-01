# t40 复判：`audit-naming-ambiguity.md` 的 R-4 修订（t39）

> 复核者 logic-adversary。只出报告，源码一行不改；写权限仅限本文件。
> 复判对象：`docs/audit-2026-09-28/audit-naming-ambiguity.md`（672 行）的 **R-4 段**（`:472-648`）及其连带项。**§3（七个模块名）与 §4（不是缺陷清单）上一轮已判通过，本轮不重判。**
> 方法：**先逐字复刻 t39 的谓词脚本**在真实树上跑（`/tmp/t40-replay.txt`），再用**自造最小输入**探它没测的分支（`/tmp/t40probe/`），最后跑反过火探针。所有结论都有命令与输出。

## 0. 复判一句话

| 你的 finding | 结论 |
| --- | --- |
| **V35-01** 漏名 | **已关闭（且比我提的更全）**——我漏了 `test_support`，它补上了 |
| **V35-02** `build` 豁免判据（造探针） | **未关闭（阻塞）**——文档判据已改对，但**实现是按"名字"判、不是按"站点"判**：站点多于一个时结果取决于 `os.walk` 顺序，crate 级 `build` 仍可能被静默豁免 |
| **V35-03** 名字数 vs 站点数 | **已关闭**——三个指标各自给谓词/现状/验收，我逐字复算全部命中（296/440、14/14→12/12、16/27→15/26、7 对） |
| 5 条 medium/low | **全部关闭**（逐条给行号） |
| 反过火 | R-4a/R-4b **0 误报**（26 个正规名字）；R-4c **7 对中 1 对建议登记豁免**（新 finding V40-01） |

**最终判定：仍不可用（列阻塞项）**——只剩 **V35-02 一条硬阻塞**，修法是脚本里 3 行的**逐站点判定**（见 §2.3）；另加 **V40-01**（R-4c 的 `app↔application` 建议登记豁免，非阻塞）。修完即可作权威基线。

---

## 1. V35-01 漏名：**已关闭**

**证据（复刻它的脚本，`/tmp/t40-replay.txt` 原样）**：

```text
[R-4a]
  顶层目录: ['run_method', 'debug_method', 'build_method']
  包名: ['xirang-run-method', 'xirang-build-method', 'xirang-debug-method']
  lib名: ['xirang_run_method', 'xirang_build_method', 'xirang_debug_method']
  模块名: ['data', 'face_helpers', 'misc', 'support', 'test_support']
      data: [('studio/studio/ui/graph.rs', 20, 'graph')]
      face_helpers: [('run_method/macros/face.rs', 7, 'face')]
      misc: [('studio/studio/app/state/state.rs', 14, 'state')]
      support: [('studio/studio/app/app.rs', 34, 'app')]
      test_support: [('build_method/renderer.rs', 32, 'renderer')]
  合计 名字 14 / 声明处 14
```

- 我 t35 点名的 4 个（`support`/`misc`/`data`/`face_helpers`）**逐条命中，路径与行号与我一致** ✓。
- **它多找到 1 个我漏的**：`test_support`（`build_method/renderer.rs:32`）。我实测该行确为 `#[cfg(test)] pub(crate) mod test_support {` ✓——**撤回我 t35 "漏 4 名"的表述，正确数是 5 名**（我当时的扫描把 inline 形式与 `pub(crate)` 前缀都漏了）。
- `mir`↔`mirror` 已列入：R-4c 的 7 对里第 6 对是 `mir↔mirror`（`core/src/registry_core/mir/mir.rs:1` vs `macro/src/mirror.rs:1`）✓，且 7 对的**两侧都真实存在**（两侧定义处见 §5.1 的表，逐条由我扫描得到）✓。
- "净违规 17" 已删除，改为三指标分别给数 ✓（见 §3）。
- **根因披露诚实**：修订记录（`:14-43`）写明上一版贴出的"本次输出"取自更早一次**只判首 token** 的运行、规则从未真正执行——这是与 t30（我自己的 D-3 谓词配错）**同型**的失误，它主动写出来了，认可。

## 2. V35-02 `build` 豁免判据：**未关闭（唯一阻塞项）**

### 2.1 文档层面：已改对

`:485-489` 撤销"名字全等豁免"，改为 `ALLOWED_QUALIFIED = {("diagnostic","build")}` 的 **(父模块, 名字) 对**；`:494-497` 给了 4 行自测。方向正确 ✓。

### 2.2 探针：**实现按"名字"判，不是按"站点"判**

看脚本的主分支（`:550-557`）：

```python
ss = sites.get(x, 1)
if isinstance(ss, list) and (ss[0][2], x) in ALLOWED_QUALIFIED:   # ← 只看第一个站点
    ex.append((x, ss)); continue
bn.append(x); bs += len(ss)                                     # ← 整个名字一锅端
```

我造了最小输入（`/tmp/t40probe/`：`kernel/src/lib.rs` 里 crate 级 `pub mod build;` + `kernel/src/diagnostic/diagnostic.rs` 里 `mod build;`，正是合并方案 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:194` 落地后的状态），跑它的分支：

```text
[探针A] 真实 walk 顺序（lib.rs 在前）
  sites['build'] = [('kernel/src/lib.rs', 5, '<crate root>'), ('kernel/src/diagnostic/diagnostic.rs', 2, 'diagnostic')]
  豁免: []   报出: ['build'] 声明处 2      → crate 级 build 被报出 ✓
[探针B] 交换顺序（diagnostic 在前，模拟 walk 顺序不同）
  豁免: ['build']   报出: []               → crate 级 build 被静默豁免 ✗
[探针C] 逐站点判（应有的语义）
  kernel/src/lib.rs:5                parent=<crate root>  build -> 违规
  kernel/src/diagnostic/diagnostic.rs:2 parent=diagnostic  build -> 豁免
```

**结论**：`("build", <crate root>) → 违规` 只在"这个名字的第一个站点恰好是 crate 级"时成立。站点 ≥2 时**结果由 `os.walk` 顺序决定**——同一个输入两次运行可能给出不同答案，而 `conventions` 门禁必须是确定性的。

**它的自测为什么没抓到**：`:570-572` 的自测是

```python
for name,parent in [...]: verdict="豁免" if (parent,name) in ALLOWED_QUALIFIED ... 
```

它测的是**单输入判定函数**，没有测 `:550-557` 的**扫描分支**（多站点合并）——所以自测 4 行全绿，而真跑多站点时判错。这是"测了函数、没测数据流"的典型。

### 2.3 修法（3 行，且验收口径本来就是按站点的）

把豁免下沉到**站点**粒度：

```python
for x, ss in sites.items():
    if len(toks(x)) != 1 or toks(x)[0] not in VERBS: continue
    for (p, i, parent) in ss:
        if (parent, x) in ALLOWED_QUALIFIED: ex.append((x, p, i)); continue
        bn.append((x, p, i))          # 逐站点报出
print("声明处", len(bn))              # 与 R-4b 的"声明处数 = 0"直接对应
```

同时把自测从 4 行**单点**扩成 1 个**多站点**用例（crate 级 + `diagnostic` 级同时存在 → 断言"恰好报出 1 处、豁免 1 处"）。R-4b 的验收词已经从"名字数"改成"**声明处数 = 0**"（`:609`），逐站点判定与之一致；现在的实现反而与验收口径不匹配。

## 3. V35-03 名字数 vs 站点数：**已关闭**

**我复刻脚本得到的三个指标，与它的报告逐字一致**：

| 指标 | 它的报告（`:606-610`） | 我的复算 | 一致 |
| --- | --- | --- | --- |
| 枚举规模 | 296 名字 / 440 声明处 | **296 / 440** ✓（含 inline `mod X {`；我 t35 的 271/339 只扫 `;`，是更窄口径） | ✓ |
| R-4a | 命中 14/14 → 净 **12 名字 / 12 声明处** | 14/14，同一批 5 个模块名 → 净 12/12 ✓ | ✓ |
| R-4b 严格口径 | 命中 16 名字 / 27 声明处 → 净 **15 / 26** | 16/27，`build` 以父路径对豁免 → 15/26 ✓ | ✓ |
| R-4b 放宽口径 | 19 名字 / 36 声明处 | **19 / 36**（加 `graft`/`plan`/`record`/`pass`）✓ | ✓ |
| R-4c | 违规 **7 对** / 13 名；单复数豁免 7 对 | 7 对 / 7 对，逐对两侧存在 ✓ | ✓ |

**"违规 → 0" 现在机械可判定** ✓：R-4a 按名字数、R-4b 按声明处数、R-4c 按对数，三个验收词互不混淆。

**一处表述要修（不影响数字）**：`:612-614` 把我 t35 的 33 处与本文的 27 处之差归因于"动词表**与**声明形式（两种 `mod` 形式都计入）"。我实测：**差额全部来自动词表**——`graft` 少 7 处、`walk` 多 1 处（33 − 7 + 1 = 27）；R-4b 那 16 个名字的 27 处**全部是 `mod X;` 形式，inline `mod X {` 贡献 0 处**。数字对，归因要去掉后半句（否则读者以为换口径能再变一批数）。

## 4. 5 条 medium/low：**全部关闭**

| 原 finding | 状态 | 证据（行号） |
| --- | --- | --- |
| ① `support`/`misc` 与 §D 重复计数 | **关闭** | `:23`（修订记录）、`:497`（自测注释）、`:608`（R-4a 的"扣除"列写明 **= `audit-naming-review.md` D-1 的 NAM-01/NAM-02，同一对象只算一次**）、`:624`（改法表不重复给建议）✓ |
| ② D-9 与 R-2 两张表 | **关闭** | `:437-441`：标题即写"与 `audit-naming-review.md` 的 D-9 **合成一张**"，并用引言说明两表**谓词对象不同**（D-9 = 文件 stem；R-2 = 命名空间名）且"同一对象只计一次"，还给了落地建议（落成 `conventions` 能读的一份常量，`lexicon` 有先例）✓ |
| ③ 方案引用点名文件 | **关闭** | `grep -c "方案 \`:" = 0`；`audit-publish-surface-merge-plan.md` 出现 **21 次**（含 `:194` 的 `pub mod build;`、`:40-43`、`:89-93`、`:122-126`）；我 t35 的"未点名文件"已消除 ✓ |
| ④ AMB-06 降 MINOR | **关闭** | `:26`（修订记录逐条）、`:52`（§1 标题改为 **MAJOR 5 条 / MINOR 15 条**——我复算它的严重度行：MAJOR = AMB-01…05，MINOR = AMB-06…20）✓ |
| ⑤ 831 谓词与 8-vs-13 feature 对象 | **关闭** | `:46` 给出 831 的精确正则、范围（10 crate 的 `src/**/*.rs`，**含** `*_tests.rs`；不含 `tests/`/`examples/`/`benches/`）、逐 crate 分解、以及排除 `*_tests.rs` 后的 **829**；`:653` 写明 feature **8** = "12 个清单去重并集（含 `default`）"、方案的 **13** 是另一对象；`:44` 写明模块名 296/440 改用含 inline 的新口径 ✓（我复算 831 的分解：core 479 + run_method 124 + build_method 62 + plugin-host 50 + conventions 49 + studio 44 + debug_method 13 + macro 4 + cli 4 + mcp 2 = 831 ✓ 加总正确） |

## 5. 反过火：**0 误报**

**探针**：取 26 个正规模块名（`identity` `declaration` `tree` `syntax` `authoring` `diagnostic` `requirements` `plugin` `source` `release` `lexicon` `entry_pages` `renderer` `verifier` `admission` `deployment` `scope` `locals` `inspection` `snapshot` `validation` `manifest` `overlay` `metadata` `fixtures` `trace`）跑 R-4a/R-4b 两条谓词：

```text
抽样 26 个；R-4a/R-4b 误报: 0
```

- **token 精确匹配是对的**：`metadata` 含子串 `data`、`test_support` 含 `support`，但 `toks()` 分词后 `metadata` 的末 token 是 `metadata` 而非 `data`，所以**不会**因"含类别词"被误报 ✓（这是它的谓词写得比我 t35 的直觉更稳的地方）。
- **R-4c 对好名字**：`identity`/`lexicon`/`renderer` 各参与一对，改法表把修改落在**短侧**（`ide`→`ide_mirror`、`lex`→`lexer`、`render`→`renderer`）✓。
- **但我要撤回上一版报告里"R-4c 不会误伤好名字"这句**——见下面的新 finding。

### 5.1 新 finding **V40-01（pair 级过火，MINOR）**：7 对里有 1 对建议登记豁免而不是改名

逐对核两侧的**真实定义处**（我的扫描输出，非引述）：

| 对 | 短侧定义处 | 长侧定义处 | 我的判定 |
| --- | --- | --- | --- |
| `app` ↔ `application` | `studio/studio/studio.rs:5`（Studio TUI 的 app） | `core/src/registry_core/syntax/entries.rs:13`（`application!` 宏的条目） | **过火**：跨 crate、跨领域，两侧都不是坏名字；`app` 是 TUI 的规范叫法，`application` 是宏的领域词。建议**登记豁免**，不要为消对而改名 |
| `app` ↔ `apply` | 同上 | `mcp/lib.rs:67` + `core/src/registry_core/tree/graft_ops/record.rs:40` | 真近名（一字母之差，同一工作区词汇）✓ 保留 |
| `ide` ↔ `identity` | `build_method/renderer.rs:21` | `core/src/registry_core.rs:18` | 真近名（`ide` 单独作模块名过泛）✓ 保留 |
| `json` ↔ `jsonl` | `core/src/registry_core.rs:20` + `cli/explain.rs:34` | `core/src/registry_core/mir/mir.rs:23` | 真近名（AMB-12 已单独立项）✓ 保留 |
| `lex` ↔ `lexicon` | `core/src/registry_core/source/source.rs:11`（词法扫描） | `core/src/registry_core.rs:22`（共享文本契约） | 真近名（`lex`→`lexer` 的改法正确）✓ 保留 |
| `mir` ↔ `mirror` | `mcp/lib.rs:103` + `core/src/registry_core.rs:24` + `debug_method/src/lib.rs:23` | `macro/src/lib.rs:59` | 真近名（我 t35 的 V35-07）✓ 保留 |
| `render` ↔ `renderer` | `core/src/registry_core/mir/mir.rs:29` + `run_method/authoring/manifest/face/face.rs:18` | `build_method/lib.rs:65` | 真近名 ✓ 保留 |

**结论**：R-4c 的谓词（前缀 + 剩余无下划线 + 非复数）命中 7 对，其中 **6 对是真近名、1 对（`app↔application`）建议走登记豁免**。谓词本身不必改，但验收口径要补一句："违规对数 = 0"的实现方式是「**在更短/更泛的一侧改名**」**或**「**在 `ALLOWED_PAIRS` 里登记 + 一行理由**」——否则未来有人为了过门禁把 `app` 或 `application` 改名。这与我 §D 的 D-8（别名同名同义、需一行说明）是同一纪律。
- 另 `:634` 有个错字 `strudio` → `studio`（纯文字，顺手改）。

## 6. 两条低优先级备注（不影响关闭判定）

1. **§6 的覆盖率算术**：`:661` 写"296 个模块名里的 270 个（只与谓词比对；被点名的 21 个逐个读过）"——296 − 270 = 26 ≠ 21。按我的复算，"被点名的**模块名**"确实是 21 个（R-4a 的 5 + R-4b 的 16 + R-4c 的 13 − 与目录/包/lib 的 9 个重名 − 重叠），建议把 270 改成 275 或把 21 改成 26，让减法自洽。
2. `:608` 的括号算式 "目录 3 + 包 3 + lib 3 + 模块 3 = 12" 正确 ✓（我核过加法）。

---

## 7. 结论

- **V35-01 已关闭**（并撤回我"漏 4 名"的表述：正确是 5 名，`test_support` 是我漏的）。
- **V35-03 已关闭**（三指标、逐项复算一致；仅一处归因表述要修）。
- **5 条 medium/low 全部关闭**（逐条行号）。
- **反过火**：R-4a/R-4b 在 26 个正规名字上 **0 误报**（token 精确匹配避开了 `metadata` 这类子串陷阱）；**R-4c 7 对里 1 对（`app↔application`）是 pair 级过火**，建议登记豁免（V40-01，非阻塞）。
- **V35-02 未关闭**：文档判据已对，**实现仍按名字判**——多站点时 `build` 是否被报出取决于 `os.walk` 顺序（探针 A/B 给出相反结果），门禁会**不确定**；它的 4 行自测只测了判定函数、没测扫描分支，因此漏掉了这一点。

**仍不可用（列阻塞项）**：唯一阻塞项是 **V35-02 的逐站点化**（脚本 3 行 + 1 个多站点自测用例；R-4b 的验收词"声明处数 = 0"已经为此准备好了）；建议同时采纳 **V40-01**（给 `ALLOWED_PAIRS` 留一个登记位）。**按 §2.3 修完后，R-4 即可作命名门禁的权威基线**——R-4a/R-4b/R-4c 三指标的现状值、谓词与脚本我都已逐字复算通过，无需再动。
