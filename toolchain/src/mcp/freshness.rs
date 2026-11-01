//! The freshness level of a member's published records, and what this answer paid to
//! know it.
//! 成员已发布记录的新鲜度等级，以及这份答案为知道它付出了什么。
//!
//! The verdict itself is unchanged and stays where it was: `build_output_is_current`
//! hashes the sources, and nothing here substitutes a timestamp for that hash — this
//! repository has been burned by `cp -a`/`rsync -a` keeping an mtime while the content
//! changed.
//! 判据本身没有改，也仍在原处：`build_output_is_current` 对源码取哈希，这里没有任何东西拿时间戳
//! 代替那个哈希——本仓踩过 `cp -a`/`rsync -a` 保留 mtime 而内容已变的坑。
//!
//! What this module adds is the *level* a report prints, because "the same answer every
//! time" and "the same answer without paying for the hash again" are different claims:
//! 本模块加的是报告打印的**等级**，因为"每次都给出同一个答案"与"不再付一次哈希的钱也给出同一个
//! 答案"是两个不同的声称：
//!
//! - `freshness: content-verified at HH:MM:SS` — the content hash ran for this answer.
//! - `freshness: reused (content-verified at HH:MM:SS, Ns ago; window 30s)` — this answer
//!   read a verification taken inside the declared window. It never reads as `current`:
//!   复用与"当前"是两个词，把它们混同正是这套等级要阻止的事。
//! - `build stale (run `xirang check`)` — the record no longer describes these sources,
//!   or carries no fingerprint at all.
//! - `freshness: not checked (its records were not used for this answer)` — the member
//!   contributed nothing to this answer, so nothing was paid for it.
//! - `unknown (...)` — a caller asked for reuse only, and the window held nothing.
//!
//! The window is [`REUSE_WINDOW_SECONDS`], a constant this module documents and its tests
//! assert. A caller overrides the policy per request with the `freshness` argument
//! (`reuse` is the default, `verify` pays immediately, `reuse-only` never pays), and every
//! tool whose answer can carry a member census advertises it.
//! 窗口是本模块文档化、其测试也断言的常数 [`REUSE_WINDOW_SECONDS`]。调用方按请求用 `freshness`
//! 参数覆盖策略（默认 `reuse`，`verify` 立即付费，`reuse-only` 永不自付），而每个答案可能带成员
//! 普查的工具都会声明它。
//!
//! The rule the levels exist to protect is **no claim without a check**: a member whose
//! records this answer used is verified, or its verification is reused *and labelled as
//! reused*; a member the answer did not use says so instead of borrowing the word. The
//! single-package build evidence (`build_evidence.rs`) answers a different question — the
//! package's own output — and keeps its own `current`/`stale` spelling.
//! 这些等级要守的规则是**不校验就不声称**：据以作答的成员会被核验，或复用上一次核验并**标明是
//! 复用**；没有被这份答案用到的成员如实说出来，而不是借用那个词。单包的构建证据
//! （`build_evidence.rs`）回答的是另一个问题——这个包自己的产物——并保留它自己的
//! `current`/`stale` 拼法。
//!
//! The verification is remembered per **process, per thread**, which is what makes a
//! long-lived bridge cheap: the second call about the same member inside the window buys
//! nothing new. It is not a hidden memoization — the report says `reused` and names the
//! moment — and it is not process-wide, so two tests running side by side do not read each
//! other's verdicts.
//! 核验按**进程内、线程内**记着，这正是长驻桥便宜的原因：窗口内关于同一个成员的下一次调用不再
//! 买新东西。它不是隐藏的记忆化——报告写着 `reused` 并点名时刻——也不是进程级的，因此并行跑的两条
//! 测试不会读到彼此的裁决。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::build_method::build_output_is_current;

