//! What the configured root is, and the merged answer a virtual manifest needs.
//! 配置的根是什么，以及虚拟清单所需要的合并答案。
//!
//! Every identity in this bridge is hashed over `(namespace, source path, declared
//! name)`, and the namespace is the *package's own name* — that is the design, not a
//! coincidence: the declaration macros read `crate::XIRANG_NAMESPACE`, which `host!()` defines from the package name, so
//! "package = namespace" is exactly what a built host compiled. A workspace root is
//! not a package, so it has no namespace of its own, and every tree tool used to
//! refuse there with Cargo's own words. The machine was there; the entrance was not.
//! 本桥里的每个身份都是对 `(命名空间, 源码路径, 声明名)` 取散列，而命名空间就是**包自己的名字**
//! ——这是设计而不是巧合：声明宏读的是 `crate::XIRANG_NAMESPACE`，而 `host!()` 用包名定义它，因此"包 = 命名空间"恰恰
//! 就是已构建宿主编译出的那一个。工作区根不是一个包，因此它没有自己的命名空间，而每个树级工具过去
//! 都在那里用 Cargo 自己的话拒绝。机器在，入口不在。
//!
//! This module is that entrance. When the root is a virtual manifest, `cargo metadata`
//! enumerates the members, and each one is read **from its own published records
//! first** (`<member>/target/xirang/out`, through `build_method`'s readers — see
//! `published.rs`). Deriving a member's faces is the *fallback*, taken only when it
//! published nothing, because deriving every member is what made a workspace answer
//! cost the sum of every member's source walk. The merged view groups by package,
//! and **every member appears with its status**: `published` (its own records, with
//! freshness), `not built` (no records, so the tree below was derived now and says
//! so), `no faces` (the record — or the derived tree — declares none, and saying so
//! is the answer), or `unresolvable` (with the reason). Neither silence nor a
//! whole-answer error is an answer: a framework member must not be dressed up as an
//! empty tree, and a nested package Cargo cannot resolve must say why rather than
//! vanish.
//! 本模块就是那个入口。根是虚拟清单时，`cargo metadata` 枚举成员，而每个成员**先按它自己已发布的
//! 记录**读取（`<member>/target/xirang/out`，经 `build_method` 的读取器——见 `published.rs`）。
//! 推导一个成员的面是**回落**路径，只在它什么都没发布时才走，因为逐成员推导正是让一份工作区答案
//! 等于每个成员源码遍历之和的原因。合并视图按包分组，而且**每个成员都带着它的状态出现**：
//! `published`（它自己的记录，带新鲜度）、`not built`（没有记录，因此下面的树是现推的，并且会说出来）、
//! `no faces`（记录——或推导出的树——不声明任何面，说出来就是答案）、或 `unresolvable`（带上原因）。
//! 沉默与整条报错都不是答案：框架成员不许被装扮成一棵空树，Cargo 解析不了的嵌套包必须说出原因而不是消失。
//!
//! The override still names one tree. `XIRANG_NAMESPACE` wins before Cargo is asked,
//! so with it set a virtual root is read as the package that name denotes — the one case
//! where a caller can give a workspace root an identity by hand.
//! 覆盖仍然只命名一棵树。`XIRANG_NAMESPACE` 在询问 Cargo 之前胜出，因此设置它之后，虚拟根会被
//! 读作那个名字所指的包——这是调用方唯一能手工给工作区根一个身份的情形。
//!
//! online: a virtual root merges members whose records may be absent, stale or newer than the build, and the merged answer has to say which each member was.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use serde_json::Value;
use xirang_kernel::lexicon;

use crate::build_method::FaceView;
use crate::mcp::build_evidence::out_dir;
use crate::mcp::published::{self, FACES_UNKNOWN, Publication, PublishedTree};
use crate::mcp::registry::namespace_from;
use crate::mcp::resolve::derived_faces;
use xirang_kernel::identity::NodeId;

/// What the configured root names.
/// 配置的根命名的是什么。
pub(crate) enum Scope {
    /// One package; the string is its identity namespace.
    /// 一个包；字符串就是它的身份命名空间。
    Package(String),
    /// A virtual manifest; these are its members, each with its own context.
    /// 一份虚拟清单；这些是它的成员，各自带着自己的上下文。
    Workspace(Vec<Member>),
    /// A manifest Cargo cannot resolve; the reason it gave.
    /// 一份 Cargo 解析不了的清单；它给出的原因。
    Unresolvable(String),
}

/// One member of a workspace, and what its own records or sources say.
/// 工作区的一个成员，以及它自己的记录或源码说了什么。
pub(crate) struct Member {
    /// The package name, which is this member's identity namespace.
    /// 包名，也就是这个成员的身份命名空间。
    pub(crate) name: String,
    /// The member's package directory, which is what its tools take as `root`.
    /// 成员的包目录，也就是它的工具接受的 `root`。
    pub(crate) dir: PathBuf,
    /// Whether that member published records, had to be derived, or could not be read.
    /// 该成员发布了记录、不得不现推，还是读不了。
    pub(crate) state: MemberState,
}

/// What one member's own evidence is.
/// 一个成员自己的证据是什么。
pub(crate) enum MemberState {
    /// The member's own `target/xirang/out`, read through `build_method`'s readers.
    /// 该成员自己的 `target/xirang/out`，经 `build_method` 的读取器读取。
    Published(PublishedTree),
    /// No readable published records, plus the reason — and the tree derived now, so
    /// the answer carries the same face half a single-package answer would.
    /// 没有可读的已发布记录，加上原因——以及现推的那棵树，因此答案带着与单包答案相同的面那一半。
    Unpublished {
        /// Why nothing was published, in the reader's own words.
        /// 为什么什么都没发布，用读取方自己的话。
        reason: String,
        /// The faces the derivation found.
        /// 推导找到的面。
        faces: Vec<FaceView>,
        /// The `unparsable faces N` line, or the empty string.
        /// `unparsable faces N` 那一行，或空串。
        unparsable: String,
    },
    /// The derivation could not run; the reason it gave.
    /// 推导跑不了；它给出的原因。
    Unresolvable(String),
}

/// The tree half of one member's answer, as the member holds it.
/// 一个成员答案里的树那一半，按该成员持有的样子。
///
/// Borrowed rather than owned, because the two halves live in different places:
/// a published tree was read into the member, and a derived one either was derived
/// at scope time or has to be derived per answer. A body that names faces by
/// logical path takes [`Member::derived_tree`] instead, and says so in its reply.
/// 借用而不是拥有，因为两半住在不同的地方：发布树是读进成员里的，而推导出的树要么在解析作用域时
/// 就已推导，要么得逐答案推导。按逻辑路径点名面的主体改用 [`Member::derived_tree`]，并在回复里
/// 说出来。
pub(crate) enum Tree<'a> {
    /// The member's own published records.
    /// 该成员自己已发布的记录。
    Published(&'a PublishedTree),
    /// The sources, derived: at scope time when nothing was published, now otherwise.
    /// 源码推导出的树：什么都没发布时在解析作用域时推导，否则此刻推导。
    Derived {
        /// The faces the derivation found.
        /// 推导找到的面。
        faces: &'a [FaceView],
        /// The `unparsable faces N` line, or the empty string.
        /// `unparsable faces N` 那一行，或空串。
        unparsable: &'a str,
    },
}

