//! What else a change to one face touches, transitively.
//! 改动一个面还会牵动什么，按传递闭包给出。
//!
//! `xirang.usages` answers the direct neighbourhood and `xirang.converge` answers
//! one face's constraints; neither says how far a change travels. This does, over the
//! three dependency kinds this tree actually has, each of which is *declared* rather
//! than guessed: a face's parent owns it (so its descendants are in the radius), a
//! face that `requires` a capability the face provides is a consumer, and a declared
//! graft cut that names the face hands it over. The traversal is bounded by `depth`
//! and by a visited set, so a capability cycle is a shorter path rather than a hang,
//! and what it did not reach is reported as unreached instead of implied to be safe.
//! `xirang.usages` 回答直接邻域，`xirang.converge` 回答一个面的约束；两者都不说一次改动能走多远。
//! 这里说，而且只走这棵树真正拥有的三种依赖，每一种都是**声明**的而不是猜的：一个面的父级拥有它（因此
//! 它的后代在半径内）、`requires` 了该面所提供能力的面是消费者、点名该面的已声明 graft 切口会把该面交出去。
//! 遍历受 `depth` 与已访问集合限制，因此能力环变成一条更短的路径而不是死循环；没走到的会被如实报成
//! 未到达，而不是被暗示为安全。
//!
//! online: impact is a transitive property of the call graph, and the build publishes no call edges.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use crate::build_method::declared_grafts;
use serde_json::Value;

use crate::mcp::apply::load_registry;
use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::registry::namespace;
use crate::mcp::resolve::resolve_node;

/// The traversal depth when the caller names none.
/// 调用方没有指定时的遍历深度。
const DEFAULT_DEPTH: usize = 3;

/// The deepest traversal this tool will run, whatever the caller asks for.
/// 无论调用方要多少，本工具会跑的最深遍历。
const MAX_DEPTH: usize = 16;

/// One dependency edge: the node it reaches, and why it is in the radius.
/// 一条依赖边：它到达的节点，以及它为何在半径之内。
struct Edge {
    to: String,
    reason: String,
}

