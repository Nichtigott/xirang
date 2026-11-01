# 设计：测试面（哪个特性面是红的）

维护者的评测把这条空缺量出来了：第 1 轮第 11 轮的缺陷**在默认面隐形**（`cargo test --workspace --offline` 全绿，红只在 `--all-features`；`studio` 不在 `xirang-toolchain` 的 `default = ["build", "run"]` 里，默认面根本不编译那个文件），而**两端（我们的桥与 codegraph）都答不了"红在哪个面"** —— 两组都靠自己把特性面加宽才看见红。

## 1. 为什么现在答不了

桥索引的是**结构**（源码、注册面、调用边、已发布记录），不索引**测试结果** ✗。"哪个面是红的"是后者。要答它，只有两条路：读一份已经存在的测试结果，或者**自己去跑测试**。

## 2. 两个候选设计

### A. 静态面清单（可立即做，答"该跑哪个面"）

从 `cargo metadata`（桥已经在用：`workspace::scope` 解析成员时调用过 ✓）读每个成员的：

- `features.default`（默认面是哪些特性）；
- 全部特性名（`features` 的键，含 `dep:`/`/` 形态要原样标注）；
- 每个 target 的 `required-features`。

`xirang.status` 里多一节，形状例如：

```
faces  xirang-toolchain: default=[build, run] all=[authoring, build, cli, …, studio]  required=[xirang-dev -> [dev-supervisor]]
note   a file compiled only under a non-default feature cannot fail on the default face: run
       `cargo test --all-features` (or `--features <name>`) before believing a green default run
```

它回答的是"**该跑哪个面**"，**不是**"哪个面红" ✓ —— 但第 11 轮真正卡住人的正是前者：代理需要一个理由去加宽特性面。成本低（纯粹的清单读入，不新增 I/O 类别），落点是一个新模块（`toolchain/src/mcp/faces.rs`）加 `status_tool` 一行调用，避免 `tools.rs` 顶到 600 行棘轮。

### B. 观测到的测试面（需维护者单独拍板）

`xirang.check`（名字待定）：显式 opt-in 地在成员目录里跑一条**固定**命令，只回摘要。要点是把"跑测试"带来的新面积压到最小：

- **显式请求**：默认不跑；请求要说出面的选择（`--all-features` / 具名特性 / 默认面）——与写入路径同一条纪律（默认预览、动作要请求说出来）。
- **硬超时 + 明确未知**：超时 ⇒ 印 `unknown (timed out after <d>)`，**不是** pass ✓（本仓"宁可拒绝也不给看似合理的答案"那条教义）。
- **证据戳**：命令、退出码、起止时间、日志落盘路径（原始输出不塞进回复，只给摘要 + 路径）✓。
- **不猜**：不根据源码/门控"推断"哪些测试会失败 ✗（那是假绿）。
- **边界**：只覆盖 cargo 测试；进程/网络类测试的挂起靠超时兜住并如实报未知。

这条会引入一个**新的能力类别**（长驻 stdio 服务里跑子进程、并发与超时管理、日志落盘），因此它与"写入路径"同级，值得单独一次拍板，而不是顺手加进去。

### C. 明确不做

让桥"猜"测试结果、或把默认面全绿读成"没问题" ✗ —— 第 11 轮的教训正是这条。

## 3. 判据与钉子（无论走 A 还是 B）

1. **每个面各一条钉子**：A ⇒ 清单里必须出现 `studio` 且注明它非默认；B ⇒ 超时分支必须印 `unknown`，而"没跑到任何测试"必须与"跑了且通过"区分开（`0 passed` 是"什么都没跑"，本仓已在别处踩过）。
2. **来源可区分**：静态清单与观测结果在答案里必须能分辨（照 `published.rs` 那条"每份答案都说出自己用的是哪一种"的规则）。
3. **自我描述同步**：`tools/list` 的描述、`initialize` 的 instructions、模块文档要一起改（本仓的教义）。

## 4. 建议

先做 **A**（半天内可落地、直接止住第 11 轮那类盲区），**B 单独拍板**（它是能力扩张，且需要定超时/并发/日志三条契约）。

## 5. 已落地（2026-09-29）

候选 **A** 与 **B** 都已实现（同一批）：

- **A**：新模块 `toolchain/src/mcp/faces.rs`（从 `workspace::metadata_json` 读 Cargo 自己的看法，命令拼法仍只有一份），`xirang.status` 末尾多一节。真树上实测输出：
  `faces  xirang-toolchain: default=[build, run] all=[authoring, build, cli, dev-supervisor, evidence, mcp, …]`
  加那条盲区备注 —— 第 11 轮那个盲区现在在调用方跑测试**之前**就写在答案里了。
- **B**：新工具 `xirang.check {face, timeout_ms?, root?}`（目录第 22 项），四条钉子：失败运行如实报出退出码/结果行/失败测试名；`timeout_ms` 到点 ⇒ 印 `unknown (timed out…只杀直接子进程…日志在…)` **而不是 pass**；没有 `test result:` 行 ⇒ 印"什么都没跑，这不是通过"；不说 `face` ⇒ 按名拒绝（没有默认面）。输出走**文件**而不是管道（没人排空的管道会填满并让子进程死锁，本仓已经付过一次代价）。
- 判据同 §3：静态清单与观测结果在答案里可分辨（`faces` 行 vs `check` 行的命令与退出码）✓；自我描述同批改（`tools/list` 描述 + 模块文档）✓。

## 6. 相关记录

- `docs/audit-2026-09-29/audit-agent-simulation.md` §7（第 2 轮：F6/F7 的答案、`root` 基准缺陷的修复）。
- `docs/audit-2026-09-29/audit-chain-eval.md`（机械半边：14 轮种子注入与机械判分）。
