//! Pins for the `cut` action of the write path: what it renders, what it refuses, and in which class.
//! 写入路径 `cut` 动作的钉子：它渲染什么、拒绝什么、以及在**哪一类**拼写里。
//!
//! Split out of `apply_tests.rs` for the 800-code-line test ratchet, and it is one subject: an entry
//! joining a host's graft plan. Audit `F6` lives here — the class check that keeps a logical path out
//! of a plan written in the typed spelling — together with the renderer round-trip and the two
//! refusals that predate it.
//! 从 `apply_tests.rs` 拆出来是为了 800 代码行的测试棘轮，而它是一个题目：一条条目加入宿主的 graft 计划。
//! 审计 `F6` 住在这里——那条不让逻辑路径混进类型化计划的类别检查——连同渲染器的回环与它之前就有的两条拒绝。

use std::path::PathBuf;

use serde_json::json;

use crate::mcp::apply::apply;
use crate::mcp::apply::apply_tests::package;

/// A cut is rendered by the kernel, read back before it is written, and previewed with its impact.
/// 切口由内核渲染、写盘前先读回、预览里带上影响面。
///
/// Measured (maintainer's question): `graft`/`add`/`new` split into "the tool writes it" and "the
/// author writes it by hand", and cuts were the hand-written half — the bridge only counted them
/// (`apply.rs` counted `cut(` sites). Now it writes one, through the kernel's renderer, and refuses
/// to write anything that does not read back as exactly one more declaration.
/// 量到的（维护者的问题）：`graft`/`add`/`new` 分成"工具写"与"作者手写"两半，而切口正是手写那一半——桥只
/// 数它们。现在它会写一条，经内核的渲染器，并拒绝写下任何读回来不是"恰好多一条声明"的东西。
/// A typed cut naming no face is refused **at the moment the mistake is made**, not one step later.
/// 命名不到任何面的类型化切口，在**犯错的那一刻**就被拒，而不是等到下一步。
///
/// Measured 2026-10-18 while building the §7 permutation fixture: a typed cut addressed the moved face by the wrong
/// module path (the source layout inserts a second `object` directory), `apply cut` wrote the declaration anyway, and
/// `apply move` then refused that same declaration with "unresolved endpoint" — one step too late, pointing at the
/// declaration instead of at the mistake. The written text could not have compiled either.
/// 2026-10-18 搭 §7 排列夹具时实测：类型化切口用错的模块路径指向被搬的面（源码布局会再插一个 `object` 目录），
/// `apply cut` 照样写下了那条声明，随后 `apply move` 才以 "unresolved endpoint" 拒绝它——晚了一步，而且指的是那条
/// 声明而不是那个错。写下的文本本身也编译不过。
#[test]
fn a_typed_cut_that_names_no_face_is_refused_before_it_is_written() {
    let root =
        std::env::temp_dir().join(format!("mcp-apply-cut-unresolved-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/control-button");
    let status = std::process::Command::new("rsync")
        .args(["-a", "--exclude", "target"])
        .arg(format!("{}/", example.display()))
        .arg(format!("{}/", root.display()))
        .status()
        .expect("rsync runs");
    assert!(status.success(), "the example host copies");
    let entry = root.join("src/lib.rs");
    let before = std::fs::read(&entry).expect("host entry");
    let request = json!({
        "action": "cut",
        "cut": "crate::totally::bogus::NODE_ID",
        "graft": "control_button_graft::button_fast::NODE_ID",
        "apply": true,
    });
    let refused = apply(&root, &request).expect_err("a cut naming no face is refused");
    assert!(
        refused.contains("names no face in this tree") && refused.contains("nothing was written"),
        "the refusal says what is wrong and that nothing was written: {refused}"
    );
    assert_eq!(
        std::fs::read(&entry).expect("host entry"),
        before,
        "the refusal wrote nothing"
    );
    // The same request one character away from the face's real module path is still accepted: this refuses a mistake,
    // not the action.
    // 同一请求只差一个字符、写的是那个面真正的模块路径时仍然被接受：被拒的是错误，不是这个动作。
    apply(
        &root,
        &json!({
            "action": "cut",
            "cut": "crate::control::object::button::NODE_ID",
            "graft": "control_button_graft::button_fast::NODE_ID",
        }),
    )
    .expect("the real module path is accepted");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_cut_is_rendered_by_the_kernel_and_read_back_before_it_is_written() {
    // The host is the repository's own example, copied: a thin fixture has no registration faces,
    // so the impact line would have nothing to name and the pin would prove less than it claims.
    // 宿主用本仓自己的示例（复制一份）：薄夹具没有注册面，影响面那行就没有名字可点，钉子会证明得比它声称的少。
    let root = std::env::temp_dir().join(format!("mcp-apply-cut-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/control-button");
    let status = std::process::Command::new("rsync")
        .args(["-a", "--exclude", "target"])
        .arg(format!("{}/", example.display()))
        .arg(format!("{}/", root.display()))
        .status()
        .expect("rsync runs");
    assert!(status.success(), "the example host copies");
    let plan = std::fs::read_to_string(root.join("src/lib.rs")).expect("host entry");

    let reply = apply(
        &root,
        &json!({
            "action": "cut",
            "cut": "crate::control::object::slider::NODE_ID",
            "graft": "carrier::slider_fast::NODE_ID",
            "full": true,
        }),
    )
    .expect("the preview answers");
    assert!(
        reply.contains(
            "xirang_toolchain::graft(xirang_toolchain::subtree(crate::catalog::control::object::slider::NODE_ID)).with(carrier::slider_fast::NODE_ID)"
        ),
        "the entry is the kernel's rendering: {reply}"
    );
    assert!(
        reply.contains("part of what this application publishes")
            && reply.contains("root/control/slider"),
        "and the impact names the face the cut covers, in the right direction: {reply}"
    );
    assert!(
        !reply.contains("stops shipping"),
        "a cut **starts** publishing a subtree — the round-9 review caught this message saying the \
         opposite: {reply}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).expect("plan"),
        plan,
        "a preview does not touch the project's own file"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Applying it writes exactly one more declaration, and that declaration parses.
/// 落盘写下**恰好**多一条声明，而且那条声明是能解析的。
#[test]
fn an_applied_cut_adds_exactly_one_parseable_entry() {
    let (root, _name) = package("cut-apply");
    let plan = "xirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    \
                cut(crate::control::object::button::NODE_ID)\n        \
                graft(control_button_graft::button_fast::NODE_ID),\n);\n";
    std::fs::write(root.join("src/lib.rs"), plan).expect("host entry");
    let before = xirang_kernel::syntax::entries::graft_entries(plan)
        .expect("the plan parses")
        .len();

    apply(
        &root,
        &json!({
            "action": "cut",
            "cut": "crate::control::object::slider::NODE_ID",
            "graft": "carrier::slider_fast::NODE_ID",
            "full": true,
            "apply": true,
            "confirm": true,
        }),
    )
    .expect("the write happens");
    let written = std::fs::read_to_string(root.join("src/lib.rs")).expect("plan");
    let after = xirang_kernel::syntax::entries::graft_entries(&written)
        .expect("the written plan still parses");
    assert_eq!(after.len(), before + 1, "{written}");
    assert_eq!(after[1].cut, "crate::control::object::slider::NODE_ID");
    assert_eq!(after[1].graft, "carrier::slider_fast::NODE_ID");
    assert!(after[1].full, "`full` survives: {written}");
    assert_eq!(
        after[0].cut, "crate::control::object::button::NODE_ID",
        "and the existing entry is untouched"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The two refusals: a spelling the kernel will not write, and a host with no plan.
/// 两种拒绝：内核不肯写的拼写，以及没有计划的宿主。
#[test]
fn a_cut_that_cannot_be_written_is_refused_with_its_reason() {
    let (root, _name) = package("cut-refuse");
    std::fs::write(
        root.join("src/lib.rs"),
        "xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK, cut(a::b::NODE_ID) graft(c::d::NODE_ID),);\n",
    )
    .expect("host entry");
    let refused = apply(
        &root,
        &json!({"action": "cut", "cut": "crate::to::NODE_ID; panic!()", "graft": "carrier::x::NODE_ID"}),
    )
    .expect_err("statements are not identity paths");
    assert!(refused.contains("Rust identity path"), "{refused}");

    let (bare, _name) = package("cut-no-plan");
    let preview = apply(
        &bare,
        &json!({"action": "cut", "cut": "crate::a::NODE_ID", "graft": "carrier::x::NODE_ID"}),
    )
    .expect("a native const can start the first plan");
    assert!(preview.contains("XIRANG_GRAFTS_0"), "{preview}");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&bare);
}

/// The tool's own enumeration of its actions matches the ones it dispatches.
/// 工具对自身动作的枚举，与它真正分派的那些一致。
///
/// Measured (round-9 review, by running the thing): `--list apply` and the `bogus action` refusal
/// both omitted `cut` (and one omitted `deepen`), so a reader was told the tool cannot do what it
/// does. A promise surface that lags the capability surface is the same defect the client's `keys:`
/// line had — the reader believes the enumeration.
/// 量到的（第九轮复核，靠真跑）：`--list apply` 与 `bogus action` 的拒绝都漏了 `cut`（其中一个还漏了
/// `deepen`），于是读者被告知这个工具做不到它其实做得到的事。承诺面落后于能力面，与客户端 `keys:` 行那个
/// 缺陷同族——读者相信枚举。
#[test]
fn the_advertised_actions_match_the_dispatched_ones() {
    let (root, _name) = package("cut-enum");
    let refused = apply(&root, &json!({"action": "bogus"})).expect_err("bogus is refused");
    for action in ["add", "edit", "rename", "delete", "deepen", "cut"] {
        assert!(
            refused.contains(action),
            "the refusal names every action it supports, including `{action}`: {refused}"
        );
    }
    let missing = apply(&root, &json!({})).expect_err("no action is refused");
    for action in ["add", "edit", "rename", "delete", "deepen", "cut"] {
        assert!(
            missing.contains(action),
            "and so does the missing-action refusal, including `{action}`: {missing}"
        );
    }
    let listed = crate::mcp::tools::tools();
    let apply = listed
        .iter()
        .find(|tool| tool["name"] == "xirang.apply")
        .expect("apply is advertised");
    let enum_values = apply["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .expect("the schema enumerates the actions");
    for action in ["add", "edit", "rename", "delete", "deepen", "cut"] {
        assert!(
            enum_values.iter().any(|value| value == action),
            "the schema's enum carries `{action}`: {enum_values:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A request that carries **no `fields` object** is a different mistake from a `fields`
/// object missing `module`, and one shared sentence made the first one unreadable: it named
/// `fields.module`, which the caller had sent — as a dotted key the tool never sees as an
/// object. The round-13 benchmark measured the cost (three refusals in a row).
/// 请求里**没有 `fields` 对象**与"`fields` 里缺 `module`"是两种不同的错，而共用一句话让前一种读不懂：
/// 那句话点名 `fields.module`，而调用方**确实**发了它——以工具根本看不到的点号键形式。第十三轮量出了
/// 代价（连吃三次拒绝）。
#[test]
fn a_request_without_a_fields_object_is_refused_with_the_object_spelling() {
    let (root, _) = package("fields-object-shape");
    let refused = apply(&root, &json!({"action": "add", "parent": "root"}))
        .expect_err("add without a `fields` object is refused");
    assert!(
        refused.contains("`fields`")
            && refused.contains("--fields '{")
            && refused.contains("accepted shape"),
        "the refusal names the object spelling to paste: {refused}"
    );
    // The reverse: with a `fields` object present, the other branch keeps its own wording
    // and does not offer the object spelling again.
    // 反向：`fields` 对象在时，另一支保持自己的措辞，不再提对象拼法。
    let missing_module = apply(
        &root,
        &json!({"action": "add", "parent": "root", "fields": {"kind": "Button"}}),
    )
    .expect_err("add without `fields.module` is refused");
    assert!(
        missing_module.contains("`fields.module`") && !missing_module.contains("--fields '{"),
        "the two branches stay distinguishable: {missing_module}"
    );
}

/// A package that really derives one child face, so a cut's class check has a tree to read.
/// 一个真的推导得出一个子面的包，好让切口的类别检查有棵树可读。
///
/// The layout is the build's own (`<name>/<name>.rs`), which is what the derivation reads; a
/// description of a tree would pin nothing here, because the refusal's whole value is the spelling
/// it derives from the face's real source path.
/// 布局就是构建自己那一套（`<name>/<name>.rs`），也正是推导读的东西；在这里描述一棵树什么都钉不住，
/// 因为这条拒绝的全部价值就在于它从面**真实的**源文件路径推导出的那个拼写。
pub(crate) fn face_package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-apply-faces-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let write = |relative: &str, text: &str| {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
        std::fs::write(path, text).expect("fixture file");
    };
    write(
        "Cargo.toml",
        &format!(
            "[package]\nname = \"fixture-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    );
    write("src/lib.rs", "//! fixture\npub mod control;\n");
    write(
        "src/control/control.rs",
        "pub struct Control;\npub struct ControlParts;\n\ncrate::root_object! {\n    kind: Control,\n    \
         parts: ControlParts,\n    needs_registry: true,\n    parent: \
         crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write("src/control/object/object.rs", "pub mod dial;\n");
    write(
        "src/control/object/dial/dial.rs",
        "pub struct Dial;\npub struct DialParts;\n\ncrate::control_object! {\n    kind: Dial,\n    \
         parts: DialParts,\n    parent: crate::control::NODE_ID,\n}\n",
    );
    root
}

/// F6: a logical path cannot join a plan written in the typed spelling, and the refusal hands back
/// the spelling to paste instead.
/// F6：逻辑路径不能加入一份用类型化拼写写成的计划，而拒绝会把应当照抄的拼写交回给调用方。
///
/// Both halves are the acceptance: the refusal names `crate::control::object::dial::NODE_ID`
/// — derived from the face's own `src/control/object/dial/dial.rs`, not assembled from the logical
/// path, which never mentions `object` — and the same request written in that spelling goes through
/// in one call. Nothing is written on the refused path, which is what makes this a refusal rather
/// than an error after the fact.
/// 两半都是验收：拒绝里点名 `crate::control::object::dial::NODE_ID`——由面自己的
/// `src/control/object/dial/dial.rs` 推导，而不是从从不提及 `object` 的逻辑路径拼装——而同一个请求用
/// 那个拼写写出来，一次就过。被拒的那条路上什么都没写，这正是"拒绝"与"事后报错"的区别。
#[test]
fn a_logical_path_is_refused_by_a_typed_plan_with_the_spelling_to_use() {
    let root = face_package("cut-class");
    let plan = "xirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    \
                cut(crate::control::object::button::NODE_ID)\n        \
                graft(control_button_graft::button_fast::NODE_ID),\n);\n";
    std::fs::write(root.join("src/lib.rs"), plan).expect("host entry");

    let refused = apply(
        &root,
        &json!({
            "action": "cut",
            "cut": "root/control/dial",
            "graft": "carrier::dial_fast::NODE_ID",
            "apply": true,
            "confirm": true,
        }),
    )
    .expect_err("a logical path does not join a typed plan");
    assert!(
        refused.contains("crate::control::object::dial::NODE_ID"),
        "the refusal hands back the spelling derived from this tree: {refused}"
    );
    assert!(
        refused.contains("src/control/object/dial/dial.rs"),
        "and the file it derived it from, so a reader can check it: {refused}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).expect("plan"),
        plan,
        "the refused request wrote nothing"
    );

    apply(
        &root,
        &json!({
            "action": "cut",
            "cut": "crate::control::object::dial::NODE_ID",
            "graft": "carrier::dial_fast::NODE_ID",
            "apply": true,
            "confirm": true,
        }),
    )
    .expect("the typed spelling goes through in one call");
    let written = std::fs::read_to_string(root.join("src/lib.rs")).expect("plan");
    assert!(
        written.contains(
            "xirang_toolchain::graft(crate::catalog::control::object::dial::NODE_ID).with(carrier::dial_fast::NODE_ID)"
        ),
        "and lands as the entry the caller asked for: {written}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The class check stays quiet when there is no class to join, or when the plan already mixes both.
/// 没有类可加入、或计划本来就混用两类时，这条检查保持沉默。
///
/// The asymmetry is deliberate and is the half that keeps the tool from refusing a request the
/// grammar accepts: a plan whose entries are logical paths is a legal plan, and the typed form is
/// one of its spellings rather than a replacement for it.
/// 这个不对称是刻意的，也正是"不拒绝语法本就接受的请求"的那一半：条目为逻辑路径的计划是合法计划，而
/// 类型化那一形是它的拼写之一，不是它的替代品。
#[test]
fn the_class_check_is_silent_without_a_class_to_join() {
    let root = face_package("cut-class-quiet");
    // A plan that names its cuts by logical path: the same `root/control/dial` request is its own
    // class, so it is not this check's business.
    let string_plan = "xirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    \
                       cut \"root/control/button\" graft \"button_fast\",\n);\n";
    std::fs::write(root.join("src/lib.rs"), string_plan).expect("host entry");
    let reply = apply(
        &root,
        &json!({
            "action": "cut",
            "cut": "root/control/dial",
            "graft": "dial_fast",
        }),
    );
    assert!(
        reply.is_ok(),
        "a logical path joins a logical-path plan: {reply:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
