//! The shape of a registry mirror: `<registry>/<package>/<version>/xirang.toml`. A registry source
//! resolves the highest version it can see, so the directory itself is the index; this checker makes
//! a mirror's promises inspectable before it is shared (and is what a remote registry would serve).
//! 注册表镜像的形状：`<registry>/<package>/<version>/xirang.toml`。registry 源取它能看到的最高的
//! 版本，因此目录本身就是索引；这个检查器让镜像的承诺在被分享之前可以先被核对（远端注册表将来要提供的
//! 也正是这份形状）。

use std::path::Path;

use super::manifest::Manifest;

/// Validate a registry mirror and report what it holds.
/// 校验一个注册表镜像并报告它有什么。
pub(crate) fn check(directory: &Path) -> Result<String, String> {
    if !directory.is_dir() {
        return Err(format!("{} is not a directory", directory.display()));
    }
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    let mut packages = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let package = entry.file_name().to_string_lossy().into_owned();
        if package.starts_with('.') || !entry.path().is_dir() {
            continue;
        }
        let mut versions = Vec::new();
        let version_entries = std::fs::read_dir(entry.path())
            .map_err(|error| format!("cannot read {}: {error}", entry.path().display()))?;
        for version_entry in version_entries {
            let version_entry = version_entry
                .map_err(|error| format!("cannot read {}: {error}", entry.path().display()))?;
            let version = version_entry.file_name().to_string_lossy().into_owned();
            if version.starts_with('.') || !version_entry.path().is_dir() {
                continue;
            }
            let manifest = version_entry.path().join("xirang.toml");
            if !manifest.is_file() {
                continue;
            }
            let parsed = Manifest::load(&version_entry.path())?;
            if parsed.package.name != package {
                return Err(format!(
                    "registry {}: version `{version}` of package `{package}` declares itself \
                     `{}` in its manifest",
                    directory.display(),
                    parsed.package.name
                ));
            }
            if parsed.package.version != version {
                return Err(format!(
                    "registry {}: version `{version}` of package `{package}` declares itself \
                     `{}` in its manifest",
                    directory.display(),
                    parsed.package.version
                ));
            }
            versions.push(version);
        }
        if versions.is_empty() {
            return Err(format!(
                "registry {}: package `{package}` has no version directory with an xirang.toml",
                directory.display()
            ));
        }
        versions.sort_by(|a, b| super::sources::compare_versions(a, b));
        let highest = versions.last().cloned().unwrap_or_default();
        packages.push((package, versions.len(), highest));
    }
    packages.sort();
    if packages.is_empty() {
        return Err(format!(
            "registry {} holds no package (the shape is `<package>/<version>/xirang.toml`)",
            directory.display()
        ));
    }
    let versions: usize = packages.iter().map(|(_, count, _)| count).sum();
    let mut out = format!(
        "registry {}: {} package(s), {} version(s)\n",
        directory.display(),
        packages.len(),
        versions
    );
    for (package, count, highest) in packages {
        out.push_str(&format!(
            "  {package}: {count} version(s); highest {highest}\n"
        ));
    }
    Ok(out)
}

