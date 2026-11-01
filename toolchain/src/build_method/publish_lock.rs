//! The publish lock: **one writer per tree** (audit `M7`, P3.4-lock).
//! 发布锁：**一棵树一个写者**（审计 `M7`，P3.4-lock）。
//!
//! Two XiRang runs can look at one tree at the same time — the CLI and the MCP bridge, two agent
//! sessions, or a host's build script and an editor's `check`. Each publish writes a *set* of files
//! (`pruning_manifest.tsv`, the graph, the manifests, `graph.generation`), and each file is atomic
//! on its own (`write_if_changed` writes a temp file and renames), but the set is not. Without a
//! lock the later writer wins file by file, so a reader can find a new fingerprint beside an old
//! manifest, and a run can silently overwrite a generation another run just published.
//! 两次 XiRang 运行可能同时看着一棵树——CLI 与 MCP 桥、两个 agent 会话、或者宿主的构建脚本与编辑器的
//! `check`。每次发布写的是一**组**文件（`pruning_manifest.tsv`、图、各清单、`graph.generation`），每份
//! 文件各自原子（`write_if_changed` 写临时文件再改名），但这一组不是。没有锁时后写者逐份文件取胜，于是
//! 读者可能读到"新指纹 + 旧清单"，一次运行也可能静默覆盖另一次刚发布的代。
//!
//! What the lock is, precisely: a **file created with `create_new`** (atomic on every filesystem this
//! workspace supports), holding the holder's pid and start time, removed when the publishing run
//! ends. A holder that dies without removing it leaves a **stale** lock, and the next writer takes it
//! over rather than waiting forever. What it is *not*: a general-purpose cross-process mutex, and not
//! a reader's guarantee — a reader that wants a consistent **set** reads the readiness record
//! (`graph.generation`) last, which the pipeline already writes last for exactly this reason.
//! 锁究竟是什么：用 **`create_new` 创建的一个文件**（在本工作区支持的所有文件系统上都是原子的），里面
//! 写着持有者的 pid 与开始时间，发布的那次运行结束时删除。持有者在没删除的情况下死掉会留下一把**陈旧**
//! 锁，下一个写者接管它而不是永远等下去。它**不是**：通用跨进程互斥，也不是给读者的保证——想要一致**一组**
//! 的读者最后读就绪记录（`graph.generation`），而管线早已正因此把它写在最后。
//!
//! The wait is bounded (`XIRANG_LOCK_WAIT_MS`, default [`DEFAULT_WAIT_MS`]) because a refusal a
//! caller can read and act on beats a hang: the refusal names the lock file, the holder and the two
//! ways forward.
//! 等待是有界的（`XIRANG_LOCK_WAIT_MS`，默认 [`DEFAULT_WAIT_MS`]），因为一句调用方读得懂、能照做的拒绝
//! 胜过挂起：拒绝里点名锁文件、持有者，以及两条出路。

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use xirang_kernel::lexicon;

/// How long a publish waits for the lock by default.
/// 发布默认等这把锁多久。
///
/// The write phase is milliseconds at any scale this workspace measured (the work is discovering,
/// parsing and rendering; the writes are already in memory), so a wait this long only ever pays off
/// against another process's *whole* publish, not against a slow disk.
/// 在本工作区量过的任何规模上，写入阶段都是毫秒级（活是发现、解析与渲染；写入内容已在内存里），因此等
/// 这么久只可能用在"另一个进程的**整次**发布"上，而不是慢磁盘上。
pub(crate) const DEFAULT_WAIT_MS: u64 = 30_000;

/// How long a lock whose holder cannot be identified is honoured before it counts as stale.
/// 无法辨认持有者的锁被尊重多久之后算作陈旧。
///
/// A lock file this module did not write (an older version, a truncated write, a user's note) has no
/// pid to check, so age is the only evidence left. Ten minutes is far longer than any publish this
/// workspace measured and far shorter than "forever" — the alternative, honouring it indefinitely,
/// makes one stray file wedge every build of that tree.
/// 不是本模块写的锁文件（更早的版本、被截断的写入、用户留的字条）没有 pid 可查，剩下的唯一证据就是年龄。
/// 十分钟远长于本工作区量过的任何发布、又远短于"永远"——另一种做法（无限期尊重它）会让一个杂散文件卡死
/// 那棵树的每一次构建。
pub(crate) const STALE_AGE: Duration = Duration::from_secs(600);

