# 审计 E — 门禁与工具（conventions / tools / .github / examples / 双语文档）

审计对象：工作树 HEAD `cf0c378`（120 项未提交改动），本轮只出报告，源码一行未改。
写权限：仅 `docs/audit-2026-09-28/audit-lane-gates.md`（按队长广播改名，旧名 `lane-gates.md` 已 `mv` 掉，目录内不留非 `audit-` 前缀的本人产物）。
本人亲自跑过、用作本报告基线的命令与结果：

| 命令 | 结果 |
| --- | --- |
| `cargo test -p xirang-conventions --offline` | 99 passed / 0 failed（含 12 道门禁的出厂树断言） |
| `cargo test --workspace --offline` | 每一段 `test result` 均为 `ok`（用于确认报告目录改名后门禁不再被本人文件弄红；总计数以 executor 的基线为准） |
| `tools/xirang-publish --check-table` | exit 0，`dependency table matches the manifests (9 crates)` |
| `tools/xirang-publish --workspace-version` | `0.1.6`（与 `Cargo.toml` 的 `[workspace.package] version` 一致） |
| `sh -n tools/*`（6 个脚本） | 全部通过 |
| `tools/xirang-visual --list` / `--no-build zzz` | 列出 8 个场景 / exit 2 + 提示可用场景，未写任何文件 |
| `rustc -W help \| grep missing.docs` | `missing-docs  allow` |

证据口径：每条发现标 `[实测]`（我跑过命令/读过运行输出）或 `[读码]`（逐行读过判定代码，未执行变异——本轮无写权限做源码变异）。
编号说明：初稿里计划过的 `G-14`/`G-15` 在成稿时被合并/删除（一条是我自己否掉的"同一文件三处 tag 守卫不一致"，一条并进了 G-25），因此编号在 `G-13` 与 `G-16` 之间留空；编号不做重排，以免与已经发出的消息对不上。

---

## 1. 假阴性（门禁假绿方向）

### G-01 MAJOR — 门禁-假阴性：`lint` 把 `#![deny(warnings)]` 当作"带着 missing_docs"，保护可被静默移除

- file:line：`conventions/src/lint.rs:225-227`（`if trimmed == "#![deny(warnings)]" { return true; }`）
- 现象：`missing_roots()` 问的是"这个 crate 根还开着缺失文档 lint 吗"。`#![deny(warnings)]` 被直接判为"开着"。实测 rustc：`missing_docs` 的默认级别是 `allow`，`warnings` lint group 只含默认 `warn` 的 lint，因此 `#![deny(warnings)]` **不**让 `missing_docs` 生效。
- 判据 `[实测]`：
  - `rustc -W help | grep missing.docs` → `missing-docs  allow  detects missing documentation for public members`；
  - `/tmp/lint_probe.rs` = `#![deny(warnings)]` + `pub fn undocumented() {}` → `rustc --crate-type lib` **exit 0，零输出**；
  - 同一文件换成 `#![warn(missing_docs)]` → 输出 `warning: missing documentation for the crate`。
  也就是说，把 `#![warn(missing_docs)]` 换成 `#![deny(warnings)]` 后，`missing_roots` 报空、`no_item_silences_the_lint` 也报空，而 clippy 门禁的 `-D warnings` 同样抓不到任何东西——两道防线一起消失。
- 最小修复方向：删掉这条特例，只接受 `#![warn(...missing_docs...)` / `#![deny(...missing_docs...)`；若确实想认可 `deny(warnings)`，必须同时要求文本里出现 `missing_docs`。
- 复核手段：在 `lint.rs` 的测试里加一条负例（`#![deny(warnings)]` + 未文档化 public item），断言 `missing_roots` 非空——今天是空的；外加上面那两条 rustc 命令。

### G-02 MAJOR — 门禁-假阴性：上传步骤的 tag 守卫用 `contains` 判否，等价否定写法整类放行

- file:line：`.github/workflows/release.yml:122`（发布步骤的 `if:`）vs `conventions/src/release_workflow.rs:96-111`
- 现象：门禁要求上传步骤的 `if:` 不含 `!startsWith` / `!=` / `||`，且**包含** `startsWith(github.ref, 'refs/tags/')`。这个 `contains` 组合漏掉"同一守卫的否定式"：`startsWith(github.ref, 'refs/tags/') == false` 含 TAG_GUARD、不含三个否定记号，于是被读成"守卫正确"，而 GitHub 表达式语义下它等价于"只在**非** tag ref 上跑"，即每个分支 push 都会发布。
- 判据 `[读码]`：`conventions/src/release_workflow.rs:96-111` 的四个判定（`contains("!startsWith")`、`contains("!=")`、`contains("||")`、`contains(TAG_GUARD)`）逐条套在条件串 `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/') == false` 上：前三条为假、第四条为真 → 不产生任何 finding。这正是该门禁文档自己记录过的同一类绕过（`release_workflow.rs:16-21`："`!startsWith(...)` passes a `contains` test"），只堵了 `!` 前缀那一种拼法。
- 最小修复方向：把条件判定从"子串黑名单"改成"必须匹配一个只允许肯定式的形状"（例如去掉空白后要求条件中每个 `&&` 项的形态在白名单内），或至少额外拒绝 TAG_GUARD 之后紧跟 `== false` / `= false` / `== 0` 的写法。
- 复核手段：在 `release_workflow_tests.rs` 加一条 fixture（步骤 `if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/') == false`，`run: tools/xirang-publish --publish --yes`），断言 findings 含 `positive tag guard` 或 `true without a tag ref`——今天是空。

### G-03 MINOR（潜在，实测为漏检）— 门禁-假阴性：`doc_blocks` 的 markdown 半边不按 CommonMark 配对围栏字符/长度

- file:line：`conventions/src/doc_blocks.rs:174-180`（围栏识别只比反引号/波浪号前缀）、`conventions/src/doc_blocks.rs:188-200`（已开块遇到**任何**围栏行就当作闭合）
- 现象：门禁的文档说 CommonMark 有两种围栏字符，于是两种都认；但闭合时既不比对字符、也不比对长度。结果两类内容整段隐形：① 4 个反引号包住的 rust 围栏块——开围栏被读成 info string，`names_rust` 为假，整块从不打开；② `~~~rust` 块内部出现一行反引号围栏时，该行被当作闭合，其后的内容不再被解析。
- 判据 `[实测]`：我把探针写进自己的报告文件（当时的 `docs/audit-2026-09-28/lane-gates.md`）后跑 `cargo test -p xirang-conventions --offline documented_rust_blocks_parse`，输出里**只有**第 3 行那条（rust 围栏 + `pub struct BrokenP1 {`），我用 4 反引号与 `~~~rust`+内嵌反引号刻意写坏的另两个块**一条都没被报出**。该文件现已改名豁免，探针内容不再留在报告里。
- 最小修复方向：记住开围栏的字符与长度，只在"同一字符且长度 ≥ 开围栏长度"时闭合（CommonMark 的规则）；info 解析时按反引号/波浪号各自剥前缀。
- 复核手段：给 `doc_blocks_tests.rs` 加三条 fixture（4 反引号包围坏块、`~~~` 内含反引号后接坏块、两种围栏混用），断言 findings 数；今天前两类为 0。当前出厂树里 4 反引号与 `~~~` 围栏各 0 处（`git ls-files '*.md'` 全量扫过），所以是潜在而非活动缺陷。

### G-04 MINOR（潜在）— 门禁-假阴性/假阳性各半：`size` 只认字面前缀 `#[cfg(test)` 与字面 `mod x;` / `pub mod x;`

