# XiRang 1.0 路线图（四轮审计汇总）
# XiRang 1.0 roadmap (four audit rounds)

本文是四轮审计的收敛结果与执行计划。结论按"能不能上 1.0"排序，不按发现顺序。
This is the consolidated plan from four audit rounds. Findings are ordered by
what blocks 1.0, not by when they were found.

## 审计轮次 / Audit rounds

| 轮次 | 形式 | 范围 | 主要产出 |
| --- | --- | --- | --- |
| 第一轮（大） | 人工 + 实测 | 命名、文件职责吻合度、实现链路（绕路/重合）、鲁棒性、投产判断 | 内核零 I/O 与零真实 unsafe；`missing_docs` 506 条；20 个公开 `Result<_, String>`；三份 `StdSourceTree` 适配器；`RegistryError` 访问器多于字段；`face_write` 两张顺序表偏重 |
| 第二轮（大） | 三方只读审计（graft / kernel / 工具与发布）+ 人工复核 | 逐行最小化判断 | ~90 条 `F<n>`，其中 3 条经手工复核；静态/owned 四对重复校验；`health_check` 零调用；宏 arm 吞参数；范围 `" to "` 四处拆合；Studio 假 trace；`package-audit` 包名错 |
| 第三轮（小） | crates.io 事实核查 | 发布门槛 | 无发布前审查；机器只卡 package 成功、license、版本可解析、包名、大小；版本不可变只能 yank；`xirang*` 名字全空；core 包不随带 LICENSE 文本 |
| 第四轮（大） | 新工具（crates.io 与上游 changelog 事实核查、codegraph 结构查询）+ 三个只读子审计（散文规则 vs 门禁 / 死代码与重复 / 文档与示例验证）+ 对进程后端的定向对抗阅读 | 依赖演化、进程后端、身份与信任基元、规则 vs 门禁、文档验证 | 进程后端"硬超时"曾经不硬（实测边界：帧 65 536 字节成功、65 537 字节超时）；`wasmi` 落后 18 个月两个大版本；`NodeId` 组合未钉而它落盘；手写 SHA-256 缺差分预言机；路线图声称的 `compile_fail` doctest 无任何 CI 命令编译；内核纯净性等四条规则无门禁；36 个文档 Rust 围栏零验证 |

## 进度快照 / Progress snapshot

已完成:B1、B2、B3a、B3b(外部面 20→1 必填)、B3c-1(范围端点改为字段)、B3c-2(计划记录接进 overlay)、B3d(对抗性审计五条)、B4 第 1–8 项与发布卫生、"下一批"第 1–11 条,以及第四轮的 C1–C6(离线部分,见下方第四轮节)。各批次的三件套 + doc 由我在树静止后统一复跑,绿。

**1.0 剩余**:首次发布**已完成**——`0.1.0`(2026-09-25)与 `0.1.1`(2026-09-26)九个 crate
都已上 crates.io,发布工作流的最后一步两次都在检出之外构建一次性消费者并按版本解析到全部九个。
包审计的**内容**半边对九个 crate 始终生效；**打包**半边只在每个带版本号的内部依赖都已进入 index
时才构建，因此工作区版本尚未发布时它报 `skipped: 8`——这个状态每次推进版本线都会重现，不是
"首次发布前"的一次性状态。版本线继续走 `0.1.x`(设计仍在深化),因此 1.0 剩下的不是
一次发布,而是路线图登记的工程里程碑(MIR 适配器、单态化/间接调用、公开面冻结)与下面这批欠账。
第四轮另登记两件未做之事:`wasmi` 升级(需要联网一次抓取,第四轮节说明为何没有擅自做),
以及新门禁所钉住的欠账(超出尺寸上限的文件、29 个未编译的文档片段、7 处死代码与 8 处必须
保持一致的重复;尺寸上限已于 2026-09-28 由 450 放宽到 600)。

- **B3d 修正了一处归因**:F1 的垃圾默认名不是审计指认的"完整 arm"产生,而是由*两个都显式写了的紧凑 arm* 消费 `$crate::NoPreset` 后经 `stringify!` 产生;修法是删掉那条重发 arm,让带 `handle` 的紧凑 arm 自己成为默认化 arm。三个示例面现在实测 `preset == "NoPreset"`、`parts == "NoParts"`。
- **F2 的责任在计划本身**:B2 要求"四处路径判断合成一个 `path_is_under`",但那四处对"相等"的语义并不一致(准入包含相等,连接器严格在其下),合并把连接器的准入静默放宽了。已加 `path_is_strictly_under` 并把两种含义写在一处。教训:合并重复前逐处比对边界语义,不能只看"看起来一样"。
- **F5 确立一条原则**:零调用者的删除,在真实调用者出现时反转。`RegistryError` 的 `node/path/source/message/children` 已补回,另三个保持删除,因为文档化的 `health_check` 宿主 API 就是调用者。
- **trace 接入定为 1.x**(`docs/design-trace-ingest.md`);1.0 只加一条 `LIVE SAMPLE` 标注的回归测试。设计还查明示例 trace 不只是假的,而是选不中的:它的节点与注册树不同域,`Live` 永不点亮;后来某一轮给样本补了一个被观测局部值(该轮起文中的"不记录 locals"不再成立),但那个值同样只对样本自己的节点可见,而注册树里没有那个节点。1.x 的第一片(trace artifact)已开工,见该设计的 §3 与 `toolchain/run_method/src/runtime/trace/snapshot/`。
- **B4 的"有效树 dump"有真实边界**:CLI 无法为任意宿主重建 `Registry`(基树与外部树只存在于宿主 crate 内)。已提供宿主侧端口 `Registry::dump_effective`,CLI 的 `explain --overlay` 明确标注为 `static-projection`。

**下一批已完成(尺寸回归 + 未接线能力 + 三条定夺项)**

1. ✅ 尺寸回归第一轮(当时的样本均 ≤450):`record.rs` 900 → 85 父 + `reconcile.rs` 192 / `reports.rs` 136 / `apply.rs` 263;`toolchain/cli/src/lib.rs` 944 → 243 + `commands/*`;`explain.rs` 598 → 197 + `explain_report/overlay/json.rs`;`face_view.rs` 561 → 435 + `scope_view.rs` 165。公开路径全部逐字兼容。
   ⚠️ **第四轮更正:这条当时写成"非测试文件均 ≤450"并不成立。** 实测有 15 个非测试文件超过它(最大 `kernel/src/registry_core/declaration/runtime_checks.rs` 751)。现在不再是口头约定:`conventions` crate 的 `size` 门禁把它变成**只能变短的棘轮**——把超标文件连实测行数一起钉在 `BASELINE` 里,新增超标文件会失败,已缩回上限之内的过期项也会失败(防止清单悄悄变成许可)。清单从 15 项开始,第四轮的第 8 条把 `diagnostic/build.rs` 缩回 429 行后,棘轮**强制**删掉了那一项。条目数不在散文里复述——它就是 `BASELINE.len()`,而 `size.rs` 的模块文档已写明复述数字的散文终会与它漂移(本条此前复述的那个数字正是一例,已删)。
