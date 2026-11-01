//! The adoption ledger at `<root>/.xirang/adopted/entries`: which routes this tree trusts
//! **provisionally**, and whether the bytes each decision was taken on are still here.
//! `<root>/.xirang/adopted/entries` 上的采信台账：这棵树**暂时**采信哪些路线，以及每次决定据以
//! 做出的那些字节是否还在。
//!
//! The maintainer's rule for this surface: 「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定
//! 是稳步发展的，只是暂时采信」. So reading never renews anything, a lapsed entry is reported as a
//! fact with the file to look at, and the only way a line is added is a request that says `apply`,
//! says `confirm`, and names who confirmed it and why. The word this module never prints is
//! "verified": an adoption is a lease, and the honest level is `(provisional)`.
//! 维护者对这个面的规矩：「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定是稳步发展的，只是暂时
//! 采信」。因此读取永不续期，失效条目被当作一个事实报出并点名该去看的文件，而唯一的加行途径是一个说出
//! `apply`、说出 `confirm`、并点名"谁确认的、为什么"的请求。本模块永不打印的词是"已验证"：采信是租约，
//! 诚实的等级就是 `(provisional)`。

use std::path::{Path, PathBuf};

use serde_json::Value;
use xirang_kernel::adoption::{
    AdoptionEntry, AdoptionObject, AdoptionPrints, AdoptionState, AdoptionVerdict,
    adoption_fingerprint, parse_adoption, parse_adoption_objects, parse_adoption_prints,
    render_adoption_object, state_of,
};
use xirang_kernel::lexicon::{ADOPTION_DIR, ADOPTION_FILE, XIRANG_DIR};

use crate::mcp::freshness::wall_clock;

#[path = "adopted_family.rs"]
mod family;
pub(crate) use family::family_leases;

/// The ledger path for one package root.
/// 一个包根的台账路径。
fn ledger_path(root: &Path) -> PathBuf {
    root.join(XIRANG_DIR).join(ADOPTION_DIR).join(ADOPTION_FILE)
}

/// The object bindings this ledger carries, as the kernel's comment lines spell them.
/// 这份台账携带的对象绑定，按内核的注释行写法。
fn object_bindings(root: &Path) -> Result<Vec<AdoptionObject>, String> {
    let path = ledger_path(root);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    parse_adoption_objects(&text)
        .map_err(|error| format!("{}:{} {}", path.display(), error.line, error.message))
}

/// Read the files a decision rests on, refusing to leave the root.
/// 读取一次决定依托的文件，拒绝离开根。
///
/// Shared with `consistency --specimen`, which compares the shape those same bytes declare: one
/// reader for "the bytes a lease rests on", so a containment rule cannot drift between two copies.
/// 与 `consistency --specimen` 共用——它比较的正是同一批字节所声明的形状：租约依托哪些字节只有一个读法，
/// 因此"不许离开根"这条规则不会在两份副本之间漂移。
pub(crate) fn read_files(root: &Path, files: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut read = Vec::new();
    for file in files {
        let relative = Path::new(file);
        if relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        }) {
            return Err(format!(
                "`{file}` must stay inside the configured source root"
            ));
        }
        let path = root.join(relative);
        if !crate::mcp::source_index::is_safe_child(root, &path) {
            return Err(format!(
                "`{file}` is not readable inside the configured source root"
            ));
        }
        let contents = std::fs::read_to_string(&path)
            .map_err(|error| format!("{} is not readable: {error}", path.display()))?;
        read.push((file.clone(), contents));
    }
    Ok(read)
}

/// One entry's covered bytes as they read now, with the lease verdict those bytes give.
/// 一条条目覆盖的字节此刻读到的样子，连同它们给出的租约判定。
pub(crate) struct EntryReading {
    /// The covered bytes that were read, in ledger order.
    /// 已读到的被覆盖字节，按台账顺序。
    pub(crate) current: Vec<(String, String)>,
    /// The lease verdict for those bytes.
    /// 这些字节给出的租约判定。
    pub(crate) state: AdoptionState,
}

/// Read one entry's covered files and judge the lease, keeping the failure **per entry**.
/// 读取一条条目覆盖的文件并判定租约，把失败留在**逐条**这一层。
///
/// A missing or escaping file is a fact about *this* lease, never a query-wide error: a reader that
/// asked about several entries must still receive the ones whose bytes are here (A01). The read
/// still refuses to leave the root, and nothing here renews the lease.
/// 缺失或越界的文件是关于**这一条**租约的事实，绝不是整次查询的错误：同时问几条条目的读者仍必须收到
/// 那些字节还在的条目（A01）。读取仍然拒绝离开根，且这里没有任何东西会续期租约。
pub(crate) fn read_lease(root: &Path, entry: &AdoptionEntry) -> Result<EntryReading, String> {
    let current = read_files(root, &entry.files)?;
    let state = state_of(entry, &current);
    Ok(EntryReading { current, state })
}

