//! Tests for the two bounds `xirang.callgraph` needs: definitions and callers.
//! `xirang.callgraph` 需要的两道上限的测试：定义数与调用者数。
//!
//! The measured failure these exist against is the one `callgraph.rs`'s module doc
//! records: a common name (`new`) matched definitions throughout a real-tree corpus,
//! and every call site of that name was listed for each of them, which arrived as a
//! 4.5 MB reply. An answer that an agent cannot read is not an answer, so both bounds
//! are pinned here.
//! 这些测试所针对的实测失败就是 `callgraph.rs` 模块文档记下的那一条：常见名（`new`）在真实语料里
//! 匹配到许多定义，而每个定义都列出该名字的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的
//! 答案不算答案，因此两道上限都钉在这里。

use std::path::PathBuf;

use serde_json::json;

#[test]
fn catalogue_executes_a_discovered_tool_through_the_advertised_entry() {
    let root = package("gateway");
    let direct = super::tool_call(
        &root,
        json!(1),
        &json!({
            "name": "xirang.read", "arguments": {"path": "src/lib.rs"}
        }),
    );
    let routed = super::tool_call(
        &root,
        json!(1),
        &json!({
            "name": "xirang_tools", "arguments": {
                "tool": "read", "arguments": {"path": "src/lib.rs"}
            }
        }),
    );
    assert_eq!(routed, direct);
    for arguments in [
        json!({"arguments": {}}),
        json!({"tool": "read", "arguments": []}),
        json!({"tool": "xirang_tools", "arguments": {}}),
        json!({"tool": "read", "arguments": {"path": "../outside.rs"}}),
    ] {
        let reply = super::tool_call(
            &root,
            json!(2),
            &json!({
                "name": "xirang_tools", "arguments": arguments
            }),
        );
        assert_eq!(reply["result"]["isError"], true, "{reply}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn advertised_schemas_keep_the_complete_call_contract() {
    fn without_descriptions(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("description");
                for value in map.values_mut() {
                    without_descriptions(value);
                }
            }
            serde_json::Value::Array(items) => {
                for value in items {
                    without_descriptions(value);
                }
            }
            _ => {}
        }
    }
    let catalogue = super::tools();
    for advertised in super::advertised() {
        let full = catalogue
            .iter()
            .find(|entry| entry["name"] == advertised["name"])
            .unwrap();
        let mut actual = advertised["inputSchema"].clone();
        let mut expected = full["inputSchema"].clone();
        without_descriptions(&mut actual);
        without_descriptions(&mut expected);
        assert_eq!(actual, expected, "{}", advertised["name"]);
    }
}

#[test]
fn catalogue_writes_keep_preview_confirmation_and_root_rules() {
    let root = package("gateway-write");
    let call = |args| {
        super::tool_call(
            &root,
            json!(1),
            &json!({
                "name":"xirang_tools", "arguments":{"tool":"new_project", "arguments":args}
            }),
        )
    };
    let mut args = json!({"directory":"app", "package":"gateway-app", "kind":"binary"});
    let preview = call(args.clone());
    assert_eq!(preview["result"]["isError"], false, "{preview}");
    assert!(!root.join("app").exists());
    std::fs::create_dir(root.join("app")).unwrap();
    args["apply"] = json!(true);
    let refused = call(args.clone());
    assert_eq!(refused["result"]["isError"], true, "{refused}");
    assert!(!root.join("app/Cargo.toml").exists());
    args["confirm"] = json!(true);
    let applied = call(args);
    assert_eq!(applied["result"]["isError"], false, "{applied}");
    assert!(root.join("app/Cargo.toml").is_file());
    let outside = call(json!({"root":"..", "directory":"escape", "package":"escape"}));
    assert_eq!(outside["result"]["isError"], true, "{outside}");
    let _ = std::fs::remove_dir_all(root);
}

/// A throwaway package: `cargo metadata` can name it, so the tools that need a
/// namespace get one, and it holds no face so every evidence answer is the empty
/// one.
/// 一个一次性包：`cargo metadata` 能给它命名，因此需要命名空间的工具能得到一个；它不含任何面，
/// 所以每个证据答案都是空的那一种。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-dispatch-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    root
}

/// The three evidence tools must be advertised with the argument that narrows
/// each one, because an agent can only call what `tools/list` names.
/// 证据工具必须被列出，并且带上各自缩小范围的那个参数，因为代理只能调用 `tools/list` 点名的东西。
#[test]
fn the_evidence_tools_are_advertised_with_their_narrowing_arguments() {
    let listed = super::tools();
    for (name, key) in [
        ("xirang.explain", "node"),
        ("xirang.explain", "overlay"),
        ("xirang.callgraph", "limit"),
        ("xirang.diff", "limit"),
        ("xirang.trace", "query"),
        ("xirang.impact", "node"),
        ("xirang.grafts", "limit"),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        assert!(
            tool["inputSchema"]["properties"].get(key).is_some(),
            "{name} does not advertise `{key}`: {tool}"
        );
    }
}

