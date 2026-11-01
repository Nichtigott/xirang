//! The build-time graph, published as one artifact beside the records (audit `M7`, P1.1).
//! 构建期的图，作为记录旁边的一个产物发布（审计 `M7`，P1.1）。
//!
//! The facts this file carries were already in the build's hands — the resolved parent chain, the
//! file each face lives in, the names each face's file calls — but they were spread across
//! `pruning_manifest.tsv`, `file_manifest.tsv` and the reader's own re-derivation, so "who depends on
//! whom" was a question every reader had to answer for itself. This is that answer as **data**: one
//! row per edge, with the node count, the edge count and a content hash in the header.
//! 这份文件承载的事实本来就在构建手里——解析后的父链、每个面所在的文件、每个面的文件调用的名字——但它们
//! 散落在 `pruning_manifest.tsv`、`file_manifest.tsv` 与读者各自的重新推导里，于是"谁依赖谁"成了每个读者
//! 都得自己回答的问题。这里把那个答案变成**数据**：每条边一行，头部带节点数、边数与内容哈希。
//!
//! The one resolution it does make is the **unambiguous** one: a called name that exactly one file in
//! this tree declares is a dependency on that file, so the edge is `calls-file`; a name two files
//! declare, or none, stays a `name:` node. Ambiguity is therefore published as ambiguity rather than
//! guessed, and a reader that can resolve it further does — the read path's call graph owns that rule
//! 它做的唯一一次解析是**无歧义**的那一次：一个恰好被这棵树里一份文件声明的被调用名字，就是对那份文件的
//! 依赖，于是边是 `calls-file`；被两份文件声明、或没有任何文件声明的名字留作 `name:` 节点。因此歧义按歧义
//! 发布，而不是猜一个；能进一步解析它的读者去解析——那条规则属于读路径的调用图。
//!
//! A graph that could not be published is **not** written at all: the pipeline only writes the
//! fingerprint after every payload landed, so a partial graph would be read as a complete one.
//! 发布不出来的图**一个字节都不写**：管线只在每份载荷都落地之后才写指纹，因此半份图会被读成完整的。

use std::fmt::Write as _;
use std::path::Path;

use super::face_view::PruningRow;
use super::registry_syntax::GraftSyntax;
use super::{graft_view::graft_cut_label, write_if_changed};

/// The first line of the graph artifact, so a reader can name what it is holding.
/// 图产物的第一行，好让读者说出自己手里是什么。
pub(crate) const GRAPH_MARKER: &str = "xirang-build-graph";

/// What the writer published: the counts and the digest a reader (and the readiness record) needs.
/// 写入方发布了什么：计数与摘要——读者（以及就绪记录）需要的东西。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GraphHeader {
    /// How many distinct nodes the edges name.
    /// 边点名了多少个不同的节点。
    pub(crate) nodes: usize,
    /// How many edges the body holds.
    /// 正文里有多少条边。
    pub(crate) edges: usize,
    /// How many faces the edges name.
    /// 边点名了多少个面。
    pub(crate) faces: usize,
    /// The digest of the edge body, which is what a reader compares.
    /// 边正文的摘要，也就是读者用来比对的东西。
    pub(crate) digest: String,
}

/// One edge of the build-time graph, with the kind that made it.
/// 构建期图的一条边，连同构成它的那种关系。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    from: String,
    to: String,
    kind: &'static str,
}

