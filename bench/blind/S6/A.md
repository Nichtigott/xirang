# S6 · 在 button 自己的文件里加深一层零件层 · 我们臂

run 目录：`/tmp/scen-runs/S6-ours/`（`tree/` = 被测树，`cargo-target/` = 构建目录，`baseline/`、`after/` = 改动前后的证据快照，`calls.log` = wrapper 调用记录）
工具调用：`nl` wrapper **18 次**（全部 MCP 工具调用都走 wrapper；`--list`/`--help`/`--shapes` 这类说明页也计入，见文末清单）。

## 结论

零件层已经**写在 button 自己的文件**里；面的数量与身份、公开路径、构建期计划、其它文件都没动；`cargo test --offline` 绿。

## ① 改动落在哪个文件

**唯一被改的文件：`/tmp/scen-runs/S6-ours/tree/src/button/button.rs`**

```
baseline  d2775e819a3a883339aefe65c0332f30fd367d864b61f9acf677c0f6307e7f06  ./src/button/button.rs
after     29b143e5620caab7de3340ba6dec9781c6c4841426eb3f200b5fa00271d4dd98  ./src/button/button.rs
```

写它的调用（唯一一次写）：

```
nl --call apply --action deepen --node root/button \
   --inside '{"parts":{"label":"String","pressed":"bool"}}' --apply true \
   --root /tmp/scen-runs/S6-ours/tree
```

`apply` 的结果行：`applied /tmp/scen-runs/S6-ours/tree/src/button/button.rs`。先跑过一次不带 `--apply true` 的同参数调用（预览给出 diff），然后才落盘。

## 零件层是什么

`src/button/button.rs` 现在（行号为定稿后）：

| 行 | 内容 |
| --- | --- |
| 9–11 | `pub struct Button { parts: ButtonParts }` —— 面本身从 unit struct 变成"携带一个私有 parts 字段" |
| 13–16 | `crate::root_object! { kind: Button, parent: crate::root_node_id(env!("CARGO_PKG_NAME")) }` —— 声明**一字未动** |
| 18–27 | `pub struct ButtonParts { pub label: String, pub pressed: bool }` —— 零件结构体 |
| 29–32 | `impl nichlink_toolchain::runtime::PartsContract for ButtonParts { type Output = ButtonParts; const PROVIDED_PARTS: &'static [&'static str] = &["label", "pressed"]; }` |
| 34–39 | `impl Button { pub fn parts(&self) -> &ButtonParts { &self.parts } }` —— 访问它的 accessor |

即：**一个 parts struct（`ButtonParts`，两个零件 `label: String` / `pressed: bool`）+ 一个访问它的 accessor（`Button::parts(&self) -> &ButtonParts`）**，两层都落在 button 自己的文件里，没有新面、没有新文件、没有动 `src/lib.rs` 的 graft 计划。

## ② 树的形状一个字都没动 —— 判据怎么核的

要守住的是四个可观察维度：面的数量与身份、公开路径、构建期计划、其它文件。逐维核法：

### (a) 主证据（一条命令 + 它的输出）：整棵树的文件哈希只有 button.rs 变

```
$ cd /tmp/scen-runs/S6-ours
$ (cd tree && find . -type f -not -path './target/*' | sort | xargs sha256sum) > baseline/tree.sha256   # 改动前
$ (cd tree && find . -type f -not -path './target/*' | sort | xargs sha256sum) > after/tree.sha256      # 改动后
$ diff baseline/tree.sha256 after/tree.sha256
4c4
< d2775e819a3a883339aefe65c0332f30fd367d864b61f9acf677c0f6307e7f06  ./src/button/button.rs
---
> 29b143e5620caab7de3340ba6dec9781c6c4841426eb3f200b5fa00271d4dd98  ./src/button/button.rs
```

八条哈希里只有一条不同 ⇒ **其它文件（`Cargo.toml`、`Cargo.lock`、`build.rs`、`src/lib.rs`、`src/label/label.rs`、`src/slider/slider.rs`、`.vscode/nichlink-face.code-snippets`）逐字节没动**；改动面收敛在被点名的那一个文件里。

### (b) 面的数量与身份、公开路径：`verify` + `registry`

