# mcp-prober 的思考链（2026-09-29，逐条转录）

这是一次会议记录，不是我复述的。来源是 mcp-prober 自己的会话日志，题头给出可复现的抽取方式。

来源与抽取：
- 会话 id：`5fe8e506-261a-491e-957f-ca9be9e29af7`（mcp-prober，t1 的 owner）
- 日志：`~/.dsh/sessions/--home-nich-Moirai_N3-xirang--/5fe8e506-261a-491e-957f-ca9be9e29af7/session.v4.jsonl.zstd`
- 该日志是**多帧 zstd**（531 帧），`zstdDecompressSync` 只解得出第一帧（284 字节的 session 头）⇒ 按魔数 `28 b5 2f fd` 切帧逐帧解，得 **2,161,964 字节 / 822 个事件**（其中 assistant/message 125、tool/call 150、tool/result 150）。
- 本文只含 assistant 消息里的 `reasoning` 与 `text` 两块，逐 step 照抄；推理合计 123183 字节，正文合计 1280 字节。
- 工具往返（含 MCP 的请求与响应原文）在 `audit-mcp-probe-raw.md`；两份合起来才是完整证据。

## step 1（turn 1，in 27906 / out 119 tokens）

输出：

I'll start by claiming the task and getting oriented.

## step 2（turn 1，in 1038 / out 189 tokens）

推理：

Let me mark in_progress and start exploring the MCP code.

## step 4（turn 1，in 1169 / out 80 tokens）

推理：

Let me look at tools.rs for tool registration, and protocol.rs for the JSON-RPC layer.

## step 5（turn 1，in 8188 / out 184 tokens）

推理：

Now let's look at lib.rs (the JSON-RPC entry), protocol.rs, and check the build status. Let me start the build in the background since it may take a while.

Let me first check if the binary is already built.

## step 6（turn 1，in 1760 / out 234 tokens）

推理：

The binary already exists (built 9月29 16:35). Let me check if it's up to date with sources. Let me just start `cargo build` in background to make sure it's fresh, while I write the Python driver.

Actually first check protocol.rs to understand initialize handshake.

## step 7（turn 1，in 4787 / out 354 tokens）

推理：

Now let me write the Python driver. It should:
- spawn the binary
- send initialize, tools/list, tools/call
- read responses

Let me write it to /tmp/mcpprobe.py.

Note: stdout has a BufWriter with flush after each response, so line-based reading works.

Let me build a driver that takes a list of calls and prints results as JSON.

Also need to determine the package root. The repo root is /home/nich/Moirai_N3/nichlink — is that a cargo package? Let's check Cargo.toml. The mcp resolve_root: XIRANG_PACKAGE_ROOT else cwd. The workspace root Cargo.toml is a virtual manifest probably. Let me check which package has registration faces — "button 这个注册面" mentioned. Let me check target/xirang/out and examples.

Let me look at what's around: examples/ dir with host packages.

## step 8（turn 1，in 259 / out 304 tokens）

推理：

Let me check where target/xirang/out is — no `out` under target/xirang. Maybe the examples have their own target dir? Let's check examples/control-button/target/xirang/out.

Also let's look at what the workspace target/xirang contains — "cache" only.

Let's check both example dirs.

## step 9（turn 1，in 1084 / out 1000 tokens）

推理：

Interesting: target/xirang/out doesn't exist. Let me check what's in the cache dir and whether a build has been run.

Actually `target/xirang/cache` — let me look. Also `examples/control-button/target/xirang/cache`.

Let me check if there's `out` somewhere else. Maybe the build publishes to `target/xirang/out` but hasn't been run in this checkout for the workspace root. Let's check the mcp registry.rs / build_evidence.rs for the exact path.

Let's now write the Python driver and start probing tools/list. Meanwhile check build job.

Let me write the driver.

## step 11（turn 1，in 260 / out 187 tokens）

推理：

Let me look at build_evidence.rs and resolve.rs in mcp to find the output path.

## step 12（turn 1，in 2428 / out 114 tokens）

推理：

Now let me run the tools/list and count. Note the binary is freshly built.

Let me start probing.

## step 13（turn 1，in 212 / out 329 tokens）

推理：

17 tools, exactly matching the task's list. Good.

Now for the "registered not implemented / implemented not registered" check: the DISPATCH table has 17 entries matching. But there are modules like `tree_delta.rs`, `converge_trace.rs`, `preview.rs`, `apply_target.rs`, `resolve.rs`, `overlay.rs` which are helpers. Let me check if there's any handler function defined in a module but not in the catalog. Search for `pub(crate) fn` in mcp modules.

Also check the tools_tests to see what's pinned.

Let me now run all 17 tools against the control-button example. First, let's initialize the environment properly: XIRANG_PACKAGE_ROOT=/home/nich/.../examples/control-button. But note `resolve_root` — let me check it accepts a root argument.

Let me look at resolve.rs.

## step 14（turn 1，in 2913 / out 354 tokens）

推理：

Now let me write a batch probe script that runs all 17 tools with real arguments and captures outputs. Let me build the call list.

First I need to know the real objects:
- registry faces: control-button example, logical paths like `root/control/object/button`
- graft plan: `.xirang/external-grafts/...` — check if it exists in the example
- MIR text: need a -Zunpretty=mir dump; maybe there are test fixtures. Check for .mir files in repo.
- trace: `.xirang/traces/xirang.trace` — check existence.

Let me gather.

## step 15（turn 1，in 619 / out 385 tokens）

推理：

No .mir, no .xirang, no graft.plan in the workspace. So MIR text must be fabricated (I can write a small MIR text dump to /tmp). The task says use real objects from the repo — "MIR 文本" — I could generate a real MIR dump with `cargo rustc -Zunpretty=mir` but that needs nightly. Let me check the toolchain.

Actually let me check rustc version and whether nightly is available.

Also `.xirang/external-grafts/...` — no such dir; the graft plan may be constructed. `xirang.grafts` reads `.xirang/external-grafts/<selector>/graft.plan`. No such files. Let me check tests for the format, or the graft example. There's `examples/control-button/examples/graft_record.rs`.

Let me check what these look like.

## step 16（turn 1，in 1963 / out 306 tokens）

推理：

rustc 1.96.0 stable — no -Zunpretty. So MIR text can't be produced from real rustc. But I can construct a real MIR text dump from test fixtures. Let me check mir_tests.rs / mir.rs for the accepted format, and mir.rs format spec for JSONL.

Actually the task says "MIR 文本" as one of the real objects. I can construct a MIR snippet matching the real accepted format (this is a real input format the tool must parse). Also `jsonl: true` mode writes JSONL, so I can round-trip: write a text MIR to /tmp, run `mir` with jsonl:true output path... wait, does it write to disk? "jsonl: true makes this tool that writer, and what it prints reads back here" — so it prints JSONL on stdout. Then I can save it to /tmp and read it back. That's a good round-trip test.

Let me look at mir.rs to understand MIR text parsing.

## step 17（turn 1，in 1856 / out 245 tokens）

推理：

Let me look at the MIR text format parser in kernel: `xirang_kernel::mir`. Actually easier: check mir_tests.rs for a sample text dump.

## step 19（turn 1，in 1543 / out 265 tokens）

推理：

Good. Now let me set up the probe root. I'll use the control-button example as the package root. But `root` argument can override too. Let me use XIRANG_PACKAGE_ROOT for the process.

But note: for `xirang.mir` path must be within root (relative). Let me check load_mir / path resolution — probably requires path relative to root and safe. Let me check.

## step 20（turn 1，in 1267 / out 1728 tokens）

推理：

Now let me set up probe artifacts: I'll create a small probe directory inside the control-button example? No — that would modify the repo (untracked files), violating "git status --porcelain should only show the report". I could create files in /tmp and use them as the package root via XIRANG_PACKAGE_ROOT. The package root itself can be a throwaway package in /tmp that has a Cargo.toml (so cargo metadata names it) plus `.xirang/external-grafts/...` and a MIR text file.

But requirement 2 says "用仓库里真实的对象" — use real objects from the repo. I can:
- run tools against the real control-button package root (real faces, real graft plan declared in lib.rs).
- For MIR text, I can generate a genuine MIR text? Not possible on stable. But I can create a MIR text dump of the real `button_fast.rs` functions manually in /tmp — hmm.

Alternative: I create a /tmp probe package whose root contains a copy? copy is not "real object from repo".

Better plan: use the real package root `examples/control-button` for most tools. For artifacts that need files under root (mir, trace, grafts records), use the `root` argument to point at... no, root must stay inside XIRANG_PACKAGE_ROOT.

Hmm, `resolve_root(base, requested)`: base = canonicalized XIRANG_PACKAGE_ROOT, requested joined to base. So root must stay inside.

So if I set XIRANG_PACKAGE_ROOT to the repo root `/home/nich/Moirai_N3/nichlink`, then roots inside (like `examples/control-button`) can be requested. And then `load_mir(root=examples/control-button, path=...)` requires path inside the example. Hmm but for `mir`, `root` argument goes through resolve_root, so root can be e.g. `docs` — wait root must be a directory. `tools/call` resolves root from `arguments.root` relative to the package root (repo root). So I can set root to any dir inside the repo. But `mir`'s snapshot_for(root) calls registry::namespace(root) which runs cargo metadata to get package name.

Hmm. Actually a simpler approach for MIR: I need a MIR text file inside the repo... but I can't create repo files except the report.

Wait — can I? "不改仓库代码、不提交；收工时 git status --porcelain 应只看到你那个报告文件." So any file I create in the repo must be cleaned up. I could create a temp file inside the repo (untracked), probe, then delete. But risk. Better: is /tmp writable as package root? Yes: XIRANG_PACKAGE_ROOT=/tmp/mcp-probe-pkg. That package can contain a `Cargo.toml` (needed for namespace) and MIR text, graft plans. That's a synthetic package though, not "real objects".

Compromise: use the real control-button package as the primary root (real faces/real graft plan). For the MIR text and graft plan artifacts, the tool's inputs are files; there are no real ones in the repo (no `.xirang/`, no `.mir`). I'll note that and construct the inputs from the repo's *real* declarations: the graft plan content I write comes from copying the real typed plan in `examples/control-button/src/lib.rs`; the MIR text comes from the real byte content of a real function in `button_fast.rs` (I'll reflect real functions). And the JSONL round trip will use the real package namespace.

Actually, better idea for MIR: check if `graft.plan` documents exist as fixture in tests — e.g. `toolchain/src/mcp/grafts_tests.rs` writes plan documents. Let me look at the format so I can produce one identical to what the repo's own writer writes.

Also, does .xirang/external-grafts exist anywhere in repo? No. So grafts tool will report "no external graft plans" against the real example — that's a real result and worth recording as such. Then I can create a real plan in a temp package to test the positive path.

Hmm, let me reconsider: a temp package in /tmp is legitimate because the tool reads files; the report can say the input was constructed at /tmp (not committed). The task explicitly says write the driver in /tmp. For probe *inputs* it doesn't forbid /tmp. But requirement says "用仓库里真实的对象（button 这个注册面、graft 计划、MIR 文本、某条 diff 之类）". The graft 计划 refers to the declared plan in the host entry (lib.rs static_graft_plan!) — that IS real. And `xirang.grafts` joins plans on disk with declarations from the host entry; with no on-disk plans the tool answers "no external graft plans under .xirang/external-grafts/". That's a real (negative) result.

I think best approach:
- Primary: XIRANG_PACKAGE_ROOT = examples/control-button (real). Run all 17 tools with real args.
- Secondary: create a temp package in /tmp with `.xirang/external-grafts/<selector>/graft.plan` copied from repo's writer format, plus a MIR text file, to exercise positive paths. Document clearly that these inputs are synthetic because the repo ships no such artifacts.

Actually wait — better: I could exercise the positive graft path by creating the `.xirang/external-grafts/` dir inside the real example temporarily and removing it after. That's what the example's own `graft_record.rs` does in temp_dir though. Hmm, `grafts.rs` reads root/.xirang/external-grafts. If I create it under examples/control-button, it's untracked file → I delete it after. That gives a real host entry's declaration joined with a real plan. That's much better evidence. Risk: leaving files. I'll be careful, use a trap/cleanup.

Actually, is that acceptable given "不改仓库代码"? It says don't modify repo code and don't commit; final git status must be clean except the report. Creating and deleting temp probe inputs is transient. Hmm, but a `.xirang/` dir might be gitignored, so git status wouldn't show it anyway — which is dangerous for verification. I'll use the /tmp package approach for artifacts to be safe, and note it. Actually let me check .gitignore.

Let me be pragmatic: use /tmp package for artifact-based tools (mir, grafts, trace, unified), and real example for registry/search/inspect/callgraph/read/status/explain/diff/impact/usages/converge/verify/apply. I'll clearly label synthetic inputs.

Hmm, but actually for `xirang.diff` — needs build evidence (`target/xirang/out`) which doesn't exist. So diff will report "no build evidence". That's a real finding: the tools that need build evidence are unusable in this checkout without running `xirang check`. Let me verify by running.

Let me now run the full sweep. First, set up the probe package in /tmp.

Let me check the grafts.rs and trace.rs for input formats.

## step 21（turn 1，in 2558 / out 1949 tokens）

推理：

Good. `.xirang/` is NOT gitignored, so I'd see it in git status. I'll avoid creating it in the repo.

Plan: create a synthetic package in /tmp for artifact tools. But the `namespace` requires `cargo metadata` to name the package. Let me check registry::namespace.

Actually — for MIR/grafts, root resolution requires root to be inside XIRANG_PACKAGE_ROOT. If XIRANG_PACKAGE_ROOT = /tmp/pkg, root defaults to /tmp/pkg. That works.

But wait, for a /tmp package, `cargo metadata` runs in that dir; offline shouldn't matter (no deps). Let me test.

Alternatively: I can set XIRANG_PACKAGE_ROOT=/tmp/probe-root and *copy* the real control-button example there. Hmm, but that changes file!() / NodeId. Simplest: one /tmp package with a couple of faces matching repo style.

Actually, hold on. Better idea for MIR: use the real repo package as root, and place the MIR text file... requires inside root. Nope.

Hmm — what about running the MCP process with cwd = repo root and XIRANG_PACKAGE_ROOT unset? Then base = repo root (cwd has Cargo.toml → resolve_package_root returns cwd presumably). Then root argument can be any dir inside the repo including `kernel/src`? No, mir requires path inside root; root=kernel/src is a dir → mir path can be `registry_core/mir/text.rs`... but that's not MIR text. A MIR *text dump* needs a `.mir` file. Actually extension only distinguishes jsonl; so any text file works, e.g. I could point at an actual `.rs` file — but that wouldn't be MIR.

