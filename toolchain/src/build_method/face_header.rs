//! Toolchain-side mapping for the C++ header generator.
//! C++ 头生成器的工具链侧映射。
//!
//! The spelling and the duplicate refusal live in `xirang_multilang::cpp_header`; this module only
//! maps the build's own records (`StaticFaceRecord`, resolved native faces) into the neutral
//! `NamedFace`, derives the compatibility namespaces from the object ledger, and keeps the historical
//! `face_header::render` path working.
//! 拼法与重复拒绝住在 `xirang_multilang::cpp_header`；本模块只把构建自己的记录
//! （`StaticFaceRecord`、已解析的声明面）映射成中立的 `NamedFace`，从对象台账推导兼容命名空间，并保住
//! 历史路径 `face_header::render`。

use std::path::Path;

use super::static_plan::StaticFaceRecord;

pub use xirang_multilang::cpp_header::NamedFace;

/// The header text for every face this build knows, deterministic in face order.
/// 本次构建已知的全部面对应的头文件文本，按面顺序确定。
pub(crate) fn render(
    faces: &[StaticFaceRecord],
    native: &[NamedFace],
    compatibility: &[(String, [u8; 16])],
    namespace: &str,
) -> Result<String, String> {
    let mut entries: Vec<NamedFace> = faces
        .iter()
        .map(|face| NamedFace {
            module: face.module.clone(),
            kind: face.kind.clone(),
            id: face.id.into_bytes(),
        })
        .collect();
    entries.extend(native.iter().cloned());
    let root = xirang_kernel::identity::root_node_id(namespace).into_bytes();
    xirang_multilang::cpp_header::render_with_compatibility(
        &entries,
        compatibility,
        Some((root, namespace)),
    )
}

/// Map the build's resolved native faces into the header's neutral records.
/// 把构建解析出的外语面映射成头文件的中立记录。
pub(crate) fn named_native(native: &[super::declared_native::Resolved]) -> Vec<NamedFace> {
    native
        .iter()
        .map(|face| NamedFace {
            module: super::declared_native::module_of(&face.source),
            kind: face.name.clone(),
            id: face.id.into_bytes(),
        })
        .collect()
}

/// A C++ identifier for one face: its module path and kind, with every character that is not
/// alphanumeric folded to `_`. This is the version-1 spelling; new references use the nested path.
/// 一个面的 C++ 标识符：模块路径与种类名，非字母数字的字符一律折成 `_`。这是版本 1 的拼法；
/// 新引用使用嵌套路径。
pub(crate) fn identifier(module: &str, kind: &str) -> String {
    xirang_multilang::cpp_names::flat(module, kind)
}

