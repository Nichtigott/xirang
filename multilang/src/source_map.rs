//! The project's declared sources: which extra roots a read may enter, and where **this machine**
//! keeps each one.
//! 项目声明的来源：一次读取可以进入哪些额外的根，以及**本机**把每一个放在哪里。
//!
//! Two files, because they answer two different questions and travel differently:
//! 两份文件，因为它们回答两个不同的问题，而且传播方式不同：
//!
//! * `xirang.sources.tsv` at the package root is **part of the project**. It names each source and
//!   the package-relative place the project materializes it at. It never carries an absolute path:
//!   where a checkout sits is not a fact about the project.
//! * `.xirang/source-bindings.tsv` is **machine-local**. It maps a source id to the absolute root
//!   this machine keeps it at. It is ignored by version control, because another machine will have
//!   it somewhere else — or not at all, which is the case this module refuses rather than guesses.
//! * 包根的 `xirang.sources.tsv` 是**项目的一部分**：它点名每个来源，以及项目把它物化在包内的哪个位置。
//!   它绝不携带绝对路径：检出放在哪儿不是关于项目的事实。
//! * `.xirang/source-bindings.tsv` 是**本机**的：它把来源 id 映射到本机保留它的绝对根。它被版本控制
//!   忽略，因为另一台机器会把它放在别处——或者根本没有，而那正是本模块拒绝猜测、只如实报错的情形。
//!
//! This is what makes an explicit external source different from an undeclared link. The link is
//! only a materialization; the declaration is the permission, and the binding is the one place the
//! machine-specific path is written down. Without both, a link out of the package stays refused.
//! 这正是"显式外部来源"与"未声明链接"的区别：链接只是物化，声明才是许可，而绑定是本机路径唯一的落笔处。
//! 两者缺一，包外链接仍然被拒绝。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::object_map::relative_path;

/// The project file that declares sources. Part of the package, safe to commit.
/// 声明来源的项目文件。属于包的一部分，可以入库。
pub const SOURCES_FILE: &str = "xirang.sources.tsv";

/// The machine-local file that binds a declared source to a path here. Never committed.
/// 把已声明来源绑定到本机路径的文件。绝不入库。
pub const BINDINGS_FILE: &str = ".xirang/source-bindings.tsv";

/// The source id that means "the package itself". It is implicit and cannot be declared.
/// 表示"包自身"的来源 id。它是隐式的，不能被声明。
pub const WORKSPACE_SOURCE: &str = "workspace";

/// The first line of the project file.
/// 项目文件的第一行。
pub const SOURCES_HEADER: &str = "# xirang sources v1";

/// The first line of the machine file.
/// 本机文件的第一行。
pub const BINDINGS_HEADER: &str = "# xirang source bindings v1";

/// How a declared source is supplied.
/// 已声明来源的供给方式。
///
/// One variant today, and the parser refuses any other spelling by name instead of treating an
/// unknown kind as "local" — a default here would be a capability nobody implemented.
/// 今天只有一个取值；解析器按名拒绝其它拼写，而不是把未知类型当成 `local`——那样的默认值等于一个没人实现的
/// 能力。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// A directory on this machine, materialized inside the package (possibly through a link).
    /// 本机的一个目录，在包内物化（可以经链接）。
    Local,
}

impl SourceKind {
    /// The spelling the project file uses.
    /// 项目文件使用的拼写。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
        }
    }

    /// Parse one spelling, naming the supported set when it is not one.
    /// 解析一种拼写；不是其中之一时点名受支持的集合。
    fn parse_kind(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            other => Err(format!(
                "`{other}` is not a source kind this reader understands; the kinds are `local`"
            )),
        }
    }
}

/// One source the project declares.
/// 项目声明的一个来源。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDeclaration {
    /// The shared identifier. It is what a binding and a diagnostic name.
    /// 共享标识符。绑定与诊断点名的就是它。
    pub id: String,
    /// How it is supplied.
    /// 它的供给方式。
    pub kind: SourceKind,
    /// Where the project materializes it, package-relative. Always a relative path.
    /// 项目把它物化在哪里，包内相对。永远是相对路径。
    pub location: String,
}

/// The project's declarations.
/// 项目的声明集合。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    declarations: Vec<SourceDeclaration>,
}

/// A declared source resolved for this machine.
/// 为这台机器解析好的已声明来源。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRoot {
    /// The shared identifier.
    /// 共享标识符。
    pub id: String,
    /// The package-relative materialization the project names.
    /// 项目点名的包内相对物化位置。
    pub location: PathBuf,
    /// The root this machine actually reads, when a binding supplies it.
    /// 本机实际读取的根（当绑定提供它时）。
    pub bound: Option<PathBuf>,
}

impl SourceMap {
    /// An empty map: a project that declares no extra source.
    /// 空映射：没有声明任何额外来源的项目。
    pub fn empty() -> Self {
        Self::default()
    }

