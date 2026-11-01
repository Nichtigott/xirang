//! Manifest-shape rules for features and targets, as a gate.
//! 面向特性与 target 的清单形状规则，作为门禁。
//!
//! `AGENTS.md` states two rules about feature-gated targets in prose: a target that only
//! works inside this checkout carries `required-features` (Cargo's per-target equivalent of
//! `publish = false`), and a feature that gates test-only fixtures stays off by default.
//! Nothing executed them — deleting `required-features = ["dev-supervisor"]` from
//! `studio/Cargo.toml` left every other gate silent, and `cargo install xirang-toolchain`
//! would then ship a tool with nothing to rebuild. A rule that only prose states decays, so
//! this module reads every member manifest and refuses those shapes.
//! `AGENTS.md` 用散文陈述了两条关于按特性门控的 target 的规则：只在本检出内工作的 target 带
//! `required-features`（Cargo 按 target 的 `publish = false` 等价物），门控测试用 fixture 的特性
//! 保持默认关闭。没有任何东西执行它们——从 `studio/Cargo.toml` 删掉
//! `required-features = ["dev-supervisor"]`，其余门禁全部沉默，而 `cargo install
//! xirang-toolchain` 会装出一个没有东西可重建的工具。只有散文陈述的规则会腐化，因此本模块读取
//! 每个成员的清单并拒绝那些形状。
//!
//! A third rule is about the documents rather than a manifest: the install command the
//! READMEs print has to install a binary. `toolchain` gates every binary behind a feature, so
//! `cargo install xirang-toolchain` on its own installs nothing at all — Cargo says so and
//! exits successfully, which is the failure mode nobody notices. The rule reads each
//! **fenced** `cargo install`, resolves the features it names against `[features]`, and
//! refuses a command that leaves `xirang` uninstalled. Prose is deliberately outside the
//! rule: the READMEs explain the defect in a sentence, and a sentence that says a command
//! installs nothing is not a command a reader copies. The boundary has its own test, so
//! widening this to prose has to update the contract first.
//! 第三条规则针对文档而不是清单：README 打印的安装命令必须真的装上一个二进制。`toolchain`
//! 把每个二进制都门控在某个特性之后，因此裸 `cargo install xirang-toolchain` 什么都不装——
//! Cargo 会这么说，然后成功退出，这正是没人会注意到的失败形态。规则读取**围栏里**的每条
//! `cargo install`，把它点名的特性对着 `[features]` 求闭包，并拒绝让 `xirang` 装不上的命令。
//! 散文被有意排除在规则之外：README 会用一句话解释这个缺陷，而"某条命令什么都不装"的句子不是
//! 读者会照抄的命令。这条边界有自己的测试，因此把它扩到散文必须先改契约。

use std::fs;
use std::path::Path;

/// Targets that must not be installable from the published package, as
/// `(crate directory, target name, the feature that keeps it out)`.
/// 不得从已发布包安装的 target，形如 `(crate 目录, target 名, 把它挡在外面的特性)`。
///
/// The rule needs the target named here rather than derived: nothing in the manifest says
/// "this binary only works in this checkout" — `required-features` *is* that statement, and
/// the point of the gate is to notice when the statement goes missing.
/// 这条规则需要在这里点名 target，而不是推导出来：清单里没有任何东西说"这个二进制只在本检出里
/// 工作"——`required-features` **就是**那句话，而门禁的意义正是在那句话消失时发现它。
const WORKSPACE_ONLY_TARGETS: &[(&str, &str, &str)] =
    &[("toolchain", "xirang-dev", "dev-supervisor")];

/// Features that must stay off by default, as `(crate directory, feature)`.
/// 必须保持默认关闭的特性，形如 `(crate 目录, 特性)`。
///
/// `prototype-fixtures` gates test-only fixtures rather than public API, so a published
/// package that turned it on would ship a fixture host nobody asked for.
/// `prototype-fixtures` 门控的是测试用 fixture 而非公开 API，因此打开它的已发布包会带上一个没人
/// 要的 fixture 宿主。
const OFF_BY_DEFAULT: &[(&str, &str)] = &[("toolchain", "prototype-fixtures")];