/// `xirang.impact` is wired to its implementation too, and asking it for a radius
/// with no face named is an error response rather than a silent empty one.
/// `xirang.impact` 同样接到了实现上，而没点名任何面就要半径会得到错误响应，而不是一份静默的空答案。
#[test]
fn the_impact_tool_refuses_a_call_without_a_node() {
    let root = package("impact-dispatch");
    let reply = super::tool_call(
        &root,
        json!(1),
        &json!({"name": "xirang.impact", "arguments": {}}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text: {reply}"));
    assert!(text.contains("requires node"), "{text}");
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The dispatch table and the catalog list the same tools in the same order, so the
/// order is a machine-checked fact rather than an eyeballed one. The failure this
/// prevents: `xirang.apply` sat sixth in the catalog and last in the dispatch, and
/// nothing noticed (audit `BR-12`).
/// 分派表与目录以同样的顺序列出同样的工具，因此这个顺序是被机器检查的事实，而不是靠眼睛对的。
/// 它防止的失败：`xirang.apply` 在目录里排第 6、在分派里排最后，而没有任何东西发现（审计 `BR-12`）。
#[test]
fn the_dispatch_table_follows_the_catalog() {
    let catalog = super::tools();
    let listed: Vec<&str> = catalog
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .unwrap_or_else(|| panic!("a catalog entry without a name: {tool}"))
        })
        .collect();
    let dispatched: Vec<&str> = super::DISPATCH.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        listed, dispatched,
        "the catalog and the dispatch table must name the same tools in the same order"
    );
}

/// Every name the catalog lists is routed somewhere: a tool `tools/list`
/// advertises but the dispatch has no arm for answers `unknown tool`, which a
/// client only discovers when it calls it. Before this test the two lists were
/// complete but nothing kept them so — adding a tool to the catalog alone was
/// silent (audit `BR-12`).
/// 目录列出的每个名字都被路由到某处：`tools/list` 声明而分派没有对应臂的工具会回答
/// `unknown tool`，而客户端只有调用时才发现。在这条测试之前两张名单是齐全的，但没有任何东西维持
/// 这一点——只往目录里加一个工具是静默的（审计 `BR-12`）。
#[test]
fn every_listed_tool_is_dispatched() {
    let root = package("every-tool");
    for tool in super::tools() {
        let name = tool["name"]
            .as_str()
            .unwrap_or_else(|| panic!("a catalog entry without a name: {tool}"));
        let reply = super::tool_call(&root, json!(1), &json!({"name": name, "arguments": {}}));
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} returned no text: {reply}"));
        assert!(
            !text.contains("unknown tool"),
            "{name} is advertised but not dispatched: {reply}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The description of `xirang.usages` names every field the implementation
/// prints. It stopped at `runtime checks` while `usages.rs` also printed
/// `module`, `stable_name`, `getting_from_other_registry`, `flow_provider`, and
/// `needs_registry` — so an agent asking "what can I set" got a shorter answer
/// than the tool's own contract (audit `BR-7`).
/// `xirang.usages` 的描述点名实现打印的每一个字段。它停在 `runtime checks`，而 `usages.rs`
/// 还打印 `module`、`stable_name`、`getting_from_other_registry`、`flow_provider` 与
/// `needs_registry`——于是一个问"我能设什么"的代理拿到的答案比这个工具自己的契约更短
/// （审计 `BR-7`）。
#[test]
fn the_usages_description_names_every_field_it_prints() {
    let listed = super::tools();
    let usages = listed
        .iter()
        .find(|tool| tool["name"] == "xirang.usages")
        .expect("xirang.usages is advertised");
    let description = usages["description"]
        .as_str()
        .unwrap_or_else(|| panic!("xirang.usages has no description: {usages}"));
    for field in [
        "module",
        "preset",
        "parts",
        "name_zh",
        "name_en",
        "summary_zh",
        "summary_en",
        "stable_name",
        "exports",
        "requires",
        "provides",
        "handle_traits",
        "handle_contracts",
        "part_traits",
        "part_contracts",
        "registration_rule",
        "admission",
        "flow",
        "flow_provider",
        "runtime_checks",
        "getting_from_other_registry",
        "needs_registry",
    ] {
        assert!(
            description.contains(field),
            "`{field}` is read back by xirang.usages and must be named in its description: {description}"
        );
    }
}

/// The description of `xirang.verify` names **both** verdicts it prints.
/// `xirang.verify` 的描述点名它打印的**两个**判断。
///
/// The second line (`connector verdict: ok|rejected`) was added because a tree can pass
/// the static verdict and be refused by the authoring connector (audit `F1`), and the
/// description kept promising one verdict — so an agent read a green light on a tree the
/// other surface refuses.
/// 第二行（`connector verdict: ok|rejected`）之所以存在，是因为一棵树可以通过静态判断而被创作
/// 连接器拒绝（审计 `F1`），而描述一直只承诺一个判断——于是代理在一棵被另一个面拒绝的树上读到了
/// 绿灯。
#[test]
fn the_verify_description_names_the_connector_verdict() {
    let listed = super::tools();
    let verify = listed
        .iter()
        .find(|tool| tool["name"] == "xirang.verify")
        .expect("xirang.verify is advertised");
    let description = verify["description"]
        .as_str()
        .unwrap_or_else(|| panic!("xirang.verify has no description: {verify}"));
    for expected in [
        "connector verdict: ok",
        "connector verdict: rejected",
        "authoring connector",
        "isError",
    ] {
        assert!(
            description.contains(expected),
            "`{expected}` is part of what verify prints and must be named in its description: {description}"
        );
    }
}

/// `xirang.mir` refuses a snapshot it cannot print whole, and its description says so.
/// `xirang.mir` 拒绝它无法整体打印的快照，而它的描述把这一点说出来。
///
/// The writer's promise is "what it prints reads back here" (audit `X2`), so the one case
/// where it prints nothing has to be declared rather than discovered by a parse error.
/// 写入方的承诺是"它打印出来的东西能在这里读回来"（审计 `X2`），因此它唯一什么都不打印的那种情形
/// 必须被声明出来，而不是靠一条解析错误去发现。
#[test]
fn the_mir_description_declares_the_refusal_above_the_reply_cap() {
    let listed = super::tools();
    let mir = listed
        .iter()
        .find(|tool| tool["name"] == "xirang.mir")
        .expect("xirang.mir is advertised");
    let description = mir["description"]
        .as_str()
        .unwrap_or_else(|| panic!("xirang.mir has no description: {mir}"));
    assert!(
        description.contains("reads back here"),
        "the promise is what makes the refusal necessary: {description}"
    );
    assert!(
        description.contains("reply cap") && description.contains("refused by name"),
        "the one case that prints nothing must be declared: {description}"
    );
}

/// A package whose own faces the connector refuses: a provider, and a consumer whose
/// requirement no face answers. `face_views` still derives both — that divergence is the
/// whole point (audit `F1`) — so a read tool has a node to name.
/// 一个连接器拒绝其自身面的包：一个提供者，以及一个需求无人满足的消费者。`face_views` 仍然把两者
/// 都推导出来——这处分歧正是要点（审计 `F1`）——因此读工具有节点可点名。
fn rejected_package(label: &str) -> PathBuf {
    let root = package(label);
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn wire() {\n    crate::label::Label;\n}\n",
    )
    .expect("the entry references one face");
    std::fs::create_dir_all(root.join("src/label")).expect("label module");
    std::fs::write(
        root.join("src/label/label.rs"),
        "crate::root_object! { kind: Label, \
         parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")), provides: [\"cap.render\"], }\n",
    )
    .expect("the provider face");
    std::fs::create_dir_all(root.join("src/widget")).expect("widget module");
    std::fs::write(
        root.join("src/widget/widget.rs"),
        "crate::root_object! { kind: Widget, \
         parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")), \
         requires: [\"cap.theme\" => \"Theme\"], }\n",
    )
    .expect("the face whose requirement nothing answers");
    root
}

