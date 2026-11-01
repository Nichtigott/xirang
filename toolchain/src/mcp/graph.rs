//! `xirang.graph`: who depends on whom, read from the build's own graph (audit `M7`, P1.3).
//! `xirang.graph`：谁依赖谁，读的是构建自己的图（审计 `M7`，P1.3）。
//!
//! The graph is an **accelerator**, not a verdict: `graph_edges.tsv` is published by the last
//! finished build, and every answer here opens with the index line, so a reader always sees whether
//! the edges describe the sources on disk now. When they do not, the answer says so and keeps
//! reading — a graph query has no derivation to fall back on, which is exactly why the line has to be
//! in the answer rather than left to the caller to ask for.
//! 图是**加速器**，不是裁决：`graph_edges.tsv` 由上一次完成的构建发布，而这里的每份答案都以索引那一行开头，
//! 因此读者总能看到这些边描述的是不是磁盘上此刻的源码。不是时，答案说出来并继续读——图查询没有可回退的推导，
//! 这正是那一行必须在答案里、而不是留给调用方自己去问的原因。
//!
//! Three questions, one tool: what the graph holds (no arguments), what one node reaches and what
//! reaches it (`node`, with `direction` and `depth`), and which nodes are inseparable (`cycles`).
//! 三个问题、一个工具：图里有什么（不给参数）、一个节点能到哪以及谁到得了它（`node`，配 `direction` 与
//! `depth`）、以及哪些节点分不开（`cycles`）。
//!
//! A node is named by **the record's own vocabulary** — a logical path, a declared path, a kind, a
//! `registry_name` or a source path resolves to the face the record published, and a raw node key
//! (`face:`/`file:`/`name:`/`cut:`/`graft:`) is taken as written. Resolution is the record's job
//! because the record is what holds those spellings; this tool does not invent a second vocabulary.
//! 节点用**记录自己的词汇**来点名——逻辑路径、声明路径、kind、`registry_name` 或源码路径会解析到记录发布的
//! 那个面，而裸节点键（`face:`/`file:`/`name:`/`cut:`/`graft:`）按字面取用。解析是记录的职责，因为那些拼写
//! 就住在记录里；本工具不发明第二套词汇。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::mcp::index::Graph;

/// How many edges one answer prints before it says it withheld the rest.
/// 一份答案在说出其余被截下之前打印多少条边。
const DEFAULT_LIMIT: usize = 20;

/// The deepest walk one answer takes, so a query cannot turn into a whole-graph dump.
/// 一份答案最深走几层，因此一次查询不会变成整张图的倾倒。
const MAX_DEPTH: usize = 3;

/// The kinds that are **dependencies** between compilation units.
/// 属于编译单元之间**依赖**的那几种边。
///
/// `in` and `parent` say where a face lives, and `calls` is a name the build could not resolve to a
/// declaration — none of the three is a reason two things must ship together. Cycles are computed
/// over this set and the answer says so, because "no cycles" over the wrong edge set would be a
/// reassuring lie.
/// `in` 与 `parent` 说一个面住在哪，而 `calls` 是构建没能解析到声明的名字——三者都不是"两样东西必须一起发布"
/// 的理由。环在这套边上计算，而答案会说出来，因为在错误的边集上说"没有环"是一句让人安心的假话。
const DEPENDENCY_KINDS: &[&str] = &["calls-file", "graft"];

/// The catalogue's description of this tool, kept beside the handler so the two cannot drift apart.
/// 本工具在目录里的描述，与处理函数放在一起，因此两者不会漂开。
pub(crate) const DESCRIPTION: &str = "Follow the build's own graph: who depends on whom, as the last finished build \
             published it. With no arguments it is a census — how many nodes and edges, the edge \
             kinds, and the nodes with the most edges touching them. With `node` it is a \
             neighbourhood: the node resolved through the **record's** vocabulary (logical path, \
             declared path, kind, `registry_name`, source path, or a raw `face:`/`file:`/`name:`/\
             `cut:`/`graft:` key) and the edges within `depth`, in `direction` — `out`, `in` or \
             `both`. With `cycles: true` it is the strongly connected components over the \
             **dependency** edges (`calls-file`, `graft`), which are the sets that must not be split \
             across crates; structural `in`/`parent` edges and unresolved `calls` names are not \
             dependencies and the answer says so. Every answer opens with the index line: whether \
             `graph_edges.tsv` describes the sources on disk now, which generation published it, and \
             what to run when it is behind — a graph query has no derivation to fall back on, so \
             that line is part of the answer rather than something to ask for.";

/// The input schema the catalogue advertises for this tool.
/// 目录为本工具广告的输入 schema。
pub(crate) fn schema() -> serde_json::Value {
    serde_json::json!({"type":"object","properties":{
        "node":{"type":"string","description":"a face (logical path, declared path, kind, registry_name or source path) or a raw node key (`face:…`, `file:…`, `name:…`, `cut:…`, `graft:…`)"},
        "direction":{"type":"string","enum":["out","in","both"],"description":"which way to walk from `node` (default both)"},
        "depth":{"type":"integer","minimum":1,"maximum":3,"description":"how many hops (default 1)"},
        "cycles":{"type":"boolean","description":"answer with the strongly connected components over the dependency edges instead of a neighbourhood"},
        "limit":{"type":"integer","minimum":1,"maximum":200,"description":"how many rows to print (default 20)"},
        "root":{"type":"string"}
    }})
}