/// Report every ledger entry, or add one when the request asks to.
/// 报出台账里的每条条目；当请求要求写入时追加一条。
/// The ledger's parsed entries, for readers that need the rows rather than a rendered verdict.
/// 台账解析后的条目，供需要"行"而不是"渲染结论"的读者使用。
///
/// The parser is the kernel's, so a second reading of the same format cannot drift from it.
/// 解析器是内核那一个，因此同一格式不会出现第二份会漂移的读法。
pub(crate) fn entries(root: &Path) -> Result<Vec<AdoptionEntry>, String> {
    let path = ledger_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_adoption(&text)
        .map_err(|error| format!("{}:{}: {}", path.display(), error.line, error.message))
}

/// What an anchor's newest record says about its own authority right now.
/// 某个 anchor 最新一条记录此刻对自身授权的说法。
///
/// A rule is accepted **against an authority**, so whether that authority still holds is not a
/// detail the rule book may skip: a lapsed lease is a decision that needs a person to renew it, and
/// a rule accepted against it would be a rule nobody currently stands behind. The four answers are
/// kept apart on purpose — "there is no such anchor", "the bytes moved", "the lease cannot be read"
/// and "it holds" send a caller to three different next steps.
/// 规则是**对照一个授权**被接受的，因此那个授权是否仍然成立不是规则簿可以跳过的细节：失效的租约是一个
/// 需要人来续期的决定，而对照它接受的规则会是一条当前无人背书的规则。四个答案有意分开——"没有这个
/// anchor"、"字节动过"、"租约读不了"、"仍然成立"把调用方引向三种不同的下一步。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnchorStanding {
    /// The newest record's bytes are the ones it was confirmed on.
    /// 最新记录的字节仍是它被确认时的那些。
    InForce,
    /// A covered file moved, and the record says which one.
    /// 被覆盖的文件动过，记录点名是哪一个。
    Lapsed(String),
    /// The covered bytes cannot be read, so the lease cannot be judged.
    /// 被覆盖的字节读不到，租约判不了。
    Unreadable(String),
    /// No ledger line names this anchor.
    /// 台账里没有一行点名这个 anchor。
    Missing,
}

/// Judge one anchor's authority from its **newest** ledger record.
/// 按**最新**一条台账记录判定一个 anchor 的授权。
///
/// The newest line is the one in force (the lease rule `conformance` already renders), so a caller
/// that only asks "does this anchor exist" can accept a rule against a confirmation that was
/// superseded or whose bytes are gone. This reads the same `state_of` the ledger's own reader uses.
/// **最后一条**生效（`conformance` 已经渲染的租约规则），因此只问"这个 anchor 存不存在"的调用方会对照
/// 一份被后续记录取代、或字节已不在的确认去接受规则。这里读的是台账自己的读者所用的同一个 `state_of`。
pub(crate) fn anchor_standing(root: &Path, anchor: &str) -> Result<AnchorStanding, String> {
    let entries = entries(root)?;
    let Some(newest) = entries.iter().rev().find(|entry| entry.anchor == anchor) else {
        return Ok(AnchorStanding::Missing);
    };
    match read_lease(root, newest) {
        Ok(reading) => match reading.state.verdict {
            AdoptionVerdict::Provisional => {
                // The bytes are still the bytes; the next question is whether the **object** they
                // were about is still the object at that anchor. A lease about one object must not
                // back a commitment about another, and the additive `# object` line is what makes
                // that answerable without changing the entry format.
                // 字节仍是那批字节；下一个问题是它们所针对的**对象**是否仍是那个锚点上的对象。关于一个对象的
                // 租约不得为关于另一个对象的承诺背书，而加法式的 `# object` 行正让这一点无需改条目格式即可回答。
                let bound = object_bindings(root)?
                    .into_iter()
                    .find(|binding| binding.anchor == anchor && binding.at == newest.at);
                let Some(bound) = bound else {
                    return Ok(AnchorStanding::InForce);
                };
                match crate::mcp::rules::resolve_subject(root, anchor) {
                    Ok(Some(subject)) if subject.stable.to_string() == bound.stable => {
                        Ok(AnchorStanding::InForce)
                    }
                    Ok(Some(subject)) => Ok(AnchorStanding::Lapsed(format!(
                        "a different object now holds `{anchor}` (the decision was about {}, now {})",
                        bound.stable, subject.stable
                    ))),
                    Ok(None) => Ok(AnchorStanding::Lapsed(format!(
                        "no face resolves at `{anchor}` any more, so the object this decision was about is gone"
                    ))),
                    Err(cause) => Ok(AnchorStanding::Unreadable(cause)),
                }
            }
            AdoptionVerdict::Lapsed { file } => Ok(AnchorStanding::Lapsed(format!(
                "the bytes it was confirmed on moved (first at {file})"
            ))),
        },
        Err(cause) => Ok(AnchorStanding::Unreadable(cause)),
    }
}

