# 独立验证：B2 第二批三组（门禁形状 / 工具脚本 / 形状白名单）

- 验证者：kernel-auditor（t9，**非作者**：t1/t2/t4 的交付我一条未参与）
- 对象：t1（G-03/G-04/G-05/G-07/G-08 + 棘轮 rustfmt 归一化 + 四处注释准确性）、t2（G-09/G-10/G-11/G-12）、t4（N-1/N-2/N-3）
- 日期：2026-09-28（本检出，`--offline`）
- 源码：**真实树零改动**（本轮我只写本文件；所有变异在 `/tmp/nk-t9` 的检出副本里做）
- 手段：自写夹具驱动各门禁的**真实入口**（`doc_blocks::findings`、`size::oversized`、`naming::findings`、`release_version::findings`、`purity::findings`、`release_workflow::findings`/`workflow_findings`、`doc_anchors::findings`）+ 7 组变异回退 + 真工具脚本矩阵

## 0. 判定一览

| 条目 | 判定 | 一句话 |
| --- | --- | --- |
| G-03 围栏配对（doc_blocks） | **证实** | 同字符/不短于开围栏/其后无 info 才闭合；4 反例报出、8 正规形状静默；变异回旧规则 → 作者 2 条钉子红 |
| G-04 测试挂载拼法（size） | **部分证实** | 可见性与 `#[cfg(…)]` 表达式判定证实；**文档点名的 `mod  x ;` 拼法未实现**（F-2） |
| G-05 非发布成员版本要求 | **证实** | 写出来的版本被绑到发布线；我独立复现"别的检查都看不见它"这个洞 |
| G-07 naming 剥注释 | **证实** | 注释不算要求位置；另有邻域披露（F-3，转义引号下的 `package = "…"` 半边） |
| G-08 属性栈含文档注释 | **证实** | `#[cfg(any())]` + `/// …` + 条目 → 不报；无装饰的对照条目照报 |
| 棘轮 rustfmt 归一化 | **证实** | 三个方向都验：归一化 >600 报、归一化 ≤600 不报、问不出 rustfmt/无 edition 退回磁盘行数 |
| 注释准确性四处 | **3 证实 + 1 部分证实** | (b) 最深挂载链 6 文件/5 边我独立复算一致；(d) 删了 29，但同一句仍写"五个 crate"（F-4） |
| G-09 `--workspace-version` 独立模式 | **证实** | 真工具六种组合全 exit 2；单独 0.1.6/exit 0；`--help` 单独 76 行；变异去守卫 → `--workspace-version --check-table` exit 0 且表从未核对 |
| G-10 release-audit 符号审计不再静默为空 | **证实** | 自造 nm 失败 + 零符号两种夹具：TSV 记 `nm-failed`/`0`、unaudited.tsv 记原因、`2 of 3 …` exit 1；只剩好工件 exit 0 且收尾行可达；变异回 `|| :` 吞掉 → exit 0 且表里两个 0 |
| G-11 visual `--help` 截断 | **证实** | 39 行、头部 36 行注释逐行出现、`Why tmux`/`是面板的纯文本` 各 1；变异回 `sed -n '2,26p'` → 28 行、两短语 0 |
| G-12 GNU 专属写法 | **证实（静态 + 定向仿真）** | `sed -i`/`date +%s%3N` 全 tools/ 0 命中；带探测的 `%s%N` 在伪造 BSD `date` 下 exit 0 且 `startup_ms` 仍是数字，旧写法在同一仿真下 `value too great for base`；rehearsal 改临时文件 + `mv` 且转义 `\ & \|`。**完整 rehearsal 未独立复跑**（见 §9） |
| N-1 tag 守卫形状白名单 | **证实** | 5 种白名单外形状 + 无 `if:` 全报；4 种肯定式 + 折叠式全收；现有 `ci.yml`/`release.yml` 仍 0 发现（原因见 §4.1）；变异回子串规则 → 作者 3 条钉子 + 我的探针红 |
| N-2 `takes_input` 结构化判定 | **证实** | `inputs['name']` 与 `inputs.name` 都算读、注释不算；变异回 `contains("inputs.")` → 两条红 |
| N-3 doc_anchors 扩面 | **部分证实** | 行为证实（清单/工作流纳入、`.md` 与无扩展名仍在覆盖外、真实树 0 发现）；**"52 处"这个数字复现不出**（F-1：门禁实际口径里今天只有 1 处） |

## 1. 装置与手段

