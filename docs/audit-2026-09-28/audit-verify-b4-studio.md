# B4-studio 独立验证（t35）

**对象**：作者 `studio-auditor` 交付的 B4-studio 批次——9 条修复
（`LGC-LG-50`、`STU-S-02/03/05/08`、`STU-C-01/02/03/04`）、新增钉子
`studio/studio/app/tests/b4_studio.rs`（6 条），以及"第 10 条 `STU-S-06` 需内核入口、
本轮不修"的结论。
**验证者**：`logic-adversary`（t35，非作者），装置**全部自建**：8 条行为断言 + 8 处变异反证，
都跑在 `/tmp/t35-verify`（`tar` 副本，`CARGO_TARGET_DIR=/tmp/t35-target`）里；本检出只读，
唯一写入是本文件。

**被验对象的哈希（跑门禁前后逐字节相同，`diff` 为空）**：

```
102b3b108db0cbeb5d5a416c73980b6e  studio/studio/app/writers.rs
705e2ef598c8d208447adc13a344929d  studio/studio/app/search_queries.rs
f32bbf6a3f7d39ff82f22a646a600612  studio/studio/app/support.rs
5568563b1ebc34367dadf339b89858d1  studio/studio/app/navigation.rs
70ffece4aa2d3e117ed98fa0b30f38ff  studio/studio/app/lifecycle.rs
3a465aabf607152493353378d338a8db  studio/studio/app/graft.rs
b1c12e9a2153549afca1f36a14843d8a  studio/studio/app/trace.rs
faca8e1e28e6b6ec98d3e97ddc0bf283  studio/studio/app/mutations.rs
e087bfcb849e8f592322e261bb1c5260  studio/studio/app/app.rs
31fb35b803ce9646192133d766111816  studio/studio/app/state/search.rs
914aad0987debace06cd06af705f304a  studio/studio/app/tests/b4_studio.rs
bae1696fe2a611cbc94d5a05fe2efbad  studio/studio/ui/panels.rs
bf46752b4deb2039ebc55dc7255ed552  core/src/registry_core/source/items.rs
7dcfa50f1761fb8e871cf46a97ce5686  core/src/registry_core/source/source.rs
```

## §1 逐条判定

| # | 条目 | 判定 | 我的装置（不采信作者钉子的绿） |
| --- | --- | --- | --- |
| 1 | `LGC-LG-50` 编辑器启动失败不再被 graft 横幅盖掉 | **证实** | `open_editor_file(missing) → Err` + `event == failure` + `editor_request.is_none()`；真文件仍能打开并带上行号（`mine_editor_failure_is_returned`）。变异 M1（把 `Err` 改成 `Ok`）→ 该断言红 |
| 2 | `STU-S-02` 检视器行清单唯一（12 行；渲染器/计数/按键三者一致） | **证实** | 对夹具**每个**注册面断言 `detail_field_count() == detail_rows().len() == 12`，并把游标推到末行后按 `↓` 断言不越界（`mine_detail_rows_equals_the_count`）；**真渲染一帧**（`TestBackend` 220×60）后逐行数出 12 行标签（`mine_the_renderer_draws_every_row_of_the_list`）。变异 M4（计数 +2）→ 第一条红；变异 M8（渲染器只画 10 行）→ **只有渲染那条红**，正说明两半各管一件事 |
| 3 | `STU-S-03` 快照替换后重裁证据并丢弃旧 MIR | **证实** | `mir_graph = Some(default)` 后 `reload()` 与 `poll_hot_reload()`（真触发：碰夹具源码文件 + 等 600ms）都断言 `mir_graph.is_none()`（`mine_snapshot_swap_drops_the_stale_mir_graph`）；三条调用路径（`install_trace` / `poll_hot_reload` / `reload`）由 `grep rejudge_evidence` 逐一确认。变异 M2（`rejudge_evidence` 不再清 `mir_graph`）→ 红 |
| 4 | `STU-S-05` 搜索备忘以（query, `last_source_stamp`）为键 | **证实** | 端到端：先在夹具源码里追加一个不存在的符号并 `search_rows` 得空 → 追加真函数 + `poll_hot_reload`（更新戳）→ 同一查询必须看到新符号（`mine_search_memo_follows_the_source_stamp`）。变异 M5（备忘忽略戳）→ 红 |
| 5 | `STU-S-08` 插件入口闸门按行判定 | **证实** | 两个方向都由我自己的工程夹具驱动：注释副本不算导入（真行**被追加一次**）；真行已在时不再追加（仍只有一份）（`mine_plugin_entry_gate_reads_lines`）。变异 M3（退回子串判定）→ 第一个方向红 |
| 6 | `STU-C-01` `retreat_graph_focus` 委托 `advance_graph_focus` | **证实** | 对两个起始焦点分别调用两个入口，断言结果状态相同且确实翻转（`mine_retreat_shares_the_advance_body`）。变异 M6（退化为 no-op）→ 红 |
| 7 | `STU-C-02` `writers.rs` 自述与实现相符 | **证实** | 我自己的针：文档必须指向 `super::mutations` 且不得再自称拥有写入；同时断言 `mutations.rs` 确实是写入方（`std::fs::write` + `submit_plugin`） |
| 8 | `STU-C-03` `search_queries.rs` 自述（adjacent）与实现相符 | **证实** | 文档说 adjacent，实现确实是 `dedup_by`（相邻去重） |
| 9 | `STU-C-04` `support.rs` 自述覆盖实际三块内容 | **证实** | 文档必须点名 `cargo` 探测，且不得再只说"交互几何与编辑器辅助" |
| 10 | `STU-S-06` "需内核入口、本轮不修" | **结论已被子批次内的另一笔取代**（详见 §5） | 内核 `item_symbols` 已在（`core/.../source/items.rs`，t36 落地）；Studio 已改调它、前缀词表 0 残留 |
| — | 作者 6 条钉子 | 6/6 跑绿（作为一项输入，不构成判定） | `cargo test -p xirang-studio --offline --all-features --lib b4_studio` → `6 passed` |

