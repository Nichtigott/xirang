# t13 独立复核 — 门禁 E 路（conventions / tools / .github / examples / 双语文档）

- 复核对象：`docs/audit-2026-09-28/audit-lane-gates.md`（25 条发现：MAJOR 6 / MINOR 19 / CRITICAL 0）。
- 工作目录 `/home/nich/Moirai_N3/nichlink`；HEAD `cf0c378` + 未提交改动。本轮只出报告，**源码一行未改**；本文件之外的写入只发生在 `/tmp`（复核装置，见文末）。
- 复核纪律：**没有重跑作者的脚本**，也没有重跑作者的探针。我用一套**与作者不同的装置**做判定：
  1. 自建最小夹具树（`/tmp/gate-review/f-*/`），直接调用门禁 crate 的**公开 API**（`xirang_conventions::lint::missing_roots`、`release_workflow::findings`、`doc_blocks::findings`、`doc_anchors::findings`、`naming::findings`、`release_version::findings`、`size::oversized`、`purity::findings`），探针在 `/tmp/gate-review/probe.rs`；
  2. 「修前必须为红」按相反方向做：**在 HEAD 上跑，看它是否真的放行**（放行＝证实作者的假阴性指控；报出＝证伪）；
  3. 只读判定标 `[读码]`，其余标 `[实测]` 并给命令 + exit code + 关键输出行。
- 探针保真性控制：同一探针跑真实工作区，复现出厂结论——`lint_required` 11 条、`missing_roots` 空、`naming` 空、`release_version` 空、`size` 恰为钉住的那 1 个文件、`workflow_findings` 空。因此下面的夹具结论不是探针自身的怪癖。
- 结论汇总：**25/25 已判**（6/6 MAJOR + 19/19 MINOR，超出"MINOR 抽 1/3"）。**证实 25 / 证伪 0 / 部分证实 0 / 无法判定 0**；严重度**建议全部维持**（两处触发条件提示见 G-02、G-05 的备注）。另有作者漏掉的同类问题 3 条、以及 2 处需修正的表述（都在第 3 节末）。

| 原 id | 结论 | 严重度 | 复核手段 |
| --- | --- | --- | --- |
| G-01 | 证实 | 维持 MAJOR | 夹具 + rustc 实测 |
| G-02 | 证实（且比报告更宽） | 维持 MAJOR | 夹具 + text 级门禁 |
| G-03 | 证实 | 维持 MINOR | 夹具（自建 md） |
| G-04 | 证实 | 维持 MINOR | 夹具（705 行文件） |
| G-05 | 证实 | 维持 MINOR（触发条件见备注） | 夹具 |
| G-06 | 证实 | 维持 MAJOR | 夹具（同内容双文件对照） |
| G-07 | 证实 | 维持 MINOR | 夹具（注释里的包名） |
| G-08 | 证实 | 维持 MINOR | 夹具（属性 + `///`） |
| G-09 | 证实 | 维持 MINOR | 产品工具实测 + 读码 |
| G-10 | 证实 | 维持 MINOR | 读码（`nm … \|\| :`） |
| G-11 | 证实 | 维持 MINOR | 产品工具实测 |
| G-12 | 证实 | 维持 MINOR | 读码（GNU 专属拼法） |
| G-13 | 证实 | 维持 MAJOR | 夹具（release.yml 副本改 pin） |
| G-16 | 证实 | 维持 MAJOR | `--check-table` + 清单自推导 |
| G-17 | 证实 | 维持 MINOR | 读码 |
| G-18 | 证实 | 维持 MINOR | `git grep` |
| G-19 | 证实 | 维持 MINOR | 读码 |
| G-20 | 证实（含其自我更正） | 维持 MINOR | 自写第三套计数器 |
| G-21 | 证实 | 维持 MAJOR | 读码 + grep |
| G-22 | 证实 | 维持 MINOR | 探针实测 11 条根 |
| G-23 | 证实 | 维持 MINOR | 读码 + 探针 |
| G-24 | 证实 | 维持 MINOR | 读码 |
| G-25 | 证实（半条表述需修正） | 维持 MINOR | 读码 + CHANGELOG |
| G-26 | 证实 | 维持 MINOR | 读码 |
| G-27 | 证实 | 维持 MINOR | grep + 夹具（G-05 同装置） |