OK: decision — create a probe package at /tmp/mcp-probe-pkg containing:
- Cargo.toml (name mcp-probe-pkg)
- src/button/button.rs (a real registration face copied in style from the repo's example, using `crate::root_object!`)
- .xirang/external-grafts/button_fast/graft.plan — generated by the repo's own writer? I don't have a CLI for that easily. I can hand-write the document format. Let me check GraftPlanDocument::render_graft_plan_document format.

- button.mir containing a real MIR text dump — I'll take genuine MIR text for a function? I can hand-write a plausible one AND additionally verify the tool's behavior on a real file: actually rustc 1.96 stable... Hmm, can I get MIR on stable? `-Zunpretty` is nightly-only. But `cargo rustc -- --emit=mir` is *stable*! `--emit=mir` produces `.mir` files (binary-ish, not -Zunpretty text). Not the same. And `RUSTC_BOOTSTRAP=1` lets stable rustc accept -Z flags! That's a real known trick. Let me try `RUSTC_BOOTSTRAP=1 cargo rustc ... -Zunpretty=mir`. That would produce *genuinely real* MIR text from the real example crate. That's excellent evidence.

Let me try that on the control-button example.

Then I'd need the .mir file inside the root to read it — I can write it to /tmp and set XIRANG_PACKAGE_ROOT to a root that contains it... no. Alternatively copy the .mir into the repo temporarily, probe, delete. Risk manageable: `*.mir` untracked → I delete it right after. Actually the report requires git status clean at the end; a temp file inside repo during probing is fine if deleted. Let me consider: does `load_mir` allow absolute paths? `root.join(relative)` with absolute relative → join with absolute path yields the absolute path; then `candidate.is_absolute()` → lexically_inside = false → refused. So no absolute.

OK: temporary copy into repo, then delete. I'll do it carefully with explicit rm and verify git status.

Hmm, actually simpler and cleaner: set XIRANG_PACKAGE_ROOT to a /tmp directory that I populate by *copying* the generated MIR + a Cargo.toml. The MIR text itself is genuinely produced by rustc from the real example crate. That satisfies "real object" (real rustc MIR of real code) while keeping the repo clean. And the snapshot namespace would be that of the /tmp package, but the delta comparison would use two MIR files with same namespace.

Hmm but then `mir` with jsonl:true stamps the *probe package's* namespace, not the example's. Fine.

Actually wait. Even better: the snapshot namespace is only needed for comparison. Let me keep it simple:
- Generate real MIR text from `examples/control-button` via RUSTC_BOOTSTRAP=1.
- Create /tmp/mcp-probe-pkg with Cargo.toml named e.g. `control-button`?? No — cargo metadata in /tmp dir named "control-button" gives namespace "control-button" which differs from the example's namespace. Doesn't matter for mir.

Let me now check: does RUSTC_BOOTSTRAP work?

Let me start with the primary sweep against the real example first (quick), then artifacts.

Let me write a better driver that runs a list of calls and dumps labeled results.

## step 23（turn 1，in 523 / out 458 tokens）

推理：

Now let's run the primary sweep. Calls list:

1. search query "button"
2. search query "paint" limit 5
3. inspect path "src/control/object/button/button.rs"
4. callgraph function "paint"
5. callgraph function "base_registry"
6. read path src/lib.rs line 55 context 10
7. status
8. apply action add fields... (preview) — careful not to write. Use apply:false default. But preview runs on a throwaway copy. Safe.
9. registry
10. explain (no node)
11. explain node root/control/object/button
12. explain overlay true
13. diff
14. diff records true
15. trace
16. mir path missing
17. unified path missing
18. grafts
19. impact node root/control/object/button
20. usages node root/control/object/button
21. converge node root/control/object/button
22. converge trace true
23. verify
24. unknown tool
25. missing required arg cases

Let me first run registry + a few basics to learn the real logical paths and whether things work.

## step 24（turn 1，in 408 / out 709 tokens）

推理：

Working. Note `search` with limit 10 printed only one file line? Maybe truncated in my display? No — chars=223, so the output is short. There's a function `paint` in button.rs; search for "button" only matches file paths.

Now, let me build the full sweep spec. First get the logical paths: root/control, root/control/button, root/control/slider.

Let me write the full spec.

## step 25（turn 1，in 2521 / out 580 tokens）

推理：

Very useful. Notes:
- Logical paths are `root/control/button`, NOT `root/control/object/button`. The task prompt said "button 这个注册面" — fine. Interesting: the tool's own description example in `converge` says "root/...". The `impact`/`usages`/`converge` errors match.
- `xirang.verify` actually ran the build and produced `target/xirang/out`? It said "build current". So verify publishes build evidence as a side effect. But this now means the example's target dir got files. That's under target/ (gitignored). Fine.

Wait — verify ran and now `explain` should say current. Let me re-run explain/diff now that build evidence exists. Also note that before verify, explain said "build stale" and diff said "no build evidence" — that's because a prior `verify`? Actually explain's freshness said stale, meaning out dir existed but outdated. Hmm — "build stale (run xirang check)". Let me check whether target/xirang/out now exists in the example.

Let me re-run explain and diff after verify to see the fresh state.

Also I need to test with the correct node path `root/control/button`.

Let me run batch 3.

## step 27（turn 1，in 1513 / out 558 tokens）

推理：

Excellent. All working with real paths.

Important finding: `xirang.usages` says "fields unreadable (this module was not generated by XiRang)" and "unreadable faces 3 (hand-written modules are not read back)". This is the interesting negative: the tool's advertised promise is "every field the write path accepts read back from the generated module" — but the repo's only real host is hand-written, so the flagship `usages`/`converge` field reporting is entirely unavailable on the repo's own example. That's a capability-face gap worth naming.

Wait — actually, is the example hand-written? It uses `crate::control_object!` macro (declarative). The generated module path is for faces produced by the authoring executor (`fields.module`). Let's check what "generated module" means: `getting_from_other_registry`... Let me check usages.rs to know the exact criterion.

Now let me also test `xirang.apply` (preview), and test the "generated module" path by using preview of add.

Also need:
- `xirang.mir` with real MIR text (RUSTC_BOOTSTRAP).
- `xirang.unified` with real MIR + absent trace.
- `xirang.grafts` with a real plan on disk.
- `xirang.trace` with a real trace artifact (can I record one? The repo's scaffolded host records traces; the control-button example doesn't. Maybe `xirang new` scaffold... too heavy). Could I craft a trace artifact? It needs matching identity namespace/registry root and frame nodes. That's doable: the format? Let me check trace.rs / read_trace_artifact.

Let me first try RUSTC_BOOTSTRAP for real MIR.

## step 28（turn 1，in 431 / out 829 tokens）

推理：

Real MIR text produced (2174 lines). Now, to feed it to `xirang.mir`, it must be inside the package root. I'll set up a /tmp probe package whose root contains the .mir file. But the tool needs the package to be named by cargo metadata (for snapshot). Let me create /tmp/mcp-probe-pkg with Cargo.toml.

But wait — better: I could use the `root` argument of the MCP call to point at a subdirectory of the *actual* root... The MIR file isn't inside the repo. 

Alternative: set XIRANG_PACKAGE_ROOT=/tmp/mcp-probe and copy the real example's src there? No, keep it minimal — the MIR text is the real rustc output of the real example; that's the "real object". The containing package only supplies a namespace. I'll document that.

Hmm, actually there's a neater option: put the package root at /tmp/mcp-probe-pkg and ALSO copy the control-button example there (cp -r), so graft plans etc. are the real ones. But copying changes NodeId (file!() paths... actually file!() is relative to workspace root, so copying might preserve relative paths). Not worth it.

Decision: /tmp/mcp-probe-pkg with:
- Cargo.toml (name = "mcp-probe-pkg")
- src/lib.rs
- button.mir (real rustc MIR text of the example)
- .xirang/external-grafts/button_fast/graft.plan (a real plan document)
- .xirang/traces/xirang.trace (maybe crafted)

Let me check the graft plan document format and trace format.

Actually, for grafts: the plan must reference an identity in the tree and a slot path. With a synthetic package, I need faces. Let me instead build the probe package with faces copied from the example... hmm, but then face_views need the xirang macros → requires the toolchain dependency. Too heavy? Actually it's a workspace with path deps; a /tmp package with path dep to the repo would work offline but changes the workspace... `cargo metadata` would need to resolve path deps: `xirang-toolchain = { path = "/home/nich/Moirai_N3/nichlink/toolchain" }` — offline should work since deps are already vendored/cached.

That's getting heavy but gives real positive-path evidence for grafts on a package with faces. Alternatively — simplest — I can temporarily create `.xirang/external-grafts/...` inside the real control-button example, run grafts, then delete. The `.xirang` dir is not gitignored so I'd see it, and I delete it. Risk is low and cleanup verifiable. And it uses the REAL host entry declaration (the strongest evidence). I'll do that: create, probe, delete, verify git status clean of it.

Actually careful: `verify` published build evidence into `target/xirang/out` — gitignored. Fine.

Plan for grafts: create `examples/control-button/.xirang/external-grafts/button_fast/graft.plan` with a document targeting `root/control/button`, then call grafts, then rm -rf the `.xirang` dir.

Let me check the plan document format from GraftPlanDocument::render_graft_plan_document.

## step 29（turn 1，in 887 / out 239 tokens）

推理：

Format is simple. Now let me set up the grafts probe inside the real example, run grafts, then clean up.

I'll do it in a single bash command that creates, runs, and removes.

Also let's test MIR. I'll set up the /tmp probe package.

Let me check trace artifact format first — for `unified` I need a trace. Let me check trace.rs and read_trace_artifact to see the artifact format (JSON?).

## step 31（turn 1，in 1989 / out 361 tokens）

推理：

Trace artifact is a rendered text format. Let me find `render_trace_artifact` and `from_trace` to know if I can craft one. Better: is there a way to actually record one? The scaffolded project `xirang new` produces a host with trace_call!. I could scaffold a project in /tmp with the CLI... `cargo run -p xirang-toolchain --bin xirang -- new /tmp/x`. That would be genuinely real (produced by the toolchain), then run it with XIRANG_TRACE, producing a real trace artifact. Then point MCP at that root. That's the strongest real-object evidence, and it also gives a "generated module" face for `usages` fields!

That's very valuable: it tests `usages`' generated-field path and `converge`'s requires-verdict, plus trace/unified.

Let me check the CLI: `xirang new`. Let me look at cli commands.

Time budget: this is worth it — the "usages fields unreadable" finding is a big deal for the capability-face conclusion, and I want to know whether generated faces work.

Let me check the xirang CLI binary exists in target/debug.

## step 32（turn 1，in 586 / out 242 tokens）

推理：

Let me scaffold a project in /tmp. `xirang new <name> --path <workspace>`? --path means source from a local checkout. Let's try `xirang new probehost --lib --path /home/nich/Moirai_N3/nichlink` in /tmp.

Actually the scaffold probably uses templates from `toolchain/build_time`. Let's try.

## step 33（turn 1，in 137 / out 236 tokens）

推理：

The scaffold requires a pre-batch2 layout, so `new` is stale relative to the current checkout layout ("core/, build_method/, run_method/" no longer exist). That's itself a finding (CLI, not MCP — out of scope, but relevant as "the only documented path to a generated face is broken"). Careful: my task is MCP capability face; mentioning the CLI `new` failure is context for why `usages` positive path can't be reached. But I should not over-claim; it's an adjacent observation. Let me verify the claim by checking the code quickly — maybe there's a flag.

Actually let me check the scaffold template location requirement.

## step 34（turn 1，in 608 / out 582 tokens）

推理：

Confirmed: `is_checkout` looks for pre-batch-2 directory names. Let me check `is_checkout`.

Anyway. I'll record it as an adjacent finding (out of the MCP scope but explains the reachability of the `usages` field path). Actually, hold on — I should focus. Let me check if there's a fixture with a generated module in the repo: `toolchain/tests/fixtures/node-editor/` (prototype-fixtures). Let me look.

Also, `apply.rs` writes generated modules. So I can use `xirang.apply` with `action: add, apply: true`... no, that writes to the repo. But `apply` preview runs on a throwaway copy and reports the resulting tree + diff. Does the preview create a generated module in the copy? Yes! And then... the copy is deleted, so I can't run `usages` on it.

Hmm. Alternative: `apply` with `root` pointing at a /tmp package inside XIRANG_PACKAGE_ROOT... root must be inside package root. So set XIRANG_PACKAGE_ROOT=/tmp/mcp-probe-pkg, put a copy of the control-button example there as `probe`, then root="probe". But copying the example needs its Cargo.toml path deps fixed (they point to ../../../toolchain presumably). Let me check.

Actually the example Cargo.toml uses `path = "../../kernel"` etc. If I copy to /tmp, I can rewrite those to absolute paths. That is a "copy of real objects" — acceptable, and I'd document it.

Then `apply add` with apply:true there would write a generated module, and `usages` could read it back. That's a real positive path test AND it tests the write path in a sandbox.

Hmm, but the MCP `apply` resolves the namespace via cargo metadata — needs the copied package to build metadata. Fine.

Let me try the copy approach. Actually simpler: `cp -r` the example dir and fix Cargo.toml paths with absolute.

Let me look at the example Cargo.toml.

## step 35（turn 1，in 488 / out 159 tokens）

推理：

The node-editor fixture is also hand-written (`control_object!` macros likely). Let me check whether any repo file contains a *generated* module style (fields like `pub const MODULE: &str` ...). Let me check what `usages` reads to call a module "generated".

Let me read usages.rs relevant part.

## step 36（turn 1，in 1578 / out 825 tokens）

推理：

`authored_face` reads the generated module. Generated by the authoring executor. So `apply` with apply:true is the producer. Fine.

Let me decide how much depth to invest. Time budget: I should aim for a solid report. Key remaining probing:
1. `apply` preview (safe, real).
2. `apply` actually writing → do it in a copied package under /tmp (so repo untouched), then `usages` on the generated face — this is the strongest positive path test. Worth it.
3. MIR real text → /tmp root.
4. grafts with a real plan → /tmp root (if I copy the example there, the host entry declaration is the real one).
5. trace artifact — could produce by running a host that records traces. The scaffold is broken, so I'd need to write a small host myself that calls trace_call!. There may be a test that produces a trace artifact. Let me check if any test writes `.xirang/traces`. Simpler: craft the trace artifact via the documented render format — let me look at TraceArtifact::render and parse; if it's simple text, I can hand-write a valid one and note it's synthetic.

Given the "6-question debug comparison" is the most important part, I shouldn't burn too much on trace. Let me check trace artifact format quickly and decide.

Let me set up the /tmp copy package now. Steps:
- cp -r examples/control-button /tmp/probe/control-button (excluding target)
- fix Cargo.toml path deps to absolute
- cp -r examples/control-button-graft too
- set XIRANG_PACKAGE_ROOT=/tmp/probe, root default = /tmp/probe/control-button? No: root must be inside package root; if XIRANG_PACKAGE_ROOT=/tmp/probe then root="control-button".

Wait, but `resolve_package_root` may pick cwd. Setting XIRANG_PACKAGE_ROOT=/tmp/probe works (base). And each call passes root="control-button". But then `namespace` runs cargo metadata in /tmp/probe/control-button → it's a member of no workspace; path deps absolute → fine.

Let me do it.

## step 37（turn 1，in 446 / out 433 tokens）

推理：

Good. Now add graft plan + MIR file into this sandbox copy.

- `/tmp/probe/control-button/.xirang/external-grafts/button_fast/graft.plan` with target = the real NodeId `ff1c57d9b110ef57eac134a3cb4e46b6` and target_path=root/control/button, graft=button_fast, full=false. And an orphan one to check unkept.
- `/tmp/probe/control-button/button.mir` = real MIR text.

Then run grafts/mir/unified/apply/usages/converge on root="control-button".

Note: MIR file inside the package root means `status` counts .rs files only, fine.

Let's set up and run.

## step 39（turn 1，in 2215 / out 567 tokens）

推理：

Excellent — mir, unified, jsonl all work on real rustc MIR.

But there's a REAL BUG found: `xirang.grafts` reports `orphan_fast` as "declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`" — but orphan_fast targets `root/elsewhere`, which no declaration names. The expected verdict is "NOT declared by the host entry" / unkept. So the grafts tool — with a *typed* declaration plan (`cut(NODE_ID) graft(NODE_ID)`) — marks every plan as declared by the same first cut. That is exactly the "会误导" case: a plan the release prunes is reported as kept.

Wait, let me check: with typed declarations, `cut.cut_label()` returns the Rust path `crate::control::object::button::NODE_ID`. How does graft_plan_rows decide `declared`? Let me look at build_time::graft_plan_rows. Possibly for typed cuts it compares the numeric identity of the cut target's *resolved* id. Hmm, but the tool reported declared for root/elsewhere which doesn't match.

Hold on — maybe `declared` matching is by *label string* and the label is the Rust path, which doesn't match any plan's target_path. Let me read graft_plan_rows.

Also maybe the row is matched differently. Let me check. This could be a genuine finding (or maybe my synthetic setup is at fault: perhaps the real rule compares target path against cut label; with typed cut labels being Rust paths, path comparison never matches → declared would be false always; but we got true).

Hmm wait — but interesting: both rows got the SAME declaration (cut label button). So matching must be positional/first-cut rather than by name. Let's read the code.

Also let me run `xirang grafts` CLI on the same root to compare — the module doc claims "the same answer as the CLI, from the same rule".

Let me look at graft_plan_rows.

## step 40（turn 1，in 2307 / out 194 tokens）

推理：

`cut.names_face(&document.target_path, module)`. For orphan_fast: target_path = "root/elsewhere", module = None (faces don't contain identity ff1c... wait, target identity ff1c... IS button's id, so module = "control::object::button"). So names_face("root/elsewhere", Some("control::object::button")).

Let me read `names_face`.

## step 41（turn 1，in 1979 / out 1032 tokens）

推理：

So matching for typed expressions: wanted = `format!("{module}::NODE_ID")` where module comes from the *plan's target identity* mapped to a face module. For orphan_fast, document.target = ff1c... (button's id) → module = "control::object::button" → wanted = "control::object::button::NODE_ID". The declared expression cut = "crate::control::object::button::NODE_ID" → normalized strips "crate::" → "control::object::button::NODE_ID" == wanted → **true**.

