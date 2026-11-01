# 独立验证 B1 三条 CRITICAL 修复（t51）

验证人：gates-auditor。工作树 `HEAD=a524956`；**源码最终状态与开工时逐字节相同**（8 个相关文件的 sha256 见 §5，跑门禁前后一致），本轮唯一写入是本文件。
装置：`/tmp/vs-probe`（我自己的探针 crate，path 依赖真实的 `build_method` / `core`，`[workspace]` 自成一体；t10 起就在用，本轮新增 `admission` / `lock` / `identity_pair` / `x1` 四个模式）；夹具 `/tmp/vs-probe/fx/{pkg-a,pkg-b}`；备份 `/tmp/b1mut-backup/`。

## 已核范围

**本轮亲手核过**：三条修复各自的独立探针（不复用作者测试）＋每条一次变异测试（把实现改回旧行为 → 探针与作者的钉子变红 → `cp` 还原 → 哈希/grep/diff 三重确认零残留）＋ t44 的 official 路径等价性 ＋ t9 的 X-1 现状 ＋ 全仓进程级缓存普查 ＋ 五条门禁在**哈希钉住的树**上逐条跑。

**没核**：三条修复的*技术选型*（例如该不该在 core 里加 `with_appended_line`）；Studio TUI 的交互路径（我只核到 `mutations.rs:299/320` 的调用点与内核 API 的行为，没有驱动真实 TUI）；其它 B1 条目（B1-1/B1-4 由别人验，`audit-verify-fix-b1-1.md` / `-b1-4.md` 是他们的产物）；`audit-logic-safety-report.md` 里其余 200+ 条的结论。

**抽样/口径说明**：身份缓存那条我用**端到端**（两个夹具包、`run_for`）而不是库里用法，热/冷 × 两种包顺序共 4 组，每组 1 次运行（顺序无关性不是概率性断言，4 组足以覆盖）；变异测试每条只做 1 种旧行为（见 §2–§4 的说明，M3 的旧行为是「键只有相对路径 + 无读取侧复核」这一整体）。

**两条必须披露的过程事实（诚实优先）**：
1. 我的第一次 `identity_pair` 实现有**我自己的 bug**：它按「哪个 unit 文件含这条相对路径」在共享 units 目录里取**第一个**命中，于是读到了 pkg-a 的文件来判断 pkg-b，四组都误报 FAIL。改为按「包自己的绝对路径 + 源字节」算 fingerprint 精确打开该包自己的 unit 文件后，四组全部 PASS（§4）。误报的中间输出仍留在会话里，不作证据。
2. 我的变异窗口（18:31–18:33）与**另一位成员的 `cargo test --workspace` 运行重叠**（当时系统里有别人的 cargo 进程）。因此**别人在那个窗口内跑出的 workspace 结果可能看到的是我的临时变异体**，应当作废重跑；我自己的变异实验每一步都有哈希备份、并在事后逐字节还原（§2–§4 的还原行），我自己的门禁运行改在 18:36:39–18:37:25 并用源文件哈希钉住（§5）。

---

## 1. 装置与证据口径

```sh
# 探针（我自己的装置，不复用作者的测试）
cd /tmp/vs-probe && cargo build --offline
./target/debug/vs-probe admission                      # B1-1
./target/debug/vs-probe lock                           # B1-2（重复身份 / 无换行 / official 等价 / 拒写不动文件）
./target/debug/vs-probe x1                             # t9 X-1 的现状
./target/debug/vs-probe identity_pair <target> ab|ba    # B1-3（冷/热 × 两种包顺序）
```

`identity_pair` 的夹具是**它自己搭的**：两个包 pkg-a / pkg-b，各有 `src/control/**`（从 `examples/control-button/src/control` 复制），因此共享同一条相对路径 `control/object/button/button.rs` 而命名空间不同；探针按 `NodeId::from_namespaced_path(<包名>, <相对路径>, <kind>)` **自己算期望身份**，再与管道为每个包写出的 unit 文件（`<CARGO_TARGET_DIR>/xirang/cache/units/<该包自己的 fingerprint>.tsv`）里的 `node` 字段比对。

---

## 2. B1-1（t43）`parse_admission_expression` 两列表非空不再丢 deny —— **证实**