/// How long, in seconds, a content verification may be reused without paying again.
/// 一次内容核验可以不付费复用的时长（秒）。
///
/// The window is a declared constant rather than a tuning detail: the answer prints it
/// (`window 30s`), this module's doc names it, and its tests assert it, so a caller can
/// read what "reused" means without reading this file. It is short on purpose — the point
/// is to make a burst of calls on one member cheap, not to remember a tree for a session.
/// 窗口是声明出来的常数而不是调参细节：答案里印着它（`window 30s`），本模块文档点名它，其测试
/// 断言它，因此调用方不用读这个文件就知道"复用"意味着什么。它有意很短——目的是让针对同一个成员的
/// 一串调用便宜，而不是把一棵树记一整个会话。
pub(crate) const REUSE_WINDOW_SECONDS: u64 = 30;

/// What a census row prints for a member this answer did not use.
/// 这份答案没有用到的成员，其普查行打印的内容。
pub(crate) const NOT_CHECKED: &str =
    "freshness: not checked (its records were not used for this answer)";

/// What a `reuse-only` call prints when the window holds no verification.
/// `reuse-only` 调用在窗口内没有核验时打印的内容。
pub(crate) const UNKNOWN: &str = "unknown (no content verification inside the reuse window, and \
                                  this call asked not to pay for one)";

/// The word every stale verdict is spelled with, in one place.
/// 每个"过期"裁决共用的唯一词形。
const STALE: &str = "build stale (run `xirang check`)";

/// How much of the freshness question one call is willing to pay for.
/// 一次调用愿意为新鲜度问题付多少。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Policy {
    /// Reuse a verification from inside [`REUSE_WINDOW_SECONDS`], otherwise pay.
    /// 复用 [`REUSE_WINDOW_SECONDS`] 之内的核验，否则付费。
    Reuse,
    /// Pay for a content verification now, whatever the window holds.
    /// 现在就付一次内容核验，无论窗口里有什么。
    Verify,
    /// Never pay; print [`UNKNOWN`] when the window holds nothing.
    /// 永不自付；窗口里没有东西时打印 [`UNKNOWN`]。
    ReuseOnly,
}

/// The policy a request asks for, by the `freshness` argument it carries.
/// 请求按它携带的 `freshness` 参数要求的策略。
///
/// An unknown value reads as the default rather than failing the call: the argument
/// narrows a cost, and a typo in it must not take a query down with it. The default is the
/// one that cannot mislead — reuse is always labelled as reuse.
/// 无法识别的取值按默认处理而不是让调用失败：这个参数只是收窄一项开销，其中的错别字不该把一次查询
/// 一起带走。默认值是那个不会误导人的——复用永远被标明为复用。
pub(crate) fn policy_from(arguments: &Value) -> Policy {
    match arguments.get("freshness").and_then(Value::as_str) {
        Some("verify") => Policy::Verify,
        Some("reuse-only") => Policy::ReuseOnly,
        _ => Policy::Reuse,
    }
}

