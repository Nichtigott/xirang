//! Tests for the declared-native reader: who compiles a source, and what it says.
//! 声明式本地源码读取器的测试：谁编译一份源码，以及它声明了什么。

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xirang-declared-native-{}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

/// A source that declares one face, with the parent spelling handed in.
/// 一份声明了一个面的源码，父的拼写由参数给出。
fn declarer(root: &Path, relative: &str, parent: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent directory"))
        .expect("the source directory");
    std::fs::write(
        &path,
        format!(
            "#include \"xirang/soil.hpp\"\n\
             #include \"xirang_faces.hpp\"\n\
             struct Area {{}};\n\
             XIRANG_FACE(r_area, (::xirang::soil<Area>().under(::xirang::faces::{parent}).named(\"Area\").exports(\"area\")));\n"
        ),
    )
    .expect("the source");
}

/// A marked declaration with a nested parent reference: the spelling the C++ SDK documents.
/// 带标记声明配嵌套父引用：C++ SDK 文档里的那种拼法。
fn marked_declarer(root: &Path, relative: &str, parent: &str, owns: bool) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent directory"))
        .expect("the source directory");
    std::fs::write(
        &path,
        format!(
            "#include \"xirang/soil.hpp\"\n\
             #include \"xirang/faces.hpp\"\n\
             struct Gauge {{}};\n\
             XIRANG_SOIL\n\
             constexpr auto SOIL = ::xirang::soil<Gauge>()\n\
                 .under(::xirang::faces::{parent})\n\
                 .named(\"Gauge\")\n\
                 .needs_registry({owns});\n"
        ),
    )
    .expect("the source");
}

/// A source with no declaration: a helper, not a face.
/// 一份没有声明的源码：帮手，不是面。
fn helper(root: &Path, relative: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent directory"))
        .expect("the source directory");
    std::fs::write(&path, "int helper() { return 1; }\n").expect("the source");
}

fn face(source: &str, parent: &str) -> DeclaredFace {
    DeclaredFace {
        source: source.to_owned(),
        parent: parent.to_owned(),
        name: "Area".to_owned(),
        owns_registry: false,
    }
}

#[test]
fn a_declared_source_is_discovered_without_a_configuration_file() {
    let root = scratch("discover");
    declarer(&root, "native/area/area.cpp", "shop__panel_Panel");
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].source, "native/area/area.cpp");
    assert_eq!(faces[0].parent, "shop__panel_Panel");
}

#[test]
fn a_source_without_a_declaration_is_not_compiled() {
    let root = scratch("helper-only");
    helper(&root, "native/util/util.cpp");
    assert!(
        read(&root).expect("the walk succeeds").is_none(),
        "a source that never declares a face is not a face's implementation"
    );
}

#[test]
fn a_helper_next_to_a_declarer_is_left_alone() {
    let root = scratch("helper-next-to-declarer");
    declarer(&root, "native/area/area.cpp", "shop_Shop");
    helper(&root, "native/area/support.cpp");
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].source, "native/area/area.cpp");
}

#[test]
fn a_generated_directory_is_not_walked() {
    let root = scratch("generated-directory");
    declarer(
        &root,
        "crates/shop-panel/native/area/area.cpp",
        "shop__panel_Panel",
    );
    declarer(&root, "native/area/area.cpp", "shop_Shop");
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(
        faces
            .iter()
            .map(|face| face.source.as_str())
            .collect::<Vec<_>>(),
        vec!["native/area/area.cpp"],
        "the generated crates beside the host are not this package's sources"
    );
}

#[test]
fn a_fragment_compiles_the_faces_whose_parent_it_claimed() {
    let declared = vec![
        face("native/area/area.cpp", "shop__panel_Panel"),
        face("native/shop/thing.cpp", "shop_Shop"),
    ];
    let owned = owned(declared, Some("shop::panel"), false, &[]);
    assert_eq!(owned.len(), 1);
    assert_eq!(owned[0].source, "native/area/area.cpp");
}

#[test]
fn a_host_compiles_what_it_did_not_hand_away() {
    let declared = vec![
        face("native/area/area.cpp", "shop__panel_Panel"),
        face("native/shop/thing.cpp", "shop_Shop"),
    ];
    let owned = owned(declared, None, false, &["shop::panel".to_owned()]);
    assert_eq!(owned.len(), 1);
    assert_eq!(owned[0].source, "native/shop/thing.cpp");
}

#[test]
fn a_facade_compiles_nothing() {
    let declared = vec![face("native/area/area.cpp", "shop__panel_Panel")];
    assert!(owned(declared, None, true, &[]).is_empty());
}

