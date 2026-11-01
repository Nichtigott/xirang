# 独立验证：B1-4（Studio Edit 表单 / 检视器不再丢 deny，t52）

- 被验证的修复：`studio/studio/app/source_index.rs` 的 `admission_text`（`studio/studio/app/source_index.rs:71`）——t52 让它在 allow 与 deny 同时非空时渲染**两个子句** `allow:…;deny:…`；两个消费方是 Edit 表单预填（`studio/studio/app/keyboard.rs:79`）与检视器那一行（`studio/studio/ui/panels.rs:151`，经 `studio/studio/ui/search.rs:97` 的 `format_admission` 别名）。
- 复核人 / 独立性声明：复核人 **kernel-auditor**。t52 的作者是另一位成员（studio 路），我**不是**这条修复的作者。我所是的作者那条是 B1-1/t43（内核 `parse_admission_expression`），其独立验证是 t51 与 t55（t55 因此已由队长改派）。本报告是 B1-4 的**第一条独立验证**。
- 结论：**证实**（范围内无证伪、无部分证实项）。修复在两条真实路径上都成立，且三条钉子都经变异证明是活的。

## 1. 我的装置（不复用作者的测试）

装置在检出之外：`/tmp/nk-b14` = 工作树完整副本（`rsync -a --exclude target --exclude .git --exclude docs`，未改动检出）。我在副本里加了自己写的一个探针文件 `studio/studio/app/tests/b14_probe.rs`（经副本的 `studio/studio/app/tests.rs` 以 `#[path = …] mod b14_probe;` 挂载，与既有测试同形），以及自己的临时工程生成器。两条探针都在副本里跑：

1. `b14_prefill_and_inspector_keep_the_deny_veto` —— 一个真实往返：
   - 生成一次性宿主工程：`src/control/control.rs` 里一个 `control_object!` 面，其 `admission: crate::Admission::new(&["ui"], &["ui/experimental"])`（编辑器所写的构造形式）；
   - `App::load()`（真加载器，`generated_snapshots_from` 扫 `src/`）；
   - **真实按键路径**：`App::handle_key(KeyEvent::from(KeyCode::Char('e')))`，从 `Overlay::Edit` 里取出 Edit 表单的值（预填）；**写回跳**；**真实渲染路径**：`ratatui` `TestBackend`(200×50) + `crate::studio::ui::draw`，把一帧的缓冲文本按行拼出来找 deny 路径。
   - 判据是语义的，不是比文本：`parse_admission_owned(预填)` 重建出运行期门禁，然后断言 `gate.accepts("ui") == true` 且 **`gate.accepts("ui/experimental") == false`**；另用保存路径的渲染器 `render_admission(预填)` 断言写回源码的表达式仍含 deny。失败先收集、最后一次断言，因此一次运行报出所有被破坏的路径。
2. `b14_kernel_grammar_refusals_hold` —— 直接问内核：4 个规范拼法（`ANY`/`allow:a,b`/`deny:c`/`allow:a,b;deny:c`）必须被渲染器逐字节复现；5 个畸形子句（`allow:a;allow:b`、`allow:a;veto:b`、`allow:a;deny:`、`allow:a;b`、`allow:`）必须被内核拒绝。

作者的两个钉子（`both_lists_survive_the_compact_rendering`（`studio/studio/app/source_index.rs:153`）、`the_kernel_reads_back_every_value_this_renderer_emits`（`studio/studio/app/source_index.rs:172`））**不是**我的装置；它们只在变异阶段作为观察对象（确认它们真的会红）。

## 2. 绿（原样输出）

```
=== FINAL APPARATUS baseline ===
test studio::app::tests::b14_probe::b14_kernel_grammar_refusals_hold ... ok
test studio::app::tests::b14_probe::b14_prefill_and_inspector_keep_the_deny_veto ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out
BASE_EXIT=0
```

作者的两条钉子在静置树（检出内）也绿，见 §5 门禁输出里的 `admission_text_tests::… ok`。

## 3. 变异 A：把 `admission_text` 改回 t52 之前的行为（只输出 allow）

在**副本**里把函数体改成：

```text
if !admission.allowed_paths.is_empty() { return format!("allow:{}", admission.allowed_paths.join(",")); }
if !admission.denied_paths.is_empty()   { return format!("deny:{}", admission.denied_paths.join(",")); }
"ANY".to_owned()
```

**我的装置变红（exit 101），一次运行报出三条：**

```
test studio::app::tests::b14_probe::b14_prefill_and_inspector_keep_the_deny_veto ... FAILED
the rebuilt gate no longer vetoes the denied path: prefill="allow:ui" gate=OwnedAdmission { allowed_paths: ["ui"], denied_paths: [] }
the saved declaration would drop the deny list: prefill="allow:ui" saved=crate::Admission::new(&["ui"], &[])
the inspector row dropped the denied path:
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 99 filtered out
MUT_EXIT=101
```

