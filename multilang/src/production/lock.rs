//! `xirang.lock`: the resolved local dependency graph, recorded so a later build can say whether
//! it changed.
//! `xirang.lock`：已解析的本地依赖图，记录它以便后续构建说出是否变化。

use super::manifest::Kind;
use super::project::Project;

/// The lock file's name.
/// 锁文件的文件名。
const LOCK: &str = "xirang.lock";

/// Compare the resolved graph with the recorded one, and write the recording when allowed.
/// 把已解析的图与记录的图比较；允许时写入记录。
///
/// Returns `Ok(Some(line))` when the file was (re)written, `Ok(None)` when it already matched, and
/// `Err` when `--locked` forbids the update the graph requires. The check runs before any compile
/// action, so a refusal costs nothing but the resolution itself.
/// 文件被（重）写时返回 `Ok(Some(line))`，已经一致时返回 `Ok(None)`，而 `--locked` 禁止图所要求的
/// 更新时返回 `Err`。检查发生在任何编译动作之前，因此一次拒绝只花掉解析本身的代价。
pub(super) fn verify(project: &Project, locked: bool) -> Result<Option<String>, String> {
    let path = project.root.join(LOCK);
    let expected = render_lock(project);
    let existing = std::fs::read_to_string(&path);
    let existing = match existing {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    if existing.as_deref() == Some(expected.as_str()) {
        return Ok(None);
    }
    if locked && existing.is_none() {
        return Err(format!(
            "--locked needs an existing {}; this project has none. Run `xirang build` once \
             without --locked to record the graph it resolves",
            path.display()
        ));
    }
    if locked {
        let recorded = existing
            .as_deref()
            .and_then(units)
            .map(|units| units.join(", "))
            .unwrap_or_else(|| "an unreadable xirang.lock".to_owned());
        return Err(format!(
            "xirang.lock records a different dependency graph than the manifests resolve to: \
             recorded {recorded}, resolved to {}. --locked forbids updating it; run `xirang build` \
             without --locked to record the new graph, and read the diff before committing it",
            units(&expected)
                .map(|units| units.join(", "))
                .unwrap_or_default()
        ));
    }
    std::fs::write(&path, &expected)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(Some(
        "xirang.lock: updated (the dependency graph changed)".to_owned(),
    ))
}

/// Re-resolve and record the graph in one transaction. The caller has already loaded the project,
/// so every unit and edge is valid; this writes the new text only after that whole validation, to a
/// sibling temporary file that is renamed over the lock, and `--check` writes nothing.
/// 在一次事务里重新解析并记录依赖图。调用方已经加载过项目，因此每个单元与边都合法；本函数在整份校验
/// 之后才把新文本写到同目录临时文件再改名覆盖锁文件，`--check` 不写任何东西。
pub(super) fn update(project: &Project, check: bool) -> Result<String, String> {
    let path = project.root.join(LOCK);
    let expected = render_lock(project);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    if existing.as_deref() == Some(expected.as_str()) {
        return Ok(format!(
            "xirang.lock: already records the resolved graph ({} unit(s)); nothing to update",
            project.units.len()
        ));
    }
    let before = existing.as_deref().map(parse_locked).unwrap_or_default();
    let after = parse_locked(&expected);
    let diff = render_diff(&before, &after);
    if check {
        return Ok(format!(
            "xirang.lock: would be updated (--check: no file written)\n{diff}"
        ));
    }
    let temporary = path.with_extension("lock.tmp");
    if let Err(error) = std::fs::write(&temporary, &expected) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("cannot write {}: {error}", temporary.display()));
    }
    if let Err(error) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("cannot replace {}: {error}", path.display()));
    }
    Ok(format!("xirang.lock: updated\n{diff}"))
}

/// One unit a lock text records.
/// 锁文本记录的一个单元。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Locked {
    name: String,
    version: String,
    source: Option<String>,
    path: String,
}

impl Locked {
    /// The unit in one line, naming where it came from.
    /// 单元的一行描述，点名它来自哪里。
    fn line(&self) -> String {
        match &self.source {
            Some(source) => format!(
                "{} {} (source {source}, path {})",
                self.name, self.version, self.path
            ),
            None => format!("{} {} (path {})", self.name, self.version, self.path),
        }
    }
}

/// Parse the units a lock text records.
/// 解析锁文本记录的单元。
fn parse_locked(text: &str) -> Vec<Locked> {
    let mut out = Vec::new();
    let mut current: Option<Locked> = None;
    for line in text.lines() {
        let line = line.trim();
        if line == "[[unit]]" {
            if let Some(unit) = current.take() {
                out.push(unit);
            }
            current = Some(Locked {
                name: String::new(),
                version: String::new(),
                source: None,
                path: String::new(),
            });
        } else if let Some(unit) = current.as_mut() {
            if let Some(value) = quoted(line, "name = ") {
                unit.name = value;
            } else if let Some(value) = quoted(line, "version = ") {
                unit.version = value;
            } else if let Some(value) = quoted(line, "source = ") {
                unit.source = Some(value);
            } else if let Some(value) = quoted(line, "path = ") {
                unit.path = value;
            }
        }
    }
    if let Some(unit) = current {
        out.push(unit);
    }
    out.retain(|unit| !unit.name.is_empty());
    out
}