thread_local! {
    /// The policy this thread's current tool call asked for.
    /// 本线程当前工具调用所要求的策略。
    static POLICY: Cell<Policy> = const { Cell::new(Policy::Reuse) };
    /// The verifications this thread has already paid for, keyed by `(root, out)`.
    /// 本线程已经付过费的核验，按 `(root, out)` 索引。
    static VERIFIED: RefCell<HashMap<(PathBuf, PathBuf), Verified>> =
        RefCell::new(HashMap::new());
    /// The stamps this answer already walked, keyed by `(root, out)` (audit `T1`, cut 6).
    /// 这份答案已经走过的树戳，按 `(root, out)` 索引（审计 `T1` 第六刀）。
    ///
    /// A stamp is only a **guard for the memo**, and one answer asks the same guard three times —
    /// `roots_with_freshness` pays, the census prints the level, and `TreeDelta::read` reads the
    /// verdict. Walking the tree three times per member to answer the same question cost **2.0 s of
    /// an 8.5 s answer** at 50,000 files, measured; this shares one walk with those three readers
    /// and nothing else. It is cleared by [`begin_answer`], so a stamp never vouches for a tree
    /// across two answers — which is what keeps the pin
    /// `a_published_face_is_ok_and_a_new_one_is_added_since_build` green, since the file it adds
    /// between the two searches changes the count and fails the stored stamp.
    /// 戳只是**给记忆用的守卫**，而一份答案会问同一个守卫三次——`roots_with_freshness` 付费、普查打印
    /// 等级、`TreeDelta::read` 取裁决。在 50,000 文件上，为回答同一个问题而每个成员走三遍树花了
    /// **8.5 s 答案里的 2.0 s**（实测）；这里把一遍走法分给那三个读者，仅此而已。它由 [`begin_answer`]
    /// 清空，因此戳绝不跨两份答案为一棵树作保——这正是让钉子
    /// `a_published_face_is_ok_and_a_new_one_is_added_since_build` 保持绿色的东西：它在两次搜索之间
    /// 新增的文件会改变计数、让存下的戳对不上。
    static STAMPS: RefCell<HashMap<(PathBuf, PathBuf), (usize, u64)>> =
        RefCell::new(HashMap::new());
    /// Which answer this thread is currently producing (audit `T1`, cut 7).
    /// 本线程当前正在产出的第几份答案（审计 `T1` 第七刀）。
    ///
    /// A verdict that an answer **paid for itself** is not a reuse, and saying `reused (... 0s ago)`
    /// about it is a worse answer than saying `content-verified`: measured on the 50,000-file
    /// workspace, sharing the payment between the readers silently turned every roster row from
    /// `freshness: content-verified at 23:34:22` into `freshness: reused (... 5s ago)`, which is a
    /// different claim about the same work. This counter is how the line tells the two apart.
    /// 一份答案**自己付过费**的裁决不是复用，而用 `reused (... 0s ago)` 说它是比 `content-verified`
    /// 更差的答案：在 50,000 文件的工作区上实测，把付费在几个读者之间共享，静默地把每一行普查从
    /// `freshness: content-verified at 23:34:22` 变成了 `freshness: reused (... 5s ago)`——关于同一件
    /// 工作，这是两种不同的声称。这个计数器就是那行字区分两者的依据。
    static ANSWER: Cell<u64> = const { Cell::new(0) };
}

/// Open a new answer: the stamps walked for the previous one vouch for nothing here.
/// 开启一份新答案：上一份答案走过的戳在这里不作保。
///
/// Called where an answer begins — by [`set_policy`], which every dispatched call goes through, and by
/// the workspace census reader, which is where a direct in-process caller's answer starts. Only the
/// **stamp memo** is dropped; the verifications themselves stay, because that is what the reuse window
/// is: a verdict may be reused across answers, but only after the tree behind it is walked again.
/// 在答案开始的地方调用——[`set_policy`]（每次派发调用都经过它）与工作区普查读取方（进程内直接调用方的
/// 答案从这里开始）。被丢掉的只是**戳记忆**；核验本身留着，因为那正是复用窗口的含义：一份裁决可以跨答案
/// 被复用，但只在那棵树被重新走过一遍之后。
pub(crate) fn begin_answer() {
    ANSWER.with(|answer| answer.set(answer.get() + 1));
    STAMPS.with(|stamps| stamps.borrow_mut().clear());
}

/// Which answer this thread is producing.
/// 本线程正在产出的第几份答案。
fn answer_epoch() -> u64 {
    ANSWER.with(Cell::get)
}

/// One content verification, with when it happened and what it said.
/// 一次内容核验，连同它发生的时刻与它给出的结论。
struct Verified {
    /// The monotonic moment the hash ran, which is what the window is measured against.
    /// 哈希运行的单调时刻，窗口就是相对它度量的。
    at: Instant,
    /// That moment as `HH:MM:SS` (UTC), which is what the report prints.
    /// 那个时刻的 `HH:MM:SS`（UTC），也就是报告打印的。
    clock: String,
    /// Whether the output still described the sources at that moment.
    /// 那一刻产物是否仍在描述这批源码。
    current: bool,
    /// The cheap stamp of the sources at that moment (audit `T1`, cut 6).
    /// 那一刻源码的廉价戳（审计 `T1` 第六刀）。
    ///
    /// A verdict is cached **only while this stamp still matches**: a file count plus the newest
    /// modification time under the tree, both read from directory entries without opening a single
    /// file. Without it, sharing one verdict between the provenance reader and the verdict readers
    /// meant one call could answer from another call's tree — which is the pin
    /// `a_published_face_is_ok_and_a_new_one_is_added_since_build` reddening, measured.
    /// 裁决**只在戳仍然匹配时**被复用：树下的文件数 + 最新修改时间，两者都只读目录项、不打开任何文件。
    /// 没有它，把一份裁决在"出处读者"和"裁决读者"之间共享，就意味着一次调用可能拿另一次调用的树作答——
    /// 那正是钉子 `a_published_face_is_ok_and_a_new_one_is_added_since_build` 变红的原因（实测过）。
    stamp: (usize, u64),
    /// The answer that paid for this verification (audit `T1`, cut 7).
    /// 为这次核验付费的那份答案（审计 `T1` 第七刀）。
    answer: u64,
}

