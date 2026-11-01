//! `xirang.consistency`: compare siblings under one parent, and name the ones that differ.
//! `xirang.consistency`：比较同一父级下的兄弟，点名不一样的那几个。
//!
//! The failure this answers is the one the maintainer named "供应链驳杂": one object under a parent
//! follows a different convention from its siblings — a `button` computing offsets in a relative
//! space while `slider` and `timeline` use the world's — and every file is locally plausible, so
//! reading them one at a time does not show it. The sibling set is a fact only a registration tree
//! has; the comparison below is text (which names each file calls), and it says so.
//! 这条回答的失败正是维护者说的"供应链驳杂"：同一父级下有一个对象跟它的兄弟用了不同的约定 —— 比如
//! `button` 在相对坐标里算偏移，而 `slider` 与 `timeline` 用世界坐标 —— 每个文件单看都自洽，逐个读发现
//! 不了。**同族集合**是只有注册树才有的事实；下面这套比较读的是文本（每个文件调用了哪些名字），并且
//! 它自己说了这一点。
//!
//! It covers the **derived** tree (the normal case for a development checkout). When a member's tree
//! comes from published records instead, the reply says so and names where the fields do live rather
//! than filling them with defaults.
//! 它覆盖**推导**出来的树（开发检出的常态）。当成员的树来自发布记录时，答案会明说，并点名字段实际住在
//! 哪里，而不是拿默认值把空填上。
//!
//! online: the shape comparison compares declarations that may have been added since the build, and a shape is a property of the source text.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use xirang_kernel::adoption::{AdoptionVerdict, verdict_of};
use xirang_kernel::syntax::{FaceSyntax, parse_faces};

use crate::mcp::source_index::{SourceFile, load_sources};
use crate::mcp::workspace::{Member, Scope};

#[path = "consistency_support.rs"]
mod support;

pub(crate) use support::tree_census;
// The shape vocabulary the write path's family verdict reads, re-exported so there is one spelling of
// "what does this declaration declare" rather than two (audit `W5-6`).
// 写入路径的家族判定所读的形状词汇，重导出以便"这条声明声明了什么"只有一份拼写而不是两份（审计 `W5-6`）。
pub(crate) use support::current_identity;
pub(crate) use support::declared_shape;
pub(super) use support::deviations;
pub(crate) use support::roots_with_freshness;

use support::*;

#[path = "consistency_specimen.rs"]
mod specimen;
use specimen::specimen_comparison;

#[cfg(test)]
#[path = "consistency_tests.rs"]
mod consistency_tests;

/// The one face a ledger-named file declares, or why the shape could not be read from it.
/// 台账点名的一个文件所声明的那个注册面；读不出形状时给出原因。
///
/// A missing or multiple declaration is a reason, never an empty shape: "this file declares none of
/// these fields" and "this file's declaration could not be read" are different facts, and filling
/// the second with the first is how a comparison would silently report `0 deviations`.
/// 声明缺失或不止一个时给的是原因，绝不是一个空形状："这个文件没有声明这些字段"与"这个文件的声明读不出来"
/// 是两件不同的事实，而用前者去填后者，正是一份比较会静默报出 `0 deviations` 的来路。
pub(super) fn one_face(text: &str) -> Result<FaceSyntax, String> {
    let mut faces = parse_faces(text).map_err(|error| error.to_string())?;
    match faces.len() {
        0 => Err("it declares no registration face".to_owned()),
        1 => Ok(faces.pop().expect("exactly one face")),
        count => Err(format!(
            "it declares {count} registration faces, so none was selected"
        )),
    }
}

/// The indexed text of a face's own source file, when the index has it.
/// 某个面自己那个源文件的已索引文本（索引里有的话）。
fn source_text<'a>(sources: &'a [SourceFile], face_source: &str) -> Option<&'a str> {
    let wanted = format!("src/{}", face_source.trim_start_matches("./"));
    sources
        .iter()
        .find(|file| file.relative == wanted)
        .map(|file| file.source.as_str())
}

