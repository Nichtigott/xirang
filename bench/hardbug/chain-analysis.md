# 逐题链路实证：两个工具给了什么 → agent 怎么想 → 为什么这么做

来源：两臂会话原文（`data.message` 的 reasoning 块 + `tool/result` 正文）逐题抽取，脚本
`target/hardbug/chains2.py`，产物 `target/round7/chains2-{mcp,cg}/<q>.md`。下面是 r4（陷阱题）与
s1（两臂打平）的实录，以及由此得出的引导结论。

## r4（我们 8 次仪器调用 / cg 2 次；真正的工作步两边都 ≈8）

**mcp 的链**（原文节选）
1. `bash find/wc`（摸底文件与行数）
2. **`check`** → 回复里不但有 `root/rust_files=13/functions=24/faces/exit=0`，还**带了 `entry.rs:1-41` 的
   源码窗口**。它的推理当场立住：*"a zero entry is being posted when it shouldn't be… `postable()` isn't
   correctly rejecting zero amounts. The docu…"* —— **它是从工具回复里读到契约的**。
3. **`callgraph postable`** → `matches 1 src/model/entry.rs:38 fn postable` + `callers (1):
   src/store/memory.rs::post` + `source: /// Whether this entry may be posted at all…`（**契约文档首行**）。
   推理：*"I found the actual defect — it uses OR instead of AND…"*
4. `read entry.rs offset 28`（看确切那一行）
5. `edit` 成 `has_receipt && amount != 0`
6. `cargo test` → 9 passed
7. **构造反证**：`|| amount != 0` ⇒ 2 failed；只留 `has_receipt` ⇒ 1 failed。推理：*"Both quick fixes cause
   test failures… unless the trap is about something else"* ⇒ **继续验证**。
8. `check`（终态门）

**cg 的链**
1. `cargo test`（看失败）
2. **`codegraph explore -p r4-cg "Store post entry postable zero amount"`** → *Found 20 symbols across 4
   files* + **blast radius**（一次模糊检索给出候选集）
3. `read entry.rs offset 29 limit 13`（看到同一段契约）
4. `edit`（同样的正解）
5. `cargo test` → 绿
6. `grep -rn postable` + `git diff --stat` ⇒ **只读证据，没有构造反证**
7–8. 追加 VERIFY 段、写答案

**差异与原因（关键）**
- **我们的工具赢在一处：答案自带契约**（`callgraph` 印定义处 doc 首行、`check` 带源码窗口）⇒ agent
  **引用了契约**，因此知道"只翻一个算符不够"。cg 得靠一次额外的 `read` 才拿到同样的信息。
- **cg 赢在一处：`explore` 一次给候选集 + blast radius**；我们没有等价入口（只能 `search` 字面量 ⇒
  猜名字 ⇒ 2–3 跳）。
- **我们多出来的"步"大部分不是探索**：① 交付行数上限（≤30 行）逼它**改写答案 3 次**；② 每次桥调用都要
  经一个 bash 包装脚本来记账 ⇒ **每次调用多一步**；③ 终态门重跑；④ 反证构造（**这是质量，不是浪费**）。
  ⇒ 若把 ① 改成结构化字段、② 改成桥自带 `--log`，我们的"步数/调用数"会当场掉一大截。

## s1（两臂都 3 次仪器调用，打平）

- **mcp**：`status`（成员/树/面 + "非默认特性不可能在默认面失败"的提示）→ `callgraph post`（**直接给出
  `callers (1): crates/report/tests/report.rs::store`**）→ 再去 `cat` 测试文件确认调用点。
- **cg**：`init` + `callers post`（*Callers of "post" (1): function store
  crates/report/tests/report.rs:11*）→ 再 `explore "who calls Store::post …"` 复核一次。
- ⇒ **两边都一个仪器调用就能答**；差别只在记账与复核习惯。**我们的 `callgraph` 已经给了"调用者在另一个
  成员 + 是否测试"**（这一点与第六轮点名的缺口相反，值得记下：它已经修好了）。

## 引导结论（怎么让 AI 做对的事）——每条都给证据与预期

| # | 引导 | 证据 | 预期 |
| --- | --- | --- | --- |
| G1 | **契约必须永远随答案走**：`callgraph`/`explain`/`why`/`digest` 都印"定义处 doc 首行 + 契约行"，并标明**哪句是契约、哪句是实现** | r4 两臂都是**读到契约**才没走捷径；cg 多花一次 `read` | 陷阱题一次到位；少一次 `read` |
| G2 | **加 `locate <症状串>`**（模糊检索） | cg 的 `explore` 一次给 20 个符号 + blast radius；我们靠 `search` 字面量猜名字 | r4/r2/r3/s7/s8 各少 1–2 次调用 |
| G3 | **桥自带 `--log`**（把每次调用与回复写进指定文件），取消逐调用 bash 包装 | 两臂都为记账写包装脚本；mcp 每次桥调用多一步 | **每次仪器调用少 1 步**（113 → ~80 步档） |
| G4 | **交付格式改成结构化字段**，不要"≤30 行" | mcp 为凑行数**改写答案 3 次** | 每题少 2–3 步；也避免为凑行数删内容 |
| G5 | **流程表写明"手上有症状 ⇒ 先 `locate`/`why`，不要先 `check`"** | r4 的 `check` 是**跑整套测试**，而失败已在手；它自己先跑过 `cargo test` | 省一次重跑套件的调用与它的整篇回复 |
| G6 | **把"构造反证"写进流程表与判分口径** | mcp 臂**自发**构造两条捷径反证并因此赢下 r4；cg 只读证据 | 让"构造"成为标准动作，而不是靠成员自觉 |
| G7 | **答案避开"空话式自证"**：要求"命令 + 原始输出 + 退出码"，而不是"我已验证" | 两臂都在末尾追加 VERIFY 段（好习惯），但我们的臂为它多写了脚本 | 记账成本下降，证据强度不降 |

## 一句话

**我们的工具已经能做到"答案自带契约"（这是 r4 能赢的机制）；缺的是"一次给候选集"（locate）与
"记账不占步"（自带日志/结构化交付）。引导上要把"先 locate、见契约、构造反证"变成流程表里的默认动作。**