2. ✅ Feature 2 落地:`health_check` 的宿主 API 文档(含 `no_run` 示例,用补回的 `node()/path()/source()/message()/children()`)、五个检查单测、`examples/control-button` 的 `health_check` example + 端到端测试、`toolchain/run_method/README*.md` 的宿主调用段。
3. ✅ trace 1.0 回归测试:`trace_legend_reads_live_sample_and_never_claims_live_data`(断言图例为 `LIVE SAMPLE`,剥掉后不残留裸 `LIVE`),注释指向 `docs/design-trace-ingest.md`。
4. ✅ 记录路径有了真实宿主:`examples/control-button/examples/graft_record.rs`,把包根重定向到带 `Drop` 清理的临时目录,同时证明类型化声明 `TypedDeclarationKept` 与字符串声明 `DeclarationOverridden`,且基树与 `builtin_static_plan()` 不变。
5. ✅ **A1:`XIRANG_ENTRY` 只解析一次、两个读取者共用**(你选严格档)。构建侧新增 `entry::HostEntry`(`Configured`/`Declared`/`Convention`)与 `resolve_host_entry`,由 `host_entry_from_environment` 每构建读环境一次;`pipeline` 把同一个值交给 `SourceScope` 与 `host_graft_entries(&entry)`,后者不再自行解析入口。被指定却不是文件的入口现在**构建失败**(`XIRANG_ENTRY names …`,与 `host_entry_source` 给创作面的 `Err` 同义),而不是让一半回退到 `main.rs`;未设置时行为逐字节不变。测试钉住:配置入口同时决定剪枝与切口表、相对/绝对路径都相对包根、缺文件即失败、约定入口缺失仍只是"没有入口"。
6. ✅ **B:兄弟区间的顺序契约写成契约。** 跨度按 `registry_name` 排序(`resolution.rs`),现在这件事既写在代码注释里,也写进 `docs/graft.md`/`graft.zh-CN.md` 的新节 "Ranges over siblings" 与两个根 README。写反的端点(`mid to alpha`)**报错**而不是静默交换,错误同时给出两个端点、规则和正确的写法;两条测试钉住(`a_range_covers_the_siblings_between_its_endpoints_in_registry_name_order` 用注册序与名字序不同的四兄弟夹具,`a_backwards_range_is_refused_with_both_endpoints_and_the_order_rule`)。`GraftError::InvalidRange` 的载荷改为完整原因句、`Display` 原样打印(公开类型形状不变,跨父级那条消息逐字节不变)。
7. ✅ **C19:诊断字段表。** `toolchain/cli/README*.md` 记录 `check --json` 的文档契约(十一键恒在、`line=0` 表示无行号、JSON 顺序 == 人类可读顺序、退出码仍非零)与逐 phase 的字段表;`toolchain/run_method/README*.md` 记录五个读取器,以及"顶层 `message()` 是固定聚合句、失败检查名在 `children()[0].message()`、`Display` 是推荐渲染"这条规则。
8. ✅ **C24:Studio 对未声明槽位仍然写计划,但改成醒目警告。** 写入顺序合法("先写记录、后声明槽位"),因此不拒绝;但横幅以 `Warning:` 开头(状态栏据此上警示色),说明发布态会剪掉该槽位、运行期会把这条记录当作 `UnkeptSlot` 跳过(即嫁接不生效且不报错)、缺声明的是哪个入口文件,并附上按同一个内核构造器渲染出的可直接粘贴子句。测试 `graft_warns_about_an_undeclared_slot_after_writing_the_plan` 钉住"计划已写入 + 警告内容 + 子句与记录逐字相同"。

9. ✅ **运行期不再有沉默的嫁接。** `apply_recorded_grafts` 现在在返回前把每条 `RecordReport` 与每个不可读计划打印到 stderr(每项一行:未生效/损坏为 `warning:`,优先级裁决为 `note:`),同样的条目仍留在 `GraftOverlay`。原因是设计上"跳过不是致命错误"(`Registry::resolve_record`:一条陈旧记录不该让其余记录失效)本身没错,但**把"不失败"和"不告知"混为一谈**了——只取 `.effective` 的宿主什么都看不到,而那是一次悄无声息从未发生的嫁接(源码树正常、运行中的二进制没应用它)。渲染器是纯函数,由 `graft_report_lines_mark_problems_as_warnings` 钉住;示例 `graft_record` 新增第三种情形做端到端演示,实测 stderr 输出 `warning: graft record \`slider_fast\` addresses \`root/control/button\`, which no declaration keeps alive; record skipped`。构建期那半边本来就有 `cargo:warning`(`graft_plan_check`),Studio 那半边是第 8 条。

10. ✅ **`--all-features` 那条 CI 真的绿了,而且 `prototype-fixtures` 不再空转。** 本会话新增的 `features` job 从未被执行过,一跑就红:`studio` 的 `prototype-fixtures`(HEAD 里就有的特性)门控着 10 条测试,它们断言一个当时并不存在于仓库的 `node_editor` 原型宿主包,因此从来没有编译过、更没有通过过。按"这是设计需要就深化完成"的决定:签入仅源码的夹具宿主包 `toolchain/studio/tests/fixtures/node-editor/`(自带 `[workspace]` 表,不是工作区成员、不参与编译、无 `target/`),10 条测试全部接上它(`select_project` + `env!("CARGO_MANIFEST_DIR")`);其中两条断言**实测**证据的测试改为安装自己的追踪(`toolchain/studio/src/studio/app/tests/fixtures.rs::fixture_live_trace`,只在该特性下挂载),而不是依赖 Studio 内置演示样本 `sample_live_trace`——那是生产演示数据,不该被测试反向绑架。**断言一条都没有删改或弱化**。实测:`cargo test --workspace --all-features --all-targets` 347 passed / 0 failed(此前是编译失败),默认特性 341 passed,`cargo check --workspace --no-default-features --all-targets` 亦绿。AGENTS.md 已记下:这条特性只由那条 CI 任务覆盖,必须常绿。

11. ✅ **S1+:失败放在信息最全的那一层(你选的路)。** 运行期:`apply_recorded_grafts` 现在对**没有任何合法解读**的两类直接 `Err`——解析不了的计划(一次报出全部不可读计划,且不应用任何记录)、目录选择器与计划里的 `graft` 不一致的记录;内核 `Registry::resolve_record` 相应地拒绝后者,公开枚举里那个从此不可达的 `RecordReport::SelectorDirectoryMismatch` 变体已删除(0.1.0 未发布,公开面未冻结)。构建期:计划的目标槽位**没有任何声明(含门控关掉的声明)可能命名**时,从 `cargo:warning` 升级为**构建错误**,消息里带可直接粘贴的 `static_graft_plan!` 子句,`phase=static-plan` 并指向计划文件;顺手修掉那条文档/代码不一致(`graft_plan_check` 文档说门控声明算数,而 `host_graft_entries` 把门控关掉的声明过滤掉后才交给校验器——现在 `HostGraftEntries { declared, enabled }` 把两个视图分开:校验器读 `declared`,切口表读 `enabled`)。保留为打印警告的只有运行期无法区分成因的两类:`UnkeptSlot`(声明可能被本次 feature 组合编译掉,记录也可能写在声明那次构建之前)与 `RecordSelectorUnresolved`(外部实现可能随 feature/插件启停);四条优先级裁决仍是 `note`。实测:未声明计划 → `check` 退出 1 并打印子句;门控关掉的同名声明 → `check` 通过(feature 开关两种取值都通过)。测试改动:`a_directory_that_disagrees_with_the_document_is_refused`、`an_unparseable_plan_fails_the_apply`、`a_directory_that_disagrees_with_its_plan_is_refused`、`an_undeclared_plan_target_is_an_error_with_the_clause`、`a_gated_declaration_still_counts`。

**仍待你定**

- **首次发布**:需要你的 crates.io token 与授权(我不能代按)。驱动脚本
  `tools/xirang-publish` 已就绪并实测(dry-run 完整校验 `xirang-kernel`、其余八个报"等待中",
  退出 0),发布动作留给你一行命令。
- **`missing_docs` 递增白名单**(B4-7):1.0 最后一块能靠代码本身完成的质量门,正按模块推进,
  进度见下。

**missing_docs 进度**

- 已完成,九个 crate 全覆盖(B4-7)。见该条。

**已记录未改 / Recorded, not changed**

