# 独立验证：t64 的 SUR-S11 行为修复（注释感知的规则读取）

- 验证者：kernel-auditor（t74，**非作者**：作者 boundary-architect）
- 真实树**零改动**（本轮只写本文件）；我的夹具与变异在 `/tmp/nk-t74`（自带 `CARGO_TARGET_DIR=/tmp/nk-t74-target`）
- 判定：**SUR-S11 证实**（三轴全部成立，API 与顺序不变）；出厂 3 份规则源**未过火**

## 1. 我的装置（不采信作者钉子原文）

副本内新增 `build_method/t74_probe_tests.rs`（经 `#[cfg(test)] #[path]` 挂在本模块上，因此可调私有 `rule_method_strings`），6 条用例全部用**我自己的数据**（`t74.*` 前缀）：

| 用例 | 断言 |
| --- | --- |
| `a_commented_example_is_not_a_requirement` | 行注释与块注释里的 `.require_exports(&["ghost.export"])`/`["ghost.block"]` 都不是需求；同文本里的真实调用读到 → `vec!["control.render"]` |
| `both_calls_to_one_method_are_read_in_order` | 同一方法调两次 → `["t74.first","t74.second"]`（顺序保持）；另一方法 → `["t74.trait"]` |
| `nested_parentheses_do_not_truncate_the_argument_list` | 参数含 `pick(double(1), (2, 3))`、闭包 `call(\|\| other())` → 仍读到 `["t74.deep"]`，且后面的 `.require_preset` 正常 |
| `comments_inside_the_arguments_are_skipped_and_escapes_are_decoded` | 参数内注释 `/* "t74.commented" */` 不产出需求；`"t74.quote\"inside"` 解码为 `t74.quote"inside`；`r"t74.raw"` 原样 |
| `an_unused_method_is_empty_and_an_unbalanced_list_reads_to_the_end` | 未被调用的方法 → 空；括号始终不配平 → 读到底（不猜边界） |
| `the_shipped_rule_sources_still_read_to_their_requirements` | 出厂 3 份源各读出 1 条：`examples/control-button` → `["control.render"]`、studio 根 → `["control.render"]`、studio object → `["node_editor.render"]` |

6 条在修复后的树上**全绿**；`cargo test -p xirang-build-method --offline` 也随之 EXIT=0。

## 2. 三条红侧（队长点名的两轴 + 我的补轴）—— 全部先跑红

| 红侧 | 变异（副本内） | 我用例的红原文 |
| --- | --- | --- |
| ① 注释示例不得成为需求 | `code_occurrences` 换回朴素 `match_indices` | `left: ["ghost.export","ghost.block","control.render"] right: ["control.render"]` |
| ③ 嵌套 `)` 不得截断 | `call_end` 换回第一个 `)` | `left: [] right: ["t74.deep"]` |
| ④ 转义/原始字符串要解码 | `unescaped` 换成恒等（返回原文本） | `left: ["t74.quote\\\"inside","t74.raw"] right: ["t74.quote\"inside","t74.raw"]` |

②"同一方法调两次两处都读到"由我用例 2 直接覆盖（真实树上绿）；它的红侧是**修前的整个缺陷**（修前该形态只读到第一处——与 ① 同一次修复引入的遍历所有出现处）。每条变异还原后 `copy == pristine` 逐字节相等（Python `read_bytes()` 比较，三条全 True）；判定只取 hash 稳定那次（我 pin 的 `contracts.rs 0f93e0d0…` 在验证前后一致）。

## 3. 反过火

- **出厂规则源**：见 §1 最后一条用例——3 份 `registry_rule.rs` 各读出 1 条需求，值与源码一致（studio object 的是 `node_editor.render`，属正例）。
- **build_method 现有用例**：`cargo test -p xirang-build-method --offline` EXIT=0；`cargo test --workspace --offline` EXIT=0（58 个 ok 块），即两个出厂宿主的整条构建链（含规则读取）仍全过。
- **注释抽查（作者主张"一字未删"）**：我读了 `renderer/pass.rs:140-146`（说明"宿主可能开着 `#![warn(missing_docs)]` 且无法编辑生成文件 → 裸静态量保持隐藏、只有宿主真正调用的两个函数带文档"）与 `renderer/tree.rs:78-96`（说明"为什么必须 `#[path]` 而不是 `include!`：保住 `//!` 头、保住 `file!()`；以及为什么 IDE 影子声明必须存在、`cfg(rust_analyzer)` 下 rustc 只读真实声明"）。两段留的都是**为什么**（非直觉取舍），不是复述代码 ✓。其它段落我没逐字比对 `git show HEAD:`，因此"一字未删"这句我只能抽查背书。

## 4. 门禁（同一批 hash）

`cargo test -p xirang-build-method --offline` **0**；`cargo test --workspace --offline` **0**（58 ok 块、0 FAILED）；`cargo clippy --workspace --all-targets --offline -- -D warnings` **0**；`cargo test -p xirang-conventions --offline` **0**（131 passed）；`cargo fmt --all -- --check` **0**（0 行）。

## 5. 未覆盖 / 新发现

1. **未覆盖**：`literal()`/`skipped()` 的完整形态矩阵（我覆盖了行/块注释、普通串、原始串、转义引号、字符位置、字节串**未**单独覆盖）；`call_end` 的 `None`（全程不配平）我只在"缺一个 `)` 读到尾"这一形态上测过；作者钉子里我没有逐条复跑（按任务书不采信其原文）。
2. **新发现（low）**：`rule_method_strings` 对**注释里的方法名**已免疫，但对**同名方法的文档性提及**（例如 Markdown 代码围栏之外的散文 `.require_exports(...)`）无从区分——这是文本层面的固有边界，只能靠注释纪律；建议在 `contracts.rs` 的注释里写一句"不要在非注释散文里写方法调用形态"，免得下一个人以为门禁能兜住。
3. **行为变更背书**：t64 自述"以前被漏读的第二处需求、以前凭空读出的注释需求，从此走向正确，可能导致重复调用的规则源开始报出以前静默通过的契约错"——我在我的夹具上确认了这个方向（用例 2 读两处、用例 1 不再读注释），**未**去跑任何真实宿主的契约错回归（两个示例仍绿，见 §3）。
