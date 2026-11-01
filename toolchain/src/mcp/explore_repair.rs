//! The repair projection: what one read-only query must lead with when something is broken.
//! 修复投影：出问题时，一次只读查询必须先给出的东西。
//!
//! `explore` already returns current source and lexical relations. What a repair needs on top of
//! those is the *shape* of the evidence: which function is the shared implementation, which files
//! consume it, how each one is reached from outside the crate, which tests already protect it, and
//! what is still unknown. Every line below comes from the same parsed view the source blocks come
//! from — no second scan and no Cargo invocation on the common path.
//! `explore` 已经返回当前源码与词法关系。修复工作在其之上还需要证据的**形状**：哪个函数是共享实现、
//! 哪些文件消费它、每一个如何从 crate 之外到达、哪些测试已经保护它、以及什么仍然未知。下面每一行都
//! 来自源码块所依据的**同一个已解析视图**——没有第二次扫描，常见路径上也没有 Cargo 调用。
//!
//! Three rules keep this honest rather than helpful-sounding. **A route is derived, never asserted**:
//! the Rust path this module prints comes from the file layout, and the line that would confirm it
//! is quoted when it was found and named as missing when it was not. **A lexical edge is not
//! coverage**: every relation line says so, and a test that never ran is not protection. **Unknown
//! stays unknown**: macro and trait-object lines are counted and reported as dispatch the lexical
//! graph does not resolve, instead of being silently dropped.
//! 三条规则让它诚实而不是"听起来有用"：**路线是推导出来的，不是断言的**——它印出的 Rust 路径来自文件
//! 布局，能确认它的那一行找到了就引用、没找到就说明没找到；**词法边不是覆盖**——每一条关系行都写明这
//! 一点，而从未运行过的测试不是保护；**未知就保持未知**——宏与 trait object 行被计数并报告为词法图解析
//! 不了的派发，而不是被静默丢掉。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::SourceFile;
use crate::mcp::relations::{Relations, SHARED_FROM, Witness, candidate_callers};

/// How many delegation-chain rows the projection prints before it summarises the rest as a count.
/// 投影在把其余部分摘要成一个计数之前，最多印多少条委派链行。
const MAX_CHAINS: usize = 6;

/// One lexical call edge, as the shared reverse-relation query reports it.
/// 一条词法调用边，形状与共用的反向关系查询所报告的一致。
struct Edge {
    /// The file the calling function lives in.
    /// 调用函数所在的文件。
    file: String,
    /// The calling function's first source line, trimmed for one-line display.
    /// 调用函数的第一行源码，去空白后单行显示。
    text: String,
    /// The calling function's own name, which is what a test above it would call.
    /// 调用函数自己的名字——它上面的测试调用的正是这个名字。
    function: String,
    /// The name the call resolved to; for a consumer edge this is the provider.
    /// 这次调用解析到的名字；对消费者边而言就是提供者。
    provider: String,
    /// How many lexical hops the caller sits from the selected implementation.
    /// 这个调用者距选中的实现有多少词法跳。
    depth: usize,
    /// The caller's own first line, so a chain row can be located without re-reading the file.
    /// 调用者自己的首行行号，使一条链行无需重读文件就能被定位。
    line: usize,
}

/// One package's contribution to the projection: where its library root is and what it is called.
/// 一个包对这份投影的贡献：它的库根在哪里、叫什么。
struct Package {
    manifest: PathBuf,
    name: Option<String>,
    lib: Option<PathBuf>,
    lib_relative: Option<String>,
}