- file:line：`conventions/src/size.rs:151-152`（`bare` / `bare_public` 两个字面量）、`conventions/src/size.rs:170`（`previous.starts_with("#[cfg(test)")`）
- 现象：`declaration_of` 用精确字符串判断"这条声明把文件挂成测试"，而 rustc/cargo 对以下拼法等价：`#[cfg(all(test))]`（同一含义）、`pub(crate) mod tests;`（同一挂载，只是多一个可见性限定）。这两种拼法下 `declaration_of` 返回 `None` → 文件不被豁免 → 被算作源码度量。
- 判据 `[读码]`：`#[cfg(test)]` 与 `#[cfg(all(test))]` 都是"仅在测试构建编译"；`mod tests;` 与 `pub(crate) mod tests;` 指向同一个 `tests.rs`。`size.rs:151-171` 的判据是 `trimmed == bare` / `== bare_public` / `starts_with("#[cfg(test)")`，三处对拼法敏感。方向上：测试文件可能被报成新欠账（假阳性）；而把真实超大源码改名 `x_tests.rs` 再用 `pub(crate) mod` 挂载不会被误豁免（方向是对的）。
- 最小修复方向：`mod` 声明先剥 `pub` / `pub(...)` 前缀再比较；`#[cfg]` 判定改为"含 `cfg(` 且其中出现 `test` 词元"（注意 `#[cfg(not(test))]` 要排除）。
- 复核手段：`size.rs` 已有现成夹具 `a_mounted_test_file_is_exempt`（`size.rs:430-455`），把里面的 `mod probe_tests;` 改成 `pub(crate) mod probe_tests;`、把 `#[cfg(test)]` 改成 `#[cfg(all(test))]`，两条断言今天会变红。

### G-05 MINOR — 门禁-假阴性：两个 example 宿主的版本要求没有任何门禁覆盖

- file:line：`examples/control-button/Cargo.toml:14`、`examples/control-button-graft/Cargo.toml:13`；豁免处 `conventions/src/release_version.rs:115-120`（`publish = false` 成员整体 `continue`）
- 现象：两个宿主对 `xirang-run-method` 写的 `version = "0.1.6"` 是发布会一起移动的字面量，但它们不参与任何检查：`release_version` 跳过分包成员；`tools/xirang-publish --check-table` 的成员循环在 `publish = false` 处 `continue`（`tools/xirang-publish:567-572`），其 `metadata_requirement_problems` 也只在 `shipping` 集合里找包；`tools/xirang-package-audit` 同样跳过。今天两处都恰好是 0.1.6（版本线一致），所以问题不可见；下次推进版本线时，漏改这两处**不会**让任何命令变红。
- 判据 `[读码]`：三处跳过点如上；两处字面量今日与 `[workspace.package] version` 相同（全量 grep：`0.1.6` 在 `*/Cargo.toml` 里共 19 处依赖要求行，其中恰好这 2 行的宿主 `publish = false`）。
- 最小修复方向：`release_version` 对非发布成员也检查"带 `version` 的依赖要求必须等于发布版本"（或一律省略 `version`）；或在 `--check-table` 里对非发布成员做同样的 caret 检查。
- 复核手段：把 `examples/control-button/Cargo.toml:14` 临时改成 `0.1.5`，跑 `cargo test -p xirang-conventions --offline` 与 `tools/xirang-publish --check-table`——今天都绿（我未执行该变异，本轮无写权限）。

---

## 2. 假阳性（误报无辜代码方向）

### G-06 MAJOR — 门禁-假阳性：record 豁免只看**文件名前缀**，不看目录

- file:line：`conventions/src/doc_blocks.rs:133-141`（`is_record` 只取 `path.file_name()`）、`conventions/src/doc_blocks.rs:69`（`RECORD_PREFIXES = ["audit", "design"]`）、`conventions/src/doc_anchors.rs:18-20`（散文措辞"Anchors in audit and design documents are exempt"）
- 现象：`docs/audit-*.md` 豁免，但 `docs/audit-2026-09-28/lane-kernel.md` 不豁免——它的文件名是 `lane-kernel.md`。于是本轮审计的报告目录整体被当**活文档**两向扫描：`doc_blocks` 要求每条 rust 围栏能解析，`doc_anchors` 要求每条 `.rs:NNN` 此刻可解析（含配对 token 必须落在所引行区间内）。审计记录里摘录 Rust 片段、写"当时的位置"是常态，因此这条边界会把"记录"逼成"活文档"，或逼人删掉锚点。
- 判据 `[实测]`：同一份报告文件双向证明——旧名 `lane-gates.md` 时，`documented_rust_blocks_parse` 与 `the_shipped_documentation_anchors_resolve` 都点名它（前者报 `docs/audit-2026-09-28/lane-gates.md:3`，后者报 3 条：`no such file`、`line 99999 is past the end … (508 lines)`、配对 token `has_cjk` 不在所引行上）；改名 `audit-lane-gates.md` 后两条命令对它的发现归零。同一时刻同目录 peer 报告也各自产生发现（`lane-kernel.md` 1 条、`lane-studio.md` 9 条）。含目录名的形态 `audit-2026-09-28/` 不参与判定。
- 最小修复方向：`is_record` 改为看**任一路径段**的前缀（任一组件以 `audit`/`design` 开头即豁免），这样 `docs/audit-2026-09-28/` 整体是记录目录；同步改 `doc_anchors.rs:18-20` 的措辞（现在说的是 documents，实现说的是 file names）。
- 复核手段：把任意 lane 报告临时改回非 `audit-` 前缀，跑 `cargo test -p xirang-conventions --offline documented_rust_blocks_parse` 与 `the_shipped_documentation_anchors_resolve`（各 1 次）。

### G-07 MINOR — 门禁-假阳性：`naming` 的"要求位置"扫描不跳过注释

- file:line：`conventions/src/naming.rs:238-276`（`package_arguments` 在工作流**原文**上找 `-p`/`--package`）、`conventions/src/naming.rs:199-234`（`requirement_names` 在脚手架模板原文上找 `xirang-… =` 与 `package = "…"`）
- 现象：该门禁的边界声明是"只读名字处于**要求**位置的地方"（`naming.rs:143-149`），但 YAML 注释与 Rust 注释不剥离。于是工作流里一句注释 `# 旧名：cargo test -p xirang-old` 会被报成"names `xirang-old`, which is not a workspace package"；脚手架模板里一行注释同理。这正是"让人把门禁关掉"的那种误报——它要求你去改记录性注释。
- 判据 `[读码]`：`package_arguments` 直接在 `text` 上 `find(needle)`，没有注释剥离；`requirement_names` 也直接在 `text` 上扫描。旁证：`naming_tests.rs:119` 的 fixture 里那句注释会被读到，只因它点名的名字恰好合法才没有 finding——说明"注释会被读"是既成行为。
- 最小修复方向：YAML 先剥行尾 `#`（注意引号内的 `#`）；模板用"剥注释、保留字面量"的 mask 后再扫（不能用 `mask_non_code`，它会把模板里的要求文本一起抹掉）。
- 复核手段：给 `naming_tests.rs` 加一条 fixture：工作流与模板各放一个**注释里**的不存在包名，断言 findings 为空。今天为红。

### G-08 MINOR（潜在）— 门禁-假阳性：共享助手 `drop_governed_lines` 把属性与声明之间的文档注释误判成"声明"

- file:line：`conventions/src/lib.rs:247-282`（`!started` 分支只用 `trimmed.starts_with('#')` 判"还是属性"）；受影响的调用方 `conventions/src/purity.rs:84`（`#[cfg(any())]` 豁免）、`conventions/src/shims.rs:135-137`
- 现象：`started` 的判定是"这一行不以 `#` 开头就当声明"。`///` 文档注释不以 `#` 开头，于是：第 1 行 `#[cfg(any())]` 进入豁免态；第 2 行 `///` 被当成"声明"，`started = true`；第 3 行（与被豁免条目同缩进的 `pub fn`）让豁免解除且该行**被保留** → 报出一处"内核 I/O"。没有 `///` 时同一形状正确豁免（第 2 行就是声明、被抹白，第 3 行更深的行也抹白）。
- 判据 `[读码]`：状态机如上；`purity.rs:237-243` 的注释明确说该助手是为"属性与声明同缩进"这一最正统拼法服务的，而"属性 + 文档注释 + 条目"是同一件事的第三种拼法。当前出厂树没有这种形状（两道门禁都绿），故为潜在误报。
- 最小修复方向：`started` 改为"既不是属性（`#`）也不是注释/文档行（`//`、`/*`）"，即把注释行留在属性栈里一起抹白。
- 复核手段：给 `purity_tests.rs` 的 `a_never_compiled_item_at_the_same_indentation_is_exempt` 加一个变体（属性 + `///` + 含 `std::fs` 的函数），断言 findings 为空。今天该变体会红。

---

## 3. 工具与脚本（退出码、用法校验、失败时写什么）

### G-09 MINOR — 工具-脚本：`--workspace-version` 不与其它模式互斥，且在派发前 `exit 0`

