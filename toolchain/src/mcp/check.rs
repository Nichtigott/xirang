//! An observed test run: which face was tested, what the run said, and nothing it did not say.
//! 一次被观测到的测试运行：测的是哪个面、运行说了什么，以及它没说的一律不算。
//!
//! This is the one tool that runs a process, and it exists because the alternative was measured and
//! found wanting: a defect compiled only under a non-default feature cannot fail on the default
//! face, so a green default run is not evidence about that feature, and neither this bridge nor the
//! control tool could say which face a red run would appear on. `xirang.status` answers the
//! static half (which faces exist, which one is the default); this answers the other half by
//! **observing** a run instead of guessing one. The design is `docs/design-mcp-test-faces.md`.
//! 这是唯一会跑进程的工具，它存在的理由是：另一条路被量过、而且不够用——只在非默认特性下编译的缺陷在
//! 默认面上不可能失败，因此默认面全绿不是关于那个特性的证据，而桥与对照工具都答不了"红会出现在哪个面"。
//! `xirang.status` 答静态那一半（有哪些面、哪个是默认面），这里用**观测**一次运行来答另一半，而不是
//! 猜它。设计见 `docs/design-mcp-test-faces.md`。
//!
//! Three rules keep observation honest, and each one is a pin:
//! 三条规矩让"观测"保持诚实，每一条都有钉子：
//!
//! - **Naming a face is the point, but the default is a convenience.** A caller who says no face
//!   gets `default`: the round measured 12 such calls and **6 of them left the tool for a shell
//!   `cargo test`** instead of naming one — a refusal that costs a turn to learn nothing. The
//!   face still decides the answer, and `status` says which faces exist.
//!   **点名面是要点，但缺省是便利。** 不说面的调用方拿到 `default`：那一轮量到 12 次这样的调用，
//!   其中 **6 次离开工具去 shell 跑 `cargo test`**，而不是补上一个面名——一次白花一轮的拒绝。面仍然
//!   决定答案，而 `status` 说出有哪些面。
//! - **A timeout or cancellation is unknown**, and terminates the execution process group.
//!   Deliberately detached descendants are outside that group.
//!   **超时或取消是 unknown**，并终止执行进程组；主动脱离的后代不属于该组。
//! - **Nothing ran is not a pass.** A log with no `test result:` line reports that, rather than an
//!   empty success.
//!   **什么都没跑不是通过。** 没有 `test result:` 行的日志会这么说，而不是给一个空的成功。
//!
//! online: a check has to see the sources and the build output as they are right now — that is what `check`/`verify` are for, and the record would be a second opinion about the thing being checked.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::mcp::build_evidence::out_dir;
use crate::mcp::truncation::withheld;
use xirang_kernel::maintenance::{UnknownReason, VerificationKind, VerificationOutcome, classify};

#[path = "check_census.rs"]
mod census;
pub(crate) use census::census_sample;

#[path = "check_routing.rs"]
mod routing;
pub(crate) use routing::next_step;

/// How long a run may take before it is declared unknown.
/// 一次运行在被判为 unknown 之前最长可以跑多久。
const DEFAULT_TIMEOUT_MS: u64 = 900_000;

/// The shortest and longest timeout a request may ask for.
/// 请求可以要求的最短与最长超时。
const MIN_TIMEOUT_MS: u64 = 1_000;
const MAX_TIMEOUT_MS: u64 = 3_600_000;

/// How many detail lines of each kind the summary keeps.
/// 摘要对每一类明细保留多少行。
const SAMPLE: usize = 12;

/// One test the log reported as failed, with the assertion words it carried, when they were read.
/// 日志报为失败的一个测试，以及它携带的断言原话（读到的话）。
///
/// The name and the message stay together because a negative control is only evidence when **this**
/// test failed on **this** violation: a message matched anywhere in a build log, or a failure in a
/// sibling test, does not attest to the intended check (A09).
/// 名字与消息必须在一起，因为只有当**这一个**测试因**这一条**违约失败时，负控制才是证据：在构建日志里
/// 任意位置匹配到的消息、或兄弟测试的失败，都不能为目标检查背书（A09）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FailedTest {
    /// The test's own full name, as cargo printed it.
    /// 测试自己的完整名字，按 cargo 打印的样子。
    pub(crate) name: String,
    /// The unique binary identity of the target instance it ran in, when the stream named one.
    /// 它所在 target 实例的唯一二进制身份（流点名了的话）。
    pub(crate) binary: Option<String>,
    /// The assertion words read from that test's own `---- <name> stdout ----` section.
    /// 从那个测试自己的 `---- <name> stdout ----` 段里读到的断言原话。
    pub(crate) message: Option<String>,
}

