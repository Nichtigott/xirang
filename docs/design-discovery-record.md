# 发现记录：让 N 个碎片只付 1 份发现（③A 的设计）

**English (short).** Design for the batch's ③A: today every generated package's build script runs the whole
pipeline over the host root, so a partition *multiplies* the build-script cost (measured: 3.28× total CPU,
while rendering — the only part a package does differently — is 3%). The fix is a **tree-wide discovery
record**: discovery and analysis are computed once per tree fingerprint and published as a content-addressed
record; each package reads it, applies its own shape, and renders. Measured support: 7 of the 11 artifacts a
package writes are byte-identical across host / fragment / facade, and the freshness proof this design relies
on is already cheap — walking 1003 files and hashing all of them costs 0.06 s.

---

## 1. 问题（实测，2026-10-09）

量器：`target/xirang-bench/build-cost/`（1003 个 `.rs`、7.9 MB、1001 个面；直接跑构建脚本二进制；3 次重复，抖动 ±1%）。

| 形状 | cold | warm |
| --- | --- | --- |
| 单 crate 宿主 | 5.43 s | 3.38 s |
| 碎片（幽灵，认领 `control`） | 6.89 s | 3.63 s |
| facade | 5.68 s | 3.80 s |

一个碎片 ≈ 一个整宿主（1.27×），三个包合起来 **17.8 s vs 5.43 s = 3.28×**，而它渲染的那一份只值 **0.19 s（3%）**。
10k 面时这一段占构建墙的 **73.3%**（103.885 / 141.635 s，`scale-logs/compile-wall.json`）⇒ 分片不拆发现，分布式无从谈起。

（同一量器在 2026-10-10 重跑时读到 cold 7.3 / 6.8 / 9.2 / 7.8 s、warm 4.5 / 4.6 / 4.6 / 5.1 s——机器更忙；**跨时段只比比例，绝对值看同一张表内部**。）

### 1.1 钱花在哪：两个量器

**外部量器**（只量 I/O）：走树 + stat 1003 个文件 **0.020 s** · 读全部文件 + sha256 **0.043 s**（`find … | xargs sha256sum`，各 3 次）⇒ **I/O 不是墙**。

**管线自报的逐相位表**（阶段 0 加的 `xirang build phases:` 一行；同一棵 1001 面的树，一次 cold（先删发现缓存）一次 warm；**各相位之和 = `total`，逐位对上** ✓）：

| 相位 | cold | warm | 这一相里是什么 |
| --- | --- | --- | --- |
| `discover` | 0.081 s | 0.103 s | 走树 + 逐文件指纹 + 身份缓存 |
| `scope` | 1.709 s | 0.528 s | 入口/范围解析 + **unit 缓存的写与读**（面解析的主要落点） |
| `declare` | 3.826 s | 1.691 s | 语法检查 + requirements / contracts / registry_rule / stable_name / parent_macro（**多遍重解析整棵树**） |
| `plan` | 1.471 s | 0.620 s | static plan + 切口记录与计划文件的交叉校验 |
| `shape` | 0.063 s | 0.062 s | 读声明 + 规划挂载与壳 + 属主 |
| `render` | 0.491 s | 0.235 s | 渲染 `generated_lib.rs` |
| `write` | 1.575 s | 1.384 s | **推导**并写 11 份产物（`write_*_manifest` 都从 nodes 重新算行，不是纯 I/O） |
| **total** | **9.218 s** | **4.624 s** | |

（同一量器在还没有这一行的早先一版里读到 cold 5.4 / warm 3.4 s——本次机器更忙，**绝对值只在本表内可比**，跨时段只比比例。）

三条读数：

1. **"解析"散在三个相位里**：cold−warm 的差分别是 `scope` +1.18 · `declare` +2.14 · `plan` +0.85 ⇒ unit 缓存只覆盖其中一条路，其余几遍照旧全量解析 ✓（与下面那 ≈20 个 `parsed_face` 调用点一致）。
2. **`write` 一个相位就占 warm 的 30%（1.38 s）**，而它大半是**推导**（`write_pruning_manifest` / `write_graph_manifest` / … 都从 nodes 重新算行）⇒ 7 份树级产物每包重算重写一遍，正是记录该接管的那部分。
3. `render` 只有 0.24 / 0.49 s、`discover` 0.1 s ⇒ **真正按包计的只有渲染这一小块**。

`parsed_face(...)` 在非测试代码里有 **≈20 个调用点**（`face_syntax_check.rs` 3 处、`manifests.rs` 5 处、`contracts.rs` 2 处、`static_plan.rs`、`scope_faces.rs`、`node_identity.rs`、`face_view.rs`、`graft_view/matching.rs`、`renderer/aliases.rs` …），而**只有 `registration_phase::aggregate` 那一条路吃 unit 缓存**（`registration_phase.rs:92`，由 `face_syntax_check.rs:20` 的 `cache_units` 传入）⇒ 每个包把这棵树解析了**很多遍**，其中一遍被缓存。

