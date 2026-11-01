//! The backend record a checkout may keep beside the built-in backends.
//! 检出可以在内置后端之外保留一份后端记录。
//!
//! A record is a declaration, not an executor: it says which language a backend answers for and
//! which abilities it claims, so `xirang languages` can report it and a refusal can name it. No
//! record can replace a built-in backend, and recording one does not add a build executor — the
//! report says `declared` for exactly that reason, and `xirang build` still refuses a project
//! whose language has no executor.
//! 记录是声明而不是执行器：它说明后端负责哪门语言、声明哪些能力，使 `xirang languages` 能报告它、
//! 拒绝语能点名它。任何记录都不能替换内置后端，记录一个后端也不会新增构建执行器——报告里写
//! `declared` 正是这个意思，而 `xirang build` 仍会拒绝一门没有执行器的语言的项目。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use xirang_kernel::interchange::{Capability, LanguageId};

/// The record file a checkout may keep at its root.
/// 检出可以放在根部的记录文件。
pub(super) const FILE: &str = "xirang.backends.toml";

/// The record protocol this build understands.
/// 本构建理解的记录协议。
pub(super) const PROTOCOL: u32 = 1;

/// One backend a record declares.
/// 记录声明的一个后端。
#[derive(Debug)]
pub(super) struct Entry {
    /// The language the backend answers for.
    /// 该后端负责的语言。
    pub language: String,
    /// The programs that would carry the backend out.
    /// 将要执行该后端的程序。
    pub backend: String,
    /// The argv that makes the entry executable, when the record gives one.
    /// 让条目可执行的 argv（在记录给出时）。
    pub command: Option<Vec<String>>,
    /// The abilities the record claims.
    /// 记录声明具备的能力。
    pub capabilities: BTreeSet<Capability>,
    /// Why one ability is not claimed, by ability word.
    /// 某项能力为何没有被声明，按能力词索引。
    pub reasons: BTreeMap<String, String>,
    /// The line the entry starts on, for refusals.
    /// 条目起始行号，供拒绝语点名。
    pub line: usize,
    /// Whether the actions this backend plans run inside a write domain.
    /// 该后端计划的动作是否在一个写域里运行。
    pub sandbox: bool,
    /// Whether the backend process itself runs inside a write domain.
    /// 后端进程本身是否在一个写域里运行。
    pub planner_sandbox: bool,
}

/// Everything a record declares.
/// 记录声明的全部内容。
#[derive(Debug, Default)]
pub(super) struct Record {
    /// The declared backends, in file order.
    /// 按文件顺序声明的后端。
    pub entries: Vec<Entry>,
}

/// Read the record under `root`; a checkout without one has an empty record.
/// 读取 `root` 下的记录；没有记录的检出得到空记录。
/// The record file a user-level installation writes: `$XIRANG_BACKENDS` when it is set, else
/// `$XDG_CONFIG_HOME/xirang/<FILE>`, else `$HOME/.config/xirang/<FILE>`.
/// 用户级安装写入的记录文件：设置了 `$XIRANG_BACKENDS` 就用它，否则 `$XDG_CONFIG_HOME/xirang/<FILE>`，
/// 再否则 `$HOME/.config/xirang/<FILE>`。
pub(super) fn user_path() -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("XIRANG_BACKENDS") {
        return Some(PathBuf::from(named));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|base| base.join("xirang").join(FILE))
}

/// Every record that applies to this checkout: the project's own first, then the user's.
/// 适用于本检出的每一份记录：先是项目自己的，再是用户级的。
pub(super) fn discover(root: &Path, builtin: &[&str]) -> Result<Vec<(PathBuf, Record)>, String> {
    let mut paths = vec![root.join(FILE)];
    if let Some(user) = user_path() {
        paths.push(user);
    }
    let mut sources = Vec::new();
    for path in paths {
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        sources.push((path, parse(&text, builtin)?));
    }
    Ok(sources)
}