---

## 1. MAJOR 六条逐条复核

### G-01 · 证实 · 维持 MAJOR
- **原指控**：`missing_roots` 把 `#![deny(warnings)]` 当成"带着 missing_docs"，而该属性并不让 `missing_docs` 生效。
- **复核（两侧都实测，装置与作者不同）**：
  - 夹具 `/tmp/gate-review/f-g01`：两个成员，`deny/src/lib.rs` = `#![deny(warnings)]` + `pub fn undocumented() {}`，`bare/src/lib.rs` = 同内容但无属性。
    `./probe lint_required f-g01` → 列出两条（两个成员都在要求集合里），exit 0；
    `./probe lint_missing f-g01` → **只列 `bare/src/lib.rs`**，exit 0。
    即：`#![deny(warnings)]` 那一侧被认为"已经带着 lint"。
  - rustc 侧（`cd /tmp/gate-review/rustc-g01`）：`rustc --crate-type lib --edition 2024 --emit=metadata -o out/deny_warnings.rmeta deny_warnings.rs` → **exit 0，零输出**；同内容换成 `#![warn(missing_docs)]` → 输出 `warning: missing documentation for the crate`（exit 0）；换成 `#![deny(missing_docs)]` → exit 1。`rustc -W help` 里 `missing-docs  allow`。
  - 门禁那一条判定在 `conventions/src/lint.rs:225`（`trimmed == "#![deny(warnings)]"` → true）。
- **结论与影响链**：把 `#![warn(missing_docs)]` 换成 `#![deny(warnings)]` 之后，（a）`lint` 门禁绿，（b）rustc 无任何输出，（c）`clippy -D warnings` 没有可升级的 lint ⇒ 保护静默消失而三道防线全绿。指控成立。
- **严重度**：维持 MAJOR（"保护可被静默移除"属于自我描述与事实相反的那一类）。
- **备注**：作者引用的 `rustc -W help` + `/tmp/lint_probe.rs` 与我这两条独立测量一致；我的 `-o /dev/null` 初次尝试被 rustc 的临时目录拒绝（那是我自己的探针错误，不影响结论，已换成显式输出路径重测）。

### G-02 · 证实（且比报告更宽） · 维持 MAJOR
- **原指控**：上传步骤的 tag 守卫用 `contains` 判否，`startsWith(github.ref, 'refs/tags/') == false` 整类放行。
- **复核（真跑了门禁的 text 入口）**：
  - `./probe workflow_text f-g02/neg-bang.yml` → 1 条 `step 2 uploads under a condition that can be true without a tag ref: … !startsWith(…)`（对照组证明门禁在工作）；
  - `./probe workflow_text f-g02/neg-eqfalse.yml` → **`wf-finding-count: 0`**（`== false` 放行，证实）；
  - `./probe workflow_text f-g02/neg-eq0.yml` → **0**（`== 0` 同样放行）；
  - `./probe workflow_text f-g02/pos-true.yml`（`if: true`）→ 1 条（"without a positive tag guard"，说明漏判只发生在"子串齐备但语义取反"这一类）；
  - `pos.yml`（真守卫）→ 0。
- **判定点**：`conventions/src/release_workflow.rs:96` 的三个 `contains` 子串黑名单，对 `startsWith(...) == false` 全部为假、对 `TAG_GUARD` 为真 ⇒ 结构上必然放行。
- **严重度**：维持 MAJOR（出厂文件本身没写错，坏的是门禁强度，属"明确行为错误"而非静默数据损坏）。
- **备注（重要）**：作者给的最小修法"额外拒绝 TAG_GUARD 之后紧跟 `== false` / `== 0`"**不够**——我另测出两种更根本的等价否定写法同样 0 发现（见第 3 节 N-1：`!(startsWith(...))` 与 `! startsWith(...)`）。修法应改成"形状白名单"（例如要求条件折叠空白后每个 `&&` 项都在允许形状内，且 `startsWith` 之前不得出现 `!`/括号取反），这也正是作者自己在"最小修复方向"里列的备选。

