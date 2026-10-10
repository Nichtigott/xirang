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

### 1.1 钱花在哪（外部量出来的）

| 相位 | 实测 | 量法 |
| --- | --- | --- |
| 走树 + stat 1003 个文件 | **0.020 s** | `find src -type f`，3 次 |
| 读全部文件 + sha256 | **0.043 s** | `find … -print0 \| sort -z \| xargs -0 sha256sum`，3 次 |
| 面解析（缓存拿得走的那部分） | **≈2.0 s** | cold − warm（`discovery_cache` 的 unit 命中） |
| 渲染 + 落盘 | **0.19 s** | 同一二进制：渲染 0 面/5.8 KB 5.245 s vs 渲染 1001 面/1.47 MB 5.432 s |
| **其余（未归因）** | **≈3.1 s** | warm 3.38 s 减去上面三项 |

⇒ 结论一：**I/O 不是墙**（走树+哈希 0.06 s）。结论二：warm 之后仍有 3.1 s，只可能是"整棵树被反复分析"。
证据：`parsed_face(...)` 在非测试代码里有 **≈20 个调用点**（`face_syntax_check.rs` 3 处、`manifests.rs` 5 处、`contracts.rs` 2 处、`static_plan.rs`、`scope_faces.rs`、`node_identity.rs`、`face_view.rs`、`graft_view/matching.rs`、`renderer/aliases.rs` …），而**只有 `registration_phase::aggregate` 那一条路吃 unit 缓存**（`registration_phase.rs:92`，由 `face_syntax_check.rs:20` 的 `cache_units` 传入）。
⇒ 每个包把这棵树解析了**很多遍**，其中一遍被缓存。

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

- 单包从 **6.9 s（碎片）/ 5.4 s（宿主）** 降到 **≤1.2 s**（读记录 + 按形状投影 + 渲染 + 落盘）；
- 三包总 CPU 从 **17.8 s** 降到 **≤9 s**（= 1 份分析 5.4 s + 3 份 ≤1.2 s 的投影；投影做到 ~0.3 s 的话落在 6.5 s 上下），即 10k 面时构建脚本墙的 73% 从"每包一份"变回"整棵树一份"。

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

**阶段 0（先量，不改行为）**：给既有的 verbose 开关（`XIRANG_BUILD_VERBOSE`，`lexicon.rs:171`）加每个相位的耗时行，把 §1.1 里那 3.1 s 摊开成逐相位数字。判据＝同样的树跑两次，相位和 ≈ 构建脚本总耗时（自证守恒）。**这一步是这次改动的第一件要做的事**：记录该装什么由它决定，而不是由我的推测决定。

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
