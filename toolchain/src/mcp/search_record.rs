//! The record half of a face answer: what the published records say, and whether they may say
//! "no such face".
//! 面答案的记录那一半：已发布记录说了什么，以及它们能否说"没有这个面"。
//!
//! Split out of `search.rs` for the size ratchet. The two questions here are the ones a record
//! reader must keep apart: **freshness** (is this record about the tree as it is now?) and
//! **coverage** (does it carry every face the sources declare?). An empty answer is only final
//! when both hold, and the coverage half is `workspace::record_covers_producers` — one answer for
//! `registry`, `search` and `status`.
//! 从 `search.rs` 拆出，为尺寸棘轮。这里的两问正是记录读者必须分开的：**新鲜度**（这份记录说的是不是
//! 此刻这棵树？）与**覆盖**（它是否携带源码声明的每个面？）。只有两者都成立时空答案才是终局，而覆盖那一半
//! 是 `workspace::record_covers_producers`——`registry`、`search` 与 `status` 共用同一个答案。
//!
//! online: a record that cannot answer is derived from the sources as they are now, so an empty
//! answer never claims the build's authority for a tree the build did not see.

use std::path::Path;

use xirang_kernel::identity::NodeId;

use crate::mcp::search::package_tree_lines;
use crate::mcp::tree_delta::{FaceStatus, TreeDelta};

/// Says which tree the face rows below came from when they came from the build's records.
/// 下面那些面行来自构建记录时，说明它们读的是哪棵树。
pub(super) const RECORD_SOURCE: &str = "tree  answered from the build's published records (a record carries \
                             path, kind and registry_name; the module is not in one, so a query \
                             that only matches a module derives from the sources)";

/// Answer the path/function half from the published file manifest (audit `T1`).
/// 由已发布的文件清单回答路径/函数那一半（审计 `T1`）。
///
/// `None` means the record cannot answer — absent, stale, or describing no file — and the caller
/// scans the sources instead. The manifest must describe **every** file, the same rule every reader
/// in this bridge follows: one that listed only the files carrying functions would answer a path
/// question about a subset of the tree, which is the silent wrong answer this area keeps producing.
/// `None` 意为记录答不了——不存在、不新鲜、或一份文件都没描述——调用方改为扫源码。清单必须描述**每一份**
/// 文件，与本桥每个读者遵循的是同一条规则：一份只列出"带函数的文件"的清单，会把路径问题答成整棵树的一个子集，
/// 而"格式正常但错"正是这一带反复生产的答案。
pub(super) fn record_source_lines(
    members: &[(std::path::PathBuf, bool)],
    query: &str,
    limit: usize,
    hits: &mut usize,
    withheld_hits: &mut usize,
) -> Result<Option<Vec<String>>, String> {
    // A workspace root has no record of its own — its members do — so the members are read, the same
    // shape `consistency`'s census uses. Without this, a query against a workspace root fell back to
    // scanning every file in it (measured 12.8 s at 50,000 files) even though every member's manifest
    // was current.
    // 工作区根没有自己的记录——它的成员才有——因此读的是成员，与 `consistency` 的普查同形。少了这一步，
    // 对着工作区根的查询会退回扫描它里面的每个文件（在 50,000 文件上实测 12.8 s），尽管每个成员的清单都是
    // 新鲜的。
    // No member at all means no record to answer from — a root cargo cannot resolve, or a directory
    // with no manifest. That is a fall-back, not an empty list: the scan below still answers the
    // source half, which is exactly what such a tree needs.
    // 一个成员都没有，就意味着没有可据以作答的记录——cargo 解析不了的根，或没有清单的目录。那是**回退**，
    // 不是空清单：下面的扫描照样回答源码那一半，而那正是一棵这样的树所需要的。
    if members.is_empty() {
        return Ok(None);
    }
    let mut rows = Vec::new();
    for (root, current) in members {
        if !*current {
            return Ok(None);
        }
        let out = crate::mcp::build_evidence::out_dir(root);
        let Ok(member_rows) = crate::build_method::read_file_manifest(&out) else {
            return Ok(None);
        };
        if member_rows.is_empty() {
            return Ok(None);
        }
        rows.extend(member_rows);
    }
    let needle = query.to_ascii_lowercase();
    let mut lines = Vec::new();
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for row in &rows {
        if row.source.to_ascii_lowercase().contains(&needle) && seen.insert(row.source.as_str()) {
            if *hits < limit {
                lines.push(format!("file  src/{}", row.source));
                *hits += 1;
            } else {
                *withheld_hits += 1;
            }
        }
    }
    for row in &rows {
        if row.function == "-" || !row.function.to_ascii_lowercase().contains(&needle) {
            continue;
        }
        if *hits < limit {
            lines.push(format!("fn    {} -> src/{}", row.function, row.source));
            *hits += 1;
        } else {
            *withheld_hits += 1;
        }
    }
    Ok(Some(lines))
}

