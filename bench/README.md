# 考题与对照装置（永久化，2026-10-10）

最新真实 agent 配对验收入口为 `agent-ab.py`，协议见
[18 会话验收](../docs/agent-ab-acceptance-2026-10-10.md)。它使用独立项目目录和客户端配置，
不移动本仓库的 `AGENTS.md`。先运行 `prepare` 与 `controls`，确认正确修复通过、局部修复和
绕行反例真实失败，再运行 `run` 和 `score`。`score` 产生实际补丁匿名包；机械通过仍须结合
完整轨迹审计及独立盲评。模型 token 来自服务商 usage，缺失用量不会当作零费用。

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

## 五、MCP 优化的可复现检查（2026-10-10）

默认责任链入口的前后对照用 `mcp-responsibility-bench.py`：原生提供者与两个消费者，
可附加 50,000 份无关源码；经实际 MCP stdio 测函数名/对象路径，保留每次回复、耗时、
源码文件数、6 类证据覆盖与二进制哈希。首次请求不是冷磁盘，结构查询仍有包级发现成本；
跨功能版本时前后返回的证据可能不同，不能将耗时直接解释成同任务胜负。
English: this is a retrieval benchmark with synthetic native declarations, not a model repair trial.

```sh
python3 bench/mcp-responsibility-bench.py --before BEFORE_BINARY --after AFTER_BINARY \
  --noise-files 50000 --output target/bench-runs/responsibility-UNIQUE
```

纯性能变更请加 `--require-identical`，要求两臂全部冷/热回复逐字节一致。
`--after-first` 反转二进制次序，`--first-case object` 让对象查询从空进程解析缓存开始；
第二类查询的首次调用已经被第一类查询预热。Linux 还记录服务进程 VmRSS/VmHWM（不含 Cargo 子进程）。
本轮 5 万/10 万文件数据及限制见[解析复用报告](../docs/mcp-parser-reuse-2026-10-10.md)。
English: use identical-answer gating for performance-only changes, reverse arm order and choose the
first query explicitly. Linux memory samples exclude child processes; first-request timing is not cold disk.

`mcp-callgraph-bench.py BEFORE AFTER --output <新目录>` 对两个二进制做同树工具测量，
保存墙时间、回复与哈希；它不运行模型、不统计 token。`score-m1.py` 的 v2 使用终态
patch churn 计算编辑放大率，支持 `--json`；历史 v1 结果不改写。两者口径见
[本轮说明](../docs/mcp-optimization-2026-10-10.md)。评分器的反例用以下命令运行，CI 也执行：

```sh
python3 -m unittest discover -s bench -p 'test_*.py'
```


### CodeGraph 同题源码上下文对照

`mcp-codegraph-bench.py` 创建独立同构的 Rust 夹具，仅为 CodeGraph 那份建立索引。
记录一次性 CLI 和持久 MCP 的冷/热耗时、初始化/索引成本、预先指定源码片段的覆盖、
回复字节数、完整往返帧和实现哈希；不推断模型 token、补丁质量或动态派发能力。
症状词与类型名查询防止只测精确函数名。CodeGraph MCP 使用 direct 模式、关闭 watcher，
保留默认的跨请求源码不抑制行为；未测多客户端共享 daemon 的并发吞吐。

```sh
python3 bench/mcp-codegraph-bench.py target/debug/xirang-mcp CODEGRAPH_BIN \
  --repeats 5 --output target/bench-runs/codegraph-explore-UNIQUE
```

OUTPUT 必须不存在。对照设计、局限和本轮读数见
[`mcp-codegraph-comparison-2026-10-10.md`](../docs/mcp-codegraph-comparison-2026-10-10.md)。

加 `--fixture ledger` 复用 `tools/xirang-mcp-eval` 的原 Kimi 账本题，注入其四个独立缺陷，
比较限定方法名和原题症状文字的上下文。`--build-label` 标注实际构建配置；debug 与 release
不得混为同一实现优化的加速比。该对照仍只测工具取回证据，没有模型执行修复。


