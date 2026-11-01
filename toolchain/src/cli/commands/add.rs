//! `xirang add <name> --path <directory>` or `xirang add <name> --source <name> [location]`: record
//! one dependency in `xirang.toml`. The source form may also declare the source itself.
//! `xirang add <name> --path <directory>` 或 `xirang add <name> --source <name> [位置]`：把一条依赖
//! 写进 `xirang.toml`。source 形式还可以顺便声明那个源。
//!
//! The command owns argv only. What makes an entry valid — the package name, the library kind, the
//! absence of a duplicate, and the absence of a cycle — is decided by the multilingual surface,
//! before anything is written.
//! 本命令只拥有 argv。「一条条目是否成立」——包名、是否库、是否重名、是否会闭合成环——由多语言面
//! 在写入之前判定。

use std::io::Write;
use std::path::PathBuf;

/// `xirang add` 的用法。
const USAGE: &str = "usage: xirang add <name> --path <directory>\n       \
     xirang add <name> --source <source>\n       \
     xirang add <name> --source <source> (--path <directory> | --git <url> --rev <rev> | \
     --registry <directory>)";

/// Parse the argument list and record the dependency, or say why it is not one.
/// 解析参数并记录依赖，或说明这串参数为何不成其为一次调用。
pub(crate) fn add_dependency(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let usage = USAGE;
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut source: Option<String> = None;
    let mut git: Option<String> = None;
    let mut rev: Option<String> = None;
    let mut registry: Option<String> = None;
    let mut project: Option<PathBuf> = None;
    while let Some(argument) = args.next() {
        if argument == "-h" || argument == "--help" {
            return crate::cli::usage(out);
        }
        if let Some(slot) = match argument.as_str() {
            "--path" => Some(&mut path),
            "--source" => Some(&mut source),
            "--git" => Some(&mut git),
            "--rev" => Some(&mut rev),
            "--registry" => Some(&mut registry),
            _ => None,
        } {
            let value = args
                .next()
                .ok_or_else(|| format!("{argument} needs a value; {usage}"))?;
            if slot.replace(value).is_some() {
                return Err(format!("{argument} may be given once; {usage}"));
            }
            continue;
        }
        if argument == "--project" {
            let value = args
                .next()
                .ok_or_else(|| format!("--project needs a directory; {usage}"))?;
            if project.replace(PathBuf::from(value)).is_some() {
                return Err(format!("--project may be given once; {usage}"));
            }
            continue;
        }
        if argument.starts_with('-') {
            return Err(format!("unexpected option '{argument}'; {usage}"));
        }
        if name.replace(argument).is_some() {
            return Err(format!("unexpected argument; {usage}"));
        }
    }
    let name = name.ok_or_else(|| format!("`xirang add` needs a package name; {usage}"))?;
    let project = project.unwrap_or_else(|| PathBuf::from("."));
    let summary = match source {
        Some(source) => {
            let declaration = match (path, git, rev, registry) {
                (None, None, None, None) => crate::multilang::SourceDeclaration::Existing,
                (Some(path), None, None, None) => crate::multilang::SourceDeclaration::Path(path),
                (None, Some(url), Some(rev), None) => {
                    crate::multilang::SourceDeclaration::Git { url, rev }
                }
                (None, None, None, Some(directory)) => {
                    crate::multilang::SourceDeclaration::Registry(directory)
                }
                (None, Some(_), None, _) => {
                    return Err(format!(
                        "--git needs --rev: a git source is pinned; {usage}"
                    ));
                }
                (None, None, Some(_), _) => {
                    return Err(format!("--rev needs --git; {usage}"));
                }
                _ => {
                    return Err(format!(
                        "declare at most one source location: --path, --git with --rev, or \
                         --registry; {usage}"
                    ));
                }
            };
            crate::multilang::add_from_source(&project, &name, &source, declaration)?
        }
        None => {
            if git.is_some() || rev.is_some() || registry.is_some() {
                return Err(format!(
                    "--git, --rev and --registry declare a source; add --source <name>; {usage}"
                ));
            }
            let path = path.ok_or_else(|| {
                format!("`xirang add` needs --path <directory> or --source <source>; {usage}")
            })?;
            crate::multilang::add_dependency(&project, &name, &path)?
        }
    };
    writeln!(out, "{summary}")
        .map_err(|error| format!("cannot write the dependency summary: {error}"))
}
