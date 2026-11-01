//! The three release fault points measured against the real tree: where a landing stops, and what the base and the
//! ledger say afterwards.
//! 对着真树量出的三个发布故障位点：落地在哪里停住，以及停下之后基树与账本各自说什么。
//!
//! Why they live together rather than beside the rest of the candidate pins: each one injects a **real** filesystem
//! failure (a directory occupying the staging path, a record directory that accepts no new entry) and then reads the
//! tree back. Grouping them is what makes "which fault points are covered" answerable by looking at one page.
//! 为什么它们住在一起、而不是散在候选钉子旁边：每一条都注入一次**真实**的文件系统失败（用目录占住暂存路径、让记录
//! 目录不再接受新条目），然后把树读回来。把它们放在一起，"哪些故障位点被覆盖了"才只需要看一页就能回答。

use serde_json::{Value, json};

use super::candidate;
use super::candidate_tests::{PASSING, fixture, opened, refused};

/// EX02: a landing writes the file it names, never the place a link points at. A copy that replaces a file with a
/// symbolic link is refused by name, the base file keeps its bytes, and the link's target is not touched — the
/// refusal is the point, because "the link disappeared" would otherwise be read as "the file was deleted".
/// EX02：落地写它点名的文件，绝不写链接指向的地方。把文件换成一个符号链接的副本会被点名拒绝，基树文件保留自己的
/// 字节，链接目标也没被碰——拒绝本身是要点，否则"链接不见了"会被读成"文件被删了"。
///
/// The link points **inside** the candidate tree on purpose. Since the shared source scope owns
/// "an undeclared link never widens the sources", an out-of-tree link is now refused one layer
/// earlier (while reading the candidate's faces), so a fixture pointing outside would pin that rule
/// twice and leave this guard — the landing's own "never write through a link" — untested. A link
/// inside the tree passes the reader and reaches this writer, which refuses **any** link it would
/// write through, including one whose target is a file of the same tree.
/// 链接有意指向候选树**内部**。既然共同来源范围拥有"未声明的链接绝不扩大来源"，越界链接现在会在更早一层
/// （读取候选注册面时）被拒绝，因此指向外面的夹具只会把那条规则钉两遍，却让这里——落地自己的"绝不穿过链接
/// 写"——没被测到。树内链接能通过读取方、到达这个写入方，而它拒绝**任何**它将要穿过的链接，包括目标就是
/// 同一棵树里的文件的链接。
#[cfg(unix)]
#[test]
fn a_landing_never_writes_through_a_link() {
    let root = fixture();
    std::fs::write(
        root.join("src/alias.rs"),
        "pub fn alias() -> i64 {\n    1\n}\n",
    )
    .unwrap();
    let (id, tree) = opened(&root, json!(["src"]));
    let target = tree.join("src/target.rs");
    std::fs::write(&target, "pub fn target() -> i64 {\n    2\n}\n").unwrap();
    std::fs::remove_file(tree.join("src/alias.rs")).unwrap();
    std::os::unix::fs::symlink(&target, tree.join("src/alias.rs")).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let refusal = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refusal.contains("symbolic link") && refusal.contains("never the place a link"),
        "the landing names the link and refuses instead of following it: {refusal}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/alias.rs")).unwrap(),
        "pub fn alias() -> i64 {\n    1\n}\n",
        "the base tree's file still says what it said"
    );
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "pub fn target() -> i64 {\n    2\n}\n",
        "and the link's target was neither written nor removed"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The other half of the same rule: a link that leaves the tree is refused by the **source scope**,