领域场景另由 `consistency_lease_tests.rs` 固定：同族结构偏差与采信失效同答，
续期不修复结构，旧台账不伪造逐文件归因，读取不续期。运行：

```sh
cargo test -p xirang-toolchain --features mcp --lib consistency_tests::leases
```

### 完整 Rust 工作流的机械验收

长链与成本反例可运行：

```sh
python3 bench/mcp-repair-loop-bench.py target/debug/xirang-mcp \
  target/bench-runs/repair-loop-UNIQUE --counterproof
```

它分别记录首次/重复 affected、关闭总账后的 check、编译错误交付，以及双消费者、症状补丁、
提供者修复和替换实现绕行反例。保存原始 MCP 帧、时延、UTF-8 字节与源码摘要。
这是脚本验收，不是模型修复胜率；字节不等于供应商 token。详见
[全流程审计](../docs/mcp-repair-loop-audit-2026-10-10.md)。

```sh
cargo build -p xirang-toolchain --features mcp --bin xirang-mcp
python3 tools/xirang-mcp-workflow-eval target/debug/xirang-mcp \
  --output target/bench-runs/mcp-workflow-UNIQUE
```

通过真实 stdio 的已广告入口执行建造、同族新增、加深、合同修改拒绝、搬移和外部实现落地，
在每轮用 Cargo 验证原有测试；修复题含红→绿→撤销后红反证及三轮边界回归。检查源码/设计记录
指纹、身份变化规则、兄弟与外部消费者保留、采信失效不被查询续期。输出目录必须不存在，
完整往返、测试输出、逐步指纹和 `results.json` 均保留；失败也保留现场。CI 执行此流程。

这是确定性脚本验收，补丁由脚本给定，不计入模型胜率。完整比较还需同模型独立任务运行、
四轴 token 和匿名补丁评分；源码片段命中率、churn 或延迟不能代替这些判据。

### 缓存、超大树与解耦覆盖

`tools/xirang-mcp-hardbug tokens SESSION` 分列未命中、cache read、cache write、总输入和输出；
cache write 不算命中，计费应单列写入价格。对应反例也由上面的 `test_*.py` 门禁运行。

`mcp-scale-bench.py SOURCE_TREE XIRANG_BIN CODEGRAPH_BIN --output NEW_DIR` 复制已有注册面规模树，
校验同源，记录 CG 索引成本、持久 MCP 首次/重复请求、原始回复和 Linux 服务进程 RSS。
不清除 OS 页缓存、不运行模型、不声称整树编译；夹具与 CG 安装须另行准备。

最新缓存账、1k/50k 规模结果及 graft/move/crate 覆盖矩阵见
[本轮核查](../docs/mcp-cache-scale-decoupling-2026-10-10.md)。大树重复查询目前 CG 更快。
工作流门禁另补手写行为、子树 move、跨 crate 拒绝与六次搬迁记录保留；
`xirang-partition-rehearsal` 在每次改 crate 边界后实际运行 overlay，并与未拆分有效树比较。

### 重复上下文、范围读取与修改可见性

`mcp-context-bench.py XIRANG_BIN --root ROOT --query QUERY --scope REL_PATH --output NEW_DIR`
通过真实 MCP 对比全树/限定范围请求（首次与三次重复），再验证显式回执复用与清空回执后的完整重发。
`--scope` 可重复。记录原始消息、回复字节、延迟和二进制摘要；范围查询不代表全树召回，字节不是模型 token。

`mcp-update-bench.py INDEXED_FIXTURE XIRANG_BIN CODEGRAPH_BIN --file REL_RS --output NEW_DIR`
在**可丢弃且已建立 CodeGraph 索引的夹具**中，给两臂分别插入新符号并启动各自计时，交替顺序重复三轮。
CodeGraph 开启 watcher；逐次保存回复和 stderr，以核查目录监听上限或降级。
测的是编辑到新符号源码首次可见，不是整个关系图完成更新；最后恢复原文件字节。
不要对真实业务源码运行此变更量器。结果及局限见
[实测与共享索引设计](../docs/mcp-context-index-2026-10-10.md)。