/// Face rows answered from the member's published records, when they can be.
/// 能从成员的已发布记录作答的那些面行。
///
/// The records carry everything a name match needs and a verdict needs — identity, source, and the
/// three declared facts — so this path costs no source walk at all. It answers `None` when the
/// records are unreadable or nothing in them matches, and the caller derives instead; that is not a
/// fallback of last resort but the honest half of the split, because a record does **not** carry the
/// face's module, and a query that only matches a module can only be answered from the sources.
/// 记录携带了名字匹配与裁决所需的一切——身份、源码路径，以及那三项声明的事实——因此这条路径完全不花
/// 源码遍历。记录读不了、或其中没有命中时返回 `None`，由调用方改为推导；那不是最后的兜底，而是这个分工
/// 诚实的那一半：记录**不**携带面的模块，而只匹配模块的查询只能由源码作答。
pub(super) fn record_face_lines(
    root: &Path,
    current: bool,
    query: &str,
    limit: usize,
    hits: &mut usize,
    withheld_hits: &mut usize,
) -> Result<Option<Vec<String>>, String> {
    let out = crate::mcp::build_evidence::out_dir(root);
    let Ok(rows) = crate::build_method::read_pruning_manifest(&out) else {
        return Ok(None);
    };
    // The rows are read **once** and handed to the delta as well: the record half and the delta read
    // the same manifest, and reading it twice parsed 50,000 rows twice for one 50,000-file answer
    // (audit `T1`, cut 7). One row per face is then taken from the rows this delta already holds.
    // 这些行**只读一次**，并同样交给差异：记录那一半与差异读的是同一份清单，读两遍等于为一份 50,000 文件的
    // 答案把五万行解析两遍（审计 `T1` 第七刀）。随后"每个面一行"取自这份差异已经持有的那些行。
    let built = TreeDelta::from_rows(rows, current);
    // One row per tracked symbol, so a face appears as many times as it tracks symbols.
    // 每个被跟踪符号一行，因此一个面会出现它跟踪符号数次。
    let by_id = built.one_row_per_id();
    // A **stale** record no longer describes this tree, and every verdict below is relative to it:
    // the round's `kind`-change pin needs the derivation to see that a face was re-identified, and the
    // `added since build` pin needs it to see a face the build never saw. Presenting the old rows with
    // a warning is what a *listing* may do; a **name lookup with verdicts** may not, because the caller
    // asked what is there now.
    // **陈旧**的记录不再描述这棵树，而下面每条裁决都是相对它说的：那一轮的 `kind` 变更钉子需要推导来看出
    // 某个面被重新识别过，`added since build` 钉子需要它看见构建从未见过的面。带上警告把旧行摆出来，是一份
    // **清单**可以做的；而**带裁决的名字查找**不可以——调用方问的是现在有什么。
    let mut lines = Vec::new();
    for (id, row) in &by_id {
        // Four spellings, and the fourth is the one this used to be missing: `path` is what the
        // **declaration** wrote (often `-`, because a macro-derived face does not state one), while
        // `logical_path` is the path every other tool reports and the one a caller has in hand. Without
        // it, `search {query: "root/button"}` found nothing in the record and fell back to deriving —
        // which is why the empty-answer rule could not be trusted: an empty record answer did not mean
        // "no such face", it meant "this record reader cannot see the spelling you asked for".
        // 四种拼写，而第四种正是这里过去缺的那一种：`path` 是**声明**写下的（常常是 `-`，因为宏派生的面不
        // 写它），而 `logical_path` 是其它每个工具报告的那个路径、也是调用方手里有的那个。少了它，
        // `search {query: "root/button"}` 在记录里什么都找不到、退回推导——这正是"空答案"不能信的原因：
        // 记录答空并不等于"没有这个面"，而是"这个记录读者看不见你问的那种拼写"。
        let declared = [
            row.path.as_deref(),
            row.logical_path.as_deref(),
            row.kind.as_deref(),
            row.registry_name.as_deref(),
        ];
        if !declared
            .iter()
            .flatten()
            .any(|field| field.to_ascii_lowercase().contains(query))
        {
            continue;
        }
        if *hits < limit {
            // A stale record still answers the **name** (that is this reader's whole point), but its
            // row describes the build, not the tree: the identity to judge is the one the file carries
            // **now**, and deriving it costs one parse of the one file this row is about. Without
            // this, a `kind` changed in place came back `[ok]` — the record's own id compared against
            // itself — while the caller was asking what is there now.
            // 陈旧的记录照样回答**名字**（这正是这个读者的意义），但它的行描述的是那次构建、不是这棵树：要裁决
            // 的身份是文件**此刻**携带的那个，而推出它只需解析这一行所指的那一个文件。少了这一步，就地改过的
            // `kind` 会回 `[ok]`——拿记录自己的 id 与它自己比——而调用方问的是现在有什么。
            let judged = if built.current {
                *id
            } else {
                crate::mcp::consistency::current_identity(root, row).unwrap_or(*id)
            };
            lines.push(record_face_line(judged, row, &built));
            *hits += 1;
        } else {
            *withheld_hits += 1;
        }
    }
    // An empty result still yields to the derivation, and the reason is measured rather than assumed:
    // **freshness does not notice a file that was added**. The pin
    // `a_published_face_is_ok_and_a_new_one_is_added_since_build` writes a face beside a current
    // record and expects `[added since build]` — that face is in no record, so an empty record answer
    // cannot be final. Making it final is worth about 90 s at 50,000 files, so the order is: first let
    // `build_output_is_current` cover the **discovered file set**, then let the record's empty answer
    // be final.
    // 空结果仍然让位给推导，而理由是量出来的、不是假定的：**新鲜度不会注意到新增的文件**。钉子
    // `a_published_face_is_ok_and_a_new_one_is_added_since_build` 在一份新鲜的记录旁边写下一个面，并期待
    // `[added since build]`——那个面不在任何记录里，因此记录的空答案不可能是最终答案。把空答案定为最终答案
    // 在 50,000 文件上约值 90 s，所以次序是：先让 `build_output_is_current` 覆盖**被发现到的文件集合**，
    // 再让记录的空答案成为最终答案。
    if lines.is_empty() {
        // Two different answers hide behind "no hits": the record **is** the whole tree and it
        // declares nothing under this name (an empty vector), or the record cannot speak for this
        // tree at all (`None`). Returning the provenance header for the first case made it
        // indistinguishable from a one-hit answer to every caller — which is how `search` came to
        // treat a record that does not carry the C++ faces as the final word on them.
        // "没有命中"后面藏着两种不同答案：记录**就是**整棵树、它在这个名字下什么都没声明（空向量），
        // 或者记录根本不能为这棵树代言（`None`）。第一种情形过去返回"来源"那行，使它对每个调用方都与
        // "一个命中"无法区分——这正是 `search` 把一份不携带 C++ 面的记录当成关于它们的最终答案的方式。
        return Ok((built.known && built.current).then(Vec::new));
    }
    let mut answer = vec![RECORD_SOURCE.to_owned()];
    if built.known && !built.current {
        answer.push(
            "tree  build stale (run `xirang check`); the statuses below compare against that \
             build"
                .to_owned(),
        );
    }
    answer.extend(lines);
    Ok(Some(answer))
}

