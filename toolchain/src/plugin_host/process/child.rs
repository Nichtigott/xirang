//! Child-process plumbing for the process adapter: spawn, drain, frame, reap.
//! 进程适配器的子进程底层机制：派生、排空、分帧、回收。
//!
//! This is the half of `process` that talks to the operating system. It is
//! separate so the adapter's contract — limits, the program, the operation
//! framing the host promises — stays readable in one page, and so the pipe and
//! spawn hazards below get room for the explanation they need.
//! 这是 `process` 中与操作系统对话的那一半。它独立出来，是为了让适配器的契约——限制、
//! 程序、宿主承诺的操作分帧——保持在一页内可读，也让下面的管道与派生陷阱有地方把话说明白。

use std::{
    io::{self, Read},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use crate::plugin_host::HostError;

use super::{ProcessLimits, ProcessProgram};

/// How often the child's exit status is polled while it runs.
/// 子进程运行期间轮询其退出状态的间隔。
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Attempts made when `exec` reports the staged file as busy.
/// `exec` 报告暂存文件忙时的尝试次数。
const SPAWN_ATTEMPTS: usize = 10;

/// Spawn the staged plugin, retrying the one transient failure `exec` reports.
/// 派生已暂存的插件，并对 `exec` 唯一会报的瞬时失败进行重试。
///
/// `ETXTBSY` ("Text file busy") means the file is open for writing somewhere.
/// The staged copy is private and was just written by this host, so the
/// condition is a race with a concurrent fork/exec rather than a real conflict:
/// running several plugin calls at once hit it here, reported as
/// `ExecutableFileBusy` by `stderr_larger_than_the_pipe_buffer_does_not_block`.
/// A host that spawns plugins from several threads can hit the same race, so it
/// is retried a bounded number of times and then reported as the I/O error it
/// is; nothing else is retried.
/// `ETXTBSY`（"Text file busy"）表示该文件在某处被以可写方式打开。这份暂存副本是私有的、
/// 刚由本宿主写入，因此该状况是与并发的 fork/exec 竞争，而不是真实冲突：并发执行多个插件
/// 调用时在这里撞上了它，被 `stderr_larger_than_the_pipe_buffer_does_not_block` 报成
/// `ExecutableFileBusy`。从多个线程派生插件的宿主会撞上同一种竞争，因此这里做有界重试，
/// 之后仍作为 I/O 错误上报；其他错误一律不重试。
///
/// The child's environment and working directory are the host's choice, not the
/// child's: `inherit_env: false` clears what the host would otherwise hand over,
/// the program's own variables are then the only ones present, and a configured
/// `current_dir` decides where it runs. None of that confines the filesystem or
/// the network — that still has to come from outside this workspace.
/// 子进程的环境与工作目录由宿主选择，而不是由子进程决定：`inherit_env: false` 清掉宿主本来
/// 会交出去的东西，此后程序自己声明的变量是唯一存在的；配置了 `current_dir` 就由它决定在哪里
/// 运行。这些都不约束文件系统或网络——那仍然必须来自本工作区之外。
pub(super) fn spawn_staged(
    program: &ProcessProgram,
    limits: ProcessLimits,
    operation: &str,
) -> Result<Child, HostError> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let mut command = Command::new(&program.executable);
        command
            .args(&program.arguments)
            .arg(operation)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if !limits.inherit_env {
            command.env_clear();
        }
        for (key, value) in &program.environment {
            command.env(key, value);
        }
        if let Some(directory) = &program.current_dir {
            command.current_dir(directory);
        }
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < SPAWN_ATTEMPTS
                    && error.kind() == std::io::ErrorKind::ExecutableFileBusy =>
            {
                thread::sleep(POLL_INTERVAL);
            }
            Err(error) => return Err(error.into()),
        }
    }
}

/// Bytes of stderr kept for the failure message.
/// 失败消息保留的 stderr 字节数。
const STDERR_BYTES: usize = 4096;

/// Read a child's stderr to EOF, keeping at most [`STDERR_BYTES`].
/// 读到子进程 stderr 的 EOF，最多保留 [`STDERR_BYTES`] 字节。
///
/// Reading to EOF even after the cap is reached is deliberate: stopping early
/// would let a chatty child block on a full pipe, which is the same failure this
/// adapter removed from stdout.
/// 达到上限后仍读到 EOF 是有意的：提前停止会让话多的子进程阻塞在满管道上，那正是本适配器
/// 从 stdout 上移除掉的同一种故障。
pub(super) fn read_stderr(reader: &mut impl Read) -> String {
    let mut kept = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let room = STDERR_BYTES.saturating_sub(kept.len());
                kept.extend_from_slice(&buffer[..read.min(room)]);
            }
        }
    }
    String::from_utf8_lossy(&kept).into_owned()
}

/// Read a stream to EOF, keeping nothing.
/// 把一个流读到 EOF，不保留任何内容。
///
/// Used after a response frame: a child that keeps writing after its answer must
/// not block on a full pipe. The frame ends the call — the adapter delivers it as
/// soon as it is complete — so this drain only keeps that child from blocking
/// while it finishes or lingers; a child that never exits is killed and reaped by
/// the caller, not here.
/// 用在响应帧之后：已经给出答案却继续写入的子进程绝不能阻塞在满管道上。帧会结束这次调用
/// ——适配器一收到完整帧就交付——因此这里的排空只是让那个子进程在收尾或挂住期间不阻塞；
/// 永不退出的子进程由调用方杀掉并回收，而不是在这里。
pub(super) fn drain_to_eof(reader: &mut impl Read) {
    let mut scratch = [0_u8; 1024];
    while let Ok(read) = reader.read(&mut scratch) {
        if read == 0 {
            break;
        }
    }
}