供应链根因与修复质量的现有覆盖、调用绕行反例和新增完整任务的验收设计见
[责任链审计](../docs/mcp-supply-chain-audit-2026-10-10.md)。H1/H3 的答案文本评分、M1 churn
和生成大树延迟不能代替实际修复后的责任层、可替换性及多消费者回归；新设计尚未运行模型对照。

### 声明密集夹具与阶段计时 / Dense declarations and phase timings

`mcp-responsibility-bench.py` 支持 `--extra-faces 2500`，加入独立原生注册声明，
与 `--noise-files` 的普通源码分开计数。使用 `--require-identical` 核对两臂每份完整回答；
用 `--after-first --first-case object` 反转顺序，让对象查询从空进程缓存开始。
`XIRANG_MCP_PROFILE=1` 把 face views、声明、build evidence、trace 阶段耗时写入 stderr。
性能比较单独运行，不与编译/完整测试争用资源。

当前结果见 [解析缓存报告](../docs/mcp-cache-capacity-2026-10-10.md)。
[共享读取实验](../docs/mcp-shared-reads-2026-10-10.md) 因性能回退已撤回，报告保留反例数据。
Use extra native faces to distinguish declaration-heavy trees from ordinary-file cache hits.
Retained phase timings support attribution; the discarded shared-read experiment is not shipped.

## MCP completion and freshness

`mcp-completion-bench.py BINARY OUTPUT [--files N] [--queries-only]` exercises cold/warm/restart,
misses, source edits, inventory moves/additions/deletions, executable obligations and stable MIR.
Output must be new; wire traffic, answers, byte counts, elapsed times and assertions are retained.
It does not report provider tokens or autonomous AI repair rates.

该脚本验收冷/热/重启、未命中、编辑及清单变化，并实际执行验证义务与稳定版 MIR；输出目录必须不存在。
字节与墙时间不冒充供应商 token、账单或模型修复成功率。实现与验证范围见
[收口报告](../docs/mcp-completion-2026-10-10.md)。
# Mixed coordinate / dispatch faults

`coordinate-ab.py` constructs a compiled 10-crate, 1150-Rust-file synthetic UI project.
Button, Slider and Timeline share six coordinate-policy layers. Logic errors and wrong
LOCAL/WORLD dispatch can cancel, including a case whose three visible tests are all green.
The finite injection model enumerates 5832 assignments; the real-agent pilot uses three
preregistered cases, three paired repeats each, with DeepSeek V4.1 Flash.

```sh
python3 bench/coordinate-ab.py prepare target/bench-runs/coordinate-v1
CARGO_TARGET_DIR="$PWD/target/bench-runs/coordinate-cargo" python3 bench/coordinate-ab.py controls target/bench-runs/coordinate-v1
python3 bench/coordinate-ab.py run target/bench-runs/coordinate-v1
CARGO_TARGET_DIR="$PWD/target/bench-runs/coordinate-cargo" python3 bench/coordinate-ab.py score target/bench-runs/coordinate-v1
python3 bench/agent_ab_audit.py target/bench-runs/coordinate-v1
CARGO_TARGET_DIR="$PWD/target/bench-runs/coordinate-cargo" python3 bench/coordinate_regression_probe.py target/bench-runs/coordinate-v1
python3 bench/coordinate_report.py target/bench-runs/coordinate-v1
```

Use a fresh output directory for each prepare. Controls must pass before model calls.
Agent worktrees have independent build outputs; only the serial judge shares dependencies,
cleaning both adjudicated packages whenever it changes source copies, including switching back.
Default, alternate, release, two replacement implementations, intermediate contracts and actual
registered routes are judged independently of visible screenshots. Do not infer success from
agent prose. The protocol is in `docs/coordinate-ab-acceptance-2026-10-11.md`.

The supplemental regression probe skips Viewport, whose current policy is numerically neutral,
while preserving final outputs. It runs only the original and agent-added tests, not the hidden
judge. Report this separately from the preregistered repair success rate; it measures whether the
submitted regression suite protects the provider route against an output-preserving future edit.

