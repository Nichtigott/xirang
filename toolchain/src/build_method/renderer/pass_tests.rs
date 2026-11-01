//! Tests for the render pass: what each crate shape compiles, and what it leaves to another crate.
//! 渲染过程的测试：每种 crate 形状编译什么、又把什么留给另一个 crate。
//!
//! A sibling file rather than an inline module: with the tests inside it, `pass.rs` crossed the
//! source budget (600 code lines) the moment the render walk started splitting containers itself.
//! 同级文件而不是内联模块：当渲染遍历开始自己切分容器时，把测试放在 `pass.rs` 里面会让它越过源码预算
//! （600 代码行）。

use super::ShapeRender;
use super::render_lib;
use crate::build_method::Node;
use crate::build_method::SourceScope;
use crate::build_method::diagnostics::BuildDiagnostics;
use crate::build_method::registry_syntax::GraftSyntax;
use crate::build_method::renderer::test_support::{temporary_directory, write_registry};
use std::fs;
use std::path::Path;
use xirang_kernel::registry_core::syntax::SyntaxLocation;

/// Everything a render produced: the crate-root text plus every container file it emitted.
/// 一次渲染产出的一切：crate 根文本，加上它发射的每一个容器文件。
///
/// Since `stem 2`, step 3 the walk splits as it descends instead of handing back one big source
/// for a splitter to parse again — so a test asking "does this face load its real file" has to look
/// at both halves. The split is a relocation, not a change of content: the `#[path]` line that used
/// to sit inside the crate-root text now sits at the top of the container's own file.
/// 自 `stem 2` 第③步起，遍历在下行时就切分，而不是交回一大份源码让切分器再解析一遍——因此"这个面是否
/// 按真实文件载入"这类断言要看**两半**。这次切分是**搬家**而不是改内容：过去住在 crate 根文本里的
/// `#[path]` 行，现在住在容器自己文件的开头。
fn rendered(output: &str, modules: &[(String, String)]) -> String {
    let mut all = output.to_owned();
    for (_, body) in modules {
        all.push_str(body);
    }
    all
}

#[test]

