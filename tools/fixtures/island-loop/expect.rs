//! What the gate asked for, written by the gate itself — never read back from the product.
//! 门禁自己提出问题，也由它自己写下期望——从不从产品那里抄答案。
//!
//! The gate writes this file from the requests it sent (module, file, parent), so a tree that
//! agrees with itself but not with the request fails here.
//! 门禁用它发出的请求（模块、文件、父级）写下本文件，因此一棵自洽却与请求不符的树会在这里失败。

/// One face the gate asked the product to create.
/// 门禁要求产品创建的一个注册面。
pub struct Face {
    /// The logical path the requests named.
    /// 请求点名的逻辑路径。
    pub path: &'static str,
    /// The kind the request declared.
    /// 请求声明的 kind。
    pub kind: &'static str,
    /// The package-relative source this layout must record for the file the request named.
    /// 本布局必须为请求点名的文件记录下的包内相对源码。
    pub source: &'static str,
    /// The logical path of the face it hangs under; empty means the registry root.
    /// 它挂在其下的面的逻辑路径；空串表示注册树根。
    pub parent: &'static str,
}

/// One object's ledger entry: the stable identity, where it lives now, and every instance.
/// 一个对象的台账条目：稳定身份、它现在住在哪里，以及它的每个实例。
pub struct Object {
    /// The logical path the object was **created** at — its stable identity derives from it.
    /// 对象**被创建**时的逻辑路径——它的稳定身份由它推导。
    pub created_at: &'static str,
    /// The logical path it declares now.
    /// 它此刻声明的逻辑路径。
    pub path: &'static str,
    /// The package-relative source it currently comes from.
    /// 它此刻来自的包内相对源码。
    pub source: &'static str,
    /// The explicit directory mapping the object carries.
    /// 该对象携带的显式目录映射。
    pub directory: &'static str,
    /// The kind each instance was declared with.
    /// 每个实例声明时的 kind。
    pub kind: &'static str,
    /// Every source instance, oldest first, as `(package-relative source, record spelling)`.
    /// 每个源码实例，最旧的在前，形如 `(包内相对源码, 记录拼写)`。
    pub instances: &'static [(&'static str, &'static str)],
}

pub const FACES: &[Face] = &[__FACES__];

pub const OBJECTS: &[Object] = &[__OBJECTS__];
