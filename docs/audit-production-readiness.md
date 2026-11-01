# Production-readiness audit of `b963632` — full punch list
# `b963632` 投产就绪审计——全量清单

This is the full, de-duplicated punch list for "can a real user run this today".
It merges five read-only audits run against `main = b963632` (working tree clean):
one release-mechanics audit, one kernel panic/truncation audit, one
execution-surface audit, one audit that re-verified every finding recorded in
`docs/audit-2026-09-21.md`, `docs/audit-adversarial-b1-b3.md`,
`docs/audit-graft-vs-readme.md`, `docs/threat-model.md` and `docs/roadmap-1.0.md`,
plus my own reproduction of the worst items against the real binary.
Nothing here is a style preference: every row names the code that settles it.
本文是"今天真的能让用户跑吗"这一问题的全量去重清单。它合并了针对
`main = b963632`（工作树干净）的五份只读审计：发布机制审计、内核 panic/截断审计、执行面
审计、对 `docs/audit-2026-09-21.md` / `docs/audit-adversarial-b1-b3.md` /
`docs/audit-graft-vs-readme.md` / `docs/threat-model.md` / `docs/roadmap-1.0.md`
中每一条既有结论的复核审计，以及我自己对最严重几条用真实二进制的复现。这里没有一条是风格
偏好：每一行都点名了能settle它的代码。

Severity, and what each tag means:
严重度，以及每个标记的含义：

- **CRITICAL** — a real user hits it on a plausible input, and the result is a
  crash or data written to the wrong place.
- **MAJOR** — wrong answer, an unenforced security control, a silent partial
  success, or a contract the code contradicts.
- **MINOR** — documentation truth, hygiene, consistency, or a low-probability
  edge.
- `[实测]` reproduced against the real binary in this audit; `[代码]` settled by
  reading the cited code; `[报告]` reported by a delegated audit and not
  independently re-run (the ones that matter were [实测]/[代码]).
- **CRITICAL** —— 用户用正常输入就会撞上，后果是崩溃或写错位置。
- **MAJOR** —— 错误答案、未生效的安全控制、静默的部分成功、或代码与自己的契约矛盾。
- **MINOR** —— 文档真实性、卫生、一致性或低概率边界。
- `[实测]` 本轮用真实二进制复现；`[代码]` 由所引代码判定；`[报告]` 由委派的子审计报告、
  我未独立复跑（要紧的都已复核为 [实测]/[代码]）。

The good news, recorded first so it is not lost under the list: the kernel's own
contracts are pinned (four `NodeId` literals + the SHA-256 NIST vector + graft
plan `version=1`), kernel purity/mounting/size/`missing_docs`/doc-parse are
executable gates, every gate battery is green (`fmt`, clippy both feature sets,
401 tests default, 436 `--all-features`, 395 in release, doctests, doc
`-D warnings`, `--locked` build, package contents for all nine crates — those are
the counts at audit time; the status section below has them after the fixes), and
the real CLI's happy path and its documented failure path both behave.
先说好消息，免得它被清单淹没：内核自身的契约是被钉住的（四个 `NodeId` 字面量 + SHA-256 的
NIST 向量 + graft plan `version=1`），内核纯净性/模块挂载/尺寸/`missing_docs`/文档解析都有可
执行门禁，各套门禁全绿（`fmt`、两套特性的 clippy、默认 401 条测试、`--all-features` 436 条、
release 模式 395 条、doctest、doc `-D warnings`、`--locked` 构建、九个 crate 的包内容——这些是
审计当时的数字，修完之后的数字见下面的状态一节），真实 CLI 的成功路径与它文档化的失败路径都
符合预期。

## Status of the list / 清单状态

Updated as the fixes landed; this is the state after the tenth batch.
随修复落地而更新；下面是第十批之后的状态。

- **40 of 48 rows are `✅ FIXED`** — all four CRITICAL, all fifteen MAJOR, all
  sixteen MINOR and all five RELEASE items. Each row carries its own fix, the
  evidence that was run, and the test, gate or tool that keeps it from coming
  back.
- **5 rows are measurements rather than fixes, and are marked as measured**: U1
  (no linked artifact carries an `.inventory` section), U2 (allocation counts on
  the release read path), U3 (the external-path rehearsal, now reproducible as
  `tools/xirang-external-rehearsal` and observed at 27 passing tests), U4 (what
  a declared Wasm table costs the host), U5 (`CallEvidence::Live` reachability).
- **Re-verification earned its keep**: checking the five measured rows turned up a
  second gap in M1's fix — its first guard measured delimiters and generic
  arguments, so seven other shapes (`& & & …`, `* * * …`, `1 + 1 + …` and four
  more) still aborted the process. That row records the finding, the third measured
  shape, and the gate that now refuses nothing in this repository.
- **The gates at the end of that work, all offline**: `fmt --check` clean, 453
  tests passing by default and 492 with `--all-features` (0 failures either way),
  clippy `-D warnings` clean with and without `--all-features`, `cargo doc -D
  warnings` clean, doctests clean, `--no-default-features` clean, and two checks
  that need no network at all — `tools/xirang-publish --check-table` and
  `tools/xirang-external-rehearsal` (27 tests, the example hosts built outside
  the checkout).
- **3 rows cannot be settled in this checkout and are recorded as unverified with
  the reason, rather than assumed true**: U6 (editor completion counts are
  LSP-only), U7 (the revisions the second and third rounds compared against are
  not in this repository's history), U8 (an online `cargo publish --dry-run`
  needs the network and a token).
- **What was left to a human is done, and one decision is now made**: `v0.1.0` was
  tagged and pushed, the `CARGO_REGISTRY_TOKEN` secret was set, and on 2026-09-25
  `tools/xirang-publish --publish --yes` took all nine crates to crates.io —
  stopping once on crates.io's new-crate rate window and finishing on a re-run —
  with `--verify-consumers` green in the same run. **The version line stays on
  `0.1.x`** (decided 2026-09-25): the public surface is not frozen, each release is
  a small step, and raising the line to `1.0.0` remains a separate later decision.
  The run's evidence is in the P4 section of
  [`audit-2026-09-25-post-fix.md`](audit-2026-09-25-post-fix.md).
- **已交给人的事已完成，其中一个决定也已经做出**：`v0.1.0` 已打 tag 并推送、
  `CARGO_REGISTRY_TOKEN` 已配置；2026-09-25，`tools/xirang-publish --publish --yes`
  把九个 crate 送上 crates.io——中途因 crates.io 的新 crate 速率窗口停过一次，重跑后完成——
  同一次运行的 `--verify-consumers` 通过。**版本线保持 `0.1.x`**（2026-09-25 决定）：公开面
  未冻结、每次发布都是一小步，抬到 `1.0.0` 仍是以后单独的决定。运行的证据见
  [`audit-2026-09-25-post-fix.md`](audit-2026-09-25-post-fix.md) 的 P4 节。
- **40 / 48 行是 `✅ FIXED`**——四条 CRITICAL、十五条 MAJOR、十六条 MINOR 与五条 RELEASE
  全部在内。每行都带自己的修法、跑过的证据，以及防止它复发的测试、门禁或工具。
- **5 行是实测而不是修复，并已标为已测**：U1（已链接产物不含 `.inventory` 段）、U2（发布读路径
  的分配计数）、U3（外部路径演练，现在是可复跑的 `tools/xirang-external-rehearsal`，实测 27 条
  测试通过）、U4（声明的 Wasm 表在宿主一侧的真实开销）、U5（`CallEvidence::Live` 可达性）。
- **复核是有回报的**：核对那五行实测项时，找出了 M1 修法里的第二个缺口——它的第一版守卫只量
  定界符与泛型实参，于是另外七种形状（`& & & …`、`* * * …`、`1 + 1 + …` 等）依然会打死进程。
  该行记录了这次发现、新增的第三种形状，以及那道"本仓库没有一个文件会被拒"的门禁。
- **那次工作结束时的门禁，全部离线**：`fmt --check` 干净、默认 453 条测试通过、`--all-features`
  492 条（两者都是 0 失败）、两套 clippy `-D warnings` 干净、`cargo doc -D warnings` 干净、
  doctest 干净、`--no-default-features` 干净，以及两项完全不需要网络的检查——
  `tools/xirang-publish --check-table` 与 `tools/xirang-external-rehearsal`（27 条测试，
  示例宿主在检出之外构建通过）。
- **3 行无法在本检出定案，因此带原因记为未验证，而不是默认成立**：U6（编辑器补全条目数是
  LSP-only）、U7（第二轮与第三轮据以比较的旧修订不在本仓库历史里）、U8（在线
  `cargo publish --dry-run` 需要网络与 token）。
- **剩下的属于人而不是代码**：`git tag -a v0.1.0` 与 push、以及创建一个
  `CARGO_REGISTRY_TOKEN` secret（R2），然后 `tools/xirang-publish --publish --yes` 与
  `tools/xirang-publish --verify-consumers`（R1）；还有 `0.1.0` 与 `1.0.0` 的版本线选择
  （m2）。这些行各自都写明了这一点。

## CRITICAL / 致命

