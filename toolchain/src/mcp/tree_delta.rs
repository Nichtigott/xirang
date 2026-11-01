//! Where one face stands relative to the build's own manifest.
//! 一个面相对构建自己的清单处于什么状态。
//!
//! `xirang.diff` states the whole delta and `xirang.search` annotates each hit
//! with the same verdict, and the verdict is one rule: a face the build published
//! is `ok`, a face it never saw is `added since build`, and a face whose *source*
//! the build recorded under a different identity is `re-identified`. Two copies of
//! that rule would let the diff and the search disagree about the same face, which
//! is exactly the drift the tree vocabulary exists to prevent — so it lives here,
//! once, and both tools read it.
//! `xirang.diff` 说出整份差异，`xirang.search` 给每个命中标注同一个结论，而这个结论是一条规则：
//! 构建发布过的面是 `ok`，构建从未见过的面是 `added since build`，而构建把它的**源码**记在另一个身份
//! 之下的面是 `re-identified`。这条规则有两份副本，就会让 diff 与 search 对同一个面给出不同说法——
//! 而这正是这棵树的词汇要消除的漂移——因此它只住在这里，两个工具都读它。

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::build_method::{FaceView, PruningRow, read_pruning_manifest};
use xirang_kernel::identity::NodeId;

use crate::mcp::build_evidence::out_dir;

/// What the build says about one face the sources declare right now.
/// 构建对源码此刻声明的某个面给出的说法。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaceStatus {
    /// The build published this exact identity.
    /// 构建发布的正是这个身份。
    Ok,
    /// The build has no face at this source under any identity.
    /// 构建在这个源码位置没有任何身份的面。
    AddedSinceBuild,
    /// The build recorded this source under a different identity — a `kind`
    /// change under an unmoved file, which no text diff sees.
    /// 构建把这段源码记在另一个身份之下——文件没动而 `kind` 变了，文本 diff 看不见。
    Reidentified(NodeId),
}

impl FaceStatus {
    /// The word for the `ok` status.
    /// `ok` 状态的那个词。
    pub(crate) const OK: &'static str = "ok";
    /// The word for a face the build never saw.
    /// 构建从未见过的面的那个词。
    pub(crate) const ADDED_SINCE_BUILD: &'static str = "added since build";
    /// The word for a face the build recorded under another identity.
    /// 构建记在另一个身份之下的面的那个词。
    pub(crate) const REIDENTIFIED: &'static str = "re-identified";

    /// The word this status is printed as, everywhere it is printed.
    /// 这个状态在任何被打印的地方所用的那个词。
    ///
    /// `status` unifies the *rule*, and this unifies the *word*: without it the
    /// same face came back `reidentified` from `xirang.diff`'s count line and
    /// `re-identified` from `xirang.search`'s annotation, which is the drift
    /// the tree vocabulary exists to prevent. The count line names its buckets
    /// before any face is in hand, so it reads the constants above — the words
    /// still live here and nowhere else.
    /// `status` 统一的是**规则**，这里统一的是**词形**：没有它，同一个面会从 `xirang.diff`
    /// 的计数行回来成 `reidentified`、从 `xirang.search` 的标注回来成 `re-identified`——
    /// 而这正是这棵树的词汇要消除的漂移。计数行在任何面到手之前就说出它的桶名，因此它读上面的
    /// 常量；词形仍然只住在这里。
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Ok => Self::OK,
            Self::AddedSinceBuild => Self::ADDED_SINCE_BUILD,
            Self::Reidentified(_) => Self::REIDENTIFIED,
        }
    }
}

/// The built side of the source-versus-build comparison, read once.
/// 源码对构建比较中"构建那一侧"，只读一次。
pub(crate) struct TreeDelta {
    /// Whether the published output still describes these sources.
    /// 已发布的产物是否仍在描述这批源码。
    pub(crate) current: bool,
    /// Whether a pruning manifest was readable at all.
    /// 是否读到了修剪清单。
    pub(crate) known: bool,
    /// The identities the build published.
    /// 构建发布过的身份。
    ids: BTreeSet<NodeId>,
    /// The identity the build published per source path.
    /// 构建为每个源码路径发布过的身份。
    by_source: HashMap<String, NodeId>,
    /// The manifest rows, for the caller that reports a removal by source.
    /// 清单行，供按源码报告移除的调用方使用。
    pub(crate) rows: Vec<PruningRow>,
}