- file:line：`tools/xirang-publish:96-143`（互斥检查只覆盖 `--check-table`/`--verify-consumers` 与 `--publish`）、`tools/xirang-publish:289-292`（`workspace_version_only` 直接打印并 `exit 0`）
- 现象：`--workspace-version` 在互斥表之外，并且它的 `exit 0` 发生在 `--check-table` 分支（`:541`）与发布循环（`:693`）之前。因此 `tools/xirang-publish --check-table --workspace-version` **实测**只打印 `0.1.6` 并 exit 0，表检查根本没跑；同理 `--publish --yes --workspace-version` 在干净树上会 exit 0 而什么都没上传（干净树才走到 `:289`；脏树会先被 `:152` 的 dirty 检查拦下，所以今天只能观察到前者的行为）。脚本自己在 `:131-143` 用长注释说明"检查模式与 `--publish` 组合会什么都不做却 exit 0，宁可拒绝"——正是同一个隐患，只是漏了第三个模式。
- 判据 `[实测]`：`tools/xirang-publish --check-table --workspace-version; echo $?` → `0.1.6` / `0`。
- 最小修复方向：把 `workspace_version_only` 纳入互斥检查（与任何其它模式同现即 exit 2），或把它的提前返回移到所有模式判定之后。
- 复核手段：上面那条命令；期望 exit 2 或"表检查已跑"的输出。

### G-10 MINOR — 工具-脚本：`xirang-release-audit` 的符号审计可能静默为空且仍报成功

- file:line：`tools/xirang-release-audit:23-30`（`nm … || :` 吞掉失败；`count` 只写 TSV）、`:74`（无条件打印 `symbol_audit=…`）
- 现象：`nm` 不可用或失败时，`$symbols` 文件仍是空文件（或只写一行 `nm unavailable`），`count` 变成 0/`unavailable`，脚本照常走完并打印 `symbol_audit=<tsv>`。TSV 里每个 artifact 的 `defined_symbols` 可以是 0，没有任何断言要求非空——"审计了符号"这句话在 nm 坏掉的那天就是空的。
- 判据 `[读码]`：`:24` 的 `|| :` 与 `:74` 的收尾打印；`:17-36` 的循环没有任何非空校验。
- 最小修复方向：`nm` 失败即报错（或明确标注该 artifact 未审计），并对 `count == 0` 报警；收尾行改为只在至少一个 artifact 有符号时打印。
- 复核手段：造一个假的 `nm`（PATH 前置一个 exit 1 的脚本）跑一次，观察仍 exit 0 且 TSV 全 0。

### G-11 MINOR — 工具-脚本：`xirang-visual --help` 用硬编码行范围打印，句子中途截断

- file:line：`tools/xirang-visual:115-119`（`sed -n '2,26p' "$0"`）
- 现象：头部注释块是第 2–36 行，`--help` 只印到第 26 行：实测最后一行停在"写两个文件：`<scene>.txt`"，而文件第 27 行紧接着"是面板的纯文本——布局…"；同时整段 "Why tmux"（第 32–36 行）不出现。行号范围是无注释的魔数，注释一改就漂。
- 判据 `[实测]`：`tools/xirang-visual --help | tail -6` 的输出如上；`tools/xirang-visual:32-36` 是未打印的段落。
- 最小修复方向：改用 `tools/xirang-publish:111-117` 的同款 awk（"打印开头连续注释块，遇到首行代码即停"），去掉行号范围。
- 复核手段：`tools/xirang-visual --help | tail -3` 应结束在一个完整句子上。

### G-12 MINOR — 工具-脚本：两处 GNU 专属语法，macOS 上会失败（CI 只在 Linux 跑，所以不红）

- file:line：`tools/xirang-external-rehearsal:64`（`sed -i "s|…|…|g"` 无备份后缀）、`tools/xirang-release-audit:70-72`（`date +%s%N`）
- 现象：BSD/macOS `sed -i` 要求跟一个备份后缀，无后缀写法会把替换脚本当成后缀、把文件名当成脚本，报错退出；BSD `date` 没有 `%N`，`$(( (end - start) / 1000000 ))` 会在非数字上失败。这两条命令 CI 只在一个 Linux 单元上跑（`.github/workflows/ci.yml:37`、`:50`），而 CI 矩阵含 `macos-latest`、`AGENTS.md` 把三条 cargo 命令写成可在本机跑的验证入口，因此 macOS 贡献者手动跑这两个工具会撞到与门禁无关的失败。
- 判据 `[读码]`：两处语法如上；`tools/xirang-scale-audit:8` 对 `/usr/bin/time` 有存在性回退，同一份工具集里标准不一。
- 最小修复方向：`sed -i.bak` 后删备份（或改用 `awk`/临时文件）；时长改用 `date +%s`（×1000）或换成 `scale_audit` 自己打印毫秒。
- 复核手段：`command -v gsed || echo BSD-sed`；或在 macOS 上跑 `tools/xirang-external-rehearsal`。

### 工具-脚本的干净项
6 个脚本 `sh -n` 全过；未登记场景名被拒绝（exit 2、不写文件）；`--publish` 必须配 `--yes`；`--check-table`/`--verify-consumers` 与 `--publish` 的组合被拒绝；index 探针对"网络失败"与"404 未发布"分流（不把不可达读成未发布）；所有写盘都落在 `target/`。详见第 9 节。

---

## 4. CI 与发布工作流

### G-13 MAJOR — CI-发布：持有 `CARGO_REGISTRY_TOKEN` 的 job 的 action pin 只由注释承诺，没有门禁

- file:line：`.github/workflows/release.yml:42-57`（两个 SHA pin + 理由注释）、`.github/workflows/release.yml:123-125`（步骤级 `CARGO_REGISTRY_TOKEN`）；门禁边界 `conventions/src/release_workflow.rs:60-114`（只检查上传步骤的命令与 tag 守卫）
- 现象：该文件自己的注释把 SHA pin 说成防线——"a retagged upstream action would otherwise run before the publish step and could rewrite what that step executes"。但把 `actions/checkout@11d5960a…` 换回 `@v4`、或把 `dtolnay/rust-toolchain@6bed0761…` 换回 `@stable`，`cargo test --workspace` 全绿：没有任何门禁读 `uses:` 的 ref 形态。`ci.yml` 有意保持 tag pin（无秘密），这个差异只存在于注释里，没有任何检查区分两者。
- 判据 `[读码]`：`release_workflow.rs` 的 `step_uploads` 只看命令与 `if:`；`steps()` 解析出的每步 body 里 `uses:` 行只被 `delegated_path` 用来跟进**本地**（`./…`）action。今天两个 ref 都是 40 位十六进制（`.github/workflows/release.yml:54-55`）。
- 最小修复方向：在 `release_workflow.rs` 加一条：若某 job 的任一步骤带 `CARGO_REGISTRY_TOKEN`（或该 workflow 出现该名字），则同 job 内所有 `uses: <owner>/<repo>@<ref>` 的 `<ref>` 必须是 40 位十六进制；`ci.yml` 不受影响。
- 复核手段：把 `.github/workflows/release.yml:54` 改成 `actions/checkout@v4`，跑 `cargo test -p xirang-conventions --offline the_shipped_workflows_only_publish_a_tag`——今天仍绿。

### CI-发布干净项
`permissions: contents: read` 两个工作流都有；发布步骤**在** tag 检查、表检查、fmt/test/clippy/doc、包审计、发布件审计之后；`concurrency` + `timeout-minutes` 有界；`continue-on-error` 全仓 0 处；无 `inputs.` 触发的手动发布开关；上传步骤自带肯定式 tag 守卫；CI 有意不传 `--offline`，本地工具默认传（`XIRANG_VISUAL_CARGO_FLAGS`/`XIRANG_REHEARSAL_CARGO_FLAGS` 两处显式传空）。详见第 9 节。

---

## 5. 文档漂移（docs/README/CHANGELOG vs 代码、`--help`、子命令集合）

### G-16 MAJOR — 文档-漂移：roadmap 与 `xirang-package-audit` 头部写的发布层级顺序与活表矛盾，且按它发布必然失败