fn generated_static_plan_owns_build_method_graft_selectors() {
    let root = temporary_directory("static-graft-plan");
    let output = render_lib(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[GraftSyntax {
            cfg: None,
            cut: "root/control/button".to_owned(),
            cut_end: None,
            graft: "button_fast".to_owned(),
            full: false,
            location: SyntaxLocation { line: 3, column: 5 },
            expressions: None,
            spans: None,
        }],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );

    assert!(output.contains("StaticPlan::with_grafts"));
    assert!(output.contains("StaticGraftCut::new(\"root/control/button\", \"button_fast\""));
    assert!(!output.contains("pub fn builtin_graft_plan"));
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A string path that literally contains `" to "` is one cut, and the
/// renderer must not hand its halves to `new_range`. This is the consumer
/// side of the range-as-data change: the parser keeps `cut_end` empty for
/// such a path, and the renderer reads that field instead of the path text.
/// 字面含有 `" to "` 的字符串路径是一条切口，渲染器绝不能把它的两半交给
/// `new_range`。这是“区间即数据”改动的消费侧：解析器对这类路径让 `cut_end`
/// 保持为空，渲染器读取该字段而不是路径文本。
#[test]
fn a_path_containing_the_range_word_is_rendered_as_one_cut() {
    let root = temporary_directory("string-cut-to-word");
    let entries = crate::build_method::registry_syntax::graft_entries(
        r#"xirang_kernel::static_graft_plan!(FRAMEWORK, cut "root/a to b" graft "g");"#,
    )
    .expect("declaration parses");
    let output = render_lib(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &entries,
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );

    assert!(
        output.contains(r#"StaticGraftCut::new("root/a to b", "g", false)"#),
        "the whole path must stay one cut: {output}"
    );
    assert!(
        !output.contains("new_range"),
        "a literal path is not a range: {output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// The `SUBTREE` marker exists **exactly** on the nodes that have children — which is what makes
/// "a crate is a subtree" the compiler's rule rather than a convention: a leaf is mounted straight
/// at its own file, so it gets no inline module, so it gets no marker, so `…::button::SUBTREE`
/// cannot be written. Nothing else enforces that rule (it lives in this renderer's shape, not in a
/// line of code), so it is pinned here.
/// `SUBTREE` 标记**恰好**存在于有子节点的节点上——这正是"crate 是一棵子树"成为编译器规则而不是约定的
/// 原因：叶子直接挂到它自己的文件上，因此没有内联模块、拿不到标记，于是 `…::button::SUBTREE` 写不出来。
/// 除此之外没有东西守这条规则（它住在本渲染器的**形状**里，而不是某一行代码里），所以在这里钉住。
#[test]
fn the_subtree_marker_exists_only_where_a_subtree_does() {
    let root = temporary_directory("subtree-marker");
    let control = root.join("control/control.rs");
    let button = root.join("control/object/button/button.rs");
    write_registry(&control, "root_object", "Control", "crate::ROOT_NODE_ID");
    write_registry(
        &button,
        "control_object",
        "Button",
        "crate::control::NODE_ID",
    );
    let nodes = vec![Node {
        name: "control".to_owned(),
        file: Some(control),
        children: vec![Node {
            name: "object".to_owned(),
            file: None,
            children: vec![Node {
                name: "button".to_owned(),
                file: Some(button),
                children: Vec::new(),
            }],
        }],
    }];

    let mut modules = Vec::new();
    let output = render_lib(
        &root,
        &nodes,
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut modules,
    );
    let output = rendered(&output, &modules);

    assert_eq!(
        output.matches("pub const SUBTREE").count(),
        2,
        "exactly the two nodes with children carry the marker:\n{output}"
    );
    // Since `stem 2`, step 3 a container's body **is** its own file, so the question "does this
    // container carry the marker" is asked of that file. The keys are the walk's own:
    // `{parent_key}{name.len()}_{name}`, length-prefixed so `a_b/c` cannot collide with `a/b_c`.
    // 自 `stem 2` 第③步起，容器的体**就是**它自己的文件，因此"这个容器带标记吗"要问那个文件。键是遍历
    // 自己的：`{父键}{名字长度}_{名字}`，带长度前缀，好让 `a_b/c` 不会与 `a/b_c` 撞上。
    let container = |key: &str| {
        modules
            .iter()
            .find(|(written, _)| written == key)
            .map(|(_, body)| body.as_str())
            .unwrap_or_else(|| panic!("a container file named {key}: {modules:?}"))
    };
    assert!(
        container("7_control").contains("pub const SUBTREE"),
        "the container face carries the marker"
    );
    assert!(
        container("7_control6_object").contains("pub const SUBTREE"),
        "the directory node with children carries the marker"
    );
    assert!(
        output.contains("pub mod button;\n"),
        "the leaf is mounted straight at its file:\n{output}"
    );
    assert!(
        !output.contains("pub mod button {\n"),
        "a leaf gains no inline module, so it gains no marker:\n{output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// Every baked face identity is asserted against the face's own `NODE_ID`, so a face compiled
/// from a different namespace or `#[path]` spelling fails the build instead of quietly owning an
/// identity the plan never heard of (measured 2026-10-06: without this, remapping the host's
/// `src` path built cleanly while every face identity had moved).
/// 烤进去的每个面身份都会与面自己的 `NODE_ID` 对账，因此用不同命名空间或 `#[path]` 拼写编译出来的面会让
/// 构建失败，而不是悄悄拥有一个计划从没听说过的身份（2026-10-06 实测：没有这条断言时，把宿主的 `src`
/// 路径 remap 走后构建照样成功，而每个面的身份都已经变了）。
#[test]
fn every_baked_face_identity_is_asserted_against_the_face_itself() {
    let root = temporary_directory("static-identity");
    let records = [crate::build_method::static_plan::StaticFaceRecord {
        written_as_literal: false,
        kind: "Face".to_owned(),
        preset: "Preset".to_owned(),
        parts: "Parts".to_owned(),
        id: xirang_kernel::registry_core::identity::NodeId::from_bytes(b"face"),
        parent: xirang_kernel::registry_core::identity::NodeId::from_bytes(b"parent"),
        owns_registry: false,
        source: "control/object/button/button.rs".to_owned(),
        module: "control::object::button".to_owned(),
    }];
    let output = render_lib(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &records,
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );

    assert!(
        output.contains("assert_static_identity(crate::control::object::button::NODE_ID"),
        "the face's own id must be checked against the baked one: {output}"
    );

    // The literal spelling carries the same module-level `NODE_ID` (the writer bakes it into the generated
    // region), so its assertion is the same real cross-check. It used to be emitted with the baked id on
    // **both** sides; the maintainer's 2026-10-16 ruling removed that tautology — a literal face without the
    // constant now fails to compile here instead of passing a check that compared a value with itself.
    // 字面量拼法同样带模块级 `NODE_ID`（写者把它烘进生成区域），因此它的断言也是同一个真交叉核对。它过去被
    // 发射成两边都是烤进去的 id；维护者 2026-10-16 的裁定删掉了那个同义反复——没有这个常量的字面量面现在会在
    // 这里编译失败，而不是在一次"自己与自己比较"的检查上通过。
    let mut literal_records = records;
    literal_records[0].written_as_literal = true;
    let literal = render_lib(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &literal_records,
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );
    assert!(
        literal.contains("assert_static_identity(crate::control::object::button::NODE_ID"),
        "a literal-form face carries its own `NODE_ID`, so its assertion is a real cross-check: {literal}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// Faces load as real modules through `#[path]`, never as copies. A leaf
/// face keeps its own module name so the public path is unchanged; a face
/// that owns child registries must also be their container, so it loads
/// into a same-named child module and re-exports.
/// 注册面通过 `#[path]` 以真实模块载入，绝不是副本。叶子面保留自身模块名，
/// 公开路径不变；拥有子注册机的面必须同时充当容器，因此载入同名子模块再
/// 重导出。
#[test]
fn faces_load_by_path_without_copies_or_injected_constants() {
    let root = temporary_directory("path-loaded-faces");
    let control = root.join("control/control.rs");
    let button = root.join("control/object/button/button.rs");
    write_registry(&control, "root_object", "Control", "crate::ROOT_NODE_ID");
    write_registry(
        &button,
        "control_object",
        "Button",
        "crate::control::NODE_ID",
    );
    let nodes = vec![Node {
        name: "control".to_owned(),
        file: Some(control.clone()),
        children: vec![Node {
            name: "object".to_owned(),
            file: None,
            children: vec![Node {
                name: "button".to_owned(),
                file: Some(button.clone()),
                children: Vec::new(),
            }],
        }],
    }];

    let mut modules = Vec::new();
    let output = render_lib(
        &root,
        &nodes,
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut modules,
    );
    let output = rendered(&output, &modules);

    let control_path = xirang_kernel::declaration::portable_path(&control.to_string_lossy());
    let button_path = xirang_kernel::declaration::portable_path(&button.to_string_lossy());
    assert!(
        output.contains(&format!("#[path = {control_path:?}]")),
        "folder face must load its real file: {output}"
    );
    assert!(
        output.contains(&format!("#[path = {button_path:?}]")),
        "leaf face must load its real file: {output}"
    );
    assert!(
        !output.contains("include!("),
        "faces must be modules, not included copies: {output}"
    );
    assert!(
        !output.contains("__REGISTRATION_SOURCE") && !output.contains("__REGISTRATION_MODULE_NAME"),
        "identity is derived at the declaration site, so nothing is injected: {output}"
    );
    assert!(
        output.contains("mod control;") && output.contains("pub use control::*;"),
        "folder face loads into a same-named child module and re-exports: {output}"
    );
    assert!(
        output.contains("pub mod button;"),
        "leaf face keeps its own module name: {output}"
    );
    assert!(
        !output.contains("pub mod button {"),
        "leaf face must not gain a container module: {output}"
    );
    assert!(
        !output.contains("__xirang_ra_"),
        "one module tree for rustc and rust-analyzer"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A top-level leaf face already sits where rust-analyzer applies
/// `#[path]`, so it must not gain a shadow or a `cfg` gate.
/// 顶层叶子面本来就位于 rust-analyzer 应用 `#[path]` 的位置，因此不应多出
/// 影子声明或 `cfg` 开关。
#[test]
fn top_level_faces_keep_a_single_declaration() {
    let root = temporary_directory("top-level-face");
    let control = root.join("control.rs");
    write_registry(&control, "root_object", "Control", "crate::ROOT_NODE_ID");
    let nodes = vec![Node {
        name: "control".to_owned(),
        file: Some(control.clone()),
        children: Vec::new(),
    }];

    let output = render_lib(
        &root,
        &nodes,
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );

    assert!(
        output.contains("pub mod control;"),
        "top-level leaf face keeps its declaration: {output}"
    );
    assert!(
        !output.contains("__xirang_ra_"),
        "a face the IDE already sees needs no shadow: {output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// The `#[path]` text the renderer writes is the kernel's portable form, not
/// a fold of its own. The two agree on Windows precisely because both fold,
/// so a path whose *name* carries a backslash is where a divergent copy shows
/// up — and that assertion is the same one on every platform.
/// 渲染器写出的 `#[path]` 文本用的是内核的可移植形式，而不是它自己的一份折叠。两者在
/// Windows 上一致恰恰因为都做了折叠，因此名称里含反斜杠的路径正是分叉副本会露出的地方——
/// 而这个断言在每个平台上都是同一句话。
#[test]
fn a_rendered_path_is_the_kernel_portable_form() {
    let root = temporary_directory("portable-path");
    let face = root.join("control\\control.rs");
    write_registry(&face, "root_object", "Control", "crate::ROOT_NODE_ID");
    let nodes = vec![Node {
        name: "control".to_owned(),
        file: Some(face.clone()),
        children: Vec::new(),
    }];

    let output = render_lib(
        &root,
        &nodes,
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );

    let folded = xirang_kernel::declaration::portable_path(&face.to_string_lossy());
    assert!(
        folded.ends_with("control/control.rs"),
        "the fixture has to exercise a fold: {folded}"
    );
    assert!(
        output.contains(&format!("#[path = {folded:?}]")),
        "the rendered path must be the kernel's portable form: {output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A subtree the crate shape hands to **another** crate is not rendered here — and returning on it
/// skips its whole subtree, which is what "the same face is not compiled twice" means (audit `M7`,
/// P3.2). The records are a different question: the tree is unchanged, so every face is still
/// published in the manifests; what changes is which crate emits the module.
/// crate 形状交给**另一个** crate 的子树不在这里渲染——而在它上面直接返回会连整棵子树一起跳过，这正是
/// "同一个面不编译两遍"的含义（审计 `M7`，P3.2）。记录是另一个问题：树没变，因此每个面仍发布在清单里；
/// 变的只是"哪个 crate 发射这个模块"。
#[test]
fn a_subtree_handed_to_another_crate_is_not_rendered_here() {
    let root = temporary_directory("cut-out");
    let control = root.join("control/control.rs");
    let button = root.join("control/object/button/button.rs");
    write_registry(&control, "root_object", "Control", "crate::ROOT_NODE_ID");
    write_registry(
        &button,
        "control_object",
        "Button",
        "crate::control::NODE_ID",
    );
    let container = |name: &str| Node {
        name: name.to_owned(),
        file: None,
        children: Vec::new(),
    };
    let mut object = container("object");
    object.children.push(Node {
        name: "button".to_owned(),
        file: Some(button),
        children: Vec::new(),
    });
    let mut parent = Node {
        name: "control".to_owned(),
        file: Some(control),
        children: Vec::new(),
    };
    parent.children.push(object);
    let nodes = vec![parent];
    let render = |cut_out: &[String]| {
        let mut modules = Vec::new();
        let output = render_lib(
            &root,
            &nodes,
            &BuildDiagnostics::default(),
            &BuildDiagnostics::default(),
            &SourceScope {
                roots: None,
                reason: "test",
            },
            &[],
            &[],
            ShapeRender {
                cut_out,
                only: None,
                mounts: &[],
                ancestors: &[],
                facade: false,
                owners: &[],
            },
            Path::new("xirang_modules"),
            &mut modules,
        );
        rendered(&output, &modules)
    };

    // The control: with nothing cut out, both modules are emitted.
    // 对照：什么都没切出时，两个模块都发射。
    let whole = render(&[]);
    assert!(whole.contains("pub mod control"), "{whole}");
    assert!(whole.contains("pub mod object"), "{whole}");
    assert!(whole.contains("pub mod button"), "{whole}");

    // The claim: the declared subtree is not compiled here, and neither is anything below it.
    // 主张：被声明的子树不在这里编译，它下面的一切也不。
    let cut = render(&["control::object".to_owned()]);
    assert!(cut.contains("pub mod control"), "the parent stays: {cut}");
    assert!(
        !cut.contains("pub mod object"),
        "the cut-out subtree: {cut}"
    );
    assert!(!cut.contains("pub mod button"), "nor below it: {cut}");

    // And a path that names nothing changes nothing — the set is matched exactly.
    // 而点名了不存在的东西不会改变什么——这个集合是精确匹配的。
    let elsewhere = render(&["control::missing".to_owned()]);
    assert_eq!(whole, elsewhere, "an unmatched path renders the same tree");
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A ghost crate renders **only** the fragment it was created to compile: the claimed subtree is
/// mounted, the nodes above it become **shells** (module paths, no face file — mounting an
/// ancestor would register that face a second time, in this crate's registry), and every sibling
/// is left out (audit `M7`, P3.2).
/// 幽灵 crate **只**渲染它被创建来编译的那个碎片：认领的子树被挂载，它上面的节点变成**壳**（保留模块
/// 路径、不挂面文件——挂载祖先会把那个面第二次注册进本 crate 的注册机），而每个兄弟都不发射
/// （审计 `M7`，P3.2）。
#[test]
fn a_ghost_renders_its_fragment_and_shells_its_ancestors() {
    let root = temporary_directory("ghost-fragment");
    let control = root.join("control/control.rs");
    let button = root.join("control/object/button/button.rs");
    let panel = root.join("panel/panel.rs");
    let gauge = root.join("panel/gauge/gauge.rs");
    write_registry(&control, "root_object", "Control", "crate::ROOT_NODE_ID");
    write_registry(
        &button,
        "control_object",
        "Button",
        "crate::control::NODE_ID",
    );
    write_registry(&panel, "root_object", "Panel", "crate::ROOT_NODE_ID");
    write_registry(&gauge, "control_object", "Gauge", "crate::panel::NODE_ID");
    let leaf = |name: &str, file: &std::path::Path| Node {
        name: name.to_owned(),
        file: Some(file.to_path_buf()),
        children: Vec::new(),
    };
    let container = |name: &str, file: Option<&std::path::Path>, children: Vec<Node>| Node {
        name: name.to_owned(),
        file: file.map(|path| path.to_path_buf()),
        children,
    };
    let nodes = vec![
        container(
            "control",
            Some(&control),
            vec![container("object", None, vec![leaf("button", &button)])],
        ),
        container("panel", Some(&panel), vec![leaf("gauge", &gauge)]),
    ];
    let claimed = ["control::object".to_owned()];
    let mut modules = Vec::new();
    let output = render_lib(
        &root,
        &nodes,
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender {
            cut_out: &[],
            only: Some(&claimed),
            facade: false,
            owners: &[],
            ancestors: &[("control".to_owned(), [7u8; 16])],
            // The spelling the plan computed: a walk out of the ghost and into the host's `src`,
            // which is what `file!()` will report and therefore what the identity is built from.
            // 规划算出的拼写：走出幽灵、走进宿主 `src`，也就是 `file!()` 会报告、因而身份由之构建的那个。
            mounts: &[(
                "control::object::button".to_owned(),
                "../../../../host/src/control/object/button/button.rs".to_owned(),
            )],
        },
        Path::new("xirang_modules"),
        &mut modules,
    );
    let output = rendered(&output, &modules);

    // The claimed subtree is here, and its leaf's file is mounted.
    // 认领的子树在，它的叶子文件被挂载。
    assert!(output.contains("pub mod control"), "{output}");
    assert!(output.contains("pub mod object"), "{output}");
    assert!(output.contains("pub mod button"), "{output}");
    assert!(
        output.contains("../../../../host/src/control/object/button/button.rs"),
        "the claimed leaf is mounted through the planned spelling: {output}"
    );
    assert!(
        !output.contains(
            &root
                .join("control/object/button/button.rs")
                .display()
                .to_string()
        ),
        "and not through this fixture's own path: {output}"
    );
    // The ancestors are shells: their module paths exist, their face files are not mounted, and
    // they carry the identity a fragment's `parent:` names.
    // 祖先是壳：模块路径在、面文件不挂载，而且携带碎片 `parent:` 点名的那个身份。
    assert!(
        !output.contains("control/control.rs"),
        "an ancestor's file is not mounted here: {output}"
    );
    assert!(
        output.contains("pub const NODE_ID: ::xirang_toolchain::run_method::registry_core::NodeId")
            && output.contains(&format!("from_raw({:?})", [7u8; 16])),
        "the shell carries the ancestor's identity: {output}"
    );
    // Everything outside the claim is left out entirely.
    // 认领之外的一律不发射。
    assert!(!output.contains("pub mod panel"), "{output}");
    assert!(!output.contains("panel/gauge/gauge.rs"), "{output}");
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A face written in the new spelling has nothing at module level for the tree to consume, so the tree
/// assembles the snapshot itself — and both consumers read that one constant rather than inventing two.
/// 用新拼法写的面在模块级没有任何东西可供生成树消费，因此生成树**自己拼出**那份快照——两处消费者读的是
/// 同一个常量，而不是各自变一个。
#[test]
fn a_literal_form_face_gets_an_assembled_snapshot_and_both_consumers_read_it() {
    let root = temporary_directory("literal-assembly");
    let records = [crate::build_method::static_plan::StaticFaceRecord {
        written_as_literal: true,
        id: xirang_kernel::registry_core::identity::NodeId::from_bytes(b"card"),
        parent: xirang_kernel::registry_core::identity::NodeId::from_bytes(b"root"),
        kind: "Card".to_owned(),
        preset: "MyPreset".to_owned(),
        parts: "MyParts".to_owned(),
        owns_registry: false,
        source: "card/card.rs".to_owned(),
        module: "card".to_owned(),
    }];
    let output = render_lib(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &records,
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
    );

    assert!(
        output.contains("assemble_face::<card::Card>"),
        "the tree names the face's type to read its type-like half: {output}"
    );
    assert!(
        output.contains("kind: \"Card\"") && output.contains("preset: \"MyPreset\""),
        "and it supplies the names a hit path cannot re-read: {output}"
    );
    assert!(
        output.contains("assert_static_registration(") && output.contains("__XIRANG_ASSEMBLED_0)"),
        "the rule assertion checks the assembled snapshot: {output}"
    );
    assert!(
        output.contains("        __XIRANG_ASSEMBLED_0,"),
        "and the registrations list publishes the same one, not a second: {output}"
    );
    assert!(
        !output.contains("card::REGISTRATION"),
        "nothing may name a module-level constant a literal-form face does not have: {output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A face a C++ build declared lands in the same table and the same snapshot list
/// as a `#[soil]` face. That is the whole point of the external-face slice: one
/// tree, two declaration languages, one identity space — not two tables a reader
/// would have to merge.
/// C++ 构建声明的面与 `#[soil]` 面进的是同一张表与同一份快照清单。这正是外部面切片的
/// 全部意义：一棵树、两种声明语言、一个身份空间——而不是两张要读者自己合并的表。
#[test]
fn an_external_face_lands_in_the_same_table_and_registration_list() {
    let root = temporary_directory("external-face-table");
    let record = crate::build_method::external_face::ExternalFaceRecord::new(
        xirang_kernel::registry_core::NodeId::from_raw([7; 16]),
        xirang_kernel::registry_core::NodeId::from_raw([9; 16]),
        false,
        "Area".to_owned(),
        vec!["area".to_owned(), "area_of".to_owned()],
        "native/area/area.cpp".to_owned(),
    );
    let output = super::render_lib_with_external(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
        &[record],
        &[],
    );
    // The table entry: identity and parent are the record's, and `owns_registry` is
    // the flag bit the reader decoded.
    assert!(output.contains("NodeId::from_raw([7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7])"));
    assert!(
        output
            .contains("NodeId::from_raw([9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9]), false)")
    );
    // The snapshot: assembled from the record alone, both type parameters empty.
    assert!(output.contains("const __XIRANG_EXTERNAL_0:"));
    assert!(output.contains("assemble::<::xirang_toolchain::run_method::registry_core::NoPreset"));
    assert!(output.contains(r#"exports: &["area", "area_of"]"#));
    assert!(output.contains(r#"file: "native/area/area.cpp""#));
    // And it is in the list the host actually registers.
    assert!(output.contains("        __XIRANG_EXTERNAL_0,"));
    // The record's ownership flag reaches the snapshot: a parent that declares `.owns_registry()`
    // owns one at run time, and a child under it is not dropped for a missing registry. Measured
    // 2026-10-22: this line was missing and the runtime dropped the child in silence.
    // 记录的属主标志必须进到快照里：声明了 `.owns_registry()` 的父面在运行期真的自持注册机，它下面的
    // 子面不会因为"找不到注册机"被丢掉。2026-10-22 实测：少了这一行，运行期把子面静默丢了。
    assert!(
        output.contains("needs_registry: false,"),
        "a record that owns no registry must say so: {output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// 拥有注册机的外部面在快照里写着 `needs_registry: true`——这是"父面能拥有子面"落到运行期的那一位。
/// A foreign face that owns a registry says `needs_registry: true` in its snapshot — the bit that
/// lets it be a parent at run time.
#[test]
fn an_external_face_that_owns_a_registry_says_so() {
    let root = temporary_directory("external-face-owns");
    let record = crate::build_method::external_face::ExternalFaceRecord::new(
        xirang_kernel::registry_core::NodeId::from_raw([5; 16]),
        xirang_kernel::registry_core::NodeId::from_raw([6; 16]),
        true,
        "Area".to_owned(),
        vec!["area".to_owned()],
        "native/area/area.cpp".to_owned(),
    );
    let output = super::render_lib_with_external(
        &root,
        &[],
        &BuildDiagnostics::default(),
        &BuildDiagnostics::default(),
        &SourceScope {
            roots: None,
            reason: "test",
        },
        &[],
        &[],
        ShapeRender::whole(),
        Path::new("xirang_modules"),
        &mut Vec::new(),
        &[record],
        &[],
    );
    assert!(
        output.contains("needs_registry: true,"),
        "an owning foreign face has to say it owns a registry: {output}"
    );
    // And the static table carries the same flag, so the tree knows it before registration.
    assert!(
        output
            .contains("NodeId::from_raw([6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6]), true)"),
        "the static table has to carry the ownership flag: {output}"
    );
    fs::remove_dir_all(root).expect("temporary fixture cleanup");
}
