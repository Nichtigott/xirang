//! `xirang install` / `xirang rollback`: place a packaged delivery under a prefix as immutable
//! generations, verifying every file's bytes against the manifest before anything is written, and
//! keep the previous generation so a bad delivery can be taken back without rebuilding.
//! `xirang install` / `xirang rollback`：把打包好的交付按不可变世代安放到前缀下；写入任何东西之前先
//! 逐文件按清单校验字节，并保留上一代，使坏交付不必重建就能退回。

use std::path::{Path, PathBuf};

/// The history file that names the current and previous generations.
/// 记录当前与上一代的历史文件。
const HISTORY: &str = "history.json";

/// Verify the package at `package` and return its manifest, refusing the first mismatch by name.
/// 校验 `package` 处的包并返回清单；第一处不符就按名拒绝。
fn verified(package: &Path) -> Result<serde_json::Value, String> {
    let path = package.join("manifest.json");
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "`{}` is not a package (no readable manifest.json: {error})",
            package.display()
        )
    })?;
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("`{}` is not readable JSON: {error}", path.display()))?;
    if manifest["protocol"] != 1 {
        return Err(format!(
            "`{}` speaks package protocol {}; this build reads protocol 1",
            path.display(),
            manifest["protocol"]
        ));
    }
    for file in manifest["files"]
        .as_array()
        .ok_or_else(|| format!("`{}` has no `files` list", path.display()))?
    {
        let relative = file["path"]
            .as_str()
            .ok_or_else(|| format!("`{}` has a file entry without `path`", path.display()))?;
        let wanted = file["sha256"]
            .as_str()
            .ok_or_else(|| format!("`{relative}` has no `sha256` in the manifest"))?;
        let bytes = std::fs::read(package.join(relative))
            .map_err(|error| format!("`{relative}` is missing from the package: {error}"))?;
        let found = xirang_kernel::sha256_hex(&bytes);
        if found != wanted {
            return Err(format!(
                "`{relative}` does not match the manifest (sha256 {found}, manifest says {wanted}); nothing was installed"
            ));
        }
    }
    Ok(manifest)
}

/// Read the history file; a missing or unreadable one means no generation has been installed yet.
/// 读历史文件；缺失或读不懂意味着还没有安装过任何世代。
fn history(prefix: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(prefix.join(HISTORY)) else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (None, None);
    };
    (
        value["current"].as_str().map(str::to_owned),
        value["previous"].as_str().map(str::to_owned),
    )
}

/// Point `prefix/current` at `generation` with one rename, so a reader never sees a half state.
/// 用一次改名把 `prefix/current` 指向 `generation`，读取方看不到中间态。
fn point(prefix: &Path, generation: &str) -> Result<(), String> {
    let current = prefix.join("current");
    let link = prefix.join(format!(".current-{generation}"));
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(generation, &link).map_err(|error| {
        format!(
            "cannot point {} at {generation}: {error}",
            current.display()
        )
    })?;
    std::fs::rename(&link, &current)
        .map_err(|error| format!("cannot replace {}: {error}", current.display()))?;
    Ok(())
}

/// Write the history file next to the generations, atomically.
/// 在世代目录旁原子地写历史文件。
fn remember(prefix: &Path, current: &str, previous: Option<&str>) -> Result<(), String> {
    let value = serde_json::json!({"current": current, "previous": previous});
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("cannot render the history: {error}"))?;
    let path = prefix.join(HISTORY);
    let temporary = prefix.join(format!("{HISTORY}.tmp"));
    std::fs::write(&temporary, text)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))
}

