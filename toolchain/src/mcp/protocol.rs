//! JSON-RPC framing and method dispatch for the MCP bridge.
//! MCP 桥的 JSON-RPC 分帧与方法分派。

use serde_json::{Value, json};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::mcp::tools::{advertised, tool_call};

/// Protocol revision advertised during initialization.
/// 初始化时通告的协议版本。
const PROTOCOL_VERSION: &str = "2025-06-18";
/// Default result cap for `xirang.search`.
/// `xirang.search` 的默认结果上限。
pub(crate) const DEFAULT_LIMIT: usize = 40;
/// Hard cap on lines returned by `xirang.read`.
/// `xirang.read` 返回行数的硬上限。
pub(crate) const MAX_READ_LINES: usize = 240;

/// Largest request line the bridge accepts, in bytes.
/// 本桥接受的最大请求行字节数。
///
/// The bridge is long-lived and reads from a client it does not control, so
/// `BufRead::lines()` was an allocation a hostile or buggy client could grow
/// without limit. One MiB is far above any real request — the largest thing a
/// reader sends is one short tool call — and far below the size that would
/// trouble the machine.
/// 本桥长期存活，且读取的是它无法控制的客户端，因此 `BufRead::lines()` 等于把一个可无限增长
/// 的分配交给敌对或有 bug 的客户端。1 MiB 远高于任何真实请求——读取方发的最大东西是一次简短的
/// 工具调用——又远低于会让机器难受的量。
pub(crate) const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// The project this bridge reads, resolved by `lexicon`'s rule.
/// 本桥读取的项目，按 `lexicon` 的规则解析。
///
/// The bridge calls the same `lexicon::resolve_package_root` the authoring
/// executor and Studio call, so the agent, the editor and the executor cannot
/// disagree about which project is open. Only the last-resort fallback is the
/// bridge's own and it stays the working directory, deliberately: a stdio bridge
/// is started inside the project the agent is working on, whereas the crate's
/// own manifest path is a path on the machine that compiled it — wrong for an
/// installed binary.
/// 本桥调用创作执行器与 Studio 所调用的同一个 `lexicon::resolve_package_root`，因此代理、
/// 编辑器与执行器不会对"打开的是哪个项目"产生分歧。只有最后兜底属于本桥自己，并且有意保持
/// 为当前目录：stdio 桥是在代理正在处理的项目里启动的，而本 crate 自己的清单路径是编译它的
/// 那台机器上的路径——对已安装的二进制来说是错的。
pub(crate) fn package_root() -> PathBuf {
    let configured = env::var_os(xirang_kernel::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = env::current_dir().ok();
    let fallback = current.clone().unwrap_or_else(|| PathBuf::from("."));
    xirang_kernel::lexicon::resolve_package_root(
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
        &fallback,
    )
}

/// Run the MCP stdio bridge over the process's own streams.
/// 在进程自己的流上运行 MCP stdio 桥。
///
/// A transport failure is reported rather than swallowed. This process exists to
/// answer on stdout, so an unreadable stdin or an unwritable stdout means it can
/// no longer do its job; exiting zero there would tell the caller the session
/// ended normally.
/// 传输失败会被报告而不是吞掉。本进程的存在意义就是在 stdout 上作答，因此 stdin 读不了或
/// stdout 写不了意味着它再也做不了这件事；在那里以 0 退出会告诉调用方这次会话正常结束。
pub fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_with(&mut stdin.lock(), &mut io::BufWriter::new(stdout.lock()))
}

