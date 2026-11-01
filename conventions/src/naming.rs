//! Crate, directory, and library names have to agree.
//! crate 名、目录名与 library 名必须一致。
//!
//! `AGENTS.md` change rule 4 states the rule (`xirang-<x>` / `<x>/` /
//! `xirang_<x>`) and says why it matters: scaffold templates in `build_method/`
//! and CI reference these names too. Nothing checked it, so the rule held only
//! while reviewers remembered it — and the failure is quiet in the worst way. A
//! directory whose crate is named something else still builds here; it breaks a
//! host that writes `xirang_<x>::…`, a scaffold that generates `<x>`, or a CI
//! step that runs `cargo test -p xirang-<x>`.
//! `AGENTS.md` 的改动规则 4 陈述了这条规则（`xirang-<x>` / `<x>/` / `xirang_<x>`）并说明
//! 了它为何重要：`build_method/` 里的脚手架模板与 CI 也引用这些名字。此前没有任何东西检查它，
//! 于是这条规则只在评审者记得时成立——而它的失败是最安静的那种：目录里那个 crate 叫别的名字
//! 在这里照样能构建，破的是写着 `xirang_<x>::…` 的宿主、生成 `<x>` 的脚手架、或跑
//! `cargo test -p xirang-<x>` 的 CI 步骤。
//!
//! The example hosts are outside the rule rather than an exception to it: they are a
//! different family (`xirang-example-*` with their own lib names), they are
//! `publish = false`, and `AGENTS.md` says a host documents its own types and is not
//! linted like a crate. Every other member is checked, including the gates crate.
//! 示例宿主在规则之外，而不是规则的例外：它们是另一个家族（`xirang-example-*` 与它们自己的
//! lib 名）、`publish = false`，而 `AGENTS.md` 说宿主自己负责文档化自己的类型、不像 crate 那样
//! 参与 lint。其余每个成员都被检查，门禁 crate 本身也不例外。

use std::fs;
use std::path::Path;

use crate::{crate_directories, is_real_directory, relative, rust_sources};

/// Directories whose library name deliberately does not follow `xirang_<dir>`.
/// 库名有意不遵循 `xirang_<dir>` 的目录。
///
/// One entry, measured rather than assumed: the kernel's library is `xirang`, not
/// `xirang_core`, and every host, README and doctest writes that name. The crate
/// table in `AGENTS.md` records it as `xirang-kernel (lib xirang)`.
/// 只有一项，且是实测而非假设：内核的 library 是 `xirang` 而不是 `xirang_core`，每个宿主、
/// README 与 doctest 都写这个名字。`AGENTS.md` 的 crate 表把它记为
/// `xirang-kernel (lib xirang)`。
pub const LIB_NAME_EXCEPTIONS: &[(&str, &str)] = &[];

/// One crate directory whose names disagree.
/// 一个名字互相不一致的 crate 目录。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// The crate directory, relative to the workspace root.
    /// crate 目录，以工作区根为基准。
    pub directory: String,
    /// What disagrees, with both values.
    /// 什么不一致，以及两个值。
    pub reason: String,
}

