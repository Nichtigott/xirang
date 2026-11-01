//! The shape comparison's vocabulary and the excerpt reader behind it.
//! 形状比较的词汇表，以及它背后的摘录读取器。
//!
//! Split out of `consistency.rs` because that file is under a 600-code-line ratchet and the ledger
//! comparison is what pushed it over. What lives here is the **decidable half**: which fields a
//! specimen is compared on, how one spelling of a trait field is matched under one key, and how the
//! lines a deviation names are quoted back (audit `F1`, `W4-6`). The comparisons themselves — the
//! sibling set, the ledger verdict, the two signals — stay in the parent, because they are what the
//! tool answers with.
//! 从 `consistency.rs` 拆出来，因为那个文件受 600 代码行棘轮约束，而台账比对正是把它顶过去的东西。这里
//! 住的是**可判定的那一半**：标本按哪些字段比、trait 字段的两种拼写怎么归到一个键下、以及一处偏离点名的
//! 那些行怎么被引回来（审计 `F1`、`W4-6`）。比较本身——同族集合、台账裁定、两种信号——留在父模块，因为
//! 那才是这个工具作答的东西。
//!
//! online: an excerpt is quoted from the source file as it is now, and a face added since the build has no record row at all.

use std::path::Path;

use xirang_kernel::syntax::FaceSyntax;

use crate::build_method::FaceView;
use crate::mcp::source_index::load_sources;

/// How one shape field is compared between a specimen and a sibling.
/// 某个形状字段在标本与兄弟之间怎么比。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ShapeComparison {
    /// Only whether the sibling declares the field at all: its value is this object's own name.
    /// 只比这个兄弟到底有没有声明该字段：它的取值就是这个对象自己的名字。
    Presence,
    /// The labels the specimen states must appear in the sibling's list. Extra labels are that
    /// sibling's own business, like everything else here — the comparison is one-directional.
    /// 标本写下的标签必须出现在兄弟的清单里。多出来的标签是那个兄弟自己的事，与这里其余部分一样——比较
    /// 是单向的。
    Labels,
}

/// One declared-shape field: the **key** a comparison matches on, the spelling the declaration
/// actually wrote, and the value.
/// 一个已声明形状字段：比较据以匹配的**键**、声明**实际写下**的拼写、以及取值。
///
/// The key and the spelling are deliberately two things. The kernel accepts two spellings for a
/// trait field — `handle_contracts` (compiler-checked Rust paths) and `handle_traits` (the plain
/// labels) — and both are editable through `apply`. Reporting the *derived* spelling instead of the
/// one in the file is what audit `F1` caught: a reader was handed a name whose value could not be
/// pasted back where the declaration keeps it. So the comparison still matches on one key (a family
/// is not an outlier for choosing the other spelling), while every line a reader may copy names the
/// spelling that is actually written.
/// 键与拼写是刻意分开的两件事。内核接受两种 trait 字段拼写——`handle_contracts`（参与编译检查的 Rust
/// 路径）与 `handle_traits`（纯标签）——而两者都能经 `apply` 编辑。报**推导**拼写而不是文件里那一个，
/// 正是审计 `F1` 抓到的：读者拿到的名字，其取值放不回声明保存它的位置。因此比较仍只认一个键（同族不会
/// 因为选了另一种拼写而被判离群），而每一行**读者可能照抄**的输出都点名文件里真正写着的那个拼写。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct DeclaredField {
    /// What comparisons match on; two spellings of one field share it.
    /// 比较据以匹配的东西；同一个字段的两种拼写共用它。
    pub(crate) key: &'static str,
    /// The spelling the declaration wrote — the one a reader may paste back into `apply`.
    /// 声明写下的拼写——读者可以照抄回 `apply` 的那一个。
    pub(crate) spelling: &'static str,
    /// The rendered value.
    /// 渲染后的取值。
    pub(crate) value: String,
}

/// One entry of [`SHAPE_FIELDS`]: what to compare, under which key, and how.
/// [`SHAPE_FIELDS`] 的一项：比什么、用哪个键、怎么比。
pub(super) struct ShapeField {
    /// The key comparisons match on.
    /// 比较据以匹配的键。
    pub(crate) key: &'static str,
    /// The spellings a declaration may write, the compiler-checked one first when there are two.
    /// 声明可以写下的拼写；有两个时把参与编译检查的那个放前面。
    pub(super) spellings: &'static [&'static str],
    /// How the value is compared.
    /// 取值怎么比。
    pub(super) comparison: ShapeComparison,
}

