//! The adoption ledger: routes this tree trusts **provisionally**, and the rule that a decision
//! is only as good as the bytes it was taken on.
//! 采信台账：这棵树**暂时**采信的路线，以及"决定只在它据以做出的那些字节上成立"这条规则。
//!
//! The maintainer's own words for what this is: 「采信是可以改的松动的，但是要强制人去确认可修改，
//! 因为肯定是稳步发展的，只是暂时采信」. So an entry is a **lease** and never a certificate: it
//! names the bytes it was taken on, this module answers whether those bytes still are what they
//! were, and *nothing here* renews or revokes anything by itself. A lapsed entry is a fact to be
//! confirmed by a person, which is why the thing this module refuses to provide is the word
//! "verified".
//! 维护者对这件事的原话：「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定是稳步发展的，只是暂时
//! 采信」。因此条目是**租约**、绝不是证书：它点名自己据以做出的那些字节，本模块回答那些字节是否还是原来的
//! 样子，而**这里没有任何东西**会自行续期或撤销。失效的条目是一个要由人来确认的事实——这也正是本模块拒绝
//! 提供"已验证"这个词的原因。

use std::collections::BTreeMap;

use crate::identity::sha256_hex;

/// One adopted route, as one ledger line spells it.
/// 一条被采信的路线，按台账里的一行书写。
///
/// The line is `anchor|certifies|evidence|verifier|at|files|fingerprint|reason`: what was
/// adopted, what that covers, what it was judged on, who confirmed it, when, the files whose
/// bytes the decision rests on, the fingerprint of those bytes at that moment, and **why** this
/// confirmation was given — the field a reader needs to tell a re-confirmation from a first one.
/// 行格式是 `anchor|certifies|evidence|verifier|at|files|fingerprint`：采信了什么、覆盖什么、据什么
/// 判断、谁确认的、何时、决定依托哪些文件的字节，以及那一刻那些字节的指纹。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdoptionEntry {
    /// The route's anchor: a face identity, a logical path, or any name this tree uses.
    /// 路线的锚点：面身份、逻辑路径，或这棵树使用的任何名字。
    pub anchor: String,
    /// What the decision covers, in the adopter's own words.
    /// 这次决定覆盖什么，用采信者自己的话。
    pub certifies: String,
    /// What it was judged on (a trace run, a converge answer, a record state).
    /// 据什么判断（一次 trace 运行、一份 converge 答案、一种记录状态）。
    pub evidence: String,
    /// Who confirmed it. A person, or the channel a person spoke through.
    /// 谁确认的。一个人，或一个人发话所经的渠道。
    pub verifier: String,
    /// When, as the ledger wrote it.
    /// 何时，按台账写下的样子。
    pub at: String,
    /// The files the decision rests on, relative to the package root.
    /// 决定依托的文件，相对包根。
    pub files: Vec<String>,
    /// The fingerprint of those files' bytes at the moment of adoption.
    /// 采信那一刻那些文件字节的指纹。
    pub fingerprint: String,
    /// Why this confirmation was given, in the confirmer's own words.
    /// 这次确认的理由，用确认者自己的话。
    pub reason: String,
}

/// What is wrong with one ledger line.
/// 台账某一行出了什么问题。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdoptionError {
    /// One-based line number.
    /// 从 1 开始的行号。
    pub line: usize,
    /// What is wrong.
    /// 出了什么问题。
    pub message: String,
}

/// Parse a ledger: one entry per line, `#` comments and blank lines skipped.
/// 解析一份台账：每行一条条目，跳过 `#` 注释与空行。
pub fn parse_adoption(ledger: &str) -> Result<Vec<AdoptionEntry>, AdoptionError> {
    let mut entries = Vec::new();
    for (index, line) in ledger.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        if fields.len() != 8 {
            return Err(AdoptionError {
                line: index + 1,
                message: format!("must contain 8 fields, found {}", fields.len()),
            });
        }
        if fields.iter().any(|field| field.trim().is_empty()) {
            return Err(AdoptionError {
                line: index + 1,
                message: "contains an empty field".to_owned(),
            });
        }
        let files = fields[5]
            .split(',')
            .map(str::trim)
            .filter(|file| !file.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if files.is_empty() {
            return Err(AdoptionError {
                line: index + 1,
                message: "names no file, so the decision rests on nothing".to_owned(),
            });
        }
        entries.push(AdoptionEntry {
            anchor: fields[0].to_owned(),
            certifies: fields[1].to_owned(),
            evidence: fields[2].to_owned(),
            verifier: fields[3].to_owned(),
            at: fields[4].to_owned(),
            files,
            fingerprint: fields[6].to_owned(),
            reason: fields[7].to_owned(),
        });
    }
    Ok(entries)
}

