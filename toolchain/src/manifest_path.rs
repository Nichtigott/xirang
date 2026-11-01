//! How a manifest spells a path: relative to the manifest's own directory, with `/` separators.
//! 清单如何拼写一条路径：相对清单自己所在目录，使用 `/` 分隔符。

use std::path::Path;

/// `to` relative to `from`, with `/` separators — the spelling a manifest's `path = "…"` uses.
/// `to` 相对 `from` 的路径，使用 `/` 分隔符——清单 `path = "…"` 使用的拼写。
///
/// Two writers produce this spelling: the crate split writes a ghost package's dependency path, and
/// the host-edge writer writes the path back into the host manifest. They must agree, because a
/// manifest that spells one directory two ways makes cargo look for a package that is not there (or
/// find the same one twice), so the rule has one home instead of one copy per writer.
/// 两个写入方产出这种拼写：crate 拆分写下幽灵包的依赖路径，宿主边写入方把路径写回宿主清单。两者必须一致
/// ——同一份清单用两种拼法写同一个目录，会让 cargo 去找一个不存在的包（或把同一个包找到两次）——因此这条
/// 规则只有一个家，而不是每个写入方一份。
pub(crate) fn relative(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts: Vec<String> =
        std::iter::repeat_n("..".to_owned(), from.len() - common).collect();
    parts.extend(
        to[common..]
            .iter()
            .map(|component| component.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}
