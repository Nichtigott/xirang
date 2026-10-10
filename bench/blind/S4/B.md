# S4 对照臂 answer —— 新增控件 `slider`，不破坏既有形状

结论：新增 1 个文件（`src/slider/slider.rs`）、在宿主入口**追加** 1 行计划条目；既有 4 个面的源文件、
`Cargo.toml`、`build.rs`、`.vscode/*` 逐字节未变，既有计划条目逐字未变；`cargo test --offline` 绿
（3 passed，其中 2 条是新对象的）。判据 ①–④ 全部做到。codegraph 调用共 **10 次**（其中 1 次被拒、
1 次重复预览、另 0 次无谓调用，见 §6）。

所有命令都在 `/tmp/scen-runs/S4-cg/tree` 下执行，`CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target`，
`BENCH_CALLS=/tmp/scen-runs/S4-cg/calls.log`；工具调用一律走 wrapper
`/home/nich/Moirai_N3/nichlink/bench/bin/cg`，每次查询都带 `-p /tmp/scen-runs/S4-cg/tree`。
`/home/nich/Moirai_N3/nichlink` 下除 wrapper 之外的路径未被读取（唯一例外是 cargo 通过树自己的
`Cargo.toml` 里的 `path` 依赖去编译 `toolchain/`，那是构建本身，不是我主动去查）。

---

## 1. 新增 / 改动的文件

| 文件 | 动作 | before sha256 | after sha256 |
| --- | --- | --- | --- |
| `src/slider/slider.rs` | **新增** | — | `01beeb78d497878c4964a37dd34c6655b99ddbd23390b8a1386f5df07e18a19b` |
| `src/lib.rs` | 追加 1 行 | `f6e5a6c014fac38fbfb60e55918df9499dcf7fd442973908934ab675af2f0a4f` | `f007c58791d4882c09129aa19a8142e7b334b10b3f206427963581d340fc08d6` |

`src/lib.rs` 的改动就是插入一行，没有改动任何既有行（`diff -u lib.rs.orig src/lib.rs`）：

```
@@ -9,4 +9,5 @@
     FRAMEWORK,
     cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),
     cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),
+    cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID),
 );
```

`src/slider/slider.rs` 全文（同族形状的 1:1 镜像，见 §3）：

```rust
// generated-by=NichLink
//! Slider registration face.
//! Slider 注册面。

use crate::{NoParts, NoPreset};

/// Registration-only marker for the Slider face.
/// 仅用于 Slider 注册面的 handle 标记，不代表运行时 object 实现。
pub struct Slider;

crate::root_object! {
    kind: Slider,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}

#[cfg(test)]
mod tests { /* 两条测试，见 §5 */ }
```

---

## 2. 判据 ①：新面真的在树里（一条命令可复核）

**主命令**（别人一条命令就能复核；断言读的是构建产出的树，不是本文件里写的字）：

```sh
cd /tmp/scen-runs/S4-cg/tree && \
CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline -- slider
```

输出：

