//! The install log: an append-only record of every state change an install prefix went through,
//! chained by hashes, plus a `xirang log verify` that recomputes the chain and compares it with the
//! generations still on disk. A line is written only after the change it records has happened, so
//! the log never predicts the future; what it can do is tell an edit, a deletion or a gap in the
//! middle from the history that produced what is under the prefix today.
//! 安装日志：一份只追加的、按哈希链起来的记录，写着某个安装前缀经历过的每一次状态变化；外加
//! `xirang log verify`，它重算这条链，并与磁盘上仍在的世代对照。每一行都在它所记录的变化**已经发生**
//! 之后才写，因此日志从不预言未来；它能做的是把「中段被改过、被删过或有缺口」与「前缀里今天这份历史
//! 就是由此产生的」区分开。
//!
//! The boundary is written down rather than implied: everything the chain is checked against lives
//! under the same prefix, so a rewrite of the whole prefix - log and records together - is not
//! detectable here. Detecting partial edits is what makes this useful: a single edited field breaks
//! one digest, a deleted generation breaks a record, a deleted line breaks the link after it.
//! 边界是写下来的，不是暗示的：链被对照的一切都住在同一个前缀下，因此「把整个前缀一起重写」在这里
//! 检测不出来。让这份日志有用的是检测局部改动：改一个字段会破坏一处摘要，删一个世代会破坏一处记录，
//! 删一行会破坏它后面那一环。

use std::path::Path;

use serde_json::{Map, Value};

/// The append-only log, one JSON object per line.
/// 只追加的日志，每行一个 JSON 对象。
pub(crate) const LOG: &str = "log.jsonl";

/// The record a generation carries: which log entry made it current.
/// 世代自带的记录：是日志里的哪一条使它成为当前。
pub(crate) const RECORD: &str = "provenance.json";

/// The chain starts at a fixed value so the first line has something to point at.
/// 链从一个固定值开始，于是第一行也有东西可指。
pub(crate) fn genesis() -> String {
    "0".repeat(64)
}

/// One recorded state change.
/// 一次被记录下来的状态变化。
#[derive(Debug)]
pub(crate) struct Entry {
    /// Position in the chain, counted from one.
    /// 链上的位置，从 1 开始。
    pub(crate) seq: u64,
    /// Seconds since the Unix epoch when the change was recorded.
    /// 记录这次变化时距 Unix 纪元的秒数。
    pub(crate) at: u64,
    /// `install` or `rollback`.
    /// `install` 或 `rollback`。
    pub(crate) action: String,
    /// The generation the change made current.
    /// 这次变化设为当前的世代。
    pub(crate) generation: String,
    /// The sha256 of the package manifest the generation came from; `None` for a rollback.
    /// 该世代来自的包清单的 sha256；回退时为 `None`。
    pub(crate) package: Option<String>,
    /// The digest of the entry before this one, or [`genesis`].
    /// 上一条的摘要，或 [`genesis`]。
    pub(crate) previous: String,
    /// This entry's own digest.
    /// 本条自己的摘要。
    pub(crate) entry: String,
}

impl Entry {
    /// The digest these fields produce. Recomputed by [`verify`] against the recorded one, so an
    /// edit to any field is reported rather than believed.
    /// 这些字段算出的摘要。[`verify`] 会重算它并与记录比对，因此任何字段被改动都会被点名而不是被相信。
    fn computed(&self) -> String {
        let mut fields = Map::new();
        fields.insert("protocol".to_owned(), Value::from(1u64));
        fields.insert("seq".to_owned(), Value::from(self.seq));
        fields.insert("at".to_owned(), Value::from(self.at));
        fields.insert("action".to_owned(), Value::from(self.action.clone()));
        fields.insert(
            "generation".to_owned(),
            Value::from(self.generation.clone()),
        );
        fields.insert(
            "package".to_owned(),
            match &self.package {
                Some(package) => Value::from(package.clone()),
                None => Value::Null,
            },
        );
        fields.insert("previous".to_owned(), Value::from(self.previous.clone()));
        digest(&fields)
    }

