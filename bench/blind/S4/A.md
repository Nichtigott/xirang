# S4 · 新增一个控件 `slider`（我们臂）

- 宿主树：`/tmp/scen-runs/S4-ours/tree`（包 `control`，已有 `button`、`label` 两个面）
- 工具：`/home/nich/Moirai_N3/nichlink/bench/bin/nl`（`BENCH_CALLS=/tmp/scen-runs/S4-ours/calls.log`）
- 全部 cargo 命令带 `CARGO_TARGET_DIR=/tmp/scen-runs/S4-ours/cargo-target`

## 0. 我亲手动过的命令（全部）

| # | 命令 | 结果 |
| --- | --- | --- |
| 1 | `nl --list` / `nl --list apply` / `nl --list check` | 工具表与两份工具说明 |
| 2 | `nl --call registry --root …/tree` | 改动前：`faces 2` |
| 3 | `nl --call consistency --root …/tree --parent root` / `--by kind` | 同族证据（见 §2） |
| 4 | `nl --call explain --root …/tree --node root/button` / `root/label` | 两个既有面的声明字段 |
| 5 | `nl --call apply --root …/tree --action add --parent root --fields '{"module":"slider"}'` | 预览：会写 `src/slider/slider.rs`，且**预先警告**"this face's name appears at none of the plan sites" |
| 6 | `nl --call apply … --apply --confirm` | 落盘 `src/slider/slider.rs` |
| 7 | `nl --call consistency --root …/tree --parent root --by kind` | 改动后：3 member, outliers 0 of 0 |
| 8 | `nl --call registry --root …/tree` | 改动后：`faces 3` |
| 9 | `nl --call check --root …/tree --target_dir …/cargo-target` | `verdict  passed (cargo exit 0)`，2 passed |
| 10 | `CARGO_TARGET_DIR=… cargo test --offline`（手跑，多次） | 全绿 |
| 11 | `sha256sum -c baseline/hashes.txt`、`diff -r baseline/tree tree` | 见 §3 |

`nl` 调用（wrapper 调用）合计 **16 次**（`--list` 3 次、`--call` 13 次；其中 1 次是我自己的错误——
`nl --call vs`，工具不存在，白花一次）。

## 1. 判据① 新面确实在（可观察终态）

### 改动前 vs 改动后的树

```
$ nl --call registry --root /tmp/scen-runs/S4-ours/tree      # 改动前
faces 2
root/button  Button  button/button.rs  3e1f685cf66d1a661f5d9cc0285e324f
root/label   Label   label/label.rs    68c941df41a8044cda4826fef0edc37c

$ nl --call registry --root /tmp/scen-runs/S4-ours/tree      # 改动后
faces 3
root/button  Button  button/button.rs  3e1f685cf66d1a661f5d9cc0285e324f
root/label   Label   label/label.rs    68c941df41a8044cda4826fef0edc37c
root/slider  Slider  slider/slider.rs  9174fc9f4a0848306a693b2ae2f4a9fd
```

### 别人能用一条命令复核（从零重新生成，不看缓存）

```sh
cd /tmp/scen-runs/S4-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S4-ours/cargo-target cargo test --offline
# 若想真的从零：先 rm -rf /tmp/scen-runs/S4-ours/cargo-target/debug/build/control-* \
#                           /tmp/scen-runs/S4-ours/cargo-target/debug/.fingerprint/control-*
# 然后看构建期产物（rustc 真正编译的那份）：
G=$(find /tmp/scen-runs/S4-ours/cargo-target/debug/build -name generated_lib.rs | head -1)
grep -n 'pub mod slider;\|slider::REGISTRATION' "$G"
```

输出（本次实测，控制产物已先删除、重新构建）：

```
$ cargo test --offline    →  2 passed; 0 failed   （改动前是 1 passed）
$ grep -n 'pub mod slider;\|slider::REGISTRATION' "$G"
47:pub mod slider;
55:    StaticFace::new(NodeId::from_raw([145, 116, 252, 159, 74, 8, 72, 48, 106, 105, 59, 42, 226, 244, 169, 253]), …, false),
62:const _ = assert_static_registration(RegistrationRule::ANY, slider::REGISTRATION);
90:        slider::REGISTRATION,
```

