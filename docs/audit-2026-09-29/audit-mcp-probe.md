# xirang MCP 能力面实测 + codegraph 同题调试对照（2026-09-29）

**装置**：`target/debug/xirang-mcp`（`cargo build -p xirang-toolchain --offline --features mcp --bin xirang-mcp`，
9 月 29 日 16:35 构建，源码未改）。stdin/stdout 上 JSON-RPC 2.0；驱动脚本写在 `/tmp/mcpprobe.py`
与 `/tmp/probe_batch.py`，**未入库**。

**被测根**：
- 真实宿主 `examples/control-button`（3 个手写面，真实 `static_graft_plan!` 类型化计划）；
- 沙箱副本 `/tmp/probe/control-button`（把该示例 `cp -r` 出去、把 `path` 依赖改成绝对路径）——
  写入类工具只在这里跑，仓库一个字节都没动。沙箱里的 graft 计划、MIR 转储、trace artifact 见下；
- 仓库根 `/home/nich/Moirai_N3/nichlink`（虚拟工作区，用于第③节的符号类问题）。

**真实输入**（不是伪造格式）：MIR 文本用 `RUSTC_BOOTSTRAP=1 cargo rustc -Zunpretty=mir` 从该示例真实产出
（2174 行）；trace artifact 用仓库自己的 `write_trace_artifact` 从沙箱包真实写出；graft 计划用仓库
`GraftPlanDocument::render_graft_plan_document` 的格式写。仓库里没有 `.mir`/`.xirang/` 成品，
所以这三类输入必须现造——凡与"仓库真实对象"有出入的地方，下文逐条标注。

**收尾**：`git status --porcelain` 只有本文件一处。

---

## ① 工具对账表

`tools/list` 返回 **17** 个工具，名字与顺序与任务书逐字一致，也与 `tools.rs` 的目录、
`DISPATCH` 表逐项一致：

| # | 注册名 | 实现 | 一次调用的实测结论 |
|---|---|---|---|
| 1 | `xirang.search` | `search.rs` | 可用；面/文件/函数三档 + 构建结论 + `unparsable faces N`。**只索引 fn**，const/类型/声明内容搜不到 |
| 2 | `xirang.inspect` | `tools.rs:393` | 可用；函数区间 + 直接调用名 + `registrations:` |
| 3 | `xirang.callgraph` | `callgraph.rs` | 可用但**按名匹配**（自报 `static-heuristic`）；跳数只有 1 跳 |
| 4 | `xirang.read` | `tools.rs:418` | 可用；行号/上下文都夹紧，越界不报错 |
| 5 | `xirang.status` | `tools.rs:455` | 可用（无参数也能答） |
| 6 | `xirang.apply` | `apply.rs` | 可用（沙箱内 add/edit/rename/delete 全环跑通）；字段形状与描述不符，见 M1–M3 |
| 7 | `xirang.registry` | `registry.rs` | 可用；面表带 NodeId |
| 8 | `xirang.explain` | `build_evidence.rs:99` / `overlay.rs` | 可用；`overlay` 与 `node` 互斥被拒绝得清楚 |
| 9 | `xirang.diff` | `diff.rs` | 可用但**依赖构建证据**；未 `verify` 前答 "no build evidence" |
| 10 | `xirang.trace` | `trace.rs:144` | 可用；缺失时给出产出办法，正值路径见② |
| 11 | `xirang.mir` | `mir.rs:47` | 文本/JSONL 双读可用；**`jsonl: true` 的回环在 >400 行时坏掉**（X2） |
| 12 | `xirang.unified` | `mir.rs:144` | 可用；实测 `live 1 / compiler candidates 80` |
| 13 | `xirang.grafts` | `grafts.rs` | 可用且与 CLI `xirang grafts` 同答；一处匹配规则见 X3 |
| 14 | `xirang.impact` | `impact.rs` | 可用；`depth: 0` 静默夹到 1 |
| 15 | `xirang.usages` | `usages.rs` | 可用；字段读回只在**生成模块**上成立（真实示例三个面全 unreadable） |
| 16 | `xirang.converge` | `converge.rs` | 可用；`answered by` 实测成立，**`UNANSWERED` 字面不可达**（M4） |
| 17 | `xirang.verify` | `verify.rs` | 可用；但它与 6/15/16 会在同一棵树上给出相反结论（X1） |

