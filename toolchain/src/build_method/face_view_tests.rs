//! Tests for the read-only face view: the paths and identities it derives, and
//! the registration files it cannot read.
//! 只读注册面视图的测试：它推导出的路径与身份，以及它读不了的注册面文件。

use super::{
    face_views, face_views_and_unreadable, face_views_with_external, read_build_scope,
    read_pruning_manifest,
};
use std::fs;
use std::path::PathBuf;

/// A throwaway package root; the caller writes `src/` files into it.
/// 一次性的包根；调用方往其中写 `src/` 文件。
fn temporary_root(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-face-view-{label}-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    fs::create_dir_all(root.join("src")).expect("fixture src");
    root
}

/// The logical path and the registry name follow the chain a runtime tree
/// would build, and the default registry name is the module's last segment.
/// 逻辑路径与 registry 名跟随运行期树会构建的链，默认注册面名取模块末段。
#[test]
fn faces_carry_their_logical_path_and_default_registry_name() {
    let root = temporary_root("paths");
    let panel = root.join("src/control/object");
    fs::create_dir_all(&panel).expect("panel directory");
    fs::write(
        panel.join("object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("panel face");
    let button = root.join("src/control/object/button");
    fs::create_dir_all(&button).expect("button directory");
    fs::write(
            button.join("button.rs"),
            "crate::root_object! {\n    kind: Button,\n    parent: crate::control::object::NODE_ID,\n}\n",
        )
        .expect("button face");

    let faces = face_views(&root, "host").expect("faces");
    assert_eq!(faces.len(), 2);
    let control = faces
        .iter()
        .find(|face| face.kind == "Control")
        .expect("control face");
    assert_eq!(control.path, "root/object");
    assert!(control.owns_registry);
    assert!(control.parent_resolved);
    let button = faces
        .iter()
        .find(|face| face.kind == "Button")
        .expect("button face");
    assert_eq!(button.registry_name, "button");
    assert!(button.parent_resolved);
    assert!(
        button.id
            == xirang_kernel::identity::NodeId::from_namespaced_path(
                "host",
                "control/object/button/button.rs",
                "Button"
            ),
        "identity must be the namespace + relative path + kind hash"
    );

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A parent the tree cannot resolve keeps the face visible and says so,
/// rather than dropping it or guessing a different parent.
/// 树解析不出的父级仍让该面可见并如实说明，而不是丢掉它或改猜另一个父级。
#[test]
fn an_unresolved_parent_is_reported_not_guessed() {
    let root = temporary_root("unresolved-parent");
    let folder = root.join("src/child");
    fs::create_dir_all(&folder).expect("child directory");
    fs::write(
        folder.join("child.rs"),
        "crate::root_object! {\n    kind: Child,\n    parent: crate::missing::NODE_ID,\n}\n",
    )
    .expect("child face");

    let faces = face_views(&root, "host").expect("faces");
    assert_eq!(faces.len(), 1);
    assert!(!faces[0].parent_resolved);

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// The two manifest readers agree with what the writers in `manifests.rs`
/// emit, including the `# result all` spelling for a whole-tree scope.
/// 两个清单读取方与 `manifests.rs` 的写入方一致，包括整树作用域的 `# result all`
/// 写法。
#[test]
fn manifests_round_trip_through_the_readers() {
    let root = temporary_root("manifests");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out dir");
    fs::write(
            out.join("source_scope.tsv"),
            "# mode\tauto\n# selected\t1\n# node\tsource\tmodule\nabc\tcontrol/button.rs\tcontrol::button\n",
        )
        .expect("scope manifest");
    let scope = read_build_scope(&out).expect("scope");
    assert_eq!(scope.mode, "auto");
    assert!(!scope.all);
    assert_eq!(scope.selected_sources.len(), 1);
    assert!(scope.selected_sources.contains("control/button.rs"));
    assert!(scope.keeps("control::button"), "a selected face is kept");
    assert!(
        scope.keeps("control"),
        "a selected face keeps its parent registry"
    );
    assert!(
        !scope.keeps("control_extra"),
        "the boundary is the `::` segment, not a text prefix"
    );

    fs::write(
        out.join("source_scope.tsv"),
        "# mode\texplicit\n# result\tall\n# selected\tall\n# reason\tscope-all\n",
    )
    .expect("all scope");
    let scope = read_build_scope(&out).expect("all scope");
    assert!(scope.all);
    assert_eq!(scope.reason.as_deref(), Some("scope-all"));

    fs::write(
        out.join("pruning_manifest.tsv"),
        "# node\tsource\tsymbol\n00000000000000000000000000000000\tcontrol/button.rs\t-\n",
    )
    .expect("pruning manifest");
    let rows = read_pruning_manifest(&out).expect("pruning rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source, "control/button.rs");
    assert_eq!(rows[0].symbol, "-");

    assert!(read_build_scope(&root.join("missing")).is_err());
    std::fs::remove_dir_all(&root).expect("cleanup");
}
/// The path is resolved from the parent chain, not from the order the walk
/// happened to visit the files in.
/// 路径由父级链解出，而不是由遍历恰好访问文件的顺序决定。
///
/// `alpha` declares a parent whose directory sorts *after* it, so a walk that
/// answered "root/<own name>" whenever the parent had not been seen yet gave
/// `root/alpha` — a path the runtime tree never had — and froze it in the
/// cache. This is the one case the older test skipped: it asserted the parent's
/// path and left the child's unasserted.
/// `alpha` 声明的父级的目录名排在它**之后**，因此"父级还没见过就用 `root/<自己的名字>`"
/// 的遍历会给出 `root/alpha`——一条运行期的树从未有过的路径——并把它冻进缓存。这正是旧测试绕过
/// 的那一格：它断言了父面的路径，却没断言子面的。
#[test]
fn a_parent_that_sorts_later_still_forms_the_full_path() {
    let root = temporary_root("order");
    let zeta = root.join("src/zeta");
    fs::create_dir_all(&zeta).expect("zeta directory");
    fs::write(
        zeta.join("zeta.rs"),
        "crate::root_object! {\n    kind: Zeta,\n    needs_registry: true,\n}\n",
    )
    .expect("zeta face");
    let alpha = root.join("src/alpha");
    fs::create_dir_all(&alpha).expect("alpha directory");
    fs::write(
        alpha.join("alpha.rs"),
        "crate::root_object! {\n    kind: Alpha,\n    parent: crate::zeta::NODE_ID,\n}\n",
    )
    .expect("alpha face");

    let faces = face_views(&root, "host").expect("faces");
    let path_of = |kind: &str| {
        faces
            .iter()
            .find(|face| face.kind == kind)
            .map(|face| face.path.clone())
            .unwrap_or_else(|| panic!("no {kind} face in {faces:?}"))
    };
    assert_eq!(path_of("Zeta"), "root/zeta");
    assert_eq!(path_of("Alpha"), "root/zeta/alpha");
    let alpha = faces
        .iter()
        .find(|face| face.kind == "Alpha")
        .expect("alpha face");
    assert!(alpha.parent_resolved, "the parent is in the tree");
    let _ = std::fs::remove_dir_all(&root);
}

/// A registration file that does not parse is named instead of dropped in
/// silence, and its child is reported with a broken chain.
/// 解析不了的注册面文件被点名，而不是静默丢弃；它的子面带断链出现。
///
/// The first half of the answer is unchanged — the file is not a face, so it
/// cannot be listed as one — and that is exactly why the second half has to
/// exist: without it the caller answers "this package declares these two faces"
/// with no count and no reason, while the build reports the same file as a
/// `face-syntax` diagnostic.
/// 答案的前半段不变——该文件不是面，因此不能被列成面——而这正是后半段必须存在的原因：没有它，
/// 调用方会回答"这个包声明了这两个面"，既没有计数也没有原因，而构建把同一个文件报成
/// `face-syntax` 诊断。
#[test]
fn a_registration_file_that_does_not_parse_is_named_not_dropped() {
    let root = temporary_root("unparsable");
    let control = root.join("src/control");
    fs::create_dir_all(&control).expect("control directory");
    fs::write(
        control.join("control.rs"),
        "crate::root_object! {\n    kind: Broken\n",
    )
    .expect("broken face");
    let button = root.join("src/button");
    fs::create_dir_all(&button).expect("button directory");
    fs::write(
        button.join("button.rs"),
        "crate::root_object! {\n    kind: Button,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("button face");

    let (faces, unreadable) = face_views_and_unreadable(&root, "host").expect("faces");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].kind, "Button");
    assert!(
        !faces[0].parent_resolved,
        "the parent file did not parse, so the chain is broken and the reply says so"
    );
    assert_eq!(unreadable.len(), 1, "{unreadable:?}");
    assert!(
        unreadable[0].contains("control/control.rs"),
        "the file that could not be read is named: {unreadable:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The **native** spelling of the external form is an external face too — and this was measured as a real defect, not
/// added for symmetry.
/// **原生**拼法的外部形式同样是外部面——而这是实测出来的真缺陷，不是为对称而加的。
///
/// The classifier used to key on the macro name alone (`external_object!` / `__external_object!`). When the external
/// form migrated to the native `#[soil] … .source("…")` spelling, a face that declares its own registry became a
/// **layout failure** instead: the shipped `examples/control-button-graft` reported all three of its out-of-tree
/// implementations as "the build can never compile it" while that crate builds and grafts correctly. The marker that
/// survived the migration is the `source` field, which exists only on the external form.
/// 分类器过去只认宏名（`external_object!` / `__external_object!`）。外部形式迁移到原生 `#[soil] … .source("…")` 拼法
/// 之后，一个声明了自己注册机的面反而变成**布局失败**：出厂的 `examples/control-button-graft` 把它三个树外实现全都报成
/// "构建永远编译不了它"，而那个 crate 能构建、也能正确嫁接。迁移之后仍然活着的标记是 `source` 字段，而它只存在于外部
/// 形式上。
#[test]
fn the_native_spelling_of_the_external_form_is_external_too() {
    let root = temporary_root("external-native");
    fs::write(
        root.join("src/button_fast.rs"),
        "#[xirang_toolchain::soil]\npub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<ButtonFast>()\n    .source(\"button_fast/button_fast.rs\")\n    .under(root_node_id(XIRANG_NAMESPACE));\n\npub struct ButtonFast;\n",
    )
    .expect("external face");
    let (views, unreadable, external) =
        face_views_with_external(&root, "graft-host").expect("an answer");
    assert!(
        views.is_empty() && unreadable.is_empty(),
        "the native external form is neither a host face nor a layout failure: {views:?} {unreadable:?}"
    );
    assert_eq!(external.len(), 1, "{external:?}");
    assert!(
        external[0].contains("button_fast.rs") && external[0].contains("external face"),
        "it is reported the same way as the literal spelling: {external:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// An `external_object!` face is a face of its own registry, not an unreadable file: the
/// generated tree deliberately does not contain it, and saying "the build can never compile
/// it" about a working example was false. The two lists stay separate so a reader can tell a
/// broken crate from a deliberately out-of-tree one.
/// `external_object!` 写的面是它自己注册机的面，不是读不了的文件：生成树有意不含它，而对着一个能工作的
/// 示例说"构建永远编译不了它"是假话。两张清单保持分开，读者才能分清"坏掉的 crate"与"有意在树外的 crate"。
#[test]
fn an_external_face_is_reported_as_external_rather_than_unparsable() {
    let root = temporary_root("external");
    fs::write(
        root.join("src/button_fast.rs"),
        "xirang_toolchain::run_method::external_object! {\n    source: \"button_fast/button_fast.rs\",\n    kind: ButtonFast,\n}\n",
    )
    .expect("external face");
    let (views, unreadable, external) =
        face_views_with_external(&root, "graft-host").expect("an answer");
    assert!(
        views.is_empty(),
        "not part of the generated tree: {views:?}"
    );
    assert!(
        unreadable.is_empty(),
        "an external face is not an unreadable one: {unreadable:?}"
    );
    assert_eq!(external.len(), 1, "{external:?}");
    assert!(external[0].contains("button_fast.rs"), "{external:?}");
    assert!(external[0].contains("external face"), "{external:?}");
    // The older entry point keeps its promise: it reports the unreadable half, and the
    // external face is not in it.
    // 旧的入口保持它的承诺：它报的是读不了的那一半，而外部面不在其中。
    let (_, only_unreadable) = face_views_and_unreadable(&root, "graft-host").expect("an answer");
    assert!(only_unreadable.is_empty(), "{only_unreadable:?}");
}

/// A generated-layout face that sits outside the layout is still a layout problem.
/// 本该在生成布局里、却长在布局之外的面仍然是布局问题。
#[test]
fn a_misplaced_generated_face_is_still_a_layout_problem() {
    let root = temporary_root("misplaced");
    fs::write(
        root.join("src/panel.rs"),
        "crate::root_object! {\n    kind: Panel,\n}\n",
    )
    .expect("misplaced face");
    let (_, unreadable, external) = face_views_with_external(&root, "host").expect("an answer");
    assert_eq!(unreadable.len(), 1, "{unreadable:?}");
    assert!(unreadable[0].contains("outside the"), "{unreadable:?}");
    assert!(external.is_empty(), "{external:?}");
}

/// A C++ declaration is a face of the same tree: the structural operations must see it, not just
/// the Rust half. Without this merge, move/graft/crate cannot even refuse a subtree containing it.
/// C++ 声明是同一棵树的面：结构操作必须看见它，而不只是 Rust 那一半。没有合并，move/graft/crate 连
/// "拒绝包含它的子树"都做不到。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_cpp_declaration_appears_in_the_same_view() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let views = face_views(&root, "shop").expect("the mixed example is viewable");
    let area = views
        .iter()
        .find(|view| view.source.ends_with("area.cpp"))
        .unwrap_or_else(|| panic!("the C++ Area face is missing: {views:?}"));
    assert_eq!(area.kind, "Area");
    assert_eq!(area.path, "root/shop/panel/Area");
    // The record's own base is data on the view: a foreign declaration is package-relative, and the
    // shared fact is the only thing a consumer joins. Nothing probes `src/` for a copy.
    // 记录自身的基准是视图上的数据：外语声明相对包根，而消费者只拼接共享事实。没有任何东西去
    // `src/` 里探一份副本。
    let layout = crate::build_method::source_layout(&root).expect("layout");
    assert_eq!(
        area.source_base,
        crate::build_method::source_ref::SourceBase::Package
    );
    let fact = area.source_ref(&layout);
    assert_eq!(fact.source, "native/area/area.cpp");
    assert_eq!(fact.object, area.id);
    let located = area
        .locate_source(&layout)
        .expect("the declared source exists");
    assert!(
        located.ends_with("native/area/area.cpp"),
        "{}",
        located.display()
    );
}

/// A **Rust** face under a foreign parent names it through the generated `xirang_external::<ident>`
/// alias; the view must resolve that alias to the foreign face, or one project answers with two
/// logical paths (the runtime's `root/shop/panel/Area/reading` and the derivation's `root/reading`).
/// **Rust** 面挂在外部父级下时用生成的 `xirang_external::<ident>` 别名点名它；视图必须把那个别名解析到
/// 外部面，否则同一个工程会给出两条逻辑路径（运行期的 `root/shop/panel/Area/reading` 与推导的
/// `root/reading`）。
///
/// Gated on `lang-cpp` because the fixture's `Area` is a C++ declaration: without the adapter the
/// view legitimately has no such face, and the test would be asserting the absence of a feature this
/// build was not asked for.
/// 门控在 `lang-cpp` 上，因为夹具里的 `Area` 是一条 C++ 声明：没有该适配器时视图本来就没有那个面，而
/// 这条测试会变成在断言"本构建没被要求的能力不存在"。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_rust_face_under_a_foreign_parent_resolves_through_the_generated_alias() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let views = face_views(&root, "shop").expect("the mixed example is viewable");
    let area = views
        .iter()
        .find(|view| view.kind == "Area")
        .unwrap_or_else(|| panic!("the C++ Area face is missing: {views:?}"));
    let reading = views
        .iter()
        .find(|view| view.kind == "Reading")
        .unwrap_or_else(|| panic!("the Rust Reading face is missing: {views:?}"));
    assert!(
        reading.parent_resolved,
        "the external alias must resolve, not fall back to `root/<name>`: {reading:?}"
    );
    assert_eq!(
        reading.parent, area.id,
        "the Rust face's parent is the C++ face it names"
    );
    assert_eq!(
        reading.path, "root/shop/panel/Area/reading",
        "and its logical path is the runtime tree's, not the fallback"
    );
}

/// The shared fact, not file existence, chooses between two readable candidates.
/// 共享事实——而不是文件是否存在——在两个都可读的候选之间做选择。
#[test]
fn the_declared_base_beats_a_same_named_copy_elsewhere() {
    let root = temporary_root("base");
    let button = root.join("src/control/object/button");
    fs::create_dir_all(&button).expect("button directory");
    fs::write(
        button.join("button.rs"),
        "crate::root_object! {\n    kind: Button,\n}\n",
    )
    .expect("button face");
    // A file at the package-relative spelling exists too. The Rust record's base is the identity
    // path, so this copy must never be offered as the source.
    // 包内相对拼写处也存在一个文件。Rust 记录的基准是身份路径，因此这份副本绝不能被当作源码。
    let decoy = root.join("control/object/button");
    fs::create_dir_all(&decoy).expect("decoy directory");
    fs::write(
        decoy.join("button.rs"),
        "crate::root_object! { kind: Decoy }\n",
    )
    .expect("decoy");

    let views = face_views(&root, "host").expect("faces");
    let view = views
        .iter()
        .find(|view| view.kind == "Button")
        .expect("button");
    let layout = crate::build_method::source_layout(&root).expect("layout");
    assert_eq!(
        view.source_base,
        crate::build_method::source_ref::SourceBase::Identity
    );
    let fact = view.source_ref(&layout);
    assert_eq!(fact.source, "src/control/object/button/button.rs");
    let located = view
        .locate_source(&layout)
        .expect("the identity source exists");
    assert_eq!(located, root.join("src/control/object/button/button.rs"));
    // Missing the named file is an error even though the decoy still exists.
    // 点名文件缺失是错误，即使诱饵仍然存在。
    fs::remove_file(root.join("src/control/object/button/button.rs")).expect("remove source");
    let error = view
        .locate_source(&layout)
        .expect_err("no fallback to the decoy");
    assert!(error.contains("no other location is tried"), "{error}");
}

/// Copy a fixture tree, so two absolute roots hold the same declarations.
/// 复制夹具树，使两个绝对根持有同一批声明。
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to).expect("copy destination");
    for entry in fs::read_dir(from).expect("fixture tree") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("fixture file");
        }
    }
}

/// The same tree at another absolute path keeps every identity and relation.
/// 同一棵树换一个绝对路径，每一份身份与关系都不变。
#[test]
fn the_same_tree_at_another_absolute_path_keeps_every_identity() {
    let first_root = temporary_root("relocated-first");
    fs::create_dir_all(first_root.join("src/control/object/button")).expect("fixture dirs");
    fs::write(
        first_root.join("src/control/object/object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("parent face");
    fs::write(
        first_root.join("src/control/object/button/button.rs"),
        "crate::root_object! {\n    kind: Button,\n    parent: crate::control::object::NODE_ID,\n}\n",
    )
    .expect("child face");

    let second_root = temporary_root("relocated-second");
    copy_tree(&first_root, &second_root);
    assert_ne!(first_root, second_root);

    let first = face_views(&first_root, "host").expect("the original tree reads");
    let second = face_views(&second_root, "host").expect("the relocated tree reads");
    assert_eq!(first.len(), 2);
    // `FaceView` carries ids, parents and logical paths and no absolute path, so equality is the
    // claim: a clone of the tree is the same tree to every consumer.
    // `FaceView` 带 id、父级与逻辑路径，且不含绝对路径，因此相等本身就是断言：树的克隆对每个消费者
    // 都是同一棵树。
    assert_eq!(first, second);

    let first_layout = crate::build_method::source_layout(&first_root).expect("layout");
    let second_layout = crate::build_method::source_layout(&second_root).expect("layout");
    for (original, relocated) in first.iter().zip(second.iter()) {
        let a = original
            .locate_source(&first_layout)
            .expect("source under the original root");
        let b = relocated
            .locate_source(&second_layout)
            .expect("source under the relocated root");
        assert!(a.starts_with(&first_root), "{}", a.display());
        assert!(b.starts_with(&second_root), "{}", b.display());
        assert_eq!(
            a.strip_prefix(&first_root).expect("relative"),
            b.strip_prefix(&second_root).expect("relative")
        );
    }
}

/// Identity is the object, not the bytes: the same source in two materialization roots keeps one id
/// while each root reads its own content.
/// 身份是对象而不是字节：同一源码落在两个物化根下仍是同一个 id，而每个根读到自己那份内容。
#[test]
fn the_same_source_in_two_materialization_roots_keeps_one_identity() {
    let first_root = temporary_root("materialized-first");
    let second_root = temporary_root("materialized-second");
    let relative = "src/control/object/button/button.rs";
    for (root, extra) in [(&first_root, ""), (&second_root, "// this root's copy\n")] {
        fs::create_dir_all(root.join("src/control/object/button")).expect("fixture dirs");
        fs::write(
            root.join(relative),
            format!("crate::root_object! {{\n    kind: Button,\n}}\n{extra}"),
        )
        .expect("face file");
    }

    let first = face_views(&first_root, "host").expect("first root reads");
    let second = face_views(&second_root, "host").expect("second root reads");
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].id, second[0].id, "identity follows the object");
    assert_eq!(first[0].path, second[0].path);

    let first_layout = crate::build_method::source_layout(&first_root).expect("layout");
    let second_layout = crate::build_method::source_layout(&second_root).expect("layout");
    let a = first[0].locate_source(&first_layout).expect("first source");
    let b = second[0]
        .locate_source(&second_layout)
        .expect("second source");
    assert_ne!(a, b, "each root resolves its own materialization");
    assert!(
        fs::read_to_string(&a).expect("first bytes").len()
            < fs::read_to_string(&b).expect("second bytes").len(),
        "the bytes are allowed to differ; only the object identity is fixed"
    );
}

/// A Python declaration is extracted but reported as not registered, never silently dropped.
/// Python 声明会被提取，但报成"未登记"，绝不静默丢弃。
#[cfg(feature = "lang-python")]
#[test]
fn a_python_declaration_is_reported_as_not_registered_yet() {
    let root = temporary_root("python-face");
    let face = root.join("src/control/object");
    fs::create_dir_all(&face).expect("face directory");
    fs::write(
        face.join("object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("rust face");
    let python = root.join("widgets");
    fs::create_dir_all(&python).expect("python directory");
    fs::write(
        python.join("gauge.py"),
        "@soil.face(under=faces.control.object.NODE_ID, named=\"Gauge\")\nclass Gauge:\n    pass\n",
    )
    .expect("python face");

    let (views, _, external) =
        face_views_with_external(&root, "host").expect("the package is viewable");
    assert_eq!(
        views.len(),
        1,
        "only the Rust face is registered: {views:?}"
    );
    assert!(
        external
            .iter()
            .any(|line| line.contains("python face `Gauge`") && line.contains("widgets/gauge.py")),
        "the declared Python face is reported, not dropped: {external:?}"
    );
}

/// A JavaScript declaration is extracted but reported as not registered, never silently dropped.
/// JavaScript 声明会被提取，但报成"未登记"，绝不静默丢弃。
#[cfg(feature = "lang-js")]
#[test]
fn a_javascript_declaration_is_reported_as_not_registered_yet() {
    let root = temporary_root("javascript-face");
    let face = root.join("src/control/object");
    fs::create_dir_all(&face).expect("face directory");
    fs::write(
        face.join("object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("rust face");
    let scripted = root.join("widgets");
    fs::create_dir_all(&scripted).expect("javascript directory");
    fs::write(
        scripted.join("gauge.js"),
        "import { faces, soil } from \"xirang\";\n\n\
         export class Gauge {}\n\n\
         soil.face({\n  under: faces.control.object.NODE_ID,\n  named: \"Gauge\",\n});\n",
    )
    .expect("javascript face");

    let (views, _, external) =
        face_views_with_external(&root, "host").expect("the package is viewable");
    assert_eq!(
        views.len(),
        1,
        "only the Rust face is registered: {views:?}"
    );
    assert!(
        external.iter().any(|line| {
            line.contains("javascript face `Gauge`") && line.contains("widgets/gauge.js")
        }),
        "the declared JavaScript face is reported, not dropped: {external:?}"
    );
    let _ = fs::remove_dir_all(root);
}

/// A TypeScript declaration is extracted but reported as not registered, never silently dropped.
/// TypeScript 声明会被提取，但报成"未登记"，绝不静默丢弃。
#[cfg(feature = "lang-ts")]
#[test]
fn a_typescript_declaration_is_reported_as_not_registered_yet() {
    let root = temporary_root("typescript-face");
    let face = root.join("src/control/object");
    fs::create_dir_all(&face).expect("face directory");
    fs::write(
        face.join("object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("rust face");
    let scripted = root.join("widgets");
    fs::create_dir_all(&scripted).expect("typescript directory");
    fs::write(
        scripted.join("gauge.ts"),
        "import { faces, soil } from \"xirang\";\n\n\
         export class Gauge {\n    reading(): number {\n        return 42;\n    }\n}\n\n\
         soil.face({\n  under: faces.control.object.NODE_ID,\n  named: \"Gauge\",\n});\n",
    )
    .expect("typescript face");

    let (views, _, external) =
        face_views_with_external(&root, "host").expect("the package is viewable");
    assert_eq!(
        views.len(),
        1,
        "only the Rust face is registered: {views:?}"
    );
    assert!(
        external.iter().any(|line| {
            line.contains("typescript face `Gauge`") && line.contains("widgets/gauge.ts")
        }),
        "the declared TypeScript face is reported, not dropped: {external:?}"
    );
    let _ = fs::remove_dir_all(root);
}

/// A declaration extracted from a **declared** source names that source, so a reader can tell a
/// package file from one the project materialized out of `xirang.sources.tsv`. The workspace is
/// implicit and stays unspoken.
/// 从**已声明**来源提取出的声明会点名那个来源，读者因此能区分包内文件与项目从 `xirang.sources.tsv`
/// 物化来的文件。工作区是隐式的，不额外说出来。
#[cfg(feature = "lang-python")]
#[test]
fn a_declaration_in_a_declared_source_names_its_origin() {
    use xirang_multilang::source_map::{SOURCES_FILE, SOURCES_HEADER};

    let root = temporary_root("python-origin");
    let face = root.join("src/control/object");
    fs::create_dir_all(&face).expect("face directory");
    fs::write(
        face.join("object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("rust face");
    let vendor = root.join("vendor/ui");
    fs::create_dir_all(&vendor).expect("declared source");
    fs::write(
        vendor.join("gauge.py"),
        "@soil.face(under=faces.control.object.NODE_ID, named=\"Gauge\")\nclass Gauge:\n    pass\n",
    )
    .expect("python face");
    fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
    )
    .expect("source declaration");

    let (_, _, external) =
        face_views_with_external(&root, "host").expect("the package is viewable");
    assert!(
        external.iter().any(|line| {
            line.contains("python face `Gauge`") && line.contains("from source `vendor-ui`")
        }),
        "the declaration names where it came from: {external:?}"
    );
    let _ = fs::remove_dir_all(root);
}

/// **The author-owned stable name is read in both spellings this tree accepts.**
/// **作者拥有的稳定名在这棵树接受的两种拼法下都会被读到。**
///
/// The field used to be read with `path`, which parses a string literal as a type path, fails, and
/// returns `None` — silently. Every view then derived a path-based identity for a face whose author
/// had chosen an identity, so the build and the view disagreed about which object a declaration is.
/// The two spellings are pinned together because fixing one and leaving the other is the same defect
/// half-fixed.
/// 这个字段过去用 `path` 读：它把字符串字面量当类型路径解析、失败并返回 `None`——而且是**静默**的。
/// 于是作者已选好身份的面，视图仍按路径推导，构建与视图对"这条声明是哪个对象"给出不同答案。两种拼法
/// 一起钉住，因为只修一边就是同一缺陷只修一半。
#[test]
fn a_stable_name_is_read_in_both_declaration_spellings() {
    use xirang_kernel::identity::StableFaceId;

    let root = temporary_root("stable-name");
    let native = root.join("src/native");
    fs::create_dir_all(&native).expect("native directory");
    fs::write(
        native.join("native.rs"),
        "#[xirang_toolchain::soil]\npub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Native>()\n    .stable_name(\"fixture.native\")\n    .under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n",
    )
    .expect("native face");
    let literal = root.join("src/literal");
    fs::create_dir_all(&literal).expect("literal directory");
    fs::write(
        literal.join("literal.rs"),
        "crate::root_object! {\n    kind: Literal,\n    stable_name: Some(\"fixture.literal\"),\n}\n",
    )
    .expect("literal face");
    let views = face_views(&root, "fixture").expect("the tree reads");
    let mut names = views
        .iter()
        .map(|view| (view.stable_name.clone(), view.object_id("fixture")))
        .collect::<Vec<_>>();
    names.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        names,
        vec![
            (
                Some("fixture.literal".to_owned()),
                StableFaceId::from_name("fixture.literal")
            ),
            (
                Some("fixture.native".to_owned()),
                StableFaceId::from_name("fixture.native")
            ),
        ],
        "a declaration that named its object keeps that identity in the view"
    );
    let _ = fs::remove_dir_all(root);
}

/// A face declared through a build-time record file is part of the read-only view.
/// 经构建期记录文件声明的面属于只读视图。
#[cfg(all(feature = "mcp", feature = "native-record"))]
#[test]
fn a_record_file_face_is_part_of_the_read_only_view() {
    let root = temporary_root("record-face");
    fs::create_dir_all(root.join("src/panel")).expect("panel");
    fs::write(
        root.join("src/panel/panel.rs"),
        "crate::root_object! {\n    kind: Panel,\n    needs_registry: true,\n}\n",
    )
    .expect("the parent face");
    let faces = face_views(&root, "host").expect("faces");
    let parent = faces
        .iter()
        .find(|face| face.kind == "Panel")
        .expect("the parent face");
    fs::write(
        root.join("xirang.toml"),
        "[record]\nfile = \"native/gauge.faces\"\n",
    )
    .expect("the manifest");
    fs::create_dir_all(root.join("native")).expect("native");
    let mut bytes = vec![0u8; 1228];
    bytes[..4].copy_from_slice(b"XFRC");
    bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
    bytes[8..10].copy_from_slice(&1228u16.to_le_bytes());
    bytes[12..28].copy_from_slice(&parent.id.into_bytes());
    bytes[28..33].copy_from_slice(b"Gauge");
    bytes[140..145].copy_from_slice(b"gauge");
    bytes[1100..1123].copy_from_slice(b"native/gauge/Gauge.java");
    fs::write(root.join("native/gauge.faces"), bytes).expect("the record");

    let (record_faces, note) = super::record_face_views(&root, "host", &faces)
        .expect("record views")
        .expect("the package declares a record file");
    assert_eq!(record_faces.len(), 1);
    assert_eq!(record_faces[0].kind, "Gauge");
    assert_eq!(record_faces[0].path, format!("{}/Gauge", parent.path));
    assert!(record_faces[0].parent_resolved);
    assert_eq!(record_faces[0].source, "native/gauge/Gauge.java");
    assert!(note.contains("record faces 1"), "{note}");
    let _ = fs::remove_dir_all(&root);
}

/// A declared record file that the build has not written yet is a sentence, not a failure.
/// 声明了但构建还没写出的记录文件是一句话，不是失败。
#[cfg(all(feature = "mcp", feature = "native-record"))]
#[test]
fn a_missing_record_file_is_reported_without_failing_the_view() {
    let root = temporary_root("record-missing");
    fs::write(
        root.join("xirang.toml"),
        "[record]\nfile = \"native/missing.faces\"\n",
    )
    .expect("the manifest");
    let (faces, note) = super::record_face_views(&root, "host", &[])
        .expect("a missing file is not a read failure")
        .expect("the package declares a record file");
    assert!(faces.is_empty());
    assert!(note.contains("record faces unavailable"), "{note}");
    assert!(note.contains("run the build"), "{note}");
    let _ = fs::remove_dir_all(&root);
}
