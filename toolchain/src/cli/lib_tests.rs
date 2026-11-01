//! End-to-end tests for the CLI dispatch surface.
//! CLI 分发表面的端到端测试。
//!
//! Split out of `lib.rs` (see the split note there): these tests drive the
//! public `run`/`run_to` entry points and the private helpers through `super`,
//! so they pin dispatch behaviour without making the dispatch page itself
//! exceed the file budget.
//! 从 `lib.rs` 拆出（见那里的拆分说明）：这些测试经 `super` 驱动公开的
//! `run`/`run_to` 入口与私有辅助函数，因此钉住分发行为，又不让分发页本身超出
//! 文件预算。

use super::{run, run_to, split_build_args};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use xirang_kernel::identity::NodeId;
use xirang_kernel::plugin::graft_document::GraftPlanDocument;

/// Drive the CLI against an in-memory sink so a test can read the exact
/// stdout document a machine would parse. `run` is the same dispatch with
/// process stdout; `run_to` exists only so the document is assertable.
/// 让 CLI 写到内存接收器，使测试能读到机器会解析的那份 stdout 文档。`run` 是
/// 同一个分发、写到进程 stdout；`run_to` 的存在只是为了让文档可被断言。
fn run_capture(args: &[&str]) -> (Result<(), String>, String) {
    let mut buffer = Vec::new();
    let result = run_to(args.iter().map(|value| (*value).to_owned()), &mut buffer);
    (result, String::from_utf8(buffer).expect("utf-8 output"))
}

/// A throwaway Cargo package with an entry and registration faces, so
/// `cargo metadata` answers a package name and the build sees a real tree.
/// 一次性的 Cargo 包，带入口与注册面，使 `cargo metadata` 能给出包名、构建能看到
/// 真实的树。
fn fixture_host(label: &str, entry: &str, faces: &[(&str, &str)]) -> PathBuf {
    let root = temporary_root(label);
    fs::create_dir_all(root.join("src")).expect("fixture src");
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), entry).expect("entry");
    for (relative, source) in faces {
        let path = root.join("src").join(relative);
        fs::create_dir_all(path.parent().expect("face parent")).expect("face directory");
        fs::write(&path, source).expect("face source");
    }
    root
}

/// The three-level tree the operator commands are tested against, matching
/// the logical paths the README's example uses. The macro names follow the
/// generated per-registry aliases (`control_object!`, `object_object!`),
/// because the build's own validation requires the parent-specific macro; a
/// `root_object!` child under a registry is reported as a mismatch and would
/// make the fixture an invalid host.
/// 操作命令测试所用的三层树，逻辑路径与 README 示例一致。宏名遵循生成的按注册机
/// 别名（`control_object!`、`object_object!`），因为构建自身的校验要求父级专用
/// 宏；注册机下用 `root_object!` 的子面会被报为不匹配，夹具就成了非法宿主。
fn control_tree() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "control/control.rs",
            "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        ),
        (
            "control/object/object.rs",
            "crate::control_object! {\n    kind: Object,\n    parent: crate::control::NODE_ID,\n    needs_registry: true,\n}\n",
        ),
        (
            "control/object/button/button.rs",
            "crate::object_object! {\n    kind: Button,\n    parent: crate::control::object::NODE_ID,\n}\n",
        ),
        // The canonical rule beside each face that owns a registry. A face with
        // `needs_registry: true` and no rule makes the generated module read a `super::registry_rule`
        // nothing mounts, so the build refuses that tree by name — a fixture without these files is a
        // tree no `cargo build` can compile, and that stayed invisible only while nothing judged it.
        // 每个拥有注册机的面旁边那份规范规则。只有 `needs_registry: true` 而没有规则，会让生成的模块去读一个
        // 没有任何东西挂载的 `super::registry_rule`，因此构建按名拒绝那棵树——没有这些文件的夹具是任何
        // `cargo build` 都编译不过的树，只因为没人判它才一直看不见。
        (
            "control/registry_rule/registry_rule.rs",
            REGISTRATION_RULE_SOURCE,
        ),
        (
            "control/object/registry_rule/registry_rule.rs",
            REGISTRATION_RULE_SOURCE,
        ),
    ]
}

/// The canonical registration rule the fixtures put beside a registry-owning face.
/// 夹具放在拥有注册机的面旁边的那份规范注册规则。
const REGISTRATION_RULE_SOURCE: &str = "use crate::RegistrationRule;\n\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();\n";

const DECLARED_BUTTON: &str = "// host entry\ncrate::static_graft_plan!(\n    FRAMEWORK,\n    cut \"root/control/object/button\" graft \"button_fast\",\n);\n";

/// `check --json` puts exactly one document on stdout, carries every
/// diagnostic the human frame would print, and still fails the command.
/// `check --json` 在 stdout 上只放一个文档，携带人类可读帧会打印的每条诊断，并且
/// 仍然让命令失败。
#[test]
fn check_json_emits_one_document_and_still_fails() {
    let root = fixture_host(
        "cli-check-json",
        "// host\n",
        &[
            (
                "one/one.rs",
                "crate::root_object! {\n    kind: One,\n    stable_name: \"dup\",\n}\n",
            ),
            (
                "two/two.rs",
                "crate::root_object! {\n    kind: Two,\n    stable_name: \"dup\",\n}\n",
            ),
        ],
    );
    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["xirang", "check", "--json", &path]);
    assert!(result.is_err(), "a duplicate stable_name must fail");
    assert_eq!(stdout.lines().count(), 1, "exactly one document: {stdout}");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["schema"], "xirang.build-diagnostics/1");
    assert!(document["count"].as_u64().expect("count") >= 1);
    assert!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["message"]
                .as_str()
                .unwrap_or_default()
                .contains("duplicate stable_name")),
        "{document}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// Without `--json` the human output **opens** with the historical
