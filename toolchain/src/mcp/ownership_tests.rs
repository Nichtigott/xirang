//! Tests for the ownership entrance: on a virtual workspace root, which member answers.
//! 归属入口的测试：在虚拟工作区根上，由哪个成员作答。
//!
//! The measured failure these exist against is the refusal the other nine tools gave at a
//! workspace root — `cannot learn the identity namespace of …`, in Cargo's own words —
//! while `registry`/`grafts`/`diff`/`search` already answered there. The nails are that a
//! face-naming tool asks **every** member and groups the answers, that a path-naming tool
//! is answered by the member whose root contains the path, and that the write path refuses
//! with its candidates instead of guessing. The mutation they are aimed at is "answer from
//! the first member": then the census, the other members' sections and the named decliner
//! all disappear, and every assertion below goes red.
//! 这些测试所针对的实测失败，是其余九个工具在工作区根上给出的拒绝——用 Cargo 自己的话
//! `cannot learn the identity namespace of …`——而 `registry`/`grafts`/`diff`/`search` 在那里已经能答。
//! 钉子有三：点名面的工具会问**每一个**成员并把答案分组；点名路径的工具由根包含该路径的成员回答；
//! 写入路径带着候选拒绝而不是猜。它们瞄准的变异是"取第一个成员作答"：那样普查、其余成员的小节与被点名的
//! 拒绝者都会消失，下面每一条断言都会变红。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// A throwaway virtual workspace whose three members cover the three statuses.
/// 一个一次性虚拟工作区，三个成员覆盖三种状态。
struct Workspace {
    root: PathBuf,
    /// A host member: it declares one face at `root/button`.
    /// 宿主成员：在 `root/button` 声明一个面。
    host: String,
    /// A framework member: a library with no registration face at all.
    /// 框架成员：一个完全没有注册面的库。
    framework: String,
    /// A binary-only member with no `src/`: its tree cannot be derived.
    /// 一个没有 `src/`、只有二进制的成员：它的树推导不出来。
    tool: String,
}

impl Drop for Workspace {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Write one fixture file, creating its directory.
/// 写一个夹具文件并建好它的目录。
fn write_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    std::fs::write(path, text).expect("fixture file");
}