**注册了没实现 / 实现了没注册：0 处。** 实测：`tools()` 与 `DISPATCH` 均 17 项、顺序一致；
`grep '^pub(crate) fn ' toolchain/src/mcp/*.rs` 里的全部入口都是助手
（`load_registry`/`source_layout`/`read_verified`/`namespace`/`entries` 等）或已被 `explain_tool`
路由的 `overlay`，没有游离的 handler。这条也被测试钉住并实测通过：

```
test mcp::tools::tools_tests::the_dispatch_table_follows_the_catalog ... ok
```

**描述与行为不符（4 处，按严重度）**

- **M1 `apply.fields.module` 是"裸 snake_case 名"，不是描述里的普通字段**。仓库里所有真实面写作
  `control::object::button`，照抄会失败：
  ```
  invalid module name `control::object::widget`; use snake_case ASCII
  ```
- **M2 `apply.fields` 除 `needs_registry` 外一律必须是字符串**，而 `tools/list` 只把 `fields` 声明成无类型 object。
  面上真实的 `exports: ["control.render"]` 用数组写会被拒：
  ```
  `exports` must be a string
  ```
  `requires` 还要 `capability=>provider` 形状：`requires entries must use capability=>provider syntax`。
- **M3 `handle_traits` 是"标签"不是 Rust 路径，且这个错会被父规则而不是工具本身报出来**。传
  `crate::control::ControlHandle` 时工具**预览成功**并写出 `handle_traits: ["crate::control::ControlHandle"]`，
  随后父注册规则才拒绝：
  ```
  error: registration rule rejected `Widget` for registry `fb97ddd5…` (rule `src/control/registry_rule/registry_rule.rs`):
    handle `Widget` must implement interface `ControlHandle`
  ```
  正确写法是裸标签 `ControlHandle`（用 `handle_contracts` 传路径也行——它会反推标签）。
- **M4 `converge` 承诺的 `UNANSWERED` 字面拿不到**。`converge.rs:156` 那一支要求"内核接受这棵树、
  但需求没人满足"；实测里需求没人满足时 `load_registry` 先失败，读者拿到的是
  `kernel verdict: this package's own faces are rejected` + connector 诊断。**结论本身是对的，
  但工具描述的措辞与读者看到的东西不是一回事。**

---

## ② 逐工具实测（失败原文）

**2.1 可用且正向闭环（真实对象）**

```
xirang.registry        → faces 3: root/control fb97dd… / root/control/button ff1c57… / root/control/slider bdb442…
xirang.explain node=root/control/button → build current / scope selected (by id, mode=auto) / pruning nothing to strip
xirang.search query=Button → face root/control/button kind=Button module=control::object::button [ok]
xirang.inspect src/control/object/button/button.rs → fn paint lines 10-12 calls=[] / registrations: Button
xirang.read  line=999999 context=120 → 夹成 src/lib.rs:1-85（不报错）
xirang.impact node=root/control → affected 4，hop1 两个子面、hop2 两条 declared cut
xirang.grafts → 与 `xirang grafts <root>` 逐行同答（含 unkept 计数）
xirang.verify → verdict ok / faces 3 (source) vs 3 (build)，并把 target/xirang/out 刷新出来
```

**写入路径（沙箱副本，`apply: true` 真写）**：

```
add   module=widget kind=Widget exports=control.render handle_traits=ControlHandle
      → action apply / applied …/src/control/object/widget/widget.rs / faces 4
edit  provides=control.theme      → applied（旧文留在 .xirang/trash/faces/…）
rename module=widget2             → 预览给出新旧两行 diff
delete confirm=true               → 预览 "would move …/.xirang/trash/theme-36a6c2…"
usages node=root/control/widget   → fields (read back from the generated module)：module/preset/parts/name_zh/name_en/
                                    exports/handle_traits/registration_rule ANY/needs_registry false …（23 项全给）
converge node=root/control/widget → requires control.theme=>Theme
                                      control.theme => Theme  answered by root/control/theme      ← 头条判断成立
```

**MIR / trace / unified（真实产物）**