```
running 2 tests
test slider::tests::the_slider_face_is_still_there ... ok
test slider::tests::the_slider_parent_is_the_host_root ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

第一条测试做的是：`crate::registrations()`（由 `build.rs` 从生成计划读出，按树序）里必须存在一条
`registration.id == crate::slider::NODE_ID` 的注册；第二条断言它的 `parent` 就是宿主根
`crate::root_node_id(env!("CARGO_PKG_NAME"))`。

**独立旁证（产物层，同一条命令带出三份清单）**：

```sh
O=$(ls -d /tmp/scen-runs/S4-cg/cargo-target/debug/build/control-*/out) && \
grep -H slider $O/source_scope.tsv $O/graft_plan.tsv $O/pruning_manifest.tsv $O/generated_lib.rs
```

输出（节选，逐字）：

```
source_scope.tsv:9174fc9f4a0848306a693b2ae2f4a9fd	slider/slider.rs	slider
graft_plan.tsv:crate::slider::NODE_ID	crate::slider::NODE_ID	false	8	1
pruning_manifest.tsv:9174fc9f4a0848306a693b2ae2f4a9fd	slider/slider.rs	-	-	Slider	-	-
generated_lib.rs:#[path = "/tmp/scen-runs/S4-cg/tree/src/slider/slider.rs"]
generated_lib.rs:pub mod slider;
generated_lib.rs:const _: () = …assert_static_registration(…RegistrationRule::ANY, slider::REGISTRATION);
generated_lib.rs:    …StaticGraftCut::from_ids(crate::slider::NODE_ID, crate::slider::NODE_ID, false),
generated_lib.rs:    …assert_contract::<crate::slider::__Preset, crate::slider::__Parts>();
generated_lib.rs:        slider::REGISTRATION,
```

`generated_lib.rs` 里 `BUILTIN_STATIC_FACES` 现在是三条（前两条与改动前逐字节相同，见 §4）：

```rust
pub static BUILTIN_STATIC_FACES: &[StaticFace] = &[
    StaticFace::new(NodeId::from_raw([62, 31, 104, 92, 246, 109, 26, 102, 31, 93, 156, 192, 40, 94, 50, 79]), NodeId::from_raw([71, 204, 69, 66, 219, 96, 84, 94, 163, 165, 40, 253, 126, 39, 230, 218]), false),
    StaticFace::new(NodeId::from_raw([104, 201, 65, 223, 65, 168, 4, 76, 218, 72, 38, 254, 240, 237, 195, 124]), NodeId::from_raw([71, 204, 69, 66, 219, 96, 84, 94, 163, 165, 40, 253, 126, 39, 230, 218]), false),
    StaticFace::new(NodeId::from_raw([145, 116, 252, 159, 74, 8, 72, 48, 106, 105, 59, 42, 226, 244, 169, 253]), NodeId::from_raw([71, 204, 69, 66, 219, 96, 84, 94, 163, 165, 40, 253, 126, 39, 230, 218]), false),
];

pub fn registrations() -> Vec<RegistrationInfo> {
    vec![button::REGISTRATION, label::REGISTRATION, slider::REGISTRATION]
        .into_iter().collect()
}
```

（`StaticFace::new` 的第三个参数 `false` = 非 full，与新计划条目里的 `false` 一致。）

**发现由计划条目驱动，宿主入口不需要 `mod` 声明**：`src/lib.rs` 只追加了计划的一行，`build.rs` 仍只有
`nichlink_toolchain::build_time::run();`，没有任何 `mod slider;`，而 `slider/slider.rs` 照样被收进
`source_scope.tsv` 并生成 `#[path=…] pub mod slider;`、`REGISTRATION` 与 `__Preset`/`__Parts` 断言。
即"新增这个面需要动宿主入口"的部分只有那条**可追加**的计划条目（判据 ③ 允许的追加）。

**变异检查（证明"在树里"不是我写在文件里的字）**：把追加的那一行临时删掉（此时 `src/lib.rs` 与原始
文件逐字节相同，而 `src/slider/slider.rs` **仍在磁盘上**），重新构建后：

```
$ diff /tmp/scen-runs/S4-cg/lib.rs.orig src/lib.rs        # 无输出：回到原始
$ CARGO_TARGET_DIR=… cargo test --offline
running 1 test
test label::tests::the_label_face_is_still_there ... ok
test result: ok. 1 passed; 0 failed; …
$ cat $OUT/source_scope.tsv
# mode	auto
# selected	2
# node	source	module
3e1f685cf66d1a661f5d9cc0285e324f	button/button.rs	button
68c941df41a8044cda4826fef0edc37c	label/label.rs	label
$ grep -c slider $OUT/generated_lib.rs
0
```

