//! Language adapters for XiRang registration authoring.
//! XiRang 注册面创作的语言适配器。
//!
//! The crate owns two things: the protocol every language implementation speaks
//! ([`registration::LanguageAdapter`]) and the language implementations themselves. The toolchain
//! composes them with its execution context — package roots, the registry, transactions — so this
//! crate never depends on the toolchain.
//! 本 crate 拥有两样东西：每个语言实现都说的协议（[`registration::LanguageAdapter`]）与语言实现
//! 本身。工具链把它们与执行上下文——包根、注册树、事务——组合起来，因此本 crate 从不依赖工具链。
//!
//! A pure-Rust host compiles this crate with no language feature on: it gets the protocol and the
//! SDK locations, not a C++ adapter it never asked for.
//! 纯 Rust 宿主在不打开任何语言特性时编译本 crate：它得到协议与 SDK 位置，而不是它从未请求的 C++
//! 适配器。

#![warn(missing_docs)]

pub mod abi_graph;
pub mod semantics;
// Historical public module paths stay as thin re-exports of the encapsulated C++ module.
// 历史公开模块路径保留为封装后 C++ 模块的薄重导出。
#[cfg(feature = "lang-cpp")]
pub use languages::cpp::declarations as cpp_declarations;
pub use languages::cpp::header as cpp_header;
pub use languages::cpp::names as cpp_names;
#[cfg(feature = "lang-cpp")]
pub use languages::cpp::native as cpp_native;
#[cfg(feature = "lang-cpp")]
pub use languages::cpp::record as cpp_record;
#[cfg(feature = "lang-cpp")]
pub use languages::cpp::syntax as cpp_syntax;
pub mod external_faces;
pub mod object_map;
pub mod registration;
pub mod sdk;
pub mod source_map;
pub mod source_ref;
pub mod source_scope;

// The multilingual production pipeline: project resolution, per-language planning and execution,
// package sources, remote workers, delivery and verification. It is one module of this crate rather
// than a second multilingual system beside it; the toolchain's execution surfaces call it through a
// thin re-export.
// 多语言生产管线：项目解析、按语言的计划与执行、包源、远程 worker、交付与验证。它是本 crate 的一个
// 模块，而不是旁边的第二套多语言系统；工具链的执行面经一层薄重导出调用它。
/// The cached file facts a plan identity is computed from.
/// 计划身份所依据的文件事实缓存。
#[cfg(feature = "production")]
pub mod facts;

/// The multilingual production pipeline: project resolution, per-language planning and execution,
/// package sources, remote workers, delivery and verification.
/// 多语言生产管线：项目解析、按语言的计划与执行、包源、远程 worker、交付与验证。
#[cfg(feature = "production")]
pub mod production;

pub mod languages;

#[cfg(feature = "lang-cpp")]
pub use languages::cpp::CppLanguage;

#[cfg(feature = "lang-python")]
pub use languages::python::PythonLanguage;

#[cfg(feature = "lang-zig")]
pub use languages::zig as zig_native;

#[cfg(feature = "lang-c")]
pub use languages::c as c_native;

#[cfg(feature = "lang-js")]
pub use languages::js::JsLanguage;

#[cfg(feature = "lang-ts")]
pub use languages::ts::TsLanguage;

#[cfg(feature = "lang-go")]
pub use languages::go::GoLanguage;

#[cfg(feature = "lang-java")]
pub use languages::java::JavaLanguage;

#[cfg(feature = "lang-csharp")]
pub use languages::csharp::CSharpLanguage;

#[cfg(feature = "lang-c")]
pub use languages::c::CLanguage;

#[cfg(feature = "lang-zig")]
pub use languages::zig::ZigLanguage;
