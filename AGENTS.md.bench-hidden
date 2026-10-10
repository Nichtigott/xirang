# XiRang architecture notes

XiRang splits the workspace into one pure kernel and thin execution surfaces.
The rule that decides where new code goes:

**Kernel = protocol vocabulary + pure methods (verbs), no execution.**
Anything that reads the filesystem, touches `std::env`, spawns processes,
drives a terminal, or binds to process lifetime belongs in an execution
surface that binds kernel methods to its own context.

## Crates

| Crate | Directory | One-line responsibility |
| --- | --- | --- |
| `xirang-kernel` (lib `xirang_kernel`) | `kernel/` | Kernel: protocol nouns and pure methods |
| `xirang-toolchain` | `toolchain/` | The seven execution surfaces of the old nine-crate layout, now one module each: `build_method` (the `cargo build` method: filesystem / `OUT_DIR`), `run_method` (the `cargo run` method: runtime state instance + trace binding), `call_evidence` (observation evidence), `plugin_host` (wasm/process plugin host execution), `studio` (ratatui authoring/inspection), `mcp` (AI-agent stdio bridge: source and registry queries, plus the previewed authoring write path), `cli` (argv dispatch, cargo subprocesses) |
| `xirang-macro` | `macro/` | Compile-time face field front end: accepted order, tolerant separators, diagnostics |
| `xirang-conventions` | `conventions/` | Repository-convention gates (`publish = false`: they walk the checkout, so an unpacked copy would have nothing to check) |

Feature names intentionally follow each crate's own vocabulary instead of one
workspace-wide scheme: `kernel` gates the parser as `syntax`, `run_method` gates
the authoring executor as `authoring`, `studio` gates test-only fixtures as
`prototype-fixtures`, and `plugin-host` uses `wasm` / `process-tools`. Do not
rename a feature to match another crate: they are public API. `prototype-fixtures`
gates test-only fixtures — the source-only host package under
`toolchain/tests/fixtures/node-editor/` — and is not part of the published surface.
It is off by default, so the CI job that runs the workspace with
`--all-features` is the only gate that exercises it; keep that job green.
`studio` also gates its `xirang-dev` rebuild supervisor behind the
non-default `dev-supervisor` feature: that binary rebuilds Studio from this
checkout, so an installed copy could never work, and `required-features` is the
per-target equivalent of `publish = false` — it is what keeps
`cargo install xirang-toolchain` from shipping a tool with nothing to rebuild.
特性名有意遵循各 crate 自己的词汇，而不是全工作区统一命名：`kernel` 把解析器门控为
`syntax`，`run_method` 把 authoring 执行器门控为 `authoring`，`studio` 把仅测试用的
fixture 门控为 `prototype-fixtures`，`plugin-host` 使用 `wasm` / `process-tools`。
不要为对齐其他 crate 而重命名特性：它们是公开 API。`prototype-fixtures` 只门控测试用
fixture——`toolchain/tests/fixtures/node-editor/` 下仅源码的宿主包——不属于发布表面。它默认
关闭，因此只有以 `--all-features` 跑整个工作区的 CI 任务会碰它；保持那个任务常绿。
`studio` 还把 `xirang-dev` 重建监督器门控在非默认的 `dev-supervisor` 特性之后：该二进制
从本检出重建 Studio，安装副本永远做不到，而 `required-features` 就是按 target 的
`publish = false` 等价物——正是它让 `cargo install xirang-toolchain` 不会带出一个没有东西可
重建的工具。

A feature that shares a name with a module gates that module, so the sameness is
the point rather than a collision: `kernel`'s `syntax` gates the registration-face
parser, `run_method`'s `authoring` gates the authoring executor, and
`plugin-host`'s `wasm` / `process-tools` name execution backends. The two that are
not capabilities say so in their own names: `prototype-fixtures` gates the
checkout-only fixture package under `toolchain/tests/fixtures/node-editor/`, and
`dev-supervisor` gates the `xirang-dev` binary that rebuilds this checkout (the
table above is the whole set — no crate outside `kernel`, `run_method`, `studio` and
`plugin-host` declares features).
与模块同名的特性门控的就是那个模块，所以同名是要点而不是撞车：`kernel` 的 `syntax` 门控注册面
解析器，`run_method` 的 `authoring` 门控 authoring 执行器，`plugin-host` 的 `wasm` /
`process-tools` 点名的是执行后端。另外两个不是能力，名字里就写明了：
`prototype-fixtures` 门控 `toolchain/tests/fixtures/node-editor/` 下仅检出可用的夹具包，
`dev-supervisor` 门控重建本检出的 `xirang-dev` 二进制（上表就是全集——`kernel`、
`run_method`、`studio`、`plugin-host` 之外没有 crate 声明特性）。