/// before a landing ever plans a byte. Both refusals name the link and its real target, because a
/// skipped link would read as "there is nothing there" — a different fact from "this is not a source".
/// 同一条规则的另一半：离开这棵树的链接由**来源范围**拒绝，早于落地规划任何字节。两种拒绝都点名链接与它的
/// 真实目标，因为被跳过的链接会被读成"那里什么都没有"——那是与"这不是来源"不同的事实。
///
/// Gated on `lang-cpp` because this refusal is reached through the **landing's** tree read: the
/// graft gate derives the candidate's faces, and the C++ declaration discovery is what walks the
/// sources. Without any language feature a candidate with no changed face never reads the sources at
/// all, so the link is never visited. Measured both ways: `--features mcp,lang-cpp` passes,
/// `--features mcp` does not compile this test. The rule itself is pinned feature-independently in
/// `multilang::source_scope_tests`.
/// 门控在 `lang-cpp` 上：这条拒绝要经**落地**的树读取才能到达——嫁接门推导候选的注册面，而遍历源码的正是
/// C++ 声明发现。没有任何语言特性时，一个没有改动面的候选根本不读源码，链接也就不会被访问。两个方向都量过：
/// `--features mcp,lang-cpp` 通过，`--features mcp` 不编译这条测试。规则本身由
/// `multilang::source_scope_tests` 以与特性无关的方式钉住。
#[cfg(all(unix, feature = "lang-cpp"))]
#[test]
fn a_link_out_of_the_tree_is_refused_by_the_source_scope() {
    let root = fixture();
    let outside = std::env::temp_dir().join(format!("xirang-link-target-{}", std::process::id()));
    let _ = std::fs::remove_file(&outside);
    std::fs::write(&outside, "pub fn outside() -> i64 {\n    2\n}\n").unwrap();
    let (id, tree) = opened(&root, json!(["src"]));
    // A file the fixture did not have, so the refusal cannot be mistaken for the test-weakening gate:
    // the point is the **scope**, and a link named `extra.rs` reaches it without removing a test.
    // 夹具本来没有的文件，因此这次拒绝不会被误读成"削弱测试"那道门：要点在**来源范围**，一条叫
    // `extra.rs` 的链接就能到达它，而不必删掉任何测试。
    std::os::unix::fs::symlink(&outside, tree.join("src/extra.rs")).unwrap();
    // `verify` runs the tests and reports a verdict; it does not read the registration faces. `land`
    // does — that is where the scope decides which sources exist, and where an undeclared link is
    // refused before a byte is planned.
    // `verify` 跑测试并给出结论，不读注册面；`land` 才读——来源范围在那里决定"哪些来源存在"，未声明的
    // 链接也在那里、在规划任何字节之前被拒绝。
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let refusal = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refusal.contains("links to") && refusal.contains("not a way to widen the sources"),
        "the scope names the link and the root it would leave: {refusal}"
    );
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "pub fn outside() -> i64 {\n    2\n}\n",
        "and the link's target was not written through"
    );
    let _ = std::fs::remove_file(&outside);
    let _ = std::fs::remove_dir_all(root);
}