/// Set the policy for the calls that follow on this thread.
/// 为本线程随后的调用设置策略。
pub(crate) fn set_policy(policy: Policy) {
    POLICY.with(|cell| cell.set(policy));
    begin_answer();
}

/// The policy this thread is under.
/// 本线程当前所处的策略。
fn policy() -> Policy {
    POLICY.with(Cell::get)
}

/// The freshness line for one published output directory.
/// 一个已发布产物目录的新鲜度行。
pub(crate) fn line(root: &Path, out: &Path) -> String {
    let key = key(root, out);
    match policy() {
        Policy::Verify => paid_line(root, out, None),
        Policy::Reuse => match lookup(&key, root) {
            Lookup::Hit(verified) => reused_line(&verified),
            Lookup::Miss { stamp } => paid_line(root, out, stamp),
        },
        Policy::ReuseOnly => match lookup(&key, root) {
            Lookup::Hit(verified) => reused_line(&verified),
            Lookup::Miss { .. } => UNKNOWN.to_owned(),
        },
    }
}

/// The key one published output is remembered under.
/// 一份已发布产物被记下的键。
fn key(root: &Path, out: &Path) -> (PathBuf, PathBuf) {
    (root.to_path_buf(), out.to_path_buf())
}

/// What this thread already knows about one published output inside the window.
/// 本线程在窗口之内对一份已发布产物已经知道的东西。
enum Lookup {
    /// A verification inside the window whose stamp still matches this tree.
    /// 窗口之内、且其戳仍与这棵树相符的一次核验。
    Hit(Verified),
    /// Nothing reusable — with the stamp the caller must pay under, when one had to be walked.
    /// 没有可复用的东西——需要走一遍时，连同调用方付费时必须用的那个戳。
    ///
    /// `None` when there was nothing to validate: a member with no remembered verdict needs no stamp
    /// walk to decide that it must pay, and the walk then happens **inside the payment** (audit `T1`,
    /// cut 7) — which is where it can run beside the other members'.
    /// 当没有任何东西需要校验时是 `None`：没有记忆裁决的成员不需要走戳就能判定它必须付费，于是这次走法
    /// 发生在**付费里面**（审计 `T1` 第七刀）——而那里它才能与其它成员的走法并行。
    Miss {
        /// The stamp of the tree as this answer sees it, when one was walked.
        /// 这份答案所见的树戳，走过时才有。
        stamp: Option<(usize, u64)>,
    },
}

/// What an answer must do about one member (audit `T1`, cut 7).
/// 一份答案对一个成员必须做的事（审计 `T1` 第七刀）。
pub(crate) enum Consultation {
    /// Reuse this verdict: the window holds one and the tree behind it has not moved.
    /// 复用这个裁决：窗口里有一份，而它背后的树没有动过。
    Remembered(bool),
    /// Pay for one, under this stamp when one was already walked.
    /// 付费；已经走过戳时就用那个戳。
    Pay {
        /// The stamp of the tree as this answer sees it, when one was walked.
        /// 这份答案所见的树戳，走过时才有。
        stamp: Option<(usize, u64)>,
    },
}