/// What a log said: the lines the reply carries, and how many `test result:` lines it had.
/// 日志说了什么：回复携带的行，以及它有几条 `test result:` 行。
///
/// The count is separate from the lines because it decides the verdict: a log with no result line is
/// not a pass, and the verdict line has to be able to say that without re-reading the log.
/// 这个计数与那些行分开，因为它决定判定：没有结果行的日志不是通过，而判定行必须能说出这件事，不能再读
/// 一遍日志。
pub(crate) struct Observation {
    /// The `result` and `failed` lines, in the order the log carried them.
    /// 按日志里的顺序排列的 `result` 与 `failed` 行。
    pub(crate) lines: Vec<String>,
    /// How many `test result:` lines the log had; zero means nothing ran.
    /// 日志里有几条 `test result:` 行；零意味着什么都没跑。
    pub(crate) results: usize,
    /// Tests actually executed (passed or failed), excluding ignored and filtered tests.
    /// 实际执行的测试数（通过或失败），不含忽略或过滤的测试。
    pub(crate) executed: usize,
    /// Tests the result lines said were skipped (`ignored` or `filtered out`).
    /// 结果行说被跳过（`ignored` 或 `filtered out`）的测试数。
    ///
    /// It is a separate count from `executed` because it answers a different question: a run that passed everything it
    /// executed has still not exercised what it skipped, and the anomaly dimension records exactly that difference.
    /// 它与 `executed` 分开计，因为它回答的是另一个问题：一次把执行到的都跑通的运行，仍然没有执行被跳过的那些，而异常维度记的
    /// 正是这个差别。
    pub(crate) skipped: usize,
    /// The tests that failed, with their own assertion words when they were read.
    /// 失败的测试，读到的话还带着它们自己的断言原话。
    ///
    /// Audit `W5-5`: routing by failure **kind** needs to know which tests failed, not only how many
    /// result lines there were. A family-shaped red and a missing build product are the two cases
    /// where the cheapest next call is not the literal search the default hint names.
    /// 审计 `W5-5`：按失败**种类**路由需要知道失败的是哪些测试，而不只是有几条结果行。同族形状的红与
    /// 缺席的构建产物，正是"最便宜的下一次调用不是默认那条字面检索"的两种情形。
    pub(crate) failed: Vec<FailedTest>,
    /// Every test name the stream carried, mapped to the **unique binary identities** it ran in.
    /// 流携带过的每个测试名，映射到它跑过的**唯一二进制身份**集合。
    ///
    /// A name alone is not an identity, and a source label is not an identity either: two packages
    /// can both run `src/lib.rs`, so the set holds the binary path from each `Running …` header. The
    /// set is built from the stream, not the retained text, so presentation truncation cannot make a
    /// duplicated name look unique (A09/A27).
    /// 名字本身不是身份，源标签也不是：两个包都可以跑 `src/lib.rs`，因此集合存的是每个 `Running …`
    /// 头里的二进制路径。该集合来自流而非保留文本，所以展示截断不能让重复的名字看起来唯一（A09/A27）。
    pub(crate) targets: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    /// Each binary identity's user-facing source label (e.g. `src/lib.rs`).
    /// 每个二进制身份对用户可见的源标签（如 `src/lib.rs`）。
    pub(crate) target_labels: std::collections::BTreeMap<String, String>,
    /// Whether a test row ran without a parsable `Running` header, so some attribution is unknown.
    /// 是否有测试行前面没有可解析的 `Running` 头，因此部分归属未知。
    pub(crate) incomplete: bool,
    /// The first `path:line` a compiler or runtime diagnostic named, when the log carried one.
    /// 日志里编译器或运行期诊断点名的第一个 `path:line`（若日志里有）。
    ///
    /// This is what makes the "nothing ran" route a **call** rather than a shrug: `why` needs a
    /// location, and rustc's own `--> file:line` is one.
    /// 正是它让"什么都没跑"那条路由成为一次**调用**而不是耸肩：`why` 需要一个位置，而 rustc 自己的
    /// `--> file:line` 就是一个。
    pub(crate) first_location: Option<String>,
}

/// One finished run and everything the reply is rendered from.
/// 一次结束的运行，以及回复渲染所依据的一切。
///
/// The text and the consumers share **this** value: the reply's first line is `verdict_text`, and
/// `check_plan` reads `verdict` and `observation.failed` rather than re-parsing the rendered string.
/// 文本与消费者共用**这一个**值：回复第一行来自 `verdict_text`，而 `check_plan` 直接读 `verdict` 与
/// `observation.failed`，不再重新解析渲染出来的字符串。
pub(crate) struct CheckOutcome {
    /// The tree the run happened in.
    /// 运行所在的树。
    pub(crate) root: PathBuf,
    /// The face the run selected (`default`, `all`, or a feature list).
    /// 运行选择的面（`default`、`all`，或特性列表）。
    pub(crate) face: String,
    /// The configured timeout, for the exit line's wording.
    /// 配置的超时，用于 exit 行的措辞。
    pub(crate) timeout: Duration,
    /// `test`, `check`, `build` or `clippy`.
    /// `test`、`check`、`build` 或 `clippy`。
    pub(crate) mode: String,
    /// Whether the run's success condition is executed tests or only compilation.
    /// 运行的通过条件是执行的测试，还是仅编译。
    pub(crate) kind: VerificationKind,
    /// The typed verdict.
    /// 类型化判定。
    pub(crate) verdict: VerificationOutcome,
    /// The finished command.
    /// 结束的命令。
    pub(crate) run: RunOutcome,
    /// What the log said.
    /// 日志说了什么。
    pub(crate) observation: Observation,
    /// The full command line, when the request carried selectors worth naming.
    /// 请求带有值得点名的选择器时的完整命令行。
    pub(crate) command_line: Option<String>,
    /// The run's id, appended to the reply and used for the log name.
    /// 运行的 id，追加到回复并用于日志名。
    pub(crate) run_id: String,
}

