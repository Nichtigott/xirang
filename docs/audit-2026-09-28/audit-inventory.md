# xirang 结构审计 · 机械清点与基线快照

> 任务 t1（executor）。机器可读版本：`docs/audit-2026-09-28/audit-inventory.json`（同目录，约 1.9 MB，所有数字都能从它复算）。
> 审计对象是**工作树**：HEAD `cf0c378`（清单第二项(记录那一面):xirang.diff records:true——graft 记录相对源码,身份换了就报 re-identified）。工作树未提交改动：审计开始时 `git status --porcelain` **121** 项（其中 `M` 101 项、`??` 20 项；任务书写的是 120，以上为实测）。本轮只出报告，源码一行未改。
> 工具链：`rustc 1.96.0 (ac68faa20 2026-05-25)` / `cargo 1.96.0 (30a34c682 2026-05-25)`。生成时间 2026-09-28T12:43:11+0800。

## 0. 门禁基线快照（四条，全部在本工作树上连续跑完）

| # | 命令 | exit code | 结果 | 关键输出 |
| --- | --- | --- | --- | --- |
| 1 | `cargo fmt --all -- --check` | **0** | ✅ 通过 | 无输出（格式零差异） |
| 2 | `cargo test --workspace --offline` | **0** | ✅ 通过 | test result 汇总：**52 个测试套件 / 740 passed / 0 failed / 25 ignored** |
| 3 | `cargo clippy --workspace --all-targets --offline -- -D warnings` | **0** | ✅ 通过 | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.66s`，无 warning |
| 4 | `tools/xirang-publish --check-table` | **0** | ✅ 通过 | `dependency table matches the manifests (9 crates)` |

四条**全部 exit 0**，基线是绿的。任何片区审计若声称门禁红，必须附上可复现命令与输出；否则以本快照为准。

- 四条按 1→4 顺序连续执行，全部在本工作树上、同一轮内取得；日志留在 `/tmp/xirang-audit-logs/{fmt_check,test,clippy,publish_check}.log`（含完整 stdout+stderr 与 `EXIT_CODE=` 行）。
- `cargo` 三条均带 `--offline`（AGENTS.md 要求）。除 `--check` 外没有跑过 `cargo fmt` 的写操作，也没有跑任何会改文件的命令。
- `tools/xirang-publish --check-table` 只读本检出，不需要网络与 token。
- **这份快照的范围是"报告文件出现之前的工作树"**：`conventions` 的 `doc_blocks`/`doc_anchors` 会把 `docs/` 下的 markdown 当活文档扫描，豁免只看**文件名前缀**（`audit*` / `design*`）。审计报告一旦以别的名字落进本目录，`cargo test --workspace` 就会变红——那是报告自己触发的门禁，不是源码回归。本目录下的报告一律用 `audit-*` 前缀（本条即为此类），所以 `cargo test` 仍应全绿；若变红，先检查是否有不符合豁免前缀的文件进了 `docs/`。
- **数据新鲜度**：工作树里最后一份被改动的源码是 `mcp/apply.rs`（12:06:03）；门禁与清点在 12:31–12:38 之间跑完，因此本快照覆盖的是最新工作树，不存在"清点早于源码"的空窗。Cargo.lock 是审计前就已被改的 ` M` 项（mtime 11:18），本轮三条 cargo 命令没有重写它。
- 三条 cargo 命令的 `target/` 是热的：`test` 增量编译 1.33s、`clippy` 0.66s。**这是指纹命中后的结果，不是跳过**——cargo 只有在指纹与当前源码一致时才复用；若工作树之后被改动，必须重跑本表。

## 1. 口径：这份清点到底数的是什么

| 项 | 定义 |
| --- | --- |
| **文件行数** | 文件行数 = 全文行数（含文档注释 ///、空行、#[cfg(test)] 测试模块、#[cfg] 门控代码），与 wc -l 在同一口径（以换行符计数）。 |
| **函数统计** | fn 定义 = 词法扫描到的每个 fn 项：自由函数、inherent impl 方法、trait 默认/必需方法、trait impl 方法、fn 内嵌套 fn。不含闭包、不含函数指针类型、不含宏展开后生成的东西。唯一的口径例外：macro_rules! 宏体里的 fn 也会被扫到，全仓库只有 1 处（run_method/macros/face_helpers.rs 的 __assert_impls）。 |
| **函数行数** | 函数行数 = fn 关键字所在行 → 匹配的闭合 } 所在行（含）；trait 里的无体声明算 1 行。lines_incl_attrs 额外含紧邻其上的 /// 文档与 #[...] 属性行。 |
| **可见性** | visibility 取 pub / pub(crate) / pub(super) / pub(in ...) / private 原文。 |
| **模块挂载边** | 模块挂载边来自父文件里的 mod 声明（hand-written）与构建期生成的 $OUT_DIR/generated_lib.rs（generated-out-dir）。 |
| **`#[cfg(test)]` 挂载** | 文件是否经由某个带 #[cfg(test)] 的 mod 声明（或 #[cfg(test)] 内联模块里的声明）挂载。 |
| **排除项** | target/ 下的一切构建产物；非 .rs 文件；Cargo.toml 只用于取包与 target 信息。 |
| **非成员包** | 带自己的 Cargo.toml 但不属于本工作区的包（studio/tests/fixtures/node-editor）单独成一项，workspace_member=false，不计入工作区成员合计。 |

**数据怎么来的（禁止手抄）**：`cargo metadata --no-deps --format-version 1 --offline` 提供包、target 与特性；源码侧用一个纯词法扫描器（去注释/去字符串 → 括号栈）取出每个 `fn` 项、每个 `mod` 声明与文件行数；模块挂载边由 `mod` 声明解析到真实文件（含 `#[path]` 与 `#[cfg]` 属性），构建期生成的 `$OUT_DIR/generated_lib.rs` 单独成类。

**五项独立对账（全部通过）**：

| 对账 | 结果 |
| --- | --- |
| 逐文件行数 vs `wc -l` | 394/394 完全一致，0 处不符 |
| `mod x;` 声明数 vs 解析出的挂载边数 | grep 得 339 行 ↔ 解析得 339 条边，0 条未解析 |
| 逐文件 `fn` 数 vs 对去注释源码做正则计数 | 2462 ↔ 2462，0 个文件不符 |
| 函数名 ASCII 校验 | 2462 个名字的字符数全部等于字节数，0 个非 ASCII 标识符 |
| 600 行棘轮基线（`conventions/src/size.rs` 钉了 `contracts.rs = 639`） | 实测 `core/src/registry_core/plugin/contracts/contracts.rs` = **639 行**，一致 |

复算：`python3 /tmp/xirang-audit-logs/census.py && python3 /tmp/xirang-audit-logs/census2.py`，再由 `/tmp/xirang-audit-logs/build_md.py` 渲染本文件。三个脚本是纯词法解析、只用标准库、只读仓库，不依赖 codegraph 索引也不联网——索引会滞后于工作树，所以清点不用它。注意脚本放在 `/tmp`（本轮写权限只开放本目录的两个产物文件）；`/tmp` 若被清理，按 §1 口径可重写（扫描器约 400 行 Python），本文件与 `audit-inventory.json` 本身已把口径写全。

## 2. crate 体量表

工作区成员 12 个（9 个发布 crate + `conventions` + 2 个 examples 宿主），另有 1 个不在工作区内的夹具包 `studio/tests/fixtures/node-editor`（它自己声明 `[workspace]`，单列在表末）。

| crate | 目录 | 文件 | 总行 | src 行 | tests 行 | fn | pub fn | test fn | 模块边 | 未挂载 | 最大文件 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `xirang-core` | `core/` | 93 | 21,787 | 21,579 | 208 | 792 | 358 | 276 | 90 | 0 | `core/src/registry_core/plugin/contracts/contracts.rs` (639) |
| `xirang-macro` | `macro/` | 3 | 917 | 917 | 0 | 26 | 12 | 7 | 2 | 0 | `macro/src/lib.rs` (402) |
| `xirang-run-method` | `run_method/` | 61 | 9,584 | 8,277 | 1,157 | 277 | 143 | 89 | 51 | 0 | `run_method/runtime/trace/frames/frames.rs` (452) |
| `xirang-build-method` | `build_method/` | 48 | 10,110 | 9,928 | 182 | 292 | 114 | 114 | 45 | 0 | `build_method/graft_plan_check.rs` (465) |
| `xirang-cli` | `cli/` | 14 | 2,802 | 2,802 | 0 | 68 | 27 | 33 | 11 | 0 | `cli/lib_tests.rs` (1,015) |
| `xirang-mcp` | `mcp/` | 39 | 8,682 | 8,682 | 0 | 242 | 44 | 141 | 37 | 0 | `mcp/apply.rs` (563) |
| `xirang-debug-method` | `debug_method/` | 5 | 481 | 444 | 37 | 19 | 10 | 6 | 3 | 0 | `debug_method/src/adapters.rs` (214) |
| `xirang-studio` | `studio/` | 72 | 12,308 | 12,209 | 99 | 333 | 138 | 138 | 68 | 0 | `studio/studio/app/tests/call_tree.rs` (702) |
| `xirang-plugin-host` | `plugin-host/` | 15 | 3,888 | 1,987 | 1,901 | 136 | 38 | 72 | 10 | 0 | `plugin-host/tests/fault_matrix.rs` (968) |
| `xirang-example-control-button` | `examples/control-button/` | 13 | 1,635 | 199 | 1,179 | 53 | 2 | 40 | 8 | 0 | `examples/control-button/tests/registry.rs` (743) |
| `xirang-example-control-button-graft` | `examples/control-button-graft/` | 4 | 134 | 134 | 0 | 1 | 1 | 0 | 3 | 0 | `examples/control-button-graft/src/button_fast.rs` (36) |
| `xirang-conventions` | `conventions/` | 20 | 6,399 | 6,399 | 0 | 209 | 36 | 113 | 19 | 0 | `conventions/src/doc_anchors.rs` (557) |
| **工作区成员合计** | — | **387** | **78,727** | **73,557** | **4,763** | **2448** | **923** | **1029** | **347** | **0** | — |
| `xirang-fixture-node-editor` (非工作区成员) | `studio/tests/fixtures/node-editor/` | 7 | 224 | 214 | 0 | 14 | 9 | 0 | 0 | 5 | — |

全仓库 `.rs` 文件共 **394** 个（成员 387 + 夹具 7）、**78,951** 行。`target/` 下的构建产物（含 `examples/control-button/target/xirang/out/generated_lib.rs`）不计入。

## 3. 最大文件 top 30

行数口径同 §1（含测试模块与文档注释）。`role` 取文件在包内的位置：`src` / `tests` / `examples` / `build`。

