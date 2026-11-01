//! The dimensions one verification run's evidence is bound to, observed once per plan.
//! 一次验证运行的证据所绑定的维度，每个计划观测一次。
//!
//! Split out of `check_plan` because these facts are not the plan's: the toolchain is read from the
//! machine, the profile and feature face come from a request, and the identity they are folded into is
//! the same value `resume` re-derives before it keeps a pass. Keeping the fold next to the plan would
//! have made the reader of a *stored* ledger the only one who needs it, which is exactly backwards.
//! 从 `check_plan` 拆出来，因为这些事实不属于计划：工具链读自这台机器，profile 与特性面来自请求，而它们折叠
//! 成的身份，正是 `resume` 在保留一次通过之前重新推导的那个值。把这个折叠留在计划旁边，会让**读存量台账的人**
//! 成了唯一需要它的人，而这正好是反的。

use std::path::Path;

use serde_json::Value;
pub(super) use xirang_kernel::maintenance::ExecutionIdentity;

/// The accepted policy one plan's verdicts are judged under, recorded so a later reuse can tell an
/// old verdict from one produced under a policy that has since changed (§6.3, condition 1).
/// 一份计划的判定所依据的、已接受的政策；记录下来，使之后的复用分得出旧判定与在已变更政策下产出的判定
/// （§6.3 第 1 条）。
pub(super) const VERIFICATION_POLICY: &str = "xirang.repair.obligations/v1";

/// The toolchain facts a plan binds its evidence to, observed once per plan rather than per
/// obligation: `rustc -vV` is a process, and running it before every obligation would make the
/// executor pay for a fact that cannot change inside one plan.
/// 计划把证据绑定到的工具链事实，**每个计划观测一次**而不是每条义务一次：`rustc -vV` 是一个进程，在每条
/// 义务前跑它，等于让执行器为一个在一次计划内不可能改变的事实付费。
pub(super) struct Environment {
    /// Compiler release and host triple, or the reason neither could be read.
    /// 编译器版本与宿主三元组，或两者都读不到的原因。
    pub(super) toolchain: String,
    /// The host triple `rustc` reports, used when the request names no `target`.
    /// `rustc` 报告的宿主三元组；请求没有点名 `target` 时用它。
    pub(super) host: String,
    /// The tool that issues the credential, with its own version.
    /// 签发凭据的工具，附它自己的版本。
    pub(super) checker: String,
}

impl Environment {
    /// Read the toolchain facts once, recording a failure as a stated unknown.
    /// 只读一次工具链事实，读不到就记录成明说的 unknown。
    pub(super) fn observe() -> Self {
        // `unknown` is a value, not an omission: an identity that dropped the dimension would
        // compare equal to another identity that also dropped it, and the two runs would be merged
        // on a fact neither of them established.
        // `unknown` 是一个取值，不是省略：丢掉这个维度的身份会与另一个也丢掉它的身份比较相等，于是两次
        // 运行在一个两边都没确立的事实上被合并。
        let observed = std::process::Command::new("rustc")
            .arg("-vV")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
        let field = |name: &str| {
            observed
                .as_deref()?
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        };
        let release = field("release:").unwrap_or_else(|| "unknown".to_owned());
        let host = field("host:").unwrap_or_else(|| "unknown".to_owned());
        let cargo = std::process::Command::new("cargo")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        Self {
            toolchain: format!(
                "rustc {release}; {cargo}{}",
                if observed.is_none() {
                    " (rustc -vV could not be read)"
                } else {
                    ""
                }
            ),
            host,
            checker: format!("xirang-toolchain {}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// One obligation's execution, with every dimension the identity requires.
/// 一条义务的执行，带齐身份要求的每个维度。
///
/// `isolation` renders the resolved build directory, so two plans that share a source tree but not
/// an output directory are two executions: artifacts in one cannot be read as evidence about the
/// other.
/// `isolation` 渲染解析后的构建目录，因此共用源码树但不共用输出目录的两份计划是两次执行：一个目录里的产物
/// 不能读成关于另一个的证据。
pub(super) fn execution_identity(
    request: &Value,
    candidate: &str,
    environment: &Environment,
    root: &Path,
) -> Result<ExecutionIdentity, String> {
    let text = |key: &str| {
        request
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let flag = |key: &str| request.get(key).and_then(Value::as_bool) == Some(true);
    let selection = format!(
        "workspace={}; package={}; test={}; filter={}; all_targets={}; deny_warnings={}",
        flag("workspace"),
        text("package").unwrap_or("none"),
        text("test").unwrap_or("none"),
        text("filter").unwrap_or("none"),
        flag("all_targets"),
        flag("deny_warnings"),
    );
    let isolation = text("target_dir")
        .map(|directory| {
            Path::new(directory).strip_prefix(root).map_or_else(
                |_| directory.to_owned(),
                |rest| rest.to_string_lossy().replace('\\', "/"),
            )
        })
        .unwrap_or_else(|| "in-tree (cargo default)".to_owned());
    ExecutionIdentity::of([
        ("candidate", candidate.to_owned()),
        ("toolchain", environment.toolchain.clone()),
        (
            "target",
            text("target").unwrap_or(&environment.host).to_owned(),
        ),
        ("profile", text("profile").unwrap_or("debug").to_owned()),
        ("features", text("face").unwrap_or("default").to_owned()),
        (
            "no_default_features",
            flag("no_default_features").to_string(),
        ),
        ("selection", selection),
        ("mode", text("mode").unwrap_or("test").to_owned()),
        ("checker", environment.checker.clone()),
        ("isolation", isolation),
    ])
}

#[cfg(test)]
#[path = "check_execution_tests.rs"]
mod check_execution_tests;