/// The declared-shape fields a specimen is compared on, and how each one is compared.
/// 对比一个标本时看哪些已声明形状字段，以及每一项怎么比。
///
/// Every name here is a field name the kernel's own vocabulary carries (`FACE_FIELD_ORDER`), and the
/// values are read through the kernel's parser (`FaceSyntax`), so this list can name fields but
/// cannot invent or re-spell them. The two trait entries resolve the `*_contracts` spelling the same
/// way the build-time contract check does — a face that states a checked path does not state its
/// label a second time.
/// 这里的每个名字都是内核词表（`FACE_FIELD_ORDER`）携带的字段名，取值经内核解析器（`FaceSyntax`）读出，
/// 因此这张清单可以点名字段，却不能发明或改写它们。两条 trait 条目按构建期合同检查同一条规则解析
/// `*_contracts` 写法——写了受检路径的注册面不必再写一遍标签。
///
/// `parts` is presence-only because it names the object's **own** parts type (`ButtonParts`): two
/// siblings stating different ones is the design, not a drift, and comparing the value would make
/// every family an outlier.
/// `parts` 只比存在性，因为它点名的是对象**自己**的零件类型（`ButtonParts`）：两个兄弟写出不同的类型正是
/// 设计而非漂移，比值会让每一个同族都成了离群。
pub(super) const SHAPE_FIELDS: &[ShapeField] = &[
    ShapeField {
        key: "parts",
        spellings: &["parts"],
        comparison: ShapeComparison::Presence,
    },
    ShapeField {
        key: "exports",
        spellings: &["exports"],
        comparison: ShapeComparison::Labels,
    },
    ShapeField {
        key: "handle_traits",
        spellings: &["handle_contracts", "handle_traits"],
        comparison: ShapeComparison::Labels,
    },
    ShapeField {
        key: "part_traits",
        spellings: &["part_contracts", "part_traits"],
        comparison: ShapeComparison::Labels,
    },
];

/// The shape field spellings this comparison reads, for the lines that name the set.
/// 这次比较读哪些形状字段**拼写**，供点名该集合的行使用。
///
/// Every spelling is named, not only the canonical key: this line is read by someone who is about
/// to write one of them, and naming a spelling the file does not carry is the `F1` defect.
/// 每一种拼写都点名，而不只是规范键：读这一行的人正要去写其中之一，而点一个文件里没有的拼写就是
/// `F1` 那个缺陷。
pub(super) fn shape_field_names() -> Vec<&'static str> {
    SHAPE_FIELDS
        .iter()
        .flat_map(|field| field.spellings.iter().copied())
        .collect()
}

