//! Stable registration-face metadata model.
//! 稳定的注册面元数据模型。

#[path = "face/face.rs"]
pub mod face;
#[path = "face_manifest.rs"]
mod face_manifest;
#[path = "parse.rs"]
pub mod parse;

/// The parsed registration-face metadata shared by authoring and Studio.
/// authoring 与 Studio 共用的注册面元数据模型。
pub(crate) use self::face_manifest::FaceManifest;
