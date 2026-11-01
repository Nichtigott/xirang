//! The transport side of the worker protocol: read one listing, fetch one file in pieces, and
//! write a fetched file only after the whole file's digest checks out. Split from the client module
//! when the transfer logic outgrew the size ratchet; the facades still live there.
//! worker 协议的传输侧：读一份清单、分片取回一个文件，并且只在整份摘要核对通过后写盘。传输逻辑超出
//! 尺寸棘轮时从客户端模块拆出；门面仍留在那里。

use std::path::Path;

use serde_json::{Value, json};

use super::worker_client::{Control, send};
use super::worker_evidence::decode_hex;

/// The transfer root a worker says it read, checked against the one the job asked for. A worker
/// that does not name one is trusted only when the job asked for the default root: then its silence
/// describes exactly the behaviour this field did not exist for. Any other request refuses, because
/// a worker that ignores `root` would answer from `build` while the caller believes it read a
/// different subtree — the one failure mode the field must never hide.
/// worker 说它读了哪个回传根，与任务要求的那个核对。没有点名的 worker 只在任务要默认根时可信：那时它
/// 的沉默描述的正是这个字段存在之前的行为。其它请求一律拒绝，因为忽略 `root` 的 worker 会从 `build`
/// 回答、而调用方以为读的是另一棵子树——这是该字段绝不能掩盖的那一种失败。
pub(super) fn answered_root(object: &Value, asked: &str) -> Result<(), String> {
    match object.get("root").and_then(Value::as_str) {
        Some(answered) if answered == asked => Ok(()),
        Some(answered) => Err(format!(
            "the worker read transfer root `{answered}` while the job asked for `{asked}`"
        )),
        None if asked == super::worker::DEFAULT_ROOT => Ok(()),
        None => Err(format!(
            "the worker's answer names no transfer root; it does not carry `root`, so a job asking \
             for `{asked}` was not read from there"
        )),
    }
}

/// The size and sha256 of a file, read in pieces. Used to verify a fetched file against the
/// listing without holding it in memory.
/// 文件的大小与 sha256，按片读取。用来在不把文件放进内存的情况下，把取回的文件与清单核对。
fn sha256_of_file(file: &Path) -> Result<(u64, String), String> {
    use std::io::Read;
    let mut handle = std::fs::File::open(file)
        .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
    let mut hasher = xirang_kernel::Sha256::new();
    let mut total = 0u64;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = handle
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
    Ok((total, hasher.hex()))
}

/// Ask a worker to list one directory of its build root.
/// 要求 worker 列出它构建根下的一个目录。
pub(super) fn listing(
    root: &Path,
    command: &str,
    path: &str,
    transfer: &str,
    control: Control,
) -> Result<Vec<(String, u64, String)>, String> {
    let directory = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let job = json!({
        "protocol": 1,
        "action": "list",
        "directory": directory.display().to_string(),
        "locked": false,
        "path": path,
        "root": transfer,
    });
    let (object, _) = send(command, &job, "list", control)?;
    match object.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            answered_root(&object, transfer)?;
            let entries = object
                .get("entries")
                .and_then(Value::as_array)
                .ok_or("the worker listed nothing: the answer carries no `entries`")?;
            let mut files = Vec::new();
            for entry in entries {
                let relative = entry
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("a listing entry names no `path`")?;
                let bytes = entry
                    .get("bytes")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| format!("the listing entry `{relative}` names no `bytes`"))?;
                let sha256 = entry
                    .get("sha256")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("the listing entry `{relative}` names no `sha256`"))?;
                files.push((relative.to_owned(), bytes, sha256.to_owned()));
            }
            Ok(files)
        }
        Some(false) => Err(object
            .get("error")
            .and_then(Value::as_str)
            .ok_or("the worker failed to list but names no `error`")?
            .to_owned()),
        None => Err("the worker answered `list` without an `ok` flag".to_owned()),
    }
}

/// Fetch every file a listing names, preserving the paths below `into` and verifying each file's
/// sha256 both against the worker's answer and the listing's own digest.
/// 取回清单点名的每个文件，保留 `into` 下的相对路径，并对 worker 的应答与清单自身的摘要各校验一次。
pub(super) fn fetch_all(
    root: &Path,
    command: &str,
    path: &str,
    into: &Path,
    transfer: &str,
    control: Control,
) -> Result<String, String> {
    let files = listing(root, command, path, transfer, control)?;
    if files.is_empty() {
        return Err(format!(
            "`{path}` names no files in transfer root `{transfer}`"
        ));
    }
    let mut total = 0u64;
    for (relative, bytes, sha256) in &files {
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(format!(
                "the listing names `{relative}`, which is not a relative path inside transfer root `{transfer}`"
            ));
        }
        let destination_directory = match relative_path.parent() {
            Some(parent) => into.join(parent),
            None => into.to_path_buf(),
        };
        fetch_file(
            root,
            command,
            relative,
            &destination_directory,
            transfer,
            control,
        )?;
        let written = into.join(relative_path);
        let (written_bytes, actual) = sha256_of_file(&written)?;
        if written_bytes != *bytes {
            return Err(format!(
                "`{relative}` is {bytes} bytes on the worker and {written_bytes} bytes here"
            ));
        }
        if actual != *sha256 {
            return Err(format!(
                "`{relative}` hashes to {actual} here while the listing named {sha256}"
            ));
        }
        total += written_bytes;
    }
    Ok(format!(
        "remote fetch-all: {} file(s), {total} bytes -> {}",
        files.len(),
        into.display()
    ))
}

