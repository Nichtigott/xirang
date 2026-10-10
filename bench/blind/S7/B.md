# S7 · 把项目外的 `button` 实现落地进基树（我们臂）

运行根：`/tmp/scen-runs/S7-ours/tree`
宿主：`/tmp/scen-runs/S7-ours/tree/host`，外部 crate：`/tmp/scen-runs/S7-ours/tree/graft`
环境：`export BENCH_CALLS=/tmp/scen-runs/S7-ours/calls.log`；所有 MCP 调用走 `…/bench-scenarios/bin/nl`；
所有 cargo 命令带 `CARGO_TARGET_DIR=/tmp/scen-runs/S7-ours/cargo-target`。

## 一句话结论

用一次 `apply --action promote --selector button_fast` 就把外部实现落地了：声明改写成
`button_fast` 自己的字段、计划里那条 graft 改成指向本面自身（`cut(crate::button::NODE_ID)
graft(crate::button::NODE_ID)`，仍点名这个面所以它留在生成树里）、记录目录进 trash，
`cargo test --offline` 绿；共 **10 次** MCP 调用（其中 2 次是用法/契约拒绝，0 次重复预览）。

## 改动的文件

| 路径（相对 host/） | 怎么变的 |
| --- | --- |
| `src/button/button.rs` | 被 `promote` 整份改写：`kind: Button` → `ButtonFast`，并带上 `button_fast` 的 name/summary/exports/handle_traits/flow |
| `src/lib.rs` | **只有第 9 行**（button 那条 graft）变：`graft(control_button_graft::button_fast::NODE_ID)` → `graft(crate::button::NODE_ID)`；slider 那条一字未动（见下 diff） |
| `.nichlink/external-grafts/button_fast/` | 整个记录目录移到 `.nichlink/trash/external-grafts/button_fast-1791038135184712221/`，原位置不再存在（`external-grafts/` 只剩一个空目录） |
| `.nichlink/trash/promoted/1791038135183791710/src/lib.rs` | 工具自己留的改前备份（不是手改） |

未动：`src/slider/slider.rs`、`../graft/`（`Cargo.toml`、`src/lib.rs`、`button_fast.rs`、
`slider_fast.rs`、`control_fast.rs`）、`Cargo.toml`、`build.rs`。

## 六条逐条核对

### ① 声明照抄外部实现的值

`nl --call apply --action promote --selector button_fast --apply true --confirm true` 写入后的
`src/button/button.rs` 与 `../graft/src/button_fast.rs` 的 `external_object!` 逐项对比：

| 字段 | `button_fast` 自己声明 | 落地后的声明 |
| --- | --- | --- |
| kind | `ButtonFast` | `kind: ButtonFast` ✅ |
| name | `{zh:"快速按钮", en:"Fast button"}` | `name: { zh: "快速按钮", en: "Fast button" }` ✅ |
| summary | `{zh:"项目外实现", en:"Out-of-project implementation"}` | `summary: { zh: "项目外实现", en: "Out-of-project implementation" }` ✅ |
| exports | `["control.render"]` | `exports: ["control.render"]` ✅ |
| handle_traits | `["ControlHandle"]` | `handle_traits: ["ControlHandle"]` ✅ |
| flow | `FlowContract::new(ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame")` | `flow: crate::FlowContract::new(crate::ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame")` ✅ |

（`preset: NoPreset` / `parts: NoParts` 两边同值：外部是显式声明，本地是 `root_object!` 的默认，
`use crate::{NoParts, NoPreset};` 保留、编译通过。）

命令与输出摘要：

```console
$ nl --call apply --action promote --selector button_fast --apply true --confirm true --root …/host
action apply
moved …/host/src/button/button.rs
landed `button_fast` into `root/button`: … the declaration … now carries the external
implementation's fields, the entry in …/src/lib.rs now points that slot at **this face itself**
…, and the record moved to `.nichlink/trash/external-grafts/`
the entry at …/src/lib.rs:9 now reads `cut(crate::button::NODE_ID) graft(crate::button::NODE_ID)`
faces 2
  root/button  ButtonFast  button/button.rs
  root/slider  Slider  slider/slider.rs
$ cat host/src/button/button.rs     # 上表右列即该文件内容
$ sed -n '14,40p' graft/src/button_fast.rs   # 上表左列
```