/// Answer one graph question (audit `M7`, P1.3).
/// 回答一个图上的问题（审计 `M7`，P1.3）。
pub(crate) fn graph(root: &Path, arguments: &Value) -> Result<String, String> {
    let state = crate::mcp::index::state(root);
    let mut output = crate::mcp::index::line(root, &state);
    output.push('\n');
    let published = match crate::mcp::index::read_graph(root) {
        Ok(graph) => graph,
        Err(error) => {
            // On a graph that cannot be read, the line above is the answer: it already says whether
            // nothing was published, whether the record is damaged, or whether a refresh is running.
            // 图读不了时，上面那一行就是答案：它已经说出是没有发布过、记录坏了、还是有一次刷新在跑。
            output.push_str(&format!("graph  unavailable ({error})\n"));
            return Ok(output);
        }
    };
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| (value as usize).clamp(1, 200));
    if arguments.get("cycles").and_then(Value::as_bool) == Some(true) {
        output.push_str(&cycles(&published));
        return Ok(output);
    }
    let Some(query) = arguments.get("node").and_then(Value::as_str) else {
        output.push_str(&census(&published, limit));
        return Ok(output);
    };
    let nodes = resolve_nodes(root, query)?;
    let direction = arguments
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("both");
    let depth = arguments
        .get("depth")
        .and_then(Value::as_u64)
        .map_or(1, |value| (value as usize).clamp(1, MAX_DEPTH));
    output.push_str(&neighbourhood(
        &published, &nodes, direction, depth, limit, query,
    ));
    Ok(output)
}

/// What the graph holds, grouped by kind, with the busiest nodes (audit `M7`, P1.3).
/// 图里有什么，按种类分组，并给出最忙的节点（审计 `M7`，P1.3）。
fn census(published: &Graph, limit: usize) -> String {
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, kind) in &published.edges {
        *by_kind.entry(kind.as_str()).or_default() += 1;
    }
    let kinds = by_kind
        .iter()
        .map(|(kind, count)| format!("{kind} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut degrees: BTreeMap<&str, usize> = BTreeMap::new();
    for (from, to, _) in &published.edges {
        *degrees.entry(from.as_str()).or_default() += 1;
        *degrees.entry(to.as_str()).or_default() += 1;
    }
    let mut ranked: Vec<(&str, usize)> = degrees.into_iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(right.0)));
    let mut output = format!(
        "graph  {} node(s), {} edge(s) — {kinds}\n",
        published.nodes,
        published.edges.len()
    );
    output.push_str("busiest nodes (edges touching them):\n");
    let shown = ranked.len().min(limit);
    for (node, degree) in ranked.iter().take(shown) {
        output.push_str(&format!("  {degree:>5}  {node}\n"));
    }
    if ranked.len() > shown {
        output.push_str(&format!(
            "{} more node(s) withheld — raise `limit` (max 200) or name one with `node`\n",
            ranked.len() - shown
        ));
    }
    output.push_str(
        "ask about one node with `node` (logical path, kind, registry_name or source path), or ask \
         `cycles: true` for the components that must stay in one crate\n",
    );
    output
}

/// The nodes one query names, by the record's own spellings (audit `M7`, P1.3).
/// 一次查询点名的节点，按记录自己的拼写解析（审计 `M7`，P1.3）。
fn resolve_nodes(root: &Path, query: &str) -> Result<Vec<String>, String> {
    for prefix in ["face:", "file:", "name:", "cut:", "graft:"] {
        if query.starts_with(prefix) {
            return Ok(vec![query.to_owned()]);
        }
    }
    let out = crate::mcp::build_evidence::out_dir(root);
    let mut found = BTreeSet::new();
    if let Ok(rows) = crate::build_method::read_pruning_manifest(&out) {
        for row in rows {
            let spellings = [
                row.logical_path.as_deref(),
                row.path.as_deref(),
                row.kind.as_deref(),
                row.registry_name.as_deref(),
                Some(row.source.as_str()),
            ];
            if spellings
                .iter()
                .flatten()
                .any(|spelling| spelling.eq_ignore_ascii_case(query))
            {
                found.insert(format!("face:{}", row.id));
            }
        }
    }
    if !found.is_empty() {
        return Ok(found.into_iter().collect());
    }
    if root.join(query).is_file() {
        return Ok(vec![format!("file:{query}")]);
    }
    Err(format!(
        "`{query}` names nothing in this index. The graph knows: a face by logical path, declared \
         path, kind, `registry_name` or source path; a file by its path relative to `src/`; a call \
         target the build could not resolve as `name:<spelling>`; and the raw keys \
         `face:`/`file:`/`name:`/`cut:`/`graft:`. Call the tool with no `node` to see what the graph \
         holds."
    ))
}