| # | 行数 | crate | role | 文件 | fn | pub fn | `#[cfg(test)]` 挂载 |
| ---: | ---: | --- | --- | --- | ---: | ---: | --- |
| 1 | **1,015** | `xirang-cli` | src | `cli/lib_tests.rs` | 32 | 0 | 是 |
| 2 | **968** | `xirang-plugin-host` | tests | `plugin-host/tests/fault_matrix.rs` | 32 | 0 | 否 |
| 3 | **743** | `xirang-example-control-button` | tests | `examples/control-button/tests/registry.rs` | 28 | 0 | 否 |
| 4 | **702** | `xirang-studio` | src | `studio/studio/app/tests/call_tree.rs` | 18 | 0 | 是 |
| 5 | **639** | `xirang-core` | src | `core/src/registry_core/plugin/contracts/contracts.rs` | 31 | 16 | 否 |
| 6 | **563** | `xirang-mcp` | src | `mcp/apply.rs` | 9 | 2 | 否 |
| 7 | **562** | `xirang-studio` | src | `studio/studio/app/tests/project.rs` | 18 | 0 | 是 |
| 8 | **557** | `xirang-conventions` | src | `conventions/src/doc_anchors.rs` | 19 | 1 | 否 |
| 9 | **551** | `xirang-core` | src | `core/src/registry_core/declaration/runtime_checks.rs` | 20 | 12 | 否 |
| 10 | **517** | `xirang-mcp` | src | `mcp/apply_tests.rs` | 12 | 0 | 是 |
| 11 | **509** | `xirang-studio` | src | `studio/studio/app/support.rs` | 23 | 21 | 否 |
| 12 | **508** | `xirang-conventions` | src | `conventions/src/lib.rs` | 19 | 10 | 否 |
| 13 | **504** | `xirang-core` | src | `core/src/registry_core/tree/connector/connector.rs` | 12 | 3 | 否 |
| 14 | **502** | `xirang-core` | src | `core/src/registry_core/syntax/entries/graft.rs` | 14 | 1 | 否 |
| 15 | **500** | `xirang-conventions` | src | `conventions/src/size.rs` | 17 | 5 | 否 |
| 16 | **469** | `xirang-core` | src | `core/src/registry_core/source/source.rs` | 9 | 5 | 否 |
| 17 | **465** | `xirang-build-method` | src | `build_method/graft_plan_check.rs` | 18 | 3 | 否 |
| 18 | **463** | `xirang-core` | src | `core/src/registry_core/tree/graft_ops/overlay.rs` | 12 | 6 | 否 |
| 19 | **461** | `xirang-core` | src | `core/src/registry_core/declaration/registration.rs` | 16 | 15 | 否 |
| 20 | **455** | `xirang-core` | src | `core/src/registry_core/plugin/catalog/catalog.rs` | 17 | 6 | 否 |
| 21 | **452** | `xirang-run-method` | src | `run_method/runtime/trace/frames/frames.rs` | 30 | 20 | 否 |
| 22 | **449** | `xirang-core` | src | `core/src/registry_core/plugin/graft/document.rs` | 18 | 5 | 否 |
| 23 | **449** | `xirang-build-method` | src | `build_method/scaffold/project.rs` | 14 | 4 | 否 |
| 24 | **448** | `xirang-mcp` | src | `mcp/tools.rs` | 8 | 2 | 否 |
| 25 | **444** | `xirang-build-method` | src | `build_method/face_view.rs` | 10 | 1 | 否 |
| 26 | **444** | `xirang-conventions` | src | `conventions/src/lint.rs` | 14 | 3 | 否 |
| 27 | **443** | `xirang-plugin-host` | src | `plugin-host/src/process.rs` | 14 | 7 | 否 |
| 28 | **439** | `xirang-build-method` | src | `build_method/entry.rs` | 10 | 7 | 否 |
| 29 | **438** | `xirang-studio` | src | `studio/studio/app/tests/trace_ingest.rs` | 18 | 0 | 是 |
| 30 | **437** | `xirang-studio` | src | `studio/studio/app/tests/edit.rs` | 6 | 0 | 是 |

超过 600 行（`conventions/src/size.rs` 的 `CEILING`，非测试源码文件的棘轮上限）的文件：**5** 个。

- `core/src/registry_core/plugin/contracts/contracts.rs` — 639 行，role=src，`#[cfg(test)]` 挂载=否（在棘轮 BASELINE 里，被显式钉住）
- `cli/lib_tests.rs` — 1,015 行，role=src，`#[cfg(test)]` 挂载=是（文件名像测试且确实挂在 `#[cfg(test)]` 后，按 size.rs 的边界豁免）
- `studio/studio/app/tests/call_tree.rs` — 702 行，role=src，`#[cfg(test)]` 挂载=是（文件名像测试且确实挂在 `#[cfg(test)]` 后，按 size.rs 的边界豁免）
- `plugin-host/tests/fault_matrix.rs` — 968 行，role=tests，`#[cfg(test)]` 挂载=否（tests/ 目录，按位置豁免）
- `examples/control-button/tests/registry.rs` — 743 行，role=tests，`#[cfg(test)]` 挂载=否（tests/ 目录，按位置豁免）

## 4. 函数名长度分布

长度按**字符数**（本仓库标识符全是 ASCII，字符数=字节数）。分三列：全部 fn、非测试 fn、pub 且非测试 fn。测试函数常是整句描述，会把分布右移，所以单列出来。

| 名字长度 | 全部 fn | 非测试 fn | pub 且非测试 fn |
| --- | ---: | ---: | ---: |
| 1–3 | 93 (3.8%) | 87 (6.1%) | 56 (6.1%) |
| 4–6 | 287 (11.7%) | 222 (15.5%) | 125 (13.6%) |
| 7–9 | 308 (12.5%) | 225 (15.7%) | 149 (16.2%) |
| 10–12 | 302 (12.3%) | 264 (18.4%) | 168 (18.2%) |
| 13–15 | 242 (9.8%) | 215 (15.0%) | 148 (16.1%) |
| 16–18 | 199 (8.1%) | 187 (13.0%) | 112 (12.2%) |
| 19–21 | 149 (6.1%) | 133 (9.3%) | 90 (9.8%) |
| 22–24 | 72 (2.9%) | 64 (4.5%) | 45 (4.9%) |
| 25–29 | 64 (2.6%) | 32 (2.2%) | 25 (2.7%) |
| 30–999 | 746 (30.3%) | 4 (0.3%) | 3 (0.3%) |
| **合计** | **2462** | **1433** | **921** |

| 集合 | 平均 | 中位数 | 最短 | 最长 |
| --- | ---: | ---: | ---: | ---: |
| 全部 fn | 22.7 | 15 | 2 | 79 |
| 非测试 fn | 12.1 | 12 | 2 | 35 |
| pub 且非测试 fn | 12.4 | 12 | 2 | 34 |

≥25 字符的函数名：**810** 个（占全部 fn 的 32.9%）。最长的 20 个：

| 长度 | 名字 | 位置 | 类型 |
| ---: | --- | --- | --- |
| 79 | `an_answered_requirement_names_its_provider_and_the_plan_lists_the_neighbourhood` | `mcp/converge_tests.rs:212` | free_fn |
| 79 | `a_declared_table_costs_the_host_by_the_element_and_the_ceiling_refuses_it_first` | `plugin-host/tests/wasm_table_cost.rs:240` | free_fn |
| 78 | `coordinates_in_viewport_accepts_inside_and_rejects_overflow_and_space_mismatch` | `core/src/registry_core/declaration/runtime_checks_tests.rs:66` | free_fn |
| 77 | `a_rejected_tree_is_reported_as_the_verdict_rather_than_hidden_behind_an_error` | `mcp/converge_tests.rs:242` | free_fn |
| 76 | `compatible_with_stays_literal_while_the_semantic_entry_point_folds_spellings` | `core/src/registry_core/plugin/contracts/contracts.rs:608` | free_fn |
| 72 | `a_range_covers_the_siblings_between_its_endpoints_in_registry_name_order` | `core/src/registry_core/tree/graft_ops/resolution.rs:313` | free_fn |
| 71 | `source_references_ignore_imports_strings_comments_and_registration_data` | `core/src/registry_core/syntax/face_tests.rs:130` | free_fn |
| 71 | `a_configured_root_that_is_not_a_directory_fails_with_the_variable_named` | `studio/tests/launch.rs:45` | free_fn |
| 70 | `enter_re_centres_on_a_tree_node_and_opens_the_editor_only_at_the_focus` | `studio/studio/app/tests/call_tree.rs:279` | free_fn |
| 69 | `application_entry_parser_keeps_duplicates_visible_to_the_build_policy` | `core/src/registry_core/syntax/entries/application.rs:118` | free_fn |
| 69 | `a_build_script_without_a_source_tree_fails_with_the_layout_diagnostic` | `build_method/pipeline_tests.rs:30` | free_fn |
| 69 | `a_declared_plan_names_its_declaration_and_an_undeclared_one_is_unkept` | `mcp/grafts_tests.rs:63` | free_fn |
| 69 | `editing_module_name_moves_the_face_and_keeps_generated_source_compact` | `studio/studio/app/tests/edit.rs:80` | free_fn |
| 68 | `the_package_root_rule_prefers_the_explicit_value_then_a_real_package` | `core/src/registry_core/lexicon/lexicon_tests.rs:37` | free_fn |
| 68 | `a_blank_create_keeps_defaults_while_a_blank_edit_writes_them_through` | `run_method/authoring/operations/face_write.rs:307` | free_fn |
| 68 | `the_artifact_path_prefers_the_override_and_falls_back_to_the_lexicon` | `run_method/runtime/trace/artifact/artifact_tests.rs:347` | free_fn |
| 68 | `an_unidentified_artifact_makes_the_delta_say_what_it_cannot_rule_out` | `mcp/mir_tests.rs:163` | free_fn |
| 68 | `a_session_without_an_artifact_installs_no_trace_and_confirms_nothing` | `studio/studio/app/tests/evidence.rs:84` | free_fn |
| 67 | `a_backwards_range_is_refused_with_both_endpoints_and_the_order_rule` | `core/src/registry_core/tree/graft_ops/resolution.rs:352` | free_fn |
| 67 | `a_kind_only_external_face_derives_its_registry_name_from_the_module` | `run_method/tests/external_compact_face.rs:31` | free_fn |

**上表几乎全是测试函数**（整句描述，是仓库的既有风格）。真正落在产品代码上的 ≥25 字符名只有 **36** 个，全数列在下面——这是 ①函数名可读性 一路要看的清单：

