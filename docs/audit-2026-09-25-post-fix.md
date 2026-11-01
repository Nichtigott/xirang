# Adversarial audit of the post-fix tree — 2026-09-25
# 修复后工作树的对抗性审计——2026-09-25

Baseline: `main = f6cf30b` plus the nine paths the fix work left uncommitted
(`CHANGELOG.md`, `Cargo.lock`, `README{,.zh-CN}.md`, `conventions/Cargo.toml`,
`conventions/src/doc_blocks.rs`, `core/.../syntax/{nesting,syntax}.rs`,
`docs/audit-production-readiness.md`). The previous audit's 48 items are closed
(`docs/audit-production-readiness.md`); nothing here repeats them.
基线：`main = f6cf30b` 加上修复工作留下的九个未提交路径。上一轮审计的 48 条已结案
（`docs/audit-production-readiness.md`）；本文不重复其中任何一条。

Method: seven read-only delegated audits, one per angle, each told to reproduce what it
could and to mark anything it could not. They are named in `Angle` on every finding. The
tree was frozen for them except for the nine paths above. **I re-read the code for every
MAJOR and CRITICAL and re-ran the decisive probe myself**; a finding marked `[实测]` is one
whose reproduction I or the delegated audit actually ran (the text says which), `[代码]` is
settled by reading, `[报告]` is relayed and unverified.
方法：七份只读委派审计，每个角度一份，都要求"能复现就复现、不能就如实标注"。每条发现都带
`Angle` 字段。除上面那九个路径外，审计期间工作树是冻结的。**每一条 MAJOR 与 CRITICAL 我都自己
重读了代码并重跑了决定性探测**；`[实测]` 表示复现确实跑过（文里说明是谁跑的），`[代码]` 由阅读
判定，`[报告]` 为转述且未核实。

Not checked by anyone: an online `cargo publish`/`--dry-run` or a real upload (no token);
GitHub-runner execution of either workflow; the macOS/Windows CI legs; `cargo-deny`'s schema;
`tools/xirang-release-audit`, `xirang-scale-audit` and `xirang-external-rehearsal` end
to end; a full visual Studio session.
谁都没查的：在线 `cargo publish`/`--dry-run` 与真实上传（无 token）；两个工作流在 GitHub runner
上的实际执行；macOS/Windows 两条 CI 腿；`cargo-deny` 的 schema；`tools/xirang-release-audit`、
`xirang-scale-audit`、`xirang-external-rehearsal` 的完整运行；完整的 Studio 视觉会话。

## CRITICAL / 致命

### 1. A member crate's version is invisible to the tag precondition and to `--check-table`; the publish tool then uploads a version no tag names
### 1. 成员 crate 的版本对 tag 前置检查与 `--check-table` 都不可见，发布工具因此会上传一个没有 tag 命名的版本

`Angle: release/CI`. The release path reads the version from exactly one place — the **first**
`version = "…"` line of the root manifest — and `cargo publish` reads it from **each member's**
manifest. Nothing compares the two.

- `tools/xirang-publish:184` — `workspace_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)`
- `.github/workflows/release.yml:48-49` — the same `sed` for the tag check
- `tools/xirang-publish:411,453` — the index wait probes `$workspace_version`; `:437,468` print it
- `tools/xirang-publish:432` — `cargo publish -p "$crate"` uses that crate's manifest version
- `tools/xirang-publish:339-358` — `--check-table` compares dependency **names** and never versions

**证据 `[实测]`（我复现）**：在 `/tmp/nla3` 的副本里把 `core/Cargo.toml` 的
`version.workspace = true` 改成 `version = "0.1.1"`：

```
$ tools/xirang-publish --check-table
dependency table matches the manifests (9 crates)          [exit 0]
$ sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1
0.1.0                       # 这是 tag 步骤读到的版本
$ cargo metadata --no-deps …  →  xirang-core 0.1.1
$ cargo package -p xirang-core --no-verify --allow-dirty --offline
   Packaging xirang-core v0.1.1 (/tmp/nla3/core)
```

因此 `v0.1.0` 这个 tag 会被接受，然后上传 `core 0.1.1`；等待循环去 index 找 0.1.0，超时后整个
工作流失败——而**已发布的版本不可撤回**。委派审计另实测了需求侧的同一种失明：把 `macro` 的
`xirang-core` 需求写成 `"0.2.0"`，`--check-table` 同样报绿。

**最小修法**：`--check-table` 与 release 工作流的 tag 步骤都改为比较**每个已发布 crate 解析后的
版本**（`cargo metadata --no-deps`）与 `$workspace_version`，并断言每处内部 `version = "…"`
都等于它；工具的 index 等待也按各 crate 的真实版本探测。

**钉子**：`tools/xirang-publish --check-table`（本轮新增的版本检查）。**已修**：`--check-table`
现在读三处——每个已发布 crate 自己声明的版本、行内写的内部需求、以及
`[dependencies.xirang-x]` 表形式的需求——只要与 `$workspace_version` 不符就退出 1 并给出可操作的
消息。实测两半：副本里把 `core` 改成 `0.1.1` → `xirang-core declares version 0.1.1, but the
workspace version is 0.1.0; a tag for 0.1.0 would publish 0.1.1`；把 `macro` 的需求改成 `0.2.0`
→ `xirang-macro asks for xirang-core 0.2.0, but every internal requirement must be 0.1.0`。
工作流在发布前就跑 `--check-table`，因此 tag 步骤读到根版本这一点不再能造成漂移。

### 2. Every function with a lifetime parameter vanished from the source index
### 2. 每个带生命周期参数的函数都会从源码索引里消失

`Angle: kernel`. `core/src/registry_core/source/source.rs` 的 `mask_non_code` 把每个 `'` 都当成
字符字面量的起始引号，一路遮到下一个撇号——包括函数的 `{`；即使撇号成对，`<'a>` 的 `>` 也被遮掉，
`angle_depth` 永不归零。

**证据 `[实测]`（我复现）**：用 /tmp 的临时 crate 直接调公开 API：

```
function_symbols("fn f<'a>(s: &'a str) -> &'a str { s }\nfn g() {}\n") = []      ← 0 / 2
function_symbols("fn f(s: &str) -> &str { s }\nfn g() {}\n")           = ["f","g"]
direct_calls("let s: &'static str = paint();", "current")                = []
direct_calls("let s: &str = paint();", "current")                        = ["paint"]
```

影响面：`mcp/index.rs:188,191`（agent 索引）与 `studio/.../graph_queries.rs:50`——也就是
"AI 与 Studio 看这个工程有哪些函数"这件事，在带生命周期的代码上**静默给出空答案**。

**已修**：`mask_non_code` 现在只在字符字面量真的闭合时才把它当引号（`'\…'` 或一个字符后紧跟
`'`），其余撇号（生命周期、标签、`&'static`）只遮掉撇号本身。**钉子**：`source_tests.rs` 的
`lifetimes_do_not_mask_the_code_that_follows_them`、`character_literals_are_masked_like_strings`
（测试模块为此从 `source.rs` 拆出，使被测模块留在 450 行棘轮内）。

### 3. Studio's Delete moved a module out of the process CWD project, not the open one
### 3. Studio 的删除把模块从"进程 CWD 的项目"里搬走，而不是打开的那个

`Angle: surfaces`. `studio/studio/app/overlay/delete.rs:11` 是唯一没有用
`with_authoring_context` 的创作调用（add/edit/graft 全都用了），而 `delete_module` 经
`generated_paths → source_root → validation::package_root()` 解析根路径，回落顺序是
`XIRANG_PACKAGE_ROOT` → 带 `Cargo.toml` 的 CWD → `env!("CARGO_MANIFEST_DIR")`。Studio 自己
的项目上下文对它不可见。

**证据 `[实测]`（委派审计，我用代码复核 + 自己写了钉子）**：把夹具项目复制到 `/tmp/audit/A` 与
`B`，`cd /tmp/audit/A && xirang-studio /tmp/audit/B`，在 pty 里按 `d` `y`：状态行报
`moved \`control\` to /tmp/audit/A/.xirang/trash/…`——**A 的整棵 `src/control` 被搬走**，
屏幕上显示的 B 一个字节没动，然后热重载又把 B 的面显示回来，用户看不出发生过什么。

**已修**：该调用现在包在 `with_authoring_context` 里。**钉子**：
`studio/studio/app/tests/edit.rs::delete_moves_the_module_of_the_open_project`（临时工程，
断言搬走的是打开项目的文件、状态行点名它；把修复撤掉就报
`Delete failed: this module was not generated by XiRang`）。仍未做：让非上下文
`package_root()` 的回落在**破坏性**操作上直接报错，而不是猜一个根。

### 4. Pressing `g` on a project with no graft plans panicked the app
### 4. 在还没有任何 graft 计划的工程上按 `g` 会让 app panic

`Angle: surfaces`. `studio/studio/ui/forms/graft.rs:194`：

```rust
(!graft.plans.is_empty()).then_some(graft.plan_selected.min(graft.plans.len() - 1))
```

`then_some` 会**立即**求值它的参数，因此空列表上 `0 - 1` 下溢。委派审计在 pty 里实测：
`attempt to subtract with overflow`，退出码 101。于是任何 debug 构建（本仓库自己的
`target/debug`、任何 `cargo run`）里，外部 graft 撰写在第一个计划存在之前根本无法使用。

**已修**：改成 `plans.len().checked_sub(1).map(|last| plan_selected.min(last))`。**钉子**：
`studio/studio/app/graft_tests.rs::the_graft_screen_renders_before_any_plan_exists`
（加载夹具工程、断言确实选中了面且界面确实打开，再渲染；把修复撤掉即 panic）。

### 5. The automatic scope misses references inside macro invocations: wrong tree, and `check` says ok
### 5. 自动作用域漏掉宏调用里的引用：生成错误的树，而 `check` 说 ok