/// Report every manifest whose feature-gated target shapes break the rules.
/// 报告每个在按特性门控的 target 形状上破坏规则的清单。
pub fn findings(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for directory in crate::crate_directories(root) {
        let manifest = directory.join("Cargo.toml");
        let name = directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let features = feature_names(&text);
        let default = default_features(&text);
        let targets = target_requirements(&text);
        for (target, required) in &targets {
            for feature in required {
                if !features.contains(feature) {
                    found.push(format!(
                        "{name}: target `{target}` requires feature `{feature}`, which the \
                         manifest does not declare"
                    ));
                } else if default.contains(feature) {
                    found.push(format!(
                        "{name}: target `{target}` requires default feature `{feature}`, so the \
                         requirement keeps nothing out; `required-features` has to name a \
                         feature that is off by default"
                    ));
                }
            }
        }
        for (directory_name, target, feature) in WORKSPACE_ONLY_TARGETS {
            if name != *directory_name {
                continue;
            }
            let carried = targets.iter().any(|(candidate, required)| {
                candidate == target && required.iter().any(|one| one == feature)
            });
            if !carried {
                found.push(format!(
                    "{name}: target `{target}` must carry `required-features = \
                     [\"{feature}\"]`; without it `cargo install` ships a tool that can only \
                     work inside this checkout"
                ));
            }
        }
        for (directory_name, feature) in OFF_BY_DEFAULT {
            if name != *directory_name {
                continue;
            }
            if default.iter().any(|one| one == feature) {
                found.push(format!(
                    "{name}: `{feature}` gates test-only fixtures and must stay off by default"
                ));
            }
        }
    }
    found.extend(install_findings(root));
    found
}

/// Documents that print install commands a reader will copy.
/// 打印读者会照抄的安装命令的文档。
const INSTALL_DOCUMENTS: &[&str] = &["README.md", "README.zh-CN.md"];

/// The documented `cargo install` commands, as `(document, line, command)`.
/// 文档里写下的 `cargo install` 命令，形如 `(文档, 行号, 命令)`。
///
/// Only fenced lines are read: a fence is the copy-paste surface, while a sentence may
/// discuss the same command to explain what goes wrong with it. A command may wrap onto the
/// next fenced line, so the window is that line and its successor; a trailing `# comment`
/// and a closing period are cut off.
/// 只读围栏里的行：围栏是"照抄"这件事的表面，而散文可以用同一句话来解释它哪里出了问题。命令
/// 可能折到围栏里的下一行，因此窗口是那一行与它后面一行；行尾的 `# 注释` 与句号会被切掉。
fn install_commands(root: &Path) -> Vec<(String, usize, String)> {
    let mut found = Vec::new();
    for document in INSTALL_DOCUMENTS {
        let Ok(text) = fs::read_to_string(root.join(document)) else {
            continue;
        };
        let fenced = fenced_lines(&text);
        for (position, (number, line)) in fenced.iter().enumerate() {
            let Some(at) = line.find("cargo install") else {
                continue;
            };
            let mut command = line[at..].trim().to_owned();
            if let (true, Some((_, next))) = (
                !command.contains("xirang-toolchain"),
                fenced.get(position + 1),
            ) {
                command.push(' ');
                command.push_str(next.trim());
            }
            if !command.contains("xirang-toolchain") {
                continue;
            }
            let command = command
                .split(" #")
                .next()
                .unwrap_or_default()
                .trim_end_matches(['.', ','])
                .trim()
                .to_owned();
            found.push(((*document).to_owned(), *number, command));
        }
    }
    found
}

/// The lines inside fenced code blocks, as `(line number, text)`.
/// 围栏代码块里的行，形如 `(行号, 文本)`。
fn fenced_lines(text: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut open = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            open = !open;
            continue;
        }
        if open {
            found.push((index + 1, line));
        }
    }
    found
}

/// The features one documented command names, from `--features <list>` or `--features=<list>`.
/// 一条文档命令点名的特性，来自 `--features <列表>` 或 `--features=<列表>`。
fn documented_features(command: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut tokens = command.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        let list = if let Some(rest) = token.strip_prefix("--features=") {
            rest.to_owned()
        } else if token == "--features" || token == "-F" {
            tokens.next().unwrap_or_default().to_owned()
        } else {
            continue;
        };
        for name in list.split([',', ' ']) {
            let name = name.trim_matches(|character: char| character == '"' || character == '\'');
            if !name.is_empty() {
                found.push(name.to_owned());
            }
        }
    }
    found
}