| 长度 | 名字 | 位置 | 类型 |
| ---: | --- | --- | --- |
| 35 | `flow_fields_semantically_compatible` | `core/src/registry_core/plugin/contracts/contracts.rs:326` | free_fn |
| 34 | `validate_registration_requirements` | `core/src/registry_core/declaration/declaration.rs:97` | free_fn |
| 30 | `application_default_trace_mode` | `run_method/runtime/trace/call_trace.rs:20` | free_fn |
| 30 | `handle_new_project_overlay_key` | `studio/studio/app/overlay/new_project.rs:7` | inherent_method |
| 29 | `parse_registration_rule_owned` | `core/src/registry_core/authoring/parse/rules.rs:140` | free_fn |
| 29 | `connector_error_with_external` | `core/src/registry_core/tree/connector/connector.rs:193` | inherent_method |
| 29 | `validate_snapshot_replacement` | `core/src/registry_core/tree/graft_ops/graft_ops.rs:100` | inherent_method |
| 29 | `derives_rule_from_the_sibling` | `run_method/authoring/manifest/face/face.rs:195` | inherent_method |
| 29 | `aggregate_parent_macro_errors` | `build_method/validation.rs:39` | free_fn |
| 28 | `module_source_from_node_path` | `core/src/registry_core/authoring/parse/parse.rs:91` | free_fn |
| 28 | `semantically_compatible_with` | `core/src/registry_core/plugin/contracts/contracts.rs:119` | inherent_method |
| 28 | `semantically_compatible_with` | `core/src/registry_core/plugin/contracts/contracts.rs:232` | inherent_method |
| 28 | `add_module_with_registration` | `run_method/authoring/operations/operations.rs:202` | free_fn |
| 28 | `render_call_report_for_trace` | `run_method/call_report/call_report.rs:30` | free_fn |
| 28 | `resolve_host_entry_reporting` | `build_method/entry.rs:276` | free_fn |
| 28 | `aggregate_stable_name_errors` | `build_method/validation.rs:30` | free_fn |
| 27 | `validate_snapshot_migration` | `core/src/registry_core/tree/graft_ops/graft_ops.rs:45` | inherent_method |
| 27 | `legacy_rule_path_for_source` | `run_method/authoring/validation/validation.rs:91` | free_fn |
| 27 | `host_entry_from_environment` | `build_method/entry.rs:205` | free_fn |
| 27 | `declaring_application_entry` | `build_method/entry.rs:389` | free_fn |
| 27 | `write_source_scope_manifest` | `build_method/manifests.rs:34` | free_fn |
| 27 | `collect_parent_macro_errors` | `build_method/validation.rs:45` | free_fn |
| 27 | `preview_canvas_width_traced` | `studio/tests/fixtures/node-editor/src/control/object/node_editor/node_editor.rs:40` | free_fn |
| 26 | `parse_admission_expression` | `core/src/registry_core/authoring/parse/admission.rs:18` | free_fn |
| 26 | `assert_static_registration` | `core/src/registry_core/release/release.rs:291` | free_fn |
| 26 | `collect_registration_chain` | `core/src/registry_core/tree/connector/connector.rs:27` | inherent_method |
| 26 | `external_provider_rejected` | `core/src/registry_core/tree/connector/connector.rs:152` | inherent_method |
| 26 | `apply_snapshot_replacement` | `core/src/registry_core/tree/graft_ops/graft_ops.rs:215` | inherent_method |
| 26 | `collect_face_syntax_errors` | `build_method/validation.rs:197` | free_fn |
| 26 | `declaration_contract_paths` | `studio/studio/app/keyboard.rs:171` | free_fn |
| 25 | `aggregate_contract_errors` | `build_method/contracts.rs:15` | free_fn |
| 25 | `auto_from_entry_reporting` | `build_method/scope.rs:201` | inherent_method |
| 25 | `handle_delete_overlay_key` | `studio/studio/app/overlay/delete.rs:7` | inherent_method |
| 25 | `handle_plugin_overlay_key` | `studio/studio/app/overlay/plugin.rs:7` | inherent_method |
| 25 | `handle_search_overlay_key` | `studio/studio/app/overlay/search.rs:7` | inherent_method |
| 25 | `is_mounted_as_test_within` | `conventions/src/size.rs:135` | free_fn |

按“下划线段数”看：1 段 601 个、2 段 663 个、3 段 355 个、4 段 78 个、5 段 58 个、6 段 114 个、7 段 149 个、8 段 156 个、9 段 126 个、10 段 82 个、11 段 39 个、12 段 32 个、13 段 6 个、14 段 1 个、15 段 2 个

短名（≤3 字符、非测试）**87** 个，前 15 个：`new`(parse)、`fmt`(parse)、`fmt`(owned)、`new`(registration)、`new`(registration)、`new`(runtime_checks)、`run`(runtime_checks)、`fmt`(source_location)、`new`(build)、`at`(build)、`len`(build)、`fmt`(error)、`new`(error)、`fmt`(error)、`fmt`(node_id)

| crate | 非测试 fn | 平均名长 | 中位数 | ≥25 字符 | ≥5 段 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `xirang-core` | 516 | 11.4 | 10 | 14 | 1 |
| `xirang-macro` | 19 | 10.9 | 11 | 0 | 0 |
| `xirang-run-method` | 188 | 13.1 | 12 | 5 | 3 |
| `xirang-build-method` | 178 | 15.2 | 15 | 10 | 0 |
| `xirang-cli` | 35 | 10.3 | 11 | 0 | 0 |
| `xirang-mcp` | 101 | 9.3 | 9 | 0 | 0 |
| `xirang-debug-method` | 13 | 8.5 | 9 | 0 | 0 |
| `xirang-studio` | 195 | 13.2 | 13 | 5 | 1 |
| `xirang-plugin-host` | 64 | 8.8 | 7 | 0 | 0 |
| `xirang-example-control-button` | 13 | 5.3 | 4 | 0 | 0 |
| `xirang-example-control-button-graft` | 1 | 17.0 | 17 | 0 | 0 |
| `xirang-conventions` | 96 | 13.0 | 13 | 1 | 1 |

## 5. 按目录的函数计数

目录 = 文件所在目录（工作区相对路径）。只统计工作区成员。空目录不出现。

共 98 个目录。按 fn 数降序，前 30：

| 目录 | 文件 | 行数 | fn | pub fn | test fn |
| --- | ---: | ---: | ---: | ---: | ---: |
| `mcp/` | 39 | 8,682 | 242 | 44 | 141 |
| `conventions/src/` | 20 | 6,399 | 209 | 36 | 113 |
| `build_method/` | 32 | 6,453 | 191 | 78 | 67 |
| `studio/studio/app/` | 20 | 3,966 | 115 | 86 | 11 |
| `studio/studio/app/tests/` | 11 | 3,373 | 95 | 1 | 95 |
| `core/src/registry_core/syntax/` | 9 | 1,948 | 78 | 34 | 26 |
| `core/src/registry_core/declaration/` | 9 | 2,468 | 76 | 48 | 16 |
| `core/src/registry_core/mir/` | 9 | 1,855 | 69 | 13 | 36 |
| `plugin-host/tests/` | 4 | 1,901 | 68 | 0 | 68 |
| `core/src/registry_core/tree/graft_ops/` | 9 | 2,490 | 65 | 24 | 34 |
| `plugin-host/src/` | 9 | 1,703 | 60 | 31 | 4 |
| `cli/` | 7 | 2,355 | 59 | 22 | 32 |
| `core/src/registry_core/identity/` | 4 | 974 | 50 | 20 | 21 |
| `core/src/registry_core/diagnostic/` | 4 | 940 | 46 | 30 | 9 |
| `core/src/registry_core/authoring/parse/` | 5 | 1,058 | 44 | 28 | 9 |
| `core/src/registry_core/source/` | 5 | 1,275 | 43 | 10 | 24 |
| `run_method/runtime/trace/artifact/` | 4 | 1,242 | 42 | 8 | 19 |
| `run_method/authoring/operations/` | 9 | 1,701 | 40 | 26 | 5 |
| `examples/control-button/tests/` | 4 | 1,179 | 40 | 0 | 40 |
| `run_method/tests/` | 8 | 1,157 | 39 | 0 | 39 |
| `build_method/scaffold/` | 3 | 1,148 | 39 | 16 | 13 |
| `build_method/graft_view/` | 7 | 1,398 | 38 | 12 | 21 |
| `core/src/registry_core/plugin/contracts/` | 1 | 639 | 31 | 16 | 4 |
| `run_method/runtime/trace/frames/` | 1 | 452 | 30 | 20 | 7 |
| `core/src/registry_core/plugin/graft/` | 2 | 781 | 28 | 13 | 9 |
| `core/src/registry_core/release/` | 1 | 409 | 27 | 22 | 2 |
| `macro/src/` | 3 | 917 | 26 | 12 | 7 |
| `run_method/authoring/external_graft/` | 3 | 567 | 25 | 14 | 10 |
| `studio/bin/` | 1 | 427 | 25 | 0 | 11 |
| `studio/studio/ui/` | 8 | 886 | 21 | 10 | 6 |

（完整表见 `audit-inventory.json`：把 `crates[].files[]` 按目录聚合即可。）

## 6. 模块挂载树

每个文件后面标：`行数 / fn 数`。标记含义：`[root]` 是 target 源根（lib/bin/test/example/bench/build），`[cfg(test)]` 表示经带 `#[cfg(test)]` 的声明挂载，`[feat=...]` 表示经 `#[cfg(feature = "...")]` 门控挂载，`[gen]` 表示由构建期生成的 `$OUT_DIR/generated_lib.rs` 挂载（工作树里没有手写 `mod` 声明）。

### `xirang-core`（`core/`，93 文件 / 21,787 行 / 792 fn，90 条挂载边）