Host usage: `[dependencies] xirang-toolchain` +
`[build-dependencies] xirang-toolchain`; the crate root calls
`xirang_toolchain::run_method::host!();` and the thin `build.rs` calls
`xirang_toolchain::build_method::run()`. `run_method`'s `entry` module was removed: the
build-time entry is now `host!()` plus `build.rs`, and `application!(entry = …)`
remains only as the optional source-scope discovery hint.
宿主用法：`[dependencies] xirang-toolchain` +
`[build-dependencies] xirang-toolchain`；crate 根部调用
`xirang_toolchain::run_method::host!();`，薄 `build.rs` 调用
`xirang_toolchain::build_method::run()`。`run_method` 的 `entry` 模块已移除：构建期入口现在是
`host!()` 加 `build.rs`，`application!(entry = …)` 仅作为可选的源码范围发现提示保留。

## Toolchain modules (batch 2: nine crates → three)

`xirang-toolchain` is one crate with seven modules, each one a former crate:
`build_method`, `run_method`, `call_evidence`, `plugin_host`, `studio`, `mcp`, `cli`.
`toolchain/src/lib.rs` declares each with `#[path = "<module>/src/lib.rs"] pub mod <module>;`
and then re-exports every module with a `cfg`-gated `pub use self::<module>::*;`, so a path
written as `crate::<name>` inside a module's own sources still resolves after the merge.
The host-facing pair is on by default — `default = ["build", "run"]` — because the documented
host entries are `build_method::run()` and `run_method::host!()`; the optional backends stay behind
their own features (`wasm`, `process-tools`, `node-graph`, `authoring`, `prototype-fixtures`,
`dev-supervisor`), so nothing optional is switched on silently. The five bin names are kept
(`xirang`, `cargo-xirang`, `xirang-mcp`, `xirang-studio`, `xirang-dev`), with
`autobins = false` and a `required-features` entry per target.
**Macro reachability needs its own audit whenever code moves**: `#[macro_export]` hoists a
macro to the crate root (a module path will not resolve it); a `macro_export` macro of the
current crate cannot be called by an absolute path from inside a macro expansion; a bare-name
`macro_rules!` call resolves at the call site, so it breaks external callers; and a private
`macro_rules!` is reachable only through textual scope, so a test that uses one has to be
mounted inside the module that defines it. **Wired back (2026-09-29)**: the six files that batch 2
left without a target (20 `#[test]` items) are targets again — nineteen as integration tests under
`toolchain/tests/` (an *external* caller reaches the tolerant arm; rule ② only bans same-crate
absolute-path calls from inside an expansion), and the collector probe in-crate because it reads
*this* crate's registry. Wiring them back is what exposed the branch only they reached: a
`collector: debug` declaration could not compile (`$crate::submit!`, not
`$crate::call_evidence::submit!`, now that the macro is hoisted), and the merged crate needed
`extern crate self as xirang_toolchain;` for expansions that still carry
`::xirang_toolchain::…` paths. `docs/b3-registration-diagnosis.md` keeps the diagnosis for the
record.
`xirang-toolchain` 是**一个 crate、七个模块**，每个模块就是原来的一个 crate：
`build_method`、`runtime`、`call_evidence`、`plugin_host`、`studio`、`mcp`、`cli`。
`toolchain/src/lib.rs` 用 `#[path = "<module>/src/lib.rs"] pub mod <module>;` 逐个声明，再用
**cfg 门控的** `pub use self::<module>::*;` 把每个模块重导出，于是各模块自己源码里写成
`crate::<name>` 的路径在合并之后仍然解析得到。**宿主面两半默认打开**：`default = ["build", "run"]`
——因为文档承诺的宿主入口就是 `build_method::run()` 与 `run_method::host!()`；可选后端各自留在自己的
特性后面（`wasm`、`process-tools`、`node-graph`、`authoring`、`prototype-fixtures`、
`dev-supervisor`），不会有任何可选能力被静默打开。五个 bin 名全部保留（`xirang`、
`cargo-xirang`、`xirang-mcp`、`xirang-studio`、`xirang-dev`），配 `autobins = false`
与逐 target 的 `required-features`。**代码一搬动，宏的可达性就要单独审计**：`#[macro_export]`
会把宏提升到 crate 根（写模块路径解析不到）；同 crate 的 `macro_export` 宏**不能在宏展开里经
绝对路径**调用；裸名 `macro_rules!` 调用在**调用点**解析，因此会弄坏外部调用者；私有
`macro_rules!` 只能靠文本作用域到达——用它的测试**必须挂载在定义它的模块之内**。**已接回（2026-09-29）**：
批 2 里没有 target 的那六个文件（20 个 `#[test]`）重新有了 target——其中十九个是 `toolchain/tests/` 下的
集成测试（**外部**调用者能到达宽容分支；规则 ② 只禁"同 crate 展开里经绝对路径调用"），采集探针则留在
crate 内，因为它读的是**本 crate** 的注册表。接回它们才暴露出"只有它们才会走到"的那一支：`collector: debug`
的声明根本编译不过（宏提升到根之后应是 `$crate::submit!` 而不是 `$crate::call_evidence::submit!`），
而合并后的 crate 还需要 `extern crate self as xirang_toolchain;` 才能让展开里那些
`::xirang_toolchain::…` 路径解析。诊断按记录留在 `docs/b3-registration-diagnosis.md`。