/// Why the published record cannot answer this tool's question.
/// 发布记录为什么答不了本工具的问题。
///
/// The comparison reads each sibling's declared fields and its own text; a record carries neither.
/// Saying that out loud is what lets the derived reading be the honest one rather than a silent
/// fallback.
/// 这次比较读的是每个兄弟的已声明字段与它自己的文本，而记录两样都不带。把这一点说出来，才让"此刻推导"
/// 是一个诚实的读法，而不是一次静默的回落。
const RECORD_CANNOT_ANSWER: &str = "this comparison reads each sibling's declared fields and its own text, which the published \
     record does not carry";

/// One member's faces, derived now, plus the line that says which tree this answer read.
/// 一个成员此刻推导出的面，以及说明这份答案读的是哪棵树的那一行。
///
/// The published record carries no `path` and no declared fields, so it can never answer this
/// question: the derived tree is not a fallback here but the only reading. That is why this goes
/// through `derived_tree` (the documented fallback every caller has to state) rather than `tree` —
/// the latter answers from the record and would make this tool refuse on exactly the trees that have
/// been built, which is the normal case.
/// 发布记录不携带 `path`、也不携带已声明字段，因此它永远答不了这个问题：推导树在这里不是回落而是唯一的
/// 读法。这也是它走 `derived_tree`（有文档的、每个调用方都必须说出来的那条回落）而不是 `tree` 的原因——
/// 后者从记录作答，会让本工具恰好在**已经构建过**的树上拒答，而那是常态。
fn member_faces(
    member: &Member,
    because: &str,
) -> Result<(String, Vec<crate::build_method::FaceView>, String), String> {
    if let Some((faces, evidence)) = published_faces(member) {
        return Ok((evidence, faces, String::new()));
    }
    let evidence = member.evidence_line(because).trim_end().to_owned();
    let (faces, unparsable) = member.derived_tree()?;
    Ok((evidence, faces, unparsable))
}

/// The sibling set: faces directly under one logical path, in tree order.
/// 同族集合：直接挂在某个逻辑路径下的面，按树序。
fn siblings<'a>(
    faces: &'a [crate::build_method::FaceView],
    parent: &str,
) -> Vec<&'a crate::build_method::FaceView> {
    let wanted = parent.trim_end_matches('/');
    faces
        .iter()
        .filter(|face| {
            face.path
                .rsplit_once('/')
                .is_some_and(|(above, _)| above == wanted)
        })
        .collect()
}

/// Which siblings differ from the majority, and what they differ by.
/// 哪些兄弟与多数派不同，以及差在哪。
///
/// Decidable, and stated as such: a name **most** siblings call and this one does not; a name this
/// one calls and **no** other sibling calls. The majority line is more than half.
/// 可判定，而且就这么说：**多数**兄弟都调、而它没调的名字；以及它调了、**别的兄弟都没调**的名字。
/// 多数线取"超过一半"。
/// Which verb this comparison's rows use: the `api` signal talks about calls, the `shape` signal
/// about declarations. One core, two subjects — the alternative is a second copy of the majority
/// arithmetic, and two copies of a majority rule is exactly how the two signals would come to
/// disagree about what "outlier" means.
/// 这次比较的行用哪个动词：`api` 信号说的是**调用**，`shape` 信号说的是**声明**。一个内核、两种主语
/// ——另一种做法是把多数表决的算术再抄一份，而两份多数规则正是两个信号会对"离群"的定义产生分歧的来路。
#[derive(Clone, Copy)]
pub(super) enum Wording {
    /// The `api` signal: names this face's file calls.
    /// `api` 信号：这个面的文件调用了哪些名字。
    Calls,
    /// The `shape` signal: fields this face declares.
    /// `shape` 信号：这个面声明了哪些字段。
    Declares,
}

impl Wording {
    fn missing(self, name: &str) -> String {
        match self {
            Wording::Calls => format!("does not call `{name}`, which the other siblings call"),
            Wording::Declares => {
                format!("does not declare `{name}`, which the other siblings declare")
            }
        }
    }