/// Build the repair projection for the functions one query selected.
/// 为一次查询选中的函数构造修复投影。
///
/// `faces` maps a source path to the logical path of the registered face declared there, so a host
/// whose modules are mounted by a build-time generated tree still gets a public route.
/// `faces` 把源码路径映射到该处声明的注册面的逻辑路径，因此模块由构建期生成树挂载的宿主同样能得到一条
/// 公开路线。
pub(super) fn project(
    root: &Path,
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    faces: &BTreeMap<String, String>,
) -> String {
    let names: Vec<String> = seeds
        .iter()
        .map(|&(file, function)| sources[file].functions[function].name.clone())
        .collect();
    if names.is_empty() {
        return String::new();
    }
    let seed_files: BTreeSet<&str> = seeds
        .iter()
        .map(|&(file, _)| sources[file].relative.as_str())
        .collect();
    let relations = Relations::new(sources);
    let reached = relations.reaching(&names);
    let witnesses = candidate_callers(sources, seeds, 2);
    let mut consumers: Vec<Edge> = Vec::new();
    let mut chains: Vec<Edge> = Vec::new();
    let mut tests: Vec<Edge> = Vec::new();
    for ((file_id, function_id), (via, depth)) in &reached {
        let file = &sources[*file_id];
        let function = &file.functions[*function_id];
        let edge = Edge {
            file: file.relative.clone(),
            text: line_of(&file.source, function.line),
            function: function.name.clone(),
            provider: via.clone(),
            depth: *depth,
            line: function.line,
        };
        if super::test_file(file) {
            tests.push(edge);
            continue;
        }
        // Only a direct caller is a consumer entry: a caller three hops away reached the provider
        // through something else, and counting it would overstate the fan-in.
        // 只有直接调用者才是消费者入口：三跳之外的调用者是经别的东西到达提供者的，把它算进来会夸大盘扇入。
        if !seed_files.contains(file.relative.as_str()) {
            if *depth == 1 {
                consumers.push(edge);
            } else if chains.len() < MAX_CHAINS {
                // A delegation chain is summarised as a path rather than dropped: "this caller sits
                // three hops from the implementation" is what tells a reader whether the chain is
                // part of the repair's blast radius, and a route list that stopped at one hop would
                // read as if nothing were behind it.
                // 委派链被摘要成一条路径而不是被丢掉："这个调用者离实现三跳"正是告诉读者这条链在不在修复的
                // 影响半径里的东西，而一条只到一跳的路线表会读成它后面什么都没有。
                chains.push(edge);
            }
        }
    }
    let shared: BTreeSet<String> = names
        .iter()
        .filter(|name| {
            consumers
                .iter()
                .map(|edge| (edge.provider.as_str(), edge.file.as_str()))
                .filter(|(provider, _)| provider == &name.as_str())
                .map(|(_, file)| file)
                .collect::<BTreeSet<_>>()
                .len()
                >= SHARED_FROM
        })
        .cloned()
        .collect();
    let mut output = String::from("repair evidence\n");
    output.push_str(&focus(
        sources, seeds, &consumers, &shared, faces, &witnesses,
    ));
    output.push_str(&routes(&consumers, &chains, &tests));
    output.push_str(&access(root, sources, seeds, &consumers, faces));
    output.push_str(&protection(&consumers, &tests, sources, seeds, &witnesses));
    output.push_str(&references(root, sources, seeds, faces));
    output.push_str(&coverage(root, sources, seeds));
    output.push_str(&next_steps(sources, seeds, faces));
    output
}

/// The raw first source line of a function, trimmed for one-line display.
/// 某个函数的第一行源码，去空白后单行显示。
fn line_of(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .map(str::trim)
        .unwrap_or_default()
        .to_owned()
}