impl TreeDelta {
    /// Read the build's manifest for this package.
    /// 读取本包构建的清单。
    ///
    /// A missing manifest is data, not an error: it is the answer to "why does
    /// nothing here know whether it ships". An output that no longer describes
    /// these sources is still read — the rows are the build's own record — and
    /// `current` carries the caveat so each caller states it in its own words.
    /// 清单缺失是数据而不是错误：它正是"这里为什么没有东西知道自己发不发布"的答案。不再描述这批源码
    /// 的产物仍会被读取——那些行是构建自己的记录——而 `current` 携带那声提醒，让每个调用方用自己的话
    /// 说出它。
    pub(crate) fn read(root: &Path) -> Self {
        let out = out_dir(root);
        // Through the one entry point that pays for the content hash (audit `T1`, cut 6): this reader
        // and the face/source halves must share one verdict per call, or the same member is hashed
        // twice — measured as 40 hashes for 20 members, 3.5 s of a 7.9 s answer.
        // 经由那个**唯一**为内容哈希付费的入口（审计 `T1` 第六刀）：本读者与面半/源码半必须在一次调用内共享
        // 同一份裁决，否则同一个成员会被哈希两次——实测 20 个成员付了 40 次哈希，占 7.9 s 里的 3.5 s。
        let current = crate::mcp::freshness::verdict(root, &out);
        Self::read_with(root, current)
    }

    /// The same reading with the freshness verdict **already known** (audit `T1`).
    /// 同一份读取，但新鲜度判定**已知**（审计 `T1`）。
    ///
    /// Freshness walks the tree's sources, and one `search` call used to ask for it twice per member —
    /// the delta and the source half — which at 50,000 files was the whole difference between 17 s and
    /// the target. Passing the verdict down is the fix; a **cache** is not, because a remembered
    /// verdict is the shape that made an earlier attempt wrong.
    /// 新鲜度要遍历树的源码，而一次 `search` 过去对每个成员问两次（delta 与源码那一半）——在 50,000 文件上，
    /// 那就是 17 s 与目标之间的全部差距。把判定往下传就是修法；**缓存**不是，因为"记住一个判定"正是让先前
    /// 一次尝试出错的那种形状。
    pub(crate) fn read_with(root: &Path, current: bool) -> Self {
        let out = out_dir(root);
        match read_pruning_manifest(&out) {
            Ok(rows) => Self::from_rows(rows, current),
            Err(_) => Self::without_rows(current),
        }
    }

    /// The refusal one comparison earns when its published side belongs to **another identity
    /// domain** than `reader`.
    ///
    /// Identity is the namespace plus the source path plus the name, and the namespace comes from the
    /// environment rather than from the sources — so a record read under another namespace holds a
    /// different id for the *same* face, and every comparison against it would report every face as
    /// moved. That is the wrong answer that looks like a right one (audit 2026-10-06, §M7.18), so it
    /// is refused here instead, naming both namespaces and the one way forward.
    /// `None` when the record carries no stamp (it predates the field): then nothing can be said
    /// about namespaces, and saying nothing beats guessing.
    /// 当一次比较的已发布一侧属于 `reader` **之外的另一个身份域**时，它所换来的拒绝。
    ///
    /// 身份 = 命名空间 + 源码路径 + 名字，而命名空间来自环境而非源码——因此在另一个命名空间下读同一份
    /// 记录，**同一个**面持有不同的 id，与它做的每次比较都会把每个面报成搬了家。那正是"看起来正确的错误
    /// 答案"（2026-10-06 审计，§M7.18），所以这里改为拒绝，并点名两个命名空间与唯一的那条出路。
    /// 记录没有携带戳时（它早于该字段）返回 `None`：此时关于命名空间什么都说不出来，什么都不说胜过去猜。
    pub(crate) fn other_domain_refusal(root: &Path, reader: &str) -> Option<String> {
        let published = crate::mcp::index::read_generation(root)
            .ok()
            .and_then(|generation| generation.namespace)?;
        if published == reader {
            return None;
        }
        Some(format!(
            "namespace mismatch: the records in {} were published under `{published}`, and this run \
             reads identities as `{reader}`. Identity is the namespace plus the source path plus the \
             name, so the same face has two different ids here — comparing them would report every \
             face as moved, which is what this refuses to do.\n\
             way forward: run `xirang check` in this tree (it republishes under `{reader}`), or set \
             XIRANG_NAMESPACE={published} and ask again",
            out_dir(root).display()
        ))
    }