After interruption, `score --resume` and `coordinate_regression_probe.py OUTPUT --resume`
reuse completed results for immutable saved finals and preserve interrupted directories.
The probe can use `--follow` while scoring runs, but must use a separate `CARGO_TARGET_DIR`.
Generate the report after both finish; it refuses an incomplete supplemental probe.
Resume support was added after the coordinate-v1 coding sessions; their original harness
snapshot remains under `OUTPUT/harness/`. It changes execution recovery, not judging criteria.

## Sequential maintenance acceptance / 连续维护验收

```sh
python3 tools/xirang-mcp-workflow-eval target/debug/xirang-mcp --output target/bench-runs/workflow-new
python3 bench/evolution-maintenance.py target/debug/xirang-mcp --output target/bench-runs/evolution-new
```

Both outputs must be new. The second script uses the first script's real MCP/Cargo harness.
One project evolves through construction, sibling addition, internal deepening, adoption renewal,
compensating faults, route/replacement counterexamples and an object move. Frozen tests retain
their bytes across the move using an explicit public re-export. Raw tool errors are observations,
not successful capability assertions; inspect them even if the script exits successfully.
No model calls, provider token estimates or new CodeGraph scores are produced.

两个目录均须不存在。脚本保存查询正文、MCP 往返、源码指纹、冻结合同、Cargo 日志和 harness 副本；
红灯必须来自实际断言失败，不能用编译失败代替。结构比对有非空 exports 声明的正负对照。
采信迁移的工具错误按观察结果单列，脚本通过不表示这些错误已修复。
实测结论与按证据排序的优化方向见[持续维护报告](../docs/mcp-evolution-results-2026-10-11.md)。

### Paired persistent agents / 持久会话配对

`evolution-ab.py` runs three pairs through five successive requirements with persisted DeepSeek
V4.1 Flash sessions. Each next turn inherits the actual working tree, not a canonical reset.
Prepare uses the retained coordinate scaffold by default (`--scaffold` can name another copy);
run requires all controls to pass and verifies the frozen scripts and binaries.

```sh
python3 bench/evolution-ab.py prepare OUTPUT --scaffold target/bench-runs/coordinate-v1/scaffold
python3 bench/evolution-ab.py controls OUTPUT
python3 bench/evolution-ab.py run OUTPUT
python3 bench/evolution-ab.py score OUTPUT
python3 bench/evolution_rescore.py OUTPUT
python3 bench/evolution_audit.py OUTPUT --reviewed
python3 bench/evolution_report.py OUTPUT
```

`run` makes real paid model calls. Re-running it against existing run directories is refused;
`score` reuses completed per-round scores and does not call a model. Raw sessions, interventions,
before/final trees, usage and judge outcomes are retained. Summary generation refuses fewer than
30 distinct rounds. Natural provider caching is reported separately from local graph/index state.

这是实际付费模型实验；冻结需求、预算、独立正负控制后才运行。三对项目连续五轮，保留历史和失败，
评分不反馈给后续轮次。共同验收允许原生采信台账或可审查项目文档，不强迫对照组调用 XiRang 专用工具。
范围与限制见[模型运行前验收方案](../docs/evolution-agent-acceptance-2026-10-11.md)。

The supplemental `evolution_rescore.py` corrects the drag-route oracle to accept legitimate
non-provider wrapper spans; WORLD detours remain checked. It retains the frozen original
scores and applies the correction to both arms. `evolution_audit.py --reviewed` separates
artifact validity from a normally completed model turn and probes only valid final projects.
The report refuses incomplete corrected scores or audits. Interrupted, unfinished score
directories must be preserved under a different name before retrying; completed results are reused.

统一复核允许合法的追踪包装，不改变提供链合同或模型产物；原始分数仍保留。结果和按证据排序的
优化方向见[真实 agent 持续维护报告](../docs/evolution-agent-results-2026-10-11.md)。
