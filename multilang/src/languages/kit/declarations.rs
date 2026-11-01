//! The shared declaration walk: which source files a language reads, and how one bad file is named.
//! 共享的声明遍历：一门语言读哪些源码文件，以及坏文件怎么被点名。
//!
//! The walk is one rule (`source_scope`), the file choice is the language's extensions, and the
//! declaration marker is the language's own predicate. What is shared is the loop: enter the files
//! the scope allows, skip the ones without a marker, read each text once, and let the language turn
//! one text into its facts. A second copy of this loop is how a language came to follow an
//! undeclared link or to forget the source spelling in an error.
//! 遍历是一条规则（`source_scope`），选哪些文件是语言的扩展名，声明标记是语言自己的谓词。共享的是那个
//! 循环：进入范围允许的文件、跳过没有标记的、每个文本只读一次，再让语言把一份文本变成它的事实。这个
//! 循环的第二份拷贝，正是某种语言开始跟随未声明链接、或在错误里忘记源码拼写的原因。

use std::path::Path;

use crate::source_scope::SourceScope;

/// What one language tells the shared declaration walk about itself.
/// 一门语言告诉共享声明遍历的东西。
pub struct DeclarationSpec {
    /// The file-name suffixes this language reads.
    /// 这门语言读取的文件名后缀。
    pub extensions: &'static [&'static str],
    /// Directories the walk never enters.
    /// 遍历从不进入的目录。
    pub skip_directories: &'static [&'static str],
    /// Whether a text carries this language's declaration marker.
    /// 一份文本是否携带这门语言的声明标记。
    pub marker: fn(&str) -> bool,
    /// What a file this process cannot read means: some readers skip it (a build that only compiles
    /// what it can read), others refuse (a fact reader must not report "no declaration" for a file
    /// it never opened).
    /// 一个本进程读不出的文件意味着什么：有的读取器跳过它（只编译读得到的源码的构建），有的拒绝
    /// （事实读取器绝不能把自己从没打开过的文件报成"没有声明"）。
    pub unreadable: Unreadable,
}

/// What the walk does with a file it cannot read.
/// 遍历对读不出的文件做什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// Skip the file.
    /// 跳过。
    Skip,
    /// Refuse the walk, naming the file.
    /// 拒绝整次遍历，并点名文件。
    Refuse,
}

/// Every declaration under one already-decided source scope.
/// 在一个已经决定好的来源范围下的每条声明。
///
/// `parse` receives the source's package-relative spelling and its text, and returns the
/// declarations the language found in it; the returned facts are in walk order (the caller sorts
/// when its consumers need a stable spelling order). An error is prefixed with the source spelling,
/// so a reader never has to guess which file it came from.
/// `parse` 收到源码的包内相对拼写与文本，返回该语言在其中找到的声明；返回的事实按遍历顺序（调用方在消费者
/// 需要稳定拼写顺序时自行排序）。错误会带上源码拼写前缀，因此读者不必猜它来自哪个文件。
pub fn read_in<T>(
    scope: &SourceScope,
    spec: &DeclarationSpec,
    parse: impl Fn(&str, &str) -> Result<Vec<T>, String>,
) -> Result<Vec<T>, String> {
    let mut found = Vec::new();
    for file in scope.files(
        |path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    spec.extensions
                        .iter()
                        .any(|extension| name.ends_with(extension))
                })
        },
        spec.skip_directories,
    )? {
        let text = match std::fs::read_to_string(&file.path) {
            Ok(text) => text,
            Err(_) if spec.unreadable == Unreadable::Skip => continue,
            Err(error) => {
                return Err(format!("{} is not readable: {error}", file.path.display()));
            }
        };
        if !(spec.marker)(&text) {
            continue;
        }
        let mut facts =
            parse(&file.source, &text).map_err(|error| format!("{}: {error}", file.source))?;
        found.append(&mut facts);
    }
    Ok(found)
}

/// Every declaration in one package, under a fresh source scope.
/// 一个包里的每条声明（新建一个来源范围）。
///
/// `read` is a bare verb, reserved for entry positions; this is a package-level helper.
/// `read` 是裸动词、只留给入口位置；本函数是包级助手。
pub fn read_package<T>(
    package_root: &Path,
    spec: &DeclarationSpec,
    parse: impl Fn(&str, &str) -> Result<Vec<T>, String>,
) -> Result<Vec<T>, String> {
    read_in(&SourceScope::with_sources(package_root)?, spec, parse)
}

#[cfg(test)]
#[path = "declarations_tests.rs"]
mod declarations_tests;