### G-06 · 证实 · 维持 MAJOR
- **原指控**：record 豁免只看**文件名前缀**（`RECORD_PREFIXES = ["audit", "design"]`），`docs/audit-2026-09-28/lane-*.md` 被当活文档。
- **复核（自建夹具，同一目录两份同内容文件对照）** `/tmp/gate-review/f-g06`：`docs/audit-2026-09-28/lane-probe.md` 与 `docs/audit-2026-09-28/audit-probe.md` 内容完全相同（一段坏 Rust 围栏 + 一条越界锚点：夹具 `probe/src/lib.rs` 的第 9999 行）。
  - `./probe doc_blocks f-g06` → `db: <夹具>/docs/audit-2026-09-28/lane-probe.md 第 5 行 :: cannot parse string into token stream`（仅 lane-probe）；
  - `./probe doc_anchors f-g06` → `da: <夹具>/docs/audit-2026-09-28/lane-probe.md :: <夹具>/probe/src/lib.rs 第 9999 行 :: line 9999 is past the end of … (1 lines)`（仅 lane-probe）；
  - 两个 `audit-probe.md` 的对应发现数均为 0。
  - 函数级旁证：`./probe record? .` → `is_record(docs/audit-x/audit-a.md) = true`、`is_record(docs/audit-x/lane-a.md) = false`（实现 `conventions/src/doc_blocks.rs:133`）。
- **结论**：证实，且这是**双向**结论（同一目录、同一内容、只差文件名）。
- **严重度**：维持 MAJOR。这条不是"报告被弄红"的抱怨：它把"记录"逼成"活文档"，而作者自己在成稿过程中就为它改过文件名（改名前后各一次），说明它会持续影响每一轮审计的产物形态。
- **备注**：作者在报告里提到的 peer 文件名（`lane-kernel.md` 等）如今都已改名，故当前出厂树是绿的；这条仍然有效，因为它管的是**规则**，不是当下这一次。

### G-13 · 证实 · 维持 MAJOR
- **原指控**：持有 `CARGO_REGISTRY_TOKEN` 的 job 的 action pin 只由注释承诺，没有任何门禁读 `uses:` 的 ref 形态。
- **复核（把 release.yml 复制到夹具再改 pin，然后跑**出厂的**入口）**：
  - `cp .github/workflows/*.yml` → `/tmp/gate-review/f-g13/.github/workflows/`，把副本里 `actions/checkout@11d5960a…` 改成 `@v4`；
    `./probe workflow_root f-g13` → **`wf-root-count: 0`**；
  - 对照夹具 `f-g13b`（同一份副本，只把上传步骤的 `if:` 改成 `!startsWith(…)`）：`./probe workflow_root f-g13b` → **1 条**，`release.yml: step 12 uploads under a condition that can be true without a tag ref: …` ⇒ 证明门禁确实在这些副本上跑过；
  - 真实工作区对照：`./probe workflow_root /home/nich/Moirai_N3/nichlink` → 0。
  - 代码面：`release_workflow.rs` 里 `uses:` 的唯一去处是 `conventions/src/release_workflow.rs:244` 的本地 action 跟进（`./…`），全文件没有 pin/长度/十六进制判定（`grep -rn 'hex' conventions/src/release_workflow.rs` 无命中；`.github/`、`tools/`、`deny.toml` 里也没有 zizmor/actionlint 之类的 pin 检查）。
- **严重度**：维持 MAJOR（上游被重打 tag 会先于发布步骤运行，而这正是该 job 自己写下的威胁模型；`.github/workflows/release.yml:54-55` 是唯一的"防线"，而它是注释）。
- **备注**：`ci.yml` 保持 tag pin 是有意的（无秘密），这条不适用于它；修法应是"含 token 的 job 内 `uses:` ref 必须是 40 位十六进制"。

