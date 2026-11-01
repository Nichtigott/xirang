//! Pins for the release shape: self-contained packages, and no remap anywhere (audit `M7`, §M7.42).
//! 发布形状的钉子：自包含的包，以及哪里都不需要 remap（审计 `M7`，§M7.42）。

use std::fs;
use std::path::{Path, PathBuf};

use super::{plan_facade, plan_ghost};
use crate::build_method::crate_plan::{PlannedCrate, PlannedMount};
use xirang_kernel::identity::NodeId;

/// A throwaway host package with the manifest a plan reads.
/// 一个一次性宿主包，带规划要读的清单。
fn host(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-release-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    // A host a facade can depend on has a **library** target: `plan_facade` refuses a binary-only host
    // by name, because cargo ignores such a dependency and the facade would then compile zero faces
    // (audit `M7`, §M7.55). The fixture describes that requirement rather than dodging it.
    // 能被 facade 依赖的宿主有**库**目标：`plan_facade` 会按名拒绝只有二进制的宿主，因为 cargo 会忽略那样
    // 的依赖，而 facade 随后会编译零个面（审计 `M7`，§M7.55）。夹具描述的是这条要求，而不是绕开它。
    fs::create_dir_all(root.join("src")).expect("the host's src");
    fs::write(root.join("src/lib.rs"), "// the host library\n").expect("the host library");
    for file in [
        "src/panel/frame/frame.rs",
        "src/panel/frame/widget/widget.rs",
        "src/panel/frame/registry_rule/registry_rule.rs",
    ] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the host's sources");
        fs::write(&path, "// a face\n").expect("a face file");
    }
    fs::write(
        root.join("add_crates.rs"),
        "pub const SHAPE: Shape = Shape {};\n",
    )
    .expect("the declaration");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"host\"\nversion = \"9.9.9\"\nedition = \"2021\"\n\n\
         [dependencies]\nxirang-toolchain = { path = \"../toolchain\" }\n\
         fast-widget = { path = \"../fast-widget\" }\n\n\
         [build-dependencies]\nxirang-toolchain = { path = \"../toolchain\" }\n",
    )
    .expect("the host manifest");
    root
}

/// One planned ghost, as the planner would have produced it for a two-face fragment.
/// 一个已规划的幽灵，形状与规划器为一个两个面的碎片产出的相同。
/// The faces the plan was computed from, as the CLI hands them to the planner.
/// 规划据以计算的那些注册面，按 CLI 交给规划器的形状。
fn faces() -> Vec<(String, String, NodeId)> {
    vec![
        (
            "panel/frame/frame.rs".to_owned(),
            "panel::frame".to_owned(),
            NodeId::from_bytes(b"frame"),
        ),
        (
            "panel/frame/widget/widget.rs".to_owned(),
            "panel::frame::widget".to_owned(),
            NodeId::from_bytes(b"widget"),
        ),
    ]
}

fn planned(root: &Path) -> PlannedCrate {
    PlannedCrate {
        name: "widgets".to_owned(),
        package: "host-widgets".to_owned(),
        directory: root.parent().expect("a parent").join("host-widgets"),
        namespace: "host".to_owned(),
        subtrees: vec!["panel::frame".to_owned()],
        ancestors: Vec::new(),
        aliases_rs: String::new(),
        mounts: vec![
            PlannedMount {
                module_path: "panel::frame".to_owned(),
                source: "panel/frame/frame.rs".to_owned(),
                spelling: String::new(),
            },
            PlannedMount {
                module_path: "panel::frame::widget".to_owned(),
                source: "panel/frame/widget/widget.rs".to_owned(),
                spelling: String::new(),
            },
        ],
        lib_rs: String::new(),
        build_rs: String::new(),
        cargo_toml: String::new(),
        config_patch: String::new(),
        remap: vec![("unused".to_owned(), String::new())],
    }
}

