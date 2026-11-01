# Changelog

English | [简体中文](#简体中文)

All notable changes to the XiRang workspace are recorded in this one file.
The nine crates are released as a single version line, so a change is described
once here instead of nine times: **per-crate changelogs are deliberately not
kept**. Release order and the reasoning behind the one-line release are in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Release state:** `0.1.0` is published. All nine crates went to crates.io together
on 2026-09-25, and the release workflow's last step built a throwaway consumer
outside the checkout and resolved all nine by version. `0.1.1` followed on
2026-09-26, the same way and with the same last step; `tools/xirang-package-audit`
now builds all nine packaged tarballs instead of skipping the eight whose
requirements had not reached the index. `0.1.3` followed on 2026-09-26 (run
`36236583886`, with `--verify-consumers` green in the same run); `0.1.2` was never
published, so its changes shipped in `0.1.3`, and the audit is back to `verified:`
all nine with `skipped: none`. The version line stays on
**0.1.x** while the design is still being deepened: each later release is a small
step (`0.1.6`, …). `0.1.5` followed on 2026-09-27 (run `36304022408`, `--verify-consumers`
green in the same run): it changes what a scaffolded manifest *requires* to the release
that generated it, which is a template fix rather than a cross-crate symbol — the
version moved because a published `0.1.4` CLI could not be corrected any other way.
"1.0" names the milestone in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md) rather than a published version.
`0.2.0` (2026-10-07) closed that gap: the three names are on the index, and
`tools/xirang-publish --verify-consumers` resolved and built them in a throwaway crate outside the
checkout. It was first published on 2026-09-29 and then **deleted** from crates.io; it was
**re-published on 2026-10-07** from the then-current checkout, so today's `0.2.0` is *not*
byte-identical to the deleted one (the checksum differs, and a lockfile pinning the old one will fail
to verify — `cargo publish` is immutable, which is why the re-publication needed the deletion first).
Prefer `0.2.1` or later once it exists.
Raising the line to `1.0.0` is a separate decision that would move every internal
`xirang-*` version requirement with it, and that step is what freezes the public
surface. The third-party audit's fixes below moved the workspace version and every
internal requirement together, which is what lets a cross-crate API change ship
without a red package audit; `0.1.3` moved the same way and for the same reason
(`mcp` now uses `xirang_toolchain::build_method::face_views`), and publishing it closed the
waiting state that change opened. `0.1.4` followed on 2026-09-27: its content is a batch
of MCP evidence tools (`xirang.mir`/`unified`/`impact`, the trace-driven `converge`,
recorded values and data edges in `xirang.trace`, and one cross-process identity
defect), but only one thing **required** the version move — `lexicon::TRACE_MODE_ENV` is
the first core symbol added after `0.1.3` shipped, `run_method` consumes it, and the
packaged `run_method` therefore cannot compile against the published `core 0.1.3`. `v0.1.4`
published on 2026-09-27 (run `36297635525`), with `--verify-consumers` green in the same
run: a throwaway crate outside the checkout resolved and built all nine by version, and
the package audit is back to `verified:` all nine with `skipped: none`.
**发布状态：** `0.1.0` 已发布。九个 crate 于 2026-09-25 一同上了 crates.io，发布工作流的最后
一步在本检出之外构建了一个一次性消费者，按版本解析到全部九个。`0.1.1` 于 2026-09-26 以同样的
方式跟进、同样有最后一步；`tools/xirang-package-audit` 现在会构建全部九个包的 tarball，而不再
跳过那八个依赖尚未进入 index 的 crate。`0.1.3` 于 2026-09-26 跟进（run `36236583886`，
`--verify-consumers` 在同一次运行里通过）；`0.1.2` 从未发布，它的改动随 `0.1.3` 一起走，包审计
也回到九个全部 `verified`、`skipped: none`。设计仍在深化期间，版本线保持
**0.1.x**：其后的每次发布都是小步（`0.1.6`……）。`0.1.5` 于 2026-09-27 跟进
（run `36304022408`，`--verify-consumers` 在同一次运行里通过）：它把脚手架清单**要求**的版本改成
生成它的那个发布，属于模板修复而不是跨 crate 符号——移动版本线是因为已发布的 `0.1.4` CLI 没有别的
办法被修正。而"1.0"是
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md) 里的里程碑名，不是已发布的版本。把版本线抬到
`1.0.0` 是另一个决定，需要连同每一处内部 `xirang-*` 版本要求一起移动——那一步才是冻结
公开面。下面三方审查的修复把工作区版本与每一处内部要求一同移动，这正是让一次跨 crate 的 API
改动得以随版本发布、而不让包审计变红的原因；`0.1.3` 以同样的方式、同样的理由移动（`mcp` 现在
使用 `xirang_toolchain::build_method::face_views`），而把它发布出去，正是关掉那次改动打开的等待态。`0.1.4`
于 2026-09-27 跟进：它的内容是一批 MCP 证据工具（`xirang.mir`/`unified`/`impact`、由 trace 驱动的
`converge`、`trace` 的值与数据边、一个跨进程身份缺陷的修复），而**必须**移动版本线的原因只有一个——
`lexicon::TRACE_MODE_ENV` 是 `0.1.3` 发布之后新增的第一个 core 符号，被 `run_method` 消费，因此
打包 `run_method` 时对着已发布的 `core 0.1.3` 编译不出来。`v0.1.4` 于 2026-09-27 发布
（run `36297635525`），`--verify-consumers` 在同一次运行里通过：检出之外的一次性 crate 按版本解析并
构建了全部九个；包审计也回到九个全部 `verified`、`skipped: none`。

`0.2.0`（2026-10-07）关掉了那个等待态：三个名字都在 index 上，而
`tools/xirang-publish --verify-consumers` 已在检出之外的一次性 crate 里按版本解析并构建了它们。
它曾于 2026-09-29 首发、随后被**从 crates.io 删除**；这一次是 **2026-10-07** 用当时的检出**重发**的，
因此今天的 `0.2.0` 与被删掉的那份**并非逐字节相同**（校验和不同，把旧校验和钉在 lockfile 里的消费者
会校验失败——`cargo publish` 不可变，这正是重发必须先有那次删除的原因）。能升到 `0.2.1` 或更高时，
优先用新版本。

## [0.1.0] — 2026-10-09

**Rename.** The project, its three published crates and its binaries are renamed from
NichLink / `nichlink-*` to **XiRang / `xirang-*`**（息壤）. This line starts at `0.1.0`
because the new crate names are new crates on the index.

**What changes for a consumer:**
- dependencies become `xirang-kernel`, `xirang-macro`, `xirang-toolchain`;
- binaries become `xirang`, `cargo-xirang`, `xirang-mcp`, `xirang-studio`, `xirang-dev`;
- the crate-root namespace constant is `XIRANG_NAMESPACE` and the state directory in a tree
  is `.xirang/` (was `.nichlink/`); environment variables are `XIRANG_*`;
- **identities change**: `NodeId = hash(namespace, relative source path, name)` and the
  namespace is the package name, so records written under the old name (graft plans, the
  adoption ledger, external plans) no longer resolve. Migration: rename `.nichlink` to
  `.xirang`, then re-run `check` and re-record.

**改名。** 项目、三个已发布 crate 与全部二进制由 NichLink / `nichlink-*` 更名为
**XiRang / `xirang-*`**（息壤）。新名字在 index 上是新的 crate，因此这条线从 `0.1.0` 开始。
对使用者的影响：依赖名改为 `xirang-*`；二进制改为 `xirang` / `cargo-xirang` / `xirang-mcp` /
`xirang-studio` / `xirang-dev`；crate 根命名空间常量改为 `XIRANG_NAMESPACE`，树里的状态目录改为
`.xirang/`，环境变量改为 `XIRANG_*`；**身份会变**（`NodeId` 的命名空间就是包名）⇒ 旧名字下写的记录
（graft 计划、采信台账、外部计划）不再解析；迁移＝把 `.nichlink` 改名成 `.xirang`，重跑 `check`
并重新登记。**本节之前的发布物，其真实名字是 `nichlink-*`** ✓。

## [0.2.2] — unreleased

Everything below landed **after** 0.2.1 was uploaded, so it needs its own version: 0.2.1 on the index
still spells the two host-facing modules `build_time` / `runtime`, and its kernel has no
`superseded_by_later`.
下面这些都在 **0.2.1 上传之后**才落地，因此需要自己的版本号：index 上的 0.2.1 仍把两个宿主面模块写成
`build_time` / `runtime`，它的 kernel 里也没有 `superseded_by_later`。

### Changed

- **One home per shared rule: the adapters, the identity spelling, the manifest paths (2026-11-01).** The
  in-place `modify` plan assembly was written out verbatim in seven language adapters; it is now
  `kit::adapter::modify`, and `Editable` / `EditHooks` carry only what a language actually decides —
  how to read a declaration, name its parent, render its arguments, and what a `kind` change renames.
  Java, C# and JavaScript share the element splitter and the dotted-constant test (Go keeps its own:
  backtick strings and `:` instead of `=`); C and C++ share the cc invocation; the kernel reads a
  leading identifier in one place; both record writers spell an identity through `NodeId`'s `Display`;
  and the CLI calls the build surface's host-edge rule instead of carrying a copy, with
  `manifest_path::relative` the one spelling both manifest writers use. Nothing a caller observes
  changes: every refusal sentence, edit range and plan shape is the one the adapters already produced.
  **一条共享规则只有一个家：适配器、身份拼写与清单路径（2026-11-01）。** 就地 `modify` 的计划装配曾在七个
  语言适配器里逐字写开；现在它是 `kit::adapter::modify`，`Editable` / `EditHooks` 只承载语言真正决定的
  东西——怎么读声明、怎么点名父级、怎么渲染实参、改 `kind` 会重命名什么。Java、C# 与 JavaScript 共用
  元素切分器与点分常量判定（Go 保留自己的：反引号字符串、用 `:` 而不是 `=`）；C 与 C++ 共用 cc 调用；
  内核在一处读前导标识符；两个记录写入方都经 `NodeId` 的 `Display` 拼写身份；CLI 调用构建面的宿主依赖边
  规则而不是自带一份副本，而 `manifest_path::relative` 是两个清单写入方共用的唯一拼法。调用方观察到的
  一切都不变：每一条拒绝句、每一个编辑区间与每一种计划形状都仍是适配器本来产出的那个。

- **The two host-facing modules are named after the cargo method they implement (2026-10-07, breaking).**
  `build_time` is now `build_method` and `runtime` is now `run_method`, so the host entries read
  `xirang_toolchain::build_method::run()` and `xirang_toolchain::run_method::host!()`. Nothing else
  moved: the modules are the same code, and the seven-module layout is unchanged. Update the two call
  sites a host has; 0.2.0 and earlier spell the old names.
  **两个宿主面模块改回"它实现的那个 cargo 方法"的口径（2026-10-07，破坏性）。** `build_time` 现名
  `build_method`，`runtime` 现名 `run_method`，宿主入口因此写成
  `xirang_toolchain::build_method::run()` 与 `xirang_toolchain::run_method::host!()`。除此之外没有
  搬动：模块还是同一份代码，七模块布局不变。宿主只需改这两处调用点；0.2.0 及更早写的是旧名。

### Fixed

- **Two real errors in generated manifests, and the overlap rule for graft cuts (2026-10-07).** The
  publishable shape wrote no version requirement for its own crates, and the scaffold wrote two
  dependency lines that disagreed with each other; both are fixed. Overlapping cuts used to be refused;
  they are now answered by declaration order — the later entry wins — with a note saying so, which is
  what a host that intentionally replaces one face twice would expect.
  **生成清单里的两处真错，以及重叠切口的判定规则（2026-10-07）。** 可发布形状没有为自己的 crate 写版本
  要求，脚手架写出的两行依赖彼此不一致；两处都已修。重叠切口过去被拒绝，现在由**声明顺序**作答——后一条赢
  ——并给出说明；这正是"故意对同一个面替换两次"的宿主会预期的行为。

- **A generated package no longer warns about its own cfg (2026-10-07).** The generated tree carries a
  rust-analyzer mirror behind `cfg(rust_analyzer)`, and the generated build scripts did not tell rustc
  that name was expected — so every package a split produced reported `unexpected_cfgs` on itself. The
  build scripts now emit the `rustc-check-cfg` line for it.
  **生成包不再对自己的 cfg 报警（2026-10-07）。** 生成树里有一层给 rust-analyzer 用的镜像，门控在
  `cfg(rust_analyzer)` 上，而生成的构建脚本没有告诉 rustc 这个名字是预期的——于是每次拆分产出的每个包都
  对自己报 `unexpected_cfgs`。构建脚本现在为它发出那行 `rustc-check-cfg`。

## [0.2.1] — 2026-10-07

### Added

- **Crate splits are operable from every authoring surface (2026-10-07).** A host declares at its
  package root which subtrees become crates of their own (`add_crates.rs`), and that declaration is now
  something the tools edit rather than something a reader only reads: `xirang crates --declare <name>
  --subtree <crate::…::SUBTREE>` adds a crate and `--undeclare <name>` removes one (both preview the
  exact text diff; `--write` applies), the bridge's `xirang.crates` gained `action: "declare"` /
  `"undeclare"`, and Studio's `CRATE PARTITION` screen lost its blind spot — `D` removes the selected
  declared crate. The edit is **text**, not a re-render: `add_crates.rs` is hand-written source the
  author also reads, so only the named entry moves and every other byte stays. Removing the last
  declared crate removes the file (the host is one crate again, which is what a declaration naming no
  crate would try to mean); `undeclare` refuses while that crate's package is still on disk (`revert`
  first, or the directory becomes an orphan nothing can reach); and a change whose resulting
  declaration does not plan is rolled back rather than left behind. `plan`/`write`/`release`/`revert`
  answer the other half — what the split would write, what is on disk, how large each package is, and
  what `cargo publish` would still want.
  **分区的声明层现在每个创作面都改得动（2026-10-07）。** 宿主在包根声明哪些子树各自成为一个 crate
  （`add_crates.rs`），而这份声明现在是被工具编辑的东西，不再只是被读的东西：`xirang crates --declare
  <name> --subtree <crate::…::SUBTREE>` 加一个 crate、`--undeclare <name>` 去掉一个（两者都先预览那段
  确切的文本差异，`--write` 才落盘），桥的 `xirang.crates` 多了 `action: "declare"` / `"undeclare"`，
  而 Studio 的 **CRATE PARTITION** 屏补上了它的盲点——`D` 移除选中的已声明 crate。改动是**文本**而不是
  重渲染：`add_crates.rs` 是作者也会读的手写源码，因此只动被点名的那一条、其余每个字节保持原样。移除最后
  一个已声明的 crate 会连文件一起移除（宿主回到"就是一个 crate"，也就是一份不点名任何 crate 的声明想表达的
  意思）；该 crate 的包还在磁盘上时 `undeclare` 会拒绝（先 `revert`，否则那个目录会变成谁也够不到的孤儿）；
  而改完之后规划不成立的改动会被回滚而不是留下。另一半问题——这次拆分要写什么、磁盘上有什么、每个包多大、
  `cargo publish` 还差什么——由 `plan`/`write`/`release`/`revert` 回答。

### Fixed

- **Three stale references left over from the nine-crates-into-three merge (2026-10-07).** The tmux
  visual check named the pre-merge fixture directory (`studio/tests/fixtures/…`, now under
  `toolchain/`), and the release and scale audits still ran `cargo … -p nichlink-run-method`, a package
  the merge removed — so the visual step could only fail and the release-audit job could only report a
  missing package. A caretaker's tooling is not compiled, which is why no gate saw them;
  `tools/nichlink-visual` also waits longer and prints a scene's stderr when its session dies, because
  the first three failures of that step were indistinguishable from "the app is slow to draw".
  **九→三合并遗留的三处引用（2026-10-07）。** tmux 视觉检查点名的是合并前的夹具目录
  （`studio/tests/fixtures/…`，现在在 `toolchain/` 下），而发布审计与规模审计仍在跑
  `cargo … -p nichlink-run-method`——一个合并已经删掉的包：因此视觉步骤只可能失败、release-audit 作业只
  可能报"包不存在"。工具不会被编译，所以没有任何门禁看见它们；`tools/nichlink-visual` 同时把等待放宽，
  并在会话死掉时打印该场景的 stderr——那一步最初三次失败与"程序画得慢"无法区分。

