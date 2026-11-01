//! Core registry modules. Concrete application faces live in the consuming crate.
//! 内核注册模块。具体应用的注册面住在消费它的 crate 里。
//!
//! Every module here is re-exported flat so a surface can glob one namespace,
//! but no name is exported twice: a type owned by `declaration` is re-exported
//! from there only, and the plugin modules re-export it solely inside their own
//! page.
//! 这里的每个模块都被平铺重导出，方便执行面 glob 一个命名空间；但没有任何名字被导出
//! 两次：`declaration` 拥有的类型只从那里重导出，插件模块只在各自页面内重导出它。

#[path = "registry_core/adoption/adoption.rs"]
pub mod adoption;
#[path = "registry_core/authoring/authoring.rs"]
pub mod authoring;
#[path = "registry_core/declaration/declaration.rs"]
pub mod declaration;
#[path = "registry_core/diagnostic/diagnostic.rs"]
pub mod diagnostic;
#[path = "registry_core/identity/identity.rs"]
pub mod identity;
#[path = "registry_core/interchange.rs"]
pub mod interchange;
#[path = "registry_core/json.rs"]
pub mod json;
#[path = "registry_core/lexicon/lexicon.rs"]
pub mod lexicon;
#[path = "registry_core/maintenance/maintenance.rs"]
pub mod maintenance;
#[path = "registry_core/mir/mir.rs"]
pub mod mir;
#[path = "registry_core/plugin/plugin.rs"]
pub mod plugin;
#[path = "registry_core/release.rs"]
pub mod release;
#[path = "registry_core/requirements.rs"]
pub mod requirements;
#[path = "registry_core/shape/shape.rs"]
pub mod shape;
#[path = "registry_core/source/source.rs"]
pub mod source;
#[path = "registry_core/syntax/syntax.rs"]
#[cfg(feature = "syntax")]
pub mod syntax;
#[path = "registry_core/tree/tree.rs"]
pub mod tree;

pub use adoption::*;
pub use authoring::*;
pub use declaration::*;
pub use diagnostic::*;
pub use identity::*;
pub use interchange::*;
pub use json::*;
pub use lexicon::*;
pub use maintenance::*;
pub use mir::*;
pub use plugin::*;
pub use release::*;
pub use requirements::*;
pub use shape::*;
pub use source::*;
#[cfg(feature = "syntax")]
pub use syntax::*;
pub use tree::*;

#[path = "registry_core/graft_builder.rs"]
pub mod graft_builder;
pub use graft_builder::*;