/// Every naming disagreement in the workspace, sorted by directory.
/// 工作区里每一处命名不一致，按目录排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    // Every member's package name, including the example hosts: they are outside
    // the naming rule below, but CI runs `cargo test -p xirang-example-…`, so a
    // reference to one of them is a reference to a real package.
    // 每个成员的包名，包括示例宿主：它们在下面的命名规则之外，但 CI 会跑
    // `cargo test -p xirang-example-…`，因此对它们的引用是对真实包的引用。
    let manifests: Vec<(std::path::PathBuf, String, String, String)> = crate_directories(root)
        .into_iter()
        .map(|directory| {
            let text = fs::read_to_string(directory.join("Cargo.toml"))
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
            (
                directory.clone(),
                relative(root, &directory),
                manifest_value(&text, "package", "name").unwrap_or_default(),
                text,
            )
        })
        .collect();
    let packages: Vec<String> = manifests
        .iter()
        .map(|(_, _, package, _)| package.clone())
        .filter(|package| !package.is_empty())
        .collect();
    for (directory, relative_directory, package, text) in &manifests {
        // The example hosts are a family of their own; see the module docs.
        // 示例宿主自成一个家族；见模块文档。
        if relative_directory.starts_with("examples/") {
            continue;
        }
        let Some(name) = directory.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let expected_package = format!("xirang-{}", name.replace('_', "-"));
        match package.as_str() {
            "" => found.push(Finding {
                directory: relative_directory.clone(),
                reason: "no `[package] name` in Cargo.toml".to_owned(),
            }),
            actual if actual == expected_package => {}
            actual => found.push(Finding {
                directory: relative_directory.clone(),
                reason: format!("package name is `{actual}`, not `{expected_package}`"),
            }),
        }
        let expected_lib = LIB_NAME_EXCEPTIONS
            .iter()
            .find(|(exception, _)| *exception == name)
            .map(|(_, lib)| (*lib).to_owned())
            // A Rust library name cannot carry a hyphen, so a hyphenated directory
            // (`plugin-host/`) maps to an underscore (`xirang_plugin_host`). This
            // gate reported the workspace's own `plugin-host` before the fold was
            // here, which is the pin below.
            // Rust 的 library 名不能带连字符，因此带连字符的目录（`plugin-host/`）映射为下划线
            // （`xirang_plugin_host`）。在这次折叠出现之前，本门禁曾把工作区自己的
            // `plugin-host` 报出来——正是下面那条钉子。
            .unwrap_or_else(|| format!("xirang_{}", name.replace('-', "_")));
        match manifest_value(text, "lib", "name") {
            Some(actual) if actual == expected_lib => {}
            Some(actual) => found.push(Finding {
                directory: relative_directory.clone(),
                reason: format!("lib name is `{actual}`, not `{expected_lib}`"),
            }),
            // No `[lib] name` means cargo derives it from the package name, which
            // the check above already tied to the directory.
            // 没有 `[lib] name` 表示 cargo 从包名推导它，而包名已被上面的检查绑到目录名。
            None => {}
        }
    }
    found.extend(referenced_names(root, &packages));
    found.sort();
    found.dedup();
    found
}