/// Fetch one file in pieces, writing each verified piece to a `.partial` file beside its
/// destination and renaming it only after the whole file's digest checks out. A failure removes
/// the partial file, so a refused or interrupted fetch leaves nothing behind — the property the
/// in-memory version had and that must survive streaming.
/// 分片取回一个文件，每片校验后写进目的地旁边的 `.partial` 文件，整份摘要核对通过才改名。失败会删掉
/// 临时文件，因此被拒绝或中断的取回什么都不留——这是内存版有、且必须保住的性质。
pub(super) fn fetch_file(
    root: &Path,
    command: &str,
    path: &str,
    into: &Path,
    transfer: &str,
    control: Control,
) -> Result<String, String> {
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("`{path}` has no file name"))?
        .to_owned();
    std::fs::create_dir_all(into)
        .map_err(|error| format!("cannot create {}: {error}", into.display()))?;
    let destination = into.join(&name);
    let partial = into.join(format!(".{name}.partial"));
    let attempt = fetch_into(
        root,
        command,
        path,
        transfer,
        control,
        &partial,
        &destination,
    );
    if attempt.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    attempt
}

/// The body of [`fetch_file`], returning the message it prints on success.
/// [`fetch_file`] 的主体，成功时返回它要打印的消息。
fn fetch_into(
    root: &Path,
    command: &str,
    path: &str,
    transfer: &str,
    control: Control,
    partial: &Path,
    destination: &Path,
) -> Result<String, String> {
    use std::io::Write;
    let directory = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let mut handle = std::fs::File::create(partial)
        .map_err(|error| format!("cannot write {}: {error}", partial.display()))?;
    let mut hasher = xirang_kernel::Sha256::new();
    let mut total: Option<u64> = None;
    let mut whole = String::new();
    let mut offset = 0u64;
    loop {
        let job = json!({
            "protocol": 1,
            "action": "fetch",
            "directory": directory.display().to_string(),
            "locked": false,
            "path": path,
            "offset": offset,
            "root": transfer,
        });
        let (object, _) = send(command, &job, "fetch", control)?;
        if object.get("ok").and_then(Value::as_bool) != Some(true) {
            let reason = object
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("the worker refused the fetch");
            return Err(format!("the worker refused to fetch `{path}`: {reason}"));
        }
        answered_root(&object, transfer)?;
        let piece_offset = object
            .get("offset")
            .and_then(Value::as_u64)
            .ok_or("the worker's fetch answer carries no `offset`")?;
        if piece_offset != offset {
            return Err(format!(
                "the worker answered offset {piece_offset} for a request at {offset}"
            ));
        }
        let length = object
            .get("length")
            .and_then(Value::as_u64)
            .ok_or("the worker's fetch answer carries no `length`")?;
        let file_total = object
            .get("total")
            .and_then(Value::as_u64)
            .ok_or("the worker's fetch answer carries no `total`")?;
        match total {
            Some(known) if known != file_total => {
                return Err(format!(
                    "the worker first said the file is {known} bytes and now says {file_total}"
                ));
            }
            None => total = Some(file_total),
            _ => {}
        }
        let sha256 = object
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or("the worker's fetch answer carries no piece `sha256`")?;
        let content = object
            .get("content")
            .and_then(Value::as_str)
            .ok_or("the worker's fetch answer carries no `content`")?;
        let bytes = decode_hex(content)
            .map_err(|error| format!("the worker's `content` is not hex: {error}"))?;
        if bytes.len() as u64 != length {
            return Err(format!(
                "the worker said the piece is {length} bytes and sent {}",
                bytes.len()
            ));
        }
        let actual = xirang_kernel::sha256_hex(&bytes);
        if actual != sha256 {
            return Err(format!(
                "a fetched piece hashes to {actual} while the worker named {sha256}"
            ));
        }
        if offset == 0 {
            whole = object
                .get("whole_sha256")
                .and_then(Value::as_str)
                .ok_or("the worker's first piece carries no whole-file `sha256`")?
                .to_owned();
        }
        handle
            .write_all(&bytes)
            .map_err(|error| format!("cannot write {}: {error}", partial.display()))?;
        hasher.update(&bytes);
        offset += length;
        if offset >= file_total {
            break;
        }
        if length == 0 {
            return Err("the worker sent an empty piece before the end of the file".to_owned());
        }
    }
    let total = total.unwrap_or(0);
    if offset != total {
        return Err(format!("the worker named {total} bytes and sent {offset}"));
    }
    handle
        .flush()
        .map_err(|error| format!("cannot write {}: {error}", partial.display()))?;
    drop(handle);
    let actual = hasher.hex();
    if actual != whole {
        return Err(format!(
            "the fetched bytes hash to {actual} while the worker named {whole}; nothing was written"
        ));
    }
    std::fs::rename(partial, destination)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    Ok(format!(
        "remote fetch: {path} ({total} bytes, sha256 {whole}) -> {}",
        destination.display()
    ))
}
