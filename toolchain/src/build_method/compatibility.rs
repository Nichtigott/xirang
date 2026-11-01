//! The module paths this tree's objects have **left behind**, kept nameable by the header alias and by
//! the derivation.
//! 这棵树里的对象**离开过**的模块路径，由头部别名与推导共同保持可被点名。
//!
//! A physical move changes a face's module path, and a C++ declaration names its parent through
//! `::xirang::faces::<module>::NODE_ID`. Rewriting that declaration is not always allowed — it may live
//! in a declared (read-only) source — so the header keeps a namespace for the retired path, and this
//! module is the one place that decides which paths those are.
//! 物理搬动改变面的模块路径，而 C++ 声明经 `::xirang::faces::<模块>::NODE_ID` 点名父级。改写那条声明并不
//! 总是被允许——它可能住在已声明（只读）来源里——因此头部为退役路径保留一节命名空间，而本模块是判定"哪些
//! 路径属于它们"的唯一地方。
//!
//! **Both halves ask here**: the header generator (what to emit) and the derivation (what a parent
//! reference may resolve to). One of them alone would be two answers to one project — the defect this
//! whole area keeps producing.
//! **两半都在这里问**：头部生成器（发射什么）与推导（父引用可以解析到什么）。只有一半，就是同一个工程两份
//! 答案——正是这一带反复生产的缺陷。

use std::collections::BTreeMap;
use std::path::Path;

use super::declared_native::DeclaredFace;
use super::registry_identity::NodeId;
use super::static_plan::StaticFaceRecord;

/// One live face as the compatibility question needs it: its source, its module path, its identity.
/// 兼容性问题所需要的一个活面：它的源码、模块路径与身份。
pub(crate) type LiveFace = (String, String, NodeId);

/// The live faces a build derives from its static plan and its declarations.
/// 一次构建从其静态计划与声明推导出的活面。
pub(crate) fn live_faces(
    rust: &[StaticFaceRecord],
    declared: &[DeclaredFace],
    namespace: &str,
) -> Vec<LiveFace> {
    let mut live = rust
        .iter()
        .map(|face| (face.source.clone(), face.module.clone(), face.id))
        .collect::<Vec<_>>();
    live.extend(declared.iter().map(|face| {
        (
            face.source.clone(),
            super::declared_native::module_of(&face.source),
            NodeId::from_namespaced_path(namespace, &face.source, &face.name),
        )
    }));
    live
}

/// The compatibility namespaces this build's header must carry.
/// 本次构建的头部必须携带的兼容命名空间。
pub(crate) fn header_namespaces(package_root: &Path, live: &[LiveFace]) -> Vec<(String, [u8; 16])> {
    let ledger = super::object_map::ObjectMap::read_from(package_root).unwrap_or_default();
    if ledger.entries().is_empty() {
        return Vec::new();
    }
    let live_bytes = live
        .iter()
        .map(|(source, module, id)| (source.clone(), module.clone(), id.into_bytes()))
        .collect::<Vec<_>>();
    super::face_header::compatibility_namespaces(&live_bytes, &ledger)
}

/// Add every retired module path to a `known` map, so a declaration that names a **moved** parent by
/// its old path resolves exactly as the header aliases it.
/// 把每个退役模块路径加进 `known` 映射，使按旧路径点名**被搬**父级的声明，像头部起别名那样解析得到。
pub(crate) fn extend_known(
    known: &mut BTreeMap<String, (String, NodeId)>,
    package_root: &Path,
    live: &[LiveFace],
) {
    for (module, id) in header_namespaces(package_root, live) {
        known
            .entry(format!("{module}::NODE_ID"))
            .or_insert((module, NodeId::from_raw(id)));
    }
}