/// Every package name the scaffold templates and CI name, checked against the manifests.
/// 脚手架模板与 CI 指名的每个包名，与清单核对。
///
/// `AGENTS.md` change rule 4 says the scaffold templates in `build_method/` and CI
/// reference these names too, and the manifest walk above cannot see them: a rename
/// that misses a template produces a host whose generated manifest requires a crate
/// that does not exist, and a CI step that runs `cargo test -p <old name>` fails only
/// when it runs.
/// `AGENTS.md` 改动规则 4 说 `build_method/` 里的脚手架模板与 CI 也引用这些名字，而上面的清单
/// 遍历看不到它们：漏改一个模板会产出"生成的清单要求一个不存在的 crate"的宿主，而 CI 里
/// `cargo test -p <旧名>` 只会在真正跑到时才失败。
///
/// Boundary: only positions where a name is a *requirement* are read — a dependency
/// key (`xirang-x = { … }`) or `package = "xirang-x"` in a template, and a
/// `-p`/`--package` argument in a workflow. Everywhere else a `xirang-…` string is
/// a tool name, a job name, or prose, and reporting those would make the gate noise. A
/// comment is the loudest case of "elsewhere": it records what once ran, so a finding there
/// asks the author to rewrite history rather than to fix a requirement. Audit `G-07`.
/// 边界：只读名字处于**要求**位置的地方——模板里的依赖键（`xirang-x = { … }`）或
/// `package = "xirang-x"`，以及工作流里的 `-p`/`--package` 实参。其他位置的 `xirang-…`
/// 字符串是工具名、任务名或散文，报出来只会让门禁变成噪声。注释是"其他位置"里最响亮的一种：它记录
/// 的是"曾经这么跑过"，那里的发现要求作者改写记录，而不是修一条要求。审计 `G-07`。
fn referenced_names(root: &Path, packages: &[String]) -> Vec<Finding> {
    let mut found = Vec::new();
    let check = |path: &Path, name: &str, found: &mut Vec<Finding>| {
        if !packages.iter().any(|package| package == name) {
            found.push(Finding {
                directory: relative(root, path),
                reason: format!("names `{name}`, which is not a workspace package"),
            });
        }
    };
    let scaffold = root.join("toolchain/src/build_method/scaffold");
    if is_real_directory(&scaffold) {
        for path in rust_sources(&scaffold) {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for name in requirement_names(&without_comments(&text)) {
                check(&path, &name, &mut found);
            }
        }
    }
    let workflows = root.join(".github/workflows");
    if is_real_directory(&workflows) {
        let Ok(entries) = fs::read_dir(&workflows) else {
            return found;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // Both extensions are workflow files to GitHub, and reading only `.yml` made
            // every `.yaml` workflow invisible to this gate.
            // 两个扩展名对 GitHub 都是工作流文件，而只读 `.yml` 让每个 `.yaml` 工作流对本门禁
            // 不可见。
            if !path
                .extension()
                .is_some_and(|extension| extension == "yml" || extension == "yaml")
            {
                continue;
            }
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for name in package_arguments(&without_yaml_comments(&text)) {
                check(&path, &name, &mut found);
            }
        }
    }
    found
}

/// `text` with its comments blanked and its string literals kept.
/// `text` 中注释被抹白、字符串字面量被保留。
///
/// The boundary this gate states is "a name in a *requirement* position", and a comment is not
/// one: a template's `// … xirang-old = { … }` is a record of an invocation that once ran, so
/// reporting it asks the author to rewrite history. Its neighbour *is* one — a template writes
/// the requirement inside a string literal — which is why neither of the kernel's two masks
/// fits: `mask_non_code` blanks the literal too, and `mask_literals` keeps the comment. This is
/// the one caller that needs the other half of each rule, so it scans for that half itself.
/// 本门禁声明的边界是"处于**要求**位置的名称"，而注释不是要求位置：模板里的
/// `// … xirang-old = { … }` 是"曾经这么跑过"的记录，报出来等于要求作者改写记录。它旁边那句
/// **是**要求位置——模板把要求写在字符串字面量里——因此内核那两道 mask 都不合用：
/// `mask_non_code` 会把字面量一起抹白，`mask_literals` 会把注释留下。这里是唯一需要两者另一半的
/// 调用方，于是自己扫出那一半。
fn without_comments(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut kept = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < characters.len() {
        let character = characters[index];
        // A quoted literal is verbatim text: a `//` inside it is part of a URL or a path, and
        // the requirement this gate reads is written inside one.
        // 引号字面量是原文：其中的 `//` 是 URL 或路径的一部分，而本门禁要读的要求正写在里面。
        if character == '"' {
            kept.push(character);
            index += 1;
            while index < characters.len() {
                let inner = characters[index];
                kept.push(inner);
                index += 1;
                if inner == '\\' && index < characters.len() {
                    kept.push(characters[index]);
                    index += 1;
                } else if inner == '"' {
                    break;
                }
            }
            continue;
        }
        if character == '/' && characters.get(index + 1) == Some(&'/') {
            while index < characters.len() && characters[index] != '\n' {
                kept.push(' ');
                index += 1;
            }
            continue;
        }
        if character == '/' && characters.get(index + 1) == Some(&'*') {
            let mut depth = 0usize;
            while index < characters.len() {
                let inner = characters[index];
                if inner == '/' && characters.get(index + 1) == Some(&'*') {
                    depth += 1;
                    kept.push_str("  ");
                    index += 2;
                    continue;
                }
                if inner == '*' && characters.get(index + 1) == Some(&'/') {
                    depth = depth.saturating_sub(1);
                    kept.push_str("  ");
                    index += 2;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                // Line breaks stay, so the text keeps its shape.
                // 换行保留，文本因此保持原形。
                kept.push(if inner == '\n' { '\n' } else { ' ' });
                index += 1;
            }
            continue;
        }
        kept.push(character);
        index += 1;
    }
    kept
}

/// `text` with its YAML comments blanked, quoted scalars respected.
/// `text` 中 YAML 注释被抹白，引号标量不受影响。
///
/// YAML starts a comment at a `#` that is at the start of a line or preceded by whitespace, so
/// `echo a#b` keeps its `#` while `# old: cargo test -p xirang-old` does not.
/// YAML 的注释从行首的 `#` 或前面有空白字符的 `#` 开始，因此 `echo a#b` 保留它的 `#`，而
/// `# old: cargo test -p xirang-old` 不会。
fn without_yaml_comments(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let mut quote: Option<char> = None;
        let mut comment: Option<usize> = None;
        for (offset, character) in line.char_indices() {
            match quote {
                Some(open) => {
                    if character == open {
                        quote = None;
                    }
                }
                None => match character {
                    '"' | '\'' => quote = Some(character),
                    '#' => {
                        let preceded_by_space = line[..offset]
                            .chars()
                            .next_back()
                            .is_none_or(char::is_whitespace);
                        if preceded_by_space {
                            comment = Some(offset);
                            break;
                        }
                    }
                    _ => {}
                },
            }
        }
        match comment {
            Some(offset) => {
                kept.push_str(&line[..offset]);
                if line.ends_with('\n') {
                    kept.push('\n');
                }
            }
            None => kept.push_str(line),
        }
    }
    kept
}