## Kernel modules (`kernel/src/registry_core/`)

- `identity` — stable ids, hashing (SHA-256), namespaces
- `declaration` — face/object vocabulary
- `tree` — registry tree operations, passive recursive registration
- `syntax` — graft syntax, application/graft entry parsing
- `authoring` — face authoring parse/validation/snapshot (fs executor lives in run_method)
- `diagnostic` — `RegistryError`, `RegistrationState`, build diagnostics, topology checks
- `requirements` — capability declarations and requirements
- `plugin` — plugin protocol, slot/channel validation
- `mir` — MIR text/JSONL parsing, call-graph merge
- `source` — lexical source scanning (functions, spans, calls)
- `release` — release-time pruning/plan vocabulary
- `json` — shared JSON string encoding for every artifact this workspace writes (build diagnostics, MIR JSONL, editor snippets)
- `lexicon` — shared text contracts (generated-entry file name, runtime crate name, environment variables, `.xirang` paths, the partition's generated-crate
directory, scope exemptions)

Module mounting is uniform workspace-wide: a module file is declared by its parent
with `#[path = "<dir>/<name>.rs"] pub mod <name>;`. There is no `mod.rs`. The
`#[path]` is what gives a module directory-relative resolution for its own
children — the behaviour `mod.rs` would give — which is why the `<dir>/<name>.rs`
layout works without one. A bare `mod x;` is equally correct when the parent is a
crate root or is itself loaded with `#[path]`, and the tree uses that form in both
positions (the count is not restated here: it changes with every new module);
the rule that actually matters is that a module is never spliced in with
`include!`. A splice changes the `file!()` a face records and therefore its
`NodeId`, silently; identities are written into on-disk graft records, so the
damage would surface later as records that no longer resolve. The only `include!`
is `host!()`'s `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`, which
pulls in the generated plan rather than a source module. The `conventions` crate
gates both rules.
模块挂载在整个工作区统一：模块文件由其父模块用
`#[path = "<dir>/<name>.rs"] pub mod <name>;` 声明，没有 `mod.rs`。`#[path]` 的作用是让
模块以所在目录为基准解析自己的子模块——也就是 `mod.rs` 能给出的行为——这正是
`<dir>/<name>.rs` 布局无需 `mod.rs` 的原因。当父文件是 crate 根或本身经 `#[path]` 载入时，
裸 `mod x;` 同样正确，树里两种位置都有这种写法（这里不再复述具体数字：它随每个新模块变化）；真正要守的规则是绝不用 `include!` 把模块拼进来。
拼接会改变注册面记录的 `file!()`，从而静默改变它的 `NodeId`；身份会写入落盘的 graft 记录，
损害会在以后表现为不再解析的记录。唯一的 `include!` 是 `host!()` 的
`include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`，它引入的是生成的计划而不是源码
模块。两条规则都由 `conventions` crate 门控。

`kernel`'s crate root re-exports the module hierarchy plus a curated vocabulary that
host code, the build step, and the runtime-check surfaces write as a bare name.
The kernel's modules also glob into the root, so every other noun is *reachable*
there as well — that is convenience, not a second contract: the module path
(`xirang_kernel::identity::NodeId`) is the official address, and only the curated names
are promised as bare ones.
`kernel` 的 crate 根部重导出模块层级，外加一份精选词汇——宿主代码、构建步骤与运行期校验面
会以裸名书写它们。内核各模块也会平铺 glob 到根部，因此其余名词同样**可以**在那里取得——
那是便利，不是第二份契约：官方地址是模块路径（`xirang_kernel::identity::NodeId`），只有精选
清单上的名字被承诺为裸名可用。

## Naming and mounting