```
├─ `src/lib.rs`  60 行 / 0 fn  [root]  `lib`
│   └─ `src/registry_core.rs`  52 行 / 0 fn    `lib::registry_core`
│       ├─ `src/registry_core/authoring/authoring.rs`  26 行 / 0 fn    `lib::registry_core::authoring`
│       │   ├─ `src/registry_core/authoring/face_field.rs`  152 行 / 1 fn    `lib::registry_core::authoring::face_field`
│       │   ├─ `src/registry_core/authoring/field_presentation.rs`  353 行 / 7 fn    `lib::registry_core::authoring::field_presentation`
│       │   ├─ `src/registry_core/authoring/parse/parse.rs`  296 行 / 17 fn  [feat=syntax]  `lib::registry_core::authoring::parse`
│       │   │   ├─ `src/registry_core/authoring/parse/admission.rs`  235 行 / 7 fn  [feat=syntax]  `lib::registry_core::authoring::parse::admission`
│       │   │   ├─ `src/registry_core/authoring/parse/flow.rs`  253 行 / 8 fn  [feat=syntax]  `lib::registry_core::authoring::parse::flow`
│       │   │   │   └─ `src/registry_core/authoring/parse/flow_tests.rs`  50 行 / 3 fn  [cfg(test)] [feat=syntax]  `lib::registry_core::authoring::parse::flow::flow_tests`
│       │   │   └─ `src/registry_core/authoring/parse/rules.rs`  224 行 / 9 fn  [feat=syntax]  `lib::registry_core::authoring::parse::rules`
│       │   ├─ `src/registry_core/authoring/snapshot/snapshot.rs`  136 行 / 1 fn  [feat=syntax]  `lib::registry_core::authoring::snapshot`
│       │   └─ `src/registry_core/authoring/validation/validation.rs`  111 行 / 8 fn    `lib::registry_core::authoring::validation`
│       ├─ `src/registry_core/declaration/declaration.rs`  363 行 / 8 fn    `lib::registry_core::declaration`
│       │   ├─ `src/registry_core/declaration/call_evidence.rs`  163 行 / 5 fn    `lib::registry_core::declaration::call_evidence`
│       │   ├─ `src/registry_core/declaration/contract.rs`  124 行 / 3 fn    `lib::registry_core::declaration::contract`
│       │   ├─ `src/registry_core/declaration/owned.rs`  316 行 / 9 fn    `lib::registry_core::declaration::owned`
│       │   ├─ `src/registry_core/declaration/registration.rs`  461 行 / 16 fn    `lib::registry_core::declaration::registration`
│       │   ├─ `src/registry_core/declaration/runtime_checks.rs`  551 行 / 20 fn    `lib::registry_core::declaration::runtime_checks`
│       │   │   └─ `src/registry_core/declaration/runtime_checks_tests.rs`  294 行 / 8 fn  [cfg(test)]  `lib::registry_core::declaration::runtime_checks::runtime_checks_tests`
│       │   └─ `src/registry_core/declaration/source_location.rs`  115 行 / 5 fn    `lib::registry_core::declaration::source_location`
│       │       └─ `src/registry_core/declaration/source_location_tests.rs`  81 行 / 2 fn  [cfg(test)]  `lib::registry_core::declaration::source_location::source_location_tests`
│       ├─ `src/registry_core/diagnostic/diagnostic.rs`  18 行 / 0 fn    `lib::registry_core::diagnostic`
│       │   ├─ `src/registry_core/diagnostic/build.rs`  429 行 / 21 fn    `lib::registry_core::diagnostic::build`
│       │   ├─ `src/registry_core/diagnostic/error.rs`  343 行 / 19 fn    `lib::registry_core::diagnostic::error`
│       │   └─ `src/registry_core/diagnostic/topology.rs`  150 行 / 6 fn    `lib::registry_core::diagnostic::topology`
│       ├─ `src/registry_core/identity/identity.rs`  254 行 / 15 fn    `lib::registry_core::identity`
│       │   ├─ `src/registry_core/identity/node_id.rs`  230 行 / 17 fn    `lib::registry_core::identity::node_id`
│       │   ├─ `src/registry_core/identity/path_text.rs`  136 行 / 4 fn    `lib::registry_core::identity::path_text`
│       │   └─ `src/registry_core/identity/sha256.rs`  354 行 / 14 fn    `lib::registry_core::identity::sha256`
│       ├─ `src/registry_core/json/json.rs`  98 行 / 4 fn    `lib::registry_core::json`
│       ├─ `src/registry_core/lexicon/lexicon.rs`  282 行 / 6 fn    `lib::registry_core::lexicon`
│       │   └─ `src/registry_core/lexicon/lexicon_tests.rs`  197 行 / 9 fn  [cfg(test)]  `lib::registry_core::lexicon::tests`
│       ├─ `src/registry_core/mir/mir.rs`  38 行 / 0 fn    `lib::registry_core::mir`
│       │   ├─ `src/registry_core/mir/call_tree.rs`  409 行 / 11 fn    `lib::registry_core::mir::call_tree`
│       │   │   └─ `src/registry_core/mir/call_tree_tests.rs`  306 行 / 19 fn  [cfg(test)]  `lib::registry_core::mir::call_tree::call_tree_tests`
│       │   ├─ `src/registry_core/mir/delta.rs`  161 行 / 7 fn    `lib::registry_core::mir::delta`
│       │   ├─ `src/registry_core/mir/jsonl.rs`  345 行 / 12 fn    `lib::registry_core::mir::jsonl`
│       │   ├─ `src/registry_core/mir/merge.rs`  154 行 / 7 fn    `lib::registry_core::mir::merge`
│       │   ├─ `src/registry_core/mir/model.rs`  169 行 / 2 fn    `lib::registry_core::mir::model`
│       │   ├─ `src/registry_core/mir/render.rs`  159 行 / 5 fn    `lib::registry_core::mir::render`
│       │   └─ `src/registry_core/mir/text.rs`  114 行 / 6 fn    `lib::registry_core::mir::text`
│       ├─ `src/registry_core/plugin/plugin.rs`  49 行 / 0 fn    `lib::registry_core::plugin`
│       │   ├─ `src/registry_core/plugin/artifact/artifact.rs`  198 行 / 8 fn    `lib::registry_core::plugin::artifact`
│       │   │   └─ `src/registry_core/plugin/artifact/artifact_tests.rs`  280 行 / 12 fn  [cfg(test)]  `lib::registry_core::plugin::artifact::artifact_tests`
│       │   ├─ `src/registry_core/plugin/catalog/catalog.rs`  455 行 / 17 fn    `lib::registry_core::plugin::catalog`
│       │   ├─ `src/registry_core/plugin/contracts/contracts.rs`  639 行 / 31 fn    `lib::registry_core::plugin::contracts`
│       │   │   └─ `src/registry_core/plugin/contracts/signing/signing.rs`  299 行 / 14 fn    `lib::registry_core::plugin::contracts::signing`
│       │   ├─ `src/registry_core/plugin/graft/graft.rs`  332 行 / 10 fn    `lib::registry_core::plugin::graft`
│       │   │   └─ `src/registry_core/plugin/graft/document.rs`  449 行 / 18 fn    `lib::registry_core::plugin::graft::document`
│       │   ├─ `src/registry_core/plugin/plugin_policy/plugin_policy.rs`  314 行 / 13 fn    `lib::registry_core::plugin::plugin_policy`
│       │   ├─ `src/registry_core/plugin/slot/slot.rs`  125 行 / 2 fn    `lib::registry_core::plugin::slot`
│       │   └─ `src/registry_core/plugin/trust/trust.rs`  417 行 / 12 fn    `lib::registry_core::plugin::trust`
│       ├─ `src/registry_core/release/release.rs`  409 行 / 27 fn    `lib::registry_core::release`
│       ├─ `src/registry_core/requirements/requirements.rs`  255 行 / 11 fn    `lib::registry_core::requirements`
│       ├─ `src/registry_core/source/source.rs`  469 行 / 9 fn    `lib::registry_core::source`
│       │   ├─ `src/registry_core/source/calls.rs`  256 行 / 6 fn    `lib::registry_core::source::calls`
│       │   ├─ `src/registry_core/source/lex.rs`  58 行 / 2 fn    `lib::registry_core::source::lex`
│       │   ├─ `src/registry_core/source/source_tests.rs`  260 行 / 12 fn  [cfg(test)]  `lib::registry_core::source::tests`
│       │   └─ `src/registry_core/source/walk.rs`  232 行 / 14 fn    `lib::registry_core::source::walk`
│       ├─ `src/registry_core/syntax/syntax.rs`  27 行 / 0 fn  [feat=syntax]  `lib::registry_core::syntax`
│       │   ├─ `src/registry_core/syntax/deep_input_tests.rs`  248 行 / 17 fn  [cfg(test)] [feat=syntax]  `lib::registry_core::syntax::deep_input_tests`
│       │   ├─ `src/registry_core/syntax/entries.rs`  18 行 / 0 fn  [feat=syntax]  `lib::registry_core::syntax::entries`
│       │   │   ├─ `src/registry_core/syntax/entries/application.rs`  125 行 / 5 fn  [feat=syntax]  `lib::registry_core::syntax::entries::application`
│       │   │   └─ `src/registry_core/syntax/entries/graft.rs`  502 行 / 14 fn  [feat=syntax]  `lib::registry_core::syntax::entries::graft`
│       │   ├─ `src/registry_core/syntax/face.rs`  316 行 / 10 fn  [feat=syntax]  `lib::registry_core::syntax::face`
│       │   │   ├─ `src/registry_core/syntax/face_tests.rs`  208 行 / 8 fn  [cfg(test)] [feat=syntax]  `lib::registry_core::syntax::face::tests`
│       │   │   ├─ `src/registry_core/syntax/fields.rs`  310 行 / 17 fn  [feat=syntax]  `lib::registry_core::syntax::face::fields`
│       │   │   └─ `src/registry_core/syntax/tokens.rs`  288 行 / 12 fn  [feat=syntax]  `lib::registry_core::syntax::face::tokens`
│       │   ├─ `src/registry_core/syntax/nesting.rs`  362 行 / 5 fn  [feat=syntax]  `lib::registry_core::syntax::nesting`
│       │   └─ `src/registry_core/syntax/reference_scan.rs`  171 行 / 9 fn  [feat=syntax]  `lib::registry_core::syntax::reference_scan`
│       └─ `src/registry_core/tree/tree.rs`  33 行 / 0 fn    `lib::registry_core::tree`
│           ├─ `src/registry_core/tree/connector/connector.rs`  504 行 / 12 fn    `lib::registry_core::tree::connector`
│           ├─ `src/registry_core/tree/entry_pages/entry_pages.rs`  82 行 / 10 fn    `lib::registry_core::tree::entry_pages`
│           ├─ `src/registry_core/tree/graft_ops/graft_ops.rs`  419 行 / 13 fn    `lib::registry_core::tree::graft_ops`
│           │   ├─ `src/registry_core/tree/graft_ops/fixtures.rs`  72 行 / 2 fn  [cfg(test)]  `lib::registry_core::tree::graft_ops::fixtures`
│           │   ├─ `src/registry_core/tree/graft_ops/overlay.rs`  463 行 / 12 fn    `lib::registry_core::tree::graft_ops::overlay`
│           │   ├─ `src/registry_core/tree/graft_ops/record.rs`  91 行 / 1 fn    `lib::registry_core::tree::graft_ops::record`
│           │   │   ├─ `src/registry_core/tree/graft_ops/apply.rs`  267 行 / 1 fn    `lib::registry_core::tree::graft_ops::record::apply`
│           │   │   ├─ `src/registry_core/tree/graft_ops/reconcile.rs`  227 行 / 4 fn    `lib::registry_core::tree::graft_ops::record::reconcile`
│           │   │   ├─ `src/registry_core/tree/graft_ops/record_tests.rs`  365 行 / 18 fn  [cfg(test)]  `lib::registry_core::tree::graft_ops::record::record_tests`
│           │   │   └─ `src/registry_core/tree/graft_ops/reports.rs`  165 行 / 1 fn    `lib::registry_core::tree::graft_ops::record::reports`
│           │   └─ `src/registry_core/tree/graft_ops/resolution.rs`  421 行 / 13 fn    `lib::registry_core::tree::graft_ops::resolution`
│           ├─ `src/registry_core/tree/header/header.rs`  27 行 / 0 fn    `lib::registry_core::tree::header`
│           ├─ `src/registry_core/tree/index/index.rs`  111 行 / 4 fn    `lib::registry_core::tree::index`
│           ├─ `src/registry_core/tree/inspection/inspection.rs`  199 行 / 5 fn    `lib::registry_core::tree::inspection`
│           ├─ `src/registry_core/tree/metadata/metadata.rs`  69 行 / 7 fn    `lib::registry_core::tree::metadata`
│           ├─ `src/registry_core/tree/ports/ports.rs`  325 行 / 9 fn    `lib::registry_core::tree::ports`
│           │   └─ `src/registry_core/tree/ports/ports_tests.rs`  215 行 / 6 fn  [cfg(test)]  `lib::registry_core::tree::ports::ports_tests`
│           ├─ `src/registry_core/tree/query/query.rs`  191 行 / 16 fn    `lib::registry_core::tree::query`
│           ├─ `src/registry_core/tree/registry.rs`  98 行 / 6 fn    `lib::registry_core::tree::registry`
│           └─ `src/registry_core/tree/transaction/transaction.rs`  402 行 / 10 fn    `lib::registry_core::tree::transaction`
├─ `tests/nesting_budget.rs`  177 行 / 4 fn  [root]  `nesting_budget`
└─ `tests/ungated_authoring_data.rs`  31 行 / 2 fn  [root]  `ungated_authoring_data`
```

