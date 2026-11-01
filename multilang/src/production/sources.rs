//! Resolving a named `[sources]` entry to a directory: a path stays a path, a git source is
//! cloned once into a content-addressed cache under `target/xirang/sources/` and checked out at
//! the exact revision the manifest names. The resolved commit becomes part of the unit's origin,
//! which is what makes a changed commit a different plan and a `--locked` build refuse it.
//! 把具名 `[sources]` 条目解析成一个目录：路径仍是路径，git 源只克隆一次到
//! `target/xirang/sources/` 下按内容寻址的缓存，再检出清单点名的确切版本。解析出的提交进入单元的
//! 来源标记——这正是“提交变了就是另一份计划、`--locked` 会拒绝”的依据。

use std::path::{Path, PathBuf};
use std::process::Command;

use super::manifest::SourceSpec;

/// Resolve one `[sources]` entry: the directory the package is looked up in, and the origin label
/// recorded for units that come from it (`team@<commit>` for a git source).
/// 解析一个 `[sources]` 条目：包在其中查找的目录，以及来自它的单元记录的来源标记（git 源为
/// `team@<commit>`）。
pub(crate) fn resolve(
    root: &Path,
    name: &str,
    package: &str,
    spec: &SourceSpec,
) -> Result<(PathBuf, String), String> {
    match spec {
        SourceSpec::Path(path) => {
            let base = if path.is_absolute() {
                path.clone()
            } else {
                root.join(path)
            };
            Ok((base.join(package), name.to_owned()))
        }
        SourceSpec::Registry { directory } => {
            let base = if directory.is_absolute() {
                directory.clone()
            } else {
                root.join(directory)
            };
            let package_directory = base.join(package);
            let mut versions: Vec<String> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&package_directory) {
                for entry in entries.flatten() {
                    let version = entry.file_name().to_string_lossy().into_owned();
                    if package_directory
                        .join(&version)
                        .join("xirang.toml")
                        .is_file()
                    {
                        versions.push(version);
                    }
                }
            }
            let Some(version) = versions.into_iter().max_by(|a, b| compare_versions(a, b)) else {
                return Err(format!(
                    "source `{name}`: registry {} has no version of package `{package}` with an xirang.toml; it holds: {}",
                    base.display(),
                    available(&base)
                ));
            };
            Ok((
                package_directory.join(&version),
                format!("{name}@{version}"),
            ))
        }
        SourceSpec::Git { url, rev } => {
            let key = format!("{url}\0{rev}");
            let digest = xirang_kernel::sha256_hex(key.as_bytes());
            let directory = root.join("target/xirang/sources").join(&digest[..16]);
            if !directory.is_dir() {
                if let Some(parent) = directory.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
                }
                let output = Command::new("git")
                    .args(["clone", "--quiet", "--no-checkout", url])
                    .arg(&directory)
                    .env("GIT_TERMINAL_PROMPT", "0")
                    .output()
                    .map_err(|error| {
                        format!(
                            "source `{name}` needs `git clone {url}`, which could not run: {error}"
                        )
                    })?;
                if !output.status.success() {
                    let _ = std::fs::remove_dir_all(&directory);
                    return Err(format!(
                        "source `{name}`: `git clone {url}` failed: {}",
                        tail(&output.stderr)
                    ));
                }
            }
            // A commit hash is a fixed input: once the cache holds it, nothing changes what it
            // means. A branch or tag is a moving reference: upstream decides what it names, so the
            // cache must be refreshed before every resolution and the name must be resolved against
            // the fetched refs (`origin/<branch>`, `refs/tags/<tag>`), never a stale local branch.
            // Refreshing can fail (offline), and then the honest answer is a refusal: the cached
            // commit may no longer be what the reference names, and using it silently would write a
            // lie into the lock.
            // 提交哈希是固定输入：缓存一旦持有，它指的东西不会再变。分支或标签是会移动的引用：它的含义由
            // 上游决定，所以每次解析前都要刷新缓存，并把名字解析到刚取回的引用（`origin/<branch>`、
            // `refs/tags/<tag>`），绝不用过期的本地分支。刷新可能失败（离线），此时诚实的答案是拒绝——缓存
            // 里的提交可能已不是该引用所指，静默使用它会让锁记下假话。
            if is_commit(rev) {
                if resolve_commit(&directory, rev).is_none() {
                    let _ = run_git(&directory, &["fetch", "--quiet", "origin"]);
                }
            } else {
                run_git(&directory, &["fetch", "--quiet", "--tags", "--force", "origin"]).map_err(
                    |error| {
                        format!(
                            "source `{name}`: `{rev}` names a branch or tag, and refreshing `{url}` failed ({error}); the cached commit may be stale, so pin a commit hash to build offline"
                        )
                    },
                )?;
            }
            let commit = resolve_commit(&directory, rev).ok_or_else(|| {
                format!("source `{name}`: `{url}` has no commit `{rev}` after a fetch")
            })?;
            run_git(&directory, &["checkout", "--quiet", "--detach", &commit])
                .map_err(|error| format!("source `{name}`: cannot check out `{rev}`: {error}"))?;
            let short = &commit[..commit.len().min(12)];
            Ok((directory.join(package), format!("{name}@{short}")))
        }
    }
}

