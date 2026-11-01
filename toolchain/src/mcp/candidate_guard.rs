//! The two boundaries a candidate is judged against beyond its path scope: protected entities and
//! the tests that were already the acceptance criteria. Both are decided from the **actual diff**,
//! never from the author's account.
//! 候选在路径范围之外还要对照的两条边界：受保护实体，以及本来就充当验收判据的那些测试。两者都由**实际差异**
//! 判定，而不是作者的自述。
//!
//! A scope is a set of paths, but the thing a person protects can be smaller than a file: one
//! function in a file that also holds the change (EX03), and the tests that were already the
//! acceptance criteria (EX11). The boundary is the **host's** record, not the candidate's: a list the
//! author writes at `open` cannot loosen a list the person did not write, so `protected` takes the
//! host entries as `extra` and `changed_tests` takes the host's allowances. What the author claims at
//! `open` is still checked (a claim may only add protection, never remove it).
//! 范围是一组路径，而人真正要保的东西可以比一个文件更小：同一个文件里既有改动、也有受保护的函数（EX03），
//! 以及本来就充当验收判据的那些测试（EX11）。边界是**宿主**的记录，不是候选的：作者在 `open` 时写的清单
//! 不能放宽人没有写过的清单，因此 `protected` 把宿主条目当 `extra`、`changed_tests` 收宿主的允许清单。
//! 作者在 `open` 时的自述仍然会被检查（自述只能增加保护，不能减少）。

use super::Record;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Every protected entity whose bytes the change moved, including the ones the host named.
/// 这次改动动了字节的每一个受保护实体，包括宿主点名的那些。
///
/// An entity the base declares but the copy no longer has (or the reverse) is **not** a pass and not
/// a violation either: it is `unknown`, because the boundary cannot be located on both sides. That is
/// said as an error rather than a quiet skip — a rename that carries the protected code along would
/// otherwise read as "nothing protected changed".
/// 基树声明过、而副本已经没有（或反过来）的实体**不是**通过，也不是违约：它是 `unknown`，因为边界在两端都
/// 定位不到。这以错误说出来而不是静默跳过——否则一次把受保护代码一起搬走的改名会被读成"受保护的东西没变"。
pub(super) fn protected(
    record: &Record,
    tree: &Path,
    extra: &[(String, String)],
) -> Result<Vec<String>, String> {
    let mut entries: Vec<(String, String)> = record.body["protect"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|entry| {
            Ok((
                entry["path"]
                    .as_str()
                    .ok_or("a `protect` entry needs `path`")?
                    .to_owned(),
                entry["entity"]
                    .as_str()
                    .ok_or("a `protect` entry needs `entity`")?
                    .to_owned(),
            ))
        })
        .collect::<Result<_, String>>()?;
    entries.extend(extra.iter().cloned());
    entries.sort();
    entries.dedup();
    let base = Path::new(record.body["base_root"].as_str().ok_or(
        "this candidate's record names no base root, so protected entities cannot be located",
    )?);
    let mut violations = Vec::new();
    for (path, entity) in entries {
        let before = std::fs::read_to_string(base.join(&path))
            .map_err(|error| format!("cannot read {path} in the base tree: {error}"))?;
        let after = std::fs::read_to_string(tree.join(&path))
            .map_err(|error| format!("cannot read {path} in the candidate: {error}"))?;
        let before = entity_text(&before, &entity).ok_or_else(|| {
            format!(
                "the protected entity `{entity}` is not a function declared in {path} at the base, so whether this change touches it is unknown; name an entity the base declares"
            )
        })?;
        let after = entity_text(&after, &entity).ok_or_else(|| {
            format!(
                "the protected entity `{entity}` is no longer declared in {path} in the candidate, so whether this change touches it is unknown: restoring the declaration or opening a candidate that removes the protection is a person's decision"
            )
        })?;
        if before != after {
            violations.push(format!("{path}: protected entity `{entity}` changed"));
        }
    }
    Ok(violations)
}