### `xirang-macro`（`macro/`，3 文件 / 917 行 / 26 fn，2 条挂载边）

```
└─ `src/lib.rs`  402 行 / 7 fn  [root]  `lib`
    ├─ `src/front_end.rs`  136 行 / 5 fn    `lib::front_end`
    └─ `src/mirror.rs`  379 行 / 14 fn    `lib::mirror`
```

### `xirang-run-method`（`run_method/`，61 文件 / 9,584 行 / 277 fn，51 条挂载边）

```
├─ `examples/scale_audit.rs`  150 行 / 3 fn  [root]  `scale_audit`
├─ `src/lib.rs`  66 行 / 0 fn  [root]  `lib`
│   ├─ `src/authoring/authoring.rs`  38 行 / 0 fn  [feat=authoring]  `lib::authoring`
│   │   ├─ `src/authoring/external_graft/external_graft.rs`  10 行 / 0 fn  [feat=authoring]  `lib::authoring::external_graft`
│   │   │   └─ `src/authoring/external_graft/plan.rs`  265 行 / 15 fn  [feat=authoring]  `lib::authoring::external_graft::plan`
│   │   │       └─ `src/authoring/external_graft/plan_tests.rs`  292 行 / 10 fn  [cfg(test)] [feat=authoring]  `lib::authoring::external_graft::plan::plan_tests`
│   │   ├─ `src/authoring/face_manifest.rs`  89 行 / 2 fn  [feat=authoring]  `lib::authoring::face_manifest`
│   │   ├─ `src/authoring/filesystem/filesystem.rs`  95 行 / 4 fn  [feat=authoring]  `lib::authoring::filesystem`
│   │   ├─ `src/authoring/manifest/manifest.rs`  13 行 / 0 fn  [feat=authoring]  `lib::authoring::manifest`
│   │   │   ├─ `src/authoring/manifest/face/face.rs`  291 行 / 9 fn  [feat=authoring]  `lib::authoring::manifest::face`
│   │   │   │   └─ `src/authoring/manifest/face/render.rs`  287 行 / 3 fn  [feat=authoring]  `lib::authoring::manifest::face::render`
│   │   │   ├─ `src/authoring/manifest/face_manifest.rs`  22 行 / 0 fn  [feat=authoring]  `lib::authoring::manifest::face_manifest`
│   │   │   └─ `src/authoring/manifest/parse/parse.rs`  264 行 / 2 fn  [feat=authoring]  `lib::authoring::manifest::parse`
│   │   ├─ `src/authoring/operations/operations.rs`  423 行 / 10 fn  [feat=authoring]  `lib::authoring::operations`
│   │   │   ├─ `src/authoring/operations/authored.rs`  228 行 / 4 fn  [feat=authoring]  `lib::authoring::operations::authored`
│   │   │   ├─ `src/authoring/operations/create.rs`  119 行 / 1 fn  [feat=authoring]  `lib::authoring::operations::create`
│   │   │   ├─ `src/authoring/operations/delete.rs`  54 行 / 1 fn  [feat=authoring]  `lib::authoring::operations::delete`
│   │   │   ├─ `src/authoring/operations/face_values.rs`  135 行 / 3 fn  [feat=authoring]  `lib::authoring::operations::face_values`
│   │   │   ├─ `src/authoring/operations/face_write.rs`  339 行 / 11 fn  [feat=authoring]  `lib::authoring::operations::face_write`
│   │   │   ├─ `src/authoring/operations/migration.rs`  288 行 / 5 fn  [feat=authoring]  `lib::authoring::operations::migration`
│   │   │   ├─ `src/authoring/operations/paths.rs`  47 行 / 2 fn  [feat=authoring]  `lib::authoring::operations::paths`
│   │   │   └─ `src/authoring/operations/trash.rs`  68 行 / 3 fn  [feat=authoring]  `lib::authoring::operations::trash`
│   │   ├─ `src/authoring/parse/parse.rs`  54 行 / 3 fn  [feat=authoring]  `lib::authoring::parse`
│   │   ├─ `src/authoring/snapshot/snapshot.rs`  18 行 / 1 fn  [feat=authoring]  `lib::authoring::snapshot`
│   │   └─ `src/authoring/validation/validation.rs`  158 行 / 9 fn  [feat=authoring]  `lib::authoring::validation`
│   ├─ `src/call_report/call_report.rs`  220 行 / 6 fn    `lib::call_report`
│   ├─ `src/macros/macros.rs`  11 行 / 0 fn    `lib::macros`
│   │   ├─ `src/macros/entry.rs`  130 行 / 0 fn    `lib::macros::entry`
│   │   ├─ `src/macros/face.rs`  268 行 / 1 fn    `lib::macros::face`
│   │   │   ├─ `src/macros/face_external.rs`  166 行 / 0 fn    `lib::macros::face::face_external`
│   │   │   ├─ `src/macros/face_helpers.rs`  189 行 / 1 fn    `lib::macros::face::face_helpers`
│   │   │   ├─ `src/macros/face_objects.rs`  172 行 / 0 fn    `lib::macros::face::face_objects`
│   │   │   └─ `src/macros/face_registration.rs`  172 行 / 0 fn    `lib::macros::face::face_registration`
│   │   └─ `src/macros/trace.rs`  56 行 / 0 fn    `lib::macros::trace`
│   ├─ `src/plugin/plugin.rs`  7 行 / 0 fn    `lib::plugin`
│   ├─ `src/registry/registry.rs`  7 行 / 0 fn    `lib::registry`
│   └─ `src/runtime/runtime.rs`  17 行 / 0 fn    `lib::runtime`
│       ├─ `src/runtime/evidence.rs`  11 行 / 0 fn    `lib::runtime::evidence`
│       ├─ `src/runtime/graft_record.rs`  333 行 / 8 fn    `lib::runtime::graft_record`
│       └─ `src/runtime/trace/trace.rs`  25 行 / 0 fn    `lib::runtime::trace`
│           ├─ `src/runtime/trace/artifact/artifact.rs`  368 行 / 7 fn    `lib::runtime::trace::artifact`
│           │   ├─ `src/runtime/trace/artifact/artifact_tests.rs`  426 行 / 19 fn  [cfg(test)]  `lib::runtime::trace::artifact::artifact_tests`
│           │   ├─ `src/runtime/trace/artifact/io.rs`  144 行 / 5 fn    `lib::runtime::trace::artifact::io`
│           │   └─ `src/runtime/trace/artifact/parse.rs`  304 行 / 11 fn    `lib::runtime::trace::artifact::parse`
│           ├─ `src/runtime/trace/call_trace.rs`  348 行 / 20 fn    `lib::runtime::trace::call_trace`
│           ├─ `src/runtime/trace/edges/edges.rs`  430 行 / 18 fn    `lib::runtime::trace::edges`
│           ├─ `src/runtime/trace/frames/frames.rs`  452 行 / 30 fn    `lib::runtime::trace::frames`
│           └─ `src/runtime/trace/locals/locals.rs`  18 行 / 0 fn    `lib::runtime::trace::locals`
│               ├─ `src/runtime/trace/locals/call_trace.rs`  228 行 / 9 fn    `lib::runtime::trace::locals::call_trace`
│               ├─ `src/runtime/trace/locals/local_id.rs`  13 行 / 0 fn    `lib::runtime::trace::locals::local_id`
│               ├─ `src/runtime/trace/locals/local_kind.rs`  33 行 / 1 fn    `lib::runtime::trace::locals::local_kind`
│               ├─ `src/runtime/trace/locals/local_value.rs`  41 行 / 0 fn    `lib::runtime::trace::locals::local_value`
│               └─ `src/runtime/trace/locals/observation.rs`  25 行 / 1 fn    `lib::runtime::trace::locals::observation`
├─ `tests/external_compact_face.rs`  117 行 / 5 fn  [root]  `external_compact_face`
├─ `tests/external_source_default.rs`  73 行 / 1 fn  [root]  `external_source_default`
├─ `tests/face_arm_defaults.rs`  168 行 / 3 fn  [root]  `face_arm_defaults`
├─ `tests/face_fields.rs`  243 行 / 6 fn  [root]  `face_fields`
├─ `tests/face_preset_parts.rs`  46 行 / 1 fn  [root]  `face_preset_parts`
├─ `tests/graft_record.rs`  293 行 / 14 fn  [root]  `graft_record`
├─ `tests/kind_only_registry_name.rs`  38 行 / 1 fn  [root]  `kind_only_registry_name`
└─ `tests/path_compat.rs`  179 行 / 8 fn  [root]  `path_compat`
```

### `xirang-build-method`（`build_method/`，48 文件 / 10,110 行 / 292 fn，45 条挂载边）