/// Refuse two records that answer for one language: a report cannot say which of them is real.
/// 拒绝两份记录为同一门语言作答：报告无法说明哪一份是真的。
pub(super) fn conflicts(sources: &[(PathBuf, Record)]) -> Result<(), String> {
    let mut seen: BTreeMap<&str, &Path> = BTreeMap::new();
    for (path, record) in sources {
        for entry in &record.entries {
            if let Some(first) = seen.get(entry.language.as_str()) {
                return Err(format!(
                    "`{}` is answered by two recorded backends: {} and {}; one checkout may record \
                     one answer per language",
                    entry.language,
                    first.display(),
                    path.display()
                ));
            }
            seen.insert(entry.language.as_str(), path.as_path());
        }
    }
    Ok(())
}

/// A backend an installation is about to record.
/// 一次安装即将记录的后端。
pub(super) struct Draft {
    /// The language the backend answers for.
    /// 该后端负责的语言。
    pub language: String,
    /// The program that carries it out.
    /// 执行它的程序。
    pub backend: String,
    /// The argv a delegated plan is sent to.
    /// 委派计划会发送到的 argv。
    pub command: Vec<String>,
    /// The abilities the installation claims, by word.
    /// 安装声明具备的能力，按能力词给出。
    pub capabilities: Vec<String>,
    /// Why an ability is not claimed, by word.
    /// 某项能力为何没有声明，按能力词给出。
    pub reasons: Vec<(String, String)>,
}

/// Check one installation draft against the record rules and build the entry to probe.
/// 按记录规则检查一份安装草稿，并构造要探测的条目。
pub(super) fn draft(draft: &Draft) -> Result<Entry, String> {
    if draft.command.is_empty() {
        return Err(format!(
            "`{}` needs a `command`: a record without a process only declares, and an \
             installation that declares nothing cannot be asked anything",
            draft.language
        ));
    }
    let mut capabilities = BTreeSet::new();
    for word in &draft.capabilities {
        let Some(capability) = Capability::parse_ability(word) else {
            return Err(format!(
                "`{word}` is not an ability word; the words are {}",
                capability_words()
            ));
        };
        capabilities.insert(capability);
    }
    if !capabilities.contains(&Capability::BuildTest) {
        return Err(format!(
            "`{}` records a process but does not claim `build-test`; every recorded process is \
             asked for a plan, so an entry without that ability would be installed dead",
            draft.language
        ));
    }
    let mut reasons = BTreeMap::new();
    for (word, text) in &draft.reasons {
        let Some(capability) = Capability::parse_ability(word) else {
            return Err(format!(
                "`reason.{word}` does not name a capability; the words are {}",
                capability_words()
            ));
        };
        if capabilities.contains(&capability) {
            return Err(format!(
                "`reason.{word}` explains a capability the entry already claims; a claim and a \
                 reason for it contradict each other"
            ));
        }
        if text.is_empty() {
            return Err(format!("`reason.{word}` needs a non-empty reason"));
        }
        reasons.insert(capability.name().to_owned(), text.clone());
    }
    Ok(Entry {
        language: draft.language.clone(),
        backend: draft.backend.clone(),
        command: Some(draft.command.clone()),
        capabilities,
        reasons,
        line: 0,
        sandbox: false,
        planner_sandbox: false,
    })
}

/// Append one entry to the user-level record, refusing anything the reader would refuse.
/// 把一条条目追加进用户级记录；读者会拒绝的内容，这里也拒绝。
/// What one installation wrote, and which backend it replaced (an upgrade).
/// 一次安装写了什么，以及它替换了哪个后端（升级时）。
pub(super) struct Installed {
    /// The user record that now holds the entry.
    /// 现在持有该条目的用户记录。
    pub path: PathBuf,
    /// The backend the entry replaced, when it was an upgrade.
    /// 该条目替换掉的后端（升级时）。
    pub replaced: Option<String>,
}