- **副本**：`rsync -a --exclude target --exclude .git /home/nich/Moirai_N3/nichlink/ /tmp/nk-t9/`（41 MB）。所有变异、夹具、探针只存在于副本；真实树全程只读（唯一写入是本文件）。
- **我的夹具**：`/tmp/nk-t9/conventions/tests/t9_probe.rs`（集成测试 = crate 之外的独立 crate，只能取公开面），19 条用例，逐条自建 synth 根（`Cargo.toml` + 成员目录 + 夹具文件），驱动上面那组真实入口。**没有复用作者任何测试或夹具**；期望值由我自己按 CommonMark / rustc / GitHub 语义推出。
- **变异**：7 组，各把修复改回旧行为（旧行为取自 t1/t2/t4 自述 + 代码注释里的"旧形态"），跑 `cargo test -p xirang-conventions --offline`（作者钉子）与 `--test t9_probe`（我的夹具）两遍，用**文件 sha256 前后对比**证明还原零残留。为避免陈旧产物，变异后 `touch` 相关文件再跑（第二轮的两个代表性变异还额外取了断言原文，且 hash 稳定那次的结果才计入）。
- **真工具矩阵**：G-09/G-11/G-12 用**真实脚本 + 真实树**跑（只读调用）；G-10 用自建 harness `/tmp/nk-t9-g10`（脚本副本 + 假 `cargo`（exit 0）+ 真 `nm` + 三个自造工件）。

**探针自身的 bug（照实披露，均不影响结论）**
1. 第一版 G-09 变异跑在错误的 cwd（脚本按自身路径推 root，副本没有 `Cargo.toml`）→ awk 报错，不作证据；改用同根对照后重跑。
2. 第一版伪造 BSD `date` 用 `exec /usr/bin/date`，而本机（NixOS）没有 `/usr/bin/date` → 回退分支拿到空串、报算术错；那是**我的假 date** 的 bug，不是工具的。改为委托 `command -v date` 的真路径后，同一仿真 exit 0、`startup_ms=0`（数字）。
3. 我第一版工作流解析脚本不跳注释行，于是把 ci.yml 里那句提到 `--publish` 的**注释**当成了上传步骤；门禁自己跳注释（`steps()` 里 `trimmed.starts_with('#')` 直接跳过），所以那次"发现"是我的解析器造的，不作证据。
4. **共享 `target/` 的陈旧产物泄漏（不是我的探针 bug，但会让门禁结果假红）**：第一轮真实树门禁里 `cargo test --workspace` 红在 `core/tests/plugin_lock_provenance.rs` 两条 X-1 钉子上，报错是 `PluginLockError { line: 1, message: "has an empty provenance column" }`——而这句话在真实树 `grep -rn` **0 命中**，只存在于 t11 的变异副本 `/tmp/t11-mut/core/…/catalog.rs` 与 `/tmp/t11-mut2/…`（他们把 X-1 改成"拒绝"语义做的反证）。判别法照队长给的两条：① 两次日志的测试二进制不同（红那次 `plugin_lock_provenance-9efa651de66d2c42`，单跑那次 `plugin_lock_provenance-c8adfee13b15a41c`）⇒ 红那次链的是别人变异构建出的 `libxirang` 产物；② 那句错误串在真实树 0 命中。**该次运行作废**，判定改用独立 `CARGO_TARGET_DIR=/tmp/t9-target` 的重量（§7）。

## 2. 组 A：t1

### 2.1 G-03 围栏配对 —— 证实

夹具（12 个根级 markdown，各自一个形状）：

```
zz-md-valid.md            ```rust + 合法代码                       → 静默（正规）
zz-md-four-invalid.md     ````rust + 非法代码 + ````                → 报（旧规则：info 读成 `rust，整块从未打开）
zz-md-rs-tag.md           ```rs + 非法代码                          → 报（新旧都报）
zz-md-tilde-invalid.md    ~~~rust + 非法代码 + ~~~                  → 报（旧规则：~~~ 不是围栏，整块不可见）
zz-md-tilde-holds-backtick.md  ~~~rust 内含一行 "```"               → 静默（正规）
zz-md-four-holds-three.md      ````rust 内含一行 ```（缩进的 let）  → 静默（正规）
zz-md-unclosed.md         ```rust + 非法代码，文件到此结束          → 报 "unterminated Rust fence"
zz-md-text-invalid.md     ```text + 非法 Rust                       → 静默（正规）
zz-md-macro-input.md      ```rust,macro-input + 非 Rust             → 静默（正规，逃逸口）
zz-md-indented.md         4 空格缩进的非法 Rust                     → 静默（正规，缩进代码块不在覆盖内）
zz-md-inline.md           行内 `std::fs` 与 ``a `b` c``             → 静默（正规）
```

绿：`cargo test -p xirang-conventions --offline --test t9_probe` 全绿（`markdown_fences_pair_by_character_and_length` 断言恰好命中上述 4 个文件，`unclosed` 那条的 error 含 `unterminated`）。另一条 `doc_comment_fences_pair_the_same_way`：`///` 注释里同时放 ```rust（合法）、~~~rust（非法）、```text（非法），断言**只报 1 条且行号=7**（tilde 开围栏那一行）——即文档注释半边与 markdown 半边同一套配对。

红（M1，变异 = 旧规则：`starts_with("```")` 前缀 + `trim_start_matches` 取 info + 任何围栏行都闭合）：