/// Report the transitive blast radius of one face.
/// 报告一个面的传递爆炸半径。
pub(crate) fn impact(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let (faces, unparsable) = crate::mcp::resolve::derived_faces(root, &namespace)?;
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "xirang.impact requires node (the face whose change to trace)".to_owned())?;
    let id = resolve_node(root, &namespace, target)?;
    let face = faces
        .iter()
        .find(|face| face.id == id)
        .ok_or_else(|| format!("no face in the derived tree has identity {id}"))?;
    let depth = number(arguments, "depth", DEFAULT_DEPTH, MAX_DEPTH);
    let limit = number(arguments, "limit", DEFAULT_LIMIT, 200);

    // The three declared dependency kinds, all pointing away from the face whose
    // change would travel along them.
    // 三种已声明的依赖，方向都背离"改动会沿它传播"的那个面。
    let mut edges: BTreeMap<String, Vec<Edge>> = BTreeMap::new();
    for child in &faces {
        if let Some(parent) = faces.iter().find(|candidate| candidate.id == child.parent) {
            edges.entry(parent.path.clone()).or_default().push(Edge {
                to: child.path.clone(),
                reason: format!("child of this face's registry (kind {})", child.kind),
            });
        }
    }
    let mut notes = Vec::new();
    match capabilities(root, &namespace, &faces) {
        Ok(consumers) => {
            for (provider, consumer, capability, kind) in consumers {
                edges.entry(provider).or_default().push(Edge {
                    to: consumer,
                    reason: format!("consumer: requires `{capability}=>{kind}`"),
                });
            }
        }
        Err(rejection) => notes.push(format!(
            "capability edges unavailable: this package's own faces are rejected ({})",
            rejection.lines().next().unwrap_or("").trim()
        )),
    }
    match declared_grafts(root) {
        Ok(declared) => {
            for cut in &declared.cuts {
                let label = cut.cut_label();
                for candidate in &faces {
                    if cut.names_face(&candidate.path, Some(&candidate.module)) {
                        edges.entry(candidate.path.clone()).or_default().push(Edge {
                            to: format!("graft `{label}`"),
                            reason: format!("declared cut at entry line {}", cut.line),
                        });
                    }
                }
            }
        }
        Err(rejection) => notes.push(format!("declared cuts unavailable: {rejection}")),
    }

    // Breadth-first, so the first time a node is reached is its shortest path; the
    // visited set is what makes a capability cycle terminate, and every further edge
    // into a node already reached still contributes its reason — a node that is both
    // a child and a consumer must say both, or the reader loses half the blast radius.
    // 广度优先，因此一个节点第一次被到达就是它的最短路径；已访问集合正是让能力环终止的东西，而每一条
    // 指向已到达节点的后续边仍会贡献它的理由——一个既是子面又是消费者的节点必须把两者都说出来，否则
    // 读取方就丢掉了半个爆炸半径。
    let mut reached: BTreeMap<String, Reached> = BTreeMap::new();
    let mut cycles = 0usize;
    let mut queue: VecDeque<(String, usize, String)> =
        VecDeque::from([(face.path.clone(), 0, face.path.clone())]);
    while let Some((current, distance, chain)) = queue.pop_front() {
        if distance >= depth {
            continue;
        }
        for edge in edges.get(&current).into_iter().flatten() {
            if edge.to == face.path {
                // Back to the origin: a capability cycle, counted rather than
                // walked, because the origin is where the walk started.
                // 回到起点：一个能力环，计数而不继续走，因为起点正是这次遍历的出发点。
                cycles += 1;
                continue;
            }
            match reached.entry(edge.to.clone()) {
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    let reached = entry.get_mut();
                    if !reached.reasons.contains(&edge.reason) {
                        reached.reasons.push(edge.reason.clone());
                    }
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let chain = format!("{chain} -> {} [{}]", edge.to, edge.reason);
                    entry.insert(Reached {
                        distance: distance + 1,
                        chain: chain.clone(),
                        reasons: vec![edge.reason.clone()],
                    });
                    queue.push_back((edge.to.clone(), distance + 1, chain));
                }
            }
        }
    }

    let mut output = format!(
        "namespace {namespace}\n{unparsable}node {}\n  path {}\n  kind {}\n",
        face.id, face.path, face.kind
    );
    for note in &notes {
        output.push_str(&format!("note: {note}\n"));
    }
    output.push_str(&format!(
        "faces {}  depth {depth}  limit {limit}\n",
        faces.len()
    ));
    let mut ordered: Vec<(&String, &Reached)> = reached.iter().collect();
    ordered.sort_by(|left, right| {
        (left.1.distance, left.0.as_str()).cmp(&(right.1.distance, right.0.as_str()))
    });
    output.push_str(&format!(
        "affected {} (transitive within depth)\n",
        ordered.len()
    ));
    for (index, (path, reached)) in ordered.iter().enumerate() {
        if index >= limit {
            output.push_str(&format!(
                "  {}\n",
                crate::mcp::truncation::withheld(
                    ordered.len() - limit,
                    ordered.len(),
                    limit,
                    "reached faces",
                    "raise `limit`, or lower `depth` to narrow the radius"
                )
            ));
            break;
        }
        let kind = faces
            .iter()
            .find(|candidate| candidate.path == **path)
            .map(|candidate| candidate.kind.as_str())
            .unwrap_or("-");
        output.push_str(&format!(
            "  hop {}  {path:<40} kind={kind}\n",
            reached.distance
        ));
        for reason in &reached.reasons {
            output.push_str(&format!("    because: {reason}\n"));
        }
        output.push_str(&format!("    how: {}\n", reached.chain));
    }
    if cycles > 0 {
        output.push_str(&format!(
            "cycles back to this face {cycles} (counted, not walked: the walk starts here)\n"
        ));
    }
    let unreached = faces
        .iter()
        .filter(|candidate| candidate.id != id && !reached.contains_key(&candidate.path))
        .count();
    if unreached > 0 {
        output.push_str(&format!(
            "not reached within depth {depth} {unreached} declared face(s) — no dependency path of \
             these kinds, not proof of independence\n"
        ));
    }
    output.push_str(
        "detail: xirang.usages (direct neighbourhood and capability tokens) · xirang.converge \
         (this face's constraints) · xirang.diff (what changed since the build). Graft records and \
         recorded traces that name this identity are not traversed.\n",
    );
    Ok(output)
}

