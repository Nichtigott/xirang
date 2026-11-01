//! Tests for the MIR channel, the merge, and the delta: the JSONL this tool
//! emits reads back as a snapshot of a named tree, a live call confirms its
//! compiler candidate while the rest stay candidates, and two snapshots diff
//! only when their trees allow it.
//! MIR 通道、合并与差异的测试：本工具输出的 JSONL 能读回来、且是一棵被点名树的快照；真实调用确认
//! 它的编译器候选、其余保持候选；两份快照只有在各自的树允许时才作差。

use std::path::{Path, PathBuf};

use crate::run_method::{CallTrace, trace_artifact_path, write_trace_artifact};
use serde_json::json;

use super::{mir, unified};

/// The MIR text `rustc -Zunpretty=mir` prints for a two-call function: one call the
/// trace below confirms, one it never makes.
/// `rustc -Zunpretty=mir` 为一个含两次调用的函数打印的 MIR 文本：一次被下面的 trace 确认，一次
/// 从未发生。
const MIR_TEXT: &str = "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    _2 = crate::other(move _1);\n    return;\n}\nfn crate::inner(_1: f32) -> f32 { return; }\nfn crate::other(_1: f32) -> f32 { return; }\n";

/// The same function after an edit: one call survived, one was dropped, one was
/// added.
/// 同一次编辑之后的同一个函数：一次调用活了下来、一次被去掉、一次被新增。
const AFTER_TEXT: &str = "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    _2 = crate::new(move _1);\n    return;\n}\nfn crate::inner(_1: f32) -> f32 { return; }\nfn crate::new(_1: f32) -> f32 { return; }\n";

