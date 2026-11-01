# 独立验证：本批「追加」任务合并轮（t13 / t16–t24）

- 验证者：kernel-auditor（t25，**非作者**：t13/t16/t17/t18/t19/t20/t21/t22/t23/t24 我一条未参与）
- 日期：2026-09-28（本检出，`--offline`）
- 真实树：**零改动**（本轮我只写本文件；全部夹具、副本、变异都在 /tmp）
- 装置：树外探针工程 `/tmp/nk-t25-probe`（path 依赖 core+run_method，**自带 target**）+ 检出副本 `/tmp/nk-t25`（`CARGO_TARGET_DIR=/tmp/nk-t25-target`，与真实树 target 完全隔离）+ 自写夹具 `conventions/tests/t25_probe.rs` + 5 组变异 + 自写 grep/解析脚本

## 0. 判定一览

| 任务 | 判定 | 一句话 |
| --- | --- | --- |
| t13 可移植路径收口 | **证实** | 全仓只剩内核函数体一处折叠；crate 外探针 68 019 个输入对 `portable_path` 与旧式 `replace` 0 处不等；把内核改成恒等 → 2 条钉子红 |
| t16 内核公开 `compact_registration_rule` | **证实** | crate 外工程真调：8 行历史字节表逐字相同 + 往返 + 文本分支 + shim 路径；Studio 纯委派、生产路径无第二处手拼；改坏内核 → Studio 两条钉子红 |
| t17/t18 措辞收敛 | **证实（有残留）** | 他们点名的判别性 grep 逐条复现（改前 >0 → 改后 0）；但全仓 sweep 发现 `CHANGELOG.md:154/1334` 仍用旧词（新条目 N-1） |
| t19 文书补记 | **证实** | 改前基线 `a0ea4db:CHANGELOG.md` 的 `compact` = 2 处（:245/:1156）与自述一致；两个入口在 EN :167-168 / ZH :1344-1345；库名写法正确（`xirang_core::` 在 *.md 0 命中） |
| t20 注释漂移 | **证实** | 文档逐句与 `catalog.rs` 实现对上（空列 `Some("")` ≠ 七字段 `None`、仍覆盖无值候选、追加被可见拒绝），并被 `accounts_for_provenance` 读码复核 |
| t21 文书补丁 | **证实** | `unmatched` CHANGELOG 2→0（HEAD :103/:1273）；残留 2 处全在 `mcp/diff.rs` 注释（删除说明）；五个桶名与 `diff.rs:217` 权威行逐字一致；roadmap「六项」1→0 |
| t22 F-2/F-3/F-4 | **证实** | F-3 自造夹具（转义引号的重命名依赖）修后报 1 条、边界静默、变异即红；F-2 三种空白拼法豁免而 `modx;`/`module x;`/`mod x = 1;` 仍度量，变异即红；F-4 两语言均无数字 |
| t23 新鲜度词形耦合钉子 | **证实** | overlay/converge 无内联词形、都读 `freshness()`；改坏 `freshness()` 的 current 分支 → 新钉子 + 既有 overlay 钉子一起红 |
| t24 钉子搬移 | **证实（一处自证不可复现）** | 553/472 行（≤600）、门禁文件只剩挂载声明、117 条测试、两条钉子名字与新家与自述一致；「断言逐字不变」我无快照可比 → 见 §8 |

## 1. 装置与手段（含自证边界）

- **树外探针**（`/tmp/nk-t25-probe`，`CARGO_TARGET_DIR=/tmp/nk-t25-target`）：真调 `xirang::authoring::parse::{compact_registration_rule, parse_registration_rule_owned, rule_syntax_from_text}`、`xirang::declaration::portable_path`、以及 Studio 该走的 shim 路径 `xirang_run_method::authoring::parse::compact_registration_rule`。这是"crate 外工程"那条要求。
- **检出副本**（`/tmp/nk-t25`，rsync 排除 target/.git）：所有变异与门禁重跑都在这里，**自带 `CARGO_TARGET_DIR=/tmp/nk-t25-target`**，从不写真实树 target。我的夹具 `conventions/tests/t25_probe.rs` 只存在于副本。
- **自写夹具**：`t25_probe.rs` 两条用例（F-3 重命名依赖夹具、F-2 五种挂载拼法夹具），驱动 `naming::findings` / `size::oversized` 真实入口；期望值由我自己推。
- **变异**：5 组，各自 in-copy → `touch` → 跑相关 test target → 还原 → 同时与"变异前副本内容"和"真实树当前内容"比 sha256。
- **自写 grep/解析脚本**：本报告里的每个计数都是我自己的脚本/命令得到的，未采信作者的 grep 或 diff。

