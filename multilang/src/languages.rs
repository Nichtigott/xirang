//! The language implementations this crate can carry, one module each.
//! 本 crate 可以携带的语言实现，每种语言一个模块。
//!
//! A language module is compiled only when its feature is on. The module name is the language id
//! the registry keys on, so adding a language is a feature, a module, and one registry entry.
//! 语言模块只在其特性打开时编译。模块名就是注册表索引的语言标识，因此新增一种语言是一个特性、一个
//! 模块、一条注册表条目。

#[path = "languages/cpp/cpp.rs"]
pub mod cpp;
pub mod kit;
pub mod naming;
pub mod rust;

#[cfg(feature = "lang-python")]
#[path = "languages/python/python.rs"]
pub mod python;

#[cfg(feature = "lang-zig")]
#[path = "languages/zig/zig.rs"]
pub mod zig;

#[cfg(feature = "lang-c")]
#[path = "languages/c/c.rs"]
pub mod c;

#[cfg(feature = "lang-js")]
#[path = "languages/js/js.rs"]
pub mod js;

#[cfg(feature = "lang-ts")]
#[path = "languages/ts/ts.rs"]
pub mod ts;

#[cfg(feature = "lang-go")]
#[path = "languages/go/go.rs"]
pub mod go;

#[cfg(feature = "lang-java")]
#[path = "languages/java/java.rs"]
pub mod java;

#[cfg(feature = "lang-csharp")]
#[path = "languages/csharp/csharp.rs"]
pub mod csharp;