/// Run the bridge over explicit streams, so the framing can be pinned by a test.
/// 在显式给定的流上运行桥，使分帧行为可以被测试钉住。
///
/// The streams are parameters rather than the process's own because the two
/// failure modes worth pinning — an unreadable request line and an unwritable
/// response — cannot be produced on a real terminal from inside a test.
/// 两个流作为参数传入而不是取进程自己的，是因为值得钉住的两种失败——请求行读不了、响应写不了
/// ——在测试里无法在真实终端上造出来。
pub fn run_with(input: &mut dyn BufRead, output: &mut dyn Write) -> Result<(), String> {
    let root = package_root();
    let _jobs = super::check_jobs::session();
    loop {
        let line = match read_frame(input)? {
            Frame::Eof => break,
            // An over-long line is refused *and* drained: the client is still
            // talking, and a refusal that left the rest of its line in the pipe
            // would corrupt the next frame.
            // 过长的行既被拒绝也被排空：客户端还在说话，而一次把该行剩余部分留在管道里的
            // 拒绝会污染下一帧。
            Frame::TooLong => {
                let response = error_response(
                    Value::Null,
                    -32600,
                    format!("invalid request: line exceeds {MAX_REQUEST_BYTES} bytes"),
                );
                write_response(output, &response)?;
                continue;
            }
            // A frame whose bytes are not UTF-8 is answered like the other malformed
            // frames and the session continues. It used to be the one that ended the
            // bridge: `main` printed to stderr and `exit(1)`, which contradicts both
            // READMEs' "writes nothing to stderr: a failure is an error response on
            // stdout" and left the client with a dead pipe instead of an answer
            // (audit `LGC-LG-22`).
            // 字节不是 UTF-8 的帧与其他畸形帧一样被作答，会话继续。它过去是唯一会终止桥的那一类：
            // `main` 往 stderr 打印并 `exit(1)`，这与两份 README 的"不写 stderr：失败是 stdout 上的
            // 错误响应"相反，也让客户端拿到一根死管道而不是答案（审计 `LGC-LG-22`）。
            Frame::Line(bytes) => match String::from_utf8(bytes) {
                Ok(line) => line,
                Err(_) => {
                    let response = error_response(
                        Value::Null,
                        -32600,
                        "invalid request: line is not valid UTF-8".to_owned(),
                    );
                    write_response(output, &response)?;
                    continue;
                }
            },
        };
        if line.trim().is_empty() {
            continue;
        }
        let responses: Vec<Value> = match serde_json::from_str::<Value>(&line) {
            // A batch is a JSON array of requests, and JSON-RPC answers it with
            // one response per request that is not a notification. Dispatching
            // the array as if it were a request collapsed it into one error.
            // 批处理是请求的 JSON 数组，JSON-RPC 对其中每个非通知请求各回一个响应。把数组
            // 当成单个请求分派会把它折叠成一个错误。
            Ok(Value::Array(requests)) if requests.is_empty() => vec![error_response(
                Value::Null,
                -32600,
                "invalid request: empty batch".to_owned(),
            )],
            Ok(Value::Array(requests)) => requests
                .iter()
                .map(|request| dispatch(&root, request))
                .filter(|response| !response.is_null())
                .collect(),
            Ok(request) => {
                let response = dispatch(&root, &request);
                if response.is_null() {
                    Vec::new()
                } else {
                    vec![response]
                }
            }
            Err(error) => vec![error_response(
                Value::Null,
                -32700,
                format!("invalid JSON: {error}"),
            )],
        };
        for response in responses {
            write_response(output, &response)?;
        }
    }
    Ok(())
}

/// One framed request line.
/// 一帧请求行。
enum Frame {
    /// One line, without its terminator.
    /// 一行，去掉了终止符。
    Line(Vec<u8>),
    /// A line longer than [`MAX_REQUEST_BYTES`]; its remainder was drained.
    /// 长于 [`MAX_REQUEST_BYTES`] 的一行；其余部分已被排空。
    TooLong,
    /// End of input.
    /// 输入结束。
    Eof,
}

/// Read one request line without allocating past the cap.
/// 读取一行请求，且分配不超过上限。
fn read_frame(input: &mut dyn BufRead) -> Result<Frame, String> {
    let mut buffer = Vec::new();
    // The `+ 1` is what distinguishes "exactly at the cap, newline next" from
    // "over the cap": reaching the cap without a terminator is the refusal.
    // `+ 1` 用来区分"正好到上限、下一个字节就是换行"与"超过上限"：到上限却没有终止符就是要
    // 拒绝的那一种。
    // `Read::take` is called through UFCS because method syntax tries to resolve
    // it on the trait object itself, which is unsized; the receiver here is the
    // *reference*, which is sized, and it borrows the reader for one frame only.
    // 这里用 UFCS 调用 `Read::take`，因为方法语法会试图在 trait 对象自身上解析它，而那是
    // unsized 的；这里的接收者是**引用**，它是 sized 的，并且只借用读取方一帧的时间。
    let read = std::io::Read::take(&mut *input, (MAX_REQUEST_BYTES + 1) as u64)
        .read_until(b'\n', &mut buffer)
        .map_err(|error| format!("cannot read stdin: {error}"))?;
    if read == 0 {
        return Ok(Frame::Eof);
    }
    if buffer.last() == Some(&b'\n') {
        buffer.pop();
        // `lines()` stripped a `\r\n` terminator too, and a client on Windows may
        // still send one.
        // `lines()` 也会剥掉 `\r\n` 终止符，而 Windows 上的客户端仍可能发它。
        if buffer.last() == Some(&b'\r') {
            buffer.pop();
        }
        return Ok(Frame::Line(buffer));
    }
    if read <= MAX_REQUEST_BYTES {
        // End of input ended a short final line that had no terminator.
        // 输入结束终结了一条没有终止符的短的最后一行。
        return Ok(Frame::Line(buffer));
    }
    // Drain the rest of the over-long line, stopping *at* its terminator. The
    // drain is bounded twice over: each pass reads at most one page, and
    // `read_until` stops at the newline instead of taking the next line with it —
    // a plain `read` into a scratch buffer did exactly that, because a slice or a
    // pipe hands over as much as fits, and the next frame vanished.
    // 排空过长行的剩余部分，并**停在**它的终止符处。排空受两重限制：每次最多读一页，而
    // `read_until` 停在该换行处、不会把下一行一起带走——直接 `read` 进暂存缓冲正是这么做的，
    // 因为切片或管道会给到能装下的全部内容，下一帧就此消失。
    let mut scratch = Vec::new();
    loop {
        scratch.clear();
        let read = std::io::Read::take(&mut *input, 4096)
            .read_until(b'\n', &mut scratch)
            .map_err(|error| format!("cannot read stdin: {error}"))?;
        if read == 0 || scratch.last() == Some(&b'\n') {
            break;
        }
    }
    Ok(Frame::TooLong)
}