### G-16 · 证实 · 维持 MAJOR
- **原指控**：`docs/roadmap-1.0.md:255` 与 `tools/xirang-package-audit:37-40` 写的发布层级把 `xirang-mcp` 放在第 2 层，与活表矛盾，按它发布会在 mcp 处解析失败。
- **复核（清单自推导 + 工具交叉检查）**：
  - `tools/xirang-publish --check-table` → `dependency table matches the manifests (9 crates)`，**exit 0**（活表由清单推导并被核对）；
  - 活表 `tools/xirang-publish:86-94`：`core | macro+build-method | run-method | debug-method+plugin-host | mcp | studio | cli`（mcp 在第 5 层）；
  - 我自己从 mcp 的清单读依赖：`mcp/Cargo.toml:27` `xirang-build-method`、`:33` `xirang-run-method`、`:39` `xirang-debug-method`、`:40` `xirang-core` ⇒ mcp 必须在 run-method 与 debug-method 之后；
  - `docs/roadmap-1.0.md:255` 与 `tools/xirang-package-audit:37-40` 的顺序都把 mcp 排在这两者**之前**。
- **结论**：矛盾成立。"按它发布会失败"这一步是 cargo 语义推论（依赖必须先上 index），我未实际发布；该推论标为 `[读码+清单事实]`，不标实测。
- **严重度**：维持 MAJOR（`--check-table` 是唯一被交叉验证的顺序，而两处散文给的是另一个顺序，且 package-audit 的头注释还会把读者引向它——见 G-21）。

### G-21 · 证实 · 维持 MAJOR
- **原指控**：`tools/xirang-package-audit:37-53` 的头部注释在解释一张**已不存在**的手工表（`<crate>:<directory>:<deps>`，由 `for entry in $crates` 读取）。
- **复核**：
  - `grep -n "crates='" tools/xirang-package-audit` → **无命中，exit 1**；
  - 实现是推导式：`tools/xirang-package-audit:227-249` 用 `awk` 读根清单 `members`、跳过 `publish = false`、`name` 取自成员清单、依赖经 `metadata_field … deps`（`cargo metadata`）解析，再拼成 `$crates`；
  - 头部那段（`:37-53`）仍写着 "The directory is spelled out rather than derived because the mapping is not a rule… Dependencies are comma separated… read by `for entry in $crates`"。
- **严重度**：维持 MAJOR——按本队注释判据①"过时注释描述的行为与实现不符＝MAJOR"，且这段过时注释复述的还是**错顺序**（G-16），两处叠加会把读者带向一个会失败的发布顺序。
- **备注**：这条与 G-16 是同一事实的两个面（一处顺序错、一处表已亡），建议修的时候一起改，并在头部改成"crate 集合与依赖从工作区推导，顺序以 `levels` 表为准"。

---

## 2. MINOR 十九条逐条复核

### G-03 · 证实 · 维持 MINOR
夹具 `/tmp/gate-review/f-g03/docs/probe.md` 里放三种坏块：3 反引号（对照）、4 反引号、`~~~rust` 内嵌一行反引号后再接坏块。
`./probe doc_blocks f-g03` → **只有**一处发现，指向夹具 `docs/probe.md` 的第 5 行（3 反引号那一块的起始行），报错文本是 `cannot parse string into token stream`。4 反引号与 `~~~` 两类**一条都没报**（判定代码 `conventions/src/doc_blocks.rs:174` 只比前缀、`:188` 遇到任何围栏行即闭合）。
另：扫全部活文档（排除 record 文件）行首四反引号与行首 `~~~` 围栏 **0 处**，与作者"潜在而非活动"的判断一致（唯一命中在 `.dsh-meow/` 私人笔记目录，不在 `markdown_files` 的覆盖范围内）。维持 MINOR。

### G-04 · 证实 · 维持 MINOR
夹具 `/tmp/gate-review/f-g04`：700 行文件三份，分别用 `#[cfg(test)] #[path]`（对照）、`#[cfg(all(test))] #[path]`、`pub(crate) mod big3_tests;` 挂载。
`./probe size f-g04` → `oversized: probe/src/big2_tests.rs (700)`、`oversized: probe/src/big3_tests.rs (700)`；对照组 `big_tests.rs` **未被报**。
即两种等价拼法都被算作源码度量（假阳性方向），而正统拼法的豁免是生效的（夹具非空转）。判定代码 `conventions/src/size.rs:151`、`:152`、`:170`。维持 MINOR。