/// One node the walk reached: how far, along which chain, and for which reasons.
/// 遍历到达的一个节点：多远、沿哪条链、以及因为哪些理由。
struct Reached {
    distance: usize,
    chain: String,
    reasons: Vec<String>,
}

/// The capability consumers of every face, as `(provider path, consumer path,
/// capability, provider kind)`.
/// 每个面的能力消费者，形如 `(提供者路径, 消费者路径, 能力, 提供者 kind)`。
fn capabilities(
    root: &Path,
    namespace: &str,
    faces: &[crate::build_method::FaceView],
) -> Result<Vec<(String, String, String, String)>, String> {
    let registry = load_registry(root, namespace)?;
    // Same rule as every other read-back: resolve against the package root the
    // context carries, never the process's own directory.
    // 与其它每一处读回同一条规矩：以上下文携带的包根为基准，绝不以进程自己的目录为基准。
    let context =
        crate::run_method::AuthoringContext::new(root.to_path_buf(), namespace.to_owned());
    let read_back = |id| context.scope(|| crate::run_method::authored_face(&registry, id));
    // Which kind offers which capability, and who wants what.
    // 哪个 kind 提供哪个能力，以及谁想要什么。
    let mut offered: BTreeMap<String, Vec<(String, BTreeSet<String>)>> = BTreeMap::new();
    let mut wanted: Vec<(String, String, String)> = Vec::new();
    for candidate in faces {
        let Ok(authored) = read_back(candidate.id) else {
            continue;
        };
        offered
            .entry(authored.kind.clone())
            .or_default()
            .push((candidate.path.clone(), tokens(&authored.provides)));
        for entry in authored.requires.split([',', '\n', ' ', '\t']) {
            let entry = entry.trim().trim_matches(['"', '[', ']', '(', ')']);
            if let Some((capability, provider)) = entry.split_once("=>") {
                wanted.push((
                    candidate.path.clone(),
                    capability.trim().to_owned(),
                    provider.trim().to_owned(),
                ));
            }
        }
    }
    let mut consumers = Vec::new();
    for (consumer, capability, provider) in wanted {
        for (provider_path, kind, provided) in offered
            .get(&provider)
            .map(|candidates| {
                candidates
                    .iter()
                    .map(|(path, provided)| (path.clone(), provider.clone(), provided))
            })
            .into_iter()
            .flatten()
        {
            if provided.contains(&capability) {
                consumers.push((provider_path, consumer.clone(), capability.clone(), kind));
            }
        }
    }
    Ok(consumers)
}

/// One caller's numeric argument, clamped into `1..=ceiling`.
/// 调用方给的数值参数，夹取到 `1..=ceiling`。
fn number(arguments: &Value, key: &str, fallback: usize, ceiling: usize) -> usize {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .map_or(fallback, |value| (value as usize).clamp(1, ceiling))
}

/// The identifier-ish tokens of a manifest text field.
/// 一个清单文本字段里的标识符式记号。
fn tokens(text: &str) -> BTreeSet<String> {
    text.split(|character: char| {
        !(character.is_alphanumeric() || character == '_' || character == '.' || character == ':')
    })
    .filter(|token| token.len() > 1 && token.chars().any(|c| c.is_alphanumeric()))
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
#[path = "impact_tests.rs"]
mod impact_tests;
