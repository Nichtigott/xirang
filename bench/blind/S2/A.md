# S2 · 从头到尾审查 `/tmp/scen-runs/S2-ours/tree` · 逐栏处置表（我们臂 · 第三次重跑）

- 题态：本次开始时树是**整棵重建的干净态**（开局 `find` 只有 8 个文件，树内**没有** `target/`、没有发布记录）。
- 被审对象：包名 `control`，5 个 `.rs`、3 个注册面：`root/button`、`root/label`、`root/slider`。
- 工具：`.../bin/nl`（nichlink MCP 桥），调用全部记入 `BENCH_CALLS=/tmp/scen-runs/S2-ours/calls.log`。
- 纪律：只读/写 `/tmp/scen-runs/S2-ours/**` 与该 wrapper；cargo 走 `CARGO_TARGET_DIR=/tmp/scen-runs/S2-ours/cargo-target`。**未改这棵树**（未调 `apply`/`plugin`/`new_project`/`adopted`，源文件 mtime 未变）。
- 一句话结论：**裁决表 = `check --census true` 的整树静态普查，共 6 栏；6 栏每栏都点名了具体缺陷**（第 1/2 栏是同一对常量 `src/button/button.rs:16` / `src/label/label.rs:16`；第 3/5/6 栏是 `src/button/button.rs:20` / `:21`；第 4 栏是 `src/lib.rs:10-11` 少了 slider），其中第 6 栏的一个子计数（未构造枚举变体 = 0）是空的；`diff`/`verify` 报 0/0/0、`trace` 栏为空、兄弟比较栏无可比。

## 一、这张"裁决表"从哪来、一共几栏

`nl --list` 的工作流表把请求放进七种形状，其中 **"a range question -> `check` and dispose of every census column"**（范围型 ⇒ 用 `check` 并逐栏处置）。生产这张表的是 `check` 在 `census: true` 下回复的**整树静态事实普查**（"what else is wrong here"）。`nl --list check` 的定义行按顺序列举它的栏，并以 **"The sixth column is branch-level"** 收尾 ⇒ **共 6 栏**：

| # | 栏名（工具原文口径） | 本树该栏的载荷 |
|---|---|---|
| 1 | numeric constants that are **re-spelled** elsewhere | 1 个值被重拼成 2 个名字 |
| 2 | numeric constants **read nowhere outside tests** | 2 个常量无生产读者 |
| 3 | production `pub fn` names **no test writes down**（declarations） | 1 条 |
| 4 | the **entry plan's own site counts** | 2 `cut(` + 2 `graft(` |
| 5 | the production functions **no test can reach**（test-reachable，含逐目录细分） | 1 of 1 |
| 6 | **branch-level**：构造性到不了的臂（`false` 守卫 + 未构造枚举变体） | 1 条臂，变体 0 |

census 回复里另有几条**不是栏**的边界行（`not covered by the test-reachability column …`、`not covered by the branch-level column …`、`not covered: …`），是第五节"未覆盖"的依据。