```
$ nl --call verify --root /tmp/scen-runs/S6-ours/tree
verdict ok (the kernel accepted the tree)
connector verdict: ok (the package's own faces were admitted)
build current
faces 3 (source) vs 3 (build)
added since build 0  gone 0  re-identified 0
the build matches the sources face for face
```

```
$ nl --call registry --root /tmp/scen-runs/S6-ours/tree    # 改动前后各一次
faces 3
root/button   Button   button/button.rs   3e1f685cf66d1a661f5d9cc0285e324f
root/label    Label    label/label.rs     68c941df41a8044cda4826fef0edc37c
root/slider   Slider   slider/slider.rs   9174fc9f4a0848306a693b2ae2f4a9fd
$ diff baseline/registry.txt after/registry.txt    →  IDENTICAL
```

```
$ nl --call explain --node root/button ...          # 改动前后逐行相同
node 3e1f685cf66d1a661f5d9cc0285e324f
  path root/button      kind Button      source button/button.rs
  module button         parent 47cc4542db60545ea3a528fd7e27e6da   registry_name button
```

3 个面、3 个同样的 NodeId、同样的逻辑路径 `root/button`、同样的模块名 `button`、`added 0 / gone 0 / re-identified 0` ⇒ 数量和身份、公开路径都没动。

### (c) 构建期计划：`cargo test` 后取 build script 的 `OUT_DIR`，逐文件比对

计划产物在 `/tmp/scen-runs/S6-ours/cargo-target/debug/build/control-4a2b5e438603350e/out/`（纪律③把 `CARGO_TARGET_DIR` 指到 `cargo-target/`，所以发布记录在这里，而不是 `tree/target/nichlink/out`）。

```
$ diff baseline/out/generated_lib.rs        after/out/generated_lib.rs        →  IDENTICAL
$ diff baseline/out/graft_plan.tsv          after/out/graft_plan.tsv          →  IDENTICAL
$ diff baseline/out/pruning_manifest.tsv    after/out/pruning_manifest.tsv    →  IDENTICAL
$ diff baseline/out/source_scope.tsv        after/out/source_scope.tsv        →  IDENTICAL
```

也就是说：生成计划（`BUILTIN_STATIC_FACES` / `BUILTIN_GRAFT_CUTS` / `BUILTIN_STATIC_PLAN`、`root_object!` 展开、`assert_contract` 钉子）、graft 计划表（3 条 `cut`/`graft`，`full=false`，entry line 8）、pruning 清单（3 行）、scope 表（3 行 selected）**全部逐字节相同**。`nl --call explain --overlay true` 也仍报 `slots 3 (replaced 3)`，三条槽位与改动前一致。

### (d) 诚实补一句：计划目录里确实有两个文件变了，但它们不是"树的形状"

```
$ diff baseline/out.sha256 after/out.sha256
1,2c1,2
< d96776b03e786ad0b500670f170935c4f927599df5b31d0ee6419a9793639de0  discovery.fingerprint
< a40f10b76ec950252a8fb832fdbba3f00c592fbd067bb1b53b7908a31fda1b29  function_manifest.tsv
---
> 36ebc3a273f004902c8488b3106011b374931081b5627775ea95351689551166  discovery.fingerprint
> 84807bfcc77c44c303871f44579bd46952658595e1e4a97277d5107b70c0ac3d  function_manifest.tsv
```

`function_manifest.tsv` 多了一行：

```
3e1f685cf66d1a661f5d9cc0285e324f  button/button.rs  button::parts
```

这是新增 accessor 的函数普查行（`discovery.fingerprint` 是源码内容指纹），是"加了零件层"的必然结果，不是面/槽位/计划的变化——面的行、槽位的行、pruning 行、scope 行都在上面 (c) 里证明逐字节未动。

## ③ `cargo test --offline` 绿

改动前后各跑一次（都带 `CARGO_TARGET_DIR`）：