**我自己的装置 bug（照实披露）**
1. 探针第一版把 `rule_syntax_from_text` 当"紧凑值解析器"用，喂了 `exports:…;handle:…`，得到 `ANY`。读码后确认它读的是**源码文本**里 `.require_*(` 标记；改成真源码片段后一致。是我的用法错，不是实现错。
2. F-2 夹具第一版把 `other` 成员的挂载文件命名成 `other_tests.rs` 而声明是 `mod big_tests;`，于是"应当豁免"的那条被度量——是我的夹具文件名错。
3. 探针把 `preset:`（空 preset）一行也断言了往返，结果被解析器拒绝（`registration_rule preset cannot be empty`）。改成"只有这一行可以不可解析"，并把这条不对称**当作 t16 自述的独立证实**记下来，而不是当失败。

## 2. t13 —— 证实

- **我自己的 grep（全仓 `*.rs`，排除 target）**：`replace('\\', "/")` 恰好 **1** 处命中，就是 `core/src/registry_core/declaration/source_location.rs:35` 的 `portable_path` 函数体；`MAIN_SEPARATOR`、`split('\\')` 0 命中；其余 `.replace(` 命中全是反方向转义（`\`→`\\`）、`/`→`::`、或字面文本替换。
- **消费者确实都转发**（我的 grep 逐处列出）：`build_method/node.rs:30`、`renderer/tree.rs:113`、`renderer/pass.rs:300`/`:301`/`:432`、`scaffold/install.rs:240`、`graft_plan_check.rs:93`；`conventions/src/lib.rs:353`、`:408`；`run_method/tests/external_source_default.rs:65`、`external_compact_face.rs:112`；`debug_method/tests/collector_integration.rs:34`；`mcp/index.rs:178-179`（它自己那层 `portable_path` 也转发内核）、`preview.rs`、`converge_trace.rs:246`；studio 两条测试引用。（比作者列的 8 处更多——多出的几处本来就是转发的，或无第二份折叠。）
- **等价性我自己的抽样复算**（crate 外探针）：语料 = 18 个手写边界串（空串、单反斜杠、`\\a\\b\\`、`C:\proj\src\…`、混合分隔符、`////`、中文、控制字符、`{}`/`%s`）+ 按长度 0..=16、每个长度 4000 条、字符集 `aA/\\.:-_ 中文\t*?<>|` 的随机串，共 **68 019** 个输入：`portable_path(s)` 与 `s.replace('\\', "/")` **0 处不等**；且非空洞——`portable_path(r"a\b") == "a/b"` 而 `!= r"a\b"`。
- **变异 M-E**（`portable_path` 改成恒等）：`cargo test -p xirang-core --features syntax --lib source_location` → exit 101，红两条：`a_recorded_path_renders_portably_on_every_platform`、`a_query_with_slashes_matches_a_backslashed_source`；还原 `identical_to_pristine=True identical_to_real_tree=True`。

## 3. t16 —— 证实

- **crate 外真调**（探针工程）：8 行字节表全部与 t16 的对照表逐字一致（`ANY` / `preset:ActionParts` / `parts:paint,shape` / `exports:control.render` / `handle:ControlHandle` / `part_trait:ActionParts` / 五项齐全 / 空 preset 的 `preset:`）；前 7 行经 `parse_registration_rule_owned` 往返回原值，第 8 行解析器**按设计拒绝**（`registration_rule preset cannot be empty`）——这条不对称是 t16 自述的，我这里独立复现到。
- **历史源码文本分支**：我自写的源码片段（`RegistrationRule::new().require_exports(&["control.render"]).require_handle_traits(&["ControlHandle"])`）→ `exports:control.render;handle:ControlHandle`；`…::ANY` → `ANY`。即文本分支确实经同一入口出字节。
- **shim 路径**：`xirang_run_method::authoring::parse::compact_registration_rule` 也取到（`preset:P`）——Studio 该走的那条。
- **Studio 纯委派**：`studio/studio/app/source_index.rs:96-98` 就是一行转发。该文件里所有 `preset:`/`parts:` 字面量都在 `#[cfg(test)] mod registration_rule_text_tests`（`:241` 的 `#[cfg(test)]` + `:242` 的 `mod`）内。全仓 grep 生产代码的手拼子句：只有内核渲染器本身 + 测试期望 + `build_method/scaffold/snippets.rs:37`（那是 CLI 脚手架的 shell 片段，不是紧凑规则）。
- **变异 M-A**（内核渲染器丢掉 preset 子句）：`cargo test -p xirang-core --features syntax --test registration_rule_entry` → exit 101，**4 红**（`historical_rule_spellings_render_unchanged_bytes`、`the_compact_renderer_is_reachable_from_outside_the_crate`、`the_renderer_and_the_parser_read_the_same_rule`、`the_source_text_branch_agrees_with_the_parsed_value`）；`cargo test -p xirang-studio --lib` → exit 101，**Studio 两条钉子红**（`the_call_site_renders_what_the_kernel_renders`、`the_rule_clauses_keep_their_historical_spelling`）——"改坏内核必须让 Studio 变红"成立。还原 `identical_to_pristine=True identical_to_real_tree=True`。
- **并发核对**：B4 期间同一文件 `authoring/parse/rules.rs` 被他人改过（`parse_requirements_owned` 签名变化，:180-198）。我复核了**渲染器区域 :70-105 与我校验的修订逐字节相同**，故 t16 判定对该符号仍成立。