/// `xirang-…` names in dependency-key or `package = "…"` position in a template.
/// 模板中处于依赖键位置或 `package = "…"` 位置的 `xirang-…` 名。
fn requirement_names(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut from = 0usize;
    while let Some(offset) = text[from..].find("xirang-") {
        let start = from + offset;
        from = start + "xirang-".len();
        let tail: String = text[from..]
            .chars()
            .take_while(|character| character.is_ascii_lowercase() || *character == '-')
            .collect();
        if tail.is_empty() {
            continue;
        }
        let after = text[from + tail.len()..].trim_start();
        // Only `=`: a template that merely starts with the name is an identifier
        // being built (`format!("xirang-auto-{}-{}", …)`), not a requirement.
        // 只认 `=`：仅仅以该名字开头的模板是在拼标识符（`format!("xirang-auto-{}-{}", …)`），
        // 不是一条依赖要求。
        if after.starts_with('=') {
            found.push(format!("xirang-{tail}"));
        }
    }
    let mut from = 0usize;
    while let Some(offset) = text[from..].find("package") {
        let at = from + offset;
        from = at + "package".len();
        // The word has to be a field name: `my_package = "…"` and `--package="…"` are not
        // `package = "…"`, and a bare substring search reads both.
        // 这个词必须是字段名：`my_package = "…"` 与 `--package="…"` 都不是 `package = "…"`，
        // 而单纯的子串搜索会把两者都读成字段。
        let preceded_by_name = text[..at].chars().next_back().is_some_and(|character| {
            character.is_alphanumeric() || character == '_' || character == '-'
        });
        if preceded_by_name {
            continue;
        }
        let Some(value) = package_field_value(text, from) else {
            continue;
        };
        if value.starts_with("xirang-") {
            found.push(value);
        }
    }
    found
}