/// Whether a package's **published record** can answer "this tree declares no such face".
/// 一个包的**已发布记录**能否回答"这棵树没有这个面"。
///
/// Freshness and coverage are different facts, and the whole point of this function is to keep them
/// apart: a record can be perfectly current and still not carry every face the sources declare. Two
/// things put a row **somewhere else**: a declaration a fragment crate compiles (a native face), and
/// a subtree handed to a generated crate (`add_crates.rs`). A package that does neither has all of
/// its rows in its own record.
/// 新鲜度与覆盖是两个不同的事实，而这个函数的全部意义就是把它们分开：一份记录可以完全新鲜，却仍不携带源码
/// 声明的每一个面。有两样东西会把一行放到**别处**：碎片 crate 编译的声明（外语面），以及交给生成 crate 的
/// 子树（`add_crates.rs`）。两样都没有的包，所有行都在它自己的记录里。
///
/// The cheap gate comes first on purpose. Deriving costs a source walk, so it is paid only where the
/// answer must differ from the record's — and only by the callers that need it (an empty answer, not
/// every query).
/// 便宜的门槛有意放在前面。推导要花一次源码遍历，因此只在"答案必然与记录不同"的地方付这笔钱，而且只由真正
/// 需要它的调用方付（一次空答案，而不是每次查询）。
///
/// The callers that only need the boolean read `producer_coverage(..)?.complete`; there is no second
/// function for them, because a second spelling of the same question is how the two answers drift.
/// 只需要布尔值的调用方读 `producer_coverage(..)?.complete`；不为它们另设一个函数，因为同一个问题的第二种
/// 拼法正是两个答案开始漂移的方式。
#[cfg(test)]
pub(crate) fn record_covers_producers(dir: &Path, namespace: &str) -> Result<bool, String> {
    Ok(producer_coverage(dir, namespace)?.complete)
}

/// What the coverage question came to, **and** the derivation it had to make to answer it.
/// 覆盖问题的结论，**以及**为回答它而不得不做的那次推导。
///
/// The two travel together because the derivation is not free and its result is exactly what a
/// caller needs next: a non-empty record answer that does not cover every producer must be replaced
/// by the derivation, and deriving a second time for that replacement is the double work an
/// independent review measured. `derived` is `None` when the cheap gate did not fire — the record
/// covers every producer without walking any source, which is the hot path.
/// 两者一起返回，因为推导不是免费的，而它的结果恰恰是调用方下一步需要的：一条不覆盖每个生产者的非空记录
/// 答案必须由推导替代，而为这次替代再推一遍正是独立复核量到的重复工作。便宜门槛没有触发时 `derived` 为
/// `None`——记录不遍历任何源码就覆盖每个生产者，那是热路径。
pub(crate) struct ProducerCoverage {
    /// Whether the record may speak for every face the sources declare.
    /// 这份记录能否为源码声明的每个面代言。
    pub(crate) complete: bool,
    /// The derivation this decision already paid for, when it had to walk the sources.
    /// 这个判定已经付过的那次推导（当它不得不遍历源码时）。
    pub(crate) derived: Option<(Vec<FaceView>, String)>,
}

/// The one place that answers "may this record speak for the whole tree?", with its evidence.
/// 唯一回答"这份记录能否为整棵树代言？"的地方，连同它的依据。
pub(crate) fn producer_coverage(dir: &Path, namespace: &str) -> Result<ProducerCoverage, String> {
    let partitions = dir.join("add_crates.rs").is_file();
    let declares_native = match crate::build_method::declared_native::read(dir) {
        Ok(Some((_, faces))) => !faces.is_empty(),
        Ok(None) => false,
        // "I cannot tell" is not "nothing is declared": a build without the C++ adapter returns an
        // error here, and answering "the record is complete" on that error is how an object vanishes
        // from every query. The caller derives instead.
        // "我说不出来"不是"什么都没声明"：没有 C++ 适配器的构建在这里返回错误，而在那个错误上回答"记录是
        // 完整的"，正是对象从每个查询里消失的方式。调用方改为推导。
        Err(_) => {
            return Ok(ProducerCoverage {
                complete: false,
                derived: None,
            });
        }
    };
    if std::env::var("XIRANG_DEBUG_COMPLETE").is_ok() {
        eprintln!(
            "record_covers_producers: dir={} partitions={partitions} native={declares_native}",
            dir.display()
        );
    }
    if !partitions && !declares_native {
        return Ok(ProducerCoverage {
            complete: true,
            derived: None,
        });
    }
    let out = out_dir(dir);
    let Ok(rows) = crate::build_method::read_pruning_manifest(&out) else {
        return Ok(ProducerCoverage {
            complete: false,
            derived: None,
        });
    };
    let (faces, unparsable) = derived_faces(dir, namespace)?;
    // Every declared face must have a row. One missing is enough: the record would answer "no such
    // face" about an object that is right there in the sources.
    // 每一个被声明的面都必须有行。少一行就够：那份记录会对一个就在源码里的对象回答"没有这个面"。
    let complete = faces
        .iter()
        .all(|face| rows.iter().any(|row| row.id == face.id));
    Ok(ProducerCoverage {
        complete,
        derived: Some((faces, unparsable)),
    })
}

impl MemberState {
    /// The one word this member's row is labelled with.
    /// 这个成员那一行所用的唯一一个词。
    ///
    /// Two axes meet here, and the words keep them apart. `published` and
    /// `not built` say where the tree came from; `no faces` and `unresolvable` say
    /// what it is. `no faces` wins over the source axis when it applies, because
    /// "this package declares nothing" is the answer a reader needs and the
    /// evidence's provenance is the footnote — and a member that was never built
    /// still must not be dressed up as an empty tree, which is why the empty
    /// derived tree of an unbuilt member is `no faces` with the reason beside it
    /// rather than `not built` alone.
    /// 两个轴在此相遇，而词汇把它们分得开。`published` 与 `not built` 说树从哪里来；`no faces`
    /// 与 `unresolvable` 说它是什么。适用时 `no faces` 胜过来源轴，因为"这个包什么都不声明"才是
    /// 读取方需要的答案，证据的出处只是脚注——而一个从未构建过的成员仍然不许被装扮成一棵空树，这
    /// 正是为什么没构建过的成员的空推导树是 `no faces`、旁边带着原因，而不是光一个 `not built`。
    pub(crate) fn status(&self) -> &'static str {
        match self {
            Self::Published(tree) if tree.records_no_faces() => "no faces",
            Self::Published(..) => "published",
            Self::Unpublished { faces, .. } if faces.is_empty() => "no faces",
            Self::Unpublished { .. } => "not built",
            Self::Unresolvable(_) => "unresolvable",
        }
    }
}