/// `xirang check: ok (<package>)` line, byte for byte, and then carries the index line the run
/// published (audit `M7`, P2.2).
/// 不带 `--json` 时人类输出**以**历史的 `xirang check: ok (<package>)` 行开头、逐字节一致，随后携带
/// 这次运行发布的索引那一行（审计 `M7`，P2.2）。
///
/// The first line's bytes are a contract and stay one; the second line was **added** deliberately,
/// because a reader has to be able to see that the index it is about to read is the one this run
/// wrote — the maintainer's "see the terminal say it is ready" — and a line appended after the
/// historical one changes no existing parser's first line.
/// 第一行的字节是契约、也仍然是；第二行是有意**新增**的，因为读者必须能看见它即将读的索引就是这次运行写下的
/// 那一份——也就是维护者说的"看到终端说就绪"——而接在历史行之后的一行，不改任何既有解析器的第一行。
#[test]
fn check_without_json_keeps_the_human_line() {
    let root = fixture_host("cli-check-human", "// host\n", &[]);
    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["xirang", "check", &path]);
    assert!(result.is_ok(), "{result:?}");
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some("xirang check: ok (cli-check-human)"));
    let index = lines.next().expect("the index line follows the verdict");
    assert!(
        index.starts_with("graph updated: generation 1, "),
        "the run names the index it published: {index}"
    );
    assert_eq!(lines.next(), None, "and there is nothing else: {stdout}");
    assert!(
        run(["xirang".to_owned(), "check".to_owned(), path]).is_ok(),
        "the public `run` entry point accepts check"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `explain` resolves a logical path and a node id to the same face, and
/// reports identity, build scope, and the declared cut that names it.
/// `explain` 把逻辑路径与节点 id 解析到同一个面，并报告身份、构建作用域与命名它
/// 的已声明切口。
#[test]
fn explain_json_reports_identity_scope_and_declared_grafts() {
    let root = fixture_host("cli-explain", DECLARED_BUTTON, &control_tree());
    let path = root.display().to_string();
    let (checked, check_stdout) = run_capture(&["xirang", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");
    let (result, stdout) = run_capture(&[
        "xirang",
        "explain",
        "--json",
        "--path",
        &path,
        "root/control/object/button",
    ]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let document: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(document["resolved"], true);
    assert_eq!(document["node"]["kind"], "Button");
    assert_eq!(
        document["node"]["source"],
        "control/object/button/button.rs"
    );
    assert_eq!(document["node"]["registry_name"], "button");
    assert_eq!(document["node"]["parent"]["path"], "root/control/object");
    assert_eq!(document["scope"]["known"], true);
    assert_eq!(document["pruning"]["known"], true);
    assert_eq!(document["declared_grafts"]["count"], 1);
    assert_eq!(
        document["declared_grafts"]["cuts"][0]["graft"],
        "button_fast"
    );

    let id = document["node"]["id"].as_str().expect("node id").to_owned();
    let (result, stdout) = run_capture(&["xirang", "explain", "--json", "--path", &path, &id]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let by_id: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(by_id["node"]["path"], "root/control/object/button");
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `explain` resolves a face that lives in a build-time record file, not in a walked source.
/// `explain` 能解析住在构建期记录文件里、而不是被遍历源码里的面。
#[test]
fn explain_json_reports_a_record_file_face() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/record-gauge/gauge");
    let path = root.display().to_string();
    let (result, stdout) =
        run_capture(&["xirang", "explain", "--json", "--path", &path, "root/Gauge"]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let document: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(document["resolved"], true);
    assert_eq!(document["node"]["kind"], "Gauge");
    assert_eq!(document["node"]["source"], "native/gauge.java");
    assert_eq!(document["node"]["path"], "root/Gauge");
}

/// `explain` refuses to present build output that predates the sources.
/// `explain` 不会把早于当前源码的构建产物当作现状提供。
///
/// It used to serve the previous build's scope as `known: true`, so the same tree
/// answered differently depending on whether a `check` had happened to run in
/// between; output left behind by a *failed* check was served the same way. The
/// failing half is pinned in `build_method::scope_view` (the run publishes no
/// fingerprint); this pins the staleness half end to end, through the real
/// command.
/// 它过去会把上一次构建的作用域当作 `known: true` 提供，于是同一棵树会因期间是否恰好跑过
/// `check` 而给出不同答案；**失败**的 check 留下的产物也是这样被提供的。失败那一半钉在
/// `build_method::scope_view`（该次运行不发布指纹）；这里端到端钉住陈旧那一半，走真实命令。
#[test]
fn explain_reports_an_unknown_scope_when_the_build_output_is_stale() {
    let root = fixture_host("cli-explain-stale", DECLARED_BUTTON, &control_tree());
    let path = root.display().to_string();
    let (checked, check_stdout) = run_capture(&["xirang", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");

    let query = [
        "xirang",
        "explain",
        "--json",
        "--path",
        &path,
        "root/control/object/button",
    ];
    let (result, stdout) = run_capture(&query);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let fresh: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(fresh["scope"]["known"], true, "{fresh}");
    assert_eq!(fresh["pruning"]["known"], true, "{fresh}");

    // A content change that keeps the host valid: the pin is about the token, not
    // about a semantic edit.
    // 一次仍然让宿主合法的内容变化：这条钉子关乎那枚凭据，而不是语义改动。
    let button = root.join("src/control/object/button/button.rs");
    let text = fs::read_to_string(&button).expect("button source");
    fs::write(&button, format!("// edited\n{text}")).expect("edited button");

    let (result, stdout) = run_capture(&query);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let stale: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(
        stale["scope"]["known"], false,
        "a previous build must not answer for these sources: {stale}"
    );
    assert_eq!(stale["pruning"]["known"], false, "{stale}");
    assert_eq!(stale["resolved"], true, "the node itself is resolved live");

    // Re-publishing restores the answer, so the command is not simply pessimistic.
    // 重新发布即可恢复答案，因此本命令并非一律悲观。
    let (checked, check_stdout) = run_capture(&["xirang", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");
    let (_, stdout) = run_capture(&query);
    let current: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(current["scope"]["known"], true, "{current}");
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// An unresolvable query is reported with its reason instead of a bare
/// failure, so an operator can correct the spelling.
/// 无法解析的查询连原因一起报告，而不是只报失败，让操作者能改正拼写。
#[test]
fn explain_reports_why_a_node_cannot_be_resolved() {
    let root = fixture_host(
        "cli-explain-missing",
        "// host\n",
        &[("one/one.rs", "crate::root_object! {\n    kind: One,\n}\n")],
    );
    let path = root.display().to_string();
    let (result, stdout) =
        run_capture(&["xirang", "explain", "--json", "--path", &path, "root/nope"]);
    assert!(result.is_err());
    let document: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(document["resolved"], false);
    assert!(
        document["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("root/nope"),
        "{document}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `explain --json` promises exactly one document on stdout, with the failure inside it. A package
/// that resolves while its *source tree* does not used to return before writing anything, so a
/// machine reader got a bare parse error and could not tell "no document" from "the tool crashed"
/// (audit `S10`; `grafts --json` already collects the same failure this way, and the resolution
/// failures above were already covered).
/// `explain --json` 承诺 stdout 恰好一份文档、失败写在里面。包解析成功而**源码树**失败时，过去会在写出
/// 任何东西之前返回，于是机器读者只拿到一个裸解析错误，分不清"没有文档"与"工具崩了"（审计 `S10`；
/// `grafts --json` 早已用这种方式收下同样的失败，而上面那两条"解析失败"路径本来就已经覆盖）。
#[test]
fn explain_json_reports_an_unreadable_source_tree_as_json() {
    let root = temporary_root("cli-explain-nosrc");
    // A manifest whose declared library target does not exist: `cargo metadata` answers a package
    // name (so `resolve_package` succeeds) while the source tree cannot be read, which is the
    // failure this path is about. Measured on the unfixed tree: this exact fixture printed **0
    // bytes** of stdout, while a merely missing `src/` or a `src` that is a plain file failed
    // earlier, in `resolve_package`, which already wrote a document — the first two versions of
    // this pin measured that other path and passed on the unfixed tree.
    // 一份"声明的库 target 并不存在"的清单：`cargo metadata` 能给出包名（因此 `resolve_package` 成功），
    // 而源码树读不了——这才是本条路径针对的失败。实测于未修的树：正是这个夹具的 stdout 是 **0 字节**；
    // 而单纯缺少 `src/`、或 `src` 是普通文件，都会更早地在 `resolve_package` 失败，那条路径早就写文档
    // 了——这条钉子的前两版量的就是那条路径，于是在未修的树上通过。
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"no-sources\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/nope.rs\"\n",
    )
    .expect("manifest");
    let path = root.display().to_string();
    let (result, stdout) = run_capture(&[
        "xirang",
        "explain",
        "--json",
        "--path",
        &path,
        "root/anything",
    ]);
    assert!(result.is_err(), "the command still fails: {result:?}");
    assert!(
        !stdout.trim().is_empty(),
        "the failure is a document, not silence: {result:?}"
    );
    let document: Value = serde_json::from_str(stdout.trim()).expect("stdout is one JSON document");
    assert!(
        document.get("reason").is_some() || document.get("error").is_some(),
        "the document names the failure: {stdout}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// The overlay keeps the same contract: a plans path that is a plain file must arrive as a document
/// carrying the failure rather than as an empty stdout (audit `S10`).
/// 覆盖报告守同一条契约：计划路径是个普通文件时，到达读者的必须是一份携带失败的文档，而不是空 stdout
/// （审计 `S10`）。
#[test]
fn explain_overlay_json_reports_an_unreadable_plans_directory_as_json() {
    let root = fixture_host("cli-overlay-badplans", "// host\n", &control_tree());
    fs::create_dir_all(root.join(".xirang")).expect(".xirang directory");
    // `graft_plan_rows` refuses a plans path that is not a directory.
    // `graft_plan_rows` 会拒绝一个不是目录的计划路径。
    fs::write(root.join(".xirang/external-grafts"), "not a directory\n").expect("plain file");
    let path = root.display().to_string();
    let (result, stdout) =
        run_capture(&["xirang", "explain", "--json", "--overlay", "--path", &path]);
    assert!(result.is_err(), "the command still fails: {result:?}");
    assert!(
        !stdout.trim().is_empty(),
        "the failure is a document: {result:?}"
    );
    let document: Value = serde_json::from_str(stdout.trim()).expect("stdout is one JSON document");
    assert!(
        document.get("error").is_some(),
        "the overlay document carries the failure in `error`: {stdout}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `build --manifest-path <p>` validates the project it is about to build, not the current
/// directory. It used to run the registration check on `.` and hand `--manifest-path` to cargo, so
/// the named project's red verdict was never seen while the *current* project's verdict was printed
/// as its conclusion — with cargo's exit code (audit `S11`). The pin therefore asserts that the
/// failure names the **named** project: "the command failed" alone is also true of the old
/// behaviour, because the current directory is not a host either.
/// `build --manifest-path <p>` 校验的是它将要构建的那个项目，而不是当前目录。它过去对 `.` 跑注册
/// 校验、把 `--manifest-path` 交给 cargo，于是被点名项目的红色判断从未被看到，而**当前**项目的判断
/// 被当成它的结论打印出来——还带着 cargo 的退出码（审计 `S11`）。因此这条钉子断言失败信息点名的是
/// **被点名的**那个项目：只说"命令失败了"在旧行为下也成立，因为当前目录同样不是宿主。
#[test]
fn build_with_a_manifest_path_validates_that_project() {
    // A host whose registrations are broken: a registry child declared as `root_object!` is a
    // mismatch the build refuses (the `control_tree` note above says why).
    // 一个注册树坏掉的宿主：注册机的子面用 `root_object!` 声明，是构建会拒绝的不匹配（上面
    // `control_tree` 的注记说明了原因）。
    let broken = fixture_host(
        "cli-build-broken",
        "// host\n",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
            ),
            (
                "control/child/child.rs",
                "crate::root_object! {\n    kind: Child,\n    parent: crate::control::NODE_ID,\n}\n",
            ),
            // The rule the registry-owning face needs, so the **only** defect here is the one this
            // test is about: a missing rule is a second, unrelated refusal.
            // 拥有注册机的面所需的那份规则，因此这里**唯一**的缺陷就是本测试针对的那一个：缺规则是另一条无关的拒绝。
            (
                "control/registry_rule/registry_rule.rs",
                REGISTRATION_RULE_SOURCE,
            ),
        ],
    );
    let manifest = broken.join("Cargo.toml").display().to_string();
    let (result, stdout) = run_capture(&["xirang", "build", "--manifest-path", &manifest]);
    assert!(
        result.is_err(),
        "a broken named project must fail the command: {result:?} {stdout}"
    );
    assert!(
        !stdout.contains("registration ok"),
        "the banner must not describe a project that was not the one checked: {stdout}"
    );
    let error = result.expect_err("checked above");
    assert!(
        error.contains("control/child/child.rs"),
        "the failure names a file in the project the manifest pointed at, not in the current \
         directory: {error}"
    );
    std::fs::remove_dir_all(&broken).expect("cleanup");
}

/// Two ways of naming one project must agree; two names for two projects is a usage error rather
/// than a silent choice between them (audit `S11`).
/// 同一个项目的两种点名方式必须一致；两个名字指向两个项目时，这是用法错误，而不是在它们之间静默选一个
/// （审计 `S11`）。
#[test]
fn build_refuses_two_names_for_two_projects() {
    let first = fixture_host("cli-build-one", "// host\n", &control_tree());
    let second = fixture_host("cli-build-two", "// host\n", &control_tree());
    let named = second.join("Cargo.toml").display().to_string();
    let error = super::build_target(
        Some(first.display().to_string()),
        &["--manifest-path".to_owned(), named.clone()],
    )
    .expect_err("two projects cannot both be the target");
    assert!(error.contains("different projects"), "{error}");
    // The same project named both ways is fine, and both spellings of the option are read.
    // 同一个项目用两种方式点名是可以的，而该选项的两种拼写都能被读到。
    let same = first.join("Cargo.toml").display().to_string();
    assert_eq!(
        super::build_target(
            Some(first.display().to_string()),
            &[format!("--manifest-path={same}")]
        )
        .expect("one project, two spellings"),
        first.display().to_string()
    );
    std::fs::remove_dir_all(&first).expect("cleanup");
    std::fs::remove_dir_all(&second).expect("cleanup");
}

/// `explain --json` emits one JSON document even when the package cannot be
/// resolved, so a machine reader is never handed an empty stdout; the human
/// path still writes nothing.
/// 包解析不出来时 `explain --json` 仍输出一个 JSON 文档，机器读者绝不会拿到空 stdout；
/// 人类可读路径仍然什么都不写。
#[test]
fn explain_json_reports_a_resolution_failure_as_json() {
    let missing = temporary_root("cli-explain-unresolvable").join("no-such-project");
    let path = missing.display().to_string();
    let (result, stdout) = run_capture(&[
        "xirang",
        "explain",
        "--json",
        "--path",
        &path,
        "root/anything",
    ]);
    assert!(result.is_err(), "an unresolvable package must fail");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["schema"], "xirang.explain/1");
    assert_eq!(document["resolved"], false, "{document}");
    assert!(
        document["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("cannot resolve"),
        "{document}"
    );

    let (result, stdout) = run_capture(&["xirang", "explain", "--path", &path, "root/anything"]);
    assert!(result.is_err());
    assert!(
        stdout.is_empty(),
        "without --json stdout stays empty: {stdout}"
    );
}

/// `explain --overlay --json` emits the projection's own schema on the same
/// failure, carrying the reason instead of an empty stdout.
/// 同样的失败下 `explain --overlay --json` 输出投影自己的 schema，携带原因而不是空 stdout。
#[test]
fn explain_overlay_json_reports_a_resolution_failure_as_json() {
    let missing = temporary_root("cli-overlay-unresolvable").join("no-such-project");
    let path = missing.display().to_string();
    let (result, stdout) =
        run_capture(&["xirang", "explain", "--overlay", "--json", "--path", &path]);
    assert!(result.is_err(), "an unresolvable package must fail");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["schema"], "xirang.explain-overlay/1");
    assert_eq!(document["kind"], "static-projection");
    assert!(
        document["error"]
            .as_str()
            .unwrap_or_default()
            .contains("cannot resolve"),
        "{document}"
    );
}

/// `grafts` lists a readable plan with its selector, target path, graft,
/// `full`, and declared state, and reports an unreadable one with its
/// reason. Read-only: no file under the host changes.
/// `grafts` 列出一条可读计划的 selector、目标路径、graft、`full` 与声明状态，并把
/// 不可读的计划连原因一起报出。只读：宿主下没有文件被改动。
#[test]
fn grafts_json_lists_plans_and_declared_state() {
    let root = fixture_host("cli-grafts", DECLARED_BUTTON, &control_tree());
    let plan_dir = root.join(".xirang/external-grafts/button_fast");
    fs::create_dir_all(&plan_dir).expect("plan directory");
    let target =
        NodeId::from_namespaced_path("cli-grafts", "control/object/button/button.rs", "Button");
    let document =
        GraftPlanDocument::new(target, "root/control/object/button", "button_fast", false);
    fs::write(
        plan_dir.join("graft.plan"),
        document.render_graft_plan_document(),
    )
    .expect("plan file");
    let broken = root.join(".xirang/external-grafts/broken_graft");
    fs::create_dir_all(&broken).expect("broken directory");
    fs::write(broken.join("graft.plan"), "version=9\n").expect("broken plan");

    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["xirang", "grafts", "--json", &path]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let report: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(report["schema"], "xirang.grafts/1");
    let plans = report["plans"].as_array().expect("plans");
    assert_eq!(plans.len(), 2);
    let button = plans
        .iter()
        .find(|plan| plan["selector"] == "button_fast")
        .expect("button plan");
    assert_eq!(button["target_path"], "root/control/object/button");
    assert_eq!(button["graft"], "button_fast");
    assert_eq!(button["full"], false);
    assert_eq!(button["declared"], true);
    assert_eq!(button["declared_by"]["graft"], "button_fast");
    let broken_row = plans
        .iter()
        .find(|plan| plan["selector"] == "broken_graft")
        .expect("broken plan");
    assert!(broken_row["error"].as_str().is_some(), "{broken_row}");
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// A plans directory that exists and cannot be read is a failure, not "no
/// plans": the command's one answer is the declaration state of each plan, and
/// an unreadable directory leaves it with nothing to answer. The document still
/// reaches stdout, so a reader sees whatever was readable.
/// 存在却读不了的计划目录是失败，而不是"没有计划"：本命令唯一的答案是每条计划的声明状态，
/// 而读不了的目录让它无话可答。文档仍会写到 stdout，因此读者能看到可读的部分。
#[test]
fn grafts_reports_an_unreadable_plans_directory() {
    let root = fixture_host("cli-grafts-unreadable", DECLARED_BUTTON, &control_tree());
    // A file where the plans directory belongs, so the path exists and this is
    // not the ordinary "no plans yet" case.
    // 计划目录的位置放一个文件：路径存在，因此这不是普通的"还没有计划"。
    fs::create_dir_all(root.join(".xirang")).expect("xirang directory");
    fs::write(root.join(".xirang/external-grafts"), "not a directory").expect("blocking file");

    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["xirang", "grafts", "--json", &path]);
    let error = result.expect_err("an unreadable plans directory must fail");
    assert!(error.contains("external-grafts"), "{error}");
    let report: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(report["plans"].as_array().map(Vec::len), Some(0));

    std::fs::remove_dir_all(root).expect("cleanup");
}

/// A host whose source tree cannot be read is a failure too: the declaration
/// column would otherwise be guessed from an empty face list.
/// 读不了源码树的宿主同样是失败：否则声明那一列会由一个空的面清单猜出来。
#[test]
fn grafts_reports_a_host_whose_sources_cannot_be_read() {
    let root = temporary_root("cli-grafts-no-src");
    // Cargo needs a target, and this package has one that is not a library
    // target: XiRang's source root follows the **library** target, so a
    // bin-only package without `src/` keeps the conventional root and `grafts`
    // reports that it is absent. (A `[lib] path` outside `src/` used to stand in
    // for this, until the build learned to read that layout.)
    // Cargo 需要一个 target，而这个包的 target 不是库目标：XiRang 的源码根跟随**库**目标，
    // 因此没有 `src/` 的纯二进制包沿用约定根，`grafts` 报告它不存在。（过去用 `src/` 之外的
    // `[lib] path` 来代替这一情形，直到构建学会读那种布局。）
    fs::create_dir_all(root.join("app")).expect("binary directory");
    fs::write(root.join("app/main.rs"), "fn main() {}\n").expect("binary target");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"no-src\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"no-src\"\npath = \"app/main.rs\"\n",
    )
    .expect("manifest");

    let path = root.display().to_string();
    let (result, _stdout) = run_capture(&["xirang", "grafts", &path]);
    let error = result.expect_err("a host without sources cannot be answered about");
    assert!(error.contains("source tree"), "{error}");

    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `explain --overlay` renders the static overlay projection: the declared
/// slot carries its replacement, and the plan records are listed.
/// `explain --overlay` 渲染静态覆盖投影：已声明槽位带出它的替换件，并列出计划记录。
#[test]
fn explain_overlay_renders_the_static_projection() {
    let root = fixture_host("cli-overlay", DECLARED_BUTTON, &control_tree());
    let plan_dir = root.join(".xirang/external-grafts/button_fast");
    fs::create_dir_all(&plan_dir).expect("plan directory");
    let target =
        NodeId::from_namespaced_path("cli-overlay", "control/object/button/button.rs", "Button");
    let document =
        GraftPlanDocument::new(target, "root/control/object/button", "button_fast", false);
    fs::write(
        plan_dir.join("graft.plan"),
        document.render_graft_plan_document(),
    )
    .expect("plan file");

    let path = root.display().to_string();
    let (checked, check_stdout) = run_capture(&["xirang", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");
    let (result, stdout) =
        run_capture(&["xirang", "explain", "--overlay", "--json", "--path", &path]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let report: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(report["schema"], "xirang.explain-overlay/1");
    assert_eq!(report["kind"], "static-projection");
    let button = report["slots"]
        .as_array()
        .expect("slots")
        .iter()
        .find(|slot| slot["path"] == "root/control/object/button")
        .expect("declared slot stays in the kept list");
    assert_eq!(button["replacement"]["graft"], "button_fast");
    // A parent face is not a declared slot but still ships, because the
    // selected child needs it; calling it "pruned" would misreport the tree.
    // 父面不是已声明槽位但仍然发布，因为被选中的子级需要它；说它"被剪掉"就是
    // 错误描述这棵树。
    assert!(
        report["slots"]
            .as_array()
            .expect("slots")
            .iter()
            .any(|slot| slot["path"] == "root/control/object" && slot["kept"] == true),
        "a needed parent stays in the kept list: {report}"
    );
    assert_eq!(
        report["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .filter(|plan| plan["selector"] == "button_fast")
            .count(),
        1
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `xirang studio` honours its path argument, so the path Studio's own error
/// text tells the reader to pass actually reaches project resolution. Before
/// this, the subcommand dropped the argument and opened whatever directory the
/// process happened to be in.
/// `xirang studio` 采纳它的路径参数，因此 Studio 自己的错误文本让读者传的路径确实
/// 到达项目解析。此前该子命令丢弃参数，打开的是进程恰好所在的那个目录。
#[test]
fn studio_honours_its_path_argument() {
    let missing = temporary_root("cli-studio-arg").join("no-such-project");
    let path = missing.display().to_string();
    let (result, _stdout) = run_capture(&["xirang", "studio", &path]);
    let error = result.expect_err("an unusable path argument must be refused");
    assert!(
        error.contains("no-such-project"),
        "the refusal must name the path argument: {error}"
    );

    let (result, _stdout) = run_capture(&["xirang", "studio", "one", "two"]);
    let error = result.expect_err("two paths must be refused");
    assert!(error.contains("at most one"), "{error}");

    let (result, stdout) = run_capture(&["xirang", "studio", "--help"]);
    assert!(result.is_ok(), "{result:?}");
    assert!(
        stdout.contains("xirang studio [path]"),
        "help and dispatch must agree on the argument: {stdout}"
    );
}

/// `check --json` emits one JSON document even when the package cannot be
/// resolved, so a machine reader is never handed an empty stdout.
/// 包解析不出来时 `check --json` 仍输出一个 JSON 文档，机器读者绝不会拿到空的 stdout。
#[test]
fn check_json_reports_a_resolution_failure_as_json() {
    let missing = temporary_root("cli-check-unresolvable").join("no-such-project");
    let path = missing.display().to_string();
    let (result, stdout) = run_capture(&["xirang", "check", "--json", &path]);
    assert!(result.is_err(), "an unresolvable package must fail");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["schema"], "xirang.build-diagnostics/1");
    assert_eq!(document["count"], 1, "{document}");
    assert!(
        document["diagnostics"][0]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("cannot resolve"),
        "{document}"
    );

    // Without `--json`, stdout stays reserved for the human line it has always
    // been; the error still names the failure on stderr.
    // 不带 `--json` 时 stdout 仍是它一直以来的那条人类可读行；错误仍在 stderr 上点名失败。
    let (result, stdout) = run_capture(&["xirang", "check", &path]);
    assert!(result.is_err());
    assert!(stdout.is_empty(), "{stdout}");
}

/// A library target the manifest names but the filesystem does not have is
/// diagnosed through the same document, instead of taking the build down and
/// printing nothing.
/// 清单命名、而文件系统里没有的库目标经同一份文档被诊断，而不是打死构建并什么都不打印。
///
/// The manifest is legal — `[lib] path` is how Cargo is told where the library
/// is — so `cargo metadata` is asked about a package, and the pipeline reaches its
/// own layout resolution: the named file does not exist, which is a
/// `face-layout` diagnostic naming it. A *present* target outside `src/` is
/// supported now and builds (`cli::check_accepts_a_library_target_outside_src`).
/// It used to reach `expect("src directory must exist")` for either case: exit 101
/// and an empty stdout, which is exactly the contract `check --json` was fixed to
/// keep.
/// manifest 是合法的——`[lib] path` 正是告诉 Cargo 库在哪里的方式——因此包会被 `cargo metadata`
/// 询问，管线走到自己的布局解析：被命名的文件不存在，这就是一条点名它的 `face-layout` 诊断。
/// 位于 `src/` 之外但**存在**的目标现在被支持并能构建（`cli::check_accepts_a_library_target_outside_src`）。
/// 过去两种情形都会走到 `expect("src directory must exist")`：退出 101 与空 stdout，而那正是
/// `check --json` 被修好要守住的契约。
#[test]
fn check_reports_a_library_target_that_is_missing_as_a_diagnostic() {
    let root = temporary_root("cli-check-missing-lib");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nname = \"probe\"\npath = \"host/lib.rs\"\n",
    )
    .expect("manifest");
    let path = root.display().to_string();

    let (result, stdout) = run_capture(&["xirang", "check", "--json", &path]);
    assert!(result.is_err(), "a package without src/ must fail");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["count"], 1, "{document}");
    assert_eq!(
        document["diagnostics"][0]["phase"], "face-layout",
        "{document}"
    );
    assert!(
        document["diagnostics"][0]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("host/lib.rs"),
        "the refusal must name the target it looked for: {document}"
    );

    // The human run keeps its shape: the failure is the returned error, and
    // stdout stays reserved for the document `--json` would have written.
    // 人类可读运行保持原样：失败由返回的错误给出，stdout 仍留给 `--json` 本会写出的文档。
    let (result, stdout) = run_capture(&["xirang", "check", &path]);
    assert!(result.is_err());
    assert!(stdout.is_empty(), "{stdout}");

    let _ = std::fs::remove_dir_all(&root);
}

/// A library target outside `src/` is read where it is: the build succeeds on a
/// host whose faces live beside its library root, and `explain` reports the source
/// path the host's own build would stamp — `host/control/control.rs`, the
/// manifest-relative path, because the declaration macros drop only a leading
/// `src/`. This is the positive half of the test above: that one pins the refusal
/// for a target that is not there, this one pins the support for a target that is.
/// `src/` 之外的库目标就地读取：注册面住在库根旁边的宿主能构建成功，而 `explain` 报告的源码路径
/// 正是宿主自己的构建会盖下的那个——`host/control/control.rs`，相对清单的路径，因为声明宏只去掉
/// 一个前导 `src/`。这是上一条测试的正向一半：那条钉住"目标不在"时的拒绝，这条钉住"目标在"时的
/// 支持。
#[test]
fn check_accepts_a_library_target_outside_src() {
    let root = temporary_root("cli-outside-src");
    let face = root.join("host/control/control.rs");
    fs::create_dir_all(face.parent().expect("face parent")).expect("face directory");
    // The canonical rule beside the registry-owning face: without it the tree is one no crate can
    // compile, and the build refuses it by name.
    // 拥有注册机的面旁边那份规范规则：没有它，这棵树任何 crate 都编译不过，而构建会按名拒绝它。
    let rule = root.join("host/control/registry_rule/registry_rule.rs");
    fs::create_dir_all(rule.parent().expect("rule parent")).expect("rule directory");
    fs::write(&rule, REGISTRATION_RULE_SOURCE).expect("rule");
    fs::write(
        &face,
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n    \
         parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    fs::write(root.join("host/lib.rs"), "// host entry\n").expect("library root");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cli-outside-src\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"host/lib.rs\"\n",
    )
    .expect("manifest");
    let path = root.display().to_string();

    let (checked, stdout) = run_capture(&["xirang", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {stdout}");

    let (explained, stdout) = run_capture(&[
        "xirang",
        "explain",
        "--json",
        "--path",
        &path,
        "root/control",
    ]);
    assert!(explained.is_ok(), "{explained:?} {stdout}");
    let document: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(document["resolved"], true, "{document}");
    assert_eq!(
        document["node"]["source"], "host/control/control.rs",
        "the identity path keeps the directory the target lives in: {document}"
    );
    assert_eq!(document["node"]["path"], "root/control", "{document}");
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// `grafts --json` emits one JSON document even when the package cannot be
/// resolved, and the document carries the failure.
/// 包解析不出来时 `grafts --json` 仍输出一个 JSON 文档，且文档携带该失败。
#[test]
fn grafts_json_reports_a_resolution_failure_as_json() {
    let missing = temporary_root("cli-grafts-unresolvable").join("no-such-project");
    let path = missing.display().to_string();
    let (result, stdout) = run_capture(&["xirang", "grafts", "--json", &path]);
    assert!(result.is_err(), "an unresolvable package must fail");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["schema"], "xirang.grafts/1");
    assert_eq!(
        document["plans"].as_array().map(Vec::len),
        Some(0),
        "{document}"
    );
    assert!(
        document["error"]
            .as_str()
            .unwrap_or_default()
            .contains("cannot resolve"),
        "{document}"
    );
}

/// `--help` is help and succeeds; a bare invocation is not a success.
/// `--help` 是帮助、应当成功；裸调不是成功。
///
/// The bare case used to print the usage and return `Ok`, so a shell pipeline read
/// `xirang` with no arguments as having done something. The usage still goes out —
/// a caller who typed nothing needs to see it — and the status now says the command
/// did not run (audit `LGC-LG-44`).
/// 裸调过去打印用法并返回 `Ok`，于是 shell 管线把不带参数的 `xirang` 读成做了事。用法照常输出
/// ——什么都没敲的调用方需要看到它——而状态现在说明命令没有运行（审计 `LGC-LG-44`）。
#[test]
fn help_succeeds_and_a_missing_command_does_not() {
    assert!(run(["xirang".to_owned(), "--help".to_owned()]).is_ok());
    let error = run(["xirang".to_owned()]).expect_err("a bare invocation is not a success");
    assert!(error.contains("no command given"), "{error}");
}

/// Every subcommand answers `-h`/`--help` the same way, and `new --help` no longer
/// takes the flag as a package name.
/// 每个子命令都以同一方式回答 `-h`/`--help`，而 `new --help` 不再把旗标当成包名。
///
/// Before this, the same flag had four meanings: refused by four subcommands, a full
/// usage from `studio`, a one-line usage from `new`, and handed to cargo by `build`
/// (audit `LGC-LG-44`).
/// 在这之前，同一个旗标有四种含义：被四个子命令拒绝、由 `studio` 给出完整用法、由 `new` 给出一行
/// 用法、被 `build` 交给 cargo（审计 `LGC-LG-44`）。
#[test]
fn every_subcommand_answers_help_with_the_usage() {
    for command in [
        "new", "check", "build", "snippets", "explain", "grafts", "studio",
    ] {
        for flag in ["-h", "--help"] {
            let (result, stdout) = run_capture(&["xirang", command, flag]);
            assert!(
                result.is_ok(),
                "{command} {flag} must be help, not an error: {result:?}"
            );
            assert!(
                stdout.contains("USAGE:") && stdout.contains(&format!("xirang {command}")),
                "{command} {flag} must print the usage banner: {stdout}"
            );
        }
    }
}

#[test]
fn unknown_command_is_an_error() {
    let result = run(["xirang".to_owned(), "bogus".to_owned()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown command 'bogus'"));
}

#[test]
fn new_requires_a_package_name() {
    let result = run(["xirang".to_owned(), "new".to_owned()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("package name"));
}

/// An option-shaped token is not a package name. This subcommand has no `--help` branch, so
/// `xirang new --help` used to take `--help` as the name and scaffold `./--help` — it was
/// the only subcommand that did not refuse the input, and the mistake wrote into whatever
/// directory the shell was in.
/// 以选项形状出现的 token 不是包名。本子命令没有 `--help` 分支，因此 `xirang new --help`
/// 过去把 `--help` 当名字并在 `./--help` 里搭起脚手架——它是唯一不拒绝这种输入的子命令，而这个
/// 错误会写进 shell 当时所在的目录。
#[test]
fn an_option_shaped_name_is_refused() {
    // `--help` left this list when it became help: the flag is answered with the
    // usage banner, and only non-help options are option-shaped names (audit `LGC-LG-44`).
    // `--help` 从这份清单里移除了——它现在是帮助：该旗标以用法横幅作答，只有非帮助的选项才是
    // "长得像选项的名字"（审计 `LGC-LG-44`）。
    for option in ["--json", "-x"] {
        let error = run(["xirang".to_owned(), "new".to_owned(), option.to_owned()])
            .expect_err("an option is not a name");
        assert!(
            error.contains(option) && error.contains("usage"),
            "the refusal names the option and the usage: {error}"
        );
    }
}

#[test]
fn build_args_split_leading_path_from_cargo_options() {
    let owned = |items: &[&str]| {
        items
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
    };
    let (path, rest) = split_build_args(&owned(&["app", "--release"]));
    assert_eq!(path.as_deref(), Some("app"));
    assert_eq!(rest, owned(&["--release"]));
    let (path, rest) = split_build_args(&owned(&["--release"]));
    assert_eq!(path, None);
    assert_eq!(rest, owned(&["--release"]));
    let (path, rest) = split_build_args(&[]);
    assert_eq!(path, None);
    assert!(rest.is_empty());
}

/// An unknown editor, or a path next to a config-dir editor, is refused
/// before anything is written.
/// 未知编辑器，或给"写配置目录"的编辑器附带路径，都在写盘之前拒绝。
#[test]
fn snippets_refuses_an_unknown_editor_and_a_stray_path() {
    let unknown = run([
        "xirang".to_owned(),
        "snippets".to_owned(),
        "--editor".to_owned(),
        "emacs".to_owned(),
    ])
    .expect_err("unknown editor");
    assert!(unknown.contains("unknown editor 'emacs'"), "{unknown}");
    assert!(unknown.contains("vscode, nvim, blink"), "{unknown}");

    let stray = run([
        "xirang".to_owned(),
        "snippets".to_owned(),
        "--editor".to_owned(),
        "nvim".to_owned(),
        "/tmp/somewhere".to_owned(),
    ])
    .expect_err("stray path");
    assert!(stray.contains("editor config"), "{stray}");
}

/// The command injects a parseable editor file covering the whole kernel
/// vocabulary, and refuses arguments it does not understand.
/// 该命令注入一个可解析、覆盖整个内核词表的编辑器文件，并拒绝看不懂的参数。
#[test]
fn snippets_injects_the_editor_file() {
    let root = std::env::temp_dir().join(format!(
        "xirang-toolchain-snippets-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("temporary directory");
    run([
        "xirang".to_owned(),
        "snippets".to_owned(),
        root.display().to_string(),
    ])
    .expect("snippets command");

    let path = root.join(crate::build_method::scaffold::SNIPPET_FILE);
    let text = std::fs::read_to_string(&path).expect("editor file");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    let snippets = parsed.as_object().expect("an object of snippets");
    assert_eq!(
        snippets.len(),
        18,
        "sixteen value-like fields the default (`Soil`) spelling authors, plus the two type-like ones; \
         `registry_name` is build-supplied, so it gets no snippet (maintainer ruling 2026-10-16)"
    );
    let parent = snippets.get("xirang-parent").expect("the parent snippet");
    assert_eq!(parent["prefix"][0], "xirang-parent");
    assert_eq!(
        parent["body"][0], ".under($1)",
        "a field snippet writes the default (`Soil`) builder-call shape"
    );
    assert_eq!(parent["scope"], "rust");

    assert!(
        run([
            "xirang".to_owned(),
            "snippets".to_owned(),
            "--bogus".to_owned()
        ])
        .is_err()
    );
    assert!(
        run([
            "xirang".to_owned(),
            "snippets".to_owned(),
            "a".to_owned(),
            "b".to_owned()
        ])
        .is_err()
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn temporary_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("temporary directory");
    root
}

/// A non-UTF-8 argument is refused by name instead of panicking.
/// 非 UTF-8 参数会被点名拒绝，而不是 panic。
#[cfg(unix)]
#[test]
fn a_non_utf8_argument_is_refused_by_name() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let bad = OsString::from_vec(b"/tmp/proj\xff".to_vec());
    let error = super::argv_strings([OsString::from("xirang"), bad]).expect_err("refused");
    assert!(
        error.contains("is not valid UTF-8"),
        "the refusal names the reason: {error}"
    );
    // A valid argv still converts, so the check refuses bytes rather than the feature.
    // 合法 argv 照常转换，因此这道检查拒绝的是字节而不是功能。
    let argv = super::argv_strings([OsString::from("xirang"), OsString::from("check")])
        .expect("valid arguments");
    assert_eq!(argv, ["xirang", "check"]);
}

/// Two dependency sources at once is refused, exactly as `build` refuses a target
/// and a `--manifest-path` that name different projects.
/// 一次给两个依赖来源会被拒绝，正如 `build` 拒绝点名不同项目的目标与 `--manifest-path`。
///
/// `--path` and `--git` used to be resolved by `(Some(workspace), _)`, so naming
/// both silently ignored the URL and scaffolded against a local workspace the
/// caller had also pointed away from. `USAGE` already spells them as alternatives,
/// so refusing is what the command line promised (audit `LGC-LG-45`).
/// `--path` 与 `--git` 过去由 `(Some(workspace), _)` 决定，因此两者都给会静默忽略 URL，并对调用方
/// 同时明确排除过的本地工作区搭脚手架。`USAGE` 本就把两者写成互斥，因此拒绝才是命令行承诺的事
/// （审计 `LGC-LG-45`）。
#[test]
fn two_dependency_sources_are_refused() {
    let error = run([
        "xirang".to_owned(),
        "new".to_owned(),
        "app".to_owned(),
        "--path".to_owned(),
        "/tmp/workspace".to_owned(),
        "--git".to_owned(),
        "https://example.invalid/repo".to_owned(),
    ])
    .expect_err("two sources must be refused");
    assert!(
        error.contains("--path") && error.contains("--git") && error.contains("one of them"),
        "the refusal names both and says to pass one: {error}"
    );
}

/// When no workspace can take the generated packages, the sentence names a way out that **works**.
/// 当没有任何工作区能收下生成包时，那句话给出一条**真的能用**的出路。
///
/// Measured on a host that declares a `[workspace]` of its own: the packages land beside it, `cargo build
/// --workspace` succeeds **without touching any of them**, and the only report was "no member list" (audit
/// `M7`, §M7.64). Two candidate ways out were tried and rejected before this sentence was written, which is
/// why the negative half matters: `members = ["../crates/…"]` is refused ("not hierarchically below the
/// workspace root"), and a workspace at the partition root is refused too ("multiple workspace roots found in
/// the same workspace") — the first version of this sentence offered the second one, and it would have sent a
/// reader down a road that does not exist.
/// 在一个自带 `[workspace]` 的宿主上实测：包落在它旁边，`cargo build --workspace` **成功却不碰其中任何一个**，
/// 而唯一的报告是"没有成员清单"（审计 `M7`，§M7.64）。这句话写出来之前试过并否掉了两条候选出路，这也是否定的
/// 那一半要紧的原因：`members = ["../crates/…"]` 被拒（"not hierarchically below the workspace root"），
/// 在分区根建工作区也被拒（"multiple workspace roots found in the same workspace"）——这句话的第一版给的正是
/// 第二条，它会把读者送上一条不存在的路。
#[test]
fn a_split_that_belongs_to_no_workspace_says_how_to_build_it() {
    let said = super::crates_command::members_reply(false, false, std::path::Path::new("/work"));
    assert!(
        said.contains("--manifest-path"),
        "the way out that was measured to work is named: {said}"
    );
    assert!(
        said.contains("no workspace can take these packages") && said.contains("/work"),
        "it says what is wrong and where: {said}"
    );
    for rejected in ["remove the host's", "delete the host's"] {
        assert!(
            !said.contains(rejected),
            "`{rejected}` was measured **not** to work and must not be offered again: {said}"
        );
    }
}