### 构建期剪枝清单（面在 rustc 展开前是否存活）

```
$ cat "$(dirname $G)/source_scope.tsv"
# mode	auto
# selected	3
# node	source	module
3e1f685cf66d1a661f5d9cc0285e324f	button/button.rs	button
68c941df41a8044cda4826fef0edc37c	label/label.rs	label
9174fc9f4a0848306a693b2ae2f4a9fd	slider/slider.rs	slider
```

**必要说明（一个真实坑）**：只跑 `apply {action:"add"}` 是不够的。文件创建后我跑 `cargo test`
只看到 `1 passed`（只有 label 的测试），且 `source_scope.tsv` 是 `# selected 2`——`slider` 在
**派生树**里（`registry` 报 3 个面），却没进**构建期源码作用域**，于是 `generated_lib.rs` 里没有
`pub mod slider;`，`slider` 模块根本没被编译。原因是这个树的作用域是「从宿主入口
（`src/lib.rs`）出发可达」，而现有两个面各自是被入口里的 graft 计划条目选中的——`apply` 的回复
其实**预先**说了这件事（`entry plan: 2 cut( and 2 graft( site(s); this face's name appears at
none of them`）。补上那一行后 `cargo test` 才变 `2 passed`、`source_scope.tsv` 才变 `# selected 3`。

## 2. 判据② 与同族形状一致

**据以判断"同族该有什么"的证据**（都是只读调用）：

1. `consistency --parent root`（默认 `by: api`）说 `family root · member control · 2 member(s)`，
   但两个成员的调用都是 `0 call(s): none`——工具自带说明：成员整段体是一个宏调用时 `api`
   没有可比内容，应改用 `by: kind` / `by: source` / `specimen`。
2. `consistency --parent root --by kind`：`button Button` / `label Label`，`outliers: 0 of 0`
   ——同族的 kind 只在拼写上不同，形状一致。
3. `registry` / `explain root/button`、`explain root/label`：两者同深度、同父
   （`parent 47cc4542…`）、都是 `source <自名>/<自名>.rs`、`module <自名>`——新增面就该是
   `root/slider`、`module slider`、`slider/slider.rs`。
4. 源码逐字对照（两个既有面已冻结、未改）：两份文件都是同一套四件套——
   `// generated-by=NichLink` + 双语 `//!` 面说明 → `use crate::{NoParts, NoPreset};` →
   `pub struct <Kind>;`（双语 doc）→ `crate::root_object! { kind: <Kind>, parent: crate::root_node_id(env!("CARGO_PKG_NAME")) }`；
   外加宿主入口里**每个面一条** `cut(crate::<m>::NODE_ID) graft(crate::<m>::NODE_ID)`。

**结论**：`slider` 四件套齐全，且第 5 件（入口计划条目）也补齐——

- 文件体与同族逐字同形（唯一差别是 `Slider`/`slider`），由 `apply` 生成（`declaration src/slider/slider.rs:11`）；
- 入口计划追加同形的一行；
- `consistency --parent root --by kind` 改动后：`3 member(s)` / `outliers: 0 of 0`。

## 3. 判据③ 既有的面一个字都没被改动

### 冻结基线

改动前我对整棵树做了备份（`baseline/tree`）与逐文件 sha256（`baseline/hashes.txt`），
并在追加前后各存了 `src/lib.rs`。

```
$ cd /tmp/scen-runs/S4-ours/tree && sha256sum -c /tmp/scen-runs/S4-ours/baseline/hashes.txt
Cargo.toml: OK
build.rs: OK
src/lib.rs: FAILED          ← 唯一一处，且是追加（下面给出逐行 diff）
src/button/button.rs: OK
src/label/label.rs: OK
sha256sum: WARNING: 1 computed checksum did NOT match

$ diff -r baseline/tree tree --exclude=cargo-target
11a12
>     cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID),
Only in tree/src: slider

$ diff -u baseline/lib.rs.before-append tree/src/lib.rs
@@ -9,4 +9,5 @@
     FRAMEWORK,
     cut(crate::button::NODE_ID) graft(crate::button::NODE_ID),
     cut(crate::label::NODE_ID) graft(crate::label::NODE_ID),
+    cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID),
 );
```