/// Write the graph the record's own rows describe (audit `M7`, P1.1).
/// 写出记录自己的行所描述的图（审计 `M7`，P1.1）。
///
/// The rows are handed in rather than read back from `pruning_manifest.tsv`: the graph is a second
/// view of one computation, so the two files cannot disagree about a face, and the 50,000-row parse
/// that reading them back would cost is the shape this batch keeps removing.
/// 行是交进来的，而不是从 `pruning_manifest.tsv` 读回来的：图是同一次计算的第二个视图，因此两份文件不可能
/// 对一个面产生分歧，而读回来要付的五万行解析正是这一批一直在去掉的那种形状。
pub(crate) fn write_graph_manifest(
    out_dir: &Path,
    rows: &[PruningRow],
    file_rows: &[(String, String, String)],
    grafts: &[GraftSyntax],
) -> Result<GraphHeader, String> {
    // Which file declares each bare name, and `None` as soon as two of them do: the ambiguity has to
    // be remembered rather than dropped, or a name declared twice would silently become an edge to
    // whichever file happened to be visited last.
    // 每个裸名由哪份文件声明，而一旦有两份就记 `None`：歧义必须被记住而不是丢掉，否则被声明两次的名字会
    // 静默变成"指向最后访问到的那份文件"的边。
    let mut declared_in: std::collections::BTreeMap<&str, Option<&str>> =
        std::collections::BTreeMap::new();
    for (source, function, _) in file_rows {
        if function == "-" {
            continue;
        }
        // The scanner reports a declaration as `module::name`, while a call site spells the bare name.
        // 扫描器把声明报成 `module::name`，而调用点拼的是裸名。
        let bare = function.rsplit("::").next().unwrap_or(function.as_str());
        declared_in
            .entry(bare)
            .and_modify(|known| {
                if known.is_some_and(|file| file != source.as_str()) {
                    *known = None;
                }
            })
            .or_insert(Some(source.as_str()));
    }
    let mut edges = Vec::new();
    for row in rows {
        let face = format!("face:{}", row.id);
        edges.push(Edge {
            from: face.clone(),
            to: format!("file:{}", row.source),
            kind: "in",
        });
        // A resolved parent that is this face itself would be a self-edge, which says nothing; the
        // reader of the record already treats an unresolvable chain as "derive".
        // 解析出来等于自身的父级是一条什么也没说的自环；记录的读者本来就把解析不出的链当作"推导"。
        if let Some(parent) = row.parent_node.as_deref()
            && parent != "-"
            && parent != row.id.to_string()
        {
            edges.push(Edge {
                from: face.clone(),
                to: format!("face:{parent}"),
                kind: "parent",
            });
        }
        for name in row
            .calls
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty() && *name != "-")
        {
            // A call that crosses files is a **dependency between compilation units**, so it is
            // recorded between the two files — that is the edge a partitioner has to keep inside one
            // crate. A call that stays inside its own file is not a dependency between units, so it
            // keeps the face as its origin and the name as its target; and a name two files declare
            // (or none) cannot be an edge at all, so it stays a name.
            // 跨文件的调用是**编译单元之间的依赖**，因此记在两份文件之间——那正是分区器必须留在同一个 crate
            // 里的边。留在自己文件里的调用不是单元之间的依赖，因此它保留面作出发点、名字作目标；而被两份文件
            // 声明（或没有文件声明）的名字根本成不了边，于是留作名字。
            match declared_in.get(name).copied().flatten() {
                Some(source) if source != row.source => edges.push(Edge {
                    from: format!("file:{}", row.source),
                    to: format!("file:{source}"),
                    kind: "calls-file",
                }),
                _ => edges.push(Edge {
                    from: face.clone(),
                    to: format!("name:{name}"),
                    kind: "calls",
                }),
            }
        }
    }
    // The graft plan is a dependency too: the host's cut site takes the grafted face, and the two
    // are spelled as Rust paths here because that is what the plan carries — resolution belongs to
    // the reader that has the declarations (see the module doc).
    // 嫁接计划也是一种依赖：宿主的切口接纳被嫁接的面，而两者在这里按计划携带的样子拼成 Rust 路径——解析
    // 属于手里有声明的读者（见模块文档）。
    for graft in grafts {
        edges.push(Edge {
            from: format!(
                "cut:{}",
                graft_cut_label(&graft.cut, graft.cut_end.as_deref())
            ),
            to: format!("graft:{}", graft.graft),
            kind: "graft",
        });
    }
    edges.sort();
    edges.dedup();

    let nodes: std::collections::BTreeSet<&str> = edges
        .iter()
        .flat_map(|edge| [edge.from.as_str(), edge.to.as_str()])
        .collect();
    let mut body = String::new();
    for edge in &edges {
        writeln!(body, "{}\t{}\t{}", edge.from, edge.to, edge.kind).unwrap();
    }
    let digest = super::registry_identity::NodeId::from_bytes(body.as_bytes()).to_string();
    // `faces` counts the faces the **record** published, not the `face:` nodes: the graph also names
    // the package root, which has no row, and a success line saying "2502 faces" beside a `registry`
    // that says "2501 faces" would be two numbers for one tree.
    // `faces` 数的是**记录**发布的面，而不是 `face:` 节点：图还会点名包根，而包根没有行；一行
    // "2502 faces" 的成功提示挨着一个说 "2501 faces" 的 `registry`，会是同一棵树的两个数。
    let faces = rows
        .iter()
        .map(|row| row.id)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let mut output = format!(
        "# graph\t{GRAPH_MARKER}\n# nodes\t{}\n# edges\t{}\n# digest\t{digest}\n# from\tto\tkind\n",
        nodes.len(),
        edges.len()
    );
    output.push_str(&body);
    write_if_changed(&out_dir.join(xirang_kernel::lexicon::GRAPH_FILE), &output)?;
    Ok(GraphHeader {
        nodes: nodes.len(),
        edges: edges.len(),
        faces,
        digest,
    })
}

