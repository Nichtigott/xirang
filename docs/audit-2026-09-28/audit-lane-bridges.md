# 审计 D `lane-bridges` —— 桥与宿主：mcp / cli / plugin-host（含安全边界）

范围：`mcp/src`（39 文件 / 8682 行，`tools.rs` 是 17 个工具的目录与分派）、`cli/src`（14 文件 / 2802 行）、
`plugin-host/{src,tests}`（11 文件 / 3888 行 + `tests/fault_matrix.rs` 968 行）。对象是工作树
（HEAD=`cf0c378`，120 项未提交改动）。**本轮只出报告，源码一行未改**；写权限只落在本文件。

方法：逐文件读源码（`codegraph_explore` 与 `read`），按「自我描述 ↔ 行为」逐条对照出厂文档
（`mcp/README*.md`、`cli/README*.md`、`plugin-host/README*.md`、`--help`/`USAGE`、工具 `inputSchema`）；
只读实测用于量化与计数（`BR-4` 的 36 / 46 MB、`BR-15` 的表格行数、`BR-C4` 的 `grep`）。没有跑 `cargo`
（未产生 `target/` 写入），没有跑 `git checkout/restore/stash`，
没有改任何源码。证据标签：`[代码]`=读源码得到，`[实测]`=在只读前提下跑过命令/脚本得到。

严重度约定沿用本轮：CRITICAL=静默错答/数据损坏/安全/不可逆；MAJOR=明确行为错误或维护性硬伤；MINOR=局部可读性/一致性。

## 摘要

| id | 严重度 | 一句话 | file:line |
| --- | --- | --- | --- |
| `BR-1` | MAJOR | 两份 mcp README 仍在承诺 `unmatched` 桶，而它已在上一轮被删（现存的是 `undeclared`） | `mcp/README.md:47`、`mcp/README.zh-CN.md:35` |
| `BR-2` | MAJOR | mcp README 称「树 diff 仍待做」，而同文件 120 行前就在描述 `xirang.diff` | `mcp/README.md:164-166`、`mcp/README.zh-CN.md:113` |
| `BR-3` | MAJOR | `xirang.callgraph` 读一个 `inputSchema` 里没有的 `limit`，而回复又叫调用方「raise `limit`」 | `mcp/tools.rs:79` vs `mcp/callgraph.rs:41-44,110` |
| `BR-4` | MAJOR | 预览 diff 把**每个非 UTF-8 文件**报成新增（本检出实测 36 个），且无上限、且每次预览整份复制 `.git` | `mcp/preview.rs:37-58,102-121` |
| `BR-5` | MAJOR | 一帧非 UTF-8 请求会结束整个桥，而同样畸形的「超长行」「坏 JSON」都被作答且会话继续 | `mcp/protocol.rs:104-106` vs `:95-103` |
| `BR-6` | MAJOR | 设了 `XIRANG_NAMESPACE` 时，`verify` 用 Cargo 名发布证据、`diff`/`search` 用覆盖名读，于是每个面都被自信地报成 `re-identified` | `mcp/verify.rs:43`、`mcp/registry.rs:59-61`、`mcp/tree_delta.rs:101-119` |
| `BR-7` | MINOR | `xirang.usages` 的描述字段清单比它实际打印的少五项 | `mcp/tools.rs:238-243` vs `mcp/usages.rs:113-141` |
| `BR-8` | MINOR | `xirang.diff` 的描述漏掉它新加的 `undeclared` 桶 | `mcp/tools.rs:146-155` vs `mcp/diff.rs:206-239` |
| `BR-9` | MINOR | 「可移植路径」同一 crate 里两种拼法：`MAIN_SEPARATOR` vs 无条件折 `\` | `mcp/index.rs:159`、`preview.rs:85`、`converge_trace.rs:246` |
| `BR-10` | MINOR | `tokens()` 三份逐字节副本、`blank()` 两份；`requires` 有两个语义不同的解析器 | `mcp/{impact,usages,converge}.rs` |
| `BR-11` | MINOR | 测试挂载方式不统一：16 处 `#[path]` + `index.rs`/`tools.rs` 的内联 `mod tests` | `mcp/index.rs:218-219`、`mcp/tools.rs:404-410` |
| `BR-12` | MINOR | 目录顺序与分派顺序不一致，且没有钉子保证两张名单彼此齐全 | `mcp/tools.rs:58-272` vs `:293-317` |
| `BR-13` | MINOR | `--help` 在七个子命令里有四种不同下场；裸 `xirang` 打印用法后退出 0 | `cli/lib.rs:139-142` 等 |
| `BR-14` | MINOR | `new --path X --git Y` 静默取 `--path`，而 `build` 对同类矛盾是拒绝的 | `cli/commands/new.rs:63-65` vs `cli/lib.rs:203-233` |
| `BR-15` | MINOR | `cli/README.zh-CN.md` 的命令表少一条 `snippets`；两份 README 的 `studio` 行都漏了 `[path]` | `cli/README.zh-CN.md:13-24`、`cli/README.md:22` |
| `BR-16` | MINOR | `Instant::now() + timeout` 在宿主给出可溢出 `Duration` 时 panic（公开字段、无校验） | `plugin-host/src/process.rs:369` |
| `BR-17` | MINOR | 进程适配器的输入超限消息在 `u32` 帧宽那一支报的是配置上限 | `plugin-host/src/process.rs:296-302` |
| `BR-18` | MINOR | 懒激活在持 `pending` 锁期间编译；失败还会把待定工件吃掉，只能重装 | `plugin-host/src/lazy_wasm/slot_state.rs:74-107` |
| `BR-19` | MINOR | 预览副本是手工拼的 `/tmp` 路径、默认权限、无排他创建，且 diff 出错时泄漏 | `mcp/preview.rs:21-30`、`mcp/apply.rs:139-144` |
| `BR-20` | MINOR | 子进程先给出完整帧、再吊住不退出时，已送达的答案被丢成 `Timeout` | `plugin-host/src/process.rs:372-409,421-425` |

注释轴（维护者 2026-09-28 追加，详见「注释清晰度与可读性」一节；`BR-C0.1..C0.5` 是既有条目的归并 id）：

| id | 严重度 | 一句话 | file:line |
| --- | --- | --- | --- |
| `BR-C1` | MAJOR | stdout 线程的注释说「已送达的答案被报成超时」已消除，实际只消掉了管道阻塞那一半 | `plugin-host/src/process.rs:341-354` |
| `BR-C2` | MAJOR | `preview.rs` 说「`target/` 是唯一可能很大的目录」，而 `.git` 既被复制又被列举 | `mcp/preview.rs:14-20` |
| `BR-C3` | MINOR | 同一实测在注释里有两个数字（142 vs 151） | `mcp/callgraph.rs:10-15` vs `:24-31` |
| `BR-C4` | MINOR | doc 注释把产品名拼成 `nihlink`（英中两半同错） | `cli/lib.rs:169,175` |
| `BR-C5` | MINOR | `slot` 一名三义；同一状态三种拼法（`reidentified` / `re-identified` / `added since build`） | `mcp/evidence.rs:112-124,221` 等 |
| `BR-C6` | MINOR | 该由类型承担的信息在注释里：无名三元组 `build_evidence`、半真的 `is_loaded` | `mcp/evidence.rs:40-55`、`plugin-host/src/lazy_wasm.rs:171-176` |
| `BR-C7` | MINOR | `apply()` 的「`work` 可能是项目也可能是副本」靠注释维持，签名看不出来 | `mcp/apply.rs:94-127` |

---

## BR-1 —— mcp README 仍在承诺已被删除的 `unmatched` 桶（文档承诺面；归并见 `BR-C0.1`）

- **严重度**：MAJOR　**类别**：自我描述与行为不一致（文档漂移）
- **file:line**：`mcp/README.md:47-48`、`mcp/README.zh-CN.md:35`；对照 `mcp/diff.rs:126-263`
- **现象**：英文版写 “``stale``, `unmatched` (a typed cut stores an expression, so an absent identity
  cannot be told from a re-identified one) and unreadable records are kept apart”，中文版写
  「`stale`、`unmatched`（类型化切口存的是表达式，身份缺席时无法分辨它与 re-identified）……」。
  而实现里 `unmatched` 桶已在上一轮按审计 `m1` **删除**（`diff.rs:179-194` 留下了删除说明），
  现在记录一侧实际有五个桶：`ok` / `undeclared` / `stale` / `re-identified` / `unreadable`
  （计数行 `diff.rs:206-212`，`undeclared` 专段 `:224-239`）。
