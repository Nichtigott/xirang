//! Source-scope derivation tests.
//! 源码范围推导测试。
//!
//! A separate page so the module it tests stays inside the size ratchet: a test
//! module is excluded from it, and this one had grown as large as the code.
//! 独立一页，使被测模块留在尺寸棘轮之内：测试模块不受棘轮约束，而这一份已经长到
//! 与被测代码相当。

use crate::build_method::diagnostics::BuildDiagnostics;
use crate::build_method::source_walk::discover_root;

/// The face files the scope proved live, named relative to `src`.
/// 作用域证明存活的注册面文件，路径相对 `src`。
fn selected_sources(
    src: &std::path::Path,
    nodes: &[super::Node],
    scope: &super::SourceScope,
) -> Vec<String> {
    let roots = scope.roots.as_ref().expect("the fixture narrows");
    let mut sources = super::collect_faces(src, nodes)
        .into_iter()
        .filter(|face| roots.contains(&face.id))
        .map(|face| super::relative_display(src, &face.source))
        .collect::<Vec<_>>();
    sources.sort();
    sources
}

/// A typed cut narrows the scope to the face it names, and a face nobody
/// declared is pruned: declaring the slot is what ships the face.
/// 类型化切口把作用域收窄到它命名的注册面，没人声明的面会被剪掉：
/// 声明槽位才是这个面被发布出来的原因。
#[test]
fn a_typed_cut_narrows_the_scope_to_the_declared_slot() {
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    let src = root.join("src");
    let entry = src.join("lib.rs");
    for module in ["a", "b"] {
        std::fs::create_dir_all(src.join(module)).expect("fixture dir");
    }
    std::fs::write(
        &entry,
        "crate::host!();\n\
             crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::a::NODE_ID) graft(\"a_fast\"),\n\
             );\n",
    )
    .expect("host entry");
    for (module, kind) in [("a", "A"), ("b", "B")] {
        std::fs::write(
            src.join(format!("{module}/{module}.rs")),
            format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
        )
        .expect("face file");
    }
    let nodes = discover_root(&src);
    let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

    assert_eq!(
        selected_sources(&src, &nodes, &scope),
        ["a/a.rs"],
        "only the declared slot stays live"
    );
    assert_eq!(scope.reason, "auto", "the scope was narrowed, not given up");

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// The scope and the generated cut table read the entry the build resolved,
/// not two independently resolved ones.
/// 作用域与生成的切口表读的是构建解析出的同一个入口，而不是各自独立解析出的两个。
///
/// The configured value is passed in explicitly because the value itself is
/// what the two readers must agree on; reading the process environment here
/// would race with every other test in this binary. The fixture makes the two
/// entries name different slots, so a reader that ignores the configured value
/// keeps `beta` live and reports `beta`'s cut, while a reader that follows it
/// keeps `alpha` live and reports `alpha`'s.
/// 被指定的值以显式参数传入，因为两个读取者必须一致的就是这个值；在这里读进程环境
/// 会与同一二进制里的其他测试竞争。夹具让两个入口声明不同的槽位，因此忽略指定值的
/// 读取者会让 `beta` 存活并报告 `beta` 的切口，跟随它的读取者则让 `alpha` 存活并
/// 报告 `alpha` 的切口。
#[test]
fn a_configured_entry_drives_the_scope_and_the_cut_table() {
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-scope-configured-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    let src = root.join("src");
    for module in ["alpha", "beta", "preview"] {
        std::fs::create_dir_all(src.join(module)).expect("fixture dir");
    }
    // The convention entry — the file that calls `host!()` — declares `beta`.
    // 约定入口（调用 `host!()` 的文件）声明 `beta`。
    std::fs::write(
        src.join("lib.rs"),
        "crate::host!();\n\
             crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::beta::NODE_ID) graft(\"beta_fast\"),\n\
             );\n",
    )
    .expect("convention entry");
    // The configured entry declares `alpha` instead.
    // 被指定的入口改为声明 `alpha`。
    std::fs::write(
        src.join("preview/preview.rs"),
        "crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::alpha::NODE_ID) graft(\"alpha_fast\"),\n\
             );\n",
    )
    .expect("configured entry");
    for (module, kind) in [("alpha", "Alpha"), ("beta", "Beta")] {
        std::fs::write(
            src.join(format!("{module}/{module}.rs")),
            format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
        )
        .expect("face file");
    }

    let nodes = discover_root(&src);
    let entry = crate::build_method::entry::resolve_host_entry(
        &src,
        &nodes,
        Some(std::path::PathBuf::from("src/preview/preview.rs")),
    );
    assert_eq!(entry.path(), src.join("preview/preview.rs"));

    let cuts =
        crate::build_method::host_graft_entries(&entry, &mut BuildDiagnostics::default()).enabled;
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!(cuts[0].cut, "crate::alpha::NODE_ID");

    let scope = super::SourceScope::auto_from_entry(&src, &nodes, entry.path());
    assert_eq!(
        selected_sources(&src, &nodes, &scope),
        ["alpha/alpha.rs"],
        "pruning follows the same entry the cut table read"
    );
    assert_eq!(scope.reason, "auto", "the scope was narrowed, not given up");

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A typed cut whose Rust path names no discovered face is an unreadable
/// declaration: the whole tree is kept rather than pruned wrongly.
/// 类型化切口的 Rust 路径指不到任何已发现注册面时，声明就是读不懂的：
/// 保留整棵树，而不是错误裁剪。
#[test]
fn an_unplaceable_typed_cut_keeps_the_whole_tree() {
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-scope-unrecognized-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    let src = root.join("src");
    let entry = src.join("lib.rs");
    std::fs::create_dir_all(src.join("a")).expect("fixture dir");
    std::fs::write(
        &entry,
        "crate::host!();\n\
             crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::missing::NODE_ID) graft(\"a_fast\"),\n\
             );\n",
    )
    .expect("host entry");
    std::fs::write(
        src.join("a/a.rs"),
        "crate::root_object! {\n    kind: A,\n}\n",
    )
    .expect("face file");
    let nodes = discover_root(&src);
    let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

    assert_eq!(scope.roots, None);
    assert_eq!(scope.reason, "graft-typed-cut-unrecognized");

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A scope value the build refuses is a diagnostic, not a panic: `check
/// --json` gets a document and the fallback prunes nothing.
/// 构建拒绝的范围取值是诊断而不是 panic：`check --json` 拿到文档，回退什么都不剪。
#[test]
fn a_refused_scope_value_is_a_diagnostic() {
    let src = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join("xirang-scope-diagnostics");
    let entry = super::HostEntry::Convention(src.join("lib.rs"));
    let cases = [
        (
            "an identity schema the build does not speak",
            "v99:0".to_owned(),
            "identity schema",
        ),
        (
            "a value that is not a node identity",
            "not-a-node-identity".to_owned(),
            "32-digit node identity",
        ),
        (
            "an identity no node in this tree has",
            format!("v{}:{}", super::IDENTITY_SCHEMA, "0".repeat(32)),
            "unknown node identity",
        ),
    ];
    for (case, raw, expected) in cases {
        let mut errors = super::BuildDiagnostics::default();
        let scope = super::SourceScope::from_raw(&raw, &src, &[], &entry, &[], &mut errors);
        assert!(
            errors.iter().any(|diagnostic| {
                diagnostic.phase == "scope" && diagnostic.message.contains(expected)
            }),
            "{case}: {:#?}",
            errors.iter().collect::<Vec<_>>()
        );
        assert!(
            scope.roots.is_none(),
            "{case}: every refusal falls back to the full tree"
        );
    }
}

/// Both spellings normalise to the same `::` module path, so the overlap rule can compare a typed
/// cut with a logical one. Overlap itself is answered by **declaration order** — the later entry wins
/// and the earlier one is dropped, with a hint naming both (the maintainer's ruling; the end-to-end
/// behaviour is pinned by `tools/xirang-graft-matrix`, leg 2).
/// 两种拼写都归一到同一个 `::` 模块路径，因此重叠规则能把类型化切口与逻辑切口放在一起比。重叠本身由
/// **声明顺序**作答——后一条赢、前一条被丢掉，并给一条提示同时点名两者（维护者的裁定；端到端行为由
/// `tools/xirang-graft-matrix` 的 leg 2 钉住）。
#[test]
fn a_cut_names_the_same_subtree_however_it_is_spelled() {
    assert_eq!(
        super::cut_subtree("crate::control::object::button::NODE_ID").as_deref(),
        Some("control::object::button")
    );
    assert_eq!(
        super::cut_subtree("crate::control::object::button::Button::NODE_ID").as_deref(),
        Some("control::object::button")
    );
    assert_eq!(
        super::cut_subtree("root/control/object/button").as_deref(),
        Some("control::object::button")
    );
    assert_eq!(
        super::cut_subtree("crate::control::NODE_ID").as_deref(),
        Some("control")
    );
    // Not a subtree this rule can judge, so it is left to the other checks rather than guessed at.
    assert_eq!(super::cut_subtree("crate::control::SOMETHING_ELSE"), None);
}

/// A `full` cut on the **root** face leaves every face live: the whole tree is the slot it names, so
/// the scope must not narrow to nothing (measured on a real host: the root cut renders, builds, and
/// carries the graft). This is the widest position the matrix has, and the one a reader reaches for
/// when the replacement covers everything.
/// **根面**上的 `full` 切口让每个面都保持存活：它点名的槽位就是整棵树，因此作用域不能收窄到空（真宿主上
/// 实测：根面切口照样渲染、构建、携带 graft）。这是位置矩阵里最宽的一格，也是替换件覆盖一切时读者会写的那个。
#[test]
fn a_full_cut_on_the_root_face_keeps_every_face_live() {
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-scope-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    let src = root.join("src");
    let entry = src.join("lib.rs");
    for module in ["a", "b"] {
        std::fs::create_dir_all(src.join(module)).expect("fixture dir");
    }
    std::fs::write(
        &entry,
        "crate::host!();\n\
             crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::NODE_ID) full graft(\"everything_fast\"),\n\
             );\n",
    )
    .expect("host entry");
    for (module, kind) in [("a", "A"), ("b", "B")] {
        std::fs::write(
            src.join(format!("{module}/{module}.rs")),
            format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
        )
        .expect("face file");
    }
    let nodes = discover_root(&src);
    let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

    // The observable property, not a shape: **no face is lost**. A root cut names the whole tree, so
    // the scope either keeps everything (`roots` is `None` — nothing to narrow) or names every face
    // explicitly. What it must never do is drop one.
    // 断言的是可观察的性质，不是某种形状：**一个面都不丢**。根面切口点名的是整棵树，因此作用域要么保持整棵
    // （`roots` 为 `None`，没有可收窄的东西），要么把每个面都列出来。它绝不能丢掉其中一个。
    let faces = super::collect_faces(&src, &nodes);
    assert_eq!(faces.len(), 2, "the fixture has both faces");
    if let Some(roots) = scope.roots.as_ref() {
        for face in &faces {
            assert!(
                roots.contains(&face.id),
                "a root cut keeps every face live: {}",
                face.module
            );
        }
    }

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A crate claim keeps the faces under it live even when a graft slot narrows the scope elsewhere.
/// crate 认领让它的面保持存活，即使某条嫁接槽位在别处收窄了作用域。
///
/// A claim is a forced-liveness root for the same reason a graft slot is: the crate that claims a
/// subtree exists to compile it. Without this, the narrowing a declared slot performs prunes the
/// claim — and **nobody** compiles it, because the host handed the same subtree away. Measured end to
/// end on the two-claim fixture before this rule existed: the ghost mounted
/// `panel/frame/widget/{widget,cap}.rs` and neither file under `panel/gauge`, which no crate of that
/// shape compiled.
/// 认领与嫁接槽位同理，是强制存活根：认领一棵子树的 crate 存在的意义就是编译它。没有这一条，声明槽位造成的收窄
/// 会剪掉那条认领——而**没有谁**会去编译它，因为宿主把同一棵子树交出去了。本条规则存在之前在双认领夹具上端到端
/// 实测：幽灵挂载了 `panel/frame/widget/{widget,cap}.rs`，而 `panel/gauge` 下的两个文件一个都没挂——那个形状里
/// 没有任何 crate 编译它们。
#[test]
fn a_crate_claim_keeps_its_subtree_live_next_to_a_graft_slot() {
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-scope-claim-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    let src = root.join("src");
    let entry = src.join("lib.rs");
    std::fs::create_dir_all(&src).expect("fixture src");
    // One slot inside `a::b::c`, and a second branch `a::d` that only a crate claim keeps alive.
    // `a::b::c` 里有一条槽位，另一条分支 `a::d` 只有 crate 认领能让它存活。
    std::fs::write(
        &entry,
        "crate::host!();\n\
             crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::a::b::c::NODE_ID) graft(\"c_fast\"),\n\
             );\n",
    )
    .expect("host entry");
    for (path, kind) in [
        ("a/a.rs", "A"),
        ("a/b/b.rs", "B"),
        ("a/b/c/c.rs", "C"),
        ("a/d/d.rs", "D"),
    ] {
        let file = src.join(path);
        std::fs::create_dir_all(file.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &file,
            format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
        )
        .expect("face file");
    }
    let nodes = discover_root(&src);
    let faces = super::collect_faces(&src, &nodes);
    assert_eq!(faces.len(), 4, "the fixture has four faces");

    // Without the claim the slot narrows the scope and `a::d` is pruned — that is the behaviour the
    // claim has to override, and asserting it here is what gives this pin teeth.
    // 没有认领时，槽位收窄作用域、`a::d` 被剪掉——这正是认领要盖过的行为，在这里断言它才让本钉子有牙。
    let without = super::SourceScope::auto_from_entry(&src, &nodes, &entry);
    let live_without = selected_sources(&src, &nodes, &without);
    assert!(
        !live_without.iter().any(|source| source == "a/d/d.rs"),
        "the slot alone prunes the claimed branch: {live_without:?}"
    );

    let claims = vec!["a::d".to_owned()];
    let with = super::SourceScope::auto_from_entry_with_claims(&src, &nodes, &entry, &claims);
    let live_with = selected_sources(&src, &nodes, &with);
    assert!(
        live_with.iter().any(|source| source == "a/d/d.rs"),
        "a claimed subtree stays live: {live_with:?}"
    );
    assert!(
        live_with.iter().any(|source| source == "a/b/c/c.rs"),
        "and the declared slot stays live too: {live_with:?}"
    );

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// **Narrowing is said out loud**: the count, the names, and a clause the reader can paste.
/// **收窄会被说出来**：计数、名字，以及一条读者能直接粘贴的子句。
///
/// Before this existed, a plan with one cut took a six-face host down to three and neither `check` nor
/// the build mentioned it — the answer lived only in `explain --overlay`, which is a different command a
/// reader has no reason to run (audit `M7`, §M7.58).
/// 在这之前，一条切口能把六个面的宿主变成三个，而 `check` 与构建都不提——答案只住在
/// `explain --overlay` 里，那是读者没有理由去跑的**另一条**命令（审计 `M7`，§M7.58）。
#[test]
fn the_faces_a_plan_leaves_outside_are_named_with_a_clause_to_keep_them() {
    let face = |name: &str| super::super::scope_faces::FaceSource {
        id: xirang_kernel::identity::NodeId::from_bytes(name.as_bytes()),
        source: std::path::PathBuf::from(format!("{name}/{name}.rs")),
        module: name.to_owned(),
    };
    let all = vec![face("panel"), face("frame"), face("gauge")];
    let roots: std::collections::BTreeSet<xirang_kernel::identity::NodeId> =
        [all[0].id].into_iter().collect();

    // One declared slot: this is the plan-with-cuts case, and it keeps the sentence that names them.
    // 一个声明槽位：这是"计划带切口"的情形，它保留点名那些切口的那句话。
    let lines = super::outside_slots_note(std::path::Path::new("src/lib.rs"), &all, &roots, 1)
        .expect("two of three faces are outside");
    let said = lines.join("\n");
    assert!(
        said.contains("2 of 3 face(s)") && said.contains("frame") && said.contains("gauge"),
        "the count and the names are the parts a reader acts on: {said}"
    );
    // The reason clause is asserted on **both** sides of the branch: a tree that declares a slot is told
    // about its cuts, and (below) one that declares none is not. Without this half, a fix that always
    // printed the no-slots wording would pass — which is exactly what the first run of this pin's own
    // red check showed.
    // 原因子句在分支的**两侧**都断言：声明了槽位的树被告知它的切口，而（下面）一个都没声明的树不会。少了
    // 这一半，一个永远打印"无槽位"措辞的修复也能通过——这正是这条钉子自己的红检第一次跑出来的结果。
    assert!(
        said.contains("the cuts in") && said.contains("src/lib.rs"),
        "a declared slot ⇒ the entry is named as the place the cuts are: {said}"
    );
    assert!(
        said.contains("cut(crate::frame::NODE_ID) graft(crate::frame::NODE_ID),"),
        "the way forward is a clause, not advice: {said}"
    );
    assert!(
        said.contains("XIRANG_SCOPE=all"),
        "and keeping the whole tree is one variable away: {said}"
    );

    // The negative half: nothing outside means nothing said. A warning that fires on every build which
    // declares a cut is noise, and noise is how a real one gets ignored.
    // 否定的那一半：没有面在外面就什么都不说。一条在每次"声明了切口"的构建上都响的警告是噪音，而噪音正是
    // 真警告被忽略的方式。
    let everything: std::collections::BTreeSet<xirang_kernel::identity::NodeId> =
        all.iter().map(|face| face.id).collect();
    assert!(
        super::outside_slots_note(std::path::Path::new("src/lib.rs"), &all, &everything, 1)
            .is_none(),
        "a build that leaves nothing out says nothing"
    );

    // The other half of the truth, and the one the 2026-10-10 queue recorded (`S8-ours`): a tree whose
    // entry declares **no** slot at all must not be told that "the cuts in <entry>" did the narrowing —
    // measured on a three-line entry with zero `cut(` sites, whose own `check` said so in the same
    // session, while this line attributed a declaration that did not exist to that file (report §7h).
    // The counts, the names and the paste-ready clause stay exactly as they were: what changes is the
    // reason, because the reason is the part that was false.
    // 真相的另一半，也是 2026-10-10 队列记下的那一半（`S8-ours`）：入口**一个槽位都没声明**的树，不该被告知
    // 是"<入口> 里的切口"造成了收窄——实测那次入口只有三行、零个 `cut(` 站点，同一次会话里它自己的 `check`
    // 就这么说，而这一行把一个不存在的声明归因给了那个文件（报告 §7h）。计数、名字与那条可粘贴的子句原样
    // 保留：变的是**原因**，因为原因才是假的那一部分。
    let honest = super::outside_slots_note(std::path::Path::new("src/lib.rs"), &all, &roots, 0)
        .expect("two of three faces are outside")
        .join("\n");
    assert!(
        !honest.contains("the cuts in"),
        "no declared slot ⇒ the entry is not said to carry cuts: {honest}"
    );
    assert!(
        honest.contains("declares no `cut`/`graft` slot") && honest.contains("active subtree"),
        "the reason is the one that holds: {honest}"
    );
    assert!(
        honest.contains("2 of 3 face(s)") && honest.contains("cut(crate::frame::NODE_ID)"),
        "and the actionable half is unchanged: {honest}"
    );
}

/// **The note names what the build really leaves out** — a container kept for a named subtree is
/// published, so it is not one of them.
/// **通知点名的是构建真正漏掉的面**——为某个被点名的子树而留下的容器是发布过的，因此不算在内。
///
/// Measured on `examples/control-button` with one extra face: the notice said
/// `2 of 4 face(s) … control · panel`, while `control` sits in that same build's
/// `BUILTIN_STATIC_FACES` — its cuts name `control::object::button` and `control::object::slider`, and
/// the module chain of those is mounted, so the container ships. A count that includes it tells the
/// reader to declare a slot for something already in, and an over-reporting warning is how a real one
/// stops being read.
/// 在 `examples/control-button` 上加一个面实测：通知说 `2 of 4 face(s) … control · panel`，而 `control`
/// 就在那次构建的 `BUILTIN_STATIC_FACES` 里——它的切口点名 `control::object::button` 与
/// `control::object::slider`，两者的模块链会被挂载，因此那个容器是发布的。把它算进去，等于让读者为一个
/// 已经在里的东西去声明槽位；而一条多报的警告，正是真警告不再被读的方式。
#[test]
fn a_container_kept_for_a_named_subtree_is_not_reported_as_left_out() {
    let face = |module: &str| super::super::scope_faces::FaceSource {
        id: xirang_kernel::identity::NodeId::from_bytes(module.as_bytes()),
        source: std::path::PathBuf::from(format!("{module}.rs")),
        module: module.to_owned(),
    };
    let all = vec![
        face("control"),
        face("control::object::button"),
        face("panel"),
    ];
    let roots: std::collections::BTreeSet<xirang_kernel::identity::NodeId> =
        [all[1].id].into_iter().collect();
    let said = super::outside_slots_note(std::path::Path::new("src/lib.rs"), &all, &roots, 1)
        .expect("`panel` is outside")
        .join("\n");
    assert!(
        said.contains("1 of 3 face(s)"),
        "only the face nobody named is counted: {said}"
    );
    assert!(
        said.contains("panel") && !said.contains("cut(crate::control::NODE_ID)"),
        "the container is not named and gets no clause: {said}"
    );
}
