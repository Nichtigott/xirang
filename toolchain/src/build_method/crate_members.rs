//! Materializing generated packages into the workspace's member list (audit `M7`, P3.5).
//! 把生成的包物化进工作区的成员清单（审计 `M7`，P3.5）。
//!
//! A generated package is the host's **sibling**, so when the host belongs to a Cargo workspace the
//! generated packages sit inside that workspace's directory tree and have to be members too: cargo
//! refuses a package that lies under a workspace it is not listed in (`current package believes it's
//! in a workspace when it's not`), and `cargo build -p <generated>` cannot even name it. Until this
//! module existed the human added them by hand — measured twice while taking the acceptance four
//! (§M7.39) — and forgetting the step leaves a partition that does not build.
//! 生成的包是宿主的**同级**，因此当宿主属于某个 Cargo 工作区时，生成的包就落在那个工作区的目录树里，必须也是
//! 成员：cargo 会拒绝一个位于它未被列入的工作区之下的包（`current package believes it's in a workspace
//! when it's not`），`cargo build -p <生成包>` 甚至点不到它。在这个模块出现之前，这一步由人手工做——验收四条
//! 那次实测两次（§M7.39）——而漏掉这一步留下的是一次编译不过的拆分。

use std::path::Path;

/// The quoted member spellings for these package directories, relative to the workspace root.
/// 这些包目录的成员拼写（带引号），相对工作区根。
pub(crate) fn entries(workspace_root: &Path, directories: &[&Path]) -> Vec<String> {
    let mut entries: Vec<String> = directories
        .iter()
        .map(|directory| format!("\"{}\"", relative(workspace_root, directory)))
        .collect();
    entries.sort();
    entries.dedup();
    entries
}

/// The `members` array a manifest declares, as it is written.
/// 清单声明的 `members` 数组，按它写下的样子。
struct Members {
    /// The line the key starts on.
    /// 键开始的那一行。
    key_line: usize,
    /// The line the closing `]` is on (the same line, in the single-line form).
    /// 结束的 `]` 所在的行（单行形式里与键同一行）。
    end_line: usize,
    /// The item tokens in order, exactly as written, quotes included.
    /// 按顺序的条目原文，含引号。
    items: Vec<String>,
}

impl Members {
    /// Whether the array is written on one line.
    /// 数组是否写在一行里。
    fn single_line(&self) -> bool {
        self.key_line == self.end_line
    }

    /// The indentation the array's items use, or the closing bracket's plus four.
    /// 数组条目使用的缩进；没有条目时是结束括号的缩进加四。
    fn item_indent(&self, lines: &[String]) -> usize {
        lines[self.key_line + 1..=self.end_line]
            .iter()
            .find(|line| line.trim_start().starts_with('"'))
            .map(|line| line.len() - line.trim_start().len())
            .unwrap_or_else(|| {
                let close = &lines[self.end_line];
                close.len() - close.trim_start().len() + 4
            })
    }
}

/// Whether a line opens the `[workspace]` table (a trailing comment is still that table).
/// 某一行是否开启 `[workspace]` 表（行尾注释不影响）。
fn is_workspace_header(line: &str) -> bool {
    line.trim().starts_with("[workspace]")
}

/// Where a manifest's `members` key is, or `None` when it declares none.
/// 清单的 `members` 键在哪里；没有声明时返回 `None`。
///
/// `Err` is the shape this action refuses: a `members` that is not an array of quoted strings, or an
/// array that never closes. Guessing what such a file meant is how a tool eats somebody's workspace.
/// `Err` 是本动作拒绝的形状：不是一串带引号字符串的 `members`，或者始终不闭合的数组。猜这种文件的意思，正是
/// 一件工具吃掉别人工作区的方式。
fn locate(lines: &[String]) -> Result<Option<Members>, String> {
    let Some(section) = lines.iter().position(|line| is_workspace_header(line)) else {
        return Ok(None);
    };
    let Some(key_line) = lines
        .iter()
        .enumerate()
        .skip(section + 1)
        .take_while(|(_, line)| !line.trim().starts_with('['))
        .find(|(_, line)| line.trim().starts_with("members"))
        .map(|(index, _)| index)
    else {
        return Ok(None);
    };
    let Some(value) = lines[key_line]
        .split_once('=')
        .map(|(_, value)| value.trim())
        .filter(|value| value.starts_with('['))
    else {
        return Err(refusal(key_line + 1, &lines[key_line]));
    };
    // The array may close on this line or on a later one: the region runs to the line carrying the
    // closing bracket, and every item inside it is a quoted string.
    // 数组可能在本行闭合，也可能在后面的行闭合：这段文本一直延伸到带结束括号的那一行，而它内部每一项都是带引号
    // 的字符串。
    let mut end_line = key_line;
    let mut region = value.to_owned();
    while !region.contains(']') {
        end_line += 1;
        let Some(line) = lines.get(end_line) else {
            return Err(refusal(key_line + 1, &lines[key_line]));
        };
        region.push('\n');
        region.push_str(line);
    }
    let open = region.find('[').expect("the value starts with `[`");
    let close = region.rfind(']').expect("the region carries a `]`");
    let mut items = Vec::new();
    for item in region[open + 1..close].split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if !item.starts_with('"') || !item.ends_with('"') || item.len() < 2 {
            return Err(refusal(key_line + 1, &lines[key_line]));
        }
        items.push(item.to_owned());
    }
    Ok(Some(Members {
        key_line,
        end_line,
        items,
    }))
}

/// The refusal for a `members` this action will not merge into, naming the line to fix by hand.
/// 对本动作不肯合并进去的 `members` 的拒绝，点名该手工修改的那一行。
fn refusal(line: usize, text: &str) -> String {
    format!(
        "add_crates: `members` at Cargo.toml:{line} is not an array of quoted strings, so this \
         action will not merge into it. Add the generated packages to it by hand, then run this \
         again:\n    {}",
        text.trim()
    )
}