/// Which selected function is the shared implementation, and how many files depend on it.
/// 选中的哪个函数是共享实现，以及有多少文件依赖它。
///
/// A single consumer is reported too, and explicitly as *not* shared: "one file calls this" is the
/// evidence that decides whether the fix belongs upstream or at the call site, and withholding it
/// would leave the reader to guess.
/// 只有一个消费者时同样报告，并明确写成**不是**共享：决定修复该落在上游还是调用点的正是"只有一个文件
/// 调用它"这条证据，扣下它等于让读者去猜。
fn focus(
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    consumers: &[Edge],
    shared: &BTreeSet<String>,
    faces: &BTreeMap<String, String>,
    witnesses: &[Witness],
) -> String {
    let mut output = String::new();
    for &(file_id, function_id) in seeds {
        let file = &sources[file_id];
        let function = &file.functions[function_id];
        let who: BTreeSet<&str> = consumers
            .iter()
            .filter(|edge| edge.provider == function.name)
            .map(|edge| edge.file.as_str())
            .collect();
        let sites: Vec<_> = witnesses
            .iter()
            .filter(|witness| {
                witness.seed == (file_id, function_id)
                    && witness.depth == 1
                    && witness.entry.is_some()
            })
            .collect();
        let chains: Vec<_> = witnesses
            .iter()
            .filter(|witness| witness.seed == (file_id, function_id) && witness.depth > 1)
            .take(4)
            .collect();
        let verdict = if shared.contains(&function.name) {
            format!(
                " — shared implementation; {} independent consumer files: {}",
                who.len(),
                who.iter().copied().collect::<Vec<_>>().join(", ")
            )
        } else if who.len() == 1 {
            format!(
                " — one consumer file only ({}), so not shared",
                who.iter().next().unwrap_or(&"?")
            )
        } else if let Some(face) = faces.get(&file.relative) {
            // A registered face is reached by the framework through its declaration, so a missing
            // lexical caller is *expected* here — reporting it as "possibly dead" would send the
            // reader looking for a caller that was never supposed to exist.
            // 注册面是被框架**经它的声明**到达的，因此这里没有词法调用者是**应有之义**——把它报成
            // "可能是死代码"会让读者去找一个本来就不该存在的调用者。
            format!(
                " — no direct call in this tree, but this file declares the registered face {face}, whose methods are reached through its registration contract"
            )
        } else if sites.is_empty() {
            // The old sentence said "no lexical caller in this tree", which reads as a fact about the
            // tree while it is only a fact about *claimed edges*. A receiver call is a call; when one
            // names this function the fact belongs in the reply, and when none does the sentence says
            // exactly what was checked.
            // 旧句子写的是"这棵树里没有词法调用者"，读起来像关于树的事实，实际只是关于**已声明边**的事实。
            // receiver 调用也是调用；有点名这个函数的时候那个事实必须写进回复，没有的时候句子要写清查了什么。
            " — no caller in this tree: no claimed edge names it, and no unresolved call site does either; callers outside this tree are not examined".to_owned()
        } else {
            format!(
                " — no claimed caller; {} unresolved receiver call site(s) may reach it (candidates, not edges)",
                sites.len()
            )
        };
        output.push_str(&format!(
            "focus: {}:{} {}{}\n",
            file.relative, function.line, function.name, verdict
        ));
        for witness in &sites {
            let (caller_file, caller_function) = witness.caller;
            let caller = &sources[caller_file];
            let site = &caller.functions[caller_function];
            output.push_str(&format!(
                "  unresolved call site: {}:{} {} — {} (candidate, not a claimed edge)\n",
                caller.relative,
                site.line,
                site.name,
                witness.entry.as_deref().unwrap_or_default()
            ));
        }
        for witness in &chains {
            let (caller_file, caller_function) = witness.caller;
            let caller = &sources[caller_file];
            let site = &caller.functions[caller_function];
            let via = &sources[witness.via.0].functions[witness.via.1].name;
            output.push_str(&format!(
                "  candidate chain (depth {}): {}:{} {} via {via} — {} (candidates, not coverage)\n",
                witness.depth,
                caller.relative,
                site.line,
                site.name,
                witness.entry.as_deref().unwrap_or("claimed call")
            ));
        }
        output.push_str(&format!(
            "  contract {}\n",
            contract_of(&file.source, function.line as u32).unwrap_or_else(|| {
                "none above the declaration; read what the callers expect of it".into()
            })
        ));
    }
    output
}

/// The doc line a function's contract is written on, when it has one.
/// 函数合同所在的那一行文档；没有就回答没有。
fn contract_of(source: &str, line: u32) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut cursor = line as usize;
    let mut contract = None;
    while cursor > 1 {
        let previous = lines.get(cursor - 2)?.trim();
        if let Some(text) = previous.strip_prefix("///") {
            let text = text.trim();
            if !text.is_empty() && !text.starts_with("```") {
                contract = Some(text.to_owned());
            }
            cursor -= 1;
            continue;
        }
        // Attributes sit between the doc and the declaration; a contract is not cancelled by them.
        // 属性位于文档与声明之间；它们不会取消一份合同。
        if previous.starts_with("#[") || previous.starts_with("#![") || previous.is_empty() {
            cursor -= 1;
            continue;
        }
        break;
    }
    contract.map(|text| shorten(&text, 160))
}