也就是说：文件在磁盘上不等于在树里——`slider` 出现在 `source_scope.tsv` / `graft_plan.tsv` /
`generated_lib.rs` / `registrations()` 里，完全由那条计划条目决定；slider 的两条测试在变异态下**根本
没被编进树**（0 条匹配、只剩 label 的 1 passed）。恢复那一行后又是 3 passed、产物恢复成 §2 的三条
（`src/lib.rs` 的 sha256 恢复为 `f007c5…`、`slider.rs` 仍为 `01beeb…`）。

---

## 3. 判据 ②：同族形状一致

**据以判断"同族该有什么"的证据**（三条，都可复核）：

1. **两个同族面自己怎么写的**（`cg explore -p /tmp/scen-runs/S4-cg/tree "registration face root_object NODE_ID parent Button Label"` 逐字返回 `src/button/button.rs` 与 `src/label/label.rs`）：两文件除类型名外完全同形——
   `// generated-by=NichLink` 头 → 双语 `//!` 模块文档 → `use crate::{NoParts, NoPreset};` →
   `pub struct <Kind>;` 标记类型（带双语文档）→ `crate::root_object! { kind: <Kind>, parent: crate::root_node_id(env!("CARGO_PKG_NAME")) }`。
   `label.rs` 多一个内联 `#[cfg(test)] mod tests`（button.rs 没有测试），也就是说**测试不是形状的一部分**。
2. **生成宏自己声明的字段表**（`out/generated_lib.rs` 里 `root_object!` 的文档）：字段顺序为
   `kind` 先写，其后 `preset, parts, name, summary, exports, stable_name, needs_registry, parent,
   getting_from_other_registry, registry_rule_path, registry_rule, admission, handle_traits,
   handle_contracts, part_traits, part_contracts, requires, provides, flow, flow_provider, plugin,
   runtime_checks`——"**每一项都可省略**"。所以形状上的**必填集就是 `kind`**；同族实际用到的另一项是
   `parent`（两兄弟都写，且值相同）。
3. **一个面必须在产物里留下的痕迹**（改动前后两份清单对照）：`source_scope.tsv` 一行（`mode=auto`）、
   `pruning_manifest.tsv` 一行（`kind` 列 = 类型名）、`graft_plan.tsv` 一行 `cut`/`graft`、
   `generated_lib.rs` 里 `pub mod` + `REGISTRATION` 常量 + `assert_static_registration` +
   `__Preset`/`__Parts` 契约断言 + `BUILTIN_STATIC_FACES` 一条。

**结论**：`slider` 的声明集与同族**逐条对齐、一条不缺，也没有多写**：

| 同族该有的 | button | label | slider（本次新增） |
| --- | --- | --- | --- |
| `// generated-by=NichLink` 头 | ✓ | ✓ | ✓ |
| 双语模块文档 | ✓ | ✓ | ✓ |
| `use crate::{NoParts, NoPreset};` | ✓ | ✓ | ✓ |
| `pub struct <Kind>;` 标记类型 | `Button` | `Label` | `Slider` |
| `root_object!` 里 `kind:` | ✓ | ✓ | ✓ |
| `root_object!` 里 `parent: crate::root_node_id(env!("CARGO_PKG_NAME"))` | ✓ | ✓ | ✓ |
| `source_scope.tsv` 行（`mode=auto`） | ✓ | ✓ | ✓（`9174fc…	slider/slider.rs	slider`） |
| `pruning_manifest.tsv` 行（`kind` 列） | `Button` | `Label` | `Slider` |
| 计划 `cut`/`graft` 行 | ✓ | ✓ | ✓ |
| `REGISTRATION` + 契约断言 | ✓ | ✓ | ✓ |
| 内联测试 | ✗ | ✓ | ✓（新对象要求，见 ④） |

`name`/`summary`/`preset`/`parts` 等可选字段同族**一个都没写**，所以为了同族一致我也没有写——
"一条不缺"不等于"多写几条"。

---

## 4. 判据 ③：既有的面与既有计划条目**逐字未变**