    /// The delta built from rows a caller has **already read** (audit `T1`, cut 7).
    /// 由调用方**已经读过**的行构成的差异（审计 `T1` 第七刀）。
    ///
    /// The pruning manifest is the same file `search`'s record half reads, and asking for it twice
    /// parsed fifty thousand rows twice per 50,000-file workspace — the read half of that answer's
    /// tree phase, measured at 1.5 s. A caller that has the rows hands them over instead.
    /// 剪枝清单就是 `search` 的记录那一半也在读的同一个文件，而问它两遍会在一个 50,000 文件的工作区上
    /// 把五万行解析两遍——那是该答案树那一半的一半，实测 1.5 s。手里已有行的调用方改为把它交过来。
    pub(crate) fn from_rows(rows: Vec<PruningRow>, current: bool) -> Self {
        let ids = rows.iter().map(|row| row.id).collect();
        let by_source = rows
            .iter()
            .map(|row| (row.source.clone(), row.id))
            .collect();
        Self {
            current,
            known: true,
            ids,
            by_source,
            rows,
        }
    }

    /// The delta for a member whose manifest could not be read.
    /// 清单读不了的成员的差异。
    pub(crate) fn without_rows(current: bool) -> Self {
        Self {
            current,
            known: false,
            ids: BTreeSet::new(),
            by_source: HashMap::new(),
            rows: Vec::new(),
        }
    }

    /// One row per face, borrowed from the rows this delta already holds.
    /// 每个面一行，借自这份差异已经持有的行。
    ///
    /// The manifest carries one row per tracked **symbol**, so a face appears as many times as it
    /// tracks symbols; the first row wins, exactly as the record half has always decided it.
    /// 清单每个被跟踪的**符号**一行，因此一个面会出现它跟踪符号数次；第一行胜出，与记录那一半一向的
    /// 判定相同。
    pub(crate) fn one_row_per_id(&self) -> std::collections::BTreeMap<NodeId, &PruningRow> {
        let mut by_id = std::collections::BTreeMap::new();
        for row in &self.rows {
            by_id.entry(row.id).or_insert(row);
        }
        by_id
    }

    /// Where one face stands, in the one vocabulary both tools use.
    /// 一个面的状态，用两个工具共用的那一套词汇。
    pub(crate) fn status(&self, face: &FaceView) -> FaceStatus {
        self.status_of(face.id, &face.source)
    }

    /// The same verdict for a face named by identity and source only.
    /// 只给身份与源码路径时的同一条裁决。
    ///
    /// The verdict never needed more than these two: the manifest keys a face by its source and
    /// remembers its identity, which is why an answer can be built from the build's published
    /// records (they carry both) without deriving the sources at all.
    /// 裁决本来就不需要多过这两样：清单按源码路径给面做键、并记住它的身份，因此答案可以完全由构建已发布
    /// 的记录构成（记录两样都有），而不必推导源码。
    pub(crate) fn status_of(&self, id: NodeId, source: &str) -> FaceStatus {
        if let Some(previous) = self.by_source.get(source) {
            // The identity is a hash over the source path and the kind, so a
            // recorded source whose identity differs is a changed `kind` — the one
            // piece of an identity that can move without the file moving.
            // 身份是对源码路径与 kind 的散列，因此记录了同一路径而身份不同，就是 kind 变了——
            // 这是身份输入里唯一能在文件不动的情况下移动的。
            return if *previous == id {
                FaceStatus::Ok
            } else {
                FaceStatus::Reidentified(*previous)
            };
        }
        if self.ids.contains(&id) {
            FaceStatus::Ok
        } else {
            FaceStatus::AddedSinceBuild
        }
    }
}