- **C1 ✅ FIXED CRITICAL 宿主 `src/` 下只要有普通平铺 `.rs`，`check`/`build`/`explain`/`grafts`
  与宿主自己的 `cargo build` 都会 panic** `[实测]` —
  `build_method/discovery.rs:10-11`（`read_dir(src).expect`）、`:20-23` 与 `:57-60`
  （`panic!("registration source ... must use `<name>/<name>.rs` layout")`）；同一条
  `discover_root` 由宿主的 `build.rs` 经 `build_method/lib.rs:119-122` →
  `pipeline.rs:14` 调起，CLI 四条命令经 `cli/lib.rs:120-124` 调起。
  实测：`./target/debug/xirang check debug_method` → 退出 101、stdout 空；把
  `examples/control-button`（合法宿主）复制到 `/tmp` 并加一个 `src/helpers.rs` → 同样
  退出 101。`discovery.rs:16` 只豁免 `lib.rs`/`main.rs`/`bin`，这条布局规则**没有写进任何
  用户文档**（README 的 `## Boundaries` 未提），而且它违反 `cli/README.md:40-41` 自己写下的
  失败契约（"失败也要把诊断文档写到 stdout，然后以非零退出"——panic 时 stdout 是空的）。
  **Fixed:** `discovery.rs` 新增 `UnplacedFace` 与 `discover_root_reporting`：不是注册面的
  平铺 `.rs` 静默跳过（复用 `parse_face` 的 `Ok(None)`，因此"是不是面"的判定与构建其余部分
  同一处），而是面的文件变成 `phase=face-layout` 诊断并指出该移到哪；`pipeline::run` 在解码
  字段的各阶段之前收集它。实测：合法宿主 + `src/helpers.rs` → `check` 退出 0；平铺文件里的面
  → 退出 1 且 `check --json` 输出完整十一键文档。Pinned by
  `build_method/pipeline.rs::an_ordinary_flat_module_is_not_a_registration_source` 与
  `::a_face_outside_the_layout_is_reported_with_its_path`。
  该布局规则已随 m16 写进 README 的 `## Boundaries`（`README.md:787-789`、
  `README.zh-CN.md:703`，中英同步），因此 C1 没有遗留项。
- **C2 ✅ FIXED CRITICAL 畸形注册面 panic 而不是诊断** `[实测]` —
  `build_method/registration_check.rs:104-105` 与
  `build_method/validation.rs:170-171` 用 `unwrap_or_else(|error| panic!(...))`，
  由 `pipeline.rs:34,37-40` 触达。后果与 C1 相同：`check --json` 不产出任何 JSON 文档。
  **Fixed:** `validation::parsed_face` 不再 panic（返回 `None`，下游所有 `&& let Some(face)`
  链因此自然跳过），`validation::face_syntax_errors` 在任何解码阶段之前把
  `phase=face-syntax` 诊断收进 `compile_errors`，`registration_check` 也改为跳过；入口侧
  （`entry::application_entry_source` 的读失败/解析失败/非 `crate::`/不可解析/多条声明）与
  `scope`/`graft-entry` 的 panic 一并改成诊断，见 **m9**。实测：`examples/control-button`
  的副本里放一个未闭合的 `src/broken/broken.rs`，改前退出 101、stdout 空；改后
  `check --json` 输出一个文档、`count: 2`（`phase=entry` 与 `phase=face-syntax` 都指向
  `broken/broken.rs:1`），退出非零。Pinned by
  `build_method/pipeline.rs::a_malformed_face_is_reported_instead_of_aborting`。
- **C3 ✅ FIXED CRITICAL Studio 会认错项目树，并可能把新面写进 XiRang 自己的源码目录**
  `[实测]` — `studio/studio/app/support.rs` 的根回退是
  `env!("CARGO_MANIFEST_DIR").parent()`（编译期 crate 源码目录），而
  `core/src/registry_core/lexicon/lexicon.rs:228-238` 对 `XIRANG_PACKAGE_ROOT`
  **原样采信、不检查存在性**；随后 `source_index.rs` 返回空注册表，`lifecycle.rs:86` 照样显示
  "Ready…"，进程退出 0；按 `a`+`s` 会在那个回退目录里创建 `src/<name>/<name>.rs`。
  **Fixed:** `support.rs` 新增可测的纯规则 `resolve_project_from(selected, explicit,
  configured, current, current_holds_package) -> Result<PathBuf, String>`（顺序：本会话选择 →
  显式路径 → 环境变量 → 持有 `Cargo.toml` 的当前目录），四个候选全部**按名字拒绝**不可用者；
  `CARGO_MANIFEST_DIR` 回退已删除，兜底改为当前目录。`app::preflight` 在 `launch_with` 里、
  **接管终端之前**运行，失败即打印一行并退出 1（见 M10）。独立二进制现在还接受
  `xirang-studio [PROJECT]`（`studio/main.rs`），并支持 `--help`。
  实测：`XIRANG_PACKAGE_ROOT=/nonexistent` 或从无 `Cargo.toml` 的目录启动 → 退出 1 且消息
  点出变量/路径。Pinned by
  `studio/studio/app/tests/project.rs::{an_unresolvable_project_is_refused_instead_of_falling_back,
  only_a_working_directory_that_holds_a_package_is_opened,
  a_relative_candidate_resolves_against_the_working_directory}`。
- **C4 ✅ FIXED CRITICAL Studio `e` … `s` 静默整文件重写，手写内容被丢且无备份** `[实测]` —
  `overlay/edit.rs` → `mutations.rs:180-182` →
  `run_method/authoring/operations/operations.rs` 的
  `atomic_write(&source, &rendered)`；重写从固定字段集重建整个文件，只有 `plugin:`
  被护住（`render.rs:22-28`），其余手写项与注释会被丢掉，且没有任何备份。
  **Fixed:** 重写前把先前的文本存进删除路径本就使用的回收目录
  （`<package>/.xirang/trash/faces/<file>-<id>-<nanos>.rs`），并把路径写进返回消息，因此
  读者看得到旧文本去了哪里；重写没有实质变化时不产生备份。新增
  `run_method/authoring/operations/trash.rs`（回收目录、纳秒时间戳、`stash_face_source`），
  并把 `delete_module` 拆到 `delete.rs` —— 这同时是一笔尺寸还款：`operations.rs` 429 → 423 行，
  离 450 的余量从 21 行回到 27 行，而 "不许新增超标条目" 的棘轮规则因此没有被触碰。
  实测：给一个注册面手写一行注释再经 `e` … `s` 保存，重写后的文件里没有那行，而
  `.xirang/trash/faces/` 下恰好一份备份含有它。Pinned by
  `studio/studio/app/tests/edit.rs::a_rewritten_face_keeps_its_previous_text_in_the_trash`。
  **仍可选（未做）:** 写入前显示 diff 并要求确认；备份是"可恢复"，确认是"不会发生"。
- **M1 ✅ FIXED MAJOR 内核 `syn` 解析无递归深度上限 → 病态嵌套直接 abort（绕过 `Result`）**
  `[实测]` — 入口：`core/src/registry_core/syntax/face.rs:149,238`、
  `syntax/entries/graft.rs:327`、`syntax/entries/application.rs:26`、
  `authoring/parse/parse.rs:107`、`authoring/parse/flow.rs:98,110`、
  `syntax/fields.rs:177,205`。`syn 2.0.119` 自身无深度守卫，而下一层的 `proc-macro2`
  显式防了它自己（fallback 词法器注释 "Nonrecursive to prevent stack overflow"）。可达面是
  "项目不拥有的文本"：Studio 的作者编辑缓冲区（`studio/studio/app/keyboard.rs:172`）
  与 authoring。触发量级约上万层括号，与栈大小相关。复现：60k 层配平括号 / 60k 层花括号 /
  50k 层 `Vec<…>` 三类在改前均以 `fatal runtime error: stack overflow, aborting`（SIGABRT）
  打死整个测试进程；未闭合的一串不溢出（`proc-macro2` 词法器先拒）。
  **Fixed:** `core/src/registry_core/syntax/nesting.rs` 在 `proc-macro2` 的 token 流上迭代地量
  定界符组深度与泛型实参链深度，超过 128（`rustc` 自己的默认 `recursion_limit`）即返回
  `FaceSyntaxError`；三个 `syn::parse_file` 入口改走 `nesting::parse_file`。量在 token 流上
  而非原始文本上，因此字符串/注释里的括号不会被误判。Pinned by
  `core/src/registry_core/syntax/deep_input_tests.rs`（四种形状）。
  仍未覆盖：`authoring` 里三处对**字段取值**的 `syn::parse_str`（`parse.rs:107`、
  `flow.rs:98,110`）依赖"取值是已扫描源码的子串"这条传递性论证，尚未各自前置扫描。
