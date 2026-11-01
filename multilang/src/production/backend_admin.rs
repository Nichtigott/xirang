//! Backend administration: the user-level record's install, upgrade and removal, and the report
//! of what a checkout registers. The rules live in `record`, `delegate` and `backends`; these are
//! the thin entry points the CLI and the scaffold share.
//! 后端管理：用户级记录的安装、升级与移除，以及本检出注册内容的报告。规则住在 `record`、
//! `delegate` 与 `backends`；这里是 CLI 与脚手架共用的薄入口。

use std::path::Path;

/// The handshake report: which languages this checkout registers, what each backend declares, and
/// which backends this checkout's record only declares.
/// 握手报告：本检出注册哪些语言、每个后端声明了什么，以及本检出记录里只是声明的后端。
pub fn languages() -> Result<String, String> {
    super::backends::report(Path::new("."))
}

/// A backend installation: what `xirang backends install` records.
/// 一次后端安装：`xirang backends install` 要记录的内容。
pub struct BackendInstall {
    /// The language the backend answers for.
    /// 该后端负责的语言。
    pub language: String,
    /// The program that carries it out.
    /// 执行它的程序。
    pub backend: String,
    /// The argv a delegated plan is sent to.
    /// 委派计划会发送到的 argv。
    pub command: Vec<String>,
    /// The abilities the installation claims, by word.
    /// 安装声明具备的能力，按能力词给出。
    pub capabilities: Vec<String>,
    /// Why an ability is not claimed, by word.
    /// 某项能力为何没有声明，按能力词给出。
    pub reasons: Vec<(String, String)>,
}

/// Install one backend into the user-level record, after its process answers a handshake.
/// 在进程回答一次握手之后，把一个后端安装进用户级记录。
///
/// The probe comes first for the same reason the delegation does: a record is the project's
/// choice, and the process is the fact. Installing without asking would write a file that looks
/// like an executor and can never answer one request.
/// 先探测的理由与委派相同：记录是项目的选择，进程才是事实。不问就装，写下的文件会看起来像执行器，
/// 却永远答不了一次请求。
pub fn install_backend(root: &Path, install: BackendInstall) -> Result<String, String> {
    let draft = super::record::Draft {
        language: install.language,
        backend: install.backend,
        command: install.command,
        capabilities: install.capabilities,
        reasons: install.reasons,
    };
    let entry = super::record::draft(&draft)?;
    let argv = entry.command.clone().unwrap_or_default();
    let report = super::delegate::handshake(root, &argv, &entry.language, false)?;
    let installed = super::record::append(entry)?;
    let path = installed.path;
    let reported = report
        .capabilities
        .iter()
        .map(|capability| capability.name())
        .collect::<Vec<_>>()
        .join(", ");
    let replaced = installed
        .replaced
        .map(|old| format!("\n  it replaces `{old}`, whose entry this installation overwrote"))
        .unwrap_or_default();
    Ok(format!(
        "installed `{}` as `{}` in {}\n  it answered a handshake as `{}` with: {reported}{replaced}",
        draft.language,
        draft.backend,
        path.display(),
        report.backend
    ))
}

/// Remove one language's recorded backend from the user record.
/// 从用户记录里移除一门语言已记录的后端。
pub fn remove_backend(language: &str) -> Result<String, String> {
    let (path, backend, remaining) = super::record::remove(language)?;
    let left = if remaining == 1 {
        "1 entry".to_owned()
    } else {
        format!("{remaining} entries")
    };
    Ok(format!(
        "removed the `{language}` entry (`{backend}`) from {}\n  the record now holds {left}",
        path.display()
    ))
}