A mount name and a file stem are allowed to differ, and every place where they do
is one of four families: historical shim re-exports (2), CLI command modules (5,
e.g. `build_command` for `commands/build.rs`), `explain`'s sub-modules (3, e.g.
`json` for `explain_json.rs`), and test modules (4, e.g. `mod tests;` for
`lib_tests.rs`). What the four families share is the reading rule: the mount name
says what the module *means*, the file name keeps the *subject*. The counts are
not restated here — every rename moves them — so the rule is what a reviewer
checks, not the number. Renaming a mount is not free: it changes the module
prefix of that module's test leaf names.
挂载名与文件 stem 允许不同，而全仓所有不同的地方就是四族：历史 shim 重导出（2 处）、CLI 命令
模块（5 处，如 `commands/build.rs` 的 `build_command`）、`explain` 的子模块（3 处，如
`explain_json.rs` 的 `json`）、测试模块（4 处，如 `lib_tests.rs` 挂成 `mod tests;`）。四族
共享的读法是：挂载名说这个模块**意味**什么，文件名保留它**是什么**。数字不在此复述——每次
改名都会移动它们——所以评审要查的是规则而不是数字。改挂载名不是免费的：它会改变该模块测试
叶子名里的模块前缀。

`pub use … as …` is an alias, and an alias needs a reason it can point at: either
the two names are the same name for the same thing, or the alias's first doc line
names the historical path it keeps alive. All four of the workspace's aliases carry
one — `ValidationChannel`, `context` as `validation`, `graft_document`, and the two
`__`-prefixed macro re-exports (the last pair is one reason written twice).
`pub use … as …` 是别名，而别名需要一个可指出的理由：要么两个名字是同名同物，要么别名的首行
文档点名它在保住哪条历史路径。工作区现有的四个别名都带理由——`ValidationChannel`、`context`
别名成 `validation`、`graft_document`，以及两个 `__` 前缀的宏重导出（最后一对是同一个理由
写了两遍）。

A test-only sibling is mounted under its own name: `<name>_tests.rs` is declared as
`mod <name>_tests;` inside `#[cfg(test)]`. The `size` ratchet keys on that attribute
rather than on the module name, so the name is a reading aid and not something a
gate enforces.
测试专用兄弟文件按它自己的名字挂载：`<name>_tests.rs` 在 `#[cfg(test)]` 里声明为
`mod <name>_tests;`。`size` 棘轮认的是那个属性而不是模块名，因此名字是给读者的便利，而不是
门禁强制的规则。