/// Install the package at `package` under `prefix` as a new generation and make it current.
/// 把 `package` 的包装到 `prefix` 下成为一个新世代并设为当前。
pub(crate) fn install(prefix: &Path, package: &Path) -> Result<String, String> {
    let manifest = verified(package)?;
    // A chain that does not verify must stop the install before anything is written: appending to
    // it would make the damage look like history. This is checked here, ahead of the first write.
    // 链本身不通过时必须在写入任何东西之前停下安装：往坏链上追加等于让损伤看起来像历史。这一步就在
    // 第一次写入之前查。
    super::log::chain(prefix)?;
    let generation = manifest["plan"]
        .as_str()
        .ok_or_else(|| "the package names no plan".to_owned())?;
    std::fs::create_dir_all(prefix)
        .map_err(|error| format!("cannot create {}: {error}", prefix.display()))?;
    let (current, _) = history(prefix);
    if current.as_deref() == Some(generation) {
        return Ok(format!(
            "generation {generation} is already current under {}\n",
            prefix.display()
        ));
    }
    let directory = prefix.join(generation);
    let mut files = 0u64;
    let mut bytes = 0u64;
    for file in manifest["files"].as_array().expect("verified files") {
        let relative = file["path"].as_str().expect("verified path");
        let source = package.join(relative);
        let target = directory.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let data = std::fs::read(&source)
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
        std::fs::write(&target, &data)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        files += 1;
        bytes += data.len() as u64;
    }
    std::fs::copy(
        package.join("manifest.json"),
        directory.join("manifest.json"),
    )
    .map_err(|error| format!("cannot copy the manifest: {error}"))?;
    point(prefix, generation)?;
    remember(prefix, generation, current.as_deref())?;
    // The state change has happened; only now is it recorded, so the log never predicts a change
    // that did not happen. If the append fails, say what is nevertheless true.
    // 状态变化已经发生，到这一步才记录它，因此日志从不预言一次没有发生的变化。追加失败时，把仍然成立的
    // 事实说出来。
    let package_digest = std::fs::read(package.join("manifest.json"))
        .map(|bytes| xirang_kernel::sha256_hex(&bytes))
        .map_err(|error| format!("cannot read the manifest back for the log: {error}"))?;
    let entry = super::log::record(prefix, "install", generation, Some(&package_digest)).map_err(
        |error| {
            format!(
                "generation {generation} is current under {} and `history.json` says so, but its log entry could not be appended: {error}; `xirang log verify` will report the gap",
                prefix.display()
            )
        },
    )?;
    Ok(format!(
        "installed generation {generation} under {}  ({files} file(s), {bytes} bytes)\ncurrent -> {generation}\nlog -> entry {} {}\n",
        prefix.display(),
        entry.seq,
        entry.entry
    ))
}

/// Make the previous generation current again, verifying its bytes first.
/// 让上一代重新成为当前，先校验它的字节。
pub(crate) fn rollback(prefix: &Path) -> Result<String, String> {
    super::log::chain(prefix)?;
    let (current, previous) = history(prefix);
    let previous = previous.ok_or_else(|| {
        format!(
            "nothing to roll back to under {}: {}",
            prefix.display(),
            match &current {
                Some(current) => format!("generation {current} is the only one installed"),
                None => "no generation has been installed".to_owned(),
            }
        )
    })?;
    let directory = prefix.join(&previous);
    verified(&directory)?;
    point(prefix, &previous)?;
    remember(prefix, &previous, current.as_deref())?;
    let entry = super::log::record(prefix, "rollback", &previous, None).map_err(|error| {
        format!(
            "generation {previous} is current under {} and `history.json` says so, but its log entry could not be appended: {error}; `xirang log verify` will report the gap",
            prefix.display()
        )
    })?;
    Ok(format!(
        "rolled back to generation {previous} under {}\ncurrent -> {previous}\nlog -> entry {} {}\n",
        prefix.display(),
        entry.seq,
        entry.entry
    ))
}

/// The prefix as an absolute path when it is relative, so the message names what a reader sees.
/// 把相对前缀变成绝对路径，使消息点名的是读取方看到的位置。
pub(crate) fn absolute(prefix: &Path) -> PathBuf {
    if prefix.is_absolute() {
        prefix.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(prefix))
            .unwrap_or_else(|_| prefix.to_path_buf())
    }
}

