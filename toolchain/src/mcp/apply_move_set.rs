//! A move's **complete** write set: what it is, whose permission it needs, and what to do when one
//! member lands outside the package.
//! 一次搬动的**完整**写入集合：它是什么、需要谁的许可，以及当某个成员落在包外时怎么办。
//!
//! The defect this exists for: `move` planned the moved declaration, its children and its consumers,
//! checked the consumers for a declared (read-only) source, and never checked the **children** — the
//! same loop, one branch apart. A per-writer check cannot be trusted to stay complete, so the set is
//! assembled first and authorized once, before the first byte.
//! 本模块存在的理由：`move` 会规划被搬声明、它的子面与消费者，对消费者检查了已声明（只读）来源，
//! 却从没检查**子面**——同一个循环、隔一个分支。逐写入方的检查不能被信任为完整，因此先装配整个集合，
//! 在写出第一个字节之前一次性授权。
//!
//! Two things a refusal owes its reader, because a refusal is where this work leaves their hands:
//! **every** offending member (not the first), and at least one **executable** way forward. The
//! preferred way forward is never "edit the third party's source": the generated C++ header keeps a
//! namespace per retired module path, so a reference written against the old path keeps resolving and
//! needs no edit at all.
//! 一次拒绝欠读者的两件事，因为拒绝正是这件工作离开他们双手的地方：**每一个**越界成员（不是第一个），
//! 以及至少一条**可执行**的出路。最优先的出路绝不是"去改第三方的源码"：生成的 C++ 头会为每个退役模块
//! 路径保留一节命名空间，因此按旧路径写下的引用仍然解析得到、根本不需要改。

use std::path::{Path, PathBuf};

use crate::build_method::FaceView;
use crate::run_method::authoring::write_scope::{Change, ChangeKind, WriteScope, describe_outside};

/// One member of the move that would have to be written outside the package.
/// 这次搬动里一个会不得不写到包外的成员。
pub(super) struct Conflict {
    /// What the refusal calls it: a child face, a consumer, a record.
    /// 拒绝时怎么称呼它：一个子面、一个消费者、一条记录。
    pub(super) subject: String,
    /// The file as the tree spells it.
    /// 树拼写的文件。
    pub(super) path: PathBuf,
    /// Why this member is in the write set: the sentence the reader needs to judge the trade.
    /// 这个成员为什么在写入集合里：读者判断取舍所需的那句话。
    pub(super) because: String,
}

/// Whether the old parent reference keeps resolving without a write, because the generated header
/// carries a compatibility namespace for the **retired module path**.
/// 旧的父引用能否在不需要写入的情况下继续解析——因为生成的头部为**退役的模块路径**携带了一节兼容命名空间。
///
/// This is the "keep the connection stable instead of editing the third party" half. It is decided
/// from facts, not hope: the module path must actually change (otherwise there is nothing to rewrite),
/// no other face may now own the old path (the header refuses to alias a live module), and the file's
/// own bytes must name the old path — a file that does not mention it needs no edit either way.
/// 这是"保持连接稳定、而不是去改第三方"的那一半。它由事实而不是期望决定：模块路径必须真的改变（否则没有
/// 什么要改写）、此刻不能有别的面拥有旧路径（头部拒绝为活模块起别名）、而文件自己的字节必须点名旧路径
/// ——根本不提它的文件本来两边都不需要改。
pub(super) fn keep_through_compatibility(
    views: &[FaceView],
    face: &FaceView,
    child: &FaceView,
    old_module: &str,
    new_module: &str,
    child_text: &str,
) -> Option<String> {
    if old_module == new_module {
        // The reference is unchanged by this move; nothing to write and nothing to keep.
        // 这次搬动不改变那条引用；没有要写的，也没有要保的。
        return Some(format!(
            "kept `{}`: the module path did not change, so its parent reference is still correct",
            child.source
        ));
    }
    if views
        .iter()
        .any(|other| other.id != face.id && other.module == old_module)
    {
        // Another face owns the retired path now: a compatibility namespace there would be
        // ambiguous, so the header emits none and the reference really is broken.
        // 另一个面此刻拥有那个退役路径：在那里加兼容命名空间会有歧义，因此头部不会发射，而引用确实断了。
        return None;
    }
    // The shared catalog path is a second, language-neutral spelling a generated C++ declaration
    // can carry. A logical move retires the old path; the generated header keeps an alias from it to
    // the object's current identity, so the declared source does not have to be written.
    // 共享目录路径是生成的 C++ 声明可能携带的第二种、语言无关的拼法。逻辑搬动会退役旧路径；生成头保留
    // 旧路径到对象当前身份的别名，因此已声明来源不必被写入。
    let old_catalog = catalog_spelling(&face.path);
    if child_text.contains(&old_catalog) {
        return Some(format!(
            "kept `{}`: it names the retired catalog path, and the generated header keeps a \
             compatibility catalog alias for `{}`, so no third-party source has to change",
            child.source, face.path
        ));
    }
    if !child_text.contains(&module_spelling(old_module)) {
        // The file does not name the old path: either it already names the new one (an earlier
        // move, or an author's edit), or it names the parent some way this tool cannot verify.
        // 文件没有点名旧路径：要么它已经点名新的（早先的搬动，或作者的编辑），要么它用本工具无法核实的方式
        // 点名父级。
        return None;
    }
    Some(format!(
        "kept `{}`: it names the retired module path, and the generated header keeps a \
         compatibility namespace for `{}`, so no third-party source has to change",
        child.source, old_module
    ))
}

