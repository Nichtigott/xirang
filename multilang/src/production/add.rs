//! `xirang add`: record one dependency in a project's `xirang.toml` — a local `path` entry, or a
//! `[sources]` entry that is declared by the same call when it is new.
//! `xirang add`：把一个依赖记进项目的 `xirang.toml`——本地 `path` 条目，或由同一次调用声明的
//! `[sources]` 条目。

use std::path::Path;

use super::manifest;
use super::sources;

/// Record a local `path` dependency in a project's `xirang.toml`.
/// 把一个本地 `path` 依赖写进项目的 `xirang.toml`。
///
/// The command edits the manifest the user already has; it never invents a second list. The
/// checks run **before** the write: a dependency that is a binary, a duplicate, or one that would
/// close a cycle is refused with the file untouched, and the edited text is parsed again before it
/// is written so a malformed edit cannot land.
/// 本命令编辑用户已有的清单，绝不另造第二份列表。所有检查都在**写入之前**：二进制依赖、重名、
/// 会闭合成环的依赖都会被拒绝且文件不动；改完的文本先重新解析，解析不过就不落盘。
pub fn add_dependency(directory: &Path, name: &str, path: &str) -> Result<String, String> {
    let root = directory.canonicalize().map_err(|error| {
        format!(
            "cannot read the project directory {}: {error}",
            directory.display()
        )
    })?;
    let manifest_path = root.join("xirang.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "{} has no xirang.toml; `xirang add` edits the manifest of an existing project",
            root.display()
        ));
    }
    if !super::valid_name(name) {
        return Err(format!(
            "`{name}` is not a package name: use lowercase letters, digits, `-` or `_`"
        ));
    }
    if path.is_empty() || path.starts_with('/') || path.contains('"') || path.contains('\\') {
        return Err(format!(
            "`{path}` is not a relative directory this manifest can carry: give a relative path \
             without quotes or backslashes"
        ));
    }
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let current = manifest::Manifest::load(&root)?;
    if current.language == manifest::Language::External {
        return Err(format!(
            "`{}` is written in an external language; its backend owns the dependency graph, so \
             `xirang add` does not edit it",
            root.display()
        ));
    }
    let manifest = manifest::Manifest::parse(&text)?;
    let dependency_dir = root.join(path);
    let dependency_root = dependency_dir.canonicalize().map_err(|error| {
        format!(
            "cannot read the dependency at {}: {error}",
            dependency_dir.display()
        )
    })?;
    if !dependency_root.is_dir() {
        return Err(format!(
            "{} is not a directory; a path dependency names a project directory",
            dependency_root.display()
        ));
    }
    let dependency = manifest::Manifest::load(&dependency_root)?;
    if dependency.package.name != name {
        return Err(format!(
            "the dependency at {} is `{}`, not `{name}`; name the package that is there",
            dependency_root.display(),
            dependency.package.name
        ));
    }
    if dependency.package.kind != manifest::Kind::Library {
        return Err(format!(
            "`{name}` is a binary project; only libraries can be dependencies in this slice"
        ));
    }
    if let Some(existing) = manifest.dependencies.get(name) {
        let existing = match existing {
            manifest::Dependency::Path(path) => path.display().to_string(),
            manifest::Dependency::Source { source, package } => format!("{source}:{package}"),
        };
        return Err(if existing == path {
            format!("this project already depends on `{name}` at `{path}`")
        } else {
            format!("this project already depends on `{name}` at `{existing}`, not `{path}`")
        });
    }
    if depends_on(
        &dependency_root,
        &root,
        &mut std::collections::BTreeSet::new(),
        0,
    )? {
        return Err(format!(
            "adding `{name}` would make this project depend on itself: `{name}` already depends on \
             this tree, so the new edge would close a cycle"
        ));
    }
    let updated = insert_in_table(
        &text,
        "[dependencies]",
        &format!("{name} = {{ path = \"{path}\" }}"),
    )?;
    let parsed = manifest::Manifest::parse(&updated)?;
    if !parsed.dependencies.contains_key(name) {
        return Err(
            "the edited manifest does not name the dependency; nothing was written".to_owned(),
        );
    }
    std::fs::write(&manifest_path, &updated)
        .map_err(|error| format!("cannot write {}: {error}", manifest_path.display()))?;
    Ok(format!(
        "added `{name} = {{ path = \"{path}\" }}` to {}\nrun `xirang build` to compile the new \
         dependency graph",
        manifest_path.display()
    ))
}