#[test]
fn a_claim_covers_a_parent_and_not_a_sibling_that_shares_its_prefix() {
    let declared = vec![
        face("native/area/area.cpp", "shop__panel_Panel"),
        face("native/panel/thing.cpp", "shop__panels_Panel"),
    ];
    let owned = owned(declared, Some("shop::panel"), false, &[]);
    assert_eq!(
        owned.len(),
        1,
        "`shop::panel` must not swallow `shop::panels`"
    );
    assert_eq!(owned[0].source, "native/area/area.cpp");
}

#[test]
fn a_written_recipe_overrides_the_extension_defaults() {
    let root = scratch("recipe-override");
    declarer(&root, "native/area/area.cpp", "shop_Shop");
    std::fs::write(
        root.join("xirang.toml"),
        "[native]\nprogram = \"clang++\"\nargs = [\"-std=c++23\"]\ninclude = [\"native\"]\n",
    )
    .expect("the manifest");
    let (recipe, _) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(recipe.program, "clang++");
    assert_eq!(recipe.arguments, vec!["-std=c++23".to_owned()]);
    assert_eq!(recipe.includes, vec!["native".to_owned()]);
}

#[test]
fn the_compiled_object_carries_the_record_the_reader_decodes() {
    let Some(program) = gxx() else {
        eprintln!("no C++ compiler on this machine; the compile half is not exercised");
        return;
    };
    let root = scratch("compile-round-trip");
    declarer(&root, "native/area/area.cpp", "shop__panel_Panel");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("the output directory");
    // The generated header stands in for pass one's output: the same constant, spelled the same way,
    // in the directory the compiler is told to search (`-I<out_dir>`).
    // 生成的头替代第一遍的产物：同一个常量、同一种拼写，放在编译器被告知要搜的目录（`-I<out_dir>`）里。
    std::fs::write(
        out.join("xirang_faces.hpp"),
        "namespace xirang::faces {\n\
         inline constexpr ::xirang::Id shop__panel_Panel = ::xirang::Id{{ 0x01, 0x02, 0x03, 0x04, \
         0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10 }};\n\
         }\n",
    )
    .expect("the header");
    let (recipe, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    let recipe = Recipe { program, ..recipe };
    let objects = compile(&root, &out, &recipe, &faces, false).expect("the source compiles");
    assert_eq!(objects.len(), 1);
    let sources = sources(&objects, &faces);
    let records = crate::build_method::external_face::records(&sources, "shop").expect("records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "Area");
    assert_eq!(records[0].exports, vec!["area".to_owned()]);
    assert_eq!(
        records[0].parent,
        xirang_kernel::NodeId::from_raw([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10
        ]),
        "the parent the source named is the parent the record carries"
    );
}

/// The C++ compiler this machine has, if it has one.
/// 本机有的 C++ 编译器，如果有的话。
fn gxx() -> Option<String> {
    for candidate in ["g++", "c++", "clang++"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return Some(candidate.to_owned());
        }
    }
    None
}

/// A source that declares one face, with name, parent, exports and ownership handed in.
/// 一份声明了一个面的源码，名字、父、导出与所有权都由参数给出。
fn write_face(root: &Path, relative: &str, name: &str, parent: &str, owns: bool, export: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent directory"))
        .expect("the source directory");
    let owns = if owns { ".owns_registry()" } else { "" };
    std::fs::write(
        &path,
        format!(
            "#include \"xirang/soil.hpp\"\n\
             #include \"xirang_faces.hpp\"\n\
             struct {name} {{}};\n\
             XIRANG_FACE(r_{name}, (::xirang::soil<{name}>().under(::xirang::faces::{parent}).named(\"{name}\").exports(\"{export}\"){owns}));\n"
        ),
    )
    .expect("the source");
}

/// 解析把父链连起来，并给出生成头里那个常量名：父面的模块段在前，叶子面自己的名字在最后。
/// 判据是"链上的父都能解析"——头里的常量的存在时间必须早于 C++ 源码的编译，所以标识符是构建期
/// 唯一能交出去的东西。
/// Resolution follows the parent chain and names the constant the generated header carries: the
/// parent face's module segments first, the leaf's own name last. The judgement is that every
/// parent on the chain resolves — the constants have to exist before the C++ source compiles, so
/// the identifier is the only thing the build can hand over at that point.
#[test]
fn resolving_a_chain_names_each_face_the_way_its_header_does() {
    let root = scratch("chain");
    declarer(&root, "native/area/area.cpp", "shop__panel_Panel");
    write_face(
        &root,
        "native/area/gauge.cpp",
        "Gauge",
        "native__area_Area",
        false,
        "gauge",
    );
    let declared = read(&root)
        .expect("a readable declaration")
        .expect("the declarations")
        .1;
    let panel = crate::build_method::registry_identity::NodeId::from_namespaced_path(
        "demo",
        "shop/panel/panel.rs",
        "Panel",
    );
    let mut known = std::collections::BTreeMap::new();
    known.insert(
        "shop__panel_Panel".to_owned(),
        ("shop::panel".to_owned(), panel),
    );
    let resolved = resolve("demo", &declared, &known).expect("a resolvable chain");
    assert_eq!(resolved.len(), 2, "{resolved:?}");
    let area = resolved
        .iter()
        .find(|face| face.name == "Area")
        .expect("the parent face");
    assert_eq!(area.ident, "native__area_Area");
    assert_eq!(area.parent, panel);
    let gauge = resolved
        .iter()
        .find(|face| face.name == "Gauge")
        .expect("the child face");
    assert_eq!(gauge.ident, "native__area__gauge_Gauge");
    assert_eq!(
        gauge.parent, area.id,
        "the child hangs under the face its declaration names"
    );
    assert_eq!(
        gauge.id,
        crate::build_method::registry_identity::NodeId::from_namespaced_path(
            "demo",
            "native/area/gauge.cpp",
            "Gauge"
        )
    );
}

/// 父名拼错时拒绝并点名源码与拼写，同时列出真正存在的名字——否则读者只能猜。
/// A misspelt parent is refused by naming the source and the spelling, and by listing the names
/// that do exist; otherwise the reader can only guess.
#[test]
fn a_parent_no_known_face_spells_is_refused_with_the_names_that_work() {
    let root = scratch("unknown-parent");
    declarer(&root, "native/area/area.cpp", "shop__panel_Pannell");
    let declared = read(&root)
        .expect("a readable declaration")
        .expect("the declarations")
        .1;
    let error = resolve("demo", &declared, &std::collections::BTreeMap::new())
        .expect_err("a parent nobody spells is refused");
    assert!(error.contains("cannot resolve"), "{error}");
    assert!(error.contains("native/area/area.cpp"), "{error}");
    assert!(error.contains("shop__panel_Pannell"), "{error}");
}

/// 两个声明互相点名对方当父时，谁也算不出身份——解析必须拒绝并点名两份源码，而不是让构建期
/// 停在半路或发出一张只有一半面的表。
/// When two declarations name each other as parent, neither identity can be derived: resolution has to
/// refuse and name both sources rather than stop halfway or emit a table with half the faces.
#[test]
fn a_cycle_between_declared_faces_is_refused_not_half_resolved() {
    let root = scratch("parent-cycle");
    write_face(
        &root,
        "native/first.cpp",
        "First",
        "second_Second",
        false,
        "first",
    );
    write_face(
        &root,
        "native/second.cpp",
        "Second",
        "first_First",
        false,
        "second",
    );
    let declared = read(&root)
        .expect("a readable declaration")
        .expect("the declarations")
        .1;
    let error = resolve("demo", &declared, &std::collections::BTreeMap::new())
        .expect_err("a cycle has no first face to resolve");
    assert!(error.contains("cannot resolve"), "{error}");
    assert!(error.contains("native/first.cpp"), "{error}");
    assert!(error.contains("native/second.cpp"), "{error}");
}

/// 归属跟着父链走：`Gauge` 挂在 `Area` 下，而 `Area` 的父才在认领的子树里——两者必须都归那个 crate，
/// 否则宿主会去编译一个不属于它的面，而碎片反而编译不到它。
/// Ownership follows the parent chain: `Gauge` hangs under `Area`, and only `Area`'s parent is inside
/// the claimed subtree — both belong to that crate, or the host compiles a face it does not own while
/// the fragment never compiles it.
#[test]
fn ownership_follows_the_parent_chain_not_the_parents_spelling() {
    let root = scratch("chain-ownership");
    declarer(&root, "native/area/area.cpp", "shop__panel_Panel");
    write_face(
        &root,
        "native/area/gauge.cpp",
        "Gauge",
        "native__area_Area",
        false,
        "gauge",
    );
    let declared = read(&root)
        .expect("a readable declaration")
        .expect("the declarations")
        .1;
    let fragment = owned(declared.clone(), Some("shop::panel"), false, &[]);
    assert_eq!(
        fragment
            .iter()
            .map(|face| face.source.as_str())
            .collect::<Vec<_>>(),
        vec!["native/area/area.cpp", "native/area/gauge.cpp"],
        "the fragment owns the whole chain"
    );
    let host = owned(declared.clone(), None, false, &["shop::panel".to_owned()]);
    assert!(host.is_empty(), "the host owns nothing under the claim");
}

/// Ownership is judged on both parent spellings: a fragment's claim covers a face whose parent is
/// written `shop::panel::NODE_ID` exactly as it covers the flat spelling.
/// 归属在两种父拼法上都判定：碎片认领覆盖以 `shop::panel::NODE_ID` 书写的父，与覆盖扁平拼法等价。
#[test]
fn ownership_is_judged_on_the_nested_spelling_too() {
    let faces = vec![
        face("native/area/area.cpp", "shop::panel::NODE_ID"),
        face("native/area/gauge.cpp", "shop::panel::NODE_ID"),
    ];
    let fragment = owned(faces, Some("shop::panel"), false, &[]);
    assert_eq!(fragment.len(), 2);
    let other = owned(
        vec![
            face("native/area/area.cpp", "shop::panel::NODE_ID"),
            face("native/area/gauge.cpp", "shop::panel::NODE_ID"),
        ],
        Some("shop::elsewhere"),
        false,
        &[],
    );
    assert!(
        other.is_empty(),
        "a fragment whose claim is another subtree compiles none of these faces"
    );
}

/// 一个把父写成生成头常量的面：父路径是 `crate::xirang_external::<ident>::NODE_ID`。内核的父解析器只认
/// 以 `::NODE_ID` 结尾的路径，所以**解析**能成；**身份**只能来自构建期安装的那张表。
/// A face whose parent is a generated-header constant: the path is
/// `crate::xirang_external::<ident>::NODE_ID`, which the kernel's parent parser accepts because it
/// ends in `::NODE_ID` — so the parse succeeds and the identity can only come from the map the
/// build installs.
#[test]
fn a_parent_declared_in_another_language_resolves_only_while_the_build_has_the_map() {
    let relative = "group_0/object/node_0/node_0.rs";
    let source = r#"
#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil =
    xirang_toolchain::Soil::of::<Node0>().under(crate::xirang_external::native__area_Area::NODE_ID);

pub struct Node0;
"#;
    let face = crate::build_method::face_syntax_check::parsed_face(source, relative)
        .expect("the leaf parses");
    let src = std::env::temp_dir();
    assert_eq!(
        crate::build_method::discovery_cache::cached_parent_id(&src, &face),
        None,
        "with no map installed the identity is unknown, and `None` is the honest answer"
    );
    let expected = xirang_kernel::identity::NodeId::from_namespaced_path(
        "pin",
        "native/area/area.cpp",
        "Area",
    );
    let mut map = std::collections::BTreeMap::new();
    map.insert("native__area_Area".to_owned(), expected);
    let resolved = crate::build_method::discovery_cache::with_native_parents(&map, || {
        crate::build_method::discovery_cache::cached_parent_id(&src, &face)
    });
    assert_eq!(resolved, Some(expected));
}

/// The marked spelling is discovered without a configuration file, and its nested parent is read
/// as written so resolution can look it up by module.
/// 带标记的拼法无需配置文件就被发现，其嵌套父按原样读出，供解析按模块查找。
#[test]
fn a_marked_declaration_is_discovered_with_its_nested_parent() {
    let root = scratch("marked");
    marked_declarer(
        &root,
        "native/area/gauge.cpp",
        "native::area::NODE_ID",
        true,
    );
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].source, "native/area/gauge.cpp");
    assert_eq!(faces[0].parent, "native::area::NODE_ID");
    assert_eq!(faces[0].name, "Gauge");
    assert!(faces[0].owns_registry, "`.needs_registry(true)` owns one");
}