/// One field's rendered value as the labels it states.
/// 某个字段渲染后的取值，按它写下的标签拆开。
pub(super) fn shape_labels(value: &str) -> Vec<&str> {
    value
        .split(", ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .collect()
}

/// The labels a trait field carries, from either spelling, or `None` when neither is written.
/// 某个 trait 字段携带的标签（两种写法都认），两者都没写时为 `None`。
///
/// The value is always labels — the kernel derives them from the paths — while the returned
/// **spelling** is whichever one the file actually carries, so a reader can paste it back.
/// 取值永远是标签（由内核从路径派生），而返回的**拼写**是文件里真正写着的那个，读者据此可以照抄回去。
pub(super) fn trait_shape(
    face: &FaceSyntax,
    contracts: &'static str,
    labels: &'static str,
) -> Option<(&'static str, String)> {
    let wrote_contracts = face.field(contracts).is_some();
    let wrote_labels = face.field(labels).is_some();
    if !wrote_contracts && !wrote_labels {
        return None;
    }
    let value = face
        .string_list(labels)
        .map(|values| values.join(", "))
        .or_else(|| face.field(labels))
        .or_else(|| face.field(contracts))?;
    // The checked spelling wins when a face wrote both: that is the one the compiler reads, and the
    // labels beside it are the redundant half.
    // 两者都写时以受检的那个拼写为准：编译器读的是它，旁边那份标签才是多余的一半。
    let spelling = if wrote_contracts { contracts } else { labels };
    Some((spelling, value))
}

/// A `read` call for the first quoted line of one excerpt block.
/// 一个摘录块里第一行引用所对应的 `read` 调用。
///
/// The excerpt is already a coordinate (`src/x.rs:12  text`), so the call that opens it is a
/// substring away — and a command a reader has to assemble is the thing `W4-5` measured as an extra
/// step.
/// 摘录本身就是坐标（`src/x.rs:12  text`），因此打开它的调用只差一次截取——而"要读者自己拼的命令"
/// 正是 `W4-5` 量到的那多出来的一步。
pub(super) fn read_command(shown: &[String]) -> Option<String> {
    let (path, line) = shown.first()?.trim().split_once("  ")?.0.split_once(':')?;
    Some(format!("--call read --path {path} --line {line}"))
}

/// The tree census every answer about a tree can open with.
/// 每个关于一棵树的答案都可以用它开头的那行普查。
///
/// Audit `W4-5`/`W1-2`: the round measured sessions opening with `status` just to learn how big the
/// tree is, then asking the real question. The size is a fact every one of these tools already
/// holds, so it rides on the answer instead of costing a call.
/// 审计 `W4-5`/`W1-2`：那一轮量到会话先发一次 `status`、只为知道树有多大，然后再问真正的问题。这个大小是
/// 这些工具**本来就持有**的事实，因此它随答案一起走，而不是花掉一次调用。
pub(crate) fn tree_census(root: &Path) -> Option<String> {
    // Audit `T3`: this one line used to cost a **full derivation plus a full source scan** — measured
    // at 4.76 s on a 2,500-face tree, and it is printed on every `consistency` answer, including the
    // ones whose whole answer is one family. The build's record already states the faces it published
    // and which source each came from, so the census is a manifest read when the record is current.
    // 审计 `T3`：这一行过去要付一次**整树推导 + 整份源码扫描**——在 2,500 面的树上实测 4.76 s，而它印在
    // 每一条 `consistency` 答复上，包括那些整个答案就是一个家族的时候。构建的记录本来就说出了它发布过的面
    // 以及每个面来自哪份源码，因此记录新鲜时，这张普查就是一次清单读取。
    if let Some(line) = published_census(root) {
        return Some(line);
    }
    let files = load_sources(root).ok()?.len();
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let (faces, _) = crate::mcp::resolve::derived_faces(root, &namespace).ok()?;
    Some(format!("tree: {} face(s), {files} file(s)", faces.len()))
}

/// The census from the published record, when this root's record is current (audit `T3`).
/// 记录新鲜时，由已发布记录给出的普查（审计 `T3`）。
///
/// The two numbers are the same **facts** the derivation reports — how many faces the build published
/// and how many source files carry them — and the line says which reading it is, because a reader
/// comparing two trees must not have to guess whether a count came from the build or from a scan of
/// the sources.
/// 这两个数与推导报告的是同样的**事实**——构建发布了多少个面、有多少份源码承载它们——而这一行会说出它是
/// 哪一种读法，因为要对比两棵树的读者不该去猜某个计数来自构建还是来自一次源码扫描。
fn published_census(root: &Path) -> Option<String> {
    // A workspace root has no record of its own: its members do. Summing them keeps the fast path
    // for the shape this is measured on — a 20-member workspace whose root would otherwise pay a
    // 50,000-face derivation just to print one line (measured: 59 s against 6 s for the same
    // answer once the members answer).
    // 工作区根没有自己的记录：它的成员才有。把成员加起来，才能让这条快路覆盖实测的那个形状——否则一个
    // 20 成员的工作区根为了印一行字要付 50,000 个面的推导（实测：59 s，而让成员来答之后同一份答案是 6 s）。
    let roots = match crate::mcp::workspace::scope(root).ok()? {
        crate::mcp::workspace::Scope::Package(_) => vec![root.to_path_buf()],
        crate::mcp::workspace::Scope::Workspace(members) => {
            members.into_iter().map(|member| member.dir).collect()
        }
        crate::mcp::workspace::Scope::Unresolvable(_) => return None,
    };
    let mut faces = 0usize;
    let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for root in roots {
        let out = crate::mcp::build_evidence::out_dir(&root);
        if !crate::mcp::freshness::verdict(&root, &out) {
            return None;
        }
        let rows = crate::build_method::read_pruning_manifest(&out).ok()?;
        if rows.is_empty() {
            return None;
        }
        faces += rows.len();
        // Sources are keyed by member as well: two members may each carry `control/control.rs`.
        // 源码按成员做键：两个成员可能各有自己的 `control/control.rs`。
        files.extend(
            rows.iter()
                .map(|row| format!("{}/{}", root.display(), row.source)),
        );
    }
    Some(format!(
        "tree: {faces} face(s), {} source file(s) (read from the published record)",
        files.len()
    ))
}

/// One parsed face's declared shape: the shape fields it carries, in the order of [`SHAPE_FIELDS`].
/// 一个已解析注册面的已声明形状：它携带的形状字段，按 [`SHAPE_FIELDS`] 的顺序。
pub(crate) fn declared_shape(face: &FaceSyntax) -> Vec<DeclaredField> {
    let mut shape = Vec::new();
    for field in SHAPE_FIELDS {
        let declared = match field.spellings {
            [contracts, labels] => {
                trait_shape(face, contracts, labels).map(|(spelling, value)| DeclaredField {
                    key: field.key,
                    spelling,
                    value,
                })
            }
            [name, ..] => face
                .string_list(name)
                .map(|values| values.join(", "))
                .or_else(|| face.field(name))
                .map(|value| DeclaredField {
                    key: field.key,
                    spelling: name,
                    value,
                }),
            [] => None,
        };
        if let Some(declared) = declared {
            shape.push(declared);
        }
    }
    shape
}

/// One sibling's shape against the specimen's: what it lacks, and which stated labels it is missing.
/// 某个兄弟的形状对标本而言缺什么、以及缺了哪些已写下的标签。
///
/// One direction only, and that is the point: the specimen is the baseline, so the question is
/// whether it was followed, not whether a sibling invented something more. A sibling carrying a
/// field, or a label, that the specimen does not is therefore not a row here, and the boundary
/// sentence says so.
/// 只判一个方向，而这就是要点：标本是基准，因此问题是"有没有照它做"，不是"兄弟有没有多发明什么"。因此一个
/// 携带了标本没有的字段或标签的兄弟不在这里成行，而边界句会说清这一点。
///
/// Matching is on the **key**, naming is by the specimen's **spelling**: a sibling that chose the
/// other spelling of a trait field is not lacking anything, while the row for one that lacks it
/// tells the reader the name the baseline actually uses.
/// 匹配按**键**、命名按标本的**拼写**：选了 trait 字段另一种拼写的兄弟并不缺什么，而真的缺的那一个，
/// 行里给出的名字是基准真正在用的那个。
pub(super) fn shape_gaps(specimen: &[DeclaredField], sibling: &[DeclaredField]) -> Vec<String> {
    let mut notes = Vec::new();
    for field in specimen {
        let Some(other) = sibling.iter().find(|candidate| candidate.key == field.key) else {
            notes.push(format!("lacks `{}`", field.spelling));
            continue;
        };
        let comparison = SHAPE_FIELDS
            .iter()
            .find(|candidate| candidate.key == field.key)
            .map(|candidate| candidate.comparison);
        if comparison != Some(ShapeComparison::Labels) {
            continue;
        }
        let stated = shape_labels(&other.value);
        for label in shape_labels(&field.value) {
            if !stated.contains(&label) {
                notes.push(format!("lacks `{}: {label}`", field.spelling));
            }
        }
    }
    notes
}

/// How many source lines one outlier may carry.
/// 一个离群者最多可以带几行源码。
///
/// The round that measured the alternative (codegraph answered the same question with a 6,781-character
/// slab) is why this is a count and not "the file": what decides the answer is the one line that
/// deviates, and ten lines is enough to show it with its neighbours.
/// 量到另一种做法的那一轮（codegraph 对同一个问题给了 6,781 字符的整片）正是这里写"行数"而不是写"整个
/// 文件"的原因：决定答案的是偏离的那一行，而十行足够把它和邻居一起显示出来。
pub(super) const EXCERPT_LIMIT: usize = 10;

/// The field names a specimen's gap notes name.
/// 标本的差异注里点名的字段名。
///
/// `lacks \`handle_contracts: ControlHandle\`` and `lacks \`parts\`` both carry the field before the
/// colon; the label after it is a value, not a name to search the source for.
/// `lacks \`handle_contracts: ControlHandle\`` 与 `lacks \`parts\`` 都在冒号前带字段名；冒号后那个是取值，
/// 不是要在源码里找的名字。
pub(super) fn gap_fields(gaps: &[String]) -> Vec<String> {
    gaps.iter()
        .filter_map(|gap| backticked(gap))
        .map(|named| {
            named
                .split_once(':')
                .map_or(named, |(field, _)| field)
                .trim()
                .to_owned()
        })
        .collect()
}

/// The first backticked word in a note, when it has one.
/// 一条注里第一个反引号包起来的词（有时）。
pub(super) fn backticked(note: &str) -> Option<&str> {
    let start = note.find('`')? + 1;
    let rest = &note[start..];
    let end = rest.find('`')?;
    Some(rest[..end].trim())
}

/// The label one face is reported under: its own path's last segment.
/// 一个面被报告时用的标签：它自己路径的最后一段。
pub(super) fn label_of(face: &FaceView) -> String {
    face.path
        .rsplit('/')
        .next()
        .unwrap_or(&face.path)
        .to_owned()
}

/// Up to `limit` lines of one file's text that mention any needle, as `path:line  text`.
/// 一份文件文本里提到任一 needle 的最多 `limit` 行，形如 `path:line  text`。
///
/// A line is trimmed and cut at [`LINE_LIMIT`] characters: the point is the decisive line, and one
/// generated declaration can be a thousand characters wide — quoting it whole would put back the
/// slab this excerpt exists to replace.
/// 每行去掉首尾空白、并在 [`LINE_LIMIT`] 字符处截断：要点是那一行决定性的内容，而一条生成出来的声
/// 明可以宽达上千字符——整条引用会把这段摘录本要替代掉的整片又装回来。
pub(super) fn lines_naming(
    text: &str,
    path: &str,
    needles: &[String],
    limit: usize,
) -> Vec<String> {
    if limit == 0 || needles.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if rows.len() >= limit {
            break;
        }
        if !needles.iter().any(|needle| line.contains(needle.as_str())) {
            continue;
        }
        let trimmed = line.trim();
        let shown = if trimmed.chars().count() > LINE_LIMIT {
            trimmed.chars().take(LINE_LIMIT).collect::<String>() + "…"
        } else {
            trimmed.to_owned()
        };
        rows.push(format!("    {path}:{}  {shown}", index + 1));
    }
    rows
}