### ② 落地后仍在生成树里，且计划不再指向项目外 crate

两者都由构建产物直接读到（`cargo test` 之后，`CARGO_TARGET_DIR=…/cargo-target`）：

```console
$ cat …/cargo-target/debug/build/host-*/out/source_scope.tsv
# node	source	module
4cd43f787fa9a4de011ff2fb9212c330	button/button.rs	button          <- 这个面在生成的作用域里
a7e347c04f3bae92a7d9d74d50be3103	slider/slider.rs	slider

$ cat …/out/pruning_manifest.tsv
4cd43f787fa9a4de011ff2fb9212c330	button/button.rs	-	-	ButtonFast	-	-

$ cat …/out/graft_plan.tsv
crate::button::NODE_ID	crate::button::NODE_ID	false	9	1                       <- button：不再有 control_button_graft
crate::slider::NODE_ID	control_button_graft::slider_fast::NODE_ID	false	9	1      <- slider：保持外部（要求④⑤）

$ grep -n 'NODE_ID\|REGISTRATION' …/out/generated_lib.rs
61: StaticGraftCut::from_ids(crate::button::NODE_ID, crate::button::NODE_ID, false)
62: StaticGraftCut::from_ids(crate::slider::NODE_ID, control_button_graft::slider_fast::NODE_ID, false)
80:  button::REGISTRATION,
81:  slider::REGISTRATION,

$ cat …/cargo-target/nichlink/cache/units/7270d8a9….tsv
path	button/button.rs
node	4cd43f787fa9a4de011ff2fb9212c330
kind	ButtonFast
contract	exports	["control.render"]
contract	handle_traits	["ControlHandle"]

$ nl --call explain --overlay true --root …/host
slots 2 (replaced 2):
  root/button   kind=ButtonFast  <- graft=crate::button::NODE_ID          full=false form=typed (entry line 9)
  root/slider   kind=Slider      <- graft=control_button_graft::slider_fast::NODE_ID full=false …
```

要点：那条 cut **不是被删掉**——它仍然点名 `root/button`（`graft(crate::button::NODE_ID)`），这正是
"这个面留在生成树里"的机制；消失的只是它对外部 crate 的指向。生成树里的 button 记录
（`source_scope.tsv` / 单元缓存 / `button::REGISTRATION`）都在。

### ③ 记录目录进了 trash，原位置不存在

```console
$ find .nichlink -type f | sort
.nichlink/trash/external-grafts/button_fast-1791038135184712221/graft.plan
.nichlink/trash/promoted/1791038135183791710/src/lib.rs
$ ls -la .nichlink/external-grafts/      # 只剩空目录，button_fast/ 不存在
total 8
drwxr-xr-x 2 nich users 4096 …  .
$ test -e .nichlink/external-grafts/button_fast && echo EXISTS || echo GONE
GONE
$ cat .nichlink/trash/external-grafts/button_fast-*/graft.plan
version=1
target=55ee21e05ad4374175ec37b0624658ee
target_path=root/button
graft=button_fast
full=false
```

### ④ `../graft/` 保留，且仍给 slider 用

```console
$ ls ../graft/src
button_fast.rs  control_fast.rs  lib.rs  slider_fast.rs        # 四个文件都在（mtime 09月29日，未改）
$ grep -n slider host/src/lib.rs
10:    cut(crate::slider::NODE_ID) graft(control_button_graft::slider_fast::NODE_ID),
$ cargo test --offline   # 输出 "Compiling nichlink-example-control-button-graft v0.1.0 (…/tree/graft)" —— 仍被构建
```

### ⑤ slider 一字不许改

声明与文件用 diff 证；计划里的 slider 那行被 lib.rs 的改前备份直接对比出来：