## 4. t17 / t18 —— 证实（有残留，新条目 N-1）

- **判别性 grep 我自己复现**（`git show HEAD:<file>` vs 工作树，逐文件计数）：`slot name` / `槽位名` / `or slot name` / `模块或槽位名` / `default_slot_name` / `Registry slot name` / `注册机槽位名` / `registry slot name` / `注册槽位名` / `same slot name` / `同一个槽位名` 在**他们点名的每个文件**上都是"改前 >0 → 改后 0"：`mcp/search.rs`(1/2→0)、`mcp/search_tests.rs`(2/2→0)、`mcp/tools.rs`(1→0)、`mcp/README.md`(1→0)、`mcp/README.zh-CN.md`(1→0)、`build_method/face_view.rs`(2/2/1→0)、`core/.../declaration/owned.rs`(1/1/1/1→0)、`core/.../graft_ops/resolution.rs`(1/1/1/1→0)、`run_method/macros/face_registration.rs`(1/1/1/1→0)、根 `README.md`/`README.zh-CN.md`(1/1→0)。他们"非判别性"那条自述也与我一致（那些文件改前也是 0）。
- **残留在别处（我的全仓 sweep）**：同一概念的**唯一**残留是 `CHANGELOG.md:154`（EN "matches logical path, `kind`, module and slot name first"）与 `:1334`（ZH "模块与槽位名"）——那正是 mcp 现在叫 `registry_name` 的字段。其余 `slot name` 命中属**别的概念**：`core/.../plugin/contracts/contracts.rs:46`（插件槽）、`studio/studio/ui/ui.rs:49`+`studio/studio/app/keyboard.rs:68`（Studio 布局槽位）、`run_method/authoring/face_manifest.rs:85`+`core/tests/ungated_authoring_data.rs:4`（创作表单字段词典）。详见 N-1。

## 5. t19 / t21 —— 证实

- **t19 的"改前基线"我自己取**：`git show a0ea4db:CHANGELOG.md | grep -c compact` = **2**，且正是 `:245`（compact JSONL）与 `:1156`（compact encoding）——与自述逐字一致；携带该交付的提交 `5f408c8` 里是 10。两个入口现在出现在 EN `:167-168`、ZH `:1344-1345`，与实现（`rules.rs:82` 的 `pub fn compact_registration_rule`、`admission.rs` 的 `pub fn compact_admission`）一致。
- **库名写法**：`grep -rn 'xirang_core::' --include=*.md .` = **0**；他们改用 `xirang::…` 是对的（包名 `xirang-core`、lib 名 `xirang`）。
- **被点名的钉子都存在**：`core/tests/compact_admission_entry.rs`、`core/tests/registration_rule_entry.rs`、Studio 的 `the_call_site_renders_what_the_kernel_renders`/`the_call_site_carries_no_second_rule_renderer`。
- **t21**：`unmatched` 在 `CHANGELOG.md` = 0（HEAD `:103`/`:1273` 各 1）；残留 2 处都在 `mcp/diff.rs` 的注释（`:189`/`:198`，写的正是"这里曾有一个 unmatched 桶"）；五个桶名与权威行 `mcp/diff.rs:217`（`"ok {}  undeclared {}  stale {}  re-identified {}  unreadable {unreadable}\n"`）逐字相同；`六项` 在 `docs/roadmap-1.0.md` 1 → 0。
- t9 的 F-1（"52 处"数字）已被记账：`docs/audit-2026-09-28/audit-report.md:1883` 记 F-1 并给出更正口径（门禁实际口径 1 处、全树 124 处）——本任务不重复。

