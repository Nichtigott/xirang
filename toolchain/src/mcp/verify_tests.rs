//! Tests for the verify loop: a healthy tree passes and the run publishes the
//! evidence the delta reads, and a broken tree fails by naming the node.
//! 校验闭环的测试：健康的树通过、且那次运行发布了差异所读的证据；坏掉的树以点名节点的失败作答。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::verify;
use crate::mcp::apply::apply;

/// A throwaway package with no faces.
/// 一个没有注册面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-verify-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// The provider/consumer pair the kernel admits.
/// 内核允许的那一对提供者与消费者。
fn provider_and_consumer(root: &Path) {
    apply(
        root,
        &json!({"action": "add", "apply": true,
                "fields": {"module": "label", "kind": "Label", "needs_registry": true,
                           "provides": "cap.render"}}),
    )
    .expect("the provider is admitted");
    apply(
        root,
        &json!({"action": "add", "parent": "root/label", "apply": true,
                "fields": {"module": "slider", "kind": "Slider",
                           "requires": "cap.render=>Label"}}),
    )
    .expect("the consumer is admitted");
}

/// A healthy tree passes, and because the same call published the build evidence the
/// delta below it describes *this* tree: face for face, not the difference against a
/// stale build.
/// 健康的树通过；而且同一次调用发布了构建证据，因此它下面的差异描述的是**这棵**树：逐面一致，而不是
/// 与一次过期构建之间的差别。
#[test]
fn a_healthy_tree_passes_and_the_run_publishes_the_evidence() {
    let (root, _) = package("healthy");
    apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is added");
    let reply = verify(&root, &json!({})).expect("the verdict renders");
    assert!(reply.contains("verdict ok"), "{reply}");
    assert!(
        reply.contains("scope: static registration only; Rust compilation/tests NOT run"),
        "{reply}"
    );
    assert!(reply.contains("build current"), "{reply}");
    assert!(
        reply.contains("the build matches the sources face for face"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Two packages verified from one process each keep their own namespace.
/// 在一个进程里校验两个包时，各自保留自己的命名空间。
///
/// The build pipeline's namespace used to be a first-write-wins process pin, so
/// the *second* package's published evidence was stamped with the first one's
/// namespace and this tool reported every face as re-identified — a wrong answer
/// that looks like a right one. Measured on CI and reproduced here with
/// `--test-threads=1`: `re-identified 1` with the first package's id on the run
/// side. The check is now scoped to the run, and this pins it through the tool
/// an agent actually calls rather than through the internals.
/// 构建管线的命名空间过去是"先到先得"的进程固定值，因此**第二个**包发布的证据会盖上第一个包的
/// 命名空间，本工具于是把每个面都报成身份变了——一个看起来正确的错误答案。CI 上实测到，并在本地用
/// `--test-threads=1` 复现：`re-identified 1`，运行侧是第一个包的 id。现在该判断被限定在这一次运行
/// 内，而这条测试通过代理真正调用的工具、而不是内部实现来钉住它。
#[test]
fn a_second_package_in_one_process_keeps_its_own_namespace() {
    for label in ["first", "second"] {
        let (root, name) = package(label);
        apply(
            &root,
            &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
        )
        .expect("the face is added");
        let reply = verify(&root, &json!({})).expect("the verdict renders");
        assert!(reply.contains(&format!("namespace {name}")), "{reply}");
        assert!(
            reply.contains("the build matches the sources face for face"),
            "{reply}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// A tree whose requirement has no provider fails the verdict, and the diagnostic the
/// kernel already wrote names the offending node and its source location.
/// 需求没有提供者的树会让判断失败，而内核本来就写好的诊断点名了出问题的节点与它的源码位置。
#[test]
fn a_broken_tree_fails_the_verdict_and_names_the_node() {
    let (root, _) = package("broken");
    provider_and_consumer(&root);
    let path = root.join("src/label/label.rs");
    let source = std::fs::read_to_string(&path).expect("the provider's source");
    let without = source
        .replace(".provides(&[\"cap.render\"])", ".provides(&[])")
        .replace("provides: [\"cap.render\"],", "provides: [],")
        .replace("provides: &[\"cap.render\"],", "provides: &[],");
    assert_ne!(
        source, without,
        "the fixture must have a provides field to drop"
    );
    std::fs::write(&path, without).expect("the hand edit lands");
    let reply = verify(&root, &json!({})).expect("the verdict renders");
    assert!(reply.contains("verdict failed"), "{reply}");
    // The requirements diagnostic names the phase, the consumer node, its source
    // line, the field it wanted and the provider kind it expected — so the reply is
    // actionable without a second call.
    // requirements 诊断点名了阶段、消费者节点、它的源码行、它想要的字段与它期望的提供者 kind——
    // 因此这份回复不需要第二次调用就能行动。
    assert!(reply.contains("phase=requirements"), "{reply}");
    assert!(reply.contains("missing capability"), "{reply}");
    assert!(reply.contains("field=cap.render"), "{reply}");
    assert!(reply.contains("expected=Label"), "{reply}");
    assert!(reply.contains("label/object/slider/slider.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The tree the two judging surfaces disagree on, and why the reply carries both.
/// 两个判断面会给出不同结论的那一棵树，以及回复为何两行都带。
///
/// The static verdict judges the faces the build ships, and the authoring
/// connector judges the faces on disk. An entry that names one face narrows the
/// build's scope to it, so the widget below is left out of the static
/// requirements pass while `load_registry` still registers it — the same
/// divergence that made `verify` report `verdict ok` on a tree `apply`, `usages`
/// and `converge` refused (audit `F1`). The reply has to carry both lines,
/// because both are true of this tree and only one of them says "green light".
/// 静态判断评的是构建会发布的面，创作连接器评的是磁盘上的面。点名了一个面的入口会把构建
/// 作用域收窄到它，于是下面的 widget 被排除在静态需求检查之外，而 `load_registry` 仍会注册
/// 它——正是这处分歧让 `verify` 在一棵被 `apply`、`usages` 与 `converge` 拒绝的树上报
/// `verdict ok`（审计 `F1`）。回复必须两行都带，因为两行对这棵树都成立，而只有其中一行说的是
/// "绿灯"。
#[test]
fn a_connector_rejection_is_reported_next_to_the_static_verdict() {
    let (root, _) = package("connector");
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
    let reply = verify(&root, &json!({})).expect("the verdict renders");
    assert!(reply.contains("verdict ok"), "{reply}");
    assert!(reply.contains("connector verdict: rejected"), "{reply}");
    assert!(
        reply.contains("the package's own faces were rejected"),
        "{reply}"
    );
    assert!(
        reply.contains("input `cap.theme` has no provider"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A directory that is not a package is refused by name before anything runs.
/// 不是包的目录会在任何东西运行之前被按名拒绝。
#[test]
fn a_directory_without_a_manifest_is_refused() {
    let bare = std::env::temp_dir().join(format!("mcp-verify-bare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    let error = verify(&bare, &json!({})).expect_err("a non-package is refused");
    assert!(error.contains("needs a Cargo package"), "{error}");
    let _ = std::fs::remove_dir_all(&bare);
}

/// `verify` declares `limit` and `root`, and only those reach `diff`: forwarding the whole
/// request let an undeclared `records: true` replace the promised tree delta with the records
/// report, so the same call silently answered a different question than its description
/// promises.
/// `verify` 只声明 `limit` 与 `root`，只有它们会传进 `diff`：把整个请求透传，会让一个未声明的
/// `records: true` 把承诺的树差异换成记录报告——同一次调用因此静默回答了与它描述所承诺的另一个问题。
#[test]
fn an_undeclared_argument_does_not_change_the_delta() {
    let (root, _) = package("undeclared");
    provider_and_consumer(&root);
    let plain = verify(&root, &json!({})).expect("the report renders");
    let noisy = verify(&root, &json!({"records": true})).expect("the report renders");
    assert_eq!(
        plain, noisy,
        "an argument this tool does not declare is not forwarded"
    );
    assert!(
        !noisy.contains("records"),
        "the answer is the tree delta, not the records report: {noisy}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The namespace the run publishes under is the **readers'** one, even when
/// `XIRANG_NAMESPACE` overrides it. `verify` used to hand `check_for` Cargo's package name
/// while `diff`/`search` resolve identities through `registry::namespace` (which honours the
/// override), so a tree that had just verified as healthy came back with *every* face
/// `re-identified` — a wrong answer that looks like a right one (audit `LGC-LG-13`).
/// 本次运行用于发布的命名空间是**读取者**的那一个，即使 `XIRANG_NAMESPACE` 覆盖了它。`verify`
/// 过去把 Cargo 的包名交给 `check_for`，而 `diff`/`search` 经 `registry::namespace`（认可覆盖值）
/// 解析身份，于是刚刚校验为健康的树会带着**每个**面 `re-identified` 回来——一个看起来正确的错误答案
/// （审计 `LGC-LG-13`）。
///
/// The override is process-global, so the run happens in a child process of this test binary:
/// setting it in-process would race every other test that reads a namespace (the same reason
/// `registry.rs` exposes `namespace_from` for tests).
/// 覆盖是进程级的，因此这次运行发生在本测试二进制的子进程里：在进程内设置它会与其它每个读命名空间的
/// 测试抢跑（这也是 `registry.rs` 为测试暴露 `namespace_from` 的原因）。
#[test]
fn the_override_is_the_namespace_the_run_publishes_under() {
    let (root, _) = package("namespace-override");
    apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is added");
    // `apply` schedules a background refresh (audit `M7`, P1.2) and this test spawns a reader right
    // after it: without waiting, the child can read records the refresh rewrote under **this**
    // process's namespace while the child reads as its own — which the reader now refuses by name
    // (§M7.18) instead of answering wrongly. Waiting here makes the test deterministic without
    // hiding anything: the product's answer for that state is a refusal, and it is pinned separately.
    // `apply` 会排一次后台刷新（审计 `M7`，P1.2），而本测试紧接着要起一个读者：不等它，子进程就可能读到
    // 被这次刷新改写成**本进程**命名空间的记录，而它按自己的命名空间读——那种状态读者现在会点名拒绝
    // （§M7.18），而不是给出错答案。在这里等待让测试变确定，且不掩盖任何东西：产品对那种状态的答复是拒绝，
    // 而它由另一条钉子单独钉住。
    assert!(
        crate::mcp::index::wait_until_idle(&root, std::time::Duration::from_secs(60)),
        "the refresh this test started must finish before it reads"
    );
    let child = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--ignored",
            "--exact",
            "mcp::verify::verify_tests::the_override_child",
            "--nocapture",
        ])
        .env("XIRANG_NAMESPACE", "t48-override")
        .env("T48_FIXTURE", &root)
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(stdout.contains("verdict ok"), "{stdout}");
    assert!(
        stdout.contains("re-identified 0"),
        "a freshly verified tree must not read as one whose every identity moved: {stdout}"
    );
    assert!(
        !stdout.contains("re-identified 1"),
        "the writer and the readers must use one namespace: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The child half of [`the_override_is_the_namespace_the_run_publishes_under`]: it runs under the
/// override the parent sets, on the fixture the parent built.
/// 上面那条钉子的子进程半边：它在父进程设置的覆盖值下、在父进程搭好的夹具上运行。
#[test]
#[ignore = "run by the_override_is_the_namespace_the_run_publishes_under in a child process"]
fn the_override_child() {
    let root = PathBuf::from(std::env::var("T48_FIXTURE").expect("the fixture path"));
    let verdict = verify(&root, &json!({})).expect("the verdict renders");
    println!("verdict line: {}", verdict.lines().next().unwrap_or(""));
    let delta = crate::mcp::diff::diff(&root, &json!({})).expect("the delta renders");
    for line in delta.lines() {
        if line.starts_with("added ") || line.starts_with("faces ") {
            println!("{line}");
        }
    }
}

/// The 2026-10-10 defect queue, item ③ ("`verify` 别往树里写记录"), decided by measurement: `verify`
/// publishes the build evidence and touches **nothing else** in the tree.
/// 2026-10-10 缺陷队列第 ③ 条（"`verify` 别往树里写记录"），用实测裁定：`verify` 发布构建证据，而树里
/// **别的地方一个字节都不动**。
///
/// Measured through the real bridge on a copy of the shipped example first: outside `target/` the tree
/// was 14 files before and 14 files after, byte-identical, while `<pkg>/target/xirang/out/` gained the
/// published evidence. That is the same split the build itself uses (generated Rust goes to Cargo's
/// `OUT_DIR`, the evidence to `<pkg>/target/xirang/out/`, and the bridge reads *that* one), so the
/// item as worded does not hold: what `verify` writes into the tree is its own excluded output
/// directory. The remaining half of the complaint — that this directory ignores a caller's target dir
/// — is the queue's items ④/⑤ and is recorded there, not here.
/// 先用真桥在出厂示例的副本上实测：`target/` 之外，调用前后都是 14 个文件、逐字节相同，而
/// `<包>/target/xirang/out/` 多出了发布的证据。这正是构建自己用的那套分工（生成的 Rust 进 cargo 的
/// `OUT_DIR`，证据进 `<包>/target/xirang/out/`，而桥读的是后者），因此这条按原话**不成立**：`verify`
/// 往树里写的只有它自己的那个被排除的输出目录。剩下的一半——这个目录不认调用方给的 target 目录——是
/// 队列的第 ④/⑤ 条，记在那边，不在这一条。
#[test]
fn verify_publishes_evidence_and_touches_nothing_else_in_the_tree() {
    let (root, _) = package("touches-nothing");
    apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is added");
    // Everything the package owns **except** its own output directory: sources, manifest, and every
    // `.xirang/` record. The comparison is a file list plus the bytes of each file, because "the file
    // is still there" and "the file is unchanged" are different claims.
    // 这个包拥有的一切，**除了**它自己的输出目录：源码、清单、以及 `.xirang/` 下的每条记录。比的是
    // 文件清单加上每个文件的字节，因为"文件还在"与"文件没变"是两个不同的主张。
    let snapshot = |root: &Path| -> Vec<(String, Vec<u8>)> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(directory) = stack.pop() {
            for entry in std::fs::read_dir(&directory)
                .expect("the tree reads")
                .flatten()
            {
                let path = entry.path();
                let relative = path
                    .strip_prefix(root)
                    .expect("below the root")
                    .to_string_lossy()
                    .into_owned();
                if relative == "target" || relative.starts_with("target/") {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                } else {
                    files.push((relative, std::fs::read(&path).expect("the file reads")));
                }
            }
        }
        files.sort();
        files
    };
    let before = snapshot(&root);
    let reply = verify(&root, &json!({})).expect("the verdict renders");
    assert!(reply.contains("verdict ok"), "{reply}");
    assert_eq!(
        before,
        snapshot(&root),
        "the run published its evidence under `target/` and touched nothing else"
    );
    assert!(
        root.join("target/xirang/out/generated_lib.rs").is_file(),
        "and the evidence really is there, so this is not a green from doing nothing"
    );
    let _ = std::fs::remove_dir_all(&root);
}