/// The module paths this tree's objects have **left behind**, each pointing at the object's current
/// identity.
/// 这棵树里的对象**离开过**的模块路径，各指向该对象的当前身份。
///
/// The input is the ledger (`xirang.objects.tsv`, written by every create/move) plus what the build
/// derives now: only an entry whose **current** source is one of this build's faces gets an alias, and
/// only its retired sources are turned into module paths. That makes the alias a fact about this
/// tree, not a guess: a ledger entry whose source no longer exists contributes nothing, and a retired
/// module path that a live face now occupies is skipped (the live face is the answer there).
/// 输入是台账（`xirang.objects.tsv`，每次创建/搬动都会写）加上构建此刻推导的东西：只有当某条目的
/// **当前**源码就是本次构建的某个面时它才得到别名，而且只有它退役的源码会变成模块路径。这让别名成为关于
/// 这棵树的事实而不是猜测：源码已不存在的条目什么都不贡献，而被活面占据的退役模块路径会被跳过（那里答案
/// 是活面）。
pub(crate) fn compatibility_namespaces(
    live: &[(String, String, [u8; 16])],
    ledger: &xirang_multilang::object_map::ObjectMap,
) -> Vec<(String, [u8; 16])> {
    use std::collections::BTreeSet;
    let by_source = live
        .iter()
        .map(|(source, module, id)| (source.as_str(), (module.as_str(), *id)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut live_modules: BTreeSet<String> =
        live.iter().map(|(_, module, _)| module.clone()).collect();
    let mut aliases: Vec<(String, [u8; 16])> = Vec::new();
    for entry in ledger.entries() {
        let Some((_, id)) = by_source.get(entry.source.as_str()) else {
            continue;
        };
        for instance in entry.instances.iter().rev().skip(1) {
            if instance.source == entry.source {
                continue;
            }
            let module = super::declared_native::module_of(&instance.source);
            if !live_modules.insert(module.clone()) {
                continue;
            }
            aliases.push((module, *id));
        }
    }
    aliases.sort_by(|left, right| left.0.cmp(&right.0));
    aliases
}

#[cfg(test)]
#[path = "face_header_tests.rs"]
mod face_header_tests;

/// Render the C++ header and append the logical catalog the Rust side also carries.
/// 渲染 C++ 头，并追加 Rust 侧同样携带的逻辑目录。
pub(crate) fn render_with_catalog(
    faces: &[StaticFaceRecord],
    native: &[NamedFace],
    compatibility: &[(String, [u8; 16])],
    namespace: &str,
    catalog: &str,
) -> Result<String, String> {
    let mut header = render(faces, native, compatibility, namespace)?;
    header.push_str(catalog);
    Ok(header)
}

/// Write the generated C++ header and the version-1 compatibility include.
/// 写出生成的 C++ 头与版本 1 兼容 include。
pub(crate) fn write_cpp_files(out_dir: &Path, header: &str) -> Result<(), String> {
    std::fs::create_dir_all(out_dir.join("xirang")).map_err(|error| {
        format!(
            "cannot create {}: {error}",
            out_dir.join("xirang").display()
        )
    })?;
    super::discovery_cache::write_if_changed(&out_dir.join("xirang/faces.hpp"), header)?;
    let compatibility = "// @generated by build.rs; do not edit.\n\
                         // 由 build.rs 生成，请勿手工修改。\n\
                         \n\
                         #pragma once\n\
                         \n\
                         // The version-1 header name; the identities live in xirang/faces.hpp.\n\
                         // 版本 1 的头文件名；身份住在 xirang/faces.hpp。\n\
                         #include \"xirang/faces.hpp\"\n";
    super::discovery_cache::write_if_changed(&out_dir.join("xirang_faces.hpp"), compatibility)
}

/// Write the generated Zig identity module: the root and one constant per module.
/// 写出生成的 Zig 恒等模块：根，以及每个模块一个常量。
#[cfg(feature = "lang-zig")]
pub(crate) fn write_zig_identity(
    out_dir: &Path,
    faces: &[StaticFaceRecord],
    native: &[super::declared_native::Resolved],
    namespace: &str,
) -> Result<(), String> {
    let mut entries: Vec<(String, [u8; 16])> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for face in faces {
        if seen.insert(face.module.clone()) {
            entries.push((face.module.clone(), face.id.into_bytes()));
        }
    }
    for face in native {
        let module = super::declared_native::module_of(&face.source);
        if seen.insert(module.clone()) {
            entries.push((module, face.id.into_bytes()));
        }
    }
    let module = xirang_multilang::zig_native::faces_module(
        xirang_kernel::identity::root_node_id(namespace).into_bytes(),
        &entries,
    );
    super::discovery_cache::write_if_changed(&out_dir.join("xirang_faces.zig"), &module)
}

/// Seed the module constants a Zig declaration resolves its parent through.
/// 预置 Zig 声明用于解析父级的模块常量。
#[cfg(feature = "lang-zig")]
pub(crate) fn fill_static_zig_modules(
    known: &mut std::collections::BTreeMap<String, (String, super::registry_identity::NodeId)>,
    faces: &[StaticFaceRecord],
) {
    for face in faces {
        known
            .entry(xirang_multilang::zig_native::module_flat(&face.module))
            .or_insert_with(|| (face.module.clone(), face.id));
    }
}

/// Seed the module constants of the resolved native faces, for a Zig child under a Zig parent.
/// 预置已解析外语面的模块常量，供 Zig 子面挂在 Zig 父面下。
#[cfg(feature = "lang-zig")]
pub(crate) fn fill_native_zig_modules(
    known: &mut std::collections::BTreeMap<String, (String, super::registry_identity::NodeId)>,
    native: &[super::declared_native::Resolved],
) {
    for face in native {
        let module = super::declared_native::module_of(&face.source);
        known
            .entry(xirang_multilang::zig_native::module_flat(&module))
            .or_insert_with(|| (module, face.id));
    }
}

/// Write every generated header: the C++ header with its catalog and compatibility include, and the
/// Zig identity module when that language is compiled in.
/// 写出全部生成件：带目录与兼容 include 的 C++ 头，以及编译进 Zig 时生成的恒等模块。
///
/// Returns the failures a build script must report, and the one refusal that belongs to the build's
/// diagnostics rather than to writes (a header that cannot be rendered at all).
/// 返回构建脚本必须报告的写失败，以及属于构建诊断而不是写失败的那一个拒绝（整个头都渲染不出来）。
pub(crate) fn write_headers(
    out_dir: &Path,
    faces: &[StaticFaceRecord],
    native: &[super::declared_native::Resolved],
    live: &[super::compatibility::LiveFace],
    package_root: &Path,
    namespace: &str,
    catalog: &str,
) -> (Vec<String>, Option<String>) {
    let mut errors = Vec::new();
    let named = named_native(native);
    let compatibility = super::compatibility::header_namespaces(package_root, live);
    match render_with_catalog(faces, &named, &compatibility, namespace, catalog) {
        Ok(header) => {
            if let Err(error) = write_cpp_files(out_dir, &header) {
                errors.push(error);
            }
        }
        Err(refusal) => return (errors, Some(refusal)),
    }
    #[cfg(feature = "lang-zig")]
    if let Err(error) = write_zig_identity(out_dir, faces, native, namespace) {
        errors.push(error);
    }
    #[cfg(feature = "lang-c")]
    if let Err(error) = write_c_header(out_dir, faces, native, namespace) {
        errors.push(error);
    }
    (errors, None)
}

/// Write the generated C header: the root identity and one module constant per face.
/// 写出生成的 C 头：根身份与每个面一个模块常量。
#[cfg(feature = "lang-c")]
pub(crate) fn write_c_header(
    out_dir: &Path,
    faces: &[StaticFaceRecord],
    native: &[super::declared_native::Resolved],
    namespace: &str,
) -> Result<(), String> {
    let root = xirang_kernel::identity::root_node_id(namespace).into_bytes();
    let mut text = String::from(
        "/* Generated by xirang's build; do not edit. */\n\
         /* 由 xirang 构建生成；请勿手工修改。 */\n\
         #ifndef XIRANG_FACES_H\n\
         #define XIRANG_FACES_H\n\n",
    );
    text.push_str(&format!(
        "/* The registry root, for a declaration that names no parent. */\n\
         /* 注册树根，供不点名父级的声明使用。 */\n\
         #define XIRANG_ROOT_NODE_ID {}\n\n",
        c_id_literal(root)
    ));
    let mut seen = std::collections::BTreeSet::new();
    for (module, id) in faces
        .iter()
        .map(|face| (face.module.clone(), face.id.into_bytes()))
        .chain(native.iter().map(|face| {
            (
                super::declared_native::module_of(&face.source),
                face.id.into_bytes(),
            )
        }))
    {
        if seen.insert(module.clone()) {
            text.push_str(&format!(
                "/* {module} */\n#define XIRANG_FACES_{} {}\n\n",
                flat_module(&module),
                c_id_literal(id)
            ));
        }
    }
    text.push_str("#endif\n");
    std::fs::create_dir_all(out_dir.join("xirang")).map_err(|error| {
        format!(
            "cannot create {}: {error}",
            out_dir.join("xirang").display()
        )
    })?;
    super::discovery_cache::write_if_changed(&out_dir.join("xirang/faces.h"), &text)
}

/// The flat spelling of a module: `::` folded to `__`, with the trailing `_` a flat face
/// identifier carries removed, and a leading `_` when a segment starts with a digit.
/// 模块的扁平拼法：`::` 折成 `__`，去掉扁平面标识符末尾的 `_`；段以数字开头时补前导 `_`。
#[cfg(feature = "lang-c")]
fn flat_module(module: &str) -> String {
    // One implementation, shared with the C authoring adapter: the header's `XIRANG_FACES_<flat>`
    // and the adapter's rendered parent must be the same spelling.
    // 与 C 创作适配器共用的唯一实现：头文件的 `XIRANG_FACES_<flat>` 与适配器渲染的父级必须是同一个拼法。
    xirang_multilang::languages::naming::c_flat_module(module)
}

/// Seed the flat module spelling a C declaration names its parent by (`XIRANG_FACES_PANEL`).
/// 预置 C 声明点名父级所用的扁平模块拼法（`XIRANG_FACES_PANEL`）。
#[cfg(feature = "lang-c")]
pub(crate) fn fill_c_flat_modules(
    known: &mut std::collections::BTreeMap<String, (String, super::registry_identity::NodeId)>,
    faces: &[StaticFaceRecord],
) {
    for face in faces {
        known.insert(
            xirang_multilang::languages::naming::c_flat_module(&face.module),
            (face.module.clone(), face.id),
        );
    }
}

/// One identity as a C initializer list.
/// 一个身份，按 C 初始化列表写。
#[cfg(feature = "lang-c")]
fn c_id_literal(bytes: [u8; 16]) -> String {
    let items: Vec<String> = bytes.iter().map(|byte| format!("0x{byte:02x}")).collect();
    format!("{{ {} }}", items.join(", "))
}
