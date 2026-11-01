# t42 复判：R-4 第二轮（多站点豁免顺序无关性 + V40-01/02/03）

> 复核者 logic-adversary。只出报告，源码一行不改；写权限仅限本文件。
> 对象：`docs/audit-2026-09-28/audit-naming-ambiguity.md`（788 行）的 **R-4 段**。**R-4a/R-4b/R-4c 的谓词与数在上轮（t40）已逐字复算通过**；本轮只核 V35-02（硬阻塞）、V40-01/02/03 与反过火，并在作者动过数时重算。
> 方法：把报告里**原样抽取**的脚本（`:519-602`，落成 `/tmp/r4_from_report.py`）当成被测对象跑——**不是读描述、也不是照它的文字重写一份**；再自建夹具做顺序无关性探针。

> **探针产物说明（给下游的锚点检查器）**：本报告出现的 `/tmp/r4_from_report.py`、`/tmp/t42probe/*`、`/tmp/t41probe/*` 都是**临时夹具**，不在仓库内；它们的 `file:line` 只在"探针运行输出"的意义上成立，`conventions` 的仓库锚点检查器会（正确地）把它们判为不在树内。仓库内引用一律是仓库相对路径。

## 0. 判定

| 项 | 结论 |
| --- | --- |
| **V35-02** 多站点豁免（硬阻塞） | **已关闭**——真树三跑（无 env / walk / reverse）裁决逐字相同；自建两夹具两序均 **1 报 1 免**，报出的是 crate 级站点且带 `file:line`，豁免的是 `diagnostic::build` 且带 `file:line` |
| **V40-01** `app`↔`application` 登记 | **已关闭**——`ALLOWED_PAIRS` 已登记 + 一行理由；验收词已改写；登记是**外科式**的（只豁免该对，其余 6 对照报） |
| **V40-02** "与声明形式"半句 | **已关闭**——§5 正文的归因只剩动词表一项 |
| **V40-03** §6 算术 | **已关闭**——`275` 与 `296−275=21` 自洽 |
| 反过火 | **0 误报**（26 个正规名字，`metadata` 子串陷阱未触发） |
| 新发现 | **V42-01（low，非阻塞）**：改法表/落地顺序两行散文仍把 `application` 写进"改一侧"，与登记结论不一致；另有错字 `strudio` |

**最终判定：R-4 可作命名门禁的权威基线。** 签字范围见 §7。

---

## 1. V35-02：多站点豁免 —— **已关闭**（本轮亲手造探针）

### 1.1 我自建的探针（不依赖作者的夹具）

两组夹具，**用相反的创建顺序**造同构的两站点形态（crate 级 `pub mod build;` + `diagnostic::build`）：

- `/tmp/t42probe/mine`：先写 `src/lib.rs`（crate 级声明在第 5 行），后写 `src/diagnostic/diagnostic.rs`；
- `/tmp/t42probe/mine2`：先写 `diagnostic/diagnostic.rs`，后写 `src/lib.rs`（第 4 行）。

两组 × 两序（`SITE_ORDER=walk|reverse`）实跑**作者本轮的脚本**：

```text
### 我的探针夹具 /tmp/t42probe/mine  SITE_ORDER=walk
    [顺序=walk] build: 访问序 [('.../src/lib.rs', 5), ('.../src/diagnostic/diagnostic.rs', 2)] | 新逻辑(逐站点) 报出 1 处, 豁免 1 处 | 旧逻辑(只看首站点) 报出
[R-4b] 严格口径：违规声明处 1 违规名字 1 豁免声明处 1
     ('build', '/tmp/t42probe/mine/src/lib.rs', 5, '<crate root>')
    豁免: [('build', '/tmp/t42probe/mine/src/diagnostic/diagnostic.rs', 2, 'diagnostic')]
### 我的探针夹具 /tmp/t42probe/mine  SITE_ORDER=reverse
    [顺序=reverse] build: 访问序 [('.../src/diagnostic/diagnostic.rs', 2), ('.../src/lib.rs', 5)] | 新逻辑(逐站点) 报出 1 处, 豁免 1 处 | 旧逻辑(只看首站点) 豁免
[R-4b] 严格口径：违规声明处 1 违规名字 1 豁免声明处 1
     ('build', '/tmp/t42probe/mine/src/lib.rs', 5, '<crate root>')
    豁免: [('build', '/tmp/t42probe/mine/src/diagnostic/diagnostic.rs', 2, 'diagnostic')]
```

`mine2`（相反创建序）与两序同样给出 **1 报 1 免**，报出项恒为 `src/lib.rs`（crate 级）、豁免项恒为 `diagnostic/diagnostic.rs`。

