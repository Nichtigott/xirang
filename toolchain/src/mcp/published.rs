//! The build's published records for one member, read as a tree answer.
//! 一个成员已发布的记录，作为一份树级答案来读。
//!
//! The build already derives this tree once and writes it to
//! `<package>/target/xirang/out`; re-deriving it per member is what made a
//! workspace-rooted call cost the sum of every member's source walk. This module
//! reads those files instead — through `build_method`'s own readers, so the
//! formats keep their one parser and it is not here — and hands the answer on in
//! the record's own vocabulary: the face rows `pruning_manifest.tsv` carries
//! (`node`, `source`, `symbol`), the scope verdict `source_scope.tsv` carries,
//! and the `discovery.fingerprint` the freshness rule reads.
//! 构建已经把这棵树推导过一次并写进 `<package>/target/xirang/out`；逐成员重新推导正是让
//! 工作区根上的一次调用等于每个成员源码遍历之和的原因。本模块改为读那些文件——经
//! `build_method` 自己的读取器，因此各格式保持它们唯一的解析器、而且不在本模块——并用记录
//! 自己的词汇交出答案：`pruning_manifest.tsv` 携带的面行（`node`、`source`、`symbol`）、
//! `source_scope.tsv` 携带的作用域结论，以及新鲜度规则所读的 `discovery.fingerprint`。
//!
//! What the record carries changed on 2026-09-29: the pruning rows now publish the
//! four facts the **declaration** spelled — `path`, `kind`, `registry_name` and
//! `parent` (`-` where it named none) — so a face can be matched by logical path
//! without re-reading its source. What is still derived is everything that needs
//! resolution rather than spelling: the parent's `NodeId` (`parent_resolved`), the
//! module, and the identity cross-check. That is why [`Publication`] is a reading of
//! a *tree* rather than a replacement for the derivation, and why every answer built
//! on it says which of the two it used.
//! 记录携带什么，在 2026-09-29 变了：剪枝行现在发布**声明**拼出的四项事实——`path`、`kind`、
//! `registry_name`、`parent`（未声明处为 `-`）——因此一个面可以不重读自己的源码就按逻辑路径匹配。
//! 仍然推导的是需要**解析**而不是拼写的东西：父级的 `NodeId`（`parent_resolved`）、模块，以及身份
//! 交叉核对。这就是为什么 [`Publication`] 是对**一棵树**的读数而不是推导的替代品，也是为什么每一份
//! 由它构成的答案都会说出自己用的是哪一种。
//!
//! Never glob `target/debug/build/<pkg>-<hash>/out/`: one package measured 134
//! hashed copies with the old ones still there, so a glob can hand back a stale
//! record. The published path is `<package>/target/xirang/out` and nothing else.
//! 绝不要 glob `target/debug/build/<pkg>-<hash>/out/`：同一个包实测有 134 份哈希分身、旧的还
//! 在，因此 glob 可能取到陈旧记录。已发布的路径就是 `<package>/target/xirang/out`，不是别的。

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use crate::build_method::{BuildScopeView, PruningRow, read_build_scope, read_pruning_manifest};
use xirang_kernel::identity::NodeId;

use crate::mcp::build_evidence::out_dir;

/// The scope reason the build writes when its walk found no registration face.
/// 遍历没有找到注册面时构建写下的作用域原因。
pub(crate) const NO_FACE_REASON: &str = "no-registration-face";

/// What an answer prints when the face manifest was unreadable but the scope was not.
/// 作用域可读、面清单读不了时，答案打印的内容。
///
/// Not `no faces`: the scope file was readable, so this member *was* built, and the
/// face half is one file away from being answerable. Reporting it as faceless would
/// turn a damaged record into a claim about the package, which is the failure the two
/// states are kept apart to prevent. One spelling, in the module that produces the
/// state, so the roster row and the report cannot drift apart.
/// 而不是 `no faces`：作用域文件可读，因此这个成员**确实**被构建过，而面那一半只差一个文件就能
/// 作答。把它报成没有面会把一份损坏的记录变成关于这个包的断言，而区分这两个状态正是要阻止这件事。
/// 只有一份词形，放在产出该状态的模块里，因此普查行与报告不会漂开。
pub(crate) const FACES_UNKNOWN: &str =
    "faces unknown (no readable pruning_manifest.tsv; run `xirang check`)";