- **Platform assumptions the CI matrix had never exercised (2026-10-07).** macOS spells a temp path
  through a symlink (`/var` → `/private/var`) and Windows through `\?\C:\…` with `\` separators, so
  two spellings of one file are not string-equal there; the publish lock answers "is that pid alive"
  through `/proc`, which only Linux has, and the rule on other platforms is deliberately "assume
  alive" — the test asserted a takeover those platforms cannot perform. The assertions now compare
  through `Path` or normalize first, and the lock pin branches on what the platform can actually
  answer.
  **CI 矩阵从未跑到的平台假设（2026-10-07）。** macOS 的临时目录经符号链接拼写（`/var` →
  `/private/var`），Windows 则是 `\?\C:\…` 加 `\` 分隔符，因此同一个文件的两处拼写在那里并不逐字相等；
  发布锁经 `/proc` 回答"那个 pid 还活着吗"，而只有 Linux 有它，其它平台上的规则有意是"假定活着"——测试却
  断言了一次那些平台做不到的接管。现在断言改经 `Path` 比较或先归一化，而锁那条钉子按平台**能**回答什么分支。

### Added

- **The split writes into `crates/`, and its declaration is a function (2026-10-07).** A partition's
  generated packages no longer sit beside `host/` and `graft/` where a reader cannot tell generated
  from hand-written: they land under `crates/`, one directory for everything the split produced, and
  the declaration at the package root is now `pub fn add_crates() -> Shape { Shape::of("prefix",
  &[…]) }` — method names an editor completes, instead of field names the author had to remember.
  Both spellings are still read (`Shape { package_prefix: …, crates: … }` keeps working).
  **拆分写进 `crates/`，声明改成函数（2026-10-07）。** 生成包不再与 `host/`、`graft/` 挤在同一层
  （读者分不出哪个是生成的），而是落在 `crates/` 下——拆分产出的一切都在那一个目录里；包根的声明则写成
  `pub fn add_crates() -> Shape { Shape::of("前缀", &[…]) }`——作者笔下是编辑器会补全的**方法名**，而不是
  必须记住的字段名。两种拼写都照旧被读取（`Shape { package_prefix: …, crates: … }` 继续可用）。

- **`tools/nichlink-graft-matrix`: the graft shapes a real host has to answer (2026-10-07).** A cut on
  the root face with `full`, two overlapping cuts, and a re-partition of the host that carries both
  cuts — each checked against a real cargo, in CI.
  **`tools/nichlink-graft-matrix`：只有真宿主能回答的几种 graft 形状（2026-10-07）。** 根面加 `full`
  的切口、两条重叠切口、以及"装着两条切口的那棵子树被重新划分"——各用真 cargo 验一遍，进 CI。

### Fixed

- **Generated build scripts no longer carry `unsafe` (2026-10-07).** The claim a fragment compiles and
  the facade flag used to travel through `XIRANG_SHAPE_*`, which a generated build script could only
  set with `unsafe { std::env::set_var(…) }` under edition 2024. They are arguments now
  (`run_for_partition(…, Some("claims"), false)`), the shape lives on the build input, and a nail
  asserts the generated script contains `run_for_partition` and neither `unsafe` nor `set_var`.
  **生成的构建脚本不再带 `unsafe`（2026-10-07）。** 碎片编译哪些认领、以及"我就是 facade"这两件事，
  过去经 `XIRANG_SHAPE_*` 传递，而生成的构建脚本在 edition 2024 下只能靠
  `unsafe { std::env::set_var(…) }` 设置它们。现在它们是参数（`run_for_partition(…, Some("claims"),
  false)`），形状落在构建输入上，并有一条钉子断言生成的脚本含 `run_for_partition`、不含 `unsafe` 与
  `set_var`。

- **A claimed subtree that matches no published face is refused, not silently empty (2026-10-07).** A
  misspelled path (or a plan built from an earlier declaration) used to produce a crate with zero
  sources that compiles, ships, and answers every later question wrongly. It now stops, names the
  nearest published face, and says to run `xirang check` first.
  **点名不到的认领被拒绝，而不是安静地变成空 crate（2026-10-07）。** 写错的路径（或一份由上一版声明
  构建的计划）过去会产出一个零源码的 crate——它能编译、能发布，并在之后每个问题上都答错。现在当场停下、
  给出最近的那个已发布面，并说明先跑 `xirang check`。

- **Refusals about the declaration say who checks the paths (2026-10-07).** They claimed "the compiler
  checks the paths", which no compiler can do: a partitioned host no longer compiles the subtrees its
  declaration names. NichLink resolves them against the registration tree, and the message now says so
  — and names the spelling to write when the `::SUBTREE` marker is misspelled.
  **关于声明的拒绝说清了"谁在检查路径"（2026-10-07）。** 它们原本写着"编译器会检查这些路径"，而没有任何
  编译器做得到：被划分的宿主不再编译声明点名的那些子树。NichLink 对着注册树解析它们，文案现在这么说——
  并在 `::SUBTREE` 标记写错时直接给出该写的拼写。

- **The declaration reader took only the first `Shape {` (2026-10-07).** A declaration written as a
  function has two (`-> Shape {` and the literal), so the reader answered with the empty text between
  them and reported "no `package_prefix`". It now takes the last opening and accepts `Shape::of(…)` too.
  **声明读取器只认第一个 `Shape {`（2026-10-07）。** 写成函数的声明有两个（`-> Shape {` 与字面量），
  于是读取器答出两者之间的空文本、报"没有 `package_prefix`"。现在取最后一个开头，并且也接受
  `Shape::of(…)`。


## [0.2.0] — 2026-09-29, re-published 2026-10-07

### Changed

- **The publish surface is three crates now.** Batch 1 renamed the kernel (`core/` → `kernel/`,
  `nichlink-core` → `nichlink-kernel`, library `xirang` → `xirang_kernel`); batch 2 merged the
  seven thin execution surfaces into one crate, `nichlink-toolchain` (library `xirang_toolchain`).
  Nine published names became three: **`nichlink-kernel`, `nichlink-macro`, `nichlink-toolchain`**.
  发布面现在是三个 crate：批 1 改了内核名（`core/` → `kernel/`、`nichlink-core` →
  `nichlink-kernel`、库 `xirang` → `xirang_kernel`）；批 2 把七个薄执行面合并为一个 crate
  `nichlink-toolchain`（库 `xirang_toolchain`）。已发布的九个名字变成三个：**`nichlink-kernel`、
  `nichlink-macro`、`nichlink-toolchain`**。

- **Host usage.** A host writes `[dependencies] nichlink-toolchain` +
  `[build-dependencies] nichlink-toolchain`, calls `xirang_toolchain::run_method::host!();` at its
  crate root, and its thin `build.rs` calls `xirang_toolchain::build_method::run()`. The optional
  backends stay behind features (`wasm`, `process-tools`, `node-graph`, `authoring`,
  `prototype-fixtures`, `dev-supervisor`); `build_method`/`runtime` are on by default.
  宿主用法：`[dependencies] nichlink-toolchain` + `[build-dependencies] nichlink-toolchain`，
  crate 根部调用 `xirang_toolchain::run_method::host!();`，薄 `build.rs` 调用
  `xirang_toolchain::build_method::run()`。可选后端留在特性后（`wasm`、`process-tools`、
  `node-graph`、`authoring`、`prototype-fixtures`、`dev-supervisor`）；
  `build_method`/`runtime` 默认开启。

- **The old names are yanked (done 2026-09-29).** `0.2.0` resolved on the index and
  `tools/nichlink-publish --verify-consumers` is green, the eight merged-away
  names are yanked at **every** version they ever published (`0.1.0`, `0.1.1`, `0.1.3`, `0.1.4`,
  `0.1.5`) — forty yanks in all (yanking only the newest would let a fresh `= "0.1"` resolve back
  to an older un-yanked one; yank is reversible with `--undo`): `nichlink-core`,
  `nichlink-run-method`, `nichlink-build-method`, `nichlink-debug-method`, `nichlink-plugin-host`,
  `nichlink-studio`, `nichlink-mcp`, `nichlink-cli`. **`nichlink-macro` is kept** (it never merged).
  The executable command list, in order, is in `docs/merge-batch3-publish.md`.
  旧名 yank（**2026-09-29 已完成**）：`0.2.0` 在 index 上可解析、且 `tools/nichlink-publish
  --verify-consumers` 绿之后，把八个已合并掉的名字按**每一个已发布版本**（`0.1.0`、`0.1.1`、`0.1.3`、`0.1.4`、`0.1.5`）yank
  ——共四十次（只 yank 最新版会让新的 `= "0.1"` 退回更旧、未 yank 的那版；yank 可用 `--undo` 撤销）：
  `nichlink-core`、`nichlink-run-method`、`nichlink-build-method`、`nichlink-debug-method`、
  `nichlink-plugin-host`、`nichlink-studio`、`nichlink-mcp`、`nichlink-cli`。**`nichlink-macro`
  保留**（它没有被合并）。可按序执行的命令清单见 `docs/merge-batch3-publish.md`。

- **Wired back (2026-09-29).** Batch 2's six target-less test files (**20** `#[test]` functions, the `(b)` class) are targets again: nineteen as integration tests under `toolchain/tests/` (an external caller reaches the tolerant arm of `__control_object!` — the in-crate ban is on absolute-path calls from inside an expansion), and one mounted in-crate because it collects *this* crate's opted-in declarations. Wiring them back exposed the branch only they reached: a `collector: debug` declaration could not compile (`$crate::submit!`, not `$crate::call_evidence::submit!`), and the merged crate gained `extern crate self as xirang_toolchain;` for the expansions that carry `::xirang_toolchain::…` paths. The earlier probe evidence — both build faces' `--list` matching none of them, and a `compile_error!` in each file leaving `cargo check --all-targets` at exit 0 — is kept as the record of what the gap was.
  **已接回（2026-09-29）**：批 2 的六个没有 target 的测试文件（**20** 个 `#[test]`，即 `(b)` 类）重新有了 target：十九个是 `toolchain/tests/` 下的集成测试（外部调用者能到达 `__control_object!` 的宽容 arm——crate 内的禁令针对的是「展开里经绝对路径调用」），一个留在 crate 内挂载，因为它采集的是**本 crate** 已选择加入的声明。接回它们才暴露出「只有它们才会走到」的那一支：`collector: debug` 声明根本编译不过（应是 `$crate::submit!`，不是 `$crate::call_evidence::submit!`），而合并后的 crate 也补上了 `extern crate self as xirang_toolchain;`，供那些带 `::xirang_toolchain::…` 路径的展开解析。此前的探针证据（两个构建面 `--list` 命中 0、给每个文件插 `compile_error!` 后 `cargo check --all-targets` 仍 exit 0）作为「当时的洞长什么样」的记录保留。

### Added

- **Host-declared crate partitioning (`add_crates.rs`, 2026-10-07).** A host declares at its package
  root which subtrees become crates of their own — ordinary Rust (`Crate::named("widgets").at(&[…::SUBTREE])`
  inside a `Shape`), so the compiler checks the paths and an editor completes them. `xirang crates
  --check` prints the plan, `--write` materializes it (one package per declared crate, plus a *facade*
  that carries the cross-crate half: the graft table and the two contract assertions), and `--revert`
  takes it back. The **development** shape mounts a fragment's files out of the host package (an edit
  is visible in both crates at once; a workspace-root `--remap-path-prefix` keeps the identities
  identical); **`--release`** writes the same partition as packages that can be published — each
  carries the sources its own build reads, so no `#[path]` reaches outside a package and there is
  nothing to remap. Identities survive both shapes, because a face derives its identity input from
  `file!()` by dropping `CARGO_MANIFEST_DIR` and one leading `src/`. Generated packages are
  materialized into the host's workspace `members` (merged, never rewritten), `XIRANG_NAMESPACE` is
  the crate-root constant that names the identity namespace, and `tools/nichlink-partition-rehearsal`
  builds the workspace, standalone and release shapes in CI.
  **宿主声明的 crate 分区（`add_crates.rs`，2026-10-07）。** 宿主在包根声明哪些子树各自成为一个
  crate——就是普通 Rust（`Shape` 里的 `Crate::named("widgets").at(&[…::SUBTREE])`），因此编译器会查
  路径、编辑器能补全。`xirang crates --check` 打印计划，`--write` 物化它（每个声明的 crate 一个包，
  外加承载跨 crate 那一半——graft 表与两条契约断言——的 *facade*），`--revert` 收回来。**开发**形状把
  碎片的文件从宿主包里挂载出来（一次编辑两个 crate 同时可见；工作区根的 `--remap-path-prefix` 让身份
  保持一致）；**`--release`** 把同一次拆分写成可发布的包——每个包携带自己构建要读的源码，因此没有
  `#[path]` 伸到包外、也没有任何东西需要 remap。身份在两个形状里都保得住，因为注册面的身份输入由
  `file!()` 去掉 `CARGO_MANIFEST_DIR` 与其后的一个 `src/` 得出。生成的包会被物化进宿主所在工作区的
  `members`（合并、从不重写），`XIRANG_NAMESPACE` 是点名身份命名空间的 crate 根常量，而
  `tools/nichlink-partition-rehearsal` 在 CI 里构建工作区、独立与发布三个形状。

### Fixed

- **`xirang.verify` no longer reports half a verdict (2026-09-29).** It printed
  `verdict ok (the kernel accepted the tree)` from the static face alone, while `apply`, `usages`
  and `converge` judge the same tree through `load_registry`, which reads every face file rather
  than the build's active scope. A tree could pass the first and be refused by the second, so an
  agent reading only the first line carried a green light into a rejection. `verify` now prints a
  second line — `connector verdict: ok` or `connector verdict: rejected` followed by the same
  connector error tree — reusing `load_registry` and the existing reply bound. The `verdict ok`
  wording is unchanged.
  **`xirang.verify` 不再只报一半裁决（2026-09-29）。** 它原先只凭静态面打印
  `verdict ok (the kernel accepted the tree)`，而 `apply`、`usages`、`converge` 经
  `load_registry` 评判同一棵树，读的是每个面文件而不是构建的活跃作用域。一棵树因此可以通过
  前者、被后者拒绝，只读第一行的代理会把绿灯带进拒绝里。现在 `verify` 多印第二行
  `connector verdict: ok` 或 `connector verdict: rejected`（其后是同一份连接器错误树），
  复用 `load_registry` 与既有的回复尺寸上限；`verdict ok` 的措辞一字未动。

## [0.1.6] — never released (its content shipped in 0.2.0)

### Added

- **`xirang.grafts`: the external graft plans, and whether the host entry keeps them.**
  A plan under `.xirang/external-grafts/<selector>/graft.plan` is an authoring record the
  build never opens; when the host entry's `static_graft_plan!` does not name the slot a
  plan targets, the release prunes that slot and the record can never take effect. The
  build *warns* about that, and a long `cargo` log is where a warning goes to die — the CLI
  answered it since `xirang grafts`, and now the bridge does too, from one rule
  (`xirang_toolchain::build_method::graft_plan_rows`, extracted from the CLI in this release, so the
  two surfaces cannot answer differently). Each row names the selector, the logical path
  and replacement the plan targets, whether it covers the whole subtree, and either the
  declaration that keeps it (`cut \`…\` graft \`…\`` at its entry line) or
  `NOT declared by the host entry` — with a closing `unkept plans N` count, because that is
  what a reader has to act on. A declaration is about the **slot** a plan targets, not the
  implementation the plan selects — the build's own question — so the plan's graft and the
  declaration's are both printed and a difference is visible rather than silently accepted.
  An unreadable plan carries its reason instead of being skipped, and an unreadable entry
  leaves the state `unknown` rather than turning a missing answer into a false
  `not declared`. Measured end to end on a scaffolded host: with a declared plan the reply
  names entry line 6, and after adding a plan for `root/ghost` it reports
  `unkept plans 1` — which is exactly what `cargo build` then refuses with
  (`external graft plan \`ghost_fast\` targets \`root/ghost\`, which no declaration in the host
  entry names`), before anyone waits for a release to find out. Five tests; three measured
  red (make every plan declared, skip an unreadable plan, turn `unknown` into
  `not declared`).

- **`xirang.diff` now also compares the external graft records against the sources
  (`records: true`).** A record stores the identity it was written for, so a face that
  changed identity under an unmoved slot breaks it *silently*: the path is still there, the
  record still parses, and nothing in a text diff or a build log says so. The comparison
  reuses the tree diff's vocabulary, and every record lands in one of five buckets: `ok`;
  `undeclared` (the identity is in the tree, but no `static_graft_plan!` cut names its slot —
  the release prunes that slot, which is what `xirang.grafts` reports as `NOT declared by the
  host entry` and counts under `unkept plans`); `stale` (nothing in the tree has that identity
  or that path); `re-identified` (the path is there and the identity moved, reported as
  `old -> now`), which is the one thing no symbol graph can tell you about a graft; and
  `unreadable` (a record file that could not be read), counted rather than dropped. There is no
  separate bucket for a typed cut: the plan's path is always a logical path (every writer uses
  `registry.path_for`), and with the identity absent nothing resolves a typed declaration's
  module, so such a record is `stale` — and the reply prints the path it looked for rather than
  guessing. Measured on a scaffolded host with three records:
  `ok 1  stale 1  re-identified 1`, the last resolving
  `00000000000000000000000000000001 -> 43c1869f312b81a54005c13a1af5b8ae (root/slider)`. Two
  tests; the two load-bearing ones measured red (show the stale identity as "now"; let a
  typed cut fall through to `stale`).

- **`xirang.explain` renders the *overlay* projection (`overlay: true`), and the traversal
  behind it moved into `xirang_toolchain::build_method::overlay_projection`.** `explain` answered one face,
  or projected the tree the build scoped; neither said which slot a declared graft cut replaces,
  which the CLI's `explain --overlay` had answered since it existed. The bridge now answers it
  from the same traversal, so the two surfaces cannot disagree about which slot is replaced —
  the `graft_plan_rows` move of this release, one item later; `DeclaredGraft::form` moved with it
  so a cut's spelling is one rule too, and the CLI's JSON document is field-for-field unchanged.
  Each row names the logical path, kind and source plus either the replacement
  (`graft … full … form … (entry line N)`) or nothing, and slots the scope does not keep are
  listed as pruned. An unknown scope leaves `selected`/`kept` unknown rather than calling a
  surviving face pruned, and the reply carries the note saying this is a static projection — the
  live effective tree is `Registry::dump_effective` inside a host that links both registries.
  `node` and `overlay` are mutually exclusive. Measured through the bridge on a checked host with
  one declared plan: `slots 1 (replaced 1)` with
  `<- graft=button_fast full=false form=string (entry line 2)`, `pruned 1` (the auto scope selected
  only the declared slot), and `plan records 1 [kept by cut `root/button` … at entry line 2]`. Six
  bridge tests and three for the shared traversal; two mutations measured red across both surfaces
  (let a selected child's parent fall into `pruned`; read a missing scope as `pruned`).

- **A MIR artifact is now a *snapshot*, and `xirang.mir` diffs two of them
  (`against: "<path>"`).** A MIR dump is a snapshot of *some* tree and rustc's text format cannot
  say which, so the JSONL this tool writes carries a header naming the identity namespace and
  registry root it came from — the convention the trace artifact already uses. Two snapshots of one
  tree diff into the call-graph delta: added and gone `caller -> callee` relations (matched as
  pairs and deduplicated, so a moved MIR line is not a delta) plus function symbols, counted from
  the baseline forward. A snapshot of *another* tree is refused by name; an artifact that cannot
  name its tree (the ordinary `-Zunpretty=mir` dump) still diffs, and the reply says a comparison
  across two trees cannot be ruled out. Re-emitting another tree's artifact is refused rather than
  relabelled, and `xirang.unified` refuses to merge a foreign snapshot with this package's
  trace. Measured through the bridge: `jsonl: true` on a dump writes
  `{"kind":"snapshot","namespace":"e2e-016","root":"7da1…"}` as the first line, and the delta
  reports `relations added 1 gone 1  functions added 1 gone 1` with `crate::outer -> crate::new`
  added and `crate::outer -> crate::other` gone; a hand-written `somewhere-else` snapshot comes
  back `REFUSED: the two artifacts describe different trees`, and the text-dump pair answers with
  the unidentified caveat. Six bridge tests and one core snapshot test; three mutations measured
  red (swap the delta direction; skip the foreign-snapshot check; stop stamping the header).

- **`xirang.search` searches the tree, and each face hit says what the build thinks of it.** The
  tool answered "which file or function has this name" and nothing about the registry; it now
  matches logical path, `kind`, module and `registry_name` first — the spellings the other tools use —
  and annotates each hit `ok`, `added since build`, `re-identified` (with both identities) or
  `build unknown`, before the unchanged file and function hits. The verdict comes from
  `crate::tree_delta`, the one rule `xirang.diff` states (extracted here), so a face cannot be
  `ok` in one tool and `added` in the other. A root whose identity namespace cannot be learned
  still answers the source half and says the tree half is unavailable. Measured through the bridge
  on the same host: `face  root/slider … [ok]`; after adding a face,
  `face  root/gauge … [added since build]` under the stale-build line; and after changing a `kind`
  under an unmoved file, `[re-identified (daca0f7b… -> bc2df33a…)]`. Five tests; the shared rule
  was mutation-checked (every published face read as `ok`) and four tests went red across diff and
  search together.

- **Two kernel entry points replace the compact renderers the execution surfaces had to keep
  themselves: `xirang::authoring::parse::compact_admission` and
  `xirang::authoring::parse::compact_registration_rule`.** Studio's `admission_text` and
  `registration_rule_text` assembled the compact clause form themselves in
  `toolchain/studio/src/studio/app/source_index.rs`, because no public entry could render it from the
  owned value — the second-implementation family `FIXR-01` recorded, whose admission copy
  widened the deny gate the kernel had just fixed. Both entries are public now, Studio's call
  sites are pure delegation, and the kernel's own historical branches render through them, so
  the bytes consumers already read did not move: the single-list
  `allow_paths(`/`deny_paths(` branches and the registration rule's source-text branch
  (`rule_syntax_from_text`) go through the one renderer. Each entry is pinned from outside the
  crate by `core/tests/compact_admission_entry.rs` and
  `core/tests/registration_rule_entry.rs`, and Studio's `registration_rule_text_tests`
  (`the_call_site_renders_what_the_kernel_renders`,
  `the_call_site_carries_no_second_rule_renderer`) pins that its call site carries no renderer
  of its own.

- **`xirang::authoring::parse::try_render_requirements`, the entry that refuses a `requires`
  list it cannot render.** `render_requirements` rewrites an author's list from its parsed
  entries, so a malformed entry vanished from the result and a caller that wrote it back
  deleted a field the author had written. The strict sibling returns
  `Result<String, FaceParseError>` and names the entry it refused (`capability=>provider`),
  and the manifest renderer calls it. `render_requirements` stays `-> String` and lossy
  because that is the published `0.1.x` signature; its documentation now says so and points
  at the strict entry (audit `LGC-LG-28`). Pinned through the real `render_source` by
  `a_malformed_requires_entry_refuses_the_rewrite`, measured red with the caller put back on
  the lossy entry.

- **`xirang_toolchain::build_method::face_views_and_unreadable`, so an unreadable face is named
  instead of dropped.** `face_views` answered only with the faces that could be placed, so a
  registration file that does not parse and a node file whose syntax failed disappeared from
  the reply with nothing to look at. The new entry returns `(Vec<FaceView>, Vec<String>)` — the
  faces plus the paths it could not read, reusing `validation::face_syntax_errors` — and
  `face_views` delegates to it, so the two cannot disagree; its documentation states the
  `parent_resolved == false` half. Pinned by
  `a_registration_file_that_does_not_parse_is_named_not_dropped`, red before the fix with an
  empty unreadable set (audit `LGC-LG-11`).

- **`xirang::source::item_symbols` and `SourceItem`: the kernel owns the declaration
  vocabulary.** Studio's search view assembled symbol rows from a 13-entry `strip_prefix` word
  list of its own, so every keyword the kernel grew was a second place to forget and a
  bodyless `fn` in a trait was reported missing. `item_symbols` answers in source order with
  one `INTRODUCERS` table (`name`, `signature`, `line`, `is_function`; `let` deliberately
  absent, since a binding has no name a call graph can reach), and Studio renders exactly that
  list — its file carries no vocabulary of its own (`strip_prefix("` occurs 0 times). Pinned
  from outside the crate by `core/tests/b4_item_symbols.rs` (5 tests) and inside Studio by
  `source_rows_tests` (4), both measured red against the old word list (audit `STU-S-06`).

- **`xirang::authoring::parse::try_parse_requirements_owned`, the strict sibling that keeps a
  published signature published.** `parse_requirements_owned(&str) -> Vec<OwnedRequirementSpec>`
  is what hosts on the `0.1.x` line call, and it is lossy on purpose: it drops what it cannot
  parse. Tightening it would break an immutable version, so the strict judgement lives in a new
  entry and the published one keeps its exact signature and behaviour; both read the same
  `requirement_item` rule, so published, strict and validator answer from one implementation
  (audit `LGC-LG-28`). Pinned by `the_strict_requires_entry_refuses_a_malformed_entry` and
  `a_malformed_requires_entry_is_refused_not_dropped`, red under a mutation that delegates the
  strict entry to the lossy one.

- **`xirang::authoring::parse::try_rule_syntax_from_text`, the strict reading of a rule text.**
  `rule_syntax_from_text` stays tolerant and keeps its published `-> String` signature, because
  hosts on the `0.1.x` line call it: a `.require_exports(…)` written inside a comment *is* read
  as the rule (the real declaration disappears), and a list named by a constant — `&EXPORTS`,
  whose literals the text scan cannot see — degrades silently to `ANY`, which means "no
  structural requirement", so the author's declaration vanishes without a diagnostic. The
  strict sibling returns `Result<String, FaceParseError>` and names both the face file and the
  rule file it could not honour; `rule_syntax_for_source`, the one production call site, reads
  through it in this release, while a *missing* rule file is still `Ok("ANY")` — an absent rule
  is not a malformed one (audit `K-10`). Pinned through the production entry point and from
  outside the crate, measured red for both shapes, and re-measured under three mutations by the
  independent review (`docs/audit-2026-09-28/audit-verify-k10-family.md`). Additive on the
  unreleased `0.1.6` line: the published tolerant entry and the dependency table do not move.
All four are additive on the unreleased `0.1.6` line: no version bump, and
`tools/nichlink-publish --check-table` is unaffected because it compares the manifests. What
they do touch is `tools/nichlink-package-audit`'s second half — isolated packaging waits for a
crate's versioned `nichlink-*` dependencies to reach the index, so while `0.1.6` is unpublished
those crates are skipped (a skip, not a failure), and this batch deepens that wait because
Studio now calls kernel entries that are not published yet. The contents half already lists the
new modules, and the release order stays core first.
### Changed

- The CLI's `grafts` now renders rows computed by `xirang_toolchain::build_method::graft_plan_rows`;
  its JSON and text output are byte-identical (pinned by its own tests), and the rule that
  decides "is this plan's slot declared" has one home instead of two.
- `explain --overlay`'s projection now comes from
  `xirang_toolchain::build_method::overlay_projection`, so the CLI and the bridge cannot disagree about
  which slot a cut replaces; its JSON and text output are unchanged.
- **The naming batch (B7+B8): modules say what they mean, published names stay
  put.** Files that were named after their folder are named after their subject
  instead: `run_method`'s trace-snapshot directory, `authoring/context` and
  `authoring/face_file`; `mcp`'s `build_evidence`, `resolve` and `source_index`;
  `build_method`'s `discovery_cache`, `source_walk`, `registration_phase`,
  `face_syntax_check`, `discovery_node`, `node_identity` and `scope_faces`; the
  kernel's single-file modules `json`, `release` and `requirements` (the redundant
  `<dir>/<dir>.rs` nesting collapsed); `studio`'s `hot_zones` and `editor_launch`.
  The public *symbol* surface only grew: nine kernel names gained their real name
  and kept the historical one as a one-line forwarder, so nothing that compiled
  stops compiling and no shim re-export had to be dropped. The old→new mapping, the
  four mount-name families and the retained-name decisions are recorded in
  `docs/audit-2026-09-28/audit-naming-batch-plan.md` (with `AGENTS.md` carrying the
  rules and the retention table).

## [0.1.5] — 2026-09-27

### Fixed

- **A generated manifest now requires the release that generated it, instead of
  `0.1.0` forever.** Every scaffolded host carried `version = "0.1.0"` on both NichLink
  dependencies, from both dependency sources. Caret semantics hid it for four releases
  (`^0.1.0` accepts anything below `0.2.0`), so the literal was not merely stale — it
  was *load-bearing-wrong*: the moment the line reached `0.2.0`, `xirang new` would
  generate a host that cannot resolve, and the failure would read as a Cargo problem
  rather than a stale template. `dependency_specs` now writes
  `env!("CARGO_PKG_VERSION")` — the generating tool's own release, which cannot go
  stale — and the scaffold pin asserts against that value rather than against a
  literal, so the next release moves it for free. Measured: the published `0.1.4` CLI
  generated `version = "0.1.0"`; the `0.1.5` one generates `version = "0.1.5"`. Pinned
  red by putting the old literal back.

## [0.1.4] — 2026-09-27

### Added

- **`xirang.impact`: how far a change to one face travels.** `xirang.usages` gives
  the direct neighbourhood and `xirang.converge` one face's constraints; neither
  says what else a change touches. This walks the three dependency kinds this tree
  actually *declares*: a face's descendants (its registry owns them), the faces whose
  `requires` names a capability it provides, and the declared graft cuts that hand it
  over. Every reached node carries its shortest hop distance, *all* the reasons that
  reached it (a node that is both a child and a consumer must say both) and the chain
  of hops, because "why is this in my blast radius" is the question a repair asks.
  The walk is bounded by `depth` and by a visited set, so a capability cycle is
  counted — `cycles back to this face N` — rather than re-walked, and a face it did
  not reach is reported as unreached within the depth, which is not proof of
  independence. Measured through the bridge on a real host: a two-face package answers
  `hop 1 root/slider/gauge` with `because: child of this face's registry (kind Gauge)`
  and `because: consumer: requires \`cap.paint=>Slider\``; an unknown node is refused by
  name. Five tests, three measured red (drop the extra reasons, drop the cycle count,
  ignore the depth bound). Graft *records* and recorded traces naming the same identity
  are not traversed, and the reply says so.
- **`xirang.trace` now answers what a run *saw*, not only what it ran.** The call
  report says which functions were active; `values: true` reads the same
  identity-checked artifact and reports the recorded locals grouped by the frame that
  captured them — name, rendered type and value, role (`input`/`let`/`return`/
  `consumer`), `observed` or `unobserved`, and the callsite — plus the locals captured
  before any traced call, and the observed data edges with the transformation each one
  carries. That is the half a symbol graph structurally cannot have, and it is the
  half a cross-file bug is usually about. Two honesty rules are built in: a local or
  edge naming something the artifact does not contain is *counted* rather than
  dropped, so an incomplete artifact cannot read as a complete one, and `query`
  narrows this report exactly as it narrows the call report, saying so when nothing
  matched. Measured end to end on a real host: three captured values grouped under
  `frame 2 Slider::render` with their callsites inside the face's own file, and
  `count -> shown (transform)` / `shown -> value (used by paint::value)` as the data
  edges. Pinned by two tests, both measured red (one by folding every local out of its
  frame, one by disabling the query filter).
- **`xirang.converge` now also starts from a recorded run, not only from a face.**
  A bug report arrives in two shapes: an agent knows which face it is looking at, or
  it has a run that misbehaved and only the trace says what that run touched. With
  `trace: true` the tool reads the package's trace artifact (through the same
  identity check `xirang.trace` applies, absent and refused answers included),
  collapses the tree to the files that both declare a face and actually ran, names
  the frames that landed in each, counts the frames that fell outside any declared
  face, and ends with the read plan and a pointer to `xirang.converge node=<path>`
  for one face's constraints. Frames are matched to faces **by source file**, and the
  reply says so, because a face is a declaration and a frame is an active function —
  the honest boundary of this step, which is still the collapse that matters: a
  350-file tree becomes the handful of files that ran. Measured on a real host: a
  3-frame run over a one-face package answers `faces that ran (1 of 1 declared)` with
  `frames in a face 1 / outside any declared face 2` and a one-file read plan. The
  matching rewrites the compiler's path through its last complete `src/` segment, and
  that turned out to be load-bearing: the same run records `main` as `src/main.rs`
  and a frame inside the face's module as an **absolute**
  `/home/…/src/slider/slider.rs`, because the generated library mounts that module by
  absolute path. Pinned by three tests, the positive one measured red by reducing the
  match to an exact string comparison.
- **The MIR channel now has a reader, a writer, and a merge.** `xirang.mir` reads
  either a `rustc -Zunpretty=mir` text dump or the compact JSONL artifact, chosen by
  extension, and keeps the formats' asymmetry instead of flattening it: JSONL parses
  strictly, so a malformed line fails the whole read, while a text dump never fails
  because a line that is not a call is simply not a call. `jsonl: true` makes the tool
  the writer that existed nowhere in the workspace — Studio could render and parse
  this artifact and nothing ever produced one. `xirang.unified` then merges that
  graph with the package's recorded trace through
  `xirang_toolchain::call_evidence::UnifiedCallGraph`, the one place the two evidence sources
  are joined, so a call the trace confirms carries `evidence=Live` and *replaces* its
  compiler candidate rather than sitting beside it, while the rest stay `evidence=Mir`.
  An absent trace is a weaker answer and not a broken one: the merge still answers and
  labels every relation a compiler candidate. `mcp` gained a `nichlink-toolchain`
  dependency for it; the version line moved to `0.1.4` for a different reason (a core
  symbol `run_method` consumes), which is why this entry rides along rather than
  demanding the move itself. The producer of the *text* stays outside the bridge, and the tool says
  so rather than hiding it: that is `cargo rustc -Zunpretty=mir` on a nightly
  toolchain.
- `xirang.verify` closes the loop the other tools opened: it re-runs the kernel's
  registration validation over a package and reports the tree delta the run just
  published, so an edit is *confirmed* rather than merely written. It drives the same
  entry the CLI's `check` drives, which is why its verdict cannot drift from
  `xirang check`, and it refreshes the build evidence as a side effect. A failed
  verdict is the answer, not a tool failure: the diagnostics name the phase, node,
  source line, field and expected provider, and the reply stays `isError: false`.
  Writing it also pinned an API asymmetry that had been surviving review: `check_for`
  takes the package *directory* while `package_name` takes its manifest *file* — pass
  the file to the first and the pipeline looks for `<Cargo.toml>/src` and reports
  "is not a source directory". The CLI's local is named `manifest` and holds the
  directory, which is how the confusion hid.
- A scaffolded host now demonstrates the whole runtime-evidence chain, which is the
  half nothing in this workspace did. `xirang new` writes a `src/main.rs` that
  records one frame under the mode the environment asks for and, when
  `XIRANG_TRACE` (the mode) or `XIRANG_TRACE_FILE` (the path) is set, writes the
  artifact where every reader looks — `.xirang/traces/xirang.trace`. Without one
  of those variables it records and writes nothing at all, so the release path still
  collects nothing; the point is that the chain is visible and runnable. Measured end
  to end: scaffold → build → run (no file) → run with `XIRANG_TRACE=full` (a
  141-byte artifact with `mode=full` and one `main` frame) → `xirang.trace` renders
  the call tree. A library host gets the pointer instead of the demo, because it has
  no `main` to write at the end of. Writing this also caught a stale claim of my own:
  the bridge's "no project in this workspace records one yet" message is now false.

- **The build's evidence, the tree delta, and the runtime trace — three tools that
  answer from something other than source text.** `xirang.explain` reads the
  files the build published under `target/xirang/out` and answers what actually
  ships per face: whether the scope selected it and whether release pruning strips
  its symbols, plus identity, path, kind, source, module, parent and slot — or the
  whole scoped tree, bounded by `limit`. `xirang.diff` states the face-level
  delta between the sources now and the build's manifest: added, gone, and
  re-identified under an unmoved file (a `kind` change is an identity change, which
  no text diff sees). `xirang.trace` reads a recorded trace artifact and renders
  the headless call report — what actually ran — after checking the artifact's
  identity (namespace, registry root, every frame's node); a foreign artifact is
  refused by name rather than drawn, absence names the way to produce one, and a
  long report is truncated with its total named. Declared graft state and the
  contract/admission fields stay out: the first belongs to `xirang grafts`, the
  second needs a loaded registry and a build.
- Two read-path defects the NichUI test project measured are fixed. The scan no
  longer indexes `.xirang/`, so a deleted fact stops answering `status` and
  `search` from its own recoverable backup. And `xirang.callgraph` is bounded —
  definitions by `limit`, callers by a cap — names a multi-definition match as
  ambiguous and tells the reader to pass `path`, and labels its caller lists as
  name-matched. Measured before the fix: one `{"function":"new"}` reply against a
  350-file corpus was 4.5 MB, because 151 definitions each listed every call site
  of the name.
- `xirang.usages` closes the write path's read-back gap: the fields an agent can
  *set* through `xirang.apply` (preset, parts, localized names, exports,
  `requires`, `provides`, handle and part traits and contracts, registration rule,
  admission, flow, runtime checks) are now reportable, together with a face's parent
  and children as the tree has them and the capability tokens other faces mention in
  either direction. Capability matches are on declared tokens rather than a resolved
  graph and the reply says so; a hand-written module has no generated field list, so
  those faces are counted as unreadable instead of being shown empty. Writing the
  pins taught the authoring contract twice: a parent must own a registry before the
  kernel admits a child, and a requirement is written `capability=>ProviderKind` —
  the kernel refuses a bare capability name by name.
- `xirang.converge` joins those answers into the one an agent needs before touching
  a face: the build's scope and pruning verdicts, the tree's edges, the declared
  fields, whether each `capability=>ProviderKind` requirement is answered (named when
  it is), the files to read, and which tool carries the detail. The interesting half
  is the rejected case, and writing its pin found it: loading the package's own faces
  *validates* them, so a tree whose requirement has no provider makes every
  registry-backed tool refuse to load — which is exactly when an agent wants a
  verdict. `converge` now reports that rejection as its headline finding (the kernel
  already names the offending node and its source location) and still prints the read
  plan, because that needs only the source-derived tree. The write path has the other
  half: an edit that would orphan a descendant's requirement is refused by name, with
  the descendant and its `file:line` in the diagnostic.

### Changed

- `mcp`'s READMEs say which path `xirang.apply`'s `node`/`parent` take: the
  *registry* path `xirang.registry` reports (`root/control/button`), not the
  file's (`control/object/button/button.rs`). Aiming with the file-shaped one is
  refused by name, which a play session against a scaffolded host walked into.

### Fixed

- **`XIRANG_TRACE` is a lexicon constant now, so renaming it cannot break hosts
  silently — and it is the reason this release exists.** The collection-mode variable
  was a bare literal in the recorder while its sibling `XIRANG_TRACE_FILE` was
  published as `lexicon::TRACE_FILE_ENV`, so the shared-contract test guarded one name
  and not the other; hosts are told to *set these variables* and the scaffold template
  *writes one into generated code*. `lexicon::TRACE_MODE_ENV` now carries it,
  `trace_mode_from_env` reads the constant, and the generated `src/main.rs` asks for
  `lexicon::TRACE_MODE_ENV` the way it already asked for `TRACE_FILE_ENV`. It could not
  ride `0.1.3`: the symbol is in `core`, `run_method` consumes it, and the package audit
  builds each tarball against the **published** dependency, so it failed with
  `error[E0425]: cannot find value TRACE_MODE_ENV in module crate::registry_core::lexicon`
  against `nichlink-core-0.1.3`. That is the version rule working, not a defect, and
  this release is the move it asked for. Measured: a freshly scaffolded binary still
  records and writes its artifact under `XIRANG_TRACE=full`; renaming the constant is
  red in the lexicon pin, and putting the literal back in the template is red in the
  scaffold pin.
- **A second package built or verified in one process is no longer stamped with the
  first one's namespace.** `build_method`'s identity namespace was a first-write-wins
  process pin, and `check_for` set it from the package it was given — so in a
  long-lived process (the MCP bridge, a Studio session) the *second* package's
  published evidence carried the first package's namespace, and `xirang.diff`
  reported every face as re-identified. CI caught it in `verify`'s own test:
  `reidentified 1`, `ba9a8808…` on the run's side and `77fc3680…` on the sources',
  which the ids decode to `namespace=mcp-verify-broken path=label/label.rs
  name=Label` and `namespace=mcp-verify-healthy …` — the build side had been stamped
  by the test that ran first. The namespace is now scoped to the run, on the running
  thread (`run_as_package`), so a run's identities belong to the package being run
  while every other thread keeps reading what the process pinned; the process-wide
  pin stays for build scripts, and `freeze_test_namespace` keeps working. Pinned by
  `a_run_namespace_wins_over_the_pin_and_puts_it_back` and, through the tool an agent
  actually calls, `a_second_package_in_one_process_keeps_its_own_namespace`. Both
  measured red by neutering the scope, and the second by neutering the wiring
  separately — the unit pin tests the mechanism, the bridge pin tests that the
  mechanism is used.
- A preview no longer reports the operation as done. `xirang.apply` runs the real
  operation on a throwaway copy, and the executor describes what it did in the past
  tense, so a delete preview printed `would move …` and then, in the same reply,
  ``moved `button` to …`` — a sentence true of the copy and false of the project.
  The reply now scopes that sentence (`preview effect: …`) rather than dropping it,
  because it carries the parent identity and the old-to-new module names the diff
  does not; an apply still prints it bare. Found by playing the bridge against a
  scaffolded host: the real project had no trash entry and the reply said it had
  moved one.

## [0.1.3] — 2026-09-26

### Added

- The MCP bridge's write path, `xirang.apply`: an agent can create or rewrite a
  registration face through the **same authoring executor Studio uses**, so the
  kernel's admission, parent-rule, and topology checks run on the change instead of
  being re-implemented in the bridge. `mcp` gained a `nichlink-toolchain`
  dependency (feature `authoring`) for it; the version line stays `0.1.3`, which is
  still unreleased, so no new symbol needed a new version to be distinguishable
  from a published one. `action` is `add` or `edit`, `parent` takes a logical path
  (the one `xirang.registry` reports) or an identity, and **a request is previewed
  unless `apply: true`**: the preview runs the real operation against a throwaway
  copy of the package — copying, rather than writing and reverting, is what cannot
  leave a half-edited tree behind — and returns the file diff plus the registration
  tree that results; `apply: true` writes to the project and names the files it
  wrote and anchors the declaration it produced as `<path>:<line>`, the same shape a
  refusal uses for its `file:line:column`. Both ends of the preview are the same
  `run` call, which is why a preview cannot drift from the apply. `rename` changes `fields.module` and `delete` moves
  the module into NichLink's recoverable trash; both reuse the same preview. And
  `edit` is a **patch**: the face is read back first (`authored_face`, new public
  API), the request's fields are overlaid, and the complete set is handed to the
  executor — the executor's own contract is "rebuild from what you are given",
  which is right for an editor showing every field and wrong for an agent asking
  for one change, because it would blank the other twenty-two. Graft writes,
  plugins, project scaffolding, tree diffs, and the consistency analysis are still
  to come (`docs/roadmap-1.0.md` item 7). The executor's boundary is inherited
  unchanged and now documented: it rewrites the faces **NichLink generated** and
  refuses a hand-written one (`this module was not generated by NichLink`), because
  rewriting a file it did not author would discard content it does not model.
- `xirang_toolchain::build_method::source_layout` and `SourceLayout` are public API: the
  write path needs the same answer the build uses about where a package's faces
  live, and guessing `src/` there would author a face the build never reads.
- `nichlink-toolchain`'s `xirang.registry` tool: the registration faces this package
  declares, one row per face with its logical path, kind, source, and the
  `NodeId` the host compiled. The rows are derived by
  `xirang_toolchain::build_method::face_views` — the same derivation the CLI's `explain`
  uses — so `mcp` gained a `nichlink-toolchain` dependency and the version line
  moved to `0.1.3`. This closes a standing dishonesty in the bridge: it
  registered only five source-text tools while its documentation had advertised a
  registry query, so an agent re-derived the tree by grepping for macro names.
  The namespace is the part that hides a real design question: a package name
  *is* the `NodeId` namespace, so the tool resolves `XIRANG_NAMESPACE` first,
  then the name Cargo reports, and otherwise **refuses** — it never falls back to
  `xirang.default`, because every id below such an answer names a node no host
  compiled. Contract, admission, and registration-rule data still need the built
  face snapshots and are still absent from the bridge.
- A library target outside `src/` is read where it is. A package whose manifest
  declares `[lib] path = "host/lib.rs"` now has its registration faces read from
  `host/`, and each face's identity path is `host/…` — the manifest-relative path,
  because the declaration macros drop one leading `src/` and nothing else. The two
  decisions that had been one hardcoded constant, `<root>/src`, are now two bases
  resolved once (`toolchain/build_method/src/source_layout.rs`): the tree the walk reads and
  the base an identity path is relative to; `relative_display` and the
  `src.join(relative)` inversions keep working, and entry resolution — the one
  place that needs both — takes the layout. A `[lib] path` naming no file is a
  `face-layout` diagnostic naming it instead of a walk of some other tree; the
  manifest read is deliberately narrow (one key, one table, quotes, comments, the
  dotted `lib.path = "…"` spelling) and never spawns `cargo`, because the pipeline
  runs inside a build script. `[[bin]]` targets do not move the root: a package may
  have several and nothing chooses between them. `toolchain/build_method/src/entry.rs` came
  back **under** the 450-line ceiling on the way (its conventional-entry choice
  moved to `entry_default.rs`), so its size-ratchet entry is deleted rather than
  enlarged.
- `xirang_toolchain::build_method::package_name`: the Cargo-authoritative package-name
  read moved out of `nichlink-toolchain`, so the command line and the MCP bridge ask one
  authority instead of each carrying a copy. The CLI's commands behave exactly as
  before, and the two tests that pinned the private copy moved with it. Nothing in
  the build pipeline calls it: a build script receives `CARGO_PKG_NAME` from Cargo,
  which is the same value.
- `nichlink-toolchain`'s `PluginAdmission`: the host-side path from the plugin
  locks to a loadable artifact. It reads
  `<package_root>/.xirang/plugins/{official,user}.lock`, selects the manifest
  against that catalogue, verifies it (`verify_signed` for an official source,
  `verify_artifact` for a user one), and names the lane the earned assurance
  buys; `install` (Wasm) and `load_process` (process) do admission and loading in
  one step. Before this, **no in-tree code outside `core` called
  `verify_signed`**, so the Official lane was unreachable.
- Studio's trace **loader**, the consumer half of the artifact above:
  `TraceStatus { Absent, Loaded, Mismatch { reason } }` and `App::install_trace`,
  run once at startup. Discovery is `trace_artifact_path`, and four identity
  checks decide whether the artifact belongs to this session — the parser refuses
  an unsupported layout version, then the namespace, the registry root, and every
  recorded node that no longer resolves. A matching artifact puts the recorded
  values in the DATA panel; a refused one installs nothing, keeps the registry
  visible, and states the reason on the event line.
- `nichlink-toolchain`'s process adapter can narrow the child:
  `ProcessLimits::inherit_env` (default `true`) clears the environment,
  `ProcessProgram::environment` names the variables the child may then see, and
  `ProcessProgram::current_dir` chooses where it runs.
- The trace artifact's first slice in `nichlink-toolchain`: `TraceArtifact`
  (`from_trace`, `render`, `parse`, `into_trace`), `write_trace_artifact`,
  `read_trace_artifact`, and `trace_artifact_path`, plus the
  `TRACE_DIR`/`TRACE_FILE`/`TRACE_FILE_ENV` contracts in the kernel lexicon. A
  host can now write its recorded `CallTrace` as a versioned document that a
  separate reader can rebuild; the Studio loader is the next slice
  (`docs/design-trace-ingest.md` §3.5). `write_trace_artifact` takes the host's
  identity namespace as a parameter — for a host crate that is
  `env!("CARGO_PKG_NAME")` — because the process cannot read its own compiled
  namespace back and the reader's `XIRANG_NAMESPACE` is not it. The document is
  line-oriented text:
  values escape `\\`, `\t`, `\n`, `\r`, no raw control character reaches the
  file, and a malformed or unknown-key document is refused by line rather than
  guessed.

### Changed

- `nichlink-toolchain` depends on `serde_json` for `package_name`. It is the
  only JSON the crate reads, and `syn` was already there, so a host's build script
  pays for one more leaf rather than a second implementation of the rule.
- Documentation claims re-measured and corrected for the tool set and the trace
  loader: the root READMEs and both discussion introductions still said Studio
  rendered a built-in sample trace and had no ingest path, and that the bridge had
  no registry query. The MCP tool lists now include `xirang.registry`, and the
  trace sentence says Studio loads the artifact its host writes and reports
  `TRACE: none` without one.
- Studio's built-in trace sample is gone. With a loader in place it could only
  mislead, so the legend reads `TRACE: none` when nothing is loaded, `LIVE` when
  an artifact passed the identity checks, and `TRACE mismatch` when one was
  refused; the DATA panel drops `· built-in sample ·` and says `no trace
  attached` or the mismatch reason. A session can no longer label evidence it
  does not have.
- Studio reads the host crate's own package name and authors under it, instead of
  defaulting to `xirang.default`. The two ends have to agree — the host's build
  script stamps `env!("CARGO_PKG_NAME")` as the identity namespace — and a session
  that rebuilt the tree under a different name put every recorded `NodeId` (a
  trace, a graft record) out of reach. The name now comes from Cargo
  (`xirang_toolchain::build_method::package_name`), the same authority the CLI and the MCP
  bridge use, rather than from a literal `[package] name` line scan: TOML's dotted
  `package.name = "x"` form is the same table without a `[package]` header, so the
  scan found nothing and Studio authored such a project in the wrong identity
  domain. `XIRANG_NAMESPACE` still overrides everything verbatim, and a
  manifest Cargo names no package for (a virtual workspace root) still falls back
  to the documented default — authoring creates a tree, so a default is meaningful
  there.
- Studio's hand-drawn call-tree canvases are gone: `rataflow` (the `node-graph`
  feature, default on) is the only call-tree drawer, 1756 lines of canvas code
  and their geometry tests went with them, and the `g` (drawer) and `v` (layout)
  keys no longer exist. A build without `node-graph` keeps the panel and says
  which feature the tree needs instead of drawing nothing.
- Forwarding a mouse *release* to the `rataflow` widget, which emits
  `NodeClicked` on release while Studio only forwarded the press — so widget
  click-to-select could never fire. It does now.

### Fixed

- Studio derived the wrong identity namespace for a TOML manifest that names its
  package in the dotted `package.name = "x"` form: that is the same table as
  `[package]`, a line scan sees no header, and the session fell back to
  `xirang.default` while the host compiled under the real name — so the same
  faces existed in two identity domains and a trace or graft record written on one
  side could not resolve on the other. The literal reader is gone and Studio takes
  Cargo's answer, one subprocess per project adoption (the adopted namespace is
  cached in the project context, so no write path pays for it).
