//! P2 pins: a rule is about an **object**, and a move carries it.
//! P2 钉子：规则是关于**对象**的，而一次搬动会带着它走。

use serde_json::json;
use std::path::{Path, PathBuf};

/// A package with a provider face that declares an author-owned stable name, a parent to move it
/// under, a policy file and a lease in force.
/// 一个包：带作者拥有的稳定名的 provider 面、一个可供搬进去的父面、一个政策文件，以及一份生效租约。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-rules-identity-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").expect("policy");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","stable_name":"FixtureProvider","provides":"cap.render"},"apply":true}),
    )
    .expect("the provider face is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("the parent face is created");
    crate::mcp::adopted::adopted(
        &root,
        &json!({"anchor":"policy","certifies":"fixture policy","evidence":"desk check","verifier":"identity test","reason":"the fixture policy is the authority","files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the lease is in force");
    root
}

/// A package with a manifest, a library target and the policy file — and **no faces**.
/// 一个有清单、一个库目标与政策文件、而**没有任何面**的包。
///
/// `package()` already creates a provider with the author-owned `FixtureProvider`; a second face
/// declaring the same stable name is refused by the registry (correctly — that is a real identity
/// collision). A test that builds its own face tree starts here.
/// `package()` 已经创建了带作者稳定名 `FixtureProvider` 的 provider；第二个声明同一稳定名的面会被注册器
/// 拒绝（这是对的——那是一次真实的身份冲突）。要自己搭面树的测试从这里开始。
fn bare_package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-rules-bare-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").expect("policy");
    root
}

/// Propose and accept a rule that names `provides cap.render` over `root/provider`.
/// 提议并接受一条对 `root/provider` 要求 `provides cap.render` 的规则。
fn accept_rule(root: &Path, id: &str) {
    accept_rule_at(root, id, "root/provider", "cap.render", "retain cap.render");
}

/// Propose and accept one rule over `selector`, expecting `provides <capability>`.
/// 对 `selector` 提议并接受一条规则，期望 `provides <capability>`。
fn accept_rule_at(root: &Path, id: &str, selector: &str, capability: &str, requirement: &str) {
    crate::mcp::rules::rules(
        root,
        &json!({"action":"propose","rule_id":id,"category":"protocol","selector":selector,
                "requirement":requirement,"origin":"accepted_policy","sources":["policy.txt"],
                "expects":{"provides":capability}}),
    )
    .expect("the rule is proposed");
    crate::mcp::rules::rules(
        root,
        &json!({"action":"accept","rule_id":id,"authority":"policy","confirm":true}),
    )
    .expect("the rule is accepted");
}

/// The lease every fixture needs, over `policy.txt`.
/// 每个夹具都需要的那份租约，落在 `policy.txt` 上。
fn adopt_policy(root: &Path) {
    crate::mcp::adopted::adopted(
        root,
        &json!({"anchor":"policy","certifies":"fixture policy","evidence":"desk check","verifier":"identity test","reason":"the fixture policy is the authority","files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the lease is in force");
}

/// A package whose library target lives outside `src/`, as `[lib] path = "source/lib.rs"`.
/// 一个库目标住在 `src/` 之外的包，即 `[lib] path = "source/lib.rs"`。
fn custom_library_package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-rules-custom-lib-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("source")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"source/lib.rs\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("source/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").expect("policy");
    root
}

/// The rule record's JSON.
/// 规则记录的 JSON。
fn record(root: &Path, id: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(root.join(format!(".xirang/rules/{id}.json")))
        .expect("the rule is on disk");
    serde_json::from_str(&text).expect("the rule is JSON")
}

/// The rule's append-only history, one JSON object per line.
/// 规则只追加的历史，一行一个 JSON 对象。
fn history(root: &Path, id: &str) -> Vec<serde_json::Value> {
    std::fs::read_to_string(root.join(format!(".xirang/rules/{id}.history.jsonl")))
        .expect("the history is on disk")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each history line is JSON"))
        .collect()
}

/// **A rule accepted against an object keeps naming that object after a move.**
/// **对照一个对象接受的规则，在搬动之后仍然点名那个对象。**
///
/// The rule is bound by the ledger's stable id, the move rewrites the record inside the same
/// transaction as the moved bytes, and the history records why the revision changed.
/// 规则按台账的稳定 id 绑定，搬动在"被搬字节"的同一个事务里改写记录，而历史记下修订为什么变。
#[test]
fn a_rule_follows_its_object_through_a_move() {
    let root = package("move");
    accept_rule(&root, "keep-render");
    let before = record(&root, "keep-render");
    let stable = before["object"]["stable"]
        .as_str()
        .expect("the rule is bound to an object")
        .to_owned();
    assert_eq!(before["revision"], json!(2), "{before}");
    assert!(
        crate::mcp::rules::rules(&root, &json!({"action":"list"}))
            .unwrap()
            .contains(&stable),
        "the listing names the object the rule is about"
    );

    let moved = crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/provider","to":"root/panel","apply":true}),
    )
    .expect("the move applies");
    assert!(
        moved.contains("follows the object"),
        "the move reply names the rule it carried: {moved}"
    );

    let after = record(&root, "keep-render");
    assert_eq!(
        after["object"]["stable"].as_str(),
        Some(stable.as_str()),
        "the object identity is the one the rule was accepted against: {after}"
    );
    assert_eq!(
        after["object"]["path"].as_str(),
        after["scope"]["selector"].as_str(),
        "the binding and the selector agree after the migration: {after}"
    );
    assert!(
        after["scope"]["selector"]
            .as_str()
            .expect("selector")
            .starts_with("root/panel/"),
        "the selector followed the move: {after}"
    );
    assert_eq!(
        after["revision"],
        json!(3),
        "a movement of the knowledge is a revision: {after}"
    );
    let history = history(&root, "keep-render");
    assert_eq!(
        history.last().and_then(|event| event["action"].as_str()),
        Some("moved"),
        "the history says why the revision changed: {history:?}"
    );
    assert_eq!(
        history.last().and_then(|event| event["revision"].as_u64()),
        Some(3),
        "the history event carries the revision it produced"
    );
    // And the rule still decides against the moved object.
    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("met (structural)"),
        "the rule is still met by the object at its new location: {listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **A rule bound to one object does not apply to a different object that reuses its old path.**
/// **绑定到某个对象的规则，不适用于复用其旧路径的另一个对象。**
///
/// The moved face declared an author-owned `stable_name`, so a new face created at the old path
/// derives a different identity. The rule's selector is then pointed back at the old path by hand —
/// the state a failed or skipped migration leaves — and the decision must refuse to apply it to the
/// impostor rather than reporting a verdict about the wrong object.
/// 被搬的面声明了作者拥有的 `stable_name`，因此在旧路径上新建的面会推导出另一个身份。随后手工把规则的
/// 选择器指回旧路径——一次失败或被跳过的迁移留下的状态——判定必须拒绝适用于这个冒名者，而不是给出关于
/// 错误对象的结论。
#[test]
fn a_bound_rule_does_not_apply_to_another_object_at_its_old_path() {
    let root = package("rebind");
    accept_rule(&root, "keep-render");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/provider","to":"root/panel","apply":true}),
    )
    .expect("the move applies");

    // A different object appears at the old path.
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.other"},"apply":true}),
    )
    .expect("the impostor is created");

    // Simulate the rule's selector being stale: the binding still names the moved object.
    let path = root.join(".xirang/rules/keep-render.json");
    let mut value = record(&root, "keep-render");
    value["scope"]["selector"] = json!("root/provider");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap())
        .expect("the stale selector");

    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("the subject is a different object than the one this rule is bound to"),
        "identity must win over the reused spelling: {listed}"
    );
    assert!(
        !listed.contains("met (structural)") && !listed.contains("violated (structural)"),
        "a different object is not decided either way by this rule: {listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **Every revision is in the history, and a concurrent writer with a stale expected revision is
/// refused.**
/// **每一次修订都在历史里，而带着过期预期修订的并发写入会被拒绝。**
#[test]
fn history_records_every_revision_and_a_stale_expectation_is_refused() {
    let root = package("history");
    accept_rule(&root, "keep-render");
    let events = history(&root, "keep-render");
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event["action"].as_str())
            .collect::<Vec<_>>(),
        vec!["proposed", "accepted"],
        "the history holds the whole life of the rule: {events:?}"
    );
    assert_eq!(
        events.last().and_then(|event| event["revision"].as_u64()),
        Some(2)
    );

    let stale = crate::mcp::rules::rules(
        &root,
        &json!({"action":"retire","rule_id":"keep-render","reason":"no longer needed","expected_revision":1,"confirm":true}),
    )
    .expect_err("a revision the writer never saw is a conflict");
    assert!(
        stale.contains("concurrent edit") && stale.contains("revision 2"),
        "{stale}"
    );
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"retire","rule_id":"keep-render","reason":"no longer needed","expected_revision":2,"confirm":true}),
    )
    .expect("the current revision is accepted");
    let events = history(&root, "keep-render");
    assert_eq!(
        events.last().and_then(|event| event["action"].as_str()),
        Some("retired"),
        "{events:?}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **A rule follows its object when the package's library target is outside `src/`.**
/// **当包的库目标住在 `src/` 之外时，规则仍跟着它的对象走。**
///
/// The logical path is the same shape as the default layout, but every physical coordinate differs.
/// A migration that reads `src/` by hand passes the default test and fails here; this pin is the
/// custom-`[lib] path` half of P2's "the move carries the knowledge" claim.
/// 逻辑路径与默认布局同形，但每一个物理坐标都不同。手工读 `src/` 的迁移能通过默认测试、却在这里失败；
/// 这条钉子就是 P2"搬动带着知识走"的自定义 `[lib] path` 那一半。
#[test]
fn a_rule_follows_its_object_under_a_custom_library_target() {
    let root = custom_library_package("move");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","stable_name":"FixtureProvider","provides":"cap.render"},"apply":true}),
    )
    .expect("the provider is created under the library target");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("the parent is created");
    adopt_policy(&root);
    accept_rule(&root, "keep-render");

    let before = record(&root, "keep-render");
    assert_eq!(
        before["object"]["path"].as_str(),
        Some("root/provider"),
        "the rule is bound before the move: {before}"
    );
    assert!(
        before["object"]["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("source/")),
        "the binding names the library target's source: {before}"
    );

    let moved = crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/provider","to":"root/panel","apply":true}),
    )
    .expect("the move applies under a custom library target");
    assert!(
        moved.contains("rule `keep-render` follows the object"),
        "the move reply names the rule it carried: {moved}"
    );

    let after = record(&root, "keep-render");
    assert_eq!(
        after["object"]["path"].as_str(),
        Some("root/panel/provider"),
        "the binding followed the logical move: {after}"
    );
    assert_eq!(
        after["scope"]["selector"].as_str(),
        Some("root/panel/provider"),
        "the selector followed too: {after}"
    );
    assert!(
        after["object"]["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("source/panel/")),
        "the binding names the new physical source: {after}"
    );
    assert_eq!(
        history(&root, "keep-render")
            .last()
            .and_then(|event| event["action"].as_str()),
        Some("moved"),
        "the history says why the revision changed"
    );
    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("met (structural)"),
        "the rule still decides against the moved object: {listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **A rule on a nested override follows its parent's logical move without its file moving.**
/// **嵌套覆盖上的规则跟随父级的逻辑搬动，而它自己的文件不动。**
///
/// An explicit-directory face (an island) owns its physical directory; a move of that parent is a
/// logical move for its descendants, so the child's `source` must not change while its `path` does.
/// A migration that rewrites both from the parent's new directory would quietly point the binding at
/// a file that does not exist.
/// 显式目录里的面（孤岛）拥有自己的物理目录；搬动这个父级对后代只是逻辑搬动，因此子面的 `source`
/// 不得改变、而 `path` 必须改变。按父级的新目录同时改写两者的迁移，会把绑定悄悄指向一个不存在的文件。
#[test]
fn a_rule_on_a_nested_override_follows_a_logical_move() {
    let root = bare_package("island");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("the parent face is created");
    // The provider goes into an explicit directory: the standard child would otherwise be placed by
    // the parent's own layout rule, which is exactly what this test must not assume.
    // 把 provider 放进显式目录：否则标准子面会按父级自己的布局规则放置，而这正是本测试不能假定的。
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","language":"rust","parent":"root/panel",
                "file":"components/provider/provider.rs",
                "fields":{"module":"provider","kind":"Provider","stable_name":"FixtureProvider","needs_registry":true,"provides":"cap.render"},"apply":true}),
    )
    .expect("the island provider is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","language":"rust","parent":"root/panel/provider",
                "fields":{"module":"child","kind":"Child","provides":"cap.child"},"apply":true}),
    )
    .expect("the nested override is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"other","kind":"Other","needs_registry":true},"apply":true}),
    )
    .expect("the destination parent is created");
    adopt_policy(&root);
    accept_rule_at(
        &root,
        "keep-child",
        "root/panel/provider/child",
        "cap.child",
        "retain cap.child",
    );

    let before = record(&root, "keep-child");
    let child_source = before["object"]["source"]
        .as_str()
        .expect("the child is bound to a source")
        .to_owned();
    assert!(
        child_source.starts_with("components/provider/"),
        "the nested override keeps the island's directory: {before}"
    );

    let moved = crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/panel/provider","to":"root/other","apply":true}),
    )
    .expect("the island parent moves");
    // The child inherited the island's directory, so the ledger never recorded it and the move
    // re-derives its identity. The rule must be reported as left behind, **not** rebound to the
    // object at the new path.
    // 子面继承了孤岛的目录，因此台账从未记录它，搬动会重新推导它的身份。规则必须被报成留在原处，
    // **不能**换绑到新路径上的对象。
    assert!(
        moved.contains("rule `keep-child` was not carried"),
        "the move says which knowledge it could not carry: {moved}"
    );
    assert!(
        moved.contains("re-derives that object's identity"),
        "and says why: {moved}"
    );

    let after = record(&root, "keep-child");
    assert_eq!(
        after["revision"],
        json!(2),
        "a rule that could not follow is not rewritten: {after}"
    );
    assert_eq!(
        after["scope"]["selector"].as_str(),
        Some("root/panel/provider/child"),
        "its selector still names the old path: {after}"
    );
    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("unknown:"),
        "the commitment is now unknown rather than silently satisfied: {listed}"
    );
    assert!(
        !listed.contains("met (structural)"),
        "and it is never reported as met after losing its object: {listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **A descendant that opted into an author-owned name keeps its identity, so its rule follows.**
/// **选择了作者稳定名的后代保持身份，因此它的规则跟得过去。**
///
/// This is the other half of the island case: the ledger never records the child, but `stable_name`
/// is the author-owned half of its identity, so the move preserves it and the rule is rewritten with
/// the new path — no stranding report.
/// 这是孤岛情形的另一半：台账从不记录子面，但 `stable_name` 是它身份里作者拥有的那一半，因此搬动保持
/// 身份、规则被改写到新路径——没有 stranded 报告。
#[test]
fn a_rule_follows_a_named_descendant_of_an_island() {
    let root = bare_package("island-named");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("the parent face is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","language":"rust","parent":"root/panel",
                "file":"components/provider/provider.rs",
                "fields":{"module":"provider","kind":"Provider","needs_registry":true,"provides":"cap.render"},"apply":true}),
    )
    .expect("the island provider is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","language":"rust","parent":"root/panel/provider",
                "fields":{"module":"child","kind":"Child","stable_name":"FixtureChild","provides":"cap.child"},"apply":true}),
    )
    .expect("the named child is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"other","kind":"Other","needs_registry":true},"apply":true}),
    )
    .expect("the destination parent is created");
    adopt_policy(&root);
    accept_rule_at(
        &root,
        "keep-child",
        "root/panel/provider/child",
        "cap.child",
        "retain cap.child",
    );

    let moved = crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/panel/provider","to":"root/other","apply":true}),
    )
    .expect("the island parent moves");
    assert!(
        moved.contains("rule `keep-child` follows the object"),
        "an author-owned name is kept across the move: {moved}"
    );
    assert!(
        !moved.contains("was not carried"),
        "nothing is stranded when the identity survives: {moved}"
    );
    let after = record(&root, "keep-child");
    assert_eq!(
        after["object"]["path"].as_str(),
        Some("root/other/provider/child"),
        "the binding followed the object: {after}"
    );
    assert_eq!(
        history(&root, "keep-child")
            .last()
            .and_then(|event| event["action"].as_str()),
        Some("moved")
    );
    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("met (structural)"),
        "the rule still decides against the moved named child: {listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **A delete names the rules it strands — in the preview and in the write.**