`Angle: build/macro`. `core/src/registry_core/syntax/face.rs:277` 的 `ReferenceVisitor::visit_macro`
覆盖了 syn 的默认访问，既不进入宏的 token 流，也不设 `conservative`（只有 `include!`/
`macro_rules` 被标记）。只被 `vec![…]`/`println!`/`assert!` 引用的注册面因此被
`build_method/scope.rs:236-357` 静默剪掉，`scope.rs:352` 也永远看不到"不确定"。

**证据 `[实测]`（委派审计）**：在示例宿主上加一个只被
`vec![crate::control::object::dial::NODE_ID]` 引用的面 `control/object/dial/`：
`xirang check` → **退出 0、`check: ok`**，`source_scope.tsv` 只选了 2 个（button、slider），
`generated_lib.rs` 里没有 `dial`；随后 `cargo check` → `error[E0433]: cannot find \`dial\` in
\`object\` --> src/lib.rs:71:34`。也就是说**构建期说没问题，编译期才炸**——这正是"错树"这一类。

**已修（最终形态：收集路径，而不是放宽作用域）**：`ReferenceVisitor` 现在进入宏调用的 token，
把其中写出的 `::` 路径收集进引用集（`collect_macro_paths`），因此**既保住被宏命名的注册面，又保住
作用域的收窄**。构建自己的接线保持透明（`host!`/`application!`/`static_graft_plan!`/`graft_plan!`/
`__graft_plan_cuts!` 由入口与 graft 扫描器读；`control_object!`/`*_object!`/`external_object!`/
`__…` 的内容是注册元数据）。落地过程中被两条既有测试纠正了两次，记下来因为它们都是真信号：
①"凡宏都放宽"会让入口里的 `host!()` 与注册面里的声明宏把收窄关死；②"凡含标识符都放宽"会被
`format!`/`println!` 触发，同样等于关死收窄。最终规则是**只跟随路径形状**：节点身份只能经
`::` 路径抵达，而匹配器 `path_mentions_module` 也只认路径——宏外的裸标识符今天同样不产生任何
选择，因此宏内的裸标识符保持同一语义。**残留（已在代码文档里写明）**：在展开期从裸标识符**构造**
路径的宏（`paste!` 那一类）在 token 里没有路径，本扫描看不到它。

**钉子**：`face.rs` 的 `a_macro_invocation_is_read_for_the_paths_it_spells`（宏里的路径必须被收集；
无宏文件保持收窄；三种构建自有宏与两种纯字面量宏不得放宽），另加既有的
`source_references_ignore_imports_strings_comments_and_registration_data` 与示例宿主的
`the_declared_slots_define_the_build_time_scope`。**端到端实测**（/tmp 重建委派审计的场景：示例宿主
+ 一个只被 `vec![crate::control::object::dial::NODE_ID]` 引用的 `dial` 面）：

| | `xirang check` | `source_scope.tsv` | 计划里的 dial | 宿主 `cargo check` |
| --- | --- | --- | --- | --- |
| 旧规则 | ok（退出 0） | `selected 2` | **0** | `error[E0433]: cannot find \`dial\` in \`object\`` |
| 修复后 | ok（退出 0） | **`selected 3`** | **7** | 通过 |

**顺带的模块拆分**：修复把 `face.rs` 顶过 450 行棘轮，因此把词法引用扫描（`ReferenceVisitor` +
两个宏辅助函数）拆成 `core/src/registry_core/syntax/reference_scan.rs`（173 行），`face.rs` 回到
316 行，并按约定删掉了 `size.rs` 里 `face.rs` 那条已陈旧的基线债务条目。

### 6. A malformed or gated `static_graft_plan!` panics instead of emitting `phase=graft-entry`
### 6. 畸形或带 cfg 的 `static_graft_plan!` 会 panic，而不是发出 `phase=graft-entry`

`Angle: build/macro`. `build_method/graft_view/query.rs:105-110` 用
`graft_entries(&source).unwrap_or_else(panic!)`；`query.rs:74` 对任何构建无法求值的 `#[cfg]`
（例如 `#[cfg(unix)]`）也 panic——而同样的 cfg 出现在**注册面**上时 `static_plan.rs:107` 是发
`face-cfg` 诊断。`pipeline.rs:65` 在 `scope.rs:219` 已经建好 graft-entry 诊断之后才调它，panic 把
那份诊断丢掉。

**证据 `[实测]`（委派审计）**：夹具宿主 + `static_graft_plan!(FRAMEWORK, cut);` 后
`xirang check --json` → **退出 101、stdout 为空**，stderr 是
`panicked at build_method/graft_view/query.rs:106:9: invalid graft declaration in …graft cut
expects …`。这与 C2 是同一类：签名承诺诊断，实际是 panic。

**已修**：`&mut BuildDiagnostics` 现在穿过 `host_graft_entries`/`read_graft_entries`，读失败、
解析失败与不可求值的门控三类都 push `phase=graft-entry`（读不到时不再 panic；解析失败的消息与
`scope` 已推的那条**逐字相同**，因此两者折叠成一行而不是重复；`GraftSyntax` 没有行号字段，所以
cfg 拒绝也以消息点名入口路径）。四处调用点（`pipeline`、`graft_plan_check`、`scope_tests`、本文件
测试）一并更新。**钉子**：`query.rs::a_malformed_graft_declaration_is_reported_not_fatal` 与
`::an_unevaluable_graft_gate_is_reported_not_fatal`——把两处诊断换回 `panic!`，两条都变红（已验证）。
**端到端实测**（示例宿主 + `static_graft_plan!(FRAMEWORK, cut);`）：

| | 退出码 | stdout | stderr |
| --- | --- | --- | --- |
| 修复前 | **101** | **0 字节** | `panicked at build_method/graft_view/query.rs:128:13` |
| 修复后 | **1** | **347 字节 JSON**（`schema`、`count:1`、`phase=graft-entry`） | 一行 `registration check failed (1 diagnostic(s); JSON on stdout)` |

## MAJOR / 重要

### 2. The kernel-purity gate is bypassed by a brace import and by `std::io`
### 2. 内核纯净性门禁可被树形导入与 `std::io` 绕过

`Angle: gates`. `conventions/src/purity.rs:39-45` lists five literal tokens and `:64-81`
searches each **line** for them. A brace-group import never produces `std::fs`, and
`std::io` — which is I/O — is not in the list at all.

**证据 `[实测]`（我复现 + 委派审计独立复现）**：在 `/tmp/nla2` 副本里加入
`core/src/registry_core/zz_audit_probe.rs`：

```rust
use std::{env, fs};
pub fn probe() -> String {
    let _ = fs::read_to_string("/etc/hostname");
    let _ = env::var("HOME");
    let _ = std::io::stdout();
    String::new()
}
```

`CARGO_TARGET_DIR=/tmp/nla2-target cargo test -p xirang-conventions --offline` →
**10 passed, 0 failed**。对照实验（委派审计做）：一行 `std::fs::read_to_string(...)` →
`the_kernel_does_no_io_and_reads_no_environment` FAILED。

**最小修法**：禁用表补 `std::io`（以及 `std::thread`/`std::os`），并在扫描前把
`use std::{a, b};` 规范化成逐条路径（或像 `nesting.rs` 那样直接量 token 流）。
**钉子**：无——纯净性门禁没有任何反例测试。

### 3. A second `include!` evades the mounting gate by delimiter
### 3. 第二个 `include!` 换个定界符就能绕过挂载门禁

`Angle: gates`. `conventions/src/mounting.rs:58` records an `include!` only when the
left-trimmed line **begins with the exact bytes** `include!(`, and the test at `:88-94`
asserts the total **count is 1** — so a splice the scanner cannot see leaves the count at 1.

**证据 `[实测]`（我复现 + 委派审计独立复现）**：`cli/zz_audit_brace.rs`：

```rust
pub mod spliced {
    include! {"zz_body.rs"}
}
```

→ `modules_are_mounted_without_splicing_identity` **ok**。同一条单行写法
`pub mod spliced { include!("zz_body.rs"); }` 也一样。`rustfmt` 不会改写花括号形式，而 rustc
自己的诊断建议的正是它。对照：行首为 `include!("zz_body.rs");` → FAILED。

**最小修法**：在整行里找 `include` 后跟 `!` 的 token（接受 `()`/`[]`/`{}` 三种定界符），保留
"必须有一处是 `generated_lib.rs`"的断言，但不再依赖总数。
**钉子**：无。

### 4. An unterminated Rust fence is never parsed
### 4. 未闭合的 Rust 围栏从不被解析

`Angle: gates`. `conventions/src/doc_blocks.rs:124-165` 的 `markdown_findings` 在文件结束时把
仍然开着的围栏丢掉；`doc_comment_findings` 则在 `:246` 正确收尾——是遗漏，不是设计。

**证据 `[实测]`（委派审计，代码我已复核）**：向 `README.md` 追加
`` <!-- audit probe -->\n```rust\npub struct Broken {\n ``（无闭合围栏）→
`documented_rust_blocks_parse` **ok**；同样的文本补上闭合围栏 → FAILED。

**最小修法**：循环结束后把仍然开着的围栏作为"未闭合"上报，并在 `a_broken_block_is_reported`
旁边加一条用例。**钉子**：无。

### 5. `#[allow(missing_docs)]` survives the lint gate once rustfmt formats it
### 5. `#[allow(missing_docs)]` 一旦被 rustfmt 折行就能通过 lint 门禁

`Angle: gates`. `conventions/src/lint.rs:80-95` requires `#[allow(` and `missing_docs` on the
**same physical line**, while rustfmt itself splits a long attribute list. The item then
ships undocumented and clippy cannot see it, because the attribute removes the lint.

**证据 `[实测]`（委派审计，代码我已复核）**：

```rust
#[allow(
    missing_docs
)]
pub struct Undocumented;
```

→ `no_item_silences_the_lint` **ok**；写成单行 → FAILED。

**最小修法**：把非注释文本去掉空白后再搜 `allow(`…`missing_docs`，或跨行跟踪属性。
**钉子**：无。