## 二、逐栏处置表

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
|---|---|---|---|---|
| **1. re-spelled 数值常量** | **有问题**：全树唯一被重拼的值是 `1000`，两处——`src/button/button.rs:16`（`pub const LIMIT: i64 = 1000;`）与 `src/label/label.rs:16`（`pub const LIMIT_AGAIN: i64 = 1000;`）。同一个值、两个名字、两个文件。**反证收尾见第三节**：literal 只回 2 行 | `nl --call search --root <tree> --literal 1000`；`nl --call check --root <tree> --face all --census true --target_dir /tmp/scen-runs/S2-ours/cargo-target` | literal：`src/button/button.rs:16: pub const LIMIT: i64 = 1000;` + `src/label/label.rs:16: pub const LIMIT_AGAIN: i64 = 1000;`（恰好 2 行）；census：`respelled 1000 is declared as \`LIMIT\` (src/button/button.rs:16) and written again at src/label/label.rs:16`，反向一条对称 | census 只判**命名的**数值常量；字符串常量、结构重复、跨 crate 重拼不在内（`not covered:` 行明说 string constants 不在内） |
| **2. 测试外无人读的常量** | **有问题**：`LIMIT` 的唯一引用在 `clamp` 的不可达臂里（`src/button/button.rs:22`，第 6 栏判死）＝无人读；`LIMIT_AGAIN` 在 `src/label/label.rs` 之后再无出现 | `nl --call check … --census true`；`nl --call read --root <tree> --path src/button/button.rs --whole true`；`nl --call callgraph --root <tree> --function clamp` | census：`unreferenced \`LIMIT\` (src/button/button.rs:16) is not read anywhere outside tests` 与 `unreferenced \`LIMIT_AGAIN\` (src/label/label.rs:16) is not read anywhere outside tests`；read 显示 `LIMIT` 只出现在 16（声明）与 22（不可达臂）；callgraph：`callers (0): -` | "读"是**名字级文本读**：宏展开、`#[cfg]` 分支、跨 crate 读不计 |
| **3. 没有测试写下名字的生产 `pub fn`** | **有问题（1 条）**：`clamp`（`src/button/button.rs:20`）没有任何测试写下它的名字 | `nl --call check … --census true`；`nl --call callgraph --root <tree> --function clamp` | census：`decl   no test names \`clamp\` (src/button/button.rs:20)`、`declarations: 1 production \`pub fn\` name(s) appear in no test file`；callgraph：`matches 1 / src/button/button.rs:20 fn clamp / callers (0): - / callees: -` | 文本级计数：测试里**写下名字**才算；经别名、宏展开或间接调用到达的不计 |
| **4. 入口计划的 site 计数** | **有问题（本树最要紧）**：计划只有 2 `cut(` + 2 `graft(`（`src/lib.rs:10` button、`src/lib.rs:11` label），而树上有 3 个注册面——`root/slider` 不在计划里。**干净态下这一步只有 `explain` 给得出**：`registry` 只列 node/source（本轮它没有 scope 判定列），`explain` 判 slider `not-selected` | `nl --call check … --census true`；`nl --call read --root <tree> --path src/lib.rs --whole true`；`nl --call explain --root <tree>`；`nl --call registry --root <tree>` | census：`entry plan: 2 \`cut(\` site(s) and 2 \`graft(\` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)`；read 8–12 行原文：`static_graft_plan!(FRAMEWORK, cut(crate::button::NODE_ID) graft(crate::button::NODE_ID), cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),)`；explain：`scope mode=auto all=false reason=- selected_ids=2 selected_sources=2` + `root/slider Slider not-selected slider/slider.rs` + `pruned 3`；registry：`faces 3`（button/label/slider，无 scope 列） | 计划**该不该**收 slider 属设计意图，工具只报静态选择结果（census 自陈 "which of them this application ships is the plan's own business"）。构建面 `verify`/`diff` 只比"面 vs 构建"，不比"计划 vs 面" |
| **5. 测试到不了的生产函数** | **有问题**：`1 of 1` 生产函数无测试可达 = `clamp`；逐目录 `src/button 1 of 1`（其余目录 0），不可达面全在 `src/button` | `nl --call check … --census true`；`nl --call affected --json '{"files":["src/button/button.rs","src/label/label.rs","src/slider/slider.rs"],"root":"/tmp/scen-runs/S2-ours/tree"}'` | census：`test-reachable: 1 of 1 production function(s) no test can reach (3 function(s) indexed …)`、`by directory: src/button 1 of 1`、`fn     no test reaches \`clamp\` (src/button/button.rs:20)`；affected：button.rs `1 definition(s)`、label.rs `1 definition(s)`、slider.rs `0 definition(s)`，三处都写 `tests: none reference these definitions` | ⚠️ 工具局限：`src/label/label.rs:18-23` 有 `#[cfg(test)] mod tests` + `#[test] fn the_label_face_is_still_there`，`check` 的 result 行也报 `1 passed`，而 `affected` 对 label.rs 仍说 "none"——它匹配的是**函数定义**，测试点名的类型不算。故 `affected` 的 "none" 不能当覆盖面证据，本栏以 census 的 `by directory` 行为准。census 第 5 栏另陈边界：动态分派、函数指针、FFI、宏展开、trait 方法、闭包不可见；按名匹配会误算同名；`main` 从不列出；静态走查不是覆盖测量 |
| **6. 构造性到不了的臂（branch-level）** | **有问题（1 条臂）**：`src/button/button.rs:21` 的 `if false { return LIMIT; }`，位于 `clamp`（20–25 行）内；上方契约注释（18–19 行）自己写着 "an arm no input can reach / 一条任何输入都到不了的臂"。**该栏第二个子计数为空（反证见第三节）** | `nl --call check … --census true`；`nl --call read --root <tree> --path src/button/button.rs --whole true`；`nl --call callgraph --root <tree> --function clamp` | census：`branch-level: 1 constructively unreachable arm(s) in this tree (1 \`false\` guard(s), 0 never-constructed variant(s); …)`、`\`if false\` guards an arm in \`clamp\` at src/button/button.rs:21 that no run can enter`；read 逐行给出 20–25 | 数据相关条件**完全不判**（字段/参数/比较/`match` 值）；只认 `false` 字面量（`1 == 2`、`!true`、`const bool`、`cfg!(…)` 不读）；宏展开、动态分派、函数指针、FFI 不可见；`pub` 枚举不判；`unsafe`/derive/树外消费者可能证伪 |

