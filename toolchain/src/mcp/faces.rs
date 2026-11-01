//! Which feature faces a tree compiles, and which one the default face is.
//! 一棵树编译哪些特性面，以及默认面是哪一个。
//!
//! Measured need: a defect in a file compiled only under a non-default feature cannot fail on the
//! default face, and the evaluation had exactly that round — `studio` is not in
//! `xirang-toolchain`'s `default`, so `cargo test --workspace` was green while the red was one
//! feature away. Neither this bridge nor the control tool could say which face a red run would
//! appear on, and both agents had to widen the feature set by hand.
//! 量出来的需求：只在非默认特性下编译的文件里的缺陷，在默认面上不可能失败，而评测里就有这么一轮——
//! `studio` 不在 `xirang-toolchain` 的 `default` 里，于是 `cargo test --workspace` 全绿、红只差一个
//! 特性。桥与对照工具都答不了"红会出现在哪个面"，两个代理都只能自己把特性面加宽。
//!
//! This answers the actionable half from Cargo's own view of the tree — **which faces exist and
//! which one is the default** — and says plainly what it cannot answer: which face is *red* is a
//! fact about a test run, not about the tree, and this module never guesses it. The design, and the
//! separately-decided opt-in that would observe a run, are in `docs/design-mcp-test-faces.md`.
//! 这里答的是可行动的那一半，取自 Cargo 自己对这棵树的看法——**有哪些面、哪个是默认面**——并明说它
//! 答不了什么：哪个面**是红的**是关于一次测试运行的事实、不是关于这棵树的事实，本模块从不猜它。设计、
//! 以及那个"观测一次运行"的、需单独拍板的选择，见 `docs/design-mcp-test-faces.md`。

//! online: the count must agree with what `registry` derives from the sources, and a face declared
//! since the build — or a Python declaration the build does not register yet — is only visible there.
//! 在线推导：这个数字必须与 `registry` 从源码推导的答案一致，而构建之后才声明的面——或者构建尚未
//! 登记的 Python 声明——只在源码里看得见。

use std::path::Path;

use serde_json::Value;