```
├─ `src/lib.rs`  206 行 / 3 fn  [root]  `lib`
│   ├─ `src/build_input.rs`  46 行 / 2 fn    `lib::build_input`
│   ├─ `src/cache.rs`  254 行 / 8 fn    `lib::cache`
│   ├─ `src/contracts.rs`  297 行 / 9 fn    `lib::contracts`
│   ├─ `src/diagnostics.rs`  5 行 / 0 fn    `lib::diagnostics`
│   ├─ `src/discovery.rs`  272 行 / 14 fn    `lib::discovery`
│   ├─ `src/entry.rs`  439 行 / 10 fn    `lib::entry`
│   │   ├─ `src/entry_paths.rs`  88 行 / 2 fn    `lib::entry::entry_paths`
│   │   └─ `src/entry_tests.rs`  258 行 / 8 fn  [cfg(test)]  `lib::entry::entry_tests`
│   ├─ `src/entry_default.rs`  91 行 / 3 fn    `lib::entry_default`
│   ├─ `src/face_view.rs`  444 行 / 10 fn    `lib::face_view`
│   │   └─ `src/scope_view.rs`  287 行 / 7 fn    `lib::face_view::scope_view`
│   ├─ `src/faces.rs`  96 行 / 3 fn    `lib::faces`
│   ├─ `src/graft_plan_check.rs`  465 行 / 18 fn    `lib::graft_plan_check`
│   ├─ `src/graft_view.rs`  33 行 / 0 fn    `lib::graft_view`
│   │   ├─ `src/graft_view/declared.rs`  185 行 / 5 fn    `lib::graft_view::declared`
│   │   ├─ `src/graft_view/matching.rs`  171 行 / 9 fn    `lib::graft_view::matching`
│   │   ├─ `src/graft_view/overlay_rows.rs`  165 行 / 1 fn    `lib::graft_view::overlay_rows`
│   │   │   └─ `src/graft_view/overlay_rows_tests.rs`  231 行 / 8 fn  [cfg(test)]  `lib::graft_view::overlay_rows::overlay_rows_tests`
│   │   ├─ `src/graft_view/plan_rows.rs`  198 行 / 3 fn    `lib::graft_view::plan_rows`
│   │   │   └─ `src/graft_view/plan_rows_tests.rs`  36 行 / 1 fn  [cfg(test)]  `lib::graft_view::plan_rows::plan_rows_tests`
│   │   └─ `src/graft_view/query.rs`  412 行 / 11 fn    `lib::graft_view::query`
│   ├─ `src/identity.rs`  197 行 / 10 fn    `lib::registry_identity`
│   ├─ `src/identity_cache.rs`  101 行 / 4 fn    `lib::identity_cache`
│   ├─ `src/manifests.rs`  215 行 / 9 fn    `lib::manifests`
│   ├─ `src/node.rs`  26 行 / 1 fn    `lib::node`
│   ├─ `src/node_id.rs`  117 行 / 5 fn    `lib::node_id`
│   ├─ `src/package.rs`  190 行 / 6 fn    `lib::package`
│   ├─ `src/pipeline.rs`  273 行 / 5 fn    `lib::pipeline`
│   │   └─ `src/pipeline_tests.rs`  229 行 / 9 fn  [cfg(test)]  `lib::pipeline::pipeline_tests`
│   ├─ `src/registration_check.rs`  259 行 / 5 fn    `lib::registration_check`
│   ├─ `src/renderer.rs`  62 行 / 2 fn    `lib::renderer`
│   │   ├─ `src/renderer/aliases.rs`  194 行 / 6 fn    `lib::renderer::aliases`
│   │   ├─ `src/renderer/ide.rs`  138 行 / 3 fn    `lib::renderer::ide`
│   │   ├─ `src/renderer/pass.rs`  400 行 / 6 fn    `lib::renderer::pass`
│   │   └─ `src/renderer/tree.rs`  197 行 / 2 fn    `lib::renderer::tree`
│   ├─ `src/scaffold.rs`  30 行 / 0 fn    `lib::scaffold`
│   │   ├─ `src/scaffold/install.rs`  275 行 / 8 fn    `lib::scaffold::install`
│   │   ├─ `src/scaffold/project.rs`  449 行 / 14 fn    `lib::scaffold::project`
│   │   └─ `src/scaffold/snippets.rs`  424 行 / 17 fn    `lib::scaffold::snippets`
│   ├─ `src/scope.rs`  388 行 / 6 fn    `lib::scope`
│   │   └─ `src/scope_tests.rs`  236 行 / 5 fn  [cfg(test)]  `lib::scope::scope_tests`
│   ├─ `src/source_layout.rs`  247 行 / 4 fn    `lib::source_layout`
│   ├─ `src/static_plan.rs`  241 行 / 8 fn    `lib::static_plan`
│   ├─ `src/syntax.rs`  9 行 / 0 fn    `lib::registry_syntax`
│   └─ `src/validation.rs`  352 行 / 15 fn    `lib::validation`
├─ `tests/missing_source_tree.rs`  66 行 / 3 fn  [root]  `missing_source_tree`
└─ `tests/outside_src_layout.rs`  116 行 / 4 fn  [root]  `outside_src_layout`
```

### `xirang-cli`（`cli/`，14 文件 / 2,802 行 / 68 fn，11 条挂载边）

```
├─ `src/bin/cargo-xirang.rs`  23 行 / 1 fn  [root]  `cargo-xirang`
├─ `src/bin/xirang.rs`  15 行 / 1 fn  [root]  `xirang`
└─ `src/lib.rs`  293 行 / 10 fn  [root]  `lib`
    ├─ `src/commands/build.rs`  42 行 / 1 fn    `lib::build_command`
    ├─ `src/commands/check.rs`  80 行 / 1 fn    `lib::check_command`
    ├─ `src/commands/new.rs`  125 行 / 3 fn    `lib::new_command`
    ├─ `src/commands/snippets.rs`  119 行 / 1 fn    `lib::snippets_command`
    ├─ `src/commands/studio.rs`  43 行 / 1 fn    `lib::studio_command`
    ├─ `src/explain.rs`  269 行 / 3 fn    `lib::explain`
    │   ├─ `src/explain_json.rs`  24 行 / 2 fn    `lib::explain::json`
    │   ├─ `src/explain_overlay.rs`  236 行 / 1 fn    `lib::explain::overlay`
    │   └─ `src/explain_report.rs`  273 行 / 7 fn    `lib::explain::report`
    ├─ `src/grafts.rs`  245 行 / 4 fn    `lib::grafts`
    └─ `src/lib_tests.rs`  1,015 行 / 32 fn  [cfg(test)]  `lib::tests`
```

### `xirang-mcp`（`mcp/`，39 文件 / 8,682 行 / 242 fn，37 条挂载边）

```
├─ `src/lib.rs`  118 行 / 0 fn  [root]  `lib`
│   ├─ `src/apply.rs`  563 行 / 9 fn    `lib::apply`
│   │   └─ `src/apply_tests.rs`  517 行 / 12 fn  [cfg(test)]  `lib::apply::apply_tests`
│   ├─ `src/callgraph.rs`  120 行 / 1 fn    `lib::callgraph`
│   │   └─ `src/callgraph_tests.rs`  82 行 / 3 fn  [cfg(test)]  `lib::callgraph::callgraph_tests`
│   ├─ `src/converge.rs`  244 行 / 4 fn    `lib::converge`
│   │   └─ `src/converge_tests.rs`  308 行 / 11 fn  [cfg(test)]  `lib::converge::converge_tests`
│   ├─ `src/converge_trace.rs`  256 行 / 2 fn    `lib::converge_trace`
│   ├─ `src/diff.rs`  268 行 / 2 fn    `lib::diff`
│   │   └─ `src/diff_tests.rs`  248 行 / 10 fn  [cfg(test)]  `lib::diff::diff_tests`
│   ├─ `src/evidence.rs`  261 行 / 7 fn    `lib::evidence`
│   │   └─ `src/evidence_tests.rs`  144 行 / 7 fn  [cfg(test)]  `lib::evidence::evidence_tests`
│   ├─ `src/grafts.rs`  124 行 / 1 fn    `lib::grafts`
│   │   └─ `src/grafts_tests.rs`  214 行 / 8 fn  [cfg(test)]  `lib::grafts::grafts_tests`
│   ├─ `src/impact.rs`  299 行 / 4 fn    `lib::impact`
│   │   └─ `src/impact_tests.rs`  191 行 / 7 fn  [cfg(test)]  `lib::impact::impact_tests`
│   ├─ `src/index.rs`  331 行 / 18 fn    `lib::index`
│   ├─ `src/mir.rs`  416 行 / 10 fn    `lib::mir`
│   │   └─ `src/mir_tests.rs`  343 行 / 14 fn  [cfg(test)]  `lib::mir::mir_tests`
│   ├─ `src/nodes.rs`  76 行 / 2 fn    `lib::nodes`
│   ├─ `src/overlay.rs`  187 行 / 3 fn    `lib::overlay`
│   │   └─ `src/overlay_tests.rs`  185 行 / 8 fn  [cfg(test)]  `lib::overlay::overlay_tests`
│   ├─ `src/preview.rs`  180 行 / 8 fn    `lib::preview`
│   ├─ `src/protocol.rs`  301 行 / 8 fn    `lib::protocol`
│   │   └─ `src/protocol_tests.rs`  248 行 / 14 fn  [cfg(test)]  `lib::protocol::protocol_tests`
│   ├─ `src/registry.rs`  128 行 / 4 fn    `lib::registry`
│   │   └─ `src/registry_tests.rs`  102 行 / 4 fn  [cfg(test)]  `lib::registry::registry_tests`
│   ├─ `src/search.rs`  152 行 / 3 fn    `lib::search`
│   │   └─ `src/search_tests.rs`  169 行 / 8 fn  [cfg(test)]  `lib::search::search_tests`
│   ├─ `src/tools.rs`  448 行 / 8 fn    `lib::tools`
│   │   └─ `src/tools_tests.rs`  99 行 / 4 fn  [cfg(test)]  `lib::tools::tools_tests`
│   ├─ `src/trace.rs`  347 行 / 5 fn    `lib::trace`
│   │   └─ `src/trace_tests.rs`  198 行 / 9 fn  [cfg(test)]  `lib::trace::trace_tests`
│   ├─ `src/tree_delta.rs`  120 行 / 2 fn    `lib::tree_delta`
│   ├─ `src/usages.rs`  225 行 / 5 fn    `lib::usages`
│   │   └─ `src/usages_tests.rs`  180 行 / 7 fn  [cfg(test)]  `lib::usages::usages_tests`
│   └─ `src/verify.rs`  98 行 / 2 fn    `lib::verify`
│       └─ `src/verify_tests.rs`  172 行 / 7 fn  [cfg(test)]  `lib::verify::verify_tests`
└─ `src/main.rs`  20 行 / 1 fn  [root]  `main`
```

### `xirang-debug-method`（`debug_method/`，5 文件 / 481 行 / 19 fn，3 条挂载边）

```
├─ `src/lib.rs`  41 行 / 0 fn  [root]  `lib`
│   ├─ `src/adapters.rs`  214 行 / 12 fn    `lib::adapters`
│   ├─ `src/collector.rs`  34 行 / 0 fn    `lib::collector`
│   └─ `src/mir.rs`  155 行 / 6 fn    `lib::mir`
└─ `tests/collector_integration.rs`  37 行 / 1 fn  [root]  `collector_integration`
```

### `xirang-studio`（`studio/`，72 文件 / 12,308 行 / 333 fn，68 条挂载边）