- file:line：`docs/roadmap-1.0.md:255`（"core → macro/build_method/mcp → run_method → debug_method/plugin-host → studio → cli"）、`tools/xirang-package-audit:37-40`（逐字复制同一顺序）vs 活表 `tools/xirang-publish:86-94`
- 现象：`xirang-mcp` 在第 2 层——但它真实的内部依赖是 `xirang-build-method` + `xirang-core` + `xirang-debug-method` + `xirang-run-method`（`mcp/Cargo.toml:27`、`:33`、`:39`、`:40`），因此它必须在 run_method、debug_method 之后；活表把它放在第 5 层（core | macro+build-method | run-method | debug-method+plugin-host | mcp | studio | cli）。按文档的顺序手动发布会在 `cargo publish -p xirang-mcp` 处解析失败。
- 判据 `[实测+读码]`：`tools/xirang-publish --check-table` 从清单推导依赖并逐条核对两张表，实测 exit 0 / `9 crates`——即活表是被门禁交叉验证过的唯一顺序；roadmap:255 的散文顺序不被任何门禁检查，且与能被推导出来的事实矛盾。
- 最小修复方向：把 `docs/roadmap-1.0.md:255` 与 `tools/xirang-package-audit:37-40` 改成与活表一致，或删掉顺序、改为"顺序以 `tools/xirang-publish` 的 `levels` 表为准（`--check-table` 会核对）"。
- 复核手段：把 roadmap 里的顺序与 `tools/xirang-publish:86-94` 并排比对；或按 roadmap 的层序逐层问 `deps_of`。

### G-17 MINOR — 文档-漂移：size 棘轮的条目数在文档里是 14，在代码里是 1

- file:line：`docs/roadmap-1.0.md:40`（现在时："清单从 15 项开始 … 现为 14 项"）vs `conventions/src/size.rs:66-67`（`BASELINE` 只有 `contracts.rs` 一项，639 行）
- 现象：同一份 roadmap 第 29 行已经写了"尺寸上限已于 2026-09-28 由 450 放宽到 600"，而第 40 行仍把欠账说成 14 项。上限放宽后 13 项缩回上限内、棘轮强制删除，剩下的只有 1 项。读者（含维护者）会以为还有 14 个超标文件要拆。
- 判据 `[实测]`：`cargo test -p xirang-conventions --offline the_size_ceiling_holds_except_for_the_pinned_debt` 绿，且 `size.rs:66-67` 就一项；`size.rs:24-28` 明确写了"条目数不在这里复述——它就是 `BASELINE.len()`"，同 crate 的 `mounting.rs:10` 却复述了（见 G-20）。
- 最小修复方向：把 roadmap 那句的"现为 14 项"改成不写数字（引用 `BASELINE`）。
- 复核手段：`sed -n '66,67p' conventions/src/size.rs` 与 `sed -n '40p' docs/roadmap-1.0.md` 对照。

### G-18 MINOR — 文档-漂移：CHANGELOG 用现在时引用了工作树里已不存在的清单拼写

- file:line：`CHANGELOG.md:31`（"would move every internal `version = "0.1.4"` requirement with it"）与中文半边 `CHANGELOG.md:56`（同一句）
- 现象：这是在描述**当前**状态（把版本线抬到 1.0.0 会连带的那些要求），而工作树里的这类字面量是 `0.1.6`；`version = "0.1.4"` 一处都不存在（我全量 `git grep` 过，只命中这两行 CHANGELOG）。
- 判据 `[实测]`：`git grep -n 'version = "0\.1\.4"'` → 仅 `CHANGELOG.md:31`、`:56` 两行。
- 最小修复方向：写成 `version = "0.1.x"`，或不举例。
- 复核手段：上面那条 grep。

### G-19 MINOR — 文档-漂移：`AGENTS.md` 的门禁清单比实际门禁宽，且缺 4 道

- file:line：`AGENTS.md:151-168`（"因此默认门禁已经会在下列情形失败："+ 清单）vs `conventions/src/features.rs:30-31`、`:74-88`
- 现象：清单里"a feature-gated target whose `required-features` is missing"只有**手列**的那一个 target（`studio`/`xirang-dev`）能被判"缺失"——manifest 里没有任何东西声明"这个 target 只在检出内工作"；其余 crate 的 target 即使漏了 `required-features` 也不会红（能红的是"`required-features` 指名的特性不存在/默认开着"这两种）。同一清单也没有提到 `naming`、`release_version`、`release_workflow`、`doc_anchors` 四道门禁。
- 判据 `[读码]`：`features.rs:30-31` 的 `WORKSPACE_ONLY_TARGETS` 只有一项；`features.rs:58-73` 只做"指名的特性存在且非默认"。
- 最小修复方向：把清单写成"例如"，或为"缺失"那条加上"（对手列 target 判定）"限定，并补上缺失的四道门禁名。
- 复核手段：给任意其它 crate 的 `[[bin]]` 加一行 `required-features = ["nope"]`（会红）与给某 target 加一个真实特性却不写 `required-features`（不会红）——后者说明"缺失"只对手列 target 成立。

### G-27 MINOR — 文档-漂移：roadmap 说依赖要求写 `0.1.0` 以便不必连锁重发，而工作树里写的是 `^0.1.6`，且两道门禁**强制**它等于工作区版本

- file:line：`docs/roadmap-1.0.md:76-79`（"依赖版本要求写 `0.1.0`（caret，`>=0.1.0,<0.2.0`），所以补丁版本不会连锁要求重发依赖方，只有真正用到新行为时才抬下界"）vs 工作树（`build_method/Cargo.toml:16` 等 19 处 `version = "0.1.6"`）+ 门禁 `conventions/src/release_version.rs:281-303`（`record`：`version == release` 才通过）+ `tools/xirang-publish:462-512`（`want="^$workspace_version"`，不等即报问题）
- 现象：roadmap 把"独立发版"记成"包形状保持九个 crate"这条决策的理由之一，并说下界停在 `0.1.0`、补丁版本不连锁。实际上下界随每次发布一起抬（今天 19 处全是 `0.1.6`），而且 `release_version` 与 `--check-table` 都要求"成员要求 == 工作区版本"，因此"只把一个 crate 发到新版本、其余不动"在清单层就被判红——两条口径（散文的意图 vs 门禁的强制）不一致。
- 判据 `[实测]`：`tools/xirang-publish --check-table` 实测 exit 0 / 9 crates，而它内部的推导就是 `want="^$workspace_version"`（`tools/xirang-publish:463`）与 `req != want` → 报问题（`:505`）；把任意一处要求改成 `0.1.0` 会让它报 `asks for ^0.1.0, but every internal requirement must be ^0.1.6`（我未执行该变异，本轮无写权限；判定读自上述两处代码路径）。
- 最小修复方向：把 roadmap 那句改成与实现一致（"下界随发布线一起抬，因此发布是一条线，不是九个"），或（若确想独立发版）把要求下界固定为 `0.1.0` 并放宽两道门禁——后者是设计决策，留给维护者。
- 复核手段：`git grep -c 'version = "0\.1\.6"' -- '*/Cargo.toml'`（19）对照 `docs/roadmap-1.0.md:76-79`；以及 `tools/xirang-publish:463`、`conventions/src/release_version.rs:289-294`。

---

## 6. 注释清晰度与可读性（队长追加）

判据优先级：① 过时注释（描述的行为 ≠ 实现）按 MAJOR；② 讲为什么/不变量/陷阱，而不是复述代码；③ 双语两边说的是不是同一件事；④ 术语一致性；⑤ 该由名字/类型承担的信息；⑥ 注释掉的死代码/悬挂 TODO；⑦ 位置与密度。

### G-20 MINOR — 注释-准确性：`mounting.rs` 的散文数字已过期（29 → 33），而散文里的常量没有对账机制

