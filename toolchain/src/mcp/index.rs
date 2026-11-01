//! The published index: the build-time graph, its readiness record, and whether the two describe
//! the sources that are on disk right now (audit `M7`, P1.2 / P2).
//! 已发布的索引：构建期的图、它的就绪记录，以及两者是否描述着磁盘上此刻的源码（审计 `M7`，P1.2 / P2）。
//!
//! Two files, one question. `graph_edges.tsv` is the graph and `graph.generation` is the record that
//! says a run **finished** — the pipeline writes it last, so its presence means the payloads landed
//! and its absence means the previous run (or none) is what is on disk. A record that does not match
//! the graph beside it is refused rather than half-read, because "the index is ready" is exactly the
//! claim a truncated file must not be able to make.
//! 两个文件、一个问题。`graph_edges.tsv` 是图，`graph.generation` 是说**一次运行完成了**的记录——管线最后
//! 才写它，因此它在＝载荷落地了，它不在＝磁盘上是上一次运行（或者没有）。与旁边的图对不上的记录会被拒绝，
//! 而不是读一半，因为"索引已就绪"正是被截断的文件绝不能作出的那个声称。
//!
//! The staleness question here is answered **cheaply** (a walk of directory entries, the same stamp the
//! bridge's verification memo uses) and that is deliberate: it is a signal about an accelerator, not a
//! verdict about the records. The records keep their byte-exact rule — a reader that needs a verdict
//! asks `freshness`, and this module never overrides it.
//! 这里的过期问题用**廉价**方式回答（走一遍目录项，与桥的核验记忆用的是同一个戳），这是有意的：它是关于
//! 一个加速器的信号，不是关于记录的裁决。记录保持它们逐字节的规则——需要裁决的读者去问 `freshness`，
//! 本模块绝不覆盖它。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use crate::mcp::build_evidence::out_dir;

/// The header of a published graph, and the edges under it.
/// 已发布图的头部，以及它下面的边。
pub(crate) struct Graph {
    /// How many distinct nodes the edges name, as the header recorded it.
    /// 边点名了多少个不同的节点，按头部记录的值。
    pub(crate) nodes: usize,
    /// How many edges the body holds.
    /// 正文里有多少条边。
    pub(crate) edges: Vec<(String, String, String)>,
}

/// The readiness record a finished pipeline run publishes.
/// 一次完成的管线运行所发布的就绪记录。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Generation {
    /// Which run this was, counted from the previous record.
    /// 这是第几次运行，从上一份记录数起。
    pub(crate) generation: u64,
    /// The digest of the graph this run published.
    /// 这次运行发布的图的摘要。
    pub(crate) digest: String,
    /// How many faces the run's record holds.
    /// 这次运行的记录里有多少个面。
    pub(crate) faces: usize,
    /// How many `.rs` files the run's sources had.
    /// 这次运行的源码有多少个 `.rs` 文件。
    pub(crate) files: usize,
    /// How many nodes the graph names.
    /// 图点名了多少个节点。
    pub(crate) graph_nodes: usize,
    /// How many edges the graph holds.
    /// 图里有多少条边。
    pub(crate) graph_edges: usize,
    /// The stamp of the sources at that moment, as `(file count, newest mtime)`.
    /// 那一刻源码的戳，形如 `(文件数, 最新 mtime)`。
    pub(crate) stamp: (usize, u64),
    /// The moment the run finished, as `HH:MM:SS` (UTC).
    /// 那次运行结束的时刻，`HH:MM:SS`（UTC）。
    pub(crate) finished_at: String,
    /// The namespace these records were published under, when the record carries it.
    /// 这些记录发布时所用的命名空间——记录携带它时才有。
    ///
    /// `None` means the record predates the stamp (or an older binary wrote it), and a reader must
    /// then say nothing about namespaces rather than guess: identity is
    /// `hash(namespace, source path, name)`, so a record read under another namespace holds a
    /// different id for the same face, and "every face moved" is what that looks like when nobody
    /// knows to check this field (audit 2026-10-06, §M7.18).
    /// `None` 意为记录早于这枚戳（或更早的二进制写的），此时读者对命名空间**什么都不要说**，而不是猜：
    /// 身份是 `hash(命名空间, 源码路径, 名字)`，因此在另一个命名空间下读同一份记录，同一个面持有不同的 id，
    /// 而"每个面都搬了家"正是没人知道该查这个字段时它看起来的样子（2026-10-06 审计，§M7.18）。
    pub(crate) namespace: Option<String>,
}