So the matching is by the plan's *target identity's module*, NOT by `target_path`. My orphan plan had target=button id but target_path=root/elsewhere. Since the tool matched on module derived from the identity, it says declared. Is that a bug or intended? The doc says "A typed cut names its target with a Rust path, so it is matched through the same module mapping the build uses". The plan's target is the identity; target_path is a separate field. Hmm — the match uses module from identity, which is arguably right (the identity is the authoritative part) — but it ignores target_path entirely.

Wait, but then `stale`/`undeclared` distinction: `diff records:true` said "ok 2" for both, including orphan_fast with target_path=root/elsewhere. The diff doc says `ok` means the record resolves. Hmm.

Is this a *real* inconsistency? A realistic orphan plan would have a target identity that doesn't exist in the tree (module=None) — then only string cuts match, so declared=false. My synthetic orphan used button's identity with a wrong path, which is an odd combination. Let me redo with a more realistic orphan: target = some random/unknown NodeId (not in the tree), target_path = root/control/button. Then module=None → typed cut can't match → declared=false → "NOT declared". Let me test both realistic cases to be fair:

Case A: plan targets a face identity that is in the tree but which the entry does NOT declare (e.g. slider's id), target_path=root/control/slider. Entry declares both button and slider... so not that.

Hmm, entry declares button and slider both. So to get an undeclared plan I need a target whose module isn't named by any cut. Options: identity of the `root/control` face (Control, module control::control) — not declared → module = "control::control" → wanted "control::control::NODE_ID" → no cut matches → declared=false → "NOT declared by the host entry" + unkept count. That's a realistic orphan (plan for a slot nobody handed over). Let me test that.

My earlier orphan test was a malformed combination. I should test the realistic one and not report a false bug. But it's still worth noting: `target_path` in the plan is ignored when the identity resolves — the tool prints target_path but judges by module. Hmm, actually is that a *misleading* behavior? If a plan says target_path=root/elsewhere but target=button's id, the tool says "declared" and prints target=root/elsewhere. A maintainer reading it sees "target=root/elsewhere ... declared" — that IS misleading, but such a plan is self-inconsistent (the writer always writes matching pairs). Low severity. I'll note it as a minor finding with the evidence, clearly labeled as a hand-constructed inconsistent plan.

Let me now redo the graft test with a realistic orphan.

Also test the CLI `xirang grafts` on the same root for the "same answer" claim.

Let me also test apply (preview + real write in sandbox) and then usages on generated face.

Let me run.

## step 42（turn 1，in 184 / out 408 tokens）

推理：

Good — CLI agrees with MCP here (MCP would too). Let me confirm MCP output for this realistic case.

Also note CLI doesn't report "unkept plans 1" — let me see full CLI output (head cut it?). Let me get full.

Now run MCP grafts again + diff records + apply.

## step 43（turn 1，in 850 / out 305 tokens）

推理：

Findings:
- grafts/diff records work and agree with CLI.
- `apply` rejected `exports: ["control.render"]` with "`exports` must be a string". The tool's catalog says fields are the face's fields; the real declaration uses `exports: ["control.render"]` (an array). So the JSON input expects `exports` as a string? Let me check apply.rs / the authoring field parsing. Maybe it expects a comma-separated string. That's an input-format quirk worth documenting: the schema `fields: {"type":"object"}` gives no guidance, and the natural JSON array (mirroring the Rust `exports: [...]`) is refused.

Let me check what apply accepts for exports, then retry.

Let me grep apply.rs for "must be a string".

## step 45（turn 1，in 977 / out 403 tokens）

推理：

So every field is a string except `needs_registry`. The catalog's schema just says `fields: object`, no per-key types. So the natural array form for `exports` is refused — and the error message "`exports` must be a string" doesn't tell you what to pass instead (a comma-separated list probably: the generated module renders `exports: ["control.render"]` from a string?). Let me retry with `exports: "control.render"` and see the preview. That will show the generated diff — good evidence.

Let me run apply preview again with string fields.

## step 46（turn 1，in 336 / out 426 tokens）

推理：

More findings:
- `fields.module` must be a bare snake_case module name, not a path. The tool description says "`add` (create `fields.module` under `parent`)" and "`rename` (change `fields.module`)" — it doesn't say "a bare snake_case name". Also catalog says "`node` names the face and `parent` the parent, by logical path". Let's retry with module "widget".
- `edit` on a hand-written face fails "this module was not generated by XiRang" — consistent with usages. So the write path only works on XiRang-generated modules — meaning in this repo, `apply` cannot edit any of the real faces. That's a major capability-face finding: the write tool is unavailable on hand-written faces (which are all real faces in the repo).
- delete requires confirm.

Let me retry add with module "widget" and see the preview (does it need to write to a file? "create fields.module under parent").

Then apply:true in the sandbox to generate the module, then usages to read it back.

## step 48（turn 1，in 425 / out 373 tokens）

推理：

Two more findings:
1. `apply: true` placed *inside* `fields` is rejected as "not an editable registration-face field" — expected, since `apply` is a sibling of `fields`. But the error message is decent. Let me test the correct form: `{"action":"add", ..., "apply": true}` with `apply` at top level, plus `handle_traits` to satisfy the parent rule. Actually the parenting rule requires the handle implement `ControlHandle`. The generated module presumably writes `impl ControlHandle for Widget` if `handle_traits` is given. Let me pass `handle_traits: "crate::control::ControlHandle"` and see if the preview succeeds.

This is a real capability test: can the write path actually produce a face in this repo's example? Let's try.

## step 49（turn 1，in 302 / out 390 tokens）

推理：

Still rejected — the `handle_traits` field means something else (traits the face's *part* handles satisfy?). Let me check what the registry rule requires and how the authoring executor lets you satisfy it. Maybe there's `admission`/`registration_rule` fields. Let me look at the rule file and how add_module_from_face handles handle_traits.

Actually, this is a real finding: the write path can compose a face that the parent's registration rule rejects, and the tool reports the rejection (that's *correct* behavior — it ran the kernel checks, exactly as advertised). It's evidence that the admission checks work. To get a successful add I'd need to know the required field. Let me look at the generated module for a face created by the authoring executor — check `NewModuleFace` → generated code, particularly how `handle_traits`/`handle_contracts` render, and what the rule checks.

Let me check the registry rule in the example.

## step 51（turn 1，in 654 / out 466 tokens）

推理：

`handle_traits` is the *label* ("ControlHandle"); when `handle_contracts` is also given, labels derive from the contract paths via `trait_names_from_paths`. I passed `handle_traits: "crate::control::ControlHandle"` — the label became the whole path, which doesn't match `require_handle_traits(&["ControlHandle"])`. Let me retry with `handle_traits: "ControlHandle"`.

This is a real usability trap worth documenting: the natural value (the Rust path `crate::control::ControlHandle`) is accepted but silently produces a wrong label → the parent rule rejects. The catalog description for `apply` mentions `handle_contracts`/`part_contracts` are add-only but says nothing about `handle_traits` being a bare label.

Let's retry.

## step 52（turn 1，in 890 / out 471 tokens）

推理：

The preview works. Now test `apply: true` in the sandbox to actually write, then `usages` should read back the generated fields — this closes the loop and gives positive evidence for the field read-back and the `requires` verdict in `converge`.

Let me do the write with a `requires` that's unanswered, to test the UNANSWERED verdict.

Plan:
1. apply add (apply:true) widget with exports+handle_traits+requires "control.render=>SomethingElse" (unanswered) → check file written.
2. usages root/control/widget → fields read back.
3. converge root/control/widget → requirement answered/UNANSWERED.

Note: parent rule requires exports control.render — provided. handle_traits ControlHandle — provided. requires is a capability requirement, should be fine.

Let's do it.

## step 53（turn 1，in 832 / out 444 tokens）

推理：

Excellent evidence. The write path's admission/connector checks work; the preview accurately reported rejection (though preview succeeded earlier because requires was absent — the preview ran the real op). Note that the rejection happened before writing (file not created) — usages says no face. Good: atomic.

Also `verify` correctly flagged the unkept graft plan. And note: after a failed verify, "build stale".

Now let me do a successful add without the unsatisfiable `requires`, then read back usages/converge.

Then also test trace artifacts. Let me try to produce a real trace artifact. The `verify` failure is caused by control_fast plan; I'll remove it before further tests to keep the sandbox clean.

Let's proceed.

## step 54（turn 1，in 672 / out 428 tokens）

推理：

The face itself exports control.render, and it requires control.render — self-reference creates no provider. Let me just add without `requires` (should succeed), then use `usages` to confirm the field read-back.

Then separately test `converge`'s UNANSWERED verdict by editing the generated face? Editing a generated face is possible: `apply edit` with fields. handle_contracts not allowed in edit. `requires` is editable. Let's try edit after the add.

## step 55（turn 1，in 1318 / out 396 tokens）

推理：

The write path works end-to-end (in a sandbox copy). Excellent. Big positive evidence. Also note `usages` reads back all fields, but note the "unreadable faces 3" line counts only 3 — the 4 faces minus the generated one. Wait it says "unreadable faces 3 (hand-written modules are not read back)" — hmm, the widget's own read_back succeeded, and 3 hand-written ones are unreadable. But the line at the top says "fields ... read back" for widget. Consistent.

Note in `[1]`: `converge` in [2] says "scope not-selected (mode=auto)" — because verify's published build predates the new face. Fine.

Now: trace artifact. Let me see if I can produce a real one. Look at `write_trace_artifact` usage in a test or example, and whether there is a small host that records. Search toolchain tests for trace artifact writing.

Actually — the trace artifact format: let me check `render_trace_artifact`. If it's a simple text format I can craft a valid one for the sandbox root, and the MCP trace/unified tools would read it. Crafting is legitimate but "synthetic". Alternatively find a test that writes one and copy the format exactly.

Let me look for tests that build a trace and write artifacts.

## step 56（turn 1，in 1196 / out 337 tokens）

推理：

I can craft a valid trace artifact: header version/namespace/root/mode + frames with node ids of the real tree. The frames' `node` must be a NodeId in the tree (identity check). I have real NodeIds from `registry`.

Let me craft:
```
version=1
namespace=xirang-example-control-button
root=b6a6bea94077152dbb7dd780a2708acf
mode=...
frame=1	-	ff1c57d9b110ef57eac134a3cb4e46b6	control::object::button::paint	src/control/object/button/button.rs	10	5
```
Need to know `mode` spelling and `version` value. Let me check parse + version const + mode_spelling + source_fields.

Let me read the file around parse.

## step 57（turn 1，in 670 / out 328 tokens）

推理：

Rather than hand-craft (risk of format errors yielding a misleading conclusion), better: write a tiny Rust host that uses the runtime to record a trace and write the artifact. But that's a lot of work. 

Alternative: check if the repo has a test that writes a trace artifact to a temp dir with real frames — I could adapt it. Let me search tests for `write_trace_artifact`.

Actually, a much better idea: the trace artifact's frames reference NodeIds. `verify`/`registry` gives me real ones. Crafting is fine as long as I clearly label it: "trace artifact hand-written in the documented `version=1` format with the real NodeIds of the control-button tree". The MCP tool's *validation* (refusing a foreign tree) is exactly what I want to test, and I can test both: a matching-namespace artifact (accepted) and a foreign one (refused). That's high-value evidence for the "可信度链路" stability theme.

Let me check the parse function to get the exact field order right. Let me read parse_trace_artifact.

## step 58（turn 1，in 840 / out 566 tokens）

推理：

Rather than crafting by hand (error-prone, and any error would be my bug not the tool's), better idea: write a tiny Rust example in /tmp that depends on the repo's toolchain crate and calls `write_trace_artifact` with a trace built from the real NodeIds. That's ~30 lines and produces a *genuinely written* artifact by the same writer the host uses. But compiling takes time; and I'd need a crate in /tmp with path dep — cargo build offline, fine.

Hmm, cost/benefit: trace is 1 of 17 tools. The task requires "每个工具真跑一次" and "记录返回要点/可用不可用". I already ran `trace` against a real project → "trace absent" + how to produce one. That's a valid run and a valid (negative) result. But to claim 可用 I'd want a positive path.

Let me try the Rust writer approach — it's the most trustworthy. Actually simpler: `trace` requires the artifact at `<root>/.xirang/traces/xirang.trace`. I can write a small Rust binary in /tmp/probe that depends on xirang-toolchain and writes the artifact for a given root+namespace. Let me estimate: cargo build of a tiny crate linking the toolchain (already compiled, but a new target dir → recompile deps ~ 1-2 min). Acceptable.

Actually, wait: /tmp/probe/control-button is already a crate depending on the toolchain with `run` feature. I can add a small `examples/` file there and run it with `cargo run --example`. It has dev-dependencies with run+authoring. Let me add `/tmp/probe/control-button/examples/trace_probe.rs` that constructs a CallTrace via trace_call! macro... The macro-based recording needs the runtime environment. Simpler: use the public API: `CallTrace::with_mode`, `CallSite { node, function, frame_id, source }`, `write_trace_artifact`.

Let me check what's public: `xirang_toolchain::runtime::{CallTrace, write_trace_artifact, trace_artifact_path}` and `FrameRecord`, `CallSite`.

Let me check the re-exports quickly.

## step 59（turn 1，in 584 / out 232 tokens）

推理：

`TraceArtifact` is public with `TraceFrame`. I can construct `TraceArtifact { version, namespace, root, mode, frames: vec![TraceFrame{...}], locals, edges }`? Fields may be private. Let me check TraceFrame fields and TraceArtifact fields visibility.

Simpler approach: `CallTrace` + `write_trace_artifact(trace, path, namespace)` — from_trace builds the artifact. Need `FrameRecord`/`CallSite` public. Let me grep.

Let me just write the example and let the compiler tell me. Actually, to save time, let me check pub fields.

## step 60（turn 1，in 1130 / out 479 tokens）

推理：

TraceFrame.function is `&'static str` — constructing requires interning. `TraceFrame` might have a constructor. This is getting deep. 

Better: use `CallTrace` + `write_trace_artifact`. `CallSite` fields? `trace.frames = Vec<FrameRecord>` where `FrameRecord { call: CallSite, parent }` and `CallSite { node, function: &'static str, frame_id, source }` — also &'static str, but a string literal works.

So in a /tmp example:
```rust
use xirang_toolchain::runtime::{CallTrace, CallSite, FrameRecord, write_trace_artifact, trace_artifact_path};
use xirang_toolchain::runtime::registry_core::NodeId;

let node: NodeId = "ff1c57...".parse().unwrap();
let mut trace = CallTrace::with_mode(...TraceMode::Full);
trace.frames = vec![FrameRecord{ call: CallSite{ node, function: "control::object::button::paint", frame_id: 1, source: None }, parent: None }];
write_trace_artifact(&trace, &trace_artifact_path(root), "xirang-example-control-button")?;
```
Need `TraceMode` in scope. Let me check `with_mode` param and FrameRecord pub path. Let me grep for a test that does this.

Let me search for `write_trace_artifact(` in tests.

## step 61（turn 1，in 1164 / out 164 tokens）

推理：

`FrameRecord` is `pub(super)` — not constructible outside. But there's `trace_tests.rs` in the MCP module (in-crate) that does it. Let me read toolchain/src/mcp/trace_tests.rs lines ~20-70 to see how it builds a trace — that tells me the public path.

## step 62（turn 1，in 1188 / out 945 tokens）

推理：

There's a public API: `CallTrace::full()`, `recorded.with(node, "fn", |r| ...)`, `write_trace_artifact`. So I can write a /tmp example crate... or simpler: since the trace_test package is a *throwaway package with a hand-written root face*, I can replicate that exact fixture in Rust in /tmp.

But even better: I can make the artifact inside the sandbox copy by writing a tiny example in /tmp/probe/control-button/examples/. Let's do that — it's the real host copy, real NodeIds.

Let me write:
```rust
//! probe: write a real trace artifact for the control-button tree.
use std::path::Path;
use xirang_toolchain::runtime::{CallTrace, LocalKind, trace_artifact_path, write_trace_artifact};

fn main() {
    let root = Path::new("/tmp/probe/control-button");
    let namespace = "xirang-example-control-button";
    let button: xirang_toolchain::NodeId = "ff1c57d9b110ef57eac134a3cb4e46b6".parse().unwrap();
    let control: xirang_toolchain::NodeId = "fb97ddd5f2b803d1b7f40a776d8a22d9".parse().unwrap();
    let mut recorded = CallTrace::full();
    recorded.with(control, "base_registry", |r| {
        r.with(button, "button::paint", |r| {
            let input = r.local("label", "&str", "ok", LocalKind::Input);
            let doubled = r.transform(input, "painted", "ControlFrame", 1);
            r.consume(doubled, "render", "frame");
        });
    });
    write_trace_artifact(&recorded, &trace_artifact_path(root), namespace).unwrap();
    println!("wrote {}", trace_artifact_path(root).display());
}
```
Need to check `with`, `local`, `transform`, `consume` signatures and `NodeId` path. From the test: `recorded.with(node, "button", |recorded| { recorded.local("input", "u32", 1, LocalKind::Input); });` and `let input = recorded.local(...)` returns something, `recorded.transform(input, "doubled", "u32", 2)`, `recorded.consume(doubled, "render", "count")`.

`user_node` parse: `xirang_kernel::identity::NodeId`; the example crate has `xirang_toolchain::NodeId` re-export? In lib.rs of the example they use `NodeId` bare after host!(). Let me use `xirang_toolchain::runtime::registry_core::NodeId`. Hmm, from mcp resolve.rs: `use xirang_kernel::NodeId;`. The example crate doesn't depend on kernel directly. Let me check `xirang_toolchain::NodeId` export: in the example's lib.rs they write `pub const FRAMEWORK: FrameworkId` after host!() which re-exports kernel at crate root... For the example crate, they'd need `use xirang_toolchain::...`. Let me just check if `xirang_toolchain::NodeId` exists.

Let me grep the example for NodeId import.

Actually `crate::control::NODE_ID` is used, and in button.rs they don't import NodeId. Simpler: parse with `.parse()` on a type annotation from `FrameView`... I'll try `xirang_toolchain::NodeId` and fall back.

Let me check quickly.

## step 65（turn 1，in 406 / out 674 tokens）

推理：

Good. Now write the probe example. NodeId type: need to construct. Let me find how to parse a NodeId from hex string — `NodeId: FromStr`. Type: `xirang_kernel::identity::NodeId`, but the example crate may not depend on kernel. However `xirang_toolchain::runtime` re-exports? Let's check: `pub use call_report::*; pub use runtime::*;` and runtime module re-exports kernel? The mcp code uses `xirang_kernel::NodeId` directly (toolchain depends on kernel). For my example crate, I can add `xirang-kernel = { path = ... }` as a dependency. Easy.

Let me write it.

## step 66（turn 1，in 350 / out 319 tokens）

推理：

A genuine trace artifact, written by the repo's own writer. Now probe `trace`, `trace values:true`, `unified`, `converge trace:true`.

Also test the "foreign tree" refusal: copy the artifact and change namespace → refused.

## step 67（turn 1，in 1426 / out 411 tokens）

推理：

All trace tools work. Note: `converge trace:true` says "faces that ran (0 of 4 declared, matched by source file)" — my synthetic frames' source file is examples/trace_probe.rs, not the face file, so 0 matched. That's an interesting real limitation: frame→face matching is by *callsite source file* (declared-at is used for display), and a real host's callsite is in the caller's file, so a face's `paint` called from another file won't match. Hmm — actually the doc says "frames are matched to faces by source file, which the reply states, because a face is a declaration and a frame is a function". Matching uses `frame.call.source.file` (the *call site*), not `declared-at`. So a frame only counts as "in a face" if the *callsite* is in the face's declaring file. For `base_registry` called from lib.rs (a face file? no). Hmm, in my artifact both frames' callsites are in examples/trace_probe.rs. If I had used `with_at` with the real source files it'd match. This is a design subtlety; I could note it as a potential misleading point: `declare-at=` is displayed but matching uses call-at. Actually wait — which one does it use? Let me check converge_trace.rs quickly to be accurate.

Let me check.

## step 69（turn 1，in 507 / out 828 tokens）

推理：

Confirmed: matching uses the frame's *callsite* file (`frame.source.file`), not the face's declaring file as recorded in the frame's node/declared-at. The `trace` report displays `declared-at=<face source>` — which is derived from the node identity, not the artifact. So `converge trace:true` matches "faces that ran" by callsite file, while `trace` displays `declared-at` (the face's file). A reader could reasonably expect "faces that ran" to be based on `declared-at`. In my probe the frames' nodes WERE button/control faces (declared-at shows the face files) yet converge said "0 of 4 declared". That's a genuine potential-misleading point IF a real recording has the callsite elsewhere. In a real host, `trace_call!` frames are recorded with the caller's location, and the node is the face's node. So for a face-level frame entered from a non-face file (e.g. main.rs calling into a face), declared-at=face file but call-at=main.rs → converge counts it as "outside any declared face" while `trace` shows it as `<face>::<fn>#id declared-at=<face file>`. That's an inconsistency between two tools' answers about "which face ran". Worth listing as a minor/medium finding: two tools disagree in wording, and the one an agent would ask ("which faces ran in this run") can report 0 while the call tree clearly shows face nodes.

That's a strong finding — let me make it sharper by constructing the artifact so a frame's callsite is in a non-face file but its node is a face (which is what a real host does). My current artifact already is exactly that: node=button face, callsite=examples/trace_probe.rs. `trace` displays `declared-at=control/object/button/button.rs:15:1` and converge says the frame is outside any declared face. Good — that's real, reproducible evidence.

Now: `xirang.search` — test more thoroughly (face matching, unparsable faces, limit, stale announcement). And `inspect` on a file with no functions. And `read` bounds. And error cases for missing required args. And unknown tool. And the notification/envelope handling? That's protocol, not tools — maybe brief.

Let me also check `xirang.search`'s "tree half needs the identity namespace; a root Cargo cannot name still answers the source half".

Let me now do a batch of edge/negative cases + search variants. Also test all 17 tools with missing required args to check each error message (useful for the 对账 table "描述与行为不符").

Let me run a batch of "no args" calls for all 17.

## step 70（turn 1，in 1105 / out 1191 tokens）

推理：

Good. Now some more: search variants, unknown tool, path traversal, and inspect on a file with no functions.

Also I should test the "17 names in the task list vs tools/list" reconciliation — done, exact match in order and count. Also check: any implemented-but-unregistered handler? The DISPATCH has 17. Are there handler fns in modules not in DISPATCH? e.g. `overlay` is routed via explain_tool. Let me grep for pub(crate) fns that look like tools: registry.rs registry, search.rs search, callgraph.rs callgraph, diff.rs diff, impact.rs impact, grafts.rs grafts, usages.rs usages, converge.rs converge, verify.rs verify, trace.rs trace, mir.rs mir/unified, apply.rs apply, build_evidence.rs explain, overlay.rs overlay, preview.rs (helper), tree_delta.rs (helper), converge_trace.rs (helper), apply_target.rs, resolve.rs, source_index.rs.

Let me check each module's public entry to list any extra "tool-shaped" function.

Also check descriptions vs behavior mismatch candidates:
1. `xirang.read` description: "Read a bounded source window around a line." — behavior ok. But max context is 120 and MAX_READ_LINES 240; catalog says context max 120, ok.
2. `xirang.search` description says it matches "logical path, kind, module, or registry_name". Let me test kind match: query "Button" should match face kind.
3. `xirang.status` claims "indexed Rust file/function counts" — ok.
4. `xirang.apply` schema has no `required` beyond action, and description says `fields.module` is "create fields.module under parent" — but module must be a bare snake_case name, and it's placed at `<parent>/<module>/<module>.rs`. Also `exports` must be a comma-separated string, not an array — the description doesn't say. Also `apply` must be top-level, not in fields (description shows `apply` as a sibling — actually the schema puts `apply` top-level, consistent).
5. `xirang.impact` `depth` min 1 max 16 — behavior clamp. Let me test depth=0 → clamp to 1? and depth=99.
6. `xirang.trace` description mentions "`XIRANG_TRACE_FILE`, else `<root>/.xirang/traces/xirang.trace`" — behavior ok.
7. `xirang.mir` `path` required, `against`, `jsonl`. Behavior ok. Error when jsonl+against — good.
8. `xirang.grafts` limit max 400 while schema says 400 — ok.
9. `xirang.usages` — description lists fields; behavior prints them. ok.
10. `xirang.converge` — "With `node`, ... whether each `capability=>ProviderKind` requirement is actually answered by something in the package, named when it is and UNANSWERED when it is not". In my tests with the generated widget, `requires -` so we didn't see the verdict. Let me test a face with requires: the widget's `requires` can be edited. Let's do `apply edit` on widget to set requires "control.theme=>ThemeProvider" and then converge → expect UNANSWERED. That's a good positive test of the headline verdict.

Let me do that.

Also test `xirang.explain` with both `overlay: true` and `node` → description says mutually exclusive; check behavior.

Let me run these.

## step 71（turn 1，in 1596 / out 932 tokens）

推理：

More findings:

1. **`xirang.search` "no matches" for `control.render`** despite the face declaring `exports: ["control.render"]`. The description says search matches faces on "logical path, kind, module, or registry_name" — exports are not searched. That's consistent with the doc but a real gap for debugging ("where is the capability `control.render` declared?"). It's a "not misleading, but missing" item. Worth listing.

2. **`converge` UNANSWERED verdict unreachable** — `apply edit` with an unanswerable `requires` is *rejected by the connector check before writing*, so the face never has an unanswered requirement. Meaning: is `converge`'s UNANSWERED verdict dead code in practice? Not necessarily — a hand-written face could declare an unanswered requirement... but `usages`/`converge` refuse to read hand-written faces ("requires unreadable"). So the headline verdict of `converge` ("the verdict no single tool can give") is unreachable: a face whose requires is readable is a generated face, and a generated face can't be written with an unanswered requirement (the connector rejects it). That's a strong finding!

Wait — careful. Can a generated face have an unanswered requirement? The `requires` are answered by faces within the same package? The connector "data-flow attachment" seems to require a provider *in the loaded registry*. If the requirement is answered by a provider that exists, edit succeeds. So UNANSWERED can only appear when the requirement genuinely can't attach... which is exactly when the write is rejected. Unless the provider is removed later (delete the provider face). Let's test: add widget2 that provides something, then widget requires it, then delete widget2 → widget's requirement becomes unanswered. Deleting a face requires confirm. Let's try! That would make converge's UNANSWERED verdict reachable and prove it works. Good — that's the rigorous way: don't claim dead code without trying.

Let me test:
- add widget2 (kind Widget2, exports control.render, handle_traits ControlHandle, provides "control.theme=Widget2")? The `provides` syntax? From converge: "capability=>ProviderKind". provides might be "control.theme=>Widget2".
- then edit widget requires "control.theme=>Widget2" → should attach.
- then delete widget2 with confirm:true → does delete check that dependents remain? Maybe it rejects. Let's see.

That's a few calls. Let me do it.

3. `impact depth:0` → clamped to 1 (schema min 1). Fine.

4. `explain overlay+node` → clear error. Good.

5. path traversal refused. Good.

Let me run the widget2 experiment.

## step 72（turn 1，in 1408 / out 224 tokens）

推理：

Interesting: even after successfully adding a provider face `theme` with `provides: control.theme=>ThemeProvider`, the `requires: control.theme=>ThemeProvider` edit was still rejected — "input `control.theme` has no provider; expected provider kind `ThemeProvider`".

Hmm. Maybe `provides` syntax is different, or the provider kind must match the *kind* of the providing face (Theme kind, not ThemeProvider). Let me check the generated widget/theme file to see how `provides` was rendered, and how the connector resolves providers.

Let me look at the generated theme.rs.

## step 73（turn 1，in 341 / out 245 tokens）

推理：

The provider face declares `provides: ["control.theme=>ThemeProvider"]` and exists in the tree, yet the connector says no provider. Possibly the "provider kind" must be the *actual kind* of the providing face's handle (`Theme`), not an arbitrary label; or the connector needs the requirement's provider kind to match a registered face kind. Let me check the kernel's connector logic.

This is a notable finding either way: **the write path can produce a face the connector then refuses, and `converge`'s UNANSWERED verdict is unreachable in the tested routes.** But let me verify the provider-kind rule before claiming.

Let me grep for "expected provider kind".

## step 74（turn 1，in 778 / out 366 tokens）

推理：

The rule matches `info.provides` entries — the capability token *without* the `=>ProviderKind`? Look: `value == &requirement.capability` where requirement.capability = "control.theme". The face declared `provides: ["control.theme=>ThemeProvider"]`. So if `info.provides` holds the raw string "control.theme=>ThemeProvider", comparing to "control.theme" fails.

Hmm, but wait: the error says "has no provider" (no provider with that capability found at all), not "found but wrong kind". So the provided token didn't match. Let me check how `provides` is parsed — does the macro split `capability=>ProviderKind`? Let me check what `info.provides` contains: maybe the declared provides should be the *capability alone*, and the provider kind comes from the face's `kind`. Let me check the macro's handling of provides.

Let me grep in kernel for `provides` parsing with `=>`.

## step 76（turn 1，in 622 / out 775 tokens）

推理：

`provider_for_capability(parent, capability)` — the provider must be an *ancestor* of the requiring face, or (the later fallback at line 318-327) found anywhere in the tree with `provides` containing the capability. The fallback uses `root.find_where(|info| info.provides.iter().any(|value| value == &requirement.capability))` → but wait, that fallback is inside `.or_else(...)` attached to `ancestor_capability(...)` and then `.map(...)` builds the "kind mismatch" message. Hmm — actually at that point (line 317+) we're in the branch where `provider` (from provider_for_capability) was None, i.e. "has no provider" path. The `.or_else` finds any node at all whose provides contains capability, and then `.map()` renders "found X but kind is Y instead of Z", else the "no provider" message.

So the fallback at 321 compares `info.provides` entries directly to "control.theme". Our Theme face's provides entry — is it stored as "control.theme" or "control.theme=>ThemeProvider"? If the fallback found it, the message would be "found ... but its kind is Theme instead of ThemeProvider". We got "no provider", meaning the stored provides entry is NOT the bare "control.theme".

Hmm, but does the widget face's own `requires` end up parsed as capability="control.theme", provider="ThemeProvider"? Yes, presumably.

So the stored provides token must be something else. Let me check how the macro declaration `provides: ["control.theme=>ThemeProvider"]` is parsed into info.provides — maybe it splits into capability only. Let me grep in macro or kernel for provides parsing.

Wait — but importantly: the generated face uses `crate::control_object!` and the widget is a child of Control. `provider_for_capability(parent=control, "control.theme")` requires the provider to be a child/descendant of `control`... Theme IS a child of control. Hmm.

Let's check `provider_for_capability` — maybe it looks among the registry's children for a face whose `provides` contains the capability token AND whose kind matches? No, kind isn't a param.

Let me just directly test with the CLI/`usages` what tokens are recorded: `usages` prints "this face provides: -" for hand-written... but for the generated theme face it should print its tokens. Let me run `usages root/control/theme` — but I deleted theme in the sandbox (preview only, not applied; delete was preview so theme still exists).

Let me run usages on theme.

## step 77（turn 1，in 847 / out 468 tokens）

推理：

`usages` reports "this face provides: ThemeProvider, control.theme" — so it splits "capability=>ProviderKind" into tokens. But the connector's fallback compares the raw entry. So the kernel stores `provides` as raw strings including "=>"? Hmm — but `usages` splits. If the kernel's `info.provides` contained the raw "control.theme=>ThemeProvider", then the fallback comparison `value == &requirement.capability` ("control.theme") fails → "no provider" — exactly what we saw.

So: the write path produced a face whose `provides` the connector cannot match, i.e., declaring `provides: "capability=>ProviderKind"` in the generated module yields a provider that never satisfies a `requires` with the same token. That's a genuine cross-surface inconsistency: the authoring/`usages` side parses `=>` into two tokens, while the connector's fallback path compares raw. But hold on — maybe the *primary* path `provider_for_capability` handles the `=>` properly, and the failure is elsewhere. Since the primary path also returned None (we reached the fallback), and the fallback raw-compares...

Let me test the primary path more carefully: is `provider_for_capability` looking at children of `control`? Let me read it — maybe it requires the provider to be a *descendant* of the requiring face's parent, and it compares the capability token which is stored... Let me read it to be accurate before writing the finding.

This is a significant finding worth verifying. Let me read provider_for_capability.

## step 78（turn 1，in 1193 / out 293 tokens）

推理：

`provider_for_capability` compares `info.provides` entries to the capability string directly. So `info.provides` must contain bare capability tokens ("control.theme"), not "control.theme=>ThemeProvider".

So the question: how does the macro/declaration turn `provides: ["control.theme=>ThemeProvider"]` into `info.provides`? Let's check the macro expansion path — search kernel syntax/declaration for provides parsing.

Hmm, but `usages` displays "this face provides: ThemeProvider, control.theme" — it splits the *authored* string on "=>". That's the MCP side reading the generated manifest, not the compiled snapshot.

Let me look at how `provides` from the macro is compiled into `RegistrationSnapshot.provides`. grep in macro crate.

## step 80（turn 1，in 269 / out 360 tokens）

推理：

So `provides` entries are copied verbatim from the declaration: `provides: ["control.theme=>ThemeProvider"]` → info.provides = ["control.theme=>ThemeProvider"]. And `requires: ["control.theme=>ThemeProvider"]` → parsed into capability/provider (there must be a parser for requires). So the convention is asymmetric: `requires` uses `capability=>ProviderKind`, `provides` is a bare capability token — but the *authoring write path* accepted `provides: "control.theme=>ProviderKind"` without complaint and rendered it into the module, producing a provider that can never satisfy any requirement.

Wait, let me double check whether `requires` really parses `=>`. The error message "requires entries must use capability=>provider syntax" came from the write path. And the connector compares `requirement.capability` to provides entries — if requires were stored raw too, the comparison would be "control.theme=>ThemeProvider" vs "control.theme=>ThemeProvider" and it would match. Since it didn't match, requires IS split (capability="control.theme") and provides is raw.

Actually let me confirm requires splitting by checking `RequirementSpec` parse. Let me grep.

## step 81（turn 1，in 476 / out 738 tokens）

推理：

Confirmed: `requires` is split on `=>` into (capability, provider). `provides` is verbatim. So an author who writes `provides: "capability=>ProviderKind"` (mirroring the requires syntax, which is exactly what the tool's own MCP `usages` prints: "this face provides: ThemeProvider, control.theme") creates a provider token that never matches.

To be fair: is the correct `provides` value just "control.theme"? Then would the widget's requires "control.theme=>ThemeProvider" resolve? capability="control.theme" matches provides "control.theme" → provider found → then kind check: `provider.kind` must equal requirement.provider ("ThemeProvider"). The Theme face's kind is "Theme" → then the error would be "found ... but its kind is `Theme` instead of `ThemeProvider`". So provider kind must equal the face's `kind`. So the correct requirement would be "control.theme=>Theme".

So: to make an answered requirement, `requires: "control.theme=>Theme"` and `provides: "control.theme"`. Let's verify — this gives a *positive* evidence for converge's "named when it is" verdict and settles the semantics. Then to reach UNANSWERED, edit the requires to a capability nobody provides... but the connector rejects that write. Unless the provider is deleted afterwards (delete is preview-only in my test; I need apply:true to actually delete).

So the path to UNANSWERED: 
1. theme provides "control.theme", widget requires "control.theme=>Theme" → accepted.
2. delete theme (apply:true, confirm:true) → does delete check dependents? Let's see.
3. converge widget → requires readable, provider gone → UNANSWERED.

That would prove converge works AND expose a real workflow: after deleting a provider, the tree is broken but the tool reports it. Let's test.

Let me redo: I need to fix theme's provides. I can `apply edit` theme with provides "control.theme" (provides is presumably editable; CREATE_FIELD_ORDER vs edit order — handle_contracts are add-only but provides should be editable). Let's try.

## step 82（turn 1，in 1253 / out 495 tokens）

推理：

The theme edit was only a *preview* (not applied) — I didn't pass apply:true, so theme still has "control.theme=>ThemeProvider". Hence widget's requires still fails.

Let me apply the theme edit, then retry widget requires "control.theme=>Theme".

Also interesting: `apply edit` preview keeps previous text in `.xirang/trash/faces/...` — note this means `usages`'s "unreadable faces 3" filter excludes `.xirang/`? Earlier source_index said `.xirang/` is filtered for `.rs` files. Good.

Let me run the sequence with apply:true for theme, then widget.

## step 83（turn 1，in 1907 / out 278 tokens）

推理：

Excellent — this is a strong positive path:
- `converge` reports `control.theme => Theme answered by root/control/theme` (named when it is).
- After deleting the provider, converge reports "kernel verdict: this package's own faces are rejected ... input `control.theme` has no provider" — the UNANSWERED case is covered by a *kernel verdict* block rather than the word "UNANSWERED". Hmm, the catalog claims "named when it is and UNANSWERED when it is not". Here the `requires` line is *absent* (replaced by the kernel verdict block), so the literally-promised "UNANSWERED" spelling does not appear. Let me check the source: does converge print "UNANSWERED" anywhere?

Let me grep converge.rs for UNANSWERED.

This is a real "描述与行为不符" candidate. Let me verify.

## step 85（turn 1，in 892 / out 342 tokens）

推理：

The `UNANSWERED` branch exists but requires `rejected == None` (the package's own faces registered OK) while the requirement has no provider. `rejected` comes from running the kernel validation on the package. Can that state occur? The kernel's connector rejects an unattached input → rejected=Some. So UNANSWERED requires the requirement to be unanswered yet the kernel to accept the tree. Where is `requirements` computed? Maybe it checks the *derived* tree while `rejected` comes from the loaded registry. If a requirement names a provider kind that exists but the capability is provided by a face in a *different* registry... hmm.

Actually: maybe `rejected` is None when the requirement's answer is in an *external* registry (graft) or when the requirement is declared on a face whose parent rule doesn't enforce connectors. Or when the face has `flow`/`flow_provider`. Hmm.

Let me look at how `requirements` is computed in converge.rs to see whether UNANSWERED is reachable. If it's unreachable, that's a finding: the tool's headline verdict is dead code. But careful — I need evidence. Let me read converge.rs top part.

## step 86（turn 1，in 1728 / out 685 tokens）

推理：

`rejected` is Some only when `load_registry` fails. `load_registry` presumably runs the kernel validation (connector included). So when a requirement is unanswered, load_registry fails → rejected=Some → the UNANSWERED branch never runs.

Could there be a case where load_registry succeeds but a requirement has no provider? The connector checks requirements only for faces with `flow`/requires? The connector's `attachment failed` triggers for declared requires. If the connector skipped something the `offerings` map marks unanswered... e.g. requirement answered by an *ancestor* provider: `provider_for_capability` includes `ancestor_capability` — but `offerings` is built from `authored.provides` tokens of *faces*, and for an ancestor provider, the offering face is the parent (which may be hand-written → unreadable → not in offerings!). Then: parent face provides "x" via a hand-written declaration (compiled snapshot has provides=[…]), the connector finds the ancestor provider → load_registry OK; but `offerings` has no entry (unreadable) → converge prints UNANSWERED even though the kernel accepted it. That's a *false* UNANSWERED — the exact inverse misleading case!

Hmm, that's plausible but I'd need to construct it: a hand-written parent face that provides a capability, and a generated child requiring it. In the sandbox: the parent `control/control.rs` is hand-written; does it provide anything? `derive Registry` host... Let me check whether I can add `provides` to the hand-written control.rs and a generated child requiring it. The child is generated (readable) → authored.requires = "x=>ControlKind". The connector: ancestry provider = parent (control) whose provides contains "x" → accepted. offerings: faces include control (hand-written → read_back Err → unreadable) → offerings has no "control" kind entry. parse_requirements(requires, offerings) → looks for offerings[provider_kind] → none → UNANSWERED.

That would be a REAL false verdict: converge says UNANSWERED for a requirement the kernel accepted. That's a great "会误导" finding. Let's test it in the sandbox. I need to modify the hand-written control.rs in /tmp (allowed, it's a copy).

Wait — but does the parent's `provides` even get consulted? `ancestor_capability(target, capability)` where target = entry.info.parent. Let me check ancestor_capability: it looks at ancestors of the target → ancestor_providers. Should include `control` face.

Let me check what the control.rs declaration looks like and add `provides: ["theme.tokens"]`. Then create a generated child `widget` requiring `theme.tokens=>Control`.

Let me do it.

## step 88（turn 1，in 1484 / out 587 tokens）

推理：

The widget still carries the old requires `control.theme=>Theme` (whose provider was deleted), so the tree is already broken and the new edit can't be evaluated. I need to first clear the widget's requires (edit with requires "" — is empty allowed? `text()` returns "" → maybe). Let me set requires to "" (empty) and then to the ancestor-answered one.

Interesting also: `verify` says "verdict ok" while `usages`/`converge` say "the package's own faces were rejected"! Wait — verify ran AFTER and said ok with "faces 4 (source) vs 4 (build)". Hmm! That's a *contradiction*: verify's kernel validation says OK, while converge/usages' `load_registry` says the connector rejected widget.

Hold on: verify ran after the failed apply edit, and `apply` is rejected so the file wasn't written. But the widget's requires `control.theme=>Theme` was applied successfully earlier (spec20 call 1) and its provider theme was deleted (spec20 call 4). So the tree should be broken → verify should fail. But verify says ok!

Hmm, maybe `verify` uses a different validation entry (the CLI's `check` entry) that doesn't run the connector? Or ... wait, "faces 4 (source) vs 4 (build)". The build evidence was published by this verify run.

Actually hold on: maybe the delete moved theme.rs to `.xirang/trash/...` and the source walk skips `.xirang`, so theme is gone. The widget's requires needs a provider of kind Theme → none → connector should reject. But verify says ok.

This is a **real contradiction between two surfaces**: `xirang.verify` (the CLI's `check` entry) accepts a tree that `xirang.usages`/`converge` (and the write path) reject via `load_registry`. That is a big stability finding — two tools give opposite verdicts on the same package at the same moment. Let me confirm by running the CLI `xirang check` on the same root.

Let me check.

## step 89（turn 1，in 199 / out 397 tokens）

推理：

Confirmed: the CLI `check` and MCP `verify` say OK for a tree whose generated face requires `control.theme=>Theme` with no provider anywhere, while MCP `apply`/`usages`/`converge` (which run the full `load_registry` including the connector) reject it.

Hmm wait — is `load_registry` in MCP doing something the check doesn't? Let me check `load_registry`: maybe it registers faces one at a time (via `register_all`) rather than the compile-time snapshot batch, so the connector runs differently (e.g. `registry_connector` on a batch). And `check`/verify uses the build-time static pass which may skip the connector (`phase=static plan` only covers static plan + registration rule?).

Let me look at what `check` validates vs `load_registry`.

This is the crux finding. Let me investigate briefly to describe it accurately.

`load_registry` in mcp/apply.rs.

## step 90（turn 1，in 1122 / out 334 tokens）

推理：

`load_registry` uses `register_snapshot_batch` (which runs the connector). CLI `check` presumably validates via build-time diagnostics (static plan + registration rule + scope), maybe not the connector. So `verify`/CLI `check` do NOT run the data-flow connector, while the write path and `usages`/`converge` do. Two surfaces give opposite verdicts.

Hmm, wait: is that "opposite verdicts on the same tree"? verify says "verdict ok (the kernel accepted the tree)". The MCP table says verify "drives the same entry the CLI's check drives". So `verify` and `check` agree. But `usages` says "the package's own faces were rejected". So within the MCP bridge itself, two tools disagree about the same package at the same instant: `xirang.verify` → ok; `xirang.usages`/`converge`/`apply` → rejected.

That's a strong, reproducible "会误导" finding: an agent that runs verify first (as the catalog suggests) gets "the kernel accepted the tree" and then every write/read is refused.

Actually, let me double check whether verify's kernel validation includes the connector. Let me look at the `verify.rs` implementation.

## step 91（turn 1，in 1797 / out 523 tokens）

推理：

`verify` uses `build_time::check_for` which is the *static* pass. Let me confirm that the static pass doesn't run the connector — the earlier verify failure output showed "phase=static plan". So check covers static plan + registration rule (and maybe admission), but not the data-flow connector. Confirmed by the CLI agreeing ("xirang check: ok").

Meanwhile `load_registry` → `register_snapshot_batch` → connector → rejected.

Hmm, but wait. Is the difference "the static pass doesn't see hand-generated snapshots' requires"? Also note: `verify` at spec21[3] says "faces 4 (source) vs 4 (build)". Hmm.

Actually — hold on. Is my sandbox tree *really* invalid? The widget requires `control.theme=>Theme`; theme was deleted. The connector in `register_snapshot_batch` rejects. The static check doesn't. Which one is "the kernel"? Both are kernel code. The point stands: **two MCP tools give contradictory verdicts on the same tree**, and the one whose description says "the kernel accepted the tree" is the lenient one. This is a genuine stability/credibility issue and directly on-theme for "可信度链路".

But careful: is this an artifact of my synthetic sandbox (deleting a provider is only possible via the MCP write path, which itself is preview-gated)? The delete succeeded with apply:true + confirm:true, so yes it's a reachable state through the tools themselves: apply(add theme with provides) → apply(edit widget requires X) → apply(delete theme, confirm) → then verify says ok while usages says rejected. Fully reproducible through the bridge's own write path. 

Let me now verify this is deterministic and record the exact sequence. Also let me check: does the CLI's `check` ever run the connector? Let me grep check_for phases quickly to describe accurately.

## step 97（turn 1，in 2057 / out 2094 tokens）

推理：

Confirmed: the static `check` pipeline covers requirements (`aggregate_requirements`) — hmm, `aggregate_requirements` is called. So why did check pass? Because the widget's requires `control.theme=>Theme`… `aggregate_requirements(src, &nodes, false, &scope, ...)`: maybe it checks that the requirement provider kind exists among *static* faces in scope. theme.rs was moved to `.xirang/trash/`, so it's not in the source tree → no provider of kind Theme → should error...

Unless `aggregate_requirements` only validates that the requirement's *capability token* is well-formed, and the "provider kind" check is the connector's job. Evidently check passed, so the static pass doesn't enforce attachment.

OK — this is a solid finding. I have enough. I should be careful in wording: "the static pass (CLI check / MCP verify) accepts a tree the runtime connector (load_registry, used by apply/usages/converge) rejects".

Now, I've spent a lot of effort. Remaining must-dos:
1. The 6-question debug comparison — the most important part. I need to send the questions to the captain for codegraph answers.
2. Write the report.

Let me define the 6 questions now (fixed before running) and run xirang side, and message the captain immediately so he can work in parallel.

6 questions (2 call-chain, 2 where-is/who-calls, 2 what-changed/impact):

Q1 (调用链): `base_registry` 到 `register_all` 的调用链是怎么走的？（调用者与被调用者）
Q2 (调用链): `Registry::register_snapshot_batch` 在失败时经过哪些函数产出 connector 诊断？（追踪 `register_snapshot_batch` → `connector_error`）
Q3 (某符号在哪 / 谁调用它): `paint` 这个函数定义在哪几个文件、各自行号是多少？谁调用它？
Q4 (某符号在哪 / 谁调用它): `face_views` 定义在哪、被哪些地方调用？
Q5 (哪里改了 / 影响面): 如果改 `kernel/src/registry_core/mir/text.rs` 里的 `MirGraph::from_mir_text`，影响面有哪些（谁调用它）？
Q6 (哪里改了 / 影响面): 改 `kernel/src/registry_core/tree/connector.rs` 的 `names_face`… hmm names_face is in build_time. Let me pick: 改 `toolchain/src/build_time/src/graft_view/plan_rows.rs` 的 `graft_plan_rows` 会影响谁？

Hmm — for a fair comparison, the questions must be answerable by both. codegraph indexes the whole repo (Rust). xirang-mcp's source tools index the *package root*. I'll run xirang with root = repo root? That's a virtual workspace (no [package]), so namespace fails; but search/inspect/callgraph/read/status only need the source root... Let's see: XIRANG_PACKAGE_ROOT=/home/nich/Moirai_N3/nichlink → `package_root()` resolves; for search, the tree half needs namespace → "tree half unavailable" but the source half answers. callgraph — does it need namespace? Let me check callgraph.rs: it likely uses load_sources only.

Let me verify by running the 6 questions with root = repo root.

Differences to expect: xirang's callgraph is heuristic name-based (callers matched by name across the whole tree), so it may list unrelated call sites; codegraph is AST-based. That's the core comparison.

Let me first fix the questions and send to captain, then run xirang side.

Also: the "哪里改了" class — xirang has `diff` (face-level, needs build evidence) and `impact` (face-level, needs a face). For a question about a *source file* change, xirang has no answer at all (files aren't faces). That's a key finding: xirang MCP is registry-face-oriented; codegraph is symbol-oriented. Questions 5 and 6 should be phrased so both can attempt.

Let me phrase the 6 questions in a neutral "debug question" form:

Q1 调用链：`Registry::register_snapshot_batch` 被调用后，到产生 `connector_error` 的那段调用链经过哪些函数？
Q2 调用链：`graft_plan_rows` 到 `GraftPlanDocument::parse_graft_plan_document` 之间的调用链是什么？
Q3 符号位置＋调用者：`names_face` 定义在哪个文件哪一行？谁调用它？
Q4 符号位置＋调用者：`MirGraph::from_mir_text` 定义在哪？谁调用它？
Q5 影响面：如果给 `Registry::provider_for_capability` 加一个参数，哪些文件/函数需要一起改？
Q6 影响面：`anchor`… let me use: 如果改 `toolchain/src/mcp/tools.rs` 里的 `DISPATCH` 表，哪些地方会受影响？

Hmm, Q6 should be about "哪里改了" — a change. Let me make:
Q5: 改 `provider.capability` 的匹配规则（`kernel/src/registry_core/tree/connector.rs` 里比较 `info.provides` 的那段）——影响面有哪些（谁调用 `provider_for_capability` / `ancestor_capability`）？
Q6: 改 `NichlLink`… ok simpler: `Registry::register_snapshot_batch` 的调用方有哪些（谁调用它）？这既是"谁调用它"也是"影响面"。

Let me settle on 6 stable questions:

- C1 (调用链) `register_snapshot_batch` → `connector_error` 的调用链。
- C2 (调用链) `graft_plan_rows` → `parse_graft_plan_document` 的调用链。
- W1 (在哪/谁调用) `names_face` 在哪、谁调用。
- W2 (在哪/谁调用) `from_mir_text` 在哪、谁调用。
- I1 (影响面) 给 `Registry::provider_for_capability` 加一个参数，谁受影响。
- I2 (影响面) 改 `DISPATCH`（toolchain/src/mcp/tools.rs），谁受影响。

Good. Send to captain now, then run xirang side.

## step 98（turn 1，in 1683 / out 581 tokens）

推理：

The xirang side gives answers. Note DISPATCH (a const) is not found by symbol name — search "DISPATCH" returned `dispatch` (case-insensitive!) and test functions. That's a miss: `search` matched `dispatch` lowercase, and did not find the `DISPATCH` const at tools.rs:316. Let me double-check by searching "DISPATCH" — the results show `fn dispatch` in protocol.rs, not the const. So `search` only indexes functions (and faces + files), not consts/statics/types. That's an important limitation: it cannot locate a const.

Wait, is search case-insensitive? "DISPATCH" → "dispatch" yes. Let me confirm with a different query to see. Also check `inspect` on tools.rs to see if DISPATCH is listed (inspect only lists fns).

W1: `names_face` definition found, but "谁调用它" needs callgraph. Let me run callgraph for names_face and from_mir_text, provider_for_capability, and also `search` for the consts.

Let me also run `xirang.callgraph` for provider_for_capability, names_face, from_mir_text.

Note: no "where is the const DISPATCH" answer → xirang can't locate it; codegraph will. Good contrast.

Let me run those.

## step 99（turn 1，in 1635 / out 459 tokens）

推理：

Great. Note: `inspect toolchain/src/mcp/tools.rs` lists fns but not the `DISPATCH` const → confirms `search`/`inspect` index only functions (plus files/faces). Also `include` of `mod tools_tests` is a #[path] module, not an fn.

Note the xirang answers for C1: `register_snapshot_batch` callers are mostly tests (name-based matching across the whole tree, 41 callers) and callees include `connector_error`. It does NOT give a path from register_snapshot_batch → connector_error (no transitive chain) — the tool only gives 1-hop callers/callees. codegraph gives a call path.

Now I'm waiting for the captain's codegraph answers. Meanwhile, let me finish the remaining probes and start writing the report.

Remaining probes to be thorough:
- `xirang.read` boundary (line beyond file end, context clamp) — quick.
- The `unparsable faces N` line — need a package with an unparsable registration file. Could create in sandbox: a file with two macro invocations. Quick test to confirm the line appears.
- Protocol: initialize/tools/list done. Maybe test notification handling (not required).

Let me do a couple quickly, then check mailbox.

Actually, let me also verify the "registered but not implemented / implemented but not registered" question rigorously: compare DISPATCH to catalog (test pins it) and check for other exported handler-shaped fns.

Let me grep the mcp module for `pub(crate) fn` to list all entry points.

## step 100（turn 1，in 700 / out 253 tokens）

推理：

Every "tool-shaped" entry maps into the 17 (registry.rs has `registry` + `namespace`; build_evidence has helpers; trace.rs has trace + helpers; mir.rs has mir+unified; overlay.rs has overlay routed from explain). No orphan handler. Good — I can state the reconciliation cleanly.

Wait: `trace.rs:144` and `:167` — what are those? Let me check they're helpers not tools. And `mir.rs:144` = `unified`. Let me confirm trace.rs 144/167.

Let me check quickly.

## step 101（turn 1，in 441 / out 355 tokens）

推理：

Clean. No orphan handlers.

Now let me test the `unparsable faces N` line and `read` bounds, then write the report while waiting for codegraph answers.

Let me do a quick batch.

For unparsable: create a face file with two macro invocations in the sandbox. e.g. `.xirang`? No, a source file: src/control/object/broken/broken.rs containing two `crate::control_object! {}` blocks. But that would make the tree invalid for other tools... it's the sandbox, and it's my last test there. Actually the unparsable line appears in `search`/`registry`/`usages` outputs. Let me add it at the very end.

Let me do read bounds first + then unparsable.

## step 102（turn 1，in 1016 / out 413 tokens）

推理：

read clamps correctly (999999 → whole file up to 240 lines; the file is 85 lines so 1-85).

Now the unparsable-faces test, and then write the report.

Actually, let me not break the sandbox tree further — I'll copy the sandbox to a second root for this test. Simpler: add the broken file, run registry/search/usages, then remove it. Let's do it.

## step 103（turn 1，in 669 / out 275 tokens）

推理：

`unparsable faces 1` works.

Now let me check for codegraph answers in my mailbox. Let me check team status / wait for the captain. Meanwhile I'll start drafting the report.

Let me check if there's mail. `agent_teams_status` shows members; mail arrives at model steps. I'll do a status check.

Actually let me start writing the report now, leaving the §3 codegraph column to fill in when the captain replies.

Let me first confirm the repo is clean of my probes (I wrote nothing into the repo yet), and check git status.

Also note: I ran `verify` against the real example, creating target/xirang/out (gitignored). Fine.

Let me check git status and then write the report skeleton.

## step 104（turn 1，in 687 / out 2078 tokens）

推理：

No captain reply yet. I'll draft the report now and fill the codegraph column when it arrives.

Let me write the report. Content plan (≤300 lines):

# MCP 能力面实测与 codegraph 同题对照（2026-09-29）

0. 环境与方法 (brief): binary, root, sandbox at /tmp, RUSTC_BOOTSTRAP for MIR.

① 工具对账表 (17 rows: name / 注册 / 实现 / 描述与行为是否相符 / 结论)
② 逐工具实测（含失败原文）
③ 6 题对照
④ 可用性结论
⑤ 缺口清单

Let me be careful about line count. 300 lines max. Let me write it tight.

Key evidence set:

对账:
- tools/list returns exactly 17, in catalog+DISPATCH order.
- No orphan handler; DISPATCH==catalog (pinned by tools_tests::the_dispatch_table_follows_the_catalog).
- Descriptions vs behavior mismatches found:
  a) `xirang.apply`: `fields.module` must be a bare snake_case name (a logical path is refused: "invalid module name `control::object::widget`; use snake_case ASCII"); `exports`/`requires`/`provides` must be **strings** (arrays refused: "`exports` must be a string"); `handle_traits` is a bare label, passing the Rust path silently produces `handle_traits: ["crate::control::ControlHandle"]` → parent rule rejects. Schema declares `fields` as an untyped object. → 描述不全。
  b) `xirang.converge`: description promises "UNANSWERED when it is not"—the literal UNANSWERED branch (converge.rs:156) is only reachable when the kernel accepts the tree; in practice an unanswered requirement makes `load_registry` fail first, so the reader gets "kernel verdict: this package's own faces are rejected" instead. Never observed UNANSWERED in any run.
  c) `xirang.trace` description promises frames matched...; no. Let's use: trace's "declared-at" display vs converge's matching by call-at (cross-tool inconsistency). That's a behavior-vs-description issue for converge ("faces that ran").
  d) `xirang.search` description: matches faces on logical path, kind, module, registry_name — it does NOT search declaration content (`exports`, `requires`): query "control.render" → "no matches" although 4 faces declare it. Not a lie, but the description's list is what it is; the gap is that it's the only search.
  e) `xirang.status`'s description "indexed Rust file/function counts" ok.
  f) `xirang.impact` schema `depth` minimum 1 but depth:0 silently clamps to 1 — schema is the contract (0 is out of schema), so not a mismatch; note it as minor (no error, silent clamp).
  g) `implementation but not registered`: none. `registered but not implemented`: none.
  h) Tool count claims in module docs (lib.rs "Five tools index Rust source text") — stale-ish: five index source text (search/inspect/callgraph/read/status) — that's 5, correct.