- file:line：`conventions/src/mounting.rs:10`（"which is why **29 such declarations** exist"；中文半边同句在 `:16`）
- 现象：该句在解释"裸 `mod x;` 在父文件是 crate 根或本身经 `#[path]` 载入时同样正确"，并把数量写成 29。按与该句同义的语法口径（屏蔽注释与字符串后取 `mod <name>;`，再回溯它上面的 `#[path]` 属性；目标文件必须存在）实测今天是 **33 条裸声明**（另有 299 条带 `#[path]`，合计 332 条声明）。`AGENTS.md:88-90` 对同一类计数明确写了"（the count is not restated here: it changes with every new module）"——本仓库已经决定不复述计数，而门禁 crate 自己仍在复述。幅度只有 29→33，所以是 MINOR；要害不是差多少，而是**没有任何机制对账散文里的常量**：`doc_anchors` 只查成对写出的 `.rs:<line>`，查不到散句里的数字（这条边界见 G-06 与第 9 节）。
- 判据 `[实测]`：我按 `conventions/src/mounting.rs:87-122`（`declarations`）+ `:135-176`（`path_attribute_before`）+ `:235-246`（`target_of`）的语义写了独立复算脚本（Python，屏蔽 `//` 与 `/* */` 与字符串字面量，再回溯属性/可见性）：全工作区 332 条声明，其中**裸 33 / 带 `#[path]` 299**，全部目标文件存在、0 条解析不到——与 boundary-architect 在 `docs/audit-2026-09-28/audit-structure-base.md` §2.1 的 33 一致（它的合计是 339，多出的 7 条应是它把生成计划目录里的声明也算进去了；裸声明两边都是 33）。**更正我自己先前的一次误测**：line-anchored 数法（"这一行以 `mod`/`pub mod` 开头"）会数出 335 条，因为 `core/src/registry_core.rs` 一类的文件把 `#[path = "…"]` 写在上一行、`pub mod x;` 单独一行——那样数出来的是"声明行"而不是"裸声明"，差了 10 倍。口径必须在报告里写明，否则两个数都像对的。
- 最小修复方向：删掉数字，改为"这些声明在本树里很常见（计数不在此复述：它随每个新模块变化）"，与 `AGENTS.md` 和 `conventions/src/size.rs:24-28` 的既有做法对齐；若确实想钉住规模，就在门禁里断言（例如"裸声明数 ≥ N"），而不是写在散文里。
- 复核手段：复算脚本 `/tmp/decl_count.py`（本轮临时产物，未进仓库）输出 `whole tree: decls=332 pathed=299 bare=33`；或直接在 `conventions/src/mounting.rs` 里临时把 `Mounts` 的 `declarations` 收集起来计数。

### G-21 MAJOR — 注释-准确性：`xirang-package-audit` 的头部注释描述了一张已不存在的表

- file:line：`tools/xirang-package-audit:37-53`（"The order below mirrors docs/roadmap-1.0.md decision 1: …"、"Each entry is `<crate>:<directory>:<comma-separated versioned xirang-* deps>`. The directory is spelled out rather than derived because the mapping is not a rule…"）
- 现象：脚本里**没有**任何 `crates='…'` 表：`:227-249` 从根清单的 `members` 推导每个成员，从 `cargo metadata --no-deps` 读依赖，目录直接取成员路径。也就是说这十几行注释在解释"一份手工表、一个手工顺序、目录为什么必须写出来"——三者都已不存在。更糟的是它复述的顺序本身是错的（G-16）：读者会被引向去找一张表，并相信那个顺序。
- 判据 `[读码]`：`tools/xirang-package-audit:205-249`（推导）vs `:37-53`（散文）；`tools/xirang-package-audit:90-100` 的注释还保留了"扫描 TOML 漏掉的三种拼法"这段已改写的实现史，与实现（改从 metadata 读）不一致。
- 最小修复方向：把 `:37-53` 换成"crate 集合与依赖从工作区推导（同 `tools/xirang-publish --check-table` 的口径），顺序以 `levels` 表为准"；删掉"目录为何手写"那段（它解释的是被替换掉的实现）。
- 复核手段：`grep -n "crates='" tools/xirang-package-audit`（无命中）对照 `:37-53` 的措辞。

### G-22 MINOR — 注释-准确性：`lint.rs` 的模块文档低估了自己的范围

- file:line：`conventions/src/lint.rs:15-20`（"Boundary: the requirement covers the nine published crates' library roots."）vs `conventions/src/lint.rs:27-38`（函数文档：除示例宿主外每个 crate 目录的库根 + 一个二进制根）
- 现象：`required_roots` 实测返回 10 个库根（含**未发布**的 `conventions/src/lib.rs`）外加 `mcp/main.rs`，共 11 个。模块级那句只说九个已发布 crate（再加一句 MCP 二进制），漏掉了 `conventions` 自己——而这恰恰是注释里说要防的一件事（`:29-38`：删掉 `conventions` 自己的属性曾经无人发现）。同一模块的两段文档对同一事实给出不同口径。
- 判据 `[实测]`：按根清单 `members` 复算：`core/macro/run_method/build_method/cli/debug_method/studio/plugin-host/mcp/conventions` 各有 `src/lib.rs` → 10；`mcp/main.rs` 存在 → 11。
- 最小修复方向：模块级那句改成"除示例宿主外的每个 crate 目录的库根（含 `conventions` 自己）+ MCP 的二进制根"，与函数文档统一。
- 复核手段：上面那次复算；或读 `lint.rs:39-60` 的实现。

### G-23 MINOR — 注释-准确性：`doc_anchors.rs` 对豁免范围的措辞与实现不符（与 G-06 同源）

- file:line：`conventions/src/doc_anchors.rs:18-20`（"Anchors in audit and design documents are exempt for the same reason their code blocks are: they record what was true when they were written."）；实现 `conventions/src/doc_blocks.rs:133-141`
- 现象：实现豁免的是"文件名以 `audit`/`design` 开头"，不是"审计与设计文档"。本轮报告目录 `docs/audit-2026-09-28/` 下的文件就不是豁免对象——注释把一个**文件名约定**说成了一个**文档类别**。`doc_blocks.rs:53-69` 的同类措辞也偏松，但那里至少写了 "File-name prefixes the gate does not cover"，比 `doc_anchors.rs` 准确。
- 最小修复方向：改写成"文件名以 `audit`/`design` 开头的文档豁免（见 `doc_blocks::RECORD_PREFIXES`）"；若采纳 G-06 的修复，则改成"记录目录/记录文件"。
- 复核手段：读 `doc_anchors.rs:18-20` 与 `doc_blocks.rs:133-141` 对照；或用 G-06 的双向改名实验。

### G-24 MINOR — 注释-准确性：`size.rs` 一句话里的层数与它列出的证据不一致

- file:line：`conventions/src/size.rs:124-130`
- 现象：先说"2026-09-28 实测本树出厂的最深挂载链是 6 层"，括号里给的却是四个文件（`authoring.rs` → `parse/parse.rs` → `parse/flow.rs` → `parse/flow_tests.rs`）。四个名字对应 3 条边/4 个节点，"6 层"无论按节点还是按边都对不上，读者无法判断该以哪个为准——而这句话正是用来论证递归上限 16 的余量的。
- 最小修复方向：把括号里的链条补全到 6 个节点（或改成"4 个节点"），并写清"层"是指节点数还是边数。
- 复核手段：读该段；如需实测，可对 `size.rs:131-143` 的递归加一次最大深度打印（当前上限 16，余量声明依赖这句）。

### G-25 MINOR — 注释-准确性：`ci.yml` 留了一条条件已经满足的悬挂指令

- file:line：`.github/workflows/ci.yml:49-55`（"Add `cargo package -p xirang-build-method --locked --allow-dirty` **after the first crates.io release**."）+ `.github/workflows/ci.yml:56-69`
- 现象："首次 crates.io 发布"已于 2026-09-25 发生（`CHANGELOG.md:14-17`：0.1.0 九个 crate 一同上线，0.1.1/0.1.3/0.1.4/0.1.5 相继跟进），因此这条注释今天读起来像一条待办；而紧邻的 `tools/xirang-package-audit` 步骤已经把九个 crate 的打包纳入（`:69`），`cargo package -p xirang-core` 单独一步（`:55`）于是与它重复。要么补上那句命令（若确要独立验证 build-method 的 tarball），要么删掉这条注释。
- 最小修复方向：删掉悬挂指令，或把它改成不带时间条件的说明（"package-audit 覆盖九包；core 的单独一步是早期残留"）。
- 复核手段：`grep -n 'after the first crates.io release' .github/workflows/ci.yml`。

### G-26 MINOR — 注释-可读性：用户可见输出里用"from today / 从今天起"

- file:line：`tools/xirang-package-audit:449-455`（summary 文本 "Contents are checked for every crate from today."）、`AGENTS.md:196`（"are verified for all nine crates from today"）
- 现象："今天"是写下那句话的那一天；对 0.1.5 已发布的今天，这句话不含任何信息量（读者无法判断"今天"指哪一天、当时是什么状态）。同一脚本的头部注释也用 "Before the first publish" 作为时态锚点（`:20-35`），而首次发布早已完成。
- 最小修复方向：改成无条件成立的说法（"内容检查对每个 crate 都生效，不需要 registry"），把历史成因留给 CHANGELOG/roadmap。
- 复核手段：读 `tools/xirang-package-audit:449-455` 的 summary 字符串（或跑一次脚本看输出）。

