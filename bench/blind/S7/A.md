# S7 对照臂 · 把项目外 `button_fast` 实现落地进基树

运行目录：`/tmp/scen-runs/S7c-cg/`（树 `/tmp/scen-runs/S7c-cg/tree/`，cargo 目标 `/tmp/scen-runs/S7c-cg/cargo-target/`）
工具调用记录：`/tmp/scen-runs/S7c-cg/calls.log`（codegraph wrapper 调用 **6** 次，见文末）

## 0. 结论

六条要求全部做到；`cargo test --offline` 绿（1 条已有 + 1 条我新增的钉子）。
落地后的 Button 面**仍在生成树里**（生成计划里 2 个 face、`pub mod button;`、运行时
`registrations().len() == 2` 且 `source.file == "button/button.rs"`），计划里那条指向项目外
crate 的 `cut(...)` 已经不在（生成的 `graft_plan.tsv` 只剩 slider 一条）。

## 1. 改了哪些文件

| 路径（相对 `tree/host`） | 改动 |
| --- | --- |
| `src/button/button.rs` | 重写注册面声明：由外部实现 `button_fast` 声明的那一份照抄而来（`kind` 也随之变成 `ButtonFast`，标记类型 `pub struct Button` → `pub struct ButtonFast`） |
| `src/lib.rs` | `static_graft_plan!` 里删掉 `cut(crate::button::NODE_ID) graft(control_button_graft::button_fast::NODE_ID)`；新增 `pub const BUTTON_FACE_NODE: NodeId = crate::button::NODE_ID;` 在入口点名该槽位（理由见 §3②） |
| `tests/landed_button_face.rs` | **新增**（没有删任何已有测试）：钉住「落地后的 Button 面带的是外部那份值、且仍被生成树挂载」 |
| `.nichlink/external-grafts/button_fast/` → `.nichlink/trash/button_fast/` | 整目录移入 trash（内容一字未动） |

未动：`src/slider/slider.rs`、`Cargo.toml`、`build.rs`、`../graft/**`、`.vscode/**`。

`tree/` 下"场景初始化（22:45）之后被改过的文件"（`find -newermt '2026-10-03 22:46'`）：
`host/src/button/button.rs`、`host/src/lib.rs`、`host/tests/landed_button_face.rs`，以及我为了用 codegraph 建的 `tree/.codegraph/**`。
`../graft/` 四个源文件一个都没出现在这份列表里。

## 2. 我手动跑过的命令（摘要）

```sh
export BENCH_CALLS=/tmp/scen-runs/S7c-cg/calls.log
CG=/home/nich/Moirai_N3/nichlink/bench/bin/cg

$CG --help                                             # 看命令集
$CG init /tmp/scen-runs/S7c-cg/tree --no-color         # 给「我自己的树」建索引（8 files / 24 nodes）
$CG explore --no-color -p /tmp/scen-runs/S7c-cg/tree "…button_fast…static_graft_plan…"
$CG explore --no-color -p /tmp/scen-runs/S7c-cg/tree "…blast radius…button NODE_ID…"
$CG status --no-color -p /tmp/scen-runs/S7c-cg/tree   # ← 该子命令不认 -p，exit 1（唯一一次失败调用）
$CG status --no-color /tmp/scen-runs/S7c-cg/tree      # 改用位置参数：Added 1 / Modified 2

# 全程 cargo 都带 CARGO_TARGET_DIR=/tmp/scen-runs/S7c-cg/cargo-target
cargo test --offline                                   # 基线（改动前）：1 passed
cargo build --offline                                  # 改动后第一版：被 .nichlink 里的外部记录挡住（见 §3③）
mkdir -p .nichlink/trash && mv .nichlink/external-grafts/button_fast .nichlink/trash/button_fast
cargo test --offline                                   # 绿
cargo clean -p host && cargo test --offline            # 从零重建 host 后再验一次，仍绿
```

中间为了定位"删掉 cut 后面为什么消失"，我做过几次**只改 `src/lib.rs`/`button.rs` 的对照构建**
（脚本 `/tmp/scen-runs/S7c-cg/variant.sh`、`libvariant.sh`，日志 `V0-original.build.log`、
`L1-bare-ref.lib.log`、`P1-keep-probe.lib.log`、`P2-application-probe.lib.log`、
`P3-verb-probe.lib.log`）——这些是 shell + cargo，没有消耗 codegraph 调用。

## 3. 逐条怎么核的

### ① `button.rs` 的声明 = `button_fast` 自己声明的那一份