```
thread 'doc_blocks::doc_blocks_tests::a_fence_is_paired_by_its_own_character_and_length' panicked at conventions/src/doc_blocks_tests.rs:211:5:
assertion `left == right` failed: the tilde fence is not closed by a backtick line, and a four-backtick fence is still a Rust fence: []
  left: []
 right: [3, 9]
thread 'doc_blocks::doc_blocks_tests::a_sanctioned_tag_and_every_fence_spelling_are_read' ... FAILED
thread 'doc_comment_fences_pair_the_same_way' panicked at <副本>/conventions/tests/t9_probe.rs:140:5: left: 0  right: 1
thread 'markdown_fences_pair_by_character_and_length' ... FAILED      （我的夹具同向红）
```

还原后 `zero-residue: True`（sha256）。

**边界（我顺手发现的，非本次修复引入）**：非 Rust 围栏不被跟踪，因此 `~~~text` 块里的一行 ```rust 会被读成顶层 Rust 围栏，并以 "unterminated" 报出——`a_rust_fence_nested_inside_a_text_fence_is_read_as_top_level` 把这条观察钉住（1 条发现）。本仓没有任何活文档这样嵌套围栏（真实树 0 发现），故记为边界而不是缺陷。

### 2.2 G-04 测试挂载拼法 —— 部分证实

夹具（一个合成工作区 7 个成员，每个 610 行的被挂载文件）：

| 成员 | 声明 | 今天的结果 | 应当 |
| --- | --- | --- | --- |
| `exempt_all` | `#[cfg(all(test))]` + `mod big_tests;` | 豁免 | 豁免 ✓ |
| `exempt_visibility` | `#[cfg(test)]` + `pub(crate) mod big_tests;` | 豁免 | 豁免 ✓ |
| `exempt_spaced` | `#[cfg(test)]` + `mod   big_tests ;` | **被度量（报 610）** | 豁免 ✗（F-2） |
| `measured_not` | `#[cfg(not(test))]` | 报 610 | 报 ✓ |
| `measured_any` | `#[cfg(any(test, feature = "x"))]` | 报 610 | 报 ✓ |
| `measured_plain` | 610 行的 `src/lib.rs`（无挂载） | 报 610 | 报 ✓ |
| `under_ceiling` | 590 行 | 静默 | 静默 ✓ |

红（M2，变异 = 字面比较 `mod x;`/`pub mod x;` + `starts_with("#[cfg(test)")`）：作者两条钉子红（`every_spelling_of_a_test_mount_is_exempt`、`a_negative_or_conditional_cfg_is_not_a_test_mount`），我的夹具 `test_mount_spellings_and_cfg_expressions` 也红；还原零残留。

**F-2**：`conventions/src/size.rs:216` 的注释写"`mod  x ;` 是 rustc 眼中的同一条声明"，但 `declares_module`（同文件 :204）只把**空白串**归一化（`split_whitespace().join(" ")`），`;` 前那个空格让整串对不上 → 该拼法仍被度量。树里今天没有这种拼法（`grep -rnE "mod +[A-Za-z_]+ +;"` 只命中这句注释本身），所以只是"自述 vs 行为"不一致：要么比较前把 `;` 前的空白也吃掉，要么把那半句注释删掉。严重度 low（无现实影响）。

### 2.3 G-05 非发布成员的版本要求 —— 证实

夹具（根 `[workspace.package] version = "0.1.6"`）：已发布成员写 0.1.6 → 静默；`publish = false` 成员写 `xirang-run-method = "0.1.5"` → **1 条**（`…/unpublished/Cargo.toml`，理由含 0.1.5）；`publish = false` 成员只写 path 依赖（无版本）→ 静默。

**独立复现"只有这条门禁看得见它"**：在副本里把 `examples/control-button/Cargo.toml` 的 `version = "0.1.6"` 改成 `0.1.5`（无 git 操作，改完逐字节还原）后：

```
$ tools/xirang-publish --check-table ; echo exit=$?
dependency table matches the manifests (9 crates)
exit=0
$ cargo test -p xirang-conventions --offline --lib release_version::release_version_tests::the_shipped_manifests
test ...the_shipped_manifests_name_one_version ... FAILED      （panicked at conventions/src/release_version_tests.rs:168）
```

变异（M3，变异 = 非发布成员直接 `continue`）：作者的 `an_unpublished_members_stale_requirement_is_reported` 红，我的夹具同向红；还原零残留。

### 2.4 G-07 naming 剥注释 —— 证实（含邻域披露 F-3）

夹具：`build_method/scaffold/probe.rs` 里的 `// … xirang-linecomment = …`、`/* … xirang-blockcomment = … */`、以及**字符串字面量里**的 `xirang-ghost = {{ package = \"xirang-elsewhere\" }}`；`.github/workflows/ci.yml` 里的注释 `# cargo test -p xirang-commented is history` 与两条真要求（`-p xirang-ghost`、`--package=xirang-elsewhere`）。

