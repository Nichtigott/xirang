# 考题与对照装置（永久化，2026-10-10）

**English (short).** The exam recipes live here, in the repository, because `target/` is
git-ignored: every fixture under it dies on a clean clone, a `cargo clean`, or — on this machine —
a reboot (`/tmp` is wiped at boot by NixOS). This directory keeps what cannot be regenerated from
nothing: the task texts, the pre-registered judging keys, the fixture *generators*, the harness
scripts and the hash manifests. The trees themselves are rebuilt on demand; the run outputs stay
under `target/bench-runs/`.

中文：**考题目录**。放在仓库里而不是 `target/`，因为 `target/` 是 gitignored：一次干净克隆、
一次 `cargo clean`、或这台机器的一次重启（NixOS 开机清 `/tmp`）就会让那些夹具消失。这里只收
**不能从零再生的东西**——题面、预登记判据、夹具**生成器**、量器脚本、哈希清单；**树按需重建**，
运行产物留在 `target/bench-runs/`。

---

## 一、kimi 定的测试面（共 11 个，不是一个）

| # | 测试面 | 规格（docs/） | 出题台 / 装置 | 测什么 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | **场景 bench（k27）** | `design-scenario-bench-k27.md` | 本目录（`briefs/` `keys.md` `build-s*.sh` `score-*.py` `bin/`） | T1–T8 矩阵 + S1–S7 场景；四轴 token · 调用与形状 · 质量与补丁感 | 10-03 先导/扩展 + 2026-10-10 起全跑 |
| 2 | **hardbug 四类复杂 bug** | `design-hardbug-bench-spec.md` · `design-mcp-hardbug-plan.md` §4 | `tools/xirang-mcp-hardbug`（W7 出题台） | H1 供应链驳杂 → `consistency` · H2 采信未落实 → `conformance`+台账 · H3 跨文件因果链 → `why` · H4 单文件多算法 → `digest`；每题带**构造真值 + 理想路径 + 必须拒绝的捷径**；判分＝根因命中/调用数/token/反证/最小性/附带损伤 | W8 一次大测试已跑（收尾 `462ee13`）；设计种子见 `hardbug/` |
| 3 | **realbug 真实修复语料** | `tools/xirang-realbug` 文档 | 同上（脚本自足） | 从真实仓库挑 commit（同时改源码与测试）⇒ 反转源码那一半 ⇒ 树按它自带的测试变红；**真值＝那个 commit**，无题库 | 已落地（T-17） |
| 4 | **chain-eval 调用链** | `tools/xirang-chain-eval*`（4 个脚本） | 同目录 | 种子从树自己的调用图挑站点 ⇒ 构造故障 ⇒ 两工具作答 ⇒ **字符串 + 边算术**判分 | 已落地 |
| 5 | **mcp-eval 手设真 bug + 理想路径** | `tools/xirang-mcp-eval` | `tools/xirang-mcp-eval-chains`（从会话日志量链） | 手设真 bug + **预设完美 debug 路径** ⇒ 量"思维链拟合度"：推理字符 · 工具调用 · 理想路径覆盖率 | 已落地 |
| 6 | **test-faces** | `design-mcp-test-faces.md` | — | "哪个特性面是红的"（A 静态面清单 / B 观侧面） | 已落地（§5） |
| 7 | **判定夹具 35 条** | `design-scenario-cases.md` | T-21 用法 | 七形状 × 四措辞 + 一判别对（问句判定，不是工具对照） | 已写好，按需用 |
| 8 | **W1–W8 能力批次 + W8 裁决** | `design-mcp-hardbug-plan.md` | §4 预登记判据 / §6 调用数主战场 | 四类难题各一题、两臂、机械判分六项 + **证伪条件先写死** | W8 已跑：无绝对优势、只赢代价轴 |
| 9 | **26 题逐题协议** | `tools/nichlink-perquestion-protocol.py` | `bench/trees/TREES.json` + round9 树 | **一题一会话一树**的逐题 token/步数（各题 usage 自算，不做归属切分） | 第十/十一轮跑过 |
| 10 | **8 条单题探针（h1–h4）** | — | `h1–h4` 四类各一题（供应链/采信/跨文件/多算法） | 单题版对照，会话 id 见 `tools/nichlink-compare-table.py` | 跑过 |
| 11 | **真实规模与归属（量器，不是对照）** | — | `tools/xirang-real-usage` · `xirang-real-projects` · `xirang-real-project-test` | 真实消费者、150 个真 crate、分区后的归属与墙时间 | 已接 CI |

## 二、怎么跑一轮（四步 + 装置清单）