**(a) 源文件逐字节比对**（改动前的哈希写进 `before-sha256.txt`，改动后重算，逐行 diff 只差 `src/lib.rs` 一行）：

```
$ find src .vscode build.rs Cargo.toml -type f | sort | xargs sha256sum   # 改动后
503f3b635623400249e12926d3996a57cb95fb62c16b23014920213650a9bba6  build.rs
3eb4378ce5697aa9c0139ce4f9b98db6c2cdc4b83bf02c25a66e5e8e1c42ded1  Cargo.toml
d2775e819a3a883339aefe65c0332f30fd367d864b61f9acf677c0f6307e7f06  src/button/button.rs
ed59c158a3150bfd6c1696d646657cade443e253dcc64a141afd85c6a73c57cf  src/label/label.rs
f007c58791d4882c09129aa19a8142e7b334b10b3f206427963581d340fc08d6  src/lib.rs
01beeb78d497878c4964a37dd34c6655b99ddbd23390b8a1386f5df07e18a19b  src/slider/slider.rs
fa946ed07e3dccadb98c060964730f69304a1ff7a206d6dcc2dcdd9a909eb1b1  .vscode/nichlink-face.code-snippets
```

`build.rs` / `Cargo.toml` / `button.rs` / `label.rs` / `.vscode/*` 的哈希与改动前**完全相同**
（`Cargo.lock` 也未被改写：`3284` 字节、mtime 仍是 `2026-10-03 23:11:38`，sha256
`a31a6c2b0364892913e9d8fb178878695502ef87d7205383d76eef34ceb0d29d`）。唯一变化的既有文件是
`src/lib.rs`，且 §1 的 diff 显示**只多了一行**。

**(b) 既有计划条目逐字未变**：`graft_plan.tsv` 改动前 2 行、改动后 3 行，前两行逐字相同：

```
# 改动前                          # 改动后（前两行一致，末行新增）
# cut	graft	full	line	column  # cut	graft	full	line	column
crate::button::NODE_ID  crate::button::NODE_ID  false  8  1   （同一行）
crate::label::NODE_ID   crate::label::NODE_ID   false  8  1   （同一行）
                                 crate::slider::NODE_ID  crate::slider::NODE_ID  false  8  1  ← 新增
```

**(c) 既有面的身份（NodeId）逐字未变**：`source_scope.tsv` / `pruning_manifest.tsv` 里 button、label
的行改动前后完全相同，NodeId 仍是 `3e1f685cf66d1a661f5d9cc0285e324f`（Button）、
`68c941df41a8044cda4826fef0edc37c`（Label）；`generated_lib.rs` 的 `BUILTIN_STATIC_FACES` 里前两条
`StaticFace::new(NodeId::from_raw([62, 31, 104, 92, …])` 与 `([104, 201, 65, 223, …])` 的字节数组
改动前后逐字节相同（见 §2 引文与改动前 dump）。

**(d) 既有面的公开路径逐字未变**：`generated_lib.rs` 里
`#[path = "/tmp/scen-runs/S4-cg/tree/src/button/button.rs"] pub mod button;` 与 label 的对应两行
改动前后相同；`assert_static_registration(…, button::REGISTRATION)`、
`assert_contract::<crate::button::__Preset, crate::button::__Parts>()` 等既有行未动。既有测试
`label::tests::the_label_face_is_still_there` 仍然跑且通过（§5）。

**(e) 宿主入口只做了追加**：唯一被改的既有文件 `src/lib.rs` 的改动是"在 `labels` 行之后、闭合 `);`
之前插入一行"，没有删除、没有改写、没有重排任何既有行；`build.rs`、`Cargo.toml` 一字未动。

---

## 5. 判据 ④：`cargo test --offline` 绿 + 为新对象加测试

命令与完整输出（`cargo test --offline`，日志另存 `/tmp/scen-runs/S4-cg/cargo-test.log`）：

