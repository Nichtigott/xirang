//! Native sources a project declares **once**, compiled and registered by the crate that owns them.
//! 项目**声明一次**的本地语言源码，由拥有它们的那个 crate 编译并登记。
//!
//! A foreign face's owner is decided by the parent it names, not by where its file happens to sit: the
//! logical tree says `Panel` owns `Area`, so the crate that compiles `Panel` compiles `Area` too —
//! including when `Panel` was handed to a generated partition crate. That is the one rule this module
//! exists to implement; the entry points call it for both shapes (host and fragment), which is what
//! keeps a host from compiling a source it handed away and a fragment from registering a face whose
//! parent it does not have.
//! 一个外部面的属主由**它点名的父**决定，而不是由它的文件恰好放在哪里决定：逻辑树说 `Panel` 拥有
//! `Area`，因此编译 `Panel` 的那个 crate 也编译 `Area`——包括 `Panel` 被交给生成的划分 crate 的情形。
//! 这正是本模块存在的唯一规则；两个形状（宿主与碎片）的入口都调用它，宿主因此不会编译它交出去的源码、
//! 碎片也不会登记一个它没有父的面。
//!
//! Measured before this existed: the host compiled the C++ source, registered the face, and the fragment
//! registered nothing, so a host batch carried `Area` without the `Panel` the fragment had taken away and
//! the kernel refused the batch (`parent registry … was not found for `Area``).
//! 在本模块之前实测：宿主编译 C++ 源码并登记那个面，碎片什么都不登记，于是宿主批次带着 `Area`、却没有被
//! 碎片拿走的 `Panel`，内核拒绝该批次（`parent registry … was not found for `Area``）。

use std::path::{Path, PathBuf};

use super::external_face::ExternalSource;

#[path = "declared_native_scan.rs"]
mod scan;

use self::scan::{lookup, names_face, under};

/// The C++ recipe, owned by the language implementation and re-exported for the reader.
/// C++ 配方，由语言实现拥有，读取器这边重导出。
pub(crate) use xirang_multilang::external_faces::Recipe;

/// One marked declaration read out of a native source.
/// 从一份本地源码里读出的一条带标记声明。
#[derive(Clone)]
pub(crate) struct DeclaredFace {
    /// The package-relative spelling of the source, which is part of the face's identity.
    /// 源码在包内的相对拼写——它是面身份的一部分。
    pub(crate) source: String,
    /// The identifier after `xirang::faces::` — the parent the author named.
    /// `xirang::faces::` 之后的标识符——作者点名的父。
    pub(crate) parent: String,
    /// The name the declaration gives the face; it is part of the identity, so this scan has to carry
    /// it: a parent whose identity is only known after compiling cannot be named by a child that has
    /// to compile first.
    /// 声明给这个面的名字；它是身份的一部分，因此这次扫描必须带着它：身份要等编译后才知道的父，没法被
    /// 一个必须先编译的子面点名。
    pub(crate) name: String,
    /// Whether this face keeps a registry of its own — a face can only own children when it does.
    /// 这个面是否自持注册机——只有自持注册机的面才能拥有子面。
    pub(crate) owns_registry: bool,
}

/// A declaration with the identity and the tree position this build derived for it.
/// 一条声明，连同本次构建为它推导出的身份与树位置。
#[derive(Clone, Debug)]
pub(crate) struct Resolved {
    pub(crate) source: String,
    pub(crate) name: String,
    pub(crate) ident: String,
    pub(crate) id: super::registry_identity::NodeId,
    pub(crate) parent: super::registry_identity::NodeId,
    pub(crate) owns_registry: bool,
}

