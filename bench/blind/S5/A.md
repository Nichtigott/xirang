# S5 · 同族对象核查（我们臂）

包：`control`（`/tmp/scen-runs/S5-ours/tree`），命名空间 `control`。
同族对象：`root/button`、`root/slider`、`root/label`（三者 parent 相同，是 `root` 下直接兄弟）。
审查题：**未改动这棵树的任何源文件**（只跑了题目要求的 cargo/test/check，因此 `tree/target/nichlink/{out,cache}` 里落了构建产物，见"没核查到的范围"第 6 条）。

---

## ① 那一次同族比较

**我用来做同族比较的那一次调用**（题目七形状里 "a family" 那一格）：

```sh
export BENCH_CALLS=/tmp/scen-runs/S5-ours/calls.log
/home/nich/Moirai_N3/nichlink/bench/bin/nl \
  --call nichlink.consistency --parent root --root /tmp/scen-runs/S5-ours/tree
```

输出摘要（exit 0）：

```
family root · member control · 3 member(s)
  button                   0 call(s): none
  label                    0 call(s): none
  slider                   0 call(s): none
outliers: 0 of 3
not covered by this comparison: … a call written inside a macro body is not read as a
call (the kernel's rule), so an object whose whole body is one macro invocation reads as
calling nothing — **on such a tree `api` has nothing to compare, so use `by: kind` or
`by: source` …, or `specimen` …**
```

**这一次调用在我这棵树上不成立**——不是"0 异类 ⇒ 三家都没问题"，而是它自己点名的覆盖缺口：三个对象的声明体都是**一个宏调用**（`crate::root_object! { … }`），内核规则不把宏体内的调用读作调用，所以 `api` 信号**没有可比对象**（`0 call(s): none` → `outliers: 0 of 3` 里的分子分母都空）。

按它给的下一步，我把同族的其余三个信号一次试完（仍是一次比完同族，没有逐个从头读对象）：

| 信号 | 命令 | 结果 |
| --- | --- | --- |
| `by kind` | `--call nichlink.consistency --parent root --by kind` | `button=Button / label=Label / slider=Slider`，`outliers: 0 of 0`——它比值，而三个 kind 本就该互不相同，无可比组 |
| `by source` | `--call nichlink.consistency --parent root --by source` | `button/button.rs`、`label/label.rs`、`slider/slider.rs`，`outliers: 0 of 0` |
| `specimen` | `--call nichlink.adopted --root …` | `no adoption ledger at /tmp/scen-runs/S5-ours/tree/.nichlink/adopted/entries` → 没有 ledger 基线 |

**因此真正点名异类的那一步不是 consistency，而是一次并排读同族契约落盘**（一步比完同族，不是逐个重读对象）：

```sh
cat -n /tmp/scen-runs/S5-ours/tree/target/nichlink/cache/units/*.tsv
```

三份 unit（每个对象一份已解析落盘的声明契约，同一构建指纹代际）并排读出的**同族差异只有一处**：

| unit 文件 | path | node | kind | line | contract 行 |
| --- | --- | --- | --- | --- | --- |
| `be8e6fddda1fb360a2011bb09190f625.tsv` | `button/button.rs` | `3e1f685cf66d1a661f5d9cc0285e324f` | `Button` | 11 | 无 |
| `884afcf408c31bc1ae507e5ecba2a611.tsv` | `slider/slider.rs` | `9174fc9f4a0848306a693b2ae2f4a9fd` | `Slider` | 11 | 无 |
| `709aef826d0d5977a68dbd0050aec434.tsv` | `label/label.rs` | `68c941df41a8044cda4826fef0edc37c` | `Label` | 11 | **`contract	exports	["control.render"]`** |

三个 unit 的前 7 行（path/node/kind/parent/line）同形，parent 都是 `47cc4542db60545ea3a528fd7e27e6da`；差别只在第 8 行：只有 label 有 `contract` 行。这正是一张"mixed supply chain"图：每份文件单看都成立，但 label 的声明与它的两个兄弟不是同一种约定。

（旁证：`--call nichlink.explain --node root/{button,slider,label}` 三次的派生投影也同形——identity/path/kind/source/module/parent/registry_name 一一对应，`exports` 不在该投影里，`explain` 明说 contract/admission 字段不在此报告，所以这一步只能靠落盘契约。）

---

## ② 逐对象裁定表

