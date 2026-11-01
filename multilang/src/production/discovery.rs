//! Loading a project's units from disk: the root and every local `path` dependency, plus the
//! inventory each unit contributes and the digest a plan is keyed on.
//! 从磁盘加载项目的单元：根与每一条本地 `path` 依赖，以及每个单元贡献的清点与计划取键用的摘要。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{Compiler, Project, Unit};
use crate::production::manifest::{Dependency, Kind, Language, Manifest};

/// The directory names neither inventory nor content hashing descends into.
/// 清点与内容摘要都不进入的目录名。
const SKIPPED: [&str; 6] = [
    "target",
    ".git",
    ".xirang",
    ".codegraph",
    ".direnv",
    "node_modules",
];

/// Load a project and every local dependency it names.
/// 加载一个项目及其点名的每一条本地依赖。
pub(crate) fn load(root: &Path) -> Result<Project, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let manifest = Manifest::load(&root)?;
    let sources = manifest.sources.clone();
    let overrides = manifest.overrides.clone();
    let mut used_overrides = BTreeSet::new();
    let mut units = Vec::new();
    // A directory reached twice is one unit: two consumers depending on the same library is a
    // diamond, not a cycle. A real cycle is rejected below by `acyclic`.
    // 被到达两次的目录是同一个单元：两个消费者依赖同一个库是菱形，不是环。真正的环由下面的
    // `acyclic` 拒绝。
    let mut loaded = BTreeSet::new();
    // The directory each unit was reached from, so the graph is edges between units rather than
    // edges between names: two sources may hold packages of the same name, and a dependency field
    // may be named differently from the package it resolves to.
    // 每个单元是从哪个目录到达的，于是依赖图是单元之间的边而不是名字之间的边：两个来源可以持有同名包，
    // 而依赖字段名也可以与它解析到的包名不同。
    let mut deps_by_dir: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut queue = vec![(root.clone(), manifest, None)];
    while let Some((directory, manifest, source)) = queue.pop() {
        if !loaded.insert(directory.clone()) {
            continue;
        }
        let unit = inventory(&directory, manifest, source)?;
        if unit.manifest.language == Language::External && !unit.manifest.dependencies.is_empty() {
            return Err(format!(
                "unit `{}` is written in an external language and names dependencies; this slice \
                 leaves an external project's dependency graph to its backend",
                unit.manifest.package.name
            ));
        }
        for (name, dependency) in unit.manifest.dependencies.clone() {
            let (resolved, origin, package) = match dependency {
                Dependency::Path(path) => (directory.join(&path), None, None),
                Dependency::Source { source, package } => {
                    let Some(spec) = sources.get(&source) else {
                        let declared = if sources.is_empty() {
                            "nothing".to_owned()
                        } else {
                            sources.keys().cloned().collect::<Vec<_>>().join(", ")
                        };
                        return Err(format!(
                            "dependency `{name}` names source `{source}`, which the root \
                             manifest's [sources] table does not declare; it declares: {declared}"
                        ));
                    };
                    let key = format!("{source}/{package}");
                    if let Some(path) = overrides.get(&key) {
                        used_overrides.insert(key.clone());
                        let path = if path.is_absolute() {
                            path.clone()
                        } else {
                            root.join(path)
                        };
                        (path, Some(format!("override:{key}")), Some(package))
                    } else {
                        let (directory, label) =
                            crate::production::sources::resolve(&root, &source, &package, spec)?;
                        (directory, Some(label), Some(package))
                    }
                }
            };
            let resolved = resolved.canonicalize().map_err(|error| {
                format!(
                    "dependency `{name}` at {} cannot be resolved: {error}",
                    resolved.display()
                )
            })?;
            if !resolved.join("xirang.toml").is_file() {
                return Err(format!(
                    "dependency `{name}` at {} has no xirang.toml; this slice resolves local \
                     directories only",
                    resolved.display()
                ));
            }
            let loaded = Manifest::load(&resolved)?;
            if let (Some(package), Some(origin)) = (&package, &origin)
                && &loaded.package.name != package
            {
                let message = if let Some(key) = origin.strip_prefix("override:") {
                    format!(
                        "dependency `{name}` overrides `{key}` for package `{package}`, but the \
                         package at {} is named `{}`; an override has to land on the package it names",
                        resolved.display(),
                        loaded.package.name
                    )
                } else {
                    format!(
                        "dependency `{name}` asks source `{origin}` for package `{package}`, but \
                         the package at {} is named `{}`; a source lookup must land on the package \
                         it names",
                        resolved.display(),
                        loaded.package.name
                    )
                };
                return Err(message);
            }
            deps_by_dir
                .entry(directory.clone())
                .or_default()
                .push(resolved.clone());
            queue.push((resolved, loaded, origin));
        }
        units.push(unit);
    }
    let unused: Vec<&str> = overrides
        .keys()
        .filter(|key| !used_overrides.contains(*key))
        .map(String::as_str)
        .collect();
    if !unused.is_empty() {
        return Err(format!(
            "override `{}` matches no dependency; an override that changes nothing is a typo \
             waiting to happen",
            unused.join("`, `")
        ));
    }
    // The root is the unit whose directory is the project root; put it first.
    let position = units
        .iter()
        .position(|unit| unit.directory == root)
        .ok_or("the project root disappeared from its own inventory")?;
    units.rotate_left(position);
    // A package name no longer has to identify one unit: two sources may both hold `greeter`, and
    // the dependency graph above keeps them apart by directory. A colliding name gets a stable
    // suffix from its location so artefacts, archives and export directories stay distinct; a
    // name that occurs once stays exactly what the manifest says.
    // 包名不再必须唯一标识一个单元：两个来源可以都持有 `greeter`，上面的依赖图用目录把它们分开。
    // 重名者得到由位置决定的稳定后缀，使产物、归档与导出目录互不相同；只出现一次的名字与清单一致。
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for unit in &units {
        *counts
            .entry(unit.manifest.package.name.clone())
            .or_default() += 1;
    }
    for unit in &mut units {
        if counts
            .get(&unit.manifest.package.name)
            .is_some_and(|count| *count > 1)
        {
            let location = relative_location(&root, &unit.directory);
            let suffix: String = if location == "." {
                "root".to_owned()
            } else {
                location
                    .chars()
                    .map(|character| {
                        if character.is_ascii_alphanumeric() {
                            character
                        } else {
                            '_'
                        }
                    })
                    .collect()
            };
            unit.key = format!("{}__{}", unit.manifest.package.name, suffix);
        }
    }
    let by_directory: BTreeMap<PathBuf, usize> = units
        .iter()
        .enumerate()
        .map(|(index, unit)| (unit.directory.clone(), index))
        .collect();
    let edges: Vec<Vec<usize>> = units
        .iter()
        .map(|unit| {
            let mut edges: Vec<usize> = deps_by_dir
                .get(&unit.directory)
                .map(|directories| {
                    directories
                        .iter()
                        .filter_map(|directory| by_directory.get(directory).copied())
                        .collect()
                })
                .unwrap_or_default();
            edges.sort_unstable();
            edges.dedup();
            edges
        })
        .collect();
    acyclic(&units, &edges)?;
    for unit in &units[1..] {
        if unit.manifest.package.kind != Kind::Library {
            return Err(format!(
                "dependency `{}` is a binary; a path dependency has to be a library",
                unit.manifest.package.name
            ));
        }
    }
    Ok(Project { root, units, edges })
}