/// What a reader finds when it asks whether the index describes these sources.
/// 读者问"索引是否描述着这批源码"时找到的东西。
pub(crate) enum State {
    /// The index was published for these sources, and the graph matches its record.
    /// 索引就是为这批源码发布的，且图与它的记录相符。
    Ready(Generation),
    /// The index describes other sources than the ones on disk now.
    /// 索引描述的不是磁盘上此刻这批源码。
    Behind {
        /// What is published.
        /// 已发布的东西。
        published: Generation,
    },
    /// Nothing was published for this root.
    /// 这个根上什么都没有发布过。
    Absent,
    /// A record is there and does not hold together.
    /// 有一份记录，但它不成立。
    Damaged(String),
}

/// What starting a refresh did.
/// 启动一次刷新的结果。
pub(crate) enum Started {
    /// A refresh is now running in the background.
    /// 一次刷新正在后台运行。
    Started,
    /// One was already running for this root, so this call joined it.
    /// 这个根上已经有一次在跑，于是这次调用加入了它。
    AlreadyRunning,
}

/// The refreshes this process has running, and when each started.
/// 本进程正在运行的刷新，以及各自开始的时刻。
static RUNNING: Mutex<BTreeMap<PathBuf, Instant>> = Mutex::new(BTreeMap::new());

/// Read the graph this root published, refusing a body that does not match its header.
/// 读取这个根发布的图，并拒绝与头部不符的正文。
pub(crate) fn read_graph(root: &Path) -> Result<Graph, String> {
    let path = out_dir(root).join(xirang_kernel::lexicon::GRAPH_FILE);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let body_start = text
        .find("# from\tto\tkind\n")
        .ok_or_else(|| format!("{} has no column header", path.display()))?
        + "# from\tto\tkind\n".len();
    let body = &text[body_start..];
    let listed_nodes: usize = header_value(&text, "# nodes\t")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("{} has no node count", path.display()))?;
    let listed_edges: usize = header_value(&text, "# edges\t")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("{} has no edge count", path.display()))?;
    let digest = header_value(&text, "# digest\t")
        .ok_or_else(|| format!("{} has no digest", path.display()))?;
    let mut edges = Vec::new();
    for line in body.lines().filter(|line| !line.is_empty()) {
        let mut fields = line.split('\t');
        match (fields.next(), fields.next(), fields.next()) {
            (Some(from), Some(to), Some(kind)) => {
                edges.push((from.to_owned(), to.to_owned(), kind.to_owned()));
            }
            _ => {
                return Err(format!(
                    "{} has a row that is not `from<TAB>to<TAB>kind`: `{line}`",
                    path.display()
                ));
            }
        }
    }
    let nodes: std::collections::BTreeSet<&str> = edges
        .iter()
        .flat_map(|(from, to, _)| [from.as_str(), to.as_str()])
        .collect();
    let recomputed = xirang_kernel::identity::NodeId::from_bytes(body.as_bytes()).to_string();
    if edges.len() != listed_edges || nodes.len() != listed_nodes || recomputed != digest {
        return Err(format!(
            "{} does not match its header (header: {listed_nodes} node(s) / {listed_edges} \
             edge(s) / {digest}; body: {} node(s) / {} edge(s) / {recomputed})",
            path.display(),
            nodes.len(),
            edges.len()
        ));
    }
    Ok(Graph {
        nodes: listed_nodes,
        edges,
    })
}

/// Read the readiness record, refusing one that is incomplete or that does not match the graph.
/// 读取就绪记录，并拒绝不完整、或与图对不上的那一份。
pub(crate) fn read_generation(root: &Path) -> Result<Generation, String> {
    let path = out_dir(root).join(xirang_kernel::lexicon::GENERATION_FILE);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let marker = header_value(&text, "# generation\t");
    if marker.as_deref() != Some(xirang_kernel::lexicon::GENERATION_MARKER) {
        return Err(format!(
            "{} is not a readiness record (first line says {:?})",
            path.display(),
            marker
        ));
    }
    let number = |key: &str| -> Result<u64, String> {
        header_value(&text, key)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("{} has no readable `{key}`", path.display()))
    };
    let size = |key: &str| -> Result<usize, String> {
        header_value(&text, key)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("{} has no readable `{key}`", path.display()))
    };
    let stamp = header_value(&text, "stamp\t")
        .and_then(|value| {
            let (files, newest) = value.split_once(':')?;
            Some((files.parse().ok()?, newest.parse().ok()?))
        })
        .ok_or_else(|| format!("{} has no readable `stamp`", path.display()))?;
    let generation = Generation {
        namespace: header_value(&text, xirang_kernel::lexicon::GENERATION_NAMESPACE_KEY),
        generation: number("generation\t")?,
        digest: header_value(&text, "digest\t")
            .ok_or_else(|| format!("{} has no digest", path.display()))?,
        faces: size("faces\t")?,
        files: size("files\t")?,
        graph_nodes: size("graph_nodes\t")?,
        graph_edges: size("graph_edges\t")?,
        stamp,
        finished_at: header_value(&text, "finished_at\t").unwrap_or_default(),
    };
    // A record that names a graph the graph file does not hold is exactly the half-written state
    // this file exists to make impossible; refusing it is what keeps "ready" checkable.
    // 一份点名了图文件里并不存在的图的记录，正是这个文件存在的意义所在的那种半写状态；拒绝它，才让"就绪"
    // 保持可检查。
    let graph = read_graph(root)?;
    if graph.nodes != generation.graph_nodes || graph.edges.len() != generation.graph_edges {
        return Err(format!(
            "{} names a graph of {} node(s) and {} edge(s), while {} holds {} node(s) and {} \
             edge(s)",
            path.display(),
            generation.graph_nodes,
            generation.graph_edges,
            xirang_kernel::lexicon::GRAPH_FILE,
            graph.nodes,
            graph.edges.len()
        ));
    }
    Ok(generation)
}