**探针（当前树）**
```text
$ ./target/debug/vs-probe admission
ctor_compact=Ok("allow:ui,ui/controls;deny:ui/experimental")
allowed=["ui", "ui/controls"] denied=["ui/experimental"]
deny_survives=true allow_complete=true
PROBE_ADMISSION=PASS
rendered_source=crate::Admission::new(&["ui", "ui/controls"], &["ui/experimental"])
round_trip=Ok("allow:ui,ui/controls;deny:ui/experimental") stable=true
```
关键点：构造形式 `Admission::new(&["ui","ui/controls"], &["ui/experimental"])` 读回的紧凑值**同时带两张列表**，再经 `parse_admission_owned` 解析后 `denied_paths=["ui/experimental"]` 未丢；`render_admission` → `parse_admission_expression` 往返稳定。调用链是真实的：`kernel/src/registry_core/authoring/snapshot.rs:80` 用 `parse_admission_owned` 读注册面。

**变异测试（旧行为）**：备份 `admission.rs`（sha256 `c14af278…`）后，把构造函数那一支改回旧的三分支（`(false, _) => allow:…`，即 deny 被丢）。
```text
$ ./target/debug/vs-probe admission | tail -4      # 变异体
deny_survives=false allow_complete=true
PROBE_ADMISSION=FAIL
rendered_source=crate::Admission::new(&["ui", "ui/controls"], &[])   ← deny 被丢，写回即放宽门禁
$ cargo test -p xirang-core --offline --all-features --lib both_lists
test …both_lists_survive_the_read_and_write_round_trip ... FAILED
test result: FAILED. 1 passed; 1 failed
```
**还原**：`cp /tmp/b1mut-backup/admission.rs` 回来 → sha256 与备份相同、`grep -c 'return Ok(compact_admission(&allow, &deny));'` = 1、`git diff --stat` 回到 7 files / 909+/103- ✓ 零残留。探针复绿（PASS）。

**结论：证实。** 附带确认：作者的钉子在该变异下**确实会红**（不是恒真）。

---

## 3. B1-2（t44）写盘不再产出宿主拒绝读的锁 —— **证实**

**探针（当前树）**
```text
$ ./target/debug/vs-probe lock
duplicate_identity=Err("plugin lock line 2: duplicates package identity `alpha` `0.1.0`")
unterminated_lock_append=Ok("Ok")
unterminated_parses_two=true text="user|…|extension\nuser|…|extension\n"
official7_records=1 strict_contains=false contains_record=true
refusal_left_entry_untouched=true
PROBE_LOCK=PASS
```
覆盖验收的三件事：① 同身份（五元组）不同 checksum 的追加被 `with_appended_line` **拒绝**，返回 `Err` 而不是写出一份宿主读不了的锁；② 末行无换行的锁追加新身份后**补上换行**，结果 `PluginCatalog::parse` = Ok 且 2 条记录（旧行为会粘成 `extensionuser|…`）；③ 我按调用方契约复刻「被拒就不写」——入口文件写入前后 `before == after`**逐字节未变**。
调用点是真实的：`toolchain/src/studio/studio/app/mutations.rs:320` 用 `PluginCatalog::with_appended_line(&existing, &line)`、`:299` 用 `contains_record(&candidate)`。

**变异测试（旧行为）**：备份 `catalog.rs`（sha256 `3e932bfb…`）后把 `with_appended_line` 改成旧写入方那套——不校验、不补换行：`Ok(format!("{lock}{line}\n"))`。
```text
$ ./target/debug/vs-probe lock | tail -6           # 变异体
duplicate_identity=Ok("Ok")                        ← 解析器会拒绝的记录被写出去
unterminated_lock_append=Ok("Ok")
unterminated_parses_two=false text="…extensionuser|framework.two|beta|…"   ← 两条粘成一条
refusal_left_entry_untouched=false
PROBE_LOCK=FAIL
$ cargo test -p xirang-studio --offline --all-features --lib lock_writes
… an_official_append_that_would_duplicate_an_identity_is_refused_not_written ... FAILED
… a_duplicate_user_identity_is_never_written_unreadably ... FAILED
… a_lock_without_a_trailing_newline_is_never_glued_to_the_next_record ... FAILED
… toggling_the_mode_is_never_written_as_a_duplicate_identity ... FAILED
test result: FAILED. 0 passed; 4 failed
```
**还原**：`cp` 回来 → sha256 相同、`grep -c 'Self::parse(&candidate)?'` = 1、变异体形状 0、`git diff --stat` 回到 909+/103- ✓；探针复绿（PASS）。

