//! Which paths one operation may write, decided once for the whole write set.
//! 一次操作可以写哪些路径——对整个写入集合只判定一次。
//!
//! The rule is the same one every reader in this repository follows, turned around: **the real
//! location decides, never the spelling.** A package may read through a declared source (a
//! legitimate materialization can be a link), but a declaration is a read permission, not a write
//! permission — writing through the link would put bytes outside the package.
//! 规则与本仓每个读取方遵守的是同一条，只是反过来：**由真实位置决定，绝不由拼写决定。** 包可以经已声明
//! 来源读取（合法物化可以是链接），但声明是读权限、不是写权限——穿过链接写会把字节放到包外。
//!
//! Why it is a set and not a per-file test: a write set is authorized **before** the first byte is
//! written, and every member of it is judged by the same rule. A per-file check that runs inside the
//! write loop turns the second file's refusal into a rollback of the first — and the two lists drift
//! the moment somebody adds a writer in the middle (measured: `move`'s child-face loop had no check
//! at all while its consumer loop did).
//! 为什么它是"集合"而不是逐文件判断：写入集合在写出第一个字节**之前**被授权，且其中每个成员由同一条规则
//! 判定。跑在写入循环里的逐文件检查会把第二个文件的拒绝变成对第一个文件的回滚——而只要有人在中间加一个写入方，
//! 两张清单就会漂移（实测：`move` 的子面循环一个检查都没有，而它的消费者循环有）。

use std::path::{Path, PathBuf};

/// What a path in the set is going to happen to.
/// 集合里的一个路径将要遭遇什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChangeKind {
    /// Its bytes are created or replaced.
    /// 创建或替换它的字节。
    Write,
    /// It is deleted.
    /// 删除它。
    Remove,
}

/// One member of a complete write set.
/// 完整写入集合里的一个成员。
#[derive(Clone, Debug)]
pub(crate) struct Change {
    /// The path as the operation spells it.
    /// 操作书写的路径。
    pub(crate) path: PathBuf,
    /// What happens to it.
    /// 它遭遇什么。
    pub(crate) kind: ChangeKind,
    /// What the refusal should call this member: a file, an object, a record.
    /// 拒绝时该怎么称呼这个成员：一个文件、一个对象、一条记录。
    pub(crate) subject: String,
}

/// One member whose **real** location is outside the package.
/// 一个真实位置在包外的成员。
#[derive(Clone, Debug)]
pub(crate) struct Outside {
    /// What the refusal should call it.
    /// 拒绝时该怎么称呼它。
    pub(crate) subject: String,
    /// What would have happened to it had it been inside.
    /// 如果它在包内，本来会发生什么。
    pub(crate) kind: ChangeKind,
    /// The path as spelled.
    /// 书写的路径。
    pub(crate) spelled: PathBuf,
    /// Where it really is: the resolved path, or the nearest existing ancestor of one that does not
    /// exist yet.
    /// 它真正在哪里：已解析的路径，或者"尚不存在"时最近的已存在祖先。
    pub(crate) real: PathBuf,
}

/// The package root every write must resolve inside.
/// 每一次写入都必须解析到其内部的包根。
#[derive(Clone, Debug)]
pub(crate) struct WriteScope {
    real_root: PathBuf,
}

impl WriteScope {
    /// Bind the scope to one package root, resolved once.
    /// 把作用域绑定到一个包根，只解析一次。
    pub(crate) fn new(root: &Path) -> Result<Self, String> {
        let real_root = std::fs::canonicalize(root)
            .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
        Ok(Self { real_root })
    }

    /// Whether one path resolves inside the package.
    /// 一个路径是否解析到包内。
    ///
    /// The nearest **existing** ancestor decides for a path that is not there yet (a destination, a
    /// record directory), because that is the part a link can already redirect. A path that exists is
    /// resolved whole, so a file symlink pointing outside is refused rather than written through.
    /// 对尚不存在的路径（目的地、记录目录）由最近的**已存在**祖先决定，因为那正是链接此刻已经能改道的一段。
    /// 已存在的路径整个解析，因此指向包外的文件链接会被拒绝，而不是被穿过写入。
    pub(crate) fn real_location(&self, path: &Path) -> Result<PathBuf, String> {
        let mut probe = path.to_path_buf();
        while !probe.exists() {
            probe = probe
                .parent()
                .ok_or_else(|| format!("{} has no existing ancestor", path.display()))?
                .to_path_buf();
        }
        std::fs::canonicalize(&probe)
            .map_err(|error| format!("cannot resolve {}: {error}", probe.display()))
    }

    /// Whether one path is inside, as a plain question.
    /// 一个路径是否在包内——作为一个单纯的问题。
    pub(crate) fn inside(&self, path: &Path) -> Result<bool, String> {
        Ok(self.real_location(path)?.starts_with(&self.real_root))
    }

    /// Refuse the whole set, naming every member that would land outside.
    /// 拒绝整个集合，并点名每一个会落到包外的成员。
    ///
    /// All of them are reported, not the first: a caller that fixes one path and retries should not
    /// discover the next one only after that.
    /// 报出全部而不是第一个：修好一个路径再重试的调用方，不该在那里之后才发现下一个。
    pub(crate) fn authorize(&self, changes: &[Change]) -> Result<(), Vec<Outside>> {
        let mut outside = Vec::new();
        for change in changes {
            match self.real_location(&change.path) {
                Ok(real) if real.starts_with(&self.real_root) => {}
                Ok(real) => outside.push(Outside {
                    subject: change.subject.clone(),
                    kind: change.kind,
                    spelled: change.path.clone(),
                    real,
                }),
                Err(reason) => outside.push(Outside {
                    subject: change.subject.clone(),
                    kind: change.kind,
                    spelled: change.path.clone(),
                    real: PathBuf::from(reason),
                }),
            }
        }
        if outside.is_empty() {
            Ok(())
        } else {
            Err(outside)
        }
    }
}

/// A write set's refusal as one paragraph: every offending member, in the caller's own vocabulary.
/// 写入集合的拒绝写成一段话：每个越界成员，用调用方自己的词汇点名。
pub(crate) fn describe_outside(outside: &[Outside], root: &Path) -> String {
    let mut message = format!(
        "{} path(s) in this write set resolve outside the package {} — a declared source is a place \
         a read may enter, not a place a write may land:",
        outside.len(),
        root.display()
    );
    for member in outside {
        message.push_str(&format!(
            "\n {} ({} would be {}) — `{}` resolves to `{}`",
            member.subject,
            change_noun(member.kind),
            verb(member.kind),
            member.spelled.display(),
            member.real.display()
        ));
    }
    message
}

#[cfg(test)]
#[path = "write_scope_tests.rs"]
mod write_scope_tests;
/// The noun a refusal uses for one change.
/// 拒绝里对一次变更使用的名词。
fn change_noun(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Write => "this path",
        ChangeKind::Remove => "this path",
    }
}

/// The verb a refusal uses for one change.
/// 拒绝里对一次变更使用的动词。
fn verb(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Write => "written",
        ChangeKind::Remove => "deleted",
    }
}