/// One face hit built from a published record.
/// 由一条已发布记录构成的面命中。
pub(super) fn record_face_line(
    id: NodeId,
    row: &crate::build_method::PruningRow,
    built: &TreeDelta,
) -> String {
    let status = if !built.known {
        "build unknown (run `xirang check`)".to_owned()
    } else {
        let verdict = built.status_of(id, &row.source);
        if let FaceStatus::Reidentified(previous) = verdict {
            format!("{} ({previous} -> {id})", verdict.label())
        } else {
            verdict.label().to_owned()
        }
    };
    format!(
        "face  {:<40} kind={:<14} registry={:<16} source={}  [{status}]",
        row.path.as_deref().unwrap_or("-"),
        row.kind.as_deref().unwrap_or("-"),
        row.registry_name.as_deref().unwrap_or("-"),
        row.source
    )
}

/// What the tree half of a face query came to.
/// 面查询的树那一半得出了什么。
pub(super) enum FaceAnswer {
    /// Rows from the published record (the header is already the first row).
    /// 来自已发布记录的行（表头已是第一行）。
    Record(Vec<String>),
    /// The record answered "nothing", and it **covers** every producer — the empty answer is final.
    /// 记录答了"什么都没有"，且它**覆盖**每个生产者——空答案就是终局。
    EmptyRecord,
    /// Rows derived from the sources (no header; the caller adds the source line when non-empty).
    /// 由源码推导出的行（无表头；调用方在非空时加上来源行）。
    Derived(Vec<String>),
}

