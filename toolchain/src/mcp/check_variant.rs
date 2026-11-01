//! Immutable verification variants: a declared fault injected into a private copy, never the base tree.
//! 不可变的验证变体：把声明的故障注入一份私有副本，绝不注入基树。
//!
//! A counterproof answers "does the protection actually catch this fault", and the only honest way to
//! ask is to *have* the fault. Reverting the fix inside the working tree answers it too, at the cost of
//! a full `cargo clean` afterwards and of a window in which the base tree is wrong — the shape the
//! benchmark measured. Here the fault lives in a copy under the plan's own output directory, named by
//! the **content digest of the declared edits**, so two variants with identical mtimes and different
//! bodies cannot share a build directory and a variant can never be published as the candidate.
//! 反证回答的是"这个保护真的抓得住这个故障吗"，而唯一诚实的问法是**让故障真的存在**。在工作树里恢复旧实现
//! 也能回答，代价是事后一次全仓 `cargo clean`，以及一段基树本身是错的窗口——正是基准量到的形状。这里故障住在
//! 计划自己输出目录下的一份副本里，目录名取自**声明改动的内文摘要**，因此 mtime 相同、正文不同的两个变体不可能
//! 共用构建目录，而变体也绝无可能被当成候选项发布出去。

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The largest single file a variant may write, so one declared edit cannot become a payload channel.
/// 变体单个文件可写入的上限，使一条声明的改动不会变成一条载荷通道。
const MAX_EDIT_BYTES: usize = 256 * 1024;

/// The most edits one variant may declare.
/// 一个变体可以声明的改动条数上限。
const MAX_EDITS: usize = 32;

/// One declared change to a variant copy.
/// 对变体副本的一条声明改动。
pub(super) struct Edit {
    /// Package-relative path, as the tree spells it.
    /// 包内相对路径，按树里的拼法。
    pub(super) path: String,
    /// The bytes to write, or `None` to delete the file.
    /// 要写入的字节；`None` 表示删除该文件。
    pub(super) content: Option<String>,
}

/// What a variant is for: proving a protection catches a fault, or proving consumers follow a
/// replacement.
/// 变体是为什么而做的：证明某个保护抓得住故障，或证明消费者跟随替换件。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// A counterproof: the declared fault must be **caught** by the named test.
    /// 反证：声明的故障必须被点名的测试**抓住**。
    Fault,
    /// A legal replacement: the consumers must still follow the new provider and still pass.
    /// 合法替换：消费者必须仍然跟随新的提供者并且仍然通过。
    Replacement,
}

/// A route the variant must keep: after the edits, this caller must still call this callee.
/// 变体必须保持的一条路线：改动之后，这个调用者必须仍然调用这个被调用者。
///
/// This is how "the consumer copied the formula and stopped delegating" is detected. Behaviour alone
/// cannot see it — a local re-implementation of the same formula passes every behavioural test — so the
/// judgement has to be about the **route**, and it has to be made on the variant's own bytes.
/// 这就是"消费者把公式抄了过去、不再委派"被检出的方式。只看行为看不见它——同一公式的本地重写会通过每一
/// 条行为测试——因此判定必须是关于**路线**的，而且必须落在变体自己的字节上。
pub(super) struct Route {
    /// Package-relative path of the file that must keep calling.
    /// 必须继续调用的那个文件的包内相对路径。
    pub(super) caller: String,
    /// The function name the call must resolve to.
    /// 这次调用必须解析到的函数名。
    pub(super) callee: String,
}

/// The edits that turn the base tree into one variant copy.
/// 把基树变成一个变体副本的那些改动。
pub(super) struct Variant {
    edits: Vec<Edit>,
    kind: Kind,
    route: Option<Route>,
}

