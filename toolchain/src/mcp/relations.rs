//! One reverse lexical relation query for exploration and verification planning.
//! 探索与验证计划共用的反向词法关系查询。
use super::source_index::SourceFile;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// How many independent consumer files a provider needs before it counts as **shared**.
/// 一个提供者要有多少个独立消费者文件才算**共享**。
///
/// One home for the number, because two readers ask the same question from opposite ends: the repair
/// projection reports whether a function is a shared root cause, and the plan turns an uncovered
/// consumer of a shared provider into a duty. If either used its own threshold, a function could be
/// "shared" in one answer and "not shared" in the other — and the plan is the one that would then
/// stay silent about a missing protection.
/// 这个数字只有一个家，因为两个读者从相反的两端问同一个问题：修复投影报告某个函数是不是共享根因，而计划把
/// 共享提供者身上未被覆盖的消费者变成一条义务。若两边各用各的阈值，同一个函数可能在一边答"共享"、在另一边
/// 答"不共享"——而保持沉默、对缺失的保护一言不发的正好会是计划。
pub(super) const SHARED_FROM: usize = 2;

pub(super) struct Relations<'a> {
    sources: &'a [SourceFile],
    callers: BTreeMap<&'a str, BTreeSet<(usize, usize)>>,
    targets: BTreeMap<&'a str, Vec<&'a super::source_index::Function>>,
}
impl<'a> Relations<'a> {
    pub(super) fn new(sources: &'a [SourceFile]) -> Self {
        let mut callers: BTreeMap<&str, BTreeSet<(usize, usize)>> = BTreeMap::new();
        let mut targets: BTreeMap<&str, Vec<&super::source_index::Function>> = BTreeMap::new();
        for (f, file) in sources.iter().enumerate() {
            for (i, function) in file.functions.iter().enumerate() {
                targets
                    .entry(&function.qualified)
                    .or_default()
                    .push(function);
                for call in &function.calls {
                    callers.entry(call).or_default().insert((f, i));
                    // A module-qualified free function (`provider::rate`) still has a bare
                    // definition in another module; the target check below decides whether that
                    // bare spelling really names it.
                    // 模块限定的自由函数（`provider::rate`）在另一个模块里仍有一个裸定义；下面的目标
                    // 检查决定那个裸拼写是否真的点名它。
                    if let Some(bare) = call.rsplit("::").next()
                        && bare != call
                    {
                        callers.entry(bare).or_default().insert((f, i));
                    }
                }
            }
        }
        Self {
            sources,
            callers,
            targets,
        }
    }
    /// Return each reachable caller once, with the edge that discovered it and its distance.
    /// 每个可达调用方只返回一次，附带发现它的边与距离。
    pub(super) fn reaching(&self, names: &[String]) -> BTreeMap<(usize, usize), (String, usize)> {
        let mut queue: VecDeque<_> = names.iter().map(|name| (name.as_str(), 1)).collect();
        let mut visited = BTreeSet::new();
        let mut reached = BTreeMap::new();
        while let Some((name, depth)) = queue.pop_front() {
            if !visited.insert(name) {
                continue;
            }
            let definitions = self.targets.get(name).cloned().unwrap_or_default();
            for &(f, i) in self.callers.get(name).into_iter().flatten() {
                let function = &self.sources[f].functions[i];
                let matched = function.calls.iter().any(|call| {
                    definitions
                        .iter()
                        .any(|definition| super::callgraph::call_matches(definition, call))
                });
                if !matched {
                    continue;
                }
                reached
                    .entry((f, i))
                    .or_insert_with(|| (name.to_owned(), depth));
                queue.push_back((&function.qualified, depth + 1));
            }
        }
        reached
    }
    pub(super) fn tests_reaching(&self, names: &[String], touched: &SourceFile) -> Vec<String> {
        let mut tests = BTreeSet::new();
        if super::callgraph::looks_like_a_test(&touched.relative, &touched.source) {
            tests.insert(touched.relative.clone());
        }
        for ((f, _), _) in self.reaching(names) {
            let file = &self.sources[f];
            if super::callgraph::looks_like_a_test(&file.relative, &file.source) {
                tests.insert(file.relative.clone());
            }
        }
        tests.into_iter().collect()
    }
}

/// One caller a seed may reach, the edge that reached it, and how strong that edge is.
/// 种子可能到达的一个调用方、发现它的那条边，以及那条边的强弱。
pub(super) struct Witness {
    /// The function whose body holds the call (or, for a claimed edge, the call itself).
    /// 调用所在的函数（对已声明的边而言，就是发出调用的那个函数）。
    pub(super) caller: (usize, usize),
    /// The seed the chain starts from.
    /// 这条链起始的种子。
    pub(super) seed: (usize, usize),
    /// The function through which this caller reaches the seed (`== seed` at depth 1).
    /// 这个调用方经由它到达种子（深度 1 时 `== seed`）。
    pub(super) via: (usize, usize),
    /// Why the edge is only a candidate, verbatim; `None` means the call matched a definition.
    /// 这条边为什么只是候选的原话；`None` 表示调用真的匹配到了某个定义。
    pub(super) entry: Option<String>,
    /// How many hops from the seed: 1 is a direct caller, 2 a caller of a caller.
    /// 距种子的跳数：1 是直接调用者，2 是调用者的调用者。
    pub(super) depth: usize,
}

impl Witness {
    /// Whether the syntactic call matched a definition (still not coverage, but not a guess).
    /// 这条调用的拼写是否真的匹配到某个定义（仍不是覆盖，但也不是猜的）。
    pub(super) fn claimed(&self) -> bool {
        self.entry.is_none()
    }
}