- 五字段诊断只对连接器/准入失败成立,身份相位的 `node`/`function` 为空:这是字段表的真实边界,已写进 `toolchain/cli/README*.md`,不改代码。
- Studio 的 `submit_graft` 不再把"未声明"藏进普通成功消息(见第 8 条);仍不阻止写入,因为拒绝会禁掉"先写计划后补声明"这条合法路径。

## 已定决策 / Settled decisions

1. **包形状:九个 crate 保持不动,九个全部发布。** 合并方案否决,理由记录在案:user
   只 `cargo add` 一个库、`cargo install` 一个 bin 包,因此发布面本可收窄;但选择保留
   编译隔离(每个 crate 单独成包、单独带自己的 README/LICENSE),改为把发布卫生做全(见 B4)。
   发布顺序以 `tools/xirang-publish` 的 `levels` 表为准(`kernel` → `macro`/`build_method`
   → `run_method` → `debug_method`/`plugin-host` → `mcp` → `studio` → `cli`;`--check-table`
   在每次 push 与每次发布前把那张表与清单核对),本文不另立一份顺序。依赖版本下界**随发布线
   一起抬**(写 `^<工作区版本>`):`release_version` 与 `--check-table` 两道门禁都强制它等于
   工作区版本,所以"只把一个 crate 发到新版本、其余不动"在清单层就被判红——发布是一条线、
   不是九个,只有整条线一起走才发得出去。
2. **运行期校验:接通。** 把 `Registry::health_check` 文档化为宿主 API(在值的边界调用,
   `call_path` 未接 trace 时传空),补五个检查的测试与一个 example。`runtime_checks`
   字段因此变诚实,约 200 行从死代码变成有文档的能力。
3. **`graft.plan`:真接进 `overlay`。** 给 overlay 一个消费计划文件的入口,并补一条
   record→application 的测试。仍需确定:计划与 `static_graft_plan!` 同时命名同一槽位且
   graft 名不同时谁优先——建议记录优先并告警,待实现时确认。
4. **零调用者公开项:发布前全删**(逐项先确认不是留给宿主的 API)。
5. **B1 存疑(kind-only 的 `registry_name` 等价性)推到 API 收口批。**
6. **CI 包检查**:脚本对"依赖尚未发布"的包 warn 并跳过、整体 exit 0,去掉
   `continue-on-error`。于是这一步会抓 core 的打包回归,并在其余八个上线后自动覆盖它们
   (两者均已生效:九个 crate 的内容半边始终生效,打包半边对每个内部依赖已上 index 的 crate
   生效)。

## 批次 / Batches

每批独立完成、独立跑门禁（`fmt` / `test --workspace` / `clippy -D warnings` /
`doc -D warnings`），独立汇报。不并行跨批。

### B0 待你拍板的三个决定

1. `Registry::health_check` 与它后面约 200 行运行期校验：接一个调用者，还是删除？
   （删是公开 API 变更，会动根部白名单里的 `RuntimeValue`/`Coordinates`/`Provenance`。）
2. `graft.plan` 是否接入 `Registry::overlay`？不接就删 `GraftPlanDocument::cut()/plan()`，
   并在文档里把"记录"与"应用"的边界写死。
3. 零调用者的公开项（`RegistryIndex` 查询、`RegistryError` 读访问器、`StaticPlan::new`、
   `CutTarget::path/id`、`accepts_with_catalog`、`requires_isolation`、
   `ProductionPolicyNotStrict`、三个 artifact verify 包装）：清掉，还是保留并
   `#[doc(hidden)]` 标注？

### B1 修真实的错（不改公开 API）

| # | 项 | 证据 | 验收 |
| --- | --- | --- | --- |
| 1 | ✅ `face_objects` arm 绑定 `$preset`/`$parts` 却展开成 `NoPreset`/`NoParts`（已复核机制，已修） | `toolchain/src/run_method/macros/face_objects.rs:92-95` | 已落地：`__face_ty_or!`/`__face_ty_name_or!` 按每个绑定分别判有无，省略者保持默认、写下的原样转发；钉子 `toolchain/run_method/tests/face_preset_parts.rs`、`toolchain/run_method/tests/face_arm_defaults.rs` |
| 2 | ✅ arm 5 不可达（已修：该 arm 已删除） | `toolchain/src/run_method/macros/face_objects.rs`（无行号：被删的 arm 不再有位置） | 删除后既有宏测试全绿 |
| 3 | `register_snapshot_batch` 尾部缺父循环不可达，且重复构造同一错误 | `declaration/../../tree/transaction.rs:58-71` | 删除；transaction 全部测试绿 |
| 4 | `UnknownReplacement`/`UnknownTarget` 用原树根 id 报错，丢掉选择器 | `tree/graft_ops/overlay.rs:151-156`、`resolution.rs:43-51` | 错误携带选择器字符串；新增测试断错文案含 selector |
| 5 | `same_symbol` 每次比较两次堆分配且在双重循环内 | `mir/merge.rs:53-55` | 改 `strip_suffix` 零分配；既有 merge 测试绿 |
| 6 | Studio 帮助文字三处错 + 折叠是死代码 | `ui/status.rs:27`、`ui/search.rs:47,137`、`overlay/search.rs:65,282,380`、`app/search_queries.rs:10-16` | 帮助文字与实际键位一致；死折叠的键位与标记删除（不新增行为） |
| 7 | Studio "live trace" 是硬编码样例，README 却声称权威 | `app/lifecycle.rs:46`、`app/sample.rs`（硬编码样例所在文件，已在本项收口时删除，职责移到 `app/trace.rs`）、`README.md:714` | 面板与图例标注为示例，README 同步；真实 ingest 留到 B4 |
| 8 | `tools/xirang-package-audit` 引用不存在的包且不在 CI | `tools/xirang-package-audit:8` | 改 `xirang-toolchain`，接入 CI；脚本能跑通（core 已发前仍会因版本依赖失败，脚本里注明） |
| 9 | `README.zh-CN.md` 把 `StaticPlan::find`（现名 `find_face`，旧名保留为一行转发器）写成 O(log n)，实际线性 | `README.zh-CN.md:569` vs `release.rs:203-217` | 更正为 O(n)，与英文 README 一致 |
| 10 | MCP 宣称能查合同，5 个工具只读源码 | `toolchain/Cargo.toml:7`、`toolchain/README.md:13`、`toolchain/src/mcp/tools.rs:10-38` | 已按"补工具"收口（2026-09-26）：`xirang.registry` 报告构建推导出的注册树（`face_views`）。contract/admission 数据仍缺——它们在已构建的 `RegistrationSnapshot` 里，不在源码里；宣称已按此收紧 |

### B1 完成情况（已收口，附两条新增存疑）

10 项全部落地，全量门禁（fmt / workspace test / `--features authoring` / clippy / doc）绿。
两处修正值得记录：

- **"arm 5 不可达"的判断只在语法层面对**。删除它之后，kind-only 输入改走完整 arm，
  于是编译期多发射一条 `cfg(rust_analyzer)` 镜像，`debug_method` 的测试目标因此
  `unexpected_cfgs` 报错（per-crate 门禁没覆盖到）。已在 `toolchain/call_evidence/Cargo.toml`
  补 `[lints.rust] check-cfg`，与 `run_method` 同样的声明。**新增存疑**：kind-only
  输入的 `registry_name` 在删 arm 前后是否逐字相同，需要一个等价性测试来钉（当前
  没有能直接对比的基线，因为宏文件是 P4 新建的未跟踪文件）。
- **`graft_plan_check` 的端到端测试在全量并行跑时撞过一次临时目录名**（同一纳秒）。
  已给 `build_method` 的测试临时目录加进程内 `AtomicU64` 序号（`graft_plan_check.rs`
  与 `pipeline.rs` 四处），与早先 macOS 上的同类修复一致。
