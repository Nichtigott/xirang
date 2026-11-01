# t3 变更报告：S1 —— `verify` 的第二行连接器判定

切片 = `audit-mcp-workflow.md` 排序表 S1 行（:229），理由 :237-240，口径 :242-246；针对 F1
（:114-119，t1 的 X1）。改动 `toolchain/src/mcp/verify.rs`（+26 行）、`verify_tests.rs`
（+52 行，1 条钉子）；未动内核、CLI `check`、`converge`、`usages`；驱动与截留在 `/tmp/t3/`。

## 1. 改前复现 X1（自己跑）

装置：`examples/control-button` 副本 + 手工放一个全树无提供者的 `widget` 面（requires 为
`["control.theme" => "Theme"]`）；一个 stdio 会话 `apply`（`apply:false` 预览）→ `verify`，
二进制由 `git show HEAD:…verify.rs` 重建。apply: `isError=True chars=1141`，正文
`the package's own faces were rejected: …` + `data-flow attachment failed: input control.theme
has no provider; expected provider kind Theme`；verify: `isError=False chars=218`，只有
`verdict ok (the kernel accepted the tree)`。

机制（两处都不改）：静态面只查 active scope 里的面（`face_syntax_check.rs:22-27` 展开 scope、
`registration_phase.rs:42-45` 按 active retain 丢掉范围外需求）；`load_registry`
（`apply.rs:510`）读每个面文件、不看作用域。control-button 入口用 `static_graft_plan!` 把
scope 收窄到两个切口，新放的 widget 不在其中 ⇒ 静态跳、连接器照判（t2 那次是 `apply:true`，
这里 `apply:false`，拒绝相同且确定不改树）。

## 2. 改动本体

`verify.rs` 在 `verdict` 之后新增（+26 行含双语文档注释）：`match
crate::mcp::apply::load_registry(root, &package)`，`Ok` 印 `connector verdict: ok (the
package's own faces were admitted)`，`Err(rejection)` 印 `connector verdict: rejected` 加同一个
`bounded`（`MAX_DIAGNOSTIC_LINES`）截断的拒绝正文；末尾
`Ok(format!("{verdict}{connector}\n{delta}"))`。`verdict ok` 字面与 `check_for` 调用一字未动；
拒绝是答案不是故障：仍 `Ok`、`isError=false`（与 `converge` 同约定）。

## 3. stdio 桥实测（改后，同树同会话）

verify: `chars=1386 lines=17`，前两行 `verdict ok (the kernel accepted the tree)` 与
`connector verdict: rejected`，随后是同一份连接器错误树与原 delta。健康树（去掉 widget）：
`verdict ok` + `connector verdict: ok (the package's own faces were admitted)`。

## 4. 钉子与变异（有牙）

新钉子 `verify_tests.rs::a_connector_rejection_is_reported_next_to_the_static_verdict`：裸包
+ 入口 `pub fn wire() { crate::label::Label; }`（scope 收窄到 label）+ 悬空需求的 widget 面
（同 §1 机制），断言两行在同一回复里并存。变异（`/tmp/t3/mutate.sh`，自动还原）把
`load_registry(root, &package)` 换成 `Ok::<(), String>(())`（永远报 ok）→ `… FAILED /
panicked at verify_tests.rs:179:5 / connector verdict: ok …` / `0 passed; 1 failed`；
还原 → `... ok` / `1 passed; 0 failed`。

## 5. 五条 verify（最终树重跑，全部 exit 0）

`cargo fmt --all -- --check`（无输出）；`cargo test -p xirang-toolchain --offline --features
mcp`（14 处 ok，含新钉子）；`cargo test --workspace --offline`（36 处 ok）；`cargo clippy
--workspace --all-targets --offline --all-features -- -D warnings`（`Finished dev profile`）；
`cargo test -p xirang-conventions --offline`（`ok. 139 passed`，含 anchors resolve）。

边界：`git status --porcelain` 只有 ` M verify.rs`、` M verify_tests.rs`、`?? docs/audit-2026-09-29/`；
examples/ 与 toolchain/tests/fixtures/ 未动；无新增 `#[allow(missing_docs)]`、无新增公开项；未提交。

## 6. 没做的（后续项）

A2（usages isError=true vs converge/本片 false）只记不修；F2/S2、A1/S3、M1–M3/A5→S4 各自是
审计里另一行；`tools/list` 的 verify 描述没加这句（S4 的活）。措辞边界：`connector verdict:
rejected` = 连接器面不接受这棵树；若 `load_registry` 因读取失败出错，正文逐字给出该错误。