pub(super) fn append(entry: Entry) -> Result<Installed, String> {
    let path = user_path().ok_or_else(|| {
        "no user record path: set XIRANG_BACKENDS, XDG_CONFIG_HOME, or HOME".to_owned()
    })?;
    let builtin = super::backends::registered_languages();
    let mut record = Record::default();
    if path.is_file() {
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        record = parse(&text, &builtin)?;
    }
    let replaced = record
        .entries
        .iter()
        .position(|existing| existing.language == entry.language)
        .map(|index| record.entries.remove(index).backend);
    record.entries.push(entry);
    let rendered = render(&record);
    // The write is only useful if it reads back: parse it before touching the disk.
    parse(&rendered, &builtin)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, &rendered)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))?;
    Ok(Installed { path, replaced })
}

/// Remove one language's entry from the user record as one transaction, and say what was removed.
/// 从用户记录里把一门语言的条目作为一次事务移除，并说明移除了什么。
pub(super) fn remove(language: &str) -> Result<(PathBuf, String, usize), String> {
    let path = user_path().ok_or_else(|| {
        "no user record path: set XIRANG_BACKENDS, XDG_CONFIG_HOME, or HOME".to_owned()
    })?;
    if !path.is_file() {
        return Err(format!(
            "there is no user record at {}; nothing records `{language}`",
            path.display()
        ));
    }
    let builtin = super::backends::registered_languages();
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut record = parse(&text, &builtin)?;
    let Some(index) = record
        .entries
        .iter()
        .position(|entry| entry.language == language)
    else {
        let recorded: Vec<&str> = record
            .entries
            .iter()
            .map(|entry| entry.language.as_str())
            .collect();
        return Err(format!(
            "the user record at {} records no `{language}`; it records {}",
            path.display(),
            if recorded.is_empty() {
                "nothing".to_owned()
            } else {
                format!("`{}`", recorded.join("`, `"))
            }
        ));
    };
    let removed = record.entries.remove(index);
    let rendered = render(&record);
    // The write is only useful if it reads back: parse it before touching the disk.
    parse(&rendered, &builtin)?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, &rendered)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))?;
    Ok((path, removed.backend, record.entries.len()))
}

/// Quote one record value.
/// 引用一个记录值。
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(character),
        }
    }
    out.push('"');
    out
}

/// Render a record in the shape its parser reads.
/// 按解析器能读的形状渲染一份记录。
pub(super) fn render(record: &Record) -> String {
    let mut text = format!("protocol = {PROTOCOL}\n");
    for entry in &record.entries {
        text.push_str("\n[[backend]]\n");
        text.push_str(&format!("language = {}\n", quote(&entry.language)));
        text.push_str(&format!("backend = {}\n", quote(&entry.backend)));
        let words: Vec<String> = entry
            .capabilities
            .iter()
            .map(|capability| quote(capability.name()))
            .collect();
        text.push_str(&format!("capabilities = [{}]\n", words.join(", ")));
        if entry.planner_sandbox {
            text.push_str("planner_sandbox = \"true\"\n");
        }
        if let Some(command) = &entry.command {
            let argv: Vec<String> = command.iter().map(|part| quote(part)).collect();
            text.push_str(&format!("command = [{}]\n", argv.join(", ")));
        }
        for (word, reason) in &entry.reasons {
            text.push_str(&format!("reason.{} = {}\n", word, quote(reason)));
        }
    }
    text
}