impl Member {
    /// One package read as its own member, so a single-package answer and a merged
    /// one go through the same body with the same evidence.
    /// 把一个包当作它自己的成员来读，因此单包答案与合并答案经同一个主体、带着同一份证据。
    pub(crate) fn package(dir: &Path, name: String) -> Self {
        member(dir.to_path_buf(), name)
    }

    /// The one word this member's row is labelled with.
    /// 这个成员那一行所用的唯一一个词。
    pub(crate) fn status(&self) -> &'static str {
        self.state.status()
    }

    /// The tree half as this member holds it: its records, or the tree derived at
    /// scope time. Never reads a source file.
    /// 该成员持有的树那一半：它自己的记录，或解析作用域时推导出的树。绝不读源码文件。
    ///
    /// This is the entrance a body is handed its tree through, so it is also where a
    /// member's records are marked **used** — the answer is about to be built from them —
    /// and a used record is one whose freshness is checked. A member the answer never
    /// reaches through here is reported as not checked instead of being claimed current.
    /// 这就是主体拿到它的那棵树的入口，因此这里也是把成员记录标为**已用**的地方——答案即将据它构成
    /// ——而已用的记录才会被核验新鲜度。答案从未经这里到达的成员报成"未核验"，而不是被声称成当前。
    ///
    /// `Err` is the derivation that could not run, which is the answer a caller
    /// must report rather than swallow.
    /// `Err` 是跑不了的推导，那是调用方必须报出而不是吞掉的答案。
    pub(crate) fn tree(&self) -> Result<Tree<'_>, String> {
        match &self.state {
            // Handing the tree over is not the same as answering from it: a body that only
            // asks whether a tree exists must not buy a content hash. The records mark
            // themselves used where their *content* is read (`faces`, `has_identity`,
            // `evidence_line`), which is the moment this answer starts resting on them.
            // 把树交出去不等于据它作答：只探"有没有树"的主体不该买一次内容哈希。记录在**内容**被读到的
            // 地方自己标已用（`faces`、`has_identity`、`evidence_line`），那才是这份答案开始依托它的时刻。
            MemberState::Published(tree) => Ok(Tree::Published(tree)),
            MemberState::Unpublished {
                faces, unparsable, ..
            } => Ok(Tree::Derived { faces, unparsable }),
            MemberState::Unresolvable(reason) => Err(reason.clone()),
        }
    }

    /// The faces this member names by logical path, derived now when the member published records.
    /// 这个成员按逻辑路径点名的面；成员发布过记录时此刻推导。
    ///
    /// **Why it still derives** (the sentence here used to say "because the record carries no
    /// `path` and no `kind`", which stopped being true on 2026-09-29 when the pruning row began
    /// publishing those four facts — audit `W3-1` then added three more columns, so the record now
    /// carries the source hash, the declaration fingerprint and the direct calls too). What the
    /// record still does **not** carry is the two things a `FaceView` needs to be *constructed*
    /// rather than filled in: the resolved parent **`NodeId`** (the record publishes the parent as
    /// the Rust path the declaration spelled) and `owns_registry`. And a face added since the build
    /// is in the sources and not in the record at all. Until those are published, a caller that
    /// needs whole `FaceView`s pays this walk; the ones that need only the published columns read
    /// them (`published()`), and every caller states which tree it used (`evidence_line`).
    /// **为什么仍然推导**（这句话过去写的是"因为记录不携带 `path` 与 `kind`"，而那句从 2026-09-29 剪枝行
    /// 开始发布那四项事实起就不再成立；审计 `W3-1` 又加了三列，因此记录现在也带着面源码哈希、声明指纹与
    /// 直接调用名）。记录仍然**不**携带的，是把 `FaceView` **构造出来**（而不是填空）所需的两样：解析后的
    /// 父级 **`NodeId`**（记录发布的 `parent` 是声明拼出的 Rust 路径）与 `owns_registry`；而构建之后新增
    /// 的面只在源码里、根本不在记录里。在它们被发布之前，需要整份 `FaceView` 的调用方要付这次遍历；只需要
    /// 已发布列的调用方读记录（`published()`），而每个调用方都说出自己用了哪棵树（`evidence_line`）。
    ///
    /// This is the fallback the published path exists to avoid, and it is taken
    /// only by answers whose question the record cannot answer. Every caller
    /// states that it took it (`evidence_line`), so a reply never leaves a reader
    /// guessing which of the two trees it is looking at.
    /// 这就是发布路径要避免的那次回落，只有记录答不了其问题的答案才会走。每个调用方都会说出自己走了
    /// 它（`evidence_line`），因此回复从不让读取方猜自己看的是两棵树里的哪一棵。
    pub(crate) fn derived_tree(&self) -> Result<(Vec<FaceView>, String), String> {
        match &self.state {
            MemberState::Unpublished {
                faces, unparsable, ..
            } => Ok((faces.clone(), unparsable.clone())),
            MemberState::Published(..) => derived_faces(&self.dir, &self.name),
            MemberState::Unresolvable(reason) => Err(reason.clone()),
        }
    }

    /// The coverage answer with the derivation it already paid for (see [`producer_coverage`]).
    /// 覆盖结论，连同它已经付过的那次推导（见 [`producer_coverage`]）。
    pub(crate) fn producer_coverage(&self) -> Result<ProducerCoverage, String> {
        producer_coverage(&self.dir, &self.name)
    }

    /// The member's published records, when it has readable ones.
    /// 成员的已发布记录（当它有可读的记录时）。
    pub(crate) fn published(&self) -> Option<&PublishedTree> {
        match &self.state {
            MemberState::Published(tree) => Some(tree),
            _ => None,
        }
    }

    /// Whether the published record alone can say this member owns `id`.
    /// 仅凭已发布的记录能否说明这个成员拥有 `id`。
    ///
    /// A positive answer is final — the build wrote that identity into this
    /// package's manifest, so no other package holds it. A negative one is not:
    /// a face added since the build is in the sources and not in the record, so a
    /// caller that needs "no" still has to derive.
    /// 肯定答案是终局——构建把那个身份写进了这个包的清单，因此别的包不会持有它。否定答案不是：
    /// 构建之后新增的面在源码里而不在记录里，因此需要"没有"的调用方仍然必须推导。
    ///
    /// A **positive** answer is also what marks this record used: the reply's "this member
    /// owns it" came out of the record. A probe that came back negative is only routing —
    /// every member is asked, and the one that answers is the one the answer rests on.
    /// **肯定**答案同时也是把这份记录标为已用的东西：回复里"这个成员拥有它"出自这份记录。否定回来的
    /// 探问只是路由——每个成员都被问到，而答案真正依托的是答出肯定的那一个。
    pub(crate) fn publishes(&self, id: NodeId) -> bool {
        // `has_identity` marks the record used when — and only when — it answers yes, so the
        // rule has one spelling rather than two.
        // `has_identity` 在答"是"时（也只有那时）把记录标为已用，因此这条规则只有一处拼写。
        self.published().is_some_and(|tree| tree.has_identity(id))
    }

    /// The freshness line this member's census row carries.
    /// 这个成员的普查行所携带的新鲜度行。
    ///
    /// Only a member whose records an answer used gets a level: the census is composed
    /// after the bodies precisely so it can see which ones they read, and a member nothing
    /// was answered from says `not checked` rather than borrowing `current`. A member with
    /// no records at all gets the empty string — its row already says `not built (reason)`.
    /// 只有被某个答案用过的成员才拿到等级：普查在主体之后构成，正是为了看见主体读了哪些；没有答案据以
    /// 作答的成员写着"未核验"，而不是借用"当前"。完全没有记录的成员拿到空串——它那一行已经写着
    /// `not built (原因)`。
    pub(crate) fn freshness_line(&self) -> String {
        match &self.state {
            MemberState::Published(tree) if tree.was_used() => tree.freshness(),
            MemberState::Published(_) => crate::mcp::freshness::NOT_CHECKED.to_owned(),
            MemberState::Unpublished { .. } | MemberState::Unresolvable(_) => String::new(),
        }
    }

    /// The line naming which of the two trees this answer used.
    /// 说出这份答案用的是两棵树里哪一棵的那一行。
    ///
    /// A published answer names the directory and the fingerprint. A derived one
    /// says why it had to derive, in the record reader's own words — and a body
    /// that derived *over* a published member passes `because`, which is the
    /// question the record could not answer, so the line never leaves a reader
    /// guessing which tree it is looking at.
    /// 发布答案点名目录与指纹。推导出的答案用记录读取方自己的话说出为什么不得不推导——而在发布过的
    /// 成员之上**仍然**推导的主体传入 `because`，也就是记录答不了的那个问题，因此这一行从不让读取方
    /// 猜自己看的是哪棵树。
    pub(crate) fn evidence_line(&self, because: &str) -> String {
        match &self.state {
            MemberState::Published(tree) if !because.is_empty() => format!(
                "tree derived now ({because}); {} was read first and does not carry what this \
                 answer names faces by\n",
                tree.out.display(),
            ),
            MemberState::Published(tree) => tree.evidence_line(),
            MemberState::Unpublished { reason, .. } => format!(
                "tree derived now (no published records at {}; {})\n",
                out_dir(&self.dir).display(),
                one_line(reason),
            ),
            MemberState::Unresolvable(reason) => {
                format!("tree unavailable ({})\n", one_line(reason))
            }
        }
    }
}