- **M2 ✅ FIXED MAJOR 插件信任链半接线：签名保证不可达、吊销不在激活路径上** `[实测]`+`[代码]` —
  `core/src/registry_core/plugin/artifact/artifact.rs` 的 `PluginArtifact::verify_artifact` 是
  `PluginAssurance` 的**唯一构造点**，永远写 `Digest`；`plugin/slot/slot.rs:94` 却要求 Official
  通道的工件带 `PluginAssurance::Signature` → **Official 通道恒不可入**（失败关闭，安全但功能死）。
  `verify_with`（策略层，已含撤销检查）与 `Ed25519Verifier` 都在，缺的只是把验证结果记下来的那一步。
  **Fixed（接线而不是删除）:** 新增 `PluginArtifact::verify_signed(policy, verifier)`：先跑
  `policy.verify_with`（摘要 → **撤销** → 官方密钥指纹 → 签名），通过后记录
  `PluginAssurance::Signature`。这是 `Signature` 的唯一构造入口，也就是 Official 通道唯一能接纳
  工件的途径。实测三组：已签名工件拿到 `Signature` 且 `validate_artifact(... Official ...)` 通过、
  纯摘要工件保持 `Digest` 且在同一通道被拒、已吊销版本即使签名会被宿主接受也报 `Revoked`、
  被拒签名报 `SignatureNotVerified`。Pinned by
  `plugin::artifact::tests::{a_signed_artifact_is_the_only_way_into_the_official_channel,
  a_revoked_version_is_refused_even_with_a_good_signature,
  a_refused_signature_never_earns_the_stronger_assurance}`。
  **仍如实记录:** `PluginPolicy::decision` 与 `PluginCatalog::contains_manifest` 在仓内仍无调用者
  ——它们是**给宿主的**审计/查询 API（Studio 走的是同一把锁上的 `PluginCatalog::contains`），
  因此"来源与版本在发布/激活前未被校验"这一半，现在由 `verify_signed` 的策略链承担；这两个函数
  本身不是"未接线的控制"，而是宿主的选择入口。
  （2026-09-28 复核：`[实测]` 这两处在仓内**确有**调用者——`plugin-host/src/admission.rs:166` 调
  `PluginPolicy::decision`（内核之外），`core/src/registry_core/plugin/plugin_policy/plugin_policy.rs:125`
  的 `accepts` 与 `:157` 的锁检查分别调 `decision` 与 `contains_manifest`（内核之内）。因此"仓内仍无
  调用者"作为断言已不成立；原文保留，因为它是当时那一轮的记录，订正见 09-27 索引 `PH-6`。）
- **M3 ✅ FIXED MAJOR wasm 表元素无上限，编译期限制与工件字节也未限** `[报告]`+`[实测]` —
  `plugin-host/src/wasm.rs` 设了 `memory_size`/`instances`/`memories`/`tables`/
  `trap_on_grow_failure`，但**从未调用 `.table_elements(..)`**；wasmi 默认不限表元素且表是
  即时实例化的，因此 `(table 100000000 funcref)` 会让宿主在 `memory_bytes` 之外分配数百 MB。
  另外 `Config` 只设了 `consume_fuel(true)`（`EnforcedLimits` 与
  `min_avg_bytes_per_function` 保持默认=无限），`Module::new` 前的工件字节也无上限。
  **Fixed:** `WasmLimits` 新增两个有文档的字段——`table_elements`（默认 4096）接到
  `StoreLimitsBuilder::table_elements`，`max_module_bytes`（默认 16 MiB）在 `Module::new`
  **之前**手工检查（编译先于任何运行期限制，因此这一条只能手工查），并接上 wasmi 自己的
  `EnforcedLimits::strict()`（它的数值，不是这里编的）。实测三组：`table_elements: 0` 时连
  1 元素的表也无法激活、默认上限下 1 亿元素的表被拒、超上限工件报 `artifact is …`。
  Pinned by
  `fault_matrix.rs::wasm_faults::{the_table_element_limit_is_enforced, a_huge_table_is_refused,
  an_oversized_artifact_is_refused_before_compilation}`。
  测试期间发现并记录了一条 API 事实：**安装是惰性的，激活发生在第一次 `call`**，因此这三条
  测试必须先 `install` 再 `call` 才能触达限制。
- **M4 ✅ FIXED MAJOR 进程后端只 drain 到第一帧：合法帧 + 之后 >64 KiB 洪水 ⇒ Timeout
  且丢弃有效响应** `[报告]`+`[实测]` — `process.rs` 的 stdout 线程只调一次 `read_frame`，
  尽管它上面的注释已经写着"在整个子进程生存期内排空 stdout"。
  **Fixed:** 帧**先**送出（延迟不变），随后 `child::drain_to_eof` 把 stdout 读到 EOF——这正是
  `read_stderr` 早已遵循的同一条规则。实测：一个 `cat <帧>; head -c 200000 /dev/zero` 的子进程
  现在交付答案；把 `drain_to_eof` 去掉后同一条测试失败（退出状态 141 / SIGPIPE），证明它真的
  钉住了旧缺陷。Pinned by
  `fault_matrix.rs::process_faults::a_child_that_writes_past_its_answer_still_delivers_it`。
- **M5 ✅ FIXED MAJOR 运行期检查的 i64 界限经 f64 比较，越界判定错误** `[报告]` —
  `core/src/registry_core/declaration/runtime_checks.rs` 的
  `check_number_range` 用 `*value >= min as f64 && *value <= max as f64`：`|bound| > 2^53`
  时 `as f64` 会舍入，于是 `number_in_range(9007199254740993, 9007199254740993)` 接受
  `9007199254740992.0`——一个低于它自己下限的数。输入来自 `RuntimeCheckSpec::parse_list`。
  **Fixed:** 先拒绝 f64 无法精确说出的边界（`(bound as f64) as i128 != i128::from(bound)`，
  用 `i128` 是因为它不像 `as i64` 那样在两端饱和），把它当作创作错误报告；能精确表示的边界
  照常比较。Pinned by
  `runtime_checks_tests::number_in_range_refuses_a_bound_it_cannot_state_exactly`（两种情形都测：
  不可表示 → 拒绝，恰好可表示的 `2^53` → 照常判定）。
  顺带一笔还款：该文件的测试模块搬成 `runtime_checks_tests.rs`，棘轮条目从 **751 → 551**。
- **M6 ✅ FIXED MAJOR MCP 树遍历跟随符号链接目录，可读出包根之外** `[报告]` —
  `mcp/index.rs` 的 `StdSourceTree::is_directory` 是裸 `path.is_dir()`（跟随链接），而
  `load_file` 只用语法 `strip_prefix` 做根检查：根内一个指向根外的链接能通过前缀比较，整棵
  外部树因此被拉进索引，其中的文件也进了回答。`inspect`/`read` 走的 `load_one` 本来就
  canonicalize ✓。
  **Fixed:** `StdSourceTree` 现在携带**一次解析好的规范根**；`is_directory` 对解析到根外的目录
  返回 false（不进入），`read_text` 与 `load_file` 各自再做一次 canonicalize 检查（纵深防御），
  而 `load_sources` 过滤掉解析到根外的条目——跳过而不是报错，因为根就是这类回答所声明的范围，
  而对同一路径的**直接** `inspect`/`read` 仍按名字拒绝。Pinned by
  `index::tests::a_link_out_of_the_source_root_is_neither_walked_nor_read`（unix symlink）与
  `::a_file_inside_the_source_root_is_still_read`（守卫针对位置而非链接本身）。
- **M7 ✅ FIXED MAJOR `xirang mcp` 永远不会以非零退出** `[报告]` — `cli/lib.rs` 调
  `xirang_mcp::run()`（返回 `()`）后 `Ok(())`；`mcp/protocol.rs` 的循环在 stdin 读错或
  stdout 写失败时静默 `break`。传输层失败 ⇒ 退出 0、stderr 无消息。
  **Fixed:** `run()` 改为 `Result<(), String>`；分帧抽成可测的 `run_with(&mut dyn BufRead,
  &mut dyn Write)`（两个值得钉住的失败在真实终端上无法从测试里造出来）；CLI 用
  `map_err` 传播，独立二进制打印一行并以 1 退出（与 studio 的入口同形）。Pinned by
  `protocol::tests::{an_unwritable_response_is_reported, a_closed_input_ends_the_session_cleanly}`。
- **M8 ✅ FIXED MAJOR `grafts` 静默部分成功** `[报告]` — `cli/grafts.rs` 用
  `face_views(...).unwrap_or_default()` 丢掉源码树读取错误（声明那一列于是变成猜的），
  `plan_rows` 把读不了的计划目录变成"没有计划"，不可读的宿主入口只打印而退出码仍是 0。
  **Fixed:** 三处读取失败都收集进 `problems`；`plan_rows` 改为 `Result<Vec<Value>, String>`
  （**不存在**目录仍是 `Ok(空)`，存在却读不了则是错误）；报告照常写出，然后由
  `report_problems` 返回 `Err("incomplete: could not read …")`，因此 `--json` 的读者既拿到
  可读的那些条目、又看到非零退出码。`explain --overlay` 同一函数改为传播而不是报告"没有计划"。
  **有意保留退出 0 的情形:** 某条计划本身解析不了——这正是本命令要报告的东西（"这份能不能用"
  的答案是"不能"），不是回答不出来。Pinned by
  `tests::{grafts_reports_an_unreadable_plans_directory, grafts_reports_a_host_whose_sources_cannot_be_read}`。