/// Whether `directory` reaches `needle` through its local `path` dependencies.
/// `directory` 是否沿本地 `path` 依赖走到 `needle`。
fn depends_on(
    directory: &Path,
    needle: &Path,
    seen: &mut std::collections::BTreeSet<std::path::PathBuf>,
    depth: usize,
) -> Result<bool, String> {
    const MAX_DEPTH: usize = 8;
    if depth > MAX_DEPTH {
        return Err(format!(
            "the dependency graph is deeper than {MAX_DEPTH}; refusing to judge whether the new \
             edge closes a cycle"
        ));
    }
    if !seen.insert(directory.to_path_buf()) {
        return Ok(false);
    }
    let manifest = manifest::Manifest::load(directory)?;
    for dependency in manifest.dependencies.values() {
        let manifest::Dependency::Path(path) = dependency else {
            continue;
        };
        let resolved = directory.join(path);
        let Ok(canonical) = resolved.canonicalize() else {
            continue;
        };
        if canonical == needle || depends_on(&canonical, needle, seen, depth + 1)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Insert one entry into a table, keeping the rest of the file as it was.
/// 往一张表里插一条条目，其余内容按原样保留。
fn insert_in_table(text: &str, table: &str, entry: &str) -> Result<String, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let header = lines
        .iter()
        .position(|line| line.trim_start().split('#').next().unwrap_or("").trim() == table);
    match header {
        Some(index) => {
            let mut end = index + 1;
            while end < lines.len() && !lines[end].trim_start().starts_with('[') {
                end += 1;
            }
            let mut at = end;
            while at > index + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            lines.insert(at, entry.to_owned());
        }
        None => {
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            lines.push(String::new());
            lines.push(table.to_owned());
            lines.push(entry.to_owned());
        }
    }
    let mut updated = lines.join("\n");
    updated.push('\n');
    Ok(updated)
}

/// What `xirang add --source` says about that source: it already exists, or this call declares it.
/// `xirang add --source` 对那个源的交代：它已经存在，或这次调用正在声明它。
pub enum SourceDeclaration {
    /// The `[sources]` entry already exists; the dependency points at it.
    /// `[sources]` 条目已存在；依赖指向它。
    Existing,
    /// Declare a local directory source.
    /// 声明一个本地目录源。
    Path(String),
    /// Declare a git source at one exact revision.
    /// 声明一个钉在确切版本上的 git 源。
    Git {
        /// The repository URL.
        /// 仓库地址。
        url: String,
        /// The exact revision to pin.
        /// 要钉住的确切版本。
        rev: String,
    },
    /// Declare a local registry source.
    /// 声明一个本地注册表源。
    Registry(String),
}

/// Record a dependency reached through a named `[sources]` entry, declaring that entry when it is
/// new. Every check runs before the write: the source resolves to a library with the package's own
/// name, the dependency is not a duplicate, and the new edge does not close a cycle.
/// 记一条经具名 `[sources]` 条目到达的依赖；条目是新的就一并声明。所有检查都在写入之前：源要解析到
/// 同名库、依赖不重复、新边不闭合环。
pub fn add_from_source(
    directory: &Path,
    name: &str,
    source: &str,
    declaration: SourceDeclaration,
) -> Result<String, String> {
    let root = directory.canonicalize().map_err(|error| {
        format!(
            "cannot read the project directory {}: {error}",
            directory.display()
        )
    })?;
    let manifest_path = root.join("xirang.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "{} has no xirang.toml; `xirang add` edits the manifest of an existing project",
            root.display()
        ));
    }
    if !super::valid_name(name) {
        return Err(format!(
            "`{name}` is not a package name: use lowercase letters, digits, `-` or `_`"
        ));
    }
    if !super::valid_name(source) {
        return Err(format!(
            "`{source}` is not a source name: use lowercase letters, digits, `-` or `_`"
        ));
    }
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let current = manifest::Manifest::load(&root)?;
    if current.language == manifest::Language::External {
        return Err(format!(
            "`{}` is written in an external language; its backend owns the dependency graph, so \
             `xirang add` does not edit it",
            root.display()
        ));
    }
    let manifest = manifest::Manifest::parse(&text)?;
    let spec = match &declaration {
        SourceDeclaration::Existing => match manifest.sources.get(source) {
            Some(spec) => spec.clone(),
            None => {
                return Err(format!(
                    "`{source}` is not a declared source; this project declares {}",
                    declared(&manifest)
                ));
            }
        },
        SourceDeclaration::Path(path) => {
            member(path, "directory")?;
            manifest::SourceSpec::Path(std::path::PathBuf::from(path))
        }
        SourceDeclaration::Git { url, rev } => {
            value(url, "URL")?;
            value(rev, "revision")?;
            manifest::SourceSpec::Git {
                url: url.clone(),
                rev: rev.clone(),
            }
        }
        SourceDeclaration::Registry(directory) => {
            member(directory, "registry directory")?;
            manifest::SourceSpec::Registry {
                directory: std::path::PathBuf::from(directory),
            }
        }
    };
    if manifest.sources.contains_key(source) && !matches!(&declaration, SourceDeclaration::Existing)
    {
        return Err(format!(
            "`{source}` is already declared as {}; name that entry or choose another name",
            describe(&manifest.sources[source])
        ));
    }
    let (resolved, label) = sources::resolve(&root, source, name, &spec)?;
    let dependency = manifest::Manifest::load(&resolved)?;
    if dependency.package.name != name {
        return Err(format!(
            "source `{source}` resolves `{name}` to {}, but that package is `{}`; name the package              that is there",
            resolved.display(),
            dependency.package.name
        ));
    }
    if dependency.package.kind != manifest::Kind::Library {
        return Err(format!(
            "`{name}` is a binary project; only libraries can be dependencies in this slice"
        ));
    }
    if let Some(existing) = manifest.dependencies.get(name) {
        return Err(match existing {
            manifest::Dependency::Source {
                source: declared,
                package,
            } => format!(
                "this project already depends on `{name}` through source `{declared}` (package                  `{package}`)"
            ),
            manifest::Dependency::Path(path) => format!(
                "this project already depends on `{name}` at `{}`",
                path.display()
            ),
        });
    }
    if depends_on(&resolved, &root, &mut std::collections::BTreeSet::new(), 0)? {
        return Err(format!(
            "adding `{name}` would make this project depend on itself: it already depends on this              tree through local paths, so the new edge would close a cycle"
        ));
    }
    let mut updated = match &declaration {
        SourceDeclaration::Existing => text.clone(),
        _ => insert_in_table(
            &text,
            "[sources]",
            &format!("{source} = {}", render_source(&spec)),
        )?,
    };
    updated = insert_in_table(
        &updated,
        "[dependencies]",
        &format!("{name} = {{ source = \"{source}\", package = \"{name}\" }}"),
    )?;
    let parsed = manifest::Manifest::parse(&updated)?;
    match parsed.dependencies.get(name) {
        Some(manifest::Dependency::Source {
            source: declared,
            package,
        }) if declared == source && package == name => {}
        _ => {
            return Err(
                "the edited manifest does not name the dependency; nothing was written".to_owned(),
            );
        }
    }
    if let Some(declared_spec) = parsed.sources.get(source)
        && declared_spec != &spec
    {
        return Err("the edited manifest declares another source; nothing was written".to_owned());
    }
    std::fs::write(&manifest_path, &updated)
        .map_err(|error| format!("cannot write {}: {error}", manifest_path.display()))?;
    Ok(format!(
        "added `{name} = {{ source = \"{source}\", package = \"{name}\" }}` to {}\n  source `{source}` resolves it to {} ({label})\nrun `xirang build` to compile the new dependency graph",
        manifest_path.display(),
        resolved.display()
    ))
}