## 6. t20 / t22 / t23 / t24

- **t20 证实**：`studio/studio/app/tests/lock_writes.rs` 的 doc 现在写"十字段三个空列各解析成 `Some("")`＝声明此处没有值，刻意不同于七字段的 `None`＝没提到；该记录仍覆盖同样没点值的候选 → 信任规则放行 → 追加被执行并被可见拒绝"，并点名 `core/tests/plugin_lock_provenance.rs` 的 `an_explicitly_empty_provenance_column_pins_absence`（`：115` 存在）。逐句对实现：`catalog.rs:203-205`（`fields.get(7..9).map(ToString::to_string)`，空列 → `Some("")`）、`catalog.rs:372-377`（`Some("")` 分支 = `candidate.is_none_or(str::is_empty)`，故 `Some(""), None` → true）、`studio/.../mutations.rs` 用 `contains_record`。
- **t22 F-3 证实（我自己的夹具）**：夹具里模板写成与出厂模板同形的转义引号（`"runtime = {{ package = \"xirang-missing\", … }}"`），修后 `naming::findings` 恰好 1 条并点名该包；边界 `my_package = "…"` 与 `--package="…"` 静默。**变异 M-C**（不剥开引号的转义）→ 我的夹具红 + 作者搬移后的钉子 `naming::naming_tests::a_renamed_dependency_is_named_through_its_escaped_package_field` 红；还原零残留。实现读码：`package_field_value`（`naming.rs:387-400`）处理 `=` 两侧空白、开/闭引号的转义。
- **t22 F-2 证实**：我自己的夹具（7 个成员、各 610 行）→ `mod   big_tests ;`、`mod big_tests;`、`pub(crate) mod  big_tests ;` **豁免**；`modx;`、`module x;`、`mod x = 1;` 三种**仍被度量**（只有真的 `mod big_tests;` 那个成员豁免）。**变异 M-D**（退回词元连接）→ 我的夹具红 + 作者的 `size::size_tests::a_space_before_the_semicolon_is_the_same_declaration` 红；还原零残留。
- **t22 F-4 证实**：`conventions/src/mounting.rs` 两语言都没有数字（EN `churning crates` + "How many crates that would touch is deliberately not restated"；ZH "这会牵动多少 crate 刻意不在这里复述"），`五`/`29` 在该文件 0 命中。
- **t23 证实**：`grep '"current"\|"stale (run' mcp/overlay.rs mcp/converge.rs` = **0**；两处都读 `evidence.freshness()`（overlay.rs:72、converge.rs:133 一带）；耦合钉子 `evidence_tests` 里逐字写着 `\nbuild current\n`（5 处）与 `\nbuild stale (run \`xirang check\`)`。**变异 M-B**（`"current"`→`"healthy"`）→ `evidence::evidence_tests::every_report_spells_the_freshness_word_that_one_place_produces` **与既有** `overlay::overlay_tests::a_published_scope_marks_the_slots_it_prunes` 同时红（mcp 107 passed / 2 failed）；还原零残留。
- **t24 证实（搬移事实）**：`conventions/src/size.rs` **553** 行、`naming.rs` **472** 行（≤600）；两个门禁文件里 `#[cfg(test)]` 只出现在挂载声明上（`:551-552` 挂 `size_tests.rs`、`:470-471` 挂 `naming_tests.rs`）；`size_tests.rs` 409 / `naming_tests.rs` 311；lib 测试 **117** 条；两条钉子按自述落在新家（`naming::naming_tests::a_renamed_dependency_is_named_through_its_escaped_package_field`、`size::size_tests::a_space_before_the_semicolon_is_the_same_declaration`），模块前缀变化与他们自己披露的一致。**没能复现的那半**：他们说的"断言逐字不变 / 叶子名逐名 diff 为空"需要搬移前的快照，我手上没有（见 §8 未覆盖）。

## 7. 变异表（5 组，一次跑完，判定取 hash 稳定那次）