- **M9 ✅ FIXED MAJOR `xirang-dev` 在 Studio 子进程崩溃时仍报成功** `[报告]` —
  `studio/bin/xirang-dev.rs` 对任何退出状态都 `Ok(())`，于是"启动即崩"与"读者正常退出"
  对调用方是同一件事。
  **Fixed:** 抽出 `exited(status)`：成功状态仍是 `Ok(())`，非零则返回
  `Err("Studio exited with {status}")`，由 `main` 打印一行并以 1 退出。Pinned by
  `xirang-dev::tests::a_studio_that_exits_non_zero_is_reported`（跑真实的 `sh -c "exit 7"` /
  `exit 0` 两种子进程；该二进制在 `dev-supervisor` 特性后，CI 的 `--all-features` 任务覆盖它）。
- **M10 ✅ FIXED MAJOR Studio 在项目缺失/无效时退出 0 且界面为空** `[实测]` — 与 C3 同一根因；
  `source_index.rs` 对不存在的根返回空注册表，`lifecycle.rs:86` 显示 "Ready…"，只有终端初始化
  失败才会让入口非零。
  **Fixed:** `launch_with` 在接管终端前运行 `app::preflight`，失败即 `Err`；`main` 打印一行
  `xirang-studio: …` 并 `exit(1)`（不再打印两次，也不再出现 `Error: Custom { … }` 调试外衣）。
  实测：缺项目、坏环境变量、无 TTY 三种情形都退出 1。Pinned by
  `studio/tests/launch.rs::{a_path_that_is_not_a_project_fails_with_a_message,
  a_configured_root_that_is_not_a_directory_fails_with_the_variable_named,
  help_prints_usage_and_succeeds}`（进程级：真实二进制的退出码与 stderr）。
- **M11 ✅ FIXED MAJOR 源码树遍历没有 visited 集，目录符号链接环会无限递归** `[代码]` —
  `core/src/registry_core/source/walk.rs` 的 `collect_rust_sources` 是全工作区唯一的递归遍历，
  `SourceTree` 若返回环（例如执行面未 canonicalize 的 symlink 环）就递归到栈溢出。
  **Fixed:** 新增公开常量 `MAX_DEPTH = 128` 与一层 `depth` 参数：超过上限即返回
  `Err("source tree nests deeper than 128 directories at …; it is cyclic, or too deep for this bound")`。
  选上限而不是 visited 集，是因为内核的文件系统事实来自调用方、**无法 canonicalize 路径**，
  分不出"指向祖先的链接"与"确实很深的目录"。Pinned by
  `walk::tests::{a_cyclic_tree_stops_at_the_depth_bound, a_flat_tree_yields_every_rust_file}`。
- **M12 ✅ FIXED MAJOR 每个函数都从偏移 0 重数换行（O(n·f)），MCP 索引会走这条路** `[报告]` —
  `core/src/registry_core/source/source.rs` 对每个函数做两次
  `source[..offset].bytes().filter(...).count()`，而 `mcp/index.rs` 在全树上调用它。
  **Fixed:** `function_symbols` 改为一次前向扫描：它报告的两个偏移都是非递减的（循环从刚取下的
  函数末尾继续），因此从上次的偏移继续数即可，整趟变线性；`target < counted_to` 保留一条正确性
  兜底（旧代价、正确答案），而不是给出错答案。Pinned by
  `source::tests::every_function_reports_its_own_lines_in_a_long_file`（200 个函数逐个断言
  `line`/`end_line`）。
- **M13 ✅ FIXED MAJOR Studio 删除 graft 记录是单键无确认，且坏记录删不掉** `[实测]` —
  `studio/studio/app/overlay/graft.rs` 的 `d` 直接调用删除；
  `run_method/authoring/external_graft/plan.rs` 的 `remove_external_graft` 先
  `read_external_graft` 再改名，因此**读不懂的记录一条也删不掉**——而那恰恰是最需要能删掉的
  情况。
  **Fixed:** ①删除不再以解析为前提：新增 `external_graft_directory(selector)`（校验选择器是
  目录名而不是路径，并确认目录存在），`remove_external_graft` 只解析目录再改名进 trash，
  字节保留，因此删错仍可手工恢复；②`d` 改为**两次确认**：第一次只进入待删状态并在事件行点名
  记录，任何其他键解除，第二次才移动（状态存在 `GraftState::pending_delete`，因此提示画得
  出来），页脚改为 `d delete (press twice)`。Pinned by
  `run_method/authoring/external_graft/plan.rs::tests::a_broken_record_can_still_be_removed`
  与 `studio/studio/app/tests/graft.rs::graft_composes_an_external_overlay_plan_without_touching_source`
  （已扩为：一击不删、别的键解除、两击才删）。
- **M14 ✅ FIXED MAJOR Studio 新建项目与插件选择无确认，且可能留下半个项目** `[实测]` —
  新建项目的相对目录按 Studio 的 CWD 解析、脚手架中途失败留下部分目录、空目录检查与写入之间
  有 TOCTOU；插件选择在单次按键上追加 `use <crate> as _;` 与锁行，锁写失败会留下半对文件。
  **Fixed:** ①相对目录改为相对**打开的项目**解析（`package_root()`），不再相对进程 CWD；
  ②`build_method/scaffold/project.rs` 新增 `write_project(root, files, existed)`：失败时
  移除自己写下的内容；若目录本来就存在（TOCTOU 窗口内可能有别人的文件）则**不删**，而是把
  "留下了部分项目"写进错误消息——两种情形都有测试；③插件写入改为**两次确认**（
  `PluginState::pending_submit`，任何其他键解除），锁写失败时把入口文件恢复成先前的字节
  （原本不存在则删除），并在事件行说明；两次追加共用新的 `append_line` 助手。
  Pinned by `build_method/scaffold/project.rs::tests::a_failed_scaffold_removes_what_it_wrote`
  与 `studio/studio/app/tests/forms.rs::a_plugin_selection_takes_two_presses_and_rolls_back_a_half_write`。
  **有意未加:** 新建项目向导的第二道确认。它的 `s` 已经是一次对三字段表单的显式提交，而
  M13/M14 里加两击确认的两处都是"列表行上的一次裸按键"；这一条按后者的形态记录在案。
- **M15 ✅ FIXED MAJOR 宿主 crate 在 `--cfg rust_analyzer` 下对嵌套面编译失败** `[实测]` —
  实测 `cargo rustc -p xirang-example-control-button --lib --offline -- --cfg
  rust_analyzer` → 退出 101，`generated_lib.rs` 报 `E0433 cannot find registry_rule in super`。
  根因写在渲染器自己的注释里（`build_method/renderer/ide.rs`）：`rust-analyzer` 只在文件或
  展开的**顶层**应用 `#[path]`，因此嵌套面必须再有一条 crate 根影子声明，而那条影子里 `super`
  就是 crate 根；于是面里派生的规则路径 `super::registry_rule::REGISTRATION_RULE`
  （`macro/src/lib.rs` 的 `face_rule_or` 在 `needs_registry: true` 且作者省略规则时发出）在
  IDE 视角下解析不到，整个 crate 在编辑器的 cfg 下类型检查失败。
  **Fixed:** 让解析器为两种工具各答一次——`face_rule_or` 现在发出一个块：
  `#[cfg(not(rust_analyzer))] let __rule = super::registry_rule::REGISTRATION_RULE;`
  `#[cfg(rust_analyzer)] let __rule = <调用方的兜底值>; __rule`。`rustc` 编译的东西与改动前逐字
  节相同；IDE 侧拿到 `RegistrationRule::ANY`，因此镜像类型正确。实现细节：模板整体可解析，兜底
  token 由新增的 `front_end::splice` 原样拼进占位符（保留 span 与 `$crate` 卫生性）；第一版尝试
  在 `__registration_face!` 的**宏参数**上加 `#[cfg]` 字段，被下游宏匹配拒绝（`no rules expected
  '#'`），第二版用 `use` 导入兜底值，被 `RegistrationRule` 是结构体、关联常量不可 `use` 拒绝
  ——都记在这里，免得后人重走。实测：嵌套面宿主与只有镜像的宿主在 `--cfg rust_analyzer` 下均退出
  0。Pinned by `examples/control-button/tests/ide_mirror.rs`
  （`#[ignore]`：它把两个宿主在一条别处不用的 cfg 下重新构建一遍，用独立 target 目录避免与测试
  持有的构建锁互等），由 `.github/workflows/ci.yml` 的 "IDE mirror (rust_analyzer cfg)" 步骤显式
  运行——在这之前**没有任何步骤编译过这条 cfg**。派生规则的**值**不必新钉：既有的
  `parent_rule_rejects_a_child_that_misses_a_required_export` 只有在 `control` 面真的带着兄弟模块
  那条 `require_exports(["control.render"])` 规则时才会通过。

## MINOR / 次要