/// Whether the index describes the sources on disk right now (audit `M7`, P1.2 / P2.3).
/// 索引是否描述着磁盘上此刻的源码（审计 `M7`，P1.2 / P2.3）。
pub(crate) fn state(root: &Path) -> State {
    let published = match read_generation(root) {
        Ok(generation) => generation,
        Err(error) => {
            return if out_dir(root)
                .join(xirang_kernel::lexicon::GENERATION_FILE)
                .exists()
            {
                State::Damaged(error)
            } else {
                State::Absent
            };
        }
    };
    let current = crate::build_method::source_stamp(root);
    if published.stamp == current {
        State::Ready(published)
    } else {
        State::Behind { published }
    }
}

/// Start a refresh in the background, or join the one already running (audit `M7`, P1.2).
/// 在后台启动一次刷新，或加入已经在跑的那一次（审计 `M7`，P1.2）。
///
/// This is the write path's half of the maintainer's "writing a file is a scheduling problem": the
/// file lands, the index starts building, and nobody waits for it. It drives the **same** entry the
/// CLI's `check` and the bridge's `verify` drive, so a refresh cannot publish a tree those two would
/// refuse. One refresh per root at a time, because two runs publishing the same directory would race
/// each other's payloads.
/// 这是写入路径那一半的"写一个文件是个调度问题"：文件落地、索引开始构建，而没有任何人等它。它驱动的是
/// CLI 的 `check` 与桥的 `verify` 所驱动的**同一个**入口，因此一次刷新不可能发布一棵那两者会拒绝的树。
/// 同一个根一次只跑一次刷新，因为两次运行发布同一个目录会互相抢载荷。
pub(crate) fn start(root: &Path) -> Result<Started, String> {
    let package = crate::mcp::registry::namespace(root)?;
    let key = root.to_path_buf();
    let out = out_dir(root);
    {
        let mut running = RUNNING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if running.contains_key(&key) {
            return Ok(Started::AlreadyRunning);
        }
        running.insert(key.clone(), Instant::now());
    }
    std::thread::spawn(move || {
        // The verdict is not thrown away — it is *published*: a run that fails removes the
        // fingerprint and writes no readiness record, which is how the next reader learns that the
        // refresh did not land.
        // 判断结果没有被丢掉——它被**发布**了：一次失败的运行会移除指纹、不写就绪记录，下一个读者就是这样
        // 得知这次刷新没有落地。
        let _ = crate::build_method::check_for(&key, &out, &package);
        RUNNING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&key);
    });
    Ok(Started::Started)
}

/// What a write path says after it landed a file: start the index, then report it (audit `M7`, P1.2).
/// 写入路径在文件落地之后要说的话：启动索引，然后报告它（审计 `M7`，P1.2）。
///
/// The whole story in one place because three readers must tell it the same way: the bridge's write
/// reply, the CLI's line after `check`, and the graph tool. A preview publishes nothing, so its caller
/// does not reach here at all.
/// 整段说法住在一处，因为三个读者必须用同一种方式讲它：桥的写入回复、CLI 在 `check` 之后那一行、以及图
/// 工具。预览不发布任何东西，因此它的调用方根本走不到这里。
pub(crate) fn after_write(root: &Path) -> String {
    let started = start(root);
    let state = self::state(root);
    let mut line = line(root, &state);
    if let Err(error) = started {
        // The line above already says the way forward; this says why this surface did not take it.
        // 上面那行已经说出了出路；这一句说出这个执行面为什么没有走那条路。
        line.push_str(&format!(" (this surface did not start one: {error})"));
    }
    line
}