/// How wide one quoted source line may be, in characters.
/// 引用的一行源码最多多宽（字符）。
pub(super) const LINE_LIMIT: usize = 200;

/// The registration declaration one file carries, as quoted lines, up to `limit`.
/// 一份文件携带的那条注册声明，按行引用，最多 `limit` 行。
///
/// It exists for the deviation that has **no line of its own**: a field the face does not declare
/// cannot be quoted, so what a reader gets instead is the declaration it does have. The block runs
/// from the macro invocation to its closing brace, which is the whole declaration by this grammar's
/// own shape.
/// 它是为**自己没有行**的那种偏离而存在的：一个面没声明的字段引不出来，于是读者拿到的是它**确实**有的那条
/// 声明。块从宏调用起到它的收尾大括号，按这套语法自己的形状，那就是整条声明。
pub(super) fn declaration_block(text: &str, path: &str, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if !inside {
            if !line.contains('!') || !line.contains('{') {
                continue;
            }
            inside = true;
        }
        let trimmed = line.trim();
        let shown = if trimmed.chars().count() > LINE_LIMIT {
            trimmed.chars().take(LINE_LIMIT).collect::<String>() + "…"
        } else {
            trimmed.to_owned()
        };
        rows.push(format!("    {path}:{}  {shown}", index + 1));
        if rows.len() >= limit {
            break;
        }
        if trimmed.starts_with('}') {
            break;
        }
    }
    rows
}