绿：恰好 3 条发现（模板键名 1 + 工作流 2），三条注释一条都没报。红（M4，变异 = 原文不剥注释）：作者的 `a_name_in_a_comment_is_not_a_requirement` 红（3 → 我这边会多出注释里的名字），我的夹具同向红；还原零残留。

**F-3（前置邻域，非 G-07 引入）**：`requirement_names`（`conventions/src/naming.rs:328`）匹配的是未转义的 `package = "`，而真实模板写的是转义引号（`build_method/scaffold/project.rs:115`：`package = \"xirang-run-method\"`）——那半边在真实拼法下不匹配。今天被同一个名字的**依赖键**掩盖（键名与包名相同，:115 两处都如此），所以真实树 0 发现不受影响；但一条**重命名**的内部依赖（`kernel = { package = "xirang-core" }`）写在模板字符串里就是隐形的。我的两条夹具分别演示了：未转义（raw string）→ 报；转义 → 不报。严重度 low–medium，属 G-07 边界之外，供挂账决策。

### 2.5 G-08 属性栈含文档注释 —— 证实

夹具（合成 `core/src/`）：`#[cfg(any())]` + `/// A doc comment between the attribute and its item.` + `use std::fs;` → **不报**；同一根里 `core/src/control.rs` 的 `/// …` + `use std::env;`（无属性装饰）→ **报 1 条**（证明门禁本身没被改坏）。

红（M5，变异 = `belongs_to_the_attribute_stack` 回成"只认 `#`"）：作者的 `tests::a_doc_comment_between_an_attribute_and_its_item_is_governed_too` 红、我的 `a_doc_comment_between_an_attribute_and_its_item_is_governed` 红。

**M5 的 residue 说明（并发窗口）**：首轮 M5 还原后 sha256 报 `False`——原因是**真实树的 `conventions/src/lib.rs` 在我窗口内被改动**（mtime 19:38:46，t1 补第二条同族钉子），我的还原脚本从真实树拷回，于是副本拿到的是新内容，与"变异前"的旧内容不同。这不是我的残留：重新 `rsync` 后副本与真实树逐文件同 hash，且第二轮（探针）变异 M5 报 `zero-residue=True`。

### 2.6 棘轮 rustfmt 归一化 —— 证实（三个方向，按队长要求）

- **归一化后 >600 → 报**：1 行内 700 条语句（磁盘 1 行，rustfmt 后 ~702 行）→ `oversized` 报 1 条且计数 >600（`the_ratchet_counts_the_rustfmt_normalized_file`）。
- **未格式化、但归一化后 ≤600 → 不报**：① 1 行内 300 条语句（磁盘 1 行，rustfmt 后 304 行）；② 400 个空函数体（磁盘 800 行，rustfmt 折成 ~402 行）——两者都静默（`an_unformatted_file_that_formats_under_the_ceiling_is_not_reported`）。第二条同时证明归一化**不是单向**的：它也会把行数降下来，量的是"本仓库要求落在磁盘上的那个文件"。
- **问不出 rustfmt → 退回磁盘行数（旧方向）**：① 根清单不写 edition（`workspace_edition` 问不出）→ 同一个 800 磁盘行的文件**被报，计数 800**（`a_root_without_an_edition_measures_the_disk_file`）；② 在 `PATH` 最前面放一个**总是失败的 rustfmt 桩**（`--version` 也失败）→ `the_measurement_follows_the_rustfmt_probe_when_it_can_be_asked` 走 else 分支，同样报 800 行；去掉桩（真 rustfmt 可用）同一条用例走 if 分支，`oversized = []`。
- **反过火旁证**：`cargo fmt --all -- --check` 在本树 exit 0（另见 §7），即归一化对本树是恒等映射，不会因为"未格式化"新增任何发现。

### 2.7 注释准确性四处

- (a) **BASELINE 不复述项数**：`conventions/src/size.rs:73` 只写 `[BASELINE.len()]`，无数字；`docs/roadmap-1.0.md:40` 同样把数字删掉并留了更正说明。**证实**。
- (b) **最深挂载链**：我用自写脚本（解析 `#[path = "…"]` 跟随的 `mod` 与裸 `mod x;`，忽略 target/）独立复算整棵树的挂载深度：**最深 6 个文件 / 5 条边**，链正是 `core/src/lib.rs → core/src/registry_core.rs → core/src/registry_core/authoring/authoring.rs → core/src/registry_core/authoring/parse/parse.rs → …/parse/flow.rs → …/parse/flow_tests.rs`；第二深 5 文件（studio），第三 5 文件（run_method）。与 `conventions/src/size.rs:134` 起的文字逐条一致。**证实**。
- (c) **lint 边界**：`conventions/src/lint.rs:15` 的段落与 `required_roots` 的函数文档（同文件 :32 起）说的是同一条边界（除示例宿主外每个 crate 目录的库根，含 `conventions` 自己，外加上午带该属性的那一个二进制根）。**证实**。
- (d) **mounting 的数字**：`conventions/src/mounting.rs:10` 的 "29 处声明" 已删，改为"这类声明很常见 + 计数不复述"。但**同一句仍写 "churning five crates"（五个 crate）**：我用两种口径都复现不出 5——(i) 出现裸 `mod x;` 的 crate 目录 = **10** 个；(ii) 严格口径（裸 `mod x;` 出现在既非 crate 根、也非 `#[path]` 载入的父文件里）= **2** 个 crate（studio、plugin-host）。该短语是**修前就有的**（`git diff` 的 -/+ 两侧都在），不是本次引入；但既然这半句正在被编辑，"五个" 要么给出测量口径与日期，要么一起删。**部分证实** + F-4。