/// Provider-to-consumer edges, each labelled with the evidence class that produced it.
/// 提供者到消费者的边，每条都标注产出它的证据类别。
fn routes(consumers: &[Edge], chains: &[Edge], tests: &[Edge]) -> String {
    if consumers.is_empty() && chains.is_empty() && tests.is_empty() {
        return String::new();
    }
    let mut output =
        String::from("routes (static lexical call sites; not resolved dispatch or coverage):\n");
    let mut seen = BTreeSet::new();
    for edge in consumers {
        let row = format!("  consumer {} -> {}\n", edge.text, edge.provider);
        if seen.insert(row.clone()) {
            output.push_str(&row);
        }
    }
    for edge in tests {
        let row = format!("  test     {} -> {}\n", edge.text, edge.provider);
        if seen.insert(row.clone()) {
            output.push_str(&row);
        }
    }
    for edge in chains {
        let row = format!(
            "  chain    {}:{} {} -> {} ({} lexical hop(s); further delegation is a path, not a consumer entry)\n",
            edge.file, edge.line, edge.function, edge.provider, edge.depth
        );
        if seen.insert(row.clone()) {
            output.push_str(&row);
        }
    }
    output
}

/// How each relevant file is reached from outside its own crate, and on what basis.
/// 每个相关文件如何从它自己的 crate 之外到达，依据是什么。
fn access(
    root: &Path,
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    consumers: &[Edge],
    faces: &BTreeMap<String, String>,
) -> String {
    // Seed files first — the target contract and its implementation outrank its callers — then the
    // consumer entry files the routes named.
    // 目标文件在前——目标合同与它的实现优先于调用者——随后是路线点名的消费者入口文件。
    let mut wanted: Vec<&str> = seeds
        .iter()
        .map(|&(file, _)| sources[file].relative.as_str())
        .collect();
    for edge in consumers {
        if !wanted.contains(&edge.file.as_str()) {
            wanted.push(&edge.file);
        }
    }
    let mut output = String::new();
    let mut packages: BTreeMap<PathBuf, Package> = BTreeMap::new();
    let mut announced: BTreeSet<PathBuf> = BTreeSet::new();
    for file in wanted {
        let Some(package_root) = enclosing_package(root, file) else {
            output.push_str(&format!(
                "public_access: {file} is outside any package this root holds; its public route was not examined\n"
            ));
            continue;
        };
        let package = packages
            .entry(package_root.clone())
            .or_insert_with(|| Package::of(&package_root));
        if announced.insert(package_root.clone()) {
            output.push_str(&format!(
                "public_access: package {}; lib {} ({})\n",
                package.name.as_deref().unwrap_or(
                    "unknown — the manifest names no [package] name; read it before quoting a path"
                ),
                package.lib_relative.as_deref().unwrap_or("none"),
                package.manifest.display()
            ));
        }
        let Some(lib) = &package.lib else {
            output.push_str(&format!(
                "  {file}: no library root in this package, so no dependent crate can reach it; an integration test or binary is the only caller\n"
            ));
            continue;
        };
        let derived = module_path(lib, &root.join(file));
        let quoting = if derived.is_empty() {
            "crate".to_owned()
        } else {
            format!("crate::{derived}")
        };
        output.push_str(&format!(
            "  {file}: {quoting} [{}]\n",
            module_basis(lib, &derived)
        ));
        if let Some(face) = faces.get(file) {
            output.push_str(&format!(
                "  {file}: also the registered face {face}; its module mounting lives in the build-time generated library, not in the hand-written tree\n"
            ));
        }
    }
    output
}

impl Package {
    /// Resolve one package's name and library root from the tree, without asking Cargo.
    /// 不询问 Cargo 地读出一个包的包名与库根。
    ///
    /// `explore` is a read-only path the agent calls first and often; spawning `cargo metadata` here
    /// would let a concurrent build's lock turn an evidence query into a stall. The two facts this
    /// needs — `[package] name` and the declared library target — are read from the manifest and the
    /// tree, and a manifest this narrow read cannot answer stays `None`, so the reply says it does not
    /// know instead of guessing a package name.
    /// `explore` 是代理最先、最常调用的只读路径；在这里 spawn `cargo metadata` 会让一次并发构建的锁把
    /// 证据查询变成停顿。它需要的两件事——`[package] name` 与声明的库目标——从清单与树里读出，而这次窄读
    /// 答不出的清单保持 `None`，于是回复说它不知道，而不是猜一个包名。
    fn of(package_root: &Path) -> Self {
        let manifest = package_root.join("Cargo.toml");
        let name = std::fs::read_to_string(&manifest)
            .ok()
            .and_then(|text| package_name(&text));
        let layout = crate::build_method::source_layout(package_root).ok();
        let lib = layout
            .as_ref()
            .and_then(|layout| layout.target.clone())
            .or_else(|| {
                let default = package_root.join("src/lib.rs");
                default.is_file().then_some(default)
            });
        let lib_relative = lib.as_ref().and_then(|lib| relative(package_root, lib));
        Self {
            manifest,
            name,
            lib,
            lib_relative,
        }
    }
}