/// The tree half of one package's face query, in the order the facts have to be asked.
/// 一个包的面查询的树那一半，按事实必须被问到的次序。
///
/// Coverage is asked **before** the record is believed. A record can be fresh and still not carry
/// every face the sources declare (a fragment crate registers its own; a subtree handed to a
/// generated crate registers there), and a **non-empty** answer from such a record is a subset of
/// the matches: "this row exists" does not mean "no other producer matches". The first version
/// returned that subset as final and lost four of five faces on a broad query; the derivation is
/// now the authority whenever coverage is incomplete, and the coverage decision hands its own
/// derivation down so the walk is paid once.
/// 覆盖要在采信记录**之前**问。一份记录可以新鲜，却仍不携带源码声明的每个面（碎片 crate 自己登记；
/// 交给生成 crate 的子树在那边登记），而这种记录的**非空**答案只是命中集合的一个子集："这一行存在"
/// 不等于"没有别的生产者命中"。第一版把这个子集当终局，在一次宽查询上漏掉了五个面中的四个；现在覆盖
/// 不完整时推导才是权威，而覆盖判定把自己那次推导交下来，使遍历只付一次。
pub(super) fn face_answer(
    root: &Path,
    namespace: &str,
    current: bool,
    query: &str,
    limit: usize,
    hits: &mut usize,
    withheld_hits: &mut usize,
) -> Result<FaceAnswer, String> {
    let coverage = crate::mcp::workspace::producer_coverage(root, namespace)?;
    let before_hits = *hits;
    let before_withheld = *withheld_hits;
    match record_face_lines(root, current, query, limit, hits, withheld_hits)? {
        Some(lines) if !lines.is_empty() => {
            if coverage.complete {
                return Ok(FaceAnswer::Record(lines));
            }
            // The record's hits are a subset, so its counters are discarded together with its rows.
            // 记录的那些命中只是子集，因此它的行与计数一起作废。
            *hits = before_hits;
            *withheld_hits = before_withheld;
            let (faces, unparsable) = match coverage.derived {
                Some(derived) => derived,
                None => crate::mcp::resolve::derived_faces(root, namespace)?,
            };
            Ok(FaceAnswer::Derived(package_tree_lines(
                root,
                &faces,
                &unparsable,
                query,
                limit,
                hits,
                withheld_hits,
            )))
        }
        Some(_) if coverage.complete => Ok(FaceAnswer::EmptyRecord),
        Some(_) | None => {
            let (faces, unparsable) = match coverage.derived {
                Some(derived) => derived,
                None => crate::mcp::resolve::derived_faces(root, namespace)?,
            };
            Ok(FaceAnswer::Derived(package_tree_lines(
                root,
                &faces,
                &unparsable,
                query,
                limit,
                hits,
                withheld_hits,
            )))
        }
    }
}
