//! Shared, language-neutral pieces every language implementation reuses.
//! 每个语言实现都复用的、语言中立的共享件。
//!
//! A language implementation owns only its spelling: how a face is declared, what its AST nodes are
//! called, which command compiles it. Everything else — the record format a native declaration
//! compiles into, the naming rules, the native build recipe, the parse/render grammar and the
//! adapter skeleton — lives here, so adding a language does not reimplement them.
//! 一个语言实现只拥有它的拼法：面怎么声明、AST 节点叫什么、用哪条命令编译。其余一切——原生命令
//! 编译成的记录格式、命名规则、native 构建配方、parse/render 语法与适配器骨架——住在这里，
//! 因此新增一门语言不会把它们再实现一遍。

#[cfg(feature = "native-record")]
#[path = "kit/record.rs"]
pub mod record;

#[path = "kit/adapter.rs"]
pub mod adapter;

#[cfg(feature = "semantics")]
#[path = "kit/ast.rs"]
pub mod ast;

#[path = "kit/declarations.rs"]
pub mod declarations;

#[path = "kit/grammar.rs"]
pub mod grammar;

#[path = "kit/native.rs"]
pub mod native;

#[path = "kit/names.rs"]
pub mod names;