### 我另外加的四栏（不属 census，用来把"面/构建/运行时"收口）

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
|---|---|---|---|---|
| **7. 面级 delta** | **无问题（0/0/0），但带 clean 态口径** | `nl --call diff --root <tree>` | `build current` / `faces 3 (source) vs 3 (build)` / `added since build 0  gone 0  re-identified 0` / `the build matches the sources face for face` | 干净态下它先声明 `tree derived now (a comparison needs the tree the sources declare now, which no published record can carry); …/target/nichlink/out was read first and does not carry what this answer names faces by` ⇒ 这是**派生对派生**的比对，不是"源码 vs 某份落盘构建"；`records: true` 那一半未跑（见第五节） |
| **8. 内核 + 连接器校验** | **无问题** | `nl --call verify --root <tree>` | `verdict ok (the kernel accepted the tree)` / `connector verdict: ok (the package's own faces were admitted)`；面数 3 vs 3、`added/gone/re-identified 0`（同第 7 栏的派生口径说明） | 只答"内核收不收这棵树"，不答计划选择、不答运行期 |
| **9. 运行时 trace** | **空栏（无制品，不是"没问题"）** | `nl --call trace --root <tree>` | `trace absent: /tmp/scen-runs/S2-ours/tree/.nichlink/traces/nichlink.trace`，并给出产出办法：`trace_call!` + `NICH_LINK_TRACE` / `NICH_LINK_TRACE_FILE` | 运行期到底跑了什么、`clamp` 有没有被调用过，本次**没有任何运行时证据**（`check` 只报静态面） |
| **10. 兄弟一致性** | **无可比（不是"没有异类"这么强）** | `nl --call consistency --root <tree> --parent root`；`nl --call consistency --root <tree> --parent root --by kind` | `--parent root`：三兄弟 `0 call(s): none`，`outliers: 0 of 3`——三面的 body 都是一次宏调用，`api` 模式**无从比较**（工具自陈应改用 `by: kind`/`by: source`/`specimen`）；`--by kind`：`button Button / label Label / slider Slider`，`outliers: 0 of 0`——每个 kind 只有一个成员，**比较对数为 0** | `0 of 0` 说明它几乎没比较，不能当"三个面形状一致"的证据；要真比形状需 `specimen`（本树无 adoption ledger，`conformance` 没有可指对象） |

## 三、反证收尾（③）

两处都用**一条命令**判空：

