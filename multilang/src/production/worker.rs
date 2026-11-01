//! The remote-worker protocol: one job in, one result out, both JSON. The transport is any command
//! that reads the job on stdin and writes the result on stdout — `ssh host xirang worker` today's
//! example, a VM guest agent later. This slice ships the protocol and the command transport; the
//! SSH/KVM transports and the image pipeline are not implemented.
//! 远程 worker 协议：一份任务进、一份结果出，都是 JSON。传输就是任何“从 stdin 读任务、往 stdout
//! 写结果”的命令——今天例如 `ssh host xirang worker`，以后是虚拟机内的 guest agent。本切片交付协议与
//! 命令传输；SSH/KVM 传输与镜像流水线未实现。

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// One job a worker is asked to run.
/// worker 被要求运行的一份任务。
pub(super) struct Job {
    pub(super) action: String,
    pub(super) directory: PathBuf,
    pub(super) locked: bool,
    pub(super) path: Option<String>,
    pub(super) offset: Option<u64>,
    pub(super) length: Option<u64>,
    pub(super) root: Option<String>,
}

/// Parse one job document, refusing unknown or missing fields by name.
/// 解析一份任务文档，未知或缺失字段按名拒绝。
pub(super) fn job(text: &str) -> Result<Job, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("the job is not JSON: {error}"))?;
    let object = value.as_object().ok_or("a worker job is a JSON object")?;
    for key in object.keys() {
        if ![
            "protocol",
            "action",
            "directory",
            "locked",
            "path",
            "offset",
            "length",
            "root",
        ]
        .contains(&key.as_str())
        {
            return Err(format!(
                "unknown job key `{key}`; a job carries protocol, action, directory, locked, path, offset, length and root"
            ));
        }
    }
    match object.get("protocol") {
        Some(Value::Number(number)) if number.as_u64() == Some(1) => {}
        Some(other) => {
            return Err(format!(
                "this worker speaks protocol 1; the job says {other}"
            ));
        }
        None => return Err("the job has no `protocol`; this worker speaks protocol 1".to_owned()),
    }
    let action = object
        .get("action")
        .and_then(Value::as_str)
        .ok_or("the job has no string `action`")?;
    if action != "build"
        && action != "test"
        && action != "evidence"
        && action != "fetch"
        && action != "list"
    {
        return Err(format!(
            "`{action}` is not an action this worker runs; it runs `build`, `test`, `evidence`, \
             `fetch` and `list`"
        ));
    }
    let directory = object
        .get("directory")
        .and_then(Value::as_str)
        .ok_or("the job has no string `directory`")?;
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        return Err(format!(
            "the job names `{}`, which is not an absolute directory; a worker does not guess the root",
            directory.display()
        ));
    }
    let locked = match object.get("locked") {
        None => false,
        Some(Value::Bool(locked)) => *locked,
        Some(other) => {
            return Err(format!(
                "the job has `locked = {other}`; it must be `true` or `false`"
            ));
        }
    };
    let path = match object.get("path") {
        None => None,
        Some(Value::String(path)) => Some(path.clone()),
        Some(other) => {
            return Err(format!("the job has `path = {other}`; it must be a string"));
        }
    };
    if (action == "fetch" || action == "list") && path.is_none() {
        return Err(format!("a `{action}` job names what it wants in `path`"));
    }
    let offset = match object.get("offset") {
        None => None,
        Some(Value::Number(number)) => Some(
            number
                .as_u64()
                .ok_or("the job's `offset` must be a non-negative whole number")?,
        ),
        Some(other) => {
            return Err(format!(
                "the job has `offset = {other}`; it must be a whole number"
            ));
        }
    };
    let length = match object.get("length") {
        None => None,
        Some(Value::Number(number)) => Some(
            number
                .as_u64()
                .ok_or("the job's `length` must be a non-negative whole number")?,
        ),
        Some(other) => {
            return Err(format!(
                "the job has `length = {other}`; it must be a whole number"
            ));
        }
    };
    if action != "fetch" && (offset.is_some() || length.is_some()) {
        return Err("`offset` and `length` only mean something for a `fetch` job".to_owned());
    }
    if length == Some(0) {
        return Err("the job's `length` must be at least one byte".to_owned());
    }
    let root = match object.get("root") {
        None => None,
        Some(Value::String(root)) => {
            if root.is_empty() {
                return Err(
                    "the job's `root` is empty; it names a directory under `target/xirang`"
                        .to_owned(),
                );
            }
            Some(root.clone())
        }
        Some(other) => {
            return Err(format!("the job has `root = {other}`; it must be a string"));
        }
    };
    if root.is_some() && action != "fetch" && action != "list" {
        return Err("`root` only means something for a `fetch` or `list` job".to_owned());
    }
    Ok(Job {
        action: action.to_owned(),
        directory,
        locked,
        path,
        offset,
        length,
        root,
    })
}