/// Run git in `directory` and return stdout, refusing a non-zero exit with its stderr tail.
/// 在 `directory` 里跑 git 并返回 stdout；非零退出时带上 stderr 尾部拒绝。
fn run_git(directory: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(tail(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Order two registry versions: numeric dot segments compare as numbers, anything else as text.
/// 比较两个注册表版本：数字点段按数值比较，其余按文本比较。
pub(super) fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let numbers = |value: &str| -> Option<Vec<u64>> {
        value
            .split('.')
            .map(|part| part.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()
    };
    match (numbers(a), numbers(b)) {
        (Some(left), Some(right)) => left.cmp(&right),
        _ => a.cmp(b),
    }
}

/// Name what a registry holds, for a refusal that lets the reader pick a real package.
/// 点出注册表里有哪些包，让拒绝信息能让读者挑一个真实存在的包。
fn available(base: &Path) -> String {
    let mut packages: Vec<String> = std::fs::read_dir(base)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    packages.sort();
    if packages.is_empty() {
        "nothing".to_owned()
    } else {
        packages.join(", ")
    }
}

/// Whether `rev` is a commit hash rather than a moving branch or tag name.
/// `rev` 是否是提交哈希（而不是会移动的分支/标签名）。
fn is_commit(rev: &str) -> bool {
    (7..=40).contains(&rev.len()) && rev.chars().all(|character| character.is_ascii_hexdigit())
}

/// Resolve `rev` to a commit id, preferring the fetched remote refs so a moved branch or tag is
/// followed instead of a stale local branch.
/// 把 `rev` 解析成提交 id；优先刚取回的远端引用，从而跟随移动过的分支/标签，而不是过期的本地分支。
fn resolve_commit(directory: &Path, rev: &str) -> Option<String> {
    let candidates = [
        format!("refs/remotes/origin/{rev}^{{commit}}"),
        format!("refs/tags/{rev}^{{commit}}"),
        format!("{rev}^{{commit}}"),
    ];
    for candidate in candidates {
        if let Ok(text) = run_git(directory, &["rev-parse", "--verify", "--quiet", &candidate]) {
            let text = text.trim();
            if !text.is_empty() {
                return Some(text.to_owned());
            }
        }
    }
    None
}

/// The last non-empty line of stderr, for a refusal that says what git said.
/// stderr 最后一条非空行，让拒绝带上 git 的原话。
fn tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .rfind(|line| !line.trim().is_empty())
        .unwrap_or("no message")
        .to_owned()
}

/// Read one `[sources]` entry: a quoted path string, or `{ git = "…", rev = "…" }`.
/// 读一个 `[sources]` 条目：带引号的路径字符串，或 `{ git = "…", rev = "…" }`。
pub(crate) fn inline_source(name: &str, value: &str, number: usize) -> Result<SourceSpec, String> {
    let value = value.trim();
    if value.starts_with('"') {
        return Ok(SourceSpec::Path(PathBuf::from(super::manifest::string(
            value, number,
        )?)));
    }
    let inner = value
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or_else(|| {
            format!(
                "line {number}: source `{name}` must be a quoted path, `{{ path = \"…\" }}`, or `{{ git = \"…\", rev = \"…\" }}`"
            )
        })?;
    let mut path = None;
    let mut url = None;
    let mut rev = None;
    let mut registry = None;
    for part in inner.split(',') {
        let (key, value) = part.split_once('=').ok_or_else(|| {
            format!("line {number}: expected `key = \"value\"` in source `{name}`")
        })?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "path" => path = Some(super::manifest::string(value, number)?),
            "git" => url = Some(super::manifest::string(value, number)?),
            "rev" => rev = Some(super::manifest::string(value, number)?),
            "registry" => registry = Some(super::manifest::string(value, number)?),
            other => {
                return Err(format!(
                    "line {number}: source `{name}` names `{other}`; a source takes `path`, `registry`, or `git` and `rev`"
                ));
            }
        }
    }
    if let Some(path) = path {
        if url.is_some() || rev.is_some() || registry.is_some() {
            return Err(format!(
                "line {number}: source `{name}` mixes `path` with `git`/`rev`/`registry`; a source is one or the other"
            ));
        }
        return Ok(SourceSpec::Path(PathBuf::from(path)));
    }
    if let Some(directory) = registry {
        if url.is_some() || rev.is_some() {
            return Err(format!(
                "line {number}: source `{name}` mixes `registry` with `git`/`rev`; a source is one or the other"
            ));
        }
        if directory.is_empty() {
            return Err(format!(
                "line {number}: source `{name}` leaves `registry` empty"
            ));
        }
        return Ok(SourceSpec::Registry {
            directory: PathBuf::from(directory),
        });
    }
    let (Some(url), Some(rev)) = (url, rev) else {
        return Err(format!(
            "line {number}: source `{name}` is a git source and must name both `git` and `rev`; an unversioned git source would change between builds"
        ));
    };
    if url.is_empty() || rev.is_empty() {
        return Err(format!(
            "line {number}: source `{name}` leaves `git` or `rev` empty"
        ));
    }
    Ok(SourceSpec::Git { url, rev })
}
