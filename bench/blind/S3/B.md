# S3 · button 部件宽度换算症状 —— 根因、最小修、反证（对照臂 s3-cg）

树的路径：`/tmp/scen-runs/S3-cg/tree`（crate `control`）。所有 codegraph 调用都经 wrapper
`/home/nich/Moirai_N3/nichlink/bench/bin/cg`，并且每次查询都显式带
`-p /tmp/scen-runs/S3-cg/tree`（从未指向检出）。`BENCH_CALLS=/tmp/scen-runs/S3-cg/calls.log`。

`cargo` 调用全部带 `CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target`。

---

## ① 根因

**`/tmp/scen-runs/S3-cg/tree/src/button/button.rs:19`**

```rust
18  pub fn width(amount: i64, floor: i64) -> i64 {
19      if amount > floor {      // ← 根因：比较方向反了
20          floor
21      } else {
22          amount
23      }
24  }
```

分支结构写成了「大于下限时返回下限」，正好把守卫的两个分支对调：`amount < floor`（低于下限）
时走 `else` 原样返回 `amount`，于是**低于下限的宽度没有被抬到下限**。函数第 16–17 行的文档
契约是“低于下限的宽度一律抬到下限 / anything below the floor is raised to it”，实现与契约相反。

复现（改前同一条命令）：

```
thread 'button::tests::a_width_below_the_floor_is_raised_to_it' panicked at
/tmp/scen-runs/S3-cg/tree/src/button/button.rs:32:9:
assertion `left == right` failed
  left: 1
 right: 2
```

`left: 1 / right: 2` 正是 `width(1, 2)` 返回了入参而没返回下限的特征，与「分支对调」的判断吻合，
而不是参数写反（那会是别的数对）。

## ② 最小修

只改**一个 token**，把第 19 行的比较方向翻回来：

```diff
-    if amount > floor {
+    if amount < floor {
```

修后（当前磁盘内容）：

```rust
18  pub fn width(amount: i64, floor: i64) -> i64 {
19      if amount < floor {
20          floor
21      } else {
22          amount
23      }
24  }
```

只动这一行就够，所以没有再引入 `amount.max(floor)` 之类等价重写：那不是“必要的那处”之外的东西，
且会把 diff 从 1 个字符放大。边界仍正确：`amount == floor` 走 `else` 返回 `amount`，等于下限本身；
`amount > floor` 原样返回；只有 `amount < floor` 被抬到 `floor`。

**反证锚点**：改之前 `width(1, 2) == 1` 使测试红（上文 panic），改之后 `width(1, 2) == 2` 使测试绿
（下文 ③）——同一棵树、同一条命令、唯一差异就是这一个 token。

## ③ 反证：命令与输出

### 反证 1（任务书要求的主反证命令）

```
$ cd /tmp/scen-runs/S3-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline
   Compiling control v0.1.0 (/tmp/scen-runs/S3-cg/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.60s
     Running unittests src/lib.rs (/tmp/scen-runs/S3-cg/cargo-target/debug/deps/control-9407f06accd8f323)

running 1 test
test button::tests::a_width_below_the_floor_is_raised_to_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

（`0 passed` 的那一段是 doc-test，不是「什么都没跑」：真正的门是上面 `1 passed; 0 failed`。）

### 反证 2（独立于失败测试的边界探针，期望值只从文档契约推导）

为了不让“反证”退化成“把测试改绿”，临时加了一个只读公开 API、独立于 crate 内测试的探针
`tree/tests/width_boundary.rs`：

```rust
#[test]
fn width_boundary_probe() {
    assert_eq!(control::button::width(1, 2), 2);   // 低于下限 -> 抬到下限
    assert_eq!(control::button::width(0, 2), 2);   // 低于下限 -> 抬到下限
    assert_eq!(control::button::width(2, 2), 2);   // 恰在下限 -> 不变
    assert_eq!(control::button::width(5, 2), 5);   // 高于下限 -> 不变
}
```

```
$ CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline --test width_boundary
     Running tests/width_boundary.rs (/tmp/scen-runs/S3-cg/cargo-target/debug/deps/width_boundary-f73fcc58c163385f)

running 1 test
test width_boundary_probe ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

探针跑完后即删除（`rm -rf tree/tests`），当前树回到「只差一个 token」的最小 diff，并再次跑完
整 `cargo test --offline` 仍为 `1 passed; 0 failed`。

### 反证 3（另一次完整绿灯，证明删除探针后的最终状态）