/// The added, removed and changed units between two recordings.
/// 两次记录之间新增、移除与变化的单元。
fn render_diff(before: &[Locked], after: &[Locked]) -> String {
    let mut lines = Vec::new();
    for old in before {
        match after.iter().find(|new| new.name == old.name) {
            Some(new) if new == old => {}
            Some(new) => lines.push(format!("~ {}\n  -> {}", old.line(), new.line())),
            None => lines.push(format!("- {}", old.line())),
        }
    }
    for new in after {
        if !before.iter().any(|old| old.name == new.name) {
            lines.push(format!("+ {}", new.line()));
        }
    }
    if lines.is_empty() {
        "  (same units; only the file text changed)".to_owned()
    } else {
        lines.join("\n")
    }
}

/// The text the lock must have for this resolution.
/// 这次解析对应的锁文件文本。
fn render_lock(project: &Project) -> String {
    let mut text = String::from(
        "# xirang.lock — written by `xirang build`; the resolved local dependency graph.\n\
         # xirang.lock — 由 `xirang build` 写入；已解析的本地依赖图。\nschema = 1\n",
    );
    for unit in &project.units {
        let relative = unit
            .directory
            .strip_prefix(&project.root)
            .map(|path| {
                let text = path.display().to_string();
                if text.is_empty() {
                    ".".to_owned()
                } else {
                    text
                }
            })
            .unwrap_or_else(|_| unit.directory.display().to_string());
        text.push_str(&format!(
            "\n[[unit]]\nname = \"{}\"\nversion = \"{}\"\nkind = \"{}\"\npath = \"{}\"\n",
            unit.manifest.package.name,
            unit.manifest.package.version,
            match unit.manifest.package.kind {
                Kind::Library => "library",
                Kind::Binary => "binary",
            },
            relative
        ));
        if let Some(source) = &unit.source {
            text.push_str(&format!("source = \"{source}\"\n"));
        }
        // The export surface and the test list are part of what the lock promises: a lock that
        // recorded only names and versions would let a removed export (and the test that used to
        // call it) go unnoticed, because removing both makes the remaining tests pass.
        // 导出面与测试清单是锁承诺的一部分：只记名字与版本的锁，会让“删掉导出、再删掉调用它的
        // 测试”这种两错相抵的改动悄悄通过，因为剩下的测试照样通过。
        for export in &unit.manifest.exports {
            text.push_str(&format!(
                "export = \"{}({} params) -> {:?} {:?}\"\n",
                export.name,
                export.parameters.len(),
                export.returns,
                export.errors
            ));
        }
        for test in &unit.tests {
            let relative = test
                .strip_prefix(&unit.directory)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| test.display().to_string());
            text.push_str(&format!("test = \"{relative}\"\n"));
        }
    }
    text
}

/// The `name version` pairs a lock text records, or `None` when it does not parse as one.
/// 锁文本记录的 `名字 版本` 对；读不成锁时返回 `None`。
fn units(text: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut source: Option<String> = None;
    let mut path: Option<String> = None;
    let mut surface: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line == "[[unit]]" {
            flush(
                &mut out,
                &mut name,
                &mut version,
                &mut source,
                &mut path,
                &mut surface,
            )?;
        } else if let Some(value) = quoted(line, "name = ") {
            name = Some(value);
        } else if let Some(value) = quoted(line, "version = ") {
            version = Some(value);
        } else if let Some(value) = quoted(line, "source = ") {
            source = Some(value);
        } else if let Some(value) = quoted(line, "path = ") {
            path = Some(value);
        } else if let Some(value) = quoted(line, "export = ") {
            surface.push(format!("export {value}"));
        } else if let Some(value) = quoted(line, "test = ") {
            surface.push(format!("test {value}"));
        }
    }
    flush(
        &mut out,
        &mut name,
        &mut version,
        &mut source,
        &mut path,
        &mut surface,
    )?;
    Some(out)
}

/// Push the unit gathered so far, with where it came from.
/// 把已收集的单元连同来源压入列表。
fn flush(
    out: &mut Vec<String>,
    name: &mut Option<String>,
    version: &mut Option<String>,
    source: &mut Option<String>,
    path: &mut Option<String>,
    surface: &mut Vec<String>,
) -> Option<()> {
    if let Some(name) = name.take() {
        let version = version.take()?;
        // The path is part of a unit's identity: two units may share a name and a version and
        // still be different units, so a lock that compared only name and version would let one
        // stand in for the other under `--locked`.
        // 路径是单元身份的一部分：两个单元可以同名同版本而仍是不同单元，只比较名字与版本的锁会让
        // 其中一个在 `--locked` 下冒充另一个。
        let path = path.take().unwrap_or_default();
        let origin = source
            .take()
            .map(|source| format!(" (source {source}, path {path})"))
            .unwrap_or_else(|| format!(" (path {path})"));
        let surface = std::mem::take(surface).join("; ");
        out.push(format!("{name} {version}{origin} [{surface}]"));
    }
    Some(())
}

/// The quoted value of `prefix`, when the line has one.
/// 行内 `prefix` 的引号值（若有）。
fn quoted(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