/// Every existing test the change removed or weakened without the host naming it.
/// 这次改动删除或削弱、且宿主没有点名的每一个既有测试。
///
/// Deleting a test is not automatically a vice: a refactor that renames one is legitimate. The
/// difference between "a legitimate rename" and "the criteria were weakened until they passed" is
/// **who decided it** — so the host's confirmation entry names the test (`--allow-removed-test
/// <path>::<name>`), and a candidate's own list cannot do that for itself. A test that survives but
/// loses assertions is reported too: removing an assertion is weakening with the test's name kept.
/// 删除测试并非天然是坏事：一次改名重构是合法的。"合法改名"与"把判据削弱到通过"之间的区别是**谁决定的**——
/// 因此由宿主的确认入口点名那个测试（`--allow-removed-test <path>::<name>`），候选自己的清单不能替自己允许。
/// 存活但少断言的测试也会被报出来：留下测试名而删掉断言，就是留下名字的削弱。
pub(super) fn changed_tests(
    record: &Record,
    tree: &Path,
    changed: &[String],
    allowed: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    let base =
        Path::new(record.body["base_root"].as_str().ok_or(
            "this candidate's record names no base root, so removed tests cannot be located",
        )?);
    let mut violations = Vec::new();
    for path in changed.iter().filter(|path| path.ends_with(".rs")) {
        let Ok(before) = std::fs::read_to_string(base.join(path)) else {
            continue;
        };
        let after = std::fs::read_to_string(tree.join(path)).unwrap_or_default();
        let before = test_assertions(&before);
        let after = test_assertions(&after);
        for (name, assertions) in before {
            let named = allowed.contains(&format!("{path}::{name}"));
            match after.get(&name) {
                None if !named => violations.push(format!("{path}: test `{name}` was removed")),
                Some(now) if now < &assertions && !named => violations.push(format!(
                    "{path}: test `{name}` lost assertion(s) ({assertions} -> {now})"
                )),
                _ => {}
            }
        }
    }
    Ok(violations)
}

/// The line span of a function, rendered so two sides can be compared by text.
/// 一个函数的行跨度，渲染成文本以便两端比较。
fn entity_text(source: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let (start, end) = xirang_kernel::source::function_source_range(&lines, name)?;
    let end = end.min(lines.len().saturating_sub(1));
    Some(lines.get(start..=end)?.join("\n"))
}

/// Every test function a source file declares, folded to how many assertion-bearing lines its body
/// has. The count is the cheap, honest proxy for "the criteria are still as strong": a body that only
/// lost a formatting newline keeps its count, a body that lost an assertion does not.
/// 一个源码文件声明的每一个测试函数，折叠成它函数体里带断言的行数。这个计数是"判据还一样强"的廉价而诚实的
/// 代理：只丢了格式换行的函数体计数不变，丢了断言的不行。
fn test_assertions(source: &str) -> BTreeMap<String, usize> {
    let lines: Vec<&str> = source.lines().collect();
    let names = test_names(&lines);
    names
        .into_iter()
        .filter_map(|name| {
            let (start, end) = xirang_kernel::source::function_source_range(&lines, &name)?;
            let end = end.min(lines.len().saturating_sub(1));
            let body = lines.get(start..=end)?.join("\n");
            Some((name, assertion_count(&body)))
        })
        .collect()
}

/// The `#[test]` function names a file declares, including an attribute and a signature on one line
/// and the attribute spellings this workspace uses (`#[test]`, `#[tokio::test]`, `#[async_std::test]`).
/// 一个文件声明的 `#[test]` 函数名，包括属性与签名写在同一行、以及本工作区使用的属性拼法
/// （`#[test]`、`#[tokio::test]`、`#[async_std::test]`）。
pub(super) fn test_names(lines: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut pending = false;
    for line in lines {
        let trimmed = line.trim();
        if is_test_attribute(trimmed) {
            pending = true;
        }
        if !pending {
            continue;
        }
        if let Some(name) = fn_name(trimmed) {
            names.insert(name);
            pending = false;
        } else if !trimmed.starts_with('#') && !trimmed.is_empty() && !trimmed.starts_with("//") {
            pending = false;
        }
    }
    names
}

/// Whether one trimmed line is a test attribute.
/// 一行（去掉缩进后）是不是测试属性。
fn is_test_attribute(trimmed: &str) -> bool {
    [
        "#[test]",
        "#[tokio::test",
        "#[async_std::test",
        "#[test_case",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

/// How many lines of a body carry something that can fail.
/// 一个函数体里有多少行带着可能失败的东西。
fn assertion_count(body: &str) -> usize {
    body.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
                && [
                    "assert",
                    "panic!",
                    "should_panic",
                    "unwrap_err",
                    "expect_err",
                ]
                .iter()
                .any(|needle| trimmed.contains(needle))
        })
        .count()
}

