//! Process-isolated plugin execution with a hard per-call deadline.
//! 带单次调用硬超时的进程隔离插件执行。

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crate::run_method::{PluginAdapter, VerifiedPluginArtifact};
use tempfile::{Builder, TempPath};

use crate::plugin_host::{HostError, PluginInstance};

#[path = "process/child.rs"]
mod child;

use child::{POLL_INTERVAL, drain_to_eof, kill_and_reap, read_frame, read_stderr, spawn_staged};

/// Limits for one isolated process call.
/// 单次隔离进程调用的限制。
#[derive(Clone, Copy, Debug)]
pub struct ProcessLimits {
    /// Wall-clock deadline for one call; on expiry the host kills the **direct**
    /// child. A process the plugin started itself is not in that group — killing a
    /// group needs `libc` — so it can outlive the deadline with the host's uid,
    /// environment, filesystem and network. The threat model's residual risks say
    /// the same thing; confine the child yourself if that matters.
    /// 单次调用的挂钟超时；到期即由宿主终止**直接**子进程。插件自己启动的进程不在这一组里——按组杀
    /// 需要 `libc`——因此它可以带着宿主的 uid、环境、文件系统与网络活过截止时间。威胁模型的残余风险段
    /// 写的是同一件事；在意的话请在宿主之外限制子进程。
    pub timeout: Duration,
    /// Largest request payload accepted, in bytes.
    /// 接受的最大请求负载字节数。
    pub max_input_bytes: usize,
    /// Largest response payload accepted, in bytes. Checked against the length
    /// the child declares, before the host allocates the buffer.
    /// 接受的最大响应负载字节数。以子进程声明的长度为准，在宿主分配缓冲区之前检查。
    pub max_output_bytes: usize,
    /// Whether the child inherits this host's environment.
    /// 子进程是否继承本宿主的环境。
    ///
    /// The default is `true`, which is what a host that has always run its plugin
    /// this way expects. Setting it to `false` clears the environment instead, so
    /// an untrusted plugin cannot read the host's tokens, credentials, or
    /// configuration out of it; a host that needs to pass something specific adds
    /// it with [`ProcessProgram::environment`], and the plugin executable itself
    /// still starts because it is run by absolute path.
    /// 默认是 `true`，也就是一直这样运行插件的宿主所期望的行为。设为 `false` 时会清空环境，
    /// 使不受信任的插件无法从中读到宿主的令牌、凭据或配置；需要传特定变量的宿主用
    /// [`ProcessProgram::environment`] 显式添加，而插件可执行文件本身仍能启动，因为它以绝对
    /// 路径执行。
    pub inherit_env: bool,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            max_input_bytes: 1024 * 1024,
            max_output_bytes: 1024 * 1024,
            inherit_env: true,
        }
    }
}

/// Executable, fixed arguments, and child environment for an isolated plugin.
/// 隔离插件使用的程序、固定参数与子进程环境。
#[derive(Clone, Debug)]
pub struct ProcessProgram {
    executable: PathBuf,
    arguments: Vec<String>,
    environment: Vec<(String, String)>,
    current_dir: Option<PathBuf>,
}