- The trace artifact's writer creates the directory it writes into. The
  documented host usage pairs `write_trace_artifact` with `trace_artifact_path`,
  and on a fresh project that path's `.xirang/traces/` does not exist yet: the
  write failed with "No such file or directory", and the message named the
  writer's own temporary file instead of the missing directory. Creating it is
  what the authoring executor's writer already does for
  `.xirang/external-grafts/`. The `run_method` READMEs now document the
  writer/reader pair and the `XIRANG_TRACE_FILE` override — the public API had
  no host-facing page, because the design document is not in the package.
- Studio's writers require the project the reader opened. `package_root()` still
  falls back — the session, then the environment, then the working directory — and
  that guess is reasonable for a read, because reading the wrong tree is visible.
  A write cannot afford it: a delete once resolved its root that way and moved a
  module out of whichever project the environment named. Every writer now goes
  through `with_selected_project` (or `selected_package_root`), which refuses with
  a named error when no project is selected, so no future call site can repeat the
  bug by forgetting to establish a context. Reads keep the fallback, and a launched
  session still adopts its project before the terminal is taken over, so nothing
  user-visible changes.
- `nichlink-toolchain` no longer aborts when the source tree is missing. A
  package whose `[lib] path` points outside `src/` — a legal Cargo layout — used
  to reach `expect("src directory must exist")` inside discovery: the build script
  died with exit 101 and `check --json` printed nothing at all, which is the
  contract that command keeps. It is a `face-layout` diagnostic now, naming the
  tree it looked for: structured callers get it back (`check --json` writes the
  document) and the build script fails loudly with the rendered text. A directory
  that disappears mid-walk is reported the same way instead of panicking.
  `check_for`'s doc now also says its first parameter is the package **root**,
  which is what every call site already passed.
- The MCP bridge's request framing is bounded: a line past `MAX_REQUEST_BYTES`
  (1 MiB) is refused with `-32600` and its remainder drained, so a hostile or
  buggy client can no longer make the long-lived stdio server allocate without
  limit. The drain stops at the newline — a plain read into a scratch buffer took
  the next frame with it, because a slice or a pipe hands over as much as fits.
  Requests must also carry `"jsonrpc":"2.0"`, and a member that is not a JSON
  object is answered with `-32600` rather than dropped silently; notifications
  stay silent even when their envelope is wrong, as the specification requires.