/// Reject a dependency cycle: a consumer graph is an ordering, and a cycle has no link order.
/// 拒绝依赖环：消费者图是一种次序，而环没有合法的链接顺序。
fn acyclic(units: &[Unit], edges: &[Vec<usize>]) -> Result<(), String> {
    let mut state = vec![0u8; units.len()];
    fn visit_node(
        index: usize,
        units: &[Unit],
        edges: &[Vec<usize>],
        state: &mut [u8],
    ) -> Result<(), String> {
        match state[index] {
            1 => {
                return Err(format!(
                    "the path dependencies form a cycle through `{}`",
                    units[index].manifest.package.name
                ));
            }
            2 => return Ok(()),
            _ => {}
        }
        state[index] = 1;
        for &next in &edges[index] {
            visit_node(next, units, edges, state)?;
        }
        state[index] = 2;
        Ok(())
    }
    for index in 0..units.len() {
        visit_node(index, units, edges, &mut state)?;
    }
    Ok(())
}

/// The transitive dependencies of one unit, nearest first (a consumer before what it consumes).
/// 一个单元的传递依赖，近者在前（消费者先于它消费的东西）。
pub(super) fn dependencies_of(project: &Project, index: usize) -> Vec<usize> {
    let order = dependency_order(project);
    let position: std::collections::BTreeMap<usize, usize> = order
        .iter()
        .enumerate()
        .map(|(place, unit)| (*unit, place))
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![index];
    while let Some(unit) = stack.pop() {
        for dependency in &project.edges[unit] {
            if seen.insert(*dependency) {
                stack.push(*dependency);
            }
        }
    }
    let mut dependencies: Vec<usize> = seen.into_iter().collect();
    dependencies.sort_by_key(|unit| position.get(unit).copied().unwrap_or(usize::MAX));
    dependencies
}