/// One rendered shape, or the line that says there was none to read.
/// 渲染后的形状；无可读字段时给出那一行说明。
pub(super) fn shape_line(shape: &[DeclaredField]) -> String {
    if shape.is_empty() {
        return format!(
            "none of the fields this comparison reads ({})",
            shape_field_names().join(", ")
        );
    }
    shape
        .iter()
        .map(|field| format!("{} `{}`", field.spelling, field.value))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The repair spelled as "declare this in that file", for a face the write path may not rewrite.
/// 把修复拼成"在那个文件里声明这个"，用于写入路径不可以重写的面。
///
/// Returns `None` for a deviation that is not a missing field (a missing **label** inside a list the
/// sibling already declares), for the reason [`repair_request`] gives.
/// 对"不是缺字段"的偏离（兄弟已声明的列表里缺某个**标签**）回 `None`，理由见 [`repair_request`]。
pub(super) fn manual_repair(
    specimen: &[DeclaredField],
    gaps: &[String],
    source: &str,
    kind: &str,
) -> Option<String> {
    let mut wanted: Vec<String> = Vec::new();
    for gap in gaps {
        let named = gap.trim_start_matches("lacks `").trim_end_matches('`');
        if named.contains(':') {
            continue;
        }
        if let Some(value) = repair_value(specimen, named, kind) {
            wanted.push(format!("{named}: {value}"));
        }
    }
    if wanted.is_empty() {
        return None;
    }
    Some(format!(
        "this face is hand-written, so no `edit` request is handed back (`edit` rewrites only \
         generated faces) — add {} to src/{} by hand, or adopt the face first",
        wanted.join(", "),
        source.trim_start_matches("./")
    ))
}

/// The write-path request that pulls one deviating sibling back to the specimen's shape (audit `W5-4`).
/// 把某个偏离的兄弟拉回标本形状的那条写入路径请求（审计 `W5-4`）。
///
/// Returns `None` when the deviation is **not** expressible as "set this field": a gap that names a
/// missing **label** inside a list the sibling already declares needs the label appended to that
/// list, and inventing a whole-field rewrite there would replace the sibling's other labels with the
/// specimen's — a repair that quietly breaks something else. Those rows keep the label they named and
/// no request; this one says why rather than guessing.
/// 偏离**不能**表达成"把这个字段设成什么"时回 `None`：一条点名"兄弟已声明的列表里缺某个**标签**"的
/// gap，需要把该标签追加到那个列表上；在那里编一条整字段重写，会把兄弟自己的其它标签一并换成标本的
/// ——那是一次悄悄弄坏别的东西的修复。那种行保留它点名的标签、不给请求；这里说明理由而不是猜。
///
/// The request is built as JSON rather than as a hand-spelled command line, so escaping is right by
/// construction: it is meant to be **pasted**, and a line a reader has to repair before pasting is
/// worse than no line.
/// 请求按 JSON 构造而不是手拼命令行，因此转义天然正确：它是给人**粘贴**的，而一行还需要读者先修好才能
/// 粘贴的字，比没有这一行更糟。
pub(super) fn repair_value(specimen: &[DeclaredField], name: &str, kind: &str) -> Option<String> {
    // A field compared **by presence alone** names the object's own type, so the specimen's value is
    // not the answer: copying `parts: ButtonParts` into the slider would be a repair that breaks the
    // file it repairs. The value is the sibling's own, by the scaffold's convention `<Kind>Parts`.
    // 一个**只按存在**比较的字段点名的是对象自己的类型，因此标本的取值不是答案：把 `parts: ButtonParts`
    // 抄进 slider，是一次弄坏被修文件的修复。取值取兄弟自己的，按脚手架 `<Kind>Parts` 的约定。
    let field = specimen.iter().find(|field| field.spelling == name)?;
    let presence_only = SHAPE_FIELDS
        .iter()
        .find(|candidate| candidate.key == field.key)
        .map(|candidate| candidate.comparison)
        == Some(ShapeComparison::Presence);
    Some(if presence_only {
        format!("{kind}Parts")
    } else {
        field.value.clone()
    })
}

pub(super) fn repair_request(
    specimen: &[DeclaredField],
    face: &crate::build_method::FaceView,
    gaps: &[String],
    text: Option<&str>,
) -> Option<serde_json::Value> {
    // The write path's own question, asked before a request is handed back: is this file XiRang's
    // to rewrite? The marker is the kernel's text contract (`lexicon::GENERATED_MARKER`), not a
    // literal spelled here — and a file the index could not read is not one to claim either way.
    // 交回请求之前先问写入路径自己的问题：这个文件是 XiRang 可以重写的吗？那个标记是内核的文本契约
    // （`lexicon::GENERATED_MARKER`），不是在这里拼的字面量——而索引读不到的文件，两种情况都不该替它断言。
    let text = text?;
    if !text
        .lines()
        .any(|line| line.trim() == xirang_kernel::lexicon::GENERATED_MARKER)
    {
        return None;
    }
    let mut fields = serde_json::Map::new();
    for gap in gaps {
        // A field-level gap is ``lacks `exports` ``; a label-level one is
        // ``lacks `exports: control.render` `` and is deliberately left alone.
        // 字段级 gap 是 ``lacks `exports` ``；标签级的是 ``lacks `exports: control.render` ``，有意不动。
        let named = gap.trim_start_matches("lacks `").trim_end_matches('`');
        if named.contains(':') {
            continue;
        }
        let Some(value) = repair_value(specimen, named, &face.kind) else {
            continue;
        };
        fields.insert(named.to_owned(), serde_json::Value::String(value));
    }
    if fields.is_empty() {
        return None;
    }
    // The key is `node`, not `face`: `edit` addresses a face by its **logical path** (or its
    // identity), which is the thing this comparison already speaks in — the write path's own refusal
    // is what taught this line that, and it hands back the accepted shape so the next reader does not
    // have to discover it the same way.
    // 键是 `node` 而不是 `face`：`edit` 按**逻辑路径**（或身份）指认一个面，而那正是这次比对已经在说的
    // 东西——写入路径自己的拒绝教会了这一行，而它把可接受形状一并交回，好让下一个读者不必用同样的方式发现。
    Some(serde_json::json!({
        "action": "edit",
        "node": face.path,
        "fields": fields,
        "apply": true,
    }))
}

/// Whether one rendered line is a plain per-member value row (audit `W6-2`).
/// 某一行是不是"每个成员的取值行"（审计 `W6-2`）。
///
/// The family header, the outlier rows and the excerpt lines each carry a word of their own
/// (`family`, `outlier`, `src/…`); too wide drops the outliers, too narrow leaves the payload unbounded.
/// 家族头、离群行与原文行各有自己的词（`family`、`outlier`、`src/…`）；太宽会连离群者一起丢掉，太窄则载荷不封顶。
pub(super) fn is_member_row(line: &str) -> bool {
    line.starts_with("  ")
        && !line.starts_with("   ")
        && !line.starts_with("  outlier")
        && !line.trim_start().starts_with("src/")
}

/// The declared field names per source, from the build's own declaration record (audit `W6-2`).
/// 按源码路径索引的"声明了哪些字段"，取自构建自己的声明记录（审计 `W6-2`）。
///
/// The record is keyed by the source path **relative to `src/`** (`FaceView.source`'s own spelling),
/// and one row per declared field; a face that declares none carries a `-` row and maps to the empty
/// set, which is a different answer from "the record has never heard of this face" — the distinction
/// the caller relies on when it falls back to the sources.
/// 记录按**相对 `src/`** 的源码路径做键（也就是 `FaceView.source` 自己的拼写），每个声明字段一行；什么都
/// 没声明的面带一行 `-`、映射到空集合——这与"记录从没听说过这个面"是两个不同的答案，而调用方正是靠这个
/// 区别决定要不要回退到源码。
///
/// `None` when the record is unreadable or empty: a partial record would let the comparison judge a
/// family against a smaller one, and the sources are the honest answer there.
/// 记录读不了或为空时回 `None`：残缺的记录会让比对拿一个更小的家族去比，那种情况下源码才是诚实的答案。
pub(super) fn shape_names_from_record(
    member: &std::path::Path,
) -> Option<std::collections::BTreeMap<String, std::collections::BTreeSet<String>>> {
    let out = crate::mcp::build_evidence::out_dir(member);
    let rows = crate::build_method::read_shape_manifest(&out).ok()?;
    if rows.is_empty() {
        return None;
    }
    let mut shapes: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for row in rows {
        // Keyed in the **index's** spelling (`src/…`), which is what the caller looks faces up with:
        // the record spells the same file without that prefix, and using its spelling here made every
        // lookup miss and quietly fall back to reading the file.
        // 按**索引的**拼写（`src/…`）做键，也就是调用方查面时用的那一种：记录里同一个文件没有那个前缀，
        // 用它的拼写做键会让每次查询都落空、悄悄退回读文件。
        let entry = shapes
            .entry(crate::mcp::source_index::indexed_path(&row.source))
            .or_default();
        if row.field != "-" {
            entry.insert(row.field);
        }
    }
    Some(shapes)
}

/// The sibling sources, read at most once and **only if something asks for them** (audit `W6-2`).
/// 兄弟源码，至多读一次，而且**只有在有人要时才读**（审计 `W6-2`）。
///
/// The shape signal can be answered from the build's declaration record, in which case reading the
/// family's files to then use none of them is the work step three exists to remove — measured on one
/// 2,501-face member: eager reading made the record path 12.4 s against 12.6 s derived, i.e. no
/// saving. `api` (and the excerpts of outliers) still ask, and they pay once per member.
/// 形状信号可以由构建的声明记录回答，那种情况下为"一个都不用"而读这个家族的文件，正是第③步要拿掉的工作
/// ——在一个 2,501 面的成员上实测：急切读取让记录路径 12.4 s、推导 12.6 s，等于没省。`api`（以及离群者的
/// 原文片段）仍然要，它们每个成员付一次。
pub(super) struct LazySources<'a> {
    member: &'a std::path::Path,
    held: std::cell::RefCell<Option<Vec<crate::mcp::source_index::SourceFile>>>,
}