**Retained names (B8).** This is where the ambiguity findings' decisions land. The
decisions are the per-row review in
`docs/audit-2026-09-28/audit-naming-ambiguity-verify.md` (§2 MAJOR table, §3 MINOR
sample) and the 9→3 decision table in
`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`; the `fix_hint` field in
the findings JSON is one template sentence for all twenty and is **not** a decision.
Below, `verify.md` and `merge-plan.md` abbreviate those two files.
**保留的名字（B8）。** 歧义发现的裁定落在这里。裁定本体是
`docs/audit-2026-09-28/audit-naming-ambiguity-verify.md` 的逐条复核（§2 MAJOR 表、§3 MINOR
抽样）与 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md` 的 9→3 决策表；发现
JSON 里的 `fix_hint` 是二十条共用的一句模板，**不是**裁定。下文用 `verify.md` 与
`merge-plan.md` 指代这两份文件。

| kept | why it is kept, and what is scheduled | source |
| --- | --- | --- |
| `kernel/` + `xirang-kernel` + lib `xirang_kernel` | **Landed in batch 1**: the old `core/` directory, the `xirang-core` package and the `xirang` library are gone (the ambiguity this row registered is why `kernel/` was chosen). The *crate* merge into `xirang-toolchain` is batch 2, and the published 0.1.x names stay frozen. | `verify.md:62`, `merge-plan.md:38/:53` |
| `toolchain/build_method/`, `toolchain/runtime/`, `toolchain/plugin_host/` | Each crate name is kept and its README says what the word means in this repository; the *module* names `build_method`, `run_method`, `plugin_host` belong to the merge batch. | `verify.md:63-65`, `merge-plan.md:17/:19` |
| `Registry`, `examples/`, `picture/` | Kept. The public type is the registration-tree root and its doc line limits it to that; `examples/` holds two host packages rather than cargo example targets; `picture/` holds the brand wordmark and Studio screenshots. | `verify.md:66/:67/:79` |
| `toolchain/call_evidence/` | Kept with no extra action; the crate becomes the `call_evidence` module, so the name disappears in the merge. | `verify.md:77`, `merge-plan.md:42` |
| bin `xirang-dev`, feature `dev-supervisor` | Kept; `required-features` is per-target `publish = false`, and both names are already the contract scripts and CI write. | `verify.md:78`, `merge-plan.md:24` |
| `docs/ROADMAP.md` vs `docs/roadmap-1.0.md` | Kept, and neither may be renamed or exported into one directory on a case-insensitive filesystem: the two names would overwrite each other there. | `verify.md:80` |
| kernel modules `syntax`, `json`, `release`, `requirements`, `source`, `index`, `host` | Kept and registered here rather than renamed; each one's module doc says which sense it means. | `verify.md:81-83` |
| feature names | Kept; the table in the Crates section says which module each one gates. | `verify.md:84`, `merge-plan.md:23` |
| the bare verbs (`cli`'s `main`, build's `run`, …) | The rule is that a bare verb is only allowed at an entry position (D-5); the verb table itself lives in the kernel's `lexicon` module so the vocabulary has one home. | `verify.md:85` |

## Change rules

1. New pure logic goes into a kernel module under `kernel/src/registry_core/<name>/`
   and is registered in `kernel/src/registry_core.rs`.
2. Execution surfaces keep historical paths through shim re-exports
   (`pub use xirang_kernel::tree; pub use xirang_kernel::tree::*;`) — do not move public
   API paths without updating the shims.
3. Kernel code must not do I/O, read `std::env`, or depend on process lifetime.
   If it needs such a value, take it as a parameter.
4. Keep crate names, directory names, and lib names consistent
   (`xirang-<x>` / `<x>/` / `xirang_<x>`); scaffold templates in
   `toolchain/build_method/` and CI reference them too.

## Verify

```sh
cargo fmt --all
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo clippy --workspace --all-targets --offline --all-features -- -D warnings
tools/xirang-publish --check-table
```

The clippy line has two faces, and they are two different builds: `-D warnings` under the default
features and under `--all-features`. Read them as a pair, the same way the test runs above are read
as a pair. The all-features face is the stricter one — it is where a name carried by two root globs
(`ambiguous_glob_reexports`) or a module nested under its own name (`clippy::module_inception`)
shows up — and CI's all-features job already runs it (`ci.yml`). This checkout's own landing set
used to stop at the default face, which is how both of those lived unseen at
`toolchain/src/lib.rs` and `toolchain/src/studio/src/lib.rs` until 2026-09-29; run both before
committing.
clippy 那一行有**两面**，而它们是**两套不同的构建**：默认特性下的 `-D warnings` 与
`--all-features` 下的 `-D warnings`。请像读上面那两条测试命令一样**对比着读**。`--all-features`
那一面更严——被两条根部 glob 同时带上的名字（`ambiguous_glob_reexports`）、以及与所在模块同名的
嵌套模块（`clippy::module_inception`）都只在这一面现形——CI 的 all-features 作业本来就在跑它
（`ci.yml`）。本检出自己的落地门禁过去只跑到默认那一面，因此这两种情况在
`toolchain/src/lib.rs` 与 `toolchain/src/studio/src/lib.rs` 里一直没被看见，直到 2026-09-29；
提交前两面都要跑。

The last line is the release tables' gate: `tools/xirang-publish` publishes in a
hand-maintained dependency order, so it derives the truth from the manifests and
compares both of its tables against them. It needs no network and no token.
最后一行是发布表的门禁：`tools/xirang-publish` 按一份手工维护的依赖顺序发布，因此它从清单
推导事实，并把自己那两张表与清单对比。它不需要网络也不需要 token。

The three `cargo` commands run with `--offline` because this checkout is
developed offline. CI has a network and deliberately does **not** pass the flag: a
cold runner has an empty registry cache, so `--offline` there would fail every job
instead of making it stricter. The table check takes no such flag: it reads only
this checkout.
那三条 `cargo` 命令带 `--offline`,因为本检出在离线环境下开发。CI 有网络,因此有意**不**传
该标志:冷启动的 runner 注册表缓存为空,在那里加 `--offline` 只会让每个任务失败,而不是让它
更严格。表检查不需要这类标志:它只读本检出。

A single crate can also be verified into a false green. When a crate's tests sit behind a
non-default feature, `cargo test -p <crate>` compiles it without that feature and runs none of
them, reporting `0 passed` — which reads like a pass. Measured here: `run_method`'s
`manifest::face` module (which carries the two nesting-guard pins from audit `LGC-LG-05`) is
gated behind `authoring` in `toolchain/run_method/src/lib.rs`, so
`cargo test -p xirang-toolchain --offline -- manifest::face` prints `0 passed; 0 failed`
while the same command with `--features authoring` prints that module's pins (`8 passed` when
this was measured; the count grows with every pin, so treat it as an illustration rather than a
contract). `kernel` has the same shape, with its parser behind `syntax` (`kernel/src/lib.rs` gates
it), and every other non-default feature holds modules back the same way — so read the two runs
against each other, never a quoted total: `0 passed` is the half that stays true, and it means
nothing ran. The workspace command above and CI's `--all-features` job merge features and do
reach all of them, which is why the workspace run stays the default gate; verifying one crate
on its own means naming the feature:
`cargo test -p xirang-toolchain --offline --features authoring`. Read `0 passed` as
"nothing ran", not as "nothing failed".
单个 crate 也可能被"验证"出一个假绿。当一个 crate 的测试门控在非默认特性之后时,
`cargo test -p <crate>` 会在不带那个特性的情况下编译它,于是一个都不跑,报出 `0 passed`
——看起来就像通过了。本检出实测:`run_method` 的 `manifest::face` 模块(审计 `LGC-LG-05`
那两条嵌套守卫钉子就在其中)门控在 `toolchain/run_method/src/lib.rs` 的 `authoring` 之后,因此
`cargo test -p xirang-toolchain --offline -- manifest::face` 打印 `0 passed; 0 failed`,
而同一条命令加 `--features authoring` 会打印出那个模块的钉子(写下这段时是 `8 passed`;
这个数每加一条钉子就变,所以只当示意,不当契约)。`kernel` 是同一种形状:解析器门控在 `syntax`
之后(`kernel/src/lib.rs` 就是那道门),其它非默认特性也以同样的方式把模块挡住——所以请把两次
运行**对比着读**,不要去引某个总数:`0 passed` 才是恒真的那一半,它只意味着什么都没跑。
上面的工作区命令与 CI 的 `--all-features` 作业会合并特性,因此覆盖得到它们——这也是工作区
那条仍是默认门禁的原因;单独验证一个 crate 时请点名特性:
`cargo test -p xirang-toolchain --offline --features authoring`。把 `0 passed` 读作
"什么都没跑",而不是"什么都没失败"。

`cargo test --workspace` also runs the gates in the `conventions` crate, so the
default gate already fails on: I/O in `kernel/src`, a `mod.rs`, a second `include!`,
a kernel module file no `mod` declaration names, a deleted execution-surface shim
re-export, a doc block with only one language (`bilingual`), a crate whose name,
directory and library name disagree, or a crate name referenced by a scaffold
template or a CI `-p` that no manifest defines (`naming`), an internal
`xirang-*` requirement whose version is not the current workspace version
(`release_version`), a workflow whose upload step can run
without a tag ref (`release_workflow`), an anchor in a live document that no
longer points at a line of a `.rs`, `.toml`, `.yml` or `.yaml` file
(`doc_anchors`: the scan keys on those suffixes, reading the root and member
`Cargo.toml`s and `.github/workflows/*`; a `.md` target and an extensionless
script are not covered), a feature-gated target
whose `required-features` is missing (that half is judged for the hand-listed
in-checkout-only targets; for every other target the gate checks that the named
features exist and are not on by default), a `prototype-fixtures` that default
features turn on, a file pushed past the
600-line ratchet, a missing `#![warn(missing_docs)]`, an
`#[allow(missing_docs)]`, or a fenced Rust block in a README that no longer
parses. Add a new repository-wide rule there rather than to a prose document. The
kernel's parse entries carry a nesting guard for the same reason — a stack
overflow is not a `Result`, so a pathological source would take the whole surface
down — and `kernel/tests/nesting_budget.rs` is the other half of it: the guard must
refuse nothing this repository ships. Adding a shape to the guard means adding a
case to `deep_input_tests.rs` and re-running that gate. CI additionally runs `cargo test --workspace --all-features --doc`: `--all-targets`
skips doctests, and the `authoring`-gated `compile_fail` pin only exists under
`--all-features`.
`cargo test --workspace` 也会跑 `conventions` crate 里的门禁,因此默认门禁已经会在下列情形
失败:`kernel/src` 里出现 I/O、出现 `mod.rs`、出现第二个 `include!`、没有任何 `mod` 声明指名的
内核模块文件、被删掉的执行面 shim 重导出、只有一种语言的文档块(`bilingual`)、crate 名/目录名/
库名三者不一致或脚手架模板或 CI `-p` 指名而清单里不存在的 crate 名(`naming`)、内部
`xirang-*` 版本要求写着的版本不是当前工作区版本(`release_version`)、上传步骤能在非 tag ref 上
运行的发布工作流(`release_workflow`)、活文档里不再指向某个 `.rs`、`.toml`、`.yml` 或 `.yaml`
文件某一行的锚点(`doc_anchors`:扫描以这些后缀触发,读根与各成员的 `Cargo.toml` 以及
`.github/workflows/*`;`.md` 目标与无扩展名脚本不在覆盖内)、按特性门控的 target 少了
`required-features`(这一半只对**手列**的那几个
"只在检出内成立"的 target 判定;其余 target 检查的是指名的特性存在、且默认不开)、
`prototype-fixtures` 被默认特性打开、文件越过 600 行棘轮、缺少
`#![warn(missing_docs)]`、出现 `#[allow(missing_docs)]`、或 README 里有不再能解析的 Rust
围栏。新增全仓规则请加到那里,而不是加到散文文档里。内核的解析入口同样带一道嵌套守卫——
栈溢出不是 `Result`,畸形源码会带走整个执行面——而 `kernel/tests/nesting_budget.rs` 是它的
另一半:守卫不得拒绝本仓库出厂的任何东西。给守卫加一种形状,就要在 `deep_input_tests.rs` 里
加一条用例,并重跑那道门禁。CI 另跑
`cargo test --workspace --all-features --doc`:`--all-targets` 会跳过 doctest,而门控在
`authoring` 之后的 `compile_fail` 钉子只在 `--all-features` 下存在。

`tools/xirang-external-rehearsal` copies the two example hosts outside the
checkout, repoints their `path` dependencies here, detaches them from the
workspace and builds them from scratch. Every other check builds the examples
where the workspace's members, relative paths, shared `target/` and shared
lockfile all happen to be right; this is the one that fails when a host layout
assumption only holds in-tree. It has a third leg for the *generated* host:
with the checkout's own CLI it scaffolds a binary and a library project outside
the checkout (one letting the CLI detect this checkout, one told `--path`) and
runs `cargo test --offline` in each, because a template that names the wrong
crate compiles nowhere until somebody builds what it writes — which is how five
mis-anchored paths survived two rename batches.
`tools/xirang-publish --verify-consumers` is its counterpart for published
crates, and it needs the index.
`tools/xirang-external-rehearsal` 把两个示例宿主复制到检出之外，把它们的 `path` 依赖指向
这里，让它们脱离工作区并从零构建。其他所有检查都在"工作区成员、相对路径、共享 `target/` 与
共享 lockfile 恰好都成立"的地方构建示例；只有这一条会在"宿主布局假设只在树内成立"时失败。
它还有第三条腿，针对**生成出来**的宿主：用本检出的 CLI 在检出之外生成一个二进制项目与一个库
项目（一条让 CLI 自行探测本检出，一条显式给 `--path`），并各跑一次 `cargo test --offline`
——因为一份把 crate 名写错的模板在有人构建它写出的东西之前，在哪里都编译不过，而五处失锚正是
这样熬过了两个改名批次。`tools/xirang-publish --verify-consumers` 是它在已发布 crate 上的
对应物，那条需要 index。

`tools/xirang-partition-rehearsal` partitions a host three ways — as a member of a
workspace, with no workspace above it, and in the publishable release shape — builds each,
and checks the four facts a partition must not change: the host's own sources byte-identical,
the same `graft_plan.tsv` in every shape, the same face identities, and a `promote` that still
lands. A fifth leg builds the **committed** partitioned example (`examples/partitioned-button`),
because the four legs above build fixtures the script writes itself and nothing in the repository
would otherwise *be* a partitioned host. Three batches of this feature were verified by hand and every defect it had was found by
doing the whole thing once; this is that run, as a gate.
`tools/xirang-partition-rehearsal` 把宿主分区三次——作为某个工作区的成员、在它之上没有任何工作区、
以及写成可发布的形状——各构建一次，并检查拆分绝不能改变的四条事实：宿主自己的源码逐字节相同 · 每个形状里
同一份 `graft_plan.tsv` · 同样的注册面身份 · 以及 `promote` 仍能落地。这个特性被手工验证了三个批次，
而它每一个缺陷都是靠把整件事完整做一遍才发现的；本工具就是那次运行，作为一道门禁。

`tools/xirang-package-audit` checks two different things. Package *contents* —
every `src/**/*.rs` module and the declared README must be in the package — are
verified for all nine crates, because `cargo package --list` needs no
registry. Packaging itself (building the tarball in isolation) waits for
each crate's versioned `xirang-*` dependencies to reach the index, so while the
workspace version is unpublished eight crates are skipped — a state that recurs on
every version bump, not only before the first publish. A module that cargo does not
pick up is a crate that compiles locally and nowhere else, which is why the content
half exists.
`tools/xirang-package-audit` 检查两件不同的事。包**内容**——每个 `src/**/*.rs` 模块与
清单声明的 README 都必须在包里——对九个 crate 全部生效，因为
`cargo package --list` 不需要 registry。打包本身（从 tarball 隔离构建）要等各 crate 的
带版本号 `xirang-*` 依赖进入 index，因此工作区版本尚未发布时会有八个 crate 被跳过——这个
状态每次推进版本线都会重现，而不只是首次发布之前。cargo 没收进去的模块等于一个只在本地编译得过、
别处都编译不过的 crate，这正是内容检查存在的原因。

All nine crates carry `#![warn(missing_docs)]`, so the clippy gate above already
refuses an undocumented public item: document it (English `///` then Chinese
`///`) rather than adding `#[allow(missing_docs)]`. Three workspace members set
`publish = false` — `conventions` and the two example hosts; of those, the two
example hosts are not linted (a host is expected to document its own types),
while `conventions` carries the lint along with the published crates.
九个 crate 都开了 `#![warn(missing_docs)]`，因此上面的 clippy 门禁已经会拒绝没有文档的
公开项：请补上文档（先英文 `///` 再中文 `///`），不要加 `#[allow(missing_docs)]`。有三个
工作区成员设了 `publish = false`——`conventions` 与两个示例宿主；其中两个示例宿主没有开该
lint（宿主自己的类型由宿主负责文档化），而 `conventions` 与已发布的 crate 一样带着它。