/// A release ghost carries the fragment's own files, and its manifest says which shape it is.
/// 发布幽灵携带碎片自己的文件，而它的清单说明了它是哪一个形状。
#[test]
fn a_release_ghost_carries_the_files_its_build_reads() {
    let root = host("ghost");
    let ghost = plan_ghost(&root, &planned(&root), &faces(), &[]).expect("the ghost plans");

    // Every mounted face is copied, at the same relative source the host's records name — that path is
    // what makes the copied file's identity the one the host baked, with no remap.
    // 每个被挂载的面都被复制，且用宿主记录点名的同一个相对源码——正是那条路径让被复制的文件保住宿主烤进去的
    // 身份，不需要任何 remap。
    let copies: Vec<&String> = ghost.copies.iter().map(|(_, to)| to).collect();
    assert!(
        copies.contains(&&"src/panel/frame/frame.rs".to_owned()),
        "{copies:?}"
    );
    assert!(
        copies.contains(&&"src/panel/frame/widget/widget.rs".to_owned()),
        "{copies:?}"
    );
    // The declaration comes along: the pipeline needs it to plan the same shells the host's shape did.
    // 声明一起过来：管线需要它来规划与宿主形状相同的壳。
    assert!(copies.contains(&&"add_crates.rs".to_owned()), "{copies:?}");
    for (source, _) in &ghost.copies {
        assert!(
            source.starts_with(&root),
            "every copy comes from the host: {}",
            source.display()
        );
    }

    // The shape fingerprint, the host it was split out of, and **no** `publish = false`.
    // 形状指纹、它是从哪个宿主拆出来的，以及**没有** `publish = false`。
    assert!(ghost.cargo_toml.contains("shape = \"release\""));
    assert!(ghost.cargo_toml.contains("host = \"host\""));
    assert!(!ghost.cargo_toml.contains("publish = false"));
    assert!(
        ghost
            .cargo_toml
            .contains("fast-widget = { path = \"../fast-widget\" }"),
        "the host's dependency table comes verbatim: {}",
        ghost.cargo_toml
    );

    // The build reads **this package's** root and asks for the fragment; nothing points at the host.
    // 构建读的是**本包自己**的根，并只要那个碎片；没有任何东西指向宿主。
    assert!(ghost.build_rs.contains("env!(\"CARGO_MANIFEST_DIR\")"));
    assert!(ghost.build_rs.contains("run_for_partition"));
    assert!(!ghost.build_rs.contains("unsafe") && !ghost.build_rs.contains("set_var"));
    assert!(ghost.build_rs.contains("\"panel::frame\""));
    assert!(
        !ghost.build_rs.contains(&root.display().to_string()),
        "a release build script must not name the host's directory: {}",
        ghost.build_rs
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A release facade carries no sources: its build resolves the host as a dependency, which is the only
/// spelling that survives publishing.
/// 发布 facade 不携带源码：它的构建把宿主当依赖解析，而那是唯一能在发布后成立的拼写。
#[test]
fn a_release_facade_resolves_its_host_instead_of_carrying_it() {
    let root = host("facade");
    let planned = vec![planned(&root)];
    let facade =
        plan_facade(&root, "host", "host", "host", &planned, &[]).expect("the facade plans");

    assert!(
        facade.copies.is_empty(),
        "no sources are duplicated: {:?}",
        facade.copies
    );
    assert!(facade.cargo_toml.contains("shape = \"release\""));
    assert!(!facade.cargo_toml.contains("publish = false"));
    // The sibling dependencies are still spelled relatively, which is what a published manifest turns
    // into a registry requirement — and why the build script cannot read the host from that path.
    // 同级依赖仍用相对路径拼写，发布后的清单会把它变成注册局要求——这也正是构建脚本不能从那个路径读宿主的原因。
    assert!(
        facade.cargo_toml.contains("host = { path = "),
        "the host is a sibling dependency: {}",
        facade.cargo_toml
    );
    assert!(
        facade
            .build_rs
            .contains("\"metadata\", \"--format-version\", \"1\""),
        "the host root is resolved through cargo: {}",
        facade.build_rs
    );
    assert!(facade.build_rs.contains("run_for_partition"));
    assert!(!facade.build_rs.contains("unsafe") && !facade.build_rs.contains("set_var"));
    assert!(
        !facade.build_rs.contains(&root.display().to_string()),
        "the facade's build script must not bake the author's host path: {}",
        facade.build_rs
    );

    // A declaration with no crates has nothing for a facade to carry, and says so.
    // 没有 crate 的声明没有东西让 facade 承载，它会说出来。
    let refused = plan_facade(&root, "host", "host", "host", &[], &[]).expect_err("refused");
    assert!(
        refused.contains("at least one generated package"),
        "{refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A release ghost carries its host's graft cuts **as data**; without them it compiled the copied faces
/// and emitted an empty table.
/// 发布幽灵把宿主的 graft 切口当作**数据**携带；没有它们，它会编译复制过来的面、发射一张空表。
///
/// The development shape can read the host's tree at build time, and does; the release shape has none to
/// read, so the cuts have to arrive in the generated script. Measured on a two-ghost host: the ghost that
/// compiled the cut face emitted no cut for it in the release shape and one in the development shape, so a
/// published crate never applied the graft the plan promised (audit `M7`, §M7.55).
/// 开发形状在构建期读得到宿主的树，它也确实这么做；发布形状没有树可读，因此切口必须写进生成的脚本。
/// 在一个两幽灵的宿主上实测：编译切口面的那个幽灵在发布形状下不为它发射任何切口、在开发形状下发射一条，
/// 于是发布出去的 crate 从不应用计划承诺的嫁接（审计 `M7`，§M7.55）。
#[test]
fn a_release_ghost_carries_the_hosts_cuts_as_data() {
    let root = host("cuts");
    let cut = crate::build_method::HostCut {
        cut: "crate::panel::frame::widget::NODE_ID".to_owned(),
        cut_end: None,
        graft: "fast_widget::fast::NODE_ID".to_owned(),
        full: false,
        typed: true,
        cfg: None,
        line: 7,
        column: 8,
    };
    let carried = plan_ghost(&root, &planned(&root), &faces(), &[cut]).expect("the ghost plans");
    assert!(
        carried.build_rs.contains("run_for_partition_with_cuts"),
        "the generated script hands the cuts over: {}",
        carried.build_rs
    );
    for expected in [
        "crate::panel::frame::widget::NODE_ID",
        "fast_widget::fast::NODE_ID",
        "typed: true",
        "line: 7",
    ] {
        assert!(
            carried.build_rs.contains(expected),
            "the data survives into the script (`{expected}`): {}",
            carried.build_rs
        );
    }
    // A host that declares nothing keeps the five-argument call, so a generated script says exactly as
    // much as it needs to.
    // 什么都没声明的宿主保留五参数调用，因此生成的脚本只说它必须说的那些。
    let bare = plan_ghost(&root, &planned(&root), &faces(), &[]).expect("the ghost plans");
    assert!(
        bare.build_rs.contains("run_for_partition(")
            && !bare.build_rs.contains("HostCut")
            && !bare.build_rs.contains("run_for_partition_with_cuts"),
        "no cuts, no argument: {}",
        bare.build_rs
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A release facade refuses a host cargo will not hand over: one with no library target.
/// 发布 facade 拒绝一个 cargo 不肯交出的宿主：没有库目标的那个。
///
/// Cargo **ignores** such a dependency (it warns and builds on), and the facade's build script then took
/// its own directory for the host's — measured, the published facade carried neither faces nor cuts and
/// the build still reported success.
/// cargo 会**忽略**这样的依赖（警告一句、继续构建），而 facade 的构建脚本随后把自己的目录当成了宿主的——
/// 实测，发布出去的 facade 既不带面也不带切口，而构建仍报告成功。
///
/// The refusal lives at the **writers**, and this test says so in both halves: planning still describes the
/// package (a read-only view runs the same planner, and a declaration edit that merely looks at the plan must
/// not fail because the host is a binary — measured, putting the refusal in `plan_facade` made
/// `crates --declare … --write` refuse an edit with nothing to do with release shapes), and
/// [`refuse_binary_host`] — which `write_release` calls before a single file lands — refuses it.
/// 拒绝住在**写入方**，而这条测试两半都说了：规划仍然描述这个包（只读视图跑的是同一个规划器，而一次只是看一眼
/// 计划的声明编辑不该因为宿主是二进制而失败——实测，把拒绝放进 `plan_facade` 让
/// `crates --declare … --write` 拒绝了一次与发布形状毫无关系的编辑），而 [`refuse_binary_host`]——
/// `write_release` 在任何文件落地之前调它——拒绝它。
#[test]
fn a_binary_host_is_refused_by_the_writer_not_by_the_planner() {
    let root = host("binary");
    std::fs::remove_file(root.join("src/lib.rs")).expect("the host loses its library");
    let planned = vec![planned(&root)];
    plan_facade(&root, "host", "host", "host", &planned, &[])
        .expect("planning still describes the package it would write");
    let refused = super::refuse_binary_host(&root).expect_err("the writer refuses it");
    assert!(
        refused.contains("no library target") && refused.contains("way forward"),
        "the refusal names the reason and the way out: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