```
xirang.mir path=button.mir        → functions 11 calls 168 locals 321
xirang.unified path=button.mir    → trace /tmp/…/xirang.trace ; relations 81 (live 1, compiler candidates 80)
xirang.trace                      → frames 2 locals 3 edges 2 ; call tree 带 declared-at / call-at
xirang.trace values=true          → data edges 2：label -> painted (transform) / painted -> frame (used by render::frame)
xirang.converge trace=true        → faces that ran (0 of 4 declared, matched by source file)   ← 见 M5
```

**2.2 失败与误导的原文**

```
# X2 mir jsonl 回环（它自己的描述承诺 "what it prints reads back here"）
$ tail -1 button.jsonl
… truncated: 501 lines total, 400 shown. pass what this prints to a JSONL-suffixed file and `xirang.mir` reads it back.
$ xirang.mir path=button.jsonl
MirParseError { line: 401, message: "record must start with `{`" }
# 小图（small.mir，1 fn/1 call/1 local）写出的 small.jsonl 能读回 ⇒ 只有 >400 行才坏

# X1 同一棵树、同一时刻、两个工具相反
xirang.verify  → verdict ok (the kernel accepted the tree) / faces 4 (source) vs 4 (build) / 无诊断
$ xirang check /tmp/probe/control-button
xirang check: ok (xirang-example-control-button)
xirang.usages node=root/control/widget
the package's own faces were rejected: … <registry-connector>:0:0 … function=Registry::connector_error
  +-- error: data-flow attachment failed: input `control.theme` has no provider; expected provider kind `Theme`

# M4 UNANSWERED 不可达（同一棵树的另一时刻）
xirang.converge node=root/control/widget → kernel verdict: this package's own faces are rejected …

# X3 graft 计划匹配忽略 target_path（手工写坏的计划被报成 declared）
#    .xirang/external-grafts/orphan_fast/graft.plan: target=button 的 NodeId, target_path=root/elsewhere
xirang.grafts →
  button_fast: target=root/control/button graft=button_fast full=false [declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`]
  orphan_fast: target=root/elsewhere graft=orphan_fast full=true [declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `…::button_fast::NODE_ID`]
# 对照：把 target 换成真没人声明的面（root/control 的 NodeId）后，两边都正确报 unkept/undeclared
xirang.grafts → control_fast: target=root/control graft=control_fast full=false [NOT declared by the host entry]
                  unkept plans 1: the release prunes these slots, so the records can never take effect. …
xirang.diff records=true → ok 1  undeclared 1  stale 0  re-identified 0  unreadable 0

# M6 搜不到符号级事实
xirang.search query=DISPATCH → fn dispatch -> toolchain/src/mcp/protocol.rs:246   （常数 DISPATCH 不在索引里）
xirang.search query=control.render → no matches        （4 个面都声明了 exports: ["control.render"]）

