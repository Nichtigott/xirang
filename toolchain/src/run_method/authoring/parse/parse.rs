//! Historical-path shim: `crate::run_method::authoring::parse`.
//! 历史路径 shim：`crate::run_method::authoring::parse`。
//!
//! Parsing and rendering helpers for authored registration faces. The pure
//! source-text transformations live in the kernel `authoring` module; this shim
//! keeps the historical path and re-implements the two filesystem-bound entry
//! points as thin wrappers. The word `shim` used to appear only on the third line,
//! so a reader who grepped this file's name against
//! `authoring/manifest/parse.rs` had to read the body to learn which one is
//! the implementation (audit `NAM-10`).
//! 注册面创作文件的解析与渲染辅助函数。纯源码文本变换位于 kernel 的 `authoring` 模块；
//! 本 shim 保留历史路径，并把两个绑定文件系统的入口重新实现为薄包装。`shim` 这个词过去只出现在
//! 第三行，因此把本文件名与 `authoring/manifest/parse.rs` 对照着 grep 的读者必须先读正文
//! 才知道哪一个是实现（审计 `NAM-10`）。

use std::fs;
use std::path::{Path, PathBuf};

pub use xirang_kernel::authoring::parse::*;

use super::context::{identity_base, normalized_path, package_root};

/// One discovered face's parent facts: the record spelling and the declared kind.
/// 一个已发现面的父级事实：记录拼写与声明的 kind。
///
/// These are what a child's declaration `.under(crate::<module>::NODE_ID)` resolves to. They are
/// **facts of the package's discovered tree**, so a reader hands them in; reconstructing them from the
/// module spelling is correct only for identity-relative faces and is how an island parent became
/// "invalid parent path".
/// 这些就是子面声明 `.under(crate::<模块>::NODE_ID)` 所解析成的东西。它们是**包内已发现树的事实**，因此由
/// 读取方交进来；从模块拼写重新构造只对身份相对的面成立，而那正是孤岛父级变成"invalid parent path"的来路。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParentFacts {
    /// The record spelling this face's declaration carries.
    /// 这个面的声明所携带的记录拼写。
    pub(crate) source: String,
    /// The `kind` it declares.
    /// 它声明的 `kind`。
    pub(crate) kind: String,
}

/// Every discovered face's parent facts, by **module chain**.
/// 每个已发现面的父级事实，按**模块链**索引。
pub(crate) type ParentMap = std::collections::BTreeMap<String, ParentFacts>;

/// Read the parent's declared `kind` from its attached source file.
/// 从父注册面的附属源文件读取它声明的 `kind`。
pub(super) fn kind_from_source_path(source: &str) -> Option<String> {
    // Classification only: an unresolvable layout makes the kind unknown, and an
    // unknown kind never authorizes a write.
    // 仅用于分类：布局解析不出时种类未知，而未知的种类从不授权写入。
    //
    // The parent source is a **record spelling**, not a path relative to the identity base: a face
    // an explicit placement put outside the walk root carries its package-relative spelling, and
    // joining that to `src/` looks for a file that was never there. The locator folds each candidate
    // back, so the same rule answers both layouts.
    // 父级源码是**记录拼写**，不是相对身份基准的路径：显式落点放在遍历根之外的面携带的是包内相对拼写，
    // 把它拼到 `src/` 上会去找一个从来不存在的文件。定位器会把每个候选折叠回去，因此同一条规则回答两种
    // 布局。
    let (package_root, identity_base) = (package_root(), identity_base().ok()?);
    let path =
        xirang_multilang::source_ref::locate_record_spelling(&package_root, &identity_base, source)
            .ok()?;
    let text = fs::read_to_string(path).ok()?;
    xirang_kernel::authoring::parse::kind_from_source_text(&text)
}