/// Parse a record, refusing anything this build cannot honor.
/// 解析记录，拒绝本构建无法兑现的任何内容。
pub(super) fn parse(text: &str, builtin: &[&str]) -> Result<Record, String> {
    let mut record = Record::default();
    let mut protocol: Option<u32> = None;
    let mut current: Option<Entry> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "[[backend]]" {
            if let Some(entry) = current.take() {
                record.entries.push(entry);
            }
            current = Some(Entry {
                language: String::new(),
                backend: String::new(),
                command: None,
                capabilities: BTreeSet::new(),
                reasons: BTreeMap::new(),
                line,
                sandbox: false,
                planner_sandbox: false,
            });
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!(
                "{FILE}:{line}: expected `key = value` or `[[backend]]`, found `{trimmed}`"
            ));
        };
        let (key, value) = (key.trim(), value.trim());
        if key == "protocol" {
            if protocol.is_some() {
                return Err(format!(
                    "{FILE}:{line}: `protocol` is declared more than once"
                ));
            }
            if current.is_some() {
                return Err(format!(
                    "{FILE}:{line}: `protocol` must come before the first `[[backend]]`"
                ));
            }
            let version = value.parse::<u32>().map_err(|_| {
                format!("{FILE}:{line}: `protocol` needs a whole number, found `{value}`")
            })?;
            if version != PROTOCOL {
                return Err(format!(
                    "{FILE}:{line}: record protocol {version} is not the protocol this build \
                     understands ({PROTOCOL})"
                ));
            }
            protocol = Some(version);
            continue;
        }
        let Some(entry) = current.as_mut() else {
            return Err(format!(
                "{FILE}:{line}: `{key}` appears before any `[[backend]]`"
            ));
        };
        if let Some(reason_for) = key.strip_prefix("reason.") {
            let Some(capability) = Capability::parse_ability(reason_for) else {
                return Err(format!(
                    "{FILE}:{line}: `reason.{reason_for}` does not name a capability; the words \
                     are {}",
                    capability_words()
                ));
            };
            let text = quoted(value, line, key)?;
            if text.is_empty() {
                return Err(format!("{FILE}:{line}: `{key}` needs a non-empty reason"));
            }
            if entry
                .reasons
                .insert(capability.name().to_owned(), text)
                .is_some()
            {
                return Err(format!(
                    "{FILE}:{line}: `{key}` explains the same capability twice"
                ));
            }
            continue;
        }
        match key {
            "language" => entry.language = quoted(value, line, key)?,
            "backend" => entry.backend = quoted(value, line, key)?,
            "command" => entry.command = Some(strings(value, line, key)?),
            "sandbox" => {
                entry.sandbox = match quoted(value, line, key)?.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(format!(
                            "{FILE}:{line}: `sandbox` takes `\"true\"` or `\"false\"`"
                        ));
                    }
                };
            }
            "capabilities" => {
                entry.capabilities = words(value, line, key)?;
            }
            "planner_sandbox" => {
                entry.planner_sandbox = match quoted(value, line, key)?.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(format!(
                            "{FILE}:{line}: `planner_sandbox` takes `\"true\"` or `\"false\"`"
                        ));
                    }
                };
            }
            _ => {
                return Err(format!(
                    "{FILE}:{line}: unknown key `{key}`; a backend names `language`, `backend`, `command`, \
                     `sandbox`, `planner_sandbox`, `capabilities` or `reason.<capability>`"
                ));
            }
        }
    }
    if let Some(entry) = current.take() {
        record.entries.push(entry);
    }
    let Some(_) = protocol else {
        return Err(format!(
            "{FILE}: no `protocol` line; this build understands protocol {PROTOCOL}"
        ));
    };
    for entry in &record.entries {
        validate_entry(entry, builtin)?;
    }
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for entry in &record.entries {
        if !seen.insert((entry.language.clone(), entry.backend.clone())) {
            return Err(format!(
                "{FILE}:{}: `{}` via `{}` is declared more than once",
                entry.line, entry.language, entry.backend
            ));
        }
    }
    Ok(record)
}