/// Resolve what the root is, building a per-member identity context when it is a
/// workspace.
/// 解析根是什么；当它是工作区时，逐成员建好身份上下文。
///
/// A directory with no manifest at all is still an error rather than a scope: there is
/// nothing there to be wrong about, and the refusal names the way out
/// (`XIRANG_NAMESPACE`). A directory that *has* a manifest Cargo cannot resolve is a
/// different answer — it is a real root whose tree cannot be named — so it comes back as
/// [`Scope::Unresolvable`] and the tools report the reason in the body instead of
/// failing the call.
/// 完全没有清单的目录仍然是一个错误而不是一种范围：那里没有任何东西可以被弄错，而拒绝会点名出路
/// （`XIRANG_NAMESPACE`）。**有**清单而 Cargo 解析不了的目录是另一个答案——它是一个真实存在、
/// 却无法被命名的根——因此它以 [`Scope::Unresolvable`] 回来，工具在正文里报出原因，而不是让调用失败。
/// The workspace a member root belongs to, when this root is one of its members.
/// 当这个根是某个工作区的成员时，给出那个工作区。
///
/// The scenario round's measured failure was an answer given at a **member** root: asked there,
/// `callgraph {orphans: true}` reported two functions as orphans whose callers live in the other
/// member, because a member root simply cannot see them. The answer has to say so, and this is how
/// it learns there is something above it to point at.
/// 情景轮量到的失效是"在**成员**根上作答"：在那里问 `callgraph {orphans: true}`，两个调用者在另一个成员
/// 里的函数被报成孤儿，因为成员根本来看不见它们。答案必须说明这一点，而这个助手让它知道上面还有东西。
/// How far along a tree is: the one fact that decides which call comes next.
/// 一棵树走到哪一步了：决定"下一次该调什么"的那一个事实。
///
/// Measured (T-12): on an empty directory every tool an agent reaches for first answers about a
/// **failure** — `status` prints `rust_files=0` beside "manifest path … does not exist", `registry`
/// returns Cargo's own words about a missing manifest — and none of them says what to do instead.
/// The flow table's first scenario (`new_project`) was in the prose and not in the answers, so the
/// agent's opening calls were diagnostics on a tree that has nothing to diagnose yet.
/// 量到的（T-12）：在一个空目录上，代理最先够到的每个工具答的都是**失败**——`status` 在 "manifest path …
/// does not exist" 旁边打印 `rust_files=0`，`registry` 回的是 Cargo 自己关于缺清单的话——而没有一句说
/// 那该怎么办。流程表的第一个场景（`new_project`）在散文里、不在答案里，于是代理的开场调用是在一棵还
/// 没什么可诊断的树上做诊断。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Stage {
    /// No manifest here: there is no package to answer about yet.
    /// 这里没有清单：还没有包可以回答。
    Bare,
    /// A package with no registration faces: the tree exists and is empty of roles.
    /// 有包但没有注册面：树在，角色还空着。
    Faceless,
    /// Faces resolve: the tree is the thing the read tools answer about.
    /// 面能解析出来：这棵树就是那些读工具所回答的东西。
    Faceful,
}