impl CheckOutcome {
    /// Whether a user-facing target selector names this binary identity.
    /// 用户可见的 target 选择器是否点中这个二进制身份。
    ///
    /// The selector may be the unique binary path or the source label, but a source label shared by
    /// several packages matches **all** of them, which is exactly the ambiguity that must stay
    /// unknown rather than certify one package's failure (the duplicate-package false green).
    /// 选择器可以是唯一二进制路径或源标签，但被多个包共享的源标签会命中**全部**，这正是必须保持 unknown
    /// 而不能给某个包的失败背书的歧义（重复包假绿）。
    fn target_matches(&self, binary: &str, selector: &str) -> bool {
        binary == selector
            || self
                .observation
                .target_labels
                .get(binary)
                .is_some_and(|label| label == selector)
    }

    /// The failed occurrences of `test`, when the selector resolves to exactly one target instance.
    /// 当选择器恰好解析到一个 target 实例时，`test` 的失败出现。
    ///
    /// The name must be the **exact full name** cargo printed (`unrelated::contract` is not
    /// `contract`). With no selector, a name seen in more than one binary identity is ambiguous; a
    /// selector that matches more than one identity is ambiguous too. An incomplete stream (a test
    /// row with no header) is never enough to claim uniqueness.
    /// 名字必须是 cargo 打印的**完整原名**（`unrelated::contract` 不是 `contract`）。不给选择器时，在不止
    /// 一个二进制身份出现过的名字有歧义；命中不止一个身份的选择器也有歧义。不完整的流（有测试行没有头）
    /// 绝不足以声称唯一。
    fn matching_failures<'a>(&'a self, test: &str, target: Option<&str>) -> Vec<&'a FailedTest> {
        if self.name_is_ambiguous(test, target) {
            return Vec::new();
        }
        self.observation
            .failed
            .iter()
            .filter(|failed| {
                failed.name == test
                    && target.is_none_or(|selector| {
                        failed
                            .binary
                            .as_deref()
                            .is_some_and(|binary| self.target_matches(binary, selector))
                    })
            })
            .collect()
    }

    /// Whether the test's target identity cannot be pinned to exactly one execution instance.
    /// 该测试的 target 身份是否无法钉到恰好一个执行实例。
    pub(crate) fn name_is_ambiguous(&self, test: &str, target: Option<&str>) -> bool {
        if self.observation.incomplete {
            return true;
        }
        match (self.observation.targets.get(test), target) {
            (None, _) => false,
            (Some(instances), None) => instances.len() > 1,
            (Some(instances), Some(selector)) => {
                instances
                    .iter()
                    .filter(|binary| self.target_matches(binary, selector))
                    .count()
                    != 1
            }
        }
    }

    /// Whether the stream's attribution evidence is incomplete (a headerless test row was seen).
    /// 流的归属证据是否不完整（见到过没有头的测试行）。
    pub(crate) fn target_evidence_incomplete(&self) -> bool {
        self.observation.incomplete
    }

    /// Whether a named test failed **and** its own assertion words carried `violation`.
    /// 是否有一个点名的测试失败，**且**它自己的断言原话包含 `violation`。
    ///
    /// A negative control's evidence is this conjunction and nothing weaker: a failure in another
    /// test, a name that only suffix-matches a sibling, an ambiguous name, or the phrase appearing
    /// in an unrelated part of the log does not attest to the target (A09).
    /// 负控制的证据就是这个合取，不能再弱：别的测试失败、只是后缀匹配到兄弟的名字、有歧义的名字、或那句话
    /// 出现在日志无关位置，都不为目标检查背书（A09）。
    pub(crate) fn failed_on(&self, test: &str, target: Option<&str>, violation: &str) -> bool {
        self.matching_failures(test, target).iter().any(|failed| {
            failed
                .message
                .as_deref()
                .is_some_and(|message| message.contains(violation))
        })
    }

    /// Whether the named test appears among the failures at all, whatever its assertion said.
    /// 点名的测试是否出现在失败集合里，无论它的断言说了什么。
    pub(crate) fn named_test_failed(&self, test: &str, target: Option<&str>) -> bool {
        !self.matching_failures(test, target).is_empty()
    }
}

/// What one finished (or killed) command produced.
/// 一条结束（或被杀死）的命令产出了什么。
pub(crate) struct RunOutcome {
    /// The exit code, when the process reported one.
    /// 进程报告了退出码时的退出码。
    pub(crate) code: Option<i32>,
    /// Whether the configured timeout fired and the child was killed.
    /// 配置的超时是否触发、子进程是否被杀。
    pub(crate) timed_out: bool,
    /// Whether the caller cancelled and the child was killed.
    /// 调用方是否取消、子进程是否被杀。
    pub(crate) cancelled: bool,
    /// How long it ran.
    /// 它跑了多久。
    pub(crate) elapsed: Duration,
    /// Where the full output landed.
    /// 完整输出的落点。
    pub(crate) log: PathBuf,
}