| 变异 | 目标 | 红侧（我自己跑出来的） | 还原 |
| --- | --- | --- | --- |
| M-A t16：内核渲染器丢 preset 子句 | `core/.../parse/rules.rs` | core 4 红 + **Studio 2 条委派钉子红**（+ 环境用例 `new_project_and_explicit_root_face_compile`） | pristine==real==True |
| M-B t23：`freshness()` 的 current 词改 `healthy` | `mcp/evidence.rs` | `evidence_tests::every_report_spells_the_freshness_word_that_one_place_produces` + `overlay_tests::a_published_scope_marks_the_slots_it_prunes` | pristine==real==True |
| M-C t22 F-3：不剥开引号转义 | `conventions/src/naming.rs` | 我的 `t25_probe::a_renamed_dependency_is_named_through_its_escaped_package_field` + 作者 `naming::naming_tests::…escaped_package_field` | pristine==real==True |
| M-D t22 F-2：退回词元连接 | `conventions/src/size.rs` | 我的 `t25_probe::whitespace_spellings_mount_a_test_and_a_prefix_does_not` + 作者 `size::size_tests::a_space_before_the_semicolon_is_the_same_declaration` | pristine==real==True |
| M-E t13：`portable_path` 改恒等 | `core/.../declaration/source_location.rs` | `source_location_tests` 两条（含 `a_recorded_path_renders_portably_on_every_platform`） | pristine==real==True |

每组都是：副本内改 → `touch` → 跑目标 test target → 还原 → 同时与"变异前副本"和"真实树当前内容"比 sha256（`identical_to_pristine=True identical_to_real_tree=True`）。副本自带 `/tmp/nk-t25-target`，真实树 target 未被任何变异写入。

## 8. 五条门禁（哈希钉住）与并发窗口

**判定钉住的修订**：我在 20:28 的检出副本（`/tmp/nk-t25`，自带 `CARGO_TARGET_DIR=/tmp/nk-t25-target`）上跑；跑完逐文件比对确认 §2–§6 里校验过的 12 个交付文件与真实树**逐字节相同**，只有两处例外并已单独处理：`core/.../parse/rules.rs`（B4 改了 `parse_requirements_owned`，我复核**渲染器区域 :70-105 仍逐字节相同**，t16 判定不受影响）与 `mcp/evidence_tests.rs`（t23 的耦合钉子被**扩到五份报告**：8 条逐字断言、按报告收集失败；耦合只增不减，详见 §6）。

| 命令 | 校验修订（副本） | 真实树 20:53（哈希钉住） |
| --- | --- | --- |
| `cargo test --workspace --offline` | exit 101；39 个 ok 块，**唯一红** = `studio::app::tests::project::new_project_and_explicit_root_face_compile`（生成项目离线解析 `xirang-run-method = "^0.1.6"`，本地 git 缓存只有 0.1.5——队长报备的环境 artifact） | exit 101；39 ok 块，**唯一红** = `studio::app::tests::edit::editing_module_name_moves_the_face_and_keeps_generated_source_compact`（`studio/studio/app/tests/edit.rs:146`），同刻 `core/.../authoring/snapshot/snapshot.rs`(20:52) 与 `parse/rules.rs`(20:53) 正被写 |
| `cargo test -p xirang-conventions --offline` | **exit 0**（117 passed） | **exit 0**（117 passed） |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | exit 101；**唯一错** = `xirang-build-method` 的 `unused import: discover_root_reporting / discover_root`（B4 在飞的 `build_method/{lib,face_view,pipeline}.rs`） | exit 101；**唯一错** = `run_method/runtime/trace/artifact/parse_tests.rs:146` 的 `this assertion has a constant value`（`assert!(INTERN_LIMIT >= 1 << 16, …)`，同一在飞窗口内的文件） |
| `cargo fmt --all -- --check` | **exit 0**（先把我的夹具文件 rustfmt 干净） | **exit 0**（0 行输出） |
| `tools/xirang-publish --check-table` | **exit 0**（`dependency table matches the manifests (9 crates)`） | **exit 0**（同上） |

