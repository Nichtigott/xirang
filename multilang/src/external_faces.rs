//! The normalized facts an external object contributes, independent of the C++ binary format.
//! 外部目标文件贡献的规范化事实，与 C++ 二进制格式无关。
//!
//! The pipeline, renderer and snapshot builder depend on these facts whether or not a build carries
//! the C++ adapter. The decoder that fills them lives behind `native-record` in
//! `crate::languages::kit::record`; this module stays compiled so a pure-Rust build can still name
//! an empty external slice without compiling the reader.
//! 无论构建是否携带 C++ 适配器，管线、渲染器与快照构建都依赖这些事实。填充它们的解码器住在
//! `native-record` 后面的 `crate::languages::kit::record`；本模块保持编译，使纯 Rust 构建仍能命名一个空的外部切片，
//! 而不必编译读取器。

use std::path::Path;

use xirang_kernel::identity::NodeId;

/// One object file together with the package-relative spelling of the source that
/// produced it.
/// 一个目标文件，以及产生它的源码在包内的相对拼写。
///
/// The spelling is an input, not a guess: a face's identity hashes the path its
/// source has **inside the package**, and an object's path on disk is machine
/// specific. A caller that let this side derive the spelling from the object path
/// would get an identity that changes with the checkout's location.
/// 这份拼写是输入而不是猜测：面的身份散列的是源码在**包内**的路径，而目标文件在磁盘上的
/// 路径与机器相关。若让这一侧从目标文件路径推导拼写，得到的身份会随检出位置变化。
pub struct ExternalSource<'a> {
    /// The object file to read.
    /// 要读取的目标文件。
    pub object: &'a Path,
    /// The source's path inside its package, for example `native/area/area.cpp`.
    /// 源码在其包内的路径，例如 `native/area/area.cpp`。
    pub source: &'a str,
}

/// A declared face with the identity the build assigned it.
/// 一个已声明的面，连同构建分配给它的身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalFaceRecord {
    /// The identity this face earns from its spelling and name.
    /// 本面由其拼写与名字取得的身份。
    pub id: NodeId,
    /// The parent this declaration names.
    /// 该声明点名的父面。
    pub parent: NodeId,
    /// Whether children may hang from this face.
    /// 是否可以有子面挂在它下面。
    pub owns_registry: bool,
    /// The name this face has inside its parent.
    /// 面在父面内的名字。
    pub name: String,
    /// The exported names, in declaration order.
    /// 导出名，按声明顺序。
    pub exports: Vec<String>,
    /// The spelling this face was read from, kept for the generated snapshot's location.
    /// 读取本面所用的那份拼写，留给生成快照的位置字段。
    pub source: String,
    /// The remaining fields of a version-2 record. A version-1 record leaves them empty, which is
    /// the standard registration default.
    /// 版本 2 记录的其余字段。版本 1 记录里它们为空，也就是标准注册默认值。
    pub registry_name: String,
    /// The author-owned stable name.
    /// 作者拥有的稳定名。
    pub stable_name: String,
    /// Capability names this face provides.
    /// 本面提供的能力名。
    pub provides: Vec<String>,
    /// Capability requirements, as `(capability, provider)`.
    /// 能力需求，按 `(capability, provider)` 给出。
    pub requires: Vec<(String, String)>,
    /// The preset type path, for display and diagnostics.
    /// preset 类型路径，供显示与诊断。
    pub preset: String,
    /// The parts type path.
    /// parts 类型路径。
    pub parts: String,
    /// The construction parameter type path.
    /// 构造参数类型路径。
    pub params: String,
    /// The handle type path.
    /// handle 类型路径。
    pub handle: String,
    /// The Chinese half of the display name.
    /// 显示名的中文部分。
    pub name_zh: String,
    /// The English half of the display name.
    /// 显示名的英文部分。
    pub name_en: String,
    /// The Chinese half of the summary.
    /// 摘要的中文部分。
    pub summary_zh: String,
    /// The English half of the summary.
    /// 摘要的英文部分。
    pub summary_en: String,
    /// The external registry this face is provisioned from.
    /// 本面取自哪个外部注册机。
    pub getting_from_other_registry: String,
}

impl ExternalFaceRecord {
    /// A record with every version-2 field at its default.
    /// 一个版本 2 字段都取默认值的记录。
    ///
    /// The constructor exists so a fixture that only cares about the tree facts cannot forget a
    /// field: a version-1 record carries the same defaults, so "the author did not declare it" and
    /// "the record could not carry it" have one spelling here.
    /// 这个构造器让只关心树事实的夹具不可能漏掉字段：版本 1 记录携带同样的默认值，因此"作者没有声明"
    /// 与"记录携带不了"在这里只有一种拼法。
    pub fn new(
        id: NodeId,
        parent: NodeId,
        owns_registry: bool,
        name: String,
        exports: Vec<String>,
        source: String,
    ) -> Self {
        Self {
            id,
            parent,
            owns_registry,
            name,
            exports,
            source,
            registry_name: String::new(),
            stable_name: String::new(),
            provides: Vec::new(),
            requires: Vec::new(),
            preset: String::new(),
            parts: String::new(),
            params: String::new(),
            handle: String::new(),
            name_zh: String::new(),
            name_en: String::new(),
            summary_zh: String::new(),
            summary_en: String::new(),
            getting_from_other_registry: String::new(),
        }
    }
}

/// What a project declares about its native sources: one recipe, many sources.
/// 项目对本地源码的声明：一份配方、多个源文件。
#[derive(Clone, Debug)]
pub struct Recipe {
    /// The C++ compiler program.
    /// C++ 编译器程序。
    pub program: String,
    /// Arguments passed before the input file.
    /// 输入文件之前传入的参数。
    pub arguments: Vec<String>,
    /// Package-relative include directories.
    /// 包内相对 include 目录。
    pub includes: Vec<String>,
    /// The archiver program.
    /// 归档器程序。
    pub archive: String,
}
