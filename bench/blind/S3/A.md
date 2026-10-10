# S3 · button 宽度换算有症状 —— 根因、最小修、反证

运行目录 `/tmp/scen-runs/S3-ours`；工具通道 wrapper `…/bench/bin/nl`
（`BENCH_CALLS=/tmp/scen-runs/S3-ours/calls.log`）；所有 cargo 均带
`CARGO_TARGET_DIR=/tmp/scen-runs/S3-ours/cargo-target`。
本文件提到的文件只有任务书允许的 `/tmp/scen-runs/S3-ours/**` 与 wrapper 本身。

---

## 0. 症状（未改动时的第一手证据）

```
$ cd /tmp/scen-runs/S3-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-ours/cargo-target cargo test --offline
running 1 test
test button::tests::a_width_below_the_floor_is_raised_to_it ... FAILED

---- button::tests::a_width_below_the_floor_is_raised_to_it stdout ----
thread '…' panicked at /tmp/scen-runs/S3-ours/tree/src/button/button.rs:32:9:
assertion `left == right` failed
  left: 1
 right: 2
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

失败断言的原话就是"低于下限要抬到下限"，`width(1, 2)` 期望 `2`，实际拿到 `1`。

---

## 1. 根因（`文件:行`）

**`/tmp/scen-runs/S3-ours/tree/src/button/button.rs:19` —— 分支条件写反了。**

```rust
16 | /// 屏幕宽度换算：低于下限的宽度一律抬到下限。
17 | /// Width for a control: anything below the floor is raised to it.
18 | pub fn width(amount: i64, floor: i64) -> i64 {
19 |     if amount > floor {     // ← 根因：应为 amount < floor
20 |         floor
21 |     } else {
22 |         amount
23 |     }
24 | }
```

条件 `amount > floor` 把两个分支的适用面整体对调了：
- 当 `amount > floor`（已经在限上）时返回 `floor` —— 把高于限的宽度**压**到限；
- 当 `amount <= floor`（含低于限）时返回 `amount` —— 低于限的宽度**没被抬起**。

于是 `width(1, 2)` 走 `else` 返回 `1`，第 32 行断言红。这正是"低于下限的宽度没有被抬到下限"的症状本身。

三条独立线索都指向同一行，且互相不依赖：
1. 失败断言行 `button.rs:32`（`assert_eq!(width(1, 2), 2)`）；
2. 契约行 `button.rs:16-17`（doc 明说"低于下限的宽度一律抬到下限 / anything below the floor is raised to it"）；
3. MCP `read --whole` 打印的符号索引：`18: width lines 18-24`，函数体只有 19-23 这一处判断。

**契约与测试一致，代码是唯一的异类** —— 所以缺陷在实现，不在规格。

## 2. 修法（最小，只改一处）

`src/button/button.rs:19`，一个字符：`>` → `<`。

```diff
 pub fn width(amount: i64, floor: i64) -> i64 {
-    if amount > floor {
+    if amount < floor {
         floor
     } else {
         amount
```

修完语义恰为 `max(amount, floor)` 的一个正确分支写法：`amount < floor` 时抬到 `floor`，否则保持 `amount`（`amount == floor` 落 `else`，返回同值的 `amount`，两种写法结果相同）。
两个分支的**返回值**本来就是对的（`floor` 给"低于限"、`amount` 给"不低于限"），错的只是条件方向，所以只需改条件这一个 token。

改动范围：整棵树只动这一个文件、这一行；没有增删文件（`find` 列出的仍是原来那 7 个文件）。

### 2.1 被考虑并否决的两种"也绿"的改法

| 备选改法 | 为何不选 |
| --- | --- |
| 交换两个分支体（`if amount > floor { amount } else { floor }`） | 结果同样正确，但它改**两行**；`>`→`<` 只改一个字符，更小，且让条件读起来就是 doc 的原话（"低于下限 ⇒ 抬到下限"）。 |
| 改成 `amount.max(floor)` 一行 | 正确，但把"条件 + 分支"三行结构重写成一行，动的是三行而非一处；任务要求只改必要的那处。 |

## 3. 反证（"改完真的好了"）

反证分三段：A 只翻转那一个字符就复红（因果闭合），B 全量不变量探针贴在**改后的真实函数文本**上（不是复述逻辑），C 收尾全套测试绿。

### A. 只把第 19 行翻回 `>` —— 同一条测试立刻复红

```
$ grep -n "if amount" tree/src/button/button.rs
19:    if amount > floor {                       # 临时退回病态写法
$ CARGO_TARGET_DIR=…/cargo-target cargo test --offline --lib -- --exact button::tests::a_width_below_the_floor_is_raised_to_it
running 1 test
test button::tests::a_width_below_the_floor_is_raised_to_it ... FAILED
 right: 2
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

$ sed -i 's/if amount > floor {/if amount < floor {/' tree/src/button/button.rs
$ grep -n "if amount" tree/src/button/button.rs
19:    if amount < floor {
$ cmp tree/src/button/button.rs proof/button.rs.fixed && echo "byte-identical to the fixed file"
byte-identical to the fixed file
```

改动的**唯一**字节就是第 19 行这个比较符：翻转它是红的，翻回来是绿的（且 `cmp` 证明复位后与修复版逐字节相同）。这就把"绿"绑死在这一行上，而不是绑在缓存、别的编辑或测试被跳过上。

### B. 不变量探针：把改后文件的第 18-24 行**原样**抠出来，对全部 121 组 `(amount, floor) ∈ [-5,5]²` 与 `amount.max(floor)` 对拍

探针输入就是被修的真实文本，逐字从树里取：

```
$ sed -n '18,24p' tree/src/button/button.rs
pub fn width(amount: i64, floor: i64) -> i64 {
    if amount < floor {
        floor
    } else {
        amount
    }
}
$ rustc -o proof/probe_after proof/probe_after.rs && proof/probe_after
checked 121 pairs; violations = 0
```

同一段文本、只把 `>` 还原（`sed 's/if amount < floor {/if amount > floor {/'`）后编译：

```
$ rustc -o proof/probe_before proof/probe_before.rs && proof/probe_before
VIOLATION width(-5, -4) = -5, want -4
VIOLATION width(-5, -3) = -5, want -3
VIOLATION width(-5, -2) = -5, want -2
VIOLATION width(-5, -1) = -5, want -1
VIOLATION width(-5, 0) = -5, want 0
checked 121 pairs; violations = 110
```

探针**不是空真**：同一台探针在病态文本上抓到 110 处违反（`amount == floor` 的 11 组两种写法同值，故不是 121），在修复文本上 0 处。它覆盖了 `amount == floor` 的边界，也覆盖了负值。

### C. 收尾：任务书要求的 `cargo test --offline` 全绿，并走桥复核

```
$ cd /tmp/scen-runs/S3-ours/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S3-ours/cargo-target cargo test --offline
running 1 test
test button::tests::a_width_below_the_floor_is_raised_to_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

（读法按本仓的规矩：第二段 `0 passed` 是 doc-test 目标**什么都没跑**，不是"通过"的证据；证据是 lib 目标那行 `1 passed; 0 failed`，且上一段 A 里同一条命令能复红，说明它真的在跑。）

桥侧复核（`check` 自己跑 cargo test，判定在回复第一行）：

```
$ nl --call check --root /tmp/scen-runs/S3-ours/tree --face default --target_dir /tmp/scen-runs/S3-ours/cargo-target
verdict  passed (cargo exit 0)
check  cargo test
face   default
elapsed 1001 ms
result test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`Cargo.toml` 里没有任何 `[features]`，所以 default 面就是唯一的面，不存在"非默认面另有红"的假绿。

## 4. 排除掉的嫌疑点，各是为什么

| # | 嫌疑点 | 为什么不是根因 |
| --- | --- | --- |
| 1 | `src/button/button.rs:32-33` 的**测试期望** | 契约行 `16-17`（doc）独立写着"低于下限的宽度一律抬到下限"，与测试期望一致；两者互相印证而同意的正是"要抬"。规格与测试一致、代码是唯一异类 → 缺陷在实现。另外第 33 行 `width(5, 2) == 5`（限上不动）在病态代码里也是过的，说明红不是期望写错，而是恰好只在"低于限"这一支暴露。 |
| 2 | 第 20/22 行的**分支返回值** | `floor` 与 `amount` 摆放就是对的（低于限返回 `floor`、否则返回 `amount`）。二者中任一被改都会顺带改变 `amount == floor` 或限上情形的可读语义；真正错的是第 19 行把"低于限"这一支送进了错误的入口。探针覆盖了 `amount == floor` 全部 11 组，无违反，佐证分支体本身没问题。 |
| 3 | `src/label/label.rs`（同族兄弟） | 该文件只有 `Label` 的 `root_object!` 声明，**没有任何宽度/算术函数**；`locate` 用同一段症状词只排出 `button.rs:31` 与 `button.rs:18` 两个候选，label.rs 一个词都没命中。兄弟类比在此没有可比对象。 |
| 4 | 注册面机制：`root_object!`（`11-14`）、`lib.rs:8-11` 的 `static_graft_plan!` `cut/graft`、`build.rs` 的 `build_time::run()` | 它们生成/搬运的是**注册面身份**，与算术无关：这条测试是单元测试，直接 `use super::width` 调用，不经过任何 face、graft 或运行时对象。`why --at src/button/button.rs:19` 也把这条依赖面摊开了：`contract` 只有 16-17 两行 doc、`callers` 只有那条测试、`gate` 无 `#[cfg]`、`wiring` 是 lib.rs:8 的 cut（与宽度无关）。修复不触碰任何注册字段，所以**注册面写入通道（`apply`）在本例中不适用**，我一次都没有调它（见 §5）。 |
| 5 | 宏展开 / 动态分派把代码藏起来 | `why --at` 报 `gate: no #[cfg] attribute sits directly above this definition at src/button/button.rs:19`；`cargo test` 明确 `running 1 test`（不是 `0 passed` 的"什么都没跑"），`check` 的 census 也给出 `0 constructively unreachable arm(s)`。函数体是普通源码，肉眼可见，没有宏生成的第二个 `width`。 |
| 6 | 参数类型/签名（`i64`, 形参名 `floor`） | 测试两个实参都是字面量 `1`、`2`，失败差值恰为 `1` vs `2` —— 即"取错了分支"，不是溢出、也不是"下限值传错"。改为 `let floor: i128 = 2` 之类不会改变结果，签名不是变量。 |
| 7 | 构建缓存 / 环境（"是不是没重编") | A 段里不动任何别的东西、只翻一个字符，同一条命令从红变绿再变红，排除了缓存与环境的解释；`check` 与 shell 两条独立路径（桥内 cargo、手工 cargo）得到同一判定。 |

## 5. 我亲手动过的命令与工具调用

手动命令（shell / cargo，按时间顺序）：
1. `ls -la /tmp/scen-runs/S3-ours{,/tree}`、`find /tmp/scen-runs/S3-ours/tree -type f`、`cat Cargo.toml`、`cat -n` 四个源文件 —— 读树。
2. `CARGO_TARGET_DIR=…/cargo-target cargo test --offline`（改前）—— 拿到红与 32:9 的断言原话。
3. wrapper 帮助：`nl --list`、`nl --list apply|read|locate|why|check`。
4. 桥调用 4 次（全部 `--call`）：`locate` → `read --whole` → `why --at src/button/button.rs:19` → `check --face default --target_dir …`。
5. 修：`src/button/button.rs:19` 一个字符 `>`→`<`。
6. 反证 A：`sed -i` 退回 `>` → `cargo test --offline --lib -- --exact button::tests::a_width_below_the_floor_is_raised_to_it`（红）→ `sed -i` 复位 → `cmp` 与修复版逐字节相同。
7. 反证 B：`sed -n '18,24p'` 抠函数文本 + `rustc` 编译两个探针（`probe_after` / `probe_before`）并运行。
8. 收尾：`CARGO_TARGET_DIR=…/cargo-target cargo test --offline`（绿）+ `nl --call check`（已在 §3C）。

工具调用计数（`/tmp/scen-runs/S3-ours/calls.log`，`=== CMD` 行共 **10** 条）：
- **MCP 工具调用（`--call`）= 4**：`locate` 1、`read` 1、`why` 1、`check` 1；
- 其余 6 条是 `--list`（看工作流表 1 次 + 5 个工具各 1 次说明），是用法查询、不碰树。
- **被拒绝（refusal）= 0**；**预览/重复调用 = 0**；`apply`（写通道）调用 = 0 —— 本例缺陷落在一个普通函数体里，注册面写入通道没有对应字段，调用它只会得到拒绝或改写无关面，所以没有产生第十三轮量到的那类浪费（7 次拒绝 + 3 次重复预览）。整轮零浪费。