/// What one member's own `target/xirang/out` says.
/// 一个成员自己的 `target/xirang/out` 说了什么。
pub(crate) struct PublishedTree {
    /// The directory the reading came from, named in every answer built on it.
    /// 读数来自哪个目录；每一份由它构成的答案都会点名它。
    pub(crate) out: PathBuf,
    /// The published `discovery.fingerprint`, or `None` when the build removed it.
    /// 已发布的 `discovery.fingerprint`；构建移除它时为 `None`。
    pub(crate) fingerprint: Option<String>,
    /// The package root the content check needs, kept so it can run when it is asked for.
    /// 内容核验所需的包根；留着它是为了在被问到时才跑。
    root: PathBuf,
    /// Whether an answer *used* this record, which is what decides if it is checked.
    /// 是否有答案**用过**这份记录，而这决定了它是否被核验。
    ///
    /// A record nobody answered from costs no content hash: the census still says what the
    /// record holds, and says that freshness was not checked for it rather than claiming
    /// it. The flag is set by [`crate::mcp::workspace::Member::tree`] — the entrance a
    /// body is handed its tree through — and never by the census itself.
    /// 没有答案据以作答的记录不花内容哈希的钱：普查仍然说出记录里有什么，并说它没有被核验过新鲜度，
    /// 而不是去声称。这个标记由 [`crate::mcp::workspace::Member::tree`] 设置——也就是主体拿到它的
    /// 那棵树的入口——普查自己绝不设置它。
    used: Cell<bool>,
    /// The line this record already produced, so one answer cannot print two levels for it.
    /// 这份记录已经产出过的那一行，因此同一份答案不会为它印出两个等级。
    line: RefCell<Option<String>>,
    /// The scope `source_scope.tsv` recorded.
    /// `source_scope.tsv` 记录的作用域。
    pub(crate) scope: BuildScopeView,
    /// The face rows `pruning_manifest.tsv` recorded, or `None` when it was unreadable.
    /// `pruning_manifest.tsv` 记录的面行；读不了时为 `None`。
    pub(crate) pruning: Option<Vec<PruningRow>>,
}

impl PublishedTree {
    /// Whether an answer has used this record, which is what a check is bought for.
    /// 是否有答案用过这份记录，而这正是买一次核验的理由。
    pub(crate) fn was_used(&self) -> bool {
        self.used.get()
    }

    /// Mark this record as one an answer was built from.
    /// 把这份记录标为"某个答案据以构成"。
    pub(crate) fn mark_used(&self) {
        self.used.set(true);
    }

    /// The graded line for how fresh the published output is.
    /// 已发布产物新鲜度的分级行。
    ///
    /// The rule lives in `freshness` for every report that asks the same question, so one
    /// build cannot be described by two vocabularies; what is decided here is only that
    /// this record asks once per answer. Without that, the census (which is composed after
    /// the bodies, exactly so it can see what they used) would buy its own verification and
    /// print `reused` for a member the body had just printed `content-verified` for.
    /// 规则住在 `freshness`，每一份问同一个问题的报告都读它，因此同一次构建不会被两套词汇描述；
    /// 这里只决定这份记录每份答案只问一次。没有这一条，普查（它在主体之后才构成，正是为了看见主体
    /// 用了什么）会自己再买一次核验，于是对主体刚印出 `content-verified` 的成员印出 `reused`。
    pub(crate) fn freshness(&self) -> String {
        if let Some(line) = self.line.borrow().as_ref() {
            return line.clone();
        }
        let line = crate::mcp::freshness::line(&self.root, &self.out);
        *self.line.borrow_mut() = Some(line.clone());
        line
    }

    /// The one line every published answer opens its tree half with.
    /// 每一份发布答案的树那一半都以此行开头。
    ///
    /// The fingerprint is named rather than summarized: it is the token a reader
    /// compares against the next build, and `absent` is itself a fact — the
    /// pipeline removes it when validation fails.
    /// 指纹是被点名而不是被概括的：它是读取方与下一次构建对照的凭据，而 `absent` 本身就是个事实
    /// ——校验失败时管线会把它移除。
    pub(crate) fn evidence_line(&self) -> String {
        self.mark_used();
        format!(
            "tree published from {} (discovery.fingerprint {})\n",
            self.out.display(),
            self.fingerprint.as_deref().unwrap_or("absent"),
        )
    }

