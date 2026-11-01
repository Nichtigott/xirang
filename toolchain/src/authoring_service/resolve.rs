//! Resolving the facts a language adapter must not guess: the parent's identity and module, the
//! package layout, and the destination.
//! 解析语言适配器不许猜的事实：父的身份与模块、包布局，以及目的地。
//!
//! All of it needs the package root, the registry, or the declared native sources, so it lives in
//! the execution surface. The adapter receives **facts** and renders its own spelling; the surface
//! never decides which language's expression a request carries.
//! 这些都需要包根、注册树或声明式本地源码，因此住在执行面。适配器收到**事实**并渲染自己的拼法；执行面
//! 从不决定请求携带哪种语言的表达式。

use std::path::{Path, PathBuf};

use xirang_kernel::authoring::ParentReference;
use xirang_kernel::declaration::RegistrationSnapshot;
use xirang_kernel::identity::NodeId;

use crate::build_method::{declared_native, face_views};
use crate::source_layout::SourceLayout;

/// One parent a new face can hang under, with what its language spelling needs.
/// 新注册面可以挂靠的一个父，以及它语言拼法所需的信息。
#[derive(Clone, Debug)]
pub(crate) enum ParentFace {
    /// The registry root.
    /// 注册机根。
    Root,
    /// A Rust face from the package's own registry.
    /// 包自己注册树里的一个 Rust 面。
    Rust {
        /// The identity.
        /// 身份。
        id: NodeId,
        /// The source path, relative to the scan root.
        /// 源码路径，相对扫描根。
        source: String,
        /// The kind the build assigns it; a create records it beside the parent it hangs under.
        /// 构建为它分配的种类名；一次新建会把它记在所挂的父级旁边。
        kind: String,
        /// The Rust module address the generated tree mounts it at. This is a **fact from the
        /// generated module chain** (`FaceView.module`), not a re-derivation from its directory: an
        /// island mounts at its stem, so the two differ exactly where the feature matters.
        /// 生成树挂载它的 Rust 模块地址。这是**生成模块链的事实**（`FaceView.module`），不是从目录重新
        /// 推导：孤岛挂在 stem 上，两者恰好在功能要害处不同。
        module: String,
        /// The base this parent's `source` spelling is written against. A child's default directory
        /// is derived from it, so an island parent's children stay in the island instead of being
        /// pulled under `src/` by a prefix that only applies to identity-relative faces.
        /// 这个父的 `source` 拼写所依据的基准。子面的默认目录由它推导，因此孤岛父级的子面留在孤岛里，
        /// 而不会被一条只适用于身份相对面的前缀拉进 `src/`。
        base: xirang_multilang::source_ref::SourceBase,
        /// The public logical catalog path (`shop/panel`), below the registry root.
        /// 公开逻辑目录路径（`shop/panel`），位于注册树根之下。
        catalog: String,
    },
    /// A face declared in a native source.
    /// 声明在本地源码里的面。
    Native {
        /// The identity the build computes for it.
        /// 构建为它计算的身份。
        id: NodeId,
        /// The package-relative source spelling.
        /// 包内相对源码拼写。
        source: String,
        /// The declared name.
        /// 声明的名字。
        name: String,
        /// The public logical catalog path (`shop/panel`), below the registry root.
        /// 公开逻辑目录路径（`shop/panel`），位于注册树根之下。
        catalog: String,
    },
}

impl ParentFace {
    /// The parent identity, or `None` for the root.
    /// 父身份；根为 `None`。
    pub(crate) fn id(&self) -> Option<NodeId> {
        match self {
            Self::Root => None,
            Self::Rust { id, .. } | Self::Native { id, .. } => Some(*id),
        }
    }

    /// The module path the build assigned this parent.
    /// 构建分配给这个父的模块路径。
    ///
    /// This is the language-neutral fact every adapter renders from: C++ turns it into
    /// `::xirang::faces::<module>::NODE_ID`, Python into `faces.<module>.NODE_ID`. The surface never
    /// decides which spelling a language uses.
    /// 这是每个适配器渲染所用的语言无关事实：C++ 变成 `::xirang::faces::<module>::NODE_ID`，Python
    /// 变成 `faces.<module>.NODE_ID`。执行面从不决定一门语言用哪种拼法。
    pub(crate) fn module(&self) -> Result<String, String> {
        match self {
            Self::Root => Err(
                "the registry root has no module path; a language that can name the root uses \
                 `ParentReference::Root` directly"
                    .to_owned(),
            ),
            Self::Rust { module, .. } => Ok(module.clone()),
            Self::Native { source, .. } => Ok(declared_native::module_of(source)),
        }
    }

