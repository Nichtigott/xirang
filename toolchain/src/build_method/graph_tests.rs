//! Pins for the build-time graph: it is the record's own rows seen as edges, and its header vouches
//! for its body (audit `M7`, P1.1).
//! 构建期图的钉子：它是记录自己那些行按边来看的样子，而它的头部为它的正文作保（审计 `M7`，P1.1）。

use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway package with a parent face, a child whose `parent:` resolves to it, an orphan whose
/// `parent:` resolves to nothing, and a call inside the parent's file.
/// 一个一次性包：一个父面、一个 `parent:` 能解析到它的子面、一个 `parent:` 什么都解析不到的孤儿面，
/// 以及父面文件里的一处调用。
fn package(label: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-graph-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("src");
    fs::create_dir_all(src.join("dial")).expect("face directory");
    fs::write(
        src.join("dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n}\npub fn paint(&self) -> i32 { helper() }\n\
         fn helper() -> i32 { 7 }\n",
    )
    .expect("face file");
    fs::create_dir_all(src.join("child")).expect("child directory");
    fs::write(
        src.join("child/child.rs"),
        "crate::control_object! {\n    kind: Child,\n    parent: crate::dial::NODE_ID,\n}\n",
    )
    .expect("child face");
    fs::create_dir_all(src.join("orphan")).expect("orphan directory");
    fs::write(
        src.join("orphan/orphan.rs"),
        "crate::control_object! {\n    kind: Orphan,\n    parent: crate::nowhere::NODE_ID,\n}\n",
    )
    .expect("orphan face");
    (root, src)
}

/// Publish the record and the graph from **one** computation, the way the pipeline does.
/// 像管线那样，从**一次**计算发布记录与图。
fn publish(src: &Path, out: &Path) -> String {
    let nodes = crate::build_method::source_walk::discover_root(src);
    let rows = super::super::manifests::write_pruning_manifest(src, &nodes, out)
        .expect("the record writes");
    let files = super::super::manifests::write_file_manifest(src, &nodes, out)
        .expect("the file record writes");
    super::write_graph_manifest(out, &rows, &files, &[]).expect("the graph writes");
    fs::read_to_string(out.join("graph_edges.tsv")).expect("the graph reads back")
}

/// The graph's edges and the record's rows are two views of one computation, so they cannot disagree
/// about a face, a parent or a call (audit `M7`, P1.1).
/// 图的边与记录的行是同一次计算的两个视图，因此它们不可能对一个面、一个父级或一处调用产生分歧。
#[test]
fn the_graph_is_the_records_own_rows_seen_as_edges() {
    let (root, src) = package("rows");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let graph = publish(&src, &out);
    let rows = crate::build_method::read_pruning_manifest(&out).expect("the record reads");

    let body: Vec<&str> = graph
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("# "))
        .collect();
    let edges: Vec<(&str, &str, &str)> = body
        .iter()
        .map(|line| {
            let mut fields = line.split('\t');
            let from = fields.next().expect("from");
            let to = fields.next().expect("to");
            let kind = fields.next().expect("kind");
            assert!(
                fields.next().is_none(),
                "an edge row is exactly three columns: {line}"
            );
            (from, to, kind)
        })
        .collect();

    // Every face the record published is a node, and the **only** `face:` node the record has no row
    // for is the package root — the face every `parent:` chain ends at, which the record publishes
    // only through the logical paths of its children. "The graph knows a face the record does not"
    // is exactly the drift this artifact could introduce, and this is the half that refuses it.
    // 记录发布过的每个面都是一个节点，而记录没有对应行的 `face:` 节点**只有**包根——每条 `parent:` 链的
    // 终点，记录只经由子级的逻辑路径发布它。"图知道一个记录不知道的面"正是这份产物可能引入的漂移，而这一半
    // 就是拒绝它的地方。
    let recorded: std::collections::BTreeSet<String> =
        rows.iter().map(|row| format!("face:{}", row.id)).collect();
    let graphed: std::collections::BTreeSet<String> = edges
        .iter()
        .flat_map(|(from, to, _)| [*from, *to])
        .filter(|node| node.starts_with("face:"))
        .map(str::to_owned)
        .collect();
    let root_face = format!(
        "face:{}",
        super::super::registry_identity::package_root_node_id()
    );
    let extra: std::collections::BTreeSet<&String> = graphed.difference(&recorded).collect();
    assert!(
        extra.iter().all(|node| **node == root_face),
        "the graph may only add the package root to the record's faces, and it added {extra:?}"
    );
    assert!(
        recorded.is_subset(&graphed),
        "every published face is a node: {recorded:?} vs {graphed:?}"
    );

    let dial = rows
        .iter()
        .find(|row| row.source == "dial/dial.rs")
        .expect("the parent face");
    let child = rows
        .iter()
        .find(|row| row.source == "child/child.rs")
        .expect("the child face");
    let orphan = rows
        .iter()
        .find(|row| row.source == "orphan/orphan.rs")
        .expect("the orphan face");

    assert!(
        edges.contains(&(
            format!("face:{}", child.id).as_str(),
            format!("face:{}", dial.id).as_str(),
            "parent"
        )),
        "the resolved parent is an edge: {graph}"
    );
    assert!(
        !edges
            .iter()
            .any(|(from, _, kind)| *from == format!("face:{}", orphan.id) && *kind == "parent"),
        "an unresolved parent is not guessed into an edge: {graph}"
    );
    assert!(
        edges.contains(&(
            format!("face:{}", dial.id).as_str(),
            "file:dial/dial.rs",
            "in"
        )),
        "a face is in its file: {graph}"
    );
    assert!(
        edges.contains(&(format!("face:{}", dial.id).as_str(), "name:helper", "calls")),
        "a call that stays inside its own file keeps the face and the name: {graph}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A name two files declare is published as a name, not guessed into an edge to one of them (audit
/// `M7`, P1.1).
/// 两份文件都声明的名字按名字发布，而不是被猜成指向其中一份的边（审计 `M7`，P1.1）。
///
/// The rule the graph applies is exactly "**unambiguous** declarations", and its other half matters
/// more than its first: silently picking one of two files would turn a question the reader must ask
/// ("which one?") into an answer it will act on. This is the same asymmetry the record's own columns
/// keep — publish the fact, refuse the guess.
/// 图应用的那条规则正是"**无歧义**的声明"，而它的另一半比第一半更要紧：在两份文件里静默挑一份，会把读者
/// 必须问的问题（"是哪一份？"）变成一个它会照做的答案。这与记录自己的列保持的不对称是同一种——发布事实，
/// 拒绝猜测。
#[test]
fn a_name_two_files_declare_stays_a_name() {
    let (root, src) = package("ambiguous");
    // `helper` is declared twice, in two modules beside the face that calls it.
    // `helper` 被声明两次，就在调用它的那个面旁边的两个模块里。
    for module in ["twin", "third"] {
        fs::create_dir_all(src.join(module)).expect("module directory");
        fs::write(
            src.join(module).join(format!("{module}.rs")),
            "pub fn helper() -> i32 { 1 }\n",
        )
        .expect("declaring module");
    }
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let graph = publish(&src, &out);
    let dial = crate::build_method::read_pruning_manifest(&out)
        .expect("the record reads")
        .into_iter()
        .find(|row| row.source == "dial/dial.rs")
        .expect("the parent face");
    assert!(
        graph.contains(&format!("face:{}\tname:helper\tcalls", dial.id)),
        "an ambiguous name stays a name: {graph}"
    );
    assert!(
        !graph.contains("file:twin/twin.rs\tcalls-file")
            && !graph.contains("file:third/third.rs\tcalls-file"),
        "and it is not guessed onto either declaration: {graph}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A call that crosses files is an edge **between the two files**, which is the dependency a
/// partitioner has to keep inside one crate (audit `M7`, P1.1).
/// 跨文件的调用是**两份文件之间**的边，也就是分区器必须留在同一个 crate 里的那种依赖（审计 `M7`，P1.1）。
#[test]
fn a_call_that_crosses_files_is_an_edge_between_them() {
    let (root, src) = package("crossing");
    // The two faces call each other's uniquely declared function, so the file graph has a cycle —
    // the shape that must not be split across crates.
    // 两个面互相调用对方唯一声明的函数，因此文件图上有一个环——那正是不该被切到两个 crate 里的形状。
    fs::write(
        src.join("dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n}\npub fn dial_work() { other_work() }\n",
    )
    .expect("dial");
    fs::create_dir_all(src.join("other")).expect("other directory");
    fs::write(
        src.join("other/other.rs"),
        "crate::control_object! {\n    kind: Other,\n    parent: crate::dial::NODE_ID,\n}\n\
         pub fn other_work() { dial_work() }\n",
    )
    .expect("other");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let graph = publish(&src, &out);
    assert!(
        graph.contains("file:dial/dial.rs\tfile:other/other.rs\tcalls-file"),
        "the call crosses into the declaring file: {graph}"
    );
    assert!(
        graph.contains("file:other/other.rs\tfile:dial/dial.rs\tcalls-file"),
        "and back, which is the cycle: {graph}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The header's counts and digest describe the body, so "the graph is complete" is checkable rather
/// than assumed (audit `M7`, P1.1).
/// 头部的计数与摘要描述正文，因此"图是完整的"是可检查的而不是假定的（审计 `M7`，P1.1）。
#[test]
fn the_header_counts_and_digest_vouch_for_the_body() {
    let (root, src) = package("header");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let graph = publish(&src, &out);

    let header = |prefix: &str| -> String {
        graph
            .lines()
            .find_map(|line| line.strip_prefix(prefix))
            .unwrap_or_else(|| panic!("the header carries `{prefix}`: {graph}"))
            .trim()
            .to_owned()
    };
    assert_eq!(header("# graph\t"), super::GRAPH_MARKER);
    let nodes: usize = header("# nodes\t").parse().expect("a node count");
    let edges: usize = header("# edges\t").parse().expect("an edge count");
    let digest = header("# digest\t");

    let body_start =
        graph.find("# from\tto\tkind\n").expect("the column header") + "# from\tto\tkind\n".len();
    let body = &graph[body_start..];
    let counted: Vec<&str> = body.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(
        counted.len(),
        edges,
        "the edge count is the body's line count"
    );
    let named: std::collections::BTreeSet<&str> = counted
        .iter()
        .flat_map(|line| line.split('\t').take(2))
        .collect();
    assert_eq!(named.len(), nodes, "the node count is what the edges name");
    let recomputed =
        super::super::registry_identity::NodeId::from_bytes(body.as_bytes()).to_string();
    assert_eq!(
        digest, recomputed,
        "the digest is the body's own hash, so a truncated or edited body is detectable"
    );
    let _ = std::fs::remove_dir_all(&root);
}