/// Wait until no refresh is running for this root, or the deadline passes (audit `M7`, P2.2).
/// 等到这个根上没有刷新在跑，或者等到超时（审计 `M7`，P2.2）。
///
/// Test-only for now: no production caller waits — a reader that would rather not wait reads
/// [`state`] and is told the index is behind, and the CLI's `check` runs its pass synchronously. It
/// returns to the public surface when a caller needs the guarantee, rather than carrying an unused
/// `pub(crate)` function until then.
/// 目前只给测试用：生产上没有任何调用方在等——不愿等的读者读 [`state`]，会被告诉索引落后，而 CLI 的
/// `check` 是同步跑完那一趟的。等真有调用方需要这个保证时它再回到公开面，而不是在此之前带着一个没人用的
/// `pub(crate)`。
///
/// The signal the maintainer asked for, in its waiting form: "see the terminal say it is ready before
/// you read from it". A caller that wants that guarantee instead of the line polls this; a refresh
/// that has nothing left to do finishes in milliseconds, and a caller that would rather not wait reads
/// [`state`] and gets told the index is behind.
/// 维护者要的那个信号，以"等待"的形式："看到终端说索引就绪，再从它读"。想要这个保证而不是那一行字的调用方
/// 轮询这里；没有活要干的刷新在毫秒内结束，而不愿等的调用方读 [`state`]，会被告诉索引落后。
#[cfg(test)]
pub(crate) fn wait_until_idle(root: &Path, timeout: std::time::Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while running_for(root).is_some() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    true
}

/// Wait until every refresh this process started has finished, or the deadline passes (audit `M7`,
/// P1.2).
/// 等到本进程启动的每一次刷新都结束，或者等到超时（审计 `M7`，P1.2）。
///
/// The one-shot client needs this: it exits the moment its reply is written, and a process exit kills
/// the background refresh it just started. The wait is **bounded** and giving up is safe — the
/// pipeline writes each payload atomically and the readiness record last, so a run that stopped
/// halfway is *reported* (`index damaged` / the previous generation) rather than trusted.
/// 一次性客户端需要它：它在回复写出的那一刻就退出，而进程退出会杀掉它刚刚启动的后台刷新。这个等待是
/// **有界**的，而放弃是安全的——管线逐个原子地写载荷、最后写就绪记录，因此半途停下的一次运行会被**报告
/// 出来**（`index damaged` / 上一代），而不是被相信。
pub(crate) fn settle(timeout: std::time::Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let busy = {
            let running = RUNNING
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if running.is_empty() {
                return;
            }
            running
                .values()
                .map(|started| started.elapsed())
                .min()
                .unwrap_or_default()
        };
        // The log line is the terminal's version of the signal: a person running `--call apply` sees
        // that the index is still being built rather than a process that simply hung.
        // 这条日志就是那个信号的终端形态：跑 `--call apply` 的人看到索引还在构建，而不是一个干脆卡住的进程。
        if Instant::now() >= deadline {
            eprintln!(
                "xirang: the index refresh has been running for {}s; leaving it unpublished \
                 (run `xirang check` to finish it)",
                busy.as_secs()
            );
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// How long a refresh has been running for this root, if one is.
/// 这个根上的刷新已经跑了多久（如果有）。
pub(crate) fn running_for(root: &Path) -> Option<u64> {
    RUNNING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(root)
        .map(|started| started.elapsed().as_secs())
}

/// The line a report carries about the index (audit `M7`, P2.2 / P2.3).
/// 一份报告携带的关于索引的那一行（审计 `M7`，P2.2 / P2.3）。
///
/// One spelling, read by the write path's reply, by the graph tool and by the CLI, so a reader cannot
/// be told two different things about the same directory.
/// 只有一种拼法，由写入路径的回复、图工具与 CLI 共同读取，因此读者不会对同一个目录听到两种说法。
pub(crate) fn line(root: &Path, state: &State) -> String {
    match state {
        State::Ready(generation) => format!(
            "graph updated: generation {}, {} file(s), {} face(s), {}",
            generation.generation, generation.files, generation.faces, generation.digest
        ),
        State::Behind { published } => {
            let running = match running_for(root) {
                Some(seconds) => format!("a refresh is running ({seconds}s)"),
                None => "run `xirang check`".to_owned(),
            };
            format!(
                "index behind: generation {} covers {}; the sources have changed since — {running}",
                published.generation, published.digest
            )
        }
        State::Absent => "index not published yet: run `xirang check`".to_owned(),
        State::Damaged(why) => format!("index damaged: {why}"),
    }
}

/// The value of one `key\tvalue` line, without its key.
/// 一条 `key\tvalue` 行的取值，不含键。
fn header_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix(key))
        .map(|value| value.trim().to_owned())
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod index_tests;