# 其它失败原文（都是可行动的，记录用）
xirang.explain overlay+node → overlay renders the whole effective tree; drop `node` (use it without `overlay` for one face)
xirang.mir path=nope.mir    → … is not a readable file; produce a text dump with `cargo rustc -Zunpretty=mir` on a nightly toolchain …
xirang.trace（无 artifact）  → trace absent: <root>/.xirang/traces/xirang.trace + XIRANG_TRACE / XIRANG_TRACE_FILE 的产出办法
xirang.read path=../../../etc/passwd → path must stay inside the configured source root
xirang.diff（未构建）        → no build evidence: run `xirang check` (or `xirang build`) first …
unknown tool `nonexistent.tool`（空参调用实测：10 个工具各自点名自己缺哪个参数，status/registry/explain/diff/trace/grafts/verify 7 个无参可答）
```

**2.3 M5：两个工具对"哪个面跑了"给出不同答案**

`xirang.trace` 打 `declared-at=control/object/button/button.rs:15:1`（来自帧的 **node 身份**），
而 `converge_trace.rs:77` 把帧归到"哪个面跑了"时用的是 `source.file`，也就是**调用点所在文件**
（`call-at=examples/trace_probe.rs:13:14`）。同一份 artifact 于是同时得到
"这个帧属于 button 面"和"0 of 4 declared faces ran"。真实宿主里 face 被别的文件调用是常态，
所以这不是我的构造问题。

---

## ③ 6 题对照

**装置差异（必须写明）**：xirang 侧 = 我直接驱动 stdio 桥，题目是**工具名 + 参数**；
codegraph 侧 = 队长手上的 `codegraph_explore` MCP 服务，题目是**同义自然语言问句**，每题**只调一次、无追问**，
`maxFiles=3`；原始转录在 `/tmp/codegraph-baseline.md`（180 行），"codegraph 原文"一列是从那份转录抄的，
凡它自己打印的 `Some file sections were trimmed for size.` 与 `+N more` 都按转录记录。
两侧是**两个不同客户端**、同一台机器、同一份 `.codegraph/` 索引。

| 题 | xirang 原文（要点） | codegraph 原文（要点） | 谁更稳，为什么 |
|---|---|---|---|
| **C1** 链 `register_snapshot_batch → connector_error` | `matches 1`；callers (41) 列 20 行 + `… +21 more`；**callees 里直接有 `connector_error`** ⇒ 链长 1 跳可读出 | 不列链。符号袋：`register_snapshot_batch`(transaction.rs:32) 41 callers（列 4 + "+4 more"）、`connector_error`(connector.rs:180) 2 callers（其一在 transaction.rs）；返回源码里 `transaction.rs` **只有测试段**，另附无关 `wasm.rs` | **xirang 略稳**：它的 callee 集是**直接调用且完整名单**，`connector_error` 就在里面，一步可判；但它那 18 个 callee 里混着 `Err/Ok/Some/clone/into/new` 这类名字噪声，不能照抄。codegraph 的链只能"推"，且同一次返回里夹无关文件 |
| **C2** 链 `graft_plan_rows → parse_graft_plan_document` | 一次调用给 callers 4（含 `overlay_projection`/`diff_records`/`grafts`）+ **callees 里有 `entry_rows`**；补问一次 `callgraph entry_rows` 后 callees 里有 `parse_graft_plan_document` ⇒ 两次调用闭链 | 一次调用就把 `plan_rows.rs` **整文件**给出，`:85 → :102 → :148` 三步一次读完；另附两块无关源（`GraftState`、`FaceView`） | **codegraph 更稳**：一次调用给足闭链所需源码；xirang 要两次，且两次都只给"名字集合"，没有路径行 |
| **W1** `names_face` 在哪 / 谁调用 | 定义 `toolchain/src/build_time/src/graft_view/declared.rs:138`；callers(7) 逐条 `文件::函数`（含测试 `a_string_range_names_both_endpoints_as_data`）+ limit 截断 | 定义 `declared.rs:138` 精确；"6 callers" 列 4 文件 + "+2 more"；源码给全；**`⚠️ no covering tests found`** | **xirang 更稳**：7 条调用点全展开（含测试），codegraph 截掉的两条里正好有钉住它的测试。已实测该告警是假阴性：`test build_time::graft_plan_check::tests::a_string_range_names_both_endpoints_as_data ... ok`（该测试直接 `assert!` 区间两端与字面 `" to "` 两种行为）。另：grep 实测 6 个生产调用点 + 1 个测试函数，两边的"6/7"是口径差，不是谁错 |
| **W2** `MirGraph::from_mir_text` 在哪 / 谁调用 | 定义 `kernel/src/registry_core/mir/text.rs:23`；callers(4) = 同文件两条测试 + `mcp/mir.rs::load_mir` + `studio/app/lifecycle.rs::load_mir_snapshot` | 定义 `text.rs:23` 精确；"4 callers in text.rs, mcp/mir.rs, studio/app/lifecycle.rs"；**`⚠️ no covering tests found`**，而**它这次返回的 `text.rs` 全文里就有那两条 `#[test]`** | **xirang 更稳**：同一份证据里它把测试点名了；codegraph 同一份返回自相矛盾。已实测两条测试存在且通过：`registry_core::mir::text::tests::parses_native_textual_mir ... ok`、`…yields_an_empty_graph ... ok` |
| **I1** 给 `provider_for_capability` 加参数 | `matches 1`；**callers (1) = connector.rs::connector_errors_from**（点名到函数），callees 7 条 | "1 caller in connector.rs"（同文件）；顺带列 `provider_for`/`provider`/`changed`/`registry` 等同名噪声 + `face_field.rs` 26 个槽位常量，**没给调用点行号** | **xirang 更稳**：唯一调用点被点名（`connector_errors_from`），改动面一句话成形；codegraph 结论相同但读者要自己从一堆噪声里认 |
| **I2** 改 `tools.rs` 的 `DISPATCH` | **答不出来**：`search query=DISPATCH` 回的是 `fn dispatch -> protocol.rs:246`（常数不在索引里），`inspect tools.rs` 只列 9 个 fn，`DISPATCH` 不出现 | `DISPATCH (tools.rs:316)`，唯一使用点在**同文件 `tool_call`**（返回源码里能看见 `:374`）；另附两块无关源；打 **`⚠️ no covering tests found`** | **codegraph 完胜**：xirang 的索引维度只有 fn/文件/面，常数根本没有地址。这条也说明 xirang 的 `search` 不是"grep 替代品"。codegraph 那条测试告警同样是假阴性：`test mcp::tools::tools_tests::the_dispatch_table_follows_the_catalog ... ok`（DISPATCH 上方的注释就写着它） |