/// What one node reaches, and what reaches it, within `depth` (audit `M7`, P1.3).
/// 一个节点在 `depth` 之内能到哪、以及谁到得了它（审计 `M7`，P1.3）。
fn neighbourhood(
    published: &Graph,
    start: &[String],
    direction: &str,
    depth: usize,
    limit: usize,
    query: &str,
) -> String {
    let out_edges = direction == "out" || direction == "both";
    let in_edges = direction == "in" || direction == "both";
    let mut seen: BTreeSet<String> = start.iter().cloned().collect();
    let mut frontier: BTreeSet<String> = seen.clone();
    let mut found: Vec<String> = Vec::new();
    for _ in 0..depth {
        let mut next = BTreeSet::new();
        for (from, to, kind) in &published.edges {
            for node in frontier.iter() {
                if out_edges && from == node && !seen.contains(to) {
                    found.push(format!("out  {node} --{kind}--> {to}"));
                    next.insert(to.clone());
                }
                if in_edges && to == node && !seen.contains(from) {
                    found.push(format!("in   {node} <--{kind}-- {from}"));
                    next.insert(from.clone());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        seen.extend(next.iter().cloned());
        frontier = next;
    }
    let mut output = format!(
        "node   {query} -> {}\n",
        start
            .iter()
            .map(|node| node.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if found.is_empty() {
        output.push_str(&format!(
            "no `{}` edge within depth {depth}; the node is in the graph with none of its own\n",
            if direction == "both" {
                "in or out"
            } else {
                direction
            }
        ));
        return output;
    }
    let shown = found.len().min(limit);
    for line in found.iter().take(shown) {
        output.push_str(&format!("  {line}\n"));
    }
    if found.len() > shown {
        output.push_str(&format!(
            "{} more edge(s) withheld — raise `limit`\n",
            found.len() - shown
        ));
    }
    output
}

/// The strongly connected components that must not be split across crates (audit `M7`, P1.3).
/// 不能切到不同 crate 里的强连通分量（审计 `M7`，P1.3）。
///
/// Kosaraju over the **dependency** edges only, iterative rather than recursive so a deep graph
/// cannot take the bridge down with a stack overflow — the same reason the kernel's parser carries a
/// nesting guard.
/// 只在**依赖**边上跑 Kosaraju，用迭代而不是递归，因此深图不会以爆栈带走整个桥——与内核解析器带嵌套守卫
/// 同一个理由。
fn cycles(published: &Graph) -> String {
    let mut forward: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut backward: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for (from, to, kind) in &published.edges {
        if !DEPENDENCY_KINDS.contains(&kind.as_str()) {
            continue;
        }
        forward.entry(from.as_str()).or_default().push(to.as_str());
        backward.entry(to.as_str()).or_default().push(from.as_str());
        nodes.insert(from.as_str());
        nodes.insert(to.as_str());
    }
    // First pass: finish order.
    let mut order: Vec<&str> = Vec::new();
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    for node in &nodes {
        if visited.contains(node) {
            continue;
        }
        let mut stack = vec![(*node, false)];
        while let Some((current, expanded)) = stack.pop() {
            if expanded {
                order.push(current);
                continue;
            }
            if !visited.insert(current) {
                continue;
            }
            stack.push((current, true));
            for next in forward.get(current).into_iter().flatten() {
                if !visited.contains(next) {
                    stack.push((next, false));
                }
            }
        }
    }
    // Second pass: components, in reverse finish order.
    let mut assigned: BTreeSet<&str> = BTreeSet::new();
    let mut components: Vec<Vec<&str>> = Vec::new();
    for node in order.iter().rev() {
        if assigned.contains(node) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![*node];
        assigned.insert(node);
        while let Some(current) = stack.pop() {
            component.push(current);
            for previous in backward.get(current).into_iter().flatten() {
                if assigned.insert(previous) {
                    stack.push(previous);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    let inseparable: Vec<&Vec<&str>> = components
        .iter()
        .filter(|component| component.len() > 1)
        .collect();
    let mut output = format!(
        "cycles {} component(s) of more than one node over {} dependency edge kind(s) ({})\n",
        inseparable.len(),
        DEPENDENCY_KINDS.len(),
        DEPENDENCY_KINDS.join(", ")
    );
    output.push_str(
        "structural `in`/`parent` edges and unresolved `calls` names are not dependencies and are \
         not counted here\n",
    );
    if inseparable.is_empty() {
        output.push_str(
            "nothing has to ship together by this edge set; that is not a claim about a call graph \
             that names a definition two files declare\n",
        );
        return output;
    }
    for (index, component) in inseparable.iter().enumerate() {
        output.push_str(&format!(
            "  component {} ({} nodes): {}\n",
            index + 1,
            component.len(),
            component.join(", ")
        ));
    }
    output
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod graph_tests;