```
$ cd /tmp/scen-runs/S4-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline
   Compiling control v0.1.0 (/tmp/scen-runs/S4-cg/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.62s
     Running unittests src/lib.rs (/tmp/scen-runs/S4-cg/cargo-target/debug/deps/control-9407f06accd8f323)

running 3 tests
test label::tests::the_label_face_is_still_there ... ok
test slider::tests::the_slider_face_is_still_there ... ok
test slider::tests::the_slider_parent_is_the_host_root ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

新对象的两条测试（`src/slider/slider.rs` 内联 `#[cfg(test)] mod tests`，与 `label.rs` 的挂法同族）：

```rust
#[test]
fn the_slider_face_is_still_there() {
    let registrations = crate::registrations();
    assert!(
        registrations.iter().any(|registration| registration.id == crate::slider::NODE_ID),
        "slider must appear in the generated registration tree"
    );
    let _ = crate::slider::Slider;
}

#[test]
fn the_slider_parent_is_the_host_root() {
    let registrations = crate::registrations();
    let slider = registrations.iter()
        .find(|registration| registration.id == crate::slider::NODE_ID)
        .expect("slider must appear in the generated registration tree");
    assert_eq!(slider.parent, crate::root_node_id(env!("CARGO_PKG_NAME")));
}
```

这两条测试**只会因为"面真的进了树"而通过**：`registrations()` 是构建脚本产出的内容，`slider::NODE_ID`
与 `slider::REGISTRATION` 来自宏展开与生成的计划。§2 的变异检查已经证明：删掉计划里那一行后，slider
的测试连编译都不进（`cargo test --offline` 只剩 1 passed），恢复后才 3 passed。

---

## 6. 没做到 / 局限 / 摩擦（如实记录）

- **codegraph 调用共 10 次**（`calls.log` 里 10 条 `=== CMD`）：`--help`、`init`、`files`、
  `explore --help`、`explore`×2、`query Slider`、`sync -p`(**被拒**)、`files`(重复)、`sync <tree>`。
  其中：
  - **1 次被拒**：`cg sync -p /tmp/scen-runs/S4-cg/tree` → `error: unknown option '-p'`（`sync` 只收
    位置参数 `[path]`，`explore`/`query`/`files` 才收 `-p`）。改成 `cg sync <tree>` 后成功。
  - **1 次重复预览**：`files` 跑了两次；第二次是为"同步后再看一眼索引"，但当时还没 sync，返回的是
    同一份旧列表（4 files）。正确的顺序应是先 `sync` 再 `files`。
  - **1 次空结果（因索引滞后浪费）**：改动后立刻 `cg query -p … "Slider"` 返回
    `No results found`（索引未跟踪新文件）；`cg sync` 后才补上（`Added: 1, Modified: 1`）。
    也就是说 codegraph 侧要看到新文件，必须显式 `sync`，`init` 之后没有自动跟随新文件。
- **未做**：没有把 codegraph 索引同步后的 `files` 再打一遍（避免第二次重复预览），因此"索引里能看到
  `slider`"只有 `cg sync` 的 `Added: 1, Modified: 1` 作为证据；要自己看列表的人跑一次
  `cg files -p /tmp/scen-runs/S4-cg/tree` 即可。
- **未读禁读区**：`/home/nich/Moirai_N3/nichlink` 下我只执行了 wrapper
  `bench/bin/cg`；没有读该目录下任何其它路径，没有把 `-p` 指向检出，`cg` 每次查询
  都带 `-p /tmp/scen-runs/S4-cg/tree`。唯一"碰到"检出的是 cargo 按树自己的 `Cargo.toml`
  `path` 依赖编译 `toolchain/`——树本身就是这个宿主，这是构建，不是我在查资料。
- **一处判断的自由度**：同族要不要写 `name`/`summary`？生成宏文档说"每一项都可省略"，两兄弟也一个
  都没写，因此我按"同族一致 = 与兄弟逐条对齐"处理，没有新增可选声明（见 §3 结论表）。若评审
  期望新面额外带 `name`/`summary`，这属于"与既有形状不同"，需要显式规则而不是从同族推断。