    /// The line this entry is written as, `entry` last so the digest covers every field before it.
    /// 这条记录写成的行；`entry` 放在最后，于是摘要覆盖它之前的每一个字段。
    fn line(&self) -> String {
        let mut fields = Map::new();
        fields.insert("protocol".to_owned(), Value::from(1u64));
        fields.insert("seq".to_owned(), Value::from(self.seq));
        fields.insert("at".to_owned(), Value::from(self.at));
        fields.insert("action".to_owned(), Value::from(self.action.clone()));
        fields.insert(
            "generation".to_owned(),
            Value::from(self.generation.clone()),
        );
        fields.insert(
            "package".to_owned(),
            match &self.package {
                Some(package) => Value::from(package.clone()),
                None => Value::Null,
            },
        );
        fields.insert("previous".to_owned(), Value::from(self.previous.clone()));
        fields.insert("entry".to_owned(), Value::from(self.entry.clone()));
        Value::Object(fields).to_string()
    }
}

/// The digest of a field map: the same canonical bytes [`Entry::line`] writes and [`verify`] reads.
/// 字段映射的摘要：与 [`Entry::line`] 写、[`verify`] 读的是同一份规范字节。
fn digest(fields: &Map<String, Value>) -> String {
    xirang_kernel::sha256_hex(Value::Object(fields.clone()).to_string().as_bytes())
}

/// Parse one line, refusing an unknown key by name. A line carries exactly the fields the digest
/// covers plus `entry`; a key nobody wrote is how a reader learns the file is not this protocol.
/// 解析一行，未知键按名拒绝。一行的字段恰好是摘要覆盖的那些再加 `entry`；出现没人写过的键，正是读取方
/// 得知「这份文件不是本协议」的方式。
fn parse_line(line: &str, number: usize) -> Result<Entry, String> {
    let value: Value = serde_json::from_str(line)
        .map_err(|error| format!("line {number} is not JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("line {number} is not a JSON object"))?;
    let known = [
        "protocol",
        "seq",
        "at",
        "action",
        "generation",
        "package",
        "previous",
        "entry",
    ];
    for key in object.keys() {
        if !known.contains(&key.as_str()) {
            return Err(format!(
                "line {number} has an unknown key `{key}`; a log entry carries {known:?}"
            ));
        }
    }
    if object.get("protocol") != Some(&Value::from(1u64)) {
        return Err(format!(
            "line {number} speaks log protocol {}; this build reads protocol 1",
            object.get("protocol").unwrap_or(&Value::Null)
        ));
    }
    let field = |name: &str| {
        object
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("line {number} has no string `{name}`"))
    };
    let seq = object
        .get("seq")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("line {number} has no `seq`"))?;
    let at = object
        .get("at")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("line {number} has no `at`"))?;
    let action = field("action")?;
    if action != "install" && action != "rollback" {
        return Err(format!(
            "line {number} records action `{action}`; a state change is `install` or `rollback`"
        ));
    }
    let package = match object.get("package") {
        Some(Value::Null) => None,
        Some(Value::String(package)) => Some(package.clone()),
        _ => return Err(format!("line {number} has no `package` (a digest or null)")),
    };
    Ok(Entry {
        seq,
        at,
        action,
        generation: field("generation")?,
        package,
        previous: field("previous")?,
        entry: field("entry")?,
    })
}