三条分别对应验收的三处：**语义门禁**（重建后不再否决被 deny 的路径）、**写回跳**（保存路径会写回 `&[]` 的 deny）、**检视器**（那一行不再印出被 deny 的路径）。`b14_kernel_grammar_refusals_hold` 同时变红（变异体把规范拼法 `allow:a,b;deny:c` 复现成 `allow:a,b`）。

**作者的两个钉子在同一次变异下也变红**（这是“钉子真的在生效”的直接证据）：

```
test studio::app::source_index::admission_text_tests::both_lists_survive_the_compact_rendering ... FAILED
  left: "allow:ui"
 right: "allow:ui;deny:ui/experimental"
test studio::app::source_index::admission_text_tests::the_kernel_reads_back_every_value_this_renderer_emits ... FAILED
  left: "allow:a,b"
 right: "allow:a,b;deny:c"
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 99 filtered out; finished in 0.00s
```

## 4. 变异 B：让渲染器产出内核拒绝的值 → `debug_assert` 真的会拦

副本里把子句渲染改成产出 `allow:;deny:`（内核拒绝的畸形值）：

```
thread '…b14_prefill_and_inspector_keep_the_deny_veto' panicked at studio/studio/app/source_index.rs:85:5:
the rendered value must be one the kernel's compact parser reads: allow:;deny:
test result: FAILED. 0 passed; 1 failed; … ASSERT_PROBE_EXIT=101
```

即 `admission_text` 里那条 `debug_assert!`（`studio/studio/app/source_index.rs:85`）不是装饰：产出值一旦不是内核能读的拼法，测试构建马上 panic，并报出被拒绝的那个值。

## 5. 门禁（静置树，检出内原样输出）

```
gate 1  cargo test -p xirang-studio --offline --all-features            G1_EXIT=0   5 ok-binary / 0 FAILED
        …::source_index::admission_text_tests::both_lists_survive_the_compact_rendering ... ok
        …::source_index::admission_text_tests::the_kernel_reads_back_every_value_this_renderer_emits ... ok
gate 2  cargo test --workspace --offline                                  G2_EXIT=0   ok_binaries=52 failed=0
gate 3  cargo clippy --workspace --all-targets --offline -- -D warnings   G3_EXIT=0   error 计数 0
gate 4  cargo test -p xirang-conventions --offline                      G4_EXIT=0   99 passed; 0 failed
```

## 6. 零残留与独立复核的作者主张

- 变异只在 `/tmp/nk-b14`（我自己的副本）里进行，且每次变异前先取逐字节备份。恢复后：副本内 `MUTATION` 标记计数 **0**、我的装置复绿（2 passed）。
- 检出未被我改动：`diff` 我变异前从检出 rsync 来的 `source_index.rs` 与当前检出文件 → **无差异**；`git status` 里 `studio/` 的四个条目分别是 t52 的 `keyboard.rs`/`source_index.rs` 与 t44/t54 的 `mutations.rs`/`tests/lock_writes.rs`，None 由本次验证产生。未 commit、未做任何 git 恢复类操作。
- 作者那条 grep 主张独立复核**成立**：`grep -rn "split(';')" studio/src` → 0 命中；`grep -rn "split_once(':')" studio/src` → 0 命中（即 studio 侧没有第二份自己拆分紧凑 admission 的代码，渲染只有 `admission_text` 一处）。

## 7. 结论与范围（诚实说明）

- **结论：证实。** B1-4 关闭：在 allow 与 deny 同时非空的声明上，Edit 表单预填与检视器那一行都保住 deny，重建出的运行期门禁仍否决被 deny 的路径，且保存路径写回源码的表达式仍带否决权。三条钉子（`debug_assert` 交内核裁决、交叉测试逐字节互钉规范拼法、5 个畸形子句被内核拒绝）都经变异证明是活的。
- 未覆盖（不影响本次结论）：① 检视器在**极窄**面板下的换行/截断形态（我在 200×50 的测试终端上断言缓冲文本含 deny 路径；`detail_field` 会按 `inner_width` 换行，但 >143 列的门禁串在真实窄窗口里的可读性未测）；② `debug_assert` 只在 debug/test 构建生效（这是 `debug_assert` 的定义，不是缺陷）：release 构建下“产出值必须可被内核读回”没有运行时保险，只有作者的两条测试在 CI 里把关。
- 同族遗留（作者在 t52 报告里已提出，本报告只记录、不重开）：内核的紧凑渲染器 `compact_admission` 仍是私有，Studio 侧是第二份实现；本次证据表明两者**当前**逐字节一致（规范拼法复现那一半），但要真正收回一份实现，需要内核公开一个紧凑渲染入口——那是下一轮的结构决策。
- 证据可复现：`/tmp/nk-b14` 副本 + 我的探针文件即装置；`cd /tmp/nk-b14 && cargo test -p xirang-studio --offline --all-features b14_` 复现绿，按 §3/§4 改 `admission_text` 复现红，恢复备份复现绿。