- **m1 ✅ FIXED MINOR CHANGELOG 记了一次没有发生的发布** `[实测]` — `CHANGELOG.md` 的中英两侧
  都写着 `[0.1.0] - 2026-09-23`／`首次发布（2026-09-23）` 与"九个 crate 已一起发布"，而
  `index.crates.io` 上九个名字全是 404、`git tag --list` 为空。
  **Fixed:** 标题改为 `[0.1.0] — not published yet`／`首次发布（尚未发布）`，正文改为"发布时一同
  发布"，并在文件头部加了一条**发布状态**说明（见 m2）。中英同步。**发布之后这处措辞已被再次
  更新**：标题现在是 `[0.1.0] — 2026-09-25`／`首次发布（2026-09-25 已发布）`，头部说明也改成
  "已发布 + 版本线 0.1.x"——记录在这里，免得读者把 m1 当时的措辞当成现状。
- **m2 ✅ FIXED（决策已下：版本线走 0.1.x） MINOR 版本口径三处不一致，且没有 tag 与发布流程**
  `[实测]` — 根 `Cargo.toml` 是 `version = "0.1.0"`（十二个成员全继承，历史里只出现过这一行），
  而提交信息写着 "XiRang 0.1.1"/"1.0.0"（都没改任何版本号），`docs/roadmap-1.0.md` 以 1.0 为名。
  **Fixed（口径）:** CHANGELOG 头部现在一句话说清：首个发布的版本是 `0.1.0`（即
  `[workspace.package]` 的值），"1.0"是里程碑名。原文里的"十四处内部 `version = "0.1.0"`"是个
  腐烂的数字（2026-09-25 实测 45 处），已改为不带数字的"每一处"。
  **已决定（2026-09-25）：版本线继续走 `0.1.x`。** 公开面尚未冻结，深化期间的每次发布都是 0.1.x
  的一小步（`0.1.1`、`0.1.2`……）；抬到 `1.0.0` 并冻结公开面是以后单独的一步，届时才需要连同每一处
  内部 `version = "0.1.x"` 要求一起移动。tag 与发布流程见 R2，实际运行见
  `docs/audit-2026-09-25-post-fix.md` 的 P4 节。
  **这一步已迈出（2026-09-26）：本检出是 `0.1.1`** —— 三方审查的收尾把工作区版本与每一处内部
  要求一同移到 `0.1.1`（`release_version` 门禁守住"一个来源"），`CHANGELOG` 的 `[Unreleased]`
  随之成为 `[0.1.1]`。这次推进同时让 P1 的签名 API 改动得以上线：包审计按设计跳过依赖尚未出现在
  index 上的 crate，因此它是绿的等待态，而不是红。**`v0.1.1` 已于 2026-09-26 打 tag 并推送**：
  发布工作流（run 36206756375）全部步骤成功，九个 crate 都上了 index，包审计现在报告九个全部
  `verified`、`skipped: none`。记录见 `docs/audit-3p-2026-09-25.md` 的续做一节。
  **再迈一步（2026-09-26）：本检出是 `0.1.3`** —— MCP 的写入路径（`xirang.apply`）与实测队列
  让版本线移到 `0.1.3`（`0.1.2` 从未发布，改动随 `0.1.3` 一起走），`v0.1.3` 已打 tag 并推送：
  发布工作流（run 36236583886）全部步骤成功，九个 crate 都上了 index，`--verify-consumers` 在
  同一次运行里按版本解析到全部九个，包审计报告九个全部 `verified`、`skipped: none`。
- **m3 ✅ FIXED MINOR README 安装说明面向 checkout，且一条命令跑不通** `[实测]` —
  `README.md` 的 `cargo run -p xirang-cli -- studio` 因该包有两个 bin 且无 `default-run` 而
  报 "could not determine which binary to run"（实测退出 101）。
  **Fixed:** `cli/Cargo.toml` 加 `default-run = "xirang"`（`cargo-xirang` 按名字照常可用），
  并给 README 的安装段补上事实（当时 crates.io 上什么都没有，Git 源是唯一能解析的来源，0.1.0
  发布后才换成 `cargo install xirang-cli`）。实测该命令现在解析到
  `Running target/debug/xirang studio`，只在无 TTY 时于终端步骤失败（预期）。
  **发布之后这处也已被再次更新**：两份 README 的安装段现在以 `cargo install xirang-cli` 为主，
  Git 源作为"想要检出最新提交时"的备选（原文"现在 crates.io 上什么都没有"已不成立）。
- **m4 ✅ FIXED MINOR README 仍有两条与代码不符的描述** `[实测]` — 键位表把 `1`–`4` 说成
  "Search, inspect, data, compare pages"，而只有三个页面（`4` 是空操作）；crate 表把
  `xirang-debug-method` 说成做 "MIR subprocess orchestration"，而该 crate 里没有任何
  `Command`/`spawn`。
  **Fixed:** 键位表改为 `1`–`3`（三个页面）、`Tab` 明确为"在树与数据面板之间"、并补上真实存在
  但缺失的 `p`（选择插件并记入锁）；crate 表把 `debug_method` 描述为 MIR 文本/JSONL 解析与合并、
  `CallTrace`/数据流模型与适配器，并写明**产出** MIR 的那次 `cargo rustc` 由 Studio 运行。中英同步。
- **m5 ✅ FIXED MINOR `mcp/README.md` 说诊断走 stderr，实际一处都没有** `[实测]` —
  `grep -rn 'stderr|eprintln' mcp/src` 零命中；错误是 stdout 上的 JSON-RPC 响应。
  **Fixed:** 中英两侧改为"**不写 stderr**：失败是 stdout 上的错误响应——客户端本来就在读那里"。
- **m6 ✅ FIXED MINOR `cli/README.md` 的 "Fields by phase" 行号已漂移** `[报告]` —
  它写 `contracts.rs:136,173,192,216`（实际 `:138,:157,:181`，`:216` 是测试）、
  `requirements.rs:70`（实际 `:98`，而且它在 `core`）、`topology.rs:41,49,68`（实际
  `:50,:58,:77`）、`lib.rs:158`（实际 `:168`）；`validation.rs:59,85,113`、`static_plan.rs:109`、
  `graft_plan_check.rs:157` 仍然对。
  **Fixed:** 行号这一类引用会自然腐化，因此第二列改为**符号**（`constructed by
  requirements::missing` 等），并把第五批新增的五个 phase（`face-layout`、`face-syntax`、
  `entry`、`scope`、`graft-entry`）按各自真正设置的字段补进表里；中英两份同步。
- **m7 ✅ FIXED MINOR Studio 的 DATA 面板标题是 "built-in sample"，却永远显示不出值** `[实测]` —
  `app/sample.rs` 只记录帧，而 locals 只由 `local`/`local_with_observation` 推入，因此
  `ui/graph/data.rs` 永远渲染 "· no live locals captured"；"built-in sample" 这个标题指的是一个
  永远显示不出任何数值的样本。
  **Fixed:** 样本现在用 `CallTrace::local`（`#[track_caller]`，来源位置自动取得）记录一个被观测到
  的局部值 `requested_width: u32 = 1280`，面板因此能演示它存在的意义；标题里的 "built-in sample"
  限定词保持不变（图例 `+ live` 描述的是证据标记词表，与样本无关）。Pinned by
  `sample::tests::the_built_in_sample_carries_one_observed_local`。
- **m8 ✅ FIXED MINOR `atomic_write` 会删掉已存在的同名临时文件** `[实测]` —
  `run_method/authoring/filesystem/filesystem.rs` 用固定临时名，并在复用前**先删除**它：
  一个恰好带着该名字的同级文件会被销毁，崩溃留下的残留也会被下一次写入清掉。
  **Fixed:** 临时名改为唯一形式 `.<name>.xirang-<pid>-<counter>.tmp`，只清理自己的临时文件
  （写入或改名失败时）。Pinned by
  `filesystem::tests::writing_replaces_the_target_and_touches_nothing_else`（预置一个旧式
  `face.xirang.tmp` 同级文件，断言它原样保留、目标换成新内容、且不留下自己的临时文件）。
- **m9 ✅ FIXED MINOR 写失败与环境配置错误走 panic 而非诊断** `[实测]` —
  `build_method/cache.rs:36-39` `expect("write generated module tree")`；
  `entry.rs` 的入口解析与 `scope.rs` 的取值解析在配置错误时直接 panic（`XIRANG_ENTRY`
  指不到文件、`application!` 解析失败/非 `crate::`/不可解析/多条、`XIRANG_SCOPE` 的
  schema 不对/身份不是 32 位十六进制/身份不存在、入口里的 graft 声明解析失败），
  而不是给出带子句的诊断。
  **Fixed:** 入口与范围这两组 panic 已改为诊断：新增 `entry::resolve_host_entry_reporting`
  与 `host_entry_from_environment(…, errors)`（旧的 3 参数 `resolve_host_entry` 保留为
  **仅测试**包装）、`SourceScope::from_environment(…, errors)` 与可测的纯函数
  `SourceScope::from_raw`、`validation::{face_syntax_errors, unplaced_face_errors}`，
  由 `pipeline::run` 在建树前收集进 `compile_errors`。每一处拒绝都同时**保守回退**
  （入口回退到 Cargo 约定、范围回退到全树、graft 回退到空表），因此即使有人忽略诊断也不会
  静默剪掉注册面；构建仍然失败，因为诊断会被渲染进生成树。实测三组：`XIRANG_ENTRY`
  指不到文件、范围取值三种坏法、入口里 graft 声明坏掉，都得到诊断而不是 panic。
  Pinned by `entry_tests::a_configured_entry_that_is_not_a_file_is_a_diagnostic`、
  `entry_tests::a_malformed_application_declaration_is_a_diagnostic`、
  `scope_tests::a_refused_scope_value_is_a_diagnostic`。
  **补完（本轮）:** `cache.rs` 的 `write_if_changed` 与四个清单写出函数都改为返回
  `Result`；`pipeline::run` 收集写失败——它是生成树唯一无法承载的失败，因为本该承载它的文件正
  是写不成的那个——构建脚本路径仍以带原因的 panic 停下（那正是构建脚本的失败机制），而
  `check --json` 把它当作 `phase=out-dir` 诊断与其他诊断一起报出。Pinned by
  `build_method/pipeline.rs::an_unwritable_generated_tree_is_reported_not_fatal`。