/// A throwaway package holding one MIR text dump.
/// 一个装有一份 MIR 文本转储的一次性包。
///
/// The manifest is load-bearing since the snapshot convention: the JSONL this
/// tool writes names the tree it came from, so a root Cargo can name is what
/// makes the writer able to identify its output at all.
/// 自快照约定起，清单是承重的：本工具写出的 JSONL 点名了它的来源树，因此一个 Cargo 能命名的根
/// 正是写入方得以标识其产物的前提。
fn root(label: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("mcp-mir-{label}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).expect("source directory");
    std::fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname = \"mcp-mir\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(directory.join("src/lib.rs"), "// host entry\n").expect("library target");
    let path = directory.join("dump.mir");
    std::fs::write(&path, MIR_TEXT).expect("the MIR dump");
    (directory, path)
}

/// Emit one artifact as JSONL through the tool, which is the writer under test.
/// 经本工具把一份 artifact 输出成 JSONL，那正是被测的写入方。
fn emit(directory: &Path, source: &str, target: &str) -> String {
    let jsonl = mir(directory, &json!({"path": source, "jsonl": true})).expect("the JSONL emits");
    std::fs::write(directory.join(target), &jsonl).expect("the emitted artifact");
    jsonl
}

/// Record a trace whose only call edge is `crate::mcp::outer -> crate::mcp::inner`.
/// 记录一份唯一调用边是 `crate::mcp::outer -> crate::mcp::inner` 的 trace。
fn record_trace(directory: &Path) {
    let mut trace = CallTrace::full();
    let outer =
        xirang_kernel::identity::NodeId::from_namespaced_path("mcp-mir", "outer/outer.rs", "Outer");
    let inner =
        xirang_kernel::identity::NodeId::from_namespaced_path("mcp-mir", "inner/inner.rs", "Inner");
    trace.with(outer, "crate::outer", |trace| {
        trace.with(inner, "crate::inner", |_| {});
    });
    write_trace_artifact(&trace, &trace_artifact_path(directory), "mcp-mir")
        .expect("the trace artifact writes");
}

/// The graph reports what the dump declared, and the JSONL this tool emits is
/// readable by the same tool — which is the writer that never existed. What it
/// writes is a *snapshot*: the header names the tree the artifact came from.
/// 图报出那份转储声明了什么，而本工具输出的 JSONL 能被同一个工具读回来——那正是从未存在过的写入方。
/// 它写出的是**快照**：表头点名了 artifact 的来源树。
#[test]
fn the_jsonl_channel_round_trips_through_this_tool() {
    let (directory, _) = root("round-trip");
    let report = mir(&directory, &json!({"path": "dump.mir"})).expect("the graph renders");
    assert!(report.contains("crate::outer -> crate::inner"), "{report}");
    assert!(report.contains("crate::outer -> crate::other"), "{report}");
    assert!(report.contains(": f32"), "locals are reported: {report}");
    let jsonl = emit(&directory, "dump.mir", "dump.jsonl");
    assert!(jsonl.contains("\"kind\":\"call\""), "{jsonl}");
    assert!(
        jsonl.starts_with("{\"kind\":\"snapshot\",\"namespace\":\"mcp-mir\",\"root\":\""),
        "the writer's output names its tree first: {jsonl}"
    );
    let reread = mir(&directory, &json!({"path": "dump.jsonl"})).expect("the JSONL reads back");
    assert!(reread.contains("crate::outer -> crate::inner"), "{reread}");
    assert!(reread.contains("crate::outer -> crate::other"), "{reread}");
    assert!(
        reread.contains("snapshot namespace=mcp-mir"),
        "the reader reports the identity the writer stamped: {reread}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// X2 pin: a snapshot too large for one reply is refused, not printed short. The
/// artifact this tool prints is the one answer that must read back, and the sentence
/// a truncation carries cannot ride the payload — JSONL has no legal way to mark
/// itself incomplete — so a payload cut at the reply cap read back here as a
/// *complete* snapshot of a smaller graph (audit `X2`).
/// X2 钉子：一份对一条回复而言过大的快照会被拒绝，而不是被打印成残缺的。本工具打印的 artifact
/// 是唯一必须能读回来的答案，而截断要带的那句话搭不上载荷的便车——JSONL 没有合法方式标记自己不
/// 完整——因此在回复上限处被切开的载荷会在这里读回来成一份**较小的图的完整快照**（审计 `X2`）。
#[test]
fn a_snapshot_larger_than_one_reply_is_refused_rather_than_printed_short() {
    let (directory, _) = root("too-big");
    // 250 functions, each making one call, render as 501 JSONL lines: one past the cap.
    // 250 个函数、每个一次调用，渲染成 501 行 JSONL：恰好越过上限。
    let mut big = String::new();
    for index in 0..250 {
        big.push_str(&format!(
            "fn crate::f{index}(_1: f32) -> f32 {{\n    _2 = crate::g{index}(move _1);\n    return;\n}}\n"
        ));
    }
    std::fs::write(directory.join("big.mir"), &big).expect("the big dump");
    let error = mir(&directory, &json!({"path": "big.mir", "jsonl": true}))
        .expect_err("a snapshot that does not fit one reply is refused");
    assert!(error.contains("501 JSONL lines"), "{error}");
    assert!(error.contains("400-line reply cap"), "{error}");
    assert!(
        error.contains("`--lib` or `--bin <name>`"),
        "the way to a smaller artifact must be named: {error}"
    );

    // The ordinary path is untouched: what it prints still reads back.
    // 普通路径不受影响：它打印出来的东西仍然能读回来。
    let small = mir(&directory, &json!({"path": "dump.mir", "jsonl": true}))
        .expect("a snapshot inside the cap still prints");
    assert!(small.starts_with("{\"kind\":\"snapshot\""), "{small}");
    std::fs::write(directory.join("small.jsonl"), &small).expect("the emitted artifact");
    let reread = mir(&directory, &json!({"path": "small.jsonl"})).expect("it reads back");
    assert!(reread.contains("crate::outer -> crate::inner"), "{reread}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Two snapshots of one tree diff into the relations and functions that moved,
/// and the counts point from the baseline forward.
/// 同一棵树的两份快照作差，得到移动过的关系与函数，而计数从基线指向后一份。
#[test]
fn a_delta_names_what_changed_between_two_snapshots() {
    let (directory, _) = root("delta");
    emit(&directory, "dump.mir", "before.jsonl");
    std::fs::write(directory.join("after.mir"), AFTER_TEXT).expect("the edited dump");
    emit(&directory, "after.mir", "after.jsonl");

    let report = mir(
        &directory,
        &json!({"path": "after.jsonl", "against": "before.jsonl"}),
    )
    .expect("the delta renders");
    assert!(
        report.contains("delta: relations added 1 gone 1  functions added 1 gone 1"),
        "{report}"
    );
    assert!(
        report.contains("added:\n  crate::outer -> crate::new"),
        "{report}"
    );
    assert!(
        report.contains("gone:\n  crate::outer -> crate::other"),
        "{report}"
    );
    assert!(
        report.contains("functions added:\n  + crate::new"),
        "{report}"
    );
    assert!(
        report.contains("functions gone:\n  - crate::other"),
        "{report}"
    );
    assert!(report.contains("snapshot namespace=mcp-mir"), "{report}");
    assert!(
        !report.contains("cannot be ruled out"),
        "both artifacts name their tree, so the comparison is sound: {report}"
    );

    // The direction is the claim: swapping the two swaps added and gone.
    let reversed = mir(
        &directory,
        &json!({"path": "before.jsonl", "against": "after.jsonl"}),
    )
    .expect("the reverse delta renders");
    assert!(
        reversed.contains("added:\n  crate::outer -> crate::other"),
        "{reversed}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A `-Zunpretty=mir` text dump cannot name its tree, and the delta says so
/// instead of presenting a cross-tree comparison as sound — the ordinary case,
/// not a broken one.
/// `-Zunpretty=mir` 文本转储说不出自己的树，delta 会如实说明，而不是把跨树比较说成可靠的——这是
/// 常态，不是坏掉。
#[test]
fn an_unidentified_artifact_makes_the_delta_say_what_it_cannot_rule_out() {
    let (directory, _) = root("delta-unidentified");
    std::fs::write(directory.join("after.mir"), AFTER_TEXT).expect("the edited dump");
    let report = mir(
        &directory,
        &json!({"path": "after.mir", "against": "dump.mir"}),
    )
    .expect("the delta renders");
    assert!(
        report.contains("unidentified (no snapshot record names the tree)"),
        "{report}"
    );
    assert!(
        report.contains("a comparison across two trees cannot be ruled out"),
        "{report}"
    );
    assert!(
        report.contains("delta: relations added 1 gone 1"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A snapshot of another tree is refused by name: a call-graph delta across two
/// trees would be a plausible, wrong answer.
/// 另一棵树的快照会被按名字拒绝：跨两棵树的调用图差异会是一个看似合理、实则错误的答案。
#[test]
fn a_delta_across_two_named_trees_is_refused() {
    let (directory, _) = root("delta-refused");
    emit(&directory, "dump.mir", "before.jsonl");
    let foreign = foreign_snapshot();
    std::fs::write(directory.join("foreign.jsonl"), &foreign).expect("the foreign snapshot");

    let report = mir(
        &directory,
        &json!({"path": "foreign.jsonl", "against": "before.jsonl"}),
    )
    .expect("the refusal is an answer, not a tool failure");
    assert!(
        report.contains("REFUSED: the two artifacts describe different trees"),
        "{report}"
    );
    assert!(report.contains("somewhere-else"), "{report}");
    assert!(report.contains("mcp-mir"), "{report}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Re-emitting another tree's artifact here would relabel a tree that is not
/// this package's, so the writer refuses instead of minting a false snapshot.
/// 在这里重新输出另一棵树的 artifact 会给一棵不属于本包的树换标签，因此写入方拒绝，而不是铸出
/// 一份假快照。
#[test]
fn re_emitting_another_trees_snapshot_is_refused() {
    let (directory, _) = root("relabel");
    std::fs::write(directory.join("foreign.jsonl"), foreign_snapshot())
        .expect("the foreign snapshot");
    let error = mir(&directory, &json!({"path": "foreign.jsonl", "jsonl": true}))
        .expect_err("relabelling is refused");
    assert!(error.contains("would mislabel it"), "{error}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// The merge joins this package's trace with this package's tree, so a MIR
/// snapshot naming another tree is refused rather than merged.
/// 合并把本包的 trace 与本包的树接起来，因此点名另一棵树的 MIR 快照会被拒绝，而不是被合并。
#[test]
fn a_merge_with_a_foreign_snapshot_is_refused() {
    let (directory, _) = root("merge-refused");
    record_trace(&directory);
    std::fs::write(directory.join("foreign.jsonl"), foreign_snapshot())
        .expect("the foreign snapshot");
    let report = unified(&directory, &json!({"path": "foreign.jsonl"}))
        .expect("the refusal is an answer, not a tool failure");
    assert!(
        report.contains("REFUSED: the MIR artifact describes a different tree"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// Emitting and comparing are two different requests; asking for both is refused
/// rather than silently answering one of them.
/// 输出与比较是两个不同的请求；同时要两者会被拒绝，而不是默默回答其中一个。
#[test]
fn asking_to_emit_and_compare_at_once_is_refused() {
    let (directory, _) = root("both");
    let error = mir(
        &directory,
        &json!({"path": "dump.mir", "jsonl": true, "against": "dump.mir"}),
    )
    .expect_err("one request at a time");
    assert!(error.contains("ask for one of them"), "{error}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A hand-written snapshot of a tree that is not this package's.
/// 一份手写的、属于另一棵树的快照。
fn foreign_snapshot() -> String {
    format!(
        "{{\"kind\":\"snapshot\",\"namespace\":\"somewhere-else\",\"root\":\"{}\"}}\n{{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}}\n",
        xirang_kernel::root_node_id("somewhere-else")
    )
}

/// The merge itself: the call the trace confirms is `Live`, the call it never made
/// stays a compiler candidate — and the count line says both.
/// 合并本身：trace 确认的那次调用是 `Live`，它从未发生的那次仍是编译器候选——而计数行把两者都说出来。
#[test]
fn the_merge_confirms_a_live_call_and_keeps_the_rest_as_candidates() {
    let (directory, _) = root("merged");
    record_trace(&directory);
    let report = unified(&directory, &json!({"path": "dump.mir"})).expect("the merge renders");
    assert!(
        report.contains("relations 2 (live 1, compiler candidates 1)"),
        "{report}"
    );
    assert!(
        report.contains("crate::outer -> crate::inner  evidence=Live"),
        "{report}"
    );
    assert!(
        report.contains("crate::outer -> crate::other  evidence=Mir"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// Without a trace the merge still answers, and it says what that means: every
/// relation is a compiler candidate.
/// 没有 trace 时合并仍然作答，并说明这意味着什么：每条关系都是编译器候选。
#[test]
fn a_missing_trace_degrades_to_candidates_rather_than_failing() {
    let (directory, _) = root("no-trace");
    let report = unified(&directory, &json!({"path": "dump.mir"})).expect("the merge renders");
    assert!(report.contains("trace none"), "{report}");
    assert!(report.contains("compiler candidates 2"), "{report}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Matching function names from a foreign run cannot confirm this project's call chain.
/// 外来运行的同名函数不能确认本项目的调用链；当前与对照 trace 都必须核验身份。
#[test]
fn foreign_trace_identity_cannot_confirm_either_side_of_a_chain() {
    for baseline in [false, true] {
        for namespace_mismatch in [false, true] {
            let (directory, _) = root("foreign-trace");
            record_trace(&directory);
            let path = trace_artifact_path(&directory);
            let (mut artifact, _) = crate::run_method::read_trace_artifact(&path).unwrap();
            if namespace_mismatch {
                artifact.namespace = "another-host".into();
            } else {
                artifact.root = xirang_kernel::root_node_id("another-host");
            }
            let arguments = if baseline {
                std::fs::write(
                    directory.join("foreign.trace"),
                    artifact.render_trace_artifact(),
                )
                .unwrap();
                json!({"path":"dump.mir", "against":"dump.mir", "against_trace":"foreign.trace"})
            } else {
                std::fs::write(path, artifact.render_trace_artifact()).unwrap();
                json!({"path":"dump.mir"})
            };
            let error = unified(&directory, &arguments)
                .expect_err("foreign trace must not manufacture Live evidence");
            assert!(
                error.contains("REFUSED") && error.contains("trace"),
                "{error}"
            );
            let _ = std::fs::remove_dir_all(directory);
        }
    }
}

/// The baseline trace follows the same project-relative path rule as the MIR input.
/// 对照 trace 和 MIR 输入遵守相同的项目内路径规则。
#[test]
fn a_baseline_trace_cannot_escape_through_parent_components() {
    let (directory, _) = root("baseline-containment");
    record_trace(&directory);
    let (outside, _) = root("outside-baseline");
    record_trace(&outside);
    let relative = format!(
        "../{}/{}",
        outside.file_name().unwrap().to_str().unwrap(),
        trace_artifact_path(&outside)
            .strip_prefix(&outside)
            .unwrap()
            .display()
    );
    let error = unified(
        &directory,
        &json!({"path":"dump.mir", "against":"dump.mir", "against_trace":relative}),
    )
    .expect_err("a baseline outside the project is refused");
    assert!(error.contains("must stay inside"), "{error}");
    let _ = std::fs::remove_dir_all(directory);
    let _ = std::fs::remove_dir_all(outside);
}

/// A missing artifact and a path that escapes the root are both refused with the way
/// out, because "produce a dump with nightly" is the answer an agent needs.
/// 缺失的 artifact 与逃出根目录的路径都会被拒绝并给出出路，因为"用 nightly 产出一份转储"正是代理
/// 需要的答案。
///
/// The escaping case writes a real file *outside* the root, because existence is
/// answered before containment (canonicalization cannot judge a path that is not
/// there). Pointing at `../../etc/passwd` instead made this test platform-dependent:
/// under Linux's `/tmp` that path exists and the containment check refuses it, while
/// under macOS's `/var/folders/…` and Windows' temp directory nothing is there, so the
/// existence check refuses first with a different sentence. The security claim is
/// about an artifact that *is* there — an outside file must not be read — so the
/// fixture puts one there.
/// 逃逸那一条会在根**之外**写一个真实文件，因为存在性先于归属作答（规范化无法判断一个不存在的
/// 路径）。改用 `../../etc/passwd` 会让这条测试依赖平台：Linux 的 `/tmp` 下该路径存在、由归属检查
/// 拒绝，而 macOS 的 `/var/folders/…` 与 Windows 的临时目录下什么都不存在、由存在性检查先以另一句
/// 话拒绝。安全主张针对的是**确实存在**的 artifact——根之外的文件不得被读取——因此夹具自己放一个。
#[test]
fn a_missing_artifact_and_an_escaping_path_are_refused() {
    let (directory, _) = root("refused");
    let missing = mir(&directory, &json!({"path": "nope.mir"})).expect_err("missing is refused");
    assert!(missing.contains("Zunpretty=mir"), "{missing}");

    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a fixture directory name")
        .to_owned();
    let outside = directory.with_file_name(format!("{name}-outside"));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("the outside directory");
    std::fs::write(outside.join("secret.mir"), MIR_TEXT).expect("the outside artifact");
    let escaping = mir(
        &directory,
        &json!({"path": format!("../{name}-outside/secret.mir")}),
    )
    .expect_err("an escaping path is refused");
    assert!(escaping.contains("must stay inside"), "{escaping}");

    let _ = std::fs::remove_dir_all(&outside);
    let _ = std::fs::remove_dir_all(&directory);
}

/// A text file with no MIR in it is not minted into a snapshot. The header is the only credential
/// saying which tree an artifact describes, and stamping arbitrary prose produced a
/// "this package, zero calls" artifact that `delta`/`unified` then trust as fact — the source was
/// provable and became forgeable (audit `LGC-LG-18`).
/// 不含 MIR 的文本文件不会被造成一份快照。表头是唯一说明 artifact 描述哪棵树的凭据，而给任意散文
/// 盖上它，会产生一份"本包、零调用"的 artifact，`delta`/`unified` 随后当成事实采信——来源本可证明，
/// 于是变成了可伪造（审计 `LGC-LG-18`）。
#[test]
fn a_text_file_with_no_mir_in_it_is_not_stamped_into_a_snapshot() {
    let (directory, _) = root("not-mir");
    std::fs::write(
        directory.join("notes.txt"),
        "This is not a MIR dump.\nJust prose.\n",
    )
    .expect("prose");
    let error = mir(&directory, &json!({"path": "notes.txt", "jsonl": true}))
        .expect_err("prose carries no tree to name");
    assert!(error.contains("no MIR function"), "{error}");
    // The lenient read still answers: a dump that happens to contain no call is not an error.
    // 宽松的读取仍然作答：恰好不含调用的转储不是错误。
    let reply = mir(&directory, &json!({"path": "notes.txt"})).expect("the read answers");
    assert!(reply.contains("functions 0"), "{reply}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Two chains compared at the evidence level: the relation only the second build makes, the one
/// only the first makes, and the pair both carry with a **different** evidence label — which is
/// the bucket `mir --against` cannot express, because it compares edges rather than what
/// supports them.
/// 两条链在证据层面上的比较：只有第二次构建才有的关系、只有第一次才有的关系，以及两边都有**证据标签
/// 不同**的那一对——正是 `mir --against` 表达不了的那个桶，因为它比的是边、而不是支持边的东西。
#[test]
fn a_chain_comparison_names_both_sides_and_the_evidence_that_differs() {
    let (directory, _) = root("chain");
    std::fs::write(directory.join("dump.mir"), MIR_TEXT).expect("first dump");
    std::fs::write(directory.join("after.mir"), AFTER_TEXT).expect("second dump");
    // The package's own trace confirms `outer -> inner`; a second artifact confirms
    // `outer -> new` instead, so the two sides disagree about both of those pairs.
    // 本包自己的 trace 确认 `outer -> inner`；第二份 artifact 改为确认 `outer -> new`，于是两边对这两对
    // 的判断都不同。
    record_trace(&directory);
    let mut second = CallTrace::full();
    let outer =
        xirang_kernel::identity::NodeId::from_namespaced_path("mcp-mir", "outer/outer.rs", "Outer");
    let new = xirang_kernel::identity::NodeId::from_namespaced_path("mcp-mir", "new/new.rs", "New");
    second.with(outer, "crate::outer", |trace| {
        trace.with(new, "crate::new", |_| {});
    });
    write_trace_artifact(&second, &directory.join("second.jsonl"), "mcp-mir")
        .expect("the second trace writes");

    let report = unified(
        &directory,
        &json!({"path": "after.mir", "against": "dump.mir", "against_trace": "second.jsonl"}),
    )
    .expect("the comparison answers");
    // `only here` is empty, and that is the honest answer rather than a missing row: the second
    // side's trace *ran* `outer -> new`, and a live call is evidence of a relation the compiler's
    // dump never mentioned — so both sides carry it and the difference shows up as a label, not
    // as an extra edge. Only `outer -> other` is one-sided, because nothing but the MIR dump
    // ever claims it.
    // `only here` 为空，而这是诚实的答案而不是缺行：第二侧的 trace **实跑过** `outer -> new`，而实跑
    // 就是关系的证据——编译器转储从没提过它——因此两边都有它，差别表现在标签上而不是多出一条边。
    // 只有 `outer -> other` 是单侧的，因为除了 MIR 转储没有东西声称它。
    assert!(report.contains("only here 0"), "{report}");
    assert!(report.contains("only there 1"), "{report}");
    assert!(report.contains("crate::outer -> crate::other"), "{report}");
    assert!(report.contains("evidence differs 2"), "{report}");
    assert!(
        report.contains("crate::outer -> crate::inner  here=Live there=Mir"),
        "{report}"
    );
    assert!(
        report.contains("crate::outer -> crate::new  here=Mir there=Live"),
        "{report}"
    );
    // Sharing one trace leaves a single bucket empty and says so rather than inventing a row.
    // 共用一份 trace 会让某个桶为空，而报告如实说空，而不是编出一行。
    let shared = unified(
        &directory,
        &json!({"path": "after.mir", "against": "dump.mir"}),
    )
    .expect("the comparison answers");
    assert!(shared.contains("evidence differs 0"), "{shared}");
    assert!(
        shared.contains("shared: this side's trace"),
        "the report says which trace it used: {shared}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}
