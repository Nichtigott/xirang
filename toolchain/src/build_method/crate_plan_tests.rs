//! Pins for planning a host's ghost crates: what gets mounted, and what is refused by name.
//! 规划宿主幽灵 crate 的钉子：挂载什么，以及什么被点名拒绝（审计 `M7`，P3.2）。

use std::fs;
use std::path::{Path, PathBuf};

use super::{PlannedCrate, plan};
use crate::build_method::shape_decl::read_shape_declaration;

/// A throwaway host package: `src/` files as written below, plus a declaration.
/// 一个一次性宿主包：下面写下的 `src/` 文件，外加一份声明。
// Scratch lives under the package's `target/`, not under `/tmp`: this repository has lost whole
// fixture sets to a reboot or a cleaner twice, and a mid-run wipe shows up as a *different* test
// failing each time (which is exactly how this suite flaked in the ten-face runner).
// 临时目录放在包的 `target/` 之下，而不是 `/tmp`：本仓已经因为重启或清理**两次**丢掉整套夹具，而运行中途被
// 清掉的表现是"每次红的是另一条测试"——这套测试在十面 runner 里正是这样 flake 的。
fn host(label: &str, files: &[(&str, &str)], declaration: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let parent = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-plan-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&parent);
    let root = parent.join("host");
    fs::create_dir_all(root.join("src")).expect("src");
    for (relative, text) in files {
        let path = root.join("src").join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("face directory");
        fs::write(&path, text).expect("face");
    }
    fs::write(root.join("add_crates.rs"), declaration).expect("the declaration");
    // The ghost's manifest is built from the host's, so the fixture needs one — and its dependency
    // section is what the copy has to carry verbatim.
    // 幽灵的清单由宿主的清单生成，因此夹具需要一份——而它的依赖节正是照抄要带走的东西。
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"host\"\nversion = \"9.9.9\"\nedition = \"2024\"\n\n\
         [dependencies]\nxirang-toolchain = { path = \"../toolchain\", features = [\"run\"] }\n\n\
         [build-dependencies]\nxirang-toolchain = { path = \"../toolchain\" }\n",
    )
    .expect("the host manifest");
    root
}

/// Plan the declaration the host wrote, through the real rows.
/// 用真实的行规划宿主写下的声明。
fn plan_host(root: &Path) -> Result<Vec<PlannedCrate>, String> {
    let src = root.join("src");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out");
    let declaration = read_shape_declaration(root)
        .expect("it reads")
        .expect("it declares a shape");
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let rows = crate::build_method::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    // The pipeline hands the planner its face list as `(source, module)` pairs, computed before the
    // render; the fixture derives the same pairs from the rows the build published.
    // 管线把面清单以 `(源码, 模块)` 对交给规划器（在渲染之前算好）；夹具从构建发布的行里推导出同样的对。
    let faces: Vec<(
        String,
        String,
        crate::build_method::registry_identity::NodeId,
    )> = rows
        .iter()
        .map(|row| {
            (
                row.source.clone(),
                crate::build_method::static_plan::source_module_path(&row.source),
                row.id,
            )
        })
        .collect();
    plan(root, "myapp", &declaration, &faces, &[])
}

/// A declaration for one crate claiming `control::object`.
/// 一份声明：一个 crate 认领 `control::object`。
const DECLARATION: &str = r#"use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[crate::control::object::SUBTREE])],
};
"#;