- **判据**：`grep -rn unmatched mcp/README.md mcp/README.zh-CN.md` 各 1 命中（现在时承诺），
  `grep -rn unmatched mcp/src` 在生产代码里 0 命中（只有 `diff_tests.rs:165` 那条「不得出现」的负断言）。
  上一轮自己的记录（`docs/audit-3p-2026-09-27.md:74`）说「工具描述（`mcp/tools.rs`）同步删掉对
  `unmatched` 的承诺」——`tools.rs` 改了，两份 README 没改。
- **最小修复方向**：把两份 README 的那半句换成 `undeclared`（并补上它的后果：没有切口点名的记录会被发布剪掉），
  或改成「`ok` / `undeclared` / `stale` / `re-identified` / `unreadable`」这一行枚举，与 `diff.rs:206-212` 同字。
- **复核手段**：`grep -c unmatched mcp/README.md mcp/README.zh-CN.md` 应为 0；
  `grep -n "undeclared" mcp/README.md mcp/README.zh-CN.md` 各 ≥1；
  `cargo test -p xirang-mcp --offline` 中 `the_record_side_tells_a_stale_record_from_a_re_identified_one`
  与 `an_undeclared_record_is_not_reported_as_ok` 仍绿。

## BR-2 —— mcp README 自相矛盾：「树 diff 仍待做」（文档承诺面；归并见 `BR-C0.2`）

- **严重度**：MAJOR　**类别**：自我描述与行为不一致（文档漂移，且同文件内自相矛盾）
- **file:line**：`mcp/README.md:164-166`、`mcp/README.zh-CN.md:113`；对照 `mcp/README.md:41-48`、`mcp/diff.rs`
- **现象**：同一份 README 的 **Reads 列表第 7 条**（`:41-48`）整段描述 `xirang.diff` 的面级 delta 与
  `records: true`，而结尾一段写 “Graft writes, plugins, project scaffolding, tree diffs, and the
  consistency analysis are still to come (`docs/roadmap-1.0.md` item 7)”（中文：「graft 写入、插件、
  项目脚手架、树 diff 与一致性分析仍待做」）。树 diff 就是 `xirang.diff`，已出厂；被引用的
  `docs/roadmap-1.0.md` 第 7 条现在是别的事（「下一批」列表第 7 条是 C19 诊断字段表，第四轮第 7 条是
  `petgraph` 升级），引用也已失效。
- **判据**：`grep -n "xirang.diff" mcp/README.md` → `:41`；`grep -n "still to come" mcp/README.md` → `:165`。
  工具存在且被 `tools_tests.rs:83-98` 钉住（`xirang.diff` 断言 `no build evidence`）。
- **最小修复方向**：删掉这句过期的路线图话，或按当下实际剩下的集合重写并给出**可解析**的路线图锚点
  （若没有剩下的，就写「本桥的能力即上面两张清单」）。
- **复核手段**：改后 `grep -n "still to come\|仍待做" mcp/README*.md` 应不再把 `tree diff` 列入未做；
  `cargo test -p xirang-mcp --offline` 全绿。

## BR-3 —— `xirang.callgraph` 读一个未声明的 `limit`（而回复又让调用方去 raise 它）（工具 rustdoc 承诺面；归并见 `BR-C0.3`）

- **严重度**：MAJOR　**类别**：自我描述与行为不一致（参数契约）
- **file:line**：`mcp/tools.rs:77-80`（catalog schema）、`mcp/callgraph.rs:41-44`、`mcp/callgraph.rs:108-113`
- **现象**：catalog 给 `xirang.callgraph` 声明的 `inputSchema.properties` 只有 `function` / `path` / `root`，
  描述是「Show direct static callers and callees for one function.」；实现却读 `limit`
  （`value.clamp(1, 50)`，默认 5 个定义），并在截断行里主动叫调用方
  `… +N more definitions (raise \`limit\` or pass \`path\`)`。按 `inputSchema` 校验参数的 MCP 客户端
  无法发出这个参数，于是那句提示是**不可执行**的；反过来，一个未声明的键会静默改变答案长度。
- **判据**：`tools_tests.rs:40-59` 逐项断言 explain/diff/trace/impact/grafts 的窄化参数**必须**被声明，
  唯独没覆盖 `callgraph` 的 `limit`——这正是一条「空转的名单钉子」。
- **最小修复方向**：在 catalog 里补 `"limit":{"type":"integer","minimum":1,"maximum":50}`，描述里也点名它；
  或删掉 `limit` 读取（只留 `path`）。两条都要顺手把 `callgraph` 加进 `the_evidence_tools_are_advertised_…`
  那张表，让「声明了才准读」变成一条可执行规则。
- **复核手段**：`cargo test -p xirang-mcp --offline tools_tests` 中把 `("xirang.callgraph","limit")`
  加进那张表后，改前应红、改后应绿。

## BR-4 —— 预览 diff 把每个非 UTF-8 文件报成「新增」（且无上限、且整份复制 `.git`）（其中 `preview.rs` 的目录说明另有注释面条目 `BR-C2`）

- **严重度**：MAJOR　**类别**：写入路径 / 预览正确性（自信的错误答案）+ 无界回复
- **file:line**：`mcp/preview.rs:102-121`（`diff_package`）、`mcp/preview.rs:37-58`（`copy_directory`）、
  `mcp/apply.rs:137-150`
- **现象**：`diff_package` 先 `let after = std::fs::read_to_string(work.join(&relative)).unwrap_or_default();`
  （`:108`，读不出的文件变成空串），再对**项目侧**读文本：`Ok(before) if before == after => continue`、
  `Ok(before) => "~ …"`、`Err(_) => "+ {relative}"`（`:109-121`）。一个**两边完全相同**的非 UTF-8 文件
  两侧 `read_to_string` 都失败，于是落进 `Err(_)` 支，被报成 `+ <path>`——一次没碰过它的编辑会把它列进
  「哪些文件会变」。`copy_directory` 只跳过 `target` 与 `.xirang`（`:45-47`），因此整份 `.git`
  也在副本里、也在被列举的集合里。
- **判据**：`[实测]` 在本检出上做只读统计：排除 `target/`、`.xirang/` 后有 **36** 个非 UTF-8 文件，
  其中 **29** 个在 `.git/` 下（另有 `.git/index`、`.codegraph/codegraph.db`、`.dsh-meow/memory.db` 等）；
  即在本仓上跑一次 `xirang.apply` 预览，回复里会出现 36 条虚假的 `+ …`。
  同一脚本量出排除 `target` 的树是 46 MB（`.git` 9.9 MB），而每次预览都要复制一遍。
  `apply_tests.rs` 的夹具全是文本文件，所以现有测试看不到这件事。
  另外该 diff **没有任何上限**（`apply` 的 schema 里也没有 `limit`），与本桥其它每个答案都带界的做法相反
  （`callgraph.rs:10-15` 的模块文档正是以「代理读不下的答案不算答案」立论）。
- **最小修复方向**：① 项目侧也读不出来时，若两侧原始字节相等就直接 `continue`，不要把「读不出」当成「新增」；
  或用 `fs::read` 比字节、只有确认变化才渲染。② `copy_directory`/`collect_files` 跳过 `.git`（以及其它 VCS/工具目录）。
  ③ 给 diff 加行数上限并声明出来。
- **复核手段**：新增钉子——夹具里放一个含 `\xFF` 的二进制文件（或直接把 `.git/index` 的字节写进夹具），
  断言 `apply` 的预览回复**不含** `+ <那个文件>`；`cargo test -p xirang-mcp --offline apply_tests`。

## BR-5 —— 一帧非 UTF-8 请求结束整个桥，而同类的畸形帧都被作答

- **严重度**：MAJOR　**类别**：协议可靠性 / 自我描述与行为不一致
- **file:line**：`mcp/protocol.rs:104-106`（致命）、`:95-103`（超长行：作答后继续）、
  `:116-138`（坏 JSON / 空 batch：作答后继续）、`:227-241`（非对象成员：作答后继续）；`mcp/main.rs:15-19`
- **现象**：`Frame::Line(bytes)` 里 `String::from_utf8(bytes)` 失败会把 `run_with` 整个 `Err` 出去，
  于是 `main` 打印 stderr 并 `exit(1)`——会话终止。而同一层对「超长行」是写出 `-32600` 后 `continue`，
  对「不是 JSON」「不是对象」是写出 `-32700`/`-32600` 后继续。四类畸形帧里只有一类是致命的。
  两份 README 的开篇承诺正好相反：`mcp/README.md:7` “writes nothing to stderr: a failure is an error
  response on stdout, where the client is already reading”、`mcp/README.zh-CN.md:5` 「**不写 stderr**：
  失败是 stdout 上的错误响应」。
