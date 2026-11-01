//! The C++ header that names every face by identity — the C++ language implementation's generator.
//! 用身份为每个面命名的 C++ 头文件——C++ 语言实现的生成器。
//!
//! The build computes every identity, so the header is a build product rather than a hand-kept
//! file. The caller maps its own records into [`NamedFace`]; this module owns the spelling (nested
//! `NODE_ID` plus the version-1 flat alias) and the one refusal that belongs to the format: two
//! faces in one module would share one nested path.
//! 构建算得出每个身份，因此头部是构建产物而不是手工维护的文件。调用方把自己的记录映射成
//! [`NamedFace`]；本模块拥有拼法（嵌套 `NODE_ID` 加版本 1 扁平别名）以及格式本身的一条拒绝：同一模块
//! 的两个面会共用一个嵌套路径。

/// One face the header names: where it lives in the logical tree, its kind, and its identity.
/// 头部点名的一个面：它在逻辑树里的位置、种类名与身份。
#[derive(Clone)]
pub struct NamedFace {
    /// The module path, as the tree spells it (`shop::panel`).
    /// 模块路径，按树的拼法（`shop::panel`）。
    pub module: String,
    /// The face kind (`Panel`).
    /// 面种类名（`Panel`）。
    pub kind: String,
    /// The identity bytes.
    /// 身份字节。
    pub id: [u8; 16],
}

/// The header text for every face this build knows, deterministic in face order.
/// 本次构建已知的全部面对应的头文件文本，按面顺序确定。
///
/// The caller supplies both the Rust faces and the faces declared in a foreign language: their
/// identity is the same hash, and a declaration that hangs a face under one of them needs the same
/// constant. The foreign faces are computed from the declaration text (source spelling + declared
/// name), which is why they can exist before the compiler runs.
/// 调用方同时提供 Rust 面与在别的语言里声明的面：它们的身份是同一个散列，而把面挂到它们下面的声明需要
/// 同一个常量。外语面由声明文本（源码拼写 + 声明的名字）算出，因此能在编译器运行之前就存在。
///
/// Two faces in one module have no distinct nested path, so the header refuses them instead of
/// emitting a duplicate `NODE_ID`: a C++ declaration that named it could not say which face it meant.
/// 同一个模块里的两个面没有彼此不同的嵌套路径，因此头部**拒绝**它们，而不是发射重复的 `NODE_ID`：
/// 点名它的 C++ 声明说不出自己指的是哪一个。
/// Compatibility alias for the published `cpp_header::render` spelling.
/// 已发布拼法 `cpp_header::render` 的兼容别名。
#[deprecated(note = "use cpp_header::render_header; this alias keeps 0.2 callers compiling")]
pub fn render(entries: &[NamedFace]) -> Result<String, String> {
    render_header(entries)
}

/// Renders the C++ header for the given faces.
/// 为给定的面发射 C++ 头。
pub fn render_header(entries: &[NamedFace]) -> Result<String, String> {
    render_with_compatibility(entries, &[], None)
}

/// The one spelling a C++ declaration uses to hang under the registry root.
/// C++ 声明挂在注册树根下时使用的唯一拼法。
///
/// The root identity is not a C++ invention: it is the value the build computes for this package's
/// namespace (the same one Rust's `root_node_id(namespace)` returns), emitted as a constant because
/// C++ cannot recompute the hash. Emitting it in the shared header is what lets a C++ face mount at
/// the root without a placeholder Rust parent.
/// 根身份不是 C++ 新造的东西：它是构建为这个包命名空间算出的值（与 Rust 的
/// `root_node_id(namespace)` 同一个），因为 C++ 无法重算散列而发射成常量。把它发在共享头里，正是
/// 让 C++ 面无需占位 Rust 父面就能挂在根上的原因。
pub const ROOT_NODE_ID: &str = "::xirang::faces::ROOT_NODE_ID";