② Per-tool:
1. search — ok; face+file+fn; build verdict; `unparsable faces N`; tree half unavailable on a virtual-workspace root with a clear message. Negative: no content search (exports/requires), no const/static/type search ("DISPATCH" → returns `fn dispatch`).
2. inspect — ok; functions + registrations; consts not listed.
3. callgraph — ok but name-based; `paint` matched 2 definitions; 41 callers for register_snapshot_batch (mostly tests) — cross-tree name matching, warns itself.
4. read — ok; clamps line/context; refuses traversal.
5. status — ok.
6. apply — works end-to-end in sandbox; preview/apply/rename/delete; refuses non-generated modules; field types surprising.
7. registry — ok.
8. explain — ok; requires build evidence; overlay+node refused cleanly.
9. diff — `no build evidence` before verify; after verify, face-level delta ok; records mode ok.
10. trace — absent message actionable; positive path with a real artifact; values mode ok.
11. mir — real MIR text from RUSTC_BOOTSTRAP; jsonl emission + round trip; missing-file message names the producer.
12. unified — merges, Live vs Mir confirmed (live 1).
13. grafts — matches CLI; unkept counting; target_path ignored when identity resolves (minor).
14. impact — ok; depth clamp.
15. usages — fields read-back works on generated modules; hand-written faces counted unreadable (real example: all 3 faces unreadable).
16. converge — ok; answered verdict works; UNANSWERED unreachable.
17. verify — ok; but contradicts usages/converge (the big one).