- `PluginCatalog::contains_manifest` compared the three optional provenance
  fields — signature, key fingerprint, revocation-list snapshot — for equality,
  so a seven-field official record (what a host's own plugin UI writes) could
  never match a signed official manifest, and an official plugin could not be
  admitted through a lock the host itself wrote. They are expectations now: a
  record that carries one pins it, a record that omits it leaves it to the
  signature check.
- Documentation claims re-measured and corrected: the in-flight design said
  Studio's sample records no locals (it records one, pinned by its own test), the
  post-fix audit's items 21/22/26/27/28 and its `build.rs` scope finding are fixed
  and now annotated as such, and the roadmap no longer calls the first publish the
  only remaining 1.0 item.
- The bridge's own descriptions still called it read-only after the write path
  landed: the `mcp` crate docs, four module headers, the manifest `description`,
  `xirang mcp` in the CLI's usage text, and four places in each of the two
  READMEs. Each now says what the bridge does — source and registry queries plus
  previewed authoring writes — so a `--help` line and a docs.rs page no longer
  contradict `xirang.apply`.

## [0.1.1] — 2026-09-26

### Added

- `deny.toml` and a `cargo-deny` CI job covering advisories, licenses, and bans.
- A CI job that compiles the non-default feature set and the feature-less
  surface: `cargo test --workspace --all-features --all-targets` and
  `cargo check --workspace --no-default-features --all-targets`. This is what
  first compiles `studio`'s `prototype-fixtures` tests, and it re-checks
  `plugin-host`'s `process-tools` through `--all-features`.
- A `LICENSE` copy inside every published crate directory, so each published
  package carries its own licence text (Cargo only auto-includes a crate-local
  `LICENSE*`). `conventions/` has none and needs none: it is `publish = false`.
- `macro/README.md` plus the matching `readme` field for `nichlink-macro`.
- `repository` and `homepage` on all nine manifests, `documentation` where it
  was missing (`nichlink-macro`, `nichlink-toolchain`), and
  `[package.metadata.docs.rs] all-features = true` on the crates whose
  non-default features are public API.
- `tools/nichlink-package-audit` now warns about and skips packages whose
  versioned NichLink dependencies are not on crates.io yet, and exits 0; the CI
  audit step no longer hides a failure behind `continue-on-error`.
- `studio`: a workspace-only `dev-supervisor` feature gating `nichlink-dev`, and
  the source-only fixture host package under `toolchain/studio/tests/fixtures/node-editor/`
  that the `prototype-fixtures` tests were written against.
- `tools/nichlink-publish`: publishes the nine crates level by level in
  dependency order, waiting for each level to reach the index before starting the
  next. It is a dry run unless `--publish --yes` is given.
- A `conventions` crate (`publish = false`) whose tests are the executable form
  of five rules that previously lived only in prose: kernel purity, module
  mounting, the 450-line ceiling as a ratchet, the `missing_docs` attribute and
  the `#[allow(missing_docs)]` ban, and parseability of the Rust blocks fenced in
  the READMEs and living docs. Each gate was shown to fail on a real violation
  before being kept.
- A CI step running `cargo test --workspace --all-features --doc`.
- `tools/nichlink-visual`: a visual check that runs the real Studio binary in a
  fixed-size tmux pane, drives it with keys, and saves what the terminal actually
  shows — plain text and, with `-e`, the SGR escapes. Scenes cover the home page,
  the search overlay, the add form, the call graph, the focused call tree, an
  arrow walk through that tree, and the data-flow panel; each scene waits for the
  app and then for its own screen before capturing, because capturing between the
  two sometimes saves the previous one. Output goes to `target/visual`, so a run
  leaves nothing in the checkout.
- A rendered call-tree test asserts the picture instead of the model: it renders
  Studio with the graph overlay open, cuts the call-tree panel out of the page by
  the corners that panel drew, and checks that every node inside the drawn window
  has its label on screen, that two nodes in one column do not overlap, that a
  higher level is drawn to the right of a lower one, that every one-column hop
  has its arrow on the callee's row, and that the header counts what the model
  holds. It prints the panel it checked, so the shape can be read as well as
  asserted.
- One JSON string encoder in the kernel, `xirang::json`, shared by the build
  diagnostics document, the MIR JSONL artifact and the generated editor
  snippets. It escapes exactly what RFC 8259 requires, and its tests assert the
  absence of raw control characters rather than a round trip, because this
  workspace's own parsers are lenient enough to accept invalid output.
- `lexicon` now owns `PACKAGE_ROOT_ENV`, `NAMESPACE_ENV` and
  `DEFAULT_NAMESPACE`, plus the pure `resolve_package_root` and
  `resolve_namespace` rules, so the environment names and the resolution order
  have one definition instead of one per surface.
- `tools/nichlink-package-audit` now checks package *contents* for all nine
  crates: every `src/**/*.rs` module and the manifest's declared README must be
  in the package. `cargo package --list` needs no registry, so this half runs
  before the first publish instead of waiting for it.

- `tools/nichlink-publish --verify-consumers`, the half of a release rehearsal
  that only the published crates can answer: it builds a throwaway crate outside
  the checkout and `cargo add`s all nine by version, so resolution runs against
  the index rather than against local paths, then `cargo check`s them together. A
  version the index does not serve yet is reported as exactly that, with the
  command to run after publishing, instead of as a failure of the script.
- `tools/nichlink-publish --check-table`, which compares the tool's dependency and
  level tables against the manifests that decide the real publish order: manifests
  against the dependency table, the dependency table against the level order, and
  no crate listed before something it depends on. It needs no network, so the
  `features` CI job runs it on every push as well as the release workflow.
- `.github/workflows/release.yml`, triggered by a `v*` tag: it stops before
  uploading when the tag does not name the workspace version, then runs the
  all-features battery, `--check-table`, the package audit and the release-artifact
  audit, and only then publishes in dependency order and verifies that a consumer
  outside the checkout can resolve the result. `workflow_dispatch` runs every
  check without publishing, which is how the first release can be rehearsed
  before the tag exists.
- The nesting guard now measures the shape that took a second pass to find: a
  **linear token run** folded into one nested expression or type with no delimiter
  or angle bracket to count. Seven shapes still aborted the process before this —
  `& & & …`, `&mut …`, `&'a …`, `* * * …`, `- - - …`, `! ! ! …`, `|| || …`, and
  `1 + 1 + …` — and the last one is why the guard is about the *tree* rather than
  only the parse: a binary chain is parsed by a loop, but the left-nested
  `ExprBinary` it builds recurses once per operator in `Drop`, so the process died
  on the way out of a parse that had succeeded. Runs are separated by `,`, `;` and
  brace groups (parentheses and brackets do not separate one, so `x.f().f()…` is
  counted), the limit is 1024 tokens — sixteen times below the smallest run
  measured to survive a 256 KiB stack — and `core/tests/nesting_budget.rs` feeds
  every one of the workspace's 318 Rust files to the guarded entry point to prove
  the limit refuses nothing this repository ships. `deep_input_tests.rs` carries
  one case per shape.
- `examples/control-button/tests/static_plan_allocations.rs`, which measures the
  release read path's heap allocations with a counting global allocator instead of
  inferring them from the code shape. Reading the built-in plan allocates nothing
  (0 allocations, 0 bytes across `builtin_static_plan()`, both slices, `find` hit
  and miss, `children_of`, and a full walk), and `overlay_static` — not free, since
  it clones the tree and builds its visited-cut set — builds no `GraftPlan` and
  costs less than the dynamic spelling of the same cuts (60 allocations against 77
  for one cut, 107 against 138 for two). The file holds exactly one test because
  the counter is process-global and libtest threads tests separately.
- `tools/nichlink-external-rehearsal`, which makes the external-path rehearsal
  reproducible: it copies the two example hosts outside the checkout, repoints
  their path dependencies here, detaches them from the workspace and builds and
  tests them from scratch in their own `target/`. The example host's 27 tests pass
  there, and the built CLI checks that external project (`check: ok`). It is what
  catches a host layout assumption that only holds in-tree, and CI runs it on one
  matrix cell.
- `toolchain/plugin_host/tests/wasm_table_cost.rs`: what a Wasm table actually costs the
  host, measured with a counting global allocator instead of estimated. A
  function reference is 8 bytes, so the default `table_elements` ceiling of 4096
  is 32 KiB, the hundred-million-entry table `a_huge_table_is_refused` pins would
  have been 762 MiB, and the limiter refuses that module after 5 225 bytes of peak
  allocation because it denies the table before it exists. The same check settled
  which limit does the work: wasmi's `EnforcedLimits::strict()` caps how many
  tables and element segments a module may declare, not how large one table may
  grow, so `WasmLimits::table_elements` is the only bound on table size.
- Two observation tests in `toolchain/studio/src/studio/app/tests/evidence.rs` that replace
  an argument about `CallEvidence::Live` with a measurement: with the shipped demo
  trace installed, no edge of a real project classifies as `Live` — and the test
  asserts that the trace is installed and that its nodes belong to no node of the
  loaded project, so it cannot pass by having no trace at all. Install a trace
  over the loaded faces and the same enumeration reports exactly the edge that
  trace recorded. `Live` is thus not dead code; it is unreachable for a real
  project only because Studio ingests no real trace yet.
- Budgets in `toolchain/run_method/examples/scale_audit.rs`: 40 µs per node registered and
  20 µs per node indexed, roughly eight times the measured values and overridable
  with `XIRANG_SCALE_REGISTER_US` / `XIRANG_SCALE_INDEX_US`, so an
  order-of-magnitude regression fails the run instead of only printing a larger
  number.
- Three steps in the `features` CI job for gaps no other job covered: clippy over
  `--all-targets --all-features`, the tmux visual check (`tools/nichlink-visual
  home graph tree-demo`, where each capture must be non-empty), and a
  `tools/nichlink-publish` dry run.
- `docs/performance-baseline.md`: what registration and indexing cost at 10k and
  100k nodes, what the release artifacts cost in bytes and defined symbols, and
  the commands that re-measure both.

### Changed

- The graph page is the call tree and the values its cursor's function ran with,
  two panes and nothing else: the relation column that listed callers and callees
  beside the tree is gone (the tree draws those as edges), and the side-by-side
  comparison page went with it, along with its `Ctrl-W` binding, its second search
  field, its `SearchState` fields and the `4` page key. `[` and `]` now move the
  tree's share of the page, and the divider drag clamps in the same range.
- Edges leave the caller's **bottom** and enter the callee's **top**, which is
  what makes the picture a tree: side ports had turned every hop into a sideways
  shuffle. The focus sits in the middle of the picture whenever anything in the
  tree calls it, and at the top only when nothing does — which is what the kernel's
  tree actually says.
- The graph page's letters are commands and its text is not: typing a letter used
  to edit the query from a page where `m`, `v`, `g` and `e` are commands, and the
  catch-all typing arm sat in front of `e`, so that key could never fire (rustc
  cannot see that conflict, because the arm it shadowed carries a guard). One stale
  `graph_focus == 2` arm, left over from the removed comparison pane, went with it.
  The query is edited in the list page, which `/` returns to, and the footer names
  every key of both pages.
- The fixture's call graph now has a tree in it — `paint_node_editor` fans out to
  `layout_panels`, `clamp_canvas_width` and `preview_canvas_width_traced`, and the
  panel measurers meet again at the clamp — so the visual check has a fork, a depth
  and a fan-in to show; `tools/nichlink-visual tree-demo` draws it.
- The widget is told what the kernel knows, and keeps its own viewport. Node text
  carries the cut counts the budgets left out (`←n`/`→n`), a `▶` for the cursor and
  a `◆` for the focus — marks, not only colours, because the report that started
  this was a cursor whose movement was invisible in a terminal with no colour —
  while each edge carries its evidence kind as a label. The widget is now cached on
  the app: rebuilt when the focus changes (then fitted), re-stamped when the cursor
  moves while its viewport and pan/zoom travel with it, and untouched in between, so
  a drag is not interrupted. Mouse events over its tree go to it, which is what
  gives the panel wheel zoom, drag panning and click-to-select; its nodes are not
  draggable, because the tree is a view of the model and a node must not drift away
  from the level and lane it stands for.
- Boxes are given the room their own labels need: a lane is as wide as its widest
  box and a band as tall as its tallest, and each box is centred in that cell.
  Fixed pitches could not survive the names — a 27-character symbol was drawn
  across its neighbour, which is the crowding that made the tree hard to read —
  and the widget's layout has a regression test that fails on any pitch that
  ignores the content (checked against a fixed one: the long box overlaps).
- The four arrow keys follow the **drawn** grid instead of the model's own axes, so
  they keep their printed meaning in both drawers: with the calls running down the
  screen `↓` steps toward a callee and `↑` back toward the callers, with them
  running across it those two roles go to `→` and `←`, a level step follows an edge
  that actually exists (and prefers the lane it is already in), and a sideways
  press takes the nearest lane of the same band. A press with nothing in that
  direction says so instead of moving somewhere else.
- The call tree is drawn by the `rataflow` node-editor widget by default, with the
  hand-drawn canvases one key (`g`) away and still compiled in: the `node-graph`
  feature is a default feature now, so the crate ships the widget look, and the
  canvases remain the ones that draw the kernel's own facts (evidence kinds, cut
  counts, cursor versus focus) and fit a seven-cell column.
- The call tree can also be drawn **top-down**, and a narrow panel switches to it
  automatically: the horizontal canvas needs one column per hop and an 80-column
  pane gives the tree 19 cells, so two boxes shrank to seven cells each and their
  names to tails — while the same tree laid out downwards spends one margin column
  on the edges and then gives every box the panel's full width, one band per hop,
  callers above and callees below. The switch is automatic below the width where
  the horizontal layout can still give two columns a readable box, `v` cycles
  panel-decides / top-down / left-to-right, and the title names the mode.
- The call tree is drawn as node boxes instead of a ledger. Each function is a
  bordered cell whose content wraps its **whole** name — the ledger clipped a long
  name to 15 cells and kept only its tail, so `preview_canvas_width` read as
  `…w_canvas_width` — the cursor's box takes the heavier border and the focus's
  the green one, cut counts are badges on the box, and edges leave a port on the
  caller's border, bend through the gap and end in `▶` with their evidence mark
  beside it. Columns are as wide as the widest name in them and are narrowed
  rather than dropped when the panel is one cell short, so the callers of the
  function being read stay on screen. A legend on the bottom border names the
  markers, and clicking a box selects that node. The packing reserves room for a
  neighbour before the cursor's own column takes its width, tightens the gap from
  five cells to three when that is what buys the second column, and abbreviates a
  name to its tail with `…` rather than wrapping it into a five-row sliver — all
  three because a tree drawn as one box has no edges, which is exactly what a
  narrow terminal used to show. `[` and `]` move the same split the divider drags,
  and a panel too narrow even for the tight packing says `[ ] widens` in its title
  instead of letting the reader believe the function has no neighbours.
- The New Project and plugin forms name their rows too:
  `app::state::new_project_field` and `app::state::plugin_field` hold one constant
  per row beside the label the form prints, so a `values[0]` at a call site became
  `values[new_project_field::DIRECTORY]`, and the two label tables the renderers
  carried are gone with them.
- Package READMEs, `docs/discussion-introduction*.md`, and the workspace
  layout in the root README were corrected to describe the current nine-crate
  workspace.
- `XIRANG_ENTRY` is now resolved once per build and drives both scope pruning
  and the generated `BUILTIN_GRAFT_CUTS` table; a value that names no file fails
  the build instead of letting one reader fall back to Cargo's `main.rs`.
- A build now **fails** when an external graft plan's target slot is named by no
  declaration (it used to be a `cargo:warning`), and the message carries the
  `static_graft_plan!` clause to paste. A declaration whose `#[cfg]` is off in
  this build still counts, so a legitimately gated slot is not reported.
- `apply_recorded_grafts` prints one `warning:`/`note:` line per report, so a
  record that is skipped can no longer be silent.
- A sibling range written from the later name to the earlier one is refused with
  the ordering rule (`registry_name` order) instead of being silently swapped.
- `nichlink-toolchain`'s process backend now enforces the limits it declares.
  The request frame is written from its own thread, stdout and stderr are drained
  from theirs, and the deadline bounds the whole call, so
  `ProcessLimits::max_output_bytes` (1 MiB by default) is the real ceiling. It
  used to be the ~64 KiB pipe buffer, and exceeding it reported `Timeout`.
- `petgraph` moved from 0.6 to 0.8. It backs `CallGraph`'s private fields only,
  so no public API changes.
- `toolchain/studio/src/studio/ui/forms.rs` mounts `face_fields` with `#[path]`, like its
  four siblings. This was a style difference rather than a defect: both spellings
  resolve, because a `#[path]`-loaded parent resolves children by directory.
- Studio, the authoring executor and the MCP bridge resolve the project through
  one kernel rule instead of three copies. Behaviour is unchanged; the one
  deliberate difference is stated where it lives: the bridge's last-resort
  fallback stays the working directory, because a stdio bridge is started inside
  the project the agent works on, whereas a compiled-in manifest path belongs to
  the machine that built the binary.
- `wasmi` moved from the 0.42.1 line to a `1.0.9` floor (the lock resolves
  1.1.0). The 0.x caret requirement could never reach the fixes that matter for a
  sandbox — the memory read/write integer overflow, growing memory past the
  system limit, `rem_s(MAX, -1)` trapping where the spec demands 0, and the
  loop-local and wide-arithmetic miscompilations. One call site changed:
  `Linker::instantiate` plus `PreInstance::start` became
  `instantiate_and_start`, which runs the same `start` function, still with fuel
  set before it. Fuel remains the pre-2.0 kind, so `fuel_per_call` keeps its
  meaning; its doc now says the unit belongs to the engine and a major upgrade
  can re-scale it.
- Five duplicated rules now have one implementation each: the cut-endpoint
  rendering shared by the manifest writer and the CLI report, the CLI's JSON
  error document and command error text, Studio's plan path (from the runtime
  loader's `graft_record_root`), the PascalCase kind derivation (the kernel's
  `pascal_case`) and the `::`-bounded symbol match (the kernel's `same_symbol`,
  now public). Two of those had already diverged.
- **The registration-face authoring surface is now the size of what an author
  actually decides.** `handle` and `params` are the kind by rule and
  `registry_name` is the declaring module's last path segment, so
  `__control_object!` and `__external_object!` keep one arm each and the three
  spellings of one type are gone from declaration files; the minimum face is
  `control_object! { kind: Button }`. Trait labels are derived from the
  compiler-checked contract paths by one rule shared by all three readers (the
  file view in `FaceSyntax::string_list`, the proc macro
  `__face_trait_labels_or!`, and the applier `apply_trait_contract`), so a label
  can no longer disagree with the path the compiler checks. `FACE_FIELD_ORDER`
  went from 29 accepted keys to 24.
- **Studio's add/edit form is 26 slots, not 30**: 22 author rows followed by one
  read-only machine-value strip, with the slots named in `authoring::face_field`
  so the Studio write paths no longer index the array with bare literals. Four
  slots left with the fields they described. Help text was corrected wherever it
  contradicted the code: the tree slot claimed an override that the
  registry-name rule forbids, `external source note` said "provenance only" for
  a field registry resolution reads (it is now labelled `dependency registry`),
  and the kind/parts rows were marked derived while the form edits them.
- The call report states one fact once: `handle=` sat next to `kind=` on every
  reported function, and the search matched `params` as a second key for the
  same string. Both are the kind by rule, so only the kind is printed and only
  the kind is searched; `RegistrationInfo` keeps both fields for hosts that read
  them.
- A typed graft cut now proves the output types across the two faces it joins:
  the build emits `assert_contract::<{cut}::__Preset, {graft}::__Parts>()` beside
  `BUILTIN_GRAFT_CUTS`, so a replacement face whose parts do not supply the
  preset's parts fails the build. String and selector cuts emit no assertion, and
  the code says so where the accumulator is filled rather than falling back to
  the deleted string comparison.

### Removed

- The author-written `expected_output` / `actual_output` pair. It was compared
  only with itself in two places (`release.rs` and
  `build_method::check_output`) and never described the real types: the button
  face declared `"ControlFrame"` while its `NoPreset`/`NoParts` output is `()`.
  The type-level fact it pretended to state is now proven by the per-face
  `assert_contract` and by the per-cut assertion above. The fields, both
  comparisons, the cache keys, the authoring plumbing and the Studio slots are
  gone.
- `authoring::FACE_FIELD_NAMES`: a 30-label table read only by tests and a
  near-duplicate of the presentation labels. The layout's names live in
  `authoring::face_field` now, and the tests that used the table assert the
  stronger property instead: the slots are dense and each has presentation
  metadata.
- Rows Studio printed more than once for one fact: a `values` runtime-snapshot
  row and a `declared` file:line:function row in the search details panel (the
  file is already the `path` row), plus the `params`/`handle` rows that repeated
  the `kind` row in both details panels. `params`, `handle` and `kind` are one
  string: both macros expand them from `stringify!($kind)`.

### Fixed

- The third-party audit recorded in
  [`docs/audit-3p-2026-09-25.md`](docs/audit-3p-2026-09-25.md): every finding it
  lists, from the kernel's stack-overflow refusals and the Studio delete guard to
  the four release-tool defects and the new repository gates. The signature
  payload now covers the `RegistrationInfo` that travels with a plugin's bytes,
  which is a public-API change — `PluginManifest::signing_payload` and
  `PluginTrustPolicy::verify_with` take the registration, and
  `PluginSignatureVerifier::verify` receives the canonical payload instead of the
  raw bytes — so it ships with this version rather than as an unreleasable edit.
- The documentation gate now parses Rust fenced in `///` doc comments, not only
  in markdown. Rustdoc skips a block tagged `ignore`, so a macro-usage example
  that can never be a doctest of its crate was shown to every reader and checked
  by nothing; the 24 face-macro examples are now tagged `rust,ignore` and 21 of
  them are parsed by the gate (the other three are shaped by a macro matcher and
  carry an explicit `macro-input` tag, with the reason stated where the gate
  reads it).
- `atomic_write` uses a unique temporary name instead of a fixed
  `<file>.xirang.tmp`, which it used to delete before reuse — a sibling that
  happened to carry that name was destroyed.
- A graft plan with a repeated key is refused, as every other reader in the
  workspace refuses a repeated field; the second value used to win silently.
- The built-in Studio sample records one observed local, so the DATA panel can
  show a value rather than the placeholder it rendered forever.
- The nine manifests carry `keywords` and `categories`, and the two places that
  counted the `publish = false` members say three (the two example hosts and
  `conventions`). `assert_static_registration` documents its panic and its
  relationship to the runtime rule check, and `MirGraph::from_mir_text` documents
  that text which is not MIR yields an empty graph.
- The changelog no longer claims a release that has not happened, and the crate
  documentation matches the code. The `0.1.0` section said all nine crates were
  published while nothing is on crates.io; it now says the first release is not
  cut yet, and the header states the version line once — the first release is
  `0.1.0`, "1.0" is the milestone name, and raising the line means moving the
  fourteen internal requirements with it. The README's Studio keys no longer
  list a comparison page that was removed (and now list `p`), `debug_method` is
  no longer credited with running the MIR subprocess, the install section says
  plainly that only the Git source resolves today, the `cargo run -p
  nichlink-toolchain` commands work again (`default-run`), the MCP README no longer
  promises stderr diagnostics, and `## Boundaries` records the face-layout rule
  and the plugin-trust boundary.
- A verified plugin signature is reachable, so the official channel can admit
  anything at all. `PluginArtifact::verify_artifact` always recorded the digest
  assurance while the official channel requires the signature assurance, so the
  lane failed closed and the control the threat model names never ran. The new
  `PluginArtifact::verify_signed` runs the whole policy chain — digest,
  revocation, official key, signature — and records the stronger assurance.
  Revocation is therefore consulted before a signature is accepted.
- `nichlink-dev` reports a Studio that exits non-zero, instead of calling a crash
  a successful session.
- A Wasm plugin can no longer buy host memory through a table, and its artifact is
  bounded before compilation. The declared memory ceiling does not bound a table —
  a table is a separate, eagerly instantiated array of function references, so
  `(table 100000000 funcref)` cost hundreds of megabytes outside `memory_bytes`.
  `WasmLimits` now carries `table_elements` and `max_module_bytes`, and modules
  compile under wasmi's own strict engine limits instead of unlimited defaults.
- A process plugin that keeps writing after its answer still delivers that
  answer. The host read one frame and stopped, so a child writing past the pipe
  buffer blocked, the host waited for an exit that could not come, killed it at
  the deadline and reported `Timeout` while discarding the response it already
  held. The answer is sent on first and the rest of stdout is drained to EOF —
  the rule stderr already followed.
- The MCP bridge cannot read outside the source root it was pointed at, and it no
  longer exits zero when its transport fails. A directory symlink under the root
  was followed because the walk asked `is_dir` and the read only compared path
  prefixes; both now go through the canonical path, and a direct request for an
  escaping path is still refused by name. A read or write failure on the stdio
  transport is returned instead of ending the loop quietly, so a client that
  cannot be answered no longer sees a successful session.
- `xirang grafts` fails when it cannot answer its question instead of answering
  part of it. An unreadable source tree, an unreadable host entry and a plans
  directory that exists but cannot be read now make the command exit non-zero
  after writing whatever was readable, because the declaration column would
  otherwise be a guess. A plan whose text is simply broken is still a successful
  report of a broken plan.
- A generated tree that cannot be written is reported rather than panicking out
  of a structured caller. A build script still stops with the reason — it has
  nowhere to render a diagnostic, because the generated tree is what would carry
  it — while `check --json` reports it as an `out-dir` diagnostic.
- A runtime check whose numeric bounds the runtime cannot state exactly is
  refused instead of answered with a guess. The comparison rounded both bounds
  through `f64`, so `number_in_range(9007199254740993, 9007199254740993)`
  accepted `9007199254740992.0`, a number below its own minimum.
- The source walk stops at a depth bound instead of overflowing the stack. It is
  the workspace's only recursive traversal, and the kernel takes its filesystem
  facts from the caller, so it cannot canonicalize a path to tell a link that
  points at an ancestor from a directory that is simply deep; 128 directories is
  far past any real layout, and a tree that reaches it is reported.
- `function_symbols` counts lines in one forward pass. It re-counted from byte
  zero for every function it found, which made the pass quadratic in the number
  of functions — the shape MCP's index walks.
- A host crate with nested faces type-checks under the IDE's `rust_analyzer` cfg.
  `rust-analyzer` applies `#[path]` only at the top level, so every nested face is
  loaded a second time as a crate-root shadow; in that shadow `super` is the crate
  root, while the rule a registry-owning face derives names a module beside its
  own file, so `E0433 cannot find registry_rule in super` failed the whole crate
  in the editor's view. The resolver now answers both tools: `rustc` still
  compiles the sibling path verbatim, and the IDE gets the fallback instead. The
  check runs in CI (`IDE mirror (rust_analyzer cfg)`), because no other step
  compiled that cfg.
- Deleting an external graft record takes two presses, and a record whose text
  does not parse can be deleted at all. One `d` used to move the directory
  immediately, and the delete path read the plan first, so a broken record — the
  one a reader most needs to remove — could never be removed. The first `d` now
  arms and names the record, any other key clears the arm, and the second `d`
  moves it. Selection writes (the plugin form) work the same way, and a failure
  on the second of its two files puts the first one back.
- A new project resolves a relative directory against the project that is open,
  not against the directory Studio was started from, and a scaffold that fails
  part-way leaves nothing behind. A pre-existing directory is not deleted: the
  error says a partial project was left in it instead.
- Studio refuses to start with no project to open, instead of showing an empty
  tree and exiting zero. The old resolution fell back to the directory this crate
  was compiled in — the checkout, or the installed crate's sources — so a launch
  from anywhere else looked healthy while the next authoring command wrote a new
  face into NichLink's own tree. The rule now takes the session's selection, then
  a path argument, then `XIRANG_PACKAGE_ROOT`, then the working directory when
  it holds a `Cargo.toml`, and refuses every candidate that names something
  unusable by name. `nichlink-toolchain` also accepts `[PROJECT]` and `--help`, and a
  failed launch prints one line and exits non-zero before the terminal is taken
  over.