/// The header text plus one namespace per **module path a face left behind**.
/// 头文件文本，外加每个"面离开过的模块路径"一节命名空间。
///
/// The second list is what keeps a third party's declaration valid across a physical move: a C++
/// child names its parent through `::xirang::faces::<module>::NODE_ID`, and a move that changes the
/// parent's module path would otherwise force that child's source to be edited — a write into a
/// declared (read-only) source, or a hand edit in somebody else's tree. The alias makes the retired
/// path resolve to the object's **current** identity instead.
/// 第二份清单正是让第三方声明在物理搬动之后仍然有效的东西：C++ 子面经
/// `::xirang::faces::<模块>::NODE_ID` 点名父级，而一次改变父模块路径的搬动，否则就会逼着子面源码被改
/// ——要么写进一个已声明（只读）来源，要么去别人的树里手工改。别名让那个退役路径改为解析到对象的**当前**身份。
///
/// An alias is emitted only when no live face occupies that module path: a live face's own namespace
/// is the answer there, and two `NODE_ID`s in one namespace would be ambiguous.
/// 只有当没有活着的面占据那个模块路径时才发射别名：那里答案是活面自己的命名空间，而一个命名空间里两个
/// `NODE_ID` 会有歧义。
pub fn render_with_compatibility(
    entries: &[NamedFace],
    compatibility: &[(String, [u8; 16])],
    root: Option<([u8; 16], &str)>,
) -> Result<String, String> {
    let mut entries: Vec<NamedFace> = entries.to_vec();
    // Sorting makes the file stable under discovery order, which is what lets a build skip
    // rewriting it (`write_if_changed`) and what keeps a committed copy diffable.
    // 排序让文件在发现顺序变化时保持不变，这正是构建可以跳过重写（`write_if_changed`）、
    // 也是入库副本还能逐行比对的原因。
    entries.sort_by(|left, right| {
        (
            crate::languages::cpp::names::nested(&left.module),
            crate::languages::cpp::names::flat(&left.module, &left.kind),
        )
            .cmp(&(
                crate::languages::cpp::names::nested(&right.module),
                crate::languages::cpp::names::flat(&right.module, &right.kind),
            ))
    });
    entries.dedup_by(|left, right| {
        crate::languages::cpp::names::flat(&left.module, &left.kind)
            == crate::languages::cpp::names::flat(&right.module, &right.kind)
    });
    // One identity per nested path. Two kinds in one module would both want `NODE_ID`, and a
    // declaration that named it could not say which one it meant.
    // 每个嵌套路径只有一个身份。同一模块里的两个种类都会想要 `NODE_ID`，而点名它的声明说不出自己指的是哪一个。
    for pair in entries.windows(2) {
        if pair[0].module == pair[1].module {
            return Err(format!(
                "two faces share the module `{}` (`{}` and `{}`); the generated C++ header can name \
                 one `xirang::faces::{}::NODE_ID`, so a declaration under it could not say which \
                 face it meant. Give each face its own source file, or move one to another module",
                pair[0].module,
                pair[0].kind,
                pair[1].kind,
                crate::languages::cpp::names::nested(&pair[0].module)
            ));
        }
    }
    let mut text = String::from(
        "// @generated by build.rs; do not edit.\n\
         // 由 build.rs 生成，请勿手工修改。\n\
         \n\
         #pragma once\n\
         \n\
         #include \"xirang/soil.hpp\"\n\
         \n\
         namespace xirang::faces {\n\
         \n\
         // Version-1 spellings, kept so existing declarations keep compiling.\n\
         // 版本 1 拼法，保留使既有声明继续编译。\n",
    );
    for entry in &entries {
        text.push_str(&format!(
            "inline constexpr ::xirang::Id {} = ::xirang::Id{{{{{}}}}};\n",
            crate::languages::cpp::names::flat(&entry.module, &entry.kind),
            bytes(entry.id)
        ));
    }
    if let Some((id, namespace)) = root {
        text.push_str(&format!(
            "\n// The registry root of `{namespace}`: the same identity Rust's `root_node_id` \
             computes.\n// 包 `{namespace}` 的注册树根：与 Rust `root_node_id` 算出的同一个身份。\n\
             inline constexpr ::xirang::Id ROOT_NODE_ID = ::xirang::Id{{{{{}}}}};\n",
            bytes(id)
        ));
    }
    text.push_str(
        "\n// The author-facing spelling: one nested path per face, mirroring the logical tree.\n\
         // 面向作者的拼法：每个面一条嵌套路径，镜像逻辑树。\n",
    );
    for entry in &entries {
        text.push_str(&format!(
            "namespace {} {{\ninline constexpr ::xirang::Id NODE_ID = ::xirang::Id{{{{{}}}}};\n}}  // namespace {}\n",
            crate::languages::cpp::names::nested(&entry.module),
            bytes(entry.id),
            crate::languages::cpp::names::nested(&entry.module)
        ));
    }
    let live = entries
        .iter()
        .map(|entry| entry.module.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = live.clone();
    let mut aliases = compatibility
        .iter()
        .filter(|(module, _)| seen.insert(module.clone()))
        .collect::<Vec<_>>();
    aliases.sort_by(|left, right| left.0.cmp(&right.0));
    if !aliases.is_empty() {
        text.push_str(
            "\n// Compatibility spellings: module paths a face left behind, so a declaration written\
             \n// against one still names the same object after the move.\n\
             // 兼容拼法：面离开过的模块路径，使按它写下的声明在搬动之后仍点名同一个对象。\n",
        );
        for (module, id) in aliases {
            let nested = crate::languages::cpp::names::nested(module);
            text.push_str(&format!(
                "namespace {nested} {{\ninline constexpr ::xirang::Id NODE_ID = ::xirang::Id{{{{{}}}}};\n}}  // namespace {nested}\n",
                bytes(*id)
            ));
        }
    }
    text.push_str("\n}  // namespace xirang::faces\n");
    Ok(text)
}

/// The shared object catalog in C++: the same `crate::catalog::…::NODE_ID` spelling Rust emits.
/// C++ 里的共享对象目录：与 Rust 发射的 `crate::catalog::…::NODE_ID` 完全同一拼法。
///
/// The paths come from object relations (parent/name), not source paths; a physical move or crate
/// partition therefore does not change a public reference. The namespace is `crate::catalog` so the
/// author writes the same expression in both languages.
/// 路径来自对象关系（父级/名字），不是源码路径；因此物理搬动或 crate 分区不改变公开引用。命名空间是
/// `crate::catalog`，作者在两门语言里写同一个表达式。
pub fn render_catalog(entries: &[(String, [u8; 16])]) -> Result<String, String> {
    #[derive(Default)]
    struct Branch {
        id: Option<[u8; 16]>,
        children: std::collections::BTreeMap<String, Branch>,
    }
    let mut root = Branch::default();
    for (path, id) in entries {
        let mut branch = &mut root;
        for segment in crate::languages::cpp::names::catalog_path(path).split("::") {
            branch = branch.children.entry(segment.to_owned()).or_default();
        }
        if let Some(existing) = branch.id
            && existing != *id
        {
            return Err(format!(
                "two logical objects share the catalog path `{path}`; C++ cannot name both"
            ));
        }
        branch.id = Some(*id);
    }
    fn write_branch(branch: &Branch, text: &mut String, depth: usize) {
        let indent = "    ".repeat(depth);
        if let Some(id) = branch.id {
            text.push_str(&format!(
                "{indent}inline constexpr ::xirang::Id NODE_ID = ::xirang::Id{{{{{}}}}};
",
                bytes(id)
            ));
        }
        for (name, child) in &branch.children {
            text.push_str(&format!(
                "{indent}namespace {name} {{
"
            ));
            write_branch(child, text, depth + 1);
            text.push_str(&format!(
                "{indent}}}  // namespace {name}
"
            ));
        }
    }
    let mut text = String::from(
        "\n// The shared object catalog: the same `crate::catalog::…::NODE_ID` spelling as Rust.
\
         // 共享对象目录：与 Rust 完全同一的 `crate::catalog::…::NODE_ID` 拼法。
\
         namespace crate::catalog {
",
    );
    write_branch(&root, &mut text, 0);
    text.push_str(
        "}  // namespace crate::catalog
",
    );
    Ok(text)
}

/// A byte array as a C++ initializer list body.
/// 把字节数组写成 C++ 初始化列表的正文。
fn bytes(id: [u8; 16]) -> String {
    id.iter()
        .map(|byte| byte.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The root is emitted as a constant with the identity the caller computed, so a C++
    /// declaration can name it without a hand-written byte array.
    /// 根以调用方算出的身份发射成常量，因此 C++ 声明无需手写字节数组就能点名它。
    #[test]
    fn the_root_identity_is_emitted_as_a_constant() {
        let header = render_with_compatibility(
            &[NamedFace {
                module: "shop".to_owned(),
                kind: "Shop".to_owned(),
                id: [1; 16],
            }],
            &[],
            Some(([9; 16], "example")),
        )
        .expect("header");
        assert!(
            header.contains("inline constexpr ::xirang::Id ROOT_NODE_ID = ::xirang::Id{{9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9}};"),
            "{header}"
        );
        assert!(header.contains("registry root of `example`"), "{header}");
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn the_shared_catalog_uses_the_same_paths_as_rust() {
        let header = render_catalog(&[
            ("shop".to_owned(), [1; 16]),
            ("shop/panel".to_owned(), [2; 16]),
            ("shop/panel/Area".to_owned(), [3; 16]),
        ])
        .expect("catalog");
        assert!(header.contains("namespace crate::catalog {"), "{header}");
        assert!(header.contains("namespace shop {"), "{header}");
        assert!(header.contains("namespace panel {"), "{header}");
        assert!(header.contains("namespace Area {"), "{header}");
        assert!(header.contains("::xirang::Id NODE_ID"), "{header}");
    }

    #[test]
    fn two_objects_on_one_catalog_path_are_refused() {
        let error = render_catalog(&[
            ("shop/panel".to_owned(), [1; 16]),
            ("shop/panel".to_owned(), [2; 16]),
        ])
        .expect_err("one path cannot name two identities");
        assert!(error.contains("share the catalog path"), "{error}");
    }
}