- CI 里 `tools/xirang-package-audit` 曾用 `continue-on-error: true`，因为 core 发布前
  它必然在 `xirang-toolchain` 那步失败。**该开关已去掉**（2026-09-25 首次发布之后）：
  这一步现在与 `cargo package -p xirang-kernel` 并列，打包回归会让它失败，而不再被容忍掩盖。

### B2 单一事实来源（去重复，改一份就够）

1. `RegistrationRule::validate` 与 `OwnedRegistrationRule::validate`（`declaration/registration.rs`
   的 `impl RegistrationRule` vs `declaration/owned.rs` 的 `impl OwnedRegistrationRule`）→ 一份基于
   切片的核，两边适配。
2. `ObjectContract::validate` 与 owned 版（`contract.rs:85-105` vs `owned.rs:145-165`）→ 同上。
3. `FlowContract` 与 `OwnedFlowContract` 的字段比较与语义判定（`plugin/contracts/contracts.rs:84-101`
   vs `:159-186`）→ 一份；删零调用的 `OwnedFlowContract::matches`。
4. "路径等于或位于前缀之下"四处（`registration.rs:46-62`、`owned.rs:70-84`、`connector.rs:159,265`、
   `lexicon.rs:90`）→ 一个 `pub(crate) fn path_is_under`。
5. `render_flow_expression`/`parse_flow_value` 与 `render_admission`/`parse_admission_owned`（现名 `parse_admission_snapshot`，旧名保留为一行转发器）
   各写两遍（`authoring/parse/flow.rs`、`admission.rs`）→ 解析一份，渲染消费它。
6. `normalize_kind_name` 与 `pascal_case` 同体（`authoring/validation.rs:31-45` vs
   `field_presentation.rs:317-329`）→ 后者是前者的回退分支。

验收：每项删掉一份后，既有行为测试不变；每项新增一条"两边结果相同"的对照测试。

### B2 完成情况（已收口）

各项全部落地,core 测试 94 → 106,全量门禁绿。下表逐项列出收口结果——四对静态/Owned 校验各只剩一份核,
末行是本批把执行面副本也收回内核后的注册规范渲染:

| 项 | 唯一实现 | 对照测试 |
| --- | --- | --- |
| 注册规则五项校验 | `declaration/declaration.rs::validate_registration_requirements`（泛型切片核,`&[&str]` 与 `&[String]` 零分配适配） | `registration_rule_twins_report_identical_failures` |
| 对象合同校验 | `validate_object_contract` | `object_contract_twins_report_identical_failures` |
| 流合同比较与语义判定 | `FlowFields` + `flow_is_declared` / `flow_fields_equal` / `flow_fields_semantically_compatible` | `static_and_owned_flow_contracts_compare_identically`（6×6 矩阵） |
| 路径前缀判断 | `lexicon::path_is_under`（`is_registration_path` 改为经它表达,原有"模块名本身不算在其下"语义保留） | `admission_twins_accept_and_reject_identical_paths`、`the_shared_prefix_check_owns_equality_and_the_directory_boundary` |
| flow / admission 解析 | `parse_compact_flow`、`parse_compact_admission`（渲染方消费解析结果） | `flow_render_and_parse_read_one_parser`、`admission_render_and_parse_read_one_parser` |
| kind 归一化 | `normalize_kind_name` 改为 validate-else-`pascal_case` | `normalizing_a_kind_uses_the_shared_pascal_case_fallback` |
| 注册规范渲染 | `authoring/parse/rules.rs::compact_registration_rule`（内核唯一的紧凑拼法渲染器；`rule_syntax_from_text` 与 Studio 的 `registration_rule_text` 都经它渲染） | `registration_rule_entry.rs`（crate 之外）、`the_call_site_renders_what_the_kernel_renders`、`the_call_site_carries_no_second_rule_renderer` |

- 零调用者 `OwnedFlowContract::matches` 已删（全仓 grep 无引用），私有 `path_matches` 已删，连接器里
  `format!("{owner_path}/")` 的每次比较分配已消。
- **一处有意的边界放宽**：`render_admission("ALLOW : a")` 现在接受冒号前空格，与快照解析器一致
  （为了让两条路共用一个解析器）。只接受解析器本来就接受的值，有测试覆盖。
- 未改动任何公开签名；`pub(crate)` 辅助各带双语块注释说明"为什么直写会静默分叉"。

### B4 完成情况（第三轮：新增公开面与版本线）

第三轮的记账（`docs/audit-2026-09-28/audit-report.md` §14）把本批新增的公开面逐条核实：四个入口在补记前
对 `CHANGELOG.md` 与本文件的 grep 命中**全为 0**，因此这次是补缺，不是重复记。

| 入口 | 谁提供 | 谁消费 | 钉在哪 |
| --- | --- | --- | --- |
| `authoring::parse::try_render_requirements` | 内核 `authoring/parse/rules.rs`（严格，`-> Result<String, FaceParseError>`） | `run_method` 的 manifest 渲染器（旧 `render_requirements` 保留为有损薄包装） | `a_malformed_requires_entry_refuses_the_rewrite`（走真实 `render_source`） |
| `build_method::face_views_and_unreadable` | `toolchain/build_method/src/face_view.rs` | 面对外回答（`face_views` 委托它），不可读的面从此可查 | `a_registration_file_that_does_not_parse_is_named_not_dropped` |
| `source::item_symbols` / `SourceItem` | 内核 `source/items.rs`（`INTRODUCERS` 一张表） | Studio 的搜索视图（不再自持词表：`strip_prefix("` 0 处） | `kernel/tests/b4_item_symbols.rs`（5）+ Studio `source_rows_tests`（4） |
| `authoring::parse::try_parse_requirements_owned` | 内核 `authoring/parse/rules.rs`（严格入口） | 需要"拒绝"而非"丢弃"的调用方；已发布的 `parse_requirements_snapshot`（旧名 `parse_requirements_owned` 保留为一行转发器）签名与行为逐字保住 | `the_strict_requires_entry_refuses_a_malformed_entry` + `a_malformed_requires_entry_is_refused_not_dropped` |

- **版本线**：四个入口都随未发布的 `0.1.6` 走——工作区版本不变、无需抬版本；`tools/xirang-publish
  --check-table` 只按清单比较依赖表，不受影响。
- **隔离打包**：`tools/xirang-package-audit` 的**内容**半边已含新模块（实测 `cargo package -p
  xirang-kernel --list --offline` 列出 `src/registry_core/source/items.rs`）；**打包**半边要等带版本号的
  `xirang-*` 依赖进入 index，未发布期间 warn + 跳过（**skip ≠ 失败**）——这批把等待态加深了一层
  （Studio 已调用未发布的内核符号），发布顺序仍是 core 在前，发布后重跑 `--verify-consumers`。
- **不动的**：本批没有删改任何已发布签名，四个入口都是加法式新增；`render_requirements` 之所以留着，
  是因为它是已发布线的一部分，而它的文档现在写明有损并指向严格兄弟。

### 末轮追加：`try_rule_syntax_from_text`（K-10 的严格读法）

末轮记账（`docs/audit-2026-09-28/audit-report.md` §15）把第四轮补记的公开面逐条核实：这一个入口在补记前
对 `CHANGELOG.md` 与本文件的 grep 命中都是 **0**（t52 已记的另三个候选本轮命中非 0，故不重复记）。

| 入口 | 谁提供 | 谁消费 | 钉在哪 |
| --- | --- | --- | --- |
| `authoring::parse::try_rule_syntax_from_text` | 内核 `authoring/parse/rules.rs`（严格，`-> Result<String, FaceParseError>`） | `run_method` 的 `rule_syntax_for_source`（唯一生产调用点；规则文件缺失仍 `Ok("ANY")`） | 生产入口探针 + crate 外钉子；独立复核 `audit-verify-k10-family.md` 三组变异 |