impl ProcessProgram {
    /// Start a program specification with no fixed arguments.
    /// 以无固定参数开始描述一个程序。
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            arguments: Vec::new(),
            environment: Vec::new(),
            current_dir: None,
        }
    }

    /// Append one fixed argument, passed before every operation name.
    /// 追加一个固定参数，它排在每个操作名之前。
    pub fn argument(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    /// Set one environment variable for the child.
    /// 为子进程设置一个环境变量。
    ///
    /// This is the companion of [`ProcessLimits::inherit_env`]: with the
    /// environment cleared, these are the only variables the child sees, so a
    /// host passes what the plugin genuinely needs instead of handing over
    /// everything it has.
    /// 这是 [`ProcessLimits::inherit_env`] 的配套：清空环境后，子进程只看到这里设置的变量，
    /// 因此宿主传的是插件真正需要的东西，而不是把自己拥有的一切都交出去。
    pub fn environment(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    /// Run the child in `directory` instead of the host's working directory.
    /// 让子进程在 `directory` 中运行，而不是宿主的工作目录。
    pub fn current_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(directory.into());
        self
    }

    /// The configured executable path, before staging.
    /// 配置的可执行文件路径，尚未暂存。
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

/// Process-isolated plugin loader.
/// 进程隔离插件加载器。
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessBackend {
    limits: ProcessLimits,
}

impl ProcessBackend {
    /// Build a backend that applies `limits` to every loaded instance.
    /// 构造一个对每个已加载实例施加 `limits` 的后端。
    pub const fn new(limits: ProcessLimits) -> Self {
        Self { limits }
    }

    /// Stage the binary privately and require it to equal the artifact bytes.
    /// The returned instance runs that staged copy, so later edits to the original path
    /// cannot change what executes.
    /// 把二进制暂存到私有位置并要求它与工件字节相等。返回的实例运行该暂存副本，
    /// 因此之后改动原路径不会改变实际执行的内容。
    pub fn load_process_backend(
        &self,
        artifact: VerifiedPluginArtifact,
        program: ProcessProgram,
    ) -> Result<ProcessInstance, HostError> {
        let metadata = std::fs::metadata(program.executable())?;
        if !metadata.is_file() {
            return Err(HostError::InvalidArtifact(format!(
                "{} is not a file",
                program.executable().display()
            )));
        }
        let (_, verified_bytes) = artifact.into_parts();
        // The length the filesystem reports decides before anything is read: the
        // comparison below used to be the first use of the size, so a file whose
        // digest could not possibly match — a 2 GiB sparse file, say — was read
        // into memory in full before the mismatch could refuse it. Once the
        // lengths agree, reading it costs exactly what the verified bytes
        // already cost.
        // 由文件系统报告的长度在读取之前先做判断：下面那次比较过去是第一次用到尺寸，因此一个
        // 摘要不可能匹配的文件——比如 2 GiB 的稀疏文件——会被整份读进内存后才被拒绝。长度一致
        // 之后，读取的代价与已持有的已验证字节相同。
        if metadata.len() != verified_bytes.len() as u64 {
            return Err(HostError::InvalidArtifact(
                "process executable differs from the verified bytes".to_owned(),
            ));
        }
        if fs::read(program.executable())? != verified_bytes {
            return Err(HostError::InvalidArtifact(
                "process executable differs from the verified bytes".to_owned(),
            ));
        }
        let (program, staged_artifact) = stage_program(program, &verified_bytes)?;
        Ok(ProcessInstance {
            program,
            _staged_artifact: staged_artifact,
            limits: self.limits,
        })
    }
}

/// A process plugin. Each call gets a fresh child and a hard deadline.
/// 进程插件。每次调用使用独立子进程和硬超时。
pub struct ProcessInstance {
    program: ProcessProgram,
    _staged_artifact: TempPath,
    limits: ProcessLimits,
}

fn stage_program(
    program: ProcessProgram,
    bytes: &[u8],
) -> Result<(ProcessProgram, TempPath), HostError> {
    let suffix = program
        .executable
        .extension()
        .and_then(|extension| extension.to_str())
        .map_or_else(String::new, |extension| format!(".{extension}"));
    let mut staged = Builder::new()
        .prefix("xirang-plugin-")
        .suffix(&suffix)
        .tempfile()?;
    staged.write_all(bytes)?;
    staged.flush()?;
    set_executable(staged.path())?;
    let path = staged.into_temp_path();
    Ok((
        ProcessProgram {
            executable: path.to_path_buf(),
            arguments: program.arguments,
            // Staging moves the executable, not how the child is run: the
            // environment and working directory the host chose travel with it.
            // 暂存搬的是可执行文件，而不是子进程的运行方式：宿主选定的环境与工作目录随它一起走。
            environment: program.environment,
            current_dir: program.current_dir,
        },
        path,
    ))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), HostError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o500))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_: &Path) -> Result<(), HostError> {
    Ok(())
}

impl PluginInstance for ProcessInstance {
    fn adapter(&self) -> PluginAdapter {
        PluginAdapter::Process
    }