1. **第 6 栏的"未构造枚举变体"子计数 = 0**。判据：`nl --call check --root /tmp/scen-runs/S2-ours/tree --face all --census true --target_dir /tmp/scen-runs/S2-ours/cargo-target` —— 同一行报出两个计数 `1 \`false\` guard(s), 0 never-constructed variant(s)`；本树三个源文件只有 `struct` + `root_object!` 调用，没有任何 `enum` 声明，所以这一类**没有**条目：是"空"，不是"没查"。
2. **第 4 栏的"外部 graft 记录"是空的**。判据：`nl --call grafts --root /tmp/scen-runs/S2-ours/tree` → `host entry /tmp/scen-runs/S2-ours/tree/src/lib.rs`、`plans 0`、`no external graft plans under .nichlink/external-grafts/`。即：计划漏了 slider 这件事**目前不会**打断任何已写下的记录（`diff --records true` 因此没再跑——那一半必然为空，重复调用没有新信息）。

## 四、我亲手跑过的非 MCP 命令（shell）

```sh
export BENCH_CALLS=/tmp/scen-runs/S2-ours/calls.log
ls -la /tmp/scen-runs/S2-ours/ ; find /tmp/scen-runs/S2-ours/tree -type f | sort   # 开局：只有 8 个文件，无 target/
ls -la --time-style=+%H:%M:%S /tmp/scen-runs/S2-ours/tree/target/nichlink/ /tmp/scen-runs/S2-ours/tree/target/nichlink/out/
ls -la --time-style=+%H:%M:%S /tmp/scen-runs/S2-ours/cargo-target/
cd /tmp/scen-runs/S2-ours/tree
cat -n src/label/label.rs          # 16 行 LIMIT_AGAIN；18-23 行 #[cfg(test)] mod tests + the_label_face_is_still_there
cat -n src/slider/slider.rs        # 只有 Slider 注册，无 fn、无 test
cat -n src/button/button.rs ; cat -n src/lib.rs ; cat -n Cargo.toml ; cat -n build.rs ; wc -l Cargo.lock   # 143 行
head -c 200 .vscode/nichlink-face.code-snippets
grep -c '=== CMD' /tmp/scen-runs/S2-ours/calls.log   # 16
grep -n '=== EXIT' /tmp/scen-runs/S2-ours/calls.log  # 16 个 0，0 个非 0
```

没有跑过任何独立 `cargo` 命令：唯一的构建/测试是 `check` 自己派生的 `cargo test --all-features`（日志 `/tmp/scen-runs/S2-ours/cargo-target/check-all.log`，27–30 s）。

## 五、调用统计与未覆盖范围

### 5.1 MCP 调用统计（本次 t20）

- `BENCH_CALLS=/tmp/scen-runs/S2-ours/calls.log`，`grep -c '=== CMD'` = **16 次 MCP 调用**，`grep -n '=== EXIT'` = **16 次 exit 0、0 次被拒**。
- **0 次写通道预览 / 0 次重复预览**：`apply`/`plugin`/`new_project`/`adopted` 一次未调。
- 三个文件只花**一次** `affected`：`--json '{"files":[…]}`（客户端一个 key 只吃一个值；上一轮 voided 的写法 `--files a b c` 会 usage error，本轮直接用 `--json`）。
- 刻意**没跑**的重复调用：`check --face default`（`status` 给出 `faces control: default=[] all=[]`，本包无特性 ⇒ `cargo test` 与 `cargo test --all-features` 是同一条命令，重跑就是重复预览）；`diff --records true`（`grafts` 已判 `plans 0`，记录半栏必空）；`consistency --by source`（与 `--by kind` 同结论）。
- 与团队口径对照：本臂基线 26 次调用 / 2,153,560；本次 16 次、其中浪费类（拒绝 + 重复预览）= **0**。

### 5.2 未覆盖范围（⑤）