- **版本线**：加法式新增，随未发布 `0.1.6` 走，`--check-table` 不受影响；已发布的 `rule_syntax_from_text`
  签名与宽容行为**一字未动**（文档写明它是有损的并指向严格兄弟）。
- **含义**：这条修复把"注释里的伪子句"与"常量点名的清单"从**静默降级**变成**报错并点名**——两者过去都会让
  作者声明的结构要求无声消失，而构建不会说一个字。

### B3 API 收口（必须在首次发布前完成，原因见第三轮：版本不可变）

#### B3a 完成情况（公开面收窄,已收口）

- 零调用者公开项已删:`RegistryIndex` 五个查询(`len` 保留,并加了一条有理由的
  `#[allow(clippy::len_without_is_empty)]`)、`StaticPlan::new`、`CutTarget::path`、
  `accepts_with_catalog`、`requires_isolation`、`ProductionPolicyNotStrict`、三个 artifact
  verify 包装、`Registry::get`、五个 metadata 读方法、`tree_outline`、`debug_method` 的
  `collect!` 与重复的 `pub use inventory`。
- **B3a 反向的一项**:`RegistryError` 的八个读访问器当时随"零调用者"一并删除(字段一个
  没删,`render_at` 仍读它们;`*_mut` 与 `with_children` 保留),但字段私有,于是宿主拿到
  错误后无法映射回 `node`/`path`/`source`。既然 `Registry::health_check` 已被定为有文档的
  宿主 API,这次删除就有了真实调用者,故重新加回最小读取面 `node()`、`path()`、
  `source()`、`message()`、`children()` 五项;`source_chain()`、`call_path()`、
  `registration_chain()` 仍保持删除。原则:**零调用者删除在真实调用者出现时撤销**,
  "当时没人用"不是永久移除的充分理由。由
  `kernel/src/registry_core/diagnostic/error.rs` 的
  `an_error_still_exposes_the_facts_a_host_must_report` 钉住。
- `edit_module` **删除**而非委托:委托要用磁盘上的面反推出 28 字段 `ModuleFacePatch`,会静默
  窄化字符串形式支持的任意字段名并重算字段,是行为变化。用 `compile_fail,E0433` doctest 钉住
  "它不再存在"。**该钉子由 CI `features` 任务的 `cargo test --workspace --all-features --doc`
  编译**：`--all-targets` 会跳过 doctest，而这条 fence 门控在非默认的 `authoring` 特性之后，
  所以在第四轮审计发现之前，没有任何命令编译过它（已补上那一步）。
- `registry_rule_path` 已从 `NewModuleFace`/`ModuleFacePatch`/`ModuleFaceValues`/applier 与
  Studio 的两行 patch 里移除;Studio 第 18 个字段本来就是只读(locked),没有新增写入路径。
- `path_compat` 扩到 8 个测试,成为 1.0 的路径承诺清单。
- **过程教训**:`CutTarget::id` 的删除被拦下了。全仓 grep 找不到调用者,因为调用点只写
  `cut.cut().id()`,从不出现类型名;是 `cargo check --workspace --all-targets` 抓到的。
  以后"零调用者"的判断必须以编译全 target 为准,grep 只作初筛。

#### 已确认的语义变化(由 B1 删 arm 引入,已用测试钉住现状)

kind-only 注册面(写了 `collector` 与 `kind`、没写 `handle`)的 `registry_name` 现在是
**模块名派生**(`kind_only`),而被 B1 删掉的那个 arm 用的是 `stringify!($kind)`
(`KindOnlyFace`)。也就是说我在 B1 里"那个 arm 不可达"的判断是**错的**:它可达,而且产出不同。
`toolchain/run_method/tests/kind_only_registry_name.rs` 现在钉住的是模块名派生这一支。

**已定:保留现状(模块名派生)。** 理由:存活下来的完整 arm 与宏前端的默认值都是
`last_path_segment(module_path!())`,内核词表文档写的也是这个;注册路径的其他段本来就是小写
模块名(`control/object/button`),`KindOnlyFace` 那种大写段反而是异类。依赖旧行为的宿主应显式写
`registry_name`。

**NodeId 不随之改变(已核到代码,不再是存疑)。** 声明宏发射的常量是
`NodeId::from_namespaced_path(env!("CARGO_PKG_NAME"), $source, stringify!($kind))`
(`toolchain/src/run_method/macros/face_registration.rs:45-49`),第三个参数是 **kind**,而
`from_namespaced_path` 只把 namespace + 相对路径 + 该名字卷进哈希
(`kernel/src/registry_core/identity/node_id.rs:47-58`);构建侧同理用
`package_node_id(&relative, &kind)`。因此这次变化只影响**逻辑路径**(它由槽位/注册名拼出)、
由此而来的排序、剪枝清单行与 Studio 显示,不影响任何身份。CHANGELOG 的 Unreleased 段已记录
该变化。


1. `__external_object!` 从 20 个必填降到只剩 `kind` 必填，16 个字段按 B0 决策走默认；
   `registry_rule` 默认 `ANY`（外部 crate 没有生成规则模块）。
2. 删除/隐藏 B0 决定的零调用者公开项。
3. `registry_rule_path` 从 `NewModuleFace`/`ModuleFacePatch`/`ModuleFaceValues` 移除（当前被
   编辑后丢弃）。
4. 范围切口：`end` 作为字段一路传递，删掉 `"a to b"` 字符串编码在四处的拆合。
5. 合并 `edit_module` 到 `edit_module_face`（前者不写规则文件）。
6. `path_compat` 测试冻结为 1.0 路径承诺清单。

验收：`examples/control-button-graft` 的外部面缩到 1（或 2）个必填字段仍能编译并 overlay；
`cargo test --workspace` 与两个 example 全绿；公开路径清单只增不减（除你批准的删除）。

### B4 可运维与发布卫生

1. `BuildDiagnostics` 加 `iter()/len()` 与 JSON 输出；`xirang check --json`。
2. `xirang explain <NodeId|path>`；`xirang grafts [--json]`；有效树 dump 出口。
3. 真实 trace ingest（文件或 `XIRANG_*` 变量）或永久标注为示例。
4. ✅ `cargo-deny`（advisories/licenses/bans）+ `deny.toml`；CI 加 `--all-features` 与
   `--no-default-features` 两个 job。那 10 条 `prototype-fixtures` 测试原先是死的（连编译都
   没过），现在有夹具、全绿——见"下一批"第 10 条。
5. 九个 manifest：`repository`/`homepage`（全部缺失）、`[package.metadata.docs.rs]
   all-features = true`（`kernel` 的 `syntax`、`run_method` 的 `authoring`、`plugin-host`
   的 `process-tools` 现在根本不会上 docs.rs）；每个 crate 目录放一份 LICENSE（根的
   LICENSE 不会被复制进包，cargo 只自动包含 crate 目录下的 `LICENSE*`）；每个 crate 一份
   README（`macro` 缺）；补 `macro`/`run_method` 的 `documentation`；加一份根 CHANGELOG，
   不按 crate 分九份。