/// The fixture workspace, under a label that keeps concurrent tests apart.
/// 夹具工作区，用一个把并发测试彼此分开的标签。
fn workspace(label: &str) -> Workspace {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-ownership-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let name = |member: &str| format!("mcp-own-{label}-{member}");
    let manifest = |member: &str| {
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            name(member)
        )
    };
    write_file(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"host\", \"framework\", \"tool\"]\nresolver = \"2\"\n",
    );
    write_file(&root.join("host/Cargo.toml"), &manifest("host"));
    write_file(&root.join("host/src/lib.rs"), "// host entry\n");
    write_file(
        &root.join("host/src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: \
         crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write_file(&root.join("framework/Cargo.toml"), &manifest("framework"));
    write_file(&root.join("framework/src/lib.rs"), "// framework entry\n");
    // A package whose only target is a binary at the package root: Cargo resolves it as a
    // member, and the face derivation has no `src/` to read — the reachable
    // `unresolvable` member whose degradation has to appear in the body.
    // 唯一目标是包根下一个二进制的包：Cargo 把它解析为成员，而面的推导没有 `src/` 可读——正是可达的
    // `unresolvable` 成员，它的降级必须出现在正文里。
    write_file(
        &root.join("tool/Cargo.toml"),
        &format!(
            "{}autobins = false\n\n[[bin]]\nname = \"{}\"\npath = \"main.rs\"\n",
            manifest("tool"),
            name("tool")
        ),
    );
    write_file(&root.join("tool/main.rs"), "fn main() {}\n");
    Workspace {
        root,
        host: name("host"),
        framework: name("framework"),
        tool: name("tool"),
    }
}

/// One tool call, as the protocol would deliver it.
/// 一次工具调用，按协议交付的样子。
fn call(root: &Path, name: &str, arguments: Value) -> (String, bool) {
    let reply = crate::mcp::tools::tool_call(
        root,
        json!(1),
        &json!({"name": name, "arguments": arguments}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{name} returned no text: {reply}"))
        .to_owned();
    let failed = reply["result"]["isError"].as_bool().unwrap_or(false);
    (text, failed)
}

/// A handler that answers with the root and the `path` it was handed.
/// 一个用交给它的根与 `path` 作答的处理函数。
fn echo(root: &Path, arguments: &Value) -> Result<String, String> {
    Ok(format!(
        "body root={} path={}\n",
        root.display(),
        arguments
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("<none>")
    ))
}

/// Every file under `root`, as relative paths, so "nothing was written" is checkable.
/// `root` 之下的每个文件，以相对路径表示，好让"什么都没写"可被检查。
fn files(root: &Path) -> Vec<String> {
    fn walk_files(directory: &Path, root: &Path, found: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_files(&path, root, found);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.push(relative.display().to_string());
            }
        }
    }
    let mut found = Vec::new();
    walk_files(root, root, &mut found);
    found.sort();
    found
}

/// The first nail: the four face-naming tools answer a virtual root with a section per
/// member, a census, and the unresolvable member's degradation in the body.
/// 第一枚钉子：四个点名面的工具在虚拟根上按成员分节作答，附一份普查，而不可解析成员的降级出现在正文里。
///
/// A "first member wins" implementation loses the census, the sections after the first,
/// and the named decliner — which is exactly what this test names when it fails.
/// "第一个成员胜出"的实现会丢掉普查、第一个之后的小节，以及被点名的拒绝者——而这正是这条测试失败时
/// 会点名指出的东西。
#[test]
fn a_face_naming_tool_answers_every_member_on_a_virtual_root() {
    let fixture = workspace("nodes");
    for tool in [
        "xirang.impact",
        "xirang.usages",
        "xirang.explain",
        "xirang.verify",
    ] {
        // `verify` takes no subject; the other three name the host member's one face.
        let arguments = if tool == "xirang.verify" {
            json!({})
        } else {
            json!({"node": "root/button"})
        };
        // Each tool is measured as the answer that **opens** the root, so this pin is about which
        // roster shape this entrance composes rather than about the order the loop happens to call
        // in: after the first answer the preamble is allowed to collapse to its one-line index
        // (t1's decision), and a pin that read that shape would be asserting the call order.
        // 每个工具都按**打开**这个根的那份答案来量，因此这条钉子量的是本入口构成哪种表头形态，而不是这个
        // 循环碰巧的调用次序：第一条答案之后，前言允许塌成它那一行索引（t1 的决定），而读到那个形态的钉子
        // 断言的就是调用次序了。
        crate::mcp::workspace::forget_announced();
        let (text, failed) = call(&fixture.root, tool, arguments);
        assert!(
            !failed,
            "{tool} answers about the workspace instead of failing: {text}"
        );
        assert!(
            text.contains("members 3  queried 1  no faces 1  unresolvable 1"),
            "{tool} must carry the member census — the members it must name are `{}`, `{}` and \
             `{}`: {text}",
            fixture.host,
            fixture.framework,
            fixture.tool
        );
        assert!(
            text.contains("answers: 1 of 3 members answered this request  declined 1  no tree 1")
                || text.contains(
                    "answers: 2 of 3 members answered this request  declined 0  no tree 1"
                ),
            "{tool} must count who answered, who declined and who has no tree: {text}"
        );
        for member in [&fixture.host, &fixture.framework, &fixture.tool] {
            assert!(
                text.contains(member.as_str()),
                "{tool} dropped `{member}` from its answer: {text}"
            );
            assert!(
                text.contains(&format!("== {member} (")),
                "{tool} gave `{member}` no section of its own: {text}"
            );
        }
        assert!(
            text.contains(&format!("== {} (unresolvable)\n", fixture.tool))
                && text.contains("tree unavailable (no source tree at"),
            "{tool} must explain the member whose tree it could not read: {text}"
        );
        // The **rows** carry the reason too, which is the decision this batch took: the ownership
        // entrance composes one roster spelling, and it is the expanded one, so `explain`/`verify`
        // name each member's degradation exactly where `registry`/`diff`/`grafts` do. The padded
        // row is asserted rather than the bare sentence, because the member's section carries the
        // same sentence and would make a weaker assertion pass either way.
        // **行**里也带原因，这是本批做出的决定：归属入口只拼一种表头拼法，而且是展开的那一种，因此
        // `explain`/`verify` 点名每个成员的降级的位置与 `registry`/`diff`/`grafts` 完全相同。断言的是
        // 带对齐的那一行而不是光秃秃的那句话，因为成员小节里也有同一句话，更弱的断言两边都会通过。
        assert!(
            text.contains(&format!(
                "  {:<13} {:<30} tree unavailable (no source tree at",
                "unresolvable", fixture.tool
            )),
            "{tool}'s roster row must carry the reason the member could not be read: {text}"
        );
        assert!(
            text.contains("; not built (cannot read"),
            "{tool}'s not-built row must carry the reason it was derived now: {text}"
        );
        // The member that derived its tree but whose own body refused is named with its
        // reason, so "1 of 3" is actionable rather than a number.
        // 推导出了树、而主体拒绝的成员被连原因一起点名，因此"三个里一个"是可据以行动的东西而不是一个数字。
        if tool != "xirang.verify" {
            assert!(
                text.contains(&format!("declined {}:", fixture.framework))
                    && text.contains("no registration face at"),
                "{tool} must name the member that declined and why: {text}"
            );
            assert!(
                !text.contains("no member owns `root/button`"),
                "{tool} did answer from one member, so it must not claim none owns the face: {text}"
            );
        }
        assert!(
            text.contains(crate::mcp::workspace::DETAIL),
            "{tool} must say how to reach one package's own answer: {text}"
        );
    }
}

/// The second nail: a node no member's tree declares says exactly that, and names who
/// declined, rather than reporting a missing identity namespace.
/// 第二枚钉子：没有任何成员的树声明的节点会明确说出这一点，并点名谁拒绝了，而不是报告缺少身份命名空间。
#[test]
fn a_node_no_member_owns_says_so() {
    let fixture = workspace("unowned");
    let (text, failed) = call(
        &fixture.root,
        "xirang.impact",
        json!({"node": "root/nowhere"}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains("no member owns `root/nowhere`"),
        "the answer must say no member owns it — neither `{}` nor `{}` does: {text}",
        fixture.host,
        fixture.framework
    );
    for member in [&fixture.host, &fixture.framework] {
        assert!(
            text.contains(&format!("declined {member}:")),
            "every member that declined is named with its reason: {text}"
        );
    }
    assert!(
        !text.contains("cannot learn the identity namespace"),
        "a refusal is not the answer here: {text}"
    );
}

/// The third nail: a path-naming tool is answered by the member whose root contains the
/// path, from that member's own root, and the path it reads is relative to it.
/// 第三枚钉子：点名路径的工具由根包含该路径的成员、在该成员自己的根之下作答，而它读取的路径是相对该成员的。
#[test]
fn a_path_naming_tool_is_answered_by_the_member_that_owns_it() {
    let fixture = workspace("paths");
    let (text, failed) = call(
        &fixture.root,
        "xirang.read",
        json!({"path": "host/src/lib.rs"}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains(&format!("owner {} owns `host/src/lib.rs`", fixture.host)),
        "the owning member must be named: {text}"
    );
    assert!(
        text.contains("src/lib.rs:1-1 (1 lines)"),
        "the body is the member's own single-package answer, from its own root: {text}"
    );
    assert!(
        !text.contains("host/src/lib.rs:1-1"),
        "the path is rewritten relative to the member it is read in: {text}"
    );
}

/// A path no member owns says so, and lists the directories that were compared.
/// 没有成员拥有的路径会说出来，并列出被比较过的那些目录。
#[test]
fn a_path_no_member_owns_says_so() {
    let fixture = workspace("no-owner");
    let (text, failed) = call(
        &fixture.root,
        "xirang.read",
        json!({"path": "nowhere/src/lib.rs"}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains("no member owns `nowhere/src/lib.rs`"),
        "{text}"
    );
    assert!(text.contains(&fixture.host), "{text}");
    assert!(
        !text.contains("cannot learn the identity namespace"),
        "{text}"
    );
}

/// Two named paths that resolve to two different members are refused by name, because a
/// merge across two packages would compare two different trees.
/// 两条点名了不同成员的路径会被按名字拒绝，因为跨两个包的合并会比较两棵不同的树。
#[test]
fn paths_from_two_members_are_refused_by_name() {
    let fixture = workspace("ambiguous");
    let (text, failed) = crate::mcp::ownership::dispatch(
        &fixture.root,
        "xirang.mir",
        &json!({"path": "host/mir.jsonl", "against": "framework/baseline.jsonl"}),
        echo,
    )
    .map(|text| (text, false))
    .unwrap_or_else(|error| (error, true));
    assert!(!failed, "{text}");
    assert!(text.contains("resolves in 2 members"), "{text}");
}

/// The ownership decision itself, exercised through the entrance rather than through a
/// tool: the body is handed the owning member's root and a path relative to it.
/// 归属判断本身，经入口而不是经某个工具来演练：主体拿到的是拥有者的根，以及相对它的路径。
#[test]
fn the_owning_member_gets_its_own_root_and_a_relative_path() {
    let fixture = workspace("redirect");
    let text = crate::mcp::ownership::dispatch(
        &fixture.root,
        "xirang.read",
        &json!({"path": "host/src/button/button.rs"}),
        echo,
    )
    .expect("the request resolves");
    assert!(
        text.contains(&format!("owner {} owns", fixture.host)),
        "{text}"
    );
    assert!(text.contains("path=src/button/button.rs"), "{text}");
    assert!(
        text.contains(&format!(
            "body root={}",
            fixture.root.join("host").display()
        )),
        "{text}"
    );
}

/// The fourth nail: `apply` on a virtual root resolves a unique owner or refuses with its
/// candidates — and when it refuses, **nothing is written**.
/// 第四枚钉子：虚拟根上的 `apply` 要么解析出唯一拥有者，要么带着候选拒绝——而它拒绝时，**什么都不写**。
#[test]
fn apply_refuses_a_virtual_root_without_writing() {
    let fixture = workspace("apply");
    let before = files(&fixture.root);
    let (text, failed) = call(
        &fixture.root,
        "xirang.apply",
        json!({"action": "add", "apply": true, "fields": {"module": "src/gadget/gadget.rs", "kind": "Gadget"}}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains("REFUSED: xirang.apply needs one package"),
        "the write path says why it will not run: {text}"
    );
    assert!(
        text.contains("nothing was copied and nothing was written"),
        "{text}"
    );
    assert!(
        text.contains("pass `root` as one of these member directories"),
        "the refusal must say how to name a member: {text}"
    );
    for member in [&fixture.host, &fixture.framework, &fixture.tool] {
        assert!(
            text.contains(member.as_str()),
            "candidate `{member}` missing: {text}"
        );
    }
    assert_eq!(
        files(&fixture.root),
        before,
        "a refused write must not touch the tree"
    );
}

/// A request whose subject resolves in exactly one member is answered by that member, and
/// the answer says which one — so a write never lands where the caller did not see.
/// 主体恰好只在一个成员里解析的请求由那个成员作答，而答案说出是哪一个——因此写入绝不会落在调用方没有
/// 看见的地方。
#[test]
fn a_write_subject_with_one_owner_is_answered_by_it() {
    let fixture = workspace("unique");
    let text = crate::mcp::ownership::dispatch(
        &fixture.root,
        "xirang.apply",
        &json!({"action": "add", "parent": "root/button", "fields": {"module": "src/gadget/gadget.rs"}}),
        echo,
    )
    .expect("the request resolves");
    assert!(
        text.contains(&format!("owner {} owns `root/button`", fixture.host)),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "body root={}",
            fixture.root.join("host").display()
        )),
        "the body runs in the owning member: {text}"
    );
}

/// A subject that resolves in no member's tree, and one that resolves in several, are both
/// refusals — the write path never guesses which tree was meant.
/// 在任何成员的树里都解析不出的主体，以及在好几个成员里都解析得出的主体，都是拒绝——写入路径绝不猜指的是
/// 哪一棵树。
#[test]
fn an_unresolvable_or_ambiguous_write_subject_is_refused() {
    let fixture = workspace("write-candidates");
    let (text, failed) = call(
        &fixture.root,
        "xirang.apply",
        json!({"action": "delete", "node": "root/nothing-here", "confirm": true, "apply": true}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains("no member's tree declares `root/nothing-here`"),
        "{text}"
    );
    assert!(
        text.contains("nothing was copied and nothing was written"),
        "{text}"
    );
    let (text, failed) = call(
        &fixture.root,
        "xirang.apply",
        json!({"action": "add", "parent": "root", "fields": {"module": "src/gadget/gadget.rs"}, "apply": true}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains("`root` is declared in 2 members"),
        "the member root is spelled the same in every member, so it is no unique owner: {text}"
    );
    assert!(
        text.contains("nothing was copied and nothing was written"),
        "{text}"
    );
}

/// A member root is still one package: pointing `root` at a member keeps the answer the
/// same as before, with no census and no owner line above it.
/// 成员根本来就还是一个包：把 `root` 指向成员时答案与从前相同，上面没有普查、也没有归属行。
#[test]
fn a_member_root_still_answers_as_one_package() {
    let fixture = workspace("member");
    let (text, failed) = call(
        &fixture.root.join("host"),
        "xirang.read",
        json!({"path": "src/lib.rs"}),
    );
    assert!(!failed, "{text}");
    assert!(text.starts_with("src/lib.rs:1-1 (1 lines)"), "{text}");
    assert!(!text.contains("workspace "), "{text}");
    assert!(!text.contains("owner "), "{text}");
}

/// The catalog is the contract: every tool this entrance resolves has to say so, because
/// an agent that is not told a workspace root works there will not try it.
/// 目录就是契约：每个由本入口解析的工具都必须说出来，因为没被告知"工作区根也行"的代理不会去试。
/// A name lookup at a workspace root is answered **once**, over every member's sources: the
/// caller lives in one member and the definition in another, so a per-member fan-out could not
/// see it. Measured on the real tree before this pin existed: one symbol had 17 callers when
/// each member searched only itself and 47 once the lookup crossed members.
/// 工作区根上的名字查询**一次**遍历每个成员的源码：调用者住在一个成员里、定义在另一个成员里，因此逐成员
/// 扇出看不到它。这条钉子出现之前在真树上的实测：每个成员只搜自己时同一个符号 17 个调用者，跨成员查找后 47 个。
#[test]
fn a_workspace_name_lookup_crosses_members_in_one_answer() {
    let fixture = workspace("callgraph-global");
    write_file(
        &fixture.root.join("framework/src/shared.rs"),
        "pub fn shared_helper() {}\n",
    );
    write_file(
        &fixture.root.join("host/src/caller.rs"),
        "pub fn host_caller() {\n    shared_helper();\n}\n",
    );
    let (text, failed) = call(
        &fixture.root,
        "xirang.callgraph",
        json!({"function": "shared_helper"}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains("host/src/caller.rs::host_caller"),
        "the cross-member caller is the answer this pin exists for: {text}"
    );
    assert!(
        !text.contains("no static function match"),
        "one global answer rather than one per member: {text}"
    );
}

#[test]
fn the_resolved_tools_say_a_virtual_root_answers() {
    let listed = crate::mcp::tools::tools();
    for name in [
        "xirang.read",
        "xirang.inspect",
        "xirang.mir",
        "xirang.unified",
        "xirang.callgraph",
        "xirang.explain",
        "xirang.trace",
        "xirang.impact",
        "xirang.usages",
        "xirang.converge",
        "xirang.verify",
        "xirang.apply",
        "xirang.new_project",
        "xirang.plugin",
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        let description = tool["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} has no description: {tool}"));
        assert!(
            description.contains("virtual workspace root") || description.contains("virtual root"),
            "{name} must say a virtual workspace root is answered: {description}"
        );
    }
}