/// The package directory that holds one file, walking no further up than the queried root.
/// 容纳某个文件的包目录；向上不超过被查询的根。
fn enclosing_package(root: &Path, file: &str) -> Option<PathBuf> {
    let mut directory = root.join(file).parent()?.to_path_buf();
    loop {
        if directory.join("Cargo.toml").is_file() {
            return Some(directory);
        }
        if !directory.pop() || !directory.starts_with(root) {
            return None;
        }
    }
}

/// The path of a file relative to one directory, as a forward-slash string.
/// 某个文件相对某个目录的路径，写成正斜杠字符串。
fn relative(base: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(base)
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}

/// The candidate Rust module path of a file inside its own package's library.
/// 某个文件在它自己包的库中的候选 Rust 模块路径。
///
/// The derivation is the layout convention this workspace uses and plain Cargo accepts:
/// `<dir>/<name>/<name>.rs` is the module `<name>` under `<dir>`. When a package mounts a module
/// some other way — an explicit `#[path]`, or a generated tree — this is a candidate, which is why
/// [`module_basis`] quotes the declaration that confirms it or says none was found.
/// 推导用的是本工作区的布局约定，也是普通 Cargo 接受的形式：`<dir>/<name>/<name>.rs` 是 `<dir>` 之下
/// 名为 `<name>` 的模块。当一个包用别的方式挂载模块——显式 `#[path]`，或生成的树——这就只是一个**候选**，
/// 因此 [`module_basis`] 会引用确认它的那条声明，或说明一条也没找到。
fn module_path(lib: &Path, file: &Path) -> String {
    let Some(directory) = lib.parent() else {
        return String::new();
    };
    let Ok(rest) = file.strip_prefix(directory) else {
        return String::new();
    };
    let mut parts: Vec<String> = rest
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let Some(last) = parts.pop() else {
        return String::new();
    };
    let stem = last.strip_suffix(".rs").unwrap_or(&last).to_owned();
    if stem == "lib" || stem == "main" {
        return parts.join("::");
    }
    // `<name>/<name>.rs` names the module once, not twice.
    // `<name>/<name>.rs` 只命名模块一次，而不是两次。
    if parts.last() != Some(&stem) {
        parts.push(stem);
    }
    parts.join("::")
}

/// The declaration that confirms a derived module path, or the fact that none was found.
/// 确认推导出的模块路径的那条声明；一条也没找到时就说明这件事。
fn module_basis(lib: &Path, derived: &str) -> String {
    let Some(first) = derived.split("::").next().filter(|name| !name.is_empty()) else {
        return "the library root itself".to_owned();
    };
    let Ok(text) = std::fs::read_to_string(lib) else {
        return format!(
            "{} unreadable, so the declaration was not checked",
            lib.display()
        );
    };
    let masked = xirang_kernel::source::mask_non_code(&text);
    for (index, (masked_line, raw_line)) in masked.lines().zip(text.lines()).enumerate() {
        if !masked_line.contains(&format!("mod {first}")) {
            continue;
        }
        let raw: Vec<&str> = text.lines().collect();
        let mut attributes = Vec::new();
        let mut cursor = index;
        while cursor > 0 {
            let previous = raw[cursor - 1].trim();
            if previous.starts_with("#[") {
                attributes.push(previous.to_owned());
                cursor -= 1;
            } else {
                break;
            }
        }
        attributes.reverse();
        return format!(
            "declared at {}:{} `{}`{}{}",
            lib.display(),
            index + 1,
            raw_line.trim(),
            if attributes.is_empty() { "" } else { "; cfg " },
            attributes.join(" ")
        );
    }
    format!(
        "layout-derived only: {} declares no `mod {first}` for it; read that file (or the generated library a host!() pulls in) before quoting the path",
        lib.display()
    )
}