```
├─ `src/bin/xirang-dev.rs`  427 行 / 25 fn  [root]  `xirang-dev`
├─ `src/lib.rs`  14 行 / 0 fn  [root]  `lib`
│   └─ `src/studio/studio.rs`  125 行 / 4 fn    `lib::studio`
│       ├─ `src/studio/app/app.rs`  242 行 / 7 fn    `lib::studio::app`
│       │   ├─ `src/studio/app/call_tree_queries.rs`  283 行 / 7 fn    `lib::studio::app::call_tree_queries`
│       │   ├─ `src/studio/app/graft.rs`  354 行 / 13 fn    `lib::studio::app::graft`
│       │   │   └─ `src/studio/app/graft_tests.rs`  199 行 / 8 fn  [cfg(test)]  `lib::studio::app::graft::graft_tests`
│       │   ├─ `src/studio/app/graph_queries.rs`  188 行 / 5 fn    `lib::studio::app::graph_queries`
│       │   ├─ `src/studio/app/hot_zones.rs`  70 行 / 0 fn    `lib::studio::app::hot_zones`
│       │   ├─ `src/studio/app/interaction.rs`  22 行 / 1 fn    `lib::studio::app::interaction`
│       │   ├─ `src/studio/app/keyboard.rs`  177 行 / 2 fn    `lib::studio::app::keyboard`
│       │   ├─ `src/studio/app/keyboard_overlay.rs`  54 行 / 1 fn    `lib::studio::app::keyboard_overlay`
│       │   │   ├─ `src/studio/app/overlay/add.rs`  39 行 / 1 fn    `lib::studio::app::keyboard_overlay::add`
│       │   │   ├─ `src/studio/app/overlay/delete.rs`  32 行 / 1 fn    `lib::studio::app::keyboard_overlay::delete`
│       │   │   ├─ `src/studio/app/overlay/edit.rs`  44 行 / 1 fn    `lib::studio::app::keyboard_overlay::edit`
│       │   │   ├─ `src/studio/app/overlay/graft.rs`  121 行 / 1 fn    `lib::studio::app::keyboard_overlay::graft`
│       │   │   ├─ `src/studio/app/overlay/new_project.rs`  47 行 / 1 fn    `lib::studio::app::keyboard_overlay::new_project`
│       │   │   ├─ `src/studio/app/overlay/plugin.rs`  60 行 / 1 fn    `lib::studio::app::keyboard_overlay::plugin`
│       │   │   └─ `src/studio/app/overlay/search.rs`  187 行 / 1 fn    `lib::studio::app::keyboard_overlay::search`
│       │   ├─ `src/studio/app/lifecycle.rs`  233 行 / 10 fn    `lib::studio::app::lifecycle`
│       │   ├─ `src/studio/app/mutations.rs`  395 行 / 7 fn    `lib::studio::app::mutations`
│       │   ├─ `src/studio/app/namespace.rs`  98 行 / 2 fn    `lib::studio::app::namespace`
│       │   ├─ `src/studio/app/navigation.rs`  173 行 / 7 fn    `lib::studio::app::navigation`
│       │   ├─ `src/studio/app/pointer.rs`  365 行 / 4 fn    `lib::studio::app::pointer`
│       │   ├─ `src/studio/app/search_queries.rs`  142 行 / 3 fn    `lib::studio::app::search_queries`
│       │   ├─ `src/studio/app/source_index.rs`  102 行 / 5 fn    `lib::studio::app::source_index`
│       │   ├─ `src/studio/app/state/state.rs`  45 行 / 0 fn    `lib::studio::app::state`
│       │   │   ├─ `src/studio/app/state/forms.rs`  263 行 / 9 fn    `lib::studio::app::state::forms`
│       │   │   ├─ `src/studio/app/state/graft.rs`  123 行 / 1 fn    `lib::studio::app::state::graft`
│       │   │   ├─ `src/studio/app/state/misc.rs`  181 行 / 5 fn    `lib::studio::app::state::misc`
│       │   │   ├─ `src/studio/app/state/new_project_field.rs`  48 行 / 1 fn    `lib::studio::app::state::new_project_field`
│       │   │   ├─ `src/studio/app/state/pages.rs`  41 行 / 1 fn    `lib::studio::app::state::pages`
│       │   │   ├─ `src/studio/app/state/plugin_field.rs`  69 行 / 1 fn    `lib::studio::app::state::plugin_field`
│       │   │   └─ `src/studio/app/state/search.rs`  79 行 / 0 fn    `lib::studio::app::state::search`
│       │   ├─ `src/studio/app/support.rs`  509 行 / 23 fn    `lib::studio::app::support`
│       │   ├─ `src/studio/app/tests.rs`  90 行 / 0 fn  [cfg(test)]  `lib::studio::app::tests`
│       │   │   ├─ `src/studio/app/tests/call_tree.rs`  702 行 / 18 fn  [cfg(test)] [feat=prototype-fixtures]  `lib::studio::app::tests::call_tree`
│       │   │   ├─ `src/studio/app/tests/edit.rs`  437 行 / 6 fn  [cfg(test)]  `lib::studio::app::tests::edit`
│       │   │   ├─ `src/studio/app/tests/evidence.rs`  150 行 / 5 fn  [cfg(test)] [feat=prototype-fixtures]  `lib::studio::app::tests::evidence`
│       │   │   ├─ `src/studio/app/tests/fixtures.rs`  62 行 / 1 fn  [cfg(test)] [feat=prototype-fixtures]  `lib::studio::app::tests::fixtures`
│       │   │   ├─ `src/studio/app/tests/forms.rs`  216 行 / 9 fn  [cfg(test)]  `lib::studio::app::tests::forms`
│       │   │   ├─ `src/studio/app/tests/graft.rs`  301 行 / 4 fn  [cfg(test)]  `lib::studio::app::tests::graft`
│       │   │   ├─ `src/studio/app/tests/graph.rs`  214 行 / 4 fn  [cfg(test)]  `lib::studio::app::tests::graph`
│       │   │   ├─ `src/studio/app/tests/navigation.rs`  46 行 / 2 fn  [cfg(test)]  `lib::studio::app::tests::navigation`
│       │   │   ├─ `src/studio/app/tests/project.rs`  562 行 / 18 fn  [cfg(test)]  `lib::studio::app::tests::project`
│       │   │   ├─ `src/studio/app/tests/source.rs`  245 行 / 10 fn  [cfg(test)]  `lib::studio::app::tests::source`
│       │   │   └─ `src/studio/app/tests/trace_ingest.rs`  438 行 / 18 fn  [cfg(test)]  `lib::studio::app::tests::trace_ingest`
│       │   ├─ `src/studio/app/trace.rs`  228 行 / 8 fn    `lib::studio::app::trace`
│       │   └─ `src/studio/app/writers.rs`  42 行 / 2 fn    `lib::studio::app::writers`
│       ├─ `src/studio/terminal.rs`  173 行 / 9 fn    `lib::studio::terminal`
│       └─ `src/studio/ui/ui.rs`  276 行 / 11 fn    `lib::studio::ui`
│           ├─ `src/studio/ui/forms.rs`  32 行 / 0 fn    `lib::studio::ui::forms`
│           │   ├─ `src/studio/ui/forms/delete.rs`  61 行 / 1 fn    `lib::studio::ui::forms::delete`
│           │   ├─ `src/studio/ui/forms/face.rs`  130 行 / 3 fn    `lib::studio::ui::forms::face`
│           │   ├─ `src/studio/ui/forms/face_fields.rs`  87 行 / 2 fn    `lib::studio::ui::forms::face_fields`
│           │   ├─ `src/studio/ui/forms/graft.rs`  258 行 / 2 fn    `lib::studio::ui::forms::graft`
│           │   ├─ `src/studio/ui/forms/plugin.rs`  71 行 / 1 fn    `lib::studio::ui::forms::plugin`
│           │   └─ `src/studio/ui/forms/project.rs`  72 行 / 1 fn    `lib::studio::ui::forms::project`
│           ├─ `src/studio/ui/graph.rs`  115 行 / 2 fn    `lib::studio::ui::graph`
│           │   ├─ `src/studio/ui/graph/data.rs`  129 行 / 1 fn    `lib::studio::ui::graph::data`
│           │   ├─ `src/studio/ui/graph/node_graph.rs`  312 行 / 7 fn  [feat=node-graph]  `lib::studio::ui::graph::node_graph`
│           │   └─ `src/studio/ui/graph/nodes.rs`  273 行 / 6 fn    `lib::studio::ui::graph::nodes`
│           ├─ `src/studio/ui/mark.rs`  14 行 / 0 fn    `lib::studio::ui::mark`
│           ├─ `src/studio/ui/overlay.rs`  97 行 / 1 fn    `lib::studio::ui::overlay`
│           ├─ `src/studio/ui/panels.rs`  201 行 / 4 fn    `lib::studio::ui::panels`
│           ├─ `src/studio/ui/search.rs`  98 行 / 1 fn    `lib::studio::ui::search`
│           │   ├─ `src/studio/ui/search/detail.rs`  317 行 / 3 fn    `lib::studio::ui::search::detail`
│           │   ├─ `src/studio/ui/search/query.rs`  16 行 / 1 fn    `lib::studio::ui::search::query`
│           │   └─ `src/studio/ui/search/results.rs`  97 行 / 3 fn    `lib::studio::ui::search::results`
│           └─ `src/studio/ui/status.rs`  53 行 / 2 fn    `lib::studio::ui::status`
├─ `src/main.rs`  43 行 / 3 fn  [root]  `main`
└─ `tests/launch.rs`  99 行 / 5 fn  [root]  `launch`
```

### `xirang-plugin-host`（`plugin-host/`，15 文件 / 3,888 行 / 136 fn，10 条挂载边）

```
├─ `src/lib.rs`  63 行 / 3 fn  [root]  `lib`
│   ├─ `src/admission.rs`  223 行 / 8 fn    `lib::admission`
│   ├─ `src/deployment.rs`  79 行 / 3 fn    `lib::deployment`
│   ├─ `src/error.rs`  90 行 / 3 fn    `lib::error`
│   ├─ `src/lazy_wasm.rs`  234 行 / 11 fn  [feat=wasm]  `lib::lazy_wasm`
│   │   └─ `src/lazy_wasm/slot_state.rs`  116 行 / 3 fn  [feat=wasm]  `lib::lazy_wasm::slot_state`
│   ├─ `src/process.rs`  443 行 / 14 fn  [feat=process-tools]  `lib::process`
│   │   └─ `src/process/child.rs`  168 行 / 5 fn  [feat=process-tools]  `lib::process::child`
│   ├─ `src/verifier.rs`  128 行 / 7 fn    `lib::verifier`
│   └─ `src/wasm.rs`  405 行 / 9 fn  [feat=wasm]  `lib::wasm`
│       └─ `src/wasm_tests.rs`  38 行 / 2 fn  [cfg(test)] [feat=wasm]  `lib::wasm::wasm_tests`
├─ `tests/admission.rs`  428 行 / 18 fn  [root]  `admission`
├─ `tests/fault_matrix.rs`  968 行 / 32 fn  [root]  `fault_matrix`
├─ `tests/process_load_cost.rs`  214 行 / 8 fn  [root]  `process_load_cost`
└─ `tests/wasm_table_cost.rs`  291 行 / 10 fn  [root]  `wasm_table_cost`
```

### `xirang-example-control-button`（`examples/control-button/`，13 文件 / 1,635 行 / 53 fn，8 条挂载边）