## Test governance

A test file lives in one of three places, decided by the module it belongs to. A
**directory module** keeps its tests in `<dir>/tests.rs` or under `<dir>/tests/`; a
**single-file module** keeps them in the sibling `<name>_tests.rs`. A test-only file in
any *fourth* shape fails a gate that names its path (`conventions::test_shape`). Which
files are judged is decided the same way the size gate decides testhood — mounted behind
`#[cfg(test)]`, directly or through an ancestor, or living under a `tests/` directory —
**and** carrying at least one test item, so a test-*support* module with no test of its
own (`tree/graft_ops/fixtures.rs` is the shipped example) is not judged. The crate-root
cargo `tests/` targets and the studio app's `tests.rs` plus its `tests/` directory are
this same rule at the crate root.

Size is a budget rather than a single ceiling. `conventions::size` counts **code lines**
— a blank line and a comment-only line do not count, and the masking that decides what a
line is is the kernel's own lexer, not a second text scraper — and gives each file the
budget of its kind: `CEILING` for source and the wider `TEST_CEILING` for a file mounted
behind `#[cfg(test)]`. Test files used to be exempt outright, and that exemption made the
largest files in the tree the ones nothing bounded; an exception that has to be written
down and then deleted again is `BASELINE`, and a stale entry there fails exactly as a
newly oversized file does.