    /// The parent as the shared request vocabulary states it: facts, not a language's expression.
    /// 父面按共享请求词汇陈述：事实，而不是某种语言的表达式。
    pub(crate) fn request_reference(&self) -> Result<ParentReference, String> {
        match self {
            Self::Root => Ok(ParentReference::Root),
            Self::Rust { id, .. } | Self::Native { id, .. } => Ok(ParentReference::Object {
                id: *id,
                module: self.module()?,
            }),
        }
    }

    /// The shared catalog reference for a language that renders `crate::catalog::…`.
    /// 共享目录引用，供渲染 `crate::catalog::…` 的语言使用。
    pub(crate) fn catalog_reference(&self) -> Result<ParentReference, String> {
        match self {
            Self::Root => Ok(ParentReference::Root),
            Self::Rust { id, catalog, .. } | Self::Native { id, catalog, .. } => {
                Ok(ParentReference::Catalog {
                    id: *id,
                    path: catalog.trim_start_matches("root/").to_owned(),
                })
            }
        }
    }

    /// The Rust reference the generated `xirang_external` module gives this parent.
    /// 生成 `xirang_external` 模块为这个父给出的 Rust 引用。
    pub(crate) fn rust_reference(&self) -> Result<String, String> {
        match self {
            Self::Root => Ok("crate::root_node_id(crate::XIRANG_NAMESPACE)".to_owned()),
            Self::Rust { .. } => Err(
                "a Rust face under a Rust parent is rendered from the parent's module path, not \
                 from a generated reference"
                    .to_owned(),
            ),
            Self::Native { source, name, .. } => {
                let ident = declared_native::ident(source, name);
                Ok(format!("crate::xirang_external::{ident}::NODE_ID"))
            }
        }
    }
}

/// Every face the package declares in a native source, as the build discovers them.
/// 包在本地源码里声明的每个面，按构建发现的样子。
pub(crate) fn native_faces(root: &Path) -> Result<Vec<declared_native::DeclaredFace>, String> {
    Ok(declared_native::read(root)?
        .map(|(_, faces)| faces)
        .unwrap_or_default())
}

/// The identity the build computes for one declared native face.
/// 构建为一条声明式本地面计算的身份。
pub(crate) fn native_identity(namespace: &str, face: &declared_native::DeclaredFace) -> NodeId {
    NodeId::from_namespaced_path(namespace, &face.source, &face.name)
}

/// Rewrite a C++ declaration's parent reference to the shared catalog spelling, when the parent
/// can be found in the current view. Old `::xirang::faces::…` and already-catalog references are
/// both accepted; an unknown reference is returned unchanged so a read-only declaration is not
/// blocked by a catalog spelling this build cannot derive.
/// 当父能在当前视图中找到时，把 C++ 声明的父引用改写为共享目录拼法。旧的 `::xirang::faces::…`
/// 与已是目录的引用都接受；未知引用原样返回，避免构建推不出的目录拼法阻塞只读声明。
pub(crate) fn cpp_catalog_reference(
    root: &Path,
    namespace: &str,
    language: &xirang_kernel::interchange::LanguageId,
    parent: &ParentReference,
    registry: &xirang_kernel::tree::Registry,
    snapshots: &[RegistrationSnapshot],
    native: &[declared_native::DeclaredFace],
) -> Result<ParentReference, String> {
    if language.as_str() != "cpp" {
        return Ok(parent.clone());
    }
    let ParentReference::Named(text) = parent else {
        return Ok(parent.clone());
    };
    let spelling = text.trim().trim_start_matches("::");
    let key = spelling
        .strip_prefix("crate::catalog::")
        .and_then(|rest| rest.strip_suffix("::NODE_ID"))
        .map(|path| path.replace("::", "/"))
        .or_else(|| {
            spelling
                .strip_prefix("xirang::faces::")
                .and_then(|rest| rest.strip_suffix("::NODE_ID"))
                .map(str::to_owned)
        });
    let Some(key) = key else {
        return Ok(parent.clone());
    };
    // The old `::xirang::faces::<module>::NODE_ID` spelling names the module chain; the registry
    // knows the identity, and its path is the logical catalog path.
    // 旧的 `::xirang::faces::<module>::NODE_ID` 拼法点名模块链；注册表知道身份，而它的 path 就是逻辑目录路径。
    let module_id = snapshots
        .iter()
        .find(|snapshot| crate::build_method::source_module_path(&snapshot.source.file) == key)
        .map(|snapshot| snapshot.id)
        .or_else(|| {
            native
                .iter()
                .find(|face| declared_native::module_of(&face.source) == key)
                .map(|face| native_identity(namespace, face))
        });
    if let Some(id) = module_id
        && let Some(path) = registry.path_for(id)
    {
        return Ok(ParentReference::Catalog {
            id,
            path: path.trim_start_matches("root/").to_owned(),
        });
    }
    let views = crate::build_method::face_views(root, namespace)?;
    let found = views.iter().find(|view| {
        view.module == key
            || view
                .path
                .strip_prefix("root/")
                .is_some_and(|path| path == key)
    });
    let Some(view) = found else {
        return Ok(parent.clone());
    };
    Ok(ParentReference::Catalog {
        id: view.id,
        path: view.path.trim_start_matches("root/").to_owned(),
    })
}

