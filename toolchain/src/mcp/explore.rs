//! Current source and one-hop context behind one read-only entry point.
//! 一个只读入口给出当前源码与一跳上下文。复用内核索引；不声称解析动态派发。
//!
//! Selection precedes rendering: named files/functions reserve space before neighbours.
//! Every request rereads disk. A persistent MCP connection is not a model's memory.
//! 先选择再分配输出预算；点名对象优先。每次重读磁盘，不把持久连接当成模型记忆。
//!
//! online: editing context includes unbuilt files and current source spans that a published build cannot certify.
//! 编辑上下文包含尚未构建的文件与当前源码范围，已发布的构建记录无法为这些字节作证。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use super::source_index::SourceFile;
use super::source_query::{self, QuerySources};

#[path = "explore_context.rs"]
mod context;
#[path = "explore_render.rs"]
mod render;
#[path = "explore_repair.rs"]
mod repair;
#[path = "explore_structure.rs"]
mod structure;

/// The same read-first contract in discovery and the full catalogue.
/// 发现入口与完整目录共用同一份先读上下文的约定。
pub(super) const DESCRIPTION: &str = "Read code. Symbols/files return current source, lexical calls/tests \
(and unclaimed receiver call sites as candidates). \
Registered targets join contracts, providers/consumers, build/grafts and runtime/verification evidence. \
root/... reads connected faces; outside callers/tests unexamined. \
Text ranks docs/bodies; no resolved dispatch. scope skips package joins. \
known_context:[] requests receipts; reuse only while source is retained; clear after compaction. \
Omit for source. Check omissions; test edits.";

/// Input contract owned by the query, shared by both discovery surfaces.
/// 查询持有参数约定，两种发现入口共用。
pub(super) fn schema() -> Value {
    serde_json::json!({"type":"object","properties":{
        "query":{"type":"string","minLength":1,"maxLength":1000,"description":"Rust symbols, files, root/... object paths or symptoms; qualified names retain ambiguous candidates"},
        "max_files":{"type":"integer","minimum":1,"maximum":12,"default":5},
        "max_bytes":{"type":"integer","minimum":2000,"maximum":24000,"default":14000,"description":"UTF-8 text reply budget, excluding JSON envelope"},
        "scope":{"type":"array","maxItems":16,"items":{"type":"string"},"description":"Optional project-relative source directories/files. Only these are searched; outside callers/tests remain unexamined. Omit to search the whole tree."},
        "known_context":{"type":"array","maxItems":128,"items":{"type":"string","pattern":"^[0-9a-f]{64}$"},"description":"Opt-in retained-context receipts. [] requests receipts; later pass only receipts whose source is still in model context. Clear after compaction. Changed bytes or evidence are always resent; omitting this field always returns source."},
        "root":{"type":"string"}
    },"required":["query"]})
}

/// One file's selected ranges and static evidence, before allocating output bytes.
/// 输出预算分配前，一个文件被选中的范围与静态证据。
#[derive(Default)]
struct Selection {
    rank: u8,
    ranges: Vec<(usize, usize, u8)>,
    notes: BTreeSet<String>,
}