/// Give every declaration its identity and its module path, before anything is compiled.
/// 在编译任何东西之前，给每条声明定出身份与模块路径。
///
/// The module path is what makes a foreign face a **parent** rather than a leaf: a child's module is its
/// parent's module plus the file stem, exactly the way a Rust child's module is its parent's module plus
/// the child's directory. The identity is the same hash the Rust side uses, which is why `.under(...)` in
/// C++ and `crate::<module>::NODE_ID` in Rust name the same value.
/// 模块路径正是让一个外语面成为**父**而不是叶子的东西：子面的模块是父的模块加上文件名，与 Rust 子面的模块是
/// 父的模块加上子目录完全一样。身份用的是 Rust 那一侧同一个散列，因此 C++ 里的 `.under(...)` 与 Rust 里的
/// `crate::<模块>::NODE_ID` 点名的是同一个值。
/// The module a declaration's source lives in, as the generated header spells it: the directories
/// under the package root, then the file stem.
/// 声明所在源码的模块，按生成头里的拼法：包根下的目录段，然后文件 stem。
pub(crate) fn module_of(source: &str) -> String {
    xirang_multilang::cpp_names::module_of(source)
}

/// The constant the generated header gives one declaration.
/// 生成头给一份声明的常量名。
pub(crate) fn ident(source: &str, name: &str) -> String {
    super::face_header::identifier(&module_of(source), name)
}