### 1.2 逐包产物对照（同一棵树，三个包各写自己的 OUT_DIR）

| 产物 | host / 碎片 / facade | 大小 |
| --- | --- | --- |
| `file_manifest.tsv` | **逐字节相同** | 46 KB |
| `function_manifest.tsv` | 逐字节相同 | 21 B |
| `graph_edges.tsv` | 逐字节相同 | 171 KB |
| `pruning_manifest.tsv` | 逐字节相同 | 313 KB |
| `shape_manifest.tsv` | 逐字节相同 | 132 KB |
| `discovery.fingerprint` · `add-crates.lock` | 逐字节相同 | 32 B / 201 B |
| `generated_lib.rs` | **不同** | 5.8 KB / 1.47 MB / 637 KB |
| `source_scope.tsv` | **不同** | 71 B / 104 KB / 71 B |
| `graph.generation` | 不同（只有代号/摘要） | 214 B |

⇒ 11 份产物里 **7 份是树级的**（合计 ≈663 KB，被写了三遍），**只有 2 份与形状有关**（`generated_lib.rs`、`source_scope.tsv`）+ 1 份是戳。
⚠️ 注意 `graft_plan.tsv` 在这棵树上相同只是因为**本次宿主没有声明任何切口**；声明了切口的树里它按包归属（`tools/xirang-real-usage` 的四条断言正是守这个），所以它**不能**进记录。

---

## 2. 目标与验收

**目标**：N 个包的构建脚本 = **1 份发现/分析** + 各自那份渲染与落盘。

**验收（数字）**：同一棵 1000 面树、同一台机器、同一量器下：

- 单包 cold 从 **9.2 s（碎片）/ 6.8 s（宿主）** 降到 **≤1.5 s**（读记录 + 按形状投影 + 渲染 + 拷贝树级产物 + 写与形状有关的那两份）；warm 从 **4.6 s** 降到 **≤0.8 s**；
- 三包合计 cold 从 **23.8 s** 降到 **≈14 s**（= 1 份分析 9.2 s + 3 份 ≤1.5 s），warm 从 **14.3 s** 降到 **≈7 s**。

**⚠️ 诚实边界：记录买到的第一样是 CPU 与内存，不是单机墙时间。** 今天三个包的构建脚本是**并发**跑的，每个都是单线程 ⇒ 核数 ≥ 包数时，**墙已经 ≈ 单个包的分析**（冷启 ≈9 s），三个包同时把同一棵树分析三遍只是把 CPU 花成 3×（23.8 s）并占三份内存。墙时间的收益出现在三处，而这三处正是"分布式"这个词指的东西：

1. **核数少于包数**（10k 面、十个碎片时这是常态）——那时每多一个包，墙就多一份全量分析；
2. **多机**：每台机器不必都拿到源码树、也不必各跑一遍分析（记录本身就是可分发的那件东西）；
3. **`check` → `build` 之间**：同一个 AI 会话里最常走的那条路（桥先 `check` 出一份记录，随后的 `cargo build` 直接命中）。

**验收（语义，一个字都不许动）**：

1. 同一棵树，**记录命中**跑出来的 `generated_lib.rs` / `graft_plan.tsv` / `source_scope.tsv` / 逐包面与切口归属，与**全量路径**逐字节相同；
2. **诊断也要逐字节相同**（这是本次改动最容易悄悄坏掉的一半：20 个 `parsed_face` 调用点里任何一处改成"从记录读"而记录缺了它要的字段，结果不是报错而是**那条检查静默不再看见这个面** ✗）；
3. 参照 `tools/xirang-real-usage` 的四条归属断言继续绿。

---

## 3. 记录的形状

**落点**：`<target>/xirang/records/<tree-fingerprint>/`（`target` = `CARGO_TARGET_DIR` 或 `<manifest>/target`，与 `xirang/cache` 同一套解析，`identity_cache.rs:105`）——**在 OUT_DIR 之外**，因此天然可被同一台机器上的多个包、以及将来的分发共用。

**内容**：树级产物 + 逐面事实，各一份：

```
record.tsv        # schema 版本 · 树指纹 · 每个载荷文件的名字/字节数/sha256
faces.tsv         # 每个面：相对路径 · 模块 · 身份(NodeId) · kind · 父 · 声明字段原文
products/         # 7 份树级产物逐字节复制（file/function/graph/shape/pruning/graft 审计文本的"树级那一份"）
diagnostics.json  # 这次分析产生的诊断集合（身份+措辞，用来证明两条路径给出同一批结论）
```

**发布**：沿用既有的原子写法（`write_if_changed`：写唯一同级临时文件 + `rename`，`discovery_cache.rs:47`），记录**先写载荷、最后写 `record.tsv` 的指纹行**——与"载荷先发布、指纹最后写"同一条纪律（`pipeline.rs:414`）：读者只认指纹行，半份记录不可能被当成完整记录。