/// `.needs_registry(false)` is a declaration, not an omission: the face does not own a registry.
/// `.needs_registry(false)` 是一条声明而不是省略：这个面不自持注册机。
#[test]
fn needs_registry_false_is_read_as_false() {
    let root = scratch("marked-false");
    marked_declarer(
        &root,
        "native/area/gauge.cpp",
        "native::area::NODE_ID",
        false,
    );
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert!(!faces[0].owns_registry);
}

/// A nested parent is resolved by module, without an encoded kind: the generated header names one
/// `NODE_ID` per module.
/// 嵌套父按模块解析，不编码种类名：生成头每个模块只有一个 `NODE_ID`。
#[test]
fn a_nested_parent_resolves_by_module() {
    use crate::build_method::registry_identity::NodeId;
    let mut known = std::collections::BTreeMap::new();
    known.insert(
        "shop__panel_Panel".to_owned(),
        ("shop::panel".to_owned(), NodeId::from_raw([1; 16])),
    );
    let declared = vec![face("native/area/gauge.cpp", "shop::panel::NODE_ID")];
    let resolved = resolve("shop", &declared, &known).expect("the nested parent resolves");
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].parent, NodeId::from_raw([1; 16]));
}

/// The version-1 flat parent still resolves, so an existing tree keeps building.
/// 版本 1 的扁平父仍然解析，因此既有树继续构建。
#[test]
fn the_flat_parent_still_resolves() {
    use crate::build_method::registry_identity::NodeId;
    let mut known = std::collections::BTreeMap::new();
    known.insert(
        "shop__panel_Panel".to_owned(),
        ("shop::panel".to_owned(), NodeId::from_raw([2; 16])),
    );
    let declared = vec![face("native/area/gauge.cpp", "shop__panel_Panel")];
    let resolved = resolve("shop", &declared, &known).expect("the flat parent resolves");
    assert_eq!(resolved[0].parent, NodeId::from_raw([2; 16]));
}