| 对象 | 裁定 | 依据 | 反证 |
| --- | --- | --- | --- |
| `root/button`（`button/button.rs`，node `3e1f685c…`） | **没问题** | ① 契约 unit `be8e6fdd…` 只有 `path/node/kind/parent/line` 七行，**无 contract 行**；② `explain root/button` 的派生投影（source `button/button.rs`、module `button`、parent `47cc4542…`）与 slider 同形；③ 声明体只有 `kind: Button` + `parent: crate::root_node_id(env!("CARGO_PKG_NAME"))`，字段序合法（`kind` 先写；`exports` 在可选字段位上没写就是省略）；④ `check` → `verdict passed (cargo exit 0)`；`verify` → `verdict ok（内核接受该树）`、`connector verdict: ok`。 | 与本族多数（2/3）一致并不等于"正确"：若 label 那份带 `exports` 的声明才是这个族的规范，则 button 是**漏声明**的一方。反证未被证伪——本树没有任何 ledger、文档或父级规则要求同族必须声明 `exports`（`.vscode/nichlink-face.code-snippets` 与 `generated_lib.rs` 的字段清单都把 `exports` 列为可选），所以"漏写"只是未被排除，不是证据。 |
| `root/slider`（`slider/slider.rs`，node `9174fc9f…`） | **没问题** | ① 契约 unit `884afcf4…` 同样**无 contract 行**；② 与 button 的文件逐行同形（只换 `Slider` 标识符），派生投影同形、parent 同为 `47cc4542…`；③ `check` passed / `verify` ok（同上一次运行，本树全树通过）。 | 同 button：无法排除"两家一起漏写、label 才是规范"。另外 `consistency --by api` 在本树没有可比对象，所以"三家一致"这句话**不是** comparison 工具证实的，而是我并排读契约 unit 得到的——工具层面的同族比对在本树为空。 |
| `root/label`（`label/label.rs`，node `68c941df…`） | **有问题**——是"同族唯一的、无对应物的导出声明"（**可疑声明**，不是编译/校验错误） | ① 契约 unit `709aef82…` 第 8 行 `contract	exports	["control.render"]`，是**三份 unit 里唯一的 contract 行**，两个兄弟没有；② `grep -rn render` 全树只在 `src/label/label.rs:13` 命中它自己——本包内**没有任何提供者、消费者、测试**提到 `control.render`；③ `label.rs` 因此是"本文件局部成立、与同族约定不同"的那一个（consistency 文档对 mixed supply chain 的定义）；④ 同族的两个"声明值"信号（`by kind`/`by source`）只比 `kind`/`source` 的值，抓不到多出来的字段，`specimen` 又需要 ledger——所以这个异类只能通过并排读落盘契约点名（正是本题①要的那一步）。 | 不算缺陷的强反证有三条：(a) `exports` 是**合法可选**字段（`.vscode/nichlink-face.code-snippets` 里有 `exports: ` 定式，`generated_lib.rs` 的宏文档把 `exports` 列进 accepted order）；(b) 内核**接受**了这棵树（`verify` connector verdict ok），`check` 也 passed（cargo exit 0），没有任何门禁判它错；(c) `consistency` 的 specimen 语义明说"声明更多的兄弟不报告"——即"多声明"本身不被该工具视作异类。因此我的裁定是"与同族约定不一致 + 导出名在包内无对应物，需要裁定/确认"，而不是"已知错误"；若 `control.render` 本是运行期由包外实现的能力，本条应降级为"待确认"。 |

一句话：**同族 3 个对象里，button、slider 判定没问题（依据是落盘契约无 contract 行 + 派生投影同形 + check/verify 通过）；label 判定有问题，问题是它带着同族唯一的 `exports: ["control.render"]`——一个在本包内既无提供者也无人消费的声明。**

---

## ④ 我没有核查到的范围