### 6. `workflow_dispatch` + `publish: true` publishes with no tag or version check
### 6. `workflow_dispatch` 加 `publish: true` 会在没有 tag、没有版本检查的情况下发布

`Angle: release/CI`. `release.yml:46` 的 `if: startsWith(github.ref, 'refs/tags/')` 只守着 tag
那一步；上传步骤 `:80` 的条件是 `github.event_name == 'push' || inputs.publish`，而
`inputs.publish` 由使用者填写（`:11-16`）。于是在分支上手动触发并勾选 publish 就会上传清单里
写着的任何版本——正是 R2 称为"本工作流唯一硬性前提"的那一件事。同时
`CHANGELOG.md:103` 与 `:669` 都写着 `workflow_dispatch` 只跑检查、不发布，两句都不成立。

**证据 `[代码]`（我复核）**：条件里没有 `startsWith(github.ref, 'refs/tags/')`，而 tag 检查那一步
单独用该条件守着。

**最小修法**：删掉 `publish` 输入，让手动触发只跑检查（于是那两句 CHANGELOG 变成真的）——这也
与 release.yml 自己"tag 是唯一入口"的说明一致。首次发布本来就该先打 tag 再 push。
**钉子**：无。

### 7. `--publish --yes` is not idempotent and reports a correct release as failed
### 7. `--publish --yes` 不是幂等的，会把一次正确的发布报成失败

`Angle: release/CI`. `tools/xirang-publish:428-435` 只检查依赖是否已发布，从不检查**这个
crate 自己**；`$failed` 是粘性的，既参与层级等待又参与 `published` 记账（`:450,463`），最后在
`:482-484` 以 1 退出。因此在中途失败（包括 `:454-457` 自己产生的 300 秒 index 等待超时）之后
重跑，`cargo publish` 会以 registry 的"已上传"400 失败，脚本把一次**已经正确完成的发布**报成
`failed:`，而且没有脚本级的续跑方式。

**证据 `[代码]`**：委派审计未能真正上传（无 token、且被要求不传 `--publish`、cargo 限 `--offline`），
因此 crate 侧的"已上传"响应是文档化的 registry 行为而非实测。

**已修，并改用假 index 实测**：发布前先 `is_published "$crate" "$workspace_version"`，已在 index 上
就按已发布记账并跳过；`published` 改为按本层真正送上 index 的 crate（`level_published`）记账，
不再受全局 `$failed` 影响，等待可见也只等这些。对照实验（让 index 声称九个 crate 的 `0.1.0` 全在，
不触碰 crates.io）：

- 修复前：退出 1，对九个 crate 逐个 `cargo publish`（各自 `no token found`），摘要
  `verified: none` / `failed:` 九个——把一次**已经全在 index 上**的发布报成全坏；
- 修复后：退出 0，九行 `already on the index`，**零次上传尝试**，`verified:` 九个 / `failed: none`。

**钉子**：`tools/xirang-publish` 没有测试框架，因此这条的钉子就是上面那份可重跑的对照实验；
P4 是它在真实 registry 上的对应场景。

**最小修法**：发布前先 `if is_published "$crate" "$workspace_version"; then published="$published $crate"; continue; fi`；
把 `published` 改成按 crate（而非按层、且受全局 `$failed` 影响）记账。
**钉子**：无。

### 8. `function_source_range` counted braces in strings and comments
### 8. `function_source_range` 会数字符串与注释里的花括号

`Angle: kernel`. `core/src/registry_core/source/source.rs:41-64` 按原始行数字符；`let s = "{";`
因此提前闭合（或让范围永不闭合，而旧兜底会报成单行函数）。消费者：Studio 的源码预览/摘录
（`graph_queries.rs:24`、`ui/search/detail.rs:88,94`）。

**证据 `[实测]`（委派审计）**：`["fn first() {", "    let s = \"{\";", "    other();", "}", ""]`
→ `Some((0,0))`，正确答案是 `Some((0,3))`。

**已修**：改为在 `mask_non_code` 后的文本上计数，且末尾未配平时返回 `None`（而不是假装单行）。
**钉子**：`source_tests.rs::a_brace_in_a_string_does_not_close_the_function`。

### 9. A graft selector of `..` writes outside `.xirang/external-grafts/` and never loads
### 9. 选择器 `..` 会把计划写到 `.xirang/external-grafts/` 之外，而且永远不会被加载

`Angle: surfaces`. `core/src/registry_core/plugin/graft/document.rs:202-219` 只拒绝 `/`、`\`、
空白与 `"`，不拒绝 `.`/`..`；`run_method/.../external_graft/plan.rs:156-171` 直接
`join(selector)` 并写入。

**证据 `[实测]`（委派审计）**：`create_external_graft(&registry, target, "..", false)` 返回 Ok，
计划落在 `<pkg>/.xirang/graft.plan`，而 `list_external_grafts()` 不列出它（`load_graft_records`
跳过非目录）——运行期永远不会应用这份计划，CLI/Studio 与运行期对同一个包给出不同答案。Studio
侧可达：`validate_graft_selector` 接受 `..`。

**已修**：拒绝加在**共用规则**那一层——`core::...::document::validate_graft_selector`（写入方、
解析方与 Studio 都经它，Studio 用的是 `xirang_run_method::validate_graft_selector` 重导出），
条件是"以 `.` 开头"，因此 `.`、`..`、`.hidden` 一并被拒，而普通名字照常通过。**钉子**：
`document.rs::selectors_that_could_leave_the_record_directory_are_refused`（core）与
`plan_tests.rs::a_selector_that_escapes_the_record_directory_creates_nothing`（写入路径；修复前
那条钉子的失败信息逐字复现了审计的 `selector: ".."`、root `…/external-grafts/..`）。两条都用
"把规则去掉"的方式验证过会红。

### 10. `xirang explain` serves stale build output as `known: true`
### 10. `xirang explain` 把过期的构建产物当作 `known: true` 提供

`Angle: surfaces`. `cli/explain.rs:93-96` → `cli/explain_report.rs:29-127` 直接读
`out/source_scope.tsv` 与 `pruning_manifest.tsv`（`build_method/scope_view.rs:100-135`），
没有任何新鲜度检查，而 `faces` 是实时重算的。

**证据 `[实测]`（委派审计）**：同一棵树里先 `check`（无 cut）→ `explain` 报 `all:true kept:true
known:true`；然后只改 `src/lib.rs` 加上一个 `cut`，**不**重跑 check 再 `explain` → 仍然是
`known:true, reason:conservative-fallback`；再 `check` 后才变成 `all:false kept:false`。
更糟的是**失败**的 `check`（退出 1）也会把整套输出写进 `out/`（`pipeline.rs:107-130` 不区分
`compile_errors`），随后的 `explain` 会把它当作 `known:true`。

**已修（两半）**：①`build_method` 新增公开的 `build_output_is_current(root, out_dir)`：把
`out/discovery.fingerprint` 与**按当前源码重算**的指纹比对（复用构建自己的
`discovery_fingerprint`，不是第二套哈希），`explain` 与 `--overlay` 都先问它，过期就沿用既有的
"先跑 `xirang check`"提示报 `known:false`；②pipeline 只在**没有编译错误**时写这枚指纹，失败时
把它删掉——因此一次**失败**的 check 不再留下任何被当作现状的产物（其余清单留在原处，没有那枚凭据
就没有东西会把它们读作"描述了当前源码"）。**钉子**：
`build_method::scope_view::published_output_is_current_until_the_sources_change` 与
`::a_failed_check_publishes_no_trusted_output`，以及 CLI 端到端的
`lib_tests.rs::explain_reports_an_unknown_scope_when_the_build_output_is_stale`（先 check → 改一行
源码 → `explain --json` 必须 `scope.known=false` 且 `pruning.known=false`，重新 check 后恢复
`true`）。把 `build_output_is_current` 中和成恒 `true`、或去掉失败时的删除，三条都会红（已验证并
还原字节一致）。

### 11. Passive element segments escape `WasmLimits::table_elements`
### 11. 被动元素段绕过 `WasmLimits::table_elements`

`Angle: plugin`。`StoreLimits::table_elements` 只喂给 `table_growing`；被动/声明式
`(elem func $f …)` 由 `wasmi` 在实例化时物化（`initialize_table_elements` →
`builder.push_element_segment`），从不经过 limiter。紧凑 funcidx 编码是每项 1 字节，因此唯一的
约束是工件字节上限，倍率约 32×。

**证据 `[实测]`（委派审计）**：2 000 000 项的被动段（模块 2 000 103 B）在默认上限下
`status=Ok("LOADED")`，`peak_delta≈64 070 402 B`（32.0×）；50 万项的被动段配 `(table 1 funcref)`
同样能加载。因此默认 `max_module_bytes = 16 MiB` 每个槽位可换来约 512 MiB 宿主分配。

**已修**：`WasmLimits` 新增 `max_element_bytes`（默认 256 KiB），在 `Module::new` **之前**从二进制的
段头量出元素段负载总量（只走 `id`+LEB128 长度+负载；读不懂的二进制返回 0 并交给引擎报"模块损坏"，
因为引擎的诊断比预算拒绝更有用）。该字段的文档写明它同时是**分配**预算：实测每个条目约 32 字节、
紧凑编码每条约一字节，因此 256 KiB 约等于 8 MiB 上限；`EnforcedLimits::strict()` 只限制元素**段**的
数量，不限制条目。**钉子**：`fault_matrix.rs::a_large_passive_element_segment_is_refused_before_instantiation`
（40 万条目的被动段，默认限额下必须被拒；修复前它**加载并运行成功**——`[111, 107]` 即 "ok"，
逐字复现审计的探针）与对照 `::a_small_passive_element_segment_still_loads`。把检查中和成
`if false && …`，第一条立刻变红（已验证并还原字节一致）。

### 12. `verify_signed` records `Signature` without calling the verifier for non-Official sources
### 12. `verify_signed` 对非 Official 来源记录了 `Signature`，却没有调用验证器

