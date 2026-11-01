//! Which files one read of a package is allowed to look at — decided once, in one place.
//! 一次包读取允许看哪些文件——只在一个地方决定。
//!
//! Every language used to answer this for itself, and the answers had the same hole: a directory
//! entry was tested with `Path::is_dir()`, which follows symbolic links, so a link inside the package
//! could pull a file from anywhere on the machine into the package's facts. Measured on the Python
//! reader: a `linked -> …/outside` symlink plus a declaration in the link target returned
//! `Ok([PythonFace { source: "linked/outside.py", … }])`.
//! 每种语言过去各自回答这个问题，而答案有同一个洞：目录项用 `Path::is_dir()` 判断，而它会跟随符号
//! 链接，因此包内一个链接能把机器上任何位置的文件拉进包的事实里。在 Python 读取器上实测：一个
//! `linked -> …/outside` 软链加上链接目标里的一条声明，返回了
//! `Ok([PythonFace { source: "linked/outside.py", … }])`。
//!
//! The rule this module implements:
//! 本模块实现的规则：
//!
//! * the walk starts at the **package root**; a root a caller `declare`s is a **permission**, not a
//!   second starting point: it lets a link inside the package resolve to a target there, and the
//!   walk never enumerates that root on its own (a file outside the package has no package-relative
//!   spelling to record it under);
//! * every entry's real target is checked against those roots, so an **undeclared** link that leaves
//!   them is refused by name (not silently skipped: a skipped link looks like "there is nothing
//!   there", which is a different fact);
//! * a **declared** link is followed, because a legitimate materialization may be a link;
//! * a directory visited twice through different links is walked once (cycles terminate), and a file
//!   reachable twice is reported once.
//!
//! * 遍历从**包根**开始；调用方 `declare` 的根是一份**许可**，不是第二个起点：它让包内的链接可以解析到
//!   那里的目标，而遍历本身从不枚举那个根（包外的文件没有可记录它的包内相对拼写）；
//! * 每个条目的真实目标都对着那些根校验，因此**未声明的**越界链接按名拒绝（不是静默跳过：被跳过的链接
//!   看起来像"那里什么都没有"，那是另一个事实）；
//! * **已声明的**链接会被跟随，因为合法的物化完全可以是链接；
//! * 经不同链接两次走到的目录只走一次（环会终止），两次可到达的文件只报一次。

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// The roots one read may enter, and the walk that stays inside them.
/// 一次读取可以进入的根，以及留在其中的遍历。
#[derive(Clone, Debug, Default)]
pub struct SourceScope {
    /// Canonical roots, as the walk compares every real target against them.
    /// 规范化的根，遍历用它比对每个真实目标。
    roots: Vec<PathBuf>,
    /// The package root, kept separately because a relative source spelling is taken against it.
    /// 包根，单独保留：相对源码拼写是相对它取的。
    package_root: Option<PathBuf>,
}

/// One file the walk found, with the package-relative spelling it will be recorded under.
/// 遍历找到的一个文件，连同它将被记录为的包内相对拼写。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocatedFile {
    /// The path on this machine.
    /// 本机上的路径。
    pub path: PathBuf,
    /// The package-relative spelling, with forward slashes.
    /// 包内相对拼写，使用正斜杠。
    pub source: String,
}

impl SourceScope {
    /// A scope that may read the package and nothing else.
    /// 只允许读这个包的来源范围。
    pub fn package(package_root: &Path) -> Result<Self, String> {
        let canonical = std::fs::canonicalize(package_root)
            .map_err(|error| format!("cannot read {}: {error}", package_root.display()))?;
        Ok(Self {
            roots: vec![canonical],
            package_root: Some(package_root.to_path_buf()),
        })
    }

    /// A scope that may read the package **and** every source the project declares.
    /// 允许读取包**以及**项目声明的每个来源的范围。
    ///
    /// The declarations are data (`xirang.sources.tsv` plus this machine's bindings), so a link
    /// that materializes a declared source is followed while an undeclared one stays refused. A
    /// declaration with no binding is an error from the map, never a silent skip: "the source is
    /// not here on this machine" is a fact the caller has to hear.
    /// 声明是数据（`xirang.sources.tsv` 加本机绑定），因此物化已声明来源的链接会被跟随，而未声明的仍被
    /// 拒绝。没有绑定的声明由来源映射报错，绝不静默跳过："这个来源不在本机上"是调用方必须听到的事实。
    pub fn with_sources(package_root: &Path) -> Result<Self, String> {
        let mut scope = Self::package(package_root)?;
        for root in crate::source_map::SourceMap::read_from(package_root)?.roots(package_root)? {
            scope.declare(&root.location)?;
            if let Some(bound) = root.bound {
                scope.declare(&bound)?;
            }
        }
        Ok(scope)
    }

    /// Declare one more root this read may enter — a source the project named explicitly.
    /// 再声明一个这次读取可以进入的根——项目显式点名的来源。
    pub fn declare(&mut self, root: &Path) -> Result<(), String> {
        let canonical = std::fs::canonicalize(root)
            .map_err(|error| format!("cannot read {}: {error}", root.display()))?;
        if !self.roots.iter().any(|known| known == &canonical) {
            self.roots.push(canonical);
        }
        Ok(())
    }