**判定口径**：以上 9 条为**独立装置**的结论；作者钉子的绿只作对照。所有行为断言都跑在
`--all-features`（`prototype-fixtures` / `node-graph` 夹具路径），`--test-threads=1`（两个装置会改
夹具源码文件，串行避免互扰）。

## §2 装置与变异反证

装置文件：`/tmp/t35-verify/studio/src/studio/app/tests/zz_t35_verify.rs`（仅在副本里由 `tests.rs`
挂载），8 条断言：

```
mine_detail_rows_equals_the_count
mine_editor_failure_is_returned
mine_module_docs_match_their_modules
mine_plugin_entry_gate_reads_lines
mine_retreat_shares_the_advance_body
mine_search_memo_follows_the_source_stamp
mine_snapshot_swap_drops_the_stale_mir_graph
mine_the_renderer_draws_every_row_of_the_list
```

副本 + `CARGO_TARGET_DIR` 自带的变异脚本：`/tmp/t35-mutations.sh`（M1–M7）与手工 M8。每次变异后
**从 pristine 副本还原并 `cmp`**：

| 变异 | 目标 | 改法 | 红侧 |
| --- | --- | --- | --- |
| M1 | `support.rs:488-491` | `return Err(failure)` → `Ok(())` | `mine_editor_failure_is_returned` FAILED（`:102`） |
| M2 | `trace.rs:85` | 删 `self.mir_graph = None;` | `mine_snapshot_swap_drops_the_stale_mir_graph` FAILED（`:148`） |
| M3 | `mutations.rs:346` | 行判定 → `entry_text.contains(entry_line)` | `mine_plugin_entry_gate_reads_lines` FAILED（`:225`） |
| M4 | `lifecycle.rs:234` | `detail_rows().len()` → `+ 2` | `mine_detail_rows_equals_the_count` FAILED（`:71`） |
| M5 | `search_queries.rs:37` | 戳比较恒真 | `mine_search_memo_follows_the_source_stamp` FAILED（`:203`） |
| M6 | `navigation.rs:174` | `advance_graph_focus(search);` → `let _ = search;` | `mine_retreat_shares_the_advance_body` FAILED（`:270`） |
| M7 | `writers.rs:6` | 恢复旧自称（只改首字母大小写） | `mine_module_docs_match_their_modules` FAILED（`:298`）——**加固后**才红，见 §6 |
| M8 | `ui/panels.rs:124` | 渲染器只画 10 行 | `mine_the_renderer_draws_every_row_of_the_list` FAILED（`drawn=10 of 12`），而计数那条仍绿 |

**零残留**：7 个被变异文件 + 夹具源码全部 `cmp` 与 pristine 相同（`identical`），且还原后装置
8/8 复绿。**一处方法学注记**：`cp -a` 保留 mtime，运行 `cargo test` 前必须 `touch` 还原过的文件，
否则 cargo 认为目标比源码新而**不重编**，会把变异版的 `include_str!` 内容继续跑下去（我第一次
"还原后仍红"就是这个原因，已用 `touch` 排除）。

## §3 行为类改动的端到端核（不只是改文案）