impl Variant {
    /// Validate one declared variant against the case it belongs to.
    /// 针对它所属的用例校验一个声明的变体。
    ///
    /// `test` is the test the case's `expect` names, and a variant may **not** edit the file that
    /// declares it: a counterproof whose protection test was rewritten to match the answer proves
    /// nothing, and that is the one thing a variant author must not be able to do.
    /// `test` 是用例 `expect` 点名的测试，而变体**不得**改动声明它的那个文件：一个把保护测试改成迎合答案的反证
    /// 什么都证明不了，而这正是变体作者绝不能做到的那件事。
    /// `expected` is the case's own expectation, because the declared purpose and the declared
    /// outcome are one decision: a counterproof's expectation is `failed`, a replacement's is `passed`.
    /// `expected` 是用例自己的预期，因为声明的用途与声明的结果是一个决定：反证的预期是 `failed`，
    /// 替换件的预期是 `passed`。
    pub(super) fn parse(
        root: &Path,
        case: &str,
        value: &Value,
        test: Option<&str>,
        expected: &str,
    ) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| format!("case `{case}`: variant must be an object"))?;
        if object
            .keys()
            .any(|key| !["edits", "kind", "route"].contains(&key.as_str()))
        {
            return Err(format!(
                "case `{case}`: variant accepts only `edits`, `kind` and `route`; a variant that could name a root, a target directory or a command would be a second execution surface"
            ));
        }
        let rows = object["edits"]
            .as_array()
            .filter(|rows| !rows.is_empty() && rows.len() <= MAX_EDITS)
            .ok_or_else(|| {
                format!("case `{case}`: variant.edits must be 1..{MAX_EDITS} entries")
            })?;
        let mut edits = Vec::new();
        for row in rows {
            let path = row["path"]
                .as_str()
                .filter(|path| {
                    !path.is_empty()
                        && path.len() <= 512
                        && !path.starts_with('/')
                        && !Path::new(path)
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir))
                })
                .ok_or_else(|| {
                    format!("case `{case}`: variant edit `path` must be a package-relative path without `..`")
                })?;
            let content = match row.get("content") {
                Some(Value::String(content)) if content.len() <= MAX_EDIT_BYTES => {
                    Some(content.clone())
                }
                Some(Value::String(_)) => {
                    return Err(format!(
                        "case `{case}`: variant edit `{path}` exceeds {MAX_EDIT_BYTES} bytes"
                    ));
                }
                Some(Value::Null) | None => None,
                Some(_) => {
                    return Err(format!(
                        "case `{case}`: variant edit `{path}` needs a string `content` or null to remove it"
                    ));
                }
            };
            if let Some(test) = test
                && let Ok(existing) = std::fs::read_to_string(root.join(path))
                && existing.contains(test)
            {
                return Err(format!(
                    "case `{case}`: the variant would edit {path}, which declares the test `{test}` this counterproof must be caught by; a variant may not modify its own protection test"
                ));
            }
            edits.push(Edit {
                path: path.to_owned(),
                content,
            });
        }
        let kind = match object.get("kind").and_then(Value::as_str) {
            None | Some("fault") => Kind::Fault,
            Some("replacement") => Kind::Replacement,
            Some(other) => {
                return Err(format!(
                    "case `{case}`: variant kind is `fault` or `replacement`, not `{other}`"
                ));
            }
        };
        // The declared expectation and the declared purpose are one decision, not two: a counterproof
        // whose expectation is `passed` is not a counterproof, and a replacement that is supposed to
        // fail is a fault wearing the wrong label. Refusing the contradiction here keeps the reply from
        // ever having to reconcile them.
        // 声明的预期与声明的用途是**一个**决定而不是两个：预期为 `passed` 的反证不是反证，而被期待失败的
        // 替换件是一个贴错标签的故障。在这里拒绝这对矛盾，回复就永远不必去调和它们。
        match (kind, expected) {
            (Kind::Fault, "failed") | (Kind::Replacement, "passed") => {}
            (Kind::Fault, _) => {
                return Err(format!(
                    "case `{case}`: a fault variant is a counterproof, so its expected outcome is `failed`, not `{expected}`"
                ));
            }
            (Kind::Replacement, _) => {
                return Err(format!(
                    "case `{case}`: a replacement variant must still pass the consumers it replaces a provider for, so its expected outcome is `passed`, not `{expected}`"
                ));
            }
        }
        let route = match object.get("route") {
            None => None,
            Some(value) => {
                let route = value
                    .as_object()
                    .ok_or_else(|| format!("case `{case}`: variant route must be an object"))?;
                if route
                    .keys()
                    .any(|key| !["caller", "callee"].contains(&key.as_str()))
                {
                    return Err(format!(
                        "case `{case}`: variant route accepts only `caller` and `callee`"
                    ));
                }
                let caller = route["caller"]
                    .as_str()
                    .filter(|path| {
                        !path.is_empty()
                            && path.len() <= 512
                            && !path.starts_with('/')
                            && !Path::new(path)
                                .components()
                                .any(|part| matches!(part, std::path::Component::ParentDir))
                    })
                    .ok_or_else(|| {
                        format!("case `{case}`: variant route `caller` must be a package-relative path without `..`")
                    })?;
                let callee = route["callee"]
                    .as_str()
                    .filter(|name| {
                        !name.is_empty()
                            && name.len() <= 200
                            && name
                                .chars()
                                .all(|character| character.is_alphanumeric() || character == '_')
                    })
                    .ok_or_else(|| {
                        format!(
                            "case `{case}`: variant route `callee` must be a bare function name"
                        )
                    })?;
                Some(Route {
                    caller: caller.to_owned(),
                    callee: callee.to_owned(),
                })
            }
        };
        Ok(Self { edits, kind, route })
    }

    /// The route this variant must keep, when it declared one.
    /// 这个变体必须保持的路线（声明了的话）。
    pub(super) fn route(&self) -> Option<&Route> {
        self.route.as_ref()
    }

    /// The declared purpose, for the ledger and the reply.
    /// 声明的用途，供台账与回复使用。
    pub(super) fn kind_label(&self) -> &'static str {
        match self.kind {
            Kind::Fault => "fault",
            Kind::Replacement => "replacement",
        }
    }

    /// The identity of this declared variant: a digest over its edits, so the copy's directory and the
    /// run's `candidate` dimension change whenever the declared fault changes — mtimes cannot make two
    /// different variants look like one.
    /// 这个声明变体的身份：对它改动的摘要，因此声明的故障一变，副本目录与运行的 `candidate` 维度就跟着变
    /// ——mtime 无法让两个不同的变体看起来像一个。
    pub(super) fn digest(&self) -> String {
        // `None` and the empty string are different edits, so the removal case is spelled out rather
        // than collapsed into an empty body.
        // `None` 与空字符串是不同的改动，因此"删除"这一情形被写明，而不是塌缩成空正文。
        let rendered = self
            .edits
            .iter()
            .map(|edit| {
                let body = match &edit.content {
                    Some(content) => format!("write:{content}"),
                    None => "remove".to_owned(),
                };
                format!("{}\u{1}{}", edit.path, body)
            })
            .collect::<Vec<_>>()
            .join("\u{2}");
        xirang_kernel::sha256_hex(rendered.as_bytes())[..16].to_owned()
    }

    /// Build the copy under `directory`, apply the edits, and describe what changed.
    /// 在 `directory` 之下建出副本、施加改动，并描述变了什么。
    ///
    /// The copy is immutable once materialised: its directory is named by [`Variant::digest`], so a
    /// second call with the same declared fault finds the copy already there and does **not** rewrite
    /// it (a half-applied variant would be a different tree than the one the digest names).
    /// 副本一经物化就不可变：它的目录由 [`Variant::digest`] 命名，因此带着同一份声明故障的第二次调用会发现
    /// 副本已经在那里，**不会**重写它（施加到一半的变体将是另一棵树，而不是摘要点名的那一棵）。
    pub(super) fn materialize(
        &self,
        base: &Path,
        excluded: Option<&Path>,
        directory: &Path,
    ) -> Result<Materialized, String> {
        // The tree is `<variants>/<digest>/tree` and the build directory is its **sibling**, never a
        // child: a `target/` inside the copy would be part of the copy's own input closure, so every
        // `cargo test` would rewrite the bytes the run says it is bound to.
        // 树是 `<variants>/<digest>/tree`，构建目录是它的**兄弟**而不是子目录：副本里的 `target/` 会成为
        // 副本自身输入闭包的一部分，于是每次 `cargo test` 都会改写这次运行声称所绑定的字节。
        let slot = directory.join(self.digest());
        let root = slot.join("tree");
        if !root.exists() {
            let staging = slot.join(".staging");
            let _ = std::fs::remove_dir_all(&staging);
            copy_tree(base, &staging, excluded)?;
            for edit in &self.edits {
                let target = staging.join(&edit.path);
                match &edit.content {
                    Some(content) => {
                        std::fs::create_dir_all(target.parent().unwrap_or(&staging))
                            .map_err(|error| format!("variant {}: {error}", edit.path))?;
                        std::fs::write(&target, content)
                            .map_err(|error| format!("variant {}: {error}", edit.path))?;
                    }
                    None => {
                        if target.exists() {
                            std::fs::remove_file(&target)
                                .map_err(|error| format!("variant {}: {error}", edit.path))?;
                        }
                    }
                }
            }
            // The rename **is** the publish step: a copy that failed half way through keeps the
            // staging name, so nothing ever reads a tree that only some of the edits reached.
            // 这次 rename **就是**发布步骤：施加到一半就失败的副本保留 staging 名，因此没有任何东西会读到
            // 只被部分改动到达的树。
            std::fs::create_dir_all(&slot).map_err(|error| error.to_string())?;
            if let Err(error) = std::fs::rename(&staging, &root) {
                // Another writer published the same variant first; it is the same content by
                // construction, so the staging copy is simply dropped.
                // 另一个写者先发布了同一个变体；按构造它是同一份内容，因此直接丢掉 staging 副本。
                let _ = std::fs::remove_dir_all(&staging);
                if !root.exists() {
                    return Err(format!("variant publish: {error}"));
                }
            }
        }
        // The route is judged on the copy's **own** bytes, after the edits: a variant that moved the
        // call out of the caller is a variant whose behaviour may still be green, and this boolean is
        // the only thing in the run that can tell the difference.
        // 路线判定落在副本**自己**的字节上、在改动之后：把调用从调用者里搬走的变体，其行为可能仍然是绿的，
        // 而这次运行里唯一能分辨这件事的就是这个布尔值。
        let route_holds = self.route.as_ref().map(|route| {
            std::fs::read_to_string(root.join(&route.caller))
                .map(|source| {
                    super::source_index::parse_functions(&source)
                        .iter()
                        .any(|function| {
                            function
                                .calls
                                .iter()
                                .any(|call| super::callgraph::is_call_to(call, &route.callee))
                        })
                })
                .unwrap_or(false)
        });
        let inputs = super::check_task::Inputs::capture(&root, None)?;
        let report = self
            .edits
            .iter()
            .map(|edit| {
                let after = std::fs::read(root.join(&edit.path)).unwrap_or_default();
                format!(
                    "{} {} ({} byte(s) now)",
                    if edit.content.is_some() {
                        "wrote"
                    } else {
                        "removed"
                    },
                    edit.path,
                    after.len()
                )
            })
            .collect();
        Ok(Materialized {
            root,
            identity: inputs.digest(),
            report,
            route_holds,
        })
    }
}

