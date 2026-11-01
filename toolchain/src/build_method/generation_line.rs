//! Where an artifact records the generation that published it — the **format** half of the rule in `discovery_cache`.
//! 产物在哪里记录发布它的那个代次——`discovery_cache` 里那条规则的**格式**那一半。
//!
//! Split out when `discovery_cache.rs` reached its code-line budget, and the seam is the one the maintainer's ruling of
//! 2026-10-18 (option A1) created: the number lives **inside** the artifact, so "what the file looks like" and "when to
//! write it" are now two questions. This page answers the first; the writer asks it.
//! 从 `discovery_cache.rs` 里拆出来，是因为后者到了代码行预算；而这条缝正是维护者 2026-10-18 裁定（选项 A1）造出来的：
//! 那个数字住在产物**里面**，于是"文件长什么样"与"什么时候写它"成了两个问题。本页回答第一个，写入方来问它。

use std::fs;
use std::path::{Path, PathBuf};

/// The marker that makes a comment line this writer's generation record rather than prose.
/// 让一行注释成为这个写入方的代次记录、而不是散文的那个标记。
pub(super) const MARKER: &str = "xirang-generation: ";

/// The comment syntax this artifact's format uses for its generation line.
/// 这份产物的格式为代次行使用的注释语法。
///
/// `//` for Rust, `#` for everything else — the rest of what this writer publishes is TOML, and a format nobody names
/// here is likelier to be line-oriented than to be Rust.
/// Rust 用 `//`，其余用 `#`——这个写入方发布的其它产物都是 TOML，而这里没点名的格式更可能是按行解析的，而不是 Rust。
pub(super) fn prefix(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => "// ",
        _ => "# ",
    }
}

/// The artifact as published: the generation line, then the content (with any earlier generation line dropped).
/// 已发布的产物：代次行，然后是内容（此前那条代次行被去掉）。
pub(super) fn compose(path: &Path, content: &str, generation: u64) -> String {
    format!(
        "{prefix}{MARKER}{generation}\n{}",
        without_line(path, content),
        prefix = prefix(path)
    )
}

/// `content` with a leading generation line (from either scheme) taken off.
/// 去掉行首那条代次行（无论出自哪个方案）之后的内容。
pub(super) fn without_line<'a>(path: &Path, content: &'a str) -> &'a str {
    let prefix = prefix(path);
    match content.split_once('\n') {
        Some((first, rest))
            if first.starts_with(prefix) && first[prefix.len()..].starts_with(MARKER) =>
        {
            rest
        }
        _ => content,
    }
}

/// Where a **retired** artifact's generation sidecar used to live (read as a floor, deleted when replaced).
/// 已退役的旁注曾经所在的位置（作为下界读出，被替换时删除）。
pub(super) fn sibling(path: &Path) -> PathBuf {
    let name = path.file_name().map_or_else(
        || "artifact".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!("{name}.generation"))
}

/// The generation `path` carries: the one inside the artifact, floored by a retired sidecar when there is one.
/// `path` 携带的代次：产物里的那个；存在退役旁注时取两者较大者。
pub(super) fn published(path: &Path) -> u64 {
    let prefix = prefix(path);
    let from_artifact = fs::read_to_string(path)
        .ok()
        .and_then(|text| {
            let first = text.lines().next()?.strip_prefix(prefix)?;
            first.strip_prefix(MARKER)?.trim().parse::<u64>().ok()
        })
        .unwrap_or(0);
    let from_retired = fs::read_to_string(sibling(path))
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0);
    from_artifact.max(from_retired)
}