`Angle: plugin`. `core/.../plugin/artifact/artifact.rs:109-131` + `.../trust/trust.rs:134-135`：
`verify_with` 在 `source != Official` 时直接返回 `Ok(())`。

**证据 `[实测]`（委派审计）**：`Ok assurance=Signature verifier_called=false policy=open`。今天不构成
通道绕过（`slot.rs:93-94` 另行要求 `source == Official`），但该字段的文档写着它是"实际执行过的
最强检查"，而对每个 User 工件它都是假的。

**已修**：`verify_signed` 现在要求 `source == Official`，否则返回新增的
`PluginTrustError::SignatureLaneRequired`——显式拒绝而不是静默降级，因为调用方请求的是本通道做不到
的签名检查，而该字段的文档写的是"实际执行过的最强检查"；同一个工件的纯摘要路径仍然可用。
**钉子**：`artifact.rs::signature_assurance_is_refused_for_a_user_artifact`（用户来源 +
`PluginTrustPolicy::open()` → `SignatureLaneRequired`，而 `verify_artifact` 仍返回 `Digest` 保证；
把拒绝中和成 `if false && …` 即变红，已验证并还原）。

### 13. `application!(entry = …)` is refused for the canonical folder layout
### 13. `application!(entry = …)` 对规范的目录布局会被拒

`Angle: build/macro`. `build_method/entry.rs:509-523` 的 `entry_path_exists` 只检查第一段
路径，从不试发现与渲染器使用的 `<name>/<name>.rs` 形式，因此连文档里写的 `crate::app::run` 在
`app/app.rs` 布局下都会被拒，而扁平模块下一个不存在的尾巴反而通过。

**证据 `[实测]`（委派审计）**：`/tmp/audit/app1`（`application!(entry = crate::control)` 写在
`src/control/control.rs`，生成的树确实声明了 `pub mod control`）：`xirang check` → 退出 1，
`phase=entry source=control/control.rs:34 ... does not resolve to a source module under the
package root`。

**修法**：把每一段都对已发现的树解析（接受 `dir/dir.rs`），或删掉这条其声明路径从不被使用的检查。
**钉子**：现有测试只钉住扁平的 `crate::app::run` 写法。

## MINOR / 次要

- **8. 尺寸棘轮的作用域是按文件名与位置的** `Angle: gates` — `conventions/src/size.rs:66-78,82-97`：
  名为 `*_tests.rs` 的文件无论多少行都豁免（500 行的 `zz_audit_helpers_tests.rs` 通过，同样大小的
  `zz_audit_plain.rs` 失败）；只遍历 `*/src`，因此 600 行的 `cli/build.rs` 不被度量；
  `crate_directories` 只找到深度 ≤2 的 `src/`。对已在 BASELINE 里的文件，改名逃逸会被"陈旧条目"
  那颗牙咬住——那一半是好的。修法：豁免只按 `tests/` 目录或 `cfg(test)` 内容判定，并把 `build.rs`
  与 `benches` 纳入。`[实测]`（委派审计）
- **9. 两处 crate 列表是硬编码的，新增已发布 crate 无人管** `Angle: gates` —
  `conventions/src/lint.rs:28-39` 与 `tools/xirang-package-audit:68-78`。`[实测]`：删掉
  `conventions/src/lib.rs` 的 `#![warn(missing_docs)]` 后十项测试仍全绿；加一个第十个已发布成员
  `zznew` 后包审计仍打印 `contents checked: all nine crates` 并退出 0（`--check-table` 反而抓到了）。
  修法：两处都从工作区成员减去 `publish = false` 推导，与 `--check-table` 一致。
- **10. `--check-table` 读不了工作区继承的依赖，且提示语在劝人关掉自己** `Angle: release/CI` —
  `tools/xirang-publish:339-358`。`[实测]`（委派审计）：把内部依赖改成
  `[workspace.dependencies]` + `x.workspace = true` 后 `--check-table` 退出 1，消息只报不一致、
  不给修法，而唯一按字面能让它变绿的做法是从手工表里删掉那条边——那会关掉这条门禁存在的理由。
  修法：解析 `x.workspace = true` 并在消息里说明。
- **11. 纯净性是唯一没有"走过文件"下限的门禁** `Angle: gates` — `purity.rs:92-100`、
  `lib.rs:117-129`：`rust_sources` 在 `read_dir` 失败时返回空表，`findings` 不断言看过任何东西。
  内核目录一改名，纯净性就在"什么都没查"的情况下通过。修法：像
  `core/tests/nesting_budget.rs:70-75` 那样断言 `rust_sources(core/src).len() > N`。`[实测]`（委派审计）
- **12. 嵌套守卫报错了上限，且三种形状里有一种不可达** `Angle: gates` —
  `core/.../syntax/nesting.rs:121-130,152,188-201`：`TooDeep::Display` 对所有形状都印 `LIMIT`(128)，
  而 Chain 的上限是 `CHAIN_LIMIT`(1024)。`[实测]`：`guard_nesting("Vec<"×2000)` →
  "…above the limit of 128"，真正起作用的 1024 不在消息里。另外 `arguments` 每遇到 `Ident` 就清零，
  因此 `Shape::Arguments` 永远不会触发（`Vec<Vec<…>>` 实际由 Chain 拦下，这也解释了
  `deeply_nested_angle_brackets_are_refused` 为什么通过）。修法：把每形状的上限放进 `TooDeep`；
  让泛型计数不被标识符清零，或删掉该形状。委派审计另声明**找不到第四种未被量到的 aborts 形状**，
  即 M1 的修法本身看起来是完整的。
- **13. 文档门禁覆盖的文件列表是固定的** `Angle: gates` — `conventions/src/doc_blocks.rs:71-89`：
  `docs/` 只读一层，且只覆盖根与各 crate 的 `README(.zh-CN).md`。`[实测]`（委派审计）：
  `docs/zzsub/zz_audit.md` 或 `CHANGELOG.md` 里的坏围栏通过，同样的坏围栏放进 `README.md` 才失败。
  修法：递归遍历 `docs/`，或把排除写明白。
- **14. 随包发布的 `core` 测试并不自足，而发布路径看不见** `Angle: release/CI` —
  `[实测]`（委派审计）解包 `xirang-core-0.1.0.crate` 后跑
  `cargo test --features syntax --test nesting_budget`：`the walk found 83 Rust files under /tmp/unpack/iso`。
  `core/tests/nesting_budget.rs:59-60` 走的是 `CARGO_MANIFEST_DIR/..`，在真实 registry 解包目录里
  那是**别人的 crate**，而 `>100` 的下限还会通过。修法：把工作区根改成经环境变量传入，或把该测试
  移到 `publish = false` 的成员里。
- **15. `readelf` 缺失时 `.inventory` 断言被静默跳过** `Angle: release/CI` —
  `tools/xirang-release-audit:31-35` 把整条检查包在 `if command -v readelf …` 里；没有 binutils
  时脚本仍退出 0，而 README 与 `docs/performance-baseline.md` 把这条链接段检查当作"与机器无关"的
  发布承诺。修法：缺 `readelf` 直接报错退出。
- **16. `tools/xirang-visual` 用未校验的场景名写文件** `Angle: release/CI` —
  `tools/xirang-visual:156-157` 直接拼 `"$OUT_DIR/$name.txt"`，`$name` 来自 `$@`；`[实测]`
  的算术结果是 `target/visual/../../escaped.txt`。另外 `scene_marker` 的默认值使拼错的场景名也
  能驱动首页并产出非空 capture，CI 的 `test -s` 因此抓不到。修法：只接受 `scenes()` 里的名字。
- **17. 外部演练用 GNU 专有的 `sed -i`** `Angle: release/CI` —
  `tools/xirang-external-rehearsal:64`；BSD/macOS 的 sed 不接受该写法。CI 那一步只在 Linux 跑，
  因此 CI 安全，但工具自称维护者随处可跑。修法：`sed … > "$m.new" && mv`。
- **18. 无 git 时脏树前置检查静默失效** `Angle: release/CI` — `[实测]`（委派审计）在只有 `sh`
  没有 `git` 的 PATH 下复现：`git: command not found` 之后脚本继续，`--publish` 会照发。
  修法：取不到 `git status` 就报错退出。
- **19. `--verify-consumers` 把所有 `cargo add` 失败都报成"尚未发布"** `Angle: release/CI` —
  `tools/xirang-publish:264-282`。R1 记录的那次失败路径证据正是 `CARGO_NET_OFFLINE=true`，
  也就是把联通性/离线失败报成了发布缺口。修法：先用 curl 探 index 可达性，消息里点名离线模式。
- **20. CHANGELOG 的 LICENSE 断言与工作树不符** `Angle: release/CI` — `[实测]`（委派审计）：
  `CHANGELOG.md:34` 说每个 crate 目录都有 LICENSE 副本，而 `conventions/`（唯一没有它的成员）
  不成立；九个已发布 crate 的包里都有。修法：补一份或把句子收窄为"每个已发布 crate 目录"。

- **21. `check --json` 在诊断之前的失败上违反自己的 stdout 契约** `Angle: surfaces` —
  `cli/commands/check.rs:40-41` 先解析包、后进 `--json` 分支。`[实测]`（委派审计）：
  `xirang check --json /nonexistent-xyz` → 退出 1、stdout 为空、stderr 一行
  `xirang: cannot resolve …`；而 `cli/README.md:40-43` 承诺失败也把同一份文档写到 stdout。
  修法：解析失败也发一份带单条诊断的文档，或收窄 README。
  **已修（2026-09-26）:** `check --json` 现在先写出 `xirang.build-diagnostics/1` 文档（含一条
  `phase=resolve` 诊断）再返回错误；`grafts --json` 与 `explain --json`/`--overlay --json` 同轮
  一并修掉。钉子在 `cli::check_json_reports_a_resolution_failure_as_json`、
  `::grafts_json_reports_a_resolution_failure_as_json`、`::explain_json_reports_a_resolution_failure_as_json`。
