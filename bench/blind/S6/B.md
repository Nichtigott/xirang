# S6-cg 答案（对照臂）

场景：`/tmp/scen-runs/S6-cg/tree` 的 **button** 面要在**它自己的文件里**加深一层（一个 parts struct + 一个访问它的 accessor），
而树的形状一个字都不许动。

## 1. 改动落在哪个文件

**`/tmp/scen-runs/S6-cg/tree/src/button/button.rs`** —— button 面**自己的文件**。没有新建文件，
`src/lib.rs`、`src/label/label.rs`、`src/slider/slider.rs`、`build.rs`、`Cargo.toml`、`Cargo.lock`、
`.vscode/nichlink-face.code-snippets` 一个字节都没碰（见 §3 第 4 项）。

（过程中为了摸出 `parts:` 字段的契约，对同一文件做过两次中间态编译：`parts: ButtonParts` 但没有 impl
→ `E0277: the trait bound ButtonParts: PartsContract is not satisfied`；补一个空 impl → `E0046` 提示缺
`Output` 与 `PROVIDED_PARTS`。终态如下。）

## 2. 零件层是什么

终态 `src/button/button.rs`（完整 51 行）：

```rust
// generated-by=NichLink
//! Button registration face.
//! Button 注册面。

use crate::{NoParts, NoPreset, PartsContract};

/// Registration-only marker for the Button face.
/// 仅用于 Button 注册面的 handle 标记，不代表运行时 object 实现。
pub struct Button;

/// The parts layer of the Button face: the pieces this face is assembled from.
/// Button 面的零件层：这个面由哪些零件装配而成。
pub struct ButtonParts {
    /// The caption part this face carries.
    /// 该面携带的标题零件。
    pub caption: &'static str,
}

impl PartsContract for ButtonParts {
    /// The face produces nothing beyond this parts layer.
    /// 除了这层零件之外，该面不产出任何东西。
    type Output = ();

    /// The one part this layer provides.
    /// 该零件层提供的那一个零件。
    const PROVIDED_PARTS: &'static [&'static str] = &["caption"];
}

impl Button {
    /// Access this face's parts layer.
    /// 访问该面的零件层。
    pub const fn parts() -> ButtonParts {
        ButtonParts { caption: "Button" }
    }
}

crate::root_object! {
    kind: Button,
    parts: ButtonParts,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_layer_is_reachable_from_the_face() {
        assert_eq!(Button::parts().caption, "Button");
    }
}
```

- **零件 struct**：`ButtonParts { caption: &'static str }`（button.rs:11-17）。
- **契约**：`impl PartsContract for ButtonParts`（button.rs:19-27）。这是必需的——生成文件里有一条
  `assert_contract::<crate::button::__Preset, crate::button::__Parts>()`，它要求 `__Parts` 实现
  `PartsContract` 且 `Output` 与该面 `__Preset` 的 `Output` 相等（`NoParts` 走的就是这条契约）。
- **访问它的 accessor**：`Button::parts() -> ButtonParts`（button.rs:29-35），`const fn`，从面标记直接取到零件层。
- **在注册面里声明**：`root_object! { kind: Button, parts: ButtonParts, parent: … }`（button.rs:37-41）。
  `parts:` 的位置符合生成文件文档给出的字段顺序（`kind` → `preset` → `parts` → … → `parent`），
  所以零件层是**这个面自己的**一层，不是新节点。
- 附一条测试（button.rs:47-50）让 accessor 真的可达，避免"只编译不用"。

## 3. ② 的判据怎么核的（一条命令 + 输出）

**一条命令**：

```sh
bash /tmp/scen-runs/S6-cg/cargo-target/snap/prove.sh          # exit 0
```

它把 baseline 快照（`…/cargo-target/snap/before/`，在编辑**之前**跑 `cargo build --offline` 抓的）
与编辑后的 `OUT_DIR` 对比。输出：