1. **census 自陈的边界**（原文四行）：
   - 常量栏不覆盖：字符串常量、结构重复、运行期行为、散文里写的断言；静态走查不是覆盖测量。
   - test-reachable 栏不覆盖：动态分派、函数指针、FFI、宏展开、只经 trait 方法或闭包到达；按名匹配会误算同名；`main` 从不列出。
   - branch-level 栏不覆盖：数据相关条件一律不判；只有 `false` 字面量被判定；宏展开/动态分派/FFI 不可见；`pub` 枚举不判；通配或绑定到达的臂不读；`unsafe`/derive/树外消费者可能证伪。
   - 全表不覆盖：字符串常量、结构重复、运行期行为、散文断言。
2. **没调过的读工具（真实缺口，不要读成"它们没问题"）**：`usages`、`impact`、`converge`、`digest`、`mir`、`unified`、`locate`、`why`、`conformance`。理由三类：需要制品/账本（`mir`/`unified` 无 MIR 制品、`conformance` 无 adoption ledger/anchor）；需要单对象症状（`locate`/`why` 以失败症状为入口，本树三面 `check` 全绿）；其余是我判断本次不必要——`usages` 会补出"写通道可设字段表"，`impact`/`converge` 会读 `requires`/`provides`/graft 三类声明，**这两类信息本次没有取**。
3. **非 census 文件只用 shell 看过**：`Cargo.lock`（143 行）、`.vscode/nichlink-face.code-snippets`（33 个 snippet 键）、`Cargo.toml`、`build.rs`。特别地：**snippet 文件的字段名与 `usages` 报告的写通道字段表逐项是否一致，本次没有核对**。
4. **构建范围 / 题态（题目点名要如实写的）**：
   - 开局是干净态：`find` 只有 8 个文件，树内没有 `target/`；第一个读工具 `registry` 也明确报 `tree derived now (no published records at /tmp/scen-runs/S2-ours/tree/target/nichlink/out; cannot read …/source_scope.tsv: No such file or directory (os error 2))`。
   - cargo 目标目录在**树外**：`check` 的日志与构建产物在 `/tmp/scen-runs/S2-ours/cargo-target/`（`check-all.log`、`debug/`、`nichlink/`）。
   - **本轮审计期间，读工具的派生把记录落进了树内**：`tree/target/nichlink/cache/`（23:31:12）与 `tree/target/nichlink/out/`（23:31:13，`discovery.fingerprint`、`source_scope.tsv`、`graft_plan.tsv`、`pruning_manifest.tsv`、`function_manifest.tsv`、`generated_lib.rs`）——这是我开始后才出现的（开局 `find` 没有它们），是工具的派生副产物，**不是源码改动**：四个源文件 mtime 仍是重建时刻 23:29。
   - 因此在同一次会话内可以看到**口径先后**：`registry`（最早那个读）看到的是**无发布记录**的干净态；其后的 `verify`/`diff`/`grafts`/`consistency` 每一条都先读那个目录、并声明它"**不携带**它们据以命名的东西"，退回源码派生；`explain` 的 selected/not-selected 也是派生而来。若只看树内记录，会误以为构建范围就是全部——这是"目标目录在树外 + 派生写记录在树内"造成的真实缺口。
5. **面覆盖**：只跑了 `--face all`（`status`：`default=[]`、`all=[]`，两面对本包是同一条 `cargo test …`）。所以"每个 `required-features` 面都真跑过"**不是**本次结论；本树也没有任何特性面可跑。
6. **运行期**：`trace` 缺失（第 9 栏），`clamp` 究竟有没有被运行过，本次无法判。

### 5.3 处置建议（**未执行**，树保持原样）

- 第 1/2 栏：一个值只留一个名字（把 `1000` 收成一处，或删掉没人读的 `LIMIT`/`LIMIT_AGAIN`）。
- 第 3/5/6 栏：`src/button/button.rs:20-25` 的 `clamp` 要么删掉 `if false` 死臂并补一条点名测试，要么让它有真实语义。
- 第 4 栏：计划要么补上 `cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID)`（`src/lib.rs:12` 之前），要么在源码里写明"slider 有意不进计划"，免得 release 剪掉那个槽又没人知道。
- 第 9 栏：要判运行期，需宿主按 `trace_call!` + `NICH_LINK_TRACE_FILE` 落一份 trace，再跑同一条 `trace`。