pub(crate) fn explore(root: &Path, arguments: &Value) -> Result<String, String> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty() && s.len() <= 1000)
        .ok_or("query must contain symbol/file names or symptom words (1..1000 UTF-8 bytes)")?;
    let max_files = bounded(arguments, "max_files", 5, 1, 12)?;
    let budget = bounded(arguments, "max_bytes", 14000, 2000, 24000)?;
    let tokens: BTreeSet<&str> = query
        .split(|c: char| !c.is_alphanumeric() && !matches!(c, '_' | '/' | '.' | ':' | '-'))
        .filter(|s| !s.is_empty())
        .collect();
    let names: BTreeSet<_> = tokens
        .iter()
        .map(|token| token.rsplit("::").next().unwrap_or(token))
        .collect();
    let request = context::Context::from_request(root, arguments)?;
    let mut joined = if request.scopes.is_empty() {
        structure::Joined::explicit(root, query, max_files.max(3))?
    } else {
        None
    };
    let structural_scopes = joined.as_ref().map(structure::Joined::scopes);
    let mut context = QuerySources::within(
        root,
        structural_scopes.as_deref().unwrap_or(&request.scopes),
    )?;
    context.parse_matching(|file| {
        tokens
            .iter()
            .any(|token| file_matches(&file.relative, token))
            || names.iter().any(|name| file.source.contains(name))
            || joined
                .as_ref()
                .is_some_and(|joined| joined.is_target(&file.relative))
    });
    let mut selected: BTreeMap<usize, Selection> = BTreeMap::new();
    let mut seeds = BTreeSet::new();
    for (file_id, file) in context.files.iter().enumerate() {
        let lines = file.source.lines().count();
        let named = tokens
            .iter()
            .any(|token| file_matches(&file.relative, token))
            || joined
                .as_ref()
                .is_some_and(|joined| joined.is_target(&file.relative));
        if named {
            let selection = selected.entry(file_id).or_default();
            selection.ranges.push((1, lines, 0));
            selection
                .notes
                .insert("named file (includes non-function declarations)".into());
        }
        for (function_id, function) in file.functions.iter().enumerate() {
            if named || names.contains(function.name.as_str()) {
                seeds.insert((file_id, function_id));
                select_function(
                    &mut selected,
                    &context.files,
                    file_id,
                    function_id,
                    0,
                    format!(
                        "target {}:{} {}",
                        file.relative, function.line, function.name
                    ),
                );
                if tokens.iter().any(|token| {
                    token.contains("::")
                        && token.rsplit("::").next() == Some(function.name.as_str())
                }) {
                    selected.entry(file_id).or_default().notes.insert(
                        "qualified name: lexical candidates; module/type scope is not resolved"
                            .into(),
                    );
                }
            }
        }
    }
    let function_names: BTreeSet<_> = context
        .files
        .iter()
        .flat_map(|file| file.functions.iter().map(|function| function.name.as_str()))
        .collect();
    let declarations: Vec<_> = names
        .iter()
        .copied()
        .filter(|token| {
            !function_names.contains(*token) && !token.contains('/') && !token.ends_with(".rs")
        })
        .collect();
    let mut declaration_names = BTreeSet::new();
    for (file_id, file) in context.files.iter().enumerate() {
        if !declarations.iter().any(|name| file.source.contains(*name)) {
            continue;
        }
        for item in xirang_kernel::source::item_symbols(&file.source) {
            if !item.is_function && declarations.iter().any(|name| *name == item.name) {
                declaration_names.insert(item.name.clone());
                let selection = selected.entry(file_id).or_default();
                selection.ranges.push((1, file.source.lines().count(), 0));
                selection.notes.insert(format!(
                    "declaration {}:{} {}; enclosing file context",
                    file.relative, item.line, item.name
                ));
            }
        }
    }
    // A sentence may happen to contain a common symbol like `entries`; that must not suppress
    // the other symptom words. Only a fully resolved name list selects the exact-name route.
    // 一句话可能恰好含有 entries 这类常见符号；它不能屏蔽其它症状词。完整解析的名字列表才走精确路线。
    let exact_list = tokens.iter().all(|token| {
        let name = token.rsplit("::").next().unwrap_or(token);
        function_names.contains(name)
            || declaration_names.contains(name)
            || context
                .files
                .iter()
                .any(|file| file_matches(&file.relative, token))
    });
    if !exact_list
        && !tokens
            .iter()
            .any(|token| token.contains('/') || token.ends_with(".rs"))
    {
        selected.clear();
        seeds.clear();
        let terms = source_query::words(query);
        context.parse_matching(|file| source_query::mentions(&file.source, &terms));
        for row in source_query::rank(&context.files, query, &terms)
            .into_iter()
            .take(max_files)
        {
            seeds.insert((row.file_id, row.function_id));
            select_function(
                &mut selected,
                &context.files,
                row.file_id,
                row.function_id,
                0,
                format!(
                    "symptom candidate (text score {}): {}",
                    row.score,
                    row.reasons.join("; ")
                ),
            );
        }
    }
    // The same complete reverse relation query drives affected and exploration.
    // affected 与探索共用完整的反向关系查询；渲染预算不截断遍历。
    if !seeds.is_empty() {
        context.parse_matching(|_| true);
    }
    let mut sources = context.files;
    let mut definitions: BTreeMap<&str, Vec<(usize, usize)>> = BTreeMap::new();
    for (f, file) in sources.iter().enumerate() {
        for (i, function) in file.functions.iter().enumerate() {
            definitions.entry(&function.name).or_default().push((f, i));
        }
    }
    let names: Vec<_> = seeds
        .iter()
        .map(|&(f, i)| sources[f].functions[i].name.clone())
        .collect();
    for ((f, i), (via, depth)) in super::relations::Relations::new(&sources).reaching(&names) {
        let file = &sources[f];
        let function = &file.functions[i];
        let test = test_file(file);
        if test || depth == 1 {
            select_function(
                &mut selected,
                &sources,
                f,
                i,
                if test { 1 } else { 2 },
                format!(
                    "{} {}:{} {} -> {via}; via {via} -> {} ({depth} hops; lexical; not observed coverage)",
                    if test && depth == 1 {
                        "test-file caller"
                    } else if test {
                        "test-file candidate"
                    } else {
                        "caller"
                    },
                    file.relative,
                    function.line,
                    function.name,
                    names.join(", ")
                ),
            );
        }
    }
    // A receiver call (`bucket.contains(amount)`) writes no owner, so it is not a claimed edge and
    // the relation above cannot reach its body. The semantic reader still recorded it as unresolved;
    // selecting those bodies is what lets the reply carry the call site instead of denying one exists.
    // One hop further covers the caller of that call site (a test that calls the only consumer), and
    // it is labelled as a chain rather than folded into a route.
    // receiver 调用（`bucket.contains(amount)`）不写所属者，因此不是已声明的边，上面的关系够不到它的
    // 函数体。语义读取器仍把它记成未解析；把这些函数体选进来，回复才会带上调用点，而不是否认它存在。
    // 再走一跳覆盖"那个调用点的调用方"（调用唯一消费者的测试），并以链的形式标注，而不是混进路线表。
    let mut witnesses = super::relations::candidate_callers(&sources, &seeds, CANDIDATE_DEPTH);
    witnesses.sort_by_key(|witness| (witness.seed, witness.depth, witness.caller));
    witnesses.dedup_by(|left, right| left.seed == right.seed && left.caller == right.caller);
    let mut carried = 0usize;
    for witness in &witnesses {
        // A claimed direct caller is already selected by the relation above; this pass exists for
        // unclaimed sites and for chains.
        // 已声明的直接调用者上面已经选过；这一趟只为未声明的站点与链。
        if witness.claimed() && witness.depth == 1 {
            continue;
        }
        // Production stays one hop, the same rule the relation above follows: beyond the direct
        // caller only tests are carried. Selecting the production caller of a caller would turn one
        // symbol question into the whole call chain, and the test is the part the reader acts on.
        // 生产代码只走一跳，与上面那条关系同规则：越过直接调用者之后只携带测试。把"调用者的调用者"
        // 也选进来会把一次符号查询变成整条调用链，而读者真正据以行动的是那个测试。
        let (caller_file, caller_function) = witness.caller;
        let file = &sources[caller_file];
        if witness.depth > 1 && !test_file(file) {
            continue;
        }
        if carried >= MAX_UNRESOLVED_SITES {
            break;
        }
        carried += 1;
        let function = &file.functions[caller_function];
        let via = &sources[witness.via.0].functions[witness.via.1].name;
        let note = match (&witness.entry, witness.depth) {
            (Some(entry), 1) => format!(
                "unresolved receiver call site (candidate, not a claimed edge): {}:{} {} holds {}",
                file.relative, function.line, function.name, entry
            ),
            (Some(entry), depth) => format!(
                "candidate chain (depth {depth}; candidate, not coverage): {}:{} {} via {via} — {entry}",
                file.relative, function.line, function.name
            ),
            (None, depth) => format!(
                "candidate chain (depth {depth}; claimed by name, not coverage): {}:{} {} calls {via}",
                file.relative, function.line, function.name
            ),
        };
        select_function(
            &mut selected,
            &sources,
            caller_file,
            caller_function,
            if test_file(file) { 1 } else { 2 },
            note,
        );
    }
    for &(file_id, function_id) in &seeds {
        let file = &sources[file_id];
        let function = &file.functions[function_id];
        for call in &function.calls {
            let name = call.rsplit("::").next().unwrap_or(call);
            if let Some(targets) = definitions.get(name) {
                for &(target_file, target_function) in targets {
                    select_function(
                        &mut selected,
                        &sources,
                        target_file,
                        target_function,
                        2,
                        format!(
                            "callee candidate of {}:{} {} -> {call} ({} definitions)",
                            file.relative,
                            function.line,
                            function.name,
                            targets.len()
                        ),
                    );
                }
            }
        }
    }
    let mut structure_notice = String::new();
    if request.scopes.is_empty() && joined.is_none() {
        match structure::Joined::from_targets(root, &sources, &selected, max_files.max(3)) {
            Ok(answer) => joined = answer,
            Err(error) => {
                structure_notice = structure::section(
                    &format!("responsibility evidence unavailable: {error}\n"),
                    budget / 6,
                    "responsibility error",
                )
            }
        }
    } else if !request.scopes.is_empty() {
        structure_notice = "responsibility join not evaluated outside explicit scope; omit scope or use converge {node} for package obligations\n".into();
    }
    if let Some(joined) = &mut joined {
        joined.add_sources(root, &mut sources, &mut selected);
        let allocation = if budget < 4500 {
            budget / 6
        } else {
            (budget / 2).min(7000)
        };
        structure_notice = joined.report(allocation);
    }
    // The repair projection is built from the same parsed view the blocks above came from, after the
    // structural join has added the responsibility neighbours — a consumer named by the projection
    // must be a file this answer actually carries. It is budgeted like the other notices so it can
    // never crowd out the source it describes, and it is withheld entirely below `REPAIR_FLOOR`:
    // at a small `max_bytes` the target's own body is the evidence that matters, and the projection
    // is the section that can be recomputed by asking again with a larger budget.
    // 修复投影依据上面那些块所来自的同一个已解析视图构造，且在结构拼接补入责任邻居**之后**——投影点名的
    // 消费者必须是这个回答真的携带的文件。它与其他提示一样受预算约束，因此永远不会挤掉它描述的源码；而在
    // `REPAIR_FLOOR` 之下它整体不出：`max_bytes` 很小时，目标自己的正文才是要紧的证据，而投影是那个
    // "把预算调大再问一次"就能重算出来的节。
    let repair_notice = if budget < REPAIR_FLOOR {
        String::new()
    } else {
        let empty = BTreeMap::new();
        let faces = joined
            .as_ref()
            .map_or(&empty, structure::Joined::face_paths);
        let projected = repair::project(root, &sources, &seeds, faces);
        if projected.is_empty() {
            projected
        } else {
            structure::section(
                &projected,
                (budget / 6).clamp(1000, 2600),
                "repair evidence",
            )
        }
    };
    Ok(render::answer(
        &sources,
        selected,
        max_files,
        budget,
        &request,
        &structure_notice,
        &repair_notice,
    ))
}