    /// Run one operation in a fresh child process.
    /// 在全新的子进程中执行一次操作。
    fn call(&self, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        // Why the direct implementation is wrong, and where the boundary is:
        // 直白实现为什么是错的，以及边界在哪里：
        //
        // The direct shape is "write stdin, poll `try_wait`, then read stdout".
        // That shape has two unbounded blocks. Both were measured against this
        // crate before this comment was written, and both are pinned by tests in
        // `plugin-host/tests/fault_matrix.rs`:
        // 直白写法是"写 stdin、轮询 `try_wait`、再读 stdout"。它有两处无界阻塞。两处都
        // 在写下这段注释之前对本 crate 实测过，并都由 `plugin-host/tests/fault_matrix.rs`
        // 的测试钉住：
        //
        // 1. A pipe holds only about 64 KiB. A poll loop that never drains stdout
        //    lets the child block inside `write`, so it never exits, so the host
        //    kills a healthy child and reports `Timeout`. Measured: a 65_536-byte
        //    frame succeeded and a 65_537-byte frame timed out, while
        //    `max_output_bytes` claimed 1 MiB. The real ceiling was the pipe
        //    buffer, and the reported failure had the wrong kind.
        // 1. 管道只有约 64 KiB。不排空 stdout 的轮询循环会让子进程阻塞在 `write` 里，
        //    永不退出，于是宿主杀掉一个健康的子进程并报 `Timeout`。实测：65_536 字节的
        //    帧成功、65_537 字节的帧超时，而 `max_output_bytes` 声称 1 MiB。真实上限是
        //    管道缓冲，且报告出来的失败种类是错的。
        // 2. Writing stdin happened before the deadline was armed, so a child that
        //    does not read stdin pinned the caller with no timeout at all.
        //    Measured: a 1 MiB input (exactly `max_input_bytes`) to a child that
        //    never reads blocked for that child's whole lifetime.
        // 2. 写 stdin 发生在超时启动之前，因此不读 stdin 的子进程会把调用方无限期钉住。
        //    实测：1 MiB 输入（正好等于 `max_input_bytes`）写给一个从不读 stdin 的子
        //    进程，阻塞了整个子进程生存期。
        //
        // Both are pipe-capacity problems, not timeout problems, so the fix is to
        // stop using the pipes as synchronization points: input is written from
        // its own thread, stdout and stderr are drained from their own threads,
        // and the deadline bounds the whole call. The declared limits become the
        // real limits.
        // 两者都是管道容量问题而不是超时问题，因此修法是让管道不再承担同步职责：输入在
        // 自己的线程里写，stdout 与 stderr 各自有线程排空，超时覆盖整个调用。声明的限制
        // 由此成为真实的限制。
        //
        // 3. The answer is the frame, and the frame ends the call. Polling for the
        //    child's exit *after* holding a complete frame was a third way to lose an
        //    answer: a child that wrote its frame and then lingered (waiting on a
        //    signal, sleeping, slow cleanup — anything but exiting) had that answer
        //    replaced by `Timeout` at the deadline. A complete frame is now delivered
        //    as soon as it arrives, and the exit status is consulted only when no frame
        //    arrives: a non-zero exit becomes `Process` with the child's stderr, and the
        //    deadline decides only the frame that never came (a child that is still
        //    running when its frame arrives is killed and reaped, so no call leaves a
        //    process behind). Audit `LGC-LG-32`.
        // 3. 答案就是那一帧，而帧会结束这次调用。在已经拿到完整帧之后仍去轮询子进程的退出，
        //    是丢掉答案的第三条途径：写完帧后挂住的子进程（等信号、睡眠、收尾慢——总之就是不
        //    退出）会在超时点被换成 `Timeout`。现在完整帧一到就交付，只有在帧从未到来时才去看
        //    退出状态：非零退出变成带 stderr 的 `Process`，而超时点只决定那个从未到来的帧
        //    （帧到达时仍在运行的子进程会被杀掉并回收，因此没有调用会留下进程）。审计
        //    `LGC-LG-32`。
        //
        // Residual boundary, stated rather than hidden: `Child::kill` kills only
        // the direct child. A plugin that forks a grandchild inheriting the pipes
        // can keep one open; the call still returns at the deadline, but that
        // call's detached writer or reader thread can stay blocked until the
        // grandchild exits. Killing the whole process group would need `libc`,
        // which this crate does not depend on.
        // 仍然存在的边界，明说而不隐藏：`Child::kill` 只杀直接子进程。插件若派生继承了
        // 管道的孙进程，该孙进程可以让管道保持打开；调用仍会在超时点返回，但这次调用的
        // 写入或读取线程可能一直阻塞到孙进程退出。要连进程组一起杀就需要 `libc`，而本
        // crate 没有该依赖。
        validate_operation(operation)?;
        check_input_length(input.len(), self.limits.max_input_bytes)?;
        let mut child = spawn_staged(&self.program, self.limits, operation)?;

        // One frame, built once: the 4-byte little-endian length prefix followed
        // by the payload. Moving it into the writer thread is also what lets the
        // caller's slice end with this call.
        // 一次构造完整帧：4 字节小端长度前缀加负载。把它移动进写入线程，也正是调用方的
        // 切片可以随这次调用结束而失效的原因。
        let mut request = Vec::with_capacity(4 + input.len());
        request.extend_from_slice(&(input.len() as u32).to_le_bytes());
        request.extend_from_slice(input);

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::Process("child stdin was unavailable".to_owned()))?;
        // The writer runs on its own thread so a child that never reads stdin
        // cannot pin the caller. Once the child dies the pending write fails with
        // a broken pipe and this thread ends.
        // 写入放在自己的线程上，不读 stdin 的子进程因此无法钉住调用方。子进程死后挂起的
        // 写入会以 broken pipe 失败，该线程随之结束。
        thread::spawn(move || {
            let _ = stdin.write_all(&request);
            // Closing stdin is what tells a reading child the request ended.
            // 关闭 stdin 是告诉正在读取的子进程"请求结束"的方式。
            drop(stdin);
        });

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::Process("child stdout was unavailable".to_owned()))?;
        let max_output = self.limits.max_output_bytes;
        let (frames, received) = mpsc::channel();
        // A frame that never completes is reported *about this operation*: the name travels into
        // the reader so the caller learns which step lost its answer instead of getting the bare
        // `failed to fill whole buffer` (audit `LGC-LG-32`, second half). The reader's own error
        // stays inside the message, which is where that seam keeps it reachable.
        // 「帧从未完整」会**针对本次操作**上报：名字随读取线程走，于是调用方得知是哪一步丢了答案，
        // 而不是只拿到裸的 `failed to fill whole buffer`（审计 `LGC-LG-32` 第二半）。读取器自己的
        // 错误留在消息内部——接口就是靠这一点让它仍然可追。
        let reader_operation = operation.to_owned();
        // Draining stdout for the child's whole life is what removes the pipe
        // buffer from the contract. `read_frame` rejects an over-limit declared
        // length before it allocates, so the cap also bounds memory.
        // 在整个子进程生存期内排空 stdout，正是把管道缓冲从契约里移除的那一步。
        // `read_frame` 在分配之前就拒绝超过上限的声明长度，因此该上限同时约束内存。
        thread::spawn(move || {
            // The frame leaves before the rest of stdout is drained, so a child
            // that answers and then keeps talking still delivers its answer at
            // once; draining is what keeps that child from blocking on a full
            // pipe while the host waits for it to exit. Reading one frame and
            // stopping was the bug: the child blocked, the host killed it at the
            // deadline, and a delivered answer was reported as a timeout. Waiting
            // for an exit *after* holding the frame was the other half of that
            // same bug (audit `LGC-LG-32`); the call now ends on the frame, which
            // is what the polling loop below implements.
            // 帧在排空 stdout 其余部分之前送出，因此先作答、后继续说话的子进程仍会立刻交付
            // 答案；排空正是让那个子进程不会在宿主等它退出时阻塞在满管道上的东西。只读一帧就
            // 停下曾是缺陷：子进程阻塞、宿主在超时点杀掉它，而一个已经送达的答案被报成超时。
            // 拿到帧之后还去等退出，是同一个缺陷的另一半（审计 `LGC-LG-32`）；现在调用以帧
            // 结束，下面的轮询循环做的就是这件事。
            let frame = read_frame(&mut stdout, max_output, &reader_operation);
            let _ = frames.send(frame);
            drain_to_eof(&mut stdout);
        });

        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| HostError::Process("child stderr was unavailable".to_owned()))?;
        let (messages, message_text) = mpsc::channel();
        // stderr is drained for the same reason, and the thread keeps reading to
        // EOF after the retained prefix fills so that no child can block on it.
        // stderr 因同样原因被排空；保留的前缀写满之后线程仍继续读到 EOF，因此没有子进程
        // 会因它阻塞。
        thread::spawn(move || {
            let _ = messages.send(read_stderr(&mut stderr));
        });

        let deadline = Instant::now() + self.limits.timeout;
        let mut frame: Option<Vec<u8>> = None;
        let mut refusal: Option<HostError> = None;
        // The loop ends on whichever comes first: a complete frame, or the child's exit.
        // The exit status is `None` when the frame won, which is how "the exit code is
        // missing" is represented — see the third hazard paragraph above this method.
        // 循环以先到的那个结束：完整的一帧，或者子进程的退出。帧赢时退出状态是 `None`，这就是
        // 「退出码缺失」的表示法——见本方法上方那段第 3 条。
        let status = loop {
            if frame.is_none() && refusal.is_none() {
                match received.try_recv() {
                    Ok(Ok(bytes)) => frame = Some(bytes),
                    // An over-limit declared length is final: the reader has
                    // stopped reading, so waiting for a child that may now be
                    // blocked on a full pipe would degrade `Limit` into a
                    // misleading `Timeout`.
                    // 超过上限的声明长度是最终结论：读取线程已经停止读取，此时去等一个可能
                    // 正阻塞在满管道上的子进程，只会把 `Limit` 降级成误导性的 `Timeout`。
                    Ok(Err(error @ HostError::Limit(_))) => {
                        kill_and_reap(&mut child);
                        return Err(error);
                    }
                    // Any other reader failure is only remembered, because the
                    // usual cause is a child that wrote no frame at all: it is
                    // about to be reported as `Process` with its stderr, and a
                    // broken frame would be the wrong diagnosis. The remembered
                    // value is *not* raw: `read_frame` already turned it into a
                    // `Process` naming this operation and the cut, so the path
                    // that returns it below (a child that exited 0 without a
                    // complete frame) is diagnosable too (audit `LGC-LG-32`).
                    // 其他读取失败只被记下，因为常见原因是子进程根本没写帧：它马上会被以
                    // `Process` 连同 stderr 上报，而"坏帧"会是错误的诊断。被记下的值**不是**裸
                    // 错误：`read_frame` 已经把它变成点名本次操作与截断形状的 `Process`，因此
                    // 下面那条会返回它的路径（以 0 退出却没给出完整帧的子进程）同样可诊断
                    // （审计 `LGC-LG-32`）。
                    Ok(Err(error)) => refusal = Some(error),
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        refusal = Some(HostError::Process(
                            "the child stdout reader stopped".to_owned(),
                        ));
                    }
                }
            }
            // A complete frame ends the call. The answer is precisely what the child was
            // asked for, and its exit status is not part of it: polling for the exit here is
            // what turned a delivered answer into `Timeout` whenever the child lingered
            // after writing it, which is the failure the drain thread below cannot prevent
            // by itself (audit `LGC-LG-32`).
            // 完整的一帧就结束这次调用。答案正是向子进程要的东西，而它的退出状态不属于答案：
            // 在这里轮询退出，正是「子进程写完答案后挂住 ⇒ 已经送达的答案被报成 `Timeout`」
            // 的成因，而这是下面那个排空线程本身防不住的那种失败（审计 `LGC-LG-32`）。
            if frame.is_some() {
                // The product exists, so the child has no further part in this call: it is
                // killed and reaped exactly like the deadline path does. A child that exited
                // on its own is merely reaped.
                // 产物已经存在，子进程不再参与这次调用：与超时路径完全相同地杀掉并回收它。
                // 自己退出的子进程只是被回收。
                kill_and_reap(&mut child);
                break None;
            }
            if let Some(status) = child.try_wait()? {
                break Some(status);
            }
            if Instant::now() >= deadline {
                kill_and_reap(&mut child);
                return Err(HostError::Timeout);
            }
            thread::sleep(POLL_INTERVAL);
        };
        if let Some(bytes) = frame {
            return Ok(bytes);
        }
        let status = status.expect("the loop breaks on a frame or on an exit status");
        if !status.success() {
            let detail = message_text
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_default();
            let detail = detail.trim();
            return Err(HostError::Process(if detail.is_empty() {
                format!("child exited with {status}")
            } else {
                detail.to_owned()
            }));
        }
        // The child exited 0, so stderr cannot explain the missing answer; the reader's verdict
        // is the diagnosis, and it already names the operation and the shape of the cut
        // (`read_frame` wraps it). Returning it raw was the second half of `LGC-LG-32`: the
        // caller got `failed to fill whole buffer` with nothing to act on.
        // 子进程以 0 退出，因此 stderr 解释不了缺失的答案；读取线程的判定就是诊断，而它已经点名了
        // 操作与截断的形状（`read_frame` 包装过）。原样返回它正是 `LGC-LG-32` 的第二半：调用方
        // 只拿到 `failed to fill whole buffer`，无从下手。
        if let Some(error) = refusal {
            return Err(error);
        }
        // The child is gone, so the reader is at EOF and returns without further
        // blocking. The wait can expire only when a grandchild still holds the
        // write end open, and then the deadline is the honest answer.
        // 子进程已消失，读取线程已到 EOF，不会再阻塞。只有当孙进程仍持有写端时这个等待
        // 才会到期，此时返回超时才是诚实的答案。
        received
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| HostError::Timeout)?
    }
}