```
diff <(sed -n '/kind: ButtonFast/,/^}/p' ../graft/src/button_fast.rs | grep -v 'source:' | grep -v 'registry_rule_path') \
     <(sed -n '/kind: ButtonFast/,/^}/p' src/button/button.rs   | grep -v 'registry_rule_path')
8c8
<     parent: root_node_id(env!("CARGO_PKG_NAME")),
---
>     parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
```

即：`kind`/`name`/`summary`/`exports`/`handle_traits`/`flow` 六项**逐字节相同**（`preset`、`parts`、
`needs_registry`、`getting_from_other_registry`、`registry_rule`、`requires`、`provides`、
`runtime_checks` 也一并照抄）。运行时快照（`cargo test --offline --test probe_landed_button -- --nocapture`
的一次临时探针，已删）也对得上：

```
RegistrationInfo { id: 4cd43f787fa9a4de011ff2fb9212c330, kind: "ButtonFast", preset: "NoPreset", parts: "NoParts",
  name: LocalizedText { zh: "快速按钮", en: "Fast button" },
  summary: LocalizedText { zh: "项目外实现", en: "Out-of-project implementation" },
  exports: ["control.render"], handle_traits: ["ControlHandle"],
  flow: FlowContract { id: ContractId("control.render.v1"), version: 1, input: "ControlInput", output: "ControlFrame" },
  source: SourceLocation { file: "button/button.rs", … } }
```

三处**有意的改写**（都不是那六项，说明在此）：
1. `source: "button_fast/button_fast.rs"` 去掉了——宿主的 `root_object!`（= `__nichlink_object!`）
   不接受 `source:` 字段（那是 `external_object!` 专有的"来源声明"，见 `generated_lib.rs` 里的字段注释清单）。
2. `parent:` 写成 `crate::root_node_id(...)`——树内写法（原 `button.rs` 就是这么写的），值与外部那条相同。
3. `registry_rule_path:` 由 `"button_fast/button_fast.rs"` 改成 `"button/button.rs"`：这条本来就该
   指"声明所在的源码路径"，外部那条指它自己的文件，落地后自然指落地后的文件；运行时快照里它确实读作
   `button/button.rs`，`source.file` 也是 `button/button.rs`（证明值来自基树，不是那个外部 crate）。

### ② 面仍在生成树里，且那条 `cut(...)` 不再指向项目外 crate

`src/lib.rs` 现在只有 slider 那条 cut：

```rust
pub const BUTTON_FACE_NODE: NodeId = crate::button::NODE_ID;   // 入口点名该槽位

nichlink_toolchain::runtime::static_graft_plan!(
    FRAMEWORK,
    cut(crate::slider::NODE_ID) graft(control_button_graft::slider_fast::NODE_ID),
);
```

构建产物（`CARGO_TARGET_DIR=/tmp/scen-runs/S7c-cg/cargo-target/debug/build/host-c4cf0306e6784254/out/`，
`cargo clean -p host` 后重建仍是这份）：

| 文件 | 关键行 | 摘要 |
| --- | --- | --- |
| `source_scope.tsv` | `# selected 2`；`4cd43f78… button/button.rs button`；`a7e347c0… slider/slider.rs slider` | button 与 slider 都在源范围里 |
| `generated_lib.rs` | `pub mod button;`（41 行）、`BUILTIN_STATIC_FACES` 两个 `StaticFace::new`（button 的 node = `4cd43f78…`，slider 的 = `a7e347c0…`）、`registrations()` 里 `button::REGISTRATION, slider::REGISTRATION` | 生成树真的挂载了 button |
| `graft_plan.tsv` | 唯一一条 `crate::slider::NODE_ID  control_button_graft::slider_fast::NODE_ID  false` | button 那条 cut 已消失 |
| `generated_lib.rs` / `graft_plan.tsv` | `grep -c button_fast` = 0 / 0 | 计划里再没有任何项目外 button 引用 |

运行时可观察终态（不是"某一行被删掉"）：`cargo test --offline` 里我新增的钉子
`tests/landed_button_face.rs::the_landed_button_face_is_mounted_with_the_external_declarations_values`
断言 `registrations().len() == 2`、能找到 `kind == "ButtonFast"` 的面、六个字段值逐个相等、
且 `source.file == "button/button.rs"`——**通过**。