/// Check one entry against the build's rules and the built-in backends.
/// 按本构建的规则与内置后端检查一个条目。
fn validate_entry(entry: &Entry, builtin: &[&str]) -> Result<(), String> {
    let line = entry.line;
    if entry.language.is_empty() {
        return Err(format!("{FILE}:{line}: the entry names no `language`"));
    }
    if LanguageId::parse_language(&entry.language).is_none() {
        return Err(format!(
            "{FILE}:{line}: `{}` is not a language identity (lowercase ascii letters, digits, \
             `-` and `_`)",
            entry.language
        ));
    }
    if entry.backend.is_empty() {
        return Err(format!("{FILE}:{line}: the entry names no `backend`"));
    }
    if let Some(command) = &entry.command {
        if command.is_empty() {
            return Err(format!(
                "{FILE}:{line}: `command` names no program; an empty argv is not an executor"
            ));
        }
        if !entry.capabilities.contains(&Capability::BuildTest) {
            return Err(format!(
                "{FILE}:{line}: `{}` names a `command` but does not claim `build-test`; an \
                 executor that cannot build or test has nothing to run",
                entry.language
            ));
        }
    }
    if entry.capabilities.is_empty() {
        return Err(format!(
            "{FILE}:{line}: `{}` declares no capability; a record that declares nothing is not a \
             record",
            entry.language
        ));
    }
    if builtin.contains(&entry.language.as_str()) {
        return Err(format!(
            "{FILE}:{line}: `{}` already has a built-in backend; a record cannot replace it",
            entry.language
        ));
    }
    for capability in entry.reasons.keys() {
        if entry
            .capabilities
            .iter()
            .any(|declared| declared.name() == capability)
        {
            return Err(format!(
                "{FILE}:{line}: `reason.{capability}` explains an ability the entry claims; a \
                 reason says why an ability is *not* claimed"
            ));
        }
    }
    Ok(())
}

/// The ability words, comma separated, for a refusal.
/// 全部能力词，逗号分隔，供拒绝语使用。
fn capability_words() -> String {
    Capability::ALL
        .iter()
        .map(|capability| capability.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// One quoted value.
/// 一个带引号的值。
fn quoted(text: &str, line: usize, key: &str) -> Result<String, String> {
    let Some(inner) = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return Err(format!(
            "{FILE}:{line}: `{key}` needs a double-quoted value, found `{text}`"
        ));
    };
    if inner.contains('"') {
        return Err(format!(
            "{FILE}:{line}: `{key}` contains an unescaped quote; this record has no escapes"
        ));
    }
    Ok(inner.to_owned())
}

/// A quoted, comma separated array of ability words.
/// 带引号、逗号分隔的能力词数组。
fn words(text: &str, line: usize, key: &str) -> Result<BTreeSet<Capability>, String> {
    let Some(inner) = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return Err(format!(
            "{FILE}:{line}: `{key}` needs an array of quoted words, found `{text}`"
        ));
    };
    let mut words = BTreeSet::new();
    for word in inner.split(',') {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        let word = quoted(word, line, key)?;
        let Some(capability) = Capability::parse_ability(&word) else {
            return Err(format!(
                "{FILE}:{line}: `{word}` is not a capability; the words are {}",
                capability_words()
            ));
        };
        if !words.insert(capability) {
            return Err(format!(
                "{FILE}:{line}: `{word}` is listed twice in `{key}`"
            ));
        }
    }
    Ok(words)
}

/// The entry that can actually run the backend for `language`, if the record names one.
/// 记录为 `language` 点名、且真的可以运行后端的那一条；没有时返回 `None`。
#[cfg(test)]
pub(super) fn executable<'a>(record: &'a Record, language: &str) -> Option<&'a Entry> {
    record
        .entries
        .iter()
        .find(|entry| entry.language == language && entry.command.is_some())
}

/// A quoted, comma separated array of strings (a program and its arguments).
/// 带引号、逗号分隔的字符串数组（一个程序及其参数）。
fn strings(text: &str, line: usize, key: &str) -> Result<Vec<String>, String> {
    let Some(inner) = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return Err(format!(
            "{FILE}:{line}: `{key}` needs an array of quoted strings, found `{text}`"
        ));
    };
    let mut values = Vec::new();
    for word in inner.split(',') {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        let word = quoted(word, line, key)?;
        if word.is_empty() {
            return Err(format!("{FILE}:{line}: `{key}` lists an empty string"));
        }
        values.push(word);
    }
    Ok(values)
}

#[cfg(test)]
mod record_tests {
    use super::*;

    const BUILTIN: &[&str] = &["cpp", "c", "python", "rust"];