- **m10 ✅ FIXED MINOR `GraftPlanDocument::parse` 接受重复键** `[实测]` —
  `core/src/registry_core/plugin/graft/document.rs` 里第二个 `graft=` 静默获胜，而同一工作区的
  `from_jsonl` 与注册面解析器都拒绝重复字段。
  **Fixed:** 解析时记录已见键，重复即 `Malformed { line, message: "duplicate key `x`" }`。Pinned by
  `plugin::graft::document::tests::a_repeated_key_is_refused`。
- **m11 ✅ FIXED MINOR `assert_static_registration` 是公开的 panic 但没有 `# Panics` 文档**
  `[代码]` — `grep '# Panics' core/src` 零命中。
  **Fixed:** 补上 `# Panics`：说明哪些不满足会让它 panic，并说明生成的 crate 在 `const` 项里调用
  它，因此那次 panic 是点名声明来源的**编译错误**而不是运行期失败。同一处也写明它与
  `RegistrationRule::validate` 的 const 孪生关系（见 m14）。
- **m12 ✅ FIXED MINOR `MirGraph::from_mir_text` 无错返回，垃圾输入等于空图** `[代码]` —
  调用方无法区分"MIR 里没有调用"与"这根本不是 MIR"。
  **Fixed:** 选择"写明契约"而不是改签名（产出 MIR 的那条命令本来就知道 `cargo rustc` 是否成功），
  并在函数文档里写清：不是 MIR 的文本得到空图，必须区分两者的调用方自己检查输入。Pinned by
  `mir::text::tests::text_that_is_not_mir_yields_an_empty_graph`——将来改成 `Result` 的人必须
  连同这条测试与那段文档一起更新。
- **m13 ✅ FIXED MINOR 24 个 `ignore` 文档围栏永不编译** `[实测]` —
  `run_method/macros/face.rs` 里 24 个示例是宿主 crate 里宏调用**内部**的内容（命名
  `crate::…` 与生成的按注册机别名），永远无法作为 `run_method` 的 doctest 编译。
  **Fixed（两半）:** ①24 个围栏重标为 `rust,ignore`（语言写明；裸 `ignore` 连"这是 Rust"都不说），
  模块文档用一段话说明为什么它们不能编译，而不是重复 24 次；②**文档门禁现在也扫 rustdoc 注释**：
  `conventions/src/doc_blocks.rs` 新增 `doc_comment_findings`，遍历九个 crate 的非测试
  `src/**/*.rs`，把 `///`/`//!` 里的 `rust` 围栏按文件/语句块/字段列表三种读法解析。实测：21 个宏
  示例因此进入门禁（在此之前没有任何门禁看过它们），把其中一处改成
  `flow_provider crate::ControlHandle` 会让门禁失败并报出文件与行号，恢复即绿。剩下 3 个形状由宏
  匹配器决定（`name: { zh: … }`——字段值位置的裸花括号本身不是合法 Rust），标为
  `rust,ignore,macro-input`，由宏前端自己的测试钉住，门禁文档写明这一点。
- **m14 ✅ FIXED MINOR 棘轮里的尺寸债与一份重复实现** `[实测]` —
  `conventions/src/size.rs` 的清单（本轮之前 12 项，现为 10 项且全部按实测值更新，最大的是
  `contracts.rs` 645）；`release.rs` 的 `assert_static_registration` 曾被记为注册规则校验的
  "第三份拷贝"。
  **Fixed（重复的性质已查明并写明）:** 它不是可以合并的拷贝，而是 `RegistrationRule::validate`
  的 **const 孪生**：共享的 `validate_registration_requirements` 返回 `Vec<String>`，因此在常量
  求值里不可调用，const 那一侧只能停在第一个失败上并给固定消息。两处现在都写明"必须一起改"，以及
  各自被什么钉住（运行期：`parent_rule_aggregates_every_missing_structural_requirement`；const：
  工作区里每个拥有注册机的注册面按自己的规则能否编译）。
  **仍然登记为债:** 尺寸清单里的 10 个文件。
- **m15 ✅ FIXED MINOR 元数据与计数不全** `[实测]` — 九个 crate 都没有 `keywords`/`categories`；
  `AGENTS.md` 与 `deny.toml` 说"两个 `publish = false` 宿主"，实际是三个成员（两个例子 +
  `conventions`）加一个夹具包。
  **Fixed:** 九个清单各补 `keywords`（≤5 个，小写、≤20 字符）与 `categories`（按各 crate 的实际
  角色取自 crates.io 的分类表，例如 `development-tools::procedural-macro-helpers`、
  `development-tools::build-utils`、`wasm`、`command-line-utilities`）；`AGENTS.md` 中英两侧与
  `deny.toml` 的注释改为三个成员，并说明其中两个示例宿主不开 `missing_docs`，而 `conventions`
  与已发布 crate 一样带着它。
- **m16 ✅ FIXED MINOR CHANGELOG 指向的 "Known limits" 一节名不副实** `[实测]` — CHANGELOG 说
  已知限制见根 README，而 README 里那一节叫 `## Boundaries`，且其中没有 C1 的布局规则、也没有
  M2 的信任链边界。
  **Fixed（两半）:** ①中英两侧的指针现在点名 `README.md` 的 `## Boundaries` 一节；②该节补了两条
  真实边界——注册面必须位于 `<name>/<name>.rs`（普通模块静默跳过，布局外的注册面是诊断），以及
  插件信任的默认是校验和、只有 `verify_signed` 记录签名保证并打开官方通道、撤销在校验期间就被
  检查（沙箱边界仍归进程适配器，原本已写明）。中英同步。

## RELEASE / 发布

- **R1 ✅ FIXED RELEASE 首次发布演练** — `tools/xirang-package-audit` 的实测输出是
  `verified: xirang-core` / `skipped: 其余八个`（"versioned dependency not on
  index.crates.io yet"），所以**8/9 个包的隔离构建要到真实发布时才第一次跑**，而一个已发布的
  版本不可回滚（只能 yank）。发布顺序是硬链：`core` → `macro`/`build_method`/`mcp` →
  `run_method` → `debug_method`/`plugin-host` → `studio` → `cli`（`docs/roadmap-1.0.md`
  决策 1）。发布后在临时 crate 里 `cargo add` 真验证一遍，才是这次演练的收尾。

  **Fixed（工具侧）:** `tools/xirang-publish --verify-consumers` 在本检出之外建一个一次性
  crate，按版本 `cargo add` 九个 crate 再 `cargo check`——"消费者能否解析这次发布"从此是一条
  命令，而不是发布后手工敲的收尾。失败路径已实测：`CARGO_NET_OFFLINE=true
  tools/xirang-publish --verify-consumers` 退出 1，逐个列出 index 上还没有该版本的 crate 并
  明确写出"先发布、再重跑"（不是把"没发布"报成脚本故障）。**仍待你执行**：真实发布之后跑一次
  这条命令——它要求 index 上真的有这九个版本，而我没有 token，也不代按发布。
