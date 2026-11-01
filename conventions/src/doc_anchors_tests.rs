//! The anchor gate's tests, in their own file so the gate stays inside the line ceiling —
//! the split `release_workflow_tests.rs` and `doc_blocks_tests.rs` record.
//! 锚点门禁的测试，放在独立文件里使门禁留在行数上限之内——与 `release_workflow_tests.rs`、
//! `doc_blocks_tests.rs` 记录的同一种拆分。

use super::*;

/// A throwaway checkout with the given documents and sources.
/// 一个只含给定文档与源码的一次性检出。
fn synthetic(documents: &[(&str, &str)], sources: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("xirang-anchors-{}-{sequence}", std::process::id()));
    for (relative, contents) in sources.iter().chain(documents) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        fs::write(&path, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}

/// A line number can stay inside the file while the code moves under it: the paired
/// token is what makes the number checkable. Both halves are pinned — a drifted
/// reference is reported, and a reference whose token is on the named line is not.
/// 代码在下面移动时行号仍可能留在文件内：成对写出的 token 才是让行号可被检查的东西。两侧都
/// 钉住——漂移的引用被报出，token 正好在它点名那一行的引用不被报。
#[test]
fn a_paired_token_that_is_not_on_the_named_line_is_reported() {
    let sources = [("kernel/src/probe.rs", "fn first() {}\nfn second() {}\n")];
    let drifted = synthetic(
        &[("docs/note.md", "`second` (`kernel/src/probe.rs:1`)\n")],
        &sources,
    );
    let found = findings(&drifted);
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("drifted")),
        "a token that is not on the named line is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&drifted);

    let honest = synthetic(
        &[("docs/note.md", "`second` (`kernel/src/probe.rs:2`)\n")],
        &sources,
    );
    let found = findings(&honest);
    assert!(
        found.is_empty(),
        "the same reference with the right number stays clean: {found:#?}"
    );
    let _ = fs::remove_dir_all(&honest);
}

/// A path written from a crate root, and a line number that is not a line number. Both
/// spellings were silent: the missing-file check asked whether the *first* segment was a
/// workspace root directory (so `src/definitely-gone.rs` was never asked), and `:0`
/// passed the past-the-end comparison because zero never is.
/// 一处从 crate 根写起的路径，以及一个不是行号的行号。两种拼法过去都是沉默的：缺失文件检查问的
/// 是**第一段**是否为工作区根目录（因此 `src/definitely-gone.rs` 从未被问），而 `:0` 能通过
/// "越过末尾"那次比较，因为 0 永远不会越过。
#[test]
fn a_crate_relative_path_and_a_zero_line_are_reported() {
    let crate_relative = synthetic(
        &[("docs/note.md", "see `src/definitely-gone.rs:99`\n")],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&crate_relative);
    assert!(
        found.iter().any(|finding| finding.reason == "no such file"),
        "a path written as a path is checked wherever it starts: {found:#?}"
    );
    let _ = fs::remove_dir_all(&crate_relative);

    let zero = synthetic(
        &[("docs/note.md", "see `kernel/src/probe.rs:0`\n")],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&zero);
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("1-based")),
        "line 0 is not a line: {found:#?}"
    );
    let _ = fs::remove_dir_all(&zero);
}