/// The declared source names, or `nothing`.
/// 已声明的源名字，或 `nothing`。
fn declared(manifest: &manifest::Manifest) -> String {
    if manifest.sources.is_empty() {
        return "nothing".to_owned();
    }
    manifest
        .sources
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

/// A relative path a manifest can carry.
/// 清单能承载的相对路径。
fn member(value: &str, what: &str) -> Result<(), String> {
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('"')
        || value.contains('\\')
        || value.contains('\n')
    {
        return Err(format!(
            "`{value}` is not a relative {what} this manifest can carry: give a relative path              without quotes or backslashes"
        ));
    }
    Ok(())
}

/// A free value (URL, revision) a manifest can carry.
/// 清单能承载的自由取值（URL、版本）。
fn value(value: &str, what: &str) -> Result<(), String> {
    if value.is_empty() || value.contains('"') || value.contains('\\') || value.contains('\n') {
        return Err(format!(
            "`{value}` is not a {what} this manifest can carry: it may not be empty, or contain a              quote, a backslash or a newline"
        ));
    }
    Ok(())
}

/// How a source is described in a refusal.
/// 拒绝文案里如何描述一个源。
fn describe(spec: &manifest::SourceSpec) -> String {
    match spec {
        manifest::SourceSpec::Path(path) => format!("a path source at `{}`", path.display()),
        manifest::SourceSpec::Git { url, rev } => format!("a git source `{url}` at `{rev}`"),
        manifest::SourceSpec::Registry { directory } => {
            format!("a registry source at `{}`", directory.display())
        }
    }
}

/// One `[sources]` entry in the inline form the parser accepts.
/// 解析器接受的 `[sources]` 内联形式。
fn render_source(spec: &manifest::SourceSpec) -> String {
    match spec {
        manifest::SourceSpec::Path(path) => format!("{{ path = \"{}\" }}", path.display()),
        manifest::SourceSpec::Git { url, rev } => {
            format!("{{ git = \"{url}\", rev = \"{rev}\" }}")
        }
        manifest::SourceSpec::Registry { directory } => {
            format!("{{ registry = \"{}\" }}", directory.display())
        }
    }
}