### G-05 · 证实 · 维持 MINOR（附升级触发条件）
夹具 `/tmp/gate-review/f-g05`：工作区版本 `0.1.6`；成员 `alpha`（发布）与 `host`（`publish = false`）写着同一条 `xirang-core = { path = …, version = "0.1.5" }`。
`./probe release_version f-g05` → **只有** `rv: alpha/Cargo.toml :: \`xirang-core\` requires 0.1.5, but the workspace version is 0.1.6`；`host` 无任何发现（跳过点在 `conventions/src/release_version.rs:115`；`tools/xirang-publish:567-572` 与 `:664-669` 同样只在 shipped 集合上核对）。两个宿主今天都是 `0.1.6`，所以当前不可见。
**备注**：建议维持 MINOR，但把它写进"抬版本线"的必查清单——下一次抬线时这两行会静默停在旧版本而全绿（正是"给用户抄的模板"最可能漏的一角）。若维护者希望门禁而不是清单来防它，这条就从 MINOR 升为发布前拦截项。

### G-07 · 证实 · 维持 MINOR
夹具 `/tmp/gate-review/f-g07`：工作流里一句注释 `# old invocation kept for reference: cargo test -p xirang-old`，模板里一句注释 `// xirang-old = { path = "..", version = "0.1.0" }`。
`./probe naming f-g07` → 三条发现：`xirang-old`（来自工作流**注释**）、`xirang-real-typo`（真实命令，对照组）、`xirang-old`（来自模板注释）。两个注释里的名字都被报，说明 `package_arguments`（`conventions/src/naming.rs:238`）与 `requirement_names`（`conventions/src/naming.rs:199`）都不剥注释。维持 MINOR。

### G-08 · 证实 · 维持 MINOR
夹具 `/tmp/gate-review/f-g08`：同一函数、同一 `#[cfg(any())]` 属性，两个版本——直接跟声明（对照）与中间夹一行 `/// documented`。
`./probe purity f-g08-plain` → 空；`./probe purity f-g08-docs` → `purity: Finding { file: "core/src/lib.rs", line: 3, token: "std::fs" }`。
即属性与声明之间多一行文档注释就会误报（共享助手 `conventions/src/lib.rs:247` 的 `!started` 判定只认 `#` 前缀，调用方 `conventions/src/purity.rs:83`）。维持在 MINOR：出厂树当前没有这种形状，但它是"属性+文档注释+条目"这种完全正常的拼法。

### G-09 · 证实 · 维持 MINOR
`tools/xirang-publish --check-table --workspace-version` → 输出只有 `0.1.6`，**exit 0**，没有表检查那一行（对照：`tools/xirang-publish --check-table` 会打印 `dependency table matches the manifests (9 crates)`）。提前返回点在 `tools/xirang-publish:289-292`，位于 `--check-table` 分支（`:541`）之前。
`--publish --yes --workspace-version` 的"什么都不上传却 exit 0"我**没有**实测（需要干净树/网络/令牌），标 `[读码]`：同一提前返回点在发布循环之前，作者所述成立。维持 MINOR。

### G-10 · 证实 · 维持 MINOR（读码）
`tools/xirang-release-audit:24` 是 `nm -C --defined-only "$artifact" 2>/dev/null | sort -u >"$symbols" || :`（失败被 `|| :` 吞掉），`:26-28` 的 else 分支把 `count` 设成字面量 `unavailable`，`:74` 无条件 `printf 'symbol_audit=%s\n'`。循环里没有任何"符号数必须非空"的断言。我**没有**造假的 `nm` 去跑整条审计（那会连带跑 fmt/test/clippy/build），故为读码佐证。维持 MINOR。