- **22. `xirang new --path <不存在>` 报成功并写出不可用的清单** `Angle: surfaces` —
  `cli/commands/new.rs:25-29` 把 `canonicalize` 的失败吞掉并写回原始字符串。`[实测]`：
  `--path /nonexistent-checkout broken` → 退出 0、`created binary project at …`，而清单里写着
  `path = "/nonexistent-checkout/run_method"`；`--git ""` 同样退出 0。修法：要求 `--path` 能
  canonicalize 到一个含 `core/` + `run_method/` 的目录，并要求 `--git` 非空。
  **已修（2026-09-26）:** `new` 在写任何东西之前先解析 `--path` 并要求它是一个 XiRang 检出
  （`cli/commands/new.rs`，判定放在 CLI 而不是 `build_method`，因为打包后的 CLI 要对着已
  发布的核心编译）。钉子 `new_command::tests::only_a_real_checkout_is_accepted`，外加实测：
  `--path /tmp` 报 "is not a XiRang checkout" 且不创建目录。
- **23. MCP 的 JSON-RPC 收尾三处不合规** `Angle: surfaces` — `mcp/protocol.rs:96-116`。
  `[实测]`：`{"method":"some/notification"}`（无 id）被**回复**了；`{"id":5,"method":
  "notifications/initialized"}` 反而**没有**任何输出（客户端会一直等）；缺 `"jsonrpc"` 的请求被
  正常处理。修法：无 id 即通知、绝不回复；有 id 必回复；校验 `"jsonrpc":"2.0"`。
- **24. MCP 每行请求没有大小上限** `Angle: surfaces` — `mcp/protocol.rs:74-78` 的
  `input.lines()` 会先把一整行读进内存（`MAX_READ_LINES` 只管响应）。`[代码]`。修法：有界读取
  （超过几 MiB 直接 `-32600`）。
- **25. Studio 不回收启动的编辑器子进程** `Angle: surfaces` — `studio/studio/terminal.rs:44-50`
  `.spawn().map(|_| editor_name)` 立刻丢掉 `Child`（`Drop` 不 `wait`），因此像 `wezterm start`
  这种立刻返回的启动器会留下僵尸。`[代码]`（委派审计未能驱动到编辑路径，未复现）。修法：用等待
  线程或事件循环里的 `try_wait` 回收。
- **26. 安装可以发布比它确认的更旧的一代** `Angle: plugin` — `plugin-host/src/lazy_wasm/slot_state.rs:45`：
  `fetch_add` 在 `pending` 锁之前，所以并发安装中后拿到锁的那个（可能是更低的一代）获胜，
  `install` 返回 N 而激活发布 N-1。`[代码]`。修法：在锁内分配/赋值，或只在
  `generation > pending.generation` 时覆盖。
  **已修（2026-09-26）:** 代际改为在 `pending` 锁内分配。钉子
  `fault_matrix::the_highest_generation_is_the_one_that_stays_pending`（8 线程 × 64 轮，每轮用
  屏障让编号顺序与取锁顺序独立）；旧顺序下第 0 轮即红：pending 6，而发出的最大代际是 8。
- **27. 激活失败对就绪轮询不可见，旧代码继续服务** `Angle: plugin` — `slot_state.rs:71-80`、
  `lazy_wasm.rs:148-152`。`[实测]`（委派审计）：`install bad -> Ok(2)`，首次 `call` 报
  `Health(…)`，随后 `is_loaded=Ok(true) generation=Ok(Some(1))`，再次 `call` 返回**旧的**
  `Ok([122,122])`。修法：把激活错误留在槽状态里，或让 `is_loaded` 对比已安装代际。
  **已修（2026-09-26）:** 槽状态记下最近一次激活失败，`WasmPluginTable::activation_error`
  报告它，成功的激活清除它（`lazy_wasm.rs`、`lazy_wasm/slot_state.rs`）。钉子
  `fault_matrix::a_failed_activation_is_observable`；关掉记录即红。
- **28. 进程通道没有工件字节上限** `Angle: plugin` — `plugin-host/src/process.rs:108-120`：
  `ProcessLimits` 只有超时/输入/输出，`load` 先 `fs::read` 整个可执行文件再比较，还多留一份副本。
  `[代码]`。修法：先按 `metadata.len()` 与声明上限比较。
  **已修（2026-09-26）:** `load` 先比对 `metadata.len()`，一致才读。钉子
  `process_load_cost::a_wrong_length_executable_is_refused_without_reading_it`（计数分配器实测：
  修前一个 256 MiB 长度不符的文件峰值分配 268 435 478 字节，修后不足 1 MiB）。
- **29. `call_tree` 的泳道分配按树深递归，`limit`/`depth` 由调用方给** `Angle: kernel` —
  `core/.../mir/call_tree.rs:359-405`。树内调用方都传 `CALL_TREE_NODES=16`，因此是潜伏的公开 API
  风险而非现实崩溃。修法：显式栈的迭代后序。

- **30. 源码根缺失/不可读时 panic** `Angle: build/macro` — `build_method/discovery.rs:51-52`
  的 `.expect("src directory must exist")`（`:80-81` 同）会让 `[lib] path` 在 `src/` 之外的包以
  退出 101、`--json` stdout 为空收场，而不是给出 `out-dir`/`entry` 诊断。`[代码]`。修法：把
  `read_dir(src)` 的错误报成诊断。

### 14. The README's face examples use three fields the macro rejects
### 14. README 的注册面示例用了三个宏已经拒绝的字段

`Angle: docs`. `README.md:196,202-203,365`（以及 `README.zh-CN.md:180,186-187,342`）里的示例写着
`handle:`、`expected_output:`、`actual_output:`；`FACE_FIELD_ORDER`
（`core/.../declaration/registration.rs:426`）三者都没有，`macro/src/lib.rs:305-318` 对表外的键直接
报 `unknown face field \`X\``，而 `run_method/macros/face_objects.rs:37-58` 的
`__control_object!` 分支不绑定 `handle:`。也就是说**照 README 抄会编译不过**。`run_method/tests/
face_preset_parts.rs:1-2` 已写明"No `handle:` field"是当前状态。

**已修**：中英两侧的这四处字段都已删除（`README.md` 与 `README.zh-CN.md`），文档门禁随后
重跑通过（围栏仍能解析，但门禁展开不了宏，所以这条只能靠人读）。**钉子**：无——文档门禁只让
`syn` 解析围栏，展开不了宏；这正是它没能提前发现的原因。

### 15. `docs/migration.md` promises a Compare page that does not exist
### 15. `docs/migration.md` 承诺了一个不存在的 Compare 页

`Angle: docs`. `docs/migration.md:156`（`docs/migration.zh-CN.md:71`；同样说法在
`docs/ROADMAP.md:13`）写着用 `1` 到 `4` 选择 Search / Inspect / Data / **Compare**；而
`studio/studio/app/state/pages.rs:19-29` 只有 Search/Inspect/Data，`keyboard.rs:18-20` 也只映射
1/2/3。**已修**：`docs/migration.md`、`docs/migration.zh-CN.md`、`docs/ROADMAP.md` 与
`docs/ROADMAP.zh-CN.md` 现在都写 Search/Inspect/Data 三项（`1`～`3`），Compare/对比已删。
**钉子**：无。

### 16. `run_method/README.md` names a `TraceMode::from_env` that does not exist
### 16. `run_method/README.md` 点名了并不存在的 `TraceMode::from_env`

`Angle: docs`. `run_method/README.md:12`（zh:11）说 `TraceMode::from_env` 读 `XIRANG_TRACE`；
代码是自由函数 `trace_mode_from_env()`（`run_method/runtime/trace/call_trace.rs:30`），
`TraceMode` 本身只有一个 `parse`（`core/.../declaration/call_evidence.rs:121,134`）。
**修法**：文档改名，或补一个关联函数。**钉子**：无。
- **31. 文档里过期/缺失的清单** `Angle: docs` — `run_method/README.md:56-76` 约 12 个
  file:line 锚点里 7 个已漂移（`graft_ops.rs` 146→149、186→189、54→55、294→297；
  `inspection.rs` 75→76；`connector.rs` 205→206、211→212；`transaction.rs` 83→84；
  `reconcile.rs` 166→148/202）；`cli/README.md:13-22` 的命令表漏了 `xirang snippets`
  （`--help` 与 `README.md:858` 都有）；`core/README.md:9-24`、`README.md:848-850` 与
  `AGENTS.md:104-116` 的模块清单漏了 `json`（`core/src/registry_core.rs:19-20`）；
  `AGENTS.md:83` 的"树里有 29 处裸 `mod x;`"实测是 196 处（十个 `*/src` 树里），其中 155 处位于
  该句认可的位置、40 处直接在 crate 根；`CHANGELOG.md:34` 的"每个 crate 目录都有 LICENSE 副本"
  对 `conventions/` 不成立（它就是 open record #20）。修法：重新推导或删掉这些数字/锚点。

### 低严重度备注（无复现，读代码得出）
### Lower-severity notes (code-read, no reproduction)

- `conventions/src/lib.rs:106-107` 自称遍历不跟随符号链接，但 `path.is_dir()` 会跟随；委派审计
  实测把 `core/src/.../zz_audit_link` 指向 `/tmp/extdir` 就让纯净性去报了检出之外的
  `/tmp/extdir/evil.rs`（自指链接不会溢出——内核的 ELOOP 拦住 `read_dir`），属卫生问题。
- `mounting`/`lint` 遍历整个成员目录，因此 `examples/control-button/target/xirang/out/*.rs`
  这类构建产物也被扫描；委派审计实测在那里放一个含 `include!(…)` 的 `.rs` 会让挂载门禁失败
  （目前是潜伏的：`build_method/renderer/pass.rs:311` 断言渲染器不会发出 `include!`）。