```
== 1. faces: count + identity (pruning_manifest.tsv, node ids) ==
   face node=3e1f685cf66d1a661f5d9cc0285e324f  source=button/button.rs  kind=Button
   face node=68c941df41a8044cda4826fef0edc37c  source=label/label.rs  kind=Label
   face node=9174fc9f4a0848306a693b2ae2f4a9fd  source=slider/slider.rs  kind=Slider
   face count = 3
   StaticFace entries in generated_lib.rs = 3
   StaticGraftCut entries in generated_lib.rs = 3
== 2. build-time plan / registration artifacts: byte-diff vs baseline ==
   generated_lib.rs: IDENTICAL
   graft_plan.tsv: IDENTICAL
   pruning_manifest.tsv: IDENTICAL
   source_scope.tsv: IDENTICAL
== 3. public paths (from generated_lib.rs, byte-identical above) ==
   pub mod button;
   pub mod label;
   pub mod slider;
   pub use ::nichlink_toolchain::runtime::registry_core::*;
   const _: () = ...::assert_contract::<crate::button::__Preset, crate::button::__Parts>();
   const _: () = ...::assert_contract::<crate::slider::__Preset, crate::slider::__Parts>();
   const _: () = ...::assert_contract::<crate::label::__Preset, crate::label::__Parts>();
== 4. every other tree file (all except src/button/button.rs): sha256 vs baseline ==
   all other files: IDENTICAL (7 files)
== 5. intended deltas caused by editing button.rs itself (reported, not hidden) ==
   @@ -1,2 +1,4 @@
    # node	source	symbol
   +3e1f685cf66d1a661f5d9cc0285e324f	button/button.rs	button::parts
   +3e1f685cf66d1a661f5d9cc0285e324f	button/button.rs	button::the_parts_layer_is_reachable_from_the_face
    9174fc9f4a0848306a693b2ae2f4a9fd	slider/slider.rs	slider::the_slider_face_is_still_there
   discovery.fingerprint: c2e34ff13f50d924a9705f1b7c9b1cab -> 9232768b3e5d39e4bad1fcf4564584aa  (content cache key, sampled on button.rs)
   every new node id above is the SAME button face node id as the baseline row: 3e1f685cf66d1a661f5d9cc0285e324f
== verdict ==
   FACES_UNCHANGED=yes  PLAN_UNCHANGED=yes  OTHER_FILES_UNCHANGED=yes
```

对应题面四项：

| 题面要求 | 证据 |
| --- | --- |
| 面的数量与身份 | 3 个面，node id `3e1f685c…`(Button) / `68c941df…`(Label) / `9174fc9f…`(Slider)，与 baseline 逐字节相同；`StaticFace::new` 3 条、`StaticGraftCut::from_ids` 3 条不变 |
| 公开路径 | `generated_lib.rs` 逐字节相同 ⇒ `pub mod button/label/slider;`、`pub use registry_core::*`、三条 `assert_contract::<__Preset, __Parts>` 全不变。新增的 `ButtonParts` / `Button::parts` 是 button 模块**内部**的新公开项，不改任何既有路径 |
| 构建期计划 | `generated_lib.rs`、`graft_plan.tsv`、`pruning_manifest.tsv`、`source_scope.tsv` 对 baseline 全部 `diff -q` IDENTICAL |
| 其它文件 | 除 `src/button/button.rs` 外 7 个文件 sha256 全同 |

**没做到的部分（如实说）**：编辑 button.rs 本身必然让两个**发现侧**产物动：

1. `function_manifest.tsv` 多两行，而两行的 node id 就是**同一个** button 面 id `3e1f685c…`——也就是
   没有新节点，函数只是挂在同一个面上；
2. `discovery.fingerprint` 从 `c2e34ff1…` 变成 `9232768b…`。这一点我做了个对照：把 button.rs 还原成
   原件重新构建，指纹**精确回到** `c2e34ff1…`，再换回终态又变成 `9232768b…`——它是按被改动文件内容取的
   发现期指纹/缓存键，不是计划本身。