/// Find the parent a request names.
/// 找出请求点名的父。
pub(crate) fn parent_face(
    root: &Path,
    registry: &xirang_kernel::tree::Registry,
    native: &[declared_native::DeclaredFace],
    namespace: &str,
    parent: NodeId,
) -> Result<ParentFace, String> {
    if parent == registry.id() {
        return Ok(ParentFace::Root);
    }
    // A native declaration is checked before the registry: the service registers placeholder
    // snapshots for native faces so a Rust child can be validated, and those placeholders must not
    // turn a C++ parent into a Rust one.
    // 先查本地声明再查注册树：服务为本地登记占位快照好校验 Rust 子面，那些占位不能把 C++ 父变成
    // Rust 父。
    if let Some(face) = native
        .iter()
        .find(|face| native_identity(namespace, face) == parent)
    {
        if !face.owns_registry {
            return Err(format!(
                "parent `{}` does not own a Registry; only a face declared `.owns_registry()` can \
                 have children",
                face.name
            ));
        }
        let id = native_identity(namespace, face);
        // The registry placeholder for a native face was registered with the module-stem default,
        // but the declaration's `.named(…)` is the logical segment. Replace the leaf so the catalog
        // path names the object the author declared, not the source file.
        // 外语面的注册占位用的是模块 stem 默认名，而声明里的 `.named(…)` 才是逻辑段。替换叶子，让目录
        // 路径点名作者声明的对象，而不是源码文件。
        let catalog = registry.path_for(id).map_or_else(
            || format!("root/{}", face.name),
            |path| match path.rsplit_once('/') {
                Some((parent, _)) => format!("{parent}/{}", face.name),
                None => format!("root/{}", face.name),
            },
        );
        return Ok(ParentFace::Native {
            id,
            source: face.source.clone(),
            name: face.name.clone(),
            catalog,
        });
    }
    if let Some(face) = registry
        .depth_first()
        .into_iter()
        .find(|face| face.id == parent)
    {
        if !face.needs_registry {
            return Err(format!(
                "parent `{}` does not own a Registry; only a face with `needs_registry` can have \
                 children",
                face.kind
            ));
        }
        // The module comes from the same view every consumer reads, never from the source path:
        // the generated module chain is the authority, and a face placed outside the discovery root
        // mounts at its stem.
        // 模块来自每个消费者都读的那份视图，绝不从源码路径推导：生成的模块链才是权威，而放在发现根之外
        // 的面挂在它的 stem 上。
        let view = face_views(root, namespace)?
            .into_iter()
            .find(|view| view.id == parent)
            .ok_or_else(|| {
                format!(
                    "parent `{parent}` is in the registry but not in the tree this view derives; the                      registry and the generated module chain disagree"
                )
            })?;
        let module = view.module.clone();
        return Ok(ParentFace::Rust {
            id: face.id,
            source: face.source.file.clone(),
            kind: face.kind.clone(),
            module,
            base: view.source_base,
            catalog: registry
                .path_for(face.id)
                .unwrap_or_else(|| format!("root/{}", face.kind)),
        });
    }
    Err(format!(
        "parent `{parent}` is neither a face this package's registry carries nor a face its native \
         sources declare; a parent declared in another language must be discoverable from this \
         package"
    ))
}

/// A snapshot for one face in the registry, when it is there.
/// 注册树里某个面的快照（当它在时）。
pub(crate) fn snapshot_of(
    faces: &[RegistrationSnapshot],
    id: NodeId,
) -> Option<&RegistrationSnapshot> {
    faces.iter().find(|face| face.id == id)
}