### G-11 · 证实 · 维持 MINOR
`tools/xirang-visual --help | tail -4` 的最后一行停在 `每个场景在 target/visual（…）写两个文件：<scene>.txt`——句子未完；头部注释块里紧接着的下一行才是剩下的半句，而 `--help` 的范围是 `sed -n '2,26p'`（`tools/xirang-visual:116`），首个代码行在 `:38`（`set -eu`），`Why tmux` 段落（约 `:32-36`）不出现。维持 MINOR。

### G-12 · 证实 · 维持 MINOR（读码）
`tools/xirang-external-rehearsal:64` = `sed -i "s|path = \"\.\./\.\./|path = \"$ROOT/|g" "$manifest"`（无备份后缀，BSD `sed` 会把 `s|…|g` 当后缀）；`tools/xirang-release-audit:70` 与 `:72` 用 `date +%s%N`（BSD `date` 无 `%N`，随后的整数运算会失败）。本机无 macOS，故不实测；CI 只在 Linux 单元上跑这两条（`.github/workflows/ci.yml` 的 `verify` 矩阵含 macos 但这两个工具不在其中）。维持 MINOR。

### G-17 · 证实 · 维持 MINOR
`docs/roadmap-1.0.md:40` 仍写"清单从 15 项开始…**现为 14 项**"；`conventions/src/size.rs:66-67` 的 `BASELINE` 只有一条 `core/src/registry_core/plugin/contracts/contracts.rs`（639），且 `./probe size .` 实测就是这一个。维持 MINOR。

### G-18 · 证实 · 维持 MINOR
`git grep -n 'version = "0\.1\.4"'` → 只有 `CHANGELOG.md:31` 与 `CHANGELOG.md:56`（两处都是描述**当前**会随版本线移动的要求）。工作树里的这类字面量是 `0.1.6`。维持 MINOR。

### G-19 · 证实 · 维持 MINOR
`conventions/src/features.rs:30` 的 `WORKSPACE_ONLY_TARGETS` 只有一项，且同文件 `:26-27` 的注释自己写明"这条规则需要在这里点名 target，而不是推导出来" ⇒ `AGENTS.md:155-157` 那句无条件的"a feature-gated target whose `required-features` is missing"只对这一个手列 target 成立。`AGENTS.md:151-162` 的清单里也没有 `naming` / `release_version` / `release_workflow` / `doc_anchors` 四道（这四道都存在，我本轮用它们跑了夹具）。维持 MINOR。

### G-20 · 证实（含作者的自我更正） · 维持 MINOR
我用**第三套**独立计数器（Python，自家词法状态机屏蔽 `//`、`/* */`、字符串字面量，再匹配 `mod <name>;` 声明并回溯属性行找 `#[path`）跑 `git ls-files '*.rs'`：`files=380 bare=33 pathed=299 total=332`。
与作者的更正后数字（裸 33）一致，也与 boundary-architect 的 33 一致；`conventions/src/mounting.rs:10` 仍写 "29 such declarations"。作者的自我更正（335 是"声明行"口径）成立：我按行首匹配得到的也是 335 量级，按声明口径是 33。
**备注**：作者在正文里引 `AGENTS.md:88-90`，实际那句 "the count is not restated here" 在 `AGENTS.md:85`（差 3 行，属散文引用精度，非门禁锚点）。维持 MINOR。

### G-22 · 证实 · 维持 MINOR
`./probe lint_required .` 在真实工作区返回 **11** 条：十个 `src/lib.rs`（含未发布的 `conventions/src/lib.rs`）加 `mcp/main.rs`；而 `conventions/src/lint.rs:15` 的模块级文档写 "the requirement covers the nine published crates' library roots"，同文件 `:27-38` 的函数文档才说清"除示例宿主之外的每个 crate 目录 + 那一个二进制根"。维持 MINOR。

### G-23 · 证实 · 维持 MINOR
`conventions/src/doc_anchors.rs:18-20` 写 "Anchors in audit and design documents are exempt"，实现是 `conventions/src/doc_blocks.rs:133` 的**文件名前缀**判定（探针：`lane-a.md` → false，`audit-a.md` → true）。措辞把"文件名约定"说成"文档类别"。维持 MINOR（与 G-06 同源，修 G-06 时一起改）。

