//! Pins for the candidate surface: what it refuses, and what it never writes.
//! 候选执行面的钉子：它拒绝什么，以及它从不写什么。
//!
//! Every pin here is about a **refusal or an absence**, because those are the two things a managed
//! surface is worth: a happy path that lands a change proves the plumbing, and none of these do.
//! 这里的每条钉子都是关于一次**拒绝**或一次**缺席**的，因为那正是托管执行面值钱的两件事：一条落地改动的
//! 顺利路径只证明管道通了，而这里一条都不测那个。

use super::candidate;
use serde_json::{Value, json};
use std::path::PathBuf;

/// A package with one trivial test, so `verify` has something to run.
/// 一个带一条平凡测试的包，好让 `verify` 有东西可跑。
pub(super) fn fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-candidate-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    // Every candidate this fixture opens lands, and landing needs a person's confirmation of the
    // scope — so the tree carries the lease that provides one.
    // 这个夹具开的每棵候选都会落地，而落地需要**人**对范围的确认——因此树里带着提供那个人的租约。
    std::fs::create_dir_all(root.join(".xirang/adopted")).unwrap();
    std::fs::write(
        root.join(".xirang/adopted/entries"),
        "control.object.button|the provider is the shared route|converge answer|maintainer|2026-10-18|src/lib.rs|deadbeef|because\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='candidate-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 41);\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("NOTES.txt"), "outside the scope\n").unwrap();
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// Open a candidate whose scope covers `src`, and hand back its id and its tree.
/// 打开一个范围覆盖 `src` 的候选，交回它的 id 与它的树。
pub(super) fn opened(root: &std::path::Path, scope: Value) -> (String, PathBuf) {
    let reply = candidate(
        root,
        &json!({"action":"open","scope":scope,"authority":"control.object.button"}),
    )
    .unwrap();
    let id = reply
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("candidate "))
        .and_then(|rest| rest.strip_suffix(" open"))
        .unwrap()
        .to_owned();
    let id = id.split_whitespace().next().unwrap().to_owned();
    let tree = reply
        .lines()
        .find_map(|line| line.strip_prefix("tree "))
        .unwrap();
    let scope: Vec<String> = scope
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    approve(root, &id, &scope);
    (id, PathBuf::from(tree))
}

/// The approval a landing needs, written by the controller this test stands in for.
/// 落地所需的那份批准，由这条测试所代表的控制器写下。
///
/// It is written through the **real** controller store ([`crate::mcp::controller::issue`]), not injected into the
/// record: the point of the gate is that the approval lives outside the candidate and outside the request, so a
/// fixture that put it inside either one would be testing nothing.
/// 它经**真实**的控制器存储（[`crate::mcp::controller::issue`]）写下，而不是注入记录：这道门的要点正是批准住在候选之外、
/// 请求之外，因此把批准放进其中任一的夹具什么都没测。
pub(super) fn approve(root: &std::path::Path, id: &str, scope: &[String]) {
    approve_operations(root, id, &scope.join(","), "land", "1");
}

/// The tool surface cannot record a verdict: the action is gone from the catalogue **and** from the
/// schema, and the host entry is the only writer. A tool the developer can call is a tool that can
/// write its own pass, so this is a capability pin, not a wording pin.
/// 工具面不能记录判定：动作既不在目录里，也不在 schema 里，唯一的写入者是宿主入口。开发者调得到的工具就是
/// 能写自己"通过"的工具，所以这是一条**能力**钉子，不是措辞钉子。
#[test]
fn no_tool_action_records_a_review_verdict() {
    assert!(
        !super::ACTIONS.contains(&"review"),
        "a tool action that writes a verdict can write the developer's own pass: {:?}",
        super::ACTIONS
    );
    let schema = super::schema();
    let mut actions: Vec<&str> = schema["properties"]["action"]["enum"]
        .as_array()
        .expect("the action enum is part of the schema")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let mut declared = super::ACTIONS.to_vec();
    actions.sort_unstable();
    declared.sort_unstable();
    assert_eq!(
        actions, declared,
        "the catalogue and the schema must agree about which actions exist"
    );
    let refused = refused(
        &fixture(),
        json!({"action":"review","verdict":"pass","reviewer":"the author"}),
    );
    assert!(
        refused.contains("not a tool action"),
        "and the removed action says where the verdict went: {refused}"
    );
}

/// The directory the candidate records live in, for the root `candidate` canonicalises to.
/// 候选记录所在的目录——对应 `candidate` 会规范化的那个根。
pub(super) fn records(root: &std::path::Path) -> std::path::PathBuf {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .join("target/xirang/candidates")
}

