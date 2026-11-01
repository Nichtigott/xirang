//! The filesystem facts the kernel's source walk asks a surface for.
//! 内核源码遍历向执行面索取的文件系统事实。

use std::fs;
use std::path::{Path, PathBuf};

/// The plain filesystem view of a source tree.
/// 源码树的朴素文件系统视图。
///
/// Two surfaces walk a tree the same way — the build's discovery walk and the authoring executor's
/// placement read — and each wrote this `SourceTree` out by hand. The bridge's index keeps its own
/// because it confines every answer to a canonical root; the plain one has no such rule to add, so
/// the answer to "is this a directory / what is in it / what does it say" has one home.
/// 两个执行面以同样的方式遍历一棵树——构建的发现遍历与创作执行面的落位读取——而各自手写过这个
/// `SourceTree`。桥的索引保留自己那份，因为它把每个答案约束在规范化根之内；朴素的那份没有这类规则要加，
/// 于是"这是不是目录 / 里面有什么 / 它说了什么"只有一个家。
pub(crate) struct StdSourceTree;

impl xirang_kernel::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("cannot scan {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("cannot scan {}: {error}", path.display()))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}