`src/lib.rs` 前 11 行（含两行既有计划条目）与基线**逐字节相同**（`diff <(head -11 基线) <(head -11 现)`
无输出，脚本打印 `lines 1-11 byte-identical`）；没有删除、没有重排、没有任何既有行被改写。

### 既有面的声明/身份/公开路径逐字未变

- 声明：`button.rs` / `label.rs` 的 sha256 与基线相同（上）；`kind: Button` / `kind: Label`、
  `pub struct Button` / `pub struct Label`、`crate::button::Button` / `crate::label::Label`
  所在文件未动，故逐字节未变。
- 身份：`registry` 报 `3e1f685cf66d1a661f5d9cc0285e324f`（button）、
  `68c941df41a8044cda4826fef0edc37c`（label），与改动前一致；`explain` 两次输出也一致。
  生成物里两者的 `NodeId::from_raw` 字节数组（`[62,31,104,…]` / `[104,201,65,…]`）出现次数各为 1，
  且与改动前的生成物同串。
- 既有的计划条目：`graft_plan.tsv` 逐行是

  ```
  crate::button::NODE_ID	crate::button::NODE_ID	false	8	1
  crate::label::NODE_ID	crate::label::NODE_ID	false	8	1
  crate::slider::NODE_ID	crate::slider::NODE_ID	false	8	1
  ```

  前两行与基线（改动前的 `graft_plan.tsv`）逐字节相同，第三行是新增。

## 4. 判据④ `cargo test --offline` 绿 + 新对象的测试

```
$ cd /tmp/scen-runs/S4-ours/tree
$ CARGO_TARGET_DIR=/tmp/scen-runs/S4-ours/cargo-target cargo test --offline
running 2 tests
test label::tests::the_label_face_is_still_there ... ok
test slider::tests::the_slider_face_is_still_there ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
Doc-tests 0 passed; 0 failed
cargo exit=0
```

新测试（追加在 `src/slider/slider.rs` 末尾，与 label 同形）：

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn the_slider_face_is_still_there() {
        let _ = crate::slider::Slider;
    }
}
```

`nl --call check --root …/tree --target_dir …/cargo-target` 的判定行：`verdict  passed (cargo exit 0)`。

## 5. 改动清单

| 文件 | 动作 |
| --- | --- |
| `/tmp/scen-runs/S4-ours/tree/src/slider/slider.rs` | **新增**：`apply` 生成的注册面（14 行，与 button/label 同形）+ 我追加的 1 条测试（7 行） |
| `/tmp/scen-runs/S4-ours/tree/src/lib.rs` | **追加 1 行**：`cut(crate::slider::NODE_ID) graft(crate::slider::NODE_ID),`（两行既有条目与其余全部行逐字节未变） |

其余文件（`Cargo.toml`、`build.rs`、`src/button/button.rs`、`src/label/label.rs`、`.vscode/…`）零改动。

## 6. 没做到 / 需要说清楚的

- 判据③ 严格读："`src/lib.rs` 这个文件整份没变"做不到——作用域是入口可达性驱动的，新面必须被
  入口选中才会被编译。我做的是允许的那条：**只追加一行**（在既有两行之后、闭合 `);` 之前），
  既有行逐字节未变。
- 一次浪费的调用：`nl --call vs`（我误以为有 `vs` 工具），返回 `unknown tool nichlink.vs`，exit 1。
  另有一次弯路已计入正文：先跑了一次 `cargo test` 发现只有 `1 passed`，才去追为什么。
- `apply` 的预览与实际落盘的 `slider.rs` 完全一致（预览已列出完整 diff，落盘文件前 14 行与之相同）；
  `apply` **未**替我动入口计划（它的回复明说了这一点），那一行是我按同族形状追加的。
- 本答案中的"构建期产物"路径是 `cargo-target/debug/build/control-*/out/`；`tree/target/nichlink/out/`
  自始至终不存在（`registry`/`explain` 每次都会念这句），这不影响上面的证据。