/// What an anchor's claim is right now: the entry in force, its verdict, and the ledger's history.
/// 某个 anchor 的声明现在处于什么状态：生效的那一条、它的判定，以及台账里的修订次数。
///
/// The question is the ledger's own, so it is answered here rather than in a second module that
/// would have to re-read the same file. The lease semantics are the kernel's: the **newest** line
/// for an anchor is the one in force.
/// 这个问题是台账自己的，因此在这里回答，而不是另开一个模块再读一遍同一个文件。租约语义来自内核：
/// 同一个 anchor 的**最后一条**生效。
pub(crate) fn conformance(root: &Path, arguments: &Value) -> Result<String, String> {
    let entries = entries(root)?;
    let entry_prints = prints_by_entry(root)?;
    let requested = arguments
        .get("anchor")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|anchor| !anchor.is_empty());
    // No anchor means **every** anchor, one line each. Measured (T-21): asking about two entries cost
    // two calls, because the argument was required — and the fact a caller wants is usually "which of
    // these still hold", not "the third one's history". The per-anchor view below is unchanged.
    // 不带 anchor 就是**每个**锚点一行。量到的（T-21）：问两条条目要两次调用，因为这个参数是必填的——而调用方
    // 通常要的是"这些里面哪些还成立"，不是"第三条的历史"。下面的单锚点视图一个字没改。
    let Some(anchor) = requested else {
        if entries.is_empty() {
            return Ok(format!(
                "no adoption ledger at {}\nAn adoption is provisional and needs a person to \
                 confirm it: write one with `anchor`, `certifies`, `evidence`, `verifier`, `reason` \
                 and `files`, plus `apply` and `confirm`.\n",
                ledger_path(root).display()
            ));
        }
        let mut output = format!(
            "evidence: adoption ledger at {} (an adoption is a lease: the newest line for an anchor \
             is the one in force)\n",
            ledger_path(root).display()
        );
        let mut provisional = 0usize;
        let mut lapsed = 0usize;
        let mut missing = 0usize;
        let mut seen: Vec<&str> = Vec::new();
        for entry in &entries {
            if seen.contains(&entry.anchor.as_str()) {
                continue;
            }
            seen.push(&entry.anchor);
            let in_force = entries
                .iter()
                .rfind(|candidate| candidate.anchor == entry.anchor)
                .expect("the anchor came from this list");
            // Per entry: one missing file is that lease's fact, not the query's failure (A01).
            // 逐条：一个缺失文件是那条租约的事实，不是整次查询的失败（A01）。
            match read_lease(root, in_force) {
                Ok(reading) => match &reading.state.verdict {
                    AdoptionVerdict::Lapsed { file } => {
                        lapsed += 1;
                        output.push_str(&format!("  lapsed      {} ({file})\n", in_force.anchor));
                    }
                    AdoptionVerdict::Provisional => {
                        provisional += 1;
                        output.push_str(&format!("  in force    {}\n", in_force.anchor));
                    }
                },
                Err(cause) => {
                    missing += 1;
                    output.push_str(&format!("  unreadable  {} ({cause})\n", in_force.anchor));
                }
            }
        }
        output.push_str(&format!(
            "provisional {provisional}  lapsed {lapsed}  missing {missing}\nnext   `conformance \
             {{anchor: \"<one of the above>\"}}` for one anchor's history, `adopted` for the entries \
             in full\n"
        ));
        return Ok(output);
    };

    let history: Vec<&AdoptionEntry> = entries
        .iter()
        .filter(|entry| entry.anchor == anchor)
        .collect();
    if history.is_empty() {
        return Ok(format!(
            "no ledger entry names `{anchor}` in {} — the ledger holds {} entry(ies){}\n",
            root.display(),
            entries.len(),
            if entries.is_empty() {
                " (there is no ledger here)"
            } else {
                "; `adopted` lists the anchors"
            }
        ));
    }
    let effective = history.last().expect("non-empty history");
    let mut lines = vec![format!(
        "anchor `{anchor}` — {} revision(s) in the ledger; the last is in force (an adoption is a \
         lease: the newest line wins)",
        history.len()
    )];
    // Per entry: a missing covered file is this anchor's verdict, never the query's error (A01).
    // 逐条：缺失的被覆盖文件是这个 anchor 的判定，绝不是整次查询的错误（A01）。
    match read_lease(root, effective) {
        Ok(reading) => {
            match &reading.state.verdict {
                AdoptionVerdict::Provisional => lines.push(format!(
                    "  in force   provisional — certifies: {} (adopted {} by {})",
                    effective.certifies, effective.at, effective.verifier
                )),
                AdoptionVerdict::Lapsed { file } => {
                    // The known answer wins over the kernel's "first covered file": a set print cannot say
                    // which file moved, and a per-file print can.
                    // 以**已知的**为准，而不是内核给的"第一个被覆盖文件"：集合指纹说不出哪个文件动了，逐文件指纹能。
                    let named = match moved_files(
                        entry_prints.get(&(effective.anchor.clone(), effective.at.clone())),
                        &reading.current,
                    ) {
                        Some(list) if !list.is_empty() => list.join(", "),
                        _ => file.clone(),
                    };
                    lines.push(format!(
                        "  in force   lapsed at {named}: the bytes moved after the confirmation, so this needs \
                         a **person**, not an edit (adopted {} by {})",
                        effective.at, effective.verifier
                    ))
                }
            }
            lines.push(format!(
                "  bytes      {}",
                lease(
                    &effective.fingerprint,
                    &reading.state.current,
                    &effective.files,
                    false,
                    moved_files(
                        entry_prints.get(&(effective.anchor.clone(), effective.at.clone())),
                        &reading.current,
                    )
                    .as_deref(),
                )
            ));
        }
        Err(cause) => {
            lines.push(format!(
                "  in force   unreadable — {cause}: the bytes this lease rests on could not be read, \
                 so the lease cannot be judged and needs a **person**, not an edit (adopted {} by {})",
                effective.at, effective.verifier
            ));
            lines.push(format!("  bytes      unknown (unreadable: {cause})"));
        }
    }
    lines.push(format!("  covers     {}", effective.files.join(", ")));
    lines.push(
        "not covered here: whether the siblings of this anchor's object follow the same shape (ask \
         `consistency --specimen <anchor>`, which reads the declared fields from the files this \
         entry covers and compares the siblings against them), and the shape of the ledger's own \
         history beyond the newest line"
            .to_owned(),
    );
    lines.push(
        format!(
            "next   `adopted` for every entry's verdict, `consistency --specimen {anchor}` for \
             whether the siblings follow this shape"
        )
        .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}