/// Kill the child and reap it, even when the kill itself fails.
/// 杀掉子进程并回收它，即使 kill 本身失败。
///
/// `Child::kill` fails with `InvalidInput` when the child has already been
/// reaped; propagating that with `?` would skip `Child::wait` and leave a
/// zombie, so both steps are attempted and neither is reported.
/// 子进程已被回收时 `Child::kill` 会以 `InvalidInput` 失败；用 `?` 传播它会跳过
/// `Child::wait` 并留下僵尸进程，因此两步都尝试、都不上报。
pub(super) fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Read one response frame: the declared length, then that many bytes.
/// 读取一个响应帧：先是声明的长度，然后是相应字节数。
///
/// The declared length is checked against the cap *before* the buffer is
/// allocated, so an over-limit declaration costs nothing.
/// 声明的长度在分配缓冲区**之前**就与上限比较，因此超限的声明不花任何代价。
///
/// A stream that ends before the frame does is reported by [`incomplete_frame`], which names the
/// operation and the shape of the cut; the reader's own `UnexpectedEof` says only that a buffer
/// could not be filled, and a caller cannot act on that (audit `LGC-LG-32`).
/// 帧还没读完流就结束的情形由 [`incomplete_frame`] 上报：它点名操作与截断的形状。读取器自己的
/// `UnexpectedEof` 只说"某个缓冲区没被填满"，调用方无法据此做任何事（审计 `LGC-LG-32`）。
pub(super) fn read_frame(
    reader: &mut impl Read,
    max_output: usize,
    operation: &str,
) -> Result<Vec<u8>, HostError> {
    let mut encoded_length = [0; 4];
    reader.read_exact(&mut encoded_length).map_err(|error| {
        incomplete_frame(operation, "its 4-byte length prefix was cut off", &error)
    })?;
    let length = u32::from_le_bytes(encoded_length) as usize;
    if length > max_output {
        return Err(HostError::Limit(format!(
            "output is {length} bytes; limit is {max_output}"
        )));
    }
    let mut output = vec![0; length];
    // The payload is read through a counter so the refusal can report how many bytes of the
    // declared frame arrived: `0 arrived` is a child that declared a frame it never started,
    // `1 arrived` is a child that died half way in — and `read_exact` alone reads alike, since it
    // only says that a buffer could not be filled (audit `LGC-LG-32`, second half; this is the
    // minimal improvement the t77 review asked for).
    // 负载经一个计数器读取，因此拒绝消息能报出声明的那一帧**到了多少**字节：`0 arrived` 是声明了
    // 帧却一个字节都没写的子进程，`1 arrived` 是写到一半就断的子进程——而单看 `read_exact` 两者
    // 一模一样，因为它只会说"某个缓冲区没被填满"（审计 `LGC-LG-32` 第二半；这正是 t77 复核要求
    // 的最小改进）。
    let mut arrived = Arrived {
        inner: reader,
        count: 0,
    };
    arrived.read_exact(&mut output).map_err(|error| {
        incomplete_frame(
            operation,
            &format!(
                "it declared {length} bytes of answer and {} arrived, then the stream ended",
                arrived.count
            ),
            &error,
        )
    })?;
    Ok(output)
}

/// Counts the bytes a reader actually delivered, so a refusal can say how much of a declared
/// frame arrived.
/// 统计读取器实际交付的字节数，好让拒绝消息说出声明的那一帧到了多少。
///
/// `read_exact` reports only that a buffer could not be filled; the count is what separates
/// "nothing was written" from "half a frame was written". Wrapping rather than looping keeps the
/// reader's own error (its `Display` and `kind`) exactly as `read_exact` produced it.
/// `read_exact` 只会报"某个缓冲区没被填满"；把"声明了多少 / 到了多少"分开的正是这个计数。用包装
/// 而不是自写循环，是为了让读取器自己的错误（它的 `Display` 与 `kind`）原封不动地就是
/// `read_exact` 产生的那个。
struct Arrived<'a, R> {
    inner: &'a mut R,
    count: usize,
}

impl<R: Read> Read for Arrived<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.count += read;
        Ok(read)
    }
}

/// Name the frame that never completed, keeping the reader's own error recoverable.
/// 点名那个从未完整的帧，并让读取器自己的错误仍可追。
///
/// `HostError::Process` is the only variant this seam has for a broken protocol, and it has no
/// source slot, so the original error travels in the text: its `Display` and then its `kind`,
/// which is what keeps `UnexpectedEof` (or any other reader failure) traceable from the outside.
/// `HostError::Process` 是这个接口给"协议破损"的唯一变体，而它没有存放源错误的槽位，因此原始错误
/// 随文本一起走：先是它的 `Display`，然后是它的 `kind`——正是这一点让 `UnexpectedEof`（或任何
/// 其它读取失败）从外部仍然可追。
fn incomplete_frame(operation: &str, shape: &str, error: &io::Error) -> HostError {
    HostError::Process(format!(
        "the process adapter got no complete frame for `{operation}`: {shape} ({error}; kind {:?})",
        error.kind()
    ))
}