### G-24 · 证实 · 维持 MINOR
`conventions/src/size.rs:125` 写"最深挂载链是 **6 层**"，括号里给的是四个文件（3 条边/4 个节点）。层数口径不明，且与列出的证据不符；这句话正是用来论证上限 16 的余量的。维持 MINOR。

### G-25 · 证实（半条表述需修正） · 维持 MINOR
`.github/workflows/ci.yml:54` 仍有 `# after the first crates.io release.`，而首次发布已完成（`CHANGELOG.md:14-15`：`0.1.0` 已发布、九个 crate 于 2026-09-25 一同上线；`:1200` 的 `[0.1.0] — 2026-09-25` 章节同证）。悬挂指令成立。
**需修正的一半**：作者说 `cargo package -p xirang-core`（`:55`）"与 package-audit 重复"。更准确的说法是：`tools/xirang-package-audit` 的**内容**半段用 `cargo package --list`（`:363`），而**打包**半段才跑真 `cargo package`（`:434`），且对依赖未上 index 的 crate 会跳过（`:428`）——core 没有版本化内部依赖，所以它确实会被 package-audit 打包一次；两者重叠的是打包那一半，不是全部。结论方向不变（注释确实过期），但"重复"要按这个口径读。维持 MINOR。

### G-26 · 证实 · 维持 MINOR
用户可见摘要：`tools/xirang-package-audit:449` `Contents are checked for every crate from today.`；文档：`AGENTS.md:196` `verified for all nine crates from today`；另有 `tools/xirang-package-audit:26` 的同一时态锚点。首次发布（2026-09-25）之后，"今天"不再指向任何可判断的日子。维持 MINOR。

### G-27 · 证实 · 维持 MINOR
`git grep -n 'version = "0\.1\.6"' -- '*/Cargo.toml'` → **19 条**要求行（build_method 1、cli 4、debug_method 1、macro 1、mcp 4、plugin-host 1、run_method 2、studio 3，+ 两个 example 各 1），与作者 7.2 的表格一致；`docs/roadmap-1.0.md:77-79` 仍写"依赖版本要求写 `0.1.0`（caret）…补丁版本不会连锁要求重发依赖方"。
"两道门禁强制要求 == 工作区版本"由 G-05 的同一装置实测：`alpha` 写 `0.1.5` 就被 `release_version` 报出（判定 `conventions/src/release_version.rs:290`），`--check-table` 的 `want="^0.1.6"` 同向。因此"只发一个 crate、其余不动"在清单层判红，roadmap 那句与实现不一致。维持 MINOR。

---

## 3. 作者漏掉、我顺手发现的同类问题

### N-1 · [MAJOR，与 G-02 同源但更根本] `release_workflow` 的 tag 守卫还有两种等价否定写法整类放行
- 复现（同一装置，text 级门禁）：`./probe workflow_text`，内容与 `pos.yml` 完全相同，只替换上传步骤的 `if:`：
  - `if: github.event_name == 'push' && !(startsWith(github.ref, 'refs/tags/'))` → **`wf-finding-count: 0`**；
  - `if: github.event_name == 'push' && ! startsWith(github.ref, 'refs/tags/')` → **`wf-finding-count: 0`**（`!` 与函数名之间的空格让 `contains("!startsWith")` 失效；该写法在 GitHub 表达式里与前一种同义，属表达式词法层面的空白，故我把它标为"很高置信、未在 GitHub 侧实测"）；
  - 对照：`!= true` → 1 条（被 `!=` 黑名单接住）。
- 判定：`conventions/src/release_workflow.rs:96` 的三个 `contains` 只能覆盖 `!` 紧贴函数名的那一种拼法。`!(…)` 是**确定合法**的 GitHub 表达式语法且语义取反，因此这一条独立于作者的 `== false` 存在。
- 建议：把它并入 G-02 修复（这正是"黑名单再加一条"不够的证据：至少还要覆盖括号取反与前导 `!` 的空白变体）。严重度同 G-02（MAJOR）。