/// Fetch a registry mirror from an index whose sha256 the caller pins.
/// 按调用方钉住的 sha256，从索引取回一个注册表镜像。
pub(crate) fn fetch(
    index: &str,
    digest: &str,
    into: &Path,
    auth: Option<&str>,
) -> Result<String, String> {
    let digest = digest.to_ascii_lowercase();
    if digest.len() != 64
        || !digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(format!("`{digest}` is not a sha256 digest (64 hex digits)"));
    }
    if let Some(token) = auth
        && (token.is_empty() || token.chars().any(char::is_control))
    {
        return Err(
            "the bearer token is empty or carries a control character; it is not sent".to_owned(),
        );
    }
    let base = index_base(index)?;
    let bytes = download(index, 1 << 20, auth)?;
    let actual = xirang_kernel::sha256_hex(&bytes);
    if actual != digest {
        return Err(format!(
            "the index at {index} hashes to {actual}, not the pinned {digest}; nothing was fetched"
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("the index at {index} is not JSON: {error}"))?;
    let files = parse_index(&value)?;
    if into.exists() {
        return Err(format!(
            "{} already exists; a mirror is written once",
            into.display()
        ));
    }
    let staging = into.with_file_name(format!(
        ".{}.tmp-{}",
        into.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mirror"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&staging);
    let result = (|| -> Result<(), String> {
        for (relative, sha) in &files {
            let url = format!("{base}/{relative}");
            let bytes = download(&url, 8 << 20, auth)?;
            let actual = xirang_kernel::sha256_hex(&bytes);
            if actual != *sha {
                return Err(format!(
                    "{relative} hashes to {actual}, not the index's {sha}; nothing was fetched"
                ));
            }
            let destination = staging.join(relative);
            if !destination.starts_with(&staging) {
                return Err(format!("`{relative}` would escape the mirror"));
            }
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            std::fs::write(&destination, &bytes)
                .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
        }
        check(&staging)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }
    std::fs::rename(&staging, into).map_err(|error| {
        format!(
            "cannot move the fetched mirror into {}: {error}",
            into.display()
        )
    })?;
    Ok(format!(
        "fetched {index} (index sha256 {digest})\n{}",
        check(into)?
    ))
}

/// The directory part of an index URL, after the scheme is judged safe.
/// 索引 URL 的目录部分（先判断取用方式是否安全）。
fn index_base(url: &str) -> Result<String, String> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| format!("`{url}` is not a URL"))?;
    let (host, _) = rest
        .split_once('/')
        .ok_or_else(|| format!("`{url}` names no path"))?;
    let hostname = host.split(':').next().unwrap_or(host);
    match scheme {
        "https" => {}
        "http"
            if hostname == "127.0.0.1"
                || hostname == "localhost"
                || hostname.starts_with("127.")
                || host.starts_with("[::1]") => {}
        "http" => {
            return Err(format!(
                "`{url}` is plain http to `{host}`; a mirror is fetched over https, or over http only from loopback"
            ));
        }
        other => {
            return Err(format!(
                "`{url}` uses `{other}`; a mirror is fetched over https (or http from loopback)"
            ));
        }
    }
    let at = url
        .rfind('/')
        .ok_or_else(|| format!("`{url}` names no file"))?;
    Ok(url[..at].to_owned())
}

/// Download one URL through `curl`, refusing anything above `limit` bytes.
/// 用 `curl` 下载一个 URL，超过 `limit` 字节就拒绝。
fn download(url: &str, limit: usize, auth: Option<&str>) -> Result<Vec<u8>, String> {
    let mut command = std::process::Command::new("curl");
    command.args(["-fsS", "--max-time", "60"]);
    if auth.is_some() {
        // The token travels through curl's config file on stdin, never through argv: a process
        // listing must not be able to read it, and no error message below repeats it.
        // 令牌经 stdin 上的 curl 配置文件传递，绝不经过 argv：进程列表读不到它，下面的任何错误
        // 消息也不会复述它。
        command.args(["--config", "-"]);
    }
    command.arg(url);
    let Some(token) = auth else {
        let output = command
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        return finish(url, limit, output);
    };
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    {
        use std::io::Write;
        let header = format!(
            "header = \"Authorization: Bearer {}\"\n",
            escape_config(token)
        );
        child
            .stdin
            .as_mut()
            .ok_or("curl has no stdin")?
            .write_all(header.as_bytes())
            .map_err(|error| format!("cannot send the request header: {error}"))?;
    }
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for curl: {error}"))?;
    finish(url, limit, output)
}

/// Report one `curl` result, refusing a failed request or an oversized body.
/// 报告一次 `curl` 结果，拒绝失败的请求或过大的响应。
fn finish(url: &str, limit: usize, output: std::process::Output) -> Result<Vec<u8>, String> {
    if !output.status.success() {
        return Err(format!(
            "cannot download {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if output.stdout.len() > limit {
        return Err(format!(
            "{url} is larger than {limit} bytes; this fetcher refuses it"
        ));
    }
    Ok(output.stdout)
}

/// Escape one value for curl's config-file quoting (`\\` and `\"`).
/// 按 curl 配置文件的引号规则转义一个值（`\\` 与 `\"`）。
fn escape_config(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            other => escaped.push(other),
        }
    }
    escaped
}

/// The `(path, sha256)` entries an index names, validated before anything is written.
/// 索引点名的 `(路径, sha256)`，写任何东西之前先校验。
fn parse_index(value: &serde_json::Value) -> Result<Vec<(String, String)>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "the index is not a JSON object".to_owned())?;
    for key in object.keys() {
        if !["protocol", "packages"].contains(&key.as_str()) {
            return Err(format!("the index has an unknown key `{key}`"));
        }
    }
    if object.get("protocol").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err("the index's `protocol` must be 1".to_owned());
    }
    let packages = object
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "the index needs a `packages` array".to_owned())?;
    if packages.is_empty() {
        return Err("the index lists no package".to_owned());
    }
    let mut files = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for package in packages {
        let package = package
            .as_object()
            .ok_or_else(|| "a package entry is not an object".to_owned())?;
        let name = package
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "a package entry has no name".to_owned())?;
        let version = package
            .get("version")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "a package entry has no version".to_owned())?;
        let list = package
            .get("files")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "a package entry has no files".to_owned())?;
        if list.is_empty() {
            return Err(format!("`{name} {version}` lists no file"));
        }
        for file in list {
            let file = file
                .as_object()
                .ok_or_else(|| "a file entry is not an object".to_owned())?;
            let path = file
                .get("path")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "a file entry has no path".to_owned())?;
            let sha = file
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "a file entry has no sha256".to_owned())?;
            if sha.len() != 64 || !sha.chars().all(|character| character.is_ascii_hexdigit()) {
                return Err(format!("`{path}`'s sha256 is not 64 hex digits"));
            }
            validate_relative(path)?;
            if !seen.insert(path.to_owned()) {
                return Err(format!("the index lists `{path}` twice"));
            }
            files.push((path.to_owned(), sha.to_ascii_lowercase()));
        }
    }
    Ok(files)
}

/// A path that stays inside the mirror.
/// 一条留在镜像内的相对路径。
fn validate_relative(path: &str) -> Result<(), String> {
    let candidate = Path::new(path);
    if path.is_empty()
        || candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!("`{path}` is not a relative path inside the mirror"));
    }
    Ok(())
}