/// The two fingerprints a reader needs to judge a lease without re-deriving the rule.
/// 读者判断一条租约所需的两串指纹——不必再把规则重推一遍。
///
/// Measured need (W8, h2): the client wanted to know whether a lapsed entry's recorded fingerprint
/// described some *other* file's current bytes, and the reply carried neither value, so it read this
/// tree's sources and rebuilt the kernel's fingerprint composition by hand — 18,177 characters of
/// reasoning, 11.5% of that arm's chain.
/// 量出来的需求（W8 的 h2）：客户端想知道一条失效条目的记录指纹是否描述了**别的文件**的当前字节，而
/// 回复里两个值都没有，于是它去读本树的源码、手工把内核的指纹组合重新拼出来——18,177 字符的推理，占
/// 该臂整条链的 11.5%。
/// What the bytes say, in the reader's terms: they are the ones the lease was taken on, or they are
/// not — and **which file** is not.
/// 字节在读者眼里是什么：它们还是这份租约当初针对的那一份，或者不是——以及**哪个文件**不是。
///
/// The recorded value is a print (a hash over the covered set), and printing it was a mistake: a
/// twelve-character hex string is a **comparison key**, not information — a reader can neither find a
/// file with it nor change a line with it, and its only content appears when two of them sit side by
/// side. What a reader needs is the verdict per file, so that is what this renders. Where the record
/// cannot support that verdict, it says so instead of hiding it: one print for a set of files can say
/// "something in here moved" and never "this one did", and that limit belongs to the record's shape.
/// 记录值是一个指纹（对被覆盖集合取的散列），而把它印出来是错的：一串十二位十六进制是**比较键**，不是信息
/// ——读者既不能拿它去找文件，也不能拿它去改某一行，而它唯一的内容只在两串并排时才出现。读者要的是**逐文件
/// 的判定**，所以这里印的是判定。记录撑不住这个判定的地方，就如实说出来而不是藏起来：对一组文件只留一个指纹，
/// 能说"这里面有东西动了"，永远说不出"是这一个"——那是**记录形状**的限制。
fn lease(
    recorded: &str,
    current: &str,
    files: &[String],
    file_already_named: bool,
    moved: Option<&[String]>,
) -> String {
    if recorded == current {
        return "unchanged since the confirmation".to_owned();
    }
    // With a print per file the answer is the file's **name**, which is what a reader can act on; the
    // set print alone could only ever say "something in here moved". When the two disagree — the set
    // print says changed and no per-file print does — that is said rather than smoothed over, because
    // the alternative is a reader trusting a verdict nothing supports.
    // 有逐文件指纹时，答案是那个文件的**名字**——读者能据它做事；只有集合指纹时永远只能说"这里面有东西动了"。
    // 两者不一致时（集合指纹说变了、逐文件指纹一个都没变）如实说出来而不是抹平：否则读者会去信一个没有依据的判定。
    if let Some(moved) = moved {
        return match moved {
            [] => "changed since the confirmation — and no per-file print names which, so the \
                   record's two prints disagree; re-confirm rather than trust either"
                .to_owned(),
            // The caller's own line already names them in this case, so repeating the path here
            // would say the same thing twice on one row.
            // 这种情形下调用方那一行已经点名了，因此这里再写一遍路径就是在同一行上说两遍同一件事。
            many if file_already_named => format!(
                "changed since the confirmation ({} of {} covered file(s))",
                many.len(),
                files.len()
            ),
            [one] => format!("changed since the confirmation ({one})"),
            many => format!("changed since the confirmation ({})", many.join(", ")),
        };
    }
    match files {
        [only] if file_already_named => "changed since the confirmation".to_owned(),
        [only] => format!("changed since the confirmation ({only})"),
        many => format!(
            "changed since the confirmation — one or more of the {} covered files did, and which \
             one is **not in the record**: a single print covers the set, so it cannot say. A \
             per-file record would",
            many.len()
        ),
    }
}