6. **发布顺序与自动化**：新增 `tools/xirang-publish`——按 `levels` 表**分层**发布
   （core → macro/build_method → run_method → debug_method/plugin-host → mcp → studio → cli；
   该表是唯一一份顺序，`--check-table` 会把它与清单核对），
   每层发完轮询 index 直到该版本可见再进下一层（这一步是人最容易忘的）。默认是 **dry-run**，
   只有 `--publish --yes` 才真的上传（版本不可变，只能 yank）；真实发布前拒绝脏工作区，除非
   显式 `--allow-dirty`。dry-run 对"依赖版本尚未上 index"的 crate 跳过并警告，与
   `tools/xirang-package-audit` 行为一致，因此无需等首次发布即可用：当时实测完整校验了
   `xirang-kernel` 的打包、其余八个报"等待中"，退出 0。**选透明脚本而不是 release-plz /
   `cargo workspaces publish`**：那个配置我无法在离线环境里跑起来验证，交一份没验证过的配置
   正是这个仓库拒绝的东西；脚本的顺序与轮询逻辑可以在本地实测（已实测）。
   **仍然只剩你需要做的一步**：打 tag 并给发布工作流 `CARGO_REGISTRY_TOKEN` secret
   ——发布需要你的 token 与授权，我不代按、也不做 git 写操作。收尾那一步现在是
   `tools/xirang-publish --verify-consumers`（本轮新增）：它在本检出之外建一个一次性 crate
   按版本 `cargo add` 九个 crate，因此验证的是 index 上的解析而不是本地路径。`--check-table`
   在同一批里修掉了两张表的既存缺陷：`deps_of` 原先只返回多依赖 crate 的第一个依赖
   （`xirang-toolchain` 因此绕过"依赖未上 index 就不许发布"的守卫），并且边行的被依赖者被当成
   独立 crate；现在按整行读取，并由 `--check-table` 在每次 push 与每次发布前核对
   （`xirang-toolchain` 缺的 `xirang-kernel` 这条真实漂移已修正）。发布工作流见
   `.github/workflows/release.yml`；`docs/performance-baseline.md` 记录 R4 的性能基线与预算。
7. ✅ **`#![warn(missing_docs)]` 已在九个 crate 上全部开启**（比原计划的 `lexicon`/`identity`/
   `declaration`/`graft` 递增白名单更进一步：债一次还清，lint 覆盖整个 crate，因此不会再
   长回来）。总计补齐 **约 700 条**公开项的双语文档：core 510（38 个文件，含 44 条只在
   `--all-features` 下存在的 `syntax`/`authoring::parse` 项）、run_method 140、studio 129、
   plugin-host 51、build_method 13、debug_method 8、cli 1、macro 1、mcp 1。studio 的模块树是
   私有的，lint 实际只覆盖重导出的 `launch`，但 129 条内部项也一并补了。每个 crate 的
   `#![warn(missing_docs)]` 带同一段双语注释说明为什么是整 crate 而不是白名单。
   **顺手修掉两个下游问题**（都是实测发现的）：
   (a) `__registration_face!` 发射的 `NODE_ID`/`REGISTRATION` 之前没有文档——控制面藏在构建
   生成的 `#[doc(hidden)]` 模块里所以看不见，而**项目外的面由作者自己挂载**，于是开了 lint
   的外部 crate 会收到警告。现在这两个常量带双语 `#[doc]`（它们是作者的公开 API，不该用
   `#[doc(hidden)]` 掩盖）。
   (b) 构建生成的 `generated_lib.rs` 里 `pub mod <容器> {`、`builtin_static_plan()`、
   `registrations()` 没有文档，宿主开了 lint 会收到**无法自行修**的警告（文件在 OUT_DIR）。
   现在容器模块与两个函数都有双语文档，裸的 `BUILTIN_STATIC_PLAN` 与 FACES/CUTS 一样标
   `#[doc(hidden)]`。实测：`RUSTFLAGS="-W missing_docs" cargo check -p xirang-example-control-button`
   的警告只剩示例自身的 `pub` 项（它没开 lint），生成文件零警告。
   实测门禁：`RUSTFLAGS=-W missing_docs cargo check --workspace --all-features --all-targets`
   对九个 crate 的 `src/` 零警告；`cargo clippy --workspace --all-targets -D warnings` 在
   default / `--all-features` / `--no-default-features` 三种配置下都干净。
8. ✅ `xirang-dev` 不再以坏掉的状态发布。Cargo 不支持按 target 的 `publish = false`，
   因此用 `required-features = ["dev-supervisor"]`（新特性，默认关闭）把它挡在安装面之外；
   同时子 Studio 二进制改为按 **同级目录 → `PATH` → 工作区 `target/debug`** 顺序解析，且
   当它被编译时所在的检出已消失时，报错直接给出路径与修法（而不是让 Cargo 报"找不到清单"）。
   实测：`cargo install --path studio --root /tmp/… ` 只装出 `xirang-toolchain`（此前两个都
   装，`xirang-dev` 必然坏）；三条解析顺序各有单测（`the_sibling_binary_wins_over_path_and_the_workspace`、
   `path_is_searched_when_there_is_no_sibling`、`the_workspace_target_is_the_last_resort`）。

## 第四轮审计 / Round four

形式：新工具（crates.io 与上游 changelog 事实核查、codegraph 结构查询）+ 三个只读子审计
（散文规则 vs 门禁、死代码与重复、文档与示例验证）+ 对 `plugin-host` 进程后端的定向对抗
阅读。结论按"是不是真实缺陷"排序。
Form: new tools (crates.io and upstream changelog fact-checking, codegraph structural queries)
plus three read-only sub-audits and a focused adversarial read of the process backend.

### 已修 / Fixed

1. **进程后端的"硬超时"曾经不硬（真实缺陷）。** 旧实现有三处无界阻塞，前两处用真实 crate
   实测复现：
   - deadline 轮询从不排空 stdout。管道只有约 64 KiB，写得更多的子进程阻塞在 `write` 里
     永不退出，于是宿主杀掉一个**健康**子进程并报 `Timeout`。实测边界：帧 65 536 字节成功、
     65 537 字节超时，而 `max_output_bytes` 声称 1 MiB——真实上限是管道缓冲，且错误种类是错的。
   - `stdin.write_all` 发生在 deadline 之前。1 MiB 输入（正好等于 `max_input_bytes`）写给
     不读 stdin 的子进程，会阻塞整个 5 秒生存期后报 `BrokenPipe`：2 秒超时从未生效。
   - `read_frame` 在 deadline 之外：孙进程持有 stdout 写端时可永久阻塞（推理，未复现）。
   修法（不动任何公开类型）：请求帧一次构造后交给独立写入线程，stdout 与 stderr 各由独立
   线程排空，deadline 覆盖整个调用。两处顺序细节由新测试逼出来：读取线程的"超过上限"拒绝
   必须保持**最终**（否则会退化成误导性的 `Timeout`），而其他读取失败必须让位于退出状态
   （否则 `Process` 会被 `Io` 掩掉——这条是被既有测试 `timeout_and_crash_are_distinct_failures`
   抓到的）。`toolchain/plugin_host/tests/fault_matrix.rs` 新增 5 条测试（进程后端 1 → 6）钉住
   65 532/65 533/65 536/65 537 边界、超上限报 `Limit`、不读 stdin 的子进程仍按期超时、
   大输入能到达读取的子进程、以及 stderr 灌满不再阻塞。残留边界写进了代码注释：
   `Child::kill` 只杀直接子进程，孙进程可让管道保持打开，此时调用仍在 deadline 返回，但该次
   调用的分离线程可能继续阻塞；连进程组一起杀需要 `libc`，本 crate 没有该依赖。
   后续收尾：并行跑多个插件调用时 `exec` 会瞬时以 `ETXTBSY` 拒绝刚暂存的可执行文件（实测报成
   `ExecutableFileBusy`），现在派生做有界重试，其他错误原样上报；`process.rs` 随后超过 450 行，
   尺寸棘轮要求拆分而不是加欠账项，于是底层管道/派生机制移入 `process/child.rs`（367 + 128 行）。
   另有一条既有测试的断言本身不稳健（20 ms 超时 + 无消息），已放宽到 250 ms 并打印实际结果——
   正是这条消息让上面那个 ETXTBSY 从"负载下的怪失败"变成可诊断的原因。

