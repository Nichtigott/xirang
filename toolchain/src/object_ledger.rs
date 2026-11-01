//! The bridge between a `FaceView` and the project's object ledger.
//! 注册面视图与项目对象台账之间的桥。
//!
//! The ledger itself is in `xirang_multilang::object_map`; this module is the toolchain's half — the
//! one place that turns "this face is about to be created / moved" into a file the same transaction
//! writes. Keeping it here is what makes the mapping part of the preview and part of the rollback
//! instead of a side effect somebody remembers to run.
//! 台账本身在 `xirang_multilang::object_map`；本模块是工具链那一半——唯一把"这个面即将被创建 / 搬动"
//! 变成同一个事务写入的文件的地方。放在这里，才让映射成为预览与回滚的一部分，而不是某个"记得去跑"的副作用。

use std::path::{Path, PathBuf};

use xirang_kernel::authoring::PlannedFile;
use xirang_kernel::identity::NodeId;
use xirang_multilang::object_map::{LEDGER_FILE, ObjectMap, Observed};

/// One ledger write a plan intends, with the bytes it replaces.
/// 计划要写下的一次台账改动，连同它替换掉的字节。
///
/// `original` is `None` when the package has no ledger yet — the same convention `PlannedFile` uses,
/// so the authoring service's transaction already knows how to create and roll it back.
/// 包还没有台账时 `original` 是 `None`——与 `PlannedFile` 同一个约定，因此创作服务的事务本来就知道
/// 怎么创建它、怎么回滚它。
pub(crate) struct LedgerWrite {
    /// The file this write replaces. Only the bridge prints or writes it today; the authoring service
    /// consumes the same change as a `PlannedFile`.
    /// 这次写入替换的文件。今天只有桥会打印或写它；创作服务把同一个改动当 `PlannedFile` 消费。
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    path: PathBuf,
    original: Option<String>,
    updated: String,
    /// The line the bridge's reply prints.
    /// 桥的回复打印的那一行。
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    summary: String,
}

impl LedgerWrite {
    /// The write as the authoring service's plan understands it.
    /// 把这次写入表成创作服务计划懂得的形状。
    pub(crate) fn planned(&self) -> PlannedFile {
        PlannedFile {
            path: LEDGER_FILE.to_owned(),
            original: self.original.clone(),
            updated: self.updated.clone(),
            edits: Vec::new(),
            remove: false,
        }
    }

    /// The line a reply prints for this change.
    /// 回复为这次改动打印的那一行。
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    pub(crate) fn summary(&self) -> &str {
        &self.summary
    }

    /// The file it writes.
    /// 它写入的文件。
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The bytes it writes.
    /// 它写入的字节。
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    pub(crate) fn bytes(&self) -> &[u8] {
        self.updated.as_bytes()
    }
}