**第③节小结（供审计用，不下排名）**

1. **"谁调用它/影响面"**：两边都能答，xirang 给"定义文件:行 + 全部调用点（`文件::函数`，含测试）"，
   codegraph 给"文件级清单 + 计数 + 测试文件"；xirang 在**点名到函数**与**不截断**上更强，
   codegraph 在**常数/类型/非 fn 符号**上更强。
2. **"A 到 B 的链"**：两边**都不直接列路径**。codegraph 靠"一次给整文件"让人读出链，
   xirang 靠"直接 callee 集"给出第一跳、多问一次闭链。codegraph 的返回里混入无关文件的概率明显更高。
3. **截断**：两边都截断，但**语义不同**——xirang 说 `… +21 more`（就是被扣掉的调用点数，
   照着它自己能数出来），codegraph 转录里是 4 条 + `"+4 more"`，**这个 "more" 是调用点还是文件，从返回里读不出来**
   （41 个调用方时它只列 4 条）。对"影响面"这类题，两种截断都要当成风险。
4. **`⚠️ no covering tests found` 至少 3 处假阴性**（W1/W2/I2），其中 W2 与 I2 是**同一份返回内部自相矛盾**。
   这就是"一定稳"要解决的问题：可信度标记不能在"我能看见测试"的情况下说没有。
5. **同题同侧不一致**：xirang 自己也有更硬的一处——`verify`/CLI `check` 说 `verdict ok`，
   `usages`/`converge`/`apply` 同刻说 faces rejected（X1）。跨工具一致性两边都没到"可托付"的程度。

---

## ④ 可用性结论

**完善吗？分两层。**
对**注册面**这一层：完善。17/17 有实现、目录与分派一致、每个失败都点名"缺什么/下一步做什么"、
`unparsable faces N` 与各类计数不漏报；写入路径真的走了内核的准入/规则/连接器校验，不是自己重实现一遍。
对**符号**这一层：不完善。索引维度只有 fn、文件、注册面三种，
常数/静态量/类型/字段/声明内容一概搜不到（I2、M6）；`callgraph` 是名字匹配且只有 1 跳。

**可用吗？** 可用，且我没找到"整条路走不通"的工具。
- 只读面在真实宿主上 17 个工具全部给出可读答案，失败也都是可行动的（`mir`/`unified` 在仓库根报
  "produce a text dump with `cargo rustc -Zunpretty=mir`"，`trace` 报产出办法——因为仓库里没有这两类成品）；
- 写入面在沙箱副本里 add → usages 读回 → converge 判定 → rename → delete 全环跑通；
- MIR/trace/unified 在真实 rustc 产物与仓库自己的 trace writer 产物上都成立。

**但有三处会让 agent 信错的东西（按危险度）：**
1. **X1 相反结论**：先 `verify` 拿 `verdict ok`，之后每次 `apply/usages/converge` 都被拒。
   静态 pass（`check_for`）与运行时 connector（`register_snapshot_batch`）判定面不同，
   而 17 个工具把它们混在同一个"内核结论"语汇里。