/// A self-contained fragment plans into one crate, one mount, and the remap that keeps `file!()`
/// reading as the host's own source spelling.
/// 一个自足的碎片规划成一个 crate、一次挂载，以及让 `file!()` 读起来就是宿主源码拼写的那对映射。
#[test]
fn a_self_contained_fragment_plans_a_mount_and_its_remap() {
    let root = host(
        "self-contained",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n",
            ),
        ],
        DECLARATION,
    );
    let planned = plan_host(&root).expect("the fragment is self-contained");
    assert_eq!(planned.len(), 1, "one declared crate");
    let widgets = &planned[0];
    assert_eq!(widgets.name, "widgets");
    assert_eq!(widgets.package, "myapp-widgets");
    assert_eq!(
        widgets.directory,
        root.join("crates").join("myapp-widgets"),
        "a standalone host is its own workspace root, so its ghosts sit under it in `crates/`"
    );
    assert_eq!(
        widgets.namespace, "myapp",
        "the ghost declares the host's namespace"
    );
    assert_eq!(
        widgets.mounts.len(),
        1,
        "only the faces below the claimed subtree: {:?}",
        widgets.mounts
    );
    let mount = &widgets.mounts[0];
    assert_eq!(mount.module_path, "control::object::button");
    assert_eq!(mount.source, "control/object/button/button.rs");

    // The invariant that makes the remap derivable rather than guessed: the spelling is exactly the
    // prefix plus the host-relative source, so remapping the prefix to nothing leaves `file!()`
    // reading as the source the host's records name.
    // 让映射**可推导**而不是靠猜的那条不变量：拼写恰好是前缀加宿主相对源码，因此把前缀映射为空之后，
    // `file!()` 读起来就是宿主记录点名的那个源码。
    assert_eq!(widgets.remap.len(), 1, "{:?}", widgets.remap);
    let (from, to) = &widgets.remap[0];
    assert_eq!(to, "", "the prefix maps to nothing");
    assert_eq!(
        mount.spelling,
        format!("{from}{}", mount.source),
        "the spelling is the prefix plus the source"
    );
    // The spelling is the host's **absolute** path: the generated tree lives in the build script's
    // `OUT_DIR`, which cargo chooses, so a relative walk from the ghost's own directory cannot resolve.
    // The remap strips the host's `src` prefix back off, which is why identity is unaffected.
    // 拼写是宿主的**绝对**路径：生成树住在构建脚本的 `OUT_DIR`（由 cargo 决定），因此"从幽灵自己目录出发的
    // 相对走法"无法解析。remap 会把宿主的 `src` 前缀摘回去，因此身份不受影响。
    // Through `Path`, not through string prefixes: Windows spells the same path with `\`, and its
    // parser accepts `\` and `/` alike, so this says "absolute, and it ends at the source" on both.
    // 经 `Path` 而不是字符串前缀：Windows 用 `\` 拼同一条路径，而它的解析器同时接受 `\` 与 `/`，因此这里
    // 在两个平台上说的都是"绝对的，且以该源码结尾"。
    let spelling = Path::new(&mount.spelling);
    assert!(
        spelling.is_absolute() && spelling.ends_with("src/control/object/button/button.rs"),
        "the spelling is the host's absolute source path: {}",
        mount.spelling
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A face that reaches outside the fragment is refused **by name**, with the line, the reason and the
/// two ways forward — because a ghost mounts only the fragment and has no module above it.
/// 够到碎片之外的引用会被**点名**拒绝，带上行号、原因与两条出路——因为幽灵只挂载碎片，它上面没有模块。
#[test]
fn a_face_reaching_outside_the_fragment_is_refused_by_name() {
    let root = host(
        "reaching-out",
        &[
            (
                "control/control.rs",
                "pub trait ControlHandle {}\n\ncrate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "use crate::control::ControlHandle;\n\ncrate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let refused = plan_host(&root).expect_err("refused");
    assert!(
        refused.contains("control/object/button/button.rs:1"),
        "the refusal names the file and line: {refused}"
    );
    assert!(
        refused.contains("`crate::control::ControlHandle`"),
        "and the whole path that would not resolve: {refused}"
    );
    assert!(
        !refused.contains("::`,"),
        "no path is printed with a dangling `::`: {refused}"
    );
    assert!(
        refused.contains("way forward") && refused.contains("larger subtree"),
        "and the two ways forward: {refused}"
    );
    // A reference the ghost *can* resolve — its own subtree, or the constant the macros read — is not
    // a refusal, which the self-contained pin above shows from the other side.
    // 幽灵**解析得到**的引用——它自己的子树，或宏读的那个常量——不是拒绝的理由；上面那条自足的钉子从另一侧
    // 展示了这一点。
    let _ = std::fs::remove_dir_all(&root);
}

/// A fragment written the way the scaffold writes faces plans cleanly: the kernel names a face
/// imports from the crate root are names **every** generated tree has, so they are not "outside".
/// 按脚手架写法的碎片能干净地规划出来：注册面从 crate 根导入的内核名字是**每个**生成树都有的名字，因此不算
/// "在外面"。
///
/// Measured before this pin existed: a host scaffolded by `new_project` could not be partitioned at
/// all, because every generated face opens with `use crate::{NoParts, NoPreset};` and every
/// registration-rule file with `use crate::RegistrationRule;` — while the ghost resolves all three
/// (its preamble globs the kernel's registry vocabulary in exactly like the host's), so the refusal
/// was a false "would not compile" (audit `M7`, §M7.39).
/// 这条钉子出现之前实测：`new_project` 建出来的宿主**完全无法分区**，因为每个生成的注册面都以
/// `use crate::{NoParts, NoPreset};` 开头、每个注册规范文件都以 `use crate::RegistrationRule;` 开头——
/// 而幽灵三者都解析得到（它的前言与宿主一样把内核的注册词汇 glob 了进来），所以那句拒绝是一句假的"编译不过"
/// （审计 `M7`，§M7.39）。
#[test]
fn the_scaffolds_own_imports_do_not_block_a_partition() {
    let root = host(
        "scaffold-imports",
        &[
            (
                "panel/panel.rs",
                "crate::root_object! {\n    kind: Panel,\n    needs_registry: true,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n",
            ),
            (
                // The shape `promote` lands adds `crate::FlowContract` / `crate::ContractId` to a
                // face file, so the fixture carries them too — a rule that only knew the scaffold's
                // three names would refuse our own promoter's output on the next partition.
                // `promote` 落地的形状会给面文件加上 `crate::FlowContract` / `crate::ContractId`，因此夹具也
                // 带着它们——一条只认识脚手架那三个名字的规则，会在下一次分区时拒绝我们自己提升出来的产物。
                "panel/object/frame/frame.rs",
                "use crate::{NoParts, NoPreset};\n\ncrate::panel_object! {\n    kind: Frame,\n    needs_registry: true,\n    parent: crate::panel::NODE_ID,\n    flow: crate::FlowContract::new(crate::ContractId::new(\"panel.render.v1\"), 1, \"PanelInput\", \"PanelFrame\"),\n}\n",
            ),
            (
                "panel/object/frame/registry_rule/registry_rule.rs",
                "use crate::RegistrationRule;\n\npub const REGISTRATION_RULE: RegistrationRule = crate::RegistrationRule::ANY;\n",
            ),
            (
                "panel/object/frame/object/widget/widget.rs",
                "use crate::{NoParts, NoPreset};\n\ncrate::frame_object! {\n    kind: Widget,\n    parent: crate::panel::object::frame::NODE_ID,\n}\n",
            ),
        ],
        SCAFFOLD_DECLARATION,
    );
    let planned = plan_host(&root).expect("the scaffold's own imports are reachable");
    assert_eq!(planned.len(), 1, "one declared crate");
    assert!(
        planned[0]
            .mounts
            .iter()
            .any(|mount| mount.module_path == "panel::object::frame::object::widget"),
        "the claimed subtree is mounted: {:?}",
        planned[0].mounts
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The same, for the scaffold-shaped fixture above.
/// 同上，供上面那个脚手架形状的夹具使用。
const SCAFFOLD_DECLARATION: &str = r#"use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[crate::panel::object::frame::SUBTREE])],
};
"#;

/// One segment is not the same as "not a module": `crate::panel` names a top-level module all the
/// same, and a fragment that claims `control::object` has no `panel` to resolve it against.
/// 一段并不等于"不是模块"：`crate::panel` 照样点名一个顶层模块，而认领 `control::object` 的碎片没有
/// `panel` 可供它解析。
#[test]
fn a_one_segment_module_path_is_still_checked() {
    let root = host(
        "one-segment-module",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "use crate::panel;\n\ncrate::control_object! {\n    kind: Button,\n}\n",
            ),
            (
                "panel/panel.rs",
                "crate::root_object! {\n    kind: Panel,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let refused = plan_host(&root).expect_err("refused");
    assert!(
        refused.contains("reaches `crate::panel`"),
        "a bare top-level module is still a module: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Two subtrees of **one** crate may reference each other: both are mounted, so both resolve.
/// **同一个** crate 的两棵子树之间可以互相引用：两棵都被挂载，因此都解析得到。
#[test]
fn two_subtrees_of_one_crate_may_reference_each_other() {
    let root = host(
        "siblings",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n    parent: crate::panel::NODE_ID,\n}\n",
            ),
            (
                "panel/panel.rs",
                "crate::root_object! {\n    kind: Panel,\n}\n",
            ),
            (
                "panel/gauge/gauge.rs",
                "crate::control_object! {\n    kind: Gauge,\n}\n",
            ),
        ],
        r#"use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[
        crate::control::object::SUBTREE,
        crate::panel::SUBTREE,
    ])],
};
"#,
    );
    let planned = plan_host(&root).expect("a reference between two claimed subtrees resolves");
    assert_eq!(planned.len(), 1);
    // Three, not two: a claim names a subtree **rooted at** its node, so the face at that node is
    // part of it — `panel/panel.rs` belongs to the `panel` claim exactly as the button belongs to
    // `control::object`.
    // 三个而不是两个：认领点名的是**以该节点为根**的子树，因此该节点自己的面也属于它——`panel/panel.rs` 属于
    // `panel` 这条认领，正如 button 属于 `control::object`。
    assert_eq!(
        planned[0].mounts.len(),
        3,
        "both subtrees are mounted, each including its root face: {:?}",
        planned[0].mounts
    );
    // Two inline depths ⇒ two distinct prefixes, each mapping to nothing.
    // 两个不同的内联深度 ⇒ 两个不同的前缀，各自映射为空。
    // One prefix per inline depth, and the depths differ by construction: `control::object::button`
    // sits three modules deep, `panel` one, `panel::gauge` two.
    // 每个内联深度一个前缀，而深度按构造各不相同：`control::object::button` 深三层、`panel` 一层、
    // `panel::gauge` 两层。
    // One prefix, not three: an absolute spelling gives every mount the **same** host `src` prefix,
    // which is the whole point of dropping the per-depth walk (audit `M7`, §M7.37).
    // 一条前缀，而不是三条：绝对拼写让每个挂载都共享**同一个**宿主 `src` 前缀——这正是丢掉"按深度走"的要点
    // （审计 `M7`，§M7.37）。
    assert_eq!(planned[0].remap.len(), 1, "{:?}", planned[0].remap);
    assert!(
        planned[0]
            .remap
            .iter()
            .all(|(from, to)| Path::new(from).ends_with("host/src") && to.is_empty()),
        "every remap walks out of the ghost into the host's src: {:?}",
        planned[0].remap
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The two render modes read one shape differently: a host hands its claims away, a ghost is the crate
/// they were handed **to** and therefore hands nothing away (the end-to-end demonstration is what
/// found this — the plan was right and the render was not).
/// 两种渲染模式对同一份形状的读法不同：宿主把认领交出去，幽灵是它们被交给的 crate、因此什么都不交出去
/// （端到端演示发现了这件事——规划是对的，渲染不是）。
#[test]
fn a_host_hands_its_claims_away_and_a_ghost_hands_nothing_away() {
    let root = host(
        "modes",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let declaration = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares a shape");
    assert_eq!(
        super::cut_out_for(Some(&declaration), true),
        Vec::<String>::new(),
        "a ghost renders exactly the fragment it was handed"
    );
    assert_eq!(
        super::cut_out_for(Some(&declaration), false),
        vec!["control::object".to_owned()],
        "a host renders everything but the subtrees it hands away"
    );
    assert_eq!(
        super::cut_out_for(None, false),
        Vec::<String>::new(),
        "no declaration: the whole tree"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A fragment may name its **ancestor's identity** — the shell carries that one constant so
/// `parent: crate::panel::NODE_ID` still resolves in a crate that does not compile `panel` — and may
/// name nothing else of it: a trait defined above the fragment stays a refusal, which is the honest
/// boundary (audit `M7`, P3.2).
/// 碎片可以点名**祖先的身份**——壳携带那一个常量，好让 `parent: crate::panel::NODE_ID` 在一个不编译 `panel`
/// 的 crate 里仍然解析——而祖先的其它东西一概不能点名：定义在碎片之上的 trait 仍被拒绝，那是诚实的边界
/// （审计 `M7`，P3.2）。
#[test]
fn an_ancestors_identity_is_reachable_and_its_traits_are_not() {
    let declaration = r#"use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[crate::panel::frame::SUBTREE])],
};
"#;
    let files = [
        (
            "panel/panel.rs",
            "crate::root_object! {\n    kind: Panel,\n}\n",
        ),
        (
            "panel/frame/frame.rs",
            "crate::control_object! {\n    kind: Frame,\n    parent: crate::panel::NODE_ID,\n}\n",
        ),
        (
            "panel/frame/widget/widget.rs",
            "crate::control_object! {\n    kind: Widget,\n    parent: crate::panel::frame::NODE_ID,\n}\n",
        ),
    ];
    let root = host("ancestor-identity", &files, declaration);
    let planned = plan_host(&root).expect("an ancestor's identity is reachable");
    assert_eq!(planned.len(), 1);
    assert_eq!(
        planned[0].ancestors.len(),
        1,
        "exactly the faces above the claim: {:?}",
        planned[0].ancestors
    );
    assert_eq!(planned[0].ancestors[0].0, "panel");
    assert_eq!(
        planned[0]
            .mounts
            .iter()
            .map(|mount| mount.module_path.as_str())
            .collect::<Vec<_>>(),
        vec!["panel::frame", "panel::frame::widget"],
        "the fragment includes the face **at** its root, which the host hands away whole"
    );

    // The same fragment reaching for a **trait** above it is refused, by file and line.
    // 同一个碎片若去够它上面的**trait**，则被点名拒绝（文件与行）。
    let root = host(
        "ancestor-trait",
        &[
            (
                "panel/panel.rs",
                "pub trait ControlHandle {}\n\ncrate::root_object! {\n    kind: Panel,\n}\n",
            ),
            (
                "panel/frame/frame.rs",
                "use crate::panel::ControlHandle;\n\ncrate::control_object! {\n    kind: Frame,\n}\n",
            ),
            (
                "panel/frame/widget/widget.rs",
                "crate::control_object! {\n    kind: Widget,\n}\n",
            ),
        ],
        declaration,
    );
    let refused = plan_host(&root).expect_err("a trait above the fragment stays unreachable");
    assert!(
        refused.contains("crate::panel::ControlHandle") && refused.contains("way forward"),
        "the refusal names the path and the way forward: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The three files a ghost is made of, and the config the workspace root has to carry.
/// 幽灵由哪三份文件组成，以及工作区根必须携带的那份配置。
/// A claim naming nothing the build published is refused by name — never a quiet empty crate.
/// 点名不到任何已发布的面时，认领被点名拒绝——绝不变成一个安静的空 crate。
/// One crate may claim **several** subtrees: both are mounted, and the crate is the one they share.
/// 一个 crate 可以认领**多棵**子树：两棵都被挂载，而它们是同一个 crate 的。
#[test]
fn a_crate_may_claim_more_than_one_subtree() {
    let root = host(
        "two-claims",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "panel/panel.rs",
                "crate::root_object! {\n    kind: Panel,\n}\n",
            ),
        ],
        "use xirang_toolchain::run_method::{Crate, Shape};\n\npub fn add_crates() -> Shape {\n    Shape::of(\"myapp\", &[\n        Crate::named(\"widgets\").at(&[crate::control::SUBTREE, crate::panel::SUBTREE]),\n    ])\n}\n",
    );
    // The two claims are one crate's, so the fixture names it twice on purpose: what is asserted is
    // that **every** claim's subtree is mounted, not how many entries the declaration used.
    // 两条认领属于同一个 crate，因此夹具故意把名字写两遍：要断言的是**每条**认领的子树都被挂载，
    // 而不是声明里用了几条条目。
    let planned = plan_host(&root).expect("both claims plan");
    let mounted: Vec<String> = planned
        .iter()
        .flat_map(|crate_planned| crate_planned.mounts.iter())
        .map(|mount| mount.module_path.clone())
        .collect();
    assert!(
        mounted.iter().any(|module| module == "control"),
        "{mounted:?}"
    );
    assert!(
        mounted.iter().any(|module| module == "panel"),
        "{mounted:?}"
    );
    assert_eq!(planned.len(), 1, "and they are one crate, not two");
}

#[test]
fn a_claim_that_matches_no_published_face_is_refused_by_name() {
    let root = host(
        "claim-nothing",
        &[(
            "control/control.rs",
            "crate::root_object! {\n    kind: Control,\n}\n",
        )],
        DECLARATION,
    );
    let error = plan_host(&root).expect_err("it is refused");
    // The refusal names the claim it could not place (the wording lives in one place: the existing
    // "names no module" check fires first, and today's added site below it is a duplicate to remove).
    // 拒绝点名了它放不下的那条认领（文案只有一处：既有的 "names no module" 检查先触发，而今天新加的那处
    // 在它下面，是待删的重复）。
    assert!(
        error.contains("names no module in this host's tree"),
        "it says the claim matched nothing: {error}"
    );
    assert!(
        error.contains("run `xirang check`"),
        "and names the way out: {error}"
    );
}

#[test]
fn a_ghost_is_three_files_and_one_workspace_config() {
    let root = host(
        "artifacts",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let planned = plan_host(&root).expect("the fragment is self-contained");
    let widgets = &planned[0];

    // lib.rs: the host's namespace, and nothing else but the include of its own generated tree.
    // lib.rs：宿主的命名空间，除自己那棵生成树的 include 之外没有别的。
    assert!(
        widgets
            .lib_rs
            .contains("pub const XIRANG_NAMESPACE: &str = \"myapp\";"),
        "the ghost declares the host's namespace: {}",
        widgets.lib_rs
    );
    assert!(
        widgets
            .lib_rs
            .contains("include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));"),
        "and includes its own generated tree: {}",
        widgets.lib_rs
    );

    // build.rs: it reaches the host's **package root** from `CARGO_MANIFEST_DIR` plus a relative
    // walk, the host's namespace, and the fragment as the render mode. The root, not the manifest:
    // the first end-to-end build failed with `<root>/Cargo.toml/src is not a source directory`, which
    // is what passing the manifest does. The absolute path must **not** appear: a fragment is
    // committed, so naming this checkout would bind the split to the machine that generated it.
    // build.rs：它用 `CARGO_MANIFEST_DIR` 加相对走法到达宿主的**包根**，再带宿主命名空间与作为渲染模式的
    // 碎片。是包根而不是清单：第一次端到端构建报出 `<root>/Cargo.toml/src is not a source directory`，
    // 那正是传清单的后果。绝对路径**不**许出现：碎片要入库，点名本检出的写法会把这次拆分绑死在产生它的机器上。
    assert!(
        widgets
            .build_rs
            .contains("std::path::PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"))")
            && !widgets.build_rs.contains(&format!("{:?}", root))
            && !widgets
                .build_rs
                .contains(&format!("{}/Cargo.toml", root.display()))
            && widgets
                .build_rs
                .contains("xirang_toolchain::build_method::run_for"),
        "the ghost reaches the host from CARGO_MANIFEST_DIR instead of naming this checkout: {}",
        widgets.build_rs
    );
    assert!(
        widgets.build_rs.contains("run_for_partition")
            && widgets.build_rs.contains("Some(\"control::object\")"),
        "and asks for the fragment only, as an argument: {}",
        widgets.build_rs
    );
    // A generated build script must not reach for `unsafe`: the claim is a literal the generator
    // already knows, so it travels as an argument. Rust 2024 made `env::set_var` unsafe, and that
    // `unsafe` used to be in every generated build script.
    // 生成的构建脚本不得动用 `unsafe`：认领本就是生成器已知的字面量，以参数传递即可。Rust 2024 把
    // `env::set_var` 变成了 unsafe，而那个 `unsafe` 曾经出现在每一个生成的构建脚本里。
    assert!(
        !widgets.build_rs.contains("unsafe") && !widgets.build_rs.contains("set_var"),
        "and sets no environment variable: {}",
        widgets.build_rs
    );
    assert!(
        widgets.build_rs.contains("\"myapp\""),
        "with the host's namespace: {}",
        widgets.build_rs
    );

    // Cargo.toml: the host's dependencies with their relative paths re-spelled for this package, and
    // no publishing. The host spells `../toolchain`; this ghost sits one level down in `crates/`.
    // Cargo.toml：宿主的依赖按本包所在目录重拼，且不发布。宿主写的是 `../toolchain`；这个幽灵在 `crates/`
    // 下低一层。
    assert!(
        widgets
            .cargo_toml
            .contains("xirang-toolchain = { path = \"../../../toolchain\", features = [\"run\"] }"),
        "the dependencies are the host's: {}",
        widgets.cargo_toml
    );
    assert!(
        widgets.cargo_toml.contains("[build-dependencies]")
            && widgets.cargo_toml.contains("publish = false"),
        "build-dependencies too, and no publishing: {}",
        widgets.cargo_toml
    );

    // The config patch: every remap prefix, and the reason it is the workspace root's file.
    // config 补丁：每一个 remap 前缀，以及它为何属于工作区根那份文件。
    assert!(
        widgets.config_patch.contains("--remap-path-prefix="),
        "the remap entries are spelled out: {}",
        widgets.config_patch
    );
    assert!(
        widgets.config_patch.contains("workspace root"),
        "and say where they belong: {}",
        widgets.config_patch
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The spelling is a **walk between two paths**, not "sibling plus depth": a ghost the author puts
/// somewhere else walks out of its own nesting, through the shared ancestors, and in through the
/// host's path — and the invariant that makes the remap derivable holds there too.
/// 拼写是**两个路径之间的一段路**，而不是"同级 + 深度"：作者把幽灵放在别处时，它要走出自己的嵌套、穿过共同的
/// 祖先、再顺着宿主的路径走进去——而让映射可推导的那条不变量在那里同样成立。
#[test]
fn a_ghost_placed_elsewhere_still_spells_a_walk_to_the_host() {
    let root = host(
        "elsewhere",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let planned = plan_host(&root).expect("the fragment is self-contained");
    let sibling = &planned[0];
    // Where the ghost sits does not enter the spelling at all: it is the host's absolute path, so the
    // rule applied to the host alone spells what the planner spelled for a ghost sitting elsewhere.
    // That is the property that makes `--at` work, and this half of it compares two strings built
    // from the **same** root, so it does not depend on how this platform spells a temp directory.
    // 幽灵坐在哪里完全不进入拼写：拼写是宿主的绝对路径，因此单把宿主交给规则所拼出的，就是规划器为一个坐在别处的
    // 幽灵拼出的东西。这正是 `--at` 能成立的那条性质，而这一半比较的是由**同一个**根拼出的两个字符串，因此不依赖
    // 本平台如何拼写临时目录。
    let sibling_spelling = sibling
        .mounts
        .iter()
        .map(|mount| mount.spelling.clone())
        .next()
        .expect("one mount");
    assert_eq!(
        crate_plan_spelling(&root, "control/object/button/button.rs"),
        sibling_spelling,
        "the ghost's location does not matter"
    );
    // And a canonicalized host still names the same file: macOS temp directories are symlinks
    // (`/var` → `/private/var`) and Windows canonical paths carry a `\\?\` prefix, so the two
    // spellings are not string-equal there — the file they end at is what has to match.
    // 而规范化过的宿主仍然指向同一个文件：macOS 的临时目录是符号链接（`/var` → `/private/var`），Windows 的
    // 规范化路径带 `\\?\` 前缀，因此两处拼写在那里并不逐字相等——要相等的是它们结尾的那个文件。
    let host_dir = root.canonicalize().expect("the host canonicalizes");
    let deeper = crate_plan_spelling(&host_dir, "control/object/button/button.rs");
    assert_eq!(
        Path::new(&deeper).file_name(),
        Path::new(&sibling_spelling).file_name(),
        "canonical or not, it is the same file: {deeper} vs {sibling_spelling}"
    );
    assert!(
        Path::new(&deeper).ends_with("src/control/object/button/button.rs"),
        "and both spell the source below `src`: {deeper}"
    );
}

/// The planner's spelling rule, reached the way the module reaches it.
/// 规划器的拼写规则，按模块自己的方式到达它。
fn crate_plan_spelling(host_dir: &Path, source: &str) -> String {
    super::spelling_for(host_dir, source)
}

/// Two checkouts of the same layout produce the **same** fragment bytes, and neither names a path
/// outside itself: a fragment is committed (`xirang crates --write` is the submit step), so an
/// absolute host path would bind the split to the machine that generated it.
/// 两个布局相同的检出产生**相同**的碎片字节，且都不点名自身之外的路径：碎片要入库
/// （`xirang crates --write` 就是提交那一步），绝对宿主路径会把这次拆分绑死在产生它的机器上。
#[test]
fn a_generated_fragment_does_not_name_the_machine_that_made_it() {
    let files = &[
        (
            "control/control.rs",
            "crate::root_object! {\n    kind: Control,\n}\n",
        ),
        (
            "control/object/button/button.rs",
            "crate::control_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n",
        ),
    ];
    let first = host("portable-first", files, DECLARATION);
    let second = host("portable-second", files, DECLARATION);
    let left = plan_host(&first).expect("the first plans")[0]
        .build_rs
        .clone();
    let right = plan_host(&second).expect("the second plans")[0]
        .build_rs
        .clone();
    assert_eq!(
        left, right,
        "the same layout has to produce the same fragment bytes"
    );
    assert!(
        !left.contains(&first.display().to_string()),
        "the fragment names no absolute host path: {left}"
    );
    assert!(
        left.contains("std::path::PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"))")
            && left.contains("host.pop();"),
        "it walks from CARGO_MANIFEST_DIR without naming a path outside itself: {left}"
    );
}