- `is_comment`（`lib.rs:156-159`）只跳过 `//` 行，因此 `/* … std::fs … */` 块注释是**误报**。
- `core/tests/nesting_budget.rs:79-81` 对读不到的文件静默 `continue`，`>100` 的下限是唯一保护。
- `AGENTS.md` 里没有任何门禁的规则：规则 4 的 crate/目录/lib 名一致性（`[lib] name` 无处检查）、
  "新增纯逻辑放在 `core/src/registry_core/<name>/` 并在 `core/src/registry_core.rs` 注册"、
  执行面的 shim 路径、以及"先英文 `///` 再中文 `///`"的文档约定。

## Gate hardening landed in round 3 / 第 3 轮落地的门禁加固

The nine findings the audit filed against the gates themselves. Each was pinned by a
test or a mutant copy that read as "clean" before the fix — the pins for a gate are
violations it has to catch, and they are listed with the item. **The `最小修法`/`钉子`
lines on those ten rows above describe the state before this work; this section is
their status.**
审计针对门禁自身提出的九条。每一条都用"修复前读作干净"的测试或变异副本钉住——门禁的钉子就是
它必须抓到的违规，随条目列出。**上面那十行自己的"最小修法/钉子"写的是本轮之前的状态；它们的
状态以本节为准。**

- **Purity bypasses and `std::io` (MAJOR 2)** — `FORBIDDEN` gains `std::io`,
  `std::thread` and `std::os`, and a line is expanded before the search so
  `use std::{env, fs};` names both modules. Pin:
  `purity::a_brace_import_and_std_io_are_violations` (a synthetic `core/src` with the
  four-line probe), red before the fix.
- **Purity has no walk floor (item 11)** — `purity::the_walk_covers_the_kernel_tree`
  asserts the walk sees more than 40 files, so a renamed kernel directory cannot read
  as clean.