- **判据**：`grep -n "utf8\|UTF-8" mcp/protocol_tests.rs` 只在工具函数里出现，**没有**一条钉子覆盖
  非 UTF-8 帧；`protocol_tests.rs:119-137` 只钉了超长行那一类。
- **最小修复方向**：把非 UTF-8 当成与超长行同类的坏帧：写一条 `-32700`（id 为 `null`，说明该行不是 UTF-8）
  后 `continue`；「读不了 stdin」这类真传输故障才返回 `Err`（`read_frame` 的 IO 错误保持致命）。
- **复核手段**：新钉子——把一条含 `0xFF` 的行与一条 `ping` 依次喂给 `run_with`，断言 stdout 上先有一条
  `-32700`、随后 `ping` 有回复（`replies()` 辅助函数已在 `protocol_tests.rs:79-95`）。

## BR-6 —— `XIRANG_NAMESPACE` 下 `verify` 与 `diff`/`search` 各用一套命名空间

- **严重度**：MAJOR（触发条件是设置该环境变量；一旦设置，答案是自信的错误答案）
  **类别**：跨工具一致性 / 正确性
- **file:line**：`mcp/verify.rs:43`（`package_name`＝Cargo 名）、`mcp/registry.rs:59-61`（`namespace()` 先读覆盖）、
  `mcp/tree_delta.rs:71-118`、`mcp/diff.rs:40-41,62-63,92-98`、`mcp/search.rs:58-61,132-143`
- **现象**：`verify` 把 `xirang_build_method::package_name(&manifest)`（Cargo 的包名）交给 `check_for`，
  于是本次运行发布的 `pruning_manifest.tsv` 行 id 是让 Cargo 名算的
  （`build_method/manifests.rs:181` → `registry_identity::package_node_id`，运行内生效的是
  `check_for` 设下的线程局部名）。而 `diff`/`search` 的源码侧走 `crate::registry::namespace(root)`，
  它**先读 `XIRANG_NAMESPACE`**。两者不一致时，`TreeDelta::by_source` 仍按源码路径命中
  （路径与命名空间无关），于是 `status()` 对**每一个**面走 `Reidentified(previous)` 分支：
  `diff` 报 `added 0 gone 0 reidentified N`，`search` 给每条命中标 `re-identified`，而
  `build_output_is_current` 说 `build current`（指纹只散列路径与内容，与命名空间无关）——一个「刚校验过」
  的树被报告成每个身份都变了。
- **判据**：`[代码]`：`verify.rs:43` vs `registry.rs:59-61`；`collect_pruning_symbols` 用进程/线程命名空间
  （`build_method/manifests.rs:181-184`）、`face_views` 用传入的 `package` 参数
  （`build_method/face_view.rs:126-174,211-247`）。代码里已登记的分歧 `S12` 只说了「桥与 CLI 不一致」，
  没说「桥自己内部 `verify` 与 `diff` 互相打脸」。
- **最小修复方向**：`verify` 用与 `diff` 同一个命名空间（`crate::registry::namespace(root)`）作为 `check_for`
  的 `package` 参数；或在两者不一致时明确拒绝并说明该设哪一个（与 `registry.rs:74-79` 对未命名包的处理同风格）。
- **复核手段**：新钉子——置 `XIRANG_NAMESPACE=<other>`，先 `verify` 再 `diff`，断言 delta 的行是
  `added 0 gone 0 reidentified 0`（而不是「每个面都 re-identified」）。注意 `namespace_from` 已是
  可注入参数（`registry.rs:80-92`），测试不必依赖进程环境。

## BR-7 —— `xirang.usages` 的描述比它打印的字段少五项（工具 rustdoc 承诺面；归并见 `BR-C0.4`）

- **严重度**：MINOR　**类别**：自我描述与行为不一致（欠说明；行为是描述的超集）
- **file:line**：`mcp/tools.rs:238-243` vs `mcp/usages.rs:103-142`
- **现象**：工具描述把读回的字段枚举为「preset, parts, the localized names, exports, requires, provides,
  handle traits and contracts, registration rule, admission, flow, runtime checks」；实现打印 20 个标签 +
  `needs_registry`，比清单多 `module`、`stable_name`、`getting_from_other_registry`、`flow_provider`
  与 `needs_registry` 五项（对照 `apply.rs:172-196` 的 `EDITABLE_FIELDS`，这五项都是可写的）。
  上一轮 `m2` 修的是**实现**的清单（并自审了 20 个标签全部可写），工具描述这一侧没跟上。
- **判据**：`grep -n "stable_name\|flow_provider\|getting_from_other_registry" mcp/usages.rs` 有，
  `mcp/README.md:91-98` 与 `tools.rs:238-243` 都只写「handle and part traits and contracts, registration
  rule, admission, flow, runtime checks」。危害方向是「问我能设什么」的代理拿到更短的答案（与 `m2` 同一条）。
- **最小修复方向**：把描述改成与 `usages.rs:113-141` 同字（或改成「每个可写字段都会打印，含 module / stable_name /
  getting_from_other_registry / flow_provider / needs_registry」），并在 README 的对应条目同步。
- **复核手段**：`usages_tests.rs` 里已有 `every_field_the_write_path_accepts_is_read_back` 这类钉子；
  加一条**字面**断言，让 `EDITABLE_FIELDS` 的每个名字都出现在 `tools()` 的 `xirang.usages.description`
  或 README 条目里（对照 `mcp/README.md:91-98`）。

## BR-8 —— `xirang.diff` 的描述漏掉 `undeclared` 桶（工具 rustdoc 承诺面；归并见 `BR-C0.5`）

- **严重度**：MINOR　**类别**：自我描述与行为不一致（欠说明）
- **file:line**：`mcp/tools.rs:146-155` vs `mcp/diff.rs:143-170,206-239`
- **现象**：描述说记录一侧的结论是「`ok`, `stale` …, or `re-identified` …; `unreadable` counts …」，
  而实现（`M1` 之后）把「身份在树里、但没有 `static_graft_plan!` 切口点名它的槽位」单独分成 `undeclared`
  并在计数行与专段里输出。计数行是五个桶，描述是四个。
- **判据**：`grep -n "undeclared" mcp/tools.rs` → 0 命中；`grep -n "undeclared" mcp/diff.rs` → 多处。
- **最小修复方向**：在描述里补上 `undeclared`（以及它与 `xirang.grafts` 的
  `NOT declared by the host entry`／`unkept plans N` 同判这一点）。
- **复核手段**：`grep -c undeclared mcp/tools.rs` ≥1；
  `cargo test -p xirang-mcp --offline diff_tests` 全绿。

## BR-9 —— 「可移植路径」在同一 crate 里两种拼法

- **严重度**：MINOR　**类别**：跨平台路径处理 / 一致性
- **file:line**：`mcp/index.rs:155-159`（`replace(std::path::MAIN_SEPARATOR, "/")`）、
  `mcp/preview.rs:85`（`replace('\\', "/")`）、`mcp/converge_trace.rs:246`（`replace('\\', "/")`）