/// Which stage the tree at `root` is in, decided by what is on disk rather than by what a tool
/// happens to need.
/// `root` 处的树处于哪个阶段——由磁盘上有什么决定，而不是由某个工具恰好需要什么决定。
pub(crate) fn stage(root: &Path) -> Stage {
    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Stage::Bare;
    }
    // A **workspace root** is not a tree that needs starting: it has members, and the roster is a
    // real answer about it (`registry` lists them). Refusing it as "faceless" swallowed exactly that
    // answer and turned a member listing into an invitation to run `apply {action: "add"}` on a root
    // that has no package to add to — measured by this file's own pin going red.
    // **工作区根**不是一棵需要起步的树：它有成员，而成员清单是关于它的真实答案（`registry` 会列出它们）。
    // 把它当成"无面"拒绝，恰好吞掉了那个答案，并把一份成员清单变成"对一个没有包可加的根跑
    // `apply {action: "add"}`"的邀请——这是本文件自己的钉子变红量出来的。
    let declares_workspace =
        std::fs::read_to_string(&manifest).is_ok_and(|text| text.contains("[workspace]"));
    if declares_workspace
        && !std::fs::read_to_string(&manifest)
            .unwrap_or_default()
            .contains("[package]")
    {
        return Stage::Faceful;
    }
    // Decided from the sources, not from Cargo: a tree whose manifest cannot resolve still has to
    // be placeable, and the answer that matters here is "are there roles declared yet". This is the
    // same lexical scan the read tools already do, so the stage costs nothing extra.
    // 由源码决定，而不是由 Cargo 决定：清单解析不出来的树也必须能被定位，而这里要紧的答案是"声明了角色没有"。
    // 这就是读工具本来就在做的那次词法扫描，因此这个阶段判定不额外花钱。
    let Ok(sources) = crate::mcp::source_index::load_sources(root) else {
        return Stage::Faceless;
    };
    let declares = declares_workspace
        || sources.iter().any(|file| {
            xirang_kernel::syntax::parse_faces(&file.source).is_ok_and(|faces| !faces.is_empty())
        });
    if declares {
        Stage::Faceful
    } else {
        Stage::Faceless
    }
}

/// The call that moves a tree one stage forward, as a reader would type it.
/// 把一棵树推进一个阶段的那次调用，按读者会敲的样子写。
pub(crate) fn entry_point(stage: Stage) -> Option<&'static str> {
    match stage {
        Stage::Bare => Some(
            "`new_project {directory, kind, package, apply: true}` writes the skeleton — the flow \
             table's first shape, and this tree is at it",
        ),
        Stage::Faceless => Some(
            "`apply {action: \"add\", parent: \"root\", fields: {…}, apply: true}` adds the first \
             registration face — until one exists there is nothing here for the read tools to \
             answer about",
        ),
        Stage::Faceful => None,
    }
}

/// Whether this tree hosts xirang at all — a `build.rs` plus a `xirang-toolchain` dependency.
/// 这棵树到底有没有宿主 xirang —— 一个 `build.rs` 加一条 `xirang-toolchain` 依赖。
///
/// This is the question that decides what "a face" can even mean. The round measured the whole
/// evaluation corpus: **the trees under test do not host xirang** (no `build.rs`, no `host!()`;
/// only `examples/control-button` does), so on them there is no build-time scope, no derived
/// registration tree, and `0 faces` is not a defect to explain but the answer. A tool that prints
/// "0 faces" without saying that leaves the reader to infer it — and the same reader then asks a
/// face-shaped question about a symbol.
/// 这个问题决定了"面"能意味着什么。那一轮量遍了评测语料：**被测的树不宿主 xirang**（没有
/// `build.rs`、没有 `host!()`；只有 `examples/control-button` 有），因此那些树上既没有构建期作用域、
/// 也没有派生注册树，而 `0 faces` 不是要解释的缺陷、就是答案本身。只印 "0 faces" 而不说这一点的工具
/// 把推断留给读者 —— 而那位读者随后会用一个"面"形状的问题去问一个符号。
pub(crate) fn hosts_xirang(root: &Path) -> bool {
    let mut dirs = vec![root.to_path_buf()];
    // Three levels covers `<root>`, a member, and a member's crate — deeper than any host layout
    // this repository ships, and it stops at the first manifest that names the dependency.
    // 三层够覆盖 `<root>`、一个成员、成员里的 crate —— 比本仓出厂的任何宿主布局都深，而且一旦有清单
    // 点名了那条依赖就停下。
    for _ in 0..3 {
        let mut next = Vec::new();
        for dir in dirs {
            if dir.join("build.rs").is_file()
                && std::fs::read_to_string(dir.join("Cargo.toml"))
                    .is_ok_and(|manifest| manifest.contains("xirang-toolchain"))
            {
                return true;
            }
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.file_name().is_some_and(|name| name != "target") {
                        next.push(path);
                    }
                }
            }
        }
        dirs = next;
    }
    false
}

pub(crate) fn enclosing_workspace(root: &Path) -> Option<std::path::PathBuf> {
    // A root whose own manifest declares `[workspace]` **is** a workspace root, so there is no
    // enclosing one to name. Without this check a tree nested under an unrelated project got a note
    // pointing at that project: measured on `target/round10/ours/trees/fe`, whose own `Cargo.toml`
    // says `[workspace] members = ["crates/core","crates/report"]` (and `cargo metadata` agrees its
    // `workspace_root` is the tree itself), while the note said the root was the member `fe` of the
    // workspace at `/home/nich/Moirai_N3` — a different project entirely.
    // 自己的清单就声明了 `[workspace]` 的根**就是** workspace 根，没有"外层"可点名。缺这一道检查，
    // 嵌在不相干工程下的树会拿到一条指向那个工程的提示：`target/round10/ours/trees/fe` 实测——它自己的
    // `Cargo.toml` 写着 `[workspace] members = ["crates/core","crates/report"]`（`cargo metadata` 也
    // 确认 `workspace_root` 就是这棵树），而提示却说这个根是 `/home/nich/Moirai_N3` 那个 workspace 的
    // 成员 `fe`——那是**另一个工程**。
    if std::fs::read_to_string(root.join("Cargo.toml"))
        .is_ok_and(|own| workspace_table(&own).is_some())
    {
        return None;
    }
    let name = root.file_name()?.to_string_lossy().to_string();
    let mut current = root.parent();
    while let Some(directory) = current {
        if let Ok(text) = std::fs::read_to_string(directory.join("Cargo.toml"))
            && let Some(members) = workspace_table(&text)
            && members.iter().any(|entry| entry == &name)
        {
            return Some(directory.to_path_buf());
        }
        current = directory.parent();
    }
    None
}

/// The last path segment of every entry in a manifest's `[workspace] members` array.
/// 一份清单 `[workspace] members` 数组里每个条目的最后一段路径。
///
/// Compared by **whole segment**, never by substring: the round measured `fe` matching the word
/// `features` in an unrelated manifest, which made a tree look like a member of another project.
/// 按**整段**比较，绝不按子串：那一轮量到 `fe` 命中了不相干清单里的 `features` 一词，于是这棵树看起来
/// 成了另一个工程的成员。
fn workspace_table(manifest: &str) -> Option<Vec<String>> {
    let at = manifest.find("members")?;
    let open = manifest[at..].find('[')? + at;
    let close = manifest[open..].find(']')? + open;
    Some(
        manifest[open + 1..close]
            .split(',')
            .filter_map(|entry| {
                let entry = entry.trim().trim_matches(|c| c == '"' || c == '\'').trim();
                if entry.is_empty() {
                    return None;
                }
                Some(entry.rsplit('/').next().unwrap_or(entry).to_owned())
            })
            .collect(),
    )
}