/// **删除会点名它留下的规则——预览与落盘都点。**
///
/// A rule bound to a deleted object cannot follow anything: the honest act is to say which
/// commitment lost its object, so the reader can retire it or propose a successor rule. Saying it
/// only after the fact would leave the rule silently `unknown` on the next listing.
/// 绑定到被删对象的规则没有可跟随的东西：诚实的动作是说清哪一条承诺失去了对象，读者才能退役它或对继任者
/// 重新提议。只在事后说，会让这条规则在下一次列举里静默地变成 `unknown`。
#[test]
fn a_delete_names_the_rules_it_strands() {
    let root = package("delete");
    accept_rule(&root, "keep-render");
    let path = "root/provider";
    let preview = crate::mcp::apply::apply(
        &root,
        &json!({"action":"delete","node":path,"confirm":true,"apply":false}),
    )
    .expect("the delete previews");
    assert!(
        preview.contains("rule `keep-render` was not carried")
            && preview.contains("this delete removed the object it is about (`root/provider`)"),
        "the preview names the commitment it would strand: {preview}"
    );
    assert!(
        root.join(".xirang/rules/keep-render.json").is_file(),
        "the preview wrote nothing"
    );

    let applied = crate::mcp::apply::apply(
        &root,
        &json!({"action":"delete","node":path,"confirm":true,"apply":true}),
    )
    .expect("the delete applies");
    assert!(
        applied.contains("rule `keep-render` was not carried"),
        "the write says the same thing as the preview: {applied}"
    );
    let after = record(&root, "keep-render");
    assert_eq!(
        after["revision"],
        json!(2),
        "a stranded rule is reported, not rewritten: {after}"
    );
    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("unknown:"),
        "the commitment is unknown after losing its object: {listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race. The write path itself is
    // what this test pins, and it has already been asserted above.
    // 清理不是断言：后台索引刷新可能在测试结束时还在往里写，`remove_dir_all` 会输掉这个竞态。本测试钉的
    // 是写入路径本身，那一点上面已经断言过了。
    let _ = std::fs::remove_dir_all(root);
}

