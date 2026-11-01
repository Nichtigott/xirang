//! The small value readers the manifest parser uses: quoting, integers, arrays and the two
//! inline-table shapes. They live here so the parser file stays inside the size budget and the
//! accepted subset has one place to read.
//! 清单读取器用到的小型取值器：引号字符串、整数、数组与两种内联表。它们住在这里，让解析器文件
//! 留在尺寸预算内，并让被接受的子集只有一个地方可读。

use std::path::PathBuf;

use super::Dependency;

pub(super) fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

/// One value in an external language's `[language.<id>]` table: a quoted string, a whole number,
/// or a boolean, kept as text so the backend receives the spelling the project wrote.
/// 外部语言的 `[language.<id>]` 表里的一个值：带引号的字符串、整数或布尔值，按项目写下的拼写保存，
/// 原样交给后端。
pub(super) fn option_value(value: &str, number: usize) -> Result<String, String> {
    if value.starts_with('"') {
        return string(value, number);
    }
    if value == "true" || value == "false" {
        return Ok(value.to_owned());
    }
    let parsed = integer(value, number)?;
    Ok(parsed.to_string())
}

/// Read a double-quoted string with no escapes (the subset this reader accepts).
/// 读一个无双引号转义的双引号字符串（本读取器接受的子集）。
pub(crate) fn string(value: &str, number: usize) -> Result<String, String> {
    let inner = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| format!("line {number}: expected a quoted string, found `{value}`"))?;
    if inner.contains('\\') {
        return Err(format!(
            "line {number}: escapes are outside this reader's subset; write the character itself"
        ));
    }
    Ok(inner.to_owned())
}

/// Read a non-negative integer.
/// 读一个非负整数。
pub(super) fn integer(value: &str, number: usize) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("line {number}: expected an integer, found `{value}`"))
}

/// Read `{ path = "…" }`; any other key is refused rather than carried as a name.
/// 读 `{ path = "…" }`；其它键一律拒绝，而不是当成名字带下去。
pub(super) fn inline_path(value: &str, number: usize) -> Result<PathBuf, String> {
    let inner = value
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or_else(|| {
            format!(
                "line {number}: this slice resolves local dependencies only; expected \
                 `{{ path = \"…\" }}`, found `{value}`"
            )
        })?;
    let (key, rest) = inner
        .split_once('=')
        .ok_or_else(|| format!("line {number}: expected `path = \"…\"` inside `{{ … }}`"))?;
    if key.trim() != "path" {
        return Err(format!(
            "line {number}: dependency key `{}` is not `path`; version and source resolution is a \
             later slice",
            key.trim()
        ));
    }
    Ok(PathBuf::from(string(rest.trim(), number)?))
}

/// Read a `[dependencies]` value: either `{ path = "…" }` or
/// `{ source = "…", package = "…" }`. The two shapes are not mixed, and a name the source needs is
/// named in the refusal rather than guessed.
/// 读一个 `[dependencies]` 值：`{ path = "…" }` 或 `{ source = "…", package = "…" }` 两形之一。
/// 两形不许混用，源缺哪一项就在拒绝里点名，而不是猜。
pub(super) fn inline_dependency(value: &str, number: usize) -> Result<Dependency, String> {
    let inner = value
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or_else(|| {
            format!(
                "line {number}: expected `{{ path = \"…\" }}` or \
                 `{{ source = \"…\", package = \"…\" }}`, found `{value}`"
            )
        })?;
    let mut path = None;
    let mut source = None;
    let mut package = None;
    for part in inner.split(',') {
        let Some((key, rest)) = part.split_once('=') else {
            return Err(format!(
                "line {number}: expected `key = \"value\"` inside `{{ … }}`, found `{}`",
                part.trim()
            ));
        };
        let key = key.trim();
        let value = string(rest.trim(), number)?;
        match key {
            "path" => path = Some(PathBuf::from(value)),
            "source" => source = Some(value),
            "package" => package = Some(value),
            other => {
                return Err(format!(
                    "line {number}: dependency key `{other}` is not `path`, `source` or \
                     `package`; registry and version resolution is later work"
                ));
            }
        }
    }
    match (path, source, package) {
        (Some(path), None, None) => Ok(Dependency::Path(path)),
        (None, Some(source), Some(package)) => Ok(Dependency::Source { source, package }),
        (None, Some(_), None) => Err(format!(
            "line {number}: a `source` dependency also needs `package = \"…\"`"
        )),
        (None, None, Some(_)) => Err(format!(
            "line {number}: `package` is only meaningful with `source = \"…\"`"
        )),
        _ => Err(format!(
            "line {number}: a dependency names either a `path` or a `source` and `package`, not both"
        )),
    }
}

/// Read a `["a", "b"]` array of quoted strings.
/// 读一个 `["a", "b"]` 形式的引号字符串数组。
pub(super) fn array(value: &str, line: usize) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| {
            format!("line {line}: expected an array of quoted strings, found `{value}`")
        })?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| string(item.trim(), line))
        .collect()
}