/// Which consumers have protection, which need it, and what the lexical graph cannot see.
/// 哪些消费者有保护、哪些还缺、以及词法图看不见什么。
///
/// A consumer is protected only when a test calls **it** by name. A test that calls the shared
/// implementation directly protects the implementation, not the consumer's own delegation, and the
/// two are reported on separate lines because conflating them is how a missing consumer test reads
/// as covered.
/// 只有当测试按名字调用**消费者本身**时，该消费者才算被保护。直接调用共享实现的测试保护的是实现，而不是
/// 消费者自己的委派；两者分行报告，因为把二者混为一谈正是"消费者测试缺失却读作已覆盖"的来路。
fn protection(
    consumers: &[Edge],
    tests: &[Edge],
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    witnesses: &[Witness],
) -> String {
    let mut callers: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in tests {
        callers
            .entry(edge.provider.as_str())
            .or_default()
            .insert(edge.file.as_str());
    }
    let mut output = String::from("obligations:\n");
    if tests.is_empty() {
        // "Nothing in this tree tests it" is a claim about the tree; an unresolved receiver call site
        // may be exactly the test path that the lexical relation cannot claim. Say what was checked
        // instead of asserting absence, and name the candidate when the chain reaches a test file.
        // "树里没有测试覆盖它"是一句关于树的断言；未解析的 receiver 调用点可能正是词法关系声明不了的
        // 测试路径。写清查了什么，而不是断言不存在；链到达测试文件时把候选点出来。
        let candidates: Vec<_> = witnesses
            .iter()
            .filter(|witness| super::test_file(&sources[witness.caller.0]))
            .collect();
        if let Some(first) = candidates.first() {
            let (file_id, function_id) = first.caller;
            let file = &sources[file_id];
            let function = &file.functions[function_id];
            output.push_str(&format!(
                "  needs evidence: no test reaches the selected implementation by claimed edge; a candidate chain reaches {}:{} {} (depth {}; candidates, not coverage), and execution is not observed here\n",
                file.relative, function.line, function.name, first.depth
            ));
        } else if witnesses.is_empty() {
            output.push_str(
                "  needs evidence: nothing in this tree tests the selected implementation; add consumer-level protection with the fix\n",
            );
        } else {
            output.push_str(&format!(
                "  needs evidence: no test reaches the selected implementation by claimed edge; {} unresolved receiver call site(s) may still exercise it (see repair evidence), and execution is not observed here\n",
                witnesses.len()
            ));
        }
    }
    let mut reported: BTreeSet<(&str, &str)> = BTreeSet::new();
    for (file_id, function_id) in seeds {
        let name = sources[*file_id].functions[*function_id].name.as_str();
        for file in callers.get(name).into_iter().flatten() {
            if reported.insert((name, file)) {
                output.push_str(&format!(
                    "  protected (lexically) {name} <- {file}: a test file reaches it; execution and assertions are not observed here\n"
                ));
            }
        }
    }
    for edge in consumers {
        match callers.get(edge.function.as_str()) {
            Some(files) if reported.insert((&edge.function, edge.file.as_str())) => {
                output.push_str(&format!(
                    "  protected (lexically) {} <- {}: a test calls this consumer by name\n",
                    edge.function,
                    files.iter().copied().collect::<Vec<_>>().join(", ")
                ));
            }
            Some(_) => {}
            None => output.push_str(&format!(
                "  needs evidence: {} calls {}, and no test calls {} by name, so this consumer's delegation is unproven\n",
                edge.function, edge.provider, edge.function
            )),
        }
    }
    let mut macros = 0usize;
    let mut objects = 0usize;
    for &(file, _) in seeds {
        let masked = xirang_kernel::source::mask_non_code(&sources[file].source);
        macros += masked
            .lines()
            .zip(sources[file].source.lines())
            .filter(|(masked, raw)| !masked.contains("!(") && raw.contains("!("))
            .count();
        objects += sources[file]
            .source
            .lines()
            .filter(|line| line.contains("dyn "))
            .count();
    }
    // Said once, and said as a count even when it is zero: "none" is evidence that the question was
    // asked, while a missing line is indistinguishable from never having looked.
    // 只说一次，而且是零的时候也把计数说出来："none"是"问过了"的证据，而整行缺席与"从没看过"无法区分。
    output.push_str(&match (macros, objects) {
        (0, 0) => "  unknown: no macro invocation or trait-object line in the selected files\n".to_owned(),
        _ => format!(
            "  unknown: {macros} macro invocation line(s), {objects} trait-object line(s) in the selected files; their dispatch is not resolved here\n"
        ),
    });
    output
}