/// One entry's per-file prints, reached by the entry's own `(anchor, at)`.
/// 一条条目的逐文件指纹，用条目自己的 `(anchor, at)` 取。
type PrintsByEntry = std::collections::BTreeMap<(String, String), Vec<(String, String)>>;

/// The per-file prints this ledger carries, keyed by the entry they belong to.
/// 这份台账携带的逐文件指纹，按它们所属的条目索引。
///
/// Read through the kernel's own parser, so the spelling has one implementation; an old ledger simply
/// has none of these lines and comes back empty.
/// 经内核自己的解析器读，因此拼写只有一份实现；旧台账根本没有这些行，读回来就是空的。
fn prints_by_entry(root: &Path) -> Result<PrintsByEntry, String> {
    let ledger = ledger_path(root);
    let text = std::fs::read_to_string(&ledger).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(PrintsByEntry::new());
    }
    let parsed = parse_adoption_prints(&text)
        .map_err(|error| format!("{}:{} {}", ledger.display(), error.line, error.message))?;
    Ok(parsed
        .into_iter()
        .map(|AdoptionPrints { anchor, at, files }| ((anchor, at), files))
        .collect())
}

/// Which covered files' bytes moved, when the record carries a print per file.
/// 当记录逐文件带着指纹时：**哪几个被覆盖文件的字节动了**。
fn moved_files(
    prints: Option<&Vec<(String, String)>>,
    current: &[(String, String)],
) -> Option<Vec<String>> {
    let prints = prints?;
    let single = |path: &str, contents: &str| {
        adoption_fingerprint(&[(path.to_owned(), contents.to_owned())])
    };
    let mut moved = Vec::new();
    for (path, contents) in current {
        let now = single(path, contents);
        match prints.iter().find(|(recorded, _)| recorded == path) {
            Some((_, recorded)) if recorded != &now => moved.push(path.clone()),
            _ => {}
        }
    }
    moved.sort();
    Some(moved)
}