③ 6 questions.

④ Conclusion: 完善吗？可用吗？缺什么。

⑤ Gaps list with 现象/影响/值不值得做.

Now the "谁更稳" per question needs codegraph answers. Let me check mail again before writing. I'll write what I can and fill in.

Actually let me write the file now with a placeholder for §3 codegraph column and update after the captain replies... but the file is a repo artifact; partial writes are fine as long as final is complete. Let me first check if a reply arrived by doing one more tool call round.

I'll proceed to draft the full report text in my head/write it, and if codegraph answers arrive, I'll fill them in.

Let me create docs/audit-2026-09-29/ directory and write the report.

Actually — I should wait a bit for the captain. Let me do useful work: verify the round-trip of the JSONL that `mir jsonl:true` prints (feed it back). That's an advertised property ("what it prints reads back here"). Quick test.

And test `mir` with an unidentified text dump + `against` (delta says what it cannot rule out). Also test foreign snapshot refusal. These are "可信度链路" relevant — high value for the four themes.

Let me do that: 
- save jsonl output to file button.jsonl in sandbox, then mir path=button.jsonl → reads back.
- copy button.mir → button2.mir (text, no snapshot) and mir path=button2.mir against=button.jsonl → delta behavior.
- create foreign jsonl by changing namespace → refused.