**额外两条（验收要求）**
- **official 路径等价性**：锁里已有七字段 official 记录、候选是同一身份的十字段记录时——旧规则 `contains` = **false**（十字段全等才算已装 → Studio 会去追加 → 撞上解析器的重复身份拒绝），新规则 `contains_record` = **true**（身份七字段相等、来源三字段按记录侧期望）。探针输出 `strict_contains=false contains_record=true` ✓——这正是 t44 修的那处「同一条规则、两种输入拼法」。
- **t9 的 X-1 现状（十字段 + 空来源）**：仍是**原样**，没有被 t44 改动，也**不是**本次三条修复的一部分：
  ```text
  $ ./target/debug/vs-probe x1
  ten_fields_empty: Ok signature=None fingerprint=None revocation=None   ← 十字段第 8/9/10 列为空仍被接受
  ten_fields_full: Ok signature=Some("sig") fingerprint=Some("finger") revocation=Some("rev")
  empty_source: Err(plugin lock line 1: unknown plugin source ``)        ← 空 source 字段本身是被拒的
  seven_fields: Ok signature=None fingerprint=None revocation=None
  writer_accepts_ten_empty=Ok("Ok")
  ```
  即审计 LG-40 里的 X-1（「记录看起来带身份三元组，实际三个都不在」）**至今未修**，仍是 MINOR；t44 只保证**写出去的东西解析器能读**，不改变读取侧对空来源字段的语义。

**结论：证实。**

---

## 4. B1-3（t45）`CACHED_NODE_IDS` 键含命名空间 + 读取侧复核 —— **证实**

**探针（当前树，4 组）**：冷/热 × A→B / B→A，每组都打印每个包**自己**的 unit 文件里那条共享路径的身份与现算期望。
```text
$ rm -rf tgt && ./target/debug/vs-probe identity_pair /tmp/vs-probe/tgt-cold ab      # 冷 A→B
run_for(pkg-a)=Ok … pkg=pkg-a recorded=e0851ccb2e814e296cc08172825125dd expected=e0851ccb2e814e296cc08172825125dd match=true
run_for(pkg-b)=Ok … pkg=pkg-b recorded=325377c4eabe871a18a82f717f1a9494 expected=325377c4eabe871a18a82f717f1a9494 match=true
PROBE_IDENTITY=PASS
$ ./target/debug/vs-probe identity_pair /tmp/vs-probe/tgt-cold ab                    # 热 A→B（同 target 第二个进程）
PROBE_IDENTITY=PASS
$ ./target/debug/vs-probe identity_pair /tmp/vs-probe/tgt-cold ba                    # 热 B→A
PROBE_IDENTITY=PASS
$ ./target/debug/vs-probe identity_pair /tmp/vs-probe/tgt-cold2 ba                   # 冷 B→A
PROBE_IDENTITY=PASS
```
四组里两个包各自拿到的都是**自己命名空间下现算的身份**（`e0851ccb…` vs `325377c4…`，与包名绑定），没有一个包读到另一个包的条目。

**变异测试（旧行为 = 只用相对路径作键且不做读取侧复核）**：备份 `node_id.rs`（sha256 `6ffd5f2c…`）后把 `NodeIdCache::get` 改成 `self.entries.iter().find(|((_, r), _)| r == relative).map(|(_, value)| value)`。
```text
$ ./target/debug/vs-probe identity_pair /tmp/vs-probe/tgt-mut ab   # 变异体 · 冷 A→B
PROBE_IDENTITY=PASS          ← 冷态 map 为空（第一个包没有已预热的 unit），无从命中
$ ./target/debug/vs-probe identity_pair /tmp/vs-probe/tgt-mut ab   # 变异体 · 热 A→B
pkg=pkg-a recorded=e0851ccb2e814e296cc08172825125dd match=true
pkg=pkg-b recorded=e0851ccb2e814e296cc08172825125dd expected=325377c4eabe871a18a82f717f1a9494 match=false
PROBE_IDENTITY=FAIL          ← 复现旧 CRITICAL：第二个包拿到第一个包的身份，并把它写进了落盘的 unit
$ cargo test -p xirang-build-method --offline identity_cache
… an_entry_is_readable_only_in_the_namespace_it_was_primed_for ... FAILED
   assertion `left == right` failed: another package's entry is not reachable
   left: Some(e0851ccb2e814e296cc08172825125dd)  right: None
… an_entry_that_does_not_match_its_own_identity_is_refused ... FAILED
test result: FAILED. 2 passed; 2 failed
```
- **热 A→B 变异体复现了 LGC-LG-01 的旧行为**（磁盘上实锤：pkg-b 的 unit 写进了 pkg-a 的身份），所以我的探针对这条缺陷是敏感的真探针，不是恒真装置。
- 变异体 · 热 B→A **没有**复现：那个排布下第二个包没有走「从进程级缓存取身份」那条路（它的 unit 已有效、被当作命中消费），故污点不落盘。我如实记下这一处**可达性差异**——它不影响结论（正对照由 A→B 提供），但说明"旧行为需要第二个包在写 unit 行时真的去查那张进程级表"。
**还原**：`cp` 回来 → sha256 相同、变异体形状 `grep -c` = 0、`git diff --stat` 回到 909+/103- ✓；探针复绿（PASS）。