/// An anchor into a manifest or a workflow is a documented promise like any `.rs:NNN`,
/// and it used to be invisible: the scan keyed on `.rs:` alone, so a citation such as
/// `.github/workflows/release.yml:9` was never asked whether it still points anywhere.
/// 指向清单或工作流的锚点与任何 `.rs:NNN` 一样是一处文档承诺，而它过去是不可见的：扫描只以
/// `.rs:` 为触发子串，因此像 `.github/workflows/release.yml:9` 这样的引用从未被问及是否还指向
/// 某处。
#[test]
fn a_workflow_anchor_past_the_end_is_reported() {
    let root = synthetic(
        &[(
            "docs/note.md",
            "the guard is `.github/workflows/release.yml:9`\n",
        )],
        &[
            ("kernel/src/probe.rs", "fn probe() {}\n"),
            (".github/workflows/release.yml", "on:\n  push:\n"),
        ],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].anchor, ".github/workflows/release.yml:9");
    assert!(found[0].reason.contains("2 lines"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The other half of that widening: a manifest anchor that resolves stays clean, and the
/// file kinds still outside the gate are left alone rather than reported as missing — a
/// markdown target names prose, and an extensionless tool script has no suffix to key on.
/// 那次扩面的另一半：能解析的清单锚点保持干净，而仍在门禁之外的几类文件被放过、而不是报成缺失
/// ——markdown 目标命名的是散文，无扩展名的工具脚本没有可作判据的后缀。
#[test]
fn a_manifest_anchor_resolves_and_other_kinds_stay_out() {
    let root = synthetic(
        &[(
            "docs/note.md",
            "the members list is `Cargo.toml:2`, prose is `docs/note.md:9`, \
             the tool is `tools/xirang-publish:88`\n",
        )],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&root);
    assert!(found.is_empty(), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A path that climbs with `..` is the path it resolves to, not a string that matches
/// nothing: `declaration/../../tree/probe.rs` names `kernel/src/probe.rs`, and calling that
/// "no such file" is the false positive that gets a gate switched off.
/// 用 `..` 向上爬的路径就是它解析到的那个路径，而不是一个什么都匹配不到的字符串：
/// `declaration/../../tree/probe.rs` 命名的就是 `kernel/src/probe.rs`，说它 "no such file"
/// 正是那种让人把门禁关掉的假阳性。
#[test]
fn a_climbing_path_resolves_to_the_file_it_names() {
    let root = synthetic(
        &[
            ("docs/note.md", "see `declaration/../../src/probe.rs:1`\n"),
            ("kernel/src/registry_core/declaration/marker.rs", "// x\n"),
        ],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "a climbing path that names a real file is not a broken reference: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The reference this gate exists for: a line number past the end of the file.
/// 本门禁为之存在的引用：行号超出文件末尾。
#[test]
fn a_reference_past_the_end_of_a_file_is_reported() {
    let root = synthetic(
        &[(
            "docs/probe.md",
            "see `kernel/src/probe.rs:99` for the check\n",
        )],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].document, "docs/probe.md");
    assert_eq!(found[0].anchor, "kernel/src/probe.rs:99");
    assert!(found[0].reason.contains("1 lines"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A reference to a file that is gone is reported too: a rename is the other
/// way an anchor rots.
/// 指向已消失文件的引用同样被报出：改名是锚点腐化的另一种方式。
#[test]
fn a_reference_to_a_missing_file_is_reported() {
    let root = synthetic(
        &[("docs/probe.md", "moved to `kernel/src/gone.rs:1`\n")],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].reason, "no such file");
    let _ = fs::remove_dir_all(&root);
}

/// A bare name that two files share is not guessed at, and a bare name no file
/// owns is prose.
/// 两个文件共有的裸名不去猜；没有文件拥有的裸名是散文。
#[test]
fn an_ambiguous_or_unknown_bare_name_is_left_alone() {
    let root = synthetic(
        &[(
            "docs/probe.md",
            "`same.rs:99` is ambiguous, `elsewhere.rs:99` is not ours\n",
        )],
        &[
            ("kernel/src/same.rs", "fn a() {}\n"),
            ("toolchain/src/cli/same.rs", "fn b() {}\n"),
        ],
    );
    assert_eq!(findings(&root), Vec::new());
    let _ = fs::remove_dir_all(&root);
}

/// A bare name that is unique is checked, because it can be resolved without
/// guessing.
/// 唯一的裸名会被检查，因为它无需猜测即可解析。
#[test]
fn a_unique_bare_name_is_checked() {
    let root = synthetic(
        &[("docs/probe.md", "the arm is in `probe.rs:99`\n")],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].anchor, "probe.rs:99");
    let _ = fs::remove_dir_all(&root);
}

/// An audit document is a record: its anchors are as true as they were when it
/// was written, and the gate leaves it alone.
/// 审计文档是记录：它的锚点在写下时是真的，门禁不碰它。
#[test]
fn an_anchor_inside_a_record_is_exempt() {
    let root = synthetic(
        &[(
            "docs/audit-probe.md",
            "it used to be `kernel/src/probe.rs:99`\n",
        )],
        &[("kernel/src/probe.rs", "fn probe() {}\n")],
    );
    assert_eq!(findings(&root), Vec::new());
    let _ = fs::remove_dir_all(&root);
}

/// The documentation in this checkout resolves.
/// 本检出里的文档可解析。
#[test]
fn the_shipped_documentation_anchors_resolve() {
    let found = findings(&crate::workspace_root());
    assert!(
        found.is_empty(),
        "a documented anchor no longer points anywhere; fix the reference or the \
         sentence that needs it: {found:#?}"
    );
}

/// Raw output lives in fenced blocks, and a fenced block's `path:line` is a **citation**.
/// 原始输出住在围栏块里，而围栏块里的 `path:line` 是**引文**。
///
/// Recording a command's raw output and keeping every anchor resolving are two rules this repository
/// holds at once, and they collided the first time a document quoted a compiler error: the error's
/// `--> …/out/generated_lib.rs:182:1` names a build artifact, and a build diagnostic's
/// `source=board/object/tile2/tile2.rs:11` names a file outside this checkout. Both were reported as
/// stale anchors (audit `M7`, §M7.59).
/// "记录命令的原始输出"与"保持每个锚点可解析"是本仓库同时持有的两条规则，而它们在一份文档第一次引用编译器
/// 错误时就撞上了：错误里的 `--> …/out/generated_lib.rs:182:1` 点名的是一件构建产物，构建诊断里的
/// `source=board/object/tile2/tile2.rs:11` 点名的是检出之外的文件。两者都被报成失效锚点（审计 `M7`，§M7.59）。
#[test]
fn a_citation_inside_a_fenced_block_is_not_an_anchor() {
    let sources = [("kernel/src/probe.rs", "fn first() {}\n")];
    let quoted = synthetic(
        &[(
            "docs/quote.md",
            "The build said:\n\n```text\n--> …/out/generated_lib.rs:182:1\n\
             |   source=board/object/tile2/tile2.rs:11\n```\n\nand that was all.\n",
        )],
        &sources,
    );
    assert!(
        findings(&quoted).is_empty(),
        "a fenced citation names artifacts and files the checkout does not have: {:?}",
        findings(&quoted)
    );

    // The other half: the same reference in prose is still the rename-and-forget shape the gate exists
    // for. A gate that stopped checking prose would be a gate that stopped guarding.
    // 另一半：同一个引用写在正文里，仍然是门禁为之存在的"改名后忘记更新"的形状。一道不再检查正文的门禁，
    // 就是一道不再守任何东西的门禁。
    let prose = synthetic(
        &[(
            "docs/prose.md",
            "See `src/definitely-gone.rs:99` for the old shape.\n",
        )],
        &sources,
    );
    let found = findings(&prose);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].reason, "no such file");
}