/// 发布故障的第一个位点：**prepare 之后、第一次写之前**中断。基树一个字节都没写，记录仍是 `open`，journal 里
/// 留着"本来要发生什么"，而**恢复就是把障碍拿掉再落一次**——这是规格允许的"明确中断并可继续"。
/// The first release fault point: interrupted **after prepare, before the first write**. The base has not one byte
/// written, the record is still `open`, the journal holds what was about to happen, and **recovery is removing the
/// obstacle and landing again** — the specification's "explicitly interrupted and can continue".
///
/// The obstacle is made real rather than simulated: the staging path is occupied by a **directory**, so `fs::copy`
/// fails the way a filesystem failure really fails. A test that mocked the writer would be proving the mock.
/// 障碍是真的而不是模拟的：暂存路径被一个**目录**占着，于是 `fs::copy` 像真实的文件系统失败那样失败。用假
/// writer 的测试只能证明那个假货。
#[test]
fn a_landing_that_fails_before_its_first_write_can_be_continued() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    // Occupy the staging path the landing writes through with a directory. The landing names it with
    // `Path::with_extension`, which **replaces** the extension, so `src/lib.rs` stages at `src/lib.xirang-landing`.
    // A directory there is invisible to the input closure — only regular files are bound — so the base identity
    // still matches and the failure lands where a disk error would: at `fs::copy`.
    // 把落地写穿过的暂存路径用目录占住。落地用 `Path::with_extension` 命名它，而那是**替换**扩展名，因此
    // `src/lib.rs` 暂存在 `src/lib.xirang-landing`。它对输入闭包是隐形的——只有常规文件被绑定——因此基树身份仍然
    // 匹配，而失败正落在磁盘错误会发生的地方：`fs::copy`。
    let staging = root.join("src/lib.xirang-landing");
    std::fs::create_dir_all(&staging).unwrap();
    let base_before = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("src/lib.rs") && refused.contains("cannot stage"),
        "the refusal names the file that could not be written: {refused}"
    );
    // Nothing was written: the real file is untouched and the record is not landed.
    // 什么都没写：真正的文件没被碰过，而记录不是 landed。
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        base_before,
        "the base file is exactly what it was"
    );
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains("state open"),
        "a failed landing leaves the candidate open rather than claiming completion: {status}"
    );
    // The journal says what was about to happen, so a reader is not left with a half-state and no map.
    // journal 留着本来要发生什么，于是读者不会拿到一个半成品状态却没有地图。
    let journal = std::fs::read_dir(root.join("target/xirang/candidates"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".journal.jsonl"))
        })
        .expect("the landing journalled its intent");
    let journalled = std::fs::read_to_string(&journal).unwrap();
    assert!(
        journalled.contains("src/lib.rs") && journalled.contains("\"land\""),
        "the journal names the action and the paths: {journalled}"
    );
    // Recovery: remove the obstacle and land again. The base digest never moved, so this is the same candidate.
    // 恢复：拿掉障碍再落一次。基树摘要从没动过，因此这还是同一棵候选。
    std::fs::remove_dir(&staging).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id}))
        .expect("after the obstacle is gone the same candidate lands");
    assert!(
        landed.contains(" landed") && landed.contains("src/lib.rs"),
        "{landed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// 发布故障的第二个位点：**第一个文件落盘之后**、第二个失败。事务化之后这里的行为变了，而这道钉子跟着改成新的正确
/// 行为：**第一个文件被撤销**、基树逐字回到开立时的样子、记录仍是 `open`，于是同一棵候选在障碍移除后可以继续落地——
/// 而不是留下半套改动再要求新开一棵候选。
/// The second release fault point: **after the first file landed**, the second fails. Transactional landing changed
/// what is correct here, and this pin follows the new behaviour: **the first file is undone**, the base is byte-for-byte
/// what it was when the candidate opened, the record stays `open`, and the same candidate lands once the obstacle is
/// removed — instead of leaving half a change and asking for a new candidate.
#[test]
fn a_landing_that_fails_after_the_first_file_rolls_that_file_back() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src", "README.md"]));
    // Two changes, and the alphabetically first one (`README.md` < `src/lib.rs`) is the one that succeeds.
    // 两处改动，而按路径排在前面那个（`README.md` < `src/lib.rs`）是会成功的那一个。
    std::fs::write(tree.join("README.md"), "a file the copy changed\n").unwrap();
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let readme_before = std::fs::read_to_string(root.join("README.md")).unwrap_or_default();
    let source_before = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let staging = root.join("src/lib.xirang-landing");
    std::fs::create_dir_all(&staging).unwrap();
    let failure = refused(&root, json!({"action":"land","id":id}));
    assert!(
        failure.contains("src/lib.rs") && failure.contains("cannot stage"),
        "the failure names the file it could not write: {failure}"
    );
    assert!(
        failure.contains("rolled back") && failure.contains("base tree's files"),
        "the failure says the transaction was undone rather than only that it failed: {failure}"
    );
    // The half state is gone: the first file is back to the bytes the base had before this landing.
    // 半成品状态消失了：第一个文件回到了这次落地之前基树上的字节。
    assert_eq!(
        std::fs::read_to_string(root.join("README.md")).unwrap_or_default(),
        readme_before,
        "the file that had landed was written back to its old bytes"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        source_before,
        "the file that failed was never written"
    );
    // The ledger is open, and the landing record says exactly what was undone.
    // 账本仍是 open，而落地记录说清了撤销了什么。
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains("state open"),
        "it must not read as landed: {status}"
    );
    let candidates = root.join("target/xirang/candidates");
    let landing: Value = serde_json::from_str(
        &std::fs::read_to_string(candidates.join(format!("{id}.landing.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        landing["state"], "rolled_back",
        "the record keeps the undo as a state, not as a silence: {landing}"
    );
    assert_eq!(
        landing["files"].as_array().map(Vec::len),
        Some(2),
        "{landing}"
    );
    // Recovery: remove the obstacle and land the **same** candidate — the baseline never moved, because the
    // landing that failed put it back.
    // 恢复：拿掉障碍，落**同一棵**候选——基线从没动过，因为失败的那次落地把它放了回去。
    std::fs::remove_dir(&staging).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id}))
        .expect("after the obstacle is gone the same candidate lands");
    assert!(
        landed.contains(" landed") && landed.contains("src/lib.rs"),
        "{landed}"
    );
    // The finished transaction stops being open, so a later run does not think it has work to undo.
    // 已完成的事务不再开着，于是后来的运行不会以为它还有东西要撤销。
    assert!(
        !candidates.join(format!("{id}.landing.json")).exists(),
        "a finished landing closes its transaction record"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// 发布故障的第三个位点：**每个文件都落盘之后、候选记录发布之前**中断。障碍物占住候选记录自己的临时路径
/// （`<id>.json.tmp`），而落地记录的临时路径是 `<id>.landing.tmp`——于是事务本身完成、只有最后那次记录发布失败，
/// 这正是这道钉子要的位点。事务化之后，下一次落地**完成同一事务**，而不是重写文件或要求新开候选。
/// The third release fault point: **after every file landed, before the candidate record was published**. The
/// obstacle occupies the candidate record's own temporary path (`<id>.json.tmp`), while the landing record's is
/// `<id>.landing.tmp` — so the transaction itself completes and only the final record write fails, exactly the point
/// this pin is about. After the change, the next landing **finishes the same transaction** instead of rewriting files
/// or demanding a new candidate.
#[test]
fn a_landing_that_lost_its_record_finishes_the_same_transaction_on_the_next_run() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let candidates = root.join("target/xirang/candidates");
    let before = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    // `persist` stages a record at `path.with_extension("tmp")`, which **replaces** the extension: the candidate's
    // record stages at `<id>.tmp`, and the landing record at `<id>.landing.tmp`. Occupying the first with a
    // directory therefore fails only the final record write.
    // `persist` 把记录暂存在 `path.with_extension("tmp")`，而那是**替换**扩展名：候选记录的暂存名是 `<id>.tmp`，
    // 落地记录的是 `<id>.landing.tmp`。用目录占住前者，因此只有最后那次记录发布会失败。
    let obstacle = candidates.join(format!("{id}.tmp"));
    std::fs::create_dir_all(&obstacle).unwrap();
    let failure = refused(&root, json!({"action":"land","id":id}));
    // The failure names the record it could not write, and it is the **candidate's** record, not the transaction's.
    // 失败点名它写不下去的那份记录，而且是**候选的**记录、不是事务的。
    assert!(
        failure.contains("cannot write") || failure.contains("cannot publish"),
        "the failure names the record it could not write: {failure}"
    );
    // Every file really did land first: this is the "all files, no bookkeeping" half state.
    // 文件确实先全落了：这就是"文件全写、账没记"的半成品状态。
    assert_ne!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        before,
        "every file landed before the record was published"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(candidates.join(format!("{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        record["state"], "open",
        "the record must not read as landed: {record}"
    );
    let landing: Value = serde_json::from_str(
        &std::fs::read_to_string(candidates.join(format!("{id}.landing.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        landing["state"], "published",
        "the transaction remembers that it wrote every file: {landing}"
    );
    // Recovery: clear the obstacle and land again. The next run recognises the published transaction, finishes its
    // record, and does not write the files a second time.
    // 恢复：清掉障碍再落一次。下一次运行认出已发布的事务、完成它的记录，不会把文件再写一遍。
    std::fs::remove_dir(&obstacle).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id}))
        .expect("the next landing finishes the transaction that had already written every file");
    assert!(
        landed.contains("landed (recovered)") && landed.contains("nothing was written twice"),
        "{landed}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(candidates.join(format!("{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        record["state"], "landed",
        "the record is finalised: {record}"
    );
    assert!(
        !candidates.join(format!("{id}.landing.json")).exists(),
        "the finished transaction stops being open"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// 并发写入者：同一棵候选上第二个落地拿不到锁，于是"检查过的基线"不会被另一个写者覆盖。留下的锁如果持有者已经
/// 不存在，则被接管——否则一次崩溃就会永久锁死这棵候选。
/// Concurrent writers: a second landing on the same candidate cannot take the lock, so a baseline that was checked is
/// not overwritten by another writer. A lock whose holder no longer exists is taken over — otherwise one crash would
/// lock the candidate forever.
#[test]
fn a_second_landing_on_the_same_candidate_waits_and_a_stale_lock_is_taken_over() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let lock = root
        .join("target/xirang/candidates")
        .join(format!("{id}.landing.lock"));
    // A lock held by a live process — this one — is a real concurrent writer as far as the candidate is concerned.
    // 被一个活进程（就是本进程）持有的锁，从候选的角度看就是一个真实的并发写者。
    std::fs::write(&lock, format!("{}", std::process::id())).unwrap();
    let busy = refused(&root, json!({"action":"land","id":id}));
    assert!(
        busy.contains("another landing for this candidate is in progress")
            && busy.contains("nothing is written"),
        "a concurrent landing is refused before anything is written: {busy}"
    );
    // A lock whose holder is gone is taken over rather than left for a person to delete.
    // 持有者已不存在的锁会被接管，而不是留着等某个人来删。
    std::fs::write(&lock, "999999").unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id}))
        .expect("a stopped holder's lock does not lock the candidate forever");
    assert!(landed.contains(" landed"), "{landed}");
    std::fs::remove_dir_all(root).unwrap();
}