/// What was looked at and what was not, without restating the dispatch caveat the header carries.
/// 看了什么、没看什么，且不复述头部已经写过的派发说明。
fn coverage(root: &Path, sources: &[SourceFile], seeds: &BTreeSet<(usize, usize)>) -> String {
    let files: BTreeSet<&str> = seeds
        .iter()
        .map(|&(file, _)| sources[file].relative.as_str())
        .collect();
    format!(
        "coverage: {} source file(s) parsed in this one query; {} selected for source; files outside {} are not examined\n",
        sources.len(),
        files.len(),
        root.display()
    )
}

/// The calls this projection makes worth making next, from the evidence it just produced.
/// 依据它刚产出的证据，接下来值得做的调用。
fn next_steps(
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    faces: &BTreeMap<String, String>,
) -> String {
    let mut output = String::from("next\n");
    for &(file, _) in seeds {
        let path = &sources[file].relative;
        if let Some(face) = faces.get(path) {
            output.push_str(&format!(
                "  converge {{node: \"{face}\"}} for the contract chain, then check after the edit\n"
            ));
        }
        output.push_str(&format!(
            "  check {{files: [\"{path}\"], census: false}} executes the tests that exist against these bytes\n"
        ));
    }
    output
}

/// Keep a line to a readable width without cutting a multi-byte character.
/// 把一行限制在可读宽度内，且不切断多字节字符。
fn shorten(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// Read `[package] name` from one manifest, section-aware and quote-aware.
/// 从一份清单里读 `[package] name`，识别区段与引号。
///
/// Narrow on purpose, and narrow the same way `source_layout`'s library-path read is: the forms a
/// manifest in this workspace actually uses. Anything else answers `None`, which is reported, rather
/// than a directory name that would be wrong wherever it was used.
/// 有意收窄，收窄的方式与 `source_layout` 读库路径一致：本工作区的清单实际会用的形式。其余写法一律答
/// `None` 并被如实报告，而不是拿一个在哪里用都错的目录名充数。
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(header) = trimmed.strip_prefix('[') {
            in_package = header.trim_end_matches(']').trim() == "package";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }
        let value = value.trim();
        let quote = value.chars().next()?;
        if quote != '"' && quote != '\'' {
            continue;
        }
        let rest = &value[1..];
        return rest.find(quote).map(|end| rest[..end].to_owned());
    }
    None
}

#[cfg(test)]
#[path = "explore_repair_tests.rs"]
mod explore_repair_tests;

/// Which siblings are legitimate references, and why a sibling alone is not one.
/// 哪些兄弟是合法参考，以及为什么"同级"本身不够。
///
/// §2.2: 「不同角色可能需要不同组织，例如无状态控件与持久化控件。**同级不是足够的参考选择条件**。」
/// The decidable form: a reference shares the **parent** *and* the `kind`. A sibling of a different
/// `kind` is a different role, and offering it as a model is how an agent copies a persistent
/// control's shape into a stateless one.
/// 可判定的形式：参考要同一个**父级** *且* 同一个 `kind`。`kind` 不同的兄弟是另一种角色，把它当样板，正是
/// 把有状态控件的形状抄进无状态控件的来路。
///
/// It costs one face scan, and it says so instead of hiding it: `explore` is a hot path, so the
/// line prints how many faces it looked at. A projection that quietly grew a scan is how a cheap
/// query becomes an expensive one.
/// 这会多一次 face 扫描，而它明说而不藏着：`explore` 是热路径，因此这一行印出看了多少面。一条悄悄多出一次
/// 扫描的投影，正是"便宜查询变贵"的来路。
fn references(
    root: &Path,
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    faces: &BTreeMap<String, String>,
) -> String {
    // The judgment lives in its own page: it reads four signals (flow contract, capability labels, kind and
    // the adoption ledger) and states a reason with each verdict. This wrapper exists only so the projection
    // keeps one call site.
    // 判断住在它自己那一页：它读四个信号（flow 合同、能力标签、kind、采信台账），并为每个判定给出理由。这个转发只
    // 是为了让投影保持一个调用点。
    siblings::section(root, sources, seeds, faces)
}

#[path = "explore_references.rs"]
mod siblings;