/// The largest piece one fetch result carries. The content travels as hex inside JSON, so a larger
/// file is split into pieces and each piece carries its own sha256.
/// 一次 fetch 结果最多携带多大一片。内容以十六进制放在 JSON 里，因此更大的文件按片返回，每片带自己的
/// sha256。
const CHUNK_LIMIT: u64 = 4 * 1024 * 1024;

/// The most files one listing carries. A listing names every file under its directory, so this is
/// the bound that keeps a `--fetch-all` plan finite.
/// 一份清单最多携带的文件数。清单会点名其目录下每个文件，因此这个上限让 `--fetch-all` 的计划有限。
const LIST_LIMIT: usize = 256;
/// The transfer roots live under this directory of a project; `root` names one of its
/// subdirectories, so the default `build` is `target/xirang/build`.
/// 回传根住在项目的这个目录下；`root` 点名它的一个子目录，因此默认的 `build` 就是
/// `target/xirang/build`。
const TRANSFER_PARENT: &str = "target/xirang";

/// The transfer root a job means when it names none. Naming it keeps old clients working.
/// 任务没有点名时所用的回传根。点名的默认值让旧客户端继续可用。
pub(super) const DEFAULT_ROOT: &str = "build";

/// Resolve the transfer root `root` under `directory`, refusing anything that leaves
/// `target/xirang`. The returned spelling is what messages print; the canonical path is what the
/// containment check and every read use.
/// 解析 `directory` 下名为 `root` 的回传根，拒绝任何离开 `target/xirang` 的值。返回的拼写供消息使用；
/// 包含检查与每次读取用的是规范化路径。
fn transfer_root(directory: &Path, root: &str) -> Result<PathBuf, String> {
    let spelling = directory.join(TRANSFER_PARENT).join(root);
    let relative = Path::new(root);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "`{root}` must be a relative path under `{TRANSFER_PARENT}` and must not contain `..`"
        ));
    }
    let parent = directory.join(TRANSFER_PARENT);
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("`{}` holds no transfers: {error}", parent.display()))?;
    let canonical = spelling
        .canonicalize()
        .map_err(|error| format!("the transfer root has no `{root}`: {error}"))?;
    if !canonical.starts_with(&parent) {
        return Err(format!(
            "`{root}` resolves outside `{}`; this transport reads one project's transfers only",
            parent.display()
        ));
    }
    if !canonical.is_dir() {
        return Err(format!(
            "`{root}` is not a directory under `{TRANSFER_PARENT}`; a transfer root is a directory"
        ));
    }
    Ok(canonical)
}

/// The `path` a job names, checked as a relative path that cannot leave its root. The shape is
/// checked before any directory is read, so an escape is refused by name even when the root does
/// not exist yet.
/// 任务点名的 `path`，按“不会离开其根的相对路径”检查。形状先于任何目录读取被检查，因此即使根还不
/// 存在，越界也会按名被拒绝。
fn relative_under(spelling: &Path, path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!(
            "`{path}` must be relative to `{}` and must not contain `..`",
            spelling.display()
        ));
    }
    Ok(relative.to_path_buf())
}