/// The features a starting list turns on, following this crate's own implications.
/// 一组起始特性打开了哪些特性，沿着本 crate 自己的蕴含关系走。
///
/// `dep:serde_json` and `xirang-kernel/syntax` name a dependency rather than a feature, so
/// they are skipped: opening a feature never installs a binary on its own.
/// `dep:serde_json` 与 `xirang-kernel/syntax` 点名的是依赖而不是特性，因此跳过：打开一个特性本身
/// 从不安装二进制。
fn feature_closure(text: &str, start: &[String]) -> Vec<String> {
    let members: Vec<(String, Vec<String>)> = block(text, "[features]")
        .map(|body| {
            body.lines()
                .filter_map(|line| {
                    line.split_once('=')
                        .map(|(key, rest)| (key.trim().to_owned(), crate::quoted(rest)))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut enabled: Vec<String> = Vec::new();
    let mut queue: Vec<String> = start.to_vec();
    while let Some(feature) = queue.pop() {
        if enabled.contains(&feature) {
            continue;
        }
        enabled.push(feature.clone());
        if let Some((_, list)) = members.iter().find(|(name, _)| *name == feature) {
            for member in list {
                if member.contains(':') || member.contains('/') {
                    continue;
                }
                if !enabled.contains(member) {
                    queue.push(member.clone());
                }
            }
        }
    }
    enabled
}

/// Report every documented install command that would install no `xirang` binary.
/// 报告每条装不出 `xirang` 二进制的文档安装命令。
fn install_findings(root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join("toolchain").join("Cargo.toml")) else {
        return Vec::new();
    };
    let declared = feature_names(&text);
    let default = default_features(&text);
    let targets = target_requirements(&text);
    let mut found = Vec::new();
    for (document, line, command) in install_commands(root) {
        let named = documented_features(&command);
        for feature in &named {
            if !declared.contains(feature) {
                found.push(format!(
                    "{document}:{line}: the documented command `{command}` names feature \
                     `{feature}`, which `toolchain/Cargo.toml` does not declare"
                ));
            }
        }
        let start = if named.is_empty() {
            default.clone()
        } else {
            named
        };
        let enabled = feature_closure(&text, &start);
        let installs = targets.iter().any(|(target, required)| {
            target == "xirang" && required.iter().all(|feature| enabled.contains(feature))
        });
        if !installs {
            found.push(format!(
                "{document}:{line}: `{command}` installs no `xirang` binary — every `[[bin]]` \
                 in this package carries `required-features`, so the documented command has to \
                 enable the features that install the CLI (for example `--features cli`)"
            ));
        }
    }
    found
}

/// Every block header's body, as `(header, body)`, in the order written.
/// 每个段落头及其正文，形如 `(头, 正文)`，按书写顺序。
///
/// Comments are dropped, and a header is a line that is nothing but `[name]` or `[[name]]`:
/// a key whose value happens to be bracketed stays where it belongs.
/// 注释被丢弃；段落头是整行只有 `[名]` 或 `[[名]]` 的行：取值恰好带方括号的键留在原处。
fn blocks(text: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim_end();
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            found.push((trimmed.to_owned(), String::new()));
            continue;
        }
        if let Some((_, body)) = found.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    found
}

/// The value of `key = …` in one block's body, with a wrapped array joined onto one line.
/// 某个段落正文里 `key = …` 的取值，被折行的数组会接成一行。
fn value(body: &str, key: &str) -> Option<String> {
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let Some((name, rest)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        let mut value = rest.trim().to_owned();
        while value.starts_with('[') && !value.contains(']') {
            match lines.next() {
                Some(next) => {
                    value.push(' ');
                    value.push_str(next.trim());
                }
                None => break,
            }
        }
        return Some(value);
    }
    None
}

/// The body of the first block whose header is `header`.
/// 第一个头为 `header` 的段落的正文。
fn block(text: &str, header: &str) -> Option<String> {
    blocks(text)
        .into_iter()
        .find(|(candidate, _)| candidate == header)
        .map(|(_, body)| body)
}

/// The feature names a manifest declares.
/// 清单声明的特性名。
fn feature_names(text: &str) -> Vec<String> {
    block(text, "[features]")
        .map(|body| {
            body.lines()
                .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim().to_owned()))
                .filter(|key| !key.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The features the `default` array turns on.
/// `default` 数组打开的哪些特性。
fn default_features(text: &str) -> Vec<String> {
    block(text, "[features]")
        .and_then(|body| value(&body, "default"))
        .map(|value| crate::quoted(&value))
        .unwrap_or_default()
}

/// Every declared target, as `(name, the features it requires)`.
/// 每个声明的 target，形如 `(名字, 它要求的特性)`。
fn target_requirements(text: &str) -> Vec<(String, Vec<String>)> {
    blocks(text)
        .into_iter()
        .filter(|(header, _)| {
            matches!(
                header.as_str(),
                "[[bin]]" | "[[example]]" | "[[test]]" | "[[bench]]"
            )
        })
        .map(|(_, body)| {
            let name = value(&body, "name")
                .map(|value| crate::quoted(&value).into_iter().next().unwrap_or_default())
                .unwrap_or_default();
            let required = value(&body, "required-features")
                .map(|value| crate::quoted(&value))
                .unwrap_or_default();
            (name, required)
        })
        .collect()
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod features_tests;
