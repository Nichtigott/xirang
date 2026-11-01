//! Registration-face discovery for the conservative first-pass scope.
//! 保守首轮作用域的注册面发现。
//!
//! A face is a source file whose parsed root object declares a kind. Discovery
//! shares the cached node identities, so the scope gates exactly the faces the
//! generated plan will materialize.
//! 注册面是其解析出的根对象声明了 kind 的源文件。发现过程共享缓存的 node 身份，
//! 因此作用域门控的正是生成计划将会实例化的那些面。

use std::fs;
use std::path::{Path, PathBuf};

use xirang_kernel::lexicon;

use super::discovery_node::{Node, relative_display};
use super::face_syntax_check::parsed_face;
use super::node_identity::{CACHED_NODE_IDS, node_id};
use super::registry_identity::{self, NodeId};
use super::scope::SourceScope;

#[derive(Clone, Debug)]
pub(crate) struct FaceSource {
    pub(crate) id: NodeId,
    pub(crate) source: PathBuf,
    pub(crate) module: String,
}

pub(crate) fn collect_faces(src: &Path, nodes: &[Node]) -> Vec<FaceSource> {
    let mut faces = Vec::new();
    visit_faces_inner(src, nodes, &mut faces);
    faces
}

fn visit_faces_inner(src: &Path, nodes: &[Node], faces: &mut Vec<FaceSource>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if !lexicon::is_registration_path(&relative) {
                let cached = CACHED_NODE_IDS.get().and_then(|cache| cache.get(&relative));
                let parsed = cached.is_none().then(|| {
                    fs::read_to_string(file)
                        .ok()
                        .and_then(|source| parsed_face(&source, &relative))
                        .and_then(|face| face.path("kind"))
                });
                let (id, kind) = cached
                    .map(|(id, kind)| (*id, kind.clone()))
                    .or_else(|| {
                        parsed.flatten().map(|kind| {
                            (registry_identity::package_node_id(&relative, &kind), kind)
                        })
                    })
                    .unwrap_or_else(|| {
                        (
                            registry_identity::package_node_id(&relative, ""),
                            String::new(),
                        )
                    });
                if !kind.is_empty() {
                    let module = relative
                        .rsplit_once('/')
                        .map_or_else(|| relative.trim_end_matches(".rs"), |(parent, _)| parent)
                        .replace('/', "::");
                    faces.push(FaceSource {
                        id,
                        source: file.clone(),
                        module,
                    });
                }
            }
        }
        visit_faces_inner(src, &node.children, faces);
    }
}

pub(crate) fn has_selected_face(
    src: &Path,
    node: &Node,
    selected_ancestor: bool,
    scope: &SourceScope,
) -> bool {
    if scope.roots.is_none() || selected_ancestor {
        return true;
    }
    if node_id(src, node).is_some_and(|id| {
        scope
            .roots
            .as_ref()
            .is_some_and(|roots| roots.contains(&id))
    }) {
        return true;
    }
    node.children
        .iter()
        .any(|child| has_selected_face(src, child, false, scope))
}