/// The `source` a face file records: the identity spelling, derived by the build's own fold.
/// 注册面文件记录的 `source`：身份拼写，由构建所用的同一次折叠推导。
///
/// A file inside the layout's identity base is named relative to it; a file inside the package but
/// outside that base — an explicit placement such as `components/reading/reading.rs` in a `src/`
/// package — is named relative to the package root. Deriving it from the identity base alone left
/// such a file with no honest relative form and fell through to its **absolute path**, which became
/// the identity input: the discovery walk and this reader then disagreed about one file's identity,
/// and an `edit` on the face resolved to a node the loaded registry did not hold.
/// 身份基准之内的文件相对它命名；包内但在该基准之外的文件——例如 `src/` 布局下显式落点
/// `components/reading/reading.rs`——相对包根命名。只从身份基准推导会让这种文件没有诚实的相对形式，于是
/// 落到它的**绝对路径**上，而那个值成了身份输入：发现遍历与本读取方随后对同一个文件的身份各说各话，对那个
/// 面的一次 `edit` 解析到已加载注册机并不持有的节点。
pub(super) fn source_path_from_file(path: &Path) -> String {
    if let Ok(identity_base) = identity_base()
        && path.starts_with(package_root())
    {
        let package_root = package_root();
        return xirang_multilang::source_ref::record_spelling(&package_root, &identity_base, path);
    }
    after_last_src(path).unwrap_or_else(|| normalized_path(path))
}

/// Everything after the last `src/` component, when the path has one.
/// 路径中存在 `src/` 段时，最后一个 `src/` 之后的一切。
fn after_last_src(path: &Path) -> Option<String> {
    let components: Vec<_> = path.components().collect();
    let at = components
        .iter()
        .rposition(|component| component.as_os_str() == "src")?;
    let tail: PathBuf = components[at + 1..].iter().collect();
    if tail.as_os_str().is_empty() {
        return None;
    }
    Some(normalized_path(&tail))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;

/// The rule a face's own registry-rule source declares, read strictly.
/// 某个注册面自己的注册规则源所声明的规则，按严格方式读取。
///
/// A face with no rule file has no structural requirement, so a missing file is still
/// `Ok("ANY")` — absence is not a malformed rule. A file that exists is read through
/// [`xirang_kernel::authoring::parse::try_rule_syntax_from_text`], which locates the
/// `REGISTRATION_RULE` const's initializer through the AST: a clause inside a comment is
/// not part of the rule, and a rule the reader cannot recognize is refused with the face
/// file, the rule file, and the reason, instead of silently degrading to `ANY` or to a
/// rule nobody declared (audit `KRN-K-10`; the tolerant/`String` entry is kept only as
/// published API and this path no longer uses it).
/// 没有规则文件的注册面没有结构要求，因此文件缺失仍是 `Ok("ANY")`——缺席不是畸形规则。存在的文件经
/// [`xirang_kernel::authoring::parse::try_rule_syntax_from_text`] 读取，它经 AST 定位
/// `REGISTRATION_RULE` 常量的初始器：注释里的子句不属于规则，而读取器认不出的规则会被拒绝，并带上
/// 注册面文件、规则文件与理由，而不是静默降级成 `ANY` 或某条没人声明的规则（审计 `KRN-K-10`；
/// 宽容的 `String` 入口只作为已发布 API 保留，本路径不再使用）。
pub(super) fn rule_syntax_for_source(path: &Path) -> Result<String, String> {
    let Some(parent) = path.parent() else {
        return Ok("ANY".to_owned());
    };
    let canonical = parent.join("registry_rule/registry_rule.rs");
    let legacy = parent.join("registry/rules/rules.rs");
    for candidate in [&canonical, &legacy] {
        let Ok(text) = fs::read_to_string(candidate) else {
            continue;
        };
        return xirang_kernel::authoring::parse::try_rule_syntax_from_text(&text).map_err(
            |error| {
                format!(
                    "face `{}`: registration rule source `{}` was refused: {error}",
                    normalized_path(path),
                    normalized_path(candidate)
                )
            },
        );
    }
    Ok("ANY".to_owned())
}