pub(crate) fn resolve(
    namespace: &str,
    declared: &[DeclaredFace],
    known: &std::collections::BTreeMap<String, (String, super::registry_identity::NodeId)>,
) -> Result<Vec<Resolved>, String> {
    let mut idents = known.clone();
    // The registry root is a parent a declaration may name, and no face reports it: seed the one
    // spelling the adapter emits (`::xirang::faces::ROOT_NODE_ID`) with the same identity
    // `root_node_id(namespace)` returns, so the resolver and the runtime agree.
    // 注册树根是声明可以点名的父级，而没有面报告它：把适配器发射的唯一拼写
    // （`::xirang::faces::ROOT_NODE_ID`）预置为 `root_node_id(namespace)` 的同一个身份，使解析器与
    // 运行期一致。
    idents.insert(
        "ROOT_NODE_ID".to_owned(),
        (
            String::new(),
            xirang_kernel::identity::root_node_id(namespace),
        ),
    );
    let mut resolved: Vec<Option<Resolved>> = vec![None; declared.len()];
    // The logical path each resolved native face publishes in the shared catalog. A child may name
    // its native parent through `crate::catalog::…`, which is a **logical** path rather than the
    // declaration's source module; register it while resolving so a later child can look it up.
    // 每个已解析外语面在共享目录里的逻辑路径。子面可能经 `crate::catalog::…` 点名外语父，那是**逻辑**
    // 路径而非声明的源码模块；在解析时登记它，后解析的子面才能查到。
    let mut catalogs: std::collections::BTreeMap<super::registry_identity::NodeId, String> =
        std::collections::BTreeMap::new();
    let mut progress = true;
    while progress {
        progress = false;
        for (index, face) in declared.iter().enumerate() {
            if resolved[index].is_some() {
                continue;
            }
            let Some((_, parent_id)) = lookup(&idents, &face.parent).cloned() else {
                continue;
            };
            // 标识符由**声明自己住在哪**决定，不由它挂在谁下面决定：一个面的常量名不该因为父面换了
            // 位置或分区而改名。因此这里取源码路径的段，而不是父模块 + 文件 stem。
            // The identifier is decided by **where the declaration lives**, not by what it hangs
            // under: a face's constant must not change name when its parent moves or a crate takes
            // it over. So the segments come from the source path, not from the parent's module plus
            // the file stem.
            let module = module_of(&face.source);
            let ident = ident(&face.source, &face.name);
            let id = super::registry_identity::NodeId::from_namespaced_path(
                namespace,
                &face.source,
                &face.name,
            );
            idents.insert(ident.clone(), (module.clone(), id));
            let parent_key = face
                .parent
                .trim_start_matches("::")
                .strip_suffix("::NODE_ID")
                .unwrap_or_else(|| face.parent.trim_start_matches("::"));
            let parent_catalog = catalogs
                .get(&parent_id)
                .cloned()
                .unwrap_or_else(|| parent_key.replace("::", "/"));
            let catalog = if parent_catalog.is_empty() {
                face.name.clone()
            } else {
                format!("{parent_catalog}/{}", face.name)
            };
            catalogs.insert(id, catalog.clone());
            // A child written with the new shared spelling names this path, not the source module.
            // 用新共享拼写写下的子面点名的是这条路径，而不是源码模块。
            idents.insert(catalog.replace('/', "::"), (catalog.replace('/', "::"), id));
            resolved[index] = Some(Resolved {
                source: face.source.clone(),
                name: face.name.clone(),
                ident,
                id,
                parent: parent_id,
                owns_registry: face.owns_registry,
            });
            progress = true;
        }
    }
    let missing: Vec<String> = declared
        .iter()
        .zip(resolved.iter())
        .filter(|(_, resolved)| resolved.is_none())
        .map(|(face, _)| format!("{} (parent `{}`)", face.source, face.parent))
        .collect();
    if !missing.is_empty() {
        if std::env::var_os("XIRANG_DEBUG_RESOLVE").is_some() {
            eprintln!("RESOLVE FAIL missing={missing:?} idents={idents:?}");
        }
        // Both spellings are listed: a reader that wrote the nested path should be able to see the
        // nested spelling of what it could have named.
        // 两种拼法都列出：写嵌套路径的读者应当能看到它可以点名的那些嵌套拼法。
        let mut available: Vec<String> = idents
            .keys()
            .cloned()
            .chain(
                idents
                    .values()
                    .map(|(module, _)| format!("{module}::NODE_ID")),
            )
            .collect();
        available.sort();
        available.dedup();
        return Err(format!(
            "these native declarations name a parent this build cannot resolve: {}; the faces that can \
             be named are {}",
            missing.join(", "),
            available
                .iter()
                .map(|ident| format!("`{ident}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(resolved.into_iter().flatten().collect())
}

/// Read the native sources of a package, then adapt the neutral facts into this build's records.
/// 读出一个包的本地源码，再把中立的声明事实适配成本构建的记录。
///
/// Discovery, recipe defaults and source validation live in `xirang_multilang::cpp_native` with the
/// rest of the C++ implementation; this wrapper only maps the result onto [`DeclaredFace`].
/// 发现、配方默认值与源码校验住在 `xirang_multilang::cpp_native`，与其余 C++ 实现一起；本包装只把结果
/// 映射成 [`DeclaredFace`]。
/// Which native language this package builds.
/// 本包构建的是哪门本地语言。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeLanguage {
    Cpp,
    C,
    Zig,
}

/// The native language a package declares, or the one its markers name.
/// 一个包声明的本地语言，或它的标记所点名的那一门。
///
/// `[native] language` wins; otherwise the sources' markers decide. A package that declares both is
/// refused by name rather than picking one: one `[native]` recipe compiles one language.
/// `[native] language` 优先；否则由源码里的标记决定。两者都声明的包按名拒绝而不是替它挑一个：一份
/// `[native]` 配方只编译一门语言。
fn declared_language(package_root: &Path) -> Result<Option<NativeLanguage>, String> {
    let manifest = package_root.join("xirang.toml");
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    if let Some(section) = text.split("[native]").nth(1) {
        for line in section.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some(rest) = line.strip_prefix("language") else {
                continue;
            };
            let Some((_, value)) = rest.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            return match value {
                "cpp" | "c++" => Ok(Some(NativeLanguage::Cpp)),
                "c" => Ok(Some(NativeLanguage::C)),
                "zig" => Ok(Some(NativeLanguage::Zig)),
                other => Err(format!(
                    "`[native] language = \"{other}\"` is not a local language this build knows; \
                     use `cpp` or `zig`"
                )),
            };
        }
    }
    let (mut cpp, mut zig) = (false, false);
    let requests = text.contains("[native]");
    let mut pending = vec![package_root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skip = path
                    .file_name()
                    .is_some_and(|name| name == "target" || name == ".git" || name == "crates");
                if !skip {
                    pending.push(path);
                }
                continue;
            }
            // A link is not a marker: following it would read a file outside the tree, and the
            // source-scope walk is the place that refuses one — this detection must not do its job.
            // 链接不是标记：跟着它读会把树外文件读进来，而拒绝它的地方是来源范围遍历——本检测不做那件事。
            if std::fs::symlink_metadata(&path)
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(false)
            {
                continue;
            }
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            match extension {
                "zig" if text.contains("xirang.record(") => zig = true,
                "cpp" | "cc" | "cxx" | "c++" | "c"
                    if text.contains("XIRANG_SOIL") || text.contains("XIRANG_FACE(") =>
                {
                    cpp = true
                }
                _ => {}
            }
        }
    }
    match (cpp, zig) {
        (true, true) => Err(
            "this package declares both C++ and Zig native sources; one `[native]` \
             recipe compiles one language — write `[native] language = ...` or split the package"
                .to_owned(),
        ),
        (true, false) => Ok(Some(NativeLanguage::Cpp)),
        (false, true) => Ok(Some(NativeLanguage::Zig)),
        (false, false) if requests => Ok(Some(default_native_language())),
        (false, false) => Ok(None),
    }
}

#[cfg(feature = "lang-cpp")]
fn default_native_language() -> NativeLanguage {
    NativeLanguage::Cpp
}

#[cfg(not(feature = "lang-cpp"))]
fn default_native_language() -> NativeLanguage {
    NativeLanguage::Zig
}

/// Read the package's native recipe and declarations, whichever language it builds.
/// 读出包的原生配方与声明，无论它构建哪门语言。
pub(crate) fn read(package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    match declared_language(package_root)? {
        Some(NativeLanguage::Cpp) => read_cpp(package_root),
        Some(NativeLanguage::C) => read_c(package_root),
        Some(NativeLanguage::Zig) => read_zig(package_root),
        // No `[native]` table: the default language's discovery still runs, because its walk is
        // what refuses a link out of the tree. A build with no language compiled has no such walk
        // and no native sources to find.
        // 没有 `[native]` 表：默认语言的发现仍要跑——拒绝树外链接的正是它那次遍历。没有编译任何语言的
        // 构建既没有那次遍历，也没有本地源码可找。
        None => read_default(package_root),
    }
}

/// The C++ reader, owned by the language implementation and re-exported for the build.
/// C++ 读取器，由语言实现拥有，构建这边重导出。
#[cfg(feature = "lang-cpp")]
fn read_cpp(package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    let Some((recipe, faces)) = xirang_multilang::cpp_native::read_recipe(package_root)? else {
        return Ok(None);
    };
    Ok(Some((
        recipe,
        faces
            .into_iter()
            .map(|face| DeclaredFace {
                source: face.source,
                parent: face.parent,
                name: face.name,
                owns_registry: face.owns_registry,
            })
            .collect(),
    )))
}

#[cfg(not(feature = "lang-cpp"))]
fn read_cpp(_package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    Err(
        "this build does not carry the C++ adapter; enable `xirang-toolchain` features = [\
         \"lang-cpp\"] (or build the CLI with `--features cli,lang-cpp`)"
            .to_owned(),
    )
}

/// The Zig reader, owned by the language implementation.
/// Zig 读取器，由语言实现拥有。
#[cfg(feature = "lang-zig")]
fn read_zig(package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    let Some((recipe, faces)) = xirang_multilang::zig_native::read_recipe(package_root)? else {
        return Ok(None);
    };
    Ok(Some((
        recipe,
        faces
            .into_iter()
            .map(|face| DeclaredFace {
                source: face.source,
                parent: face.parent,
                name: face.name,
                owns_registry: face.owns_registry,
            })
            .collect(),
    )))
}

#[cfg(not(feature = "lang-zig"))]
fn read_zig(_package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    Err(
        "this package declares Zig native sources, but this build does not carry the Zig adapter; \
         enable it with `xirang-toolchain` features = [\"lang-zig\"] (or build the CLI with \
         `--features cli,lang-zig`)"
            .to_owned(),
    )
}

/// The faces this package does **not** compile, each named with the crate that does.
/// 本包**不**编译的那些面，每个都点名编译它的 crate。
///
/// The crate is named by the claim the parent falls under: `Crate::named("panel")` for the subtree
/// `shop::panel` means the package `shop-panel`, which is the spelling the generator writes next door.
/// crate 名取自父所在的那条认领：子树 `shop::panel` 的 `Crate::named("panel")` 就是 `shop-panel`，
/// 也就是生成器在旁边写下的那个名字。
pub(crate) fn elsewhere(
    package_root: &Path,
    declared: &[DeclaredFace],
    owned: &[DeclaredFace],
) -> Vec<String> {
    let declaration = super::shape_decl::read_shape_declaration(package_root)
        .ok()
        .flatten();
    let mut rows = Vec::new();
    for face in declared {
        if owned
            .iter()
            .any(|mine| mine.source == face.source && mine.parent == face.parent)
        {
            continue;
        }
        let claim = declaration
            .as_ref()
            .and_then(|declaration| {
                declaration
                    .crates
                    .iter()
                    .find(|(_, subtrees)| subtrees.iter().any(|subtree| under(face, subtree)))
            })
            .map(|(name, subtrees)| {
                let prefix = declaration
                    .as_ref()
                    .map(|declaration| declaration.package_prefix.as_str())
                    .unwrap_or("");
                format!(
                    "crate `{prefix}-{name}` (claim `{}`)",
                    subtrees.join("`, `")
                )
            })
            .unwrap_or_else(|| "another crate".to_owned());
        rows.push(format!("{} → {claim}", face.source));
    }
    rows.sort();
    rows.dedup();
    rows
}

/// The declarations **this crate** compiles: the ones whose parent it owns.
/// **本 crate** 要编译的那些声明：父归它的那些。
///
/// One rule, two shapes. A fragment took `only` (a comma-joined list of subtrees), so it compiles the
/// declarations whose parent lives inside one of them. A host handed those same subtrees away, so it
/// compiles exactly the declarations whose parent lives inside none of them. A facade compiles no
/// declared source: it registers the union of the **Rust** faces, and a second registrar for one foreign
/// identity is a duplicate the kernel refuses by name.
/// 一条规则、两种形状。碎片拿走了 `only`（逗号分隔的子树表），因此它编译父住在其中一棵里的声明；宿主
/// 把这些子树交了出去，因此它编译的正是父不落在任何一棵里的声明。facade 不编译任何声明源码：它登记的是
/// **Rust** 面的并集，而同一个外部身份有第二个登记者会被内核点名拒绝。
pub(crate) fn owned(
    declared: Vec<DeclaredFace>,
    only: Option<&str>,
    facade: bool,
    cuts: &[String],
) -> Vec<DeclaredFace> {
    if facade {
        return Vec::new();
    }
    // 归属跟着**父链**走，不跟着"父的拼写"走：一个 C++ 面的父可以还是 C++（`Gauge` 挂在 `Area` 下），
    // 而 `Area` 的父才是 Rust 面。只看直接父的拼写会把 `Gauge` 判给宿主——它的父 `native__area_Area`
    // 里没有认领前缀——于是宿主去编译一个属于碎片的面，碎片反而编译不到它。
    // Ownership follows the **parent chain**, not the spelling of the immediate parent: a C++ face's
    // parent can be another C++ face (`Gauge` under `Area`), and only `Area`'s parent is the Rust face.
    // Judging by the immediate parent's spelling handed `Gauge` to the host — its parent
    // `native__area_Area` carries no claim prefix — so the host compiled a face the fragment owns and
    // the fragment never compiled it.
    let mut terminal: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for face in &declared {
        let mut parent = face.parent.clone();
        let mut seen = std::collections::BTreeSet::new();
        while let Some(next) = declared.iter().find(|other| names_face(other, &parent)) {
            if !seen.insert(next.parent.clone()) {
                break;
            }
            parent = next.parent.clone();
        }
        terminal.insert(ident(&face.source, &face.name), parent);
    }
    // 两种父拼法都要认：版本 1 的扁平标识符，与版本 2 的 `module::NODE_ID`。按扁平前缀去判一条嵌套
    // 父，会把碎片自己该编的面判给宿主。
    // Both parent spellings are recognised: the version-1 flat identifier and the version-2
    // `module::NODE_ID`. Judging a nested parent by a flat prefix handed the fragment's faces to the
    // host.
    let covered = |parent: &str, claims: &[String]| -> bool {
        match parent.strip_suffix("::NODE_ID") {
            Some(module) => claims
                .iter()
                .any(|claim| module == claim || module.starts_with(&format!("{claim}::"))),
            // A Zig declaration names its parent by the **module constant** (`shop__panel`), which
            // is exactly the claim's flat spelling — not a prefix of it. Both shapes count: the flat
            // face identifier (`shop__panel_Panel`) and the module constant (`shop__panel`).
            // Zig 声明按**模块常量**（`shop__panel`）点名父级，那正是认领的扁平拼法本身，而不是它的前缀。
            // 两种形状都算：扁平的面标识符（`shop__panel_Panel`）与模块常量（`shop__panel`）。
            None => claims.iter().any(|claim| {
                let flat = claim.replace("::", "__");
                parent == flat || parent.starts_with(&format!("{flat}_"))
            }),
        }
    };
    match only {
        Some(only) => {
            let taken: Vec<String> = only
                .split(',')
                .map(str::trim)
                .filter(|claim| !claim.is_empty())
                .map(str::to_owned)
                .collect();
            declared
                .into_iter()
                .filter(|face| {
                    terminal
                        .get(&ident(&face.source, &face.name))
                        .is_some_and(|parent| covered(parent, &taken))
                })
                .collect()
        }
        None => declared
            .into_iter()
            .filter(|face| {
                terminal
                    .get(&ident(&face.source, &face.name))
                    .is_some_and(|parent| !covered(parent, cuts))
                // A face whose terminal parent is one of its own declarations is a cycle the
                // resolver refuses; keeping it here is harmless because the resolver runs first.
                // 终点父仍指向自己的声明时那是解析器会拒绝的环；留在这里无害，因为解析器先跑。
            })
            .collect(),
    }
}

/// Compile one crate's share of the declared sources, returning one object per compiled source.
/// 编译本 crate 那一份声明的源码，每个编译过的源文件交回一个目标文件。
///
/// The execution lives in `xirang_multilang::cpp_native`; this wrapper only adapts the build's face
/// records to the neutral source spellings.
/// 执行住在 `xirang_multilang::cpp_native`；本包装只把构建的面记录适配成中立的源码拼写。
/// Compile one crate's share of the declared sources, returning one object per compiled source.
/// 编译本 crate 那一份声明的源码，每个编译过的源文件交回一个目标文件。
///
/// The execution lives in the language modules; this wrapper only adapts the build's face records to
/// the neutral source spellings and picks the language the package declares.
/// 执行住在各语言模块里；本包装只把构建的面记录适配成中立的源码拼写，并按包声明的语言分派。
pub(crate) fn compile(
    package_root: &Path,
    out_dir: &Path,
    recipe: &Recipe,
    declared: &[DeclaredFace],
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    let sources: Vec<&str> = declared.iter().map(|face| face.source.as_str()).collect();
    match declared_language(package_root)? {
        None => Ok(Vec::new()),
        Some(NativeLanguage::Cpp) => compile_cpp(
            package_root,
            out_dir,
            recipe,
            &sources,
            emit_cargo_directives,
        ),
        Some(NativeLanguage::C) => compile_c(
            package_root,
            out_dir,
            recipe,
            &sources,
            emit_cargo_directives,
        ),
        Some(NativeLanguage::Zig) => compile_zig(
            package_root,
            out_dir,
            recipe,
            &sources,
            emit_cargo_directives,
        ),
    }
}

#[cfg(feature = "lang-cpp")]
fn compile_cpp(
    package_root: &Path,
    out_dir: &Path,
    recipe: &Recipe,
    sources: &[&str],
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    xirang_multilang::cpp_native::compile(
        package_root,
        out_dir,
        recipe,
        sources,
        emit_cargo_directives,
    )
}

#[cfg(not(feature = "lang-cpp"))]
fn compile_cpp(
    _package_root: &Path,
    _out_dir: &Path,
    _recipe: &Recipe,
    _sources: &[&str],
    _emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    Err("this build does not carry the C++ compiler adapter; enable `lang-cpp`".to_owned())
}

#[cfg(feature = "lang-zig")]
fn compile_zig(
    package_root: &Path,
    out_dir: &Path,
    recipe: &Recipe,
    sources: &[&str],
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    xirang_multilang::zig_native::compile(
        package_root,
        out_dir,
        recipe,
        sources,
        emit_cargo_directives,
    )
}

#[cfg(not(feature = "lang-zig"))]
fn compile_zig(
    _package_root: &Path,
    _out_dir: &Path,
    _recipe: &Recipe,
    _sources: &[&str],
    _emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    Err("this build does not carry the Zig compiler adapter; enable `lang-zig`".to_owned())
}

/// Pair each compiled object with the package-relative spelling of its source.
/// 把每个编译出的目标文件与它源码在包内的相对拼写配成一对。
pub(crate) fn sources<'a>(
    objects: &'a [PathBuf],
    declared: &'a [DeclaredFace],
) -> Vec<ExternalSource<'a>> {
    objects
        .iter()
        .zip(declared.iter())
        .map(|(object, face)| ExternalSource {
            object: object.as_path(),
            source: face.source.as_str(),
        })
        .collect()
}

