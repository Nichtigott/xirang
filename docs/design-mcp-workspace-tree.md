# MCP 的"全树"与深度：对 codegraph 的结构性优势（2026-09-29）

维护者的判断（原话）：「studio侧其实我们应该是做了全树预览的，我们可以做全树，但是我们可以更精准，而且算法层面可以做到更深度，
减少一部分思考的工作，甚至更准确地锁定问题本身，这就是我们和codegraph比的绝对优势」。
本记录只定方向与缺口证据，不含实现。

## 1. 实测缺口：全树目前只在"包"这一层成立

根 `Cargo.toml` 是 **virtual manifest**（`[workspace] members = ["kernel","macro","toolchain","conventions","examples/…"]`，没有 `[package]`）。
在**工作区根**上调用（同一进程、同一棵树）：

| 工具 | 根上的结果 |
| --- | --- |
| `xirang.status` | ✓ 可用：`rust_files=446 functions=2904`，815 ms（纯源码扫描，不需要身份） |
| `xirang.registry` | ✗ 拒绝：`cannot learn the identity namespace of …: …/Cargo.toml is not a package; cargo metadata listed …`（219 字节） |
| `xirang.grafts` | ✗ 同上 |
| `xirang.diff {records:true}` | ✗ 同上 |
| `xirang.search {query:"Button"}` | ⚠ **降级**：先自曝 `tree unavailable (…)`，再退回文件级命中（1584 字节、22 行、**无面**） |

逐**成员**就正常：`examples/control-button` → `namespace xirang-example-control-button` ✓；`examples/control-button-graft` 同形 ✓。
而 `toolchain/tests/fixtures/node-editor`（**不在** workspace 成员表里的嵌套包）**同样拒绝** ✗ ⇒ 原型夹具那棵树也拿不到。

带面的成员只有宿主：`examples/control-button/src` 3 个文件、`control-button-graft/src` 4 个、`node-editor/src` 2 个；
框架三个 crate（kernel/macro/toolchain）与 conventions 本身不含面。

⇒ **树机件有，入口没有。** 身份按设计是"包 = 命名空间"，所以"根"（不是包）天然没有命名空间；
要跨工作区给出全树，必须逐成员建身份上下文再合并。

## 2. 为什么这是结构性优势，不是速度优势

codegraph 的知识是"符号 + 文件 + 调用边"。它没有：包即命名空间的身份、注册面与注册树、graft 切口与落盘记录、
能力声明与 provider、构建裁决与记录状态。我们的答案能带这些，而且每条都能被钉：
面的 `ok` / `added since build` / `re-identified` / `build unknown`；记录的 `ok / undeclared / stale / re-identified / unreadable`；
关系的 `Live` 或 compiler candidate。**"更准"就来自这里：结论带着它的身份与证据。**

## 3. 深度方向：减少 agent 的思考，而不是减少它的输入

三层，都落在树语义上：
1. **从症状反推最小定位链**：输入一条构建诊断、一个被拒的面、或一条 stale 记录 ⇒ 输出"哪个面 / 哪条 requirement / 缺哪个 provider
   （或哪条 graft 记录指向它）"，带身份与证据；而不是让 agent 自己读三份报告去拼。
2. **跨包影响面**：给定一个符号或一个面，返回身份感知的反向闭包——**哪些包的哪些面**受影响，而不是文件级 grep 结果。
3. **收敛判定**：对一棵树给出"现在能不能发布 / 还开着哪些口"的结论，并**声明它的上限**（哪些面没检查、哪些包不可解析）。

## 4. 下一批的实现要点（未做）

- **根解析**：解析到 virtual manifest 时，用 `cargo metadata` 枚举成员，对每个成员建自己的身份上下文（命名空间＝包名，与 `NodeId` 的定义一致）。
- **合并视图**：按包/命名空间分组呈现，每个节点带构建裁决与记录状态；**不可解析的成员必须出现在答案里并说明原因**
  （现在 `node-editor` 与库 crate 要么沉默要么报错）。
- **降级必须自曝**：`search` 那句 `tree unavailable (…)` 是正确样板；其余工具跨包时也要这样，**不许**用文件级结果冒充树级答案。
- 顺带保留两个便宜项（不是战略，只是补形状）：`read` 报出文件总行数、支持一次读多份并声明上限。

## 5. 边界

实现在 `toolchain/src/mcp` **当前在飞批次落地之后**进行——同一目录不能有两个写者。
本记录只含方向、缺口证据与实现要点，不含代码。

## 6. 中途修正（2026-09-29，维护者提示「我们应该也有现成的对于全树的记录文件啊」）

上面 §4 我写的是"逐成员建身份上下文再推导"。维护者指了更省也更真的路，实测确认成立：**整棵树的记录构建期已经写到盘上，而且 MCP 已经有读取器。**

- **稳定记录集**：`<package>/target/xirang/out/` —— `source_scope.tsv`（`mode`/`selected` + `node source module`）、`pruning_manifest.tsv`、`graft_plan.tsv`（`cut graft full line column`）、`function_manifest.tsv`、`discovery.fingerprint`（一个哈希，可当新鲜度键）、以及 174 行的 `generated_lib.rs`。实测 `examples/control-button` 有全套 6 个文件。
- **现成读取器**：`toolchain/src/mcp/build_evidence.rs` 读的就是这条路径，并且是**经 `build_method` 自己的读取器**读的（不是第二份推导）；其模块文档写明它与"读源码文本"会不一致（刚写下的文件还没进宿主编译出的树），并列出它**有意不报告**的东西：`static_graft_plan!` 的声明状态归 `grafts`、contract/admission 字段需要已加载的注册机。`lib.rs`/`tools.rs` 同样写明 `xirang.explain` 读构建**发布**的文件。
- **绝不要读 `target/debug/build/<pkg>-<hash>/out/`**：同一个包实测 **134 份**哈希分身、旧的还在，按 glob 取可能取到陈旧记录。
- **可用性不对称（要处理的现实）**：`examples/control-button` 6 个文件 ✓；`examples/control-button-graft` **0** ✗；`toolchain/tests/fixtures/node-editor` **0** ✗。

**修正后的做法**：这一层以"**读已发布的记录并格式化**"为主，推导只做兜底。
1. 根是 virtual manifest 时枚举成员，对每个成员**先读它自己的 `target/xirang/out/`**（走 `build_method` 的读取器，不另写 TSV parser），按包分组给出作用域 / 剪枝 / graft 计划 / **新鲜度**（`discovery.fingerprint` 或既有判定）。
2. 每个成员都带状态：`published`（有记录 + 新鲜度）/ `not built`（没有记录，明说）/ `no faces`（框架 crate，说清原因）。**不许**把"没有记录"呈现成"空树"。
3. 只有问题需要记录里没有的东西（如 contract/admission）才回落到派生（加载注册机），并在答案里标明"这次是派生，不是发布记录"。
4. 降级自曝这条不变。

这一修正同时让 §4 的成本估算作废：读文件远比逐成员建身份上下文便宜，而且读的正是构建自己的真相源。
