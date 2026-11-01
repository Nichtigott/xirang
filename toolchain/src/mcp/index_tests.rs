//! Pins for the published index: the readiness record, what it refuses, and the line a report
//! carries about it (audit `M7`, P2.1 / P2.3).
//! 已发布索引的钉子：就绪记录、它拒绝什么，以及一份报告关于它的那一行（审计 `M7`，P2.1 / P2.3）。

use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway package with one registration face, the same shape the other bridge tests use.
/// 一个含单个注册面的一次性包，与桥的其它测试所用的形状相同。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-index-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("package directory");
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    fs::create_dir_all(root.join("src/dial")).expect("face directory");
    fs::write(
        root.join("src/dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face source");
    (root, name)
}

/// Publish the index the way a real run does, through the entry the CLI and `verify` both drive.
/// 像真实的一次运行那样发布索引，经过 CLI 与 `verify` 都驱动的那个入口。
fn publish(root: &Path, name: &str) {
    crate::build_method::check_for(root, &crate::mcp::build_evidence::out_dir(root), name)
        .expect("a healthy tree checks clean");
}

/// A finished run publishes a readiness record, and the state says the index describes these sources
/// (audit `M7`, P2.1).
/// 一次完成的运行发布就绪记录，而状态说索引描述的正是这批源码（审计 `M7`，P2.1）。
#[test]
fn a_finished_run_publishes_a_record_that_says_ready() {
    let (root, name) = package("ready");
    publish(&root, &name);
    match super::state(&root) {
        super::State::Ready(generation) => {
            assert_eq!(generation.generation, 1, "the first run is generation one");
            assert!(generation.faces >= 1, "{generation:?}");
            assert!(
                generation.files >= 2,
                "the library target and the face: {generation:?}"
            );
            assert_eq!(generation.finished_at.len(), 8, "HH:MM:SS: {generation:?}");
            let line = super::line(&root, &super::State::Ready(generation));
            assert!(line.starts_with("graph updated: generation 1, "), "{line}");
        }
        other => panic!(
            "a finished run must read as ready, not `{}`",
            super::line(&root, &other)
        ),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A record that does not match the graph beside it is refused rather than half-read (audit `M7`,
/// P2.1).
/// 与旁边的图对不上的记录会被拒绝，而不是读一半（审计 `M7`，P2.1）。
///
/// "The index is ready" is exactly the claim a truncated or hand-edited file must not be able to
/// make, so both files are checked against each other and the digest is recomputed from the body.
/// "索引已就绪"正是被截断或手改过的文件绝不能作出的那个声称，因此两份文件互相核对，而摘要从正文重算。
#[test]
fn a_record_that_does_not_match_its_graph_is_refused() {
    let (root, name) = package("damaged");
    publish(&root, &name);
    let out = crate::mcp::build_evidence::out_dir(&root);
    let graph =
        fs::read_to_string(out.join(xirang_kernel::lexicon::GRAPH_FILE)).expect("the graph reads");
    // Cut the last edge out of the body without touching the header: the counts and the digest are
    // what must notice, and a reader that trusted the body would answer "no edge" where the truth is
    // "no such row".
    // 不碰头部、只把最后一条边从正文里切掉：该发现它的是计数与摘要，而相信正文的读者会在真相是"没有这一行"
    // 的地方答"没有边"。
    let mut lines: Vec<&str> = graph.lines().collect();
    let last = lines.pop().expect("a body line");
    fs::write(
        out.join(xirang_kernel::lexicon::GRAPH_FILE),
        format!("{}\n", lines.join("\n")),
    )
    .expect("the truncated graph writes");
    assert!(
        last.contains('\t'),
        "the fixture has an edge to cut: {last}"
    );
    match super::state(&root) {
        super::State::Damaged(why) => {
            assert!(why.contains("does not match its header"), "{why}");
        }
        other => panic!(
            "a truncated graph must be refused, not `{}`",
            super::line(&root, &other)
        ),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A record whose payload never landed is absent rather than believed (audit `M7`, P2.1).
/// 载荷从未落地的那份记录算"没有发布过"，而不是被相信（审计 `M7`，P2.1）。
#[test]
fn a_run_that_published_nothing_reads_as_absent() {
    let (root, _) = package("absent");
    assert!(
        matches!(super::state(&root), super::State::Absent),
        "a tree nothing published has no index"
    );
    let line = super::line(&root, &super::State::Absent);
    assert!(line.contains("run `xirang check`"), "{line}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Editing a source makes the index *behind*, and the line says so and names the way forward (audit
/// `M7`, P2.3).
/// 改动一份源码会让索引**落后**，而那一行说出来并点名出路（审计 `M7`，P2.3）。
#[test]
fn an_edit_makes_the_index_behind_and_the_line_says_so() {
    let (root, name) = package("behind");
    publish(&root, &name);
    fs::write(
        root.join("src/dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n}\npub fn paint() -> i32 { 7 }\n",
    )
    .expect("the edit lands");
    match super::state(&root) {
        super::State::Behind { published } => {
            assert_eq!(published.generation, 1);
            let line = super::line(&root, &super::State::Behind { published });
            assert!(
                line.starts_with("index behind: generation 1 covers "),
                "{line}"
            );
            assert!(line.contains("run `xirang check`") || line.contains("a refresh is running"));
        }
        other => panic!(
            "an edited tree must read as behind, not `{}`",
            super::line(&root, &other)
        ),
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The readiness record stamps the namespace it was published under; a record **without** the stamp
/// reads as "no namespace", and a reader then says nothing about namespaces instead of guessing.
/// 就绪记录盖下它发布时所用的命名空间；**没有**这枚戳的记录读出来是"没有命名空间"，读者此时对命名空间
/// 什么都不说，而不是猜。
///
/// Identity is `hash(namespace, source path, name)` and the namespace comes from the environment, so
/// this stamp is what lets a reader tell "these records are not mine" from "every face moved" (§M7.18).
/// 身份是 `hash(命名空间, 源码路径, 名字)`，而命名空间来自环境，因此这枚戳正是让读者把"这些记录不是我的"与
/// "每个面都搬了家"分开的东西（§M7.18）。
#[test]
fn the_record_stamps_the_namespace_and_old_records_read_as_none() {
    let (root, name) = package("namespace");
    publish(&root, &name);
    let generation = super::read_generation(&root).expect("the record reads");
    assert_eq!(
        generation.namespace.as_deref(),
        Some(name.as_str()),
        "the run's namespace is stamped"
    );

    // An older binary wrote no such line: the reader degrades to `None` rather than inventing one.
    // 更早的二进制不写这一行：读者降级成 `None`，而不是编一个。
    let path =
        crate::mcp::build_evidence::out_dir(&root).join(xirang_kernel::lexicon::GENERATION_FILE);
    let text = fs::read_to_string(&path).expect("the record is readable");
    let without: String = text
        .lines()
        .filter(|line| !line.starts_with(xirang_kernel::lexicon::GENERATION_NAMESPACE_KEY))
        .map(|line| format!("{line}\n"))
        .collect();
    fs::write(&path, without).expect("the stamp is removed");
    assert_eq!(
        super::read_generation(&root)
            .expect("a record without the stamp still reads")
            .namespace,
        None
    );
    let _ = std::fs::remove_dir_all(&root);
}