2. **`NodeId` 组合钉住（落盘契约）。** `GraftPlanDocument.target` 就是 `NodeId`，从
   `.xirang/external-grafts/<selector>/graft.plan` 解析回来；而此前全仓只有两个标准向量
   钉住 `from_bytes`/`from_name` 形式，路径与命名空间组合**只有关系式断言**——把 `from_path`
   的两个摘要输入换个位置、或翻转 `separator` 标志，全部现有测试照样通过，而系统里每个 id
   都变了（后果是已落盘记录静默失效，按 S1+ 政策只表现为警告）。现补字面量钉
   （`from_path`、`from_namespaced_path`、反斜杠折叠同值、`ROOT_NODE_ID`），值由 Rust 实测
   与 `python3 hashlib` 独立复算双向核对一致。同时把两个入口的折叠不对称（`from_path` 折叠
   声明名，`from_namespaced_path` 的外层不折叠）写成注释并说明**不要"对齐"**：声明名是 Rust
   标识符、不可能含分隔符，因此不可观测，但任何改动都会重命名现有每个节点。

3. **手写 SHA-256 有了差分预言机。** 它是 `const fn`（编译期算身份），**不能**换成 `sha2`，
   所以改为**验证**它：新增 `sha2 = "0.10"` dev-dependency（经 `ed25519-dalek` 已在锁文件中，
   零新包、可离线），对空输入、长度 0..=200、填充边界 55/56/63/64/119/120/127/128、1000 与
   1 MiB 逐字节比对，另覆盖"两切片 + separator"与路径折叠两种组合（后者连两个切片的折叠都钉）。
   它守卫插件校验和接受与公钥指纹信任，此前只有一个 3 字节向量与一个 56 字节向量。

4. **四条只有散文、没有任何可执行门禁的规则变成会失败的门禁**（新 `conventions` crate，
   `publish = false`）：
   - 内核纯净性：`kernel/src` 禁 `std::fs`/`env`/`process`/`net`/`time`（注释除外；内联
     `#[cfg(test)]` 也覆盖，因为目录本身就是承诺，需要 I/O 的测试放 `kernel/tests/`）。
   - 模块挂载：无 `mod.rs`；全仓**只有一个** `include!`。后者的理由不是风格而是隐性行为：
     `include!` 是文本拼接，被拼入文件里的 `file!()` 报告的是引入方文件，会静默改变注册面的
     `NodeId`。这条门禁只查有隐性故障模式的两条，不查风格：裸 `mod x;` 在 crate 根或
     `#[path]` 载入的父文件下同样正确（全仓 29 处），对它们设门禁只会为五个 crate 带来
     没有故障模式的改动。
   - 450 行棘轮：见"下一批"第 1 条的更正。
   - `missing_docs`：属性存在性 + `#[allow(missing_docs)]` 禁令（lint 本身早由
     `clippy -D warnings` 强制，但删掉属性或用 allow 让单项闭嘴，对那个 lint 是不可见的）。
   每条都用"制造违规 → 门禁失败 → 还原"实测过。归属独立 crate 的原因：门禁遍历**仓库**，
   而已发布 crate 会把 `tests/` 打进 `.crate`，放在那里会让任何解包者失败。

5. **文档代码块解析门禁。** 30 个 markdown 文件里的 36 个 Rust 围栏此前没有任何程序抽取、
   编译甚至解析过，而九个 crate 的 README 就是 crates.io 落地页（每个清单都写
   `readme = "README.md"`）。现在逐个解析（整份文件或语句块两种读法都接受，因为多数块是有意
   的摘录）。审计与设计记录（`audit-*`/`design-*`）**有意排除**：它们是记录，其中的 `$crate`
   宏臂从来不是可独立成立的 Rust，为过门禁而改动等于改写记录。**所有 README 与活文档全部通过。**
   限制如实登记：解析能抓语法腐化，抓不到 API 漂移；让摘录真正编译是另一件更大的工作
   （`#![doc = include_str!("../README.md")]` 会把 50 个块全变成 doctest，而多数无法独立编译）。

6. **CI doctest 门禁 + 更正一处假声明。** 本文两处把 `compile_fail,E0433` doctest 说成
   `edit_module` 删除的机器钉子，但 CI 的 `features` 任务跑的是 `--all-features --all-targets`，
   而 `--all-targets` **会完全跳过 doctest**（实测 0 个 `Doc-tests` 段）；唯一跑 doctest 的
   release-audit 任务用默认特性，而那条 fence 门控在非默认的 `authoring` 之后——**它从未被
   任何命令编译过**。全工作区实际执行的 doctest 只有 1 条。`features` 任务已新增
   `cargo test --workspace --all-features --doc`，两处声明改为与事实一致。

7. **`petgraph` 0.6 → 0.8.3（离线，零代码改动）。** 它只支撑 `CallGraph` 的私有字段，不在
   公开 API 里，所以大版本升级不影响用户。

8. **`MirGraph::to_jsonl` 会写出非法 JSON（真实缺陷，已修并实测）。** `escape_json` 只转义
   `\` 与 `"`，而 RFC 8259 还要求转义 U+0000–U+001F；一个含制表符的函数名会让每一行对严格
   读取器非法，而本 crate 自己的宽松解析器仍然接受它，所以往返测试永远绿。修前用独立复现器
   证明（python `json.loads` 报 `Invalid control character`），修后同一复现器报 **VALID**。
   内核新增唯一的编码器 `xirang_kernel::json`，`diagnostic/build.rs`（原先唯一正确的那份）、
   `mir/render.rs` 与 `build_method` 的脚手架片段共用它；钉子测试断言"不存在原样控制字符"
   而不是做往返。顺带把 `diagnostic/build.rs` 缩回 429 行，尺寸棘轮因此强制删掉它的欠账项
   （15 → 14）。

9. **项目根解析三份、规则不一，已收口为内核里的一条纯规则。** 环境变量名此前是三个 crate 各
   写一遍的字面量，这违反 lexicon 自己的"文本契约只有一处定义"。现在 `lexicon` 拥有
   `PACKAGE_ROOT_ENV`/`NAMESPACE_ENV`/`DEFAULT_NAMESPACE` 与纯函数
   `resolve_package_root`/`resolve_namespace`：内核不能读环境或当前目录（新一轮门禁挡着），
   因此执行面把采集到的值作为**参数**传入，决策本身是纯的、可单测的。Studio、authoring 执行器
   与 MCP 桥都改调它。行为不变；唯一有意的差异写在 MCP 那处：它的最后兜底仍是当前目录，因为
   stdio 桥是在代理所处理的项目里启动的，而编译进去的清单路径属于构建该二进制的那台机器。

10. **包内容检查覆盖九个 crate（此前只覆盖 core）。** `cargo package --list` 不需要 registry，
    因此"每个 `src/**/*.rs` 模块与清单声明的 README 都在包里"这一半对九个 crate 全部
    生效——而本工作区用 `#[path]` 挂载模块，cargo 没收进去的文件等于一个对任何人都编译不过的
    crate。已实测：给 `kernel` 加一条 `exclude`，门禁只报 `kernel` 并点名那个文件。
    顺带修掉该脚本一个**先前就存在**的 bug：依赖表用空白分隔，而 `for entry in $crates` 也按
    空白切分，于是 `studio`/`cli` 只按第一个依赖被评估、摘要里出现并不存在的 crate。改为逗号
    分隔并据此解析。