/// Read the ledger's current text, distinguishing "absent" from "unreadable".
/// 读出台账当前的文本，并区分"不存在"与"读不了"。
fn current(root: &Path) -> Result<Option<String>, String> {
    let path = root.join(LEDGER_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// Turn a map and its previous text into a write, or `None` when nothing would change.
/// 把一张台账与它之前的文本变成一次写入；什么都不会变时返回 `None`。
fn finish(
    root: &Path,
    map: &ObjectMap,
    original: Option<String>,
    summary: String,
) -> Option<LedgerWrite> {
    let updated = map.render_text();
    if original.as_deref() == Some(updated.as_str()) {
        return None;
    }
    Some(LedgerWrite {
        path: root.join(LEDGER_FILE),
        original,
        updated,
        summary,
    })
}

/// Record one observation — and, when the placement deviated, the directory it was given.
/// 记录一条观察——当落点偏离默认布局时，连同它被指定的目录。
///
/// A declaration that follows the default layout and gives no directory is **not** recorded: the
/// ledger is the list of deviations and of objects that already have history, not a second copy of
/// the whole tree. `instance` is `None` when the caller does not know the new `NodeId` yet (a face
/// being created), which is why the ledger can hold an entry without a source instance.
/// 跟随默认布局、也没有给目录的声明**不**被记录：台账是"偏离项与已有历史的对象的清单"，不是整棵树的第二份
/// 抄本。调用方还不知道新的 `NodeId` 时（正在创建的面）`instance` 是 `None`，这也是台账可以只有条目、没有
/// 源码实例的原因。
pub(crate) fn plan_observation(
    root: &Path,
    observed: &Observed,
    instance: Option<NodeId>,
    directory: Option<String>,
) -> Result<Option<LedgerWrite>, String> {
    let mut map = ObjectMap::read_from(root)?;
    let known = map.own(observed)?.is_some();
    if !known && instance.is_none() && directory.is_none() {
        return Ok(None);
    }
    let stable = map.observe(observed, instance)?;
    let mut summary = format!(
        "ledger {stable}: {} at `{}`",
        observed.path, observed.source
    );
    if let Some(directory) = directory {
        map.set_directory(stable, Some(directory.clone()))?;
        summary.push_str(&format!(", mapped to `{directory}`"));
    }
    let original = current(root)?;
    Ok(finish(root, &map, original, summary))
}

/// Record a move: the object keeps its identity, both source instances stay resolvable, and an
/// explicit directory mapping follows the file.
/// 记录一次搬动：对象保持身份，两个源码实例都仍可解析，显式目录映射跟随文件。
///
/// The `after` observation is re-keyed to the object's stable id on purpose. Keeping the object
/// across a **logical** move (a new parent) is stronger than the review's minimum, which only
/// required physical moves to keep it — and it is what makes old references resolvable instead of
/// "repaired by the next reader". The source **instance** still changes, because it is derived from
/// the source path; the ledger is what connects the two.
/// `after` 这条观察有意重新挂到对象的稳定 id 上。跨越**逻辑**搬动（换父级）仍保持对象，比复核的最低
/// 要求更强——最低要求只要求物理搬动保持它——而这正是让旧引用"可解析"、而不是"留给下一个读者修"的原因。
/// 源码**实例**仍然会变，因为它由源码路径推导；连接两者的是台账。
#[cfg_attr(not(feature = "mcp"), allow(dead_code))]
pub(crate) fn plan_move(
    root: &Path,
    before: &Observed,
    before_id: NodeId,
    after: &Observed,
    after_id: NodeId,
    keeps_mapping: bool,
) -> Result<Option<LedgerWrite>, String> {
    plan_moves(
        root,
        &[MoveRecord {
            before,
            before_id,
            after,
            after_id,
            keeps_mapping,
        }],
    )
}

/// One object's move: both observations, both instances, and whether the destination still needs a
/// mapping.
/// 一个对象的一次搬动：两个观察、两个实例，以及目的地在默认遍历之外是否仍需要映射。
pub(crate) struct MoveRecord<'a> {
    /// The object as the tree sees it before the move.
    /// 搬动前树看到的对象。
    pub(crate) before: &'a Observed,
    /// Its instance identity before the move.
    /// 搬动前的实例身份。
    pub(crate) before_id: NodeId,
    /// The object as the tree will see it after.
    /// 搬动后树将看到的对象。
    pub(crate) after: &'a Observed,
    /// Its instance identity after the move.
    /// 搬动后的实例身份。
    pub(crate) after_id: NodeId,
    /// Whether the destination is outside the default walk and therefore needs a mapping.
    /// 目的地是否在默认遍历之外、因而需要映射。
    pub(crate) keeps_mapping: bool,
}

/// Plan several moves **against one ledger read**, so entries cannot overwrite each other.
/// 对**同一次台账读取**规划多次搬动，使条目不会互相覆盖。
///
/// Each move used to be planned separately, and each plan shipped the whole file; publishing the root
/// and then a descendant wrote the second text — computed from the pre-move state — over the first, so
/// the root's new source and path were silently rolled back (measured: the island entry still named
/// `components/reading/reading.rs` after it had moved to `src/`).
/// 过去每次搬动各自规划，而每份计划都带着整个文件；先发布根、再发布后代时，第二份文本——按搬动前的状态
/// 算出——覆盖了第一份，根的新源码与路径于是被静默回滚（实测：孤岛已搬到 `src/`，台账条目仍写着
/// `components/reading/reading.rs`）。
pub(crate) fn plan_moves(
    root: &Path,
    moves: &[MoveRecord<'_>],
) -> Result<Option<LedgerWrite>, String> {
    let mut map = ObjectMap::read_from(root)?;
    let mut lines = Vec::new();
    for record in moves {
        let stable = map.observe(record.before, Some(record.before_id))?;
        let mut keyed = record.after.clone();
        keyed.stable = stable;
        map.observe(&keyed, Some(record.after_id))?;
        let mut summary = format!(
            "object {stable} (kept across the move; instance {} → {})",
            record.before_id, record.after_id
        );
        let mapped = map
            .entry(stable)
            .is_some_and(|entry| entry.directory.is_some());
        if record.keeps_mapping {
            let directory = directory_of(&record.after.source);
            map.set_directory(stable, Some(directory.clone()))?;
            let line = if mapped {
                format!(", mapping follows to `{directory}`")
            } else {
                format!(
                    ", mapping recorded at `{directory}` (the destination is outside the default walk)"
                )
            };
            summary.push_str(&line);
        } else if mapped {
            map.set_directory(stable, None)?;
            summary.push_str(", mapping cleared (the destination is under the default walk)");
        }
        lines.push(summary);
    }
    let original = current(root)?;
    Ok(finish(root, &map, original, lines.join("; ")))
}

/// A mapped directory **other than the object's own** that already covers one package-relative file.
/// 一个**不属于该对象自己**、但已经覆盖某份包内相对文件的映射目录。
///
/// A move's destination needs its own ledger entry only when nothing else describes it. Moving a face
/// **into an island** lands under the island's mapped directory, and recording a per-face deviation
/// there would describe the same place twice.
/// 一次搬动的目的地只有在别的映射都不描述它时才需要自己的台账条目。把面搬**进孤岛**会落在孤岛映射的目录
/// 之下，在那里再记一条逐面偏离就是把同一个位置描述两遍。
#[cfg_attr(not(feature = "mcp"), allow(dead_code))]
pub(crate) fn covering_directory(root: &Path, source: &str) -> Result<Option<String>, String> {
    let map = ObjectMap::read_from(root)?;
    Ok(map
        .entries()
        .iter()
        .filter(|entry| entry.source != source)
        .filter_map(|entry| entry.directory.as_deref())
        .filter(|directory| {
            !directory.is_empty()
                && (source == *directory
                    || (source.starts_with(directory)
                        && source.as_bytes().get(directory.len()) == Some(&b'/')))
        })
        .max_by_key(|directory| directory.len())
        .map(str::to_owned))
}

/// The directory an object's owner explicitly mapped it to, when one did.
/// 对象的属主显式映射给它的目录（当有过这样一次时）。
///
/// This is what keeps a **pure physical move** inside an island: the destination of a rename under
/// the same parent is the object's mapped directory, not the parent's directory — otherwise renaming
/// a face would silently pull it out of the island it was placed in.
/// 这正是让**纯物理搬动**留在孤岛里的东西：同一父级下改名，目的地是对象被映射的目录，而不是父级的目录——
/// 否则一次改名就会把面悄悄拉出它被放置的那个孤岛。
#[cfg_attr(not(feature = "mcp"), allow(dead_code))]
pub(crate) fn mapped_directory(root: &Path, observed: &Observed) -> Result<Option<String>, String> {
    let map = ObjectMap::read_from(root)?;
    Ok(map.own(observed)?.and_then(|entry| entry.directory.clone()))
}

/// The package-relative directory part of a source path, or the empty string.
/// 源码路径的包内相对目录部分，或空串。
fn directory_of(source: &str) -> String {
    Path::new(source)
        .parent()
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "object_ledger_tests.rs"]
mod object_ledger_tests;
