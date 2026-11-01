//! The C++ language implementation, encapsulated under its language module.
//! C++ 语言实现，封装在它自己的语言模块下。
//!
//! The adapter and the lexical/record/header pieces live together here. Only the header and the
//! naming helpers are unconditional: the adapter and the authoring/record machinery are gated on
//! `lang-cpp`. Historical root paths (`cpp_header`, `cpp_names`, …) are thin re-exports in the
//! crate root, not a second implementation.
//! 适配器与词法/记录/头部实现一起住在这里。只有头部与命名助手是无条件的：适配器与创作/记录机制由
//! `lang-cpp` 门控。历史根部路径（`cpp_header`、`cpp_names` 等）是 crate 根的薄重导出，不是第二份实现。
//!
//! Uses the language kit: the record decoder, the native-driver skeleton, the name rules, the field
//! table, the adapter orchestration and the AST walker all live in `crate::languages::kit`; this
//! module keeps C++'s own lexer, clang facts and header text.
//! 使用语言 kit：记录解码器、native 驱动骨架、命名规则、字段表、适配器编排与 AST 遍历器都在
//! `crate::languages::kit`；本模块只保留 C++ 自己的词法器、clang 事实与头部文本。

#[path = "header.rs"]
pub mod header;
#[path = "names.rs"]
pub mod names;

#[cfg(feature = "lang-cpp")]
pub mod semantics;

#[cfg(feature = "lang-cpp")]
pub mod declarations {
    //! The declared facts a C++ source carries before it compiles; the lexer lives in `syntax`.
    //! C++ 源码在编译之前携带的声明事实；词法器住在 `syntax`。
    pub use super::syntax::{Declared, faces};
}
#[cfg(feature = "lang-cpp")]
#[path = "native.rs"]
pub mod native;
#[cfg(feature = "lang-cpp")]
pub mod record {
    //! The historical path of the `xirang.faces` reader, now a language-neutral kit component.
    //! `xirang.faces` 读取器的历史路径，现在是语言中立的 kit 组件。
    pub use crate::languages::kit::record::*;
}
#[cfg(feature = "lang-cpp")]
#[path = "syntax.rs"]
pub mod syntax;

#[cfg(feature = "lang-cpp")]
#[path = "adapter.rs"]
mod adapter;

#[cfg(feature = "lang-cpp")]
pub use adapter::*;