```console
$ diff -u .nichlink/trash/promoted/*/src/lib.rs src/lib.rs
@@ -8,6 +8,6 @@
 nichlink_toolchain::runtime::static_graft_plan!(
     FRAMEWORK,
-    cut(crate::button::NODE_ID) graft(control_button_graft::button_fast::NODE_ID),
+    cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),
     cut(crate::slider::NODE_ID) graft(control_button_graft::slider_fast::NODE_ID),
 );
```

整个 lib.rs 的差异**只有 button 那一行**；slider 的声明文件 `src/slider/slider.rs` 自始至终
未被写入（`grep -c 'tests' ` 仍在，内容与开工时逐字相同）。`graft_plan.tsv`/`generated_lib.rs`
里 slider 那条也仍是 `control_button_graft::slider_fast::NODE_ID`。

### ⑥ 项目根 `cargo test --offline` 绿，测试一条没删

```console
$ cd /tmp/scen-runs/S7-ours/tree/host
$ touch src/button/button.rs   # 强制重编，确保跑的是落地后的源码
$ CARGO_TARGET_DIR=/tmp/scen-runs/S7-ours/cargo-target cargo test --offline
   Compiling host v0.1.0 (/tmp/scen-runs/S7-ours/tree/host)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.44s
running 1 test
test slider::tests::the_slider_face_still_exists ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Doc-tests host: running 0 tests → ok
EXIT=0

$ nl --call check --root …/host --target-dir /tmp/scen-runs/S7-ours/cargo-target
verdict  passed (cargo exit 0)
tree   4 rust file(s), 2 function(s)
```

树里原本就只有 `src/slider/slider.rs` 一个 `#[test]`（开工前 `find` 列文件 + `cat` 已确认），
它现在照跑；`src/button/button.rs` 本来没有测试，落地面也没有。

## 我手动跑过的命令

- `export BENCH_CALLS=/tmp/scen-runs/S7-ours/calls.log`
- `nl --list` / `nl --list apply` / `nl --shapes`
- `nl --call grafts --root …/host`
- `nl --call apply --action promote --selector button_fast --root …/host`（**被拒**：promote 预览也要求
  `confirm: true`）
- `nl --call apply --action promote --selector button_fast --confirm true --root …/host`（预览）
- `nl --call apply --action promote --selector button_fast --apply true --confirm true --root …/host`（写入）
- `nl --call check --root …/host --target-dir /tmp/scen-runs/S7-ours/cargo-target`
- `nl --call explain --node root/button --overlay true --root …/host`（**被拒**：`overlay` 要整棵树、
  不能带 `node`）
- `nl --call explain --overlay true --root …/host`
- `CARGO_TARGET_DIR=…/cargo-target cargo test --offline`（多次，含 `touch` 后强制重编）
- 读证：`find`/`cat`/`diff`/`grep`/`sed` 于 `host/**`、`graft/**`、`cargo-target/**`

## 工具调用计数

`/tmp/scen-runs/S7-ours/calls.log` 共 **10** 条 `nl` 调用（8 条有答复，2 条拒绝）：
`--list`×2、`--shapes`×1、`grafts`×1、`apply promote`×3（1 拒 / 1 预览 / 1 落地）、`check`×1、
`explain`×2（1 拒 / 1 答复）。**重复预览 0 次**；两次拒绝分别是
"promote 即使预览也要求 confirm"（工具契约）与"`explain --overlay` 不接受 `node`"（我的用法），
都不改写任何文件。

## 没做到 / 需要说明的

没有未完成项。两点边界说明：

1. `apply promote` 的预览本身也要 `confirm: true`（第一次调用因此被拒）；这是当前工具的契约，
   不是我这边的遗漏——写进这里是给下一轮省一次调用。
2. `explain` 会说 `build stale (no source_scope.tsv)`：因为它按 `<root>/target/nichlink/out` 找记录，
   而本轮按纪律把 `CARGO_TARGET_DIR` 指到了 `…/cargo-target`，产物在那边。生成树的证据我取自
   `…/cargo-target/debug/build/host-*/out/*` 与 `…/cargo-target/nichlink/cache/units/*`，**不是**空记录。