- **现象**：`index.rs` 只折平台分隔符（Unix 上不动反斜杠），`preview.rs`/`converge_trace.rs` 无条件把 `\`
  折成 `/`。在 Unix 上一个名字里含反斜杠的文件（合法字节）会被 `xirang.inspect` 报成 `a\b.rs`、
  被 `apply` 的 declaration 锚点报成 `a/b.rs`——同一个桥对同一个文件给出两种拼法。
- **判据**：三处 `[代码]`；内核已有唯一实现 `portable_path`
  （`core/src/registry_core/declaration/source_location.rs:33-39`，公开地址是
  `xirang::declaration::portable_path`），本 crate 没有用。
- **最小修复方向**：三处改调它（或收一个 `mcp::portable_path` 助手转发），
  使「跨平台拼法」只有一份规则。
- **复核手段**：`grep -rn "replace('\\\\\\\\', \"/\")" mcp/src` 应为 0（除共享助手本身）；
  `cargo test -p xirang-mcp --offline` 全绿。

## BR-10 —— 三份 `tokens()`、两份 `blank()`，以及两个语义不同的 `requires` 解析器

- **严重度**：MINOR　**类别**：碎片化 / 重复实现（可读性硬伤）
- **file:line**：`mcp/impact.rs:288-295`、`mcp/usages.rs:192-199`、`mcp/converge.rs:227-234`（`tokens` 逐字节相同）；
  `mcp/usages.rs:209-211`、`mcp/converge.rs:238-240`（`blank` 相同）；
  `mcp/impact.rs:246-255` vs `mcp/converge.rs:192-207`（`requires` 解析）
- **现象**：三个工具各自持有一份完全相同的「标识符式记号」规则（连文档注释都一样），两份相同的 `blank`。
  更实质的是 `requires` 文本被解析两次且**语义不同**：`converge` 对没有 `=>` 的裸能力名报一条
  `capability => ? UNANSWERED`，`impact` 则把它从能力边集合里**静默丢弃**（`split_once("=>")` 返回 `None`
  时不 push）。同一个面因此在两个工具里有不同的能力结论。这正是本仓反复强调「一条规则一个地方」的反面
  （对照 `diff.rs:45-49`、`grafts.rs:196-201` 的注释）。
- **判据**：`diff <(sed -n '/^fn tokens/,/^}/p' mcp/impact.rs) <(sed -n '/^fn tokens/,/^}/p' mcp/usages.rs)` 为空；
  两个 `requires` 解析器行号如上一行所列。
- **最小修复方向**：提到一个共享模块（如 `mcp/faces_text.rs`）导出 `tokens`/`blank`/`parse_requires`，
  并把「裸能力名怎么答」定成一条决定（要么两边都报 UNANSWERED，要么两边都拒绝该输入）。
- **复核手段**：新钉子——同一个手写 `requires: "capability_a"` 的夹具下，`converge` 与 `impact` 对它的说法
  必须由同一条断言钉住（现在是两种）。

## BR-11 —— 测试挂载方式不统一（16 处 `#[path]` + 2 处内联，`tools.rs` 两者兼有）

- **严重度**：MINOR　**类别**：命名与碎片化（挂载方式）
- **file:line**：`mcp/index.rs:218-219`、`mcp/tools.rs:404-410`
- **现象**：`mcp/src` 的 39 个文件里，16 个源文件用 `#[cfg(test)] #[path = "<name>_tests.rs"] mod <name>_tests;`
  挂载（`apply.rs:561-563`、`protocol.rs:299-301`、……），而 `index.rs` 用内联 `mod tests { … }`（219-331），
  `tools.rs` **两种都有**（`tools_tests.rs` 的 `#[path]` 挂载 + 383 行的内联 `mod tests`，其中
  `a_huge_line_number_still_yields_a_forward_range` 测的是 `read_source`）。同一个 crate 两种挂法，
  找测试时要先猜它在哪；`AGENTS.md` 对模块挂载只规定生产模块的 `#[path]` 规则，没有覆盖测试模块。
- **判据**：`grep -rn "^mod tests {" mcp/src` → `index.rs:219`、`tools.rs:409`；其余 16 个模块是 `#[path]`。
- **最小修复方向**：把 `index.rs` 的内联测试移到 `mcp/index_tests.rs`，把 `tools.rs` 的内联两条
  并入 `tools_tests.rs`，让「一个源文件一个 `<name>_tests.rs`」成为唯一挂法（顺带让 `tools.rs` 只剩目录与分派）。
- **复核手段**：`grep -rn "^mod tests {" mcp/src` 为 0；`cargo test -p xirang-mcp --offline` 测试数与改前一致。

## BR-12 —— 目录顺序 ≠ 分派顺序，且没有钉子保证两张名单齐全

- **严重度**：MINOR　**类别**：责任/阅读顺序 + 缺钉子
- **file:line**：`mcp/tools.rs:58-272`（catalog）vs `mcp/tools.rs:293-317`（`tool_call` 的 `match`）
- **现象**：catalog 里 `xirang.apply` 排第 6、分派里排最后；catalog 里 `grafts` 在 `impact` 之前，
  分派里相反。两张名单必须逐名比对才能确认齐全（本次比对结果：**17 个工具两处都在**，见「查了、干净」），
  而 `tools_tests.rs:40-98` 只抽查了 6 个名字。「新增一个工具却漏了分派臂」会静默变成
  `unknown tool \`name\``（`:317`），「有分派臂没有目录项」则永不可达。
- **判据**：两张名单的 `[代码]` 逐项比对（本报告已完成）；`grep -c "xirang\." mcp/tools.rs` 的分布见上。
- **最小修复方向**：分派臂按 catalog 同序排列（`apply` 回到第 6 位）；补一条钉子：遍历 `tools()` 的每个
  `name` 调一次 `tool_call`，断言回复的文本不是 `unknown tool`。
- **复核手段**：新钉子（如 `tools_tests::every_listed_tool_is_dispatched`）在补臂前对缺失项应红。

## BR-13 —— `--help` 在七个子命令里有四种下场；裸 `xirang` 退出 0

- **严重度**：MINOR　**类别**：CLI 一致性 / 用法错误语义
- **file:line**：`cli/lib.rs:139-142`（顶层认 `--help`/`-h`/`help`）、`cli/commands/studio.rs:33-36`（认 `-h`/`--help`，打印 `USAGE`）、
  `cli/commands/check.rs:34`、`cli/explain.rs:58`、`cli/grafts.rs:43`、`cli/commands/snippets.rs:50`（`unexpected argument '--help'`）、
  `cli/commands/new.rs:52-57`（以 `-` 开头一律拒绝并给 `usage:`）、`cli/commands/build.rs:17-27`（把 `--help` 原样交给 `cargo build`）
- **现象**：`xirang check --help` / `explain` / `grafts` / `snippets` → 报错、stderr、退出 1；
  `xirang studio --help` → 打印完整 `USAGE`、退出 0；`xirang new --help` → 拒绝并给一行 usage；
  `xirang build --help` → 先做注册校验（在不含 `Cargo.toml` 的目录里直接报错），成功后才把 `--help` 交给 cargo。
  同一个通用旗标，四种语义。另外 `None | Some("--help") | … => 打印 USAGE; Ok(())`（`lib.rs:139-142`）让
  **裸调** `xirang` 也走同一条路：stdout 打印用法、退出 0，shell 管线看见「成功」。
- **判据**：`grep -rn '"--help"\|"-h"' cli/src` 只在 `lib.rs:139`、`studio.rs:33` 出现；
  `cli/lib_tests.rs:679` 只钉了 `studio --help`，`:847` 只钉了顶层 `--help`。
- **最小修复方向**：在 `cli/lib.rs` 加一个共享的 `help(out)` 分支，六个子命令的 argv 循环先认
  `-h`/`--help`；裸调（无子命令）打印用法到 **stderr** 并返回 `Err`，与「未知命令」同档。
- **复核手段**：钉子参数化跑 `["check","--help"] … ["mcp","--help"]`，断言都打印 `USAGE` 且 `Ok`；
  另加一条断言裸调返回 `Err`（退出码非零）。

## BR-14 —— `new --path X --git Y` 静默取 `--path`

- **严重度**：MINOR　**类别**：CLI 一致性 / 静默二选一
- **file:line**：`cli/commands/new.rs:63-65`；对照 `cli/lib.rs:203-233` 的 `build_target`
- **现象**：`--path` 与 `--git` 各自解析到 `path`/`git` 两个 `Option`，最后的 `match (path, git)` 里
  `(Some(workspace), _) => DependencySource::Local { workspace }` —— 两者都给时 `--git` 被静默忽略。
  而 `USAGE` 的 `new` 行与两份 README 都把它写成 `[--path <workspace> | --git <url>]`
  （互斥；`cli/lib.rs:45`），
  `build` 对「位置参数与 `--manifest-path` 点名不同项目」这一类矛盾是**拒绝**的
  （`lib.rs:226-230`：`pass one of them`）。
- **判据**：`new.rs:63-65` 的 `_` 臂；`cli/lib_tests.rs` 有 `an_option_shaped_name_is_refused`（`new` 的
  `-` 分支）但没有「两个来源都给」这条。
- **最小修复方向**：在 `match (path, git)` 里加 `(Some(_), Some(_)) => Err("--path and --git name two
  different sources; pass one of them")`，与 `build_target` 同款口径。
- **复核手段**：新钉子——`new probe --path <checkout> --git <url>` 必须 `Err` 且**不写任何文件**
  （用临时 CWD + 断言 `./probe` 不存在）。

## BR-15 —— 中文 CLI README 的命令表少 `snippets`；两份都漏 `studio [path]`

- **严重度**：MINOR　**类别**：自我描述与行为不一致（文档漂移；中文半边）
- **file:line**：`cli/README.zh-CN.md:13-24`、`cli/README.md:22`（studio 行）、`cli/lib.rs:41-53`（`USAGE`）
- **现象**：`USAGE` 与 `lib.rs:139-152` 的分派都有 **8** 条子命令（new/check/build/snippets/explain/grafts/studio/mcp）。
  `cli/README.md:15-23` 的表有 9 行（explain 两条），`cli/README.zh-CN.md:13-24` 只有 **8** 行——缺
  `xirang snippets`。两份 README 的 studio 行都写 `xirang studio`，而 `USAGE:52` 与
  `studio.rs:26-41` 都接受一个可选路径（`xirang studio <path>`，上一轮才修好「丢弃参数」那件事）。