- Rewriting a face keeps its previous text where the reader can find it. The
  editor rebuilds a file from the fields it models, so anything hand-added is not
  in the result; the write was atomic, but the loss was silent and permanent. The
  previous text now lands under the same `.xirang/trash/` the delete path
  already uses, and the message names the backup path. `delete_module` moved to
  its own module in the process, which paid for the addition: the operations page
  is still inside the size ratchet.
- A misconfigured entry or scope is a diagnostic instead of a panic.
  `XIRANG_ENTRY` naming no file, a malformed `application!` declaration, an
  `application!` entry that is not `crate::…` or resolves to nothing, a
  `XIRANG_SCOPE` value with the wrong schema or an identity no node owns, and a
  malformed graft declaration in the entry all used to panic. They failed the
  build, but took the build script down with them and left `check --json`
  printing nothing at all. Each now reports a diagnostic (`entry`, `scope`,
  `graft-entry`) and falls back conservatively — Cargo's convention, the full
  tree, an empty cut table — so one run reports every problem it can find, and no
  refused configuration can silently prune faces away. The scope value is parsed
  by a pure `SourceScope::from_raw` so the refusals are pinned without mutating
  the process environment.
- An ordinary module beside the registration faces no longer fails the build. Any
  flat `.rs` under a host's `src/` — `src/helpers.rs`, say — used to panic with a
  layout message, from the host's own `cargo build` as well as from every
  `xirang` command that reads the tree, because discovery treated such a file
  as a mislaid registration face. Discovery now asks the question the rest of the
  build asks (`parse_face` returning `Ok(None)` means "no face here"), skips
  ordinary modules in silence, and reports a file that really is a face as a
  `face-layout` diagnostic naming the path and the layout it belongs in — so
  `check --json` produces its document instead of an empty stdout. A face that
  does not parse is reported the same way (`face-syntax`), before the stages that
  decode fields, which now skip it instead of aborting the process.
- A deeply nested source no longer aborts the process. `syn` is recursive descent
  with no depth guard, and `proc-macro2` protects only its lexer, so
  `parse_faces` on tens of thousands of nested delimiters (or a
  `Vec<Vec<…>>` chain) ended in `fatal runtime error: stack overflow` — a
  SIGABRT that no `Result` can report and no `catch_unwind` can catch. The three
  `syn::parse_file` entry points now run a nesting scan first, measured on
  `proc-macro2`'s own token stream so brackets inside string literals and
  comments cannot be mistaken for nesting, with the limit at 128 — `rustc`'s own
  default `recursion_limit`. Four shapes are pinned by
  `core/src/registry_core/syntax/deep_input_tests.rs`.
- The process backend no longer has unbounded blocks. Three were measured and
  removed: a `try_wait` poll that never drained stdout, so a healthy child
  writing more than the pipe buffer was killed and reported as `Timeout` (the
  measured boundary was a 65 536-byte frame succeeding and 65 537 bytes timing
  out); a `stdin` write that happened before the deadline was armed, so a child
  that never reads stdin pinned the caller past it (1 MiB blocked for the child's
  whole lifetime and then reported a broken pipe); and a frame read that sat
  outside the deadline. Five regression tests pin these.
- The `compile_fail,E0433` doctest that pins the deleted `edit_module` was
  compiled by no command, because the only `--all-features` job passed
  `--all-targets` and that skips doctests entirely. It is compiled now.
- `MirGraph::to_jsonl` emitted a raw control character for a name that contained
  one, producing an artifact every strict JSON reader rejects while this crate's
  own lenient parser still accepted it. Reproduced before the fix (a tab in a
  function name made both emitted lines invalid) and verified after it against an
  independent strict parser. The scaffold snippet writer had the same weak escape
  for its VS Code JSON; both now use the kernel encoder.
- `tools/nichlink-package-audit` read a crate's dependency list with a loop that
  split on whitespace, so `nichlink-toolchain` and `nichlink-toolchain` were evaluated
  with only their first dependency and the summary listed phantom crates.
  Dependencies are comma separated and parsed as such, and the summary names each
  crate exactly once.
- The process adapter no longer reports a spurious failure when several plugin
  calls run at once. `exec` can transiently reject a freshly staged executable
  with `ETXTBSY` ("Text file busy"), which surfaced as `ExecutableFileBusy` under
  a parallel test run; spawning is now retried a bounded number of times and
  anything else is still reported unchanged.
- Dead code that `#[allow(dead_code)]` and `#[allow(unused_imports)]` were
  holding down is gone: a write-only `RegistryHeader::name` and its unused
  `admission()` accessor (with the module-level allows that hid them),
  `EntryPages::iter()`, `FaceManifest::get()`/`fields()`, `SearchRow::source_index`
  (written four times, read never), a `state::FACE_FORM_ORDER` re-export nothing
  referenced, a stale `#[allow(dead_code)]` on a function that does have a
  caller, and about forty unused imports. Removing the allows rather than
  keeping them is what found the last group: rustc names the real set.
- Every published crate now carries `#![warn(missing_docs)]` and documents its
  whole public surface (about 700 items across the nine crates), so a release
  cannot add an undocumented public item. Two downstream warnings found while
  enabling it are fixed with it: the `NODE_ID`/`REGISTRATION` constants emitted
  by `__registration_face!` and the generated `builtin_static_plan()` /
  `registrations()` / container modules now carry docs, so a host that turns the
  lint on no longer sees warnings in generated code it cannot edit.
- `nichlink-dev` can no longer be installed broken: it sits behind the
  non-default `dev-supervisor` feature, resolves the Studio binary from a
  sibling, `PATH`, or the workspace `target/debug` in that order, and reports the
  missing checkout by name when it was built from one that is gone.
- `apply_recorded_grafts` refuses, instead of skipping, three artifacts with no
  legitimate reading: a plan that does not parse (every unreadable plan is named
  at once), a record whose directory disagrees with its plan's `graft`
  (`RecordReport::SelectorDirectoryMismatch` is gone), and a record whose
  identity and path name different faces.

- The repository gates no longer accept what they were built to refuse: kernel purity
  sees `std::io`/`std::thread`/`std::os` and brace imports (`use std::{env, fs}`), has a
  walk floor, and the mounting gate finds `include!` whatever delimiter it uses (on the
  kernel's masked text, so a fixture string is not a mount); the document gate reports
  an unterminated fence and covers every markdown file under `docs/` at any depth and
  every root-level one; the lint gate reads a rustfmt-folded `#[allow(…)]` and derives
  the roots that must carry `#![warn(missing_docs)]` from the tree instead of a list a
  new crate can miss; `tools/nichlink-package-audit` derives the shipped crate set and
  their internal dependencies from the manifests; `tools/nichlink-publish --check-table`
  reads internal requirements spelled `x.workspace = true` and checks the root's
  `[workspace.dependencies]` versions; and the nesting guard names the limit that
  applies to the shape it refused (1024 for a linear run, 128 for a delimiter group)
  after the generic-argument counter, which could never fire, was removed.
- `application!(entry = …)` resolves every path segment against the package tree,
  so the canonical `<dir>/<dir>.rs` layout works (`entry = crate::control` for
  `src/control/control.rs`) and a segment that resolves to nothing in the middle is
  refused instead of passing as a function name. The resolver lives in
  `toolchain/build_method/src/entry_paths.rs`, split out with the filesystem walk moved to
  `discovery.rs` to keep `entry.rs` inside the size ratchet.
- `WasmLimits::max_element_bytes` bounds what a module can make wasmi materialize
  at instantiation: a passive element segment never grows a table, so
  `table_elements` never saw it, and the measured cost was about 32 bytes per entry
  against about one byte of compact encoding — a 2 000 103-byte module with two
  million entries cost 64 070 402 bytes of host memory. The payload is measured from
  the section headers before compilation, like the artifact ceiling.
- `PluginArtifact::verify_signed` refuses a source no verifier is consulted for
  with the new `PluginTrustError::SignatureLaneRequired`, instead of recording
  `Signature` assurance that nothing checked; that artifact's digest path still
  works.
- A graft selector that starts with `.` is refused by the rule the writer, the
  parser and Studio already share: `..` used to be accepted, wrote the plan to
  `<pkg>/.xirang/graft.plan`, and was then never listed — a record the runtime
  never applied while the author saw a created plan.
- Build output is trusted only while it still describes the sources:
  `xirang_toolchain::build_method::build_output_is_current` compares the published
  `discovery.fingerprint` against a freshly computed one, the pipeline writes that
  fingerprint only on a clean run, and `explain` (and `--overlay`) reports
  `known: false` with the existing "run `xirang check`" note when it is missing or
  stale. A failing check no longer leaves output that `explain` presents as the
  current scope.
- A malformed or unevaluable-gated `static_graft_plan!` is a `phase=graft-entry`
  diagnostic instead of a panic: `check --json` used to exit 101 with an empty
  stdout, discarding the diagnostic `scope` had already produced. The reader now
  takes the build's diagnostics collection, all three refusal kinds report through
  it, and the parse message is byte-identical to `scope`'s so the two collapse into
  one line.
- The documentation gate handed fenced Rust straight to `syn`, so a fence nesting
  tens of thousands of delimiters aborted the gate process instead of failing it —
  the same defect the kernel's parse entries had, in the one place that still had
  it. It now asks the kernel's `guard_nesting`, which is the workspace's single
  nesting measurement and is public for exactly this reason, and
  `a_pathologically_nested_fence_is_reported_not_fatal` pins that a refusal is a
  message: with the guard removed, that test aborts.
- `tools/nichlink-publish` read its own dependency table word-wise, so a crate
  with more than one internal dependency reported only the first — `nichlink-toolchain`
  was checked against `nichlink-toolchain` alone — and the guard that refuses
  to publish a crate before its dependencies are on the index was effectively off.
  The same traversal also handed back an edge line's dependencies as crates of
  their own, so nine table lines produced fourteen nodes. Both tables are now read
  as whole lines, and `--check-table` compares them against the manifests: the
  first run found a real drift, `nichlink-toolchain` depending on `nichlink-core`
  directly while the table listed only the other three. The same check reads
  workspace members with `awk` rather than a `sed` address range, because a sed
  range does not test its end address on the start line: the one-line `members`
  array made the range run on to `[workspace.package]`, whose name then entered
  the member list — invisible only because no directory of that name exists.
## [0.1.0] — 2026-09-25

The first release. All nine crates went out together in dependency order on
2026-09-25, and the workflow's last step resolved all nine from crates.io in a
throwaway consumer outside the checkout. The chain was:
`nichlink-core` → `nichlink-macro` / `nichlink-toolchain` / `nichlink-toolchain`
→ `nichlink-toolchain` → `nichlink-toolchain` / `nichlink-toolchain`
→ `nichlink-toolchain` → `nichlink-toolchain`. Dependency requirements are written as
caret `0.1.0`, so a patch release does not force dependents to republish.

- **`nichlink-core`** (library `xirang`): the pure kernel — registration
  vocabulary, the `Registry` tree with atomic page-copy transactions, admission
  and flow contracts, graft declaration/application records, plugin policy,
  MIR/source evidence models, and the `syntax` registration-face parser.
- **`nichlink-macro`**: the compile-time face-field front end (tolerant
  separators and field order, spanned diagnostics, editor mirror).
- **`nichlink-toolchain`**: build-time source discovery, identity cache,
  scope pruning, and `StaticPlan` generation.
- **`nichlink-toolchain`**: runtime trace state, the `host!`/`trace_call!`
  macros, and the `authoring` executor.
- **`nichlink-toolchain`**: MIR, `CallTrace`, data-flow, and graph evidence.
- **`nichlink-toolchain`**: verified Wasm (`wasm`, default) and process
  (`process-tools`) adapters with atomic hot deployment.
- **`nichlink-toolchain`**: the Ratatui authoring and inspection surface.
- **`nichlink-toolchain`**: the read-only MCP stdio bridge for five source queries.
- **`nichlink-toolchain`**: the unified `xirang` / `cargo-xirang` binaries.

Known limits for this line are in the root `README.md`'s `## Boundaries`
section and [`docs/threat-model.md`](docs/threat-model.md). Version-dependent behaviour
such as the kind-only `registry_name` derivation is recorded in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md).

---

## 简体中文

NichLink 工作区的所有变更都记录在这一份文件里。九个 crate 按同一条版本线发布，
因此一个变更只在这里描述一次，而不是九次：**有意不按 crate 分九份维护**。发布顺序与
理由见 [`docs/roadmap-1.0.md`](docs/roadmap-1.0.md)。

