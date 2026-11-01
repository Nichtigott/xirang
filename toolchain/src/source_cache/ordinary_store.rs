//! Disk-backed reuse of the ordinary-source proof, owned by the execution surfaces.
//! 磁盘持有的普通源码证明复用，由执行面持有。
//!
//! `ordinary` proves that a source is valid Rust carrying no declaration, and keeps that proof in
//! memory. A **declaration-less file is therefore re-parsed by every new process**: measured on a tree
//! of 1,000 faces plus 1,000 ordinary neighbours, a fresh process paid **10 250 ms** against 744 ms for
//! the warm one (`W7`/`A33`, report §7i.3) — while the faces themselves were served from the `units/`
//! cache. This module gives that proof the same treatment the units get: one small file per proof,
//! named by the proof's own **content fingerprint**, under `<target>/xirang/cache/ordinary/`.
//! `ordinary` 证明这些字节是合法 Rust、且不含声明，并把证明留在内存里。于是**没有声明的文件会被每个新进程重新
//! 解析**：在一棵 1 000 个面加 1 000 个普通邻居的树上实测，新进程付 **10 250 ms**，而热进程是 744 ms
//! （`W7`/`A33`，报告 §7i.3）——与此同时那些面本身是由 `units/` 缓存供上的。本模块给这份证明与单元同样的待遇：
//! 一条证明一个小文件，用它自己的**内容指纹**命名，落在 `<target>/xirang/cache/ordinary/` 下。
//!
//! What is stored is a **proof, not an answer**: it says "these exact bytes are valid Rust with no
//! declaration", and the caller still reads the current file every time. Freshness is by construction —
//! the key carries the content and the producing toolchain — so no timestamp ever decides a hit, which
//! is the same rule the discovery cache follows.
//! 存的是**证明而不是答案**：它说的是"这串字节是合法 Rust、且没有声明"，而调用方每一轮仍然读当前文件。
//! 新鲜性由构造保证——键里带着内容与产生它的工具链——因此没有任何时间戳决定命中，这与发现缓存遵循同一条规则。

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The record shape this module writes and accepts.
/// 本模块写出与接受的记录形状。
///
/// The key already carries the producing toolchain, so this constant only has to separate record
/// *shapes*; a future field layout bumps it and every old record stops being a proof.
/// 键里已经带着产生它的工具链，因此这个常量只需区分记录的**形状**；将来字段布局变了就抬它，旧记录随即不再是证明。
pub(crate) const SCHEMA: &str = "xirang-ordinary-1";

/// The first line of every record.
/// 每条记录的第一行。
fn schema_line() -> String {
    format!("# schema\t{SCHEMA}\n")
}

/// How many proofs one pass will hold before it stops remembering more.
/// 一趟最多记住多少条证明。
///
/// The cap exists so a pathological tree cannot grow the session without bound; hitting it costs
/// reuse, never an answer — the caller re-parses what was dropped.
/// 上限存在的意义是让畸形的树无法把会话撑到无界；触及它只会损失复用、绝不会影响答案——被丢掉的调用方会重新解析。
const SESSION_LIMIT: usize = 131_072;

/// Sources at or above this size are worth a record; below it, re-validating is cheaper than storing.
/// 达到这个大小的源码才值得留一条记录；更小的重新校验比存下来更便宜。
///
/// The same rule `snapshot::used` applies to tiny call-free leaves, and the same reason: the record is
/// a file, and a two-line file costs more to find than to parse. Measured (`W7`/`A33`): the ordinary
/// neighbours that motivated this module are ~4 KB each, i.e. four times the threshold.
/// 与 `snapshot::used` 对"小型无调用叶子"适用的规则、理由相同：一条记录就是一个文件，而两行的文件找起来比解析
/// 更贵。实测（`W7`/`A33`）：促成这个模块的那些普通邻居每个约 4 KB，是阈值的四倍。
pub(crate) const MIN_BYTES: usize = 1024;

/// One pass's proofs, and where they will be written when it ends.
/// 一趟的证明，以及它们在该趟结束时写到哪里。
struct Session {
    directory: PathBuf,
    fresh: HashSet<String>,
    dirty: bool,
}

thread_local! {
    /// A stack, not a slot: passes call passes here, and the guard restores what it found rather than
    /// clearing it (the same shape as `ParentLookupScope` and `run_as_package`).
    /// 是**栈**而不是一个格子：这里一趟会调用另一趟，而守卫把它找到的东西放回去、而不是清空（与
    /// `ParentLookupScope`、`run_as_package` 同形）。
    static SESSIONS: RefCell<Vec<Session>> = const { RefCell::new(Vec::new()) };
}