**断言逐一核对**：① 两序结果相同 ✓（违规/豁免的**内容与归属站点**都不变，只有列表顺序变）；② **恰好报出 1 处**、即 crate 根那个，且带 `file:line`（`/tmp/t42probe/mine/src/lib.rs:5` / `/tmp/t42probe/mine2/src/lib.rs:4`）✓；③ **豁免 1 处**，也带 `file:line`（`/tmp/t42probe/mine/src/diagnostic/diagnostic.rs:2`，parent `diagnostic`）✓。这正是 t40 抓到的漏洞形态，现已修好。

### 1.2 作者夹具复核（`/tmp/t41probe/plan_lib` 仍在，两序实跑）

```text
-- SITE_ORDER=walk      [顺序=walk]    build: 访问序 [(...lib.rs,1), (...diagnostic.rs,1)] | 新逻辑 报出 1 处, 豁免 1 处 | 旧逻辑 报出
-- SITE_ORDER=reverse   [顺序=reverse] build: 访问序 [(...diagnostic.rs,1), (...lib.rs,1)] | 新逻辑 报出 1 处, 豁免 1 处 | 旧逻辑 豁免
[R-4b] 严格口径：违规声明处 1 违规名字 1 豁免声明处 1   （两序相同）
    豁免: [('build', '/tmp/t41probe/plan_lib/src/diagnostic/diagnostic.rs', 1, 'diagnostic')]
```

与它报告 `:689-713` 贴出的两段输出**逐行一致** ✓；"旧逻辑在 reverse 序下整体豁免"的对照也复现 ✓。

### 1.3 队长要求加强的那条：真实树三跑（这是比夹具两序更强的断言）

```text
unset    rc=0 -> R-4b: 违规声明处 26 违规名字 15 豁免声明处 1 | 放宽口径 19/36 | R-4c 6 对 + 登记豁免 1
walk     rc=0 -> 同上（逐字相同）
reverse  rc=0 -> 同上（逐字相同）
26 条违规站点集合：三次逐字节相同 ✓
豁免: [('build', 'core/src/registry_core/diagnostic/diagnostic.rs', 14, 'diagnostic')]   ← 带 file:line ✓
```

- **三次裁决相同** ✓（`SITE_ORDER` 只作用于探针里"站点列表的访问序"，`os.walk` 的真实扫描路径在所有运行里都没变——`sites` 的构造在三个运行里是同一段代码；我把这份抽取脚本的 `SITE_ORDER` 用法限定在 R-4b 的**逐名字打印/判定分支**上逐行确认过）。
- **逐站点计数**（不是"名字命中即整名放过"）✓：真实树里 `delete` 3 处、`parse` 4 处、`apply`/`edit`/`render` 各 2 处**全部逐处计入**，合计 26。**精度声明**：真实树里 `build` 只有 1 个站点（`diagnostic`），所以新旧口径在**真树上同值**（都是 26 处 / 15 名）——两者的差别只在多站点形态下显现，也就是 §1.1 的夹具（新逻辑恒 1 报 1 免，旧逻辑随顺序在"报出"与"整体豁免"之间摆动）。不要用真树的 26 去证明顺序无关性，要用夹具。
- **诚实边界**：真实树里 `build` 只有 1 个站点（`diagnostic`），所以真树本身**不能**证明顺序无关性；顺序无关性由夹具的区分性实验（§1.1/§1.2）证明，而真树三跑证明"探针开关不改变真实结果"。两者合起来才完整。

## 2. V40-01：`app`↔`application` 登记 —— **已关闭**

- **已登记**：`:527` `ALLOWED_PAIRS={("app","application")}  # V40-01：两侧都是好名字，登记豁免（理由见报告）`；理由在修订记录 `:36`（"两侧都是好名字、分属不同层级/crate，不是同一概念的两个拼法"）✓。
- **验收词已改写**：`:725` R-4c 行 → "违规对数 = 0（改名落在更短/更泛的一侧，**或** 已登记且附一行理由）" ✓（与我 t40 的建议一致）。
- **登记是外科式的**（这点必须实测，防"顺手放过真违规"）：真实树实跑 → 违规对 **6**（`app↔apply`、`ide↔identity`、`json↔jsonl`、`lex↔lexicon`、`mir↔mirror`、`render↔renderer`）、**登记豁免 1**（`app↔application`）、单复数豁免 7 ✓。即：只有被登记的那一对被放过，其余 6 对与 7 对单复数族一个不少。
- 路径核实：`app`@`studio/studio/studio.rs:5`，`application`@`core/src/registry_core/syntax/entries.rs:13`——两侧都确实存在且是好名字 ✓。

## 3. V40-02：归因半句 —— **已关闭**