```
$ CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline
running 1 test
test button::tests::a_width_below_the_floor_is_raised_to_it ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## ④ 排除掉的嫌疑点及理由

| # | 嫌疑点 | 排除理由（证据） |
| - | - | - |
| 1 | **调用者把 `(amount, floor)` 传反**，函数本身没错 | `cg callers -p /tmp/scen-runs/S3-cg/tree width` → `Callers of "width" (0)`：全树没有任何调用者（crate 内只有测试调用它）。测试 `width(1, 2)` / `width(5, 2)` 的实参与形参顺序与文档契约一致，不存在交换。 |
| 2 | **测试写错**（期望值反了） | 文档注释（`button.rs:16-17`）“低于下限的宽度一律抬到下限 / anything below the floor is raised to it”给出的期望与测试一致：`width(1,2)==2`。实现与文档矛盾，是实现在错。 |
| 3 | **同一棵树里还有第二份 width/floor 换算**（例如 `build.rs` 生成的 `OUT_DIR/generated_lib.rs` 注入的 wrapper） | `cg query -p … width` 与 `cg query -p … floor` 全树各自只命中 `src/button/button.rs:18`（外加那条测试名）。生成的 `generated_lib.rs` 只做 `#[path]` 挂载与注册面/静态计划，函数体为零；其中 `grep -rn "fn width" <cargo-target>/**/out/` 无命中。 |
| 4 | **另一个部件 label 引起** | `src/label/label.rs` 只有一个注册面标记结构体 `Label`，没有宽度/下限逻辑；`width`/`floor` 两个符号查询都没有命中 label 文件。测试失败点也在 `button::tests`。 |
| 5 | **下限来自某个错常量/全局配置**（floor 被算小） | `floor` 是 `width` 的入参，函数体内没有常量、没有 `static`、没有环境读取；`amount`/`floor` 两个操作数都在参数位，错的是二者之间的比较方向，不是取值来源。 |
| 6 | **注册面/graft 机制（build.rs 生成、`root_object!`、`static_graft_plan!`）引起** | 失败断言是纯函数调用的返回值（`1` vs `2`），注册与 graft 只影响 `NODE_ID`/静态计划，不参与 `width` 的求值；改注册面或生成代码都不会改变这次断言。 |
| 7 | **`build.rs` / `Cargo.toml` 配置错**（构建脚本、依赖路径） | 基线失败是一次正常的**断言失败**而不是编译/生成失败：`cargo test --offline` 编译通过、测试被发现并运行、只在断言处 panic。配置类问题不会产生 `left: 1 right: 2`。 |

## ⑤ 我亲手动过的命令（可复算）

```
export BENCH_CALLS=/tmp/scen-runs/S3-cg/calls.log
find /tmp/scen-runs/S3-cg/tree -type f | sort ; cat Cargo.toml / src/*.rs / build.rs
/home/nich/Moirai_N3/nichlink/bench/bin/cg --help
/home/nich/Moirai_N3/nichlink/bench/bin/cg init --help
/home/nich/Moirai_N3/nichlink/bench/bin/cg explore --help
/home/nich/Moirai_N3/nichlink/bench/bin/cg init -y /tmp/scen-runs/S3-cg/tree
/home/nich/Moirai_N3/nichlink/bench/bin/cg explore -p /tmp/scen-runs/S3-cg/tree "button width floor"
/home/nich/Moirai_N3/nichlink/bench/bin/cg query --help
/home/nich/Moirai_N3/nichlink/bench/bin/cg callers --help
/home/nich/Moirai_N3/nichlink/bench/bin/cg files --help
/home/nich/Moirai_N3/nichlink/bench/bin/cg query -p /tmp/scen-runs/S3-cg/tree width
/home/nich/Moirai_N3/nichlink/bench/bin/cg query -p /tmp/scen-runs/S3-cg/tree floor
/home/nich/Moirai_N3/nichlink/bench/bin/cg callers -p /tmp/scen-runs/S3-cg/tree width
cd /tmp/scen-runs/S3-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline          # 改前：红
（改 button.rs:19 的 > 为 <）
cd /tmp/scen-runs/S3-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline          # 改后：绿
cd /tmp/scen-runs/S3-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline --test width_boundary   # 边界探针：绿
rm -rf /tmp/scen-runs/S3-cg/tree/tests
cd /tmp/scen-runs/S3-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-cg/cargo-target cargo test --offline          # 清理后：绿
```

## ⑥ 工具调用计数（本臂数据）

- **codegraph（wrapper `cg`）调用总数：11 次**（`/tmp/scen-runs/S3-cg/calls.log` 里 `=== CMD: cg` 计数）：
  - 6 次 `--help`（`cg`、`cg init`、`cg explore`、`cg query`、`cg callers`、`cg files`）——纯摸索 CLI 表面；
  - 1 次 `cg init -y <自己树>`（建索引）；
  - 4 次实质查询：`explore "button width floor"`、`query width`、`query floor`、`callers width`。
    **其中定位根因真正必需的是 1 次**（`explore` 一次性给出了 `width` 的逐行源码），另外 3 次用于排除嫌疑点 1/3/4。
- 全程没有把 `-p/--path` 指向检出；没有调用不带 `-p` 的 `explore`（避免回落到外层索引）。
- 一次实质查询就命中根因，说明“部件有症状”这类题在 codegraph 上的首查就足够；11 次里有 7 次是
  帮助/初始化开销。

**结论**：根因是 `src/button/button.rs:19` 的比较方向写反（`amount > floor`），最小修是把该行改成
`amount < floor`；改后 `cargo test --offline` 由 `0 passed; 1 failed` 变为 `1 passed; 0 failed`，
独立边界探针同样全绿。本臂共用 codegraph 11 次调用。