/// The value of the `package = "…"` field that starts at `from`, in either spelling.
/// 从 `from` 处开始的 `package = "…"` 字段的取值，两种拼法都算。
///
/// The whitespace around `=` is optional, and the opening quote may carry a backslash: a
/// template is Rust source, so the manifest line it generates is written inside a string
/// literal, where every quote is escaped (`package = \"xirang-kernel\"`). Reading only the
/// unescaped needle found nothing there, and the loss was invisible while a dependency key and
/// its package name agreed — the key half found the name anyway. A *renamed* dependency has
/// only this field, so a template that renamed one stayed silent (audit `G-07`, finding `F-3`).
/// `=` 两侧的空白可有可无，开引号还可以带反斜杠：模板是 Rust 源码，它生成的清单行写在字符串
/// 字面量里，因此每个引号都是转义的（`package = \"xirang-kernel\"`）。只认未转义的针在那里什么
/// 也找不到，而只要依赖键与包名一致，这个损失就不可见——键那半边反正能找到那个名字。**被重命名**
/// 的依赖只有这个字段，于是一条重命名的模板会保持沉默（审计 `G-07`，发现 `F-3`）。
fn package_field_value(text: &str, from: usize) -> Option<String> {
    let rest = text[from..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('\\').unwrap_or(rest);
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    // The closing quote is escaped the same way the opening one is, so the backslash that
    // escapes it is not part of the name: `\"xirang-toolchain\"` names one package, and the
    // shipped-tree pin reported a name ending in a backslash before this strip existed.
    // 闭引号与开引号同样被转义，因此转义它的反斜杠不属于包名：`\"xirang-toolchain\"` 点名的
    // 是一个包——在这句剥离出现之前，出厂树那条钉子报出的名字是以反斜杠结尾的。
    let value = rest[..end].strip_suffix('\\').unwrap_or(&rest[..end]);
    Some(value.to_owned())
}

/// `xirang-…` names passed as `-p`/`--package` to a command in a workflow.
/// 工作流中作为 `-p`/`--package` 传给命令的 `xirang-…` 名。
fn package_arguments(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    // The flag is matched without requiring the space that used to be part of the needle:
    // `--package=xirang-typo` is the same argument as `--package xirang-typo`, and
    // `-p a,xirang-typo` names two packages in one argument. Only the argument itself is
    // read for names, so a list elsewhere on the line is not mistaken for one.
    // 匹配标志时不再把那个空格当作针的一部分：`--package=xirang-typo` 与
    // `--package xirang-typo` 是同一个实参，而 `-p a,xirang-typo` 在一个实参里点了两个包。
    // 只在实参本身里读取名字，因此同一行别处的列表不会被误当成本次实参。
    for needle in ["-p", "--package"] {
        let mut from = 0usize;
        while let Some(offset) = text[from..].find(needle) {
            let at = from + offset;
            from = at + needle.len();
            let boundary = at == 0
                || text[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|previous| previous.is_whitespace() || previous == ':');
            if !boundary {
                continue;
            }
            let rest = text[from..].trim_start_matches(['=', ' ']);
            let argument: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
            for name in argument.split(',') {
                let name: String = name
                    .chars()
                    .take_while(|character| {
                        character.is_ascii_alphanumeric() || *character == '-' || *character == '_'
                    })
                    .collect();
                if name.starts_with("xirang-") {
                    found.push(name);
                }
            }
        }
    }
    found
}

/// The value of `key` inside `[section]`, with quotes and trailing comments removed.
/// `[section]` 中 `key` 的取值，已去掉引号与行尾注释。
fn manifest_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut current = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            current = header.trim_end_matches(']').trim().to_owned();
            continue;
        }
        if current != section || trimmed.starts_with('#') {
            continue;
        }
        let Some((left, right)) = trimmed.split_once('=') else {
            continue;
        };
        if left.trim() != key {
            continue;
        }
        let value = right.trim();
        let value = value.split('#').next().unwrap_or(value).trim();
        return Some(value.trim_matches(['"', '\'']).to_owned());
    }
    None
}

#[cfg(test)]
#[path = "naming_tests.rs"]
mod naming_tests;