impl<'a> LazySources<'a> {
    /// A reader that has not opened anything yet.
    /// 一个还什么都没打开的读取方。
    pub(super) fn new(member: &'a std::path::Path) -> Self {
        Self {
            member,
            held: std::cell::RefCell::new(None),
        }
    }

    /// Read the listed directories, unless they were already read.
    /// 读列出的那些目录，除非已经读过。
    pub(super) fn ensure(&self, directories: &[String]) -> Result<(), String> {
        if self.held.borrow().is_none() {
            let loaded = crate::mcp::source_index::load_directories(self.member, directories)?;
            *self.held.borrow_mut() = Some(loaded);
        }
        Ok(())
    }

    /// The text of one face's own file, reading the directories on first use.
    /// 某个面自己那个文件的文本；首次使用时才读目录。
    pub(super) fn text_of(&self, directories: &[String], face_source: &str) -> Option<String> {
        self.ensure(directories).ok()?;
        let wanted = format!("src/{}", face_source.trim_start_matches("./"));
        self.held
            .borrow()
            .as_ref()?
            .iter()
            .find(|file| file.relative == wanted)
            .map(|file| file.source.clone())
    }

    /// Run `read` over every held file, reading the directories on first use.
    /// 对每个已持有的文件跑 `read`；首次使用时才读目录。
    pub(super) fn with_files<T>(
        &self,
        directories: &[String],
        read: impl FnOnce(&[crate::mcp::source_index::SourceFile]) -> T,
    ) -> Result<T, String> {
        self.ensure(directories)?;
        let held = self.held.borrow();
        Ok(read(held.as_deref().unwrap_or_default()))
    }
}