- **R2 ✅ FIXED RELEASE 没有 tag、没有 release 工作流，`tools/xirang-publish` 未接 CI**
  `[实测]` — `.github/workflows/` 只有 `ci.yml`，`git tag --list` 空。

  **Fixed:** 新增 `.github/workflows/release.yml`：`v*` tag 触发，第一步把 tag 名与 workspace
  版本对齐（`v0.1.0` ↔ `0.1.0`，不一致就在上传前停下——`cargo publish` 自己只看清单里的版本），
  然后 `--check-table`、全特性 `--all-targets` 测试、全特性 doctest、全特性 clippy
  `-D warnings`、`cargo doc -D warnings`、包审计与产物审计，最后才
  `tools/xirang-publish --publish --yes`（token 走 `CARGO_REGISTRY_TOKEN` secret），并以
  `--verify-consumers` 收尾；`workflow_dispatch` 允许在没有 tag 时只跑门禁（`publish` 输入
  默认关）。CI 的 features 任务另加一步 `--check-table`，这样漂移在每次 PR 上就会暴露，而不是
  等到打 tag。
  **顺带修掉一个实测的既存缺陷**：两张发布表是手工维护的，而 `tools/xirang-publish` 的
  `deps_of` 按空白切词，于是多依赖的 crate 只返回**第一个**依赖（实测：`xirang-run-method`
  只报 `xirang-core`、`xirang-studio` 只报 `xirang-run-method`、`xirang-cli` 只报
  `xirang-build-method`），上面那个"依赖还没上 index 就不许发"的守卫因此形同虚设；同一处按词
  遍历还把边行的被依赖者当成独立 crate（九行表遍历出十四个节点）。现在两张表都按整行读取，
  新增的 `--check-table` 核对「依赖表 = manifest」「层表 = 依赖表」以及"没有 crate 排在自己的
  依赖之前"：四类漂移都实测会失败（清单漂移、表里少一个依赖、层表顺序颠倒、层表漏或重一个
  crate），并用临时树验证过修好后的表会通过。真实漂移已存在并被修正：`xirang-cli` 的
  `[dependencies]` 里有 `xirang-core`，表里没有。同一处还发现工作区成员的解析用了
  `sed -n '/^members = \[/,/\]/p'`：sed 的范围**不在起始行上测试结束地址**，因此单行数组
  会让范围一直跑到下一个含 `]` 的行——也就是 `[workspace.package]`，它随后作为一个目录名进入
  成员列表；它恰好不存在，所以这条缺陷一直隐形（顺手也证明了三种 manifest 拼法都读得到：
  平铺 `[dependencies]`、点表 `[dependencies.xirang-x]`、按目标限定的
  `[target.'cfg(…).dependencies]`，而只写在 `[dev-dependencies]` 里的依赖会被如实报成不一致）。
  **仍待你执行**：打 tag（`git tag -a v0.1.0 -m …` 与 push）与创建
  `CARGO_REGISTRY_TOKEN` secret——我不做 git 写操作。
- **R3 ✅ FIXED RELEASE CI 覆盖缺口** `[报告]` — CI 不跑 `--all-features` 的 clippy、
  不跑 `tools/xirang-visual`、不跑 `tools/xirang-publish`；`--offline` 有意不进 CI。

  **Fixed:** features 任务新增三件：全特性 clippy（`--all-targets --all-features
  -- -D warnings`，此前只存在于 `prototype-fixtures`/`authoring`/`process-tools`/`syntax`/
  `dev-supervisor` 之后的警告没有任何门禁会看到）、tmux 渲染校验
  （`tools/xirang-visual home graph tree-demo`，`XIRANG_VISUAL_CARGO_FLAGS` 置空以避开
  只有本检出才有的 `--offline`，且每份 capture 必须非空）、以及
  `tools/xirang-publish` 的 dry-run。`--offline` 仍有意不进 CI：冷 runner 的 registry 缓存
  为空，在那里加它只会让每个任务失败，而不是让门禁更严格（`AGENTS.md` 的 Verify 段说明了这
  一点）。工具侧另加一个可在本地离线跑的入口：`tools/xirang-publish --check-table`。
- **R4 ✅ FIXED RELEASE 没有性能基线与预算** `[报告]` — `run_method/examples/scale_audit.rs` 在
  10k/100k 规模上会 assert 正确性（`registry.index().len() == size + 2`）并打印耗时与
  `peak_rss`，但不与任何预算比较；`docs/ROADMAP.md` 自己把 benchmark 列为未做。

  **Fixed:** `run_method/examples/scale_audit.rs` 现在既断言正确性也断言**预算**（注册
  40 µs/node、索引 20 µs/node，约为实测值的 8 倍，可用 `XIRANG_SCALE_REGISTER_US` /
  `XIRANG_SCALE_INDEX_US` 在更慢的机器上抬高而不改文件），于是数量级回归会让运行**失败**，
  而不是只把日志里那个数字变大。实测基线与复现命令记在新增的
  `docs/performance-baseline.md`：10k 注册 35 ms/索引 15 ms，100k 522 ms/259 ms（即
  5.2 µs 与 2.6 µs 每节点），页数恒定 32，静态面字节 10k/100k 为 330 000/3 300 000。
- **R5 ✅ FIXED RELEASE 两条"产物级"宣称未在本环境复跑** — 已链接产物不含 `.inventory` 段
  （`tools/xirang-release-audit` 存在且 CI 会跑）、`--release` 下的零分配/零启动开销。

  **Fixed:** 离线复跑 `CARGO_NET_OFFLINE=true tools/xirang-release-audit` 退出 0：没有任何
  已链接产物带 `.inventory` 段，`xirang` 4 782 240 B/6 140 个定义符号、`cargo-xirang`
  4 802 024/6 186、`libxirang_macro.so` 1 184 160/5 085，`startup_ms=176`；命令、环境与
  数字一并记入 `docs/performance-baseline.md`，并在发布工作流里作为一步跑（因此每次发布都有
  一份产物证据，而不是靠谁记得手工跑）。另一半宣称（`--release` 下零分配）仍没有可复跑的
  机制，留在 U2：本条只关掉"产物级宣称在本轮完全没有证据"这一点。

## UNVERIFIED / 未验证（谁都没证过，或本环境证不了）

- **U1 已测（本轮）** 已链接 release 产物确实不含 `.inventory` 段。实测
  `CARGO_NET_OFFLINE=true tools/xirang-release-audit` 退出 0，三个产物均无该段，数字与命令
  记在 `docs/performance-baseline.md`；发布工作流把这一步固定为每次发布都会跑（R5）。
  剩下的是"换个平台/工具链是否也成立"——那需要那些平台的产物，未测。
- **U2 已测（本轮）** `docs/audit-graft-vs-readme.md` 的零分配 / 无 `.inventory` 启动宣称。
  `.inventory` 那一半由 U1 覆盖；零分配这一半现在也是实测的：新增
  `examples/control-button/tests/static_plan_allocations.rs`，用与 U4 同一套计数式全局分配器，
  而且**整个文件只有一条 `#[test]`**（计数器是进程全局的，libtest 又把同一文件的测试放在不同
  线程上，第二条测试的分配会让"0 次分配"这条断言因无关原因失败；连跑 30 次确认不抖）。
  实测（debug 与 release 同值）：
  (a) **静态读路径 0 次分配、0 字节**——`builtin_static_plan()`、`faces()`、`grafts()`、
  `len()`、`is_empty()`、`find()` 命中与落空各一次、`children_of().count()`、以及遍历全部面读
  `id()/parent()/owns_registry()`。把 `String::from("mutation")` 塞进 `StaticPlan::find`，该断言
  立刻报"allocated 16 bytes over 2 allocations"（已验证后还原，文件字节一致）。这条测试在
  `tools/xirang-external-rehearsal` 里也通过，即同一个测量在检出之外（宿主项目只有路径依赖时）
  同样成立。
  (b) `overlay_static` **不是零分配，但确实跳过计划**，而且同口径下更便宜：空 overlay（下限，
  克隆树 + 簿记）6 次/96 字节；1 个切口 静态 60 次/3 470 字节 vs 动态 77/4 053；示例的 2 个切口
  静态 107/5 628 vs 动态 138/6 438；而那份一切口 `GraftPlan` 单独要 3 次/350 字节。
  因此 C17 那句"不分配计划"是准确的，且可以加强为"同口径下比动态路径少分配约 20%"——数字记入
  `docs/performance-baseline.md`。
  **方法上的一个坑，记下来免得后人重踩**：本对比的第一版拿示例的**两个**静态切口去对一个切口的
  动态计划，得出"静态路径反而贵 30 次分配"的相反结论；是分阶段拆解（空 overlay / 1 刀 / 2 刀 /
  计划本身）把这个口径错误暴露出来的。
- **U3 已可复跑（本轮）** 当时那次"外部路径演练"是手工做的，因此不可从源码重推。现在它是
  `tools/xirang-external-rehearsal`：把 `examples/control-button` 与
  `examples/control-button-graft` 复制到临时目录、把内部的 `path` 依赖指向本检出、加上一个空
  `[workspace]` 让它们脱离工作区，然后从零构建并测试（自带全新 `target/`，因此也不依赖本检出
  恰好已有的产物）。本轮实测：**27 条测试通过、0 失败**（`registry.rs` 25 +
  `health_check.rs` 1 + U2 新增的 `static_plan_allocations.rs` 1，`ide_mirror.rs` 1 条按设计
  `#[ignore]`），随后用本检出的 CLI 指向那个外部项目：
  `xirang check: ok (xirang-example-control-button)`——首次实测时是 26 条，与当时手工得到的
  数字一致，多出来的一条正是本轮新增的分配计数测试，也就是说 U2 的测量在检出之外同样成立；
  而这次的区别是它进了仓库：CI 的 `verify` 任务在一个矩阵单元上跑它（有网络，因此把
  `XIRANG_REHEARSAL_CARGO_FLAGS` 置空）。