pub(crate) fn adopted(root: &Path, arguments: &Value) -> Result<String, String> {
    let ledger = ledger_path(root);
    if arguments.get("anchor").and_then(Value::as_str).is_some() {
        return renew(root, arguments, &ledger);
    }
    let text = std::fs::read_to_string(&ledger).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(format!(
            "no adoption ledger at {}\nAn adoption is provisional and needs a person to confirm \
             it: write one with `anchor`, `certifies`, `evidence`, `verifier`, `reason` and \
             `files`, plus `apply` and `confirm`.\n",
            ledger.display()
        ));
    }
    let entries = parse_adoption(&text)
        .map_err(|error| format!("{}:{} {}", ledger.display(), error.line, error.message))?;
    // The ledger's **path** rides on the first line in both cases. It used to appear only when the
    // ledger was empty, so a reader with a populated ledger had to go looking for the file — measured
    // in T-21: the arm spent shell calls on `find`/`ls` before it could read the two lines it was
    // asking about, and a reader that has to find the record is a reader one call poorer.
    // 台账**路径**在两种情形下都上第一行。它过去只在台账为空时出现，于是面对一份有内容的台账的读者得自己去找
    // 那个文件——T-21 实测：那一臂先花了若干 shell 调用 `find`/`ls`，才能读它在问的那两行；而要自己去找记录的
    // 读者，就是少了一次调用的读者。
    let entry_prints = prints_by_entry(root)?;
    let mut output = format!(
        "evidence: adoption ledger at {} (provisional by construction)\n",
        ledger.display()
    );
    let (mut provisional, mut lapsed) = (0usize, 0usize);
    let mut first_lapsed: Option<String> = None;
    // Every entry's current print, computed once, so a lapsed line can say **whose** print its
    // recorded one is. Measured (T-21, h2): the arm spent ~15,000 characters working out that
    // `edc72845…` in a line naming `panel.rs` is `button.rs`'s print — the ledger's two hex strings
    // say what the bytes are, never whose, and the bridge already holds every covered file.
    // 每条的当前指纹先算一遍，好让失效的那一行说出它记录的那串**是谁的**。量到的（T-21 的 h2）：那一臂花了
    // 约 15,000 字符才弄清"点名 `panel.rs` 的那一行里的 `edc72845…` 是 `button.rs` 的"——台账给的两串十六进制
    // 说了"是什么"，从不说"是谁的"，而桥本来就握着每个被覆盖的文件。
    let mut currents: Vec<(String, String)> = Vec::new();
    for entry in &entries {
        if let Ok(reading) = read_lease(root, entry) {
            currents.push((entry.anchor.clone(), reading.state.current));
        }
    }
    let mut missing = 0usize;
    for entry in &entries {
        // Per entry: one unreadable covered file is that lease's fact, never the query's failure, so
        // the readable entries below still reach the reader (A01).
        // 逐条：一个读不了的被覆盖文件是那条租约的事实，绝不是整次查询的失败，因此下面那些可读的条目
        // 仍能到达读者（A01）。
        let reading = match read_lease(root, entry) {
            Ok(reading) => reading,
            Err(cause) => {
                missing += 1;
                first_lapsed.get_or_insert_with(|| entry.anchor.clone());
                output.push_str(&format!(
                    "adoption unreadable ({cause}); needs confirmation — {}: {} [adopted at {} by {}; \
                     why: {}]\n",
                    entry.anchor, entry.certifies, entry.at, entry.verifier, entry.reason,
                ));
                continue;
            }
        };
        let current = reading.current;
        let state = reading.state;
        // The list row already says `lapsed at <file>`, so the bytes clause does not name it twice.
        // 列表那一行已经说了 `lapsed at <文件>`，因此字节那句不再重复点名。
        let moved = moved_files(
            entry_prints.get(&(entry.anchor.clone(), entry.at.clone())),
            &current,
        );
        let bytes = lease(
            &entry.fingerprint,
            &state.current,
            &entry.files,
            true,
            moved.as_deref(),
        );
        match &state.verdict {
            AdoptionVerdict::Provisional => {
                provisional += 1;
                output.push_str(&format!(
                    "adopted since {} ({bytes}) (provisional) — {}: {} [evidence: {}; confirmed by \
                     {}; why: {}]\n",
                    entry.at,
                    entry.anchor,
                    entry.certifies,
                    entry.evidence,
                    entry.verifier,
                    entry.reason,
                ));
            }
            AdoptionVerdict::Lapsed { file } => {
                lapsed += 1;
                first_lapsed.get_or_insert_with(|| entry.anchor.clone());
                // Whose print the recorded one is, when it is another entry's: that is the difference
                // between "the bytes moved" and "this line carries a copy of another file's print",
                // and only the second one explains a ledger that names one file and prints another.
                // 记录的那串是谁的（当它是别的条目的时）：这就是"字节动了"与"这一行带着另一个文件的指纹副本"
                // 之间的差别，而只有后者能解释"点名一个文件、却印着另一个文件的指纹"的台账。
                let borrowed = currents
                    .iter()
                    .find(|(anchor, print)| anchor != &entry.anchor && print == &entry.fingerprint)
                    .map(|(anchor, _)| anchor.clone());
                // With a print per file, the moved files are **known**, and the kernel's own
                // `file` is only the first covered one (a set print cannot say which moved) — so the
                // known answer wins, and the wrong-but-first one is not printed beside it.
                // 有逐文件指纹时，动过的文件是**可知的**，而内核给的 `file` 只是第一个被覆盖的文件（集合指纹
                // 说不出哪个动了）⇒ 以已知的为准，不把"排在第一个"的那个印在旁边。
                let named = match &moved {
                    Some(list) if !list.is_empty() => list.join(", "),
                    _ => file.clone(),
                };
                let whose = match &borrowed {
                    Some(anchor) => format!(
                        "; the recorded print is `{anchor}`'s current print, not this file's — a \
                         copy of another entry's bytes"
                    ),
                    None => String::new(),
                };
                output.push_str(&format!(
                    "adoption lapsed at {named} ({bytes}){whose}; needs confirmation — {}: {} [adopted \
                     at {} by {}; why: {}]\n",
                    entry.anchor, entry.certifies, entry.at, entry.verifier, entry.reason,
                ));
            }
        }
    }
    output.push_str(&format!(
        "provisional {provisional}  lapsed {lapsed}  missing {missing}\n"
    ));
    output.push_str(
        "note: nothing here renews or revokes an adoption — a lapsed one needs a person to \
         confirm the new state, and a confirmation is one more line. A later line for the same \
         anchor is the one in force; an earlier line stays as its history, which is why a lapsed \
         line can sit above a provisional one and still be the truth about the bytes it named.\n",
    );
    // The state was complete and the action was missing: the round-5 evaluation's other arm read
    // this ledger's rules, concluded that a confirmation belongs to a person, and therefore changed
    // nothing — while the question was what to do about a route the ledger does not name. Naming
    // that action here is not a licence to forge a confirmation: the request still has to say
    // `apply` and `confirm`, and the line still names who confirmed it.
    // 状态是完整的，缺的是**动作**：第五轮评测的另一臂读懂了这台账的规矩，推出"确认属于人"，于是
    // 什么都没做——而它面对的问题恰恰是"台账没点名的路线该怎么办"。这里把那个动作点出来，不是给伪造
    // 确认发许可证：请求仍然必须说出 `apply` 与 `confirm`，那一行仍然点名是谁确认的。
    output.push_str(
        // Instantiated from this ledger's own state: when an entry has lapsed, the next call is the
        // one that asks about **that** anchor, named. The prose below stays for the other case —
        // a route the ledger does not carry at all.
        // 由这份台账自己的状态实例化：有条目失效时，下一次调用就是问**那一个** anchor 的调用，且点名它。
        // 下面那段散文留给另一种情形——台账里根本没有这条路线。
        &match &first_lapsed {
            Some(anchor) => format!(
                "next   `conformance {{anchor: \"{anchor}\"}}` says whether that lease still holds \
                 and where it lapsed; a route the ledger does not name is a **new anchor** — a first \
                 confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, \
                 `reason` and `files` with `apply: true` and `confirm: true`, and this tool appends \
                 one line whose fingerprint it computes from those files\n"
            ),
            None => "next   a route this ledger does not name is a **new anchor** — a first \
                     confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, \
                     `verifier`, `reason` and `files` together with `apply: true` and \
                     `confirm: true`, and this tool appends one line whose fingerprint it computes \
                     from those files\n"
                .to_owned(),
        },
    );
    Ok(output)
}