/// The build scanner uses the same lexer as the adapter: a comment that mentions `XIRANG_SOIL`
/// before an old declaration is not a new declaration.
/// 构建扫描器与适配器用同一套词法：旧声明前提到 `XIRANG_SOIL` 的注释不是新声明。
#[test]
fn a_comment_mentioning_the_marked_spelling_is_not_a_declaration() {
    let root = scratch("comment-marker");
    let path = root.join("native/area/area.cpp");
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &path,
        "// Future migration: XIRANG_SOIL\n\
         #include \"xirang/soil.hpp\"\n\
         struct Area {};\n\
         XIRANG_FACE(r_area, (::xirang::soil<Area>().under(::xirang::faces::shop__panel_Panel).named(\"Area\")));\n",
    )
    .expect("the source");
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].parent, "shop__panel_Panel");
    assert_eq!(faces[0].name, "Area");
}

/// Legal whitespace between the marked keywords is accepted by the build scanner too.
/// 带标记关键字之间的合法空白同样被构建扫描器接受。
#[test]
fn marked_declarations_accept_legal_whitespace() {
    let root = scratch("marked-whitespace");
    let path = root.join("native/area/area.cpp");
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &path,
        "#include \"xirang/soil.hpp\"\n\
         struct Area {};\n\
         XIRANG_SOIL\n\
         constexpr   auto\n\
         SOIL = ::xirang::soil<Area>()\n\
             .under(::xirang::faces::shop__panel_Panel)\n\
             .named(\"Area\");\n",
    )
    .expect("the source");
    let (_, faces) = read(&root)
        .expect("the walk succeeds")
        .expect("a declaration is found");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].name, "Area");
}