    /// The face rows this record carries.
    /// 这份记录携带的面行。
    pub(crate) fn faces(&self) -> &[PruningRow] {
        self.mark_used();
        self.pruning.as_deref().unwrap_or_default()
    }

    /// Whether the face manifest could not be read at all.
    /// 面清单是否根本读不了。
    ///
    /// "The build tracked no face" and "nothing here can say" are different
    /// answers, and a reader that flattened them would report an unreadable file as
    /// a framework crate. The roster and the report both print this state as
    /// `unknown` rather than as zero.
    /// "构建没有跟踪任何面"与"这里说不出来"是不同的答案，把它们抹平的读取方会把一个读不了的文件
    /// 报成框架 crate。普查与报告都把这个状态印成 `unknown` 而不是零。
    pub(crate) fn faces_unknown(&self) -> bool {
        self.pruning.is_none()
    }

    /// Whether this record was published for the **whole** tree (`scope.all`), which is what
    /// decides whether a face missing from it is "not built" or "registered somewhere this record
    /// does not reach" — the two answers a reader must not have to guess between.
    /// 这份记录是否为**整棵树**发布（`scope.all`）——它决定"记录里缺一个面"是"没有构建"还是"由这份记录
    /// 到不了的地方登记"，而读者不该在这两个答案之间猜。
    pub(crate) fn selects_everything(&self) -> bool {
        self.scope.all
    }

    /// Whether the scope this record carries selected a face by identity or source.
    /// 这份记录携带的作用域是否按身份或源码选中了一个面。
    pub(crate) fn selected(&self, row: &PruningRow) -> bool {
        self.scope.all
            || self.scope.selected_ids.contains(&row.id)
            || self.scope.selected_sources.contains(&row.source)
    }

    /// Whether the record itself says this package declares no registration face.
    /// 记录本身是否说这个包不声明任何注册面。
    ///
    /// The build writes `# result all` with the reason `no-registration-face`
    /// when its walk found nothing, and that is an answer rather than a missing
    /// one. A `# result all` for any *other* reason means "every face is in
    /// scope" and the record lists no rows because it had no need to: that is not
    /// the same claim, and reading it as an empty tree is the mistake this method
    /// exists to prevent. A narrowed scope that lists no face at all is the third
    /// shape, and there the manifest is the witness: an unreadable manifest means
    /// "unknown", not "none".
    /// 遍历什么也没找到时，构建写下 `# result all` 与原因 `no-registration-face`，那是一份答案
    /// 而不是缺失的答案。任何**别的**原因下的 `# result all` 意思是"每个面都在作用域内"，记录
    /// 不列行是因为它没必要列：那不是同一个断言，而把它读成一棵空树正是本方法要阻止的错误。
    /// 第三种形状是"收窄了作用域却一个面也没列"，那里的证人是清单：清单读不了意味着"未知"而不是
    /// "一个都没有"。
    pub(crate) fn records_no_faces(&self) -> bool {
        if self.scope.all {
            self.scope.reason.as_deref() == Some(NO_FACE_REASON)
        } else {
            self.scope.selected_ids.is_empty()
                && self.pruning.as_ref().is_some_and(|rows| rows.is_empty())
        }
    }

    /// The faces this record has, by identity: what an ownership question can be
    /// answered from without reading a source file.
    /// 这份记录拥有的面，按身份：归属问题可以据此作答而不读任何源码文件。
    /// Whether this record names that identity, and marking the record used only when it
    /// does: a probe that comes back negative is routing, not answering.
    /// 这份记录是否点名了那个身份；只有答"是"时才把记录标为已用——否定回来的探问是路由，不是作答。
    pub(crate) fn has_identity(&self, id: NodeId) -> bool {
        let owned = self
            .pruning
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|row| row.id == id);
        if owned {
            self.mark_used();
        }
        owned
    }
}

/// What a member's own `target/xirang/out` holds.
/// 一个成员自己的 `target/xirang/out` 里有什么。
pub(crate) enum Publication {
    /// `source_scope.tsv` was readable, so this member has published records.
    /// `source_scope.tsv` 可读，因此这个成员有已发布的记录。
    Published(Box<PublishedTree>),
    /// No readable `source_scope.tsv`; the reader's own reason.
    /// `source_scope.tsv` 读不了；读取方自己给出的原因。
    NotBuilt(String),
}