#[cfg(all(test, feature = "lang-cpp"))]
#[path = "declared_native_tests.rs"]
mod declared_native_tests;

#[cfg(all(test, not(feature = "lang-cpp")))]
#[path = "declared_native_no_cpp_tests.rs"]
mod declared_native_no_cpp_tests;

#[cfg(all(test, feature = "lang-zig"))]
#[path = "declared_native_zig_tests.rs"]
mod declared_native_zig_tests;

/// Read with the default language when the package writes no `[native]` table.
/// 包没有写 `[native]` 表时，用默认语言读取。
#[cfg(feature = "lang-cpp")]
fn read_default(package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    read_cpp(package_root)
}

#[cfg(all(not(feature = "lang-cpp"), feature = "lang-zig"))]
fn read_default(package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    read_zig(package_root)
}

#[cfg(not(any(feature = "lang-cpp", feature = "lang-zig")))]
fn read_default(_package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    Ok(None)
}

/// The capability gap a package's declared language has in this build, if any.
/// 包的声明语言在当前构建里的能力缺口（若有）。
///
/// Returns the language's name and the feature that turns its adapter on. The caller adds the count;
/// this function only decides which gap exists.
/// 返回语言名与打开其适配器的特性。数量由调用方补；本函数只判定缺口是哪一条。
pub(crate) fn adapter_gap(
    package_root: &Path,
) -> Result<Option<(&'static str, &'static str)>, String> {
    match declared_language(package_root)? {
        Some(NativeLanguage::Cpp) if !cfg!(feature = "lang-cpp") => Ok(Some(("C++", "lang-cpp"))),
        Some(NativeLanguage::Zig) if !cfg!(feature = "lang-zig") => Ok(Some(("Zig", "lang-zig"))),
        _ => Ok(None),
    }
}