/// The sha256 of a whole file, read in pieces so a file larger than memory still hashes.
/// 整份文件的 sha256，按片读取，因此比内存更大的文件也算得出来。
fn whole_sha256(file: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut handle = std::fs::File::open(file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let mut hasher = xirang_kernel::Sha256::new();
    let mut buffer = vec![0_u8; CHUNK_LIMIT as usize];
    loop {
        let read = handle
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.hex())
}

/// Read one piece of a file under the transfer root `root`. The path is relative to that root and
/// must stay inside it; the answer carries the piece, its sha256, the file's total size, the root
/// the worker actually read, and — for the first piece — the whole file's sha256.
/// 读取回传根 `root` 下一个文件的一片。路径相对该根且必须留在根内；应答带这一片、它的 sha256、
/// 文件总大小、worker 实际读取的根，以及（仅首片）整个文件的 sha256。
fn fetch(
    directory: &Path,
    root: &str,
    path: &str,
    offset: u64,
    length: u64,
) -> Result<String, String> {
    let spelling = directory.join(TRANSFER_PARENT).join(root);
    let relative = relative_under(&spelling, path)?;
    let builds = transfer_root(directory, root)?;
    let file = builds
        .join(&relative)
        .canonicalize()
        .map_err(|error| format!("the transfer root has no `{path}`: {error}"))?;
    if !file.starts_with(&builds) {
        return Err(format!(
            "`{path}` resolves outside {}; this transport only reads what the root holds",
            spelling.display()
        ));
    }
    let metadata = std::fs::metadata(&file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a file", file.display()));
    }
    let total = metadata.len();
    if offset > total {
        return Err(format!(
            "`{path}` is {total} bytes; offset {offset} is past its end"
        ));
    }
    let take = length.min(CHUNK_LIMIT).min(total - offset);
    use std::io::{Read, Seek, SeekFrom};
    let mut handle = std::fs::File::open(&file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    handle
        .seek(SeekFrom::Start(offset))
        .map_err(|error| format!("cannot seek {}: {error}", file.display()))?;
    let mut piece = vec![0u8; take as usize];
    handle
        .read_exact(&mut piece)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = Vec::with_capacity(piece.len() * 2);
    for byte in &piece {
        hex.push(DIGITS[(byte >> 4) as usize]);
        hex.push(DIGITS[(byte & 0x0f) as usize]);
    }
    let content =
        String::from_utf8(hex).map_err(|_| "the hex encoder produced a non-ASCII byte")?;
    let mut document = json!({
        "protocol": 1,
        "action": "fetch",
        "ok": true,
        "root": root,
        "path": file.display().to_string(),
        "offset": offset,
        "length": take,
        "total": total,
        "sha256": xirang_kernel::sha256_hex(&piece),
        "content": content,
    });
    if offset == 0 {
        document["whole_sha256"] = json!(whole_sha256(&file)?);
    }
    Ok(document.to_string())
}

/// List every file under one directory of `<directory>/target/xirang/build` (or the one file a
/// path names), with each file's size and sha256.
/// 列出 `<directory>/target/xirang/build` 下某个目录里的每个文件（或某个路径点名的单个文件），带大小与
/// sha256。
fn list(directory: &Path, root: &str, path: &str) -> Result<String, String> {
    let spelling = directory.join(TRANSFER_PARENT).join(root);
    let relative = relative_under(&spelling, path)?;
    let builds = transfer_root(directory, root)?;
    let target = builds
        .join(&relative)
        .canonicalize()
        .map_err(|error| format!("the transfer root has no `{path}`: {error}"))?;
    if !target.starts_with(&builds) {
        return Err(format!(
            "`{path}` resolves outside {}; this transport only reads what the root holds",
            spelling.display()
        ));
    }
    let mut files = Vec::new();
    if target.is_dir() {
        visit_files(&target, &mut files)?;
    } else {
        files.push(target.clone());
    }
    if files.len() > LIST_LIMIT {
        return Err(format!(
            "`{path}` holds {} files; a listing carries at most {LIST_LIMIT} files",
            files.len()
        ));
    }
    let mut entries = Vec::new();
    for file in &files {
        let bytes = std::fs::metadata(file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?
            .len();
        let relative = file
            .strip_prefix(&builds)
            .map_err(|_| format!("{} is not under the transfer root", file.display()))?;
        entries.push(json!({
            "path": relative.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"),
            "bytes": bytes,
            "sha256": whole_sha256(file)?,
        }));
    }
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    Ok(json!({
        "protocol": 1,
        "action": "list",
        "ok": true,
        "root": root,
        "entries": entries,
    })
    .to_string())
}

/// Collect every regular file under `directory` into `files`, depth first.
/// 递归收集 `directory` 下的每个普通文件到 `files`。
fn visit_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// Run one job from its document and render the result.
/// 从任务文档运行一份任务并渲染结果。
pub fn worker_job(text: &str) -> Result<String, String> {
    let job = job(text)?;
    Ok(execute(&job))
}

/// Run one job and render the result document.
/// 运行一份任务并渲染结果文档。
pub(super) fn execute(job: &Job) -> String {
    let root = job.root.as_deref().unwrap_or(DEFAULT_ROOT);
    if job.action == "list" {
        return match list(
            &job.directory,
            root,
            job.path.as_deref().unwrap_or_default(),
        ) {
            Ok(document) => document,
            Err(error) => json!({
                "protocol": 1,
                "action": "list",
                "ok": false,
                "error": error,
            })
            .to_string(),
        };
    }
    if job.action == "fetch" {
        return match fetch(
            &job.directory,
            root,
            job.path.as_deref().unwrap_or_default(),
            job.offset.unwrap_or(0),
            job.length.unwrap_or(CHUNK_LIMIT),
        ) {
            Ok(document) => document,
            Err(error) => json!({
                "protocol": 1,
                "action": "fetch",
                "ok": false,
                "error": error,
            })
            .to_string(),
        };
    }
    if job.action == "evidence" {
        return match evidence(&job.directory) {
            Ok(text) => json!({
                "protocol": 1,
                "action": "evidence",
                "ok": true,
                "evidence": text,
            })
            .to_string(),
            Err(error) => json!({
                "protocol": 1,
                "action": "evidence",
                "ok": false,
                "error": error,
            })
            .to_string(),
        };
    }
    let outcome = if job.action == "test" {
        crate::production::test(&job.directory, job.locked)
    } else {
        crate::production::build_project(&job.directory, job.locked)
    };
    match outcome {
        Ok(summary) => json!({
            "protocol": 1,
            "action": job.action,
            "ok": true,
            "summary": summary,
        })
        .to_string(),
        Err(error) => json!({
            "protocol": 1,
            "action": job.action,
            "ok": false,
            "error": error,
        })
        .to_string(),
    }
}

/// The newest build evidence a project directory holds, as text.
/// 一个项目目录里最新的一次构建证据（文本）。
fn evidence(directory: &Path) -> Result<String, String> {
    let builds = directory.join("target/xirang/build");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(&builds) {
        for entry in entries.flatten() {
            let path = entry.path().join("evidence.json");
            if !path.is_file() {
                continue;
            }
            let Ok(modified) = std::fs::metadata(&path).and_then(|metadata| metadata.modified())
            else {
                continue;
            };
            if newest.as_ref().is_none_or(|(seen, _)| modified > *seen) {
                newest = Some((modified, path));
            }
        }
    }
    let Some((_, path)) = newest else {
        return Err(format!(
            "{} has no build evidence; run `xirang remote --worker … build` (or `test`) first",
            directory.display()
        ));
    };
    std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}