/// The reply budget below which the repair projection is withheld so the target's own body keeps its
/// room. Measured against the small-budget pins: at `max_bytes` 2000 the reply must still carry
/// usable source, and the projection is what yields.
/// 低于这个回复预算时扣下修复投影，好让目标自己的正文保住空间。这是对着小预算钉子量出来的：`max_bytes`
/// 为 2000 时回复仍须携带可用源码，让路的正是投影。
const REPAIR_FLOOR: usize = 3500;

/// How many candidate call sites one query carries as source.
/// 一次查询最多把多少个候选调用点当作源码携带。
///
/// A candidate body is evidence, not a route: the cap keeps a common method name (`new`, `fmt`)
/// from turning one symbol query into a whole-tree read. Eight covers the shapes the repair fixtures
/// pin (same file, other file, test file, and a test one hop past a consumer) without competing with
/// the seed for the file budget.
/// 候选函数体是证据、不是路线：这个上限防止 `new`、`fmt` 这类常见方法名把一次符号查询变成整树读取。
/// 八条覆盖修复夹具钉住的形状（同文件、别的文件、测试文件、以及消费者再往后一跳的测试），又不会与
/// 种子争文件预算。
const MAX_UNRESOLVED_SITES: usize = 8;

/// How many hops past a seed the candidate traversal walks.
/// 候选遍历越过种子走多少跳。
///
/// Depth 2 is what makes "the test calls the only consumer" visible: depth 1 finds the consumer
/// (whose call site is the missing evidence), and the test that exercises it is one hop further.
/// Deeper than that stops being evidence for this query and starts being a graph dump.
/// 2 跳才让"测试调用了那个唯一消费者"可见：1 跳找到消费者（缺的证据正是它的调用点），而调用它的
/// 测试再往后一跳。再深就不再是这次查询的证据，而是把图倒出来。
const CANDIDATE_DEPTH: usize = 2;