### N-2 · [MINOR，假阴性] `takes_input` 的输入检测同样是子串匹配，bracket 拼法放行
- 复现：`if` 之外有一行 `run: echo "${{ github.event.inputs['publish'] }}"`（带上传步骤且 tag 守卫正确）→ `wf-finding-count: 0`（既不报 "reads an input"，也不报其它）。
- 判定：`conventions/src/release_workflow.rs:282` 的 `text.contains("inputs.") || 任一行 trim == "inputs:"`；`inputs['publish']` 两种都不命中。注意 `github.event.inputs.publish` 会因含 "inputs." 而侥幸命中（我实测 1 条），所以缺口只在方括号拼法。
- 建议：与 G-02 一起改成形状判定（或补 `inputs[`）；严重度 MINOR（需要作者偏门写法才触发）。

### N-3 · [MINOR，覆盖边界] `doc_anchors` 只理解 `.rs:NNN`，非 Rust 的行号引用完全不被检查
- 事实：`anchors()` 的扫描以 `.rs:` 为触发子串（`conventions/src/doc_anchors.rs:246`），因此 `Cargo.toml:12`、`tools/xirang-publish:88`、`AGENTS.md:85` 这类引用**永远不被门禁看**；而本轮报告里这类引用不少（作者的 G-16/G-17/G-20 都在正文里引了 `.md`/`toml`/脚本行号）。
- 这不算错误（模块文档把边界写成"`<file>.rs:<line>`"），但它是"散文里的数字没有机制对账"这条结论的更强形式：**只有 `.rs` 的行号有机制**。建议在 `doc_anchors` 的模块文档里显式写出这一点（否则读者会把"锚点门禁"理解成覆盖所有行号引用）。严重度 MINOR（文档边界，不是实现缺陷）。

### 需修正的表述（不算新发现）
- G-25 的"与 package-audit 重复"要按 `--list` / 真打包两半读（见上）。
- G-20 引用的 `AGENTS.md:88-90`：实际那句在 `AGENTS.md:85`。
- 作者给 G-02 的最小修法（只加 `== false`/`== 0`）**不够**（N-1），建议直接采纳它列的"形状白名单"方案。

---

## 4. 复核装置与保真性

- 探针源码 `/tmp/gate-review/probe.rs` → 二进制 `/tmp/gate-review/probe`（`rustc --edition 2021 … --extern xirang_conventions=<target/debug/deps 里的 rlib> -L target/debug/deps`）。子命令：`lint_required` / `lint_missing` / `naming` / `release_version` / `size` / `doc_blocks` / `doc_anchors` / `purity` / `workflow_text <file>` / `workflow_root <root>` / `record?`。
- 夹具：`/tmp/gate-review/f-g01` … `f-g08`、`f-g13`、`f-g13b`、`f-g02x`（守卫变体）、`f-g02i`（输入引用变体）、`rustc-g01/`（rustc 控制组）。全部在工作区之外；工作区内除本报告外**未新建/改动任何文件**。
- 保真性控制（同一探针跑真实工作区）：`lint_required` 11 条 / `lint_missing` 空 / `naming` 空 / `release_version` 空 / `size` 恰为 1 条钉住文件 / `workflow_root .` 0 条 —— 与出厂 `cargo test -p xirang-conventions` 的结论一致，因此上面的"放行/报出"都是门禁的真实行为，而不是探针的近似。
- 我未重跑作者的任何脚本；产品命令（`tools/xirang-publish --check-table`、`--check-table --workspace-version`、`tools/xirang-visual --help`）只用于取证，且都给了 exit code。
- 锚点自检：本文件里的每条 `x.rs:NNN`、`.github/…:NNN`、`AGENTS.md:NNN` 都在写下时用 `sed -n`/`grep -n` 打印过该行，行号对应工作树当前内容。本文件名为 `audit-verify-gates.md`（`audit-` 前缀，按队长广播），正文不含 Rust 代码围栏（只用行内代码）。
- 未做/无法做：G-09 的 `--publish --workspace-version` 组合（需干净树与令牌）、G-10 的假 `nm` 端到端（会连带跑整条审计）、G-12 的 macOS 行为（本机无 macOS）——三处已在对应条目里显式标 `[读码]`。
