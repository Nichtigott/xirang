//! The registration facts a callgraph answer attaches to functions: which face a source file
//! declares, and which implementation a slot is bound to.
//! 调用图答案附加到函数上的注册事实：一个源文件声明了哪个面，以及一个槽绑定到哪个实现。
//!
//! Building them parses the tree's declarations, and a workspace query needs them for **every**
//! member. Without this type the corpus loader asked for the same facts three times per member (once
//! for the Rust sources, once for Python, once for C++) and twice over (the second pass for callee
//! definitions) — measured on this checkout: 30 s of a 41 s query was the same declaration walk
//! repeated. The type makes the fact computed once per root and applied to every language's sources.
//! 构建它们要解析树的声明，而一次工作区查询对**每个**成员都需要它们。没有这个类型时，语料加载器对同一
//! 个成员问三遍（Rust 源码一遍、Python 一遍、C++ 一遍）、整体再问两遍（第二趟找被调用者定义）——本检出
//! 实测：41 s 的查询里有 30 s 是同一次声明遍历被重复。这个类型让事实对每个根只算一次、再应用到各语言的
//! 源码上。

use std::collections::BTreeMap;
use std::path::Path;

/// The registration facts one root contributes to a callgraph answer.
/// 一个根为调用图答案贡献的注册事实。
pub(crate) struct RegistrationOwners {
    /// Every face the root declares, with the source file it names.
    /// 该根声明的每个面，及其点名的源文件。
    views: Vec<crate::build_method::FaceView>,
    /// Each slot's bound implementation, keyed by registration path.
    /// 每个槽绑定的实现，按注册路径索引。
    bindings: BTreeMap<String, String>,
}

/// Read one root's registration facts; `None` when the root has no readable registry.
/// 读取一个根的注册事实；根没有可读注册机时为 `None`。
pub(crate) fn registration_owners(root: &Path) -> Option<RegistrationOwners> {
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let views = crate::build_method::face_views(root, &namespace).ok()?;
    // Reuse the build's own normalized declaration reader and projection. No second graft parser:
    // the callgraph asks the same path the authoring/CLI surfaces ask.
    // 复用构建自己的声明读取器与投影。没有第二份 graft 解析器：调用图问的是创作/CLI 面同一条路径。
    // A root with no faces needs neither the graft declarations nor the slot bindings: both are
    // keyed by registration paths that only a face can name. Measured on this checkout, asking for
    // them anyway was 6.2 s of a 10 s registration read (the toolchain member has no faces and its
    // declaration walk is the expensive one).
    // 没有面的根既不需要 graft 声明也不需要槽绑定：两者都以只有面才能命名的注册路径为键。本检出实测：
    // 照问不误要花 10 s 注册读取里的 6.2 s（toolchain 成员没有面，而它的声明遍历正是贵的那一份）。
    let declared = if views.is_empty() {
        None
    } else {
        crate::build_method::declared_grafts(root).ok()
    };
    let bindings = if views.is_empty() {
        BTreeMap::new()
    } else {
        crate::build_method::slot_bindings(root, &views, declared.as_ref())
            .unwrap_or_default()
            .into_iter()
            .map(|binding| (binding.path, binding.implementation))
            .collect()
    };
    Some(RegistrationOwners { views, bindings })
}

impl RegistrationOwners {
    /// Attach this root's face and binding to every function of the matching files.
    /// 把这个根的面与绑定附加到匹配文件的每个函数上。
    ///
    /// A source belongs to a face when its package-relative spelling is the face's source or ends
    /// with `/` + that source; the workspace loader labels files by member, so both spellings occur.
    /// 一个源属于某个面，当它的包内相对拼写等于该面的 source，或以 `/` + 该 source 结尾；工作区加载器按
    /// 成员给文件加标签，因此两种拼写都会出现。
    pub(crate) fn apply(&self, sources: &mut [crate::mcp::source_index::SourceFile]) {
        for file in sources {
            let Some(view) = self.views.iter().find(|view| {
                file.relative == view.source
                    || file.relative.ends_with(&format!("/{}", view.source))
            }) else {
                continue;
            };
            for function in &mut file.functions {
                function.registration = Some(view.path.clone());
                function.binding = self.bindings.get(&view.path).cloned();
            }
        }
    }
}