/// Read one member's published records, or say why there are none.
/// 读取一个成员的已发布记录，或说出为什么没有。
///
/// `source_scope.tsv` is the token: it is the file the scope reader owns and the
/// first thing every build publishes, so its absence means "this member was never
/// built" rather than "this member has no faces". The two are different answers
/// and the caller spells them differently.
/// `source_scope.tsv` 就是那枚凭据：它是作用域读取方拥有的文件，也是每次构建最先发布的东西，
/// 因此它缺失意味着"这个成员从未被构建"，而不是"这个成员没有面"。两者是不同的答案，调用方也
/// 用不同的话说出来。
///
/// Reading a record is reading files; it is **not** a content verification. The hash over
/// the sources is what costs, and it is bought when a report asks for this record's
/// freshness (`PublishedTree::freshness`), which a member no answer used never does.
/// 读记录就是读文件；它**不是**内容核验。花钱的是对源码的哈希，而它在这份记录的新鲜度被报告问起
/// 时才买（`PublishedTree::freshness`）——没有被任何答案用到的成员永远不会问。
pub(crate) fn read(root: &Path) -> Publication {
    let out = out_dir(root);
    let scope = match read_build_scope(&out) {
        Ok(scope) => scope,
        Err(reason) => return Publication::NotBuilt(reason),
    };
    let fingerprint = std::fs::read_to_string(out.join("discovery.fingerprint"))
        .ok()
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty());
    // An unreadable pruning manifest is `None` rather than an empty list: "the
    // build tracked no face" and "nothing here can say" are different answers, and
    // a merged roster that flattened them would report a package as faceless
    // because a file was missing.
    // 读不了的修剪清单是 `None` 而不是空清单："构建没有跟踪任何面"与"这里说不出来"是不同的答案，
    // 把它们抹平的合并普查会因为一个文件缺失而把一个包报成没有面。
    let pruning = read_pruning_manifest(&out).ok();
    // Boxed: the enum is built once per member and read many times, and a record dwarfs the
    // "not built" variant next to it — `clippy::large_enum_variant` was right about the shape,
    // and a pointer is the shape that says "one of these is a record, the other is a sentence".
    // 装箱：这个枚举每个成员构造一次、读取多次，而一条记录比它旁边的"未构建"支大得多——
    // `clippy::large_enum_variant` 说的形状没错，而指针正是那种"一支是记录、另一支是一句话"的形状。
    Publication::Published(Box::new(PublishedTree {
        out,
        fingerprint,
        root: root.to_path_buf(),
        used: Cell::new(false),
        line: RefCell::new(None),
        scope,
        pruning,
    }))
}

#[cfg(test)]
thread_local! {
    /// How many times this thread derived a package's faces.
    /// 本线程推导过多少次某个包的面。
    ///
    /// A published answer claims something observable — that no face derivation
    /// happened for that member — and a claim nobody can check is a comment. The
    /// count is thread-local so a test that reads it is not racing the tests
    /// `cargo test` runs beside it on other threads, and it is test-only because
    /// nothing in a shipped bridge reads it: the claim it backs is a test's.
    /// 发布答案声称了一件可观测的事——那个成员没有发生面推导——而没人能核对的声称只是一句注释。
    /// 计数是线程局部的，因此读它的测试不会与 `cargo test` 在其它线程上并行跑的测试抢；它只存在于
    /// 测试中，因为出厂的桥里没有东西读它：它支撑的那项声称属于测试。
    static DERIVATIONS: Cell<usize> = const { Cell::new(0) };
}

/// Record that one package's faces were derived from source text.
/// 记下一次"某个包的面由源码文本推导出来"。
///
/// Test-only, and that is the whole point: [`derivations`] is what makes "this
/// answer read the published records" checkable, and the caller in `resolve` is
/// gated the same way.
/// 仅供测试，而这正是要点：[`derivations`] 让"这份答案读的是已发布记录"可被核对，而 `resolve`
/// 里的调用方以同样的方式门控。
#[cfg(test)]
pub(crate) fn note_derivation() {
    DERIVATIONS.with(|count| count.set(count.get() + 1));
}

/// How many derivations this thread has performed.
/// 本线程执行过多少次推导。
#[cfg(test)]
pub(crate) fn derivations() -> usize {
    DERIVATIONS.with(Cell::get)
}

#[cfg(test)]
#[path = "published_tests.rs"]
mod published_tests;