    fn sample() -> &'static str {
        "protocol = 1\n\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"ruby-backend\"\ncapabilities = [\"discovery\", \"resolution\"]\nreason.build-test = \"no ruby toolchain here\"\n"
    }

    #[test]
    fn a_recorded_backend_is_read_with_its_claims_and_reasons() {
        let record = parse(sample(), BUILTIN).unwrap();
        assert_eq!(record.entries.len(), 1);
        let entry = &record.entries[0];
        assert_eq!(entry.language, "ruby");
        assert_eq!(entry.backend, "ruby-backend");
        assert_eq!(
            entry.capabilities.iter().copied().collect::<Vec<_>>(),
            vec![Capability::Discovery, Capability::Resolution]
        );
        assert_eq!(
            entry.reasons.get("build-test").map(String::as_str),
            Some("no ruby toolchain here")
        );
    }

    #[test]
    fn the_protocol_line_is_required_and_the_version_must_be_understood() {
        let missing = parse("[[backend]]\nlanguage = \"ruby\"\n", BUILTIN).unwrap_err();
        assert!(missing.contains("no `protocol` line"), "{missing}");
        let wrong = parse("protocol = 2\n", BUILTIN).unwrap_err();
        assert!(wrong.contains("protocol 2"), "{wrong}");
    }

    #[test]
    fn an_unknown_ability_is_refused_by_name() {
        let text = "protocol = 1\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"ruby-backend\"\ncapabilities = [\"teleport\"]\n";
        let refused = parse(text, BUILTIN).unwrap_err();
        assert!(
            refused.contains("`teleport` is not a capability"),
            "{refused}"
        );
        assert!(refused.contains("build-test"), "{refused}");
    }

    #[test]
    fn a_record_cannot_replace_a_built_in_backend() {
        let text = "protocol = 1\n[[backend]]\nlanguage = \"cpp\"\nbackend = \"other\"\ncapabilities = [\"discovery\"]\n";
        let refused = parse(text, BUILTIN).unwrap_err();
        assert!(refused.contains("built-in backend"), "{refused}");
    }

    #[test]
    fn a_reason_for_a_claimed_ability_is_a_contradiction() {
        let text = "protocol = 1\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"ruby-backend\"\ncapabilities = [\"discovery\"]\nreason.discovery = \"why?\"\n";
        let refused = parse(text, BUILTIN).unwrap_err();
        assert!(refused.contains("not* claimed"), "{refused}");
    }

    #[test]
    fn two_entries_for_the_same_language_and_backend_are_refused() {
        let text = "protocol = 1\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"r\"\ncapabilities = [\"discovery\"]\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"r\"\ncapabilities = [\"resolution\"]\n";
        let refused = parse(text, BUILTIN).unwrap_err();
        assert!(refused.contains("declared more than once"), "{refused}");
    }

    #[test]
    fn a_command_makes_the_entry_executable_and_needs_build_test() {
        let text = "protocol = 1\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"ruby-backend\"\ncapabilities = [\"build-test\"]\ncommand = [\"python3\", \"backend.py\", \"--strict\"]\n";
        let record = parse(text, BUILTIN).unwrap();
        let entry =
            executable(&record, "ruby").expect("the command did not make the entry executable");
        assert_eq!(
            entry.command.as_deref(),
            Some(
                &[
                    "python3".to_owned(),
                    "backend.py".to_owned(),
                    "--strict".to_owned()
                ][..]
            )
        );
        assert!(
            executable(&record, "go").is_none(),
            "an unknown language became executable"
        );
        let text = "protocol = 1\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"r\"\ncapabilities = [\"discovery\"]\ncommand = [\"python3\"]\n";
        let refused = parse(text, BUILTIN).unwrap_err();
        assert!(refused.contains("does not claim `build-test`"), "{refused}");
    }

    #[test]
    fn an_empty_command_is_refused() {
        let text = "protocol = 1\n[[backend]]\nlanguage = \"ruby\"\nbackend = \"r\"\ncapabilities = [\"build-test\"]\ncommand = []\n";
        let refused = parse(text, BUILTIN).unwrap_err();
        assert!(refused.contains("names no program"), "{refused}");
    }
}