pub(crate) fn scope(root: &Path) -> Result<Scope, String> {
    let configured = std::env::var(lexicon::NAMESPACE_ENV).ok();
    match namespace_from(configured.as_deref(), root) {
        Ok(namespace) => Ok(Scope::Package(namespace)),
        Err(refusal) => {
            let manifest = root.join("Cargo.toml");
            if !manifest.is_file() {
                return Err(refusal);
            }
            let members = metadata_members(&manifest)
                .into_iter()
                .map(|(dir, name)| member(dir, name))
                .collect::<Vec<_>>();
            if members.is_empty() {
                return Ok(Scope::Unresolvable(refusal));
            }
            Ok(Scope::Workspace(members))
        }
    }
}

// The root whose full preamble this session has already spent, or `None` before the first.
// 本会话已经花掉完整前言的根，或第一条之前为 `None`。
//
// The decision this holds is deliberately **not** a cache of an answer: what is remembered is
// only *which root was opened*, never what was said about it, so a cheaper second answer can
// never report a tree the first one did not read.
// 这里保存的判断**有意**不是答案的缓存：记住的只是**打开过哪个根**，从不是"关于它说过什么"，因此更
// 便宜的第二次答案不可能报出一棵第一次没读过的树。
thread_local! {
    static LAST_ANNOUNCED: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Whether this answer is the one that opens a root, and if so, makes it the open one.
/// 这份答案是否就是打开某个根的那一份；是的话，把它记为当前打开的根。
///
/// "The first answer after a change of root" is a fact about the **session**, not about this
/// call, so it is kept here beside the roster that spends it rather than threaded through every
/// tool. It is thread-local for the same reason `freshness`'s reuse table is: the bridge serves
/// one session on one thread. A test that wants a tool's own answer rather than the one the
/// call order produced says so through [`forget_announced`].
/// "换根后的第一份答案"是关于**会话**的事实，不是关于这一次调用的，因此它留在这里、就在花掉它的普查
/// 旁边，而不是穿过每个工具去传。它是 thread-local 的，理由与 `freshness` 的复用表相同：桥在一个线程
/// 上服务一个会话。想要"工具自己的答案"而不是"调用次序产出的答案"的测试，通过 [`forget_announced`]
/// 说出来。
fn opens_root(root: &Path) -> bool {
    LAST_ANNOUNCED.with(|last| {
        let mut last = last.borrow_mut();
        let first = last.as_deref() != Some(root);
        if first {
            *last = Some(root.to_path_buf());
        }
        first
    })
}

/// Forget the root the last preamble opened, so a test asks what a **tool** answers rather than
/// what the call order before it produced.
/// 忘记上一次前言打开的根，好让测试问的是**工具**的答案，而不是它之前的调用次序产出的东西。
#[cfg(test)]
pub(crate) fn forget_announced() {
    LAST_ANNOUNCED.with(|last| *last.borrow_mut() = None);
}

/// The header every merged tree answer opens with: the root, the member census, and one
/// row per member with its status.
/// 每份合并树级答案开头的表头：根、成员普查，以及逐成员一行带状态。
///
/// The census is over *every* member, and each row is a fact about the tree that was
/// read rather than a result row: it is what keeps a framework crate from reading as an
/// empty host and a broken member from reading as an absent one. Its first four counts
/// are what this census has always said; `published` and `not built` are the evidence
/// axis, added so "read the build's records" and "had to derive" are told apart instead
/// of both reading as `queried`.
/// 普查覆盖**每一个**成员，而每一行都是关于"被读到的那棵树"的事实而不是结果行：正是它让框架 crate
/// 不会被读成一棵空的宿主树，也让坏掉的成员不会被读成不存在的成员。它的前四个计数是这个普查一直以来
/// 说的东西；`published` 与 `not built` 是证据轴，加进来是为了让"读了构建的记录"与"不得不推导"被
/// 区分开，而不是都读成 `queried`。
///
/// **A change of root is what buys the rows back.** The same root asked again in the same session
/// gets the one-line index instead — `workspace <root> · members N · built P · census <census>` —
/// because the rows said nothing about *this* call: they are the same four lines the previous
/// answer already carried, and the member sections below still name every member. The index
/// repeats the census verbatim rather than abbreviating it, so a reader (or a pin) that looked
/// for the counts finds them whether the answer opened the root or followed one.
/// **换根才是买回那些行的条件。** 同一个会话里再问同一个根，拿到的是那一行索引——`workspace <根> ·
/// members N · built P · census <普查>`——因为那些行说的不是**这次**调用的事：它们与上一份答案带的
/// 是同样四行，而下面的成员小节仍然点名每一个成员。索引逐字重复普查而不是缩写它，因此找这些计数的读者
/// （或钉子）无论答案是在打开一个根还是跟在别人后面，都能找到它们。
///
/// The rows themselves are folded here: the `not built` / `unresolvable` **reason** is expanded
/// only by the face and build tools ([`roster_expanded`]), which are the answers a reader goes to
/// when the one line is not enough. Every other tool still names the member's status word, and an
/// `unresolvable` member's own section — wherever the tool writes sections at all — states its
/// reason; the refusal to expand here is what keeps a lookup answer about the thing that was
/// looked up.
/// 行本身在这里是折起来的：`not built` / `unresolvable` 的**原因**只由面与构建类工具
/// （[`roster_expanded`]）展开——那一句话不够时读者正是去那些答案里看。其余工具仍然点名成员的**状态词**，
/// 而 `unresolvable` 成员自己的小节——在工具写小节的地方——说出它的原因；这里不展开，正是让一份查表答案
/// 说的是被查的那件事。
pub(crate) fn roster(root: &Path, members: &[Member]) -> String {
    roster_with(root, members, false)
}

/// The roster the face and build tools open with: the same census, with every member's
/// degradation reason spelled out.
/// 面与构建类工具开头的表头：同一份普查，外加逐成员的降级原因全文。
///
/// The reason is the reader's own words and it names an absolute path, so it is worth its space
/// exactly where the reader is asking about the **build** — `registry`, `diff` and `grafts` are
/// the three that come through [`merge`], and `explain`/`verify` reach it through the ownership
/// entrance. That caller is why this entry is `pub(crate)` rather than private to this module:
/// "the rows carry the reason" has one spelling, not a second one written where the caller is.
/// 原因是读取方自己的话，而且点名一条绝对路径，因此它值这点篇幅的地方恰恰是读者在问**构建**的时候
/// ——经 [`merge`] 进来的 `registry`、`diff` 与 `grafts` 正是这三位，而 `explain`/`verify` 经归属
/// 入口到达它。正是那个调用方让本入口成为 `pub(crate)` 而不是本模块私有物："行里带原因"只有一处
/// 拼法，而不是在调用方那里再写第二套。
pub(crate) fn roster_expanded(root: &Path, members: &[Member]) -> String {
    roster_with(root, members, true)
}

/// The roster in either of its two shapes, so the census is composed by one spelling.
/// 两种形态之一的表头，好让普查只有一处拼法。
fn roster_with(root: &Path, members: &[Member], reasons: bool) -> String {
    let census = census_line(members);
    if !opens_root(root) {
        let built = members
            .iter()
            .filter(|member| member.status() == "published")
            .count();
        return format!(
            "workspace {} · members {} · built {} · census {}\n",
            root.display(),
            members.len(),
            built,
            census.trim_end(),
        );
    }
    let mut output = format!(
        "workspace {} (virtual manifest: a workspace root is not a package, so it has no identity \
         namespace of its own; each member below has one)\n",
        root.display(),
    );
    output.push_str(&census);
    for member in members {
        output.push_str(&format!(
            "  {:<13} {:<30} {}\n",
            member.status(),
            member.name,
            detail(member, reasons),
        ));
    }
    output
}

/// The census line: one count per member status, in the four-then-two order this census has
/// always spelled.
/// 普查行：每个成员状态一个计数，按这个普查一直以来的"四加二"顺序。
fn census_line(members: &[Member]) -> String {
    let count = |word: &str| members.iter().filter(|m| m.status() == word).count();
    let with_faces = members
        .iter()
        .filter(|member| match &member.state {
            MemberState::Published(tree) => !tree.faces().is_empty(),
            MemberState::Unpublished { faces, .. } => !faces.is_empty(),
            MemberState::Unresolvable(_) => false,
        })
        .count();
    format!(
        "members {}  queried {}  no faces {}  unresolvable {}  published {}  not built {}\n",
        members.len(),
        with_faces,
        count("no faces"),
        count("unresolvable"),
        count("published"),
        count("not built"),
    )
}

/// Run one tool's per-package body for every member, under the roster.
/// 在表头之下，为每个成员跑一次某工具的逐包主体。
///
/// The body is the tool's own single-package implementation, handed the member —
/// its directory, its namespace, and the evidence this module already read — so a
/// merged answer and a single-package answer cannot disagree, because they are the
/// same code. A member whose derivation failed gets the `tree unavailable (reason)`
/// sentence in place of a body: the tool has no tree there, and it says so where
/// the reader is looking.
/// 主体就是该工具自己的单包实现，接收成员——它的目录、它的命名空间，以及本模块已经读到的证据——
/// 因此合并答案与单包答案不可能不一致，因为它们是同一段代码。推导失败的成员在主体的位置得到
/// `tree unavailable (原因)` 那一句：那里没有这棵树，而工具就在读者看的地方说出来。
pub(crate) fn merge<F>(
    root: &Path,
    members: &[Member],
    arguments: &Value,
    mut body: F,
) -> Result<String, String>
where
    F: FnMut(&Member, &Value) -> Result<String, String>,
{
    // The bodies run **before** the census is composed, and that order is load-bearing:
    // handing a body its tree is what marks that member's records *used*, and a used record
    // is the only one worth paying a content hash for. Composing the census first would print
    // `not checked` for every member — including the ones these very sections were built
    // from, which is a false statement about this answer.
    // 主体**先**跑、普查后构成，这个次序是要紧的：把树交给主体正是把该成员的记录标为**已用**的动作，
    // 而已用的记录才值得付一次内容哈希。先构成普查会对每个成员都印"未核验"——包括这些小节正是据以
    // 构成的成员，那是对这份答案本身的假陈述。
    let mut sections = String::new();
    for member in members {
        sections.push_str(&format!("\n== {} ({})\n", member.name, member.status()));
        match &member.state {
            MemberState::Unresolvable(reason) => {
                sections.push_str(&format!("tree unavailable ({reason})\n"));
            }
            _ => sections.push_str(&body(member, arguments)?),
        }
    }
    let mut output = roster_expanded(root, members);
    output.push_str(&sections);
    output.push_str(DETAIL);
    Ok(output)
}

/// The closing line every merged tree answer ends with, and the one place that spells it.
/// 每份合并树级答案结尾都会带的那一行，以及拼出它的唯一地方。
///
/// Two shapes of merged answer exist — the one [`merge`] renders, and the narrower ones
/// that resolve one owner or ask every member for its own answer — and they are read by
/// the same caller, so the sentence telling that caller how to get a single package's
/// answer has to read the same in all of them.
/// 合并答案有两种形状——[`merge`] 渲染的那一种，以及只解析出一个拥有者、或逐成员索取各自答案的更窄的
/// 那几种——而读它们的是同一个调用方，因此告诉它"怎么拿到单个包的答案"的那句话在它们之间必须读起来
/// 一样。
pub(crate) const DETAIL: &str = "detail: pass `root` as one member's directory (relative to this \
                                 root) for that package's own answer\n";

/// The body a tool answers with when the root itself has a manifest Cargo cannot resolve.
/// 根本身有一份 Cargo 解析不了的清单时，工具作答所用的正文。
///
/// This is the nested-package case: the fixture package under a test tree is a package in
/// its own right, but its manifest names path dependencies that no longer exist, so
/// `cargo metadata` fails and Cargo cannot name the namespace. Reporting the reason beats
/// both silence and a bare error, and the sentence names the two ways out.
/// 这就是嵌套包的情形：测试树下的夹具包本身是个包，但它的清单点名了已不存在的路径依赖，因此
/// `cargo metadata` 失败、Cargo 说不出命名空间。报出原因胜过沉默与光秃秃的错误，而这句话点名了两条出路。
pub(crate) fn unresolvable(root: &Path, reason: &str) -> String {
    format!(
        "unresolvable {}\n{reason}\nthis root has a Cargo.toml, and a tree answer needs the identity \
         namespace Cargo reports — every NodeId is a hash over it — so there is no tree here to \
         report. Set {} to name the tree explicitly, or point `root` at a resolvable package.\n",
        root.display(),
        lexicon::NAMESPACE_ENV,
    )
}

/// Why a member can hold no faces at all, spelled once for every census row and section.
/// 一个成员为什么可以一个面都没有；普查行与小节只拼一次。
pub(crate) const NO_FACES_REASON: &str = "no registration face under src/ — for a framework crate \
                                         that empty list is the answer rather than a missing one";

/// One member's row detail: its face count, why it has none, or why it could not be read.
/// 一个成员那一行的细节：它的面数、它为什么一个都没有，或它为什么读不了。
///
/// A derivation that dropped a registration file says so here too: `queried, 3 faces` over a
/// package whose fourth file nothing could parse would read as a whole tree, and this row is
/// where a reader looks first. The evidence is named in the same row, because "3 faces" is a
/// different fact depending on whether the build published them or this answer just derived
/// them.
/// 丢弃过注册面文件的推导在这里也要说出来：一个还有第四个文件谁也解析不了的包，若只写
/// `queried, 3 faces`，那就会被读成一棵完整的树，而这一行正是读者先看的地方。证据在同一行里点名，
/// 因为"3 个面"是不同的事实，取决于它们是构建发布的还是这份答案刚推导出来的。
///
/// `reasons` is the one difference between the two rosters: the `not built (…)` and
/// `tree unavailable (…)` tails are the reader's own words about the **build**, so they are
/// expanded where the build is the question ([`roster_expanded`]) and folded away everywhere else.
/// The status word itself is never folded: `not built` and `unresolvable` are what say the tree
/// below was derived rather than read, and that sentence is what this census exists for.
/// `reasons` 是两种表头之间唯一的差别：`not built (…)` 与 `tree unavailable (…)` 这两条尾巴是读取方
/// 关于**构建**的原话，因此在构建就是问题的地方展开（[`roster_expanded`]），其余地方折掉。**状态词
/// 本身从不折**：`not built` 与 `unresolvable` 正是说出下面那棵树是现推的而不是读来的那一句，而那句话
/// 正是这份普查存在的理由。
fn detail(member: &Member, reasons: bool) -> String {
    match &member.state {
        MemberState::Published(tree) => {
            let head = if tree.records_no_faces() {
                NO_FACES_REASON.to_owned()
            } else if tree.faces_unknown() {
                FACES_UNKNOWN.to_owned()
            } else {
                format!("{} faces", tree.faces().len())
            };
            format!("{head}; {}", member.freshness_line())
        }
        MemberState::Unpublished {
            faces,
            unparsable,
            reason,
        } => {
            let head = if faces.is_empty() {
                NO_FACES_REASON.to_owned()
            } else {
                format!("{} faces", faces.len())
            };
            let head = if unparsable.is_empty() {
                head
            } else {
                format!("{head}; {}", unparsable.trim())
            };
            if reasons {
                format!("{head}; not built ({})", one_line(reason))
            } else {
                format!("{head}; not built")
            }
        }
        MemberState::Unresolvable(reason) => {
            if reasons {
                format!("tree unavailable ({})", one_line(reason))
            } else {
                "tree unavailable".to_owned()
            }
        }
    }
}

/// The `cargo metadata` document for a manifest, or the reason Cargo gave none.
/// `cargo metadata` 为一份清单产出的文档，或 Cargo 拿不出它的原因。
///
/// One spelling of the command, so the member list and the feature faces cannot come from two
/// different invocations that disagree: everything a caller wants out of Cargo's view of a tree
/// comes through here.
/// 这条命令只有一份拼法，因此成员清单与特性面不可能出自两次互相矛盾的调用：调用方想从 Cargo 对一棵树
/// 的看法里取得的东西都经由这里。
pub(crate) fn metadata_json(manifest: &Path) -> Result<Value, String> {
    crate::cargo_metadata::read(manifest)
}

/// The members `cargo metadata` reports for a manifest, in Cargo's own order.
/// `cargo metadata` 为一份清单报告的成员，按 Cargo 自己的顺序。
///
/// A failure comes back as an empty list rather than an error, because the caller already
/// holds the reason: `package_name` ran the same command and its message names the
/// manifest and Cargo's stderr, which is the sentence worth reporting.
/// 失败以空清单而不是错误回来，因为调用方已经握有原因：`package_name` 跑过同一条命令，它的消息点名
/// 清单与 Cargo 的 stderr，那才是值得报出的句子。
fn metadata_members(manifest: &Path) -> Vec<(PathBuf, String)> {
    let Ok(metadata) = metadata_json(manifest) else {
        return Vec::new();
    };
    let (Some(packages), Some(members)) = (
        metadata["packages"].as_array(),
        metadata["workspace_members"].as_array(),
    ) else {
        return Vec::new();
    };
    members
        .iter()
        .filter_map(|id| {
            let id = id.as_str()?;
            let package = packages
                .iter()
                .find(|package| package["id"].as_str() == Some(id))?;
            let name = package["name"].as_str()?;
            let dir = Path::new(package["manifest_path"].as_str()?).parent()?;
            Some((dir.to_path_buf(), name.to_owned()))
        })
        .collect()
}

/// Read one member's own evidence: its published records when it has them, and the
/// derived tree — the reason it has none — only when it does not.
/// 读取一个成员自己的证据：有已发布记录时读记录，没有时才推导那棵树——以及它为什么没有。
///
/// This is the whole performance decision, in one function. Deriving here would
/// cost the member's entire source walk; reading the record costs a few files. The
/// derivation is not removed, it is made a *fallback*, and the state it produces
/// says which one the answer used.
/// 整个性能决定就在这一个函数里。在这里推导会花掉该成员的整趟源码遍历；读记录只花几个文件。
/// 推导没有被删掉，它变成了**回落**，而它产出的状态会说出答案用的是哪一种。
fn member(dir: PathBuf, name: String) -> Member {
    let state = match published::read(&dir) {
        Publication::Published(tree) => MemberState::Published(*tree),
        Publication::NotBuilt(reason) => match derived_faces(&dir, &name) {
            Ok((faces, unparsable)) => {
                // A tree that does not host XiRang has no build to read and no registration tree
                // to derive: an `*_object!` macro in it belongs to the application, and its unknown
                // fields are not "unparsable faces". Measured on a real application: 19 rows of
                // "`handle` is not a registration face field" for a macro this framework never
                // owned (see `docs/multilang-support-matrix-current-2026-10-27.md` §6).
                // 不宿主 XiRang 的树没有可读的构建、也没有可派生的注册树：其中的 `*_object!` 宏属于应用，
                // 它的未知字段不是"unparsable faces"。在一个真实应用上实测到 19 行
                // "`handle` is not a registration face field"，而那个宏从来不属于本框架。
                let unparsable = if faces.is_empty()
                    && !unparsable.is_empty()
                    && !hosts_xirang(&dir)
                {
                    "does not host xirang (no `build.rs` + `xirang-toolchain` dependency): these declarations are application macros, not registration faces"
                        .to_owned()
                } else {
                    unparsable
                };
                MemberState::Unpublished {
                    reason,
                    faces,
                    unparsable,
                }
            }
            Err(reason) => MemberState::Unresolvable(reason),
        },
    };
    Member { name, dir, state }
}

/// Fold a possibly multiline reason into the one line a roster row can carry.
/// 把可能多行的原因折成表头一行装得下的那一行。
///
/// It is `pub(crate)` because the ownership entrance renders the same kind of line for a
/// member that derived its tree but could not answer this request: `one_line` here, not a
/// second folding rule there, is what keeps a two-line Cargo message from breaking the
/// layout of a body that both entrances write.
/// 它是 `pub(crate)`，因为归属入口会为"推导出了树、但答不了这次请求"的成员渲染同一类行：这里
/// 的 `one_line`、而不是那里的第二条折叠规则，才是让两行的 Cargo 消息不会弄坏两个入口所写正文排版的
/// 原因。
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