/// Read the log and check the chain itself; the first defect is returned, because a link that does
/// not follow from the one before it makes every later entry unreadable as history.
/// 读日志并检查链本身；第一处缺陷即返回，因为「不承接前一条」的那一环会让之后每一条都无法再被当作
/// 历史来读。
pub(crate) fn chain(prefix: &Path) -> Result<Vec<Entry>, String> {
    let path = prefix.join(LOG);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("no readable `{}`: {error}", path.display())),
    };
    let mut entries: Vec<Entry> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let entry = parse_line(line, index + 1)?;
        let expected = index as u64 + 1;
        if entry.seq != expected {
            return Err(format!(
                "line {} records seq {}, but the log's lines are numbered from 1 without gaps; entries cannot be inserted or removed",
                index + 1,
                entry.seq
            ));
        }
        let previous = match entries.last() {
            Some(before) => before.entry.clone(),
            None => genesis(),
        };
        if entry.previous != previous {
            return Err(format!(
                "line {} names {} as the entry before it, but the entry before it is {}; the chain is broken there",
                index + 1,
                entry.previous,
                previous
            ));
        }
        let computed = entry.computed();
        if computed != entry.entry {
            return Err(format!(
                "line {} says its entry is {}, but its fields hash to {}; the line was edited after it was written",
                index + 1,
                entry.entry,
                computed
            ));
        }
        entries.push(entry);
    }
    Ok(entries)
}

/// Append one state change and describe it in the generation it names. Refuses to append to a chain
/// that does not verify, so a broken history is reported instead of extended.
/// 追加一次状态变化，并在它点名的世代里描述它。链本身不通过就拒绝追加，于是坏掉的历史会被报告，
/// 而不是被续写。
pub(crate) fn record(
    prefix: &Path,
    action: &str,
    generation: &str,
    package: Option<&str>,
) -> Result<Entry, String> {
    let entries = chain(prefix)?;
    let entry = Entry {
        seq: entries.len() as u64 + 1,
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0),
        action: action.to_owned(),
        generation: generation.to_owned(),
        package: package.map(str::to_owned),
        previous: match entries.last() {
            Some(before) => before.entry.clone(),
            None => genesis(),
        },
        entry: String::new(),
    };
    let computed = entry.computed();
    let entry = Entry {
        entry: computed,
        ..entry
    };
    let directory = prefix.join(generation);
    if directory.is_dir() {
        let record = serde_json::json!({
            "protocol": 1,
            "generation": generation,
            "seq": entry.seq,
            "entry": entry.entry,
            "package": entry.package,
        });
        let text = serde_json::to_string_pretty(&record)
            .map_err(|error| format!("cannot render the provenance record: {error}"))?;
        let path = directory.join(RECORD);
        let temporary = directory.join(format!("{RECORD}.tmp"));
        std::fs::write(&temporary, text)
            .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
        std::fs::rename(&temporary, &path)
            .map_err(|error| format!("cannot replace {}: {error}", path.display()))?;
    }
    let mut text = std::fs::read_to_string(prefix.join(LOG)).unwrap_or_default();
    text.push_str(&entry.line());
    text.push('\n');
    let path = prefix.join(LOG);
    let temporary = prefix.join(format!("{LOG}.tmp"));
    std::fs::write(&temporary, text)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))?;
    Ok(entry)
}

/// The generations under a prefix: directories that carry a manifest.
/// 前缀下的世代：带清单的目录。
fn generations(prefix: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(prefix) else {
        return found;
    };
    for entry in entries.flatten() {
        // `file_type` does not follow links: `current` is a symlink to a generation, so a reader
        // that followed it would count the same generation twice and call it `current`.
        // `file_type` 不跟随链接：`current` 是指向世代的符号链接，跟随它的读者会把同一个世代数两遍，
        // 还把它叫作 `current`。
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        let path = entry.path();
        if let (true, Some(name)) = (
            path.join("manifest.json").is_file(),
            entry.file_name().to_str(),
        ) {
            found.push(name.to_owned());
        }
    }
    found.sort();
    found
}