/// Write the readiness record for the index this run just published (audit `M7`, P2.1).
/// 为这次运行刚刚发布的索引写下就绪记录（审计 `M7`，P2.1）。
///
/// This is the **last** thing a clean run writes, and it is what makes "the index is ready" a fact a
/// reader can check instead of a duration it has to guess: the pipeline writes it only after every
/// payload landed, so its presence means this run finished, and its absence means the previous one
/// (or none) is what is on disk. The generation counts **runs**, so a reader can see that a refresh
/// happened even when the bytes did not change; the digest and the stamp are what say whether the
/// bytes did.
/// 这是干净的一次运行写的**最后**一样东西，也是让"索引已就绪"成为读者可检查的事实、而不是它必须猜的时长
/// 的东西：管线只在每份载荷都落地之后才写它，因此它在＝这次运行完成了，它不在＝磁盘上是上一次（或者没有）。
/// generation 数的是**运行次数**，因此即使字节没变读者也能看到发生过一次刷新；而摘要与戳说的是字节到底变没变。
pub(crate) fn write_generation(
    out_dir: &Path,
    root: &Path,
    header: &GraphHeader,
) -> Result<(), String> {
    let path = out_dir.join(xirang_kernel::lexicon::GENERATION_FILE);
    let generation = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("generation\t"))
                .and_then(|value| value.trim().parse::<u64>().ok())
        })
        .unwrap_or(0)
        + 1;
    let (source_files, newest) = super::source_stamp(root);
    // The namespace is stamped because it is an **input to every identity in these records** and it
    // comes from the environment, not from the sources: a reader under another namespace computes a
    // different id for the same face, so without this line it cannot tell "these records are not
    // mine" from "every face moved" (audit 2026-10-06, §M7.18).
    // 命名空间被盖进来，因为它是这些记录里**每个身份的输入**，而且来自环境、不来自源码：在另一个命名空间下
    // 的读者会为同一个面算出不同的 id，因此没有这一行它就无法把"这些记录不是我的"与"每个面都搬家了"分开
    // （2026-10-06 审计，§M7.18）。
    let output = format!(
        "# generation\t{marker}\ngeneration\t{generation}\ndigest\t{digest}\nfaces\t{faces}\n\
         files\t{source_files}\ngraph_nodes\t{nodes}\ngraph_edges\t{edges}\n\
         stamp\t{source_files}:{newest}\nfinished_at\t{clock}\n{key}{namespace}\n",
        marker = xirang_kernel::lexicon::GENERATION_MARKER,
        digest = header.digest,
        faces = header.faces,
        nodes = header.nodes,
        edges = header.edges,
        clock = clock(),
        key = xirang_kernel::lexicon::GENERATION_NAMESPACE_KEY,
        namespace = super::registry_identity::package_namespace(),
    );
    write_if_changed(&path, &output)
}

/// The wall-clock time as `HH:MM:SS` in UTC, the shape the bridge's own reports print.
/// 挂钟时间的 UTC `HH:MM:SS`，也就是桥自己的报告打印的形状。
///
/// UTC rather than local time for the same reason every other moment in this tree is: a report whose
/// clock silently followed the environment would be a different token on two machines. It is spelled
/// here rather than borrowed from the bridge's freshness reporter because this file is written by the
/// **build**, which has no bridge; a pin holds the shape, which is what the two readers share.
/// 用 UTC 而不是本地时间，理由与本树其它每个时刻相同：一个悄悄跟着环境走的时刻会在两台机器上成为不同的凭据。
/// 它写在这里而不是借用桥的新鲜度报告，是因为这份文件由**构建**写、而构建没有桥；一枚钉子守住这个形状，那才
/// 是两个读者共用的东西。
fn clock() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let (hours, minutes, seconds) = ((seconds / 3600) % 24, (seconds / 60) % 60, seconds % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod graph_tests;