**全仓进程级缓存普查（t45 作者称只有 `CACHED_NODE_IDS` 需要命名空间键）——独立复核：结论一致**

我在 9 个生产 crate 的 `src/**`（排除 `*_tests.rs`）里穷举 `OnceLock` / `OnceCell` / `LazyLock` / `lazy_static!` / 非 const `static` / `thread_local!` / `Mutex<HashMap>`：

| 位置 | 形态 | 是否需要命名空间键 | 判据 |
| --- | --- | --- | --- |
| `toolchain/src/build_time/src/node_identity.rs:37` | `static CACHED_NODE_IDS: OnceLock<NodeIdCache>` | **需要（已修）** | 唯一存 `NodeId` 的进程级表；`get` 现在 (namespace, relative) + 复核 |
| `toolchain/src/build_time/src/identity.rs:22` | `static PACKAGE_NAMESPACE_OVERRIDE: OnceLock<String>` | 不需要（它就是命名空间来源） | 先到先得是设计；每次管线运行由 `run_as_package` 的线程局部位（`toolchain/src/build_time/src/identity.rs:45`）压过它，且每一条生产路径都从 `toolchain/src/build_time/src/lib.rs:234` 进作用域 |
| `toolchain/src/runtime/src/authoring/context.rs:33` | `thread_local! ACTIVE_CONTEXT: RefCell<Option<AuthoringContext>>` | 不需要 | 线程局部 + 作用域安装，且 `AuthoringContext{package_root, namespace}` **自带命名空间** |
| `toolchain/src/studio/studio/app/project_context.rs:24` | `thread_local! PROJECT_CONTEXT` | 不需要 | 同上：`select_project` 把 `{root, manifest, namespace}` 一起装进本线程 |
| `toolchain/src/runtime/src/runtime/trace/snapshot/parse.rs:295` | `Mutex<BTreeSet<&'static str>>`（字符串 interner） | 不需要 | 去重的是 artifact 文本里的字符串，不是身份 |
| `toolchain/src/build_time/src/scope_view.rs:203`、`toolchain/src/build_time/src/graft_plan_check.rs:450`、`toolchain/src/build_time/src/face_view.rs:309`、`toolchain/src/build_time/src/package.rs:116`、`toolchain/src/runtime/src/authoring/filesystem.rs:29`、`:57`、`toolchain/src/runtime/src/runtime/trace/snapshot/io.rs:119`、`toolchain/src/mcp/preview.rs:33`、`toolchain/src/mcp/source_index.rs:229`、`toolchain/src/bin/xirang-dev.rs:289` | `AtomicU64` 序号 | 不需要 | 只发临时文件名，不带身份 |

- 没有发现 `LazyLock` / `OnceCell` / `lazy_static!` / `Mutex<HashMap<…>>` 形态的缓存；`core/` 里**没有**任何进程级 `static` 缓存。
- 一处**值得记的邻近风险**（不是缺陷）：`package_namespace()` 在**没有**运行作用域时会读那个进程级 pin，因此任何"直接调 `package_namespace()` 的旁路代码"都会粘在第一个包上；当前的每一条生产路径都经过 `run_as_package`（`toolchain/src/build_time/src/lib.rs:234`）或线程局部上下文，所以现在成立——这条不变量没有门禁钉住，建议后续加一条断言或注释。

**结论：证实。**

---

## 5. 门禁（哈希钉住的树，五条逐条）

源文件钉住（跑前 18:36:39 / 跑后 18:37:25，`diff` 为空 = **PIN_STABLE**）：`identity_cache.rs c1d8411e…`、`node_id.rs 6ffd5f2c…`、`admission.rs c14af278…`、`catalog.rs 3e932bfb…`、`mutations.rs bfb1cc61…`、`keyboard.rs be6d66e1…`、`source_index.rs 36e82e8f…`、`tests/lock_writes.rs 9d3f5466…`。