`tools/xirang-test` is the one entry point that runs *every* face: the default
`--workspace` run, `--all-features`, an isolated `--no-default-features` run, and one face
per `required-features` set, read from the member manifests rather than from a list that
would drift. Each face lists its tests and then really runs them, and the script compares
every file's written `#[test]` items against the union of the names those faces listed.
**A grey test is a defect**: a test that no face reaches has never run once, and the
workspace stays green while it rots. The command exits non-zero when a face fails or when
any test is grey. `cargo test -- --list` is the inventory, so no hand-maintained test list
is kept anywhere.

测试文件只住在三种位置之一，由它所属的模块决定。**目录模块**把测试放在 `<dir>/tests.rs` 或
`<dir>/tests/` 之下；**单文件模块**把测试放在同级的 `<name>_tests.rs`。处于任何**第四种**形状的
仅测试文件会让门禁失败并点名它的路径（`conventions::test_shape`）。判定哪些文件的方式与尺寸门禁
判定"是否测试"的方式相同——挂在 `#[cfg(test)]` 之后（直接挂或经由祖先挂），或位于 `tests/` 目录
之下——**并且**至少带一个测试条目，因此自身不含测试的测试**支撑**模块（出厂树里的
`tree/graft_ops/fixtures.rs` 就是例子）不在判定之内。crate 根的 cargo `tests/` 目标，以及 studio
app 的 `tests.rs` 加它的 `tests/` 目录，都是同一条规则在 crate 根的形态。

