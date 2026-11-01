//! Static extraction of `soil.face({…})` declarations from a package's TypeScript sources.
//! 从包的 TypeScript 源码里静态提取 `soil.face({…})` 声明。
//!
//! The parser is the JavaScript one: the declaration is the same call, and TypeScript's extra
//! spelling lives in the type annotation around it, not in the field object. This module owns only
//! the file suffix the walk accepts.
//! 解析器就是 JavaScript 那一个：声明是同一次调用，TypeScript 多出来的拼法在它周围的类型标注里，而不在
//! 字段对象里。本模块只拥有遍历接受的文件后缀。

use std::path::Path;

use crate::languages::js::faces::{self as js_faces, JsFace};
use crate::languages::kit::declarations::{DeclarationSpec, Unreadable};
use crate::source_scope::SourceScope;

/// One `soil.face({…})` declaration found in a TypeScript source.
/// 在 TypeScript 源码里找到的一条 `soil.face({…})` 声明。
pub type TsFace = JsFace;

/// What the shared walk needs to know about TypeScript.
/// 共享遍历需要知道的 TypeScript 那部分。
const DECLARATIONS: DeclarationSpec = DeclarationSpec {
    // `.mts` and `.cts` do not end in `.ts` (the suffix check is byte-exact), so the two spellings
    // this slice accepts are `ts` and `tsx`; a file with neither is not a TypeScript declaration.
    // `.mts` 与 `.cts` 并不以 `.ts` 结尾（后缀检查是逐字节的），因此本切片接受两种拼法：`ts` 与
    // `tsx`；两者都不是的文件不是 TypeScript 声明。
    extensions: &[".ts", ".tsx"],
    skip_directories: js_faces::SKIPPED,
    marker: js_faces::mentions,
    // A fact reader must not report "no declaration" for a file it never opened.
    // 事实读取器绝不能把自己从没打开过的文件报成"没有声明"。
    unreadable: Unreadable::Refuse,
};

/// Every TypeScript declaration in the package, sorted by source.
/// 包里的每条 TypeScript 声明，按源码排序。
pub fn read_declarations(package_root: &Path) -> Result<Vec<TsFace>, String> {
    read_declarations_in(&SourceScope::with_sources(package_root)?)
}

/// Every TypeScript declaration under one already-decided source scope.
/// 在一个已经决定好的来源范围下的每条 TypeScript 声明。
pub fn read_declarations_in(scope: &SourceScope) -> Result<Vec<TsFace>, String> {
    js_faces::read_declarations_for(scope, &DECLARATIONS)
}