/// The per-file prints a ledger may carry **beside** an entry, so a reader can be told which file
/// moved instead of only that something did.
/// 台账可以在条目**旁边**携带的逐文件指纹，好让读者被告知**哪个文件动了**，而不只是"有东西动了"。
///
/// Why a comment line rather than a ninth field: `parse_adoption` skips `#`, so a ledger carrying
/// these lines is read by an **older** kernel exactly as before — the information is additive in both
/// directions, while a ninth segment would make every older reader reject the whole line. The join key
/// is `(anchor, at)`, which is what identifies an entry anyway.
/// 为什么是注释行而不是第九段：`parse_adoption` 跳过 `#`，因此带着这些行的台账被**更旧**的内核读到时与从前
/// 完全一样——信息在两个方向都是加法；而第九段会让每个更旧的读者**拒绝整行**。连接键是 `(anchor, at)`，
/// 那本来就是标识一条条目的东西。
pub fn parse_adoption_prints(ledger: &str) -> Result<Vec<AdoptionPrints>, AdoptionError> {
    let mut prints = Vec::new();
    for (index, line) in ledger.lines().enumerate() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("# prints ") else {
            continue;
        };
        let parts = rest.split('|').map(str::trim).collect::<Vec<_>>();
        if parts.len() != 3 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(AdoptionError {
                line: index + 1,
                message:
                    "a `# prints` line must read `# prints <anchor> | <at> | <path>=<print>,…`"
                        .to_owned(),
            });
        }
        let mut files = Vec::new();
        for pair in parts[2]
            .split(',')
            .map(str::trim)
            .filter(|pair| !pair.is_empty())
        {
            let Some((path, print)) = pair.rsplit_once('=') else {
                return Err(AdoptionError {
                    line: index + 1,
                    message: format!("`{pair}` is not `<path>=<print>`"),
                });
            };
            files.push((path.trim().to_owned(), print.trim().to_owned()));
        }
        prints.push(AdoptionPrints {
            anchor: parts[0].to_owned(),
            at: parts[1].to_owned(),
            files,
        });
    }
    Ok(prints)
}

/// The line that carries per-file prints, as `parse_adoption_prints` reads it.
/// 携带逐文件指纹的那一行，按 `parse_adoption_prints` 读它的样子。
pub fn render_adoption_prints(anchor: &str, at: &str, files: &[(String, String)]) -> String {
    let pairs = files
        .iter()
        .map(|(path, print)| format!("{path}={print}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("# prints {anchor} | {at} | {pairs}\n")
}

/// The **object** a decision was about, when the anchor named one, carried beside its entry.
/// 一次决定所针对的**对象**（当锚点点名了一个时），跟在它的条目旁边。
///
/// The same additive shape as [`AdoptionPrints`]: a `#` comment line, so an older kernel reads the
/// ledger exactly as before while a newer one can tell that the anchor's object changed instead of
/// silently letting a lease about one object back a commitment about another.
/// 与 [`AdoptionPrints`] 同一种加法形状：一条 `#` 注释行，因此更旧的内核读这份台账与从前完全一样，而更新
/// 的内核能看出锚点的对象变了，而不是让"关于某个对象的租约"悄悄为"关于另一个对象的承诺"背书。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdoptionObject {
    /// The route's anchor, as the entry spells it.
    /// 路线的锚点，按条目的写法。
    pub anchor: String,
    /// The entry's `at`, which is what pairs this binding with that one entry.
    /// 条目的 `at`——正是它把这层绑定与那一条条目配上。
    pub at: String,
    /// The persistent object identity the anchor named when the decision was taken.
    /// 做出决定时锚点点名的持续对象身份。
    pub stable: String,
}

/// Read the object bindings a ledger carries, as `render_adoption_object` writes them.
/// 读取台账携带的对象绑定，按 `render_adoption_object` 写它的样子。
pub fn parse_adoption_objects(ledger: &str) -> Result<Vec<AdoptionObject>, AdoptionError> {
    let mut objects = Vec::new();
    for (index, line) in ledger.lines().enumerate() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("# object ") else {
            continue;
        };
        let parts = rest.split('|').map(str::trim).collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(AdoptionError {
                line: index + 1,
                message: "a `# object` line must read `# object <anchor> | <at> | <stable>`"
                    .to_owned(),
            });
        }
        if parts[2].len() != 32 || !parts[2].bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AdoptionError {
                line: index + 1,
                message: format!(
                    "`{}` is not a 32-hex object identity; write the identity the ledger uses",
                    parts[2]
                ),
            });
        }
        objects.push(AdoptionObject {
            anchor: parts[0].to_owned(),
            at: parts[1].to_owned(),
            stable: parts[2].to_ascii_lowercase(),
        });
    }
    Ok(objects)
}

/// The line that carries an object binding, as `parse_adoption_objects` reads it.
/// 携带对象绑定的那一行，按 `parse_adoption_objects` 读它的样子。
pub fn render_adoption_object(anchor: &str, at: &str, stable: &str) -> String {
    format!("# object {anchor} | {at} | {stable}\n")
}

/// Per-file prints carried beside one entry, keyed by the entry's own `(anchor, at)`.
/// 一条条目旁边携带的逐文件指纹，按条目自己的 `(anchor, at)` 对应。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdoptionPrints {
    /// The route's anchor, as the entry spells it.
    /// 路线的锚点，按条目的写法。
    pub anchor: String,
    /// When the entry was written, which is what pairs these prints with it.
    /// 条目写下的时间——正是它把这批指纹与那条条目配上。
    pub at: String,
    /// One `(path, print)` per covered file, in the order the line carried them.
    /// 每个被覆盖文件一个 `(路径, 指纹)`，按那一行携带的顺序。
    pub files: Vec<(String, String)>,
}