| 声明 | 端到端证据 |
| --- | --- |
| `open_editor_file → Result`，graft 横幅不再覆盖失败 | `Result` 由真调用驱动（缺文件 → `Err` 且 `event` 带同一句、`editor_request` 保持 `None`；真文件 → `Ok` 且请求带 `(path, line)`）。graft 调用点读 `open_editor_file(path.clone(), 1).err()` 并把失败前置到横幅（`graft.rs:126,145`），M1 证明这条 `Result` 是有牙齿的**行为契约**；**未覆盖**见 §6(a) |
| `App::detail_rows()` 是检视器唯一来源（12 行，三方一致） | 计数：`detail_field_count() == detail_rows().len() == 12`（每个面）；按键：末行按 `↓` 不越界；渲染：真帧里逐行数出 12 行（`panels.rs:124` 逐个消费 `detail_rows()`，M8 证明换掉它会被抓） |
| `rejudge_evidence()` 被三条替换路径共用（并丢弃陈旧 MIR） | 三条调用点逐一 `grep` 确认；行为：`reload` 与 `poll_hot_reload`（真触发）都清 `mir_graph`（M2 反证） |
| `search_memo` 以（query, `last_source_stamp`）为键 | 真源改动 + 热重载后同查询看到新符号（M5 反证）；**未覆盖**：性能收益本身（见 §6(e)） |
| 行基插件入口闸门 | 注释副本 → 追加一次真行；真行已在 → 不追加第二份（M3 反证） |
| `retreat_graph_focus` 委托 `advance_graph_focus` | 两个起始焦点下两入口结果相同且翻转（M6 反证） |

## §4 "零新增公开符号"的主张

口径：`git diff -- '*.rs'` 的 `+` 行里筛 `pub` 声明，`pub(crate)` / `pub(super)` / `pub(in …)`
**不计公开面**。

- **studio（本批对象）：0 处新增公开符号** —— 新增的可见性只有
  `pub(crate) fn detail_rows`、`pub(crate) type SearchMemo`、`pub(crate) use search::SearchMemo`、
  `pub(super) fn rejudge_evidence`、以及 `panels.rs` 里的 `pub(super)` 项。作者的主张在本批范围内
  **成立**。
- **全工作树 diff：另有 3 处新增公开面，都不属本批**：`build_method/face_view.rs` 的
  `pub fn face_views_and_unreadable`、`core/src/registry_core/authoring/parse/rules.rs` 的
  `pub fn try_parse_requirements_owned`（t32 的 LG-28 严格兄弟入口）、以及
  `core/src/registry_core/source/source.rs:23` 的 `pub use items::*;`（t36 的 `item_symbols`）。
  它们都是**新增**（无签名变更/删除），因此：
  - **不动版本线**：根 `Cargo.toml:7` 仍是 `version = "0.1.6"`，diff 里没有任何 manifest 的
    `version = "…"` 改动（唯一命中是文档里引用的旧字符串）；
  - **对 `package-audit` 的含义**：这三项是"已发布面变大"，不改变包内容检查（`cargo package
    --list`）的结论；按本仓的 semver 政策它们应在下次发布时带上 minor 版本，而不是本轮的 tag
    （本轮并未动版本线）。若要严格对齐"零新增公开符号"，它们的 owner 各自记账即可——本批不欠。

## §5 `STU-S-06`："未修 + 需内核"是否成立

**在当前树上：结论已被子批次内的另一笔（t36）取代——该条实际已修，不再是欠账。**

判别性证据：

1. **内核入口已在**：`core/src/registry_core/source/items.rs:56` 的
   `pub fn item_symbols(source: &str) -> Vec<SourceItem>`（新文件，`git status` 为 `??`，mtime
   **21:09**），在 `source.rs:22-23` 挂载并 `pub use items::*;` 导出 ⇒ 可达路径
   `xirang::source::item_symbols`（正是 finding 的 fix_hint 要的那份"声明/符号清单"）。
2. **Studio 侧前缀词表已 0 残留**：`studio/studio/app/search_queries.rs` 里旧的关键字前缀表
   0 命中（`grep -c 'strip_prefix("pub\|"struct" | "enum"\|INTRODUCERS'` → 0）；该文件（mtime
   **21:30**，晚于作者 20:44 的钉子）第 7 行 `use xirang_run_method::source::item_symbols;`，
   `source_rows_for_text` 直接 `for item in item_symbols(text)` 渲染（`:122`），注释明写"声明词表
   归内核"。
3. **作者的"需内核入口"前提在**其**交付时刻（20:44）**成立——那时 `items.rs` 还不存在（21:09 才
   出现，且是未跟踪新文件）；此后被 t36 取代。**因此这条不是"未修"、也不是"作者判断错误"，而是
   "结论过期"**：账目应把它从"本轮不修/需内核"改成"已由 t36 关闭"，作者无需返工。