/// The connector's rejection of a package's own faces is one fact about that tree, so
/// both read tools answer it the same way: the verdict spelled once, in the body, with
/// `isError` false. `usages` used to return it as an error while `converge` printed it, so
/// an agent routing on `isError` read the same rejection as a failure and as an ordinary
/// answer (audit `A2`).
/// 连接器对一个包自身面的拒绝是关于那棵树的同一个事实，因此两个读工具以同一种方式作答：那句话只
/// 拼一次、出现在正文里，`isError` 为 false。`usages` 过去把它当错误返回，而 `converge` 把它打印
/// 出来，于是按 `isError` 分流的代理把同一次拒绝读成了失败、也读成了普通答案（审计 `A2`）。
#[test]
fn a_connector_rejection_is_the_same_answer_from_usages_and_converge() {
    let root = rejected_package("a2");
    for tool in ["xirang.usages", "xirang.converge"] {
        let reply = super::tool_call(
            &root,
            json!(1),
            &json!({"name": tool, "arguments": {"node": "root/label"}}),
        );
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{tool} returned no text: {reply}"));
        assert_eq!(
            reply["result"]["isError"], false,
            "{tool} answers about the tree instead of failing: {reply}"
        );
        assert!(
            text.contains(crate::mcp::apply::REJECTED_VERDICT),
            "{tool} must print the one verdict spelling: {text}"
        );
        assert!(
            text.contains("input `cap.theme` has no provider"),
            "{tool} must carry the diagnostic that names the cause: {text}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Every new name is wired to its implementation, not only to the catalog.
/// 每个新名字都接到了实现上，而不只是接进目录。
#[test]
fn the_evidence_tools_are_dispatched_to_their_implementations() {
    let root = package("dispatch");
    for (name, expected) in [
        ("xirang.explain", "scope unknown"),
        ("xirang.diff", "no build evidence"),
        ("xirang.trace", "trace absent"),
        ("xirang.grafts", "no external graft plans"),
    ] {
        let reply = super::tool_call(&root, json!(1), &json!({"name": name, "arguments": {}}));
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} returned no text: {reply}"));
        assert!(text.contains(expected), "{name}: {text}");
        assert_eq!(reply["result"]["isError"], false, "{name}: {reply}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// `tools/list` is MCP-shaped: `xirang.search` is there, and every entry declares
/// an object input schema.
/// `tools/list` 是 MCP 形状的：`xirang.search` 在其中，且每一条都声明 object 输入 schema。
///
/// Moved here from an inline `mod tests` in `tools.rs`, so every source file has at
/// most one `<name>_tests.rs` and `tools.rs` holds only the catalog and the dispatch
/// (audit `BR-11`).
/// 从 `tools.rs` 里的内联 `mod tests` 移到这里，使每个源文件至多有一个 `<name>_tests.rs`，
/// 而 `tools.rs` 只剩目录与分派（审计 `BR-11`）。
#[test]
fn tools_list_is_mcp_shaped() {
    let listed = super::tools();
    assert!(listed.iter().any(|tool| tool["name"] == "xirang.search"));
    assert!(
        listed
            .iter()
            .all(|tool| tool["inputSchema"]["type"] == "object")
    );
}

/// The two Studio write actions the bridge was missing are advertised, and their
/// descriptions carry the two disciplines every write here keeps: a request is previewed
/// unless `apply` is true, and the irreversible half needs the request to say `confirm`.
/// 桥此前缺的那两个 Studio 写入动作已被声明，而它们的描述带有这里每次写入都守的两条纪律：除非
/// `apply` 为真否则只预览，以及不可逆的那一半需要请求自己说出 `confirm`。
///
/// The description is the contract: an agent can only aim at what `tools/list` names, so a
/// tool whose schema omits `apply` or `confirm` is a tool no agent will ever use safely.
/// 描述就是契约：代理只能瞄准 `tools/list` 点名过的东西，因此 schema 里没有 `apply` 或 `confirm`
/// 的工具，是任何代理都不会安全使用的工具。
#[test]
fn the_two_studio_writes_are_advertised_with_their_flags() {
    let listed = super::tools();
    for (name, keys) in [
        (
            "xirang.new_project",
            &["directory", "package", "kind", "apply", "confirm", "root"][..],
        ),
        (
            "xirang.plugin",
            &[
                "source",
                "framework",
                "package",
                "version",
                "crate",
                "checksum",
                "mode",
                "apply",
                "confirm",
                "root",
            ][..],
        ),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        let description = tool["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} has no description: {tool}"));
        assert!(
            description.contains("previewed unless `apply` is true"),
            "{name} must state the preview default: {description}"
        );
        assert!(
            description.contains("`confirm: true`"),
            "{name} must state its confirmation rule: {description}"
        );
        for key in keys {
            assert!(
                tool["inputSchema"]["properties"].get(*key).is_some(),
                "{name} does not advertise `{key}`: {tool}"
            );
        }
    }
}

/// Both new writes name the two halves of the executor they reuse, so a reader can check
/// that this is not a second implementation: the scaffold is `create_project`, and the
/// plugin record goes through the kernel's `contains_record` gate.
/// 两个新写入都点名了它们复用的执行器的两半，好让读者能核对这不是第二份实现：脚手架是
/// `create_project`，而插件记录经内核的 `contains_record` 闸门。
#[test]
fn the_new_writes_name_the_executor_they_reuse() {
    let listed = super::tools();
    for (name, expected) in [
        ("xirang.new_project", "create_project"),
        ("xirang.plugin", "contains_record"),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        let description = tool["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} has no description: {tool}"));
        assert!(
            description.contains(expected),
            "{name} must name `{expected}`, the executor it shares: {description}"
        );
    }
}

/// A throwaway package with a called function, its caller, an orphan, and a documented definition.
/// 一个一次性包：一个被调用的函数、它的调用者、一个孤儿，以及一个带文档的定义。
///
/// Shared by the acceptance pin (the table's shapes must be accepted here) and the orphan pin, so
/// the two cannot disagree about what the fixture is.
/// 由接受性钉子（表里的形状必须在这里被接受）与孤儿钉子共用，因此两者不会对"夹具是什么"产生分歧。
pub(crate) fn scratch_package(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-tools-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"fixture-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A fixture module.\n\
         /// What this function is for.\n\
         pub fn used() -> u32 { 1 }\n\
         \n\
         pub fn call_used() -> u32 { used() }\n\
         \n\
         pub fn orphan() -> u32 { 2 }\n",
    )
    .expect("fixture source");
    root
}

/// Every read answer ends by naming the next call — **once**, and on its own line.
/// 每个读答案末尾点名下一次调用 —— **只点一次**，而且自占一行。
///
/// The measured failure this guards: the hint was appended without a separator, so it arrived glued
/// to the last line (`…default runnext   registry …`) and a reader that greps for a line starting
/// with `next` saw none.
/// 它守的实测失败：提示追加时没加分隔，于是粘在答案最后一行上（`…default runnext   registry …`），
/// 按"行首是 `next`"去 grep 的读者一条也看不到。
#[test]
fn a_read_answer_names_the_next_call_exactly_once() {
    let plain = super::with_next_hint("xirang.callgraph", "callers (0): -\n".to_owned());
    assert_eq!(plain.matches("\nnext").count(), 1, "{plain}");
    assert!(plain.ends_with("what depends on it\n"), "{plain}");

    // An answer that already names one (search, check, adopted) is left alone.
    // 已经点名过的答案（search / check / adopted）不动。
    let already = "no matches in /root\nnext   pass `literal` for text\n".to_owned();
    assert_eq!(
        super::with_next_hint("xirang.callgraph", already.clone()),
        already
    );

    // A tool with no hint is left alone.
    // 没有提示的工具不动。
    assert_eq!(
        super::with_next_hint("xirang.apply", "ok\n".to_owned()),
        "ok\n"
    );

    for name in [
        "xirang.status",
        "xirang.registry",
        "xirang.explain",
        "xirang.callgraph",
        "xirang.inspect",
        "xirang.affected",
    ] {
        assert!(super::next_hint(name).is_some(), "{name} needs a hint");
    }
}

/// Every answer carries a line the caller can quote; a refusal carries none.
/// 每个答案都带一行可引用的东西；被拒的调用不带。
///
/// Measured (W8): the arm re-ran calls purely to have something to cite — "I claimed a closing
/// `status` call … I haven't run it. Let me run it" and "I did NOT actually run explain … to make the
/// claim true". The brief demands a command, its raw output and its exit code per claim, so the line
/// removes the only reason those re-runs existed.
/// 实测（W8）：那一臂**为了有东西可引**而补跑调用——「I claimed a closing `status` call … I haven't
/// run it. Let me run it」与「I did NOT actually run explain … to make the claim true」。题面要求每条
/// 声称给命令、原始输出与退出码，因此这一行去掉了那些补跑存在的唯一理由。
#[test]
fn every_answer_carries_a_quotable_evidence_line() {
    let root = scratch_package("evidence");
    let answer = super::run_tool(&root, "xirang.status", &json!({})).expect("status answers");
    let last = answer.lines().last().expect("an answer has lines");
    assert!(
        last.starts_with("evidence xirang.status "),
        "the last line names this call: {answer}"
    );
    assert_eq!(
        last, "evidence xirang.status {} → exit 0",
        "a call with no arguments renders an empty object, and says it answered"
    );
    let with_arguments = super::run_tool(&root, "xirang.search", &json!({"query": "nothing-here"}))
        .expect("search answers, even with no matches");
    let last = with_arguments.lines().last().expect("an answer has lines");
    assert!(
        last.contains("{\"query\":\"nothing-here\"}") && last.ends_with("→ exit 0"),
        "and a call with arguments carries them: {last}"
    );
    // The line is rendered from the same `arguments` value the dispatch used, so it cannot describe
    // a different call — and a refusal has no line at all, because its own text is the evidence.
    // 这一行由派发实际使用的那个 `arguments` 值渲染，因此它不可能描述另一次调用——而被拒的调用完全没有
    // 这一行，因为它的文案本身就是证据。
    let refused = super::run_tool(&root, "xirang.read", &json!({"path": 7}));
    assert!(refused.is_err(), "a bad shape is refused");
    let _ = std::fs::remove_dir_all(&root);
}

/// The stage decides what the first calls say: a bare tree is told how to start, a faceful one is
/// not told to start anything.
/// 阶段决定开场调用说什么：空树被告知怎么起步，有面的树不会被告知去起步。
///
/// Measured (T-12): on an empty directory `status` printed `rust_files=0` beside Cargo's "manifest
/// path … does not exist" and `registry` returned Cargo's own sentence — both naming the failure and
/// neither naming the way to start, while the flow table's first shape lived only in prose.
/// 量到的（T-12）：在空目录上，`status` 在 Cargo 的 "manifest path … does not exist" 旁边打印
/// `rust_files=0`，而 `registry` 回的是 Cargo 自己的句子——两个都点名失败、都不点名起步的方式，而流程表的
/// 第一个形状只活在散文里。
#[test]
fn a_bare_tree_is_told_how_to_start_and_a_faceful_one_is_not() {
    let bare = std::env::temp_dir().join(format!("xirang-mcp-bare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("scratch dir");
    let status = super::run_tool(&bare, "xirang.status", &json!({})).expect("status answers");
    assert!(
        status.contains("next   `new_project {directory, kind, package, apply: true}`"),
        "a tree with no manifest is told how to start: {status}"
    );
    let registry = super::run_tool(&bare, "xirang.registry", &json!({})).expect("registry answers");
    assert!(
        registry.contains("no registration tree here yet") && registry.contains("new_project"),
        "and so is the other opening call: {registry}"
    );
    assert!(
        !registry.contains("explain {node}"),
        "the catalogue's face hint is not appended to a tree with no faces: {registry}"
    );
    let _ = std::fs::remove_dir_all(&bare);

    // A faceful fixture keeps the ordinary hints: the entry point is for the stages that need one.
    // 有面的夹具保留普通提示：入口只给需要它的那些阶段。
    let root = scratch_package("stage-faceful");
    let status = super::run_tool(&root, "xirang.status", &json!({})).expect("status answers");
    assert!(!status.contains("new_project"), "{status}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A tool's advertised `required` keys are the ones its own call path insists on.
/// 一个工具对外声明的 `required` 键，就是它自己的调用路径坚持要的那些。
///
/// Measured (round-8 review): `conformance`'s schema still said `required: ["anchor"]` after the
/// handler learned to list every anchor when none is given — and the client's `keys:` line derives its
/// `*` marks from that list, so every reader was told `anchor*` about a call that works without it.
/// A reader that trusts the schema calls a key required; the schema has to be the same contract the
/// handler enforces.
/// 量到的（第八轮复核）：处理器已经学会"不给 anchor 就列出全部锚点"之后，`conformance` 的 schema 仍写着
/// `required: ["anchor"]`——而客户端的 `keys:` 行正是从那份清单推出 `*` 的，于是每个读者都被告诉 `anchor*`，
/// 而那次调用不带它也能用。**信 schema 的读者会以为那个键是必需的**；schema 必须与处理器执行的契约一致。
#[test]
fn the_advertised_required_keys_match_what_the_call_path_insists_on() {
    let listed = super::tools();
    let required = |name: &str| {
        listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is advertised"))["inputSchema"]["required"]
            .as_array()
            .map(|keys| {
                keys.iter()
                    .filter_map(|key| key.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    assert!(
        !required("xirang.conformance").contains(&"anchor"),
        "conformance works without `anchor` (it lists every anchor), so the schema must not demand it"
    );
    assert!(
        !required("xirang.check").contains(&"face"),
        "the call path defaults `face` to `default`, so the schema must not demand it: a schema that \
         still stars it teaches every reader a shape the tool accepts without it"
    );
    assert!(
        required("xirang.apply").contains(&"action"),
        "and a key the call path really insists on stays required: `apply` needs `action` — without \
         this half, emptying every `required` would pass"
    );
}

/// Every catalogue entry discloses what it does to the tree, and the three names that write
/// are exactly the ones the write path knows.
/// 每个目录条目都披露它对这棵树做了什么，而会写的那三个名字正是写入路径认识的那三个。
///
/// The MCP spec's `annotations` are how a client decides what needs a human in the loop
/// (`readOnlyHint`, `destructiveHint`), so a hint that is missing, or a `destructiveHint` left
/// to the protocol's default (`true`), tells a client the opposite of the truth. And the
/// classification is pinned against `ownership::subject`, because that is the other place the
/// bridge says which tools write — two lists that disagree silently is the defect this pin
/// exists to prevent.
/// MCP 规范的 `annotations` 是客户端据以决定"什么需要人在环里"的东西（`readOnlyHint`、
/// `destructiveHint`），因此一条缺席的提示、或一条被留给协议默认值（`true`）的 `destructiveHint`，
/// 都在对客户端说反话。而这个分类与 `ownership::subject` 钉在一起，因为那是桥说"哪些工具会写"的另一个
/// 地方——两份清单无声地不一致，正是这枚钉子要防的缺陷。
#[test]
fn every_tool_discloses_its_effect_and_the_writers_are_the_write_paths_own() {
    let (mut readers, mut writers) = (Vec::new(), Vec::new());
    for entry in crate::mcp::tools::tools() {
        let name = entry["name"].as_str().expect("every entry has a name");
        let title = entry["title"].as_str().unwrap_or_default();
        assert!(!title.is_empty(), "`{name}` has a title");
        let hints = &entry["annotations"];
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(
                hints[hint].is_boolean(),
                "`{name}` discloses `{hint}`: {entry}"
            );
        }
        if hints["readOnlyHint"].as_bool() == Some(true) {
            assert_eq!(
                hints["destructiveHint"].as_bool(),
                Some(false),
                "a read-only tool is not destructive: {name}"
            );
            readers.push(name.to_owned());
        } else {
            writers.push(name.to_owned());
        }
    }
    writers.sort();
    assert_eq!(
        writers,
        vec![
            "xirang.adopted".to_owned(),
            "xirang.apply".to_owned(),
            "xirang.candidate".to_owned(),
            "xirang.check".to_owned(),
            "xirang.crates".to_owned(),
            "xirang.mir".to_owned(),
            "xirang.new_project".to_owned(),
            "xirang.plugin".to_owned(),
            "xirang.refactor".to_owned(),
            "xirang.rules".to_owned(),
            "xirang.task".to_owned(),
            "xirang.verify".to_owned(),
            "xirang_tools".to_owned(),
        ],
        "the tools that are not read-only are the ones that write"
    );
    // The other half of the same fact: every writer is one `ownership` already treats as a
    // write, or the scaffold that names its own destination.
    // 同一个事实的另一半：每个写工具要么是 `ownership` 已当作写入的那个，要么是自己点名目的地的脚手架。
    for name in &writers {
        assert!(
            crate::mcp::ownership::subject(name) == crate::mcp::ownership::Subject::Write
                || name == "xirang.new_project"
                || name == "xirang.adopted"
                || name == "xirang_tools"
                || name == "xirang.check"
                || name == "xirang.verify"
                || name == "xirang.mir",
            "`{name}` is a writer the write path knows"
        );
    }
    // `apply` can remove what is there (`delete` moves a subtree into the trash) and `crates`
    // can take a generated partition back (`revert`), so those two say `destructiveHint: true`.
    // `apply` 是唯一能移除已有东西的（`delete` 把子树移进回收目录），因此它是唯一写
    // `destructiveHint: true` 的那个。
    let catalogue = crate::mcp::tools::tools();
    let destructive: Vec<&str> = catalogue
        .iter()
        .filter(|entry| entry["annotations"]["destructiveHint"].as_bool() == Some(true))
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(
        destructive,
        vec![
            "xirang.apply",
            "xirang.mir",
            "xirang.check",
            "xirang.candidate",
            "xirang.rules",
            "xirang.verify",
            "xirang.crates",
            "xirang_tools"
        ],
        "{destructive:?}"
    );
    assert!(readers.len() > writers.len(), "most tools only read");
}

/// The JSON-RPC `tools/list` reply carries them, because a client never sees the catalogue in
/// any other shape.
/// JSON-RPC 的 `tools/list` 回复里带着它们，因为客户端看不到目录的其它形状。
#[test]
fn the_tools_list_reply_carries_the_annotations() {
    // Through the real stdio entry point, because that is the only shape a client sees.
    // 走真正的 stdio 入口，因为那是客户端唯一看得到的形状。
    let mut input = std::io::Cursor::new(
        b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n".to_vec(),
    );
    let mut output = Vec::new();
    crate::mcp::protocol::run_with(&mut input, &mut output).expect("the server answers");
    let line = String::from_utf8(output).expect("the reply is utf-8");
    let reply: serde_json::Value =
        serde_json::from_str(line.lines().next().unwrap_or_default()).expect("one JSON reply");
    let tools = reply["result"]["tools"]
        .as_array()
        .expect("tools/list answers with an array");
    // The advertised frame is the two entry points plus the catalogue tool (audit `W1-1`) …
    // 广告帧是两个入口加目录工具（审计 `W1-1`）……
    assert_eq!(
        tools.len(),
        crate::mcp::tools::advertised().len(),
        "the wire reply is what `advertised()` says it is"
    );
    // … and **every** entry either list carries keeps its annotations and its title, because a
    // client that cannot tell a read tool from a write tool cannot ask for approval.
    // ……而两张清单里的**每个**条目都带着自己的 annotations 与 title，因为分不出读工具与写工具的客户端
    // 没法请求批准。
    for entry in tools.iter().chain(crate::mcp::tools::tools().iter()) {
        assert!(
            entry["annotations"]["readOnlyHint"].is_boolean() && entry["title"].is_string(),
            "the entry carries the annotations and the title: {entry}"
        );
    }
}

/// Every key a handler reads is advertised, and every entry is accounted for (audit `W2-4` again).
/// 处理函数读的每个键都被广告出去，且每个条目都有交代（再次审计 `W2-4`）。
///
/// Two failures are caught here, and both were real. **Capability ahead of the advertisement**:
/// `consistency` read `by: "shape"` and `full` while its schema offered neither, and `full` is the
/// only way out of a bounded answer — so a reader could not reach the capability at all; `mir` read
/// `against_trace` the same way. **An entry nobody accounted for**: the list has to name every
/// catalogue entry, so a new tool cannot be added without saying which keys it reads.
/// 两类失败在这里被抓，而两类都真实发生过。**能力跑到广告前面**：`consistency` 读 `by: "shape"` 与
/// `full`，而它的 schema 一个都没给，而 `full` 还是有界答案唯一的出路——于是读者根本够不到那个能力；
/// `mir` 同形地读 `against_trace`。**没人交代的条目**：清单必须点名每一个目录条目，因此新工具不可能
/// 不声明它读哪些键。
#[test]
fn every_key_a_handler_reads_is_advertised_and_every_entry_is_accounted_for() {
    let entries: Vec<serde_json::Value> = crate::mcp::tools::tools();
    let mut missing: Vec<String> = Vec::new();
    for entry in &entries {
        let name = entry["name"].as_str().expect("a name");
        let (_, keys) = READ_KEYS
            .iter()
            .find(|(listed, _)| *listed == name)
            .unwrap_or_else(|| {
                panic!("`{name}` has no READ_KEYS row: add one naming the keys it reads")
            });
        // Keys may be advertised at any depth — `parts` sits under `inside`, `cut`/`graft` under the
        // graft branch of an `anyOf` — because what matters is that the schema **names** them
        // somewhere the reader can find. A top-level-only scan would report those as missing and
        // push the fix toward flattening a schema that is already correct.
        // 键可以广告在**任何深度**——`parts` 在 `inside` 之下、`cut`/`graft` 在某个 `anyOf` 分支里
        // ——要紧的是 schema 在读者找得到的地方**点名**了它们。只扫顶层会把那些报成缺失，并把修法
        // 推向把一份本来就对的 schema 拍平。
        let mut advertised = std::collections::BTreeSet::new();
        walk_properties(&entry["inputSchema"], &mut advertised);
        for key in *keys {
            if !advertised.contains(*key) {
                missing.push(format!(
                    "{name} reads `{key}` but its schema does not declare it"
                ));
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
    assert_eq!(
        entries.len(),
        READ_KEYS.len(),
        "the catalogue and the read-keys table must name the same entries"
    );
    // And the enum that carries the signals is not narrower than the signals the handler accepts.
    // 而承载信号的 enum 不得比处理函数接受的信号更窄。
    let consistency = entries
        .iter()
        .find(|entry| entry["name"] == "xirang.consistency")
        .expect("consistency");
    let signals = consistency["inputSchema"]["properties"]["by"]["enum"]
        .as_array()
        .expect("the `by` enum")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    for signal in ["api", "kind", "source", "shape"] {
        assert!(
            signals.contains(&signal),
            "`by` must offer `{signal}`: the handler accepts it, so the schema must name it ({signals:?})"
        );
    }
}

/// Walk a schema and note every key any `properties` object in it names, at any depth.
/// 这份 schema 里任何 `properties` 对象点名的每个键，任意深度。
fn walk_properties(value: &serde_json::Value, into: &mut std::collections::BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key == "properties"
                    && let Some(fields) = child.as_object()
                {
                    into.extend(fields.keys().cloned());
                    for field in fields.values() {
                        walk_properties(field, into);
                    }
                } else {
                    walk_properties(child, into);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                walk_properties(item, into);
            }
        }
        _ => {}
    }
}

/// The keys each handler **reads**, per tool, pinned against what its schema advertises.
/// 每个处理函数**读**的键，逐工具与它 schema 广告的内容钉在一起。
///
/// Capability has twice run ahead of the advertisement: `apply` accepted `cut`/`graft`/`to`/`full`
/// while its schema named none of them (audit `W2-4`), and `consistency` accepted `by: "shape"` and
/// `full` — the second one being the **only way out** of a bounded answer — while its enum stopped at
/// three signals and its properties did not mention `full` at all. A reader can only call what the
/// schema names, so a lagging schema is a missing capability however well the handler works.
/// 能力已经两次跑到广告前面：`apply` 早就接受 `cut`/`graft`/`to`/`full`，而它的 schema 一个都没写
/// （审计 `W2-4`）；`consistency` 早就接受 `by: "shape"` 与 `full`——后者还是**有界答案唯一的出路**
/// ——而它的 enum 停在三个信号上、properties 里根本没有 `full`。读者只能调用 schema 点名的东西，
/// 因此滞后的 schema 就是缺失的能力，处理函数再能干也一样。
///
/// The pin below walks the catalogue and requires this list to name **every** entry: a new tool that
/// nobody adds here fails the suite, which is the half a hand-kept list always loses.
/// 下面的钉子遍历目录，要求这张表点名**每一个**条目：新工具没人加进来就会失败——这正是一份手工
/// 清单总会丢掉的那一半。
///
/// It lives in the test file rather than beside the catalogue because a table only the pin reads is
/// test code, and the source file it used to sit in has a budget of its own (audit `M7`).
/// 它住在测试文件里而不是目录旁边，因为只有钉子读的表就是测试代码，而它原来所在的源码文件有自己的预算
/// （审计 `M7`）。
#[path = "tools_read_keys.rs"]
mod tools_read_keys;
use tools_read_keys::READ_KEYS;

/// Execution is not a read, and a misspelled freshness policy cannot silently fall back.
/// 执行不是只读，拼错的新鲜度策略不能静默回退。
#[test]
fn execution_effects_and_freshness_are_explicit() {
    let listed = super::tools();
    for name in ["xirang.check", "xirang.verify"] {
        let tool = listed.iter().find(|tool| tool["name"] == name).unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], false);
        assert_eq!(tool["annotations"]["idempotentHint"], false);
    }
    let explore = listed
        .iter()
        .find(|tool| tool["name"] == "xirang.explore")
        .unwrap();
    assert!(
        explore["inputSchema"]["properties"]["freshness"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("verify"))
    );
    for value in [json!("verfiy"), json!(true), json!(null)] {
        assert!(
            super::run_tool(
                std::path::Path::new("/missing"),
                "xirang.check",
                &json!({"freshness":value})
            )
            .is_err()
        );
    }
}

/// W6 四入口对照表的**第一行**：gateway 那一入口的"代理第一眼看到什么"到底多大。
/// W6's four-entry table, **first row**: how big the gateway's "what an agent sees first" really is.
///
/// The comparison in §7a of the report needs three quantities; **bytes** is the one that needs no
/// paid session, so it is the one measured here rather than described. Rounds and tokens stay
/// unmeasured and are marked so — a table with one real row and two blank ones is worth more than a
/// table with three estimated ones.
/// 报告 §7a 的对照要三个量；**字节**是不需要付费会话就能测的那一个，因此这里测它而不是描述它。轮次与
/// token 保持未测并如实标注——一张有一行真数、两行空白的表，比一张三行都是估算的表值钱。
#[test]
fn the_advertised_catalogue_is_a_bounded_first_look() {
    let full = serde_json::to_string(&super::tools()).expect("the catalogue encodes");
    let advertised =
        serde_json::to_string(&super::advertised()).expect("the advertisement encodes");
    // The numbers go into the report verbatim; printing them here is what makes that possible
    // without a second measurement.
    // 数字原样进报告；在这里打印出来，正是为了不必测第二遍。
    println!(
        "W6 gateway bytes: full={} advertised={}",
        full.len(),
        advertised.len()
    );
    assert!(
        super::advertised().len() < super::tools().len(),
        "the first look is deliberately narrower than the whole catalogue: {} vs {} tools",
        super::advertised().len(),
        super::tools().len()
    );
    assert!(
        advertised.len() < full.len(),
        "and smaller in bytes: {} vs {full_len}",
        advertised.len(),
        full_len = full.len()
    );
    assert!(
        !full.is_empty() && !advertised.is_empty(),
        "neither is empty, so the comparison is about real answers"
    );
}