/// The fingerprint of the bytes a decision was taken on.
/// 一次决定据以做出的那些字节的指纹。
///
/// Sorted by path, so two callers that read the same files in different orders agree; length
/// delimited, so two different file sets cannot concatenate into the same input.
/// 按路径排序，因此以不同顺序读同样的两个调用方会得到同一个值；带长度分隔，因此两组不同的文件无法拼接
/// 成同一个输入。
pub fn adoption_fingerprint(files: &[(String, String)]) -> String {
    let mut ordered = files.to_vec();
    ordered.sort();
    let mut input = Vec::new();
    for (path, contents) in ordered {
        input.extend_from_slice(path.as_bytes());
        input.push(0);
        input.extend_from_slice(contents.len().to_string().as_bytes());
        input.push(0);
        input.extend_from_slice(contents.as_bytes());
        input.push(0);
    }
    sha256_hex(&input)
}

/// Whether an entry still holds, and which file moved if it does not.
/// 条目是否仍然成立；不成立时是哪个文件动了。
///
/// `Lapsed` is not a revocation and carries no instruction to revert anything: it says the bytes
/// the decision was taken on are not the bytes here now, and the file it names is where to look.
/// `Lapsed` 不是撤销，也不含"回退什么"的指令：它说的是"当时据以决定的字节不是现在这些"，而它点名的
/// 文件就是该去看的地方。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdoptionVerdict {
    /// The bytes are the ones the decision was taken on.
    /// 字节仍是决定据以做出的那些。
    Provisional,
    /// A file the decision rests on is missing or no longer has those bytes.
    /// 决定依托的某个文件不见了，或不再是那些字节。
    Lapsed {
        /// The file that moved, or the first one missing.
        /// 动了的那个文件，或第一个缺失的文件。
        file: String,
    },
}

/// The verdict on one entry **and** the fingerprint these bytes give it.
/// 对一条条目的判定，**以及**这些字节给出的指纹。
///
/// `verdict_of` computed this fingerprint and then dropped it, so every reader that wanted to show
/// *why* a lease holds or lapsed had to produce it again. The one reader that does show it rebuilt
/// the rule from these sources instead: the W8 round measured **18,177 characters of reasoning —
/// 11.5% of that arm's whole chain** — spent re-deriving a value this function already held.
/// `verdict_of` 过去算出这个指纹随后丢掉，于是每个想展示"租约为什么成立/为什么失效"的读者都得再产一次。
/// 唯一展示它的那个读者干脆**从这些源码里把规则重新推了出来**：W8 实测 **18,177 字符的推理——占该臂
/// 整条链的 11.5%**——全花在重建这个函数手里本来就有的值上。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdoptionState {
    /// Whether the entry still holds.
    /// 条目是否仍然成立。
    pub verdict: AdoptionVerdict,
    /// The fingerprint of the bytes the entry names, as they are here now.
    /// 条目点名的那些字节**现在**给出的指纹。
    pub current: String,
}

/// Decide whether one entry still describes these bytes, and say what fingerprint it computed.
/// 判定一条条目是否仍在描述这些字节，并说出它算出的指纹。
pub fn state_of(entry: &AdoptionEntry, current: &[(String, String)]) -> AdoptionState {
    let held: BTreeMap<&str, &str> = current
        .iter()
        .map(|(path, contents)| (path.as_str(), contents.as_str()))
        .collect();
    let named = entry
        .files
        .iter()
        .map(|file| {
            let contents = held.get(file.as_str()).copied().unwrap_or_default();
            (file.clone(), contents.to_owned())
        })
        .collect::<Vec<_>>();
    let missing = entry
        .files
        .iter()
        .find(|file| !held.contains_key(file.as_str()));
    let current = adoption_fingerprint(&named);
    let verdict = if let Some(file) = missing {
        AdoptionVerdict::Lapsed { file: file.clone() }
    } else if current == entry.fingerprint {
        AdoptionVerdict::Provisional
    } else {
        AdoptionVerdict::Lapsed {
            file: entry.files[0].clone(),
        }
    };
    AdoptionState { verdict, current }
}

/// Decide whether one entry still describes these bytes.
/// 判定一条条目是否仍在描述这些字节。
///
/// A thin reading of `state_of`, kept because four callers want only the verdict: the fingerprint is
/// one rule and lives in one place, so this cannot drift from it.
/// 对 `state_of` 的薄读法，留着是因为四个调用方只要判定：指纹是一条规则、只住一处，因此这里不会与它漂移。
pub fn verdict_of(entry: &AdoptionEntry, current: &[(String, String)]) -> AdoptionVerdict {
    state_of(entry, current).verdict
}

#[cfg(test)]
#[path = "adoption_tests.rs"]
mod adoption_tests;