## §6 未覆盖与局限（如实列出）

- **(a) graft 横幅覆盖失败那条的端到端触发不可达**：`plan_path()` 指向刚被写入的计划文件，
  没有接缝能让它在 `open_editor_file` 之前消失，因此"失败 + 横幅"这一组合只能**结构化验证**
  （调用点 `.err()` 消费 + `Some(failure) => format!("{failure}\n{banner}")`）＋**语义变异**（M1
  把 `Result` 变成永远成功，我的行为断言即红）。作者同样只用源码断言覆盖这一处。
- **(b) 文档类钉子是大小写敏感的**：M7 第一次（只把旧句首字母大写）**没有**让我的检查变红——
  作者的 6 条钉子里有 4 条是 `include_str!` 文本断言，同样会被"改大小写/改词序"的重写绕过。我把
  自己的否定针改成大小写无关后才抓住（红侧已补）。这是**低危**（必需的正向针仍在，且真正的主张
  缺席才算漂移），但值得记：文本钉子的强度上限就是它的字面量。
- **(c) 4/6 作者钉子是源码文本断言**（`module_docs_*`、`every_snapshot_swap_*`、`search_rows_is_
  memoised_*`、`a_failed_editor_launch_*` 各含文本断言）。它们不驱动行为，只钉"实现里还写着这句
  话"。本报告的 §1/§3 用行为装置补上了这一层；建议（不属本任务改动）把这几条逐步换成行为断言。
- **(d) `STU-C-04` 的另一条修法（拆模块）未采用**：作者选了"按实际内容改文档"，finding 允许二选一
  ⇒ 不算缺口。
- **(e) `STU-S-05` 的性能收益未验**：我只验了"键含戳 → 外部编辑可见"和"同一查询给出同样的行"
  （备忘不改变答案）。"一帧少扫几次源码"这条**没有**测（需要计数器或基准），本报告不据此下结论。

## §7 新发现

1. **`STU-S-06` 结论过期**（§5）：账目应从"本轮不修 + 需内核入口"改为"已由 t36 关闭"。建议在
   修复轮记账里更新，而不是再开一单。
2. **文本钉子的上限**（§6b）：4/6 钉子对"改大小写/改词序的重写"不设防；建议后续把文档类与
   "调用点契约"类钉子换成行为断言（我的 `/tmp/t35-verify` 装置可直接搬到仓里当起点）。
3. **`detail_field_count()` 与渲染器的一致性今天靠"两者都读 `detail_rows()`"**：M8 证明若将来有
   人只在渲染器里截断，行为计数断言不会报，只有真渲染断言会。建议把"渲染出 12 行"这条行为钉子
   留在仓里（现在仓里没有这一条）。

## §8 门禁（静置/哈希钉住的树上）

**最终（22:06:32–22:12:11，对象在整轮前后 `diff` 为空 `OBJECT_STABLE=yes`）**：

| 命令 | 结果 |
| --- | --- |
| `cargo test -p xirang-studio --offline --all-features` | **exit 0**（22:06:32，多个 `test result: ok`，含作者 6 条钉子） |
| `cargo test --workspace --offline` | **exit 0**（22:12:11 重跑；见下） |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | **exit 0**（22:07:37） |
| `cargo test -p xirang-conventions --offline` | **exit 0**（22:12:11 重跑） |
| `cargo fmt --all -- --check` | **exit 0**（22:08:00） |

**并发红的作废与重跑记录**（都落在本批文件之外，按队规作废）：

1. 首轮（21:56:42）：clippy/conventions 101 —— `conventions/src/release_workflow.rs:117 cannot find
   release_action_pin in crate`（他人在飞）；fmt 1 —— `mcp/apply_tests.rs`、`mcp/mir_tests.rs`。
2. 22:02:43：clippy/conventions 仍 101（`doc list item without indentation`，同一批在飞文件）；fmt 已 0。
3. 22:06:21：三条全部 0（clippy / conventions / fmt）。
4. 22:07:12：workspace 101 与 conventions 101 —— `size::size_tests::the_size_ceiling_holds_except_for_
   the_pinned_debt` FAILED（他人在飞把某个文件推过 600 行棘轮）；22:12:11 重跑转绿。

被验对象的 14 个哈希在首轮门禁、22:06 最终轮、22:08 复核轮三次运行里都逐字节相同（`diff` 为空）
⇒ 结论针对的是一个静止对象；上列红全部由并发在飞改动造成，与本批交付无关。