/// Whether a file is a test file: the shared test-shape read, plus cargo's integration directory.
/// 文件是否为测试文件：共用测试形状读取，加上 cargo 的集成测试目录。
pub(super) fn test_file(file: &SourceFile) -> bool {
    super::callgraph::looks_like_a_test(&file.relative, &file.source)
        || file.relative.starts_with("tests/")
}

fn bounded(
    args: &Value,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, String> {
    match args.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|n| (*n >= min as u64) && (*n <= max as u64))
            .map(|n| n as usize)
            .ok_or_else(|| format!("{key} must be an integer in {min}..{max}")),
    }
}

fn file_matches(path: &str, token: &str) -> bool {
    let token = token.strip_prefix("./").unwrap_or(token);
    // Basenames are convenient; a path with directories is exact within the configured root.
    // 文件名允许简写，带目录的路径则相对配置根精确匹配，不猜测根外路径。
    path == token || (!token.contains('/') && path.rsplit('/').next() == Some(token))
}

fn select_function(
    selected: &mut BTreeMap<usize, Selection>,
    sources: &[SourceFile],
    file_id: usize,
    function_id: usize,
    rank: u8,
    note: String,
) {
    let file = &sources[file_id];
    let function = &file.functions[function_id];
    let selection = selected.entry(file_id).or_insert_with(|| Selection {
        rank,
        ..Selection::default()
    });
    selection.rank = selection.rank.min(rank);
    // The renderer expands docs/attributes and merges overlapping ranges once per file.
    // 渲染器按文件展开文档与属性并合并重叠范围。
    selection
        .ranges
        .push((function.line, function.end_line, rank));
    selection.notes.insert(note);
}

#[cfg(test)]
#[path = "explore_tests.rs"]
mod tests;