## 3. 组 B：t2

### 3.1 G-09 `--workspace-version` 独立模式 —— 证实（真实脚本、真实树）

```
--workspace-version                        exit=0  0.1.6
--workspace-version --check-table          exit=2  error: --workspace-version is a mode of its own; it does not combine with …
--workspace-version --verify-consumers     exit=2  （同上）
--workspace-version --publish --yes        exit=2  （同上）
--workspace-version --yes                  exit=2  （同上）
--workspace-version --allow-dirty          exit=2  （同上）
--workspace-version --help                 exit=2  （同上）
--check-table --verify-consumers           exit=2  error: --check-table and --verify-consumers are separate modes
--bogus                                    exit=2  error: unknown argument '--bogus' (see --help)
--help 单独                                  exit=0，输出 76 行
--check-table 单独                          exit=0  dependency table matches the manifests (9 crates)
```

红（变异 = 删掉 `tools/xirang-publish:134` 起的互斥守卫，在同一个 scratch 根里对照跑）：变异体 `--workspace-version --check-table` → **exit 0 打印 0.1.6**（表从未核对）、`--workspace-version --publish --yes` → exit 0；同根未变异副本分别 exit 2。变异只在副本里，真实脚本的 md5 未变。

### 3.2 G-10 release-audit 符号审计 —— 证实（队长点名的那半，我自建夹具）

harness：脚本副本放在 `<root>/tools/`，`<root>` 只有 `target/release/` 与一个假 `cargo`（exit 0），`nm` 是真的。三个自造工件：`zz-t9-good`（cc 编译，24 个已定义符号）、`zz-t9-nosymbols`（cc 编译的、无已定义符号的 .o → `nm --defined-only` exit 0 且 0 行）、`zz-t9-nonobject`（带执行位的文本 → `nm` "file format not recognized"）。

绿（修复后）：

```
warning: nm could not audit target/release/zz-t9-nonobject (nm: … file format not recognized); it is recorded as unaudited
warning: nm found no defined symbols in target/release/zz-t9-nosymbols; it is recorded as unaudited
error: 2 of 3 release artifact(s) could not be audited for symbols:
exit=1
artifacts.tsv: zz-t9-nonobject 19 nm-failed / zz-t9-nosymbols 920 0 / zz-t9-good 15744 24
unaudited.tsv: 逐条记原因（nonobject 那句 nm 原文、nosymbols "nm reported no defined symbols"）
只剩好工件：exit=0，`symbol_audit=… (1 artifacts, each with defined symbols)`
```

红（变异 = 回旧行为：删掉"零符号记 unaudited"与"nm 失败记原因"两个分支、再删掉末尾的 unaudited 判定 → 即旧的 `|| :` 吞掉 + 写 0）：

```
mutated exit=0
artifacts.tsv: zz-t9-nonobject 19 0 / zz-t9-nosymbols 920 0 / zz-t9-good 15744 24
symbol_audit=… (1 artifacts, each with defined symbols)      ← 两个从未被审计的工件被读成干净结果
```

这与 t2 自述的旧行为一致；我的夹具（要求 exit 1 且 TSV 记 `nm-failed`）在变异下必然红。t2 报的第三种情形（`nm` 整个缺失）我没有另造 mini PATH，属未覆盖（见 §9）。

### 3.3 G-11 visual `--help` —— 证实