/// One member's content verification, paid for but not yet recorded (audit `T1`, cut 7).
/// 一个成员的内容核验，已付费但尚未记下（审计 `T1` 第七刀）。
///
/// Split from the recording so the payment can happen on **another thread**: the per-member work is
/// where the seconds are (a whole-tree hash per member), one member's verdict does not depend on
/// another's, and nothing about this value touches this thread's memo. [`record`] puts it back.
/// 与"记下"拆开，是为了让付费可以发生在**另一个线程**上：每个成员的活正是秒数所在（每成员一次整树
/// 哈希），一个成员的裁决不依赖另一个，而这个值不碰本线程的任何记忆。[`record`] 把它放回去。
pub(crate) struct Paid {
    /// Whether the output still described the sources when the hash ran.
    /// 哈希运行时，产物是否仍在描述这批源码。
    current: bool,
    /// The moment the hash ran, as `HH:MM:SS` (UTC).
    /// 哈希运行的时刻，`HH:MM:SS`（UTC）。
    clock: String,
    /// The stamp the caller handed in, kept so the next lookup can compare against it.
    /// 调用方交进来的戳，留着让下一次查找能与之对比。
    stamp: (usize, u64),
}

/// The policy this thread's current tool call asked for.
/// 本线程当前工具调用所要求的策略。
pub(crate) fn current_policy() -> Policy {
    policy()
}

/// What an answer must do about one member, under a policy it names (audit `T1`, cut 7).
/// 一份答案在**它点名的**策略下对一个成员必须做的事（审计 `T1` 第七刀）。
///
/// The policy is a parameter rather than a second read of this thread's cell because the caller may
/// be deciding for several members at once, and the decision has to be the caller's request, not
/// whatever the worker thread it later runs on happens to hold.
/// 策略是参数而不是再读一次本线程的单元，因为调用方可能一次为好几个成员做决定，而这个决定必须是调用方
/// 的请求，而不是它随后运行的那个工作线程碰巧持有的东西。
pub(crate) fn consult(root: &Path, out: &Path, policy: Policy) -> Consultation {
    let key = key(root, out);
    match policy {
        Policy::Verify => Consultation::Pay { stamp: None },
        // Never pay: without a matching remembered verdict the honest answer is "not known to be
        // current", which sends the readers down the deriving path rather than trusting the record.
        // 永不自付：没有匹配的记忆时，诚实的答案是"不知道它新鲜"，让读者走推导那条路，而不是相信记录。
        Policy::ReuseOnly => {
            Consultation::Remembered(matches!(lookup(&key, root), Lookup::Hit(v) if v.current))
        }
        Policy::Reuse => match lookup(&key, root) {
            Lookup::Hit(verified) => Consultation::Remembered(verified.current),
            Lookup::Miss { stamp } => Consultation::Pay { stamp },
        },
    }
}

/// Pay for one member's content hash on **this** thread, under a stamp the caller walked.
/// 在**本**线程上为一个成员的内容哈希付费，用的是调用方走过的那个戳。
///
/// Touches no memo: this is the half that may run on a worker thread.
/// 不碰任何记忆：这是可以跑在工作线程上的那一半。
pub(crate) fn pay_with_stamp(root: &Path, out: &Path, stamp: Option<(usize, u64)>) -> Paid {
    let stamp = stamp.unwrap_or_else(|| crate::build_method::source_stamp(root));
    Paid {
        current: build_output_is_current(root, out),
        clock: wall_clock(),
        stamp,
    }
}

/// Record a payment as one **this answer** took, and report what it said (audit `T1`, cut 7).
/// 把一次付费记为**本份答案**自己做过的，并回报它说了什么（审计 `T1` 第七刀）。
pub(crate) fn record(root: &Path, out: &Path, paid: Paid) -> bool {
    let verified = Verified {
        at: Instant::now(),
        clock: paid.clock,
        current: paid.current,
        stamp: paid.stamp,
        answer: answer_epoch(),
    };
    let current = verified.current;
    let stamp = verified.stamp;
    VERIFIED.with(|store| store.borrow_mut().insert(key(root, out), verified));
    // The stamp this payment was taken under is now this answer's stamp: remembering it here is what
    // lets the readers that follow (`TreeDelta`, the census line) reuse the verdict without walking
    // the tree again — including when the walk happened on a worker thread.
    // 这次付费所用的戳从此就是**本份答案**的戳：在这里记住它，正是让随后那些读取方（`TreeDelta`、普查
    // 行）不必再走一遍树就能复用裁决的东西——包括那次走法发生在工作线程上的情形。
    STAMPS.with(|stamps| stamps.borrow_mut().insert(key(root, out), stamp));
    current
}