- **`include!` by delimiter (MAJOR 3)** — the mount search runs on the kernel's
  `mask_non_code` text (the workspace's one rule for "what is code") and accepts any
  delimiter, so `include! {"…"}` and the one-line `pub mod spliced { include!("…"); }`
  are both seen, while a fixture *string* that mentions it is not. Pin:
  `mounting::an_include_with_another_delimiter_is_still_a_splice`.
- **Unterminated fence (MAJOR 4)** — `markdown_findings` flushes an open block at end
  of file and reports it. Pin: `doc_blocks::an_unterminated_fence_is_reported`.
- **Folded `#[allow(…)]` (MAJOR 5)** — the lint scan runs on masked text and reads an
  attribute across lines, because rustfmt folds a long allow list itself. Pin:
  `lint::a_folded_allow_attribute_is_still_a_violation`. The same change stopped a
  fixture *string* mentioning the attribute from being reported, which the raw scan
  had done once a pin existed.
- **Hardcoded crate lists (item 9)** — `lint::required_roots` derives the roots from
  the tree (every crate directory's library root except the example hosts, plus the
  MCP binary root), and `tools/xirang-package-audit` derives the shipped set and each
  crate's internal dependencies from the manifests. Pins:
  `lint::a_new_crate_root_must_carry_the_attribute`, and a mutant workspace with a
  tenth published member, where the old tool printed "all nine crates" and never
  mentioned it while the new one audits it.
- **`--check-table` and workspace inheritance (item 10)** — an internal requirement
  spelled `xirang-x.workspace = true` is read as the dependency it names, and the
  root's `[workspace.dependencies]` versions are checked once. Mutant copies: the
  inherited spelling now passes (it used to exit 1 with a message that invited
  deleting the edge) and a drifted inherited version is refused by name.
- **Nesting limit and the unreachable shape (item 12)** — `TooDeep` carries the limit
  that applies to its shape, so a linear run reports 1024 and a delimiter group 128;
  `Shape::Arguments` is deleted because its counter reset on every identifier and could
  never fire — generic chains are linear runs, and
  `deeply_nested_angle_brackets_are_refused` still passes through the chain shape. Pin:
  `deep_input_tests::the_refusal_names_the_limit_that_applies`, red before the fix
  (it printed "1025 levels … above the limit of 128").
- **Doc-gate file list (item 13)** — every root-level markdown file and every markdown
  file under `docs/` at any depth is covered. Pin:
  `doc_blocks::a_fence_in_a_nested_docs_file_is_covered`.

One gate finding is **not** fixed and stays open: the size ratchet's scope (item 8) —
its test-only exemption is by file name, and it walks only `*/src`, so `build.rs` is
never measured and `crate_directories` sees nothing deeper than two levels.
未修、仍开放的门禁发现只有一条：尺寸棘轮的作用域（第 8 条）——它按文件名豁免测试文件，且只遍历
`*/src`，因此 `build.rs` 从不被度量，`crate_directories` 也看不到深度超过两层的 `src/`。
**已修（2026-09-26）:** 两条都不再成立。`conventions/src/size.rs` 现在也度量 crate 根的
`build.rs`，而"看起来像测试"的文件只有确实挂在 `#[cfg(test)]` 之后才豁免（钉子
`size::the_crate_root_build_script_is_measured`、`::an_unmounted_test_shaped_file_is_measured`、
`::a_mounted_test_file_is_exempt`）；`crate_directories` 改为从根清单的 `members` 推导，因此任意
深度的成员都被覆盖（`conventions::tests::a_deeply_nested_member_is_a_crate_directory`）。


## Fixes landed while this audit was running / 审核进行期间已落地的修复

Four CRITICALs and one MAJOR were fixed as soon as they were reproduced, each with a
test that was shown to go red with the fix reverted. Everything else in this document
is still open.
四条 CRITICAL 与一条 MAJOR 在复现后立即修掉，每条都配了"把修复撤掉即变红"的测试。本文其余
发现仍未处理。

| # | Finding | Fix | Pin (proven red) |
| --- | --- | --- | --- |
| 1 | member version invisible to the tag check and `--check-table` | `--check-table` now compares every shipped crate's declared version and every internal requirement against the workspace version (`tools/xirang-publish`) | `--check-table` against a mutant copy: `0.1.1` member and `0.2.0` requirement both refused |
| 2 | lifetimes hid every function from the source index | `mask_non_code` masks `'` only when a character literal closes (`core/src/registry_core/source/source.rs`) | `source_tests.rs::lifetimes_do_not_mask_the_code_that_follows_them` |
| 3 | Studio's Delete acted on the process CWD project | the call is wrapped in `with_authoring_context` (`studio/studio/app/overlay/delete.rs`) | `app/tests/edit.rs::delete_moves_the_module_of_the_open_project` |
| 4 | `g` panicked on an empty graft plan list | `checked_sub` instead of eager `then_some` (`studio/studio/ui/forms/graft.rs`) | `app/graft_tests.rs::the_graft_screen_renders_before_any_plan_exists` |
| 2/3/4/5 | gate bypasses: purity, `include!`, unterminated fence, folded allow | see the gate-hardening section (`conventions/src/{purity,mounting,doc_blocks,lint}.rs`, `core/.../source/source.rs` exposes the masker) | the pins named there, each red before its fix |
| 9/10/11/12/13 | gate scope and tool tables: hardcoded lists, `--check-table` inheritance, purity floor, nesting limits, doc-gate file list | derived sets, per-shape limits, recursive walk (`conventions/`, `core/.../syntax/nesting.rs`, `tools/{xirang-publish,xirang-package-audit}`) | the pins and mutant copies named there |
| 13 | `application!(entry = …)` refused the canonical layout | every segment is resolved against the tree, accepting `<dir>/<dir>.rs` (`build_method/entry_paths.rs`) | `entry_tests.rs::an_application_entry_resolves_every_segment_against_the_tree` + the end-to-end `check` before/after |
| 11 | a passive element segment escaped both ceilings | `WasmLimits::max_element_bytes`, measured from the section headers before compilation (`plugin-host/src/wasm.rs`) | `fault_matrix.rs::a_large_passive_element_segment_is_refused_before_instantiation` + `::a_small_passive_element_segment_still_loads` |
| 12 | `verify_signed` recorded `Signature` without a verifier | `SignatureLaneRequired` unless the source is Official (`core/.../plugin/{artifact,trust}`) | `artifact.rs::signature_assurance_is_refused_for_a_user_artifact` |
| 9 | a `..` selector wrote outside the record root | the shared `validate_graft_selector` refuses a leading dot (`core/.../graft/document.rs`) | `document.rs::selectors_that_could_leave_the_record_directory_are_refused` + `plan_tests.rs::a_selector_that_escapes_the_record_directory_creates_nothing` |
| 10 | `explain` served stale output as `known: true` | `build_output_is_current` + the fingerprint is written only by a clean run (`build_method/{scope_view,pipeline}.rs`, `cli/{explain,explain_report,explain_overlay}.rs`) | `scope_view::published_output_is_current_until_the_sources_change`, `::a_failed_check_publishes_no_trusted_output`, `lib_tests.rs::explain_reports_an_unknown_scope_when_the_build_output_is_stale` |
| 6 | a malformed or gated `static_graft_plan!` panicked | diagnostics threaded through `graft_view::query` (three refusal kinds) | `query.rs::a_malformed_graft_declaration_is_reported_not_fatal`, `::an_unevaluable_graft_gate_is_reported_not_fatal` |
| 5 | macro bodies hid the faces they name | macro tokens are read for `::` paths instead of the scope being widened (`core/src/registry_core/syntax/reference_scan.rs`) | `face.rs::a_macro_invocation_is_read_for_the_paths_it_spells` + the end-to-end table above |
| 8 | `function_source_range` counted braces in strings and comments | counts over masked text, `None` when unbalanced (`core/src/registry_core/source/source.rs`) | `source_tests.rs::a_brace_in_a_string_does_not_close_the_function` |
| 14 | README face examples used removed fields | the four lines are gone from both READMEs | none (the doc gate cannot expand a macro) |
| 15 | `docs/migration.md` promised a Compare page | Search/Inspect/Data only, in four documents | none |

| # | 发现 | 修复 | 钉子（已验证会红） |
| --- | --- | --- | --- |
| 1 | 成员版本对 tag 检查与 `--check-table` 不可见 | `--check-table` 现在把每个已发布 crate 声明的版本与每处内部需求同工作区版本比对（`tools/xirang-publish`） | 用漂移副本跑 `--check-table`：`0.1.1` 的成员与 `0.2.0` 的需求都被拒 |
| 2 | 生命周期让源码索引里的函数整体消失 | `mask_non_code` 只在字符字面量闭合时才把 `'` 当引号（`core/src/registry_core/source/source.rs`） | `source_tests.rs::lifetimes_do_not_mask_the_code_that_follows_them` |
| 3 | Studio 的删除作用在进程 CWD 的项目上 | 该调用包进 `with_authoring_context`（`studio/studio/app/overlay/delete.rs`） | `app/tests/edit.rs::delete_moves_the_module_of_the_open_project` |
| 4 | 空计划列表上按 `g` 会 panic | 用 `checked_sub` 取代会立即求值的 `then_some`（`studio/studio/ui/forms/graft.rs`） | `app/graft_tests.rs::the_graft_screen_renders_before_any_plan_exists` |
| 13 | `application!(entry = …)` 拒绝规范布局 | 每一段都对目录树解析，接受 `<dir>/<dir>.rs`（`build_method/entry_paths.rs`） | `entry_tests.rs::an_application_entry_resolves_every_segment_against_the_tree` + 端到端 `check` 前后对照 |
| 11 | 被动元素段绕过两道上限 | `WasmLimits::max_element_bytes`，编译前从段头量出（`plugin-host/src/wasm.rs`） | `fault_matrix.rs::a_large_passive_element_segment_is_refused_before_instantiation` + `::a_small_passive_element_segment_still_loads` |
| 12 | `verify_signed` 未咨询验证器就记录 `Signature` | 非官方来源返回 `SignatureLaneRequired`（`core/.../plugin/{artifact,trust}`） | `artifact.rs::signature_assurance_is_refused_for_a_user_artifact` |
| 9 | `..` 选择器会写到记录根之外 | 共用规则拒绝以点开头的名字(`core/.../graft/document.rs`) | `document.rs::selectors_that_could_leave_the_record_directory_are_refused` + `plan_tests.rs::a_selector_that_escapes_the_record_directory_creates_nothing` |
| 10 | `explain` 把过期产物报成 `known: true` | `build_output_is_current` + 指纹只由干净运行写下(`build_method/{scope_view,pipeline}.rs`、`cli/{explain,explain_report,explain_overlay}.rs`) | `scope_view::published_output_is_current_until_the_sources_change`、`::a_failed_check_publishes_no_trusted_output`、`lib_tests.rs::explain_reports_an_unknown_scope_when_the_build_output_is_stale` |
| 6 | 畸形或带不可求值门控的 `static_graft_plan!` 会 panic | 诊断穿过 `graft_view::query`（三类拒绝） | `query.rs::a_malformed_graft_declaration_is_reported_not_fatal`、`::an_unevaluable_graft_gate_is_reported_not_fatal` |
| 5 | 宏内容藏起了它命名的注册面 | 改为读宏 token 里的 `::` 路径，而不是放宽作用域（`core/src/registry_core/syntax/reference_scan.rs`） | `face.rs::a_macro_invocation_is_read_for_the_paths_it_spells` + 上面的端到端表 |
| 8 | `function_source_range` 把字符串与注释里的花括号也数进去 | 改为在屏蔽后的文本上计数，不配平返回 `None`（`core/src/registry_core/source/source.rs`） | `source_tests.rs::a_brace_in_a_string_does_not_close_the_function` |
| 14 | README 的注册面示例用了已删除字段 | 两份 README 里那四行已删 | 无（文档门禁展开不了宏） |
| 15 | `docs/migration.md` 承诺了一个不存在的 Compare 页 | 四份文档改成只有 Search/Inspect/Data | 无 |

## Angles still in flight / 仍在进行中的角度

- **Documentation truth** (one delegated audit) has not reported yet; its findings are
  not in this document. Treat the doc-truth surface as unexamined until they are.
- **Build pipeline and macro front end** reported after the first draft of this
  document and is included above (findings 5, 6, 13, 30).
- 文档真实性（一份委派审计）尚未回报，其发现不在本文中；在它回来之前，把文档真实性这一面
  当作未审。
- 构建期与宏前端在第一份失败后重新发起，仍在运行；其发现同样不在本文中。

## Post-release round: what the `v0.1.0` push turned up / 发布后一轮：`v0.1.0` 的推送查出了什么

`v0.1.0` was pushed to `origin` at 2026-09-25T11:55:34Z, starting
`xirang-release` run `36132054983` and `xirang-ci` run `36132054937`. Neither
the Windows job that had been failing on `main` for three consecutive runs nor the
empty publish secret was in this document's angle list, so both are recorded here.
`v0.1.0` 于 2026-09-25T11:55:34Z 推到 `origin`，启动了 `xirang-release` 运行
`36132054983` 与 `xirang-ci` 运行 `36132054937`。那个在 `main` 上连续三次失败的
Windows 任务，以及空的发布密钥，都不在本文件的审计角度清单里，因此记在这里。

### P1. Windows CI was red on `main`, and that hid the lint, doc and package gates there
### P1. Windows CI 在 `main` 上一直是红的，而它藏住了那边的 lint、文档与打包门禁

`Angle: release/CI`. `run_method/tests/external_compact_face.rs:99` compared
`REGISTRATION.source.file` against the `/`-spelled
`"run_method/tests/external_compact_face.rs"`.
`Angle: release/CI`。`run_method/tests/external_compact_face.rs:99` 把
`REGISTRATION.source.file` 与 `/` 拼写的 `"run_method/tests/external_compact_face.rs"` 比较。

**证据 `[实测]`**：`gh run view 36013106327` → `verify (windows-latest, stable)` 与
`verify (windows-latest, 1.96.0)` 都停在 `Tests`，值为
`left: "run_method\\tests\\external_compact_face.rs"` 对
`right: "run_method/tests/external_compact_face.rs"`。该任务在此中止，`Release tests`、
`Clippy`、`Documentation`、`Package standalone crates`、`Package audit` 全部 `skipped`——
**Windows 上 clippy、文档与打包门禁从来没有真正跑过**，这才是这条红测试的代价，而不只是
它自己失败。同一批的三个 `main` 提交结果相同。

**根因**：`file!()` 记录宿主分隔符，而声明在编译期无法在不分配的前提下改写它。同目录的
`external_source_default.rs` 早已写明这一点并做了归一化，这条测试漏了。

**最小修法（已修）**：比较 `portable_path(...)` 产出的可移植形式。
**钉子**：`external_compact_face.rs` 自身（折叠一旦坏掉，它在 Windows 上就红），加上新的
`source_location_tests.rs::a_recorded_path_renders_portably_on_every_platform`。

### P2. The search surfaces compared a `file!()` path against a `/`-spelled query
### P2. 搜索面把 `file!()` 路径与 `/` 拼写的查询做比较

`Angle: surfaces`. 五处 `.contains(query)` 直接吃原始值：
`run_method/runtime/trace/frames/frames.rs:138,289`、
`run_method/runtime/trace/locals/call_trace.rs:220`、
`run_method/call_report/call_report.rs:199`、
`studio/studio/app/search_queries.rs:35`。这些面的 `source.file` 来自运行期宏里的
`file!()`（P1 已在 Windows 上实测出反斜杠），而同一个面**显示**源码位置时走
`SourceLocation` 的 `Display`/`describe`，那是 `portable_path` 的 `/` 形式。于是用户照着
屏幕重敲的查询在 Windows 上落空。`call_report.rs:164` 与 `search_queries.rs:36` 的 `path`
不受影响：那是 `Registry::path_for` 用 `format!("{}/{}")` 造的逻辑注册路径，与 `file!()` 无关。

**最小修法（已修）**：内核新增 `declaration::source_file_matches(file, needle)`（`needle` 由
调用方小写一次），五处改用它；四处用无分配的分支，只有真含反斜杠时才产出可移植副本。
**钉子**：`source_location_tests.rs::a_query_with_slashes_matches_a_backslashed_source`
（在 Linux 上以反斜杠输入运行，与 `identity.rs` 的 Windows 分隔符用例同法）。逻辑只有一份，
调用点已无逻辑可钉。

### P3. Withdrawn: the kind and module migrations do **not** have this defect
### P3. 撤回：kind 与模块迁移**没有**这个缺陷

我起初把 `migration.rs` 的两处 `snapshot.source.file == normalized_path(...)` 读成 Windows
缺陷并按此修了一遍。核实后撤回：`FaceManifest::parse_source` 先把
`source_path_from_file(path)` 写进 `values["source"]`（`run_method/authoring/manifest/parse/parse.rs:35-36`），
而该函数的**每个**返回分支都经 `normalized_path`（`run_method/authoring/parse/parse.rs:27-42`），
`snapshot_from_values` 又只读这个值（`core/src/registry_core/authoring/snapshot/snapshot.rs:30`）。
因此 authoring 快照里的 `source.file` 永远是 `/` 拼写，原比较在 Windows 上同样成立。

改动已完全回滚（`git diff run_method/authoring/operations/migration.rs` 为空），
`migration_tests.rs` 已删除。**记在这里的目的**：让下一个读到
`source.file == normalized_path(...)` 的人不必重新推导一遍，也不要再"修"它一次。

### Release run `36132054983` outcome / 发布运行的结局

**失败在 step 13 `Publish in dependency order`，但没有造成任何损害。**
`CARGO_REGISTRY_TOKEN:` 在日志里是空值（仓库 secret 未设置），`cargo publish` 在第一次
上传时以 `error: please provide a non-empty token` 失败，随后
`cannot publish xirang-macro: waiting for xirang-core`。
`[实测]`：`https://index.crates.io/ni/ch/<crate>` 与 crates.io API 对九个名字全部返回
**404**，因此**一个 crate 都没有发出**，`0.1.0` 这个版本号没有被烧掉，也不需要改版本号。

**恢复路径**：为仓库配置 `CARGO_REGISTRY_TOKEN` secret，然后 `gh run rerun 36132054983 --failed`
（重跑的是同一次 tag push，`Tag names this version` 仍然生效）。**不要**用
`workflow_dispatch` + `publish: true` 来恢复：那正是 MAJOR 6——dispatch 运行的
`github.ref` 是分支，tag 检查被跳过，而 `if` 仍然允许上传。

### P4. The crates.io new-crate rate limit means a nine-crate first release cannot finish in one run
### P4. crates.io 对新 crate 的速率上限使九连发无法在一次运行里完成

`Angle: release/CI`. The second release run — the secret configured, the tag moved to
`f3c2b98` — passed all twelve checks and hit the limit after publishing **five** crates in
`Publish in dependency order`.
`Angle: release/CI`。第二次发布运行（已配置 secret、tag 已移到 `f3c2b98`）通过了全部十二个检查，
在 `Publish in dependency order` 里发完**五个**之后撞上限速。

- **已发布**（index 返回 200）：`xirang-core`、`xirang-macro`、`xirang-build-method`、
  `xirang-mcp`、`xirang-run-method`。`[实测]`
  `https://crates.io/api/v1/crates/xirang-core` 的 `versions` 为 `["0.1.0"]`。
- **被拒**（index 404，各一次）：`xirang-debug-method`、`xirang-plugin-host`、
  `xirang-studio`、`xirang-cli`，错误为
  `status 429 Too Many Requests: You have published too many new crates in a short period of
  time. Please try again after Fri, 25 Sep 2026 12:55:31 GMT`。

`[实测]`：从第一个上传（约 12:44:30Z）到窗口结束（12:55:31Z）约十一分钟、五个新 crate，因此这是
"每窗口 N 个新 crate"的批量额度，而不是两次发布之间的间隔限制——九连发必然越过它。这**不是**本
仓库的缺陷，而是发布链必须按窗口分批的约束。cargo 侧同类问题见
[rust-lang/cargo#15754](https://github.com/rust-lang/cargo/issues/15754)（工作区发布中途失败，
错误信息不可操作）与 [mesh-llm#691](https://github.com/Mesh-LLM/mesh-llm/issues/691)（同一条 429
断链）。

**恢复方式**：等窗口过去后重跑，路径由 MAJOR 7 的修复支撑——重跑时已在 index 上的五个被跳过，
只发剩下的四个；step 14 `--verify-consumers` 在发布成功之后运行，是"陌生人能消费"的最终证明。
**钉子**：MAJOR 7 的假 index 对照覆盖了"重跑跳过已发布"这一半；速率窗口本身由 registry 决定，
本地钉不住。

### P4 outcome: the release completed on the first re-run after the window
### P4 结局：发布在窗口过去后的第一次重跑完成

`gh run rerun 36137486348 --failed` (14:14Z) published the remaining three and ran
`--verify-consumers`: **all fifteen steps `success`**, and the crates.io API reports
`versions = ["0.1.0"]` for all nine (`xirang-core` through `xirang-cli`). No
version bump was needed, because the five crates published before the rate limit are
byte-identical to the tag: the later commits touched only `tools/` and `docs/`.
`[实测]`：`gh run rerun 36137486348 --failed`（14:14Z）发完剩下三个并跑通
`--verify-consumers`——**十五步全部 `success`**，crates.io API 对九个 crate 都报
`versions = ["0.1.0"]`（`xirang-core` 到 `xirang-cli`）。不需要升版本号：速率限制前发布的
五个与 tag 逐字节相同，因为其后的提交只动了 `tools/` 与 `docs/`。

## Gate hardening landed in the post-release round / 发布后一轮落地的门禁加固

- **MAJOR 6——手动发布路径被移除，并有了门禁。** `release.yml` 不再声明 `publish` 输入，两个
  上传步骤都带上 `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')`，而不是
  依赖触发列表。新门禁 `conventions/src/release_workflow.rs` 拒绝这样的发布工作流：读取任何
  输入、把 tag 要求只留在版本检查那一步、丢掉 `workflow_dispatch`、或把受守卫的步骤改名消失。
  钉子：实际工作流干净，另有三份夹具（旧条件、在受守卫步骤旁仍带输入、受守卫步骤被改名）都会
  被报出——修复前全部为红。
- **被链接的目录不再被遍历（低严重度备注）。** `rust_sources` 与 `collect_markdown` 改用
  `symlink_metadata`，于是 `core/src/zz -> /tmp/elsewhere` 不再把检出之外的文件拉进每一个门禁。
  钉子：`conventions::tests::a_linked_directory_is_not_walked_into`（Unix）。
- **构建产物不再被遍历。** `<member>/target/xirang/out/*.rs` 是 `build_method` 写生成 Rust 的
  地方；过期产物不再能被读成源码声明。钉子：`conventions::tests::build_output_is_not_walked_into`。
- **注释与字符串不再被当作代码扫描。** `purity` 改在内核的 `mask_non_code` 上运行，取代按行的
  `//` 判断，于是 `/* std::fs */` 与 `let probe = "std::fs";` 不再是违规；`is_comment` 随其唯一
  调用方一起删除。钉子：`purity::comments_and_strings_are_not_the_code_that_is_scanned`。
- **新门禁 `conventions/src/doc_anchors.rs`。** 活文档里每个 `<path>.rs:<line>` 引用都必须可解析：
  文件要存在（先是精确的根相对路径，否则是以它结尾的唯一路径），行号要落在文件内。审计与设计
  文档作为记录豁免。它第一次运行就抓到一处真实漂移——`docs/roadmap-1.0.md` 引用了
  `run_method/macros/face_objects.rs:242-243`，而该文件只有 172 行——现改为该 arm 当前的
  行号。边界写在模块文档里：它抓不到"只差几行"的语义漂移，那仍然只能由人读出来。
- **MINOR 16。** 两份 `run_method/README*.md` 改为点名真实存在的 `trace_mode_from_env()`，而不是
  从未存在过的 `TraceMode::from_env`。

## Remaining post-release items, finished / 发布后剩余待办，做完

- **备注 4——`core/tests/nesting_budget.rs` 不再跳过读不到的文件。** 读取移进 `inspect`，它同时
  返回被拒绝的文件与读不到的文件，工作区那条测试断言后者为空：遍历找到却读不到的文件，就是守卫
  从未检查过的文件，把工作区称作"干净"会变成关于"它碰巧能打开哪些文件"的断言。钉子：
  `a_file_that_cannot_be_read_is_reported`（临时文件里放非法 UTF-8），修复前为红。
- **备注 5——AGENTS 改动规则 4 现在有门禁。** `conventions/src/naming.rs` 检查 `examples/` 之外的
  每个成员：`xirang-<目录名，`_`→`-`>`、`xirang_<目录名，`-`→`_`>`（没有 `[lib] name` 也
  可以——cargo 会推导），外加 crate 表里已经记录的唯一实测例外：`core/` 的 lib 是 `xirang`。
  它第一次运行就抓到一个真实的 bug——**在门禁自己身上**：缺了连字符折叠时它把工作区自己的
  `plugin-host` 报了出来，这现在正是钉子
  `a_hyphenated_directory_maps_to_an_underscored_lib_name`。
- **条目 31——重新实测而不是照抄。** `json` 缺失于四份模块清单与两份 README 的 core 行（该模块挂在
  `core/src/registry_core.rs:19-20`）；`cli/README.md` 的命令表缺
  `xirang snippets [path] [--editor vscode|nvim|blink|auto] [--stdout]`（`cli/lib.rs:46`）；
  `LICENSE` 那行改为"每个**已发布**的 crate 目录"，这样它恰好为真（`conventions/` 是
  `publish = false`）；裸 `mod` 的数字则删掉了——审核声称 196，而按规则本身（前面没有 `#[path]`
  的 `mod`）实测是 31（12 处在 crate 根、19 处在 `#[path]` 载入的父文件里），且一个随每个新模块
  变化的数字不该写在散文里。

### P5. The index probes had no retry, so one network blip read as "not published"
### P5. index 探针没有重试，于是一次网络抖动会被读成"未发布"

`Angle: release/CI`. `tools/xirang-publish` 与 `tools/xirang-package-audit` 的 `is_published`
都用单次 `curl -fsS --max-time 20` 判定某个精确版本是否已发布，失败即 `return 1`，而两个工具的
注释都明说"网络失败按'未发布'处理"。两种后果：在 `--publish` 模式下，一次抖动让依赖看起来缺失，
脚本就在 `error: cannot publish X: waiting for Y` 处**退出 1**，中断整次发布；在审计里，它把
crate 报成 `skipped`，或者在探针成功、而校验构建从只传播了一半的 index 取依赖时报成 `failed`。

**证据 `[实测]`**：首次发布之后，同一条 `tools/xirang-package-audit` 在相邻两次运行里对**同一棵
静止的树**给出了不同的可检查集合——一次 `verified` 了 `cli`/`debug-method`，却把一小时前就已发布的
`run-method` 报成 `skipped`，并把 `plugin-host` 报成 `failed`；五分钟后以及随后的三次运行都是
`verified: 九个 / skipped: none / failed: none`。树没有变，index 的可见性变了。

**最小修法（已修）**：探针请求加重试——`curl --retry 3 --retry-connrefused --retry-delay 2`、
`wget --tries=3 --waitretry=2`；"未发布"这个结论现在必须经受住 registry 短暂不可达。
**钉子**：本地"前两次 503、第三次 200"的对照——旧命令行 1 次请求、退出 22、判定"未发布"；
新命令行 3 次请求、退出 0、取到正确正文。**残留风险**（记在这里，不假装没有）：重试耗尽之后，
`--publish` 仍会因为一个看起来缺失的依赖而停止——这是保守方向而不是错误方向。