Let me run.

## step 105（turn 1，in 352 / out 458 tokens）

推理：

A real bug found: the advertised round trip fails. `mir jsonl:true` prints a trailing newline (line 401 empty), and reading it back fails with `MirParseError { line: 401, message: "record must start with `{`" }`. The catalog says: "what it prints reads back here" and the truncation hint says "pass what this prints to a JSONL-suffixed file and `xirang.mir` reads it back".

Wait — but maybe the printed text was *truncated* by `bounded()` at MAX_ROWS=400! The output had 401 lines: 400 rows + "… +N more"? Let me check: `bounded(&graph.to_jsonl(), "lines", "pass what this prints ...")` — if the graph has more than 400 lines, it truncates and appends a truncation line. The whole graph is 11 functions + 321 locals + 168 calls = 500 lines → truncated at 400. So the file has 400 JSON records plus 1 truncation notice line → re-reading fails at line 401.

So: `mir jsonl:true` on any artifact bigger than 400 records emits a JSONL file that its own reader rejects. The advertised round trip holds only for graphs ≤400 lines. That's a genuine, high-value finding for the "可信度链路/稳定" theme: the tool's own writer produces an artifact its own reader refuses.

Let me verify precisely by checking the tail of the file and re-running with a smaller graph (e.g. a MIR text with few calls) to confirm the round trip works when under the cap.

Let me check the tail.

## step 106（turn 1，in 222 / out 655 tokens）

推理：

Confirmed exactly: `… truncated: 501 lines total, 400 shown. pass what this prints to a JSONL-suffixed file and `xirang.mir` reads it back.` — the truncation notice is *itself* appended to the JSONL body, so the file is no longer valid JSONL. The doc instructs the reader to save it as a file and read it back; the read fails at that line.

Also the truncation notice text literally says "pass what this prints to a JSONL-suffixed file and `xirang.mir` reads it back" — which is false in this case.

Let me confirm the small-graph round trip works (to be fair): create a small MIR text file with 2 functions, emit jsonl, save, read back.

Let me do it.

## step 107（turn 1，in 1697 / out 211 tokens）

推理：

The captain replied with a summary, but he says "原始转录在 /tmp/codegraph-baseline.md（180 行，直接读它，不必让我粘贴）". I should read that file — it's on the same machine. Let me read it. It's in /tmp so it's accessible.

Also two verification tasks requested:
1. Test whether `names_face`/`from_mir_text`/`DISPATCH` have covering tests (run them).
2. Note `+N more` truncation risk for impact questions.

Let me read /tmp/codegraph-baseline.md first.

## step 109（turn 1，in 5073 / out 1031 tokens）

推理：

Now I have all the codegraph material. Let me do the two verification items the captain asked for:

1. Does `names_face` have covering tests? The doc in declared.rs says the range behavior is pinned by `graft_plan_check.rs` tests. Let me find the test(s) that call `names_face`: from xirang callgraph answer, callers of names_face were:
   - toolchain/src/build_time/src/graft_plan_check.rs::a_string_range_names_both_endpoints_as_data
   - toolchain/src/build_time/src/graft_plan_check.rs::undeclared_plan_errors
   - overlay_rows.rs::overlay_projection
   - plan_rows.rs::entry_rows
   - cli/explain_report.rs::declared_report
   - mcp/impact.rs::impact
   - studio/app/graft.rs::declaration_for
   
   So `a_string_range_names_both_endpoints_as_data` is a `#[test]` that calls names_face directly. Let me verify it's a #[test] and run it. Also check whether graft_plan_check.rs has tests calling names_face... wait, `a_string_range_names_both_endpoints_as_data` is IN graft_plan_check.rs and calls names_face? Let me verify. Actually xirang's caller list says `toolchain/src/build_time/src/graft_plan_check.rs::a_string_range_names_both_endpoints_as_data` — so yes, a test function in graft_plan_check.rs calls names_face.

   Also `toolchain/src/mcp/impact.rs::impact` calls names_face — that's a real caller (production).

   Hmm wait, codegraph said names_face has 6 callers: overlay_rows.rs, cli/explain_report.rs, mcp/impact.rs, plan_rows.rs, +2 more. The +2 more are likely graft_plan_check.rs (x2). So codegraph's caller list agrees with xirang's, but codegraph claimed "no covering tests found" — a false negative, and importantly it *truncated* the +2 more which happen to be exactly the test-bearing file.

2. `from_mir_text` — codegraph said "no covering tests found" while returning text.rs with two tests. xirang says callers include text.rs::parses_native_textual_mir and text.rs::text_that_is_not_mir_yields_an_empty_graph. So they're tests in the same file.

3. `DISPATCH` — codegraph said no covering tests; comment says tools_tests pins it; xirang callgraph can't answer for a const, but grep/tests exist. I'll run the tests.

Let me run:
- `cargo test -p xirang-toolchain --offline --features build... ` hmm which feature gates build_time? The toolchain's `build` feature. And `graft_plan_check` is in build_time module. Let me find the test names and run them.

Run: `cargo test --workspace --offline <name>` → but that compiles everything. Let's do targeted: `cargo test -p xirang-toolchain --offline -p ... ` Actually `toolchain` package = xirang-toolchain; kernel tests are in the kernel package (xirang-kernel). The tests:
- `a_string_range_names_both_endpoints_as_data` in toolchain build_time → `cargo test -p xirang-toolchain --offline --features build a_string_range_names_both_endpoints_as_data`
- `parses_native_textual_mir` + `text_that_is_not_mir_yields_an_empty_graph` in kernel → `cargo test -p xirang-kernel --offline parses_native_textual_mir`
- `the_dispatch_table_follows_the_catalog` in toolchain mcp → `cargo test -p xirang-toolchain --offline --features mcp the_dispatch_table_follows_the_catalog`

Let me run these three in one command with tail truncation.

Also, to be rigorous about "谁在钉 names_face": check whether these tests actually *assert* names_face behavior (not just call it). Let me read the test.

Let me do both.

## step 110（turn 1，in 790 / out 269 tokens）

推理：

Confirmed: `names_face` IS pinned by `graft_plan_check.rs::a_string_range_names_both_endpoints_as_data` with real assertions on its behavior (both endpoints, literal " to " not split). codegraph's `⚠️ no covering tests found` for names_face is a **false negative**.

Let me run the three tests to prove they exist and pass.

## step 111（turn 1，in 401 / out 300 tokens）

推理：

The third run: `the_dispatch_table_follows_the_catalog` — it ran with `--features mcp` but mcp lib tests aren't in the default test target? Earlier the output says "0 passed ... 1 filtered out" for some target. Let me run with `--lib` and check. Actually the toolchain package's mcp module may need `--features mcp` AND the lib target. The output shows "running 0 tests" for one target and "tests/runtime_path_compat.rs". Hmm, the lib tests should have been included. The output got truncated by tail. Let me re-run just that with a clearer capture.

Note: I must be careful — running full `cargo test -p xirang-toolchain` compiles a lot but it's fine.

Let me re-run the dispatch test properly.

## step 112（turn 1，in 258 / out 1921 tokens）

推理：

All three "no covering tests found" claims are false negatives, and I have the exact test IDs and pass output:
- `build_time::graft_plan_check::tests::a_string_range_names_both_endpoints_as_data` ok (pins names_face)
- `registry_core::mir::text::tests::parses_native_textual_mir` / `..._yields_an_empty_graph` ok (pin from_mir_text)
- `mcp::tools::tools_tests::the_dispatch_table_follows_the_catalog` ok (pins DISPATCH)

Now let me also address the captain's second point: `+N more` truncation vs xirang's same-question behavior. xirang callgraph also truncates: `register_snapshot_batch` had "callers (41): ... +21 more" — wait, xirang printed 20 callers then "… +21 more"? The output showed "… +21 more" at the end of the list (yes: `… +21 more`). Actually with limit 5, callgraph bounds *definitions* (matches) and the caller list is bounded too. The output showed 20 callers then "+21 more". So xirang lists 20 of 41 and says "+21 more" — also a truncation, but it *names the count of withheld callers* ("+21 more"). codegraph lists 4 + "+4 more" — with 41 callers, "+4 more" is *wrong*/*not* the number withheld (it says more, not 37). Hmm — careful: codegraph's "+4 more" probably means "+4 more files" not "+4 more callers". The transcript says "只列 4 + '+4 more'". I shouldn't over-interpret; I'll describe it as "the transcript records 4 files then `+4 more`" and note ambiguity. Actually to be fair and accurate, I should be careful: it's the captain's transcription of codegraph output. I'll state what's recorded and flag that the semantics of `+4 more` (files vs callers) is ambiguous in the transcript, whereas xirang's `… +21 more` is explicitly tied to the caller list count.

Hmm — for the report I want to be accurate but I only have the captain's transcript, not codegraph's raw output. I'll mark §3 codegraph column as "队长代跑的一次调用转录（/tmp/codegraph-baseline.md）" and be explicit that it's a summary of codegraph's own output, quoted from the transcript. Good.

Also for fairness: xirang side per question:
- C1: xirang.callgraph(register_snapshot_batch, limit 5) → definition + 20 callers (+21 more) + 25 callees, one of which is `connector_error`. So xirang ALSO does not give the chain; but its callee list *does* contain `connector_error` directly, so "one hop" answers it. Wait — is `connector_error` a direct callee of `register_snapshot_batch`? The callees list included `connector_error`. So the chain is one hop: register_snapshot_batch → connector_error. Hmm, but the connector_error callers list says 2: graft_ops.rs and transaction.rs. So yes, direct.

  So for C1 xirang answers better: it says the direct callee set includes `connector_error` → the chain is length 1. Whereas codegraph gave a symbol bag with 41 callers and 2 callers of connector_error. Both are "symbol bag"-ish but xirang's callee list (direct, complete, name-based) is more directly usable. However xirang's callee list is name-based and includes noise (`Err`, `Ok`, `Some`, `clone`, `from`, `into`, `new`, `push` ...) — clearly std/method noise. So neither is clean.

- C2: xirang.callgraph(graft_plan_rows) → callers 4 (overlay_projection, cli/grafts.rs::plan_rows, mcp/diff.rs::diff_records, mcp/grafts.rs::grafts) and callees include `entry_rows`. So chain = graft_plan_rows → entry_rows → (parse...). xirang gives the first hop; you'd then call callgraph on `entry_rows`. Two calls needed. codegraph gave the whole plan_rows.rs source in one call → readable in one go. So C2: codegraph better (one call, full file), though it also dumped 2 irrelevant sources.

  Let me actually run callgraph on entry_rows to see if xirang can close the chain — that's a fair "one extra call" data point. But the rule was "每题一次调用". Hmm — the captain's codegraph side was one call per question. To be fair I should report xirang's *first* call. But I can note that a second call would close it. Actually, let me just report the first call and note that the callee list already contains `entry_rows`, and a second `callgraph entry_rows` would show `parse_graft_plan_document`... let me verify that in one extra call for accuracy of the claim (I can mark it as a follow-up call, not the first answer). Fine — I'll run it to be able to state it truthfully.

- W1: xirang.search(names_face) → definition file:line. callgraph(names_face) → 7 callers with file::function. codegraph: 6 callers, 4 files + "+2 more", plus false-negative test warning. xirang's 7 callers include 2 test functions and 5 production. Note: xirang says 7, codegraph says 6 — a count discrepancy. Which is right? xirang's list: a_string_range_names_both_endpoints_as_data, undeclared_plan_errors, overlay_projection, entry_rows, declared_report, impact, declaration_for = 7. codegraph says "6 callers in overlay_rows.rs, cli/explain_report.rs, mcp/impact.rs, plan_rows.rs +2 more" = 6. Hmm: codegraph's 4 named files + 2 more = 6 call sites? If the +2 more were graft_plan_check.rs (2 functions) and studio/app/graft.rs → that's 3 more files, so 7. Codegraph says 6 → it may be missing one (studio/app/graft.rs::declaration_for) or counting files. Ambiguous. I can verify the truth by grep: count call sites of `names_face`. Let me do a quick grep to establish ground truth (allowed: it's my own verification, using grep tool). Actually the task says use codegraph for the comparison; but for *verifying* a disagreement, grep is legitimate evidence. xirang's own claim is name-based and could over-count. Let me grep `.names_face(` in the repo.

- W2: xirang: definition + 4 callers (2 tests + 2 production). codegraph: definition + 3 files + false-negative test warning. xirang gives more precise callers (including the two tests). 

- I1: xirang: definition + 1 caller (connector_errors_from) → clear. codegraph: 1 caller in same file + noise (26 slot constants). xirang better (names the caller).
- I2: xirang CANNOT locate `DISPATCH` (search returns `fn dispatch`; DISPATCH is a const, not indexed). codegraph located it at tools.rs:316 + the single use site. So codegraph wins clearly — and this is a real gap in xirang's capability face.

Let me now grep for names_face call sites to settle the count.

Then write the report.

Let me also run `callgraph entry_rows` as a follow-up.

## step 113（turn 1，in 559 / out 8226 tokens）

推理：

Ground truth: `names_face` has 6 call sites in production/test code (impact.rs:95, overlay_rows.rs:137, graft_plan_check.rs:143, plan_rows.rs:160, explain_report.rs:183, studio/app/graft.rs:324) plus 6 assert lines inside the one test. So 6 caller *functions* across 6 files (graft_plan_check.rs:143 belongs to `undeclared_plan_errors`, and the asserts belong to `a_string_range_names_both_endpoints_as_data` → 7 caller functions). So:
- codegraph "6 callers" ≈ 6 production call sites (correct count of call sites), listing 4 files +2 more — the +2 being graft_plan_check.rs and studio/app/graft.rs. And it missed the test function `a_string_range_names_both_endpoints_as_data` in the same file → hence "no covering tests found".
- xirang "7 callers" = the 6 production + 1 test function (name-based, file::function). Both are defensible; xirang's includes the test, which is exactly what makes codegraph's test warning wrong.

And `entry_rows` chain closes: entry_rows callees include `parse_graft_plan_document` and `names_face` → so C2's chain is 2 xirang calls.

Now let me write the report. I need to keep ≤300 lines. Let me draft carefully and compactly.

Also, I should decide the file path: docs/audit-2026-09-29/audit-mcp-probe.md (dir doesn't exist; create it).

Let me now compose the report.

Structure:
- 头部：装置、方法、边界声明
- ① 工具对账表 (17 rows in a table + verdicts)
- ② 逐工具实测（含失败原文）
- ③ 6 题对照
- ④ 可用性结论
- ⑤ 缺口清单

Line budget: I'll aim ~280 lines.

Let me write it.

Key content for ② — I'll compress into a table plus 失败原文 blocks for the important ones. Need to include raw errors.

Let me carefully enumerate the evidence I have:

tools/list: 17 tools, names in order == task list == DISPATCH order.
No orphan handler (grep of pub(crate) fn entry points).
No registered-but-missing handler (tools_tests pins it; test ran ok).

Description vs behavior mismatches:
M1. apply: `fields.module` must be bare snake_case, not a path → raw: "invalid module name `control::object::widget`; use snake_case ASCII".
M2. apply: `fields` values must be strings (except needs_registry) → raw: "`exports` must be a string"; `requires` needs `capability=>provider` → raw: "requires entries must use capability=>provider syntax". The schema declares `fields` as a bare object; nothing in tools/list says this.
M3. apply: `handle_traits` is a *label*, not a Rust path; passing the path silently writes `handle_traits: ["crate::control::ControlHandle"]` and the parent rule then rejects with "handle `Widget` must implement interface `ControlHandle`". (worth 会误导)
M4. converge: the promised `UNANSWERED` spelling (converge.rs:156) never appeared; an unanswered requirement makes the kernel reject the tree first, so the reader gets `kernel verdict: this package's own faces are rejected`. (会误导/不可达)
M5. converge trace: "faces that ran ... matched by source file" matches on the frame's *callsite* file, while `xirang.trace` prints `declared-at=<face file>`; with frames whose node is a face but whose callsite is elsewhere, trace shows the face while converge counts 0 faces. (会误导)
M6. search: cannot find consts/statics/types ("DISPATCH" → `fn dispatch` in protocol.rs); cannot find declaration content ("control.render" → no matches though 4 faces declare it). Not promised, but it's the only search.
M7. apply delete requires `confirm: true` — documented, ok.
M8. explain: overlay+node refused — documented, ok.

Big cross-tool contradiction:
X1. verify/CLI check say `verdict ok` while usages/converge/apply say "the package's own faces were rejected" for the same tree at the same moment. Reproduced via the bridge's own write path (add theme provides → edit widget requires → delete theme). Raw text both sides.

Big self-round-trip bug:
X2. `mir jsonl:true` output is truncated at 400 lines and the truncation notice is appended *into the JSONL body*, so "what it prints reads back here" fails: `MirParseError { line: 401, message: "record must start with `{`" }`. Works only when the graph is ≤400 lines (verified with small.mir → small.jsonl read back OK).

Other:
X3. grafts: when a plan's `target` identity resolves to a face, the declaration match ignores the plan's `target_path` (names_face uses the identity's module). A hand-made inconsistent plan (target=button id, target_path=root/elsewhere) is reported `declared at entry line 48 ...` — I'll label this as "hand-constructed inconsistent plan; low severity".
X4. usages: all real repo faces are hand-written → "fields unreadable"; the field read-back is only reachable after `apply add` (verified in the sandbox).
X5. build evidence tools (explain/diff/search verdicts) all depend on `target/xirang/out`, which exists only after `verify`/`check`; fresh checkout → "build stale"/"no build evidence". Not a bug, but the first-run experience is degraded and `verify`'s side effect is the only way to fix it through the bridge.

Now §3 table. Columns: 题 / xirang 原文要点 / codegraph 原文要点 / 谁更稳 + 为什么.

§4 结论:
- 完善吗：对"注册面"这一层完善（17/17 有实现、目录与分派一致、错误都可行动）；对"符号"这一层不完善（只有 fn/file/face 三个索引维度；const/类型/字段/内容搜索为零）。跨工具一致性有两处硬伤（X1、X2）。
- 可用吗：可用。在 examples/control-button（3 个手写面）上 17 个工具全部返回可读答案；写入路径在沙箱副本里跑通了 add/edit/rename/delete 全环。
- 缺什么：见⑤。

§5 缺口清单（现象/影响/值得做吗）— 6-8 条，排优先级。

Then the four 设想 mapping? The task says the team goal includes judging the four ideas into a landable slice and implementing the top-ranked one — that's t2/t3's job (auditor + implementer). My deliverable is the probe report; but I should give a short pointer in ④/⑤ about which gaps map to which of the four themes, since it's input to t2. Keep it short (a few lines), clearly marked as input, not a ranking decision.

Now write. Let me be careful with line count; I'll write it and then check.

Let me write the file.

输出：

Now I have everything. Writing the report.

## step 114（turn 1，in 144 / out 536 tokens）

推理：

Now let me verify: line count ≤300, git status only shows my file, and check a couple of factual claims I made:

1. "xirang 自己 25 个 callee" for register_snapshot_batch — let me count: the callees were "Err, Ok, Some, clone, connector_error, from, into, into_iter, is_empty, is_some, len, new, plan_batch, push, registry, sort_by_key, submit_snapshot_at, with_children" = 18. I wrote "25 个 callee" in C1 summary — let me fix to 18.

2. "callgraph entry_rows 后 callees 里有 parse_graft_plan_document" — yes confirmed.

3. "grep 实测 6 个生产调用点 + 1 个测试函数" — confirmed.

4. `xirang.check` "verdict ok / faces 3 (source) vs 3 (build)" for real example and "faces 4 vs 4" for sandbox. Correct.

5. "16 个工具的空参调用各自点名自己缺哪个参数，只有 status/registry/diff/grafts/trace/verify 无参可答" — count: 17 - 6 = 11 tools errored on empty args, not 16. Let me recount from spec15: errors were search, inspect, callgraph, read, apply, mir, unified, impact, usages, converge = 10. Non-error: status, registry, explain, diff, trace, grafts, verify = 7. So 10 errored. Let me fix that sentence.

Let me also double check "M2 ... 除 needs_registry 外一律必须是字符串" — yes from code.

Also "converge.rs:156" for UNANSWERED — confirmed line 156.

Also "converge_trace.rs:77" — confirmed (`.find(|face| matches_file(source.file, &face.source))`).

Let me fix the two numeric errors and check line count.

## step 116（turn 1，in 143 / out 369 tokens）

推理：

Now verify a couple more claims before completing:

1. "explain/diff/search 的构建结论都依赖 target/xirang/out" — search's tree half uses face_views (source-derived) but annotates with build verdict; yes.
2. Real example faces count 3 — yes.
3. Check the report renders and markdown table is intact. Let me read a couple of sections to sanity check.

Also the report claims `xirang.search query=control.render → no matches` — verified.
And `search query=DISPATCH → fn dispatch -> protocol.rs:246` — verified.

One more claim to double check: "只读面在真实宿主上 17 个工具全部给出可读、可行动答案" — 17 includes apply (write) and mir/unified which errored on the real root because no .mir there (the error is actionable though). I phrase it as "可读、可行动答案" which is fair since errors were actionable. But to be safe, let me soften: "只读面在真实宿主上全部给出可读、可行动答案（含可行动的失败：mir/unified 因仓库无 .mir 成品而给出产出办法）". Let me check the exact sentence and adjust.

Let me read that part.

## step 118（turn 1，in 134 / out 321 tokens）

推理：

Now final checks:
- line count
- git status
- make sure no leftover repo files (the `docs/audit-2026-09-29/` dir is the only untracked path).

Also I should double check the truncation claim: "… truncated: 501 lines total, 400 shown" — I wrote ">400 行" — correct.

Also confirm the `mir jsonl` small round trip works — verified.

Let me check whether the `verify` run created target/xirang/out in the real example (gitignored, harmless). Yes.

Let me do a final read of the report top to verify formatting, then complete the task.

Also: should I clean up `/tmp/probe`? Not required (it's /tmp, not the repo). Leave it — the captain/auditor may want to reproduce.

Let me verify line count and status.

## step 119（turn 1，in 250 / out 390 tokens）

推理：

246 lines ≤ 300. git status shows only my report dir. 

One last verification of a claim I want to be sure about: "X3: grafts 计划匹配忽略 target_path". Let me re-verify the raw evidence I cite — I removed the orphan_fast plan and replaced it with control_fast (which behaved correctly). The X3 evidence was from the earlier run:
```
orphan_fast: target=root/elsewhere graft=orphan_fast full=true [declared at entry line 48 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`]
```
This is real output I captured. In the report §2.2 I didn't include this raw text; X3 is mentioned in the table row 13 and G8. Let me add the raw snippet to §2.2 so G8 has its evidence, as the task demands evidence for every claim. Let me add one line.

Also let me double check §2.2 mentions X3. It says "一处匹配规则见 X3" in the table. G8 describes it. But evidence (命令+原文) should be present. Let me add.

Let me insert after the X1 block or near M6.

## step 120（turn 1，in 177 / out 146 tokens）

推理：

Oops, that edit was a no-op-ish (removed a blank line). Let me instead insert the X3 evidence properly. Let me view the surrounding lines.

## step 123（turn 1，in 619 / out 2494 tokens）

推理：

256 lines ≤ 300, only my report is untracked. 

Now complete the task. The contract kind is "work" — no structured fields required beyond output (implementation/repair need changedPaths; this is `work`). The task says "implementation/repair also need in-scope changedPaths" — for `work` kind, the instructions say acceptanceResults/commandsRun are needed for implementation/repair/verification/integration. Mine is `work`, but I'll include acceptanceResults/commandsRun anyway since it's harmless and informative... Actually the tool may reject fields not applicable? Let me include acceptanceResults and commandsRun (allowed generic fields) plus output and changedPaths (the report file is in-scope). The contract says "In scope: ; Out of scope: ; Acceptance: ; Verify:" — empty. So I'll supply a reasonable set.

Let me mark completed with output.

Then send the report message to the captain with source_task_id/source_attempt_id, and also send to parent agent id session-5070f212... The instructions say: "send your result to that agent with send_message({agent_id: ...})" AND rule 4 says send one short report with agent_teams_send_message to=captain. The captain is also the parent. The prompt says "The captain is also your parent: this single message satisfies both reporting duties." But the delegated-subagent footer explicitly asks to use send_message to session-5070f212. Both are the same agent presumably. To satisfy both, I'll send via agent_teams_send_message (to=captain) with source ids — that's the formal one — and also send_message to the parent agent id since it's explicitly required. Hmm, "Do not repeat it through the native send_message tool" — conflict. Rule 4 in the system prompt says don't repeat through native send_message. The user-message footer (delegation) says to use send_message. The system prompt's working rules are more authoritative and explicitly say the single agent_teams message satisfies both duties. But the parent id given is a session id; is to=captain the same? Likely yes.

I'll follow the system-prompt rule: one agent_teams_send_message to captain. That satisfies both. Risk: the parent agent (session-...) might not receive it if it's a different entity. Given "The captain is also your parent: this single message satisfies both reporting duties", I'll trust it. Actually, to be safe and cheap, a duplicate native send_message is explicitly discouraged ("Do not repeat it"). I'll do just the agent_teams one.

Hmm, but earlier I sent a message to captain and it was delivered "via live" — and the captain (parent) did receive and replied. That confirms to=captain reaches my parent. Good.

Let me update the task first.

## step 125（turn 1，in 257 / out 647 tokens）

推理：

Task completed and reported. I should now become idle. Let me do a final sanity check that the report file is intact and no repo code changed.

I'm done. Let me give a brief final answer.

输出：

t1 完成。

**交付物**：[audit-mcp-probe.md](docs/audit-2026-09-29/audit-mcp-probe.md)（256 行，唯一改动文件；`git status --porcelain` 只有 `?? docs/audit-2026-09-29/`；驱动脚本全在 /tmp，未入库）

**做了什么**

1. **工具对账**：`tools/list` 恰好 17 个，名字与顺序与任务书、`tools.rs` 目录、`DISPATCH` 表逐字一致；"注册了没实现 / 实现了没注册"= 0（grep 全部入口 + 实测 `the_dispatch_table_follows_the_catalog` ok）。描述与行为不符 4 处，全部带原文。
2. **17 个工具逐个真跑**（真实 `examples/control-button` + `/tmp` 沙箱副本 + `RUSTC_BOOTSTRAP=1` 产出的真 2174 行 MIR + 仓库自己的 `write_trace_artifact` 产物）。写入路径在沙箱里跑通 add → `usages` 读回 23 个字段 → `converge` `answered by root/control/theme` → rename → delete。
3. **三个"会误导"的硬证据**：
   - **X1**：`verify`/CLI `check` 说 `verdict ok`，同一棵树同一刻 `usages`/`converge`/`apply` 说 faces rejected（静态 pass 与运行时 connector 判定面不同）；
   - **X2**：`mir jsonl:true` 在 >400 行时把截断说明写进 JSONL 正文，读回即 `MirParseError { line: 401 }`，而描述和截断提示都写着"存成文件再读回来"；
   - **X3**：计划匹配忽略 `target_path`。
4. **6 题对照**：第③节双栏，写明了装置差异（两个客户端 / 一次调用 / `maxFiles=3` / 措辞同义非逐字）。结论 C1 xirang 略稳、C2 codegraph 更稳、W1/W2/I1 xirang 更稳、I2 codegraph 完胜。codegraph 的三处 `⚠️ no covering tests found` **实测全是假阴性**（附三条测试的 ok 输出），`names_face` 到底有没有测试钉住——已判定：**有**。
5. **④结论 + ⑤缺口清单 G1–G9**（现象/影响/值不值得做），只给证据、按 t2 的分工**不排优先级**。