/// **Under a custom library target, a deviating descendant keeps its identity across the move.**
/// **在自定义库目标下，本身也偏离的后代跨搬动保持身份。**
///
/// The island case's other edge: the child here has its **own** explicit directory, so the ledger
/// records it and the move can preserve its identity — the rule follows instead of being stranded.
/// This is the positive fixture the earlier slice was missing.
/// 孤岛情形的另一条边：这个子面拥有**自己的**显式目录，因此台账记录了它，搬动可以保持身份——规则跟随而不是
/// 被报成 stranded。这是上一片缺失的正例夹具。
#[test]
fn a_rule_follows_a_recorded_descendant_under_a_custom_library_target() {
    let root = custom_library_package("island-child");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("the destination parent is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","language":"rust","parent":"root",
                "file":"components/provider/provider.rs",
                "fields":{"module":"provider","kind":"Provider","needs_registry":true,"provides":"cap.render"},"apply":true}),
    )
    .expect("the island provider is created");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","language":"rust","parent":"root/provider",
                "file":"components/provider/child/child.rs",
                "fields":{"module":"child","kind":"Child","provides":"cap.child"},"apply":true}),
    )
    .expect("the deviating child is created");
    adopt_policy(&root);
    accept_rule_at(
        &root,
        "keep-child",
        "root/provider/child",
        "cap.child",
        "retain cap.child",
    );
    assert!(
        std::fs::read_to_string(root.join("xirang.objects.tsv"))
            .unwrap_or_default()
            .matches('\t')
            .count()
            >= 2,
        "the fixture records both deviating faces, which is what makes identity preservable"
    );

    let moved = crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/provider","to":"root/panel","apply":true}),
    )
    .expect("the island parent is re-parented");
    assert!(
        moved.contains("rule `keep-child` follows the object"),
        "a recorded descendant keeps its identity, so the rule follows: {moved}"
    );
    assert!(
        !moved.contains("was not carried"),
        "nothing is stranded when the ledger keeps the identity: {moved}"
    );
    let after = record(&root, "keep-child");
    assert_eq!(
        after["object"]["path"].as_str(),
        Some("root/panel/provider/child"),
        "the binding followed the descendant's logical path: {after}"
    );
    assert_eq!(
        after["scope"]["selector"].as_str(),
        Some("root/panel/provider/child"),
        "the selector followed too: {after}"
    );
    let source = after["object"]["source"]
        .as_str()
        .expect("the binding has a source");
    assert!(
        root.join(source).is_file(),
        "the binding names a file that exists after the move: {source}"
    );
    let listed = crate::mcp::rules::rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("met (structural)"),
        "the rule still decides against the moved descendant: {listed}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A lease about a face stops backing new rules once another object holds that anchor.**