/// The package-relative spelling of a path under the package root.
/// 包根之下路径的包内相对拼写。
pub(crate) fn package_relative(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        format!(
            "{} is outside the package root {}; an authoring plan may only write inside it",
            path.display(),
            root.display()
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

/// The identity source spelling of a Rust file: package-relative, with one leading `src/` dropped
/// when the package's identity base is `src/`.
/// Rust 文件的身份源码拼写：包内相对路径；当包的身份基准是 `src/` 时去掉一个前导 `src/`。
pub(crate) fn rust_identity_source(layout: &SourceLayout, path: &Path) -> Result<String, String> {
    let relative = package_relative(&layout.package_root, path)?;
    // The identity spelling is relative to the **identity base**, which is `src/` for the historical
    // layout and the package root for a custom library target. A face explicitly placed outside that
    // base (an island) has no scan-relative spelling; its record spelling is package-relative, and
    // refusing it here is what made `xirang face edit components/...` report a scan-root error.
    // 身份拼写相对**身份基准**：历史布局是 `src/`，自定义库目标是包根。显式落在基准之外的面（孤岛）没有
    // 扫描根相对拼写，它的记录拼写是包内相对；此前在这里拒绝它，正是 CLI 编辑孤岛报扫描根错误的原因。
    match path.strip_prefix(&layout.identity_base) {
        Ok(identity) => Ok(identity.to_string_lossy().replace('\\', "/")),
        Err(_) => Ok(relative),
    }
}

/// Where a new face goes, as a package-relative path.
/// 新注册面的落点，按包内相对路径。
///
/// The rule is one rule for every language: a child lives in `<parent directory>/object/<module>/`,
/// and the file is named after the module. The extension and the placement come from the language
/// adapter; the parent directory is package-relative for a native parent and scan-relative for a
/// Rust one, which is why the placement is a parameter.
/// 每种语言同一条规则：子面住在 `<父目录>/object/<module>/`，文件以模块命名。扩展名与落点来自语言
/// 适配器；父目录对本地父是包内相对，对 Rust 父是扫描根相对——这正是落点要作为参数的原因。
pub(crate) fn destination(
    layout: &SourceLayout,
    parent: &ParentFace,
    module: &str,
    placement: xirang_multilang::registration::SourcePlacement,
    extension: &str,
) -> Result<PathBuf, String> {
    let under_scan_root =
        placement == xirang_multilang::registration::SourcePlacement::UnderScanRoot;
    let scan_prefix = layout
        .scan_root
        .strip_prefix(&layout.package_root)
        .unwrap_or(Path::new(""));
    // **The parent's directory, in package coordinates.** A Rust face's `source` spelling is written
    // against its own base: an identity-relative face (`panel/panel.rs`) needs the scan prefix added
    // back, while a package-relative one (`components/reading/reading.rs`, an explicit placement and
    // everything that inherits its location) already is package-relative. Re-deriving the base from
    // the spelling is what pulled an island's children under `src/` — a prefix that only applies to
    // the first kind.
    // **父级目录，包坐标。** Rust 面的 `source` 拼写写在自己的基准上：身份相对的面
    // （`panel/panel.rs`）需要把扫描前缀加回去，而包内相对的那一个（`components/reading/reading.rs`，
    // 显式落点以及继承它位置的一切）本来就是包内相对。从拼写重新推导基准，正是把孤岛子面拉进 `src/` 的
    // 原因——那条前缀只适用于第一类。
    let parent_directory: PathBuf = match parent {
        ParentFace::Root => PathBuf::new(),
        ParentFace::Rust { source, base, .. } => {
            let directory = Path::new(source).parent().unwrap_or(Path::new(""));
            match base {
                xirang_multilang::source_ref::SourceBase::Identity => scan_prefix.join(directory),
                xirang_multilang::source_ref::SourceBase::Package => directory.to_path_buf(),
            }
        }
        ParentFace::Native { source, .. } => Path::new(source)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf(),
    };
    // The placement decides whether the child lives beside its parent or under the walk root; the
    // difference now only matters for a parent that is not already under that root.
    // 落点决定子面住在父旁边还是遍历根之下；这个差别如今只对"本身不在遍历根下"的父有意义。
    let _ = under_scan_root;
    Ok(parent_directory
        .join("object")
        .join(module)
        .join(format!("{module}.{extension}")))
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    /// A Rust parent is handed to an adapter as facts: identity plus the build's module path. The
    /// adapter renders its own spelling; the surface never renders another language's expression.
    /// Rust 父以事实交给适配器：身份加构建的模块路径。适配器渲染自己的拼法；执行面从不渲染别的语言的
    /// 表达式。
    #[test]
    fn a_rust_parent_is_handed_over_as_identity_and_module() {
        let parent = ParentFace::Rust {
            id: NodeId::from_raw([0; 16]),
            source: "shop/panel/panel.rs".to_owned(),
            kind: "Panel".to_owned(),
            // The module is a fact the generated chain handed in, not a re-derivation here.
            // 模块是生成链交进来的事实，不是这里重新推导的。
            module: "shop::panel".to_owned(),
            base: xirang_multilang::source_ref::SourceBase::Identity,
            catalog: "shop/panel".to_owned(),
        };
        assert_eq!(parent.module().unwrap(), "shop::panel");
        match parent.request_reference().unwrap() {
            ParentReference::Object { id, module } => {
                assert_eq!(id, NodeId::from_raw([0; 16]));
                assert_eq!(module, "shop::panel");
            }
            other => panic!("expected Object facts, got {other:?}"),
        }
    }

    /// A native parent's module comes from the declaration's own source path, so a child names the
    /// same module the parent's file gets.
    /// 本地父的模块来自声明自己的源码路径，因此子面点名的模块与父文件得到的相同。
    #[test]
    fn a_native_parent_is_handed_over_by_its_declaration_module() {
        let parent = ParentFace::Native {
            id: NodeId::from_raw([1; 16]),
            source: "native/area/area.cpp".to_owned(),
            name: "Area".to_owned(),
            catalog: "shop/panel/Area".to_owned(),
        };
        assert_eq!(parent.module().unwrap(), "native::area");
        assert!(matches!(
            parent.request_reference().unwrap(),
            ParentReference::Object { .. }
        ));
    }

    /// A C++ parent carries one logical path, and the adapter renders the shared expression.
    /// C++ 父携带一条逻辑路径，适配器据此渲染共享表达式。
    #[test]
    fn a_native_parent_carries_its_catalog_path() {
        let parent = ParentFace::Native {
            id: NodeId::from_raw([1; 16]),
            source: "native/area/area.cpp".to_owned(),
            name: "Area".to_owned(),
            catalog: "shop/panel/Area".to_owned(),
        };
        match parent.catalog_reference().unwrap() {
            ParentReference::Catalog { path, .. } => assert_eq!(path, "shop/panel/Area"),
            other => panic!("expected Catalog, got {other:?}"),
        }
    }

    /// The root is a structural fact, not a language error: it is handed over as `Root`, and each
    /// adapter decides whether it can name it.
    /// 根是结构事实，不是语言错误：它以 `Root` 交出，由每个适配器决定自己能否点名它。
    #[test]
    fn the_root_is_handed_over_as_root() {
        assert_eq!(
            ParentFace::Root.request_reference().unwrap(),
            ParentReference::Root
        );
    }

    /// The destination follows the adapter's placement, not a "non-Rust means C++" rule.
    /// 目的地跟随适配器的落点，而不是"非 Rust 就是 C++"。
    #[test]
    fn the_destination_follows_the_adapters_placement() {
        let layout = SourceLayout {
            package_root: PathBuf::from("/pkg"),
            scan_root: PathBuf::from("/pkg/src"),
            identity_base: PathBuf::from("/pkg/src"),
            target: None,
        };
        let parent = ParentFace::Rust {
            id: NodeId::from_raw([0; 16]),
            source: "shop/panel/panel.rs".to_owned(),
            kind: "Panel".to_owned(),
            module: "shop::panel".to_owned(),
            base: xirang_multilang::source_ref::SourceBase::Identity,
            catalog: "shop/panel".to_owned(),
        };
        let beside = destination(
            &layout,
            &parent,
            "gauge",
            xirang_multilang::registration::SourcePlacement::BesideParent,
            "py",
        )
        .unwrap();
        assert_eq!(
            beside,
            PathBuf::from("src/shop/panel/object/gauge/gauge.py")
        );
        let under = destination(
            &layout,
            &parent,
            "gauge",
            xirang_multilang::registration::SourcePlacement::UnderScanRoot,
            "rs",
        )
        .unwrap();
        assert_eq!(under, PathBuf::from("src/shop/panel/object/gauge/gauge.rs"));
    }
}