| # | 命令 | 结果 |
| --- | --- | --- |
| 1 | `cargo fmt --all -- --check` | **exit 0** |
| 2 | `cargo test --workspace --offline` | **exit 0**：**52** 个 test result 全 `ok`、`0 failed`、累计 **756 passed**（`test result: FAILED` 0 次） |
| 3 | `cargo clippy --workspace --all-targets --offline -- -D warnings` | **exit 0**（`error` 0 行） |
| 4 | `cargo test -p xirang-conventions --offline` | **exit 0**：`99 passed; 0 failed` |
| 5 | `tools/xirang-publish --check-table` | **exit 0**：`dependency table matches the manifests (9 crates)` |

**跑前确认**：我在 18:33–18:36 反复探测在途写者，确认当时**另有成员在跑 `cargo test`**（见开头披露），所以我的门禁没有抢到"全仓静默"窗口，改用**源文件哈希钉住**代替：跑前/跑后 8 个相关文件哈希完全相同，且期间**没有任何 `.rs` 被写**（`git status` 里只有 docs/ 下的他人产物变化）。因此这五条门禁结果对应的是**同一份、未被任何人改动的源码状态**；并发只发生在共享 `target/` 的编译缓存上（cargo 自身加锁串行化），不改变结论。若评审要求"全仓零在途"的强静默窗口，需要在没有其他成员运行时重跑一遍——这不改变本轮任何判定。

---

## 6. inScope 对账（三条修复之外无未声明源码改动）

`git diff --stat` = **7 files changed, 909 insertions(+), 103 deletions(-)**，`git status` 除文档外只有：

| 文件 | 归属 | 是否声明 |
| --- | --- | --- |
| `core/src/registry_core/authoring/parse/admission.rs` | t43（B1 修复①） | ✓ |
| `core/src/registry_core/plugin/catalog/catalog.rs` | t44（B1 修复②） | ✓ |
| `toolchain/src/studio/studio/app/mutations.rs` | t44 + t54（拆分） | ✓ |
| `?? toolchain/src/studio/studio/app/tests/lock_writes.rs` | t54（t44 的四条钉子搬到这里） | ✓ |
| `toolchain/src/build_time/src/node_id.rs`、`toolchain/src/build_time/src/identity_cache.rs` | t45（B1 修复③） | ✓ |
| `toolchain/src/studio/studio/app/keyboard.rs`、`toolchain/src/studio/studio/app/source_index.rs` | **t52（B1-4）**，其交接里已声明 | ✓（不属于 t43/t44/t45，但已声明） |

文档侧的 `docs/audit-2026-09-28/` 变动（`audit-report.md` / `audit-findings.json` / `audit-structure-map.html` 以及 `audit-verify-fix-b1-1.md`、`-b1-4.md`、`audit-report-recheck-full.md`）都是**别人的产物**，不在本任务 inScope 内、我一行未碰。我自己的三次变异全部有 `/tmp/b1mut-backup/` 备份并已逐字节还原（三处 `cmp` 全 IDENTICAL、`git diff --stat` 与开工前一致）。**结论：无未声明改动，我的改动为零残留。**

---

## 7. 结论

| 条目 | 判定 | 关键证据 |
| --- | --- | --- |
| ① t43 · admission 两列表非空不丢 deny | **证实** | 探针 `deny_survives=true` + 往返稳定；变异体下探针 FAIL 且作者钉子 `both_lists_survive_the_read_and_write_round_trip` FAILED |
| ② t44 · Studio 写盘产出解析器可读的锁、拒写不动文件 | **证实** | 探针：重复身份 `Err`、无换行锁补换行后 parse Ok 且 2 条、拒写时入口文件逐字节未变；变异体下探针 FAIL 且四条 `lock_writes` 钉子全 FAILED；official 七/十字段等价 `contains_record=true`（旧 `contains=false`） |
| ③ t45 · 身份缓存按命名空间取键 + 读取侧复核 | **证实** | 端到端 4 组（冷/热 × 两序）两包各得自己的身份；变异体热 A→B **复现旧污染**（pkg-b 落盘 pkg-a 的身份）且作者两条钉子 FAILED；全仓进程级缓存普查独立复核后与作者结论一致 |

**五条门禁在哈希钉住的同一份源码上全绿**（fmt 0；workspace 52 个二进制 / 756 passed / 0 failed；clippy -D warnings 0；conventions 99 passed；`--check-table` 9 crates）。

**三条 B1 修复可以判定为已关闭。**（X-1/LG-40 的「十字段空来源」读取语义仍未修，但它是 MINOR 且不在 B1 三条之内；本文已记录其现状。）