/// The units in the order a static link has to name their archives: every unit before the units it
/// depends on, root first. A diamond therefore lists the shared dependency once, after both
/// consumers.
/// 静态链接点名归档时的单元顺序：每个单元排在它依赖的单元之前，根排第一。菱形依赖因此只列一次
/// 共享依赖，且排在两个消费者之后。
pub(super) fn dependency_order(project: &Project) -> Vec<usize> {
    let mut visited = vec![false; project.units.len()];
    let mut post = Vec::new();
    fn visit_unit(index: usize, project: &Project, visited: &mut [bool], post: &mut Vec<usize>) {
        if visited[index] {
            return;
        }
        visited[index] = true;
        for &next in &project.edges[index] {
            visit_unit(next, project, visited, post);
        }
        post.push(index);
    }
    visit_unit(0, project, &mut visited, &mut post);
    for index in 0..project.units.len() {
        visit_unit(index, project, &mut visited, &mut post);
    }
    post.reverse();
    post
}

/// The digest of everything the plan depends on. The same inputs give the same directory;
/// a changed source, manifest, dependency or compiler gives another one.
/// 计划所依赖的一切的摘要：同样的输入给同一个目录，源码/清单/依赖/编译器任一改变给另一个。
pub(crate) fn identity(
    project: &Project,
    compiler: &Compiler,
    facts: &mut crate::facts::Facts,
    declared: &crate::production::tree::DeclaredTree,
) -> String {
    let mut text = format!(
        "xirang.multilang.plan.v14\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        compiler.program,
        compiler.version,
        compiler.c_program,
        compiler.c_version,
        compiler.python_program,
        compiler.python_version,
        compiler.rust_program,
        compiler.rust_version,
        compiler.go_program,
        compiler.go_version,
        compiler.ruby_program,
        compiler.ruby_version,
        compiler.csharp_program,
        compiler.csharp_version,
        compiler.node_program,
        compiler.node_version,
        compiler.java_program,
        compiler.java_version,
        compiler.kotlin_program,
        compiler.kotlin_version,
        compiler.php_program,
        compiler.php_version,
        compiler.swift_program,
        compiler.swift_version
    );
    // The declaration documents are an input of the plan like any source file: the digest covers
    // every id and every declared field, so editing one face is a different plan.
    // 声明文档与任何源文件一样是计划的输入：摘要覆盖每个身份与每项声明字段，因此改一个面就是另一份计划。
    text.push_str(&format!(
        "declared\0{}\0{}\0",
        declared.namespace,
        declared.digest()
    ));
    for unit in &project.units {
        // Where the unit sits, expressed **relative to the project root** (`.` for the root unit,
        // `../dep` for a sibling dependency). This keeps one checkout's plan identity equal to
        // another's while still changing when a dependency moves (2026-10-19, cross-checkout cache).
        // 单元所在位置以**项目根相对**表示（根单元是 `.`，兄弟依赖是 `../dep`）。这让一个检出的计划
        // 身份等于另一个检出的，同时依赖搬家时仍然变化（2026-10-19，跨检出缓存）。
        let location = relative_location(&project.root, &unit.directory);
        text.push_str(&format!(
            "unit\0{}\0{}\0{:?}\0{:?}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
            unit.manifest.package.name,
            location,
            unit.manifest.package.kind,
            unit.manifest.language,
            unit.manifest.cpp.standard,
            unit.manifest.c.standard,
            unit.manifest.python.standard,
            unit.manifest.rust.standard,
            unit.manifest.go.standard,
            unit.manifest.ruby.standard,
            unit.manifest.csharp.standard,
            unit.manifest.javascript.standard,
            unit.manifest.typescript.standard,
            unit.manifest.java.standard,
            unit.manifest.kotlin.standard,
            unit.manifest.php.standard,
            unit.manifest.swift.standard,
            unit.source.as_deref().unwrap_or("-")
        ));
        for export in &unit.manifest.exports {
            text.push_str(&format!(
                "export\0{}\0{:?}\0{:?}\0",
                export.name, export.parameters, export.returns
            ));
        }
        text.push_str(&format!(
            "external\0{}\0",
            unit.manifest.external.as_deref().unwrap_or("-")
        ));
        for (key, value) in &unit.manifest.options {
            text.push_str(&format!("option\0{key}\0{value}\0"));
        }
        let mut files: Vec<&PathBuf> = unit.sources.iter().chain(&unit.tests).collect();
        files.sort();
        // Paths enter the identity **relative to the unit**, not absolute. The cache and the
        // build directory can then move between checkouts (`xirang cache export`/`import`), while a
        // renamed file still lands in a new plan because its recorded name changes.
        // 路径以**单元相对**的形式进入身份，而不是绝对路径。这样缓存与构建目录可以在检出之间移动
        // （`xirang cache export`/`import`），而改名仍然落进新计划，因为记录下来的名字变了。
        for file in files {
            let relative = file
                .strip_prefix(&unit.directory)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| file.display().to_string());
            text.push_str(&format!(
                "{}\0{}\0",
                relative,
                facts.digest(file).unwrap_or_default()
            ));
        }
    }
    xirang_kernel::sha256_hex(text.as_bytes())[..16].to_owned()
}