- **判据**：`[实测]` `grep -c "^| \`xirang" cli/README.md cli/README.zh-CN.md` → 9 / 8；
  `grep -o "^| \`xirang [a-z-]*" cli/README.zh-CN.md` 无 `snippets`。
- **最小修复方向**：中文表补 `snippets` 行；两份表的 studio 行改成 `xirang studio [path]`。
- **复核手段**：`grep -c "^| \`xirang" cli/README*.md` 两边同数；与 `USAGE` 的八条逐条对照。

## BR-16 —— `Instant::now() + timeout` 可 panic（公开字段、无校验）

- **严重度**：MINOR　**类别**：函数级可靠性（panic）
- **file:line**：`plugin-host/src/process.rs:369`（`let deadline = Instant::now() + self.limits.timeout;`）、
  `plugin-host/src/process.rs:25-68`（`pub timeout: Duration`，`Default` 为 2s）
- **现象**：`timeout` 是公开字段，`ProcessLimits { timeout: Duration::MAX, .. }` 是合法构造；`impl Add<Duration>
  for Instant` 在溢出时 panic（std 的 `checked_add(...).expect("overflow when adding duration to instant")`），
  于是宿主一次配置错误会在**库里**panic，而不是拿到一个 `HostError`。本 crate 的其余输入上限（内存/表/燃料/帧宽）
  都走 `Result`。
- **判据**：`[代码]`：`process.rs:369` 无 `checked_add`；同文件其余限额路径（`:296-302`、`child.rs:156-168`）
  都返回 `HostError`。
- **最小修复方向**：`Instant::now().checked_add(self.limits.timeout).ok_or_else(|| HostError::Limit(...))?`，
  或把 `timeout` 收进一个带校验的构造器。
- **复核手段**：新钉子——一件 `ProcessLimits { timeout: Duration::MAX, .. }` 的 `call` 必须返回
  `Err(HostError::Limit)` 而不是让测试进程 panic（`#[should_panic]` **不算**：那正是现在会发生的）。

## BR-17 —— 输入超限消息在 `u32` 帧宽那一支报的是配置上限

- **严重度**：MINOR　**类别**：自我描述与行为不一致（诊断口径）
- **file:line**：`plugin-host/src/process.rs:296-302`；对照 `plugin-host/src/wasm.rs:330-343`
- **现象**：`if input.len() > self.limits.max_input_bytes || input.len() > u32::MAX as usize` 两个条件共用一条
  消息 `input is {len} bytes; limit is {max_input_bytes}`。当 `max_input_bytes > u32::MAX`（宿主可自定）而
  长度落在两者之间时，拒绝它的是**帧宽**，消息却把配置值说成上限——调用方据此调小配置也不会有用。
  wasm 侧已经为这件事分成两条消息（`wasm.rs:336-341`「a Wasm call frames its length in an i32…」，并有
  `wasm_tests.rs:13-25` 的钉子）。
- **判据**：`[代码]`：两处实现口径不一致（`PH-5` 修的就是「两个适配器口径不一致」，这里剩了消息这一半）。
- **最小修复方向**：拆成两条消息（配置上限 / `u32` 帧宽），与 `wasm.rs` 同形；或把 `check_input_length`
  提到共享位置供两个适配器使用。
- **复核手段**：把 `wasm_tests.rs` 的两条钉子按同样形状复制到进程适配器（单测一个纯函数即可，不必造 4 GiB 切片）。

## BR-18 —— 懒激活在持锁期间编译；失败的激活吃掉待定工件

- **严重度**：MINOR　**类别**：并发/责任边界（吞吐 + 重试语义）
- **file:line**：`plugin-host/src/lazy_wasm/slot_state.rs:74-107`
- **现象**：`activate` 在 `pending.lock()` 之后才 `pending.take()`，随后**在持锁状态下**执行
  `backend.load(candidate.artifact)`（编译 + 实例化 + `health_check`，即一次真实的 wasm 调用）。同一槽上
  任何并发的 `call`/`install`/`activate_pending` 都要等这次编译做完（`lazy_wasm.rs:137-145,162-169`）。
  另外失败路径把 `candidate` 连同一份已验证工件一起丢掉（`:101-106`），只留下 `activation_error` 字符串，
  因此一次瞬时失败（暂时性 OOM、锁被毒化等）后**无法重试**，只能重新 `install` 整个工件——
  而 `activation_error` 的文档说它「让轮询方能据以行动」，实际能做的只有重装。
- **判据**：`[代码]`：`load` 的调用点在 `pending` 锁的作用域内（`:78-93`）；`activate_pending` 的文档
  （`lazy_wasm.rs:147-161`）承诺它「是缺的那一步」，但没写「失败即放弃」。
- **最小修复方向**：在锁内只 `take()` 出候选、记下代际，放锁后再编译，最后用锁发布
  （`active.store` + `has_pending.store(false)`）；失败时要么把候选放回 `pending`（保留重试），要么在
  `activate_pending` 的文档里明说「失败即作废，重试需重新安装」。
- **复核手段**：`fault_matrix.rs:245-294` 那条八线程代际钉子已经覆盖并发安装；再加一条断言
  「失败后 `activate_pending` 再来一次仍给出同一个错误/仍然 `Ok(false)`」以钉住重试语义。

## BR-19 —— 预览副本的临时目录卫生与错误路径泄漏

- **严重度**：MINOR　**类别**：安全/可靠性（本地前提）+ 资源泄漏
- **file:line**：`mcp/preview.rs:21-30`（`std::env::temp_dir().join("xirang-mcp-preview-{pid}-{seq}")`
  + `remove_dir_all` + `create_dir_all`）、`mcp/preview.rs:60-64`（`remove_copy`）、
  `mcp/apply.rs:139-144`（`diff_package(root, &work)?` 之后才 `remove_copy`）
- **现象**：目标路径是**可预测**的（pid + 进程内自增），先用 `remove_dir_all` 清掉同名的东西、
  再用 `create_dir_all` + `fs::copy` 写入——没有排他创建（`mkdtemp`/`O_EXCL` 语义），也没有收紧权限
  （目录按 umask，通常 0755；文件由 `fs::copy` 保留 0644），因此工程源码会以默认权限落在共享的 `temp_dir`
  里；若同名路径已被另一个本地用户预置为**符号链接**，`create_dir_all` 会顺着它把副本写进目标目录，
  而 `remove_copy` 随后对这个路径调 `remove_dir_all`（`work != root` 的守卫只比 `PathBuf` 字面相等，
  挡不住符号链接这一层）。此外 `apply.rs:139-143` 的 `?` 在 `remove_copy`（`:144`）之前：
  `diff_package` 一旦出错（目录读不了等），副本**永久留在 /tmp**，里面是整份工程源码。
  对照：`plugin-host` 暂存子进程时用的是 `tempfile::Builder::tempfile()`（`process.rs:204-210`，
  排他创建 + 0600 + `TempPath` 自动清理），同一份工作区里两种卫生标准。
- **判据**：`[代码]`：`preview.rs:22-29` 无排他创建、无 `set_permissions`；`apply.rs:139-144` 的 `?` 早于
  `remove_copy`；`grep -n tempfile mcp/Cargo.toml` → 无该依赖。
- **最小修复方向**：① 用一个 `Drop` 守卫（或 `struct PreviewCopy(PathBuf)` 实现 `Drop`）保证任何返回路径都清理；
  ② 副本目录用排他创建 + 0700（引入 `tempfile`，或 `fs::create_dir` 直接失败于已存在 + 显式 `set_permissions`）。
- **复核手段**：新钉子——先在同名路径放一个文件/目录再调 `apply` 预览，断言副本落在**新**路径且旧路径内容未被动过；
  另一条让 `diff_package` 失败（把项目里某个目录权限去掉）后断言 `/tmp` 下没有残留副本。

## BR-20 —— 子进程先给出完整帧、再吊住不退出时，已送达的答案被丢成 `Timeout`（其中注释过头的面另有条目 `BR-C1`）

- **严重度**：MINOR　**类别**：函数级逻辑（答复丢弃；注释把已修的范围说过头）
- **file:line**：`plugin-host/src/process.rs:372-409`（轮询循环）、`:421-425`（只有成功退出后才用 `frame`）、
  `:341-354`（stdout 线程注释）