**读取**：包读到记录后，只做三件事——① 按自己的形状（claims / facade / cut_out）从 `faces.tsv` 投影出 visible 集；② 渲染 `generated_lib.rs`；③ 写 `source_scope.tsv` 与 `graph.generation`，其余 7 份从 `products/` 复制（或硬链接）。

---

## 4. 新鲜度证明：继续用内容指纹，**不引入 mtime 快路径**

今天的纪律是"缓存只是观察记录，损坏就重建、绝不被信任"（`discovery_cache.rs:1`）。这条设计**不改它**：

- 走树 + 逐文件内容指纹实测 **0.06 s**（§1.1）⇒ 每次构建都重新算 `discovery.fingerprint` 完全付得起；
- 指纹命中 ⇒ 信任记录；不命中 ⇒ 走今天的全量路径，并把新记录发布出去；
- 记录自身还要过一遍 `record.tsv` 里逐文件的 `名字 + 字节数 + sha256`（读回 ≈663 KB，毫秒级）⇒ **截断/损坏/半代**记录一律重建，不可能被信任。

⇒ **明确不做**：不按 `mtime` 判新鲜（那会引入一个新的信任假设，而且和 cargo 自己的判定叠在一起时无法解释"为什么这次相信它"）。记录的价值来自"同一棵树被算了多次"，不来自"跳过一致性检查"。

---

## 5. 分阶段落地

**阶段 0（✅ 已完成，2026-10-10）**：给既有的 verbose 开关（`XIRANG_BUILD_VERBOSE`，`lexicon.rs:171`）加了逐相位耗时行——`xirang build phases: discover=… scope=… declare=… plan=… shape=… render=… write=… total=…`，按运行顺序列出，**不改任何行为**（只在 `emit_cargo_directives && verbose` 时多打一条 `cargo:warning=`；失败的那次运行也打）。结果是 §1.1 那张表，它决定了后面几个阶段各该装什么。两条钉子：① `pipeline_tests::the_phase_line_lists_every_phase_and_ends_with_the_total` 钉住那一行的形状（读者与本文都按它解析，改名或漏相位会在这里红）；② 自证守恒——表里**各相位之和逐位等于 `total`**（9.216 vs 9.218、4.623 vs 4.624）。踩到的一处：`PhaseLog::start` 被 `verb_table` 门禁点名（裸动词不许出现在入口位以外）⇒ 改名 `for_run` ✓。
**阶段 0 顺带确认的一件事**：`write` 那 1.4 s 是**推导**（`write_*_manifest` 从 nodes 重算行）而不只是 I/O ⇒ 记录要接管的正是"每包重算一遍树级产物"这件事。

**阶段 1（树级产物）**：7 份逐字节相同的产物改为"树级算一次、各包复制"，`record.tsv` 记它们的摘要。收益＝省掉每包的 graph/manifests 计算与写入（约 663 KB × N 次写）。语义风险最低，先把记录的管子打通。

**阶段 2（逐面事实）**：把解析结果与逐面校验结论装进 `faces.tsv`，让 20 个 `parsed_face` 调用点里**能消费事实的那些**改为消费它；不能消费的（它要的是源码文本而不是字段）留在全量路径上，并**逐个点名记在文档里**——"哪些检查仍在读源码"必须是明确的清单，不能是"应该都换了吧"。

**阶段 3（投影）**：碎片只投影 + 渲染 + 写 shape 相关产物；`source_scope.tsv` 生成一次（它按形状不同）。

---

## 6. 钉子（每阶段都要有，缺一条就算没做完）

1. **等价性**：同一棵树，记录命中路径与全量路径的 `generated_lib.rs`、`graft_plan.tsv`、`source_scope.tsv`、**诊断输出**逐字节相同（一条钉子，四个文件）；
2. **陈旧拒用**：改一个面的内容后重建 ⇒ 新面必须出现在渲染里，记录命中不得掩盖它（反证：把指纹比较短路 ⇒ 这条必须红）；
3. **损坏重建**：把记录里的某个载荷改一个字节 / 截断 / 改 schema 版本 ⇒ 必须重建并给出与全量路径相同的产物（三条各一）；
4. **并发**：两个包同时首次构建（记录不存在）⇒ 不得发布半份记录，也不得有一个读到半份（沿用 publish lock 的纪律，`pipeline.rs:444`）；
5. **归属不变**：`tools/xirang-real-usage` 四条断言 + `tools/xirang-partition-rehearsal` 五条腿继续绿；
6. **数字**：把 §2 的验收数字变成 `tools/xirang-build-cost`（一棵 1000 面树、四个形状、cold/warm），带上限，超了就红。

---

## 7. 不做

- 不引入 mtime 快路径（§4）；
- 不在 MCP 层做缓存（既有纪律）；
- 不动渲染语义、不动归属规则、不动身份；
- 不为省这一步去改 `check` 的行为——`check` 是"跑一次管线"，它**也**该用记录（它是最常被 AI 桥调用的那条路），但那是阶段 2 之后的事，先把构建那条路做对。