`grep -n "声明形式"` 全文只剩 **1 处**：`:37`（修订记录里**描述这次修改本身**的那行，必须留着）。§5 正文 `:727-729` 已改为"差额**全部来自动词表**——`graft` 少计 7 处、`walk` 多计 1 处，净 −6 → 33−6=27；inline `mod X {` 对 R-4b 的贡献是 **0 处**" ✓——与我 t40 的复算结论一致。

## 4. V40-03：§6 算术 —— **已关闭**

`:776`："296 个模块名里的 **275** 个（只与谓词比对；被点名的 21 个逐个读过，**296−275=21**）" ✓ 自洽，且与我的复算（被点名的模块名恰 21 个）一致。

## 5. 反过火：**0 误报**（本轮再用作者脚本跑一遍）

**夹具** `/tmp/t42probe/legit`：26 个正规角色名词（`identity declaration tree syntax authoring diagnostic requirements plugin source release lexicon entry_pages renderer verifier admission deployment scope locals inspection snapshot validation manifest overlay metadata fixtures trace`），按本仓的 `#[path]` + `pub mod x;` 形态挂载，跑**作者本轮的脚本**：

```text
[ROOT=/tmp/t42probe/legit] 枚举: 模块名 26 声明处 26
[R-4a] 末 token ∈ BAD_TAIL：名字 0 声明处 0
[R-4b] 严格口径：违规声明处 0 违规名字 0 豁免声明处 0
[R-4c] 违规对 0 []   登记豁免 0 []   单复数族豁免 0 []
```

**误报数 0** ✓（两序都一样）。子串陷阱专项：

```text
metadata       toks=['metadata']            末 token ∈ BAD_TAIL -> False   ← 不误报 ✓
metadata_store toks=['metadata','store']    末 token ∈ BAD_TAIL -> False   ← 不误报 ✓
test_support   toks=['test','support']      末 token ∈ BAD_TAIL -> True    ← 该报 ✓
face_helpers   toks=['face','helpers']      末 token ∈ BAD_TAIL -> True    ← 该报 ✓
```

即"`metadata` 含 `data`"不会误报，而"真类别名后缀"照报——**token 精确匹配**的设计是对的。

## 6. 新发现 **V42-01（low，非阻塞）**：改法表与落地顺序两行散文滞后于 V40-01

- `:749`（改法表）仍写："`app`↔`apply`/`application`：**strudio** 的 `app` 与 mcp 的 `apply` 至少一个加限定"；
- `:759`（落地顺序）仍写："`app`↔`apply`/`application` **各改一侧**"。

两处都把已登记豁免的 `application` 拉回"要改一侧"的动作里，与 `ALLOWED_PAIRS` 及 `:725` 的新验收词**自相矛盾**（门禁值不受影响，只是散文会把执行者引向多余改名）；另 `strudio` 是错字（应为 `studio`）。
**建议**（不阻塞）：两行改为"`app`↔`apply`：至少一侧加限定；`app`↔`application` **已登记豁免，不动作**"，并修错字。

## 7. 签字范围（本轮通过 = 我签了哪些、哪些沿用）

**我本轮亲手复算/探针验证的**：
1. **R-4 脚本原样抽取后重跑**（真实树 ×3：无 env / walk / reverse）→ 枚举 296/440、R-4a 14/14、R-4b **26 处/15 名/豁免 1 处**、放宽 19/36、R-4c 6 对 + 登记 1 + 单复数 7；**三次逐字相同**、26 条违规站点集合逐字节相同、豁免项带 `file:line`。
2. **多站点顺序无关性**：自建 `/tmp/t42probe/{mine,mine2}`（相反创建序）× 两序 → 恒 **1 报 1 免**，报出 crate 级站点、豁免 `diagnostic::build`；与作者 `/tmp/t41probe/plan_lib` 的两序输出**逐行一致**；"旧逻辑 reverse 整体豁免"的对照复现。
3. **逐站点计数**：真实树 26 处（`delete` 3、`parse` 4、`apply`/`edit`/`render` 各 2…）而非按名字 15。
4. **V40-01/02/03 的文本与语义**（`:527`/`:36`/`:725`；`声明形式` 仅存于修订记录；`:776` 的 296−275=21）。
5. **反过火**：26 个正规名字 0 误报 + `metadata` 子串陷阱专项。

**沿用上一轮（t40）未经我重算的**：R-4a/R-4b/R-4c 三条谓词**语义的合理性**（本轮复跑只证明结果稳定，不重新论证"该不该这么定"）；§1 各 AMB 的现象与严重度（t35 复核）；§2 三个点名案例与 §3 七个模块名判定（t35 通过）；831 公开名的口径与 `*_tests.rs` 计数（t39/t40 核对）；`audit-inventory.json` 之外的身份/合并影响面判断（t32/t35）。

**未发现其它阻塞项**；V42-01 是散文层面的滞后，不影响门禁判定。

**R-4 可作命名门禁的权威基线。**