/// Look one published output up, walking the guard stamp exactly once.
/// 查一份已发布产物，并且只走一遍守卫用的戳。
fn lookup(key: &(PathBuf, PathBuf), root: &Path) -> Lookup {
    let window = Duration::from_secs(REUSE_WINDOW_SECONDS);
    let expired = VERIFIED.with(|store| {
        let mut store = store.borrow_mut();
        let verified = store.get(key)?;
        if verified.at.elapsed() >= window {
            store.remove(key);
            return None;
        }
        Some(Verified {
            at: verified.at,
            clock: verified.clock.clone(),
            current: verified.current,
            stamp: verified.stamp,
            answer: verified.answer,
        })
    });
    let Some(verified) = expired else {
        // Nothing to validate: the stamp walk belongs to the payment that is about to happen, where
        // it can run beside every other member's.
        // 没有东西需要校验：该走的戳属于即将发生的那次付费，而那里它能与其它每个成员的走法并行。
        return Lookup::Miss { stamp: None };
    };
    let stamp = stamp_for(key, root);
    if verified.stamp == stamp {
        Lookup::Hit(verified)
    } else {
        VERIFIED.with(|store| store.borrow_mut().remove(key));
        Lookup::Miss { stamp: Some(stamp) }
    }
}

/// Pay for the content hash, remember it, and spell the line for a payment this answer made.
/// 为内容哈希付费、记下它，并为"本份答案自己做过的付费"拼出那一行。
fn paid_line(root: &Path, out: &Path, stamp: Option<(usize, u64)>) -> String {
    let paid = pay_with_stamp(root, out, stamp);
    let clock = paid.clock.clone();
    if record(root, out, paid) {
        format!("freshness: content-verified at {clock}")
    } else {
        STALE.to_owned()
    }
}

/// Whether this published output still describes its sources — **the one place that pays for the
/// content hash** (audit `T1`, cut 6).
/// 这份已发布产物是否仍描述着它的源码——**唯一为内容哈希付费的地方**（审计 `T1` 第六刀）。
///
/// Every reader that turns freshness into a decision comes through here, which is what makes the memo
/// shared work rather than a per-reader cache: one `search` used to ask twice per member — once through
/// the provenance line, once directly for its verdicts — and at 50,000 files that was **40 content
/// hashes instead of 20**, measured as 3.5 s of a 7.9 s answer. The stamp is what makes the sharing
/// safe: a tree that gained, lost or touched a file fails the stamp and pays again, so the pin that
/// writes a face beside a current record still sees `[added since build]`.
/// 每个把新鲜度变成决定的读者都经过这里，这正是让那份记忆成为**共享工作**而不是每个读者各自一份缓存的原因：
/// 一次 `search` 过去对每个成员问两次——一次经出处行、一次直接取裁决——在 50,000 文件上那就是 **40 次内容
/// 哈希而不是 20 次**，实测占 7.9 s 答案里的 3.5 s。**戳**是让共享安全的东西：一棵多了、少了或被碰过的文件
/// 的树过不了戳、会重新付费，于是"在一份新鲜记录旁边写下一个面"的那条钉子照旧看到 `[added since build]`。
pub(crate) fn verdict(root: &Path, out: &Path) -> bool {
    let key = key(root, out);
    match policy() {
        // The caller asked to pay now, whatever the window holds.
        // 调用方要求现在就付，无论窗口里有什么。
        Policy::Verify => record(root, out, pay_with_stamp(root, out, None)),
        Policy::Reuse => match lookup(&key, root) {
            Lookup::Hit(verified) => verified.current,
            Lookup::Miss { stamp } => record(root, out, pay_with_stamp(root, out, stamp)),
        },
        // Never pay: without a matching remembered verdict the honest answer is "not known to be
        // current", which sends the readers down the deriving path rather than trusting the record.
        // 永不自付：没有匹配的记忆时，诚实的答案是"不知道它新鲜"，让读者走推导那条路，而不是相信记录。
        Policy::ReuseOnly => {
            matches!(lookup(&key, root), Lookup::Hit(verified) if verified.current)
        }
    }
}