/// The append-only renewal: a preview unless the request says `apply` and `confirm`.
/// 只追加的续期：除非请求说出 `apply` 与 `confirm`，否则只是预览。
fn renew(root: &Path, arguments: &Value, ledger: &Path) -> Result<String, String> {
    let text = |key: &str| -> Result<String, String> {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("a renewal requires `{key}`"))
    };
    let anchor = text("anchor")?;
    let certifies = text("certifies")?;
    let evidence = text("evidence")?;
    let verifier = text("verifier")?;
    let reason = text("reason")?;
    // A single `--files a` arrives as a string, not as a one-element array: the one-shot client turns
    // a repeated flag into an array and has no way to know which keys are lists. Refusing it said
    // "a renewal requires `files`" while `files` **was** given — the round measured that false refusal,
    // the workaround it drove the caller to (a repeated flag, which reached the ledger as `path,path`
    // and printed the wrong fingerprint), and the cost of both.
    // 单个 `--files a` 到手时是字符串而不是单元素数组：一次性客户端把重复的开关收成数组，却无从知道哪些
    // 键是列表。拒绝它时说"a renewal requires `files`"，而 `files` **明明给了**——那一轮量到这次假拒绝、
    // 它逼出的绕道写法（重复开关，落到台账里是 `path,path`、印出错指纹），以及两者的代价。
    let files = match arguments.get("files") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        Some(Value::String(one)) => vec![one.clone()],
        _ => return Err(
            "`files` names the paths the decision rests on: pass `--files <path>` (repeatable) \
                 or `files` in `--json`"
                .to_owned(),
        ),
    };
    if files.is_empty() {
        return Err("`files` must name at least one path".to_owned());
    }
    let current = read_files(root, &files)?;
    let fingerprint = adoption_fingerprint(&current);
    let entry = AdoptionEntry {
        anchor: anchor.clone(),
        certifies,
        evidence,
        verifier: verifier.clone(),
        at: wall_clock(),
        files: files.clone(),
        fingerprint: fingerprint.clone(),
        reason: reason.clone(),
    };
    let line = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}\n",
        entry.anchor,
        entry.certifies,
        entry.evidence,
        entry.verifier,
        entry.at,
        entry.files.join(","),
        entry.fingerprint,
        entry.reason,
    );
    // The per-file prints ride **beside** the entry, as the kernel's own comment line, so an older
    // kernel reads this ledger exactly as before and a newer one can name the file that moved.
    // 逐文件指纹以**内核自己的注释行**形式跟在条目旁边，因此更旧的内核读这份台账与从前一样，而更新的内核能
    // 点名动了的那一个文件。
    let per_file = current
        .iter()
        .map(|(path, contents)| {
            (
                path.clone(),
                adoption_fingerprint(&[(path.clone(), contents.clone())]),
            )
        })
        .collect::<Vec<_>>();
    let prints =
        xirang_kernel::adoption::render_adoption_prints(&entry.anchor, &entry.at, &per_file);
    // When the anchor names a face in this tree, record **which object** it named. The line is
    // additive (an older kernel skips it), and it is what later lets `anchor_standing` say "a
    // different object now holds this anchor" instead of silently backing a new commitment with an
    // old decision. An anchor that names a policy file (the common case) resolves to no face and
    // carries no binding — the behavior every existing ledger has.
    // 当锚点在这棵树里点名一个面时，记下**它点名的是哪个对象**。这一行是加法（更旧的内核会跳过），而它
    // 让 `anchor_standing` 以后能说"现在这个锚点上是另一个对象"，而不是用旧决定悄悄为新承诺背书。锚点点名
    // 政策文件（常见情形）时解析不到面、不携带绑定——这也是每份现有台账的行为。
    let object = crate::mcp::rules::resolve_subject(root, &entry.anchor)
        .ok()
        .flatten()
        .map(|subject| {
            render_adoption_object(&entry.anchor, &entry.at, &subject.stable.to_string())
        });
    let confirmed = arguments.get("apply").and_then(Value::as_bool) == Some(true)
        && arguments.get("confirm").and_then(Value::as_bool) == Some(true);
    if !confirmed {
        return Ok(format!(
            "preview: would append one adoption line to {}\n  {line}  {prints}{}nothing was written; \
             `apply: true` and `confirm: true` write it, because a confirmation is a person's\n",
            ledger.display(),
            object.as_deref().unwrap_or("")
        ));
    }
    if let Some(parent) = ledger.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} is not creatable: {error}", parent.display()))?;
    }
    let existing = std::fs::read_to_string(ledger).unwrap_or_default();
    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(&line);
    next.push_str(&prints);
    if let Some(object) = &object {
        next.push_str(object);
    }
    crate::run_method::authoring::write_transaction::write_one(ledger, next.as_bytes())?;
    let mut reply = format!(
        "appended one adoption line to {}\n  {line}  {prints}confirmed by {verifier}; the ledger is \
         append-only, so this confirmation is one more line rather than a rewrite\n",
        ledger.display()
    );
    // The candidates that were created before this specimen existed now have a template to follow.
    // 在这份样板存在之前创建的候选，现在有了可以跟随的模板。
    if let Some(note) = crate::mcp::apply::apply_template::candidate_note(root, &entry.anchor) {
        reply.push_str(&note);
    }
    // The other consumer of the same record: which objects already follow this specimen. That is
    // the set a template change would have to carry, so the adoption reply names it.
    // 同一份记录的另一个消费者：已经跟随这个样板的对象。模板变化时要带走的就是这一组，因此采信回复点名它。
    let (followers, unresolved) =
        crate::mcp::apply::apply_template::followers_split(root, &entry.anchor);
    // Whether any follower is behind the version just confirmed. Its own line below says which; this
    // flag exists to name the call that turns that fact into a plan.
    // 是否有跟随者落后于刚确认的版本。下面那行说出是哪些；这个标志是为了点名"把这条事实变成计划"的那次调用。
    let mut behind = false;
    if !followers.is_empty() {
        let names = followers
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        reply.push_str(&format!("template followers: {names}\n"));
        // A follower recorded at an earlier version is the set a template change has to carry. Naming
        // it here is what makes the lineage actionable instead of a list of names.
        // 记录在旧版本上的跟随者，正是模板变化时要带走的那一组。在这里点名它，谱系才是可行动的，而不只是
        // 一串名字。
        let stale = followers
            .iter()
            .filter(|(_, version)| version != &entry.fingerprint)
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>();
        if !stale.is_empty() {
            behind = true;
            reply.push_str(&format!(
                "followers on an earlier template version: {} (this adoption is {})\n",
                stale.join(", "),
                entry.fingerprint
            ));
        }
    }
    // A row whose object no longer resolves is not a follower this adoption can carry: the record
    // outlived a delete (a delete is recoverable) or a change made outside this bridge, and listing
    // it above would promise to move something that is not there.
    // 对象已解析不到的行，不是这次采信能带走的跟随者：记录活过了一次删除（删除是可恢复的）或桥外的一次
    // 改动，把它列在上面会承诺搬动一个并不存在的东西。
    if !unresolved.is_empty() {
        reply.push_str(&format!(
            "followers whose lineage needs re-pointing (the object no longer resolves): {}\n",
            unresolved.join(", ")
        ));
    }
    // The decision that produced the stale version is where a reader learns how to carry it: the plan
    // names which objects are behind, what is mechanical, and what needs a person.
    // 产生旧版本的那个决定，正是读者得知如何搬运它的地方：那份计划点名哪些对象落后、哪些是机械的、哪些需要人。
    if behind {
        reply.push_str(&format!(
            "next   `xirang.refactor {{anchor: \"{}\"}}` plans the carry for those followers\n",
            entry.anchor
        ));
    }
    Ok(reply)
}

#[cfg(test)]
#[path = "adopted_tests.rs"]
mod adopted_tests;