/// A materialised variant: where it is, what it is, and what was done to make it.
/// 一个已物化的变体：它在哪、它是什么、做了什么使它如此。
pub(super) struct Materialized {
    /// The copy's root, which every obligation of this variant runs in.
    /// 副本的根，这个变体的每条义务都在其中运行。
    pub(super) root: PathBuf,
    /// The copy's own input digest, used as that obligation's `candidate` dimension.
    /// 副本自己的输入摘要，用作该义务的 `candidate` 维度。
    pub(super) identity: String,
    /// One line per applied edit, for the ledger and the reply.
    /// 每条已施加改动一行，供台账与回复使用。
    pub(super) report: Vec<String>,
    /// Whether the declared route still holds in the copy; `None` when no route was declared.
    /// 声明的路线在副本里是否仍然成立；没有声明路线时为 `None`。
    pub(super) route_holds: Option<bool>,
}

/// Copy a source tree without its build products, so a variant starts from the same bytes the base did.
/// 复制一棵源码树但不带构建产物，使变体从基树所依据的同一批字节开始。
pub(super) fn copy_tree(base: &Path, target: &Path, excluded: Option<&Path>) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| format!("variant copy: {error}"))?;
    for entry in std::fs::read_dir(base).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        if excluded.is_some_and(|excluded| path.starts_with(excluded))
            || ["target", ".git", ".codegraph", ".direnv"]
                .iter()
                .any(|skip| name == *skip)
        {
            continue;
        }
        let destination = target.join(&name);
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            copy_tree(&path, &destination, excluded)?;
        } else if file_type.is_symlink() {
            // A variant keeps the link, not the bytes it currently points at: retargeting the base
            // between two runs must not silently change what the variant's own sources say.
            // 变体保留链接本身，而不是它当前指向的字节：两次运行之间改动基树的链接，不得静默改变变体自己源码的
            // 内容。
            let pointed = std::fs::read_link(&path).map_err(|error| error.to_string())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&pointed, &destination)
                .map_err(|error| format!("variant symlink {}: {error}", path.display()))?;
            #[cfg(not(unix))]
            {
                let _ = pointed;
                return Err("variant copies on this platform keep no symlinks".to_owned());
            }
        } else {
            std::fs::copy(&path, &destination)
                .map_err(|error| format!("variant copy {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "check_variant_tests.rs"]
mod check_variant_tests;
