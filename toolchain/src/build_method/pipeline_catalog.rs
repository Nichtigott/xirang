//! The shared logical catalog, built from object relations rather than source paths.
//! 共享逻辑目录，按对象关系而不是源码路径构建。

use std::path::Path;

use crate::build_method::declared_native;
use crate::build_method::declared_native::{DeclaredFace, Resolved};
use crate::build_method::static_plan::StaticFaceRecord;
use crate::build_method::{face_view, registry_identity};

/// The catalog of this build: `(logical path below root, identity bytes)`.
/// 本次构建的目录：`(root 之下的逻辑路径, 身份字节)`。
///
/// Rust names come from the same walk that publishes logical paths (parent ids + resolved registry
/// names); native faces contribute their declared name and resolved parent. Resolving **every**
/// declaration here is deliberate: a host that handed a subtree away still has to publish the native
/// objects it no longer compiles, or the two crates would expose different public directories.
/// Rust 名字来自发布逻辑路径的同一次遍历（父级 id + 解析后的注册名）；外语面贡献声明名与解析后的父级。
/// 这里有意解析**全部**声明：交出子树的宿主仍必须发布它不再编译的外语对象，否则两个 crate 会暴露不同的
/// 公开目录。
/// Seed the resolver table with the logical catalog paths of Rust faces.
/// 把 Rust 面的逻辑目录路径预置进解析表。
pub(crate) fn seed_known(
    package_root: &Path,
    faces: &[StaticFaceRecord],
    known: &mut std::collections::BTreeMap<String, (String, registry_identity::NodeId)>,
) {
    let names: Vec<(registry_identity::NodeId, registry_identity::NodeId, String)> = faces
        .iter()
        .map(|face| {
            let name = std::fs::read_to_string(package_root.join(&face.source))
                .ok()
                .and_then(|source| {
                    crate::build_method::parsed_face(&source, &face.source)
                        .map(|syntax| face_view::resolved_registry_name(&syntax, &face.module))
                })
                .unwrap_or_else(|| {
                    face.module
                        .rsplit("::")
                        .next()
                        .unwrap_or(face.module.as_str())
                        .to_owned()
                });
            (face.id, face.parent, name)
        })
        .collect();
    let paths = face_view::logical_paths(registry_identity::package_root_node_id(), &names);
    for (id, _, _) in &names {
        if let Some(path) = paths.get(id).and_then(|path| path.strip_prefix("root/")) {
            let path = path.replace('/', "::");
            known.insert(path.clone(), (path, *id));
        }
    }
}