/// The line that says **which tree** answered the shape signal (audit `W6-2`).
/// 说明**哪棵树**回答了形状信号的那一行（审计 `W6-2`）。
///
/// An answer that read the record and an answer that derived are both correct and are not the same
/// claim, so the section states which one it used — the same rule every other reader in this bridge
/// follows when it says `tree published from …` or `tree derived now …`.
/// 读记录的答案与推导的答案都对，但不是同一个主张，因此本节说出它用的是哪一个——与本桥其它每个读者说出
/// `tree published from …` / `tree derived now …` 是同一条规则。
pub(super) fn shape_source_line(member: &str, from_record: bool) -> String {
    format!(
        "shapes from  member {member}: {}",
        if from_record {
            "the build's declaration record (shape_manifest.tsv), current"
        } else {
            "the sources, derived now (no current declaration record)"
        }
    )
}

pub(crate) fn deviations(
    sets: &[(String, std::collections::BTreeSet<String>)],
    wording: super::Wording,
) -> Vec<(String, Vec<String>)> {
    let total = sets.len();
    // How many siblings state each name, **counted once** (audit `W6-2`). The per-name and per-sibling
    // scans this replaces were both O(sets) each, inside loops over names and siblings — i.e.
    // O(sets^2 x names) for the whole call, which on a 2,500-member family is tens of millions of
    // `contains` calls and measured as the entire cost of the answer (the record path and the derived
    // path were within 0.7 s of each other because neither was the wall; this was).
    // 每个名字被多少兄弟声明，**只数一次**（审计 `W6-2`）。它替换掉的两处扫描各自是 O(sets)，又都套在
    // 遍历名字与遍历兄弟的循环里——整次调用是 O(sets² × names)：在 2,500 成员的家族上是数千万次
    // `contains`，而实测它就是整个答案的成本（记录路径与推导路径只差 0.7 s，因为两者都不是那堵墙，它才是）。
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (_, names) in sets {
        for name in names {
            *counts.entry(name.as_str()).or_default() += 1;
        }
    }
    let shared: Vec<&str> = counts
        .iter()
        .filter(|(_, count)| **count * 2 > total)
        .map(|(name, _)| *name)
        .collect();
    let mut rows = Vec::new();
    for (sibling, names) in sets {
        let mut notes: Vec<String> = Vec::new();
        for name in &shared {
            if !names.contains(*name) {
                notes.push(wording.missing(name));
            }
        }
        for name in names {
            if counts.get(name.as_str()).copied().unwrap_or(0) == 1 {
                notes.push(wording.extra(name));
            }
        }
        if !notes.is_empty() {
            rows.push((sibling.clone(), notes));
        }
    }
    rows
}

/// The faces the build's own record describes, when it describes all of them (audit `T3`).
/// 构建自己的记录所描述的那些面——当它把每一个都描述到了时（审计 `T3`）。
///
/// `None` means "derive instead", and it is returned for three different reasons that all answer the
/// same way: the member has no published record, the record is not current, or a row lacks a column a
/// `FaceView` needs. A partial record must not be used — comparing a face against a family smaller
/// than its own is the silent wrong answer this area keeps producing.
/// `None` 意为"改为推导"，而它有三个不同的理由、答法都一样：成员没有已发布记录、记录不新鲜、或某一行缺了
/// `FaceView` 需要的列。残缺的记录不许使用——拿一个比它自己更小的家族去比，正是这一带反复生产的"格式正常
/// 但错"的答案。
pub(super) fn published_faces(
    member: &crate::mcp::workspace::Member,
) -> Option<(Vec<crate::build_method::FaceView>, String)> {
    member.published()?;
    if !crate::mcp::build_evidence::build_evidence(&member.dir).current {
        return None;
    }
    let out = crate::mcp::build_evidence::out_dir(&member.dir);
    let rows = crate::build_method::read_pruning_manifest(&out).ok()?;
    if rows.is_empty() {
        return None;
    }
    let (faces, skipped) = crate::build_method::face_views_from_pruning(&rows);
    if skipped > 0 {
        return None;
    }
    Some((
        faces,
        format!(
            "tree published from {} (this comparison reads the build's own record)",
            out.display()
        ),
    ))
}