    /// The package root this scope was built for.
    /// 本范围所针对的包根。
    pub fn package_root(&self) -> Option<&Path> {
        self.package_root.as_deref()
    }

    /// Every file under the package root whose name `wanted` accepts. A `declare`d root is entered
    /// only through a link that resolves into it, never walked on its own.
    /// 包根下每个名字被 `wanted` 接受的文件。`declare` 的根只能经解析到它的链接进入，从不单独遍历。
    ///
    /// `skipped` names directories that are not source: build output, version control, the tool's own
    /// data. They are skipped by name, which is a decision this module makes once for every caller.
    /// `skipped` 点名不是源码的目录：构建产物、版本控制、工具自己的数据。它们按名跳过，而这个决定由本
    /// 模块为所有调用方做一次。
    pub fn files(
        &self,
        wanted: impl Fn(&Path) -> bool,
        skipped: &[&str],
    ) -> Result<Vec<LocatedFile>, String> {
        let Some(package_root) = self.package_root() else {
            return Err("this source scope has no package root".to_owned());
        };
        let mut found: Vec<LocatedFile> = Vec::new();
        let mut seen_directories: BTreeSet<PathBuf> = BTreeSet::new();
        let mut seen_files: BTreeSet<PathBuf> = BTreeSet::new();
        let mut pending = vec![package_root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            let canonical = match std::fs::canonicalize(&directory) {
                Ok(canonical) => canonical,
                // Only a directory that **vanished** between the listing and the descent is not a
                // fact about the tree. Any other failure — unreadable, permission denied, a broken
                // mount — is a shape this walk must name, not skip: skipping it would report "there
                // is nothing there", which is a different answer.
                // 只有列目录与下潜之间**消失**的目录不是关于这棵树的事实。其它失败——读不了、无权限、挂载
                // 坏了——是本遍历必须点名的形状，不能跳过：跳过会报出"那里什么都没有"，那是另一个答案。
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!(
                        "cannot resolve {}: {error}; a directory this walk cannot read is reported, \
                         not skipped",
                        directory.display()
                    ));
                }
            };
            if !seen_directories.insert(canonical.clone()) {
                continue;
            }
            let entries = std::fs::read_dir(&directory)
                .map_err(|error| format!("read {}: {error}", directory.display()))?;
            for entry in entries {
                let entry =
                    entry.map_err(|error| format!("read {}: {error}", directory.display()))?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                // `DirEntry::file_type` does not follow the link, which is the distinction this
                // module needs: a link is a fact to be checked, not a directory to be entered.
                // `DirEntry::file_type` 不跟随链接，正是本模块需要的区分：链接是要校验的事实，不是可以
                // 直接进入的目录。
                let file_type = entry
                    .file_type()
                    .map_err(|error| format!("read {}: {error}", path.display()))?;
                let link = file_type.is_symlink();
                let real = if link {
                    let real = std::fs::canonicalize(&path).map_err(|error| {
                        format!("cannot resolve the link {}: {error}", path.display())
                    })?;
                    if !self.contains(&real) {
                        return Err(format!(
                            "{} links to {}, which is outside every source this read declared ({}); \
                             declare that root explicitly or remove the link — an undeclared link is \
                             not a way to widen the sources",
                            path.display(),
                            real.display(),
                            self.describe_roots()
                        ));
                    }
                    real
                } else {
                    path.clone()
                };
                let metadata = std::fs::metadata(&real)
                    .map_err(|error| format!("read {}: {error}", real.display()))?;
                if metadata.is_dir() {
                    if skipped.contains(&name.as_str()) {
                        continue;
                    }
                    pending.push(path);
                    continue;
                }
                if !wanted(&real) && !wanted(&path) {
                    continue;
                }
                if !seen_files.insert(real.clone()) {
                    continue;
                }
                let relative = path.strip_prefix(package_root).map_err(|_| {
                    format!(
                        "{} is outside {}; a source this read found is spelled relative to the \
                         package root",
                        path.display(),
                        package_root.display()
                    )
                })?;
                found.push(LocatedFile {
                    path: path.clone(),
                    source: relative.to_string_lossy().replace('\\', "/"),
                });
            }
        }
        found.sort();
        Ok(found)
    }

    /// Whether one real path is inside every declared root's subtree.
    /// 一个真实路径是否落在某个已声明根的子树里。
    fn contains(&self, real: &Path) -> bool {
        self.roots.iter().any(|root| real.starts_with(root))
    }

    /// The declared roots, as one sentence a refusal can paste.
    /// 已声明的根，写成一句拒绝可以直接引用的话。
    fn describe_roots(&self) -> String {
        self.roots
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A package-relative path, or an error naming why the spelling is not one.
/// 一个包内相对路径，或一个点名"为什么不是"的错误。
///
/// Kept here because every reader that turns a found file into a recorded source needs the same
/// answer, and a second copy would be a second rule.
/// 放在这里，是因为每个把"找到的文件"变成"记录的源码"的读取方都需要同一个答案，而第二份拷贝就是第二条
/// 规则。
pub fn relative_spelling(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        format!(
            "{} is outside {}; it cannot be spelled as a package-relative source",
            path.display(),
            root.display()
        )
    })?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{} does not reduce to a package-relative spelling",
            path.display()
        ));
    }
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
#[path = "source_scope_tests.rs"]
mod source_scope_tests;