pub(crate) fn build(
    package_root: &Path,
    declared: &[DeclaredFace],
    known: &std::collections::BTreeMap<String, (String, registry_identity::NodeId)>,
    faces: &[StaticFaceRecord],
) -> Result<Vec<(String, [u8; 16])>, String> {
    // Rust identities and parent links come from the static plan itself, not from a second walk:
    // an inherited island descendant can compute a different id in the discovery walk than the one
    // the plan mounts, and a catalog keyed by the second id would name a different object.
    // Rust 身份与父链直接取自静态计划，而不是再走一遍：孤岛继承后代在发现遍历里可能算出与计划不同的
    // id，而按第二个 id 键入的目录会点名另一个对象。
    let mut rust_names: Vec<(registry_identity::NodeId, registry_identity::NodeId, String)> = faces
        .iter()
        .map(|face| {
            let name = std::fs::read_to_string(package_root.join(&face.source))
                .ok()
                .and_then(|source| {
                    crate::build_method::parsed_face(&source, &face.source)
                        .map(|syntax| face_view::resolved_registry_name(&syntax, &face.module))
                })
                .unwrap_or_else(|| {
                    face.module
                        .rsplit("::")
                        .next()
                        .unwrap_or(face.module.as_str())
                        .to_owned()
                });
            (face.id, face.parent, name)
        })
        .collect();
    let rust_paths =
        face_view::logical_paths(registry_identity::package_root_node_id(), &rust_names);
    // A C++ declaration may name a Rust parent through the shared catalog path, which is the
    // registry-name chain rather than the module chain. Seed those paths into the resolver table.
    // C++ 声明可能经共享目录路径点名 Rust 父，而那是注册名链、不是模块链；把这些路径预置进解析表。
    let mut known = known.clone();
    for (id, _, _) in &rust_names {
        if let Some(path) = rust_paths
            .get(id)
            .and_then(|path| path.strip_prefix("root/"))
        {
            let path = path.replace('/', "::");
            known.insert(path.clone(), (path, *id));
        }
    }
    let mut native: Vec<Resolved> =
        declared_native::resolve(&registry_identity::package_namespace(), declared, &known)?;
    native.sort_by_key(|left| left.id);
    rust_names.extend(
        native
            .iter()
            .map(|face| (face.id, face.parent, face.name.clone())),
    );
    let paths = face_view::logical_paths(registry_identity::package_root_node_id(), &rust_names);
    let mut entries = Vec::new();
    for (id, _, _) in &rust_names {
        let Some(path) = paths.get(id) else {
            continue;
        };
        let Some(path) = path.strip_prefix("root/") else {
            continue;
        };
        entries.push((path.to_owned(), id.into_bytes()));
    }
    // Retired logical paths stay nameable: a C++ declaration generated with
    // `crate::catalog::panel::Area` must still resolve after the object is renamed or moved to a
    // different logical path. The ledger carries those paths; a live path always wins over an alias.
    // 退役逻辑路径继续可点名：用 `crate::catalog::panel::Area` 生成的 C++ 声明，在对象改名或换逻辑
    // 路径后仍须解析。台账携带这些路径；活路径永远优先于别名。
    if let Ok(ledger) = xirang_multilang::object_map::ObjectMap::read_from(package_root) {
        let live: std::collections::BTreeMap<String, registry_identity::NodeId> = entries
            .iter()
            .map(|(path, bytes)| (path.clone(), registry_identity::NodeId::from_raw(*bytes)))
            .collect();
        let mut aliases: std::collections::BTreeMap<String, registry_identity::NodeId> =
            std::collections::BTreeMap::new();
        for entry in ledger.entries() {
            let Some(id) = entry.current_id() else {
                continue;
            };
            for retired in &entry.retired_paths {
                let Some(path) = retired.strip_prefix("root/") else {
                    continue;
                };
                if path.is_empty() {
                    continue;
                }
                if let Some(live_id) = live.get(path) {
                    if *live_id == id
                        || entry
                            .instances
                            .iter()
                            .any(|instance| instance.id == *live_id)
                    {
                        // The object returned to its old path (possibly under a different source
                        // instance); no alias is needed, and the live path is still the same stable
                        // object. Only a **different** stable object is a conflict.
                        // 对象回到了旧路径（可能是不同源码实例）；不需要别名，活路径仍是同一个稳定对象。
                        // 只有**不同**稳定对象才是冲突。
                        continue;
                    }
                    // The path now belongs to another live object. Keeping no alias would make an
                    // old reference silently resolve to the **new** object, which is worse than a
                    // refusal: the build stops and names both objects so the reference can be
                    // migrated explicitly.
                    // 路径现在属于另一个活对象。不加别名会让旧引用静默解析到**新**对象，这比拒绝更糟：
                    // 构建停下并点名两个对象，让引用能被显式迁移。
                    return Err(format!(
                        "the retired catalog path `{path}` is now live for object {live_id}, but the \
                         alias belongs to object {id}; an old reference would silently bind to the \
                         wrong object. Nothing was published; migrate the old reference or choose a \
                         different path."
                    ));
                }
                if let Some(existing) = aliases.get(path) {
                    if *existing != id {
                        return Err(format!(
                            "the retired catalog path `{path}` names two objects ({existing} and                              {id}); a compatibility alias would be ambiguous"
                        ));
                    }
                    continue;
                }
                aliases.insert(path.to_owned(), id);
                entries.push((path.to_owned(), id.into_bytes()));
            }
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries.dedup_by(|left, right| left.0 == right.0);
    Ok(entries)
}

/// Render the catalog text, reporting one refused relation instead of panicking a build.
/// 渲染目录文本；对象关系被拒时报告错误，而不是让构建 panic。
pub(crate) fn render(
    package_root: &Path,
    declared: &[DeclaredFace],
    known: &std::collections::BTreeMap<String, (String, registry_identity::NodeId)>,
    faces: &[StaticFaceRecord],
) -> (String, String, Option<String>) {
    let entries = match build(package_root, declared, known, faces) {
        Ok(entries) => entries,
        Err(error) => return (String::new(), String::new(), Some(error)),
    };
    let rust = match crate::build_method::renderer::render_catalog(faces, &entries) {
        Ok(text) => text,
        Err(error) => return (String::new(), String::new(), Some(error)),
    };
    let cpp = match xirang_multilang::cpp_header::render_catalog(&entries) {
        Ok(text) => text,
        Err(error) => return (String::new(), String::new(), Some(error)),
    };
    (rust, cpp, None)
}

/// Append the Rust catalog to `generated`, push a refusal if any, and return the C++ catalog.
/// 把 Rust 目录追加到 `generated`，有拒绝就记录，并返回 C++ 目录。
pub(crate) fn append(
    generated: &mut String,
    package_root: &Path,
    declared: &[DeclaredFace],
    known: &std::collections::BTreeMap<String, (String, registry_identity::NodeId)>,
    faces: &[StaticFaceRecord],
    errors: &mut crate::build_method::diagnostics::BuildDiagnostics,
) -> String {
    let (rust, cpp, error) = render(package_root, declared, known, faces);
    if let Some(error) = error {
        errors.push(crate::build_method::diagnostics::BuildDiagnostic::new(
            "catalog", error,
        ));
    }
    generated.push_str(&rust);
    cpp
}

#[cfg(test)]
#[path = "pipeline_catalog_tests.rs"]
mod pipeline_catalog_tests;