/// The C reader, owned by the language implementation.
/// C 读取器，由语言实现拥有。
#[cfg(feature = "lang-c")]
fn read_c(package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    let Some((recipe, faces)) = xirang_multilang::c_native::read_recipe(package_root)? else {
        return Ok(None);
    };
    Ok(Some((
        recipe,
        faces
            .into_iter()
            .map(|face| DeclaredFace {
                source: face.source,
                parent: face.parent,
                name: face.name,
                owns_registry: face.owns_registry,
            })
            .collect(),
    )))
}

#[cfg(not(feature = "lang-c"))]
fn read_c(_package_root: &Path) -> Result<Option<(Recipe, Vec<DeclaredFace>)>, String> {
    Err(
        "this package declares C native sources, but this build does not carry the C adapter; \
         enable it with `xirang-toolchain` features = [\"lang-c\"]"
            .to_owned(),
    )
}

#[cfg(feature = "lang-c")]
fn compile_c(
    package_root: &Path,
    out_dir: &Path,
    recipe: &Recipe,
    sources: &[&str],
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    xirang_multilang::c_native::compile(
        package_root,
        out_dir,
        recipe,
        sources,
        emit_cargo_directives,
    )
}

#[cfg(not(feature = "lang-c"))]
fn compile_c(
    _package_root: &Path,
    _out_dir: &Path,
    _recipe: &Recipe,
    _sources: &[&str],
    _emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    Err("this build does not carry the C compiler adapter; enable `lang-c`".to_owned())
}