/// **关于一个面的租约，在另一个对象占据该锚点后不再为新规则背书。**
///
/// The adoption ledger now records which object an anchor named (the additive `# object` line). The
/// consumer is the rule book: accepting a commitment against a lapsed anchor is refused, so an old
/// decision cannot silently back a commitment about an object it was never about.
/// 采信台账现在记下锚点点名的是哪个对象（加法式 `# object` 行）。消费者是规则簿：对着失效锚点接受承诺
/// 会被拒绝，因此旧决定不能悄悄为一条"它从未针对过的对象"的承诺背书。
#[test]
fn an_adoption_about_a_face_stops_backing_rules_when_another_object_holds_it() {
    let root = package("adopt-object");
    crate::mcp::adopted::adopted(
        &root,
        &json!({"anchor":"root/provider","certifies":"the provider contract","evidence":"desk check",
                "verifier":"identity test","reason":"the provider is the certified route",
                "files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the face-anchored adoption applies");
    let ledger_path = root.join(".xirang/adopted/entries");
    let ledger = std::fs::read_to_string(&ledger_path)
        .unwrap_or_else(|error| panic!("the ledger at {}: {error}", ledger_path.display()));
    assert!(
        ledger.contains("# object root/provider |"),
        "the decision records which object the anchor named:\n{ledger}"
    );

    // A rule accepted against the same anchor is in force while the object is the one it was about.
    // 在对象仍是那个对象时，对着同一锚点接受的规则是生效的。
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"propose","rule_id":"adopted-rule","category":"protocol","selector":"root/provider",
                "requirement":"retain cap.render","origin":"accepted_policy","sources":["policy.txt"],
                "expects":{"provides":"cap.render"}}),
    )
    .expect("the rule is proposed");
    let accepted = crate::mcp::rules::rules(
        &root,
        &json!({"action":"accept","rule_id":"adopted-rule","authority":"root/provider","confirm":true}),
    );
    assert!(accepted.is_ok(), "the lease is in force: {accepted:?}");

    // Move the certified face away, then let a **different** object occupy the old anchor path.
    // 把被采信的面搬走，再让一个**不同的**对象占据旧锚点路径。
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/provider","to":"root/panel","apply":true}),
    )
    .expect("the certified face moves");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.other"},"apply":true}),
    )
    .expect("a different object takes the old path");

    // Accepting another commitment against that anchor must now be refused, by name.
    // 对着那个锚点接受另一条承诺现在必须被按名拒绝。
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"propose","rule_id":"after-swap","category":"protocol","selector":"root/provider",
                "requirement":"retain cap.other","origin":"accepted_policy","sources":["policy.txt"],
                "expects":{"provides":"cap.other"}}),
    )
    .expect("the second rule is proposed");
    let refused = crate::mcp::rules::rules(
        &root,
        &json!({"action":"accept","rule_id":"after-swap","authority":"root/provider","confirm":true}),
    )
    .expect_err("a different object now holds the anchor");
    assert!(
        refused.contains("no longer in force") && refused.contains("different object now holds"),
        "the refusal names the object change: {refused}"
    );
    let _ = std::fs::remove_dir_all(root);
}