/// `directory` relative to `root`, lexically (`.` for the same directory).
/// `directory` 相对 `root` 的词典序路径（同一目录为 `.`）。
fn relative_location(root: &std::path::Path, directory: &std::path::Path) -> String {
    let root_components: Vec<_> = root.components().collect();
    let directory_components: Vec<_> = directory.components().collect();
    let mut common = 0;
    while common < root_components.len()
        && common < directory_components.len()
        && root_components[common] == directory_components[common]
    {
        common += 1;
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in common..root_components.len() {
        parts.push("..".to_owned());
    }
    for component in &directory_components[common..] {
        parts.push(component.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

/// Inventory one unit: candidates under `src/`, tests under `tests/`, headers under `include/`.
/// 清点一个单元：`src/` 下的候选、`tests/` 下的测试、`include/` 下的头文件。
fn inventory(directory: &Path, manifest: Manifest, source: Option<String>) -> Result<Unit, String> {
    let (test_suffix, source_extensions): (String, Vec<String>) = match manifest.language {
        Language::Cpp => ("_test".to_owned(), strings(&["c", "cc", "cpp", "cxx"])),
        Language::C => ("_test".to_owned(), strings(&["c"])),
        Language::Python => ("_test".to_owned(), strings(&["py"])),
        // A JavaScript unit's sources are `.js` modules plus the `.c` file an N-API addon is
        // written in; the plan builds that addon when it sees a C source.
        // JavaScript 单元的源码是 `.js` 模块，外加写 N-API 插件的 `.c` 文件；计划看到 C 源码
        // 时会构建那个插件。
        Language::Javascript => ("_test".to_owned(), strings(&["js", "c"])),
        Language::Typescript => ("_test".to_owned(), strings(&["ts", "c"])),
        Language::Rust => ("_test".to_owned(), strings(&["rs"])),
        Language::Go => ("_test".to_owned(), strings(&["go"])),
        Language::Ruby => ("_test".to_owned(), strings(&["rb"])),
        Language::Csharp => ("_test".to_owned(), strings(&["cs"])),
        Language::Java => ("_test".to_owned(), strings(&["java", "c"])),
        Language::Php => ("_test".to_owned(), strings(&["php"])),
        Language::Kotlin => ("_test".to_owned(), strings(&["kt", "c"])),
        Language::Swift => ("_test".to_owned(), strings(&["swift"])),
        // An external language's layout is the one its recorded backend declares: the extensions
        // its `src/` files use and the suffix its `tests/` files carry. A backend that cannot say
        // both cannot own a language's inventory.
        // 外部语言的布局由记录里的后端声明：`src/` 文件用的扩展名与 `tests/` 文件的词尾。说不出
        // 这两样，就没有资格拥有那门语言的清点。
        Language::External => {
            let id = manifest.external.clone().ok_or_else(|| {
                format!(
                    "unit `{}` is written in an external language and names no \
                     `[language.<id>]` table",
                    manifest.package.name
                )
            })?;
            match crate::production::backends::layout(directory, &id) {
                Ok(probe) if !probe.extensions.is_empty() => (probe.test_suffix, probe.extensions),
                _ => ("_test".to_owned(), Vec::new()),
            }
        }
    };
    let mut sources = Vec::new();
    visit_unit(&directory.join("src"), &mut sources)?;
    sources.retain(|path| {
        extension(path).is_some_and(|extension| {
            source_extensions.is_empty()
                || source_extensions
                    .iter()
                    .any(|candidate| candidate == extension)
        })
    });
    sources.sort();
    let mut tests = Vec::new();
    if let Ok(entries) = std::fs::read_dir(directory.join("tests")) {
        for entry in entries {
            let path = entry
                .map_err(|error| {
                    format!("cannot read {}: {error}", directory.join("tests").display())
                })?
                .path();
            if let Some(_extension) = extension(&path)
                && (source_extensions.is_empty()
                    || source_extensions
                        .iter()
                        .any(|candidate| candidate == _extension))
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem.ends_with(&test_suffix))
            {
                tests.push(path);
            }
        }
    }
    tests.sort();
    let include = directory.join("include");
    let key = manifest.package.name.clone();
    Ok(Unit {
        manifest,
        directory: directory.to_path_buf(),
        sources,
        tests,
        include: include.is_dir().then_some(include),
        source,
        key,
    })
}

/// Own a static word list as the strings an inventory compares.
/// 把静态词表变成清点比较用的字符串。
fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// Collect every regular file under `directory`, skipping the shared skip list and nested
/// packages. A nested `xirang.toml` is another project's authority, so its tree is not this
/// unit's source.
/// 收集 `directory` 下的每个普通文件，跳过共享跳过表与嵌套包。嵌套 `xirang.toml` 是另一个项目的
/// 权威，因此它那棵树不是本单元的源码。
fn visit_unit(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(());
    };
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            if path.join("xirang.toml").is_file() {
                continue;
            }
            visit_unit(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// The lowercase extension of a path.
/// 路径的小写扩展名。
fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|extension| extension.to_str())
}
