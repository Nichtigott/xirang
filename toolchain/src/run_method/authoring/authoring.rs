//! File-backed authoring for XiRang registration faces.
//! XiRang 注册面的文件化创作支持。

use std::fs;
use std::path::{Path, PathBuf};

use crate::run_method::{NodeId, ROOT_NODE_ID, RegistrationSnapshot, Registry};

use self::context::{
    identity_base, is_parent_component, normalize_kind_name, normalized_path, package_root,
    scan_root, validate_name,
};
use self::parse::trait_names_from_paths;

/// The generation marker, re-exported so the module's own readers keep their path.
/// 生成标记，重导出以便本模块自己的读者保留原有路径。
///
/// The constant itself lives in `kernel::lexicon` with the other text contracts: the bridge asks the
/// same question before suggesting a repair, and two literals would be two answers.
/// 常量本体住在 `kernel::lexicon`，与其它文本契约一起：桥在建议修复前会问同一个问题，而两个字面量就是
/// 两个答案。
pub(crate) use xirang_kernel::lexicon::GENERATED_MARKER;

#[path = "context.rs"]
pub mod context;
#[path = "external_graft/external_graft.rs"]
pub mod external_graft;
#[path = "face_file.rs"]
mod face_file;
#[path = "filesystem.rs"]
pub mod filesystem;
#[path = "manifest/manifest.rs"]
pub mod manifest;
#[path = "operations/operations.rs"]
pub mod operations;
#[path = "parse/parse.rs"]
pub mod parse;
#[path = "snapshot/snapshot.rs"]
pub mod snapshot;
#[path = "write_scope.rs"]
pub mod write_scope;
#[path = "write_transaction.rs"]
pub mod write_transaction;
// The historical public path stays: a host that already writes
// `crate::run_method::authoring::validation::…` keeps compiling. The module itself is
// `context` (audit `NAM-03`); this alias is pinned by `conventions/src/shims.rs`, so it cannot
// be deleted by accident.
// 历史公开路径保留：已经写着 `crate::run_method::authoring::validation::…` 的宿主照样编译。
// 模块本体现在是 `context`（审计 `NAM-03`）；这条别名由 `conventions/src/shims.rs` 钉住，
// 不会因疏忽被删掉。
pub use self::context as validation;

pub use self::face_file::*;
/// The parsed face metadata shared by the form and the file authoring API.
/// 表单与文件创作 API 共用的注册面元数据模型。
pub(crate) use self::manifest::FaceManifest;