/// The digest of the packet `status` prints — the value the host review entry records against.
/// `status` 打印的那份材料包的摘要——也就是宿主评审入口据以记录判定的那个值。
pub(super) fn packet_digest(root: &std::path::Path, id: &str) -> String {
    let record = super::Record::read_at(&records(root), id).unwrap();
    super::review::digest(&record).unwrap()
}

/// Record a verdict against the packet as it stands, the way the host review entry does.
/// 对照当前那份材料包记录判定，就像宿主评审入口那样。
pub(super) fn host_review(
    root: &std::path::Path,
    id: &str,
    verdict: &str,
    reviewer: &str,
) -> Result<String, String> {
    record_against(root, id, verdict, reviewer, &packet_digest(root, id))
}

/// The same, against a packet captured earlier — the stale-packet case.
/// 同一个，但对照更早捕获的那份材料包——过期材料包的情形。
pub(super) fn record_against(
    root: &std::path::Path,
    id: &str,
    verdict: &str,
    reviewer: &str,
    packet: &str,
) -> Result<String, String> {
    super::review::record(&records(root), id, verdict, Some(reviewer), &[], packet)
}

/// The same, with the operation set and revision spelled out — a widening's approval carries both verbs and a higher
/// revision, and the pins need to state that rather than inherit it.
/// 同一个，但把操作集合与版本写出来——扩大的批准同时带两个动词与更高的版本，钉子要把这件事写出来而不是继承它。
pub(super) fn approve_operations(
    root: &std::path::Path,
    id: &str,
    scope: &str,
    operations: &str,
    revision: &str,
) {
    issue_approval(root, id, scope, operations, revision, &[]);
}

/// The same approval with the named dimensions replaced, so a pin can state **exactly** which difference must refuse
/// the use instead of describing it in prose.
/// 同一个批准，但替换掉点名的维度，好让钉子说出**确切**是哪个差异必须拒绝这次使用，而不是用散文描述它。
pub(super) fn approve_replacing(root: &std::path::Path, id: &str, replace: &[(&str, &str)]) {
    issue_approval(root, id, "src", "land", "1", replace);
}

/// The nine dimensions, then the replacements. One field list, so a dimension cannot be bound in one helper and
/// forgotten in the other.
/// 九个维度，然后是替换。一份字段清单，因此一个维度不可能在一个夹具里被绑上、在另一个里被忘掉。
fn issue_approval(
    root: &std::path::Path,
    id: &str,
    scope: &str,
    operations: &str,
    revision: &str,
    replace: &[(&str, &str)],
) {
    let mut fields = vec![
        ("budget".to_owned(), "executions=8".to_owned()),
        ("candidate".to_owned(), id.to_owned()),
        ("operation".to_owned(), operations.to_owned()),
        ("policy".to_owned(), super::POLICY.to_owned()),
        (
            "project".to_owned(),
            crate::mcp::registry::namespace(root).unwrap(),
        ),
        ("proposal".to_owned(), "1".to_owned()),
        ("revision".to_owned(), revision.to_owned()),
        ("scope".to_owned(), scope.to_owned()),
        ("task".to_owned(), "candidate".to_owned()),
    ];
    for (name, value) in replace {
        match fields.iter_mut().find(|(field, _)| field == name) {
            Some((_, slot)) => *slot = (*value).to_owned(),
            None => panic!(
                "`{name}` is not a dimension this fixture binds; add it here so the pin states every one"
            ),
        }
    }
    crate::mcp::controller::issue(root, fields, "maintainer", "interactive-cli")
        .expect("the controller issues an approval for the fixture's candidate");
}

/// The refusal message of `candidate`, which must name the rule rather than the symptom.
/// `candidate` 的拒绝信息，必须点名规则而不是症状。
pub(super) fn refused(root: &std::path::Path, arguments: Value) -> String {
    candidate(root, &arguments).expect_err("this call must be refused")
}
/// Run the host's confirmation entry the way a host would: the decision, the version it is about, and
/// nothing else. The entry prints; the test reads the code and then asks `status` what changed.
/// 像宿主那样运行确认入口：决定、它所针对的版本，没有别的东西。入口打印；测试读返回码，然后问 `status` 什么变了。
pub(super) fn confirm(root: &std::path::Path, id: &str, decision: &[&str]) -> i32 {
    let mut arguments = vec![
        "--root".to_owned(),
        root.display().to_string(),
        "--id".to_owned(),
        id.to_owned(),
    ];
    arguments.extend(decision.iter().map(|value| (*value).to_owned()));
    crate::mcp::approval::confirm(&arguments)
}

