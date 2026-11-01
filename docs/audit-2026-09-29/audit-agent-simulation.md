# Agent Teams 实机模拟评测：找链 → 定位 → 修 → 评分

维护者的题目（原话）：「你可以做一个调用链错误发现测试深度上可以深和浅都测试一下，看看 codegraph 和我们之间的实测差距，而且还可以看判断准确率，你可以实现一个多轮多场景测试，而且还要给修复质量进行评分和评估，记住测试不要用题库，避免模型过拟合导致的误报」＋「那个测试是之前说的 agent team 实际项目模拟测试」。

机械半边（14 轮种子注入、机械判分）记在 `audit-chain-eval.md`；本次是**人机都在环里**的那一半：让队友在被注入故障的**真实项目副本**上用工具找链、定位、最小修复，修复质量由四条机械判据评分。

## 1. 装置

- **题面**：由 `tools/xirang-chain-eval` 的注入器（种子驱动、程序化选站点、记录真值）在**旧 `base` ＋ 旧 `plan.json`** 上现做三套注入树：`/tmp/agent-sim/round-1`（d=1 `impl`，站点 `kernel/src/registry_core/source/calls.rs`）、`round-11`（d=4 `chain`，`toolchain/src/studio/studio/ui/ui.rs`）、`round-13`（d=5 `impl`，`kernel/src/registry_core/tree/graft_ops/overlay.rs`）。每轮另有 `-cg` 副本给对照组。真值只在队长手上（`truth/round-N.json`），成员不可读。
- **队伍**：`debugger-mcp`（只用 `xirang-mcp` 桥，stdio JSON-RPC）、`debugger-codegraph`（只用 codegraph CLI 1.5.0 ＋它自己的 MCP `explore`）、`auditor`（不写源码、不跑 cargo）。7 个任务：三轮 × 两组按依赖串行，审计依赖全部六个。
- **任务书硬约束**：你的工作树是唯一真相源；不许 `diff`/`cmp` 别的检出；不许读 `truth/**` 或 `/tmp/xirang-eval/**`；不许改断言、加 `#[ignore]`、加 `#[allow]` 让树变绿。
- **判分（队长亲自做，不信成员自述）**：① 命中 = 定位的文件:行/符号是否等于真值；② 绿恢复 = 在该树里复跑测试；③ 最小性 = 与**原始树**逐字节 diff（排除 `target`/`.codegraph`）；④ 附带损伤 = 有没有动测试或加豁免。

## 2. 结果

| 轮次 | 真值 | codegraph 组 | MCP 组 |
| --- | --- | --- | --- |
| 1 d=1 impl | `calls.rs:171` `\|\|`→`&&` | 精确；1 token；树=原始；kernel 187/0 | 精确；1 字符；树=原始；187/0 |
| 11 d=4 chain | `ui.rs:91` 删掉 `draw_overlay(frame, app, cache);` | 精确；+1 行；树=原始；studio 530/0 | 精确；+1 行；树=原始；530/0 |
| 13 d=5 impl | `overlay.rs:130` `!=`→`==` | 精确；1 token；树=原始；187/0（`--all-features` 247/0） | 精确；1 token；树=原始；187/0 |

⇒ **6/6 精确命中、6/6 逐字节还原注入（无第二处改动）、0 处动测试**。四轴（绿恢复 / 最小性 / 根因 / 无附带损伤）六格全过。

**与机械基线（一次性工具答案）对照**：第 1、13 轮一次问就中（MCP 基线 3281 B/4 次、15148 B/22 次；codegraph 24424 B/13 次）；**第 11 轮 MCP 一次问未命中**——答案只有 **37 字节、1 次调用、`link unreached`**，而同一个工具在代理手里（5 步、12 次调用）精确落到那一行。⇒ 深链上的分水岭是**一次答案 vs 在环工作流**，不是工具强弱。

## 3. 工具的决定性程度（审计员核过原始输出）

| 轮次 | MCP 组的决定性调用 | codegraph 组的决定性调用 | 两边都没做到的 |
| --- | --- | --- | --- |
| 1 | `read{lines:"138-256"}` 原样打印 171 行（与 142-144 的文档承诺并排） | `node <失败测试名>` 给断言正文（251-255） | **没人从工具拿到"这一行就是病灶"的判定**：`&&` 使自排除成死代码是"文档 + 算子语义"推出来的 |
| 11 | `inspect{ui.rs}` 的 callee 列表缺 `draw_overlay` ＋ `search{draw_overlay}` 的 `callers (0)` ＋ `affected{ui.rs}` **一步点名 3 个变红测试文件** | `callers draw_overlay` → `No callers found`（调用图独有的**否定**）＋ grep 钉死"只剩定义与导入" | 两边都只证明"少了一条边"，**没证明调用点就在 `draw` 末尾**（位置是推断，两组都如实标注） |
| 13 | `inspect{overlay.rs}` 的函数级 callee 列表把 536 行压到 `overlay_cuts` 一个体 | `node` 全文逐字 ＋ 逐跳归因 | "错误文案 → 错误值"这一步两边都得自己 grep（索引不含字符串字面量） |