- **现象**：循环里 `frame` 只是被记下；`break` 只发生在 `child.try_wait()` 返回时，而 `Instant::now() >= deadline`
  的分支先于任何「用 frame 作答」的路径返回 `Err(Timeout)`。于是「写完完整帧、然后 `sleep 100`」的子进程会
  让宿主在截止点杀进程并报 `Timeout`，把已经收到的答案丢掉；`:421-425` 的 `if let Some(bytes) = frame`
  只在子进程**成功退出**后才可达——子进程写了好帧却以非零码退出时同样丢帧并报 `Process`。
  而同文件 `:341-353` 的注释把这件事说成已经解决（“只读一帧就停下曾是缺陷：……而一个已经送达的答案
  被报成超时”）。契约本身（`ProcessLimits::timeout`「单次调用的挂钟超时」）是站得住的，过头的只是注释与
  「先拿到答案」的期望。
- **判据**：`[代码]`：`:372-409` 的分支顺序；`fault_matrix.rs:769-785` 只覆盖「写完答案后继续写**并退出**」的子进程，
  没有覆盖「写完答案后不退出」。
- **最小修复方向**：二选一——① 在 stdout 读线程给出「帧已完整 + 已到 EOF」的信号后，允许在子进程仍活着时返回该帧
  （像 `read_frame` 的 `Limit` 早退那样有明确理由）；② 保持现状，但把 `ProcessLimits::timeout` 的文档写成
  「截止时间覆盖整个调用，含子进程退出；到期即丢帧报 Timeout」，并把 `:341-354` 那句收窄。
- **复核手段**：新钉子——子进程 `printf '<framed answer>' ; sleep 30`，`timeout: 200ms`；断言得到的是答案
  （方案①）或明确断言 `Timeout`（方案②），两条路都不能留成「谁也没说」。

---

## 注释清晰度与可读性（含文档承诺面）

维护者新增的轴：注释与文档承诺的清晰度、可读性。本节按该轴把既有发现归并，并补新条目。
**既有条目的完整证据（`file:line`、判据、复核手段）留在原处不搬**，这里给出归并 id 与要点，
原条目头部留了指向本节的指针，便于逐条核对而不产生两份会各自漂移的描述。

| 归并 id | 原条目 | 要点与严重度 |
| --- | --- | --- |
| `BR-C0.1` | `BR-1` | 两份 mcp README 仍以现在时承诺已被删除的 `unmatched` 桶（过时文档承诺，MAJOR） |
| `BR-C0.2` | `BR-2` | 同一份 README 里「树 diff 仍待做」与 120 行前的 `xirang.diff` 段互相矛盾（MAJOR） |
| `BR-C0.3` | `BR-3` | `xirang.callgraph` 的 rustdoc/schema 没声明实现会读的 `limit`，回复却叫调用方去 raise 它（MAJOR） |
| `BR-C0.4` | `BR-7` | `xirang.usages` 的描述字段清单比输出少五项（MINOR） |
| `BR-C0.5` | `BR-8` | `xirang.diff` 的描述漏掉 `undeclared` 桶（MINOR） |

`BR-4` 与 `BR-20` 各只有一半属于本轴（`preview.rs` 的目录说明、`process.rs` 的 stdout 线程注释），
分别在下面 `BR-C2` 与 `BR-C1` 里单列。

### BR-C1 —— stdout 线程的注释把「已修的范围」说过头

- **严重度**：MAJOR（轴 1：注释描述的行为与实现不符）　**类别**：注释与行为不一致
- **file:line**：`plugin-host/src/process.rs:341-354`；行为证据见 `BR-20`（`plugin-host/src/process.rs:372-409,421-425`）
- **现象**：注释说「只读一帧就停下曾是缺陷：子进程阻塞、宿主在超时点杀掉它，而一个已经送达的答案被报成超时」，
  读起来像是这种误报已经不可能再发生；实际只有**管道阻塞**那一条被消掉，而「子进程写完完整帧后不退出」
  仍会走到 `Err(Timeout)` 并把帧丢掉（`BR-20`）。注释解释的机制（先送帧、再排空）是对的，
  对**结果**的陈述过强。
- **判据**：`[代码]`：`:341-354` 的断言 vs `:404-406` 的 `return Err(HostError::Timeout)` 在 `frame` 被使用（`:421`）之前。
- **最小修复方向**：把这句收窄成「本线程保证：子进程即使继续写也不会阻塞在满管道上」，
  并把「何种情况下已送达的帧会被丢弃」写进 `ProcessLimits::timeout` 的文档（与 `BR-20` 的修法合起来做一次即可）。
- **复核手段**：`BR-20` 的钉子（写完帧后 `sleep 30`）落地后，重读这段注释——若仍说误报已消除，即为漂移。

### BR-C2 —— `preview.rs` 的目录说明与它实际跳过的集合不符

- **严重度**：MAJOR（轴 1）　**类别**：注释与行为不一致
- **file:line**：`mcp/preview.rs:14-20`（模块文档与 `copy_package` 的文档）、`:37-58`（`copy_directory`）；行为证据见 `BR-4`
- **现象**：两处文档都说副本跳过 `target/` 与 XiRang 自己的运行期目录，并断言
  「`target/` is the one directory that can be large」（`target/` 是唯一可能很大的目录）。
  实际只跳过 `target` 与 `.xirang`（`:45-47`），`.git` 会被整份复制并参与 diff——
  在本检出上 `.git` 是 9.9 MB，而真实仓库常见 GB 级；这句「唯一」因此是错的，
  也是 `BR-4` 那 36 条虚假 `+ …` 的直接来源。
- **判据**：`[代码]`：`:45-47` 的跳过条件只有两个名字；`[实测]`：`.git` 9.9 MB、树 46 MB（见 `BR-4`）。
- **最小修复方向**：改成「跳过构建产物与 VCS/工具目录（当前是 `target/`、`.xirang/`、以及拟新增的 `.git/`）」，
  或删掉「唯一可能很大」这半句——一句话的断言比一份清单更容易过期。
- **复核手段**：`grep -n "one directory that can be large" mcp/preview.rs` 应为 0；
  `BR-4` 的钉子（夹具含 `.git` 风格二进制文件）同时钉住行为。

### BR-C3 —— 同一实测事实在注释里有两个数字（142 vs 151）

- **严重度**：MINOR（轴 1/7：事实漂移、同一件事写两处）　**类别**：注释事实不一致
- **file:line**：`mcp/callgraph.rs:10-15`（模块文档：142）、`mcp/callgraph.rs:24-31`（函数内注释：151）、
  `mcp/callgraph.rs:110-113`（回复文本）、`mcp/tools_tests.rs:4-9`（测试文档：151）
- **现象**：同一件实测（常见名 `new` 的定义数）在模块文档里是 **142**，在函数内注释与测试文档里是 **151**。
  两处都在讲「这些上限为什么存在」，读者无法判断哪个是真测过的数字。
- **判据**：`grep -rn "14[0-9]\|15[0-9]" mcp/callgraph.rs mcp/tools_tests.rs` 得到上面四处；
  二者不可能同时对同一次测量成立。
- **最小修复方向**：把数字只留一处（建议留在模块文档，函数内注释改为「见模块文档的实测」），
  或统一为当时真实的那个值。凡是在两处复述同一个测量结果的注释，都应按这一条收口。
- **复核手段**：`grep -rn "definitions" mcp/callgraph.rs mcp/tools_tests.rs` 里同一个测量只应出现一个数字。

### BR-C4 —— 文档注释里的工具名拼错：`nihlink`

- **严重度**：MINOR（轴 4：术语/名字一致性）　**类别**：注释错别字（双语两半同错）
- **file:line**：`cli/lib.rs:169`（英文半边）、`cli/lib.rs:175`（中文半边）
- **现象**：`build` 的 argv 规则文档写 “and `nihlink build` hands the rest of its arguments to cargo verbatim”，
  中文半边同样写 `nihlink build`。可执行的命令名是 `xirang`；这是本仓唯一一处把产品名拼错的地方
  （`grep -rn nihlink mcp/src cli/src plugin-host/src` 只有这两行）。
- **判据**：`[实测]`：`grep -rn "nihlink\b" mcp/src cli/src plugin-host/src` → 2 命中（同一 doc 块的两半）。
- **最小修复方向**：两处改成 `xirang`。顺带说明：这类错字不会被 `doc_blocks`/`doc_anchors` 门禁抓到
  （它们管围栏与锚点，不管散文里的产品名），所以属人工复核项。