这两个不是"树的形状"，而是"被改文件自身内容"的函数：只要 ① 成立它们必然动。如果"构建期计划"被理解为
连 `function_manifest.tsv` / `discovery.fingerprint` 也必须逐字节不变，那么 ① 与 ② 不可兼得（要加一个
accessor 函数就必须多一行函数清单）。我按"计划 = generated_lib.rs + graft_plan.tsv"这个可观察终态报，并把
两个移动项原样列出，没有藏。

## 4. ③ 判据：`cargo test --offline` 绿

```sh
cd /tmp/scen-runs/S6-cg/tree
CARGO_TARGET_DIR=/tmp/scen-runs/S6-cg/cargo-target cargo test --offline   # exit 0
```

```
running 2 tests
test button::tests::the_parts_layer_is_reachable_from_the_face ... ok
test slider::tests::the_slider_face_is_still_there ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
（Doc-tests: 0 passed; 0 failed）
```

编辑前 baseline 同一条命令是 `1 passed`（只有 slider 那条）；新增的那条就是零件层这条。

## 5. 我亲手跑过的命令

所有 cargo 都在 `cwd=/tmp/scen-runs/S6-cg/tree`、带 `CARGO_TARGET_DIR=/tmp/scen-runs/S6-cg/cargo-target`：

- `cargo build --offline`（baseline，抓 OUT_DIR）；`cargo test --offline`（baseline：1 passed）
- `cargo build --offline` ×3（两次中间态探 `PartsContract`，一次终态）；`cargo test --offline`（终态 2 passed）
- 对照组：`cp` 还原 `button.rs.orig` → `cargo build --offline` → 读指纹 → 换回终态 → `cargo build --offline` → `cargo test --offline`
- 快照/证据：`find … -exec sha256sum` 抓 baseline 树哈希；`cp OUT_DIR/* snap/before/`；对每个产物 `diff -u` / `diff -q`；`bash snap/prove.sh`

工具调用全部走 `cg` wrapper（`BENCH_CALLS=/tmp/scen-runs/S6-cg/calls.log` 已 export），且每条查询都带
`-p /tmp/scen-runs/S6-cg/tree`，**没有**任何一次把 `-p/--path/--project` 指向检出：

| # | 调用 | 结果 |
| --- | --- | --- |
| 1 | `cg explore --help` | 看 `-p` 选项 |
| 2 | `cg init /tmp/scen-runs/S6-cg/tree` | 索引 5 文件 / 14 节点（树内新建 `.codegraph/`） |
| 3 | `cg explore -p …/tree "button face registration root_object Button parts accessor" --max-files 4` | 编辑前定位：button.rs 就是 button 面自己的文件 |
| 4 | `cg sync -p …/tree` | **被拒**：`error: unknown option '-p'`（sync 只接位置参数） |
| 5 | `cg explore -p …/tree "ButtonParts Button parts accessor parts layer" --max-files 1` | 编辑后读回 button.rs，零件层在位 |
| 6 | `cg sync /tmp/scen-runs/S6-cg/tree` | Synced 1 changed file |
| 7 | `cg query -p …/tree ButtonParts` | `struct ButtonParts` + `method parts () -> ButtonParts` |

另外说明：读/写树内文件时我还用了宿主自带的 read/write 文件工具（都在 `/tmp/scen-runs/S6-cg/tree` 内，
未读禁读区），这些**不**经 `cg` 也不进 `BENCH_CALLS`；上面表里的 7 次是 codegraph 的计数。

## 6. codegraph 调用次数

**7 次**（6 次成功 + 1 次 `cg sync -p` 参数被拒）。计数命令：

```sh
grep -c '^=== CMD: ' /tmp/scen-runs/S6-cg/calls.log   # -> 7
```

## 7. 备注

- 树里新增的 `.codegraph/` 是 `cg init` 的索引产物（不在 `src/` 内，也不是面的文件）；`prove.sh` 的
  "其它文件"比对已经把它排除。
- 全程没有读 `/home/nich/Moirai_N3/nichlink` 下的任何其它路径：只执行了 wrapper `bin/cg`，并让 cargo 自己
  去读它声明的 `path` 依赖（`toolchain/`）——那是构建的既成事实，不是我用工具去翻检出。