```
$ tools/xirang-visual --help ; echo exit=$?
exit=0   39 行
grep -c 'Why tmux' → 1        grep -c '是面板的纯文本' → 1
头部注释块 36 行（去掉行首 `# ` 后）逐行出现在输出里：缺失 0 行
```

红（变异 = 换回写死的 `sed -n '2,26p'`，`tools/xirang-visual:125`）：输出 **28 行**，`Why tmux` 0、`是面板的纯文本` 0，句子在 `target/visual` 那句中途截断。变异在副本里，真实脚本未动。

### 3.4 G-12 GNU 专属写法 —— 证实（静态 + 定向仿真），完整 rehearsal 未复跑

- 静态：`grep -rn "sed -i" tools/` → 0；`grep -rn "date +%s%3N" tools/` → 0；`for s in tools/*; do sh -n $s; done` → 全部解析通过。
- `date +%s%N` 只剩一处（`tools/xirang-release-audit:142`），且是**带探测的**形式（`ns=$(date +%s%N 2>/dev/null) || ns=` + `case '' | *[!0-9]*)` 回退整秒）。我在 G-10 harness 里用伪造 BSD `date`（`%s%N` 打印字面 `1727000000N`）跑整条工具：**exit 0、`startup_ms=0`（仍是数字）**；把同一条旧写法单独放到同一仿真下：`sh: 1727000000N: value too great for base (error token is …)`——正是 t2 描述的旧失败模式。
- rehearsal：`tools/xirang-external-rehearsal:79`–`:81` 用 `sed … > "$manifest.repointed"` + `mv`，并对 `ROOT` 转义 `\`、`&`、`|`（`sed 's/\\/\\\\/g; s/&/\\&/g; s/|/\\|/g'`）；`sed -i` 0 命中。**我没有重跑整条 external rehearsal**（它会在检出之外从零构建示例宿主，代价大且与 t2 的仿真重复）。

## 4. 组 C：t4

### 4.1 N-1 tag 守卫形状白名单 —— 证实（含"为什么不误杀"）

绿：`release_workflow::findings(text)`（纯文本入口，我自己写 YAML）对 4 种肯定式全静默：tag 判断单独；`github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')`；带括号 + `${{ }}` 外壳；`github.repository == '…' && startsWith(…)`。另加**折叠式**（`if: >-` 跨两行）静默——这条同时钉住 `steps()` 的折叠边界（折叠止于 `if:` 键自身缩进，`run:` 不会被读进条件）。

红：5 种白名单外形状 + 无 `if:` 全报：

```
negated call:            ${{ !startsWith(github.ref, 'refs/tags/') }}          → 1 条
spaced negation:         ! startsWith(github.ref, 'refs/tags/')               → 1 条
disjunction:             startsWith(…) || github.event_name == 'push'          → 1 条
inequality:              github.ref != 'refs/heads/main'                      → 1 条
unenumerated operator:   github.run_number > 1                                → 1 条
no condition:            上传步骤没有 if:                                      → 1 条
```

变异（M6，变异 = 回 `condition.contains(TAG_GUARD)` 子串规则 + 回 `line.contains("inputs.")`）：

```
thread 'release_workflow::release_workflow_tests::a_negated_tag_test_is_reported' panicked at …/release_workflow_tests.rs:55:5:
a negated guard publishes on branches: []
thread '…both_spellings_of_a_negated_tag_test_are_reported' panicked at …:178:5:
a negated guard publishes on branches:
a negated call: []
a spaced negation: []
thread '…shapes_off_the_whitelist_are_reported' panicked at …:219:5:
a shape off the whitelist is reported:
a disjunction: [] / an inequality: [] / an operator the gate does not enumerate: []
+ 我的探针 guards_off_the_shape_whitelist_are_reported、input_reads_are_read_in_both_spellings_and_not_in_prose 红
```

**反过火：现有 `ci.yml`/`release.yml` 仍绿（真实树 0 发现），为什么**：`ci.yml` 里**没有任何上传步骤**——它跑的是 `tools/xirang-publish --check-table`（:137）与不带 `--publish` 的 `tools/xirang-publish`（:144），"提到 `--publish`"的是它们上面的**注释**，而 `steps()`（`conventions/src/release_workflow.rs:523`）把 `#` 开头的行直接跳过，注释因此读不成上传步骤；`release.yml` 的三个上传/上传后步骤写的就是白名单的两种形状：`:66` 是 tag 判断单独，`:122`、`:136` 是 `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')`。**未改任何工作流写法**。

### 4.2 N-2 `takes_input` 结构化判定 —— 证实

`inputs['publish']`（括号拼法）与 `inputs.publish`（点号拼法）都产生 "reads an input"；把同一行写成注释 `# this step never reads inputs.publish` → 0 发现。变异（M6 的第二半，回 `line.contains("inputs.")`）：作者的 `a_bracketed_input_reference_is_reported` 与 `an_input_reference_is_read_in_either_spelling_and_not_in_prose` 都红（前者 0 发现、后者把注释报出来），我的探针同向红。

### 4.3 N-3 doc_anchors 扩面 —— 部分证实（行为证实，数字复现不出）

绿：合成根里同一条 md 里放 9 个锚点——`probe/src/lib.rs:1`、`Cargo.toml:3`、`.github/workflows/ci.yml:2` 解析成功；`probe/src/lib.rs:99`、`Cargo.toml:999`、`.github/workflows/ci.yml:99` 被报；`README.md:1` 与 `tools/xirang-publish:88`（`.md` 目标与无扩展名脚本）**不报**（与模块头写的代价一致）。`doc_anchors::findings` 恰好返回那 3 条失效锚点。

真实树：`doc_anchors::findings(workspace_root())` **0 发现**。我另外把门禁**实际读到的文件集**独立重算了一遍（`markdown_files` = 根级 `*.md` + `docs/**` 去掉 `audit*`/`design*` + 各成员的两份 README，合计 **33** 份）：其中指向清单/工作流的候选锚点今天只有 **1 处**（`.toml` 1、`.yml/.yaml` 0）；把 record 文档也算上（门禁**不读**它们）则是 124 处（85 `.toml` + 39 `.yml/.yaml`）。

**F-1**：t4 报的"扩面后新被检查的非 Rust 锚点 **52 处**"复现不出——它既不是门禁实际口径（33 份文件里 1 处），也不是全 `docs/` 树（124 处，其中 104 处在本轮报告 `docs/audit-2026-09-28/*.md` 里，多份 mtime 18:40 / 19:05）。最可能的来源是"把 record 文档也数进去了"——而 `doc_anchors` 明确**豁免** record（见 `an_anchor_inside_a_record_is_exempt`，以及 `doc_blocks::markdown_files` 末尾的 `retain(… && !is_record(path))`），那个数只会随每一份新审计报告继续涨。实质结论（**全部可解析、0 发现、无需改任何文档**）成立；建议报告里改写成"覆盖面（后缀与来源：`.rs/.toml/.yml/.yaml`，根与成员清单、`.github/workflows/*`，record 豁免）"而不给计数。严重度 low（数字，不是行为）。

## 5. 反过火（正规样本的误报计数）

| 门禁 | 我跑的正规样本 | 误报 |
| --- | --- | --- |
| G-03 | 12 个 markdown 形状中 8 个正规 + 1 个 `///` 注释夹具 + 真实树全部活文档 | 0 |
| G-04 | 3 种豁免拼法 + 1 个 590 行文件 + 2 个"归一化后 ≤600"的未格式化文件 + 真实树（只报已钉的 1 条欠账） | **1**（F-2：`mod  x ;`，且它与注释承诺冲突） |
| G-05 | 已发布成员 0.1.6 + 非发布成员省略版本 + 非发布成员自定 `[package] version` + 真实树（两个示例） | 0 |
| G-07 | 3 处注释（行注释/块注释/YAML 注释）+ 真实树 | 0 |
| G-08 | 1 个被装饰条目 + 1 个无装饰对照 + 真实树 core/src | 0 |
| N-1 | 4 种肯定式 + 折叠式 + 现有两个工作流 | 0 |

## 6. 变异表（红侧 + 还原零残留）

| 变异 | 作者钉子红 | 我的探针红 | 零残留 |
| --- | --- | --- | --- |
| M1 doc_blocks 围栏配对 → 前缀子串 + 任何围栏闭合 | 2（含 `[3,9]` vs `[]` 原文） | 2 | ✓ |
| M2 size 挂载拼法 → 字面比较 + `starts_with("#[cfg(test)")` | 2 | 1 | ✓ |
| M3 release_version → 非发布成员 `continue` | 1 | 1 | ✓ |
| M4 naming → 不剥注释 | 1 | 1 | ✓ |
| M5 lib 属性栈 → 只认 `#` | 1 | 1 | ✓（首轮 hash 不符系真实树被并发修改，见 §2.5；第二轮 ✓） |
| M6 release_workflow → 子串规则 + `contains("inputs.")` | 3 | 2 | ✓ |
| M7 doc_anchors → `READ_EXTENSIONS = &[".rs"]` | 1 | 1 | ✓ |

工具脚本侧的变异（不在 cargo 里）：G-09 去互斥守卫 → 组合 exit 0；G-10 回旧吞掉 → exit 0 且表里两个 0；G-11 回写死行号 → 28 行、两短语消失。三处都只在副本/临时目录，真实脚本与真实树未动。

## 7. 五条门禁（哈希钉住）

跑前先把 12 个相关文件（`conventions/src/{lib,doc_blocks,size,naming,release_version,release_workflow,doc_anchors}.rs`、`tools/{xirang-publish,xirang-release-audit,xirang-visual,xirang-external-rehearsal}`、`AGENTS.md`）sha256 存到 `/tmp/t9-pin-before.txt`，跑完 `sha256sum -c` **全部 OK**（零漂移）：

| 命令 | 结果 |
| --- | --- |
| `cargo test -p xirang-conventions --offline` | exit 0；`113 passed; 0 failed` |
| `cargo test --workspace --offline` | exit 0；55 个 test 目标全 ok、0 FAILED |
| `cargo fmt --all -- --check` | exit 0；零输出 |
| `tools/xirang-publish --check-table` | exit 0；`dependency table matches the manifests (9 crates)` |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | exit 0 |

**并发窗口的处理（按队长要求：别人跑出的结果作废重跑）**

- 上面这一轮（19:4x）是**第一次**全绿：`/tmp/t9-gate-b.log` 里 55 个 `test result: ok`、0 条 FAILED，且 `new_project_and_explicit_root_face_compile ... ok` 在列。12 个文件 sha256 跑前=跑后。
- 之后重跑同一命令时红过一次，红点是 `core/tests/plugin_lock_provenance.rs`，错误串在真实树 0 命中、只存在于 t11 的变异副本 ⇒ **共享 `target/` 泄漏的变异产物**（§1 第 4 条），该次**作废**。
- 再后来用独立 `CARGO_TARGET_DIR=/tmp/t9-target` 重跑：`cargo test -p xirang-conventions --offline` exit 0、`cargo clippy` exit 0；`cargo test --workspace --offline` 只红在 `studio::app::tests::project::new_project_and_explicit_root_face_compile`——生成的项目 `cargo check --offline` 解析 `xirang-run-method = "^0.1.6"` 时，本地 cargo git 缓存里只有 0.1.5，这正是队长报备的离线缓存环境artifact（副本/冷 target 下红、真实树绿），**不是回归**。
- 最后在共享 `target/` 上 `touch core/src/registry_core/plugin/catalog/catalog.rs`（内容不变，sha256 仍 `89048325…`）强制 core 重建后重跑：`cargo test --workspace --offline` **EXIT=0、55 个 test 目标全 ok、0 FAILED**（`/tmp/t9-g-b3.log`）。**判定取这一轮**。
- 我自己的变异全程跑在 `/tmp/nk-t9`（自带 `/tmp/nk-t9/target`，未设 `CARGO_TARGET_DIR`），因此从未写进真实树的 `target/`；泄漏来自别人的副本。

## 8. 发现清单

| id | 严重度 | 位置 | 问题 | 建议 |
| --- | --- | --- | --- | --- |
| F-1 | low | t4 交付文本 / `conventions/src/doc_anchors.rs:166` | "52 处新锚点"复现不出；门禁实际口径（33 份非 record markdown）里今天只有 1 处，全 `docs/` 树是 124 处（104 处在本轮报告里，且 record 本就被豁免） | 报告只写覆盖面与来源，不给计数；或说明口径与日期 |
| F-2 | low | `conventions/src/size.rs:216`（注释）与 `:204`（`declares_module`） | 注释称 `mod  x ;` 与 `mod  x;` 同义，实现只归一化空白串 → 该拼法仍被度量（我的夹具实证 610 行被报） | 比较前把 `;` 前的空白也吃掉，或删掉注释里的那个拼法 |
| F-3 | low–medium | `conventions/src/naming.rs:328` vs `build_method/scaffold/project.rs:115` | `package = "…"` 半边匹配未转义引号，真实模板是转义引号 → 重命名的内部依赖写在模板字符串里就是隐形的（今天被同名依赖键掩盖） | 判定改在剥转义的文本上做，或补一条"重命名 + 转义引号"的钉子 |
| F-4 | low | `conventions/src/mounting.rs:10` | 同句仍写 "churning five crates"；我两种口径分别量到 10 与 2 个 crate | 给出测量口径与日期，或与 29 一起删掉 |

另有两条**观察**（不算缺陷）：① 非 Rust 围栏不被跟踪，`~~~text` 内的 ```rust 会被读成顶层围栏并按未闭合报出（§2.1 边界；真实树无此形状，且 record 文档本就不参与围栏解析）；② `doc_anchors` 的 record 豁免与 `doc_blocks` 同源（`markdown_files` 末尾 `retain(… && !is_record(path))`）——本文件自己（`audit-*.md`）因此也不参与门禁，这也是 F-1 那个数字只在 record 文档里增长的机制。

## 9. 未覆盖范围

1. **`tools/xirang-external-rehearsal` 的完整 BSD 仿真复跑**：只做了静态复核（无 `sed -i`、临时文件 + `mv`、转义 `\ & |`、`sh -n` 通过）+ `date` 回退的定向仿真。
2. **G-10 的 `nm` 整个缺失**（mini PATH）与 **`readelf` / `.inventory` 分支**、`XIRANG_FULL_BINARY`/`XIRANG_MINIMAL_BINARY` 分支（t2 自述的邻域缺口）我没有另造夹具——只在三种工件（好/零符号/nonobject）上验证。
3. **t1 的棘轮性能**（is_mounted_as_test 从 ~20s 回到 ~5s）我没有计时对比。
4. **其它成员的并发编辑**：窗口内真实树 `conventions/src/lib.rs` 被改过一次（19:38:46，补 G-08 第二条钉子）。§7 的门禁全部跑在"哈希钉住"的那份树上，但 §2 里 M5 首轮的 hash 比对受这次并发影响（已在 §2.5 说明并按第二轮结果判定）。
5. 我只验证了三组修复本身；**没有**复核 t1/t2/t4 自述里那些"顺带修掉"的邻域（例如 t4 的 `AGENTS.md` 描述、t1 的 `examples` 逐字节断言），那些不在本任务 acceptance 内。