/// The shared catalog spelling a generated C++ declaration uses for one logical path.
/// 生成的 C++ 声明为一条逻辑路径使用的共享目录拼法。
fn catalog_spelling(path: &str) -> String {
    format!(
        "crate::catalog::{}::NODE_ID",
        path.trim_start_matches("root/").replace('/', "::")
    )
}

/// The spelling a generated header publishes for one module path.
/// 生成的头部为某个模块路径发布的拼法。
fn module_spelling(module: &str) -> String {
    format!("::xirang::faces::{module}::NODE_ID")
}

/// Authorize the whole write set, or name **every** member that would land outside.
/// 授权整个写入集合，或者点名**每一个**会落到包外的成员。
pub(super) fn authorize(
    package_root: &Path,
    changes: &[Change],
    subject: &str,
) -> Result<(), String> {
    let scope = WriteScope::new(package_root)?;
    match scope.authorize(changes) {
        Ok(()) => Ok(()),
        Err(outside) => {
            // One implementation of "which paths are outside, and what each was going to be":
            // `write_scope::describe_outside`. A second paragraph here said the same thing in another
            // order, which is how the two lists drifted the first time.
            // "哪些路径在包外、每一条本来要做什么"只有一个实现：`write_scope::describe_outside`。这里曾
            // 另写一段，用另一种顺序说同一件事——那正是两张清单第一次漂移的方式。
            let mut message = format!(
                "{subject} would have to write outside the package, so nothing was written.\n{}",
                describe_outside(&outside, package_root)
            );
            message.push_str(&ways_forward(
                &outside
                    .iter()
                    .map(|member| member.subject.clone())
                    .collect::<Vec<_>>(),
            ));
            Err(message)
        }
    }
}

/// The change kind for one member, spelled where the set is assembled.
/// 装配集合处书写的成员变更种类。
pub(super) fn write(path: &Path, subject: impl Into<String>) -> Change {
    Change {
        path: path.to_path_buf(),
        kind: ChangeKind::Write,
        subject: subject.into(),
    }
}

/// The change kind for a removal.
/// 移除成员的变更种类。
pub(super) fn remove(path: &Path, subject: impl Into<String>) -> Change {
    Change {
        path: path.to_path_buf(),
        kind: ChangeKind::Remove,
        subject: subject.into(),
    }
}

/// The refusal for a move whose write set includes members outside the package.
/// 这次搬动的写入集合包含包外成员时的拒绝文案。
///
/// `subject` is the operation as the caller spells it (`move \`root/panel/Area\``), because a
/// refusal that does not say **what** it refused leaves the reader guessing which request it answered.
/// `subject` 是调用方拼写的操作（`move \`root/panel/Area\``），因为一条不说清**拒绝了什么**的拒绝文案，
/// 会让读者去猜它回答的是哪一次请求。
pub(super) fn describe_conflicts(
    subject: &str,
    conflicts: &[Conflict],
    package_root: &Path,
) -> String {
    let mut message = format!(
        "{subject} would have to rewrite {} declaration(s) that live outside the package {}, so \
         nothing was written:",
        conflicts.len(),
        package_root.display()
    );
    for conflict in conflicts {
        message.push_str(&format!(
            "\n {} → `{}`\n because {}",
            conflict.subject,
            conflict.path.display(),
            conflict.because
        ));
    }
    let subjects = conflicts
        .iter()
        .map(|conflict| conflict.because.clone())
        .collect::<Vec<_>>();
    message.push_str(&ways_forward(&subjects));
    message
}

/// The ways forward a refusal offers, kept to the ones that apply to the references it names.
/// 拒绝给出的出路口径，只保留适用于它所点名那些引用的。
///
/// A C++ declaration can be kept stable without any edit (the header's compatibility namespace), and
/// a Rust `xirang_external` alias cannot — telling a reader about an option that does not apply is how
/// a refusal turns into a scavenger hunt.
/// C++ 声明可以不改一行就保持稳定（头部的兼容命名空间），而 Rust 的 `xirang_external` 别名做不到——告诉
/// 读者一条不适用的出路，正是让拒绝变成"到处找答案"的方式。
fn ways_forward(subjects: &[String]) -> String {
    let mut text = String::from(
        "\nA declared source is a place a read may enter, not a place a write may land. Each of these \
         is a complete way forward:",
    );
    text.push_str(
        "\n * edit that file by hand (it is a third party's source) so its reference already names the \
         new module, then re-run this move — a reference that no longer names the old spelling needs \
         no write, so the move proceeds;",
    );
    text.push_str(
        "\n * move the object from a writable working copy: copy the declared source into the package, \
         drop its `xirang.sources.tsv` line and its `.xirang/source-bindings.tsv` binding, then move;",
    );
    if subjects
        .iter()
        .any(|subject| subject.contains("::xirang::faces") || subject.contains("child face"))
    {
        text.push_str(
            "\n * a C++ declaration that names the parent through `::xirang::faces::<module>::NODE_ID` \
             needs no edit at all: the generated header keeps a compatibility namespace for the module \
             path this move retires. This move refused because the file's text does not prove that \
             shape.",
        );
    }
    text
}