/// How often a waiting writer looks again.
/// 等待中的写者多久看一次。
const POLL: Duration = Duration::from_millis(25);

/// The held lock; dropping it releases the tree.
/// 已持有的锁；丢弃即释放这棵树。
#[derive(Debug)]
pub(crate) struct PublishLock {
    path: PathBuf,
    /// Whether this acquisition took over a lock whose holder was gone.
    /// 本次获取是否接管了一把持有者已消失的锁。
    pub(crate) stolen_from: Option<u32>,
}

impl Drop for PublishLock {
    fn drop(&mut self) {
        // Only the file this guard created: a lock another writer stole from us is theirs now, and
        // removing it would hand the tree to a third writer while they publish.
        // 只删本守卫创建的那份文件：被别的写者夺走的锁归他们，删掉它就等于在他们发布时把树交给第三个写者。
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Where the lock for one output directory lives.
/// 某个输出目录的锁住在哪里。
pub(crate) fn path_for(out_dir: &Path) -> PathBuf {
    match out_dir.parent() {
        Some(parent) => parent.join(lexicon::PUBLISH_LOCK_FILE),
        None => out_dir.join(lexicon::PUBLISH_LOCK_FILE),
    }
}

/// Take the tree's publish lock, waiting up to the configured budget.
/// 取得这棵树的发布锁，最多等待配置的预算。
pub(crate) fn acquire(out_dir: &Path) -> Result<PublishLock, String> {
    acquire_within(out_dir, wait_budget())
}

/// The configured wait budget, in milliseconds.
/// 配置的等待预算，毫秒。
pub(crate) fn wait_budget() -> Duration {
    let millis = std::env::var(lexicon::LOCK_WAIT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_WAIT_MS);
    Duration::from_millis(millis)
}

/// Take the tree's publish lock, waiting up to `budget` (tests pass a small one).
/// 取得这棵树的发布锁，最多等待 `budget`（测试传一个小的）。
pub(crate) fn acquire_within(out_dir: &Path, budget: Duration) -> Result<PublishLock, String> {
    let path = path_for(out_dir);
    let deadline = Instant::now() + budget;
    let mut waited = Duration::ZERO;
    loop {
        match create(&path) {
            Ok(()) => {
                return Ok(PublishLock {
                    path,
                    stolen_from: None,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "cannot take the publish lock {}: {error}",
                    path.display()
                ));
            }
        }
        // Somebody holds it. A holder that is gone is a leftover, not a writer: take it over. The
        // order matters — age alone would steal a lock from a *live*, slow writer, and liveness alone
        // would steal one whose pid happens to be recycled.
        // 有人持有它。已经消失的持有者是残留而不是写者：接管它。顺序有讲究——只看年龄会从**活着的**慢写者
        // 手里夺锁，而只看存活会夺走一把 pid 恰好被复用的锁。
        if let Some(holder) = holder_of(&path)
            && holder_is_gone(&holder)
        {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    // The removal is what makes us the holder: whoever created the file is gone, so
                    // the next `create_new` either succeeds for us or loses to a writer that got
                    // there first — both are correct, and neither can publish twice.
                    // 删除这个动作才让我们成为持有者：创建它的进程已经消失，因此下一次 `create_new` 要么
                    // 我们成功、要么输给先到的写者——两者都正确，谁都不可能发布两次。
                    if create(&path).is_ok() {
                        return Ok(PublishLock {
                            path,
                            stolen_from: Some(holder.pid),
                        });
                    }
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!(
                        "cannot take over the stale publish lock {}: {error}",
                        path.display()
                    ));
                }
            }
        }
        // A lock that names no holder is honoured while it is young ([`STALE_AGE`]): with no pid to
        // check, age is the only evidence, and stealing a fresh one would take the tree from a run
        // that is merely older than this code.
        // 没有点出持有者的锁在它年轻（[`STALE_AGE`] 之内）时被尊重：没有 pid 可查，年龄就是唯一证据，而
        // 夺走一把新鲜的锁等于从只是比本代码更早的一次运行手里拿走这棵树。
        if Instant::now() >= deadline {
            return Err(refusal(&path, waited));
        }
        std::thread::sleep(POLL);
        waited += POLL;
    }
}