- **U4 已测（M3 那一半）** M3 的表分配量级与可利用性。新增
  `plugin-host/tests/wasm_table_cost.rs`：自带计数式全局分配器（wasmi 不暴露任何查询表大小的
  接口，所以读不出引擎内部的那个数），并且**一个文件只放一个测试**——计数器是进程全局的，
  兄弟测试在另一线程上的分配会被算进来。实测：一个函数引用在宿主一侧是 **8 字节**
  （1 048 576 条目的表 → 峰值 8 469 408 字节），因此 M3 的默认上限 4096 是 32 KiB，而
  `(table 100000000 funcref)` 本来会是 **762 MiB**；限制器在表存在**之前**就拒绝这次分配，
  拒绝时峰值 **5 225 字节**（`table_growing` 在分配前返回 `ResourceLimiterDeniedAllocation`），
  debug 与 release 两个档给出同一个数。拿掉 `StoreLimitsBuilder::table_elements` 的接线，
  该测试立即变红（已验证后还原）。同一次核对确认了"谁在拒"：wasmi 的
  `EnforcedLimits::strict()` 只有 `max_tables`/`max_element_segments`（多少张表、多少个元素段），
  **没有**单张表的元素数上限（`wasmi-1.1.0/src/engine/limits/engine.rs`），因此
  `WasmLimits::table_elements` 是唯一约束——M3 修的那一处确实是承重的。
  **仍未验证**：`M3/W7` 里的 `W7` 这个编号在本仓库任何文件里都不存在（三份早期审计、
  roadmap、CHANGELOG 都没有），因此无法复核它指的是什么；本轮只关掉 M3 那一半。
- **U5 已测（本轮）** `CallEvidence::Live` 的可达性。新增两条运行观察
  （`studio/studio/app/tests/evidence.rs`，在夹具工程上枚举调用图页面**可能画出的每一条边**
  ——即对该工程每个函数问它的调用者与被调用者，与树/详情面板读的是同一个 `call_relations`）：
  (a) 出厂 Studio 装的演示样本确实带一条边，而且它的节点不属于已加载工程的任何节点（逐条断言），
  此时 `call_evidence` 回答 `Live` 的条数是 **0**；(b) 换成针对已加载注册面的追踪
  （`tests/fixtures.rs`）后，同一份枚举恰好报出它观测到的那一条
  （`preview_canvas_width` → `clamp_canvas_width`）。结论：`Live` **不是**死代码；它对真实工程
  不可达的唯一原因是装的追踪不属于该工程（Studio 缺 trace ingest，见 `app/lifecycle.rs` 的
  TODO），而这一条此前只是从 `NodeId` 出处推的。把 `call_evidence` 改成无条件返回 `Live`，
  两条测试都变红（已验证后还原），因此它们钉的是这个判定本身。
- **U6** 编辑器补全的条目数与"值位无候选"的结论（LSP-only，无法从源码判定）。
- **U7** 第二轮/第三轮四条遗留项（`is_registration_path` 合并前边界、`Registry::get` 旧签名、
  `requires_isolation`/`ProductionPolicyNotStrict` 旧语义、三个 verify 包装）缺旧修订，
  可达历史里没有基线。
- **U8** 在线 `cargo publish --dry-run`（需 token 与网络；`tools/xirang-publish` 离线时报
  `attempting to make an HTTP request, but --offline was specified`，退出 1）。

## Extra findings during the fix work / 修复过程中额外发现的问题

These are not among the 48 rows: the tests and checks written for those rows turned
them up. All are fixed and pinned; they are recorded so the next reader does not
have to rediscover them.
这些不在 48 条之内：它们是那些行所写的测试与检查翻出来的。全部已修并已钉住；记在这里，免得
下一个读的人重新发现一遍。

- **`tools/xirang-publish` 按词读取自己的表** `[实测]` — `deps_of` 只返回多依赖 crate 的
  第一个依赖（`xirang-cli` 只被按 `xirang-build-method` 检查），使"依赖未上 index 就不许
  发布"的守卫形同虚设；同一次遍历还把边行的被依赖者当成独立 crate（九行表产出十四个节点）。
  现在两张表都按整行读取，并由 `--check-table` 与清单对比；它立刻找出一条真实漂移——
  `xirang-cli` 直接依赖 `xirang-core`，而表里没写。同一处的工作区成员扫描原先用
  `sed -n '/^members/,/\]/p'`，而 sed 的范围不在起始行上测试结束地址，于是
  `[workspace.package]` 作为一个恰好不存在的目录名进了成员列表；现在用 `awk` 精确取数组。
- **文档门禁无守卫地把围栏 Rust 交给 `syn`** `[实测]` — 嵌套 60 000 个定界符的围栏会让门禁
  进程 abort 而不是失败，与 M1 同一类，出现在唯一还没改到的地方。它现在向内核的
  `guard_nesting` 提问（该函数为此公开，整个工作区因此只有一份嵌套度量），拒绝行为由
  `conventions/src/doc_blocks.rs::a_pathologically_nested_fence_is_reported_not_fatal` 钉住：
  把守卫那一行删掉，该测试立刻以栈溢出 abort。
- **本文档里的两处陈旧残留** `[代码]` — C1 行仍说那条布局规则没写进 README（m16 已经写了，
  `README.md:787-789`、`README.zh-CN.md:703`），m16 行尾还带着一份修前描述和一条已经执行完的
  `Fix:` 说明。两处已清理，开头那段门禁数字也标明是审计当时的数字（修完后的数字在状态一节）。
- **成本表的断言与实测数字之间没有链接** `[代码]` — `README.md` 与 `README.zh-CN.md` 现在从
  表格处指向 `docs/performance-baseline.md`，读者不必先知道那份文件存在。
## Fix order / 修复顺序

Not "by severity" but by what unblocks what; each batch is meant to land green on its own
(`fmt` / `test` / `clippy -D warnings` / `doc -D warnings`).
不按严重度排，按"什么解开什么"排；每一批都应独立落地并自带全绿门禁。

1. **C1 + C2 + m9**: panic → 诊断。这一批解开"把工具指向任何真实项目"这件事，也让
   `cli/README` 的失败契约重新成立。先写红灯测试（宿主含平铺模块、畸形面各一条），再修。
2. **C3 + M10 + M13 + M14**: Studio 的根解析、启动失败、破坏性动作的确认与原子性。
   这一批解开"Studio 会不会弄坏我的项目"。
3. **M15 + C4**: 编辑器镜像的嵌套面可用性与"改写前先给人看 diff"。移植 UI 组件库时，
   这两条是每天都要用的路径。
4. **M1 + M5 + M11 + M12**: 内核对病态输入的鲁棒性与两处量级问题（深度上限、整数比较、
   visited 集、一次遍历数行）。这一批让"内核返回 `Result`"这个承诺对任意输入成立。
5. **M6 + M7 + M8 + M9**: CLI/MCP 的静默与退出码。
6. **M2 + M3 + M4**: 插件的信任链与两个资源上限。**先决定"接线还是删宣称"**——
   这三条一起改 `docs/threat-model.md`，否则文档继续为没运行的控制背书。
7. **m1 + m2 + m3**: 发布面：CHANGELOG 的假发布、版本口径、README 安装段与那条跑不通的命令。
8. **R1 + R2 + R3 + R4**: 发布演练、tag/release 流程、CI 缺口、性能基线。
9. **m4–m8 + m10–m16**: 文档真实性与卫生（可与其他批次并行，逐条独立）。
10. **U1–U8**: 能测的补测（U1/U4/U5/U8），不能测的在文档里写成"未验证"而不是默认成立。

## Method / 方法

Baseline `b963632`, working tree clean before and after (`git status --porcelain` empty; probes
wrote only to `/tmp` and the ignored `target/`). My own runs, all `--offline` except the
crates.io/`index.crates.io` lookups: `cargo fmt --all --check`; `cargo clippy --workspace
--all-targets` with and without `--all-features`, both `-D warnings`; `cargo test --workspace`
(401/0); `cargo test --workspace --all-features` (436/0); `cargo test --workspace --release
--all-targets` (395/0); `cargo test --workspace --all-features --doc` (5/0); `RUSTDOCFLAGS='-D
warnings' cargo doc --workspace --no-deps`; `cargo check --workspace --no-default-features
--all-targets`; `cargo check --workspace --all-targets --locked`; `tools/xirang-package-audit`;
`cargo test -p xirang-conventions`; `cargo test -p xirang-example-control-button`;
`./target/debug/xirang check examples/control-button [--json]`, `check /nonexistent`,
`check debug_method`, and `check /tmp/hostprobe` (a copy of the example plus `src/helpers.rs`);
`curl` against `index.crates.io` for `xirang-core` and twelve candidate crate names; reads of
`conventions/src/{purity,size,doc_blocks}.rs`, `core/.../identity/{node_id,identity}.rs`,
`docs/roadmap-1.0.md`, `docs/threat-model.md`, `.github/workflows/ci.yml`, `tools/*`, the root
and per-crate manifests, `plugin-host/src/{wasm,artifact-caller,verifier}.rs`,
`studio/studio/app/support.rs`, `build_method/{discovery,lib}.rs`. The four delegated
audits are recorded by their own evidence in the rows above (`[报告]`), each of which ran
read-only with `--offline` and left the tree clean.
基线 `b963632`，审计前后工作树都干净（`git status --porcelain` 为空；探针只写 `/tmp` 与被忽略的
`target/`）。我自己的命令除 crates.io/`index.crates.io` 查询外全部带 `--offline`，逐条见上。
四份委派审计按各自证据记在上面的 `[报告]` 行里，它们都是只读、`--offline`、且未改动工作树。