2. **X2 自坏回环**：`mir jsonl: true` 在 >400 行时把截断说明写进 JSONL 正文，
   而描述与截断提示都写着"存成文件再读回来"——读了就报 `MirParseError { line: 401 }`。
3. **M5 报错的面**：`trace` 说某个面跑了，`converge trace: true` 说 0 个面跑了，同一份 artifact。

**缺什么？** 见⑤。另：`explain`/`diff`/`search` 的构建结论都依赖 `target/xirang/out`，
新检出里一律是 `build stale` / `no build evidence`；桥上唯一能产出它的是 `verify`
（`xirang check` 在 CLI 那边）。这不是错，但"第一次打开就全是 stale"会让人误判。

---

## ⑤ 缺口清单（现象 / 影响 / 值不值得做）

| # | 现象（证据） | 影响 | 值不值得做 |
|---|---|---|---|
| G1 | **同一棵树两个内核结论**：`verify`/`check` = ok，`apply`/`usages`/`converge` = faces rejected（X1 原文在②） | agent 先拿绿灯再处处被拒；"内核"这个词在这个包里有两套判定面 | **最值得**。修法可以是让 `verify` 也跑 connector（或让两边共用一份 verdict 词汇表）。直接对应"可信度链路"与"深度链稳定性" |
| G2 | **`mir jsonl` 回环自坏**：截断说明进 JSONL 正文，>400 行即 `MirParseError { line: 401 }` | 描述承诺的便携通道在最需要它的中大型 artifact 上不可用 | **很值得**。截断说明应写到 stderr 或另起非 JSON 行并让读方跳过；改动面小、可被测试钉住 |
| G3 | **`converge trace` 与 `trace` 对面级帧口径不同**（M5） | "这次运行碰了哪些面"答 0 个，而 call tree 里明明是面节点 | **值得**。两边统一到帧的 node 身份即可，属"diff/深度链"类 |
| G4 | **`converge` 的 `UNANSWERED` 字面不可达**（M4） | 描述与读者所见不符；头条判断的失败支只在"内核通过但需求没满足"时出现，实测不可达 | 值得，但优先级低于 G1——G1 修好前这一支本来就到不了 |
| G5 | **`apply.fields` 的形状全靠试错**（M1/M2/M3）：module 裸名、值必须字符串、`handle_traits` 是标签 | 写入路径是这 17 个工具里唯一会改仓库的，试错成本最高；`handle_traits` 那个错还要等父规则报 | **值得**。`tools/list` 的 `fields` 是空 object，把取值形状写进描述或 schema 就能省掉一轮 |
| G6 | **符号索引只有 fn/文件/面**（I2、M6；`DISPATCH`、`control.render` 都搜不到） | 想让它当"仓库内代码搜索"用会落空；与 codegraph 对照时这是最大的一类差距 | 值得，但要明确划线：加 const/静态量索引是能力扩张，不是修 bug。**别把它当第③节输赢的补救** |
| G7 | **`usages`/`converge` 的字段读回只在生成模块上成立**（真实示例 3 个面全 unreadable） | 仓库自己的唯一真实宿主享受不到这条最花哨的能力 | 值得记录，不值当为它改代码——手写模块确实没有字段清单 |
| G8 | **graft 计划匹配忽略 `target_path`**（X3：`target` 能解析时按 identity 的 module 匹配，`target_path` 只被打印） | 手工写坏的、自相矛盾的计划会被报成 `declared`，维护者照着它以为槽位保住了 | 低。写入方永远写一致的一对，只有手改才踩到 |
| G9 | **`impact depth: 0` 静默夹成 1**（schema 写 minimum 1，越界不报） | 轻微；agent 以为自己问过 `depth: 0` | 低 |

**给 t2（审计）的三条对齐说明**：本报告只提供证据与"值不值得做"，**不排优先级**。
G1/G2 是"稳"这一判据下最直的两条；G3/G4 属"diff 链/深度链"；G6 属"质量类 MCP"那一格但它是能力扩张；
第③节里 codegraph 的三处 `no covering tests` 假阴性（W1/W2/I2）与 xirang 自己的 X1，
合起来正是"可信度链路标本标记"要处理的对象。