- 跑前跑后 `docs/audit-2026-09-28/audit-verify-extra.md`、七个 `conventions/src/*.rs`、`mcp/{overlay,converge,evidence,evidence_tests}.rs`、`core/.../{rules,source_location}.rs`、`studio/.../source_index.rs`、`CHANGELOG.md`、`docs/roadmap-1.0.md` 的 sha256 **全部 OK**（零漂移）。
- **真实树这一窗口为什么不是字面全绿**：B4（另一个批次）正在同一时段改 core/studio/build_method/run_method——20:42 时 21 个 `*.rs` 在 3 分钟内被写，20:47–20:53 又改了 6–10 个；我逐点定位的两条红都在**本批交付面之外**：`studio/studio/app/tests/edit.rs`（B4 的 core 快照改动波及）与 `run_method/.../parse_tests.rs:146`（新出现的 clippy 常量断言）。按队长口径，这一窗口的偶发红不作本批回归；**本批 12 个交付文件在该窗口内哈希零漂移**。
- **披露（我自己的误操作，零后果）**：在核查 cargo git 缓存时，我的 glob 没匹配到 checkout 子目录，`git -C ""` 于是在**真实仓库**里执行了一次 `git fetch origin main`（只写 `.git` 的 `FETCH_HEAD`/远端跟踪引用；`git log -1` 仍是 5f408c8、工作树 62 项/51 个 M 与 fetch 前一致，无 checkout/restore/stash、无 commit）。特此记录，避免落成未申报的动作。
- **写作收尾时（20:56）core 本身也进了在飞状态**：`error[E0063]: missing \`module_cfgs\`` → `xirang-core` 编译不过（B4 正在给结构体加字段）。这进一步说明真实树此刻无法用于判定，本报告的判定取 20:28 校验修订 + §8 表内两条已归因的真实树红点。
- 我也试过"构造干净修订"（把 B4 在飞的 `build_method/{lib,pipeline,discovery}.rs` 还原成 HEAD 后重跑）：clippy 仍红，因为 B4 同时在改 `face_view.rs` 的导入，HEAD 与在飞文件的组合本身不自洽——**不做混合修订的判定**，只如实记下。

## 9. 发现清单

| id | 严重度 | 位置 | 问题 | 建议 |
| --- | --- | --- | --- | --- |
| N-1 | low | `CHANGELOG.md:154`、`:1334` | t17/t18 把 `slot` 收成 `registry_name` 后，CHANGELOG 0.1.6 那两条 `xirang.search` bullet 仍写 "module and slot name" / "模块与槽位名"（同一概念）。他们的声明只覆盖点名的 10 个点，故不算伪证；但"一个概念一种说法"在仓库范围不成立。 | 这两处改成与 mcp 侧一致的 `registry_name`（CHANGELOG 本就在 t19/t21 的 inScope 内）。 |
| N-2 | blocker（对本批"字面全绿"，非 t13–t24） | `studio/studio/app/tests/edit.rs:146`、`run_method/runtime/trace/artifact/parse_tests.rs:146`、`build_method/{lib,face_view,pipeline}.rs`、`studio/.../project.rs` | 写作时真实树/副本的门禁红点全部落在 B4 在飞文件或已报备的离线 git 缓存环境用例上；本批 12 个交付文件哈希零漂移。 | B4 落盘后重跑五条；`parse_tests.rs:146` 的常量断言需要 clippy 认可的形状（`const _: () = assert!(…)` 或比较非 const），catalog.rs 已回到 600 行（写作时点）。 |
| N-3 | info | `studio::app::tests::project::new_project_and_explicit_root_face_compile` | 副本/冷 target 下红是离线 git 缓存（只有 0.1.5），真实树绿；重复出现，别再当回归。 | 记在案；若要在副本里跑 workspace 门禁，先预热缓存或跳过该用例并声明。 |

其余：t13–t24 的交付**没有**"已修但没证据"或"说法比实现宽"的残留（逐条自查见 §2–§6；唯一不可复现的自证是 t24 的"断言逐字不变"，已单列）。

## 10. 未覆盖范围

1. **t24 的搬移前后逐字对比**：需要搬移前快照（t22 的内联版本），我手上没有；我只验证了搬移后的位置、挂载形态、名字与断言在变异下会红。
2. **t23 的 BR-C7 行为不变探针**（预览 diff 的 sha256）：未复跑他们的脚本；我只验证了词形单源与耦合钉子。
3. **t13 的 20 万随机串**：我用自写字符集做了 6.8 万条（结论一致），没有逐字复跑他们的语料。
4. **t17/t18 的其余面**：我扫了全仓 `slot name`/`槽位名` 并逐个归类，但没有逐字重解析他们那份"10 个点名点"清单之外的其它文案。
5. **B4 在飞批次**（core snapshot/tokens、studio b4_studio、build_method discovery 等）：不在本任务 objective 内，我只用它们解释门禁红。
6. 完整 external rehearsal / nm 缺失等环境实验（t9 已列，本任务未重复）。