### 注释-可读性"值得保留"的正面样本（同节对照）
`conventions/src/size.rs:21-28`（明确"条目数不在此复述，它就是 `BASELINE.len()`"，并记录了"这句话曾声称十二项而清单只有九项"的教训）；`conventions/src/doc_blocks.rs:15`（"这里有意不写数量：散文里的数字会漂"）；`tools/xirang-publish:70-75`（解释探针为什么与 package-audit 各留一份）；`conventions/src/mounting.rs:83-86`（解释 `mod` 关键字与属性形状为什么分别读自 masked/raw）。这几处写的是"为什么/陷阱/边界"，不复述代码。

### 双语注释与文档（队长判据 ③）
- 抽查结论：判据里列出的两处不准确（G-22 `lint.rs`、G-23 `doc_anchors.rs`）在**两个语言半边都不准确**——中文半边逐句对应英文半边，所以 `bilingual` 门禁发现不了这类问题（它只问"有没有中文"，不问"两边是不是同一件事"）。这是 `bilingual` 边界的实证补充：门禁能防"漏一门语言"，防不了"两边一起错"。
- 我按同一口径粗扫了非示例 crate 的 `src/**/*.rs`：连续文档行块约 3296 个，其中 CJK 字符数 ≤3 的只有 1 个真实块（`core/src/registry_core/source/walk.rs:70-71`，`Skip it.` / `跳过。`——枚举变体的短文档，正当），另有 1 处命中来自 `conventions/src/purity_tests.rs:70` 字符串字面量里的伪文档行（正是 `conventions/src/bilingual.rs:54-60` 的屏蔽逻辑要挡的形状，说明该屏蔽有效）。**没有发现"只为过门禁而凑一句中文"的块。**

---

## 7. crate 数量的实测代价（队长追加；只给口径明确的数字，不下"该不该合"的判断）

### 7.1 发布依赖顺序表（`tools/xirang-publish`）

- 9 个发布 crate（活表条目 = 9），编成 **7 层**，在 `tools/xirang-publish:86-94`，每行一层（同一行内互不依赖）：
  `xirang-core` | `xirang-macro xirang-build-method` | `xirang-run-method` | `xirang-debug-method xirang-plugin-host` | `xirang-mcp` | `xirang-studio` | `xirang-cli`。
- 第二张表是内部依赖边，在 `tools/xirang-publish:322-332`，共 9 行 `crate:dep dep …`（合计 19 条边）。
- **推进版本线需要改这张表的行数：0 行**。两张表只写名字、不写版本；版本来自 `[workspace.package] version`（`tools/xirang-publish:255-275`）。新增/删除一个 crate 才需要动：`levels` 1 行 + `dependencies` 1 行（`--check-table` 会从清单推导并核对两者，`tools/xirang-publish:541-681`）。
- 表与清单的推导关系：`--check-table` 用 `cargo metadata --no-deps` + 根清单 `members` 推导"谁发布、依赖谁"，再与两张表对比（实测 exit 0 / 9 crates）。因此"表说错顺序"会被拦（G-16 的 roadmap 散文顺序则不会）。

### 7.2 共享版本线的连带影响（以 `0.1.6 → 0.1.7` 为例，逐文件实测）

`git grep -l '0\.1\.6'` 命中 15 个受追踪文件、44 处；分类如下：

| 文件 | 需要改的行 | 备注 |
| --- | --- | --- |
| `Cargo.toml` | 1（`[workspace.package] version`） | 唯一来源 |
| `build_method/Cargo.toml` | 1 行依赖要求（:16） | `release_version` + `--check-table` 都会核 |
| `run_method/Cargo.toml` | 2（:19、:20） | 同上 |
| `macro/Cargo.toml` | 1（:20） | 同上 |
| `debug_method/Cargo.toml` | 1（:16） | 同上 |
| `plugin-host/Cargo.toml` | 1（:33） | 同上 |
| `mcp/Cargo.toml` | 4（:27、:33、:39、:40） | 同上 |
| `studio/Cargo.toml` | 3（:72-74） | 同上 |
| `cli/Cargo.toml` | 4（:23、:24、:25、:34） | 同上（含一处 dev-dependency） |
| `examples/control-button/Cargo.toml` | 1（:14） | `publish = false` → **无门禁核**（G-05） |
| `examples/control-button-graft/Cargo.toml` | 1（:13） | 同上 |
| `Cargo.lock` | 10 处 | cargo 自动 |
| `CHANGELOG.md` | 4 处 | 手写发布记录（EN 半边 2 + 中文半边 2） |
| `tools/xirang-publish` | 6 处（**全在注释里**） | 功能上不需要改 |
| `docs/audit-3p-2026-09-25.md` | 4 处 | 记录文件，**不应**改（`audit-` 前缀 → 门禁豁免） |

口径：**必须同步修改的是 11 份清单的 20 行**（1 行版本 + 19 行依赖要求）＋ `CHANGELOG.md`（手工）＋ `Cargo.lock`（自动）；其中 2 行（两个宿主）无门禁覆盖。改名代价另算：`xirang-x` 这个名字出现在 7.5 节列出的所有位置。

### 7.3 `.github/workflows/` 里按 crate 名的 `-p`/`--package` 引用与 job 数

- job 总数 **6**：`ci.yml` 5 个（`verify`、`features`、`deny`、`release-audit`、`process-plugin`），`release.yml` 1 个（`release`）。
- `-p`/`--package` 命中 **4 处，全在 `ci.yml`**：`:53` 注释里的 `cargo package -p xirang-build-method`（悬挂指令，G-25）、`:55` `-p xirang-core`、`:153` `-p xirang-example-control-button`、`:180` `-p xirang-plugin-host`。`release.yml` 0 处——它只调用 `tools/xirang-publish`，`cargo publish -p "$crate"` 在脚本内（`tools/xirang-publish:752`）。
- **按单个 crate 设的 job 只有 1 个**（`process-plugin` → `xirang-plugin-host`）；`verify` 是 6 单元矩阵但按 `--workspace` 跑；`features` 按全特性跑工作区。其余 crate 没有专属 job。

### 7.4 每个成员的 publish / required-features / feature 门控现状（实测自各 `Cargo.toml`）

| 成员目录 | package | publish | `[[bin]]` | features | default |
| --- | --- | --- | --- | --- | --- |
| `core` | xirang-core | 发布 | 0 | `syntax` | — |
| `macro` | xirang-macro | 发布 | 0 | 无 | — |
| `run_method` | xirang-run-method | 发布 | 0 | `authoring` | — |
| `build_method` | xirang-build-method | 发布 | 0 | 无 | — |
| `cli` | xirang-cli | 发布 | 2（`xirang`、`cargo-xirang`） | 无 | — |
| `debug_method` | xirang-debug-method | 发布 | 0 | 无 | — |
| `studio` | xirang-studio | 发布 | 2（`xirang-studio`、`xirang-dev`） | `node-graph`、`prototype-fixtures`、`dev-supervisor` | `["node-graph"]` |
| `plugin-host` | xirang-plugin-host | 发布 | 0 | `wasm`、`process-tools` | `["wasm"]` |
| `mcp` | xirang-mcp | 发布 | 1（`xirang-mcp`） | 无 | — |
| `examples/control-button` | xirang-example-control-button | **false** | 0 | 无 | — |
| `examples/control-button-graft` | xirang-example-control-button-graft | **false** | 0 | 无 | — |
| `conventions` | xirang-conventions | **false** | 0 | 无 | — |

- `required-features` 只出现 3 次，全在 `studio/Cargo.toml`；其中 `xirang-dev` 那一处被 `features` 门禁钉住（`conventions/src/features.rs:30-31`）。
- 口径回答"哪些 crate 的存在只是为了让宿主按需安装"：**9 个里没有这样的 crate**——6 个是被依赖的 library（core/macro/run_method/build_method/debug_method/plugin-host），3 个带可安装的二进制（cli 2、studio 2、mcp 1）。真正在做"按需开关"的是 **feature**（`syntax`、`authoring`、`wasm`、`process-tools`、`dev-supervisor`、`prototype-fixtures`）与 `publish = false`（conventions + 2 宿主）。
- 附带事实：`xirang-dev` 是"只在检出内工作"的二进制，靠 `required-features = ["dev-supervisor"]` 挡住 `cargo install`；`prototype-fixtures` 默认关闭，只有 `.github/workflows/ci.yml:85` 的 `--all-features` 会构建它。