```
$ cd /tmp/scen-runs/S6-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S6-ours/cargo-target cargo test --offline
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.61s
running 1 test
test slider::tests::the_slider_face_is_still_there ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests control
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

工具面复核：

```
$ nl --call check --target_dir /tmp/scen-runs/S6-ours/cargo-target --census true --root /tmp/scen-runs/S6-ours/tree
verdict  passed (cargo exit 0)
tree   5 rust file(s), 3 function(s)
result test result: ok. 1 passed; 0 failed; ...
```

## 我亲手跑过的命令（shell / cargo）

```
ls -la /tmp/scen-runs/S6-ours/ ; find /tmp/scen-runs/S6-ours/tree -type f -not -path '*/cargo-target/*'
cd /tmp/scen-runs/S6-ours/tree && for f in Cargo.toml build.rs src/lib.rs src/button/button.rs src/slider/slider.rs src/label/label.rs; do cat -n "$f"; done
cd /tmp/scen-runs/S6-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S6-ours/cargo-target cargo test --offline          # 改动前
find /tmp/scen-runs/S6-ours -type d -name out ; cat .../build/control-*/out/generated_lib.rs
cd /tmp/scen-runs/S6-ours && (cd tree && find . -type f -not -path './target/*' | sort | xargs sha256sum) > baseline/tree.sha256
cd /tmp/scen-runs/S6-ours && (cd cargo-target/debug/build/control-*/out && sha256sum * | sort -k2) > baseline/out.sha256
# （落盘：nl --call apply … --apply true）
cd /tmp/scen-runs/S6-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S6-ours/cargo-target cargo test --offline          # 改动后
diff baseline/tree.sha256 after/tree.sha256 ; diff baseline/out.sha256 after/out.sha256
diff baseline/out/generated_lib.rs after/out/generated_lib.rs  （graft_plan.tsv / pruning_manifest.tsv / source_scope.tsv 同）
diff baseline/registry.txt after/registry.txt
```

所有 MCP 工具调用共 **18 次**，清单：

```
 1 nl --list                                      （工作流表/工具清单）
 2 nl --list explain
 3 nl --list apply
 4 nl --call registry        （改动前）             5 nl --call grafts          （改动前）
 6 nl --call explain --node root/button （改动前）
 7 nl --help                                      8 nl --shapes
 9 nl --call apply … --inside parts             （被拒，参数形状提示）
10 nl --call apply … inside.parts          （预览，返回 diff）
11 nl --call registry        （改动前的快照副本）
12 nl --call apply … --apply true          （★ 唯一的写）
13 nl --call check --target_dir … --census true
14 nl --call registry        （改动后）            15 nl --call explain --node root/button （改动后）
16 nl --call grafts          （改动后）            17 nl --call explain --overlay true
18 nl --call verify
```

## 没做到 / 未覆盖的，说在这里

1. **新 accessor 没有测试碰它。** `nl --call check --census true` 自己报：`fn no test reaches 'parts' (src/button/button.rs:37)`、`test-reachable 1 of 1 production function(s) no test can reach`。题面③只要求 `cargo test --offline` 绿，而且②要求"其它文件不许变"，所以我**没有**为它补测试——补一条测试要改 `src/lib.rs`（或 button.rs 里的 `#[cfg(test)]`），那会污染②的判据。
2. **`explain` 的 `build stale` / `scope unknown` 我没有消掉。** wrapper 的 `explain` 固定读 `<tree>/target/nichlink/out`，而纪律③要求所有 cargo 命令带 `CARGO_TARGET_DIR`，于是发布记录落在 `cargo-target/…/out`，那儿没有 `source_scope.tsv`。我的做法是**直接用 OUT_DIR 里的计划产物逐字节比对**（(c) 段），而不是引用 `explain` 的 scope 行。
3. **`Button` 不再是 unit struct。** 现在没有任何地方构造 `Button`（本树里没人把 `Button` 当值用），所以编译过；将来若有人写 `let _ = Button;` 或字面量模式会编译失败——这是 `apply` 的 `note` 已经点名的、零件层方案本身的代价。
4. 面内部的 `parts` 字段是**私有**的，只有 `Button::parts()` 这个 accessor 能读；`ButtonParts` 里没有提供构造函数（deepen 不写构造方）。题面只要求"一个 parts struct + 一个访问它的 accessor"，两样都在，但**没有**默认值/构造入口，这点没有做到"可直接拿来用"的程度。