/// Write one response object and its newline, flushing so a client sees it now.
/// 写出一个响应对象与它的换行，并 flush，使客户端立刻看到它。
fn write_response(output: &mut dyn Write, response: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *output, response)
        .map_err(|error| format!("cannot write stdout: {error}"))?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|error| format!("cannot write stdout: {error}"))
}

fn dispatch(root: &Path, request: &Value) -> Value {
    // A member that is not a JSON object is neither a request nor a notification:
    // JSON-RPC answers it with `-32600`, and with a null id because there is
    // nothing to correlate a reply with. Dropping it silently — which `get("id")`
    // on a string or a number did — left a client waiting forever.
    // 不是 JSON 对象的成员既不是请求也不是通知：JSON-RPC 以 `-32600` 作答，id 为 null，因为
    // 没有东西可与回复关联。静默丢弃它——对字符串或数字取 `get("id")` 就是如此——会让客户端
    // 永远等下去。
    let Some(object) = request.as_object() else {
        return error_response(
            Value::Null,
            -32600,
            "invalid request: not a JSON object".to_owned(),
        );
    };
    // JSON-RPC 2.0: a request without an `id` member is a notification and MUST
    // NOT be answered. An explicit `null` id is still a request, which is why
    // the check is for the member's absence rather than for a null value.
    // JSON-RPC 2.0：没有 `id` 成员的请求是通知，**不得**作答。显式的 `null` id 仍是
    // 请求，因此这里判断的是成员缺失，而不是值为 null。
    let Some(id) = object.get("id").cloned() else {
        return Value::Null;
    };
    // The version member is part of the envelope, not of any method, so it is
    // checked once here rather than in every arm below. A notification with the
    // wrong version is still silent: JSON-RPC forbids replying to it at all.
    // 版本成员属于信封而不属于任何方法，因此在这里检查一次，而不是在下面每个分支里。版本错误的
    // 通知仍然保持沉默：JSON-RPC 完全禁止对它作答。
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return error_response(
            id,
            -32600,
            "invalid request: `jsonrpc` must be \"2.0\"".to_owned(),
        );
    }
    let method = object.get("method").and_then(Value::as_str).unwrap_or("");
    let params = object.get("params").unwrap_or(&Value::Null);
    match method {
        "initialize" => success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "xirang-toolchain", "version": env!("CARGO_PKG_VERSION") },
                "instructions": crate::mcp::client::INSTRUCTIONS
            }),
        ),
        // An id-carrying `notifications/…` member is a *request* by the envelope rule
        // stated above, so it is answered; a genuine notification never reaches this arm
        // (it returns at the `id` check). Returning `Value::Null` here left such a client
        // waiting forever, and a one-member batch produced no output at all.
        // 带 id 的 `notifications/…` 成员按上面写明的信封规则是**请求**，因此要作答；真正的通知
        // 根本到不了这个分支（它在 `id` 检查处就返回了）。在这里返回 `Value::Null` 会让那种客户端
        // 永远等下去，而单成员 batch 则完全没有输出。
        "notifications/initialized" | "notifications/cancelled" => success(id, json!({})),
        "ping" => success(id, json!({})),
        // Audit `W1-1`: what arrives unasked is two entry points plus the catalogue tool, not the whole
        // 27-tool frame. `tools()` is still the catalogue every other reader uses (`--list`,
        // dispatch, `--list <tool>`), so nothing became unreachable — only unadvertised.
        // 审计 `W1-1`：不请自来的是两个入口加一个目录工具，而不是整帧 27 个工具。`tools()` 仍是其它每个
        // 读者用的目录（`--list`、分派、`--list <tool>`），因此没有任何东西变得够不着——只是不再被广告。
        "tools/list" => success(id, json!({ "tools": advertised() })),
        "tools/call" => tool_call(root, id, params),
        _ => error_response(id, -32601, format!("unknown method `{method}`")),
    }
}

/// Wrap a JSON-RPC result payload. 包装 JSON-RPC 结果负载。
pub(crate) fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Wrap a JSON-RPC error payload. 包装 JSON-RPC 错误负载。
pub(crate) fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod protocol_tests;