### 7.5 文档、脚手架模板与工具里按 crate 名引用的处数（合并/改名要动多少处）

| 区域 | 名字出现次数（逐个计数） | 备注 |
| --- | --- | --- |
| `*.md`（41 个文件命中） | **373** | 逐 crate：core 59 / build-method 59 / cli 55 / run-method 54 / studio 47 / plugin-host 30 / debug-method 29 / macro 22 / mcp 18 |
| `tools/`（6 个脚本） | **58** | `xirang-publish` 26（两张表）、`package-audit` 4、`visual` 3、`scale-audit` 2、`external-rehearsal` 1、`release-audit` 1 |
| `conventions/`（门禁本体 + 注释） | **20** | 含 `LIB_NAME_EXCEPTIONS`（`conventions/src/naming.rs:39`）与文档注释里的示例名 |
| `.github/`（2 个工作流） | **10** | 4 处在命令行位置、其余在注释/说明 |
| `build_method/scaffold/`（模板） | **6**（4 行要求文本） | `build_method/scaffold/project.rs:115`、`:118`、`:124`、`:127`；版本不是字面量——`build_method/scaffold/project.rs:103` 用 `env!("CARGO_PKG_VERSION")` 生成，模板只写 crate 名 |
| 其余 `.rs` 源（非模板、非门禁） | **8** | 其中真正按名引用的是 `mcp/protocol.rs:270`（serverInfo 名）、`studio/bin/xirang-dev.rs:142`（`--bin xirang-studio`），其余在注释/测试断言里 |
| 各成员清单的依赖要求行 | **19** | 见 7.2 |

口径说明：以上是"按 crate 名出现"的**站点数**，不等于"合并后必须改的处数"（许多处是历史叙述、README 章节标题、测试夹具字符串；脚手架与发布表是非改不可的两类）。逐条判定留给 boundary-architect。

---

## 8. 队长转达事项的独立复核（t7 交来两条 + 我自己的一处更正）

口径：转达不是事实，以下每条都用自己的装置复算过，结论只写事实 + 出处行号。

### 8.1 `conventions/src/mounting.rs:10` 的"29 处裸声明"是否过期 —— 是，今天是 33

- 我的独立口径（与 `conventions/src/mounting.rs` 的实现语义一致）：屏蔽注释与字符串字面量后取 `mod <name>;`，再回溯它上方的 `#[path = "…"]` 属性（穿过可见性与其它属性），用 `target_of` 的规则解析目标并要求文件存在。
- 我的实测 `[实测]`：全工作区 **332 条声明 = 裸 33 + 带 `#[path]` 299**，0 条解析不到；与 boundary-architect 的裸 33 完全一致（它给的合计 339 与我的 332 差 7，我无法在源码树里复现那 7 条——`git ls-files '*.rs'` 的 394 个文件全在内；差异来源未能定位，故只报我自己的口径）。
- 我先前在初稿里写的 335 是**错的口径**：那是"这一行以 `mod`/`pub mod` 开头"的行数，包含把 `#[path = "…"]` 写在上一行、`pub mod x;` 单独一行的声明（`core/src/registry_core.rs` 就是这种形状），所以它数的是"声明行"而不是"裸声明"。已在 G-20 更正并降级为 MINOR。教训：**散文里的数字必须先写清口径，否则两个口径都能给出"看起来对"的数**。
- 结论：那句散文今天过期（29 → 33）；`AGENTS.md:88-90` 已经选择不复述计数，`conventions/src/mounting.rs:10` 还留着，建议按第 10 节第 4 条处理。

### 8.2 `tools/xirang-publish`：真的没有"只发一个 crate"的路径吗 —— 工具内确实没有；但"手工 `cargo publish` 不受任何事后检查"这条要写清

1. **`req == ^workspace_version` 的强制确实存在，但它是 `--check-table` 的约束，不是发布路径的约束。**
   `tools/xirang-publish:462-512`（`metadata_requirement_problems`，`want="^$1"` 在 `:463`，比较在 `:505`）与 `:519-535`（`workspace_requirement_problems`）只在 `--check-table` 分支被调用（`:664`、`:669`），而 `--check-table` 在 CI（`.github/workflows/ci.yml:134`）与发布工作流（`.github/workflows/release.yml:92`）都会跑。配套的还有 conventions 门禁 `conventions/src/release_version.rs:281-303`（成员要求必须 == 工作区版本）。
2. **发布路径本身：没有选 crate 的参数，且版本被强制成工作区版本。** 全部旗标只有 `--publish`/`--yes`/`--allow-dirty`/`--verify-consumers`/`--check-table`/`--workspace-version`（`tools/xirang-publish:104-123`）；`--publish` 模式的循环是对 `levels` 整表逐层（`:693-800`），依赖未上 index 是硬错而不是跳过（`:704-708`）；上传前 `resolved_version "$crate"` 必须等于 `$workspace_version`，否则拒绝（`:743-749`）。
   - 旁路排查 1：`--workspace-version` 确实会短路（G-09），但它**什么都不上传**（直接 `exit 0`），所以它不是"只发一个 crate"的路径，而是"假成功"的路径。
   - 旁路排查 2：`--publish` 重跑时会跳过已在 index 上的版本（`:730-734`），因此"部分发布后补发缺的那几个"是**被支持的**——但那仍是同一条版本线，不是独立版本。
   - 旁路排查 3：在工具之外手敲 `cargo publish -p xirang-core` 永远是可能的；没有任何门禁能在事后发现它——tag 检查比的是 tag 与 `[workspace.package] version`（`.github/workflows/release.yml:81-89`），门禁比的是清单文本，**没有"谁上传了什么"的记录**。这是"工具内不存在，工具外不可检"的准确说法。
3. **`[workspace.dependencies]` 的继承拼法确实被支持（两处工具 + 一处门禁），但今天完全没被用到。**
   - `tools/xirang-publish:602-606`：成员里 `xirang-x.workspace = true` 被读成一条依赖边（进 `actual` 集合）；
   - `tools/xirang-publish:519-535`：`[workspace.dependencies]` 里的 `version = "…"` 被要求 == 工作区版本；
   - `conventions/src/release_version.rs:345`：`workspace.dependencies` 被算作要求段；`:244-246` 跳过带 `.` 的键（所以 `xirang-x.workspace = true` 不会被误读成"缺版本"）；`:331-338` 处理点表 `[workspace.dependencies.xirang-x]`。
   - 实测反证：根 `Cargo.toml` 里 `[workspace.dependencies]` **出现 0 次**（`grep -c 'workspace.dependencies' Cargo.toml` → 0），19 处内部要求全是各成员清单里的字面 `version = "0.1.6"`。也就是说这段支持代码今天是一条不执行的路径——对"合并/拆分"的取舍而言，"继承拼法已经能过门禁"成立，但"它已经在用"不成立。
4. **附带发现（影响同一取舍）**：`docs/roadmap-1.0.md:76-79` 把"依赖要求写 `0.1.0` …所以补丁版本不会连锁要求重发依赖方"记为保留九个 crate 的理由之一，而工作树里 19 处要求写的都是 `^0.1.6`，且两道门禁要求它们等于工作区版本（见 G-27）。按现状，**"独立发版"在清单层就被判红**；roadmap 记的口径与门禁的强制不一致。

---

## 9. 查了、干净（这一节只列我亲自核过、没有发现问题的项）