    /// Read the project's declarations. A missing file is an empty map.
    /// 读取项目的声明。文件不存在即空映射。
    pub fn read_from(package_root: &Path) -> Result<Self, String> {
        let path = package_root.join(SOURCES_FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse_text(&text, &path.display().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    /// Parse a declaration file. `origin` names the file in every error.
    /// 解析声明文件。`origin` 在每条错误里点名那个文件。
    pub fn parse_text(text: &str, origin: &str) -> Result<Self, String> {
        let mut map = Self::empty();
        for (number, line) in numbered_rows(text, origin, SOURCES_HEADER)? {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != 3 {
                return Err(format!(
                    "{origin}:{number}: a line has {} tab-separated field(s), and this format has 3 \
                     (source id, kind, package-relative location)",
                    fields.len()
                ));
            }
            let id = check_id(fields[0], origin, number)?;
            let kind = SourceKind::parse_kind(fields[1].trim())
                .map_err(|error| format!("{origin}:{number}: {error}"))?;
            let location = relative_path(fields[2].trim())
                .map_err(|error| format!("{origin}:{number}: {error}"))?;
            if let Some(previous) = map.declarations.iter().find(|known| known.id == id) {
                return Err(format!(
                    "{origin}:{number}: the source `{id}` is declared twice (the first time with \
                     location `{}`); one id is one source",
                    previous.location
                ));
            }
            map.declarations
                .push(SourceDeclaration { id, kind, location });
        }
        Ok(map)
    }

    /// Every declaration, in file order.
    /// 每个声明，按文件顺序。
    pub fn declarations(&self) -> &[SourceDeclaration] {
        &self.declarations
    }

    /// The declared source a package-relative spelling lives in, by the **longest** location that
    /// covers it. The package itself is [`WORKSPACE_SOURCE`], which is implicit and always readable.
    /// 一个包内相对拼写所在的已声明来源，按覆盖它的**最长**位置判定。包自身是 [`WORKSPACE_SOURCE`]，
    /// 它是隐式的、永远可读。
    ///
    /// This is a lookup, not a guess: the locations are the project's own declarations, and a file
    /// under none of them is the workspace's. It exists so a fact extracted from a declared source
    /// can say which source it came from instead of looking like a file of the package.
    /// 这是查表，不是猜测：位置就是项目自己的声明，不属于任何声明的文件属于工作区。它存在的意义，是让
    /// 从已声明来源提取出的事实能说出自己来自哪个来源，而不是看起来像包里的一个文件。
    pub fn origin_of(&self, source: &str) -> &str {
        let mut best: Option<&SourceDeclaration> = None;
        for declaration in &self.declarations {
            let location = declaration.location.trim_end_matches('/');
            let covered = source == location
                || source
                    .strip_prefix(location)
                    .is_some_and(|rest| rest.starts_with('/'));
            if covered
                && best.is_none_or(|current| {
                    location.len() > current.location.trim_end_matches('/').len()
                })
            {
                best = Some(declaration);
            }
        }
        best.map_or(WORKSPACE_SOURCE, |declaration| declaration.id.as_str())
    }

    /// Resolve every declared source for this machine.
    /// 为这台机器解析每个已声明来源。
    ///
    /// Three refusals carry the semantics the review asked for: a source with no binding, a binding
    /// that disagrees with the project's location, and a binding for a source nobody declared. Each
    /// names both sides; none of them falls back to a guess.
    /// 三条拒绝承载复核要求的语义：没有绑定的来源、与项目位置不一致的绑定、以及为无人声明的来源写的绑定。
    /// 每条都点名两侧；没有一条回退到猜测。
    pub fn roots(&self, package_root: &Path) -> Result<Vec<SourceRoot>, String> {
        let mut bindings = bindings(package_root)?;
        let mut roots = Vec::new();
        for declaration in &self.declarations {
            let location = package_root.join(&declaration.location);
            let bound = bindings.remove(&declaration.id);
            let bound = match bound {
                Some(bound) => {
                    // The binding says where this machine keeps the source. If the project's
                    // materialization resolves somewhere else, one of the two is stale — and a
                    // reader that picks either one silently reads a tree nobody described.
                    // 绑定说本机把来源放在哪。如果项目的物化解析到别处，两者之一已经过期——而静默采用任一方的
                    // 读取方读到的是没人描述过的树。
                    let materialized = std::fs::canonicalize(&location).map_err(|error| {
                        format!(
                            "`{}` is declared at `{}`, which this machine cannot read: {error}. \
                             Materialize it there, or repair `{SOURCES_FILE}`",
                            declaration.id, declaration.location
                        )
                    })?;
                    let bound_real = std::fs::canonicalize(&bound).map_err(|error| {
                        format!(
                            "the binding for `{}` names `{}`, which this machine cannot read: \
                             {error}; repair `{BINDINGS_FILE}`",
                            declaration.id,
                            bound.display()
                        )
                    })?;
                    if materialized != bound_real {
                        return Err(format!(
                            "`{}` is materialized at `{}` ({}) but this machine's binding names \
                             `{}` ({}); one source has one location here. Repair `{BINDINGS_FILE}` \
                             or `{SOURCES_FILE}`",
                            declaration.id,
                            declaration.location,
                            materialized.display(),
                            bound.display(),
                            bound_real.display()
                        ));
                    }
                    Some(bound)
                }
                None => {
                    // No binding: the materialization has to be the source itself — a real
                    // directory the project carries. A link out of the package is not: it is the
                    // shape that needs a binding, and saying so is the whole point of refusing.
                    // 没有绑定：物化位置必须**就是**来源本身——项目自带的一个真实目录。包外链接不是：那正是
                    // 需要绑定的形状，而如实说出来正是拒绝的意义。
                    let metadata = std::fs::symlink_metadata(&location).map_err(|error| {
                        format!(
                            "`{}` is declared at `{}`, which does not exist on this machine: \
                             {error}. Materialize it there, or bind it in `{BINDINGS_FILE}`",
                            declaration.id, declaration.location
                        )
                    })?;
                    if metadata.file_type().is_symlink() {
                        let real =
                            std::fs::canonicalize(&location).unwrap_or_else(|_| location.clone());
                        let package = std::fs::canonicalize(package_root)
                            .unwrap_or_else(|_| package_root.to_path_buf());
                        if !real.starts_with(&package) {
                            return Err(format!(
                                "`{}` is materialized as a link to {}, which is outside this \
                                 package, and this machine has no binding for it. Write the root \
                                 this machine keeps it at into `{BINDINGS_FILE}` (that file is \
                                 machine-local and is not part of the project), or materialize the \
                                 source inside the package",
                                declaration.id,
                                real.display()
                            ));
                        }
                    }
                    None
                }
            };
            roots.push(SourceRoot {
                id: declaration.id.clone(),
                location,
                bound,
            });
        }
        if let Some(undeclared) = bindings.keys().next() {
            return Err(format!(
                "`{BINDINGS_FILE}` binds `{undeclared}`, and no source in `{SOURCES_FILE}` declares \
                 it; a binding without a declaration permits nothing, so this is reported instead \
                 of ignored"
            ));
        }
        Ok(roots)
    }
}

/// The machine's bindings: source id to absolute root.
/// 本机的绑定：来源 id 到绝对根。
fn bindings(package_root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let path = package_root.join(BINDINGS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let origin = path.display().to_string();
    let mut map = BTreeMap::new();
    for (number, line) in numbered_rows(&text, &origin, BINDINGS_HEADER)? {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 2 {
            return Err(format!(
                "{origin}:{number}: a line has {} tab-separated field(s), and this format has 2 \
                 (source id, absolute root)",
                fields.len()
            ));
        }
        let id = check_id(fields[0], &origin, number)?;
        let root = PathBuf::from(fields[1].trim());
        if !root.is_absolute() {
            return Err(format!(
                "{origin}:{number}: `{}` is not an absolute path; a binding says where **this \
                 machine** keeps the source, and a relative path would be read against whatever \
                 working directory the process happens to have",
                fields[1].trim()
            ));
        }
        if let Some(previous) = map.insert(id.clone(), root.clone()) {
            return Err(format!(
                "{origin}:{number}: `{id}` is bound twice (`{}` and `{}`); one source has one \
                 location on this machine",
                previous.display(),
                root.display()
            ));
        }
    }
    Ok(map)
}

/// The data rows of a ledger-shaped file, after the header check, with their 1-based line numbers.
/// 台账形状文件的数据行：表头校验之后，连同它们的 1 基行号。
fn numbered_rows<'a>(
    text: &'a str,
    origin: &str,
    header_expected: &str,
) -> Result<Vec<(usize, &'a str)>, String> {
    let mut rows = Vec::new();
    let mut header_seen = false;
    for (offset, line) in text.lines().enumerate() {
        let number = offset + 1;
        let trimmed = line.trim();
        if !header_seen {
            if trimmed.is_empty() {
                continue;
            }
            if trimmed != header_expected {
                return Err(format!(
                    "{origin}:{number}: the first line is `{trimmed}`, and this reader understands \
                     `{header_expected}`; a file of another version is not guessed at"
                ));
            }
            header_seen = true;
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        rows.push((number, line));
    }
    if !header_seen {
        return Err(format!(
            "{origin}: an empty file; write `{header_expected}` and one line per source, or delete \
             it to mean `no extra source`"
        ));
    }
    Ok(rows)
}

/// One source id, or an error naming the reserved one and the shape an id has.
/// 一个来源 id，或点名保留 id 与 id 形状的错误。
fn check_id(value: &str, origin: &str, number: usize) -> Result<String, String> {
    let id = value.trim();
    if id == WORKSPACE_SOURCE {
        return Err(format!(
            "{origin}:{number}: `{WORKSPACE_SOURCE}` is the implicit package source and cannot be \
             declared; it is always readable"
        ));
    }
    let valid = !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        && !id.starts_with('.')
        && !id.starts_with('-');
    if !valid {
        return Err(format!(
            "{origin}:{number}: `{id}` is not a source id; an id is one path segment of ASCII \
             letters, digits, `.`, `_` or `-` (it is used as a spelling, not as a path)"
        ));
    }
    Ok(id.to_owned())
}

#[cfg(test)]
#[path = "source_map_tests.rs"]
mod source_map_tests;