/// The identifier after `fn ` in one line, when there is one.
/// 一行里 `fn ` 之后的标识符，如果有的话。
fn fn_name(line: &str) -> Option<String> {
    let at = line.find("fn ")?;
    let rest = line[at + 3..].trim_start();
    let name: String = rest
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}
/// The host's boundary: protected entities (`<path>#<entity>`) and the tests the person allowed to
/// disappear. The candidate's record never supplies the allowance — the 2026-10-19 review found that
/// an author-written `allow_removed_tests` was the whole boundary, so the author could allow itself.
/// 宿主的边界：受保护实体（`<path>#<entity>`），以及人允许消失的测试。候选记录从不提供这份允许——2026-10-19
/// 复核发现作者写的 `allow_removed_tests` 就是全部边界，于是作者可以自己允许自己。
/// The host's boundary: protected entities (`<path>#<entity>` made into pairs) and the tests the
/// person allowed to disappear. A named type keeps the tuple readable at both uses.
/// 宿主的边界：受保护实体（`<path>#<entity>` 拆成对）与人允许消失的测试。具名类型让两处使用都读得懂。
pub(super) type Boundary = (Vec<(String, String)>, std::collections::BTreeSet<String>);

pub(super) fn host_boundary(root: &Path, id: &str) -> Result<Boundary, String> {
    let boundary = crate::mcp::controller::boundary(root, id)?;
    let mut protect = Vec::new();
    for row in boundary["protect"].as_array().into_iter().flatten() {
        let text = row
            .as_str()
            .ok_or("a host `protect` entry must be a string like `path#entity`")?;
        let (path, entity) = text.split_once('#').ok_or_else(|| {
            format!("`{text}` is not `<path>#<entity>`; the host's boundary names which entity in which file")
        })?;
        protect.push((path.to_owned(), entity.to_owned()));
    }
    let allow = boundary["allow_removed_tests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.as_str())
        .map(str::to_owned)
        .collect();
    Ok((protect, allow))
}

/// The changed files whose own `#[test]` items did not run in this execution. A delta certified by a
/// suite that passed somewhere else is the "the build passed, the change was not tested" case the
/// 2026-10-19 re-review reproduced by narrowing the feature set.
/// 本次执行里自己的 `#[test]` 条目没有运行的那些被改文件。被"别处通过的那套测试"认证的差异，正是
/// 2026-10-19 复审用窄化特性集复现的"构建通过、改动没被测过"。
pub(super) fn unexecuted(
    tree: &Path,
    changed: &[String],
    log: &Path,
    targets: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    // Only lines the harness printed as `test <name> ... ok|FAILED` describe what **ran**: the stream
    // also carries `... ignored` rows, and treating those as executed is how a skipped test would
    // certify the change it was written for.
    // 只有测试框架印成 `test <name> ... ok|FAILED` 的行才说明**运行过**：流里也带着 `... ignored` 的行，
    // 把它们当成已执行，正是"被跳过的测试去认证它本该覆盖的改动"的来路。
    let executed: BTreeSet<String> = match std::fs::read_to_string(log) {
        Ok(text) => text
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("test ")?;
                let (name, tail) = rest.split_once(" ... ")?;
                (tail.starts_with("ok") || tail.starts_with("FAILED"))
                    .then(|| name.trim().to_owned())
            })
            .collect(),
        Err(_) => targets.keys().cloned().collect(),
    };
    let mut unexecuted = Vec::new();
    for path in changed {
        if !path.ends_with(".rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(tree.join(path)) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let names = test_names(&lines);
        // The executed name is the module path the harness printed (`tests::settle_is_non_negative`);
        // the source declares the bare function name, so match the last segment.
        // 执行名是测试框架印出来的模块路径（`tests::settle_is_non_negative`），源码声明的是裸函数名，
        // 所以按最后一段匹配。
        let ran = |name: &str| {
            executed.contains(name)
                || executed
                    .iter()
                    .any(|executed| executed.ends_with(&format!("::{name}")))
        };
        if !names.is_empty() && !names.iter().any(|name| ran(name)) {
            unexecuted.push(path.clone());
        }
    }
    unexecuted
}