格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)；版本号遵循
[语义化版本](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

### [Unreleased] 未发布

### [0.2.0] —— 2026-09-29 首发、2026-10-07 重发

变更：

- **发布面合并的批次 1：内核是 `kernel/`，它的库是 `xirang_kernel`。** 目录 `core/` 改为
  `kernel/`，包名 `nichlink-core` 改为 `nichlink-kernel`，库名 `xirang` 改为
  `xirang_kernel`，于是那个一身三职的历史名字（`xirang` 既是项目、又曾是一个二进制、还是内核
  库名）只剩两职。代码引用、清单依赖、门禁落点、发布工具表、CI 行与活文档锚点都在同一次提交里
  一起跟上；七个执行面仍是各自独立的 crate（那是批次 2），已发布的 0.1.x 名字保持冻结。
  新旧对照与判别性谓词见 `docs/audit-2026-09-28/audit-merge-batch1-spec.md`。

本工作区不按 crate 分维护 changelog，因此九条 crate 的同一处改动在这里只描述一次。

### [0.1.6] 未发布

新增：

- **`xirang.grafts`：外部 graft 计划，以及宿主入口是否保住它们。**
  `.xirang/external-grafts/<selector>/graft.plan` 下的计划是构建从不打开的创作记录；宿主入口的
  `static_graft_plan!` 没有点名计划所针对的槽位时，发布态会剪掉那个槽位，这条记录便永远无法生效。
  构建对此**只警告**，而冗长的 `cargo` 日志正是警告湮没的地方——CLI 从 `xirang grafts` 起就在回答
  它，现在桥也回答，而且用的是同一条规则（本发布把 `xirang_toolchain::build_method::graft_plan_rows` 从 CLI
  提出来，因此两个执行面不可能给出不同答案）。每条记录点名 selector、计划针对的逻辑路径与替换件、
  是否覆盖整棵子树，以及保住它的那条声明（`cut \`…\` graft \`…\`` 与入口行号）或者
  `NOT declared by the host entry`——并以 `unkept plans N` 收尾，因为那才是读取方要据以行动的东西。
  声明针对的是计划所瞄准的**槽位**而不是计划选择的实现（那是构建自己的问题），因此计划与声明各自的
  graft 都会打印，差异是看得见的，而不是被默默接受。读不了的计划带上原因而不是被跳过；入口读不了时
  状态是 `unknown`，而不是把一个缺失的答案变成错误的 `not declared`。脚手架宿主上端到端实测：有声明
  时回复点名入口第 6 行；再加一条针对 `root/ghost` 的计划后报 `unkept plans 1`——而 `cargo build`
  随后拒绝的正是同一件事（`external graft plan \`ghost_fast\` targets \`root/ghost\`, which no
  declaration in the host entry names`），在任何人等到发布才发现之前。五条测试，其中三条实测为红
  （把每条计划都算作已声明、跳过读不了的计划、把 `unknown` 变成 `not declared`）。

- **`xirang.diff` 现在也能把外部 graft 记录与源码对照（`records: true`）。** 记录里存着它写下时
  针对的身份，因此槽位没动而面换了身份会**悄悄**弄坏它：路径还在、记录仍能解析，而文本 diff 与构建
  日志都不会说这件事。这次比较沿用树 diff 的词汇，而每条记录都落在五个桶之一：`ok`；`undeclared`
  （身份在树里，但没有任何 `static_graft_plan!` 切口点名它的槽位——发布态会剪掉那个槽位，这正是
  `xirang.grafts` 报成 `NOT declared by the host entry` 并计入 `unkept plans` 的那种）；`stale`
  （树里没有那个身份、也没有那条路径）；`re-identified`（路径在、身份换了，报成 `old -> now`），
  而这正是符号图对一条 graft 说不出的事；以及 `unreadable`（读不了的记录文件），它被计数而不是
  丢掉。类型化切口没有单独的桶：计划的路径永远是逻辑路径（每个写入方都用 `registry.path_for`），而
  身份缺席时没有任何东西能解析类型化声明的模块，因此这类记录就是 `stale`——回复同时打印出它查找的
  那个路径，而不是猜。脚手架宿主上三条记录实测：
  `ok 1  stale 1  re-identified 1`，最后一条解析出
  `00000000000000000000000000000001 -> 43c1869f312b81a54005c13a1af5b8ae (root/slider)`。两条测试，
  其中两条承载主张的实测为红（把旧身份当作"现在"；让类型化切口落进 `stale`）。

- **`xirang.explain` 渲染覆盖投影（`overlay: true`），而它背后的遍历搬进了
  `xirang_toolchain::build_method::overlay_projection`。** `explain` 一次回答一个面，或投影构建划定作用域的
  那棵树；两者都不说哪个槽位被已声明 graft 切口替换——而 CLI 的 `explain --overlay` 从存在起就在回答
  它。桥现在用同一次遍历回答，因此两个执行面不可能就"哪个槽位被替换"产生分歧——这就是本发布里
  `graft_plan_rows` 那次搬家的下一项；`DeclaredGraft::form` 随之搬走，所以切口的拼法也只有一条规则，
  而 CLI 的 JSON 文档逐字段不变。每行点名逻辑路径、kind 与源码，以及替换件
  （`graft … full … form … (entry line N)`）或什么都没有；作用域不保住的槽位列在 pruned 里。作用域
  未知时 `selected`/`kept` 保持未知，而不是把一个仍然存活的面说成被剪掉；回复携带那条说明，写明这是
  静态投影——活的生效树是宿主导航里同时链接两棵注册树后的 `Registry::dump_effective`。`node` 与
  `overlay` 互斥。经桥在一个已 check 的宿主上实测（一份已声明计划）：`slots 1 (replaced 1)`，附
  `<- graft=button_fast full=false form=string (entry line 2)`；`pruned 1`（自动作用域只选中了那条
  已声明槽位）；以及 `plan records 1 [kept by cut `root/button` … at entry line 2]`。桥六条测试、
  共享遍历三条测试；两次变异在两个执行面上实测为红（让被选中子级的父面落进 `pruned`；把缺失的
  作用域读成 `pruned`）。

- **MIR artifact 现在是一份快照，而 `xirang.mir` 能对两份作差（`against: "<path>"`）。** MIR
  转储是**某棵**树的快照，而 rustc 的文本格式说不出是哪一棵，因此本工具写出的 JSONL 携带一个表头，
  点名它来源的身份命名空间与注册树根——也就是 trace artifact 已经在用的那种约定。同一棵树的两份快照
  作差得到调用图差异：`caller -> callee` 关系的新增与消失（按对匹配并去重，因此 MIR 行号移动不算差异）
  以及函数符号，计数从基线指向后一份。属于**另一棵**树的快照会被按名拒绝；说不出自己那棵树的 artifact
  （普通的 `-Zunpretty=mir` 转储）仍能作差，而回复会说明"跨两棵树的比较无法排除"。给另一棵树的 artifact
  重新盖戳会被拒绝而不是换标签，`xirang.unified` 也拒绝把外来快照与本包的 trace 合并。经桥实测：
  对转储执行 `jsonl: true` 会把
  `{"kind":"snapshot","namespace":"e2e-016","root":"7da1…"}` 写成第一行，而差异报出
  `relations added 1 gone 1  functions added 1 gone 1`，新增 `crate::outer -> crate::new`、消失
  `crate::outer -> crate::other`；手写的 `somewhere-else` 快照回以
  `REFUSED: the two artifacts describe different trees`；两份文本转储则带着"未标识"的限定作答。
  桥六条测试、内核一条快照测试；三次变异实测为红（交换差异方向；跳过外来快照检查；不再盖戳表头）。

- **`xirang.search` 现在会搜树，而每个面命中都会说出构建对它的看法。** 本工具过去只回答"哪个文件或
  函数叫这个名字"、对注册树一无所知；现在它优先匹配逻辑路径、`kind`、模块与 `registry_name`——也就是其它工具
  使用的那些拼法——并给每个命中标注 `ok`、`added since build`、`re-identified`（带上新旧两个身份）或
  `build unknown`，随后才是与此前相同的文件与函数命中。结论来自 `crate::tree_delta`，也就是
  `xirang.diff` 说出的那一条规则（本轮提取出来），因此一个面不可能在一个工具里是 `ok`、在另一个里是
  `added`。身份命名空间无从得知的根仍然回答源码那一半，并说明树那一半不可用。经桥在同一个宿主上实测：
  `face  root/slider … [ok]`；新增一个面之后是
  `face  root/gauge … [added since build]`（其上还有构建过期那一行）；而在文件没动的情况下改了 `kind`
  之后是 `[re-identified (daca0f7b… -> bc2df33a…)]`。五条测试；那条共享规则做了变异实测（把每个已发布
  的面都读成 `ok`），diff 与 search 合计四条测试同时变红。

- **两个内核入口取代了执行面过去只能自己保留的紧凑渲染器：`xirang::authoring::parse::compact_admission`
  与 `xirang::authoring::parse::compact_registration_rule`。** Studio 的 `admission_text` 与
  `registration_rule_text` 在 `toolchain/studio/src/studio/app/source_index.rs` 里自己装配紧凑子句，因为当时没有
  任何公开入口能从 owned 值渲染它——这正是 `FIXR-01` 记录的第二份实现一族，其中 admission 那份副本还
  放宽了内核刚修好的 deny 门禁。现在两个入口都是公开 API、Studio 的调用点变成纯委派，而内核自己的历史
  分支也经它们渲染，因此消费方已在读的字节没有移动：单列表 `allow_paths(`/`deny_paths(` 分支与注册规则的
  源码文本分支（`rule_syntax_from_text`）都走那唯一一个渲染器。两个入口分别由 crate 之外的
  `core/tests/compact_admission_entry.rs` 与 `core/tests/registration_rule_entry.rs` 钉住，Studio 的
  `registration_rule_text_tests`（`the_call_site_renders_what_the_kernel_renders`、
  `the_call_site_carries_no_second_rule_renderer`）钉住它的调用点不再自带渲染器。

- **`xirang::authoring::parse::try_render_requirements`：渲染不了就**拒绝**的入口。**
  `render_requirements` 用解析出的条目重写作者的列表，因此一条畸形条目会从结果里消失——把它写回
  文件的调用方就删掉了作者写下的一个字段。严格兄弟返回 `Result<String, FaceParseError>` 并点名它
  拒绝的那条（`capability=>provider`），manifest 渲染器改调它。`render_requirements` 仍是 `-> String`
  有损，因为那就是已发布的 `0.1.x` 签名；它的文档现在写明这一点并指向严格入口（审计 `LGC-LG-28`）。
  由 `a_malformed_requires_entry_refuses_the_rewrite` 走真实 `render_source` 钉住，变异（调用方改回
  有损入口）实测变红。

- **`xirang_toolchain::build_method::face_views_and_unreadable`：读不出来的面要**被点名**，而不是被丢掉。**
  过去 `face_views` 只回答"安放得了"的面，于是安放不了的注册面文件、语法失败的节点文件从回答里消失、
  无处可查。新入口返回 `(Vec<FaceView>, Vec<String>)`——面加上读不出来的路径（复用
  `validation::face_syntax_errors`）——`face_views` 委托它，两者因此不可能不一致；文档写明
  `parent_resolved == false` 那一半。由
  `a_registration_file_that_does_not_parse_is_named_not_dropped` 钉住，修前实测红（不可读集合为空，
  审计 `LGC-LG-11`）。

- **`xirang::source::item_symbols` 与 `SourceItem`：声明词表由内核持有。**
  Studio 的搜索视图用自己的 13 条 `strip_prefix` 词表拼符号行，于是内核每长出一个关键字就多一处
  会忘记的地方，而 trait 里无函数体的 `fn` 反而被报成缺失。`item_symbols` 按源码顺序回答，只用一张
  `INTRODUCERS` 表（`name`/`signature`/`line`/`is_function`；`let` 刻意不在表内，因为绑定没有调用图
  能到达的名字），Studio 现在只渲染这份列表——它自己的文件里不再有任何词表（`strip_prefix("` 命中 0）。
  由 crate 之外的 `core/tests/b4_item_symbols.rs`（5 条）与 Studio 的 `source_rows_tests`（4 条）钉住，
  两者对着旧词表都实测变红（审计 `STU-S-06`）。

- **`xirang::authoring::parse::try_parse_requirements_owned`：让"已发布的签名"继续是已发布的签名。**
  `parse_requirements_owned(&str) -> Vec<OwnedRequirementSpec>` 是 `0.1.x` 线上宿主调用的入口，它
  有意是有损的：解不出来的就丢。收紧它会破坏一个不可变的版本，因此"拒绝"的判断放在新入口里，已发布
  的那个保持逐字签名与行为；两者走同一条 `requirement_item` 规则，于是已发布入口、严格入口与校验器由
  一份实现回答（审计 `LGC-LG-28`）。由 `the_strict_requires_entry_refuses_a_malformed_entry` 与
  `a_malformed_requires_entry_is_refused_not_dropped` 钉住，把严格入口改成委派有损入口的变异实测变红。

四个入口都是**未发布 `0.1.6` 线上**的加法式新增：不抬版本，`tools/nichlink-publish --check-table`
也不受影响（它只按清单比较）。它们真正碰到的是 `tools/nichlink-package-audit` 的后半段——隔离打包
要等一个 crate 的带版本号 `nichlink-*` 依赖进入 index，因此 `0.1.6` 未发布期间那些 crate 会被跳过
（是跳过，不是失败）；本批把这段等待**加深**了一层，因为 Studio 现在调用尚未发布的内核入口。内容半边
已经包含这些新模块，发布顺序仍是 core 在前。
- **`xirang::authoring::parse::try_rule_syntax_from_text`：规则文本的**严格**读法。**
  `rule_syntax_from_text` 保持宽容、签名仍是已发布的 `-> String`，因为 `0.1.x` 线上的宿主在调它：
  写在**注释里**的 `.require_exports(…)` 会被当成真规则（真正声明的那条就此消失），而由常量点名的清单
  ——`&EXPORTS`，文本扫描看不到它的字面量——会静默降级成 `ANY`，那正是"没有结构要求"，于是作者声明的
  结构要求无声无息地不见。严格兄弟返回 `Result<String, FaceParseError>`，并同时点名面文件与它无法照办的
  规则文件；唯一生产调用点 `rule_syntax_for_source` 自本版起经它读取，而规则文件**缺失**仍是 `Ok("ANY")`
  ——缺席不是畸形规则（审计 `K-10`）。由生产入口与 crate 之外的钉子钉住，两条形状都实测先红，并由独立复核
  （`docs/audit-2026-09-28/audit-verify-k10-family.md`）在三组变异下重测。加法式新增，随未发布 `0.1.6`
  走：已发布的宽容入口与依赖表都不动。
变更：

- CLI 的 `grafts` 现在渲染由 `xirang_toolchain::build_method::graft_plan_rows` 计算出的记录；它的 JSON 与文本
  输出逐字节相同（由它自己的测试钉住），而"这条计划的槽位是否被声明"这条规则从此只有一个家。
- `explain --overlay` 的投影现在来自 `xirang_toolchain::build_method::overlay_projection`，因此 CLI 与桥不可能
  就"哪个槽位被切口替换"产生分歧；它的 JSON 与文本输出不变。
- **命名批次（B7+B8）：模块按它意味的东西命名，公开名一个不动。** 原先按所在目录取名的文件
  改成按它的主语取名：`run_method` 的 trace 快照目录、`authoring/context` 与
  `authoring/face_file`；`mcp` 的 `build_evidence`、`resolve` 与 `source_index`；
  `build_method` 的 `discovery_cache`、`source_walk`、`registration_phase`、
  `face_syntax_check`、`discovery_node`、`node_identity` 与 `scope_faces`；内核的单文件模块
  `json`、`release` 与 `requirements`（多余的 `<dir>/<dir>.rs` 嵌套被收平）；`studio` 的
  `hot_zones` 与 `editor_launch`。公开**符号**面只增不减：九个内核名拿到了真身名，同时把
  历史名保留为一行转发器——因此原本编译得过的东西不会编译不过，也没有一条 shim 重导出需要删除。
  新旧对照、四族挂载名与保留名裁定记在 `docs/audit-2026-09-28/audit-naming-batch-plan.md`
  （规则与保留表在 `AGENTS.md`）。

### [0.1.5] 2026-09-27

修复：

- **生成的清单现在要求"生成它的那个发布版本"，而不是永远要求 `0.1.0`。** 每个脚手架出来的宿主在
  两条 NichLink 依赖上都写着 `version = "0.1.0"`，两种依赖来源都是。caret 语义把它藏了四个发布
  （`^0.1.0` 接受 `0.2.0` 以下的一切），因此这个字面量不只是陈旧——它**错得会咬人**：版本线一到
  `0.2.0`，`xirang new` 生成的宿主就无法解析，而失败看起来会像 Cargo 的问题而不是模板陈旧。
  现在 `dependency_specs` 写 `env!("CARGO_PKG_VERSION")`——生成工具自己的发布版本，它不会陈旧——
  而脚手架钉子改成对着这个值断言、不再对着字面量断言，因此下一次发布自动带上它。实测：已发布的
  `0.1.4` CLI 生成 `version = "0.1.0"`；`0.1.5` 的生成 `version = "0.1.5"`。把旧字面量放回去即实测为红。

### [0.1.4] 2026-09-27

新增：

- **`xirang.impact`：改动一个面能走多远。** `xirang.usages` 给直接邻域，`xirang.converge` 给
  一个面的约束；两者都不说这次改动还碰了什么。这个工具遍历这棵树真正**声明**的三种依赖：一个面的后代
  （它的注册机拥有它们）、`requires` 点名了该面所提供能力的面，以及把该面交出去的已声明 graft 切口。
  每个到达的节点都带上最短跳数、**所有**到达它的理由（既是子面又是消费者的节点必须把两者都说出来）与
  跳的链条，因为"它为什么在我的爆炸半径里"正是修复时要问的问题。遍历受 `depth` 与已访问集合限制，
  因此能力环被计数——`cycles back to this face N`——而不是被反复走；没走到的面会被如实报成在深度内
  未到达，而这并不等于独立。经桥在真实宿主上实测：一个两个面的包回答
  `hop 1 root/slider/gauge`，附 `because: child of this face's registry (kind Gauge)` 与
  `because: consumer: requires `cap.paint=>Slider``；未知节点按名拒绝。五条测试，其中三条实测为红
  （去掉多余理由、去掉环计数、忽略深度上限）。点名同一身份的 graft **记录**与已记录 trace 不在遍历
  范围内，回复里写明了这一点。
- **`xirang.trace` 现在也回答一次运行**看见**了什么，而不只是跑了什么。** 调用报告说哪些函数处于
  活动状态；`values: true` 读同一份经身份核验的 artifact，报告记录下的局部值——按捕获它们的帧分组，
  带上名字、渲染后的类型与值、角色（`input`/`let`/`return`/`consumer`）、`observed`/`unobserved` 与
  调用点——外加在任何被追踪调用之前捕获的那些，以及被观察到的数据边与每条边承载的变换。这正是符号图
  结构上不可能有的那一半，也通常是跨文件缺陷真正关于的那一半。两条诚实规则内建其中：点名的东西不在
  artifact 里的局部值或边会被**计数**而不是丢掉，因此不完整的 artifact 不会读起来像完整的；`query`
  缩小这份报告的方式与缩小调用报告完全一致，并在没有命中时说出来。真实宿主端到端实测：三个捕获值分组
  在 `frame 2 Slider::render` 之下、调用点落在面自己的文件里，数据边为
  `count -> shown (transform)` 与 `shown -> value (used by paint::value)`。两条测试钉住，均实测为红
  （一条把所有局部值折出它们的帧，一条关掉 query 过滤）。
- **`xirang.converge` 现在也能从一次已记录的运行出发，而不只是从一个面出发。** 缺陷报告有两种
  形状：代理知道自己在看哪个面，或者它手上只有一次行为不对的运行、而"那次运行碰了什么"只有 trace
  说得出来。给出 `trace: true` 时，工具读取本包的 trace artifact（走 `xirang.trace` 那同一套身份
  核验，缺失与拒绝的答案也一并继承），把整棵树收敛到"既声明了面、又真的跑了"的那些文件，点名落在
  各自的帧，统计落在任何声明面之外的帧，最后给出读计划，并指向
  `xirang.converge node=<路径>` 去看某一个面的约束。帧是**按源文件**匹配到面的，回复里也这么写，
  因为面是声明、帧是正在活动的函数——这是这一步诚实的边界，而它仍然是要紧的那次收敛：350 个文件的树
  变成跑过的那几个文件。真实宿主实测：一次 3 帧的运行、一个面的包，回答
  `faces that ran (1 of 1 declared)` + `frames in a face 1 / outside any declared face 2` + 一份
  单文件读计划。匹配会把编译器的路径重写到它最后一个完整的 `src/` 段之后，而这一点后来被证明是必需的：
  同一次运行把 `main` 记成 `src/main.rs`，却把面模块里的帧记成**绝对路径**
  `/home/…/src/slider/slider.rs`，因为生成的库是按绝对路径挂载那个模块的。三条测试钉住它，其中正向
  那条把匹配退化成精确字符串比较即实测为红。
- **MIR 通道现在有读取方、写入方与合并。** `xirang.mir` 读 `rustc -Zunpretty=mir`
  文本转储或紧凑 JSONL artifact，按扩展名选择，并保留两种格式的不对称而不是抹平它：JSONL 严格解析，
  一行畸形就整体失败；文本转储从不失败，因为不是调用的行就只是不是调用。`jsonl: true` 让这个工具成为
  工作区里从来不存在的那个写入方——Studio 能渲染、也能解析这种 artifact，而从没有任何东西产出过一份。
  `xirang.unified` 随后把该图与本包已记录的 trace 经
  `xirang_toolchain::call_evidence::UnifiedCallGraph` 合并——那是两份证据唯一的汇合处——因此被 trace 确认的调用
  带 `evidence=Live` 并**取代**它的编译器候选、而不是与它并列，其余保持 `evidence=Mir`。缺失 trace
  是更弱的答案、不是坏掉的答案：合并仍然作答，并把每条关系标为编译器候选。`mcp` 为此新增
  `nichlink-toolchain` 依赖；版本线移到 `0.1.4` 是另一条原因（一个被 `run_method` 消费的 core
  符号），因此这一条是随车走，而不是它本身要求移动。*文本*的生产者仍在桥之外，工具
  把这一点说出来而不是藏起来：那是 nightly 工具链上的 `cargo rustc -Zunpretty=mir`。
- `xirang.verify` 关上了别的工具打开的那个环：它对一个包重跑内核的注册校验，并报告那次运行刚刚发布
  的树差异，因此一次编辑是被**确认过**的，而不只是被写下。它驱动 CLI 的 `check` 所驱动的同一个入口，
  所以它的判断不可能与 `xirang check` 漂移；它还顺带刷新构建证据。判断失败是答案而不是工具故障：
  诊断点名阶段、节点、源码行、字段与期望的提供者，而回复的 `isError` 仍为 false。写它时还钉住了一个
  一直躲过审阅的 API 不对称：`check_for` 收包**目录**，而 `package_name` 收它的清单**文件**——把文件
  传给前者，管线就会去找 `<Cargo.toml>/src` 并报 "is not a source directory"。CLI 的局部变量名叫
  `manifest` 却装着目录，混淆就是这样藏起来的。
- 脚手架出来的宿主现在演示整条运行期证据链，而这正是本工作区里别的任何东西都不做的那一半。
  `xirang new` 写出的 `src/main.rs` 会按环境要求的模式记录一个帧，并在设置了 `XIRANG_TRACE`
  （模式）或 `XIRANG_TRACE_FILE`（路径）时，把 artifact 写到所有读取方都看的地方——
  `.xirang/traces/xirang.trace`。没有这两个变量之一时它什么都不记录、也不写文件，因此发布路径
  仍然什么都不收集；意义在于那条链可见且可跑。端到端实测：脚手架 → 构建 → 运行（无文件）→
  `XIRANG_TRACE=full` 运行（141 字节 artifact、`mode=full`、一个 `main` 帧）→ `xirang.trace`
  渲染出调用树。库宿主拿到的是指引而不是演示，因为它没有可在结尾写入的 `main`。写这一条时还抓到自己
  一处陈旧的说法：桥那句"本工作区还没有宿主记录"现在是假的。

- **构建的证据、树的差量与运行期的 trace——三个不靠源码文本作答的工具。** `xirang.explain`
  读构建发布在 `target/xirang/out` 下的文件，逐个面回答真正会发布什么——作用域是否选中它、发布
  剪枝是否剥掉它的符号——外加身份、路径、kind、源码、模块、父级与槽位；也可以给出整棵被划定作用域的
  树（受 `limit` 限制）。`xirang.diff` 说出源码现在与构建清单之间的面级差异：新增、消失，以及文件
  没动而身份变了（`kind` 变化就是身份变化，文本 diff 看不见）。`xirang.trace` 读取已记录的 trace
  artifact，在核验它的身份（命名空间、注册机根、每个帧的节点）之后渲染无终端调用报告——真正跑了
  什么；异树 artifact 被按名拒绝而不是画出来，缺失时说出产出它的办法，报告过长会截断并给出总行数。
  声明的 graft 状态与 contract/admission 字段仍不在内：前者归 `xirang grafts`，后者需要一个已加载
  的注册机与一次构建。
- 修掉 NichUI 测试项目实测到的两个读路径缺陷。扫描不再索引 `.xirang/`，因此被删掉的东西不再从
  自己的可恢复备份里回答 `status` 与 `search`。`xirang.callgraph` 也有了上限——定义数由 `limit`
  限制、调用者由上限截断——多定义命中会说明歧义并提示传 `path`，调用者清单也标明是按名字匹配的。
  修复前的实测：在 350 文件的语料上，一条 `{"function":"new"}` 回复是 4.5 MB，因为 151 个定义各自
  列出了该名字的每一个调用点。
- `xirang.usages` 补上了写入路径的读回缺口：代理能经 `xirang.apply` **设置**的那些字段
  （preset、parts、本地化名称、exports、`requires`、`provides`、handle 与 part 的 traits/contracts、
  registration rule、admission、flow、runtime checks）现在都能被报告，外加一个面在树里的父级与子面，
  以及别的面双向提到的能力记号。能力匹配发生在声明的记号上而不是一棵已解析的图，回复里写明了这一点；
  手写模块没有生成的字段清单，因此那些面被计为不可读，而不是显示成空的。写这些钉子时作者契约教了两次：
  父级必须先拥有注册机内核才准入子面，而需求要写成 `capability=>ProviderKind`——裸能力名会被内核按名
  拒绝。

- `xirang.converge` 把那些答案拼成代理在动一个面之前真正需要的那一个：构建的作用域与剪枝判断、
  树的边、声明的字段、每条 `capability=>ProviderKind` 需求是否有答案（有就点名是谁）、该读哪些文件，
  以及细节在哪个工具里。有意思的是被拒绝的那一半，而写它的钉子发现了它：加载本包自己的面会**校验**它们，
  因此一棵需求没有提供者的树会让所有依赖注册机的工具拒绝加载——而那恰恰是代理最想要一个判断的时刻。
  现在 `converge` 把那次拒绝当作头条发现报出来（内核已经点名了出问题的节点与源码位置），并且仍然打印
  读计划，因为那只需要源码推导出的树。另一半在写入路径上：会让后代的需求失去答案的编辑会被按名拒绝，
  诊断里带着那个后代与它的 `file:line`。

修复：

- **`XIRANG_TRACE` 现在是 lexicon 常量，因此改它的名字不可能悄悄弄坏宿主——而这也是本次发布存在
  的原因。** 收集模式那个变量过去是记录器里的裸字面量，而它的同类 `XIRANG_TRACE_FILE` 是公开的
  `lexicon::TRACE_FILE_ENV`，因此共享契约测试只守住了其中一个名字；而宿主是**被告知去设置这些变量**
  的，脚手架模板还会把其中一个**写进生成的代码**。现在由 `lexicon::TRACE_MODE_ENV` 承载它，
  `trace_mode_from_env` 读该常量，生成的 `src/main.rs` 也像它早已读取 `TRACE_FILE_ENV` 那样读取
  `lexicon::TRACE_MODE_ENV`。它搭不上 `0.1.3`：该符号在 `core`、被 `run_method` 使用，而包审计用
  **已发布的**依赖构建每个 tarball，于是对着 `nichlink-core-0.1.3` 报
  `error[E0425]: cannot find value TRACE_MODE_ENV in module crate::registry_core::lexicon`。
  那是版本规则在生效、不是缺陷，而本次发布正是它要求的移动。实测：刚脚手架出来的二进制在
  `XIRANG_TRACE=full` 下仍能记录并写出 artifact；把常量改名会让 lexicon 钉子变红，把模板改回
  字面量会让脚手架钉子变红。
- **在一个进程里构建或校验第二个包时，它不再被盖上第一个包的命名空间。** `build_method` 的身份
  命名空间过去是"先到先得"的进程固定值，而 `check_for` 用它收到的包去设置它——于是在长生命周期进程
  （MCP 桥、Studio 会话）里，**第二个**包发布的证据带着第一个包的命名空间，`xirang.diff` 于是把
  每个面都报成身份变了。CI 在 `verify` 自己的测试里抓到它：`reidentified 1`，运行侧
  `ba9a8808…`、源码侧 `77fc3680…`，而这两个 id 解出来分别是
  `namespace=mcp-verify-broken path=label/label.rs name=Label` 与 `namespace=mcp-verify-healthy …`
  ——构建侧被**先跑的那个测试**盖了章。现在命名空间被限定在这一次运行、且落在运行线程上
  （`run_as_package`），因此一次运行的身份属于正在运行的那个包，而其它线程继续读到进程固定的值；
  进程级固定值仍服务于构建脚本，`freeze_test_namespace` 也照常工作。钉住它的有
  `a_run_namespace_wins_over_the_pin_and_puts_it_back`，以及通过代理真正调用的工具实现的
  `a_second_package_in_one_process_keeps_its_own_namespace`。前者把作用域去掉即实测为红；后者分别
  去掉作用域、去掉接线各测一次——单元钉子钉的是机制，桥的钉子钉的是"机制确实被用上"。
- 预览不再把操作报告成已完成。`xirang.apply` 在一份一次性副本上运行真实操作，而执行器用过去时
  描述它做了什么，因此删除预览会先打印 `would move …`，又在同一份回复里打印
  ``moved `button` to …``——这句话对副本成立、对项目不成立。现在回复给这句话限定作用域
  （`preview effect: …`）而不是丢掉它，因为它带着 diff 没有的父级身份与改名前后的模块名；落盘
  时仍然原样打印。这是拿桥对着脚手架出来的宿主玩的时候抓到的：真实项目里没有任何回收条目，而
  回复说它搬走了一个。

### [0.1.3] 2026-09-26

新增：

- MCP 桥的写入路径 `xirang.apply`：代理可以经**与 Studio 相同的 authoring 执行器**创建或重写
  注册面，因此内核的准入、父规则与拓扑校验会作用在改动上，而不是在桥里重新实现一遍。为此 `mcp`
  新增 `nichlink-toolchain` 依赖（`authoring` 特性）；版本线仍是尚未发布的 `0.1.3`，因此没有
  新符号需要靠新版本与已发布版本区分。`action` 为 `add` 或 `edit`，`parent` 接受逻辑路径
  （`xirang.registry` 报告的那条）或身份，而**除非 `apply: true`，请求只做预览**：预览在一份
  一次性的包副本上运行真实操作——用复制而不是"先写再回滚"，正是它不会留下改了一半的树的原因——
  并返回文件 diff 与将得到的注册树；`apply: true` 才写入项目，给出它写下的文件，并把产生的声明锚成
  `<path>:<line>`（与拒绝时 `file:line:column` 同一种形状）。预览的两端都是
  同一次 `run` 调用，这正是预览不可能与落盘漂移的原因。`rename` 改 `fields.module`、`delete` 把
  模块移入 NichLink 的可恢复回收目录，两者复用同一套预览。而 `edit` 是**补丁**：先读回该面
  （`authored_face`，新增公开 API），覆盖请求的字段，再把完整的一组交给执行器——执行器自己的契约是
  "用你拿到的取值重建"，这对展示所有字段的编辑器正确，而对只要求改一处的代理是错的，因为那会抹掉
  另外二十二个字段。graft 写入、插件、项目脚手架、树 diff 与一致性分析仍待做
  （`docs/roadmap-1.0.md` 第 7 条）。执行器自己的边界被原样继承并已记录在文档里：它重写的是
  **NichLink 生成**的面，对手写的面以 `this module was not generated by NichLink` 拒绝，因为
  重写一个并非它创作的文件会丢掉它并未建模的内容。
- `xirang_toolchain::build_method::source_layout` 与 `SourceLayout` 成为公开 API：写入路径需要与构建相同
  的答案——包的注册面住在哪里——而在那里猜 `src/` 会创作出构建永远不读的注册面。
- `nichlink-toolchain` 的 `xirang.registry` 工具：本包声明的注册面，每个面一行——逻辑路径、kind、
  源码，以及宿主编译出的 `NodeId`。这些行由 `xirang_toolchain::build_method::face_views` 推导——也就是
  CLI 的 `explain` 所用的同一份推导——因此 `mcp` 新增了 `nichlink-toolchain` 依赖，版本线随之
  移到 `0.1.3`。这修掉了桥里一处长期存在的不实：它只注册了五个源码文本工具，文档却宣称具备注册树
  查询，于是代理靠 grep 宏名重建那棵树。命名空间是这里藏着的真正设计问题：包名**就是** `NodeId`
  命名空间，因此工具先解 `XIRANG_NAMESPACE`，再解 Cargo 报告的包名，否则**拒绝作答**——绝不
  回落到 `xirang.default`，因为那种答案之下的每个 id 都指的是宿主从未编译过的节点。contract、
  admission 与 registration rule 数据仍需已构建的面快照，桥里仍然没有。
- `src/` 之外的库目标就地读取。清单声明 `[lib] path = "host/lib.rs"` 的包，其注册面现在从
  `host/` 读入，而每个面的身份路径是 `host/…`——相对清单的路径，因为声明宏只去掉一个前导
  `src/`，别的都不去。过去被写死为一个常量 `<root>/src` 的两个决定，现在是只解析一次的两个基准
  （`toolchain/build_method/src/source_layout.rs`）：遍历读取的树，以及身份路径所相对的基准；
  `relative_display` 与 `src.join(relative)` 那些反向拼接照常工作，而入口解析——唯一同时需要
  两者的地方——接收布局。`[lib] path` 指不到文件时是一条点名它的 `face-layout` 诊断，而不是去
  遍历别的树；这次清单读取是有意窄的（一张表里的一个键、引号、注释、点式 `lib.path = "…"` 写法），
  并且绝不 spawn `cargo`，因为管线是在构建脚本里运行的。`[[bin]]` 目标不移动源码根：一个包可能有
  多个二进制目标，没有任何东西能在它们之间做选择。顺带把 `toolchain/build_method/src/entry.rs` 带回 450 行
  **上限之内**（它的约定入口选择移到 `entry_default.rs`），因此它的尺寸棘轮项是被删除而不是被放大。
- `xirang_toolchain::build_method::package_name`：Cargo 权威的包名读取从 `nichlink-toolchain` 移出，因此命令行
  与 MCP 桥问的是同一个权威，而不是各带一份副本。CLI 各命令行为完全不变，钉住那份私有副本的两条
  测试随它一同移动。构建管线不调用它：构建脚本从 Cargo 拿到 `CARGO_PKG_NAME`，也就是同一个值。
- Studio 的 trace **加载方**，即上面那份 artifact 的消费一半：`TraceStatus { Absent, Loaded,
  Mismatch { reason } }` 与启动时运行一次的 `App::install_trace`。发现路径来自
  `trace_artifact_path`，四条身份检查决定这份 artifact 是否属于本会话——解析器先拒绝不支持的
  版式版本，然后是命名空间、注册树根、以及每个已不再解析的已记录节点。匹配的 artifact 把记录的
  数值放进 DATA 面板；被拒绝的不装入任何东西、保持注册表可见，并把原因写到事件行。
- `nichlink-toolchain` 的进程适配器可以收窄子进程：`ProcessLimits::inherit_env`（默认
  `true`）清空环境，`ProcessProgram::environment` 指明子进程随后能看到哪些变量，
  `ProcessProgram::current_dir` 决定它在哪里运行。
- `nichlink-toolchain` 的 `PluginAdmission`：宿主侧从插件锁到可加载工件的那条路。它读
  `<package_root>/.xirang/plugins/{official,user}.lock`、按这份目录筛选 manifest、校验它
  （官方来源走 `verify_signed`，用户来源走 `verify_artifact`），并给出换来的保证等级所对应的
  通道；`install`（Wasm）与 `load_process`（进程）一步完成准入与加载。在此之前**树内 `core`
  之外没有任何代码调用过 `verify_signed`**，Official 通道因此不可达。
- `nichlink-toolchain` 里 trace artifact 的第一片：`TraceArtifact`（`from_trace`、`render`、
  `parse`、`into_trace`）、`write_trace_artifact`、`read_trace_artifact` 与
  `trace_artifact_path`，以及内核词典里的 `TRACE_DIR`/`TRACE_FILE`/`TRACE_FILE_ENV` 契约。
  宿主现在能把已记录的 `CallTrace` 写成带版本的文档，由独立读取方重建；Studio 的加载方是下一片
  （`docs/design-trace-ingest.md` §3.5）。该文档是逐行文本：取值转义 `\\`、`\t`、`\n`、`\r`，
  文件里不出现任何裸控制字符，畸形或含未知键的文档按行拒绝而不是猜测。

变更：

- `nichlink-toolchain` 为 `package_name` 依赖 `serde_json`。这是本 crate 读取的唯一一份 JSON，
  而 `syn` 本来就在，因此宿主的构建脚本多付的是一片叶子，而不是这条规则的第二份实现。
- 重新实测并更正了工具集与 trace 加载方的文档主张：根 README 与两份讨论引言仍在说 Studio 渲染内置
  样例 trace、尚无 ingest 路径，以及桥没有注册树查询。MCP 工具清单现在包含 `xirang.registry`，
  而 trace 那句改成：Studio 读入宿主写出的 artifact，没有 artifact 时报告 `TRACE: none`。
- Studio 的内置 trace 示例已删除。有了加载方之后它只会误导，因此图例在什么都没装入时读
  `TRACE: none`、在 artifact 通过身份检查时读 `LIVE`、在被拒绝时读 `TRACE mismatch`；DATA
  面板不再有 `· built-in sample ·`，而是显示 `no trace attached` 或拒绝原因。会话再也不能为
  自己并不拥有的证据贴标签。
- Studio 读出宿主 crate 自己的包名，并在它之下创作，不再默认 `xirang.default`。两端必须一致
  ——宿主的构建脚本把 `env!("CARGO_PKG_NAME")` 盖成身份命名空间——而一个在别的名字下重建注册树的
  会话会让每个已记录的 `NodeId`（trace、graft 记录）都指不到东西。包名现在来自 Cargo
  （`xirang_toolchain::build_method::package_name`），也就是 CLI 与 MCP 桥所用的同一个权威，而不再来自对
  `[package] name` 的逐行字面扫描：TOML 的点式写法 `package.name = "x"` 是同一张表却没有
  `[package]` 表头，扫描什么也找不到，于是 Studio 把这样的项目创作在错误的身份域里。
  `XIRANG_NAMESPACE` 仍原样覆盖一切，而 Cargo 说不出包名的清单（虚拟工作区根）仍回落到文档化
  的默认值——创作是在创建一棵树，默认值在那里是有意义的。
- Studio 的手绘调用树画布已删除：`rataflow`（`node-graph` 特性，默认开启）成为唯一的调用树
  绘制者，1756 行画布代码与它们的几何测试随之消失，`g`（切换绘制者）与 `v`（切换排布）两个按键
  不再存在。未链接 `node-graph` 的构建保留面板，并说明这棵树需要哪个特性，而不是什么都不画。
- 向 `rataflow` 控件转发鼠标**抬起**事件：它在抬起时发出 `NodeClicked`，而 Studio 此前只转发
  按下，因此控件的点击选中从来不会触发。现在会。

修复：

- Studio 对以点式 `package.name = "x"` 命名包的 TOML 清单推导出**错误的**身份命名空间：那是与
  `[package]` 相同的表，逐行扫描看不到表头，于是会话回落到 `xirang.default`，而宿主在真实包名
  之下编译——同一批面因此存在于两个身份域里，一侧写下的 trace 或 graft 记录在另一侧解析不了。
  字面读取方已删除，Studio 改为取 Cargo 的答案，每次采纳项目一次子进程（已采纳的命名空间缓存在
  项目上下文里，因此没有写入路径为它付代价）。
- trace artifact 的写入方创建自己要写入的目录。文档化的宿主用法把 `write_trace_artifact` 与
  `trace_artifact_path` 配成一对，而在一个全新的项目里那条路径的 `.xirang/traces/` 还不存在：
  写入以 "No such file or directory" 失败，而且消息点名的是写入方自己的临时文件，而不是缺失的
  目录。创建它正是 authoring 执行器的写入方对 `.xirang/external-grafts/` 已经在做的事。
  `run_method` 的两份 README 现在也记下了这一对写入方/读取方与 `XIRANG_TRACE_FILE` 覆盖
  ——这份公开 API 此前没有任何面向宿主的页面，因为设计文档并不在包里。
- Studio 的写入方要求"读者打开的那个项目"。`package_root()` 仍会回落——先会话、再环境、最后
  工作目录——而这个猜测对读取是合理的，因为读错树看得见。写入承担不起：一次删除曾这样解析根，
  把模块从环境变量指到的那个项目里搬走。现在每个写入方都走 `with_selected_project`（或
  `selected_package_root`），没有选中项目时以具名错误拒绝，因此将来的调用点不会因为忘记建立
  上下文而重演。读取保留回落；已启动的会话仍在接管终端之前采纳项目，因此用户可见行为不变。
- `nichlink-toolchain` 在源树缺失时不再中止。`[lib] path` 指向 `src/` 之外的包——一种合法的
  Cargo 布局——过去会走到 discovery 里的 `expect("src directory must exist")`：构建脚本以退出
  101 死掉，而 `check --json` 什么都不打印，而那正是那条命令要守住的契约。现在是一条
  `face-layout` 诊断，点名它找的是哪棵树：结构化调用方拿回它（`check --json` 写出文档），构建
  脚本带着渲染后的文本响亮失败。遍历中途消失的目录也以同样方式报告，而不是 panic。
  `check_for` 的文档也补上了"第一个参数是包**根目录**"——每个调用点本来就是这么传的。
- MCP 桥的请求分帧有了上限：超过 `MAX_REQUEST_BYTES`（1 MiB）的行以 `-32600` 被拒并排空其余
  部分，敌对或有 bug 的客户端再也不能让这个长期存活的 stdio 服务无限分配。排空停在换行处——
  直接读进暂存缓冲会把下一帧一起带走，因为切片或管道会给到能装下的全部内容。请求还必须携带
  `"jsonrpc":"2.0"`，而不是 JSON 对象的成员以 `-32600` 作答而不是被静默丢弃；通知即使信封不对
  也保持沉默，这是规范要求的。
- `PluginCatalog::contains_manifest` 用相等比较三个可选来源字段——签名、密钥指纹、撤销列表
  快照——于是宿主自己的插件界面写下的七字段官方记录永远匹配不上已签名的官方 manifest，官方插件
  无法经宿主自己写下的锁准入。现在它们是**期望**：记录携带某个值就把它钉住，省略它就交给签名校验。
- 重新实测并更正了若干文档主张：设计文档说 Studio 示例不记录 locals（它记录一个，由该文件自己的
  测试钉住）；post-fix 审核的第 21/22/26/27/28 条与它的 `build.rs` 作用域发现都已修完，并就地
  标注；路线图不再把首次发布称作 1.0 唯一剩余项。
- 写入路径落地之后，桥自己的描述仍称自己是"只读"：`mcp` 的 crate 文档、四处模块头、清单
  `description`、CLI 用法文本里的 `xirang mcp`，以及两份 README 里各四处。现在每一处
  都写出桥真正做的事——源码与注册树查询，加上先预览后落盘的创作写入——因此 `--help` 的一行与
  docs.rs 的一页不再和 `xirang.apply` 自相矛盾。

### [0.1.1] 2026-09-26

新增：

- `deny.toml` 与 `cargo-deny` CI 任务，覆盖 advisories、licenses 与 bans；
- 编译非默认特性集与无默认特性表面的 CI 任务：
  `cargo test --workspace --all-features --all-targets` 与
  `cargo check --workspace --no-default-features --all-targets`。`studio` 的
  `prototype-fixtures` 测试首次因此被编译，`plugin-host` 的 `process-tools` 也经
  `--all-features` 再检查一遍；
- 每个已发布的 crate 目录内各放一份 `LICENSE`，使每个发布的包自带许可证文本（Cargo 只会自动
  包含 crate 目录下的 `LICENSE*`）；`conventions/` 没有也不需要：它是 `publish = false`。
- `macro/README.md` 与 `nichlink-macro` 的 `readme` 字段；
- 九个 manifest 补上 `repository` 与 `homepage`，缺失处补 `documentation`
  （`nichlink-macro`、`nichlink-toolchain`）；非默认特性属于公开 API 的 crate 补
  `[package.metadata.docs.rs] all-features = true`；
- `tools/nichlink-package-audit` 现在会警告并跳过版本依赖尚未上 crates.io 的包，并以 0
  退出；CI 的包审计步骤不再用 `continue-on-error` 掩盖失败；
- `studio`：新增仅限工作区的 `dev-supervisor` 特性以门控 `nichlink-dev`，并签入
  `prototype-fixtures` 测试当初针对的、仅含源码的夹具宿主包
  `toolchain/studio/tests/fixtures/node-editor/`；
- `tools/nichlink-publish`：按依赖顺序**分层**发布九个 crate，每层发完等它出现在 index 里
  再进下一层。除非给出 `--publish --yes`，否则只做 dry-run。
- 新增 `conventions` crate（`publish = false`），其测试是五条此前只存在于散文中的规则的可执行
  形式：内核纯净性、模块挂载、450 行棘轮、`missing_docs` 属性与 `#[allow(missing_docs)]` 禁令、
  以及 README 与活文档里 Rust 围栏的可解析性。每道门禁在保留之前都实测过"制造违规即失败"。
- CI 新增 `cargo test --workspace --all-features --doc` 步骤。
- 新增 `tools/nichlink-visual`：视觉检查脚本，在固定尺寸的 tmux 面板里运行真实 Studio 二进制、
  用按键驱动它，并保存终端实际显示的内容——纯文本，以及加 `-e` 时带 SGR 转义的版本。场景覆盖首页、
  搜索浮层、新增表单、调用图、聚焦的调用树、在树上用方向键走动的过程，以及数据流面板；每个场景先等
  程序起来、再等它自己那一屏出现，然后才抓屏，因为在两者之间抓屏有时会存下上一屏。输出写到
  `target/visual`，因此一次运行不会往检出里写东西。
- 新增"渲染出的调用树"测试，断言的是**图**而不是模型：它带图浮层渲染 Studio，按调用树面板自己
  画出的四角把该面板从页面里裁出，然后检查窗口内每个节点都在屏幕上有标签、同一列的两个节点不重叠、
  层号更大的画在更小的右边、每一跳一列的边都在被调用者那一行有箭头、表头数出的数量与模型一致。
  它会把检查过的面板打印出来，因此这个形状既可断言也可阅读。
- 内核新增唯一的 JSON 字符串编码器 `xirang::json`，由构建诊断文档、MIR JSONL 工件与
  生成的编辑器片段共用。它只转义 RFC 8259 要求的那一份；其测试断言"不存在原样控制字符"
  而不是做往返，因为本工作区自己的解析器宽松到会接受非法输出。
- `lexicon` 现在拥有 `PACKAGE_ROOT_ENV`、`NAMESPACE_ENV` 与 `DEFAULT_NAMESPACE`，以及纯的
  `resolve_package_root` / `resolve_namespace` 规则，因此环境变量名与解析顺序只有一处定义，
  而不是每个执行面一份。
- `tools/nichlink-package-audit` 现在检查九个 crate 的包**内容**：每个 `src/**/*.rs` 模块与
  清单声明的 README 都必须在包里。`cargo package --list` 不需要 registry，因此这一半在首次
  发布之前就生效，而不是等它。

- `tools/nichlink-publish --verify-consumers`：发布演练中只有已发布的 crate 才答得出来的那一半。
  它在本检出之外建一个一次性 crate，按版本 `cargo add` 九个 crate（因此解析发生在 index 上而不是
  本地路径上），再一起 `cargo check`。index 上还没有的版本会如实报成"尚未发布"并给出发布后该跑的
  命令，而不是报成脚本失败。
- `tools/nichlink-publish --check-table`：把工具里的依赖表与层表同决定真实发布顺序的清单对比——
  清单对依赖表、依赖表对层顺序、以及"没有 crate 排在自己的依赖之前"。它不需要网络，因此
  `features` CI 任务每次 push 都会跑，发布工作流也会跑。
- `.github/workflows/release.yml`：由 `v*` tag 触发。tag 与 workspace 版本不一致时在上传前停下，
  然后跑全特性门禁、`--check-table`、包审计与产物审计，最后才按依赖顺序发布，并验证本检出之外的
  消费者能解析这次发布。`workflow_dispatch` 只跑全部检查而不发布——首次发布因此可以在 tag 存在
  之前先演练一遍。
- 嵌套守卫现在量到第三种形状，它是第二轮才被找到的：**线性 token 串**——被折叠成一个嵌套表达式
  或类型、却没有定界符或尖括号可数的 token 串。在这之前仍有七种形状会打死进程：
  `& & & …`、`&mut …`、`&'a …`、`* * * …`、`- - - …`、`! ! ! …`、`|| || …` 以及 `1 + 1 + …`；
  最后一条正是"这件事关乎**树**而不只是解析"的原因：二元链是用循环解析的，但它建出的左嵌套
  `ExprBinary` 的 `Drop` 每个运算符递归一层，于是进程在离开一次**已经成功**的解析时死掉。串以
  `,`、`;` 与花括号组为分隔（括号与方括号不分隔，因此 `x.f().f()…` 会被数到），上限 1024 个
  token——比"在 256 KiB 栈上观察到能活下来的最小串"还低十六倍——并由新增的
  `core/tests/nesting_budget.rs` 把工作区里 318 个 Rust 文件全部喂给带守卫的入口，证明这个上限
  不会拒绝本仓库出厂的任何文件。`deep_input_tests.rs` 为每种形状留了一条用例。
- 新增 `examples/control-button/tests/static_plan_allocations.rs`：用计数式全局分配器实测发布读
  路径的堆分配，而不是从代码形状推断。读内置计划完全不分配（`builtin_static_plan()`、两个切片、
  `find` 命中与落空、`children_of`、以及遍历全部面合计 0 次分配、0 字节）；`overlay_static`
  并不免费（它会克隆树并建立已访问切口集合），但不构造 `GraftPlan`，且比同样切口的动态写法便宜
  （一个切口 60 次对 77 次，两个切口 107 次对 138 次）。该文件恰好只放一条测试，因为计数器是进程
  全局的，而 libtest 会把测试放在不同线程上。
- 新增 `tools/nichlink-external-rehearsal`，让"外部路径演练"可复跑：它把两个示例宿主复制到
  检出之外、把它们的路径依赖指向这里、让它们脱离工作区，并在自己的 `target/` 里从零构建与测试。
  示例宿主的 27 条测试在那里全部通过，构建出的 CLI 也能检查那个外部项目（`check: ok`）。它抓的
  是"只在树内成立的宿主布局假设"，CI 在一个矩阵单元上跑它。
- 新增 `toolchain/plugin_host/tests/wasm_table_cost.rs`：一张 Wasm 表在宿主一侧到底花多少，用计数式
  全局分配器实测而不是估计。一个函数引用是 8 字节，因此 `table_elements` 的默认上限 4096 是
  32 KiB，而 `a_huge_table_is_refused` 钉住的一亿条目表本来会是 762 MiB；限制器在峰值 5 225
  字节之后就拒绝了那个模块，因为它在表存在之前就拒绝。同一次核对还确定了是哪条限制在起作用：
  wasmi 的 `EnforcedLimits::strict()` 限制的是一个模块可以声明多少张表与多少个元素段，而不是
  单张表能长到多大，因此 `WasmLimits::table_elements` 是表大小的唯一约束。
- 新增两条运行观察 `toolchain/studio/src/studio/app/tests/evidence.rs`，把关于 `CallEvidence::Live` 的
  论证换成实测：装上出厂的演示追踪后，真实工程里没有任何一条边被判为 `Live`——而且测试断言的
  是"追踪确实装上了、且它的节点不属于已加载工程的任何节点"，因此它不会因为"根本没有追踪"而
  通过。换成针对已加载注册面的追踪后，同一份枚举恰好报出那条追踪记录过的边。所以 `Live`
  不是死代码；它对真实工程不可达的唯一原因是 Studio 目前不载入真实追踪。
- `toolchain/run_method/examples/scale_audit.rs` 现在带预算：注册 40 µs/节点、索引 20 µs/节点，约为实测值
  的八倍，可用 `XIRANG_SCALE_REGISTER_US` / `XIRANG_SCALE_INDEX_US` 覆盖，因此数量级回归会
  让运行失败，而不是只打印一个更大的数字。
- `features` CI 任务新增三步，补上没有其他任务覆盖的缺口：`--all-targets
  --all-features` 的 clippy、tmux 视觉检查（`tools/nichlink-visual home graph tree-demo`，
  每份 capture 必须非空），以及 `tools/nichlink-publish` 的 dry-run。
- 新增 `docs/performance-baseline.md`：10k/100k 节点下注册与索引的耗时、release 产物按字节与
  定义符号计的规模，以及重新测量两者的命令。

变更：

- 调用图页现在只有调用树与"树游标所在函数运行时的取值"两块面板，别无他物：曾经与树并排的
  "调用者/被调用者"关系栏已删除（树把那些画成了边），左右对比页也随之删除，连同它的 `Ctrl-W`
  绑定、第二个搜索框、它的 `SearchState` 字段以及 `4` 页面键。`[` 与 `]` 现在移动树在页面上的
  份额，分隔线拖动使用同一范围。
- 边从调用者的**底边**离开、进入被调用者的**顶边**，这正是让这张图成为一棵树的东西：侧边端口
  把每一跳都变成了横着挪一步。只要树里有东西调用焦点，焦点就画在图的中间；只有确实没有调用者时
  才在顶部——那正是内核的树实际说的话。
- 调用图页的字母是命令、文本不是：以前在这个页面上输入字母会编辑查询，而这里 `m`、`v`、`g`、`e`
  都是命令，且兜底的输入分支排在 `e` 之前，于是那个键永远触发不了（rustc 看不见这种冲突，因为它
  遮蔽的那条 arm 带着守卫）。一条从已删除的对比面板遗留下来的 `graph_focus == 2` arm 也一并清除。
  查询在列表页编辑，`/` 回到那里，页脚把两页的按键都列出来。
- 夹具的调用图里现在有一棵树了——`paint_node_editor` 分叉到 `layout_panels`、
  `clamp_canvas_width` 与 `preview_canvas_width_traced`，而两个测量函数又在钳制处汇合——因此视觉
  检查有分叉、有深度、也有扇入可看；`tools/nichlink-visual tree-demo` 画的正是它。
- 控件现在被告知内核知道的事情，并且保留自己的视口。节点文本带上预算漏掉的裁剪计数
  （`←n`/`→n`）、游标的 `▶` 与焦点的 `◆`——是**标记**而不只是颜色，因为引出这一轮的反馈正是
  "游标移动在没有颜色的终端里看不见"——而每条边带着自己的证据种类作为标签。控件现在缓存在 App
  上：焦点变化时重建（并重新铺满），游标移动时重盖标记而视口随行，其余情况下原样复用，因此拖动
  不会被打断。落在它那棵树上的鼠标事件交给它，这正是本面板获得滚轮缩放、拖拽平移与点击选中的
  方式；它的节点不可拖动，因为这棵树是模型的视图，节点不能偏离它所代表的层与车道。
- 盒子拿到自己标签所需的宽度：车道取其中最宽盒子的宽度，带取其中最高盒子的高度，每个盒子在该格里
  居中。固定间距扛不住名字——一个 27 字符的符号会画到邻列身上，这正是让树读不懂的拥挤——控件绘制方
  的排版现在有一条回归测试，任何无视内容的间距都会让它失败（已用固定列宽验证过：长盒子确实重叠）。
- 四个方向键跟随**画出来的**网格，而不是模型自己的轴，因此它们在两种绘制方里都保持字面含义：调用沿
  屏幕向下时 `↓` 走向被调用者、`↑` 回到调用者，沿屏幕横向时这两个角色交给 `→` 与 `←`；沿层的移动走
  的是确实存在的那条边（并优先留在原车道），横向则取同一条带内最近的车道。该方向没有节点时状态行会
  说出来，而不是挪到别处去。
- 调用树默认由 `rataflow` 节点编辑器控件绘制，手绘画布只差一个按键（`g`）且仍然编入：`node-graph`
  现在是默认特性，因此 crate 出厂即是控件的样子，而画布仍保留着画内核自身事实（证据种类、裁剪计数、
  游标与焦点的区分）以及塞得进七格宽列的那一份。
- 调用树也可以**自上而下**画，并且窄面板会自动改用这种画法：横向画布每一跳要占一整列，而 80 列
  的面板只给树 19 格，于是两个盒子各缩到七格、名字只剩尾部——而同一棵树向下排只需为边花掉一列边距，
  然后让每个盒子都拿到面板整宽，一跳一条带，调用者在上、被调用者在下。当面板窄到横向画布无法再给
  两列各一个可读盒子时自动切换，`v` 在"跟随面板 / 自上而下 / 从左到右"之间循环，标题写出当前模式。
- 调用树改为画**节点盒**，不再是账本。每个函数是一个带边框的单元格，内容按**整名**换行——
  账本把长名字裁到 15 格且只留尾部，因此 `preview_canvas_width` 读作 `…w_canvas_width`——游标的
  盒子用更重的边框、焦点的盒子是绿色，裁剪计数是盒子上的徽标，边在调用者边框上留下端口、在空隙里
  转折、以 `▶` 收尾并在旁边带证据标记。列宽取该列最宽的名字，而面板小一格时列是被**收窄**而不是
  被丢弃，因此正在阅读的函数的调用者仍留在屏幕上。底边图例列出各标记，点击盒子即选中该节点。
  打包会先给邻列留出位置再让游标列取宽，在"这样能换来第二列"时把空隙从五格收到三格，并在名字放不进
  两行时改为保留尾部加 `…`，而不是折成五行窄条——这三条都因为画成一个盒子的树没有边，而窄终端过去
  显示的正是那个。`[` 与 `]` 移动分隔线拖动用的同一个比例，而连窄空隙都放不下的面板会在标题里写
  `[ ] widens`，而不是让读者以为这个函数没有邻居。
- 新建项目与插件表单的行也具名了：`app::state::new_project_field` 与
  `app::state::plugin_field` 每行一个常量，并挨着表单打印的标签，因此调用点上的 `values[0]`
  变成 `values[new_project_field::DIRECTORY]`，渲染器原先各自携带的两张标签表也随之消失；
- 包 README、`docs/discussion-introduction*.md` 与根 README 的工作区结构已更正为当前
  的九 crate 工作区；
- `XIRANG_ENTRY` 现在每次构建只解析一次，同时驱动作用域剪枝与生成的
  `BUILTIN_GRAFT_CUTS` 表；指不到文件的值会让构建失败，而不是让一个读取者回退到 Cargo 的
  `main.rs`；
- 外部 graft 计划的目标槽位没有任何声明命名时，构建现在**失败**（原为
  `cargo:warning`），消息里带着可直接粘贴的 `static_graft_plan!` 子句。本次构建里
  `#[cfg]` 关掉的声明仍然算数，因此合法门控的槽位不会被上报；
- `apply_recorded_grafts` 为每条报告打印一行 `warning:`/`note:`，被跳过的记录因此不可能
  沉默；
- 从靠后的名字写到靠前的名字的兄弟区间会被拒绝，并说明顺序规则（`registry_name` 顺序），
  而不是静默交换。
- `nichlink-toolchain` 的进程后端现在真正执行它声明的限制：请求帧由独立线程写入，stdout 与
  stderr 各由独立线程排空，超时覆盖整个调用，因此 `ProcessLimits::max_output_bytes`（默认
  1 MiB）就是真实上限。此前真实上限是约 64 KiB 的管道缓冲，超过它报的是 `Timeout`。
- `petgraph` 从 0.6 升到 0.8。它只支撑 `CallGraph` 的私有字段，因此不改变任何公开 API。
- `toolchain/studio/src/studio/ui/forms.rs` 改为像它的四个兄弟一样用 `#[path]` 挂载 `face_fields`。这是
  风格差异而不是缺陷：两种写法都能解析，因为经 `#[path]` 载入的父文件按目录解析子模块。
- Studio、authoring 执行器与 MCP 桥改为通过同一条内核规则解析项目，而不是三份副本。行为
  不变；唯一有意的差异写在它所在之处：桥的最后兜底仍是当前目录，因为 stdio 桥是在代理所处理
  的项目里启动的，而编译进去的清单路径属于构建该二进制的那台机器。
- `wasmi` 从 0.42.1 线移到 `1.0.9` 下限（锁文件解析到 1.1.0）。0.x 的 caret 要求永远到不了
  对沙箱真正重要的那些修复——内存读写的整数溢出、内存增长越过系统上限、`rem_s(MAX, -1)`
  在本该返回 0 时 trap、以及 loop 局部变量与 wide-arithmetic 的错误编译。只有一处调用点变化：
  `Linker::instantiate` 加 `PreInstance::start` 变成 `instantiate_and_start`，它运行同一个
  `start` 函数，燃料也仍在其之前设定。燃料仍是 2.0 之前那一类，因此 `fuel_per_call` 的含义
  不变；其文档现在说明单位属于引擎，大版本升级可能重新标定它。
- 五条重复规则各归并为一份实现：清单写入方与 CLI 报告共用的切口端点渲染、CLI 的 JSON 错误
  文档与命令错误文本、Studio 的计划路径（改用运行期加载器的 `graft_record_root`）、PascalCase
  的 kind 推导（改用内核的 `pascal_case`）以及按 `::` 边界匹配符号（改用内核的 `same_symbol`，
  现已公开）。其中两条此前已经分叉。
- **注册面作者面现在只有作者真正要决定的东西那么大。** `handle` 与 `params` 按规则就是
  kind，`registry_name` 就是声明模块路径的末段，因此 `__control_object!` 与
  `__external_object!` 各自只剩一条 arm，同一个类型的三种拼写从声明文件里消失；最小注册面
  是 `control_object! { kind: Button }`。trait 标签改由"编译器检查的契约路径"按**一条规则**
  推导，三个读取方共用它（`FaceSyntax::string_list` 的文件视图、过程宏
  `__face_trait_labels_or!`、写入方 `apply_trait_contract`），因此标签不可能与编译器检查的
  路径不一致。`FACE_FIELD_ORDER` 从 29 个可接受键收到 24 个。
- **Studio 新增/编辑表单是 26 格而不是 30 格**：22 行作者输入，加上收尾的一条只读"机器取值"；
  槽位在 `authoring::face_field` 具名，因此 Studio 的写入路径不再用裸字面量索引数组。有四个
  槽位随它们描述的字段一起消失。凡是与代码相反的帮助文本都已改正：树槽位声称可以覆盖，而
  registry_name 规则不允许；`external source note` 对一个人人参与注册机解析的字段写"仅来源
  元数据"（现改名为 `dependency registry`）；kind/parts 两行标着"推导"却可编辑。
- call report 对同一个事实只说一次：`handle=` 曾与 `kind=` 并排在每个函数上打印，搜索也曾把
  `params` 当作同一字符串的第二个键。按规则两者都是 kind，因此现在只打印 kind、只搜索 kind；
  `RegistrationInfo` 为读取它的宿主保留这两个字段。
- 有类型的嫁接切口现在跨它连接的两个面证明输出类型：构建会在 `BUILTIN_GRAFT_CUTS` 旁发出
  `assert_contract::<{cut}::__Preset, {graft}::__Parts>()`，因此替换面的 parts 若不提供 preset
  要求的 parts，构建就会失败。字符串与选择器切口不发断言，这一点写在填充累加器的地方，而不是
  退回已删除的字符串比较。

移除：

- 作者书写的 `expected_output` / `actual_output` 对。它只在两处与**自己**比较过
  （`release.rs` 与 `build_method::check_output`），从未描述真实类型：button 面写着
  `"ControlFrame"`，而它的 `NoPreset`/`NoParts` 输出是 `()`。它假装陈述的类型事实现在由按面的
  `assert_contract` 与上面的按切口断言证明。字段、两处比较、缓存键、创作侧管线与 Studio 槽位
  一并删除。
- `authoring::FACE_FIELD_NAMES`：一张只有测试读、且与展示标签近乎重复的 30 行标签表。布局的
  名字现在住在 `authoring::face_field`，原来用这张表的测试改为断言更强的性质：槽位稠密，且每个
  都有展示元数据。
- Studio 为同一个事实打印多遍的行：搜索详情面板里的 `values` 运行期快照行与 `declared`
  file:line:function 行（文件已经由 `path` 行给出），以及两个详情面板里重复 `kind` 行的
  `params`/`handle` 行。`params`、`handle` 与 `kind` 是同一个字符串：两个宏都用
  `stringify!($kind)` 展开它们。

修复：

- 记录在 [`docs/audit-3p-2026-09-25.md`](docs/audit-3p-2026-09-25.md) 的三方审查：该文件列出的
  每一条都已修完，从内核的栈溢出拒绝与 Studio 删除守卫，到发布工具的四个缺陷与新增的仓库门禁。
  签名载荷现在覆盖随插件字节同行的 `RegistrationInfo`，这是一次公开 API 改动——
  `PluginManifest::signing_payload` 与 `PluginTrustPolicy::verify_with` 接收注册声明，
  `PluginSignatureVerifier::verify` 收到的是规范化载荷而不是原始字节——因此它随本版本发布，
  而不是作为一条发布不出去的改动留在树上。
- 文档门禁现在也解析 `///` 文档注释里的 Rust 围栏，而不只是 markdown。rustdoc 会跳过标了
  `ignore` 的块，因此一个永远无法成为本 crate doctest 的宏用法示例会给每个读者看到、却没有任何
  程序检查；24 个注册面宏示例现在标为 `rust,ignore`，其中 21 个进入门禁解析（另外 3 个的形状由宏
  匹配器决定，带显式的 `macro-input` 标记，理由写在门禁能读到的地方）。
- `atomic_write` 改用唯一临时名，不再用固定的 `<file>.xirang.tmp`——它过去会在复用前删掉该名字，
  于是一个恰好带着它的同级文件会被销毁。
- 重复键的 graft 计划会被拒绝，正如本工作区其他每个读取者都拒绝重复字段；过去第二个取值静默获胜。
- Studio 的内置样本记录一个被观测到的局部值，因此 DATA 面板能显示数值，而不是它一直渲染的占位。
- 九个清单带上 `keywords` 与 `categories`；两处统计 `publish = false` 成员的地方改为三个（两个示例
  宿主与 `conventions`）。`assert_static_registration` 写明了它的 panic 与它和运行期规则校验的关系，
  `MirGraph::from_mir_text` 写明"不是 MIR 的文本得到空图"。
- CHANGELOG 不再声称一次没有发生的发布，crate 文档也与代码对齐。`0.1.0` 那一节曾说九个 crate
  已发布，而 crates.io 上什么都没有；现在它说首次发布尚未切割，并由头部一句话说清版本线——
  首个发布是 `0.1.0`，"1.0"是里程碑名，抬版本线要连同十四处内部要求一起移动。README 的 Studio
  键位不再列出已删除的对比页（并补上了 `p`），`debug_method` 不再被说成运行 MIR 子进程，安装段
  明说今天只有 Git 源能解析，`cargo run -p nichlink-toolchain` 那两条命令重新可用（`default-run`），
  MCP README 不再承诺 stderr 诊断，`## Boundaries` 记录了注册面布局规则与插件信任边界。
- 已验证的插件签名现在可达，因此官方通道终于能接纳任何东西。`PluginArtifact::verify_artifact`
  永远记录摘要等级，而官方通道要求签名等级，于是那条通道失败关闭、威胁模型点名的控制从未运行。
  新的 `PluginArtifact::verify_signed` 跑完整条策略链——摘要、撤销、官方密钥、签名——并记录更强
  的等级。因此撤销在被接受的签名之前就被检查。
- `nichlink-dev` 现在会报告以非零退出的 Studio，而不是把一次崩溃称作成功会话。
- Wasm 插件再也无法通过表买到宿主内存，且工件在编译之前就有上界。声明的内存上限并不约束表
  ——表是一块独立的、即时实例化的函数引用数组，因此 `(table 100000000 funcref)` 会在
  `memory_bytes` 之外花掉数百兆。`WasmLimits` 现在带 `table_elements` 与 `max_module_bytes`，
  并且模块在 wasmi 自身的严格引擎限制下编译，而不是默认的无限。
- 已经给出答案却继续写入的进程插件仍会交付那个答案。宿主过去读一帧就停下，于是写得超过管道
  缓冲的子进程阻塞、宿主去等一个不可能到来的退出、在超时点杀掉它并报出 `Timeout`，同时丢掉
  它其实已经拿到的响应。现在答案先送出，stdout 的其余部分排空到 EOF——这正是 stderr 早已遵循
  的规则。
- MCP 桥读不到它被指向的源码根之外，且传输失败时不再以 0 退出。根下的目录符号链接过去会被
  跟随，因为遍历问的是 `is_dir` 而读取只比较路径前缀；两者现在都经规范路径，而对逃逸路径的
  直接请求仍会按名字被拒绝。stdio 传输上的读或写失败会被返回，而不是安静地结束循环，因此
  一个服务不了的客户端不会看到"会话成功"。
- `xirang grafts` 在答不出问题时失败，而不是只答一部分。读不了的源码树、读不了的宿主入口、
  以及存在却读不了的计划目录，现在都会让命令在写出可读部分之后以非零退出，因为声明那一列否则
  就是猜的。文本本身损坏的计划仍然是一次成功的"计划已损坏"报告。
- 写不成的生成树会被报告，而不是从结构化调用方那里 panic 出去。构建脚本仍带着原因停下——它
  没有地方渲染诊断，因为生成树正是本该承载诊断的东西——而 `check --json` 把它当作
  `out-dir` 诊断报出。
- 运行期检查中，运行期无法精确说出的数值边界会被拒绝，而不是用猜测作答。旧的比较把两个边界
  都经 `f64` 舍入，因此 `number_in_range(9007199254740993, 9007199254740993)` 会接受
  `9007199254740992.0`——一个低于它自己下限的数。
- 源码遍历在深度上限处停下，而不是栈溢出。它是 workspace 唯一的递归遍历，而内核的文件系统
  事实来自调用方，因此无法 canonicalize 路径来区分"指向祖先的链接"与"确实很深的目录"；
  128 层远超任何真实布局，达到它的树会被报告。
- `function_symbols` 用一次前向扫描数行。它过去为找到的每个函数都从第 0 字节重数一遍，使这趟
  遍历的复杂度与函数个数相乘——而 MCP 的索引正是按那种形状遍历的。
- 含嵌套面的宿主 crate 现在能在 IDE 的 `rust_analyzer` cfg 下通过类型检查。`rust-analyzer`
  只在顶层应用 `#[path]`，因此每个嵌套面都会被第二次载入为 crate 根影子；影子里的 `super` 是
  crate 根，而拥有注册机的面派生的规则命名的是它自己文件旁边的模块，于是
  `E0433 cannot find registry_rule in super` 让整个 crate 在编辑器视图里失败。解析器现在为两种
  工具各答一次：`rustc` 仍然逐字节编译那条同目录路径，IDE 拿到的是兜底值。这项检查进了 CI
  （`IDE mirror (rust_analyzer cfg)`），因为此前没有任何步骤编译过这条 cfg。
- 删除一条外部 graft 记录需要两次按键，而文本解析不了的记录现在也删得掉。过去一次 `d` 就
  立刻移动目录，且删除路径先读计划，因此坏记录——读者最需要删掉的那一种——永远删不掉。现在
  第一次 `d` 只进入待删状态并点名该记录，任何其他键解除，第二次 `d` 才移动。选择类写入（插件
  表单）同样如此，而它两个文件中第二个写失败时会把第一个恢复回去。
- 新建项目的相对目录相对**打开的项目**解析，而不是 Studio 启动时所在的目录；脚手架中途失败
  不再留下任何东西。本来就存在的目录不会被删除：错误会说"里面留下了部分项目"。
- 没有可打开的项目时 Studio 拒绝启动，而不是显示一棵空树并以 0 退出。旧的解析会回退到本
  crate 编译时所在的目录——检出目录，或已安装 crate 的源码——因此从别处启动看起来一切正常，
  而下一条创作命令会把新注册面写进 NichLink 自己的树里。规则现在依次取：本会话的选择、路径
  参数、`XIRANG_PACKAGE_ROOT`、持有 `Cargo.toml` 的当前目录，并把每个指不到东西的候选按
  名字拒绝。`nichlink-toolchain` 另外接受 `[PROJECT]` 与 `--help`；启动失败会打印一行并在接管
  终端之前以非零退出。
- 重写注册面时把先前的文本留在读者找得到的地方。编辑器用自己建模的字段重建文件，因此手工
  加进去的内容不在结果里；写入本身是原子的，但那次丢失既静默又永久。现在先前的文本落在删除
  路径本就使用的 `.xirang/trash/` 下，且消息点出备份路径。顺带把 `delete_module` 拆成独立
  模块，正好付掉这次新增的代价：operations 页仍在尺寸棘轮之内。
- 入口或范围配置错误是诊断而不是 panic。`XIRANG_ENTRY` 指不到文件、畸形的
  `application!` 声明、不以 `crate::` 开头或解析不到的 `application!` 入口、schema 不对或
  含有任何节点都不拥有的身份的 `XIRANG_SCOPE` 取值、以及入口里畸形的 graft 声明，过去
  全都会 panic：构建确实失败了，但它把构建脚本一起打死，并让 `check --json` 什么都不打印。
  现在每一处都给出诊断（`entry`、`scope`、`graft-entry`）并**保守回退**——Cargo 约定、全树、
  空切口表——因此一次运行能报出它查得到的每个问题，而任何被拒绝的配置都不可能静默把注册面
  剪掉。范围取值由纯函数 `SourceScope::from_raw` 解析，因此这些拒绝无需改动进程环境即可钉住。
- 注册面旁边的普通模块不再让构建失败。宿主 `src/` 下任何平铺 `.rs`——比如
  `src/helpers.rs`——过去会以布局消息 panic，宿主自己的 `cargo build` 与每条读取该树的
  `xirang` 命令都一样，因为发现过程把这种文件当成了放错位置的注册面。现在发现过程问的是
  构建其余部分问的同一个问题（`parse_face` 返回 `Ok(None)` 即"这里没有面"），普通模块静默
  跳过，而确实是面的文件变成 `phase=face-layout` 诊断并点出路径与它该在的布局——因此
  `check --json` 产出的是文档而不是空白 stdout。解析不了的面同样被报告（`face-syntax`），
  且在解码字段的各阶段之前，那些阶段现在跳过它而不是打死进程。
- 深层嵌套的源码不再打死进程。`syn` 是无深度守卫的递归下降解析器，而 `proc-macro2` 只保护
  它自己的词法器，因此对几万层嵌套定界符（或一条 `Vec<Vec<…>>` 链）调用 `parse_faces` 会以
  `fatal runtime error: stack overflow` 结束——那是一次 SIGABRT，任何 `Result` 都报不出来、
  `catch_unwind` 也拦不住。三个 `syn::parse_file` 入口现在先跑一次嵌套扫描，量在 `proc-macro2`
  自己的 token 流上，因此字符串字面量与注释里的括号不会被误当成嵌套；上限取 128——`rustc`
  自己的默认 `recursion_limit`。四种形状由
  `core/src/registry_core/syntax/deep_input_tests.rs` 钉住。
- 进程后端不再有无限阻塞。三处都经实测后移除：从不排空 stdout 的 `try_wait` 轮询（写得超过
  管道缓冲的健康子进程会被杀掉并报成 `Timeout`，实测边界是 65 536 字节的帧成功、65 537 字节
  超时）；发生在超时启动之前的 `stdin` 写入（不读 stdin 的子进程会把调用方钉在超时之外，1 MiB
  阻塞了整个子进程生存期后报管道中断）；以及位于超时之外的帧读取。五条回归测试钉住这些边界。
- 钉住 `edit_module` 删除的 `compile_fail,E0433` doctest 此前没有任何命令编译它——唯一带
  `--all-features` 的任务传了 `--all-targets`，而它会完全跳过 doctest。现在会被编译。
- `MirGraph::to_jsonl` 对含控制字符的名字会原样写出该字符，产出严格 JSON 读取器一律拒绝的
  工件，而本 crate 自己的宽松解析器仍然接受它。修复前已复现（函数名里的制表符让两行输出都
  非法），修复后用独立的严格解析器验证。脚手架片段写入器给 VS Code JSON 用的也是同一份弱
  转义；两者现在都用内核编码器。
- `tools/nichlink-package-audit` 用一个按空白切分的循环读取 crate 的依赖表，因此
  `nichlink-toolchain` 与 `nichlink-toolchain` 只按第一个依赖被评估，摘要里还列出了并不存在的 crate。
  依赖改为逗号分隔并据此解析，摘要对每个 crate 只列一次。
- 进程适配器在多个插件调用并发时不再报出假失败。`exec` 可能瞬时以 `ETXTBSY`
  （"Text file busy"）拒绝一个刚暂存的可执行文件，在并行测试中表现为 `ExecutableFileBusy`；
  现在派生会有界重试，其他错误仍原样上报。
- 被 `#[allow(dead_code)]` 与 `#[allow(unused_imports)]` 压住的死代码已清除：只写不读的
  `RegistryHeader::name` 与其无人调用的 `admission()` 访问器（连同藏起它们的模块级 allow）、
  `EntryPages::iter()`、`FaceManifest::get()`/`fields()`、`SearchRow::source_index`（写了四次、
  一次也没读）、无人引用的 `state::FACE_FORM_ORDER` 重导出、一个其实有调用者却被标了
  `#[allow(dead_code)]` 的函数，以及约四十条未使用的 import。**删除 allow 而不是保留它**，
  正是最后这一组的发现方式：rustc 会点名真实的集合。
- 九个已发布 crate 现在都开启 `#![warn(missing_docs)]` 并补全了各自的公开面文档（九个
  crate 合计约 700 项），因此后续发布无法再引入没有文档的公开项。开启过程中发现的两处下游
  警告一并修掉：`__registration_face!` 发射的 `NODE_ID`/`REGISTRATION`，以及构建生成的
  `builtin_static_plan()` / `registrations()` / 容器模块，现在都带文档，宿主开启该 lint 后
  不会再收到指向它无法编辑的生成代码的警告。
- `nichlink-dev` 不会再以坏掉的状态被安装：它位于非默认 `dev-supervisor` 特性之后，按
  同级目录、`PATH`、工作区 `target/debug` 的顺序解析 Studio 二进制，并在它被编译时所在的
  检出已消失时报出该路径。
- `apply_recorded_grafts` 对三种没有合法解读的产物改为拒绝而不是跳过：解析不了的计划
  （一次报出全部不可读计划）、目录与计划里的 `graft` 不一致的记录
  （`RecordReport::SelectorDirectoryMismatch` 已删除）、以及身份与路径指向不同面的记录。

- 仓库门禁不再接受它们本应拒绝的东西：内核纯净性现在看得见 `std::io`/`std::thread`/`std::os` 与树形
  导入（`use std::{env, fs}`）并有"走过文件"的下限；挂载门禁无论 `include!` 用哪种定界符都能发现它
  （跑在内核屏蔽后的文本上，因此夹具字符串不算挂载）；文档门禁会报告未闭合的围栏，并覆盖 `docs/` 下
  任意深度的每个 markdown 文件与每个根级 markdown 文件；lint 门禁能读出被 rustfmt 折行的
  `#[allow(…)]`，并从目录树推导必须带 `#![warn(missing_docs)]` 的根，而不是用一份新 crate 可以漏掉的
  清单；`tools/nichlink-package-audit` 从清单推导已发布 crate 集合及其内部依赖；
  `tools/nichlink-publish --check-table` 能读 `x.workspace = true` 形式的内部需求并检查根的
  `[workspace.dependencies]` 版本；嵌套守卫在删掉永远不会触发的泛型实参计数器之后，报出**适用于**
  该形状的上限（线性串 1024、定界符组 128）。
- `application!(entry = …)` 的每一段路径都对包内目录树解析，因此规范的 `<dir>/<dir>.rs` 布局可用
  （`src/control/control.rs` 对应 `entry = crate::control`），而夹在中间解析不出任何东西的段会被拒，
  不再被当作函数名放过。解析器位于 `toolchain/build_method/src/entry_paths.rs`；为使 `entry.rs` 留在尺寸棘轮
  之内，它与该文件里的文件系统遍历（并入 `discovery.rs`）一起被拆出。
- `WasmLimits::max_element_bytes` 约束一个模块能让 wasmi 在实例化时物化多少：被动元素段从不增长
  表，因此 `table_elements` 从来看不到它，而实测代价是每条约 32 字节（紧凑编码每条约一字节）——
  一个 2 000 103 字节、带两百万条目的模块花掉 64 070 402 字节宿主内存。负载与工件上限一样，在
  编译前从段头量出。
- `PluginArtifact::verify_signed` 对不会咨询验证器的来源返回新增的
  `PluginTrustError::SignatureLaneRequired`，而不是记录一个没有检查过任何东西的 `Signature` 保证；
  同一工件的纯摘要路径仍然可用。
- 以 `.` 开头的 graft 选择器被写入方、解析方与 Studio 已在共用的那条规则拒绝：`..` 过去会被接受、
  把计划写到 `<pkg>/.xirang/graft.plan`，然后永远不会被列出——一份运行期从不应用、作者却看到
  "计划已创建"的记录。
- 构建产物只在仍然描述当前源码时才被信任：`xirang_toolchain::build_method::build_output_is_current` 把已发布
  的 `discovery.fingerprint` 与新算出的指纹比对，pipeline 只在干净的一次运行写下那枚指纹，而
  `explain`（以及 `--overlay`）在它缺失或过期时报 `known: false` 并沿用既有的"跑 `xirang
  check`"提示。失败的 check 不再留下会被 `explain` 当作当前作用域提供的产物。
- 畸形或带不可求值门控的 `static_graft_plan!` 现在是 `phase=graft-entry` 诊断而不是 panic：
  `check --json` 过去以 101 退出、stdout 为空，并把 `scope` 已经产出的诊断丢掉。读取者现在接收构建
  的诊断集合，三类拒绝都经它上报，且解析失败的消息与 `scope` 的逐字相同，两者因此折叠成一行。
- 文档门禁此前把围栏 Rust 直接交给 `syn`，因此嵌套几万个定界符的围栏会让门禁进程 abort 而不是
  失败——与内核解析入口当初的缺陷同一类，而这是最后一个还有它的地方。现在它向内核的
  `guard_nesting` 提问，那是本工作区唯一的嵌套度量（它公开出来正是为了这件事），并由
  `a_pathologically_nested_fence_is_reported_not_fatal` 钉住"拒绝是一条消息"：把守卫删掉，该
  测试就会 abort。
- `tools/nichlink-publish` 按词读取自己的依赖表，因此有多个内部依赖的 crate 只报出第一个——
  `nichlink-toolchain` 只被按 `nichlink-toolchain` 检查——"依赖还没上 index 就不许发布"的守卫因此
  形同虚设。同一次遍历还把边行的被依赖者当成独立 crate，九行表产出十四个节点。现在两张表都按整行
  读取，`--check-table` 会把它们与清单对比：第一次运行就找出一条真实漂移——`nichlink-toolchain` 直接依赖
  `nichlink-core`，而表里只列了另外三个。同一个检查用 `awk` 而不是 `sed` 地址范围读取工作区成员，
  因为 sed 的范围不在起始行上测试结束地址：单行的 `members` 数组会让范围一直跑到
  `[workspace.package]`，它的名字随后进入成员列表——只因不存在同名目录才隐形。
### [0.1.0] 首次发布（2026-09-25 已发布）

九个 crate 在发布时按依赖顺序一同发布：`nichlink-core` → `nichlink-macro` /
`nichlink-toolchain` / `nichlink-toolchain` → `nichlink-toolchain` →
`nichlink-toolchain` / `nichlink-toolchain` → `nichlink-toolchain` →
`nichlink-toolchain`。依赖版本要求写 caret `0.1.0`，因此补丁版本不会连锁要求依赖方重发。

各 crate 职责见上方英文列表。本版本线的已知边界见根 `README.md` 的 `## Boundaries` 一节与
[`docs/threat-model.md`](docs/threat-model.md)；kind-only `registry_name` 派生等
随版本变化的行为记录在 [`docs/roadmap-1.0.md`](docs/roadmap-1.0.md)。