/// Every root a record read must cover, with its freshness verdict taken **once** (audit `T1`).
/// 记录读取要覆盖的每个根，以及**只取一次**的新鲜度判定（审计 `T1`）。
///
/// A workspace root has no record of its own; its members do. Taking the verdict here and passing it
/// down means one call asks each member once — it used to be asked twice, and at 50,000 files that was
/// the whole remaining cost.
/// 工作区根没有自己的记录，成员才有。判定在这里取一次并往下传，于是一次调用对每个成员只问一次——过去问两次，
/// 而在 50,000 文件上那就是剩下的全部成本。
pub(crate) fn roots_with_freshness(root: &Path) -> Vec<(std::path::PathBuf, bool)> {
    // This is where a record-reading answer starts for every in-process caller — the dispatched
    // tools get their own boundary from `set_policy`, but a direct call (`search` in a test, one
    // reader calling another) has only this one. Opening the answer here is what lets the three
    // readers below share one stamp walk per member while a stamp taken for a *previous* answer can
    // never vouch for this tree (audit `T1`, cut 6).
    // 对每个进程内调用方来说，读记录的一次答案就是从这里开始的——派发的工具由 `set_policy` 给出自己的
    // 边界，而直接调用（测试里的 `search`、一个读取方调另一个）只有这里。在这里开启答案，正是让下面三个
    // 读取方共用"每成员一遍戳"、同时让**上一份**答案取的戳绝不为此树作保的东西（审计 `T1` 第六刀）。
    crate::mcp::freshness::begin_answer();
    let roots = match crate::mcp::workspace::scope(root) {
        Ok(crate::mcp::workspace::Scope::Package(_)) => vec![root.to_path_buf()],
        Ok(crate::mcp::workspace::Scope::Workspace(members)) => {
            members.into_iter().map(|member| member.dir).collect()
        }
        Ok(crate::mcp::workspace::Scope::Unresolvable(_)) | Err(_) => Vec::new(),
    };
    // The per-member work is where the seconds are at 50,000 files — one whole-tree content hash per
    // member — and **no member's verdict depends on another's**: this is the one place in a read
    // answer that parallelism can take without changing a single verdict (audit `T1`, cut 7). It is
    // measured, not assumed: the same eight members answered in 4.392 s one after another and in
    // 0.864 s under `-P8`.
    // 在 50,000 文件上，每个成员的活正是秒数所在——每成员一次整树内容哈希——而**没有任何成员的裁决依赖
    // 另一个**：这是读答案里并行唯一能不改变任何裁决就拿走的地方（审计 `T1` 第七刀）。这是量出来的而不是
    // 假定的：同样八个成员，一个接一个答用 4.392 s，`-P8` 下 0.864 s。
    let policy = crate::mcp::freshness::current_policy();
    let mut verdicts: Vec<Option<bool>> = vec![None; roots.len()];
    let mut due = Vec::new();
    for (index, member) in roots.iter().enumerate() {
        let out = crate::mcp::build_evidence::out_dir(member);
        match crate::mcp::freshness::consult(member, &out, policy) {
            crate::mcp::freshness::Consultation::Remembered(current) => {
                verdicts[index] = Some(current);
            }
            crate::mcp::freshness::Consultation::Pay { stamp } => {
                due.push((index, member.clone(), out, stamp));
            }
        }
    }
    // Two items already pay for the thread handoff here: a member is a whole-tree content hash, not a
    // file. The budget is the machine rule the walk uses — half the cores, at most eight, one core
    // always left alone — so a tool call never takes the user's machine (audit `T1`, cut 7).
    // 这里两个条目就够付线程交接的钱：一个成员是一次整树内容哈希，不是一个文件。预算就是遍历所用的那条机器
    // 规则——一半的核、最多八个、永远留一个核——因此一次工具调用绝不拿走用户的整台机器（审计 `T1` 第七刀）。
    let paid = crate::build_method::parallel_map_with_threshold(
        &due,
        crate::build_method::worker_budget(),
        2,
        |(_, member, out, stamp)| crate::mcp::freshness::pay_with_stamp(member, out, *stamp),
    );
    for ((index, member, out, _), payment) in due.iter().zip(paid) {
        verdicts[*index] = Some(crate::mcp::freshness::record(member, out, payment));
    }
    roots
        .into_iter()
        .zip(verdicts.into_iter().map(|current| current.unwrap_or(false)))
        .collect()
}

/// The identity the face in this file carries **now** (audit `T1`).
/// 这份文件里的面**此刻**携带的身份（审计 `T1`）。
///
/// Parsed with the kernel's own face reader and hashed with the build's own identity rule, so a
/// record-answered row and a derived row cannot disagree about what "re-identified" means. `None` when
/// the file cannot be read or parsed: the caller then falls back to the recorded identity and the reply
/// keeps saying the build it describes is stale.
/// 用内核自己的面读取器解析、用构建自己的身份规则散列，因此"记录作答的行"与"推导出来的行"不可能对
/// "re-identified"给出两种意思。文件读不到或解析不了时回 `None`：调用方退回记录里的身份，而回复继续说明
/// 它所描述的那次构建已经陈旧。
pub(crate) fn current_identity(
    root: &Path,
    row: &crate::build_method::PruningRow,
) -> Option<xirang_kernel::identity::NodeId> {
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let text = std::fs::read_to_string(root.join("src").join(&row.source)).ok()?;
    let face = crate::mcp::consistency::one_face(&text).ok()?;
    // `field` and not `string`: the declaration writes `kind: X` and the kernel's face reader keeps
    // that as the field's raw value (measured: `string("kind")` is `None` here while
    // `field("kind")` is `Some("RenamedButton")`).
    // 用 `field` 而不是 `string`：声明写的是 `kind: X`，内核的面读取器把它保留为字段的原始取值（实测：
    // 这里 `string("kind")` 是 `None`，而 `field("kind")` 是 `Some("RenamedButton")`）。
    let kind = face.field("kind").or_else(|| row.kind.clone())?;
    Some(xirang_kernel::identity::NodeId::from_namespaced_path(
        &namespace,
        &row.source,
        &kind,
    ))
}