/// The version a reply printed, so a test decides the exact request it read about.
/// 一条回复印出的版本，好让测试针对它真正读到的那份请求作决定。
pub(super) fn request_version(text: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix("widening request "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("the reply prints the request's version")
        .to_owned()
}

/// A managed surface's first property: **you cannot reach a write by omitting an argument.**
/// 托管执行面的第一条性质：**省略参数到达不了写。**
#[test]
fn there_is_no_default_action_and_the_scope_is_required() {
    let root = fixture();
    let missing = refused(&root, json!({"action":"open"}));
    assert!(missing.contains("needs `scope`"), "{missing}");
    let absent = refused(&root, json!({}));
    assert!(
        absent.contains("requires an explicit action") && absent.contains("no default"),
        "{absent}"
    );
    let typo = refused(&root, json!({"action":"opens","scope":["src"]}));
    assert!(
        typo.contains("not `opens`") && typo.contains("nothing was done"),
        "{typo}"
    );
    assert!(
        !root.join("target/xirang/candidates").exists(),
        "a refused call creates no candidate directory"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The base tree is not the thing the change happens in: `open` copies, and nothing the candidate
/// does afterwards reaches the base until `land` says so.
/// 改动发生的树不是基树：`open` 复制，而候选之后做的任何事都到不了基树，直到 `land` 这么说为止。
#[test]
fn opening_a_candidate_writes_nothing_into_the_base_tree() {
    let root = fixture();
    let before = std::fs::read(root.join("src/lib.rs")).unwrap();
    let (id, tree) = opened(&root, json!(["src"]));
    assert!(tree.starts_with(root.join("target")));
    assert_eq!(
        std::fs::read_to_string(tree.join("src/lib.rs")).unwrap(),
        String::from_utf8(before).unwrap()
    );
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(status.contains("state open"), "{status}");
    assert!(status.contains("nothing was written"), "{status}");
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 41);\n}\n",
        "the edit happened in the copy, and the base still has its own bytes"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The scope is enforced at the landing, and the refusal names every path that is outside it — the
/// base stays exactly as it was.
/// 范围在落地处被强制执行，拒绝会点名每一条落在范围外的路径——而基树保持原样。
/// The scope is enforced at the landing, and an overrun becomes a **request** rather than a wall: the
/// reply names every path outside the boundary, what changed there, what else shares those paths, the
/// alternatives, and the exact command that decides it — while the base tree stays byte-identical.
/// 范围在落地处被强制执行，而越界变成一份**请求**而不是一堵墙：回复点名每条落在边界外的路径、那里改了什么、
/// 还有谁共享这些路径、替代方案、以及决定它的确切命令——而基树逐字节不变。
///
/// The old assertion here read "Widen the scope by opening a new candidate". That sentence codified the
/// wrong behaviour: it sent the reader back to the start of the work, and it made the boundary something
/// only a redo could reach. The maintainer's correction of 2026-10-18 replaced it — explain the overrun
/// and let the user decide whether the boundary grows.
/// 这里的旧断言读作"开一棵新候选来扩大范围"。那句话固化了错误行为：它把读者送回工作的起点，使边界只能靠重做
/// 一切才够得着。维护者 2026-10-18 的纠正取代了它——说明越权，由使用方决定边界是否扩大。
#[test]
fn a_change_outside_the_declared_scope_becomes_a_request_and_lands_nothing() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    // The copy's own test passes, so the scope is the only rule still standing when `land` runs.
    // 副本自己的测试是通过的，因此 `land` 跑起来时，范围是唯一还在拦的规则。
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    std::fs::write(tree.join("NOTES.txt"), "rewritten from the candidate\n").unwrap();
    let verified = candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    assert!(
        verified.contains("verification passed"),
        "the copy's own test must pass, or the scope is not the rule under test: {verified}"
    );
    let error = refused(&root, json!({"action":"land","id":id}));
    for expected in [
        "outside the declared scope",
        "NOTES.txt",
        "This is a request, not a refusal",
        "widening request ",
        "(pending)",
        "current scope       src",
        "requested increment NOTES.txt",
        "reason              ",
        "shared consumers    ",
        "alternatives",
        "xirang-mcp --confirm --root",
        "--accept --version",
        "--reject --version",
        "the boundary is unchanged while this is `pending`",
    ] {
        assert!(
            error.contains(expected),
            "the reply has to say `{expected}`: {error}"
        );
    }
    // Asking again is not a second request: the four facts that identify it did not change.
    // 再问一次不是第二份请求：识别它的四件事没变。
    let version = request_version(&error);
    let again = refused(&root, json!({"action":"land","id":id}));
    assert!(
        again.contains(&format!("widening request {version} (pending)")),
        "the same facts are the same request: {again}"
    );
    // The request is on the read surface, so it can be found without repeating the attempt.
    // 请求在读面上，因此不必重复这次尝试也能找到它。
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains(&format!("widening request {version} (pending)"))
            && status.contains("scope src\n"),
        "status prints the pending request and the unchanged boundary: {status}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("NOTES.txt")).unwrap(),
        "outside the scope\n",
        "the request happened before the first byte of the base was written"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 41);\n}\n",
        "and nothing else landed either: a landing is all-or-nothing, not first-come"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A landing is bound to the tree it was opened against: if the base moved while the candidate was