/// The stamp of a tree, walked at most once per answer (audit `T1`, cut 6).
/// 一棵树的戳，每份答案最多走一遍（审计 `T1` 第六刀）。
///
/// The walk is the whole cost of the guard, and one answer asks it three times; [`begin_answer`] is
/// what keeps the sharing inside one answer. A miss is not a verdict: the caller still compares this
/// stamp against the stored one and pays for the content hash when they differ.
/// 这次遍历就是守卫的全部成本，而一份答案要问三遍；[`begin_answer`] 就是让共享留在一份答案之内的东西。
/// 命中不是裁决：调用方仍把这个戳与存下的那个对比，不同就为内容哈希付费。
fn stamp_for(key: &(PathBuf, PathBuf), root: &Path) -> (usize, u64) {
    if let Some(stamp) = STAMPS.with(|stamps| stamps.borrow().get(key).copied()) {
        return stamp;
    }
    let stamp = crate::build_method::source_stamp(root);
    STAMPS.with(|stamps| stamps.borrow_mut().insert(key.clone(), stamp));
    stamp
}

/// The line for a verification this answer may use, saying which of the two things it is.
/// 本份答案可以使用的一次核验，其对应的行——并说出它究竟是两者中的哪一种。
///
/// **A verification this answer paid for itself is not a reuse** (audit `T1`, cut 7). The two used to
/// share one wording the moment the payment started being shared between readers, and the roster of a
/// 50,000-file workspace silently changed from `content-verified at 23:34:22` to `reused (... 5s ago)`
/// for exactly the same work — a different claim about the answer, and a worse one. The rule is now:
/// paid in **this** answer ⇒ `content-verified`; paid in an earlier one and still inside the window ⇒
/// `reused`, which names the moment and the window it relied on.
/// **本份答案自己付过费的核验不是复用**（审计 `T1` 第七刀）。付费一开始在几个读者之间共享，这两者就
/// 共用了一种措辞，而 50,000 文件工作区的普查静默地从 `content-verified at 23:34:22` 变成了
/// `reused (... 5s ago)`——同一份工作却是关于这份答案的另一种声称，而且是更差的那种。规则现在是：
/// 在**本份**答案里付费 ⇒ `content-verified`；在更早的答案里付费且仍在窗口内 ⇒ `reused`，并点名那个
/// 时刻与它所依赖的窗口。
fn reused_line(verified: &Verified) -> String {
    let age = verified.at.elapsed().as_secs();
    let paid_here = verified.answer == answer_epoch();
    match (verified.current, paid_here) {
        (true, true) => format!("freshness: content-verified at {}", verified.clock),
        (true, false) => format!(
            "freshness: reused (content-verified at {}, {age}s ago; window {REUSE_WINDOW_SECONDS}s)",
            verified.clock
        ),
        (false, true) => STALE.to_owned(),
        (false, false) => format!(
            "{STALE}; reused, content-verified at {}, {age}s ago; window {REUSE_WINDOW_SECONDS}s",
            verified.clock
        ),
    }
}

/// The current wall-clock time as `HH:MM:SS` in UTC.
/// 当前挂钟时间的 UTC `HH:MM:SS`。
///
/// UTC rather than local time because the bridge reads no time zone and a report whose
/// clock silently followed the environment would be a different token on two machines.
/// 用 UTC 而不是本地时间，因为本桥不读时区，而一个悄悄跟着环境走的时刻会让两台机器上的报告印出
/// 不同的凭据。
pub(crate) fn wall_clock() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let (hours, minutes, seconds) = ((seconds / 3600) % 24, (seconds / 60) % 60, seconds % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
#[path = "freshness_tests.rs"]
mod freshness_tests;