/// Opens the proof store for as long as it lives, writing what it learned when it ends.
/// 在它存续期间打开证明存储，并在结束时写下它学到的东西。
pub(crate) struct Scope;

impl Scope {
    /// Open a store writing records under `directory`.
    /// 打开一个把记录写到 `directory` 下的存储。
    pub(crate) fn open(directory: &Path) -> Self {
        SESSIONS.with(|sessions| {
            sessions.borrow_mut().push(Session {
                directory: directory.to_path_buf(),
                fresh: HashSet::new(),
                dirty: false,
            })
        });
        Scope
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let Some(session) = SESSIONS.with(|sessions| sessions.borrow_mut().pop()) else {
            return;
        };
        if !session.dirty || std::fs::create_dir_all(&session.directory).is_err() {
            return;
        }
        for fingerprint in session.fresh {
            let path = session.directory.join(&fingerprint);
            if path.is_file() {
                continue;
            }
            let record = schema_line();
            // Write-then-rename, with a process-unique temporary: a reader either sees no file or sees
            // a complete one, and two processes writing the same proof cannot interleave inside it.
            // 先写后改名，临时名带进程号：读取方要么看不到文件、要么看到完整的一份，而两个进程写同一条证明
            // 也不会在文件里交错。
            let temporary = session
                .directory
                .join(format!(".{fingerprint}.{}.tmp", std::process::id()));
            if std::fs::write(&temporary, record).is_ok() {
                let _ = std::fs::rename(&temporary, &path);
            }
            let _ = std::fs::remove_file(&temporary);
        }
    }
}

/// The proof's name: its content and the toolchain that can judge it, under one prefix per family.
/// 证明的名字：它的内容、以及能判定它的工具链，前面加一族一个前缀。
///
/// Deliberately the same three inputs `identity_cache::source_unit_fingerprint` uses (family tag,
/// `CARGO_PKG_VERSION`, bytes) minus the path — a proof is about bytes, so the same file moved to
/// another directory keeps its record.
/// 有意与 `identity_cache::source_unit_fingerprint` 用同样三个输入（族标签、`CARGO_PKG_VERSION`、字节），
/// 只去掉路径——证明是关于字节的，因此同一个文件搬到别的目录后仍然保留它的记录。
fn fingerprint(source: &str) -> String {
    let mut input = Vec::new();
    input.extend_from_slice(b"xirang-ordinary\0");
    input.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
    input.push(0);
    input.extend_from_slice(source.as_bytes());
    xirang_kernel::identity::NodeId::from_bytes(&input).to_string()
}

/// Whether an open store already holds the proof for these bytes.
/// 打开的存储是否已经持有这串字节的证明。
///
/// A record that does not carry this module's schema is not a proof, and neither is a directory entry
/// that disappeared between the two calls: both answer `false`, i.e. "parse it", which is always
/// correct.
/// 不带本模块 schema 的记录不是证明，两次调用之间消失的目录项也不是：两者都答 `false`，也就是"去解析"，
/// 而那是永远正确的。
pub(crate) fn proved(source: &str) -> bool {
    if source.len() < MIN_BYTES {
        return false;
    }
    let fingerprint = fingerprint(source);
    SESSIONS.with(|sessions| {
        let sessions = sessions.borrow();
        let Some(session) = sessions.last() else {
            return false;
        };
        if session.fresh.contains(&fingerprint) {
            return true;
        }
        std::fs::read_to_string(session.directory.join(&fingerprint))
            .is_ok_and(|record| record == schema_line())
    })
}

/// Remember that these bytes proved ordinary, so the pass that proved it can write the record.
/// 记住这串字节已证明是普通的，好让证明它的那一趟把记录写下来。
///
/// Called on the parse path only; with no store open this is a no-op, which is what keeps every caller
/// outside a pass behaving exactly as it did.
/// 只在解析路径上调用；没有打开存储时它是空操作，这正是"趟之外的调用方行为与从前完全一致"的由来。
pub(crate) fn remember(source: &str) {
    if source.len() < MIN_BYTES {
        return;
    }
    let fingerprint = fingerprint(source);
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.borrow_mut();
        let Some(session) = sessions.last_mut() else {
            return;
        };
        if session.fresh.len() < SESSION_LIMIT && session.fresh.insert(fingerprint) {
            session.dirty = true;
        }
    });
}

#[cfg(test)]
#[path = "ordinary_store_tests.rs"]
mod ordinary_store_tests;