尺寸是预算而不是单一上限。`conventions::size` 按**代码行**计——空行与纯注释行不计，而判断"一行是
什么"的掩码用的是内核自己的词法器，不是另写的文本刮取器——并给每个文件与其种类相称的预算：源码用
`CEILING`，挂在 `#[cfg(test)]` 之后的文件用更宽的 `TEST_CEILING`。测试文件过去整体豁免，而这个
豁免让树里最大的文件恰恰没有东西约束；必须写下来、缩回后必须删掉的例外是 `BASELINE`，其中过期的
条目会像新出现的超标文件一样让门禁失败。

`tools/xirang-test` 是跑遍**每一个**面的唯一入口：默认的 `--workspace`、`--all-features`、
隔离的 `--no-default-features`，以及每个 `required-features` 集合一个面——后者从各成员清单现读，
而不是从一份会漂移的清单里读。每个面先列出自己的测试、再真跑，而脚本把每个文件写下的 `#[test]`
条目与这些面列出的名字的并集对比。**灰测试就是缺陷**：任何面都到不了的测试从未跑过一次，而工作区
会在它腐烂时保持绿色。某个面失败、或有任何测试是灰的，命令就以非零退出。清单就是
`cargo test -- --list`，因此任何地方都不维护手写的测试清单。