fn validate_operation(operation: &str) -> Result<(), HostError> {
    if crate::run_method::validate_operation_name(operation).is_err() {
        return Err(HostError::InvalidOperation(operation.to_owned()));
    }
    Ok(())
}

/// The largest input this adapter can frame.
/// 本适配器能成帧的最大输入。
///
/// The process ABI carries the length in the frame's 4-byte prefix, so `u32::MAX` is the
/// ceiling the *frame* imposes: above it `input.len() as u32` wraps, and the child reads a
/// length that does not exist. The Wasm adapter has the same kind of ceiling at `i32::MAX`;
/// the two disagreed here for a while, because this adapter's refusal named the configured
/// limit in *both* halves (audit `BRG-BR-17`).
/// 进程 ABI 把长度放在帧的 4 字节前缀里，因此 `u32::MAX` 是**帧格式**给出的上限：超过它，
/// `input.len() as u32` 会回绕，子进程读到的长度并不存在。Wasm 适配器有同类的上限 `i32::MAX`；
/// 两者在这里有过分歧：本适配器的拒绝在**两种**情形下都报配置上限（审计 `BRG-BR-17`）。
const MAX_FRAMEABLE_INPUT: usize = u32::MAX as usize;

/// Reject an input this adapter cannot frame, naming whether the configured limit or the
/// frame width is what refused it.
/// 拒绝本适配器无法成帧的输入，并说明拒绝它的是配置上限还是帧宽。
///
/// Kept as one function for the same reason the Wasm adapter keeps its own: the frame-width
/// half is a value, so it is unit-testable without a four-gigabyte slice.
/// 收成一个函数的原因与 Wasm 适配器相同：帧宽那一半是一个值，因此无需一个四吉字节的切片就能
/// 被单测。
fn check_input_length(length: usize, configured: usize) -> Result<(), HostError> {
    if length > configured {
        return Err(HostError::Limit(format!(
            "input is {length} bytes; limit is {configured}"
        )));
    }
    if length > MAX_FRAMEABLE_INPUT {
        return Err(HostError::Limit(format!(
            "input is {length} bytes; a process call frames its length in a u32, whose maximum \
             is {MAX_FRAMEABLE_INPUT}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The input-length refusals, beside the check they pin.
    //! 输入长度拒绝，紧挨着它们所钉的那个检查。

    use super::*;

    /// The configured limit and the `u32` frame width refuse with different sentences. The
    /// frame-width half used to report the configured limit, so a caller with a large
    /// configured limit was told a number that had refused nothing (audit `BRG-BR-17`).
    /// 配置上限与 `u32` 帧宽用两句不同的话拒绝。帧宽那一半过去报的是配置上限，于是配置上限很大的
    /// 调用方会被告知一个什么都没拒绝的数字（审计 `BRG-BR-17`）。
    #[test]
    fn the_two_input_refusals_name_different_things() {
        let configured = check_input_length(5, 4)
            .expect_err("5 bytes over a 4-byte limit is refused")
            .to_string();
        let framed = check_input_length(MAX_FRAMEABLE_INPUT + 1, usize::MAX)
            .expect_err("a length above u32::MAX cannot be framed")
            .to_string();
        assert_ne!(configured, framed, "one sentence cannot name both refusals");
        assert!(configured.contains("limit is 4"), "{configured}");
        assert!(framed.contains("u32"), "{framed}");
        assert!(
            !framed.contains("limit is"),
            "the frame width is not a configured limit: {framed}"
        );
    }

    /// The frame-width half is reachable on the value path, which is the reason the check is
    /// one function: a length above `u32::MAX` never needs a four-gigabyte slice.
    /// 帧宽那一半只在值路径上即可构造，这正是该检查收成一个函数的原因：超过 `u32::MAX` 的长度
    /// 从不需要一个四吉字节的切片。
    #[test]
    fn the_frame_width_half_needs_no_allocation() {
        check_input_length(MAX_FRAMEABLE_INPUT, usize::MAX)
            .expect("exactly u32::MAX is still frameable");
        assert!(check_input_length(MAX_FRAMEABLE_INPUT + 1, usize::MAX).is_err());
    }
}
