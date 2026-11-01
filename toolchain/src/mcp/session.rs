//! What this server has already told this session, so a fixed block is not repeated.
//! 这个服务器在这个会话里已经说过的话，好让一个固定块不被重复。
//!
//! Audit `W2-2` / `W2-5` (D3): several answers carry a **constant** block — the census's boundary
//! prose, the write path's `consequences` disclaimer — that is worth reading once and is pure
//! repetition from the second call on. The measurement is the same one that produced the
//! `--list`-vs-reply debate: a fixed block repeated in every answer is paid for in every answer.
//! 审计 `W2-2` / `W2-5`（D3）：有几条答案带一个**常量**块——总账的边界散文、写入路径的 `consequences`
//! 免责声明——它值得读一次，而第二次调用起就是纯粹的重复。量法与那场 `--list`-对-回复之争同源：在每条答案
//! 里重复的常量块，就在每条答案里被付费。
//!
//! **What a session is** is the one design decision here, and it is stated rather than felt: the
//! unit is the **process that serves one client**. The stdio server lives for a connection, so
//! "already said" means "already said to this client"; a one-shot `--call` is a process of its own
//! and therefore always the first time, which is right — it has no earlier answer to refer back to.
//! **什么算一个会话**是这里唯一的设计决定，而它被写出来而不是靠感觉：单位是**服务一个客户端的那一个
//! 进程**。stdio 服务活一条连接，因此"已经说过"意味着"已经对这个客户端说过"；一次性的 `--call` 是它
//! 自己的进程，因此永远是第一次——这是对的，它没有更早的答案可回指。
//!
//! The key carries the **root**, so two trees in one session each get their own first time, and so a
//! test's scratch directory cannot be "already said" by another test's.
//! 键里带着**根**，因此同一个会话里的两棵树各自有自己的第一次，也因此一个测试的临时目录不可能被另一个
//! 测试"已经说过"。

use std::cell::RefCell;
use std::collections::BTreeSet;

thread_local! {
    /// The keys **this serving thread** has already printed in full.
    /// **这个服务线程**已经完整印过的那些键。
    ///
    /// Thread-local rather than process-global, and that is the same distinction the module
    /// documents: the stdio server reads and answers on the thread that called it, so "this
    /// session" and "this thread" are the same thing in production — while in a test binary, whose
    /// tests run **in parallel threads of one process**, a process-global ledger is shared state two
    /// tests fight over. That fight is not hypothetical: it made `the_bounds_are_said_once_…` fail
    /// only when the whole suite ran (audit `W2-2`).
    /// 用线程局部而不是进程全局，而这正是本模块记录的那个区分：stdio 服务在调用它的那个线程上读并作答，
    /// 因此生产环境里"这个会话"与"这个线程"是同一件事——而在一个**并行跑测试的进程**里，进程级账本正是两个
    /// 测试会互相抢的共享状态。那场争抢不是假设：它让 `the_bounds_are_said_once_…` 只在全量跑时失败
    /// （审计 `W2-2`）。
    static SAID: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// Whether this is the first time this session is being told `key`; records it when it is.
/// 这个会话是不是第一次被告知 `key`；是就记下它。
///
/// A borrow that cannot be taken answers `true`: repeating a paragraph is a smaller failure than
/// dropping one.
/// 借不到时回 `true`：重复一段话比漏掉一段话是小得多的失败。
pub(crate) fn first_time(key: &str) -> bool {
    SAID.with(|said| {
        said.try_borrow_mut()
            .map(|mut set| set.insert(key.to_owned()))
            .unwrap_or(true)
    })
}

/// Forget every key of **this thread**, so the next call answers as a first call.
/// 忘掉**本线程**的所有键，好让下一次调用按第一次作答。
///
/// Only tests use it: a test that calls one tool several times wants each call to be a first call,
/// and its thread-local ledger is exactly what to clear.
/// 只有测试用它：一个反复调用同一个工具的测试要每次都是"第一次"，而它自己的线程局部账本正是该清的东西。
#[cfg(test)]
pub(crate) fn forget() {
    SAID.with(|said| said.borrow_mut().clear());
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