/// Run one command, its output going straight to a log file.
/// 跑一条命令，输出直接写进日志文件。
///
/// The output goes to the file rather than to a pipe on purpose: a pipe nobody drains fills up and
/// deadlocks the child, which is a failure mode this repository has already paid for once.
/// 输出写文件而不是走管道是有意的：没人排空的管道会填满并让子进程死锁,而这个失败模式本仓已经付过一次代价。
pub(crate) fn run_command(
    command: &mut Command,
    timeout: Duration,
    log: &Path,
) -> Result<RunOutcome, String> {
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let handle =
        File::create(log).map_err(|error| format!("cannot write {}: {error}", log.display()))?;
    let errors = handle
        .try_clone()
        .map_err(|error| format!("cannot duplicate the log handle: {error}"))?;
    if super::check_jobs::cancelled() {
        return Err("cancelled before launch; outcome unknown".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::from(handle))
        .stderr(Stdio::from(errors))
        .stdin(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot run the check: {error}"))?;
    let mut timed_out = false;
    let mut cancelled = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if started.elapsed() >= timeout {
                    timed_out = true;
                    super::check_jobs::terminate(&mut child);
                    break None;
                }
                if super::check_jobs::cancelled() {
                    cancelled = true;
                    super::check_jobs::terminate(&mut child);
                    break None;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(error) => {
                super::check_jobs::terminate(&mut child);
                return Err(format!("cannot wait for the check: {error}"));
            }
        }
    };
    Ok(RunOutcome {
        code,
        timed_out,
        cancelled,
        elapsed: started.elapsed(),
        log: log.to_path_buf(),
    })
}

/// The run's verdict, as the first line of the reply, rendered from the typed value.
/// 这次运行的判定，作为回复的第一行，由类型化值渲染。
///
/// Round 7 measured the defect this line exists for, on the frozen fixture `target/round7/s5`:
/// `cargo test` failed with exit 101, the reply said so on its **sixth** line (`exit   101`), and
/// the one-shot client exited `0` — so a reader that stopped at the top of the reply, or took the
/// client's exit code for the verdict, read a failing face as green. The verdict is now line 1, and
/// the first two lines are enough to tell a red face from a green one.
/// 第七轮在冻结夹具 `target/round7/s5` 上量到这一行存在的理由：`cargo test` 以 101 失败，回复把它写在
/// **第六**行（`exit   101`），而一次性客户端以 `0` 退出——于是只在回复开头停下的读者、或把客户端退出码
/// 当判定的读者，会把失败的面读成绿。现在判定是第 1 行，头两行就足以区分红面与绿面。
///
/// It is the **run's** verdict, never the call's: a timeout, a signal and a log with no
/// `test result:` line are all `unknown`, because none of them is evidence of a pass, and the
/// client's own `0` (answered) says nothing about any of them.
/// 它是**这次运行**的判定，绝不是这次调用的：超时、信号、以及没有 `test result:` 行的日志都是
/// `unknown`，因为它们都不是通过的证据，而客户端自己的 `0`（作答）对其中任何一个都没说什么。
fn verdict_text(verdict: &VerificationOutcome, code: Option<i32>) -> String {
    match verdict {
        VerificationOutcome::Passed => "verdict  passed (cargo exit 0)".to_owned(),
        VerificationOutcome::Failed => {
            format!("verdict  failed (cargo exit {})", code.unwrap_or_default())
        }
        VerificationOutcome::Unknown(UnknownReason::Timeout)
        | VerificationOutcome::Unknown(UnknownReason::Cancelled) => {
            "verdict  unknown (the run timed out or was cancelled; a killed run is not a pass)"
                .to_owned()
        }
        VerificationOutcome::Unknown(UnknownReason::NoResultLine) => {
            "verdict  unknown (cargo exit 0, but the log has no `test result:` line: \
             nothing ran, and that is not a pass)"
                .to_owned()
        }
        VerificationOutcome::Unknown(UnknownReason::ZeroExecuted) => {
            "verdict  unknown (cargo exit 0, but zero tests executed; ignored/filtered tests do not \
             verify a repair)"
                .to_owned()
        }
        VerificationOutcome::Unknown(UnknownReason::Signalled) => {
            "verdict  unknown (the process was signalled; a signalled run is not a pass)".to_owned()
        }
    }
}

/// The head of the reply: the verdict first, then what produced it.
/// 回复的开头：判定在最前，然后是产出它的一切。
///
/// Only [`check`]'s own callers see the census, and it is appended *after* this: the verdict is the
/// thing a reader stops at, and no later section may push it down.
/// 只有 [`check`] 自己的调用方会看到总账，而它接在这之后：判定是读者会停下的那件事，后面任何一段都不
/// 能把它挤下去。
fn head(outcome: &CheckOutcome) -> Vec<String> {
    let run = &outcome.run;
    let observed = &outcome.observation;
    let mut lines = vec![
        verdict_text(&outcome.verdict, run.code),
        format!("check  cargo test{}", flags(&outcome.face)),
        format!("face   {}", outcome.face),
        format!("root   {}", outcome.root.display()),
        format!("elapsed {} ms", run.elapsed.as_millis()),
        format!("log    {}", run.log.display()),
    ];
    if run.timed_out || run.cancelled {
        lines.push(format!(
            "exit   unknown (stopped by timeout/cancel (configured limit {} ms); the process group was killed; detached descendants are outside it — and the log so far is at {})",
            outcome.timeout.as_millis(),
            run.log.display()
        ));
    } else {
        match run.code {
            Some(code) => lines.push(format!("exit   {code}")),
            None => lines.push("exit   unknown (the process was signalled)".to_owned()),
        }
    }
    lines.extend(observed.lines.iter().cloned());
    if let Some(next) = next_step(
        &outcome.face,
        run.timed_out || run.cancelled,
        run.code,
        observed,
    ) {
        lines.push(next);
    }
    lines
}

/// Test this tree's faces by running them, and report only what the run said.
/// 跑一遍这棵树的面来做测试，并只报出这次运行说了什么。
pub(crate) fn check(root: &Path, arguments: &Value) -> Result<String, String> {
    if arguments.get("files").is_some() || arguments.get("plan_id").is_some() {
        return super::check_plan::check_or_plan(root, arguments);
    }
    let outcome = outcome(root, arguments)?;
    Ok(render(arguments, &outcome))
}

/// Run the requested check and return the typed outcome, before any text is rendered.
/// 执行请求的检查，并在渲染任何文本之前返回类型化结果。
///
/// This is the one entry a consumer that needs the **result** rather than the reply uses; the reply
/// itself is [`render`] over this value, so a reader can never disagree with a parser of the text.
/// 这是需要**结果**而不是回复的消费者所用的唯一入口；回复本身就是对这个值的 [`render`]，因此读者与
/// 文本解析器不可能得出不同结论。
pub(crate) fn outcome(root: &Path, arguments: &Value) -> Result<CheckOutcome, String> {
    super::check_request::validate(arguments)?;
    // No `face` means the default face, not a refusal. The round measured what the refusal cost:
    // 12 calls arrived without `face` and 6 of them did not correct the shape but left the tool for
    // a shell `cargo test` instead — while the two faces a question usually needs share one red, so
    // the second call bought the same answer. Choosing a face is still what this tool is *for*: pass
    // one whenever you mean a specific face.
    // 没给 `face` 就是默认面，不是拒绝。那一轮量到这次拒绝的代价：12 次调用没带 `face`，其中 6 次没有
    // 改对形状，而是离开工具去跑 shell 的 `cargo test`——而一道题通常需要的两个面常常共享同一条红，
    // 第二次调用买到的是同一个答案。选面仍然是这工具的**意义**：想指定某个面时就显式传它。
    let face = arguments
        .get("face")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|face| !face.is_empty())
        .unwrap_or("default");
    let timeout = Duration::from_millis(
        arguments
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS),
    );
    let mode = arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("test");
    let mut command = Command::new("cargo");
    command
        .arg(mode)
        .current_dir(root)
        .env("CARGO_TERM_COLOR", "never");
    // Cargo writes its products inside the tree unless told otherwise, and the round measured what
    // that costs: `check` on a restored tree leaves a `target/` whose artifacts are NEWER than the
    // sources, so the next `cargo test` (or the next `check`) reuses the old binary and reports a
    // green that belongs to the previous round's code. A verifier asked to take every baseline with
    // an out-of-tree target had no way to do it through this tool at all.
    // 不特别指定时，cargo 把产物写在树内，而那一轮量到了它的代价：在还原过的树上跑 `check`，会留下一个
    // 产物**新于**源码的 `target/`，于是下一次 `cargo test`（或下一次 `check`）复用旧二进制、报出一个
    // 属于**上一轮代码**的绿。有验证者被要求"基线一律用树外 target"，而通过这个工具**根本做不到**。
    let target_dir = arguments
        .get("target_dir")
        .and_then(Value::as_str)
        .map(|directory| root.join(directory));
    if let Some(directory) = &target_dir {
        command.env("CARGO_TARGET_DIR", directory);
    }
    match face {
        "default" => {}
        "all" => {
            command.arg("--all-features");
        }
        feature => {
            command.arg("--features").arg(feature);
        }
    }
    for (key, flag) in [
        ("workspace", "--workspace"),
        ("all_targets", "--all-targets"),
        ("no_default_features", "--no-default-features"),
    ] {
        if arguments.get(key).and_then(Value::as_bool) == Some(true) {
            command.arg(flag);
        }
    }
    for (key, flag) in [
        ("package", "--package"),
        ("test", "--test"),
        ("target", "--target"),
    ] {
        if let Some(value) = arguments.get(key).and_then(Value::as_str) {
            command.arg(flag).arg(value);
        }
    }
    // `release` is the only profile that needs a flag; `debug` is Cargo's default, and spelling it
    // `--profile dev` would make two spellings of one build look like two runs in the command line
    // and in the execution identity built from it.
    // `release` 是唯一需要开关的 profile；`debug` 是 Cargo 的默认值，而把它写成 `--profile dev` 会让同一个
    // 构建的两种拼法在命令行、以及由命令行构造的执行身份里看起来像两次运行。
    if arguments.get("profile").and_then(Value::as_str) == Some("release") {
        command.arg("--release");
    }
    if let Some(filter) = arguments.get("filter").and_then(Value::as_str) {
        command.arg("--").arg(filter);
    }
    if arguments.get("deny_warnings").and_then(Value::as_bool) == Some(true) {
        command.args(["--", "-D", "warnings"]);
    }
    let command_line = format!(
        "check  cargo {}",
        command
            .get_args()
            .map(|value| serde_json::to_string(&value.to_string_lossy())
                .expect("string serialization"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    // The log is the second reason a tree gets a `target/`: `out_dir(root)` is *inside* the tree, so
    // even with cargo redirected the tool would leave `<root>/target/xirang/out/` behind. When the
    // caller names a tree-external target, the log rides with it.
    // 日志是"树里长出 `target/`"的第二个原因：`out_dir(root)` 在树**内**，所以即使 cargo 被改道，工具
    // 仍会留下 `<root>/target/xirang/out/`。调用方指定了树外 target 时，日志跟着它走。
    // Cargo accepts package/feature selectors; they are argv values, never log paths.
    // Cargo 接受 package/feature 选择器；它们只是 argv 值，不能作为日志路径。
    let log_face = if face
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_-.,+".contains(&byte))
    {
        face.to_owned()
    } else {
        format!(
            "features-{}",
            &xirang_kernel::sha256_hex(face.as_bytes())[..16]
        )
    };
    let run_id = super::check_jobs::new_id();
    let log = match &target_dir {
        Some(directory) => directory.join(format!("check-{log_face}-{run_id}.log")),
        None => out_dir(root).join(format!("check-{log_face}-{run_id}.log")),
    };
    let run = run_command(&mut command, timeout, &log)?;
    let observed = observation(
        &run.log,
        arguments.get("verbose").and_then(Value::as_bool) == Some(true),
    )?;
    // The verdict is line 1 (see `verdict_text`), ahead of everything the run produced, because
    // round 7 measured a reader taking the one-shot client's `0` for a green face while the only
    // failure report sat on line six. The `exit` line below is the run's own code, and it stays.
    // 判定是第 1 行（见 `verdict_text`），排在这次运行产出的一切之前，因为第七轮量到有读者把一次性客户
    // 端的 `0` 当成绿面，而唯一那句失败报告在第六行。下面的 `exit` 行是运行自己的码，保留。
    // A compile-only run is classified as `Compilation` and keeps its actual (test-free) counts; a
    // test run is classified as `Tests` and keeps the counts the stream observed. Neither fabricates
    // `results = 1` / `executed = 1` to feed the test classifier.
    // 仅编译的运行按 `Compilation` 分类并保留其实际的（无测试）计数；测试运行按 `Tests` 分类并保留流
    // 观测到的计数。两者都不伪造 `results = 1` / `executed = 1` 去喂测试分类器。
    let kind = if mode == "test" {
        VerificationKind::Tests
    } else {
        VerificationKind::Compilation
    };
    let verdict = classify(
        kind,
        run.timed_out,
        run.cancelled,
        run.code,
        observed.results,
        observed.executed,
    );
    let command_line = [
        "mode",
        "target",
        "all_targets",
        "deny_warnings",
        "workspace",
        "package",
        "test",
        "filter",
        "no_default_features",
    ]
    .iter()
    .any(|key| arguments.get(key).is_some())
    .then_some(command_line);
    Ok(CheckOutcome {
        root: root.to_path_buf(),
        face: face.to_owned(),
        timeout,
        mode: mode.to_owned(),
        kind,
        verdict,
        run,
        observation: observed,
        command_line,
        run_id,
    })
}

/// Render one typed outcome as the reply text, census included.
/// 把一个类型化结果渲染成回复文本，含总账。
pub(crate) fn render(arguments: &Value, outcome: &CheckOutcome) -> String {
    let mut lines = head(outcome);
    if let Some(command_line) = &outcome.command_line {
        lines[1] = command_line.clone();
    }
    lines.push(format!("run_id {}", outcome.run_id));
    if outcome.mode != "test" {
        lines.retain(|line| !line.starts_with("result none") && !line.starts_with("next   "));
        lines.push(format!(
            "scope  {} only; no behavioral tests executed",
            outcome.mode
        ));
    }
    // An explicit false disables source analysis, not merely its rendering. When requested,
    // the tree count and census share one source inventory.
    // 显式 false 关闭源码分析而非只隐藏输出；需要时，规模与总账共用一次源码清单。
    if arguments.get("census").and_then(Value::as_bool) == Some(false) {
        lines.push(
            "census skipped (requested); source/coverage obligations were not assessed".into(),
        );
    } else {
        match crate::mcp::source_index::load_sources(&outcome.root) {
            Ok(sources) => {
                lines.push(format!(
                    "tree   {} rust file(s), {} function(s)",
                    sources.len(),
                    sources
                        .iter()
                        .map(|file| file.functions.len())
                        .sum::<usize>()
                ));
                let whole = arguments.get("census").and_then(Value::as_bool) == Some(true);
                let census = crate::mcp::claims::census_from(&sources, whole);
                if whole {
                    lines.extend(census);
                } else {
                    lines.extend(census_sample(&census));
                }
            }
            Err(reason) => lines.push(format!("census unavailable ({reason})")),
        }
    }
    lines.join("\n")
}

/// The exact flags the command carried, so the answer names what it ran.
/// 命令实际带的开关，让答案说清它跑的是什么。
fn flags(face: &str) -> String {
    match face {
        "default" => String::new(),
        "all" => " --all-features".to_owned(),
        feature => format!(" --features {feature}"),
    }
}

/// The failing assertion's own words, per failing test, read out of the same log.
/// 失败断言自己的话，按失败的测试逐个，从同一份日志里读出。
///
/// Cargo prints them under `---- <name> stdout ----`, and they are the one thing the reply did not
/// carry. Measured in W8's h1 (step 5): the arm got `verdict failed` and the failing test's name,
/// then read the raw log itself — "Let's read the check log raw" — while this module's own `next`
/// line already said the message is a string in the tree. **Knowing what is missing and making the
/// caller fetch it anyway is a call spent on something we had in hand.**
/// cargo 把这句话印在 `---- <name> stdout ----` 下面，而它正是回复没有携带的那样东西。W8 的 h1 第 5 步
/// 量到：那一臂拿到 `verdict failed` 与失败的测试名，随后自己去读原始日志——「Let's read the check log
/// raw」——而本模块自己的 `next` 行早就写着"那句话是树里的一个字符串"。**知道自己少给了什么、却让调用方
/// 自己去取，就是把一次调用花在我们手里已有的东西上。**
fn why_lines(text: &str, failed: &[FailedTest]) -> Vec<String> {
    let lines = text.lines().collect::<Vec<_>>();
    failed
        .iter()
        .filter_map(|test| {
            test_message(&lines, &test.name)
                .map(|message| format!("why    {}: {message}", test.name))
        })
        .collect()
}

/// One failing test's own words, read from **that test's** section of the log.
/// 一个失败测试自己的话，从**那个测试的**日志段里读出。
///
/// The section is addressed by the test's own name (`---- <name> stdout ----`), so a phrase that
/// happens to appear elsewhere in a build log is not evidence about this test (A09). Two lines and
/// 200 characters are the bounded excerpt; `None` means the section was absent or empty, which is
/// `unknown` rather than a match.
/// 段落由测试自己的名字寻址（`---- <name> stdout ----`），因此碰巧出现在构建日志别处的短语不是关于这个
/// 测试的证据（A09）。两行、两百字符是有界摘录；`None` 表示段落缺失或为空，那是 `unknown`，不是匹配。
fn test_message(lines: &[&str], name: &str) -> Option<String> {
    const MESSAGE_LINES: usize = 2;
    const MESSAGE_CHARS: usize = 200;
    let marker = format!("---- {name} stdout ----");
    let start = lines.iter().position(|line| line.trim() == marker)?;
    let mut said = Vec::new();
    for line in lines.iter().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with("----") || trimmed.starts_with("note:") {
            break;
        }
        if trimmed.is_empty() {
            if said.is_empty() {
                continue;
            }
            break;
        }
        said.push(trimmed);
        if said.len() == MESSAGE_LINES {
            break;
        }
    }
    if said.is_empty() {
        return None;
    }
    let message = said.join(" ");
    Some(if message.chars().count() > MESSAGE_CHARS {
        let cut = message.chars().take(MESSAGE_CHARS).collect::<String>();
        format!("{cut}…")
    } else {
        message
    })
}

/// What the log said, as the run said it — and how many `test result:` lines it had, because a log
/// with none is not a pass and the verdict line has to be able to say so.
/// 日志说了什么，按运行自己说的样子——以及它有几条 `test result:` 行，因为没有的那种日志不是通过，
/// 而判定行必须能说出这件事。
fn observation(log: &Path, verbose: bool) -> Result<Observation, String> {
    // The typed facts come from the stream (`LogFacts`), not from the retained text: presentation
    // truncation drops `Running` headers and passing rows, so reading attribution back out of
    // `text` could make a duplicated test name look unique (A09/A27).
    // 类型化事实来自流（`LogFacts`），而不是保留文本：展示截断会丢掉 `Running` 头与通过的测试行，从
    // `text` 里反读归属会让重复的测试名看起来唯一（A09/A27）。
    let facts = super::check_log::read(log)?;
    let text = facts.text;
    let retained_results = text
        .lines()
        .filter(|line| line.contains("test result:"))
        .map(|line| line.trim().to_owned())
        .collect::<Vec<_>>();
    let failed: Vec<(String, Option<String>)> = facts
        .rows
        .iter()
        .filter(|row| row.failed)
        .map(|row| {
            (
                row.name.clone(),
                row.target.as_ref().map(|target| target.binary.clone()),
            )
        })
        .collect();
    let mut targets: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for row in &facts.rows {
        if let Some(target) = &row.target {
            targets
                .entry(row.name.clone())
                .or_default()
                .insert(target.binary.clone());
        }
    }
    // Every header the stream named gets a label, including a target whose tests all
    // passed or which ran none: a selector must be able to name an instance the stream
    // established, not only one that happened to fail.
    // 流点名过的每个头都拿到标签，包括全部通过或一个测试都没跑的 target：选择器必须能点名流已确立的实例，
    // 而不只是碰巧失败过的那个。
    let target_labels: std::collections::BTreeMap<String, String> = facts
        .targets
        .iter()
        .map(|(binary, label)| (binary.clone(), label.clone()))
        .collect();
    let mut lines = Vec::new();
    if facts.results == 0 {
        lines.push(
            "result none (no `test result:` line in the log: nothing ran, and that is not a pass)"
                .to_owned(),
        );
    } else {
        // Audit `W1-3`: a run has one group per test binary (unit tests, then one per integration
        // file), and every group prints its own `test result:` line. The ones that carry a failure
        // are the answer and are printed one by one; the groups that **passed** are a count and the
        // first line, because a reader who needs the rest asked for `verbose: true` — that is the
        // fold, and it is where the reply's tail came from on a wide tree.
        // 审计 `W1-3`：一次运行每个测试二进制品一组（单元测试，然后每个集成文件一组），每组印自己那行
        // `test result:`。带失败的那几行就是答案、逐行印出；**通过**的那些折成一个计数加第一行，因为需要
        // 其余部分的读者会要 `verbose: true`——这就是那次折叠，也正是宽树上回复尾部的来路。
        let (passed, failed): (Vec<&String>, Vec<&String>) = retained_results
            .iter()
            .partition(|line| line.contains("test result: ok"));
        for line in failed.iter().take(SAMPLE) {
            lines.push(format!("result {line}"));
        }
        if failed.len() > SAMPLE {
            lines.push(withheld(
                failed.len() - SAMPLE,
                failed.len(),
                SAMPLE,
                "failing result lines",
                "read the log named above",
            ));
        }
        if passed.len() == 1 || verbose {
            for line in passed.iter().take(SAMPLE) {
                lines.push(format!("result {line}"));
            }
            if passed.len() > SAMPLE {
                lines.push(withheld(
                    passed.len() - SAMPLE,
                    passed.len(),
                    SAMPLE,
                    "passing result lines",
                    "read the log named above",
                ));
            }
        } else if !passed.is_empty() {
            lines.push(format!(
                "result {} group(s) passed — pass `verbose: true` for each line; first: {}",
                passed.len(),
                passed[0]
            ));
        }
    }
    lines.extend(
        failed
            .iter()
            .take(SAMPLE)
            .map(|(name, _)| format!("failed {name}")),
    );
    if failed.len() > SAMPLE {
        lines.push(withheld(
            failed.len() - SAMPLE,
            failed.len(),
            SAMPLE,
            "failing tests",
            "read the log named above",
        ));
    }
    // The assertion's own words ride with the name of the test that produced them: the caller asked
    // what failed, and the answer is these lines, not a pointer to where they live.
    // 断言自己的话与产出它的测试名同行给出：调用方问的是"什么失败了"，而答案就是这几行，而不是"它们住在哪"
    // 的一条指引。
    // The typed failures carry the assertion's own words, addressed by each test's own name; the
    // rendered `why` lines are derived from them rather than from a second reading.
    // 类型化的失败携带断言原话，用每个测试自己的名字寻址；渲染的 `why` 行由它们派生，而不是再读一遍。
    let text_lines = text.lines().collect::<Vec<_>>();
    let failed_tests = failed
        .iter()
        .map(|(name, binary)| FailedTest {
            name: name.clone(),
            binary: binary.clone(),
            message: test_message(&text_lines, name),
        })
        .collect::<Vec<_>>();
    lines.extend(why_lines(
        &text,
        &failed_tests[..failed_tests.len().min(SAMPLE)],
    ));
    lines.extend(super::check_diagnostics::compiler_lines(&text, verbose));
    // The routing facts, read from the same text: a diagnostic's own location (rustc's `--> file:line`)
    // and whether the failing tests look like the coordinates family. Both are **observed**, never
    // guessed from the face's name.
    // 路由所需的事实，同样取自这段文本：诊断自己的位置（rustc 的 `--> file:line`），以及失败的测试是否
    // 像坐标族。两者都是**观察到的**，从不按面的名字去猜。
    let first_location = text.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix("--> ")?;
        let (path, _) = rest.rsplit_once(':')?;
        // `then` rather than `Some(..).filter(|_| ..)`: the predicate never looks at the value, and
        // clippy 1.99 says so (the filter form reads as if it did).
        // 用 `then` 而不是 `Some(..).filter(|_| ..)`：谓词根本不看那个值，clippy 1.99 正是这么说的
        // （filter 那种写法读起来像它会看）。
        (!path.is_empty()).then(|| rest.trim().to_owned())
    });
    lines.extend(
        text.lines()
            .filter(|line| line.starts_with("xirang-log-withheld:"))
            .map(str::to_owned),
    );
    Ok(Observation {
        lines,
        executed: facts.executed,
        skipped: facts.skipped,
        results: facts.results,
        failed: failed_tests,
        targets,
        target_labels,
        incomplete: facts.incomplete,
        first_location,
    })
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