## 4. 审计结论：有没有破坏测量

**没有。** 证据（审计员独立核，队长抽查）：六份报告对 `diff -r|cmp |/home/|truth|xirang-eval|work/base` **grep 零命中**；`truth/*.json` 与 `work/plan.*` 的 atime 仍是创建时刻、`/tmp/xirang-eval` 最新 atime 早于调试窗口；**六个病灶文件 sha256 与真值 `sha256_before` 逐一相等**（逐字节还原注入）；`round-N` 与 `round-N-cg` 全树相同；三棵树 `#[allow]` 均 57、`#[ignore]` 均 2 未增；计数自洽（187=184+3、530=521+9、187=170+17）⇒ 没有用 `#[ignore]` 消红。

审计发现的要点：

- **F2（高）**：cg-r1 报告"本树 `direct_calls` 只有测试在调用／无生产调用者"是**工具假阴性** ✗ —— `toolchain/src/mcp/source_index.rs:217` 就是生产调用者（跨 crate 全限定引用落在 codegraph DB 的 `unresolved_refs`，`status=failed`），而我们桥的 `callgraph callers(7)` **找到了它**。这一格是工具能力差，不是人的差。
- **F6（中）**：三份驱动脚本的 BIN 指向**主检出**的 `target/debug/xirang-mcp`（HEAD `709fa6a`，pinned `a23be52b` 的后代；被测文件与 round 树修复态逐字节相同），而 round-N 树自己也有同名二进制 ⇒ r1-mcp 报告里「the defect corrupts the tool's own answer」作为**运行事实不成立**（跑的是干净的桥）。下轮应把 BIN 指向 round-N 自身构建，专门测"桥在剖析含自身畸形副本的仓库"这个形状。
- **F7（中）**：可审计性不对称——MCP 组有逐条 JSON-RPC 日志（可机核，也是审计抓到 F1 的依据），codegraph 组 CLI 无日志，工具账只能部分用 `.codegraph` DB 反查。
- **F1**：r1 报告写 `read`×2 而日志是 ×3 = **漏记**（漏 `whole:true` 整文件读；总数 5 正确，决定性调用不受影响）。
- **F3**：r13"偶发失败"（`plugin_host_fault_matrix::process_faults`，超时/管道缓冲）**未能复现**（队长在 `--test plugin_host_fault_matrix -- process_faults` 连跑 3 次，每次 11 passed/0 failed）⇒ 判装置噪声（时序敏感测试）。
- **F4**：round 11 的题面缺陷成立——红只在 `--all-features` 面（studio 不在 `default = ["build","run"]`，`toolchain/Cargo.toml:24`；studio 在 `:39`），两组任务书给的症状命令都是默认面 ⇒ 两组各自加宽特性面并**如实披露**，命中结论不受影响；评分必须固定在 `--all-features` 面。
- **卫生问题**：cg-r11 报告 §5 引了一个跨运行数字（"r1 树边数 18,719"）自称未当证据；该报告把 `default` 行号写成 `:7`（实为 `:24`）。

## 5. 两条工具边界（本次测得，留作下一批的输入）

1. **"哪个特性面是红的"两个工具都答不了**：桥索引结构、不索引测试结果；两组都靠自己从默认面加宽到 `--all-features` 才看见红。
2. **字符串/文案检索是共用空缺**：桥的 `search` 对 `Registry::overlay` 回 `no static function match`、对 `FrameworkMismatch` 回 `no matches`（而 `inspect` 刚把它列为 callee）——方法拼法与枚举变体不可检索，且没有文本搜索；codegraph 的索引同样不含字符串字面量。三处"错误文案 → 病灶"的收窄都是人自己 grep 的。
3. codegraph 侧的边界（对照组自述）：调用图不记调用点行号；`node` 正文多判定并列、不标候选；夹具级事实（三处共用一个 `FRAMEWORK`）工具看不见。

## 6. 局限（要随结论一起读）

- 题面缺陷：第 11 轮的症状命令给的是默认面（见 F4）。
- **工具账口径不对等**：MCP 组报的是**桥调用次数**（5/12/7，日志可核），codegraph 组报的是**全部动作**（CLI＋cargo＋grep＋read ≈ 15/24/16）⇒ 两个数字不能直接比。
- 旧 rev 的注入树配**当前**二进制（见 F6）。
- pristine 参照无法物理隐藏（原始检出与 `base` 同盘）⇒ 只能靠规则 + 审计（审计用 grep、atime、sha256 三重证据补上了这一点）。
- 对照组无命令日志（见 F7）。

## 7. 第 2 轮（按审计 F6/F7 修装置；团队 `xirang-sim-eval-r2`）

同题重做一次，只改装置：两棵新的注入树（仍是 round 1 那道，`calls.rs:171` 的 `||`→`&&`，默认面 184/3 已验），`debugger-selfhosted` **必须在自己那棵树里构建并使用 `xirang-mcp`**，`debugger-logged-codegraph` 同题但**每条命令与原始输出必须落盘**。