```
├─ `build.rs`  3 行 / 1 fn  [root]  `build`
├─ `examples/graft_record.rs`  205 行 / 5 fn  [root]  `graft_record`
├─ `examples/health_check.rs`  39 行 / 1 fn  [root]  `health_check`
├─ `examples/tree.rs`  10 行 / 1 fn  [root]  `tree`
├─ `src/lib.rs`  85 行 / 2 fn  [root]  `lib`
├─ `tests/health_check.rs`  75 行 / 1 fn  [root]  `health_check`
├─ `tests/ide_mirror.rs`  53 行 / 2 fn  [root]  `ide_mirror`
├─ `tests/registry.rs`  743 行 / 28 fn  [root]  `registry`
└─ `tests/static_plan_allocations.rs`  308 行 / 9 fn  [root]  `static_plan_allocations`
```

构建期生成（工作树里没有对应 `mod` 声明，模块由 `host!()` 的 `include!` 引入）。括号里 `__xirang_ra_*` 是生成器写给 rust-analyzer 的别名声明，`mod` 后面的才是真模块名：

```
· `target/debug/build/xirang-example-control-button-0992fd9f245ea3f0/out/generated_lib.rs`（构建产物，不在 src 树里）
    ├─ `src/control/control.rs`  39 行 / 1 fn  (`mod __xirang_ra_control, control`)
    ├─ `src/control/object/button/button.rs`  40 行 / 1 fn  (`mod __xirang_ra_control_object_button, button`)
    ├─ `src/control/object/slider/slider.rs`  27 行 / 1 fn  (`mod __xirang_ra_control_object_slider, slider`)
    └─ `src/control/registry_rule/registry_rule.rs`  8 行 / 0 fn  (`mod __xirang_ra_control_registry_rule, registry_rule`)
```

### `xirang-example-control-button-graft`（`examples/control-button-graft/`，4 文件 / 134 行 / 1 fn，3 条挂载边）

```
└─ `src/lib.rs`  29 行 / 1 fn  [root]  `lib`
    ├─ `src/button_fast.rs`  36 行 / 0 fn    `lib::button_fast`
    ├─ `src/control_fast.rs`  36 行 / 0 fn    `lib::control_fast`
    └─ `src/slider_fast.rs`  33 行 / 0 fn    `lib::slider_fast`
```

### `xirang-conventions`（`conventions/`，20 文件 / 6,399 行 / 209 fn，19 条挂载边）

```
└─ `src/lib.rs`  508 行 / 19 fn  [root]  `lib`
    ├─ `src/bilingual.rs`  218 行 / 9 fn    `lib::bilingual`
    ├─ `src/doc_anchors.rs`  557 行 / 19 fn    `lib::doc_anchors`
    ├─ `src/doc_blocks.rs`  400 行 / 9 fn    `lib::doc_blocks`
    │   └─ `src/doc_blocks_tests.rs`  221 行 / 11 fn  [cfg(test)]  `lib::doc_blocks::doc_blocks_tests`
    ├─ `src/features.rs`  208 行 / 7 fn    `lib::features`
    │   └─ `src/features_tests.rs`  148 行 / 6 fn  [cfg(test)]  `lib::features::features_tests`
    ├─ `src/lint.rs`  444 行 / 14 fn    `lib::lint`
    ├─ `src/mounting.rs`  358 行 / 9 fn    `lib::mounting`
    │   └─ `src/mounting_tests.rs`  339 行 / 15 fn  [cfg(test)]  `lib::mounting::mounting_tests`
    ├─ `src/naming.rs`  306 行 / 5 fn    `lib::naming`
    │   └─ `src/naming_tests.rs`  215 行 / 11 fn  [cfg(test)]  `lib::naming::naming_tests`
    ├─ `src/purity.rs`  406 行 / 5 fn    `lib::purity`
    │   └─ `src/purity_tests.rs`  268 行 / 11 fn  [cfg(test)]  `lib::purity::purity_tests`
    ├─ `src/release_version.rs`  351 行 / 12 fn    `lib::release_version`
    │   └─ `src/release_version_tests.rs`  165 行 / 8 fn  [cfg(test)]  `lib::release_version::release_version_tests`
    ├─ `src/release_workflow.rs`  357 行 / 11 fn    `lib::release_workflow`
    │   └─ `src/release_workflow_tests.rs`  284 行 / 9 fn  [cfg(test)]  `lib::release_workflow::release_workflow_tests`
    ├─ `src/shims.rs`  146 行 / 2 fn    `lib::shims`
    └─ `src/size.rs`  500 行 / 17 fn    `lib::size`
```

### 非工作区成员的夹具包

`xirang-fixture-node-editor`（`studio/tests/fixtures/node-editor/`，7 文件 / 224 行 / 14 fn，0 条挂载边）

它按 `src/lib.rs` → `host!()` 的方式接线，但**从不编译**（Studio 只把 `src/` 当文本读），因此没有 `$OUT_DIR/generated_lib.rs`，工作树里也没有手写 `mod` 声明——下面这些文件在编译期意义上"未挂载"，这是设计的一部分，不是漏挂：

- `studio/tests/fixtures/node-editor/build.rs` — 10 行，1 fn，挂载状态 `target-root`
- `studio/tests/fixtures/node-editor/src/control/control.rs` — 32 行，1 fn，挂载状态 `未挂载`
- `studio/tests/fixtures/node-editor/src/control/object/node_editor/node_editor.rs` — 101 行，9 fn，挂载状态 `未挂载`
- `studio/tests/fixtures/node-editor/src/control/object/node_editor/object/object.rs` — 49 行，3 fn，挂载状态 `未挂载`
- `studio/tests/fixtures/node-editor/src/control/object/node_editor/registry_rule/registry_rule.rs` — 8 行，0 fn，挂载状态 `未挂载`
- `studio/tests/fixtures/node-editor/src/control/registry_rule/registry_rule.rs` — 8 行，0 fn，挂载状态 `未挂载`
- `studio/tests/fixtures/node-editor/src/lib.rs` — 16 行，0 fn，挂载状态 `target-root`

## 7. 机械观察（只陈述事实，判断留给各片区审计）

1. **规模**：工作区成员 387 个文件 / 78,727 行（src 73,557 + tests 4,763 + examples/build 407），fn 2448 个（其中 pub 923、测试 1029、pub 且非测试 912）；非工作区夹具包另有 7 文件 / 224 行。
2. **函数类型构成**：free_fn 1796、inherent_method 544、trait_impl_method 102、trait_method 12、nested_fn 8。
3. **最大函数 top 20（行数 = fn 行到闭合 `}`，含函数体）**：

| 行数 | 函数 | 位置 | 类型 |
| ---: | --- | --- | --- |
| 243 | `draw_graft` | `studio/studio/ui/forms/graft.rs:8` | free_fn |
| 236 | `render_source` | `run_method/authoring/manifest/face/render.rs:12` | inherent_method |
| 235 | `parse_face_macro_impl` | `run_method/authoring/manifest/parse/parse.rs:30` | free_fn |
| 225 | `run` | `build_method/pipeline.rs:11` | free_fn |
| 215 | `tools` | `mcp/tools.rs:58` | free_fn |
| 201 | `collect` | `core/src/registry_core/syntax/entries/graft.rs:120` | inherent_method |
| 200 | `face_field_presentation` | `core/src/registry_core/authoring/field_presentation.rs:85` | free_fn |
| 190 | `call` | `plugin-host/src/process.rs:246` | trait_impl_method |
| 183 | `overlay_report` | `cli/explain_overlay.rs:54` | free_fn |
| 183 | `handle_overlay_click` | `studio/studio/app/pointer.rs:172` | inherent_method |
| 180 | `handle_search_overlay_key` | `studio/studio/app/overlay/search.rs:7` | inherent_method |
| 174 | `converge_from_trace` | `mcp/converge_trace.rs:45` | free_fn |
| 165 | `guard` | `core/src/registry_core/syntax/nesting.rs:162` | free_fn |
| 164 | `impact` | `mcp/impact.rs:46` | free_fn |
| 162 | `auto_from_entry_reporting` | `build_method/scope.rs:201` | inherent_method |
| 162 | `handle_key` | `studio/studio/app/keyboard.rs:7` | inherent_method |
| 160 | `connector_errors_from` | `core/src/registry_core/tree/connector/connector.rs:221` | inherent_method |
| 158 | `render_node` | `build_method/renderer/tree.rs:40` | free_fn |
| 156 | `usages` | `mcp/usages.rs:33` | free_fn |
| 155 | `explain` | `cli/explain.rs:43` | free_fn |

4. **挂载与门控**：339 条手写 `mod` 挂载边，全部解析成功；其中 `#[cfg(test)]` 挂载 46 条、`#[cfg(feature = ...)]` 门控挂载 11 条。另有 8 条来自构建期生成的 `generated_lib.rs`。
5. **工作树里没有绝对路径 `#[path]`**：`#[path = "/home/..."]` 只出现在 `target/**/out/generated_lib.rs` 这类构建产物里（30 处，同一个包的多份 hash 目录），源码树里 0 处。
6. **`#[cfg(test)]` 挂载的源文件**：`src/` 下经 `#[cfg(test)]` 挂载、且 ≥200 行的文件有 34 个：`cli/lib_tests.rs`(1,015)、`studio/studio/app/tests/call_tree.rs`(702)、`studio/studio/app/tests/project.rs`(562)、`mcp/apply_tests.rs`(517)、`studio/studio/app/tests/trace_ingest.rs`(438)、`studio/studio/app/tests/edit.rs`(437)、`run_method/runtime/trace/artifact/artifact_tests.rs`(426)、`core/src/registry_core/tree/graft_ops/record_tests.rs`(365)、`mcp/mir_tests.rs`(343)、`conventions/src/mounting_tests.rs`(339)（最大的 `cli/lib_tests.rs` 1015 行，靠"挂在 `#[cfg(test)]` 后 + 名字像测试"同时躲过 600 行棘轮，这是 size.rs 明确写下的边界，不是漏网）。
7. **每 crate 的公开面密度**（pub 非测试 fn / fn）：`xirang-core` 45%、`xirang-macro` 46%、`xirang-run-method` 52%、`xirang-build-method` 38%、`xirang-cli` 40%、`xirang-mcp` 18%、`xirang-debug-method` 53%、`xirang-studio` 40%、`xirang-plugin-host` 28%、`xirang-example-control-button` 4%、`xirang-example-control-button-graft` 100%、`xirang-conventions` 17%

---

需要别的切片（例如"某个 crate 的所有 pub fn 及其行数"、"某目录的函数名清单"）直接查 `audit-inventory.json`：

- `crates[].functions[]`：`name` / `visibility` / `is_pub` / `kind` / `container` / `file` / `file_line` / `lines` / `lines_incl_attrs` / `module_path` / `in_cfg_test_module` / `in_test_target` / `cfg_features`
- `crates[].files[]`：`path` / `lines` / `role` / `mounted_by` / `reachable_from_hand_written_mod_decls` / `cfg_test_mounted` / `cfg_features_mounted` / `module_path` / `fn_count` / `pub_fn_count`
- `crates[].module_edges[]`：`parent_file` / `module_name` / `declaration_line` / `path_attr` / `cfg_test` / `cfg_features` / `child_file` / `resolved` / `source`
- `gates`：四条门禁的 exit code、关键输出行与日志路径