/// Verify one installed generation against the manifest it carries, returning its file and byte
/// counts. A generation is only trustworthy while its bytes still hash to the manifest.
/// 按世代自带的清单校验一个已安装的世代，返回文件数与字节数。只有字节仍与清单相符时该世代才可信。
fn verify_generation(directory: &Path) -> Result<(usize, u64), String> {
    let path = directory.join("manifest.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("no readable manifest.json: {error}"))?;
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("manifest.json is not readable JSON: {error}"))?;
    let files = manifest["files"]
        .as_array()
        .ok_or_else(|| "manifest.json has no `files` list".to_owned())?;
    let mut bytes = 0u64;
    for file in files {
        let relative = file["path"]
            .as_str()
            .ok_or_else(|| "a file entry has no `path`".to_owned())?;
        let wanted = file["sha256"]
            .as_str()
            .ok_or_else(|| format!("`{relative}` has no `sha256` in the manifest"))?;
        let data = std::fs::read(directory.join(relative))
            .map_err(|error| format!("`{relative}` is missing from the generation: {error}"))?;
        let found = xirang_kernel::sha256_hex(&data);
        if found != wanted {
            return Err(format!(
                "`{relative}` does not match its manifest (sha256 {found}, manifest says {wanted})"
            ));
        }
        bytes += data.len() as u64;
    }
    Ok((files.len(), bytes))
}

/// Report what a prefix holds: the current and previous generations, whether each still matches
/// its manifest, and whether the `current` link agrees with the history file. Read-only.
/// 报告一个前缀里有什么：当前与上一代、每一代是否仍与清单相符、`current` 链接是否与历史文件一致。
/// 只读。
pub(crate) fn diagnose(prefix: &Path) -> Result<String, String> {
    let (current, previous) = history(prefix);
    let Some(current) = current else {
        return Err(format!(
            "nothing has been installed under {}; `xirang history` reads a prefix that has a history file",
            prefix.display()
        ));
    };
    let mut out = format!("prefix {}\n", prefix.display());
    let link = std::fs::read_link(prefix.join("current")).ok();
    match &link {
        Some(target) if target == Path::new(&current) => {
            out.push_str(&format!(
                "  current {current} (the `current` link agrees)\n"
            ));
        }
        Some(target) => out.push_str(&format!(
            "  current {current} — WARNING: the `current` link points at {}\n",
            target.display()
        )),
        None => out.push_str(&format!(
            "  current {current} — WARNING: there is no readable `current` link\n"
        )),
    }
    let mut problems = 0usize;
    let mut current_ok = false;
    let mut previous_ok = false;
    for (role, generation) in [("current", Some(&current)), ("previous", previous.as_ref())] {
        let Some(generation) = generation else {
            continue;
        };
        let directory = prefix.join(generation);
        match verify_generation(&directory) {
            Ok((files, bytes)) => {
                match role {
                    "current" => current_ok = true,
                    _ => previous_ok = true,
                }
                out.push_str(&format!(
                    "  {role} generation {generation}: verified ({files} file(s), {bytes} bytes)\n"
                ));
            }
            Err(error) => {
                problems += 1;
                out.push_str(&format!(
                    "  {role} generation {generation}: NOT verified: {error}\n"
                ));
            }
        }
    }
    if problems > 0 {
        // A diagnosis that only says "not verified" leaves the caller to invent the repair. Name
        // the one step that is safe from what this prefix actually holds: roll back only when the
        // previous generation verifies; otherwise nothing here can be trusted, so a fresh install
        // is the only path.
        // 只说“未通过校验”的诊断会把修复办法留给调用方去猜。这里按前缀实际持有的东西点名唯一安全的一步：
        // 只有上一代通过校验时才回退；否则这里没有任何可信任的世代，重新安装是唯一出路。
        if current_ok {
            out.push_str(&format!(
                "\n  next: generation {current} verifies; a rollback would restore unverified bytes, so leave `current` alone or reinstall\n"
            ));
        } else if previous_ok {
            out.push_str(&format!(
                "\n  next: xirang rollback --prefix {}  (it restores generation {}, which verifies)\n",
                prefix.display(),
                previous.unwrap_or_default()
            ));
        } else {
            out.push_str(&format!(
                "\n  next: no generation here verifies; install a verified package: xirang install <package> --prefix {}\n",
                prefix.display()
            ));
        }
        return Err(out.trim_end().to_owned());
    }
    Ok(out.trim_end().to_owned())
}