**F6 的答案：工具自身带病时，它会给出"语义上合理"的错边——这一次被察觉了。**
- 带病桥（从注入树构建）对 `discover_children` 输出：`callers (2): src/build_time/src/source_walk.rs::discover_children, …::discover_root_reporting` 且 `callees: …, discover_children, …`，即**函数把自己列成自己的调用者兼被调用者**（队长独立复读了日志原文 ✓）。
- 成员察觉的路径是**跨读源码**：同一次 `read calls.rs:139-183` 把第 143 行文档（"the function's own name (self-recursion) is excluded"）与第 171 行病行放在同一屏；再用 `read` 读到测试钉住的期望值 `[]`。它自己的话：这条错边"语义上很合理"（递归真实存在），输出里**没有任何字段提示此边违反了 self-exclusion**，唯一的置信度线索是 `evidence: static-heuristic` ⇒ **只信工具输出的话，这条自指边不会被怀疑**。这是本轮最值钱的一条：工具撒谎的方式是"给出一个看起来对、其实是策略禁止的答案"，而识别它必须靠工具**之外的**事实（文档、被钉的测试、跑一次门禁）。
- 修复后它**重建桥再问同一个问题**：`callers (1)`、`callees` 里不再有自己 ⇒ 工具随修复转正 ✓。

**F7 的答案：命令日志让对照组的工具账第一次可以被机核**（26 个 `=== CMD:` 标记、`init`/`index`/`callers`/改前改后的 `grep` 都在账上；唯一一处 `/home/nich|truth|xirang-eval` 命中是成员自己的诚实声明 ✓）。但仍不够严：26 个标记 vs 它自述"22 条命令" vs 实际工具调用约 6 条 —— 数字对不齐；且该树不是 git 检出（`git rev-parse` 129），改动凭证只能靠改前/改后两行 grep（队长的 `diff` 对原始树覆盖了这一点 ✓）。**下一轮**：一次调用一个标记、每条都带 `=== EXIT:`，并让注入树本身是 git 检出。

**两组的结论一致**：这道题是**语义级**缺陷（一个布尔算符），codegraph 只能把行送到眼前、不会报错，识别同样靠"代码与自身文档矛盾"；codegraph 的 `no covering tests found` 覆盖标注不可信（实际有测试）。

**本轮顺带修掉的一处自我描述缺陷（评测直接测出来的）**：`read`/`inspect` 在给 `root` 时，`path` 相对**那个成员**，而这句话文档里没写、错误话术也没提示——成员照抄树相对路径得到 `path must stay inside the configured source root`，它的下一个猜测还是树相对路径。现在拒绝话术点名它检查的基准与规则（并在其中给出 `src/…` 的形状），`initialize` 的 `instructions` 也补上这条基准规则 ✓。

**局限（本轮）**：`debugger-selfhosted` 的第一次尝试崩在 `MALFORMED_RESPONSE` 上——**活其实已经干完**（树逐字节还原、kernel 187/0、日志在盘上），只差报告；按"审→补→验证→汇报"短派同一成员补交成功 ✓。也就是说本轮的"人的判断"来自它的报告与日志，而报告是崩溃后重写的（内容与日志一致，队长逐条核过 ✓）。

## 8. 下一轮装置协议（第 2 轮的两条教训，已落到工具里）

1. **注入树本身就是 git 检出**（已改 `tools/xirang-chain-eval-mutate.py`）：注入前先把原始树提交一次、注入后再提交一次，并把 `target/`、`.codegraph/` 写进 `.gitignore`。于是 `git diff HEAD~1` 就是这次注入（实测恰好一处 1 行 hunk），解题方的改动凭证不再只能靠改前/改后两行 grep，审计也能直接要 diff ✓。
2. **一次调用一个标记**：对照组的命令日志要**一条调用一个 `=== CMD:` 行、紧跟一条 `=== EXIT:` 行**（第 2 轮是 26 个标记 vs 自述 22 条 vs 实际约 6 次调用，数字对不齐 ✗）。
3. 题面的症状命令要覆盖缺陷**真正可见**的特性面（第 1 轮第 11 轮的教训；现在 `xirang.status` 会印特性面，`xirang.check` 能按面跑）。

## 9. 下一批（按价值排序）

1. 把三处工具空缺做成条目：文本/字符串搜索、方法拼法与枚举变体可检索、以及"哪个特性面是红的"（后者需要读测试结果，属能力扩张，宜先设计）。
2. ~~重跑一轮并把 BIN 指向 round-N 自身构建（F6）；对照组改为带命令日志的驱动（F7）。~~ **已做**：见 §7——F6 答案是"工具撒谎时给的是语义上合理的错边，靠跨读文档/测试才发现"，F7 的日志装置已生效但仍不够严（下一轮：一次调用一个标记且都带 `=== EXIT:`，注入树本身做成 git 检出）。
3. 题面统一给 `--all-features` 面作为症状命令（F4）。
4. 观察 `plugin_host_fault_matrix::process_faults` 的时序敏感性（F3，未复现）。