/// The feature lines for the tree under this root, and the note that makes them actionable.
/// 这个根下那棵树的特性行，以及让它们可行动的那条备注。
///
/// The label is `features`, because `face` already means the registration face here.
/// 标签写作 `features`，因为 `face` 在本仓库里已经指注册面。
///
/// A tree Cargo cannot describe answers one line naming the reason rather than nothing: an answer
/// that silently dropped the faces would read as "this tree has none".
/// Cargo 描述不了的树会回一行原因，而不是什么都不回：静默丢掉特性面的答案会被读成"这棵树没有面"。
pub(crate) fn faces_lines(root: &Path) -> Vec<String> {
    let manifest = root.join("Cargo.toml");
    let metadata = match crate::mcp::workspace::metadata_json(&manifest) {
        Ok(metadata) => metadata,
        Err(reason) => return vec![format!("features  unavailable ({reason})")],
    };
    let mut lines = Vec::new();
    let mut any_non_default = false;
    for package in metadata["packages"].as_array().into_iter().flatten() {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        let features = package["features"].as_object();
        let default = features
            .and_then(|features| features.get("default"))
            .and_then(Value::as_array)
            .map(|values| names(values))
            .unwrap_or_default();
        let all = features
            .map(|features| {
                features
                    .keys()
                    .filter(|key| key.as_str() != "default")
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !all.is_empty() {
            any_non_default = true;
        }
        let required = package["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|target| {
                let name = target["name"].as_str()?;
                let required = target["required-features"].as_array()?;
                (!required.is_empty()).then(|| format!("{name} -> {}", bracket(&names(required))))
            })
            .collect::<Vec<_>>();
        // `features`, not `faces`: this line is about **Cargo features**, while `face` in this
        // repository means the registration face — and `registry` counts those. Labelling it `faces`
        // is what produced the 2026-10-10 queue item ⑥, where `status` printed `faces … default=[]
        // all=[]` (true of the feature table) next to a `registry` listing three registration faces,
        // and a reader saw one of them as the lie (4 reproductions; report §7h).
        // 这行说的是 **Cargo 特性**，而本仓库里 `face` 指注册面——`registry` 数的正是后者。把它标成
        // `faces` 就是 2026-10-10 队列第 ⑥ 条的成因：`status` 印出 `faces … default=[] all=[]`（对特性表
        // 而言为真），旁边 `registry` 却列出三个注册面，读者只能把其中一个当成假话（4 次复现；报告 §7h）。
        lines.push(format!(
            "features  {name}: default={} all={}{}",
            bracket(&default),
            bracket(&all),
            if required.is_empty() {
                String::new()
            } else {
                format!("  required=[{}]", required.join(", "))
            }
        ));
    }
    if lines.is_empty() {
        return vec!["features  none (Cargo reported no package for this manifest)".to_owned()];
    }
    if any_non_default {
        lines.push(
            "note   a file compiled only under a non-default feature cannot fail on the default \
             face: run `cargo test --all-features` (or `--features <name>`) before believing a \
             green default run"
                .to_owned(),
        );
    }
    lines
}

/// The string values of a JSON array, in the order Cargo wrote them.
/// JSON 数组里的字符串值，按 Cargo 写下的顺序。
fn names(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// A comma-separated list in brackets, or `[]`.
/// 方括号里的逗号分隔清单，或 `[]`。
fn bracket(names: &[String]) -> String {
    format!("[{}]", names.join(", "))
}

/// How many **registration faces** this tree declares, and the line that says so.
/// 这棵树声明了多少个**注册面**，以及说出这件事的那一行。
///
/// The other half of this module's word: the lines above are about Cargo **feature** faces, while
/// `face` on its own in this repository means the registration face and `registry` counts those. The
/// 2026-10-10 defect queue item ⑥ (4 reproductions) is what leaving the two unlabelled cost: `status`
/// printed `faces … default=[] all=[]` beside a `registry` that listed three registration faces, and
/// a reader had to pick one of them to be the lie (report §7h). Deriving the count the same way
/// `registry` derives it is what makes the two answers unable to disagree.
/// 本模块这个词的另一半：上面那些行说的是 Cargo **特性**面，而本仓库里单独的 `face` 指注册面、
/// `registry` 数的就是它。2026-10-10 缺陷队列第 ⑥ 条（4 次复现）正是"两个都不加标签"的代价：`status`
/// 印 `faces … default=[] all=[]`，旁边 `registry` 列出三个注册面，读者只能挑一个当假话（报告 §7h）。
/// 用 `registry` 同一套推导算这个数，正是让两个答案不可能不一致的原因。
pub(crate) fn registered_faces_line(root: &Path) -> (String, Option<usize>) {
    let derived = crate::mcp::registry::namespace(root)
        .ok()
        .and_then(|namespace| {
            let views = crate::build_method::face_views_with_external(root, &namespace).ok()?;
            Some((namespace, views))
        });
    match derived {
        Some((namespace, (faces, _, external))) => {
            // A declaration the build does not register is the one thing a bare count would hide:
            // the number would be right about the tree and wrong about what the package declares.
            // 一条构建未登记的声明正是裸计数会藏起来的东西：数字对得上树，却对不上这个包声明了什么。
            // Each language's gap is named separately: "3 declarations" without the language reads
            // like one language's count, while the fix differs per carrier.
            // 每种语言的缺口分开点名：只写"3 条声明"会读成某一种语言的计数，而修法按载体不同。
            let mut suffix = String::new();
            for (needle, language) in [
                ("python face", "Python"),
                ("javascript face", "JavaScript"),
                ("typescript face", "TypeScript"),
            ] {
                let unregistered = external.iter().filter(|line| line.contains(needle)).count();
                if unregistered > 0 {
                    suffix.push_str(&format!(
                        "; {unregistered} declared {language} face(s) are not registered yet"
                    ));
                }
            }
            // The ledger is the second half of "what does this package declare": a mapping that the
            // tree does not follow describes a project that no longer exists, and silence about it is
            // what lets the next `add` place a child under a parent that is not there any more.
            // 台账是"这个包声明了什么"的另一半：树没有遵守的映射描述的是一个已经不存在的项目，而对它保持
            // 沉默，正是让下一次 `add` 把子面放到一个已经不在了的父级下的原因。
            if let Ok(layout) = crate::build_method::source_layout(root) {
                let observed: Vec<_> = faces
                    .iter()
                    .map(|face| face.observed(&layout, &namespace))
                    .collect();
                match xirang_multilang::object_map::ObjectMap::read_from(root) {
                    Ok(map) => {
                        let drift = map.drift(&observed);
                        let mapped = map
                            .entries()
                            .iter()
                            .filter(|entry| entry.directory.is_some())
                            .count();
                        if mapped > 0 {
                            suffix.push_str(&format!(
                                "; {mapped} object(s) have an explicit directory mapping"
                            ));
                        }
                        if !drift.is_empty() {
                            suffix.push_str(&format!(
                                "; {} mapping(s) are not followed ({})",
                                drift.len(),
                                drift.join(" | ")
                            ));
                        }
                    }
                    Err(error) => {
                        suffix.push_str(&format!("; the object ledger is unreadable: {error}"))
                    }
                }
            }
            (
                format!(
                    "registered faces {}{suffix} (derived from the sources; `registry` prints the \
                     same faces when its record covers them, and derives with its reason when it \
                     does not)",
                    faces.len()
                ),
                Some(faces.len()),
            )
        }
        None => (
            "registered faces unknown (the namespace or the sources could not be read)".to_owned(),
            None,
        ),
    }
}

#[cfg(test)]
#[path = "faces_tests.rs"]
mod faces_tests;