    fn extra(self, name: &str) -> String {
        match self {
            Wording::Calls => format!("calls `{name}`, which no sibling calls"),
            Wording::Declares => format!("declares `{name}`, which no sibling declares"),
        }
    }
}

/// Answer "which of these siblings differs?" for one parent.
/// 对一个父级回答"这些兄弟里谁不一样"。
pub(crate) fn consistency(root: &Path, arguments: &Value) -> Result<String, String> {
    // Two questions, one sibling set. `parent` asks "do these children agree with each other";
    // `specimen` asks "do they follow the shape the ledger certifies" — the baseline there is the
    // entry in force rather than the majority, which is why they are separate modes rather than one
    // flag on the other.
    // 两个问题，同一份同族集合。`parent` 问"这些子级彼此一致吗"；`specimen` 问"它们照台账采信的那个形状
    // 做了吗"——后者的基准是生效条目而不是多数派，因此它们是两种模式，而不是一个模式上的开关。
    if let Some(specimen) = arguments
        .get("specimen")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|specimen| !specimen.is_empty())
    {
        if arguments.get("parent").is_some() {
            return Err(
                "consistency takes `parent` or `specimen`, not both: `parent` compares siblings \
                 against their own majority, while `specimen` derives the parent from the anchor and \
                 compares them against the shape the ledger certifies — accepted shape: \
                 {\"specimen\":\"<anchor>\",\"root\":\"<path>\"}"
                    .to_owned(),
            );
        }
        return specimen_comparison(root, specimen);
    }
    let parent = arguments
        .get("parent")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|parent| !parent.is_empty())
        .ok_or_else(|| {
            "consistency needs `parent` or `specimen`: `parent` is the logical path whose children \
             to compare, `specimen` is a ledger anchor to compare its siblings against — accepted \
             shape: {\"parent\":\"<logical path>\",\"by\":\"api|kind|source|shape\",\"root\":\"<path>\"} \
             or {\"specimen\":\"<anchor>\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    // Which signals to run. **Absent `by` runs both `api` and `shape`**: a family outlier can be a
    // sibling calling something the rest do not, or a sibling **declaring a field the rest do not**,
    // and the second shape used to be invisible — a family whose only difference was one extra
    // declared field came back `outliers: 0 of 3`, so the one call this tool promises ("one call
    // names the outlier") did not name it. An explicit `by` still runs exactly one signal.
    // 跑哪些信号。**`by` 缺省时同时跑 `api` 与 `shape`**：同族的离群者可能是"调用了别人不调的"，也可能是
    // "**声明了别人不声明的字段**"，而後一种形状过去是看不见的——一个唯一差别就是多一行声明的同族回的是
    // `outliers: 0 of 3`，于是这个工具承诺的那一次调用（"一次调用点名离群者"）并没有点到它。显式给 `by`
    // 时仍然只跑一个信号。
    let mut remaining: Vec<String> = Vec::new();
    let mut any_outlier = false;
    let mut unread_rows = 0usize;
    let requested = arguments
        .get("by")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let Some(one) = requested
        .as_deref()
        .filter(|one| !matches!(*one, "api" | "kind" | "source" | "shape"))
    {
        return Err(format!(
            "consistency takes `by` as `api`, `kind`, `source` or `shape`, got `{one}` — \
                 accepted shape: {{\"parent\":\"<logical path>\",\
                 \"by\":\"api|kind|source|shape\",\"root\":\"<path>\"}}"
        ));
    }
    let signals: Vec<&str> = match requested.as_deref() {
        Some(one) => vec![one],
        None => vec!["api", "shape"],
    };
    let members = match crate::mcp::workspace::scope(root)? {
        Scope::Package(namespace) => vec![Member::package(root, namespace)],
        Scope::Workspace(members) => members,
        Scope::Unresolvable(reason) => {
            return Ok(format!(
                "{reason}\nno faces to compare: the tree could not be read\n"
            ));
        }
    };
    let mut sections: Vec<String> = Vec::new();
    for member in &members {
        let (evidence, faces, unparsable) = member_faces(member, RECORD_CANNOT_ANSWER)?;
        sections.push(evidence);
        // A file the derivation refused is not in the sibling set, so a comparison that stayed
        // silent about it would answer "these are all the siblings" about a tree that dropped one.
        // The drop is the tree's own line; it is carried here rather than re-derived.
        // 推导拒绝掉的文件不在同族集合里，因此对这件事保持沉默的比较，会对着一个**丢掉了一个文件**的树
        // 答出"兄弟就这些"。丢掉的实情是树自己那一行，这里只是搬运，不重新推导。
        if !unparsable.trim().is_empty() {
            sections.push(format!(
                "member {}: {} (such a file is not in the sibling set below)",
                member.name,
                unparsable.trim_end()
            ));
        }
        let set = siblings(&faces, parent);
        // Audit `W6-2`: only this family's own directories, read from **this member's** root.
        // 审计 `W6-2`：只读这一家自己的目录，且从**本成员的**根读起。
        let directories = crate::mcp::source_index::face_directories(set.iter().copied());
        let sources = LazySources::new(&member.dir);
        let record_shapes = if signals.contains(&"shape")
            && crate::mcp::build_evidence::build_evidence(&member.dir).current
        {
            shape_names_from_record(&member.dir)
        } else {
            None
        };
        if !sections.is_empty() && signals.contains(&"shape") {
            sections.push(shape_source_line(&member.name, record_shapes.is_some()));
        }
        if set.is_empty() {
            // A member with no face under it contributes nothing, and the round measured this
            // line being read as an answer rather than as an early exit: a session on a tree
            // with no registered face spent steps on family tools whose every row said "0
            // members". Emit the line only when the whole family is empty, so one sentence
            // states the shape instead of one sentence per member.
            // 底下没有面的成员不贡献任何信息，而那一轮量到这一行被当成答案而不是早退：在没有注册面
            // 的树上，一个会话为每个"0 成员"的家族工具行花了步数。只在**整个家族都空**时印一句，
            // 让一句话说清形状，而不是每个成员一句。
            continue;
        }
        // Audit `W6-2`: a family with thousands of members used to print **one row per member** —
        // measured at 2,808,408 bytes on a 50,000-face workspace, against the acceptance's "响应拍平".
        // The tool's own promise is "one call names the outlier", so the default keeps the census, the
        // outliers and their excerpts, and the per-member value rows are **bought** with `full: true`.
        // 审计 `W6-2`：成员上千的家族过去**每个成员印一行**——在五万面的工作区上实测 2,808,408 字节，
        // 与验收的"响应拍平"相反。本工具自己的承诺是"一次调用点名离群者"，因此默认保留普查、离群者与它们
        // 的原文片段，而"每个成员的取值行"由 `full: true` **买**。
        let full = arguments.get("full").and_then(Value::as_bool) == Some(true);
        for signal in signals.iter().copied() {
            let mut rows: Vec<String> = vec![format!(
                "family {parent} · member {} · {} member(s) · by {signal}",
                member.name,
                set.len()
            )];
            let mut sets: Vec<(String, BTreeSet<String>)> = Vec::new();
            let mut values: Vec<(String, String)> = Vec::new();
            for face in &set {
                let label = face
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&face.path)
                    .to_owned();
                let value = match signal {
                    "kind" => face.kind.clone(),
                    "source" => face.source.clone(),
                    // The `shape` signal: **which of the fields this comparison reads the face
                    // declares at all**. Presence, not value — a family's faces legitimately state
                    // different names, summaries and parts types, so comparing values would call
                    // every family an outlier; what is a drift is declaring a field the rest do not.
                    // Read through the kernel's own parser, the same reader `specimen` uses.
                    // `shape` 信号：**这个面到底声明了这次比较读的哪些字段**。只看存在，不看取值——同族
                    // 的面写出不同的名字、摘要与零件类型正是设计，比值会让每一个同族都成离群；而"声明了别人
                    // 不声明的字段"才是漂移。取值经内核自己的解析器读出，与 `specimen` 用的是同一个读取器。
                    "shape" => {
                        // Record first, sources otherwise; the `shapes from` line says which.
                        // 先记录、否则源码；`shapes from` 那行说明用的是哪一个。
                        let recorded = record_shapes.as_ref().and_then(|shapes| {
                            shapes
                                .get(&crate::mcp::source_index::indexed_path(&face.source))
                                .cloned()
                        });
                        if let Some(names) = recorded {
                            sets.push((label.clone(), names));
                            rows.push(format!(
                                "  {label:<24} {} field(s){}",
                                sets.last().map_or(0, |(_, names)| names.len()),
                                if sets.last().is_some_and(|(_, names)| names.is_empty()) {
                                    ": none".to_owned()
                                } else {
                                    String::new()
                                }
                            ));
                            continue;
                        }
                        let text = sources.text_of(&directories, &face.source);
                        let names = match text.as_deref().map(one_face) {
                            Some(Ok(parsed)) => declared_shape(&parsed)
                                .into_iter()
                                .map(|field| field.key.to_owned())
                                .collect::<BTreeSet<String>>(),
                            Some(Err(reason)) => {
                                unread_rows += 1;
                                rows.push(format!("  {label:<24} unreadable: {reason}"));
                                continue;
                            }
                            None => {
                                unread_rows += 1;
                                rows.push(format!("  {label:<24} its file is not in the index"));
                                continue;
                            }
                        };
                        let shown = if names.is_empty() {
                            "none".to_owned()
                        } else {
                            names.iter().cloned().collect::<Vec<_>>().join(", ")
                        };
                        sets.push((label.clone(), names.clone()));
                        rows.push(format!("  {label:<24} {} field(s): {shown}", names.len()));
                        continue;
                    }
                    _ => {
                        // The `api` signal: the names this face's own file calls, read from the same
                        // index every other reader uses.
                        // `api` 信号：这个面自己的文件调用了哪些名字，取自其它读者用的同一份索引。
                        // "This sibling's own text" is the whole object directory, not only the face
                        // file: the face file of a framework object is often one macro invocation, and
                        // a macro body's text is deliberately not read as calls (the kernel's own rule).
                        // "这个兄弟自己的文本"是整个对象目录，而不只是那个面文件：框架对象的面文件常常只有一次
                        // 宏调用，而宏体的文本按内核自己的规则**不算调用**。
                        let directory = format!("src/{}", face.source.trim_start_matches("./"));
                        let directory = directory
                            .rsplit_once('/')
                            .map(|(dir, _)| dir.to_owned())
                            .unwrap_or(directory);
                        let names: BTreeSet<String> =
                            sources.with_files(&directories, |files| {
                                files
                                    .iter()
                                    .filter(|source| {
                                        // `SourceFile::relative` is already spelled from the package
                                        // root (`src/…`), which is why the directory is built with one
                                        // `src/` and the comparison does not add a second.
                                        // `SourceFile::relative` 本来就是从包根拼的（`src/…`），因此目录
                                        // 只加一次 `src/`，比较时不再加第二次。
                                        source.relative.starts_with(&format!("{directory}/"))
                                    })
                                    .flat_map(|source| source.functions.iter())
                                    .flat_map(|function| function.calls.iter().cloned())
                                    .collect()
                            })?;
                        let shown = names.iter().cloned().collect::<Vec<_>>().join(", ");
                        sets.push((label.clone(), names));
                        rows.push(format!(
                            "  {label:<24} {} call(s): {}",
                            shown.split(", ").filter(|part| !part.is_empty()).count(),
                            if shown.is_empty() {
                                "none".to_owned()
                            } else {
                                shown
                            }
                        ));
                        continue;
                    }
                };
                rows.push(format!("  {label:<24} {value}"));
                values.push((label.clone(), value.clone()));
            }
            let mut outliers = 0usize;
            if signal == "api" || signal == "shape" {
                let wording = if signal == "api" {
                    Wording::Calls
                } else {
                    Wording::Declares
                };
                let found = deviations(&sets, wording);
                for (sibling, notes) in &found {
                    rows.push(format!("  outlier     {sibling}: {}", notes.join("; ")));
                    // W4-6: the names are in the row; the lines that carry them are what the reader
                    // would open files for. A name this sibling calls but no other does is shown in
                    // **its** file; a name the others call and this one does not is shown where the
                    // majority states it, because an absent line cannot be quoted.
                    // W4-6：名字已经在那一行里，而承载它们的源码行才是读者要打开文件去找的东西。本兄弟调了、
                    // 别人都没调的名字，在**它自己**的文件里显示；别人都调、它没调的名字，显示在多数派写它的
                    // 地方——缺掉的那一行没有东西可引。
                    let mut shown: Vec<String> = Vec::new();
                    for note in notes {
                        if shown.len() >= EXCERPT_LIMIT {
                            break;
                        }
                        let Some(name) = backticked(note) else {
                            continue;
                        };
                        let target = if note.starts_with("calls") || note.starts_with("declares") {
                            Some(sibling.clone())
                        } else {
                            set.iter().map(|face| label_of(face)).find(|other| {
                                other != sibling
                                    && sets.iter().any(|(label, names)| {
                                        label == other && names.contains(name)
                                    })
                            })
                        };
                        let Some(target) = target else {
                            continue;
                        };
                        let Some(face) = set.iter().find(|face| label_of(face) == target) else {
                            continue;
                        };
                        let Some(text) = sources.text_of(&directories, &face.source) else {
                            continue;
                        };
                        let needles = [name.to_owned()];
                        shown.extend(lines_naming(
                            &text,
                            &format!("src/{}", face.source.trim_start_matches("./")),
                            &needles,
                            EXCERPT_LIMIT.saturating_sub(shown.len()),
                        ));
                    }
                    if let Some(command) = read_command(&shown) {
                        remaining.push(command);
                    }
                    rows.extend(shown);
                }
                outliers = found.len();
                any_outlier |= outliers > 0;
            } else {
                // Counted **once** (audit `T3`), for the same reason `deviations` is: the tie test
                // below used to walk the whole table per sibling — `counts.values().max()` and a full
                // filter — which is O(siblings^2) and measured as the whole cost of the answer
                // (10.4 s on a 2,500-face family, *worse* than the shape signal it was supposed to
                // beat). Two facts computed here replace both walks: the top count, and whether more
                // than one value holds it.
                // **只数一遍**（审计 `T3`），理由与 `deviations` 相同：下面那个平局判定过去对**每个兄弟**都
                // 走一遍整表——`counts.values().max()` 加一次全量过滤——是 O(兄弟数²)，而实测它就是整个答案的
                // 成本（2,500 面家族上 10.4 s，比它本该胜过的那条 `shape` 还慢）。这里算出的两个事实取代那两次
                // 遍历：最高的计数，以及是否有不止一个取值拿到它。
                let mut counts: std::collections::BTreeMap<&str, usize> =
                    std::collections::BTreeMap::new();
                for (_, value) in &values {
                    *counts.entry(value.as_str()).or_default() += 1;
                }
                let majority = counts
                    .iter()
                    .max_by_key(|(value, count)| (**count, std::cmp::Reverse(value.len())))
                    .map(|(value, _)| *value)
                    .unwrap_or_default();
                let top = counts.values().copied().max().unwrap_or(0);
                let tied_at_top = counts.values().filter(|count| **count == top).count() > 1;
                for (sibling, value) in &values {
                    if value != majority {
                        // Ties are not outliers: with no single majority value there is nothing to call
                        // a deviation.
                        // 平局不算离群：没有唯一的多数值时，就没有可称为偏离的东西。
                        if tied_at_top && counts.get(value.as_str()) == Some(&top) {
                            continue;
                        }
                        rows.push(format!(
                            "  outlier     {sibling}: `{signal}` is `{value}` while the majority says `{majority}`"
                        ));
                        outliers += 1;
                    }
                }
            }
            any_outlier |= outliers > 0;
            rows.push(format!("outliers: {outliers} of {}", sets.len()));
            if outliers > 0 {
                // A11: the majority line is a **candidate**, never a verdict. When most siblings
                // drift together the row above names the one that kept the contract, so the answer
                // has to say out loud which way it is reading the family — otherwise a reader takes
                // "outlier" for "wrong", and the minority's correctness is the one thing this
                // comparison cannot see on its own.
                // A11：多数线是**候选**，绝不是判决。多数兄弟一起漂移时，上面那一行点的正是守住合同的那个，
                // 因此答案必须说出它按哪个方向读这个家族——否则读者会把"离群"当成"错"，而少数派是对的这件事
                // 恰恰是这份比较自己看不见的。
                // Not indented like a member row: the truncation withholds those, and this caveat
                // is exactly what must survive the bound (A11).
                // 有意不按成员行缩进：截断会扣下那些行，而这句提醒恰恰是必须越过封顶活下来的东西（A11）。
                rows.push(
                    "candidate: the majority is a candidate, not a verdict — if most of these \
                     siblings drifted together, the row above names the one that kept the contract, \
                     and the shared declaration decides which is which. This comparison reads \
                     siblings; a sibling is not a contract"
                        .to_owned(),
                );
            }
            if !full {
                let plain = rows.iter().filter(|row| is_member_row(row)).count();
                if plain > 0 {
                    rows.retain(|row| !is_member_row(row));
                    rows.push(crate::mcp::truncation::withheld(
                        plain,
                        sets.len(),
                        0,
                        "member row(s)",
                        "pass `full: true` for every member's own value",
                    ));
                }
            }
            sections.push(rows.join("\n"));
        }
    }
    sections.push(if crate::mcp::session::first_time(&format!(
        "consistency-bounds:{}",
        root.display()
    )) {
        "not covered by this comparison: it reads the **derived** tree's sibling set and each \
         sibling's own text, so a convention that lives in a shared helper, in generated code, or \
         in a parent rule is not visible here; and `api` compares the names called, not the units \
         or the arithmetic — an outlier is a place to look, not a defect; and a call written inside \
         a macro body is not read as a call (the kernel's rule), so an object whose whole body is one \
         macro invocation reads as calling nothing — **on such a tree `api` has nothing to compare, \
         but `by: shape` does: it compares which of the fields this comparison reads each \
         sibling declares at all, so it still names a sibling that declares an extra field; \
         `by: kind` / `by: source` compare the declared value, `specimen` against a \
         certified shape**"
            .to_owned()
    } else {
        "not covered: unchanged from this session's earlier comparison — the derived-tree caveat \
         and the meaning of an outlier are the ones printed then"
            .to_owned()
    });
    sections.push(
        "next   `read {path, line}` for the outlier's body, `explain {node}` for its declared fields, \
           and `by: shape` when the difference is a **declaration** rather than a call (an \
           extra declared field does not show up in `api`)"
            .to_owned(),
    );
    // W4-5: the census first, so the answer carries the tree's size instead of a second call, and
    // the closure last, judged conservatively — a family nobody deviates from and no unreadable file
    // is the one case that closes.
    // W4-5：普查在最前，答案自带树的大小而不必再花一次调用；闭合行在最后，判定从保守——没有任何兄弟
    // 偏离、也没有读不了的文件，才是唯一闭合的情形。
    if let Some(census) = tree_census(root) {
        sections.insert(0, census);
    }
    sections.push(
        crate::mcp::tools::closure(!any_outlier && unread_rows == 0, &remaining)
            .trim_end()
            .to_owned(),
    );
    Ok(format!("{}\n", sections.join("\n")))
}