1. **门禁出厂树全绿**：`cargo test -p xirang-conventions --offline` → 99 passed / 0 failed（12 道门禁各自的"出厂树/工作流/清单满足规则"断言都在里面）。
2. **12 道门禁的"修前必须为红"都有 fixture**（逐条读过测试文件）：`conventions/src/bilingual.rs:162-217`；`conventions/src/doc_blocks_tests.rs:29-60` 与 `:119-190`；`conventions/src/doc_anchors.rs:377-528`；`conventions/src/features_tests.rs:62-147`；`conventions/src/lint.rs:268-443`；`conventions/src/mounting_tests.rs:15-125` 与 `:255-339`（挂载 + shim 棘轮）；`conventions/src/naming_tests.rs:25-204`；`conventions/src/purity_tests.rs:37-267`；`conventions/src/release_version_tests.rs:29-164`；`conventions/src/release_workflow_tests.rs:19-283`；`conventions/src/size.rs:300-499`。没有"只有出厂树断言、没有红例"的门禁。
3. **我自己的探针双向验证了两道门禁确实会红**：旧名的报告文件让 `doc_blocks`/`doc_anchors` 双红并逐条点名；改名后归零（见 G-06 判据）。
4. **门禁解析代码里的 panic/unwrap**：`workspace_root`/`workspace_members`/`lines`/各 `read_to_string` 都是"读不到就带路径 panic"（`conventions/src/lib.rs:64-79`、`:124-171`、`:313-317`），空成员列表是 panic 而不是空遍历（`conventions/src/lib.rs:165-169`）；门禁没有用 `unwrap` 吞掉解析失败。`doc_blocks` 的解析用内核 `guard_nesting` 挡栈溢出（`conventions/src/doc_blocks.rs:333-343`，并有 6 万层嵌套的红例 `conventions/src/doc_blocks_tests.rs:16-23`）。
5. **挂载遍历不跟随符号链接、跳过 `target/`**：`conventions/src/lib.rs:199-226`（`is_real_directory` 用 `symlink_metadata`）与 `conventions/src/lib.rs:295-309`；两条都有红例（`conventions/src/lib.rs:418-458`）。`build_method` 的生成计划写在 `<member>/target/xirang/out/`（实测 6 个成员各有该目录），而 `SKIPPED_DIRECTORIES = ["target"]` 按名字跳过任意深度，因此 `target/**/out/generated_lib.rs`（实测 38 份）不会污染任何门禁。
6. **脚本用法校验与退出码**：6 个脚本 `sh -n` 全过；`tools/xirang-publish` 未知参数 exit 2（实测 `--bogus`）、`--publish` 无 `--yes` 拒绝、`--check-table`+`--verify-consumers` 拒绝、`--publish` 与二者组合拒绝；`tools/xirang-visual` 未登记场景名 exit 2 + 列出可用场景 + 不写任何文件（实测 `--no-build zzz`）；`tools/xirang-external-rehearsal` 复制后检查"相对路径是否真的被改指"（`:71-79`），并在收尾行如实报告 CLI 那一半是否跑过（`:99-104`）。
7. **失败时不写产物**：`tools/xirang-release-audit` 先跑完 fmt/test/clippy/doc/build，再 `mkdir` + 写 `target/xirang-audit/…`（`:7-15`）；`tools/xirang-scale-audit` 与 `tools/xirang-visual` 的输出都在 `target/`；没有一个工具往检出里写文件。
8. **发布工作流的关键性质**：上传步骤自带肯定式 tag 守卫（`:122`、`:136`）；tag 检查读的是发布路径同一个读取器（`tools/xirang-publish --workspace-version`，`.github/workflows/release.yml:81`），没有第二份副本；`workflow_dispatch` 存在且无 `inputs.`；`permissions: contents: read`；`concurrency` + `timeout-minutes` 有界；无 `continue-on-error`；发布步骤在全部检查之后。
9. **`--offline` 政策一致**：`.github/workflows` 里 0 处 `--offline`（冷 runner 注册表为空）；本地默认带 `--offline` 的三个工具都用"显式空值可覆盖"的展开写法，CI 侧显式传空（`ci.yml:47`、`:116`）。
10. **`--help` 与子命令集合一致**：`target/debug/xirang --help` 列出 8 个子命令（`new/check/build/snippets/explain/grafts/studio/mcp`），与 `README.md:895-896` 一字不差；`--help` 关于 MCP 的措辞是"queries, plus authoring writes that preview unless `apply: true`"，与 `docs/audit-3p-2026-09-27.md` 记录的 CS1 修复一致（"read-only"已不存在于根 README/ROADMAP/中文 README）。
11. **MCP 工具名无漂移**：`mcp/` 里出现的 `xirang.*` 工具名与 `mcp/README.md` 列出的 18 个集合一致。
12. **双语覆盖**：43 个受追踪 `.md` 全部含中文（含 `macro/README.md`——它没有独立 `README.zh-CN.md`，但正文是逐段英中对照）；`conventions/`、两个 example 宿主没有 README，但它们在 `publish = false` 家族里、manifest 也没声明 `readme`，不是漂移；`docs/` 下的 `audit-*`/`design-*` 记录按文件名豁免。
13. **`doc_blocks`/`doc_anchors` 的覆盖范围无盲区**：43 个 `.md` 里只有 `studio/tests/fixtures/node-editor/README.md` 不在门禁范围（它不是工作区成员——`studio/tests/fixtures/node-editor/` 是仅源码的 fixture 宿主，从不编译，设计如此）。
14. **锚点判定落在"路径"上而不是"后缀"**：`doc_anchors.rs:172-195` 的 `resolve` 先精确匹配、再要求带前导斜杠的段边界后缀，`..` 段在比较前折叠（`:199-211`）；`0` 行号与越界行号都单独判（`:88-111`）。三条都有红例。
15. **本轮报告目录的命名规则已确认可行**：我看到时目录内 `.md` 全部以 `audit-` 开头（`audit-inventory.*`、`audit-lane-gates.md`、`audit-lane-studio.md`）；`lane-kernel.md` 当时尚未改名，我直接提醒了该 peer。

---

## 10. 结构观察（不下结论，供 boundary-architect / 队长取用）

1. **门禁的"归属"判据已基本落在语法位置，只剩三处仍靠字面拼写**：`size.rs` 的 `#[cfg(test)`/`mod x;`（G-04）、`drop_governed_lines` 的 `#` 前缀（G-08）、`release_workflow` 的 `contains` 黑名单（G-02）。这三处与"历史教训"里已修好的那些（`mask_non_code`、`mask_literals`、`drop_governed_lines`）是同一类问题的三个剩余出口，形态一致：**判定问的是"出现过某个东西"，而不是"这个东西在语法上生效"**。
2. **记录 vs 活文档的界线目前用文件名编码**（G-06）。这条界线要同时服务两个相反的诉求：审计记录要能自由摘录（不该被解析门禁管），活文档的锚点必须为真（要被门禁管）。按目录编码比按文件名编码更贴近意图，且与本轮产出的结构（每 lane 一个文件、都在 `docs/audit-<日期>/` 下）相符。
3. **`release_version` 的"一个来源"不变式只覆盖已发布成员**（G-05）。crate 数量越多，手工版本线的连带面越大（7.2：11 份清单 20 行）；被排除在外的成员（2 个宿主）是最容易漏的一角，因为它们恰恰是"给用户抄的模板"。
4. **数量在散文里出现的地方仍在漂**：`docs/roadmap-1.0.md:40` 的 14 项、`conventions/src/mounting.rs:10` 的 29 处、`conventions/src/size.rs:24-28` 的处理方式（不复述）是三种不同做法；`AGENTS.md:88-90` 已经确立了"不复述计数"的准则，门禁 crate 内部还没全对齐。crate 数量本身（9 个发布 + 3 个非发布）在 README、CHANGELOG、AGENTS、`tools/*`、CI 五处口径今天一致，可复算（7.1、7.4）。
5. **门禁 crate 的 12 道规则里，只有 5 道读"工作区级"事实（清单/工作流/树），7 道读文件名或文本拼写**；这决定了它们的误报面。`features.rs` 的做法（唯一手列 target + 反查清单）是当前最"窄而准"的样本，`naming.rs` 的"要求位置"（G-07）与 `lint.rs` 的 `deny(warnings)` 特例（G-01）则是两个反例。
6. **工具集的"自述 vs 行为"分三档**：`tools/xirang-publish`（名字 + 两张表 + 长注释，头部注释与实现基本同步，只有 `--workspace-version` 的互斥缺失 G-09）、`tools/xirang-visual`（逻辑严密、`--help` 截断 G-11）、`tools/xirang-package-audit`（实现已重写为推导式，头部注释仍描述被替换掉的手工表 G-21）。同目录三个脚本的自述可信度不同，这本身是"注释与实现一起改"这件事没有门禁的证据——AGENTS.md 把"规则"搬进了门禁，但"注释里的规则"没有对应机制。
7. **`examples/` 在两个方向上被门禁有意排除**（命名规则、`missing_docs`、双语文档、版本线），换来的是"宿主自己负责"的简单性；代价是它们成为唯一"手工维护且无人核对"的清单集合（G-05、7.4）。`tools/xirang-external-rehearsal` 会构建它们，但只按 path 解析——版本字段在那条路径上不参与任何判定。