/// One traversal frontier entry: the target whose callers are being read, the seed the chain
/// started from, and the depth already walked.
/// 遍历前沿的一项：正在读取其调用方的目标、这条链起始的种子、以及已经走过的跳数。
type FrontierEntry = ((usize, usize), (usize, usize), usize);

/// Callers of `seeds`, including edges the semantic reader refused to claim.
/// `seeds` 的调用方，包括语义读取器拒绝声明为边的那些。
///
/// A receiver call (`bucket.contains(amount)`) never writes its owner, so the reader records it as
/// unresolved — with the owner when it could prove one, with the candidate list when it could not.
/// A call written inside a macro (`assert_eq!(store.post(..))`) is a bare token to `syn`, so it
/// arrives as a bare spelling; when exactly one definition in the tree bears that name, the spelling
/// is a candidate for it — never a claimed edge. The traversal goes one hop past the direct caller so
/// that a test which calls the shared function's only caller is still visible; coverage is never
/// claimed.
///
/// receiver 调用（`bucket.contains(amount)`）从不写所属者，因此读取器把它记成未解析——能证明所属者时
/// 连所属者一起，不能时连候选清单一起。写在宏里的调用（`assert_eq!(store.post(..))`）对 `syn` 只是
/// 裸 token，因此以裸拼写到达；当树里恰好只有一个定义叫这个名字时，这个拼写就是它的候选——永远不是
/// 已声明的边。遍历会越过直接调用者再走一跳，好让"测试调用了那个唯一调用者"这件事仍然可见；覆盖从不
/// 被声称。
pub(super) fn candidate_callers(
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    max_depth: usize,
) -> Vec<Witness> {
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for file in sources {
        for function in &file.functions {
            *names.entry(function.name.as_str()).or_default() += 1;
        }
    }
    let mut out: Vec<Witness> = Vec::new();
    let mut seen: BTreeSet<(usize, usize)> = seeds.clone();
    let mut frontier: VecDeque<FrontierEntry> =
        seeds.iter().map(|&seed| (seed, seed, 0usize)).collect();
    while let Some((target, seed, depth)) = frontier.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let definition = &sources[target.0].functions[target.1];
        for (file_id, file) in sources.iter().enumerate() {
            for (function_id, function) in file.functions.iter().enumerate() {
                let caller = (file_id, function_id);
                if caller == target || seen.contains(&caller) {
                    continue;
                }
                let claimed = function
                    .calls
                    .iter()
                    .any(|call| super::callgraph::call_matches(definition, call));
                let entry = if claimed {
                    None
                } else {
                    function
                        .unresolved
                        .iter()
                        .find(|entry| names_the_seed(entry, &definition.qualified, &definition.name))
                        .cloned()
                        .or_else(|| {
                            // A bare spelling is only a candidate for the **only** definition with
                            // that name, and only when the caller demonstrably touches that
                            // definition's file: `text.contains("…")` in a test must not become
                            // `Bucket::contains` just because the tree has one `contains`.
                            // 裸拼写只有在"树里只有这一个同名定义"、且调用方确实碰到那个定义所在文件时才
                            // 算候选：测试里的 `text.contains("…")` 不能因为树里只有一个 `contains`
                            // 就被算成 `Bucket::contains`。
                            let file_reached = touches_file(sources, target.0, function);
                            function.calls.iter().find_map(|call| {
                                (!call.contains("::")
                                    && call == &definition.name
                                    && file_reached
                                    && names.get(call.as_str()).copied() == Some(1))
                                    .then(|| {
                                        format!(
                                            "bare call `{call}` (the only definition with that name is {}; candidate, not a claimed edge)",
                                            definition.qualified
                                        )
                                    })
                            })
                        })
                };
                if !claimed && entry.is_none() {
                    continue;
                }
                seen.insert(caller);
                out.push(Witness {
                    caller,
                    seed,
                    via: target,
                    entry,
                    depth: depth + 1,
                });
                frontier.push_back((caller, seed, depth + 1));
            }
        }
    }
    out
}

/// Whether a caller's own body touches the file that defines the target.
/// 调用方自己的函数体是否碰到目标定义所在的文件。
///
/// Keeps a bare, macro-derived spelling from being attributed on name uniqueness alone: a test that
/// calls `text.contains(..)` and never names anything defined in `store/index.rs` is not a candidate
/// caller of `Bucket::contains`.
/// 用来防止只凭"名字唯一"就归属一个宏里的裸拼写：只调用 `text.contains(..)`、从不点名
/// `store/index.rs` 里任何东西的测试，不是 `Bucket::contains` 的候选调用方。
fn touches_file(
    sources: &[SourceFile],
    target_file: usize,
    caller: &super::source_index::Function,
) -> bool {
    let file = &sources[target_file];
    caller.calls.iter().any(|call| {
        file.functions
            .iter()
            .any(|function| super::callgraph::call_matches(function, call))
    }) || caller.unresolved.iter().any(|entry| {
        file.functions
            .iter()
            .any(|function| names_the_seed(entry, &function.qualified, &function.name))
    })
}

/// Whether one unresolved entry names a seed, outright or through its candidate list.
/// 一条未解析条目是否点名某个种子：直接写出来，或写在候选清单里。
fn names_the_seed(entry: &str, qualified: &str, name: &str) -> bool {
    let (spelling, reason) = entry.split_once(" (").unwrap_or((entry, ""));
    if spelling == qualified {
        return true;
    }
    spelling == name
        && reason.split_once("candidates: ").is_some_and(|(_, list)| {
            // The reason string ends with the entry's own closing parenthesis, so the last
            // candidate carries a `)` that is punctuation rather than part of the name.
            // 理由串以条目自己的右括号结尾，因此最后一个候选带着一个属于标点、不属于名字的 `)`。
            list.trim_end_matches(')')
                .split(", ")
                .any(|candidate| candidate == qualified)
        })
}