1. **comparison 工具的覆盖边界（它自己声明的）**：它读**派生**树的兄弟集和每个兄弟自己的文本，所以活在共享 helper、生成代码或父级规则里的约定不可见；`api` 只比"被调用的名字"，不比单位与算术；**宏体内的调用不读作调用**——本树三个对象体都是一个宏调用，所以 `api` 在本树**没有任何可比对象**，"0 异类"不等于"三家一致"。
2. **没有 ledger**：`/tmp/scen-runs/S5-ours/tree/.nichlink/adopted/entries` 不存在（`adopted` 报 `no adoption ledger`），所以 `consistency --specimen` 与 `conformance` 所回答的"当初写下的设计有没有落实"这一半，本次**查不了**。
3. **契约/准入/注册规则数据**：只有落盘快照带它们（`registry` 明说"Contract, admission, and registration-rule data … are not included"，`explain` 明说 contract/admission 字段不报告）。我是**直接读** `target/nichlink/out/*.tsv`、`generated_lib.rs` 与 `target/nichlink/cache/units/*.tsv` 得到契约差异的，没有工具把 `exports` 端出来；`inspect` 只回到 `registrations: Label` 一行。
4. **运行期**：本树 0 个测试（`cargo test` → `0 passed; 0 failed`，`check` 的 census 也是 0 条），没有运行期表面；`control.render` 在运行期是否存在、由谁消费，**没查**。
5. **包外**：三个对象的 parent（`47cc4542db60545ea3a528fd7e27e6da` = `root_node_id(env!("CARGO_PKG_NAME"))`）与被 graft 的那一侧在包外/父注册表，导出名对父注册表意味着什么、谁是它的消费者，**没查**。`lib.rs` 的 `static_graft_plan!` 是 `cut(x::NODE_ID) graft(x::NODE_ID)` 自切自接（三处同形，`graft_plan.tsv` 可查），是否为空操作/预期**没查**（它不属于本题点名的三个对象）。
6. **对树的改动**：没改任何源文件；但按题目跑了 `cargo test`、`check`，构建产物写在 `tree/target/nichlink/{out,cache}`（第一次 `registry` 还答 `tree derived now (no published records)`，跑完这些记录才存在——这是本次唯一的环境副作用）。

---

## 亲手跑过的命令

```sh
export BENCH_CALLS=/tmp/scen-runs/S5-ours/calls.log
mkdir -p /tmp/scen-runs/S5-ours/cargo-target
cd /tmp/scen-runs/S5-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S5-ours/cargo-target cargo test --offline

nl --list                                                   # 七形状指引 + 工具清单（预览）
nl --list nichlink.registry / nichlink.consistency          # 预览（各 1 次）
nl --call nichlink.registry        --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.consistency     --parent root --root /tmp/scen-runs/S5-ours/tree   # ← ① 同族比较那一次调用
nl --call nichlink.consistency     --parent root --by kind  --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.consistency     --parent root --by source --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.check           --root /tmp/scen-runs/S5-ours/tree --target_dir /tmp/scen-runs/S5-ours/cargo-target
nl --call nichlink.verify          --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.explain         --node root/button --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.explain         --node root/slider --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.explain         --node root/label  --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.adopted         --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.inspect         --path src/label/label.rs --root /tmp/scen-runs/S5-ours/tree
nl --call nichlink.why --at label/label.rs:13 --root /tmp/scen-runs/S5-ours/tree        # ← 被拒（exit 1）
nl --call nichlink.why --at src/label/label.rs:13 --root /tmp/scen-runs/S5-ours/tree    # ← 改对路径后 exit 0

nl --list nichlink.digest / nichlink.conformance / nichlink.explain                      # 预览（各 1 次）

# shell 取证（只读 /tmp/scen-runs/S5-ours/tree）
find /tmp/scen-runs/S5-ours/tree -not -path '*/target/*'
cat /tmp/scen-runs/S5-ours/tree/{Cargo.toml,build.rs,src/lib.rs,src/button/button.rs,src/slider/slider.rs,src/label/label.rs}
cat /tmp/scen-runs/S5-ours/tree/.vscode/nichlink-face.code-snippets
cat /tmp/scen-runs/S5-ours/tree/target/nichlink/out/{discovery.fingerprint,function_manifest.tsv,graft_plan.tsv,pruning_manifest.tsv,source_scope.tsv,generated_lib.rs}
cat -n /tmp/scen-runs/S5-ours/tree/target/nichlink/cache/units/*.tsv                     # ← 点名异类的那一步
grep -rn "render" --include='*.rs' --include='*.toml' --include='*.json' /tmp/scen-runs/S5-ours/tree
```

**工具调用计数**：`nl --call` **13 次**（其中 **1 次被拒**：`why --at label/label.rs:13`，exit 1，提示有 `root` 时路径要写成该 root 看得见的样子，即 `src/…`；随后同工具同参数位改写为 `src/label/label.rs:13` 得到 exit 0）；`nl --list` 预览 **6 次**，**互不重复**（总指引、registry、consistency、digest、conformance、explain），无一次重复预览。cargo 1 次（`cargo test --offline`，带 `CARGO_TARGET_DIR`）。