> 为什么 `src/lib.rs` 里那行 `BUTTON_FACE_NODE` 是必需的（实测结论，不是推测）：
> build.rs 的 source scope（`# mode auto`）只把**入口点名过的**面挂进生成树（点名的形式就是入口里
> 出现 `crate::<module>::…::NODE_ID`）。我先把 cut 删掉、其余不动重建，结果
> `source_scope.tsv` 变成 `selected 1`（只剩 slider），`generated_lib.rs` 里连 `pub mod button;`
> 都没有了——也就是说"把那一行删掉"会让这个面直接掉出生成树，②的前半条就不成立。
> 我又试了在 `static_graft_plan!` 里写 `keep(...)`/`slot(...)`/`face(...)`/`retain(...)`/`mount(...)`
> 这些形态（`P3-verb-probe`：rc=0，但 `selected` 仍是 1，`graft_plan.tsv` 里只有 slider），
> `runtime::application!(entry = …)` 在该版本里根本不存在（`L1/P2` 两个探针的报错），
> 所以"在入口直接点名 `crate::button::NODE_ID`"是这里唯一可用的显式挂载方式。落地后的实现仍然
> 是"基树里一份正常声明"，cut/graft 那一套已经不需要了——我没有留一条自指的假 cut。

### ③ 外部记录被移进 `.nichlink/trash/`

```
$ find .nichlink | sort
.nichlink
.nichlink/external-grafts
.nichlink/trash
.nichlink/trash/button_fast
.nichlink/trash/button_fast/graft.plan
$ test -e .nichlink/external-grafts/button_fast && echo YES || echo NO
NO
$ cat .nichlink/trash/button_fast/graft.plan     # 内容原封不动
version=1 / target=55ee21e05ad4374175ec37b0624658ee / target_path=root/button / graft=button_fast / full=false
```

（这条记录还是"旧 button 槽位"（`55ee21…`）的：落地后 button 的 node id 换成了 `4cd43f78…`，
所以它即便留在原处也已经无效——这一点在我删掉 cut 后第一次构建时就被构建期检查点破了：
`external graft plan button_fast targets root/button, which no declaration in the host entry names …`。
把记录移走后构建恢复。）

### ④ 项目外的 crate 保留

`../graft/src/` 仍是 `button_fast.rs`、`control_fast.rs`、`slider_fast.rs`、`lib.rs`；`host/Cargo.toml`
对 `nichlink-example-control-button-graft` 的依赖没动（slider 那条 cut 还指着它）。
`find tree -newermt '2026-10-03 22:46' -type f` 里没有任何 `graft/` 下的文件；
`cg status` 也报 `Modified: 2 files`（= `src/button/button.rs`、`src/lib.rs`）+ `Added: 1`（新测试）。

### ⑤ slider 一个字没动

- `src/slider/slider.rs` 不在"改动过的文件"列表里（`find -newermt` 与 `cg status` 双重印证）。
- 计划里 slider 那条 cut 原样保留：`cut(crate::slider::NODE_ID) graft(control_button_graft::slider_fast::NODE_ID)`，
  生成的 `graft_plan.tsv` 里仍是 `crate::slider::NODE_ID  control_button_graft::slider_fast::NODE_ID  false`。
- slider 的测试 `slider::tests::the_slider_face_still_exists` 仍在、仍绿。

### ⑥ 项目根 `cargo test --offline` 绿，已有测试一条没删

```
$ cargo clean -p host && cargo test --offline        # CARGO_TARGET_DIR=/tmp/scen-runs/S7c-cg/cargo-target
running 1 test
test slider::tests::the_slider_face_still_exists ... ok        ← 原有测试，保留
test result: ok. 1 passed; 0 failed
     Running tests/landed_button_face.rs
test the_landed_button_face_is_mounted_with_the_external_declarations_values ... ok   ← 我新增的
test result: ok. 1 passed; 0 failed
```

## 4. 没做到 / 打折扣的地方

- **没有**没做到的要求。
- 一次 codegraph 调用是失败的：`cg status -p …` 该子命令不接受 `-p`（exit 1），改用位置参数
  `cg status /tmp/scen-runs/S7c-cg/tree` 才成功。这是本次唯一的失败调用。
- 索引建在我自己树上后我没有再 `sync`（改完文件后 `cg status` 会报 `Pending Changes: Added 1 / Modified 2`）；
  因为后面不再查询，我没花一次调用去同步，索引快照是"改动前"的。
- `tree/.codegraph/` 是这次运行留下的索引目录（在允许的 `tree/` 里），没有删。
- `/tmp/scen-runs/S7c-cg/` 根下留了探针脚本与构建日志（`variant.sh`、`libvariant.sh`、
  `*-body.rs`、`*.log`），都是我过程的原始记录，供复核。

## 5. 工具调用计数

`calls.log` 里 `=== CMD:` 共 **6** 次：`--help` 1、`init` 1、`explore` 2、`status` 2（其中 1 次因 `-p` 失败）。
每一次 `query/explore/status` 都指着我自己的树（`-p /tmp/scen-runs/S7c-cg/tree`），**没有一次**指向
`/home/nich/Moirai_N3/nichlink`，也没有任何命令读过禁读区。