- **复核手段**：`grep -rn "nihlink" cli/src` 为 0（`xirang` 的匹配要用词边界）。

### BR-C5 —— 一个桥里 `slot` 有三个含义；同一状态有三种拼法

- **严重度**：MINOR（轴 4：同一概念多个名字）　**类别**：术语一致性
- **file:line**：`mcp/evidence.rs:112-124,221`（`slot` = 面的 `registry_name`）、
  `mcp/tools.rs:123-124`（explain 描述里的 `slot` 同上）、`mcp/overlay.rs:113-116` 与 `mcp/tools.rs:210-212`
  （`slot` = graft 切口针对的槽位）、`plugin-host/src/lazy_wasm.rs:29-45`（`slot` = 插件槽）；
  状态拼法：`mcp/diff.rs:100,117`（`reidentified`）、`mcp/diff.rs:207,248`（`re-identified`）、
  `mcp/search.rs:138-141`（`added since build` / `re-identified`）
- **现象**：① `slot` 在同一个桥的三种输出里指三样东西：`explain`/`evidence` 打印的 `slot <registry_name>`、
  `explain --overlay` 与 `grafts` 说的槽位、以及 plugin-host 的插件槽。代理按 `slot` 检索会同时命中三类答案。
  ② `tree_delta` 的模块文档声明「diff 与 search 对同一个面不能说两种话」，而同一个状态在两个工具里是
  `reidentified`（diff 默认 delta 的计数行与标题）与 `re-identified`（diff 的记录 delta、search 的标注），
  `added` 与 `added since build` 亦然——规则统一了，词形没有。
- **判据**：`[代码]`：上列行号；`grep -rn "reidentified\|re-identified" mcp/src` 两种拼法都在生产输出里。
- **最小修复方向**：① 面的那一列改用它真正的名字（`registry_name` 或 `slot` 保留给层叠/插件槽，
  `evidence.rs:112` 的格式串与 `tools.rs:123-124` 的描述同改）；② 状态词形由 `TreeDelta` 一侧统一导出
  （例如一个 `FaceStatus::label()`），`diff` 与 `search` 都调它——这也是 `BR-10` 那条「一条规则一个地方」的同一类收口。
- **复核手段**：`grep -rn "reidentified" mcp/src` 应为 0（统一为 `re-identified`）；
  两处 `slot` 输出各改后，`explain`/`grafts` 的现有钉子仍绿。

### BR-C6 —— 该由类型承担的信息被写进注释：无名三元组与半真名字

- **严重度**：MINOR（轴 5）　**类别**：注释在补命名的缺口
- **file:line**：`mcp/evidence.rs:40-55`（`build_evidence -> (bool, Option<BuildScopeView>, Option<Vec<PruningRow>>)`）、
  `mcp/overlay.rs:56`（调用处 `let (current, scope, _) = …`）、
  `plugin-host/src/lazy_wasm.rs:171-176`（`is_loaded`）
- **现象**：① `build_evidence` 的文档用**顺序**解释三个返回值是什么意思，调用方必须记住位置才能读懂
  `let (current, scope, pruning) = …`；`overlay.rs:56` 还直接丢掉第三个。同一 crate 的 `TreeDelta`
  用的是具名字段（好对照）。② `is_loaded` 的名字只说了一个条件，实现是
  `!has_pending && active.is_some()`，文档不得不用一整段解释「没有任何操作能让它变 true」——
  这正是 `BR-18` 里「每个调用都失败而 `is_loaded` 仍说 true」的阅读陷阱来源。
- **判据**：`[代码]`：`evidence.rs:46-55`；`lazy_wasm.rs:173-176`。
- **最小修复方向**：① 收成 `struct BuildEvidence { current, scope, pruning }`（或让三个读取各有一个入口），
  文档就不必再解释位置；② 名字改成它真正回答的问题（如 `is_serving` / `active_generation`），
  或把「无待定且已激活」这两个条件各给一个方法。
- **复核手段**：改后 `grep -rn "build_evidence" mcp/src` 的每个调用点读起来不需要查文档；
  `lazy_wasm` 的既有钉子（`fault_matrix.rs:139-160,191-233`）应保持绿。

### BR-C7 —— `apply()` 靠注释说明一组按约定成立的不变量

- **严重度**：MINOR（轴 5：结构该说的话被注释说了）　**类别**：注释承担了类型该承担的责任
- **file:line**：`mcp/apply.rs:94-127`（`work`/`applied` 双态 + 路径回写 + 消息字符串替换）、
  `mcp/apply.rs:137-150`（`report(&work, …, apply)`）
- **现象**：同一次调用里 `work` 有时是项目、有时是副本；成功分支要
  `outcome.source.strip_prefix(&work)` 再 `root.join(relative)`（`:116-118`），还要把执行器消息里的
  副本路径字符串**替换**成项目路径（`:122-125`），`report` 再按 `(moved, applied)` 选动词（`:518-523`）。
  这些不变量只靠注释与 `applied` 这个布尔值维持——读代码的人必须先读完这段注释才知道
  「`work != root` 时报告里的路径是被改写过的」。上一轮修过的两个缺陷（`:99-101` 的 `would write /tmp/...`、
  `:120-121`）都出在这层改写上。
- **判据**：`[代码]`：`:94-127` 的四处改写；`preview.rs:60-64` 的 `remove_copy` 也依赖同一个「`work` 可能是 root」
  约定（`work != root` 的守卫写在被调方，调用方看不见）。
- **最小修复方向**：把「预览还是落盘」做成一个类型（例如 `enum Target { Project, Copy(PathBuf) }`
  带 `report_path(&self, relative)` 与 `display_path(&self, path)`），改写集中在一处，
  注释就从「解释约定」变成「解释为什么要预览」。
- **复核手段**：`apply_tests.rs:102-136,330-389` 的「预览不得写成 `moved`/`would write /tmp/…`」两条钉子
  在重构后必须仍绿——它们正是这层改写的回归网。

**本轴查了、干净的部分**：① `grep -rniE "todo|fixme|xxx|still to come|暂时|以后再" mcp/src cli/src plugin-host/src`
在生产代码里 **0** 命中——没有注释掉的死代码、没有悬挂 TODO、没有「以后再改」式承诺；
② 逐块抽查的双语注释（`tools.rs`、`preview.rs`、`process.rs`、`wasm.rs`、`registry.rs` 的长块）两边说**同一件事**，
未发现「一边含糊一边具体」或只为过 `bilingual` 门禁而凑的第二语言——`mcp/tools.rs:4-36` 是其中的好例子
（英文行数多、中文行数少，但内容逐点对应）；③ 值得保留的「为什么/陷阱/实测」注释样板：
`plugin-host/src/wasm.rs:45-105`（表与元素段各为什么需要**另一道**上限，附实测字节数）、
`plugin-host/src/process.rs:247-294`（直白写法为什么是错的，附修前实测）、
`mcp/tools.rs:368-382`（行号回绕为什么会给出 `isError: false` 的错答案）、
`mcp/diff.rs:179-194`（删掉的桶为什么不可能出现）——这几处的密度（一个事实一段注释）是仓库该保持的标准。

---

## 查了、干净

以下项目逐条看过、未发现问题，写在这里以免下一轮重复劳动：

1. **工具目录与分派齐全**：catalog（`mcp/tools.rs:58-272`）与 `tool_call`（`:293-317`）逐名比对，
   17 个工具两处都在，没有「列了没接」或「接了没列」的臂。（顺序问题见 `BR-12`。）
2. **`register` 侧的转发纪律**：`xirang.verify` 只把 `limit`/`root` 转发给 `diff`（`verify.rs:71-77`），
   未声明的键（尤其 `records`）不再改变答案；与 `diff` 真正读取的键集合比对无遗漏。
3. **`confirm` 是真前置条件**：`delete` 在任何拷贝/写入之前就要求 `confirm: true`（`apply.rs:394-413`），
   且 schema 里声明的正是这个键（`tools.rs:109`）；`apply_tests.rs:397-431` 钉住「缺它即拒且文件未动」。
4. **预览不碰项目**：预览在一次性副本上跑真操作（`apply.rs:84-88`），`remove_copy` 有 `work != root` 守卫
   （`preview.rs:60-64`），`apply: true` 时 `work == root` 因此不会误删项目；`apply_tests.rs:102-136` 钉住。
5. **读路径的根约束**：`load_one`/`load_file`/`is_safe_child`（`index.rs:129-174`）都先 `canonicalize` 再比前缀，
   根外符号链接既不入索引也读不到（`index.rs:303-330` 的 Unix 钉子）；`.xirang` 回收目录不进索引
   （`index.rs:121-125` + `:227-252` 钉子），因此删掉的面不会从备份里继续作答。