/// being worked on, landing would silently revert somebody else's change.
/// 落地绑定在它当初据以打开的树上：候选在被处理期间基树若动了，落地会静默回退别人的改动。
#[test]
fn a_base_that_moved_since_the_candidate_was_opened_refuses_the_landing() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    let passing = "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n";
    std::fs::write(tree.join("src/lib.rs"), passing).unwrap();
    let verified = candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    assert!(verified.contains("verification passed"), "{verified}");

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    7\n}\n",
    )
    .unwrap();
    let error = refused(&root, json!({"action":"land","id":id}));
    assert!(
        error.contains("the base tree moved since this candidate was opened"),
        "{error}"
    );
    assert!(
        error.contains("Open a new candidate on the current tree"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        "pub fn answer() -> i64 {\n    7\n}\n",
        "the newer base edit is still there, byte for byte"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A verification belongs to the bytes it ran against. A candidate edited after its verification is
/// **not** verified, and saying otherwise would be the oldest false green there is.
/// 验证属于它跑过的那批字节。验证之后又被改过的候选**不是**已验证的，说它是，就是最古老的那种假绿。
#[test]
fn a_candidate_edited_after_its_verification_cannot_land() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    43\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 43);\n}\n",
    )
    .unwrap();
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains("NOT bound to the candidate's current bytes"),
        "{status}"
    );
    let error = refused(&root, json!({"action":"land","id":id}));
    assert!(
        error.contains("is not verified at its current bytes"),
        "{error}"
    );
    assert!(error.contains("run `verify` again"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A verification identity is the **whole** closure, not the tree bytes alone: a verdict may read any
/// environment variable, so landing must compare the identity it was computed under. Using the tree digest
/// here was the candidate-side half of the environment false green (2026-10-18 completion review).
/// 验证身份是**整个**闭包，而不只是树字节：判定可能读任何环境变量，因此落地必须比较它当初据以算出的身份。
/// 这里用树摘要，正是环境假绿在候选侧的另一半（2026-10-18 完成性复核）。
#[test]
fn a_candidate_verification_is_bound_to_the_whole_closure() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    let verified = candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let identity = super::super::check_task::Inputs::capture(&tree, None).unwrap();
    assert!(
        verified.contains(&identity.verification_digest()),
        "the verification must record the environment-bound identity, not the tree digest: {verified}"
    );
    assert_ne!(
        identity.digest(),
        identity.verification_digest(),
        "the control: the tree identity and the verdict identity answer different questions"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The happy path, which is here to prove the two refusals above are not the only thing this surface
/// can do: the in-scope change lands, the out-of-scope file does not exist in the copy's delta, and the
/// reply carries both identities rather than a claim.
/// 顺利路径。它在这里是为了证明上面那两条拒绝不是这个执行面唯一会做的事：范围内的改动落地、范围外的文件不在
/// 副本的差异里，而回复携带的是两个身份而不是一句声称。
#[test]
fn an_in_scope_change_lands_and_reports_both_base_identities() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src", "README.md"]));
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    std::fs::write(tree.join("README.md"), "a file the candidate added\n").unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(
        landed.contains("candidate ") && landed.contains(" landed"),
        "{landed}"
    );
    assert!(
        landed.contains("wrote ") && landed.contains("src/lib.rs") && landed.contains("README.md"),
        "{landed}"
    );
    assert!(
        landed.contains("each file landed through a write-then-rename")
            && landed.contains("this was a transaction over 2 file(s)"),
        "the reply states the boundary it does not cross: {landed}"
    );
    assert!(
        landed.contains("base ") && landed.contains(" -> "),
        "both base identities are reported rather than only the new one: {landed}"
    );
    // The landing names where the scope's confirmation came from **and** what it is not. The adoption
    // ledger is writable through this same surface, so calling its entry proof would be the overclaim the
    // independent review recorded (`docs/managed-maintenance/independent-review-2026-10-17.md` §2).
    // 落地写明范围的确认来自哪里，**以及它不是**什么。采信台账经同一个执行面就能写，因此把它的条目叫做证明，
    // 正是独立复核记下的那句过头话（`docs/managed-maintenance/independent-review-2026-10-17.md` §2）。
    assert!(
        landed.contains("scope confirmed by maintainer")
            && landed.contains("a declaration, not proof")
            && landed.contains("could have written itself"),
        "the landing says what the confirmation is and is not: {landed}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("README.md")).unwrap(),
        "a file the candidate added\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("NOTES.txt")).unwrap(),
        "outside the scope\n",
        "a file the copy left alone is not in the delta at all"
    );
    let again = refused(&root, json!({"action":"verify","id":id}));
    assert!(again.contains("already landed"), "{again}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The landing says **which capability** it runs at, what that capability does check, and what it does **not**
/// protect. Without this line a reader can take "the scope, the verification and the approval were checked" for
/// "unauthorized writes are prevented", which is the overclaim the ruling forbids for a plain MCP.
/// 落地写明它跑在**哪种能力**上、那种能力确实检查什么、又**不**保护什么。没有这一行，读者会把"范围、验证与批准都查过"
/// 读成"越权写入已被阻止"，而那正是裁定禁止普通 MCP 说的一句话。
#[test]
fn the_landing_says_which_capability_it_runs_at_and_what_it_does_not_protect() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    for expected in [
        "capability plain-mcp",
        "cannot prevent another tool or process from writing",
        "pseudo-terminal can answer the approval prompt itself",
        "do not read this reply as",
        // **Updated 2026-10-18**: the old sentence ended "which this bridge does not implement" — true when the
        // bridge had no managed mode, false now that it has one. The claim must still not be made here, and it
        // now has to point at the mode that can make it, so the sentence a reader relies on stays actionable.
        // **2026-10-18 更新**：旧句子以"which this bridge does not implement"结尾——在桥还没有托管模式时是真的，
        // 现在有了就不再成立。这句话仍然不许在这里说，但必须指向那个能说它的模式，好让读者依赖的那句话保持可行动。
        "that claim belongs to managed execution, and this landing does not run inside it",
        "xirang-mcp --managed --root PATH --writable PATH",
    ] {
        assert!(
            landed.contains(expected),
            "the landing has to say `{expected}`: {landed}"
        );
    }
    // A claim it must never make.
    // 一句它永远不许说的话。
    for forbidden in [
        "prevented unauthorized",
        "unauthorized writes are prevented:",
    ] {
        assert!(
            !landed.contains(forbidden),
            "the landing claims more than it does (`{forbidden}`): {landed}"
        );
    }
    // And `status` carries the same line plus the checkable reason the stronger level is not claimed.
    // 而 `status` 带同一行，外加"为什么不声称更强能力"这句可核对的话。
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    // **Updated 2026-10-18**: this used to ask for "publish loop", the reason that was true while the bridge had no
    // managed mode at all. The reason changed with the mode: a write domain exists for a command started under
    // `--managed`, and a landing runs outside it. The pin now states the reason the product states.
    // **2026-10-18 更新**：这里过去要求出现 "publish loop"，那是在桥完全没有托管模式时成立的解释。理由随模式变了：写域只为
    // 在 `--managed` 下启动的命令存在，而落地跑在它之外。钉子改为断言产品说出的那个理由。
    assert!(
        status.contains("capability plain-mcp")
            && status
                .contains("a write domain exists only for a command started under `--managed`"),
        "status says what it runs at and why not managed: {status}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The other side of the pair, at the handler level: an approval the controller issued for **another candidate**
/// does not cover this landing, and the refusal names the dimension. Without this, "an approval exists" could
/// pass for "this approval covers this write".
/// 成对验收的另一侧，在处理函数这一层：控制器为**另一个候选**签发的批准不覆盖这次落地，而拒绝点名那个维度。没有这一条，
/// "存在一份批准"就能冒充"这份批准覆盖这次写入"。
#[test]
fn an_approval_for_another_candidate_is_refused_by_name() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    approve(&root, "a-different-candidate", &["src".to_owned()]);
    let base_before = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("candidate") && refused.contains("never vouches"),
        "the refusal names the dimension and why an approval does not travel: {refused}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        base_before,
        "and nothing was written: a refusal at this gate happens before the first byte"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Discarding removes the copy and leaves the fact behind: "it was thrown away" and "it was never
/// opened" must not read the same from the disk.
/// 丢弃删掉副本并把这件事留下："它被丢掉了"与"它从没被打开过"不能从磁盘上读起来一样。
#[test]
fn discarding_removes_the_copy_and_keeps_the_fact_on_the_record() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    let reply = candidate(&root, &json!({"action":"discard","id":id})).unwrap();
    assert!(reply.contains("discarded"), "{reply}");
    assert!(reply.contains("the record stays"), "{reply}");
    assert!(!tree.exists(), "the copy is gone");
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(status.contains("state discarded"), "{status}");
    std::fs::remove_dir_all(root).unwrap();
}

/// BR-M3: a scope refusal must not be a dead end. It points at the **smallest** scope that would
/// cover what actually changed, and the proposal can be pasted.
/// BR-M3：范围拒绝不能是死胡同。它指向**覆盖实际改动的那个最小范围**，而该提案可以直接粘贴使用。
/// BR-M3: the request must not be a dead end. It names the **increment** that would cover what
/// actually changed — the parents of the changed paths, not the whole tree — and the proposal can be
/// pasted.
/// BR-M3：请求不能是死胡同。它点名**覆盖实际改动的那个增量**——改动路径的父目录，而不是整棵树——而提案可以直接
/// 粘贴使用。
#[test]
fn a_request_names_the_smallest_increment_that_would_cover_the_change() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    std::fs::write(tree.join("NOTES.txt"), "rewritten from the candidate\n").unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();

    let refusal = refused(&root, json!({"action":"land","id":id}));
    assert!(refusal.contains("outside the declared scope"), "{refusal}");
    assert!(
        refusal.contains("requested increment NOTES.txt"),
        "the request names the increment that covers the change: {refusal}"
    );
    assert!(
        !refusal.contains("requested increment ."),
        "an increment for the whole tree is not a minimum: {refusal}"
    );

    let proposal = candidate(&root, &json!({"action":"propose","id":id})).unwrap();
    assert!(proposal.contains("src/lib.rs (written)"), "{proposal}");
    assert!(proposal.contains("NOTES.txt (written)"), "{proposal}");
    assert!(
        proposal.contains("outside it: NOTES.txt"),
        "the proposal names exactly what the declared scope did not cover: {proposal}"
    );
    assert!(
        proposal.contains("smallest scope covering every changed path: NOTES.txt, src"),
        "the parents of the changed paths, not the whole tree: {proposal}"
    );
    assert!(
        proposal.contains(
            r#"ready to paste: xirang.candidate {"action":"open","scope":["NOTES.txt","src"]}"#
        ),
        "{proposal}"
    );
    assert!(
        proposal.contains("this is a **scope**, not an authorisation"),
        "a proposal must not read as a grant: {proposal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The proposal's scope has to be one that **works**: pasted into a fresh candidate, the same change
/// lands. A proposal nobody can use would be a nicer-looking refusal.
/// 提案给出的范围必须是**能用**的：粘进一棵新候选，同一批改动就能落地。没人能用的提案只是好看一点的拒绝。
#[test]
fn the_proposed_scope_actually_lands_the_change() {
    let root = fixture();
    let passing = "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n";
    let (first, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), passing).unwrap();
    std::fs::write(tree.join("NOTES.txt"), "rewritten from the candidate\n").unwrap();
    candidate(
        &root,
        &json!({"action":"verify","id":first,"face":"default"}),
    )
    .unwrap();
    refused(&root, json!({"action":"land","id":first}));

    let (second, tree) = opened(&root, json!(["src", "NOTES.txt"]));
    std::fs::write(tree.join("src/lib.rs"), passing).unwrap();
    std::fs::write(tree.join("NOTES.txt"), "rewritten from the candidate\n").unwrap();
    candidate(
        &root,
        &json!({"action":"verify","id":second,"face":"default"}),
    )
    .unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":second})).unwrap();
    assert!(landed.contains(" landed"), "{landed}");
    // §8.2 step 5: the landed tree is re-verified **by identity**. The reply must say the base now holds the identity
    // the verdict was bound to; a landing that stayed silent here would read as verified without saying what covers it.
    // §8.2 第 5 步：落地后的树按**身份**复验。回复必须说出基树现在持有着判定所绑定的身份；这里沉默的落地会被读成
    // 已验证，却不说是什么在覆盖它。
    assert!(
        landed.contains("post-landing: the base now has the verification identity")
            && landed.contains("covers what landed"),
        "the landing says what covers what was landed: {landed}"
    );
    let after = candidate(&root, &json!({"action":"status","id":second})).unwrap();
    assert!(
        after.contains("post-landing evidence: none:")
            && after.contains("the identity the verdict was bound to"),
        "status keeps the evidence answer after the landing: {after}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("NOTES.txt")).unwrap(),
        "rewritten from the candidate\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// §7: an upstream proposal carries its fan-in. "This is the shared root cause" is a claim about who
/// else calls it, and a proposal that did not name them would be asking to be believed.
/// §7：上游提案要带上它的扇入。"这是共享根因"是一句关于还有谁在调用它的主张，而不点名它们的提案是在要求被相信。
#[test]
fn the_proposal_names_the_shared_consumers_of_what_changed() {
    let root = fixture();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/arithmetic.rs"),
        "pub fn settle(n: i64) -> i64 {\n    n.max(0)\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/first.rs"),
        "pub fn first(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/second.rs"),
        "pub fn second(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
    )
    .unwrap();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(
        tree.join("src/arithmetic.rs"),
        "pub fn settle(n: i64) -> i64 {\n    if n < 0 { 0 } else { n }\n}\n",
    )
    .unwrap();
    let proposal = candidate(&root, &json!({"action":"propose","id":id})).unwrap();
    assert!(
        proposal.contains("shared consumers of what changed: src/arithmetic.rs::settle"),
        "{proposal}"
    );
    assert!(
        proposal.contains(
            "without a test calling them by name: src/first.rs::first, src/second.rs::second"
        ),
        "the fan-in includes the consumers nobody protects: {proposal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A widening is decided on **one exact request**: the version binds the task, the boundary it started from, the
/// increment and the paths. A decision on another version decides nothing, a refusal leaves the boundary alone, and an
/// acceptance clears the verdict because the obligations grew — the work continues only after the copy is verified
/// again, and then the landing lands. This is the pin that made "confirm a boolean" impossible to keep.
/// 扩大是针对**一份确切的请求**作出的决定：版本绑定了任务、它出发的边界、增量与路径。对另一个版本作出的决定什么也没决定，
/// 拒绝不动边界，接受会清掉判定——因为义务变多了——副本重新验证之后工作才继续，然后落地真的落地。正是这条钉子让"确认一个
/// 布尔值"的写法再也留不住。
#[test]
fn a_widening_is_decided_on_one_exact_request_and_then_the_work_continues() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    std::fs::write(
        tree.join("NOTES.txt"),
        "decided by the host
",
    )
    .unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();

    // ① The overrun files a request, names the increment, and lands nothing.
    // 越界登记一份请求、点名增量，并且什么都不落地。
    let first = refused(&root, json!({"action":"land","id":id}));
    assert!(
        first.contains("This is a request, not a refusal") && first.contains("(pending)"),
        "the overrun is a request waiting for a decision: {first}"
    );
    let version = request_version(&first);

    // ② A decision on another version decides nothing.
    // 对另一个版本作出的决定什么也没决定。
    assert_eq!(
        confirm(&root, &id, &["--accept", "--version", "0000000000000000"]),
        2,
        "a version the person did not read confirms a request nobody saw"
    );

    // ③ A refusal keeps the boundary where it was; the next attempt files the same increment again.
    // 拒绝把边界留在原处；下一次尝试会为同一个增量再登记一份请求。
    assert_eq!(confirm(&root, &id, &["--reject", "--version", &version]), 0);
    let again = refused(&root, json!({"action":"land","id":id}));
    assert!(
        again.contains("(pending)") && again.contains("NOTES.txt"),
        "the boundary is unchanged, so the same change is still outside it: {again}"
    );
    let second = request_version(&again);

    // ④ An acceptance grows the boundary by the increment only, and clears the verdict.
    // 接受只按增量扩大边界，并清掉判定。
    assert_eq!(confirm(&root, &id, &["--accept", "--version", &second]), 0);
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains("scope src, NOTES.txt"),
        "the boundary grew by exactly the increment: {status}"
    );
    let unverified = refused(&root, json!({"action":"land","id":id}));
    assert!(
        unverified.contains("not verified") || unverified.contains("no verification"),
        "the obligations grew, so the old verdict cannot cover them: {unverified}"
    );

    // ⑤ Verify again; then the work continues and lands.
    // 重新验证；然后工作继续并落地。
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(landed.contains("landed"), "{landed}");
    assert!(
        std::fs::read_to_string(root.join("NOTES.txt"))
            .unwrap()
            .contains("decided by the host"),
        "the path the acceptance added is on the base tree"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// §5: an old narrow grant never vouches for a wider range. The ledger keeps the widening visible
/// instead of letting "narrow, then wide" read as one authorised story.
/// §5：旧窄范围从不为更宽的范围背书。台账让那次扩大一直可见，而不是让"先窄后宽"读成一个被授权的故事。
#[test]
fn the_scope_ledger_keeps_a_widening_visible() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("NOTES.txt"), "outside\n").unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    refused(&root, json!({"action":"land","id":id}));

    let (second, _) = opened(&root, json!(["src", "NOTES.txt"]));
    let proposal = candidate(&root, &json!({"action":"propose","id":second})).unwrap();
    assert!(proposal.contains("scope ledger (3 line(s))"), "{proposal}");
    assert!(
        proposal.contains("authorised src, src, NOTES.txt")
            || proposal.contains("authorised NOTES.txt, src"),
        "the ledger shows both the narrow and the wide grant: {proposal}"
    );
    assert!(
        proposal.contains("attempted beyond it NOTES.txt"),
        "and it keeps what was attempted outside the narrow one: {proposal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A copy that passes its own test, written into a candidate tree.
/// 一份能通过自身测试的源码，写进候选树里。
pub(super) const PASSING: &str = "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n";

/// M5: a review belongs to a **version**. Bytes that moved are bytes nobody reviewed.
/// M5：评审属于一个**版本**。动过的字节就是没人评审过的字节。
#[test]
fn evidence_that_does_not_cover_the_landed_tree_keeps_the_obligation_open() {
    let root = fixture();
    let (id, _tree) = opened(&root, json!(["src"]));
    let directory = root.join("target").join("xirang").join("candidates");
    let mut record = super::Record::read_at(&directory, &id).unwrap();
    record.body["verification"] =
        json!({"candidate": "an identity from another tree", "verdict": "passed"});
    let inputs = crate::mcp::check_task::Inputs::capture(&root, None).unwrap();
    let line = super::candidate_actions::landing_evidence(&inputs, &mut record);
    assert!(
        line.contains("does not cover what landed") && line.contains("xirang.check"),
        "the reply keeps the obligation open and names the command: {line}"
    );
    assert_eq!(
        record.body["post_landing"]["reused_candidate_evidence"],
        json!(false)
    );
    assert_eq!(
        record.body["post_landing"]["candidate_verification"],
        json!("an identity from another tree")
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **EX07: when a mapping moves code to another path, the authorization moves with it — as a new
/// decision, not as the old approval.**
/// **EX07：当映射把代码搬到另一个路径时，授权跟着映射走——但走的是一份**新决定**，不是旧批准。**
///
/// This is the ordinary move shape (one object, a determined destination, the bytes kept), and what
/// the pin checks is the boundary rather than the move engine: the candidate was opened, verified
/// and approved for `src/lib.rs`, and the moment the object also lives in `src/other.rs` that
/// approval no longer describes the write set. The landing therefore refuses with a request that
/// names the path the mapping produced; accepting that exact request grows the boundary, the
/// verification it invalidates is re-run, and only then does the work continue.
/// 这是普通的移动形状（一个对象、确定的落点、字节保留），而钉子检查的是边界而不是移动引擎：候选是按
/// `src/lib.rs` 开、验证、批准的，而当对象多住在 `src/other.rs` 时，那份批准不再描述这次写入集合。于是落地
/// 拒绝并要求**点名映射产出的那条路径**；针对那份确切请求接受才会扩大边界，被它作废的验证要重跑，然后工作
/// 才继续。
#[test]
fn a_mapping_that_moves_code_to_another_path_needs_a_new_confirmation() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src/lib.rs"]));
    std::fs::write(
        tree.join("src/other.rs"),
        "pub fn extra() -> i64 {\n    1\n}\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("src/lib.rs"),
        "mod other;\n\npub fn answer() -> i64 {\n    41 + other::extra()\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    let verified = candidate(&root, &json!({"action":"verify","id":id,"face":"default"}))
        .expect("the moved code still verifies");
    assert!(verified.contains("verification passed"), "{verified}");

    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("src/other.rs") && refused.contains("not a refusal"),
        "the landing has to name the path the mapping produced and ask for it: {refused}"
    );
    assert!(
        refused.contains("outside the declared scope (src/lib.rs)"),
        "the refusal says which boundary the new path crossed: {refused}"
    );
    let version = request_version(&refused);

    assert_eq!(confirm(&root, &id, &["--accept", "--version", &version]), 0);
    let re_verified = candidate(&root, &json!({"action":"verify","id":id,"face":"default"}))
        .expect("the grown scope needs its own verification");
    assert!(
        re_verified.contains("verification passed"),
        "the boundary moved, so the pass had to be earned again: {re_verified}"
    );
    let landed = candidate(&root, &json!({"action":"land","id":id}))
        .expect("the accepted increment authorises the continuation");
    assert!(landed.contains("landed"), "{landed}");
    assert!(
        landed.contains("post-landing: the base now has the verification identity"),
        "the landing says the evidence covers what landed: {landed}"
    );
    assert!(root.join("src/other.rs").exists(), "the mapped file landed");
    assert!(
        std::fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("mod other;"),
        "the source that now names the module landed"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("NOTES.txt")).unwrap(),
        "outside the scope\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}