/// The manifest with these entries added, or `None` when it already carries them all.
/// 加上这些条目之后的清单；已经全都有时返回 `None`。
///
/// The caller computes this **before** it writes anything: a refusal must leave the tree as it found
/// it, and a member list naming a package that does not exist is the state cargo refuses to load.
/// 调用方在写任何东西**之前**先算它：拒绝必须让树保持原样，而一份点名了不存在的包的成员清单，正是 cargo
/// 拒绝加载的状态。
pub(crate) fn merged(text: &str, entries: &[String]) -> Result<Option<String>, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let missing: Vec<&String> = entries
        .iter()
        .filter(|entry| !lines.iter().any(|line| line.contains(entry.as_str())))
        .collect();
    if missing.is_empty() {
        return Ok(None);
    }
    let additions: Vec<String> = missing.into_iter().cloned().collect();
    match locate(&lines)? {
        None => {
            // No `members` at all: a workspace whose root is a package does not need one, and this is
            // the line cargo wants for the packages it now has to admit.
            // 根本没有 `members`：根就是一个包的工作区不需要它，而这就是 cargo 为这些它现在必须接纳的包想要的
            // 那一行。
            match lines.iter().position(|line| is_workspace_header(line)) {
                Some(section) => {
                    lines.insert(section + 1, format!("members = [{}]", additions.join(", ")));
                }
                None => {
                    // A standalone package is its own workspace root once it has to admit
                    // generated members: the section is appended, never replacing the package
                    // keys above it.
                    // 独立包一旦要接纳生成成员，自己就是工作区根：追加这一段，绝不替换上面的包键。
                    lines.push(String::new());
                    lines.push("[workspace]".to_owned());
                    lines.push(format!("members = [{}]", additions.join(", ")));
                }
            }
        }
        Some(members) if members.single_line() => {
            let head = lines[members.key_line]
                .split_once('=')
                .expect("the key line has an `=`")
                .0
                .trim_end()
                .to_owned();
            let value = lines[members.key_line]
                .split_once('=')
                .expect("the key line has an `=`")
                .1
                .trim();
            let inner = &value[1..value.rfind(']').expect("the array closes on this line")];
            let separator = if inner.trim().is_empty() {
                ""
            } else if inner.trim_end().ends_with(',') {
                " "
            } else {
                ", "
            };
            lines[members.key_line] =
                format!("{head} = [{inner}{separator}{}]", additions.join(", "));
        }
        Some(members) => {
            let indent = members.item_indent(&lines);
            let at = members.end_line;
            for (offset, entry) in additions.iter().enumerate() {
                lines.insert(at + offset, format!("{}{entry},", " ".repeat(indent)));
            }
        }
    }
    Ok(Some(lines.join("\n") + "\n"))
}

/// The manifest with these entries removed, or `None` when it carried none of them.
/// 移除这些条目之后的清单；一条都没有时返回 `None`。
///
/// Only entries this action wrote are removed, matched on the exact quoted spelling: a workspace that
/// lists its members through a glob (`members = ["crates/*"]`) keeps that glob, and an entry that was
/// there before this action ran is left alone unless it is byte-identical to one of ours.
/// 只移除本动作写下的条目，按**逐字节的带引号拼写**匹配：用 glob 列成员的工作区（`members = ["crates/*"]`）
/// 保留那个 glob，而本动作运行前就存在的条目，除非与我们的某一条逐字节相同，否则不动。
pub(crate) fn stripped(text: &str, entries: &[String]) -> Result<Option<String>, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let Some(members) = locate(&lines)? else {
        return Ok(None);
    };
    if !members.items.iter().any(|item| entries.contains(item)) {
        return Ok(None);
    }
    let kept: Vec<String> = members
        .items
        .iter()
        .filter(|item| !entries.contains(item))
        .cloned()
        .collect();
    // Nothing is left of the list: the key goes with its entries, which is the shape the file had
    // before this action added them.
    // 清单什么都不剩：键与它的条目一起消失，也就是本动作加上它们之前那个文件的样子。
    if kept.is_empty() {
        lines.drain(members.key_line..=members.end_line);
        return Ok(Some(lines.join("\n") + "\n"));
    }
    if members.single_line() {
        let head = lines[members.key_line]
            .split_once('=')
            .expect("the key line has an `=`")
            .0
            .trim_end()
            .to_owned();
        lines[members.key_line] = format!("{head} = [{}]", kept.join(", "));
        return Ok(Some(lines.join("\n") + "\n"));
    }
    // The key line and the closing bracket keep their own text; only the item lines are rebuilt, at
    // the indentation the list already used.
    // 键所在的行与结束括号保留它们自己的文本；只有条目行按清单原本使用的缩进重建。
    let indent = members.item_indent(&lines);
    let close = lines[members.end_line].clone();
    let head = lines[members.key_line]
        .split_once('=')
        .expect("the key line has an `=`")
        .0
        .trim_end()
        .to_owned();
    let mut rebuilt = vec![format!("{head} = [")];
    rebuilt.extend(
        kept.iter()
            .map(|item| format!("{}{item},", " ".repeat(indent))),
    );
    rebuilt.push(close);
    lines.splice(members.key_line..=members.end_line, rebuilt);
    Ok(Some(lines.join("\n") + "\n"))
}

/// A path relative to the workspace root, with `/` separators, as cargo writes member paths.
/// 相对工作区根的路径，用 `/` 分隔，与 cargo 写成员路径的方式一致。
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
#[path = "crate_members_tests.rs"]
mod crate_members_tests;