6. **`xirang.read` 的行号上界**：中心先夹进文件、再用饱和运算（`tools.rs:376-382`），
   `u64::MAX`/`usize::MAX` 都不再回绕（`tools.rs:430-447` 钉子）。
7. **wasm 输入上限的「较小者」说法成立**：`check_input_length`（`wasm.rs:330-343`）只管帧宽与配置上限，
   真正的另一道是线性内存——wasmi 1.1.0 的 `Memory::write` 只做 `resolve_memory_mut` 后写入、越界返回 `Err`，
   **不会增长**（`~/.cargo/registry/src/*/wasmi-1.1.0/src/memory/mod.rs:230-241`），
   因此 `WasmLimits::max_input_bytes` 文档里「一页内存的插件拒绝 64 KiB 以上的输入」是准确的。
8. **wasm 输出与表/元素段**：输出先比长度再分配、再校验范围（`wasm.rs:374-391`）；表上限接进 store limiter、
   被动元素段在编译前手工量段头（`wasm.rs:163-190`）；`fault_matrix.rs` 有巨大表、4 GiB 初始内存、
   被动元素段与「小段仍可加载」的对照。
9. **准入与信任没有假实现**：checksum 是真校验（`core/.../artifact/artifact.rs:77-91` →
   `plugin/trust/trust.rs:88-132` 的 `verify_bytes`）；官方通道必须有信任根、撤销先于签名
   （`admission.rs:165-185`）；`Ed25519Verifier` 会拒绝「指纹与公钥字节对不上」的密钥
   （`verifier.rs:41-49`）；签名覆盖内核构造的完整载荷（含随字节同行的注册声明，
   `contracts.rs:488-495`），`admission.rs` 有四条端到端钉子（lock 未登记／已撤销／无信任根／被篡改的 `parent`）。
10. **`lane_for` 的说法与内核一致**：`lane_for`（`admission.rs:92-97`）把 `Digest` 映到 `Community`，对应内核里
    `Community | Local` 只接受 `PluginSource::User` 的规则（`core/.../slot/slot.rs:100-106`），
    且 `Official` 要求 `assurance == Signature`（`:92-99`）。
11. **进程适配器确实执行已验证字节**：`load` 要求文件长度与内容都等于已验证字节（`process.rs:151-184`），
    执行的是私有暂存副本（`tempfile` 排他创建 + `0o500`），因此之后改原路径不改变实际执行内容；
    stdout/stderr 全程排空、输入在独立线程写、deadline 覆盖整个调用（`:303-409`），
    `fault_matrix.rs` 有六条钉子（超时 vs 崩溃、大于管道缓冲的输出、写完继续写、超限输出、
    不读 stdin 的子进程、1 MiB 输入、stderr 洪泛、环境/工作目录三个旋钮）。
12. **只读面**：`xirang.grafts`、`xirang.registry`、`xirang.search` 等不写项目；唯一会写项目的是
    `xirang.apply`（默认预览），`xirang.verify` 只写 `target/xirang/out` 并在描述里声明了该副作用
    （`tools.rs:264-265`）。
13. **CLI 的失败语义**：两条二进制都把错误写 stderr 并以 1 退出（`cli/bin/xirang.rs:11-14`、
    `bin/cargo-xirang.rs:19-22`）；`check --json` 失败时**先**在 stdout 写出文档再 `Err`
    （`commands/check.rs:59-74`），`explain`/`grafts` 的解析失败同样先出文档
    （`explain.rs:66-72,84-88`、`grafts.rs:49-70`）——`--json` 的「stdout 恰好一份文档」契约成立。
14. **`--help` 与实现的子命令集合一致**：`USAGE`（`lib.rs:44-53`）八条与 `run_to`（`:139-152`）八条一一对应，
    与两份 CLI README 的命令表（英文）也一致（中文表的缺行见 `BR-15`）。
15. **无 `unsafe`、生产路径无 `unwrap/expect/panic`**：`grep -rn "unsafe\|\.unwrap()\|\.expect(\|panic!\|unreachable!"
    plugin-host/src mcp/src cli/src` 的命中全部落在 `#[cfg(test)]` 模块内（`tools.rs:433-447`、`index.rs:233-318`、
    `commands/new.rs:117-120`）；四处切片都有显式守卫：`callers[..CALLERS]`（`callgraph.rs:89-94`）、
    `faces[0]`（`nodes.rs:71-74`）、`buffer[..read.min(room)]`（`child.rs:111-113`）、
    `&bytes[..4]`（`wasm.rs:253-255`）。
16. **`mcp/lib.rs:19` 的「五个工具索引源码文本」**：`search`/`inspect`/`callgraph`/`read`/`status`
    正是全部经 `load_sources`/`load_one` 的五个，数目与行为一致。
17. **`mcp/tools.rs:4-10` 的自我克制**：catalog 的模块文档**有意不写工具数**（并记下了「曾数成五加八」的
    教训），因此这个历史老毛病在 `tools.rs` 内部已被关掉；本轮 `BR-1/2/3/7/8` 与 `BR-C3/C4` 都发生在
    「文字那一侧」（README、工具 `description`、doc 注释），而不是实现本身。

## 结构观察

1. **测试与源码的对应关系**：`mcp/src` 的 16 个 `<name>_tests.rs` 与源文件 1:1，但 `nodes.rs`（76 行）、
   `preview.rs`（180）、`tree_delta.rs`（120）、`converge_trace.rs`（256）**没有自己的测试模块**。
   其中最要紧的是 `tree_delta.rs`：它是 `diff` 与 `search` 共用的唯一判定规则（模块文档自己也这么说），
   却只被 `diff_tests.rs`/`search_tests.rs` 间接覆盖——`BR-6` 那类命名空间问题正好落在它的盲区里。
   `preview.rs` 的 `diff_package` 也没有直接钉子，而 `BR-4`（本报告最实质的一条）就在那里。
2. **`mcp/src` 的「一条规则一个地方」做得好，但有三处漏网**：`graft_plan_rows`、`face_views`、
   `build_output_is_current`、`OVERLAY_NOTE` 都做到了单点（并有注释说明为什么不复制），
   而 `tokens`/`blank`/`requires` 解析是三份/两份（`BR-10`）。方向已经确立，只差收口。
3. **两处最易漂移的文档**：`tools.rs` 的 `description` 与三份 README 的清单条目——本轮四条
   （`BR-1/2/7/8`）都在这两个位置，其中两条是上一轮修了代码没修文档；源码注释侧另有 `BR-C3/C4`。
   建议把「工具描述字段清单 == 实现」也做成一条可执行钉子（`BR-7`/`BR-8` 的复核手段），而不是靠人读。
4. **`cli/src` 的 `run_to` sink 只被一半命令使用**：`check`/`explain`/`grafts`/`studio` 写显式 sink，
   而 `build`（`commands/build.rs:28` 的 `println!`）、`new` 的打印（`cli/commands/new.rs:81`）、
   `snippets`（`commands/snippets.rs:70,87,113`）直接写进程 stdout。机器可读命令的契约没被破坏，
   但「同一个分发入口，一半走 sink」是阅读顺序上的分叉；`lib_tests.rs` 因此有 1015 行集中在单文件里
   （超过本仓 600 行棘轮的一半，只因为是测试文件才不受约束）。
5. **`plugin-host` 的 tests 与 src 不是 1:1**：`fault_matrix.rs`（968 行，wasm 与 process 两个特性门控模块同处一文件）、
   `admission.rs`、`wasm_table_cost.rs`、`process_load_cost.rs` 按「故障矩阵 / 成本实测」组织而不是按模块。
   这是有意的（成本类钉子需要整进程测量），但 968 行的单文件在改动 is_mutated 的 wasm 限额时要来回跳；
   若下一轮要动 wasm 限额，先按 `wasm_faults`/`process_faults` 拆成两个文件成本很低。
6. **`plugin-host/src` 的分层干净**：`wasm.rs`（上限与 ABI）、`lazy_wasm.rs` + `slot_state.rs`（代际状态机）、
   `process.rs` + `process/child.rs`（进程与 OS 细节）、`admission.rs`（锁→校验→通道）、`verifier.rs`（密码学）、
   `deployment.rs`（原子发布）五层边界清楚，且每层都把「为什么直白写法是错的」写在注释里
   （管道容量、ETXTBSY、被动元素段、无界分配）。本报告在 plugin-host 侧只留下 `BR-16..BR-18`/`BR-20` 四条 MINOR，
   安全边界本身没有发现「声称校验但实际没校验」的假实现。