---

## 7. 我亲手动过的命令（按时间顺序，除 codegraph 外的 shell/cargo）

```sh
export BENCH_CALLS=/tmp/scen-runs/S4-cg/calls.log
ls -la /tmp/scen-runs/S4-cg/ ; ls -la /tmp/scen-runs/S4-cg/tree
cd /tmp/scen-runs/S4-cg/tree && find . -type f -not -path './.git/*' ; cat Cargo.toml build.rs
# 读 src/lib.rs src/button/button.rs src/label/label.rs .vscode/nichlink-face.code-snippets
# codegraph（全部经 wrapper /home/nich/Moirai_N3/nichlink/bench/bin/cg）：
#   cg --help / cg init <tree> / cg files -p <tree> / cg explore --help
#   cg explore -p <tree> "registration face root_object NODE_ID parent Button Label"
#   cg explore -p <tree> "static_graft_plan host macro lib.rs build.rs FRAMEWORK cut graft"
#   cg query -p <tree> "Slider" / cg sync -p <tree>(被拒) / cg files -p <tree> / cg sync <tree>
cd /tmp/scen-runs/S4-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline   # 基线：1 passed
find /tmp/scen-runs/S4-cg/cargo-target/debug/build -maxdepth 3 -name out -type d    # 找到 OUT_DIR
cat $OUT/{graft_plan.tsv,function_manifest.tsv,pruning_manifest.tsv,source_scope.tsv,discovery.fingerprint,generated_lib.rs}
cp src/lib.rs /tmp/scen-runs/S4-cg/lib.rs.orig
find src .vscode build.rs Cargo.toml -type f | sort | xargs sha256sum > /tmp/scen-runs/S4-cg/before-sha256.txt
mkdir -p src/slider && cat > src/slider/slider.rs <<'...'     # 新增面（形状镜像 label/button）
python3 - <<'PY' ... PY                                        # 在 lib.rs 计划里追加一行
diff -u /tmp/scen-runs/S4-cg/lib.rs.orig src/lib.rs
CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline    # 2 passed（slider 被自动发现）
# 用编译器探针确认 RegistrationInfo 有 `id`/`parent` 字段、NodeId 可比较（一次 probe 编译）
CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline    # 3 passed（最终）
find src .vscode build.rs Cargo.toml -type f | sort | xargs sha256sum      # 与 before 对比
ls -l --time-style=full-iso Cargo.lock ; sha256sum Cargo.lock
grep -H slider $OUT/{source_scope.tsv,graft_plan.tsv,pruning_manifest.tsv,generated_lib.rs}
sed -n '/BUILTIN_STATIC_FACES/,/^];/p' $OUT/generated_lib.rs ; sed -n '/pub fn registrations/,/^}/p' $OUT/generated_lib.rs
# 变异检查（临时删掉计划行 → 构建 → 观察测试与产物 → 恢复）
cp src/lib.rs /tmp/scen-runs/S4-cg/lib.rs.final
python3 - <<'PY2' ... PY2            # 删掉追加的那一行
diff /tmp/scen-runs/S4-cg/lib.rs.orig src/lib.rs
CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline           # 1 passed
CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline -- slider  # 0 tests, 1 filtered out
cat $OUT/source_scope.tsv ; grep -c slider $OUT/generated_lib.rs                   # 2 selected / 0
cp /tmp/scen-runs/S4-cg/lib.rs.final src/lib.rs
CARGO_TARGET_DIR=/tmp/scen-runs/S4-cg/cargo-target cargo test --offline           # 3 passed（最终）
find src .vscode build.rs Cargo.toml -type f | sort | xargs sha256sum             # 与 §4(a) 一致
grep -c '^=== CMD' /tmp/scen-runs/S4-cg/calls.log
```
