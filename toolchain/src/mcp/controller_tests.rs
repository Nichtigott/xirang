//! Pins for the approval store: where it lives, what it will not call "no approval", and who may write it.
//! 批准存储的钉子：它住在哪里、它不会把什么叫做"没有批准"，以及谁可以写它。

use super::{active, directory, issue};
use std::path::Path;

/// A throwaway managed root, with a sibling controller directory that the test cleans up.
/// 一个一次性被管理根，带有测试会清理的兄弟控制器目录。
fn root(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-controller-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(directory(&root));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn fields(scope: &str, candidate: &str) -> Vec<(String, String)> {
    [
        ("budget", "executions=8"),
        ("candidate", candidate),
        ("operation", "land"),
        ("policy", crate::mcp::candidate::POLICY),
        ("project", "controller-probe"),
        ("proposal", "1"),
        ("revision", "1"),
        ("scope", scope),
        ("task", "candidate"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect()
}

/// The store is a **sibling** of the managed root, keyed by the root, so two projects never share one and an
/// approval can never be read as a file of the project it approves.
/// 存储是被管理根的**兄弟**，以根作键，因此两个项目永不共用一份，而一份批准也永远不会被读成它所批准的项目的文件。
#[test]
fn the_store_lives_outside_the_managed_tree_and_never_collides() {
    let root = root("outside");
    let store = directory(&root);
    assert!(
        !store.starts_with(&root),
        "the approval store must not be inside the tree it approves: {}",
        store.display()
    );
    assert_eq!(
        store
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some(".xirang-controller"),
        "it is one directory beside the roots, not one per arbitrary place: {}",
        store.display()
    );
    let sibling = root.with_file_name(format!(
        "{}-other",
        root.file_name().unwrap().to_string_lossy()
    ));
    assert_ne!(
        directory(&root),
        directory(&sibling),
        "two roots under one parent never share an approval"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The legal control: an approval the controller writes is the approval the bridge reads, with every dimension
/// intact — and it covers a use inside its scope.
/// 正控制：控制器写下的批准就是桥读到的那份批准，每个维度都在——而且它覆盖范围之内的使用。
#[test]
fn a_written_approval_is_the_approval_that_is_read() {
    let root = root("roundtrip");
    assert!(
        active(&root).unwrap().is_none(),
        "a root nobody approved has no approval"
    );
    let issued = issue(
        &root,
        fields("src", "574196-1-0"),
        "maintainer",
        "interactive-cli",
    )
    .expect("the controller writes the approval");
    let read = active(&root)
        .unwrap()
        .expect("the bridge reads the approval back");
    assert_eq!(issued, read, "the same dimensions, the same grant");
    assert_eq!(read.field("approver"), Some("maintainer"));
    assert_eq!(read.field("channel"), Some("interactive-cli"));
    assert_eq!(read.field("scope"), Some("src"));
    assert_eq!(
        read.field("budget"),
        Some("executions=8"),
        "the budget is one of the dimensions, not a comment"
    );
    assert!(
        super::line(&read).contains("approved by maintainer through interactive-cli"),
        "a reply can say who approved what, and through which channel: {}",
        super::line(&read)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A store that exists and cannot be read is an **error**, not "no approval": folding the two would make a
/// corrupted or edited store indistinguishable from a project nobody has approved.
/// 存在但读不出来的存储是**错误**，不是"没有批准"：把两者折起来，会让损坏或被改过的存储与"没人批准过的项目"分不出来。
#[test]
fn an_unreadable_or_edited_store_is_an_error_rather_than_an_absent_approval() {
    let root = root("broken");
    let path = directory(&root).join("grant.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ not json").unwrap();
    let refusal = active(&root).expect_err("a broken store is not an absence");
    assert!(
        refusal.contains("grant.json") && refusal.contains("not readable JSON"),
        "the refusal names the file and the reason: {refusal}"
    );
    // A store whose stored digest no longer describes its dimensions was edited after it was issued.
    // 存下来的摘要不再描述它的那些维度，说明它在签发之后被改过。
    std::fs::write(
        &path,
        // `files=1` is a **readable** budget, so the store reaches the digest check this test is about; a budget the
        // checker cannot read is refused earlier and by its own name (measured in
        // `candidate_tests::an_unreadable_budget_is_refused_rather_than_treated_as_unlimited`).
        // `files=1` 是**读得懂**的预算，因此这份存储会走到本条测试真正关心的摘要检查；读不懂的预算会更早、以自己的名义
        // 被拒（实测见 `candidate_tests::an_unreadable_budget_is_refused_rather_than_treated_as_unlimited`）。
        r#"{"fields":[["approver","maintainer"],["budget","files=1"],["candidate","c"],
            ["channel","interactive-cli"],["operation","land"],["policy","p"],
            ["project","p"],["proposal","1"],["revision","1"],["scope","src"],["task","candidate"]],
            "digest":"0000"}"#,
    )
    .unwrap();
    let refusal = active(&root).expect_err("an edited digest is refused");
    assert!(
        refusal.contains("edited after it was issued"),
        "and says so in those words: {refusal}"
    );
    // And a store that is simply absent is the clean `None`, not an error.
    // 而根本不存在的存储是干净的 `None`，不是错误。
    std::fs::remove_dir_all(directory(&root)).unwrap();
    assert!(active(&root).unwrap().is_none());
    std::fs::remove_dir_all(root).unwrap();
}

/// **No tool in this bridge can issue an approval.** The bridge's one-shot client and its tool handlers are
/// scanned as text, because the property is about which code can reach [`issue`] at all — a rule that lived in
/// prose would be re-broken by the next helpful handler.
/// **这座桥里没有任何工具能签发批准。** 桥的一次性客户端与它的工具处理函数被当作文本扫描，因为这条性质关乎"哪段代码
/// 够得到 [`issue`]"，而只活在散文里的规则会被下一个"帮个忙"的处理函数重新破坏。
///
/// The allowance list is the point: exactly one file may name [`issue`] — the interactive approval front end — and
/// the day a second one appears this fails and names it.
/// 允许清单是要点：**恰好**一个文件可以点名 [`issue`]——交互式批准前端——第二天冒出第二个，这条就红并点名它。
#[test]
fn no_tool_in_this_bridge_can_issue_an_approval() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mcp");
    let allowed = ["approval.rs", "controller.rs"];
    let mut writers = Vec::new();
    for entry in std::fs::read_dir(&sources).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        // Test-only files are out of scope and the reason is structural rather than convenient: a `_tests.rs` file
        // is compiled only under `#[cfg(test)]`, is never reached from `run_tool`, and in these two cases *is* the
        // controller (the fixture issues the approval a person would). The repository's `test_shape` gate keeps
        // test-only code in test-only files, so this exclusion cannot hide a handler.
        // 仅测试的文件不在判定范围内，理由与"图省事"无关：`_tests.rs` 只在 `#[cfg(test)]` 下编译，永远从 `run_tool`
        // 到不了，而这两处恰恰**就是**控制器（夹具签发的是一个人会签发的那份批准）。仓库的 `test_shape` 门禁把仅测试的
        // 代码留在仅测试的文件里，因此这条排除藏不住处理函数。
        if name.ends_with("_tests.rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if text.contains("controller::issue(") && !allowed.contains(&name.as_str()) {
            writers.push(name);
        }
    }
    assert!(
        writers.is_empty(),
        "only the interactive approval front end may write an approval; these also tried: {writers:?}"
    );
    // And the front end is the one place a decision enters. The maintainer's correction of 2026-10-18
    // removed the terminal check and the typed name: **neither was evidence of a person**, and what the
    // requirement serves is that an overrun is explained and the user decides whether the boundary grows.
    // So the tombstone is the other way round — this file must keep being the only writer, and it must not
    // grow the old evidence back.
    // 而前端是决定进入的唯一地方。维护者 2026-10-18 的纠正去掉了终端检查与手输姓名：**两者都不是人的证据**，
    // 而这条要求服务的是"越权要说明原因、由使用方决定边界是否扩大"。因此墓碑反了过来——这个文件必须继续是唯一的
    // 写入者，而且不许把旧证据重新长回来。
    let front_end = std::fs::read_to_string(sources.join("approval.rs")).unwrap();
    assert!(
        front_end.contains("controller::issue(") && front_end.contains("fn confirm("),
        "the confirmation entry has to be the one that writes the decision, or the pin above proves nothing"
    );
    assert!(
        !front_end.contains("is_terminal"),
        "a terminal check is not evidence that a person approved (a process can allocate a pseudo-terminal) and it must not come back"
    );
    assert!(
        !front_end.contains("read_line"),
        "a typed name is a declaration, not an approval; it must not come back as the evidence"
    );
}

/// The store's digest is a **corruption check, not a signature** — said out loud, and measured here rather than left to
/// the module doc, because it is the one boundary a reader most easily over-reads: the file sits outside the root, it
/// is called "the controller's", and it holds the thing that authorises a landing.
/// 存储的摘要是**防损坏检查、不是签名**——这里把它说出来并量出来，而不是只写在模块文档里，因为这是读者最容易读过头的那条
/// 边界：文件在根之外、被称为"控制器的"、里面装着授权落地的东西。
///
/// What protects the channel is **who writes it**, not cryptography: `issue` has exactly one caller (the interactive
/// front end), and the pin `no_tool_in_this_bridge_can_issue_an_approval` keeps it that way. A process that can write
/// arbitrary files can bypass that — and it could equally write the base tree, which is why the plain-MCP capability
/// line names both.
/// 保护这条通道的是**谁来写**，不是密码学：`issue` 只有一个调用方（交互式前端），而钉子
/// `no_tool_in_this_bridge_can_issue_an_approval` 保持着这一点。能写任意文件的进程可以绕过它——而它同样能直接写基树，
/// 这就是普通 MCP 的能力行把两者一起点名原因。
#[test]
fn a_hand_written_store_is_accepted_because_the_digest_is_not_a_signature() {
    let root = std::env::temp_dir().join(format!("xirang-controller-forge-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let directory = super::directory(&root);
    std::fs::create_dir_all(&directory).unwrap();
    // The same shape `issue` writes, assembled by hand with a self-consistent digest.
    // 与 `issue` 写下的形状相同，只是手工拼出来、摘要自洽。
    let fields = vec![
        ("approver".to_owned(), "somebody".to_owned()),
        ("budget".to_owned(), "executions=1000".to_owned()),
        ("candidate".to_owned(), "forged".to_owned()),
        ("channel".to_owned(), "interactive-cli".to_owned()),
        ("operation".to_owned(), "land".to_owned()),
        ("policy".to_owned(), "forged".to_owned()),
        ("project".to_owned(), "candidate-probe".to_owned()),
        ("proposal".to_owned(), "1".to_owned()),
        ("revision".to_owned(), "9".to_owned()),
        ("scope".to_owned(), "src".to_owned()),
        ("task".to_owned(), "candidate".to_owned()),
    ];
    let grant =
        xirang_kernel::maintenance::Grant::issue(fields).expect("a legal set of dimensions");
    let body = serde_json::json!({"fields": grant.fields(), "digest": grant.digest()});
    std::fs::write(
        directory.join("grant.json"),
        serde_json::to_vec_pretty(&body).unwrap(),
    )
    .unwrap();
    let read = super::active(&root)
        .expect("the store reads")
        .expect("one approval");
    assert_eq!(
        read.field("approver"),
        Some("somebody"),
        "the hand-written store is accepted: the digest only proves the fields were not edited afterwards"
    );
    // What **is** refused: a store whose digest does not describe its dimensions (an edited file).
    // **会**被拒的是：摘要不描述其维度的存储（被编辑过的文件）。
    let mut edited = body.clone();
    edited["fields"][0][1] = serde_json::json!("somebody else");
    std::fs::write(
        directory.join("grant.json"),
        serde_json::to_vec_pretty(&edited).unwrap(),
    )
    .unwrap();
    let refused = super::active(&root).expect_err("an edited store is refused");
    assert!(
        refused.contains("edited after it was issued"),
        "the corruption check still works: {refused}"
    );
    let _ = std::fs::remove_dir_all(root);
}