/// Create the lock file and write the holder's identity into it.
/// 创建锁文件并把持有者的身份写进去。
fn create(path: &Path) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    // The holder line is deliberately the same `key\tvalue` shape every other record in this
    // workspace uses, so it is read with the same eyes. A failed write is not fatal for the same
    // reason the header is best-effort in the other records: the file's *existence* is the lock.
    // 持有者那行有意与本工作区其它记录同形（`key\tvalue`），因此读它用的是同一双眼睛。写失败不致命，理由
    // 与其它记录里表头是尽力而为的一样：**文件存在**就是锁。
    let _ = writeln!(file, "pid\t{}", std::process::id());
    let _ = writeln!(file, "started\t{started}");
    let _ = file.flush();
    Ok(())
}

/// Whether a lock's holder is gone: a dead pid, or no pid at all and old enough that no live run
/// could still be writing it.
/// 一把锁的持有者是否已经消失：pid 已死，或者根本没有 pid 且已老到不可能还有活的运行在写它。
fn holder_is_gone(holder: &Holder) -> bool {
    match holder.identifiable {
        true => !holder.alive,
        false => holder.age >= STALE_AGE,
    }
}

/// What the lock file says about its holder.
/// 锁文件关于持有者说了什么。
struct Holder {
    pid: u32,
    /// Whether a live process with that pid exists.
    /// 是否存在一个持有该 pid 的活进程。
    alive: bool,
    /// Whether the file named a holder at all.
    /// 文件到底有没有点出持有者。
    identifiable: bool,
    /// How long the file has existed, from its modification time.
    /// 这份文件已经存在多久，取其修改时间。
    age: Duration,
}

/// Read the holder out of a lock file; `None` when it does not exist.
/// 从锁文件里读出持有者；不存在时 `None`。
fn holder_of(path: &Path) -> Option<Holder> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut pid = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("pid\t") {
            pid = value.trim().parse::<u32>().ok();
        }
    }
    let age = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .unwrap_or(Duration::ZERO);
    Some(Holder {
        pid: pid.unwrap_or(0),
        alive: pid.is_some_and(process_is_alive),
        identifiable: pid.is_some(),
        age,
    })
}

/// Whether a process with this pid is running, as far as this platform can tell.
/// 就本平台所能判断的而言，持有该 pid 的进程是否在运行。
///
/// Linux answers through `/proc`; where that does not exist the answer is "assume alive", because the
/// alternative — assuming dead — would steal a lock out from under a live writer, and a stale lock
/// merely delays the next publish until [`STALE_AGE`].
/// Linux 经 `/proc` 回答；那里不存在时答案是"假定活着"，因为另一种做法（假定已死）会从活写者手里夺锁，
/// 而一把陈旧锁只是把下一次发布推迟到 [`STALE_AGE`]。
fn process_is_alive(pid: u32) -> bool {
    let proc = Path::new("/proc");
    if !proc.is_dir() {
        return true;
    }
    proc.join(pid.to_string()).is_dir()
}

/// The refusal a writer that waited out its budget earns.
/// 一个等完预算的写者所换来的拒绝。
fn refusal(path: &Path, waited: Duration) -> String {
    let holder = holder_of(path);
    let who = match &holder {
        Some(holder) if holder.identifiable => format!(
            "pid {} has held it for {} s",
            holder.pid,
            holder.age.as_secs()
        ),
        Some(holder) => format!(
            "its holder is unreadable and it is {} s old",
            holder.age.as_secs()
        ),
        None => "it disappeared while this run waited".to_owned(),
    };
    format!(
        "another XiRang run is publishing this tree: {} is held and {who}; this run waited {} ms and \
         published nothing rather than write half a generation.\n\
         way forward: wait for that run to finish (the file is removed when it does), or remove {} if \
         that process is gone — the wait is bounded by {} (milliseconds)",
        path.display(),
        waited.as_millis(),
        path.display(),
        lexicon::LOCK_WAIT_ENV
    )
}

#[cfg(test)]
#[path = "publish_lock_tests.rs"]
mod publish_lock_tests;