/// A marker only inside a string is not a declaration, so the file is not compiled as a declarer.
/// 只出现在字符串里的标记不是声明，因此该文件不会被当作声明文件编译。
#[test]
fn a_string_containing_the_marker_is_not_a_declaration() {
    let root = scratch("string-marker");
    let path = root.join("native/util/util.cpp");
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &path,
        "static const char* note = \"XIRANG_SOIL constexpr auto SOIL = fake;\";\nint helper() { return 1; }\n",
    )
    .expect("the source");
    assert!(
        read(&root).expect("the walk succeeds").is_none(),
        "a marker in a string is not a face declaration"
    );
}

/// Two declarations in one translation unit are refused with a count, not silently attributed to
/// one file's module.
/// 一个翻译单元里的两条声明按数量拒绝，而不是静默归到同一个文件模块。
#[test]
fn two_declarations_in_one_translation_unit_are_refused() {
    let root = scratch("two-declarations");
    let path = root.join("native/area/area.cpp");
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &path,
        "#include \"xirang/soil.hpp\"\n\
         struct Area {}; struct Gauge {};\n\
         XIRANG_FACE(r_area, (::xirang::soil<Area>().under(::xirang::faces::shop__panel_Panel).named(\"Area\")));\n\
         XIRANG_FACE(r_gauge, (::xirang::soil<Gauge>().under(::xirang::faces::native__area_Area).named(\"Gauge\")));\n",
    )
    .expect("the source");
    let error = match read(&root) {
        Err(error) => error,
        Ok(_) => panic!("two declarations in one file must be refused"),
    };
    assert!(error.contains("declares 2 faces"), "{error}");
}