/// What a generation's own record says, when it is readable.
/// 一个世代自己的记录说了什么（能读时）。
fn record_of(directory: &Path) -> Result<(String, String), String> {
    let path = directory.join(RECORD);
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "carries no readable `{RECORD}` ({error}); nothing says which log entry made it current"
        )
    })?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("has an unreadable `{RECORD}`: {error}"))?;
    if value["protocol"] != 1 {
        return Err(format!(
            "has a `{RECORD}` speaking protocol {}; this build reads protocol 1",
            value["protocol"]
        ));
    }
    let generation = value["generation"]
        .as_str()
        .ok_or_else(|| format!("has a `{RECORD}` with no `generation`"))?;
    let entry = value["entry"]
        .as_str()
        .ok_or_else(|| format!("has a `{RECORD}` with no `entry`"))?;
    Ok((generation.to_owned(), entry.to_owned()))
}

/// Report on a prefix: the chain, the generations that carry a record, and whether `current` and
/// the history file agree with the last recorded state change. Returns the report and whether it
/// verified; the report ends with the one step a reader can take when it did not.
/// 报告一个前缀：链、带记录的世代、以及 `current` 与历史文件是否与最后那次被记录的状态变化一致。
/// 返回报告与它是否通过；没通过时报告以读者能采取的唯一下一步结尾。
pub(crate) fn verify(prefix: &Path, json: bool) -> Result<(String, bool), String> {
    let path = prefix.join(LOG);
    let mut problems: Vec<String> = Vec::new();
    let entries = match chain(prefix) {
        Ok(entries) => entries,
        Err(error) => {
            problems.push(error);
            Vec::new()
        }
    };
    let on_disk = generations(prefix);
    if !problems.is_empty() {
        return Ok((
            report(prefix, &path, &entries, &on_disk, &problems, json),
            false,
        ));
    }
    let mut recorded: Vec<String> = entries
        .iter()
        .map(|entry| entry.generation.clone())
        .collect();
    recorded.sort();
    recorded.dedup();
    for generation in &on_disk {
        if !recorded.contains(generation) {
            problems.push(format!(
                "generation {generation} is under the prefix, but no line of the log records it becoming current; its provenance is unknown"
            ));
        }
        let directory = prefix.join(generation);
        match record_of(&directory) {
            Ok((named, entry)) => {
                if &named != generation {
                    problems.push(format!(
                        "generation {generation} carries a record that names generation {named}"
                    ));
                }
                let matches = entries
                    .iter()
                    .any(|known| known.entry == entry && &known.generation == generation);
                if !matches {
                    problems.push(format!(
                        "generation {generation} names entry {entry}, which is not an entry of the log for this generation; the record and the log do not describe the same install"
                    ));
                }
            }
            Err(error) => problems.push(format!("generation {generation} {error}")),
        }
    }
    for generation in &recorded {
        if !on_disk.contains(generation) {
            problems.push(format!(
                "the log records a state change making generation {generation} current, but that generation is no longer under the prefix; recorded generations are not deleted"
            ));
        }
    }
    let last = entries.last();
    let link = std::fs::read_link(prefix.join("current")).ok();
    match (&link, last) {
        (Some(target), Some(entry)) => {
            let target = target.display().to_string();
            if target != entry.generation {
                problems.push(format!(
                    "the `current` link points at {target}, but the last recorded state change made {} current",
                    entry.generation
                ));
            }
        }
        (None, Some(entry)) => problems.push(format!(
            "there is no readable `current` link, but the last recorded state change made {} current",
            entry.generation
        )),
        (Some(target), None) => problems.push(format!(
            "the `current` link points at {}, but the log records no state change",
            target.display()
        )),
        (None, None) => {}
    }
    let history = std::fs::read_to_string(prefix.join("history.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    match (&history, last) {
        (Some(history), Some(entry)) => {
            let current = history["current"].as_str();
            if current != Some(entry.generation.as_str()) {
                problems.push(format!(
                    "`history.json` says the current generation is {}, but the last recorded state change made {} current",
                    current.unwrap_or("<none>"),
                    entry.generation
                ));
            }
            let expected_previous = if entries.len() >= 2 {
                entries[entries.len() - 2].generation.as_str()
            } else {
                ""
            };
            let previous = history["previous"].as_str().unwrap_or("");
            if previous != expected_previous {
                problems.push(format!(
                    "`history.json` says the previous generation is {}, but the state change before the last one made {} current",
                    if previous.is_empty() { "<none>" } else { previous },
                    if expected_previous.is_empty() {
                        "<none>"
                    } else {
                        expected_previous
                    }
                ));
            }
        }
        (None, Some(entry)) => problems.push(format!(
            "there is no readable `history.json`, but the last recorded state change made {} current",
            entry.generation
        )),
        (history, None) => {
            if history.is_some() || !on_disk.is_empty() {
                problems.push(
                    "a prefix holds a history or generations, but the log records no state change; nothing says how they got here".to_owned(),
                );
            }
        }
    }
    let verified = problems.is_empty();
    Ok((
        report(prefix, &path, &entries, &on_disk, &problems, json),
        verified,
    ))
}

/// Render the report a reader acts on: what was checked, what disagreed, and the one step left.
/// 渲染读者能据以行动的报告：查了什么、哪里不一致、剩下的唯一步骤。
fn report(
    prefix: &Path,
    path: &Path,
    entries: &[Entry],
    on_disk: &[String],
    problems: &[String],
    json: bool,
) -> String {
    let head = match entries.last() {
        Some(entry) => entry.entry.clone(),
        None => genesis(),
    };
    let scope = "this compares the log with the generations under the same prefix; a rewrite of the whole prefix, log and records together, is not detectable from here because nothing outside it anchors the head";
    if json {
        let value = serde_json::json!({
            "prefix": prefix.display().to_string(),
            "log": path.display().to_string(),
            "entries": entries.len(),
            "head": head,
            "generations": on_disk,
            "current": entries.last().map(|entry| entry.generation.clone()),
            "ok": problems.is_empty(),
            "problems": problems,
            "scope": scope,
        });
        return serde_json::to_string_pretty(&value)
            .unwrap_or_else(|error| format!("cannot render the report: {error}"));
    }
    let mut out = format!("prefix {}\n", prefix.display());
    match entries.last() {
        Some(_) => out.push_str(&format!(
            "  log {}: {} recorded change(s), chain intact, head {}\n",
            path.display(),
            entries.len(),
            head
        )),
        None if path.is_file() => out.push_str(&format!(
            "  log {}: empty; no state change has been recorded\n",
            path.display()
        )),
        None => out.push_str(&format!(
            "  log {}: absent; no state change has been recorded\n",
            path.display()
        )),
    }
    let _ = entry_note(&mut out, entries, on_disk);
    if problems.is_empty() {
        out.push_str(&format!("  scope: {scope}\n"));
        return out.trim_end().to_owned();
    }
    out.push_str("  problems:\n");
    for problem in problems {
        out.push_str(&format!("    - {problem}\n"));
    }
    out.push_str(&format!(
        "  next: the log is append-only, so nothing here edits it - rebuild the prefix from verified packages (xirang install <package> --prefix {}) or restore the log from a copy, then verify again\n",
        prefix.display()
    ));
    out.trim_end().to_owned()
}

/// The generations line: every generation under the prefix is described by a line of the log.
/// 世代那一行：前缀下的每个世代都有日志中的一行在描述它。
fn entry_note(out: &mut String, entries: &[Entry], on_disk: &[String]) -> Option<()> {
    let recorded: Vec<&String> = entries.iter().map(|entry| &entry.generation).collect();
    let unrecorded = on_disk
        .iter()
        .filter(|generation| !recorded.contains(generation))
        .count();
    if on_disk.is_empty() {
        out.push_str("  generations: none\n");
    } else if unrecorded == 0 {
        out.push_str(&format!(
            "  generations: {} under the prefix, every one recorded\n",
            on_disk.len()
        ));
    } else {
        out.push_str(&format!(
            "  generations: {} under the prefix, {unrecorded} not recorded\n",
            on_disk.len()
        ));
    }
    if let Some(last) = entries.last() {
        out.push_str(&format!(
            "  current: {} (the last recorded state change)\n",
            last.generation
        ));
    }
    Some(())
}