11. **外部路径预演（离线可做的部分已做）。** 把真实的示例宿主连同它的 graft crate 复制到工作区
    之外、作为独立 workspace 构建：`build.rs`、`OUT_DIR` 生成、`host!()` 展开、`#[path]` 面挂载、
    静态 graft 全部成立，**26 条测试全绿**。另：`cargo package -p xirang-kernel` 的隔离校验构建
    通过；九个 crate 的包内容全部核对（250 个源码模块 + README + LICENSE，零缺失）。
    **仍无法离线做的一件事**：从 tarball 构建那八个 crate。它们的带版本号 `xirang-*` 依赖还
    不在 index 上，`cargo package`（含 `--no-verify`）都要向 registry 解析，因此这一步被首次
    发布本身挡住，已如实登记而不是绕过。

### 未做，等你定 / Open

- ~~**`wasmi` 0.42.1 → 1.0.9 或 2.0.0。**~~ **已做（你批准联网后）。** 下限设为 `1.0.9`
  （锁文件解析到 `1.1.0`），**只改了一处代码**：`Linker::instantiate` + `PreInstance::start`
  在 1.x 合并为 `instantiate_and_start`，它运行同一个 `start` 函数，燃料仍在其之前设定。
  选 1.x 而不是 2.0 的理由：1.0.9 起就带齐了沙箱需要的全部修复（0.48 的内存路径溢出、1.0.1
  的内存增长越界、1.0.5 的 `rem_s`、1.0.6–1.0.9 的错误编译），而 2.0 换了燃料计量基准
  （绑定到输入字节码），那会逼我在同一步里既迁移引擎又重新标定 `fuel_per_call`。
  `WasmLimits::fuel_per_call` 的文档已注明：单位属于引擎，大版本升级可能重新标定它，针对旧
  引擎调过该值的宿主应重新实测（这一条我**没有**实测，如实登记）。
- **死代码与重复已清（你问的那批）。** 删除 6 处零调用者的项（只写不读的 `RegistryHeader::name`
  与其 `admission()`、`EntryPages::iter()`、`FaceManifest::get()`/`fields()`、
  `SearchRow::source_index`、无人引用的 `state::FACE_FORM_ORDER` 重导出）、一个标错位置的
  `#[allow(dead_code)]`、两处模块级 `#[allow(dead_code)]`，以及**约四十条**未使用的 import——
  最后这一组是"删掉 allow 让 rustc 点名"发现的，子代理只看出 6 条。五条重复规则各归并成一份
  实现：切口端点渲染（3 处 → 1）、CLI 的 JSON 错误文档与错误文本（2 → 1）、Studio 的计划路径
  （2 → 1）、PascalCase kind 推导（2 → 1）、按 `::` 边界匹配符号（2 → 1，`same_symbol` 因此
  公开）。过程中尺寸棘轮两次拦下我自己：`process.rs` 越线（拆成 `process.rs` + `process/child.rs`）
  以及**把新规则写进了本就超标的 `syntax/entries/graft.rs`**（改为放进 `build_method`，而不是
  抬高钉住的行数）。
- **29 个 `ignore` 文档片段**（全在 `toolchain/run_method/src/macros/face.rs`，authoring DSL 唯一的
  在码文档）仍未编译；它们是有意的字段片段，转成可编译示例是一份独立的文档工作。
- **第四轮登记的其余新发现已全部处理**，除上述片段外没有遗留。

## 注释标准 / Comment standard（本轮起强制执行）

1. 每个文件双语 `//!`，每个公开项双语 `///`。
2. **易错逻辑必须写双语块注释**，说明三件事：为什么直白写法会错、边界在哪、
   哪条测试钉住它。只写意图与约束，不复述代码。
3. 易错清单（改动时必须带注释）：宏 arm 的参数绑定与展开、范围与 `" to "`、
   路径前缀判断、静态/owned 双份校验、identity 的哈希输入与命名空间、const 断言、
   graft 三层（声明/应用/记录）边界、feature 门控、临时目录唯一性、Windows 分隔符。
4. 注释不写"// 增加计数"这类复述；写"为什么是 1 而不是 0：单元素区间两端相同"。

## 缺失文档 lint 的边界（实测发现，无需修改）

- **宿主侧**:一个开了 `#![warn(missing_docs)]` 的宿主 crate，实测只对**作者自己写的**公开项
  报警(`pub struct Button`、各面的标记类型、`build.rs` 的 crate 文档)，
  `RUSTFLAGS="-W missing_docs" cargo check -p xirang-example-control-button --all-targets`
  共 11 处,全部在示例自身源码里。宏生成的 `NODE_ID`/`REGISTRATION` 没有报警——它们由嵌套
  `macro_rules!`(`__registration_face!`)发射,span 落在 `run_method` 的宏定义文件里。因此框架
  不需要为生成的公开项补文档,宿主只需要文档化自己的类型。
- **唯一的例外**是直接调用双下划线内部宏 `__xirang_object!` 的探针
  (`toolchain/call_evidence/tests/collector_integration.rs`),它会把 `NODE_ID`/`REGISTRATION` 放在测试
  crate 根部并触发两条警告。那是测试 crate、用的也是内部宏,不在发布面的 lint 范围里；
  如果将来要把 `-W missing_docs` 也加到测试 crate,再给这两个常量补 `#[doc]` 或改探针写法。

## 已登记但不做的 / Recorded, not planned

- 多跳 re-export（`declaration` → `plugin::contracts` → `registry_core` → 根部 → 执行面）：
  AGENTS.md 明文要求执行面保留历史路径，不动。
- 构建期 panic（`parsed_face`、`assert_static_registration`）：让构建响亮失败是设计选择。
- `Registry::registry_mut` 的"先收集路径再走"：为避开 `Arc::make_mut` 的页拷贝，不是绕路。

## 存疑待验 / Open uncertainties

- ~~`external_object!` 链上 `file!()` 落到哪。~~ 已收口并**修正了我的推断**：默认值是
  `manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!())`
  （`toolchain/src/run_method/macros/face_external.rs:102-104`），两个部分都在**声明它的** crate 里
  展开，因此项目外面报告自己的文件——但**拼写方式取决于 Cargo 怎么把它交给 rustc**：
  `manifest_relative_source` 先剥 manifest 前缀、再剥开头的 `src/`，而不以 manifest 目录开头
  的路径原样保留。因此发布包里 `src/control/control.rs` 记录为 `control/control.rs`（注册布局
  路径），而在工作区里构建的集成测试记录为 `toolchain/run_method/tests/external_source_default.rs`。
  由新测试 `toolchain/run_method/tests/external_source_default.rs::an_external_face_defaults_its_source_to_this_file`
  钉住（有默认分支、非绝对路径两条断言，并做了 Windows 分隔符归一化）。
- 非 full graft 覆写原树子注册机 namespace（`overlay.rs` 的 non-full 分支）：**已判定不可
  观测**。`header.namespace` 全仓只被 `register_snapshot_batch` 读取，而子注册机只能由内核
  经 `pub(super) fn registry_mut` 可变触达，宿主没有公开入口向被嫁接的子树注册。结论已写进
  该分支的块注释（连同"若将来出现公开可变子注册机入口，要修的就是那一行"），因此不再需要
  反例测试——没有可断言的公开观察点。
- ~~`resolve_cut_targets` 按 `registry_name` 排序是否等价于"连续兄弟区间"的意图。~~
  已收口（"下一批"第 6 条）：顺序契约写进代码、`docs/graft.md` 与 README，写反的端点报错。
- ~~`edit_module` 的严重度取决于它是否算 1.0 的公开 API。~~ 已由 B3a 关闭：它被删除，并用
  `compile_fail,E0433` doctest 钉住"它不再存在"（委托会静默窄化字符串形式支持的字段名）；
  自第四轮起，该钉子由 CI `features` 任务新增的 `--all-features --doc` 步骤真正编译
  （此前 `--all-targets` 会跳过 doctest，所以它从未被任何命令编译过）。