运行根约定：**树与产物放 `target/bench-runs/<题>-<臂>/`**（工作区内、重启不掉；`/tmp` 会没）。

```sh
# ① 造题树（逐题两臂，造完逐题复算哈希；含臂名绝对路径的夹具要内容归一化后再比）
for q in 1 2 3 4 5 6 7; do for a in ours cg; do
  sh bench/build-s$q.sh target/bench-runs/S$q-$a/tree
done; done
# ② 任务书：只给最小调用形状，不许出现任何工具名/旗标/能力名
#    （场景题面在 bench/briefs/，逐题改路径即可；模板见 briefs/S*-ours.md）
# ③ 判分键先登记后开跑：bench/keys.md（事后改动一律记修正案、下一轮生效）
# ④ 一题一个成员一会话；跑完从各会话自己的 usage 出四轴（axes.py，必须 zstd -dc 解多帧）
```

**开跑前装置清单（每条都对着一轮真实作废）**：

1. **藏起会被注入的东西**：`mv AGENTS.md AGENTS.md.bench-hidden`——harness 会把工作区根的
   `AGENTS.md` 注进每个臂会话（里面写着宿主用法与 MCP 写入面）⇒ "零指点"不成立、广告轴被污染。
   **跑完必须还原**（`mv AGENTS.md.bench-hidden AGENTS.md`）。
2. **任务书零指点**：只给 `<wrapper> <子命令> …`，不出现任何工具名/旗标/页面名/能力名。
3. **对照臂先 `cg init <自己的树>`**，索引只许指自己的树（否则索引落在检出上：工具对题树全瞎，
   而且会把被测实现当语料送给它）。
4. **点名禁止使用被测工具**（`xirang-mcp`/`bin/nl`/`xirang` CLI、对仓库 crate 跑 `cargo doc`/`cargo metadata`）。
5. **树逐题同构并复算哈希**（本目录 `trees/TREES.json` 是 26 题的登记哈希）。
6. **判分键先登记后开跑**；事后收紧只能并列报，不许改本轮分数。
7. **审计两通道**：`calls.log` 只记走 wrapper 的调用；**越界只有会话日志看得见**（`zstd -dc`）。
8. **作废现场留证**（改名留档，不删）。

## 三、本目录的文件

| 文件 | 用途 |
| --- | --- |
| `keys.md` | **预登记判据**：S1–S7 的题面与判分键、历次修正案、S5 复验与插件场景的登记 |
| `briefs/` | 两臂任务书（10-03 版；新一轮用零指点模板重写，见 `S8-*.md` 的写法） |
| `build-s1.sh` … `build-s8.sh` · `build-m1.sh` | **夹具生成器**（S1 空目录 … S8 插件空槽位；m1 多轮补丁感） |
| `score-s1.py` … `score-s7.py` · `score-m1.py` | 机械判分器（与工具无关，两臂同一把尺） |
| `axes.py` · `axes2.py` | 四轴 token（从会话 `usage` 求和；**日志是多帧 zstd，必须 `zstd -dc`**） |
| `audit-calls.py` | 禁读区审计（**口径按"本 run 目录之外"**；只是形状证据，真相看会话日志） |
| `write-briefs.py` · `write-m1-briefs.py` | 任务书生成器（含最小接线规范与两臂工具段） |
| `miss-by-tool.py` | 按工具统计"白跑"（未产生被后续引用信息的调用） |
| `bin/nl` · `bin/cg` · `bin/xirang-mcp` | 两臂 wrapper（`xirang-mcp` 是**指向当前构建的软链**，永不入库二进制拷贝） |
| `blind/` | 剥掉自称后的盲评副本 + `RUBRIC.md`（五轴 1–5 分）+ 匿名映射的机械回溯方法 |
| `hardbug/` | hardbug 出题台的设计种子与归因（`BRIEF.md` · `bench-spec.md` · `chains*.py` · 两份 analysis） |
| `trees/TREES.json` | 26 题的登记哈希（制备后逐棵复算对回它才算两臂同题同树） |

## 四、历史结果去哪找

- 场景 bench 的逐轮现场：`target/bench-runs/`（本轮的 S1–S8、AD 广告探针、T1）、
  `bench/runs/`（10-03 的 S1–S7 两臂答案）、`target/bench-k27/runs/`（kimi 时代 T1–T3）；
- hardbug：`target/hardbug/`；26 题逐题：`target/round10/`；单题探针：`target/probe-perq/`；
- 结论与账：`docs/design-mcp-optimization-backlog.md` · `docs/design-mcp-hardbug-plan.md` ·
  `docs/design-scenario-bench-k27.md`（先导批结果与修正案）· 以及每轮的报告。
