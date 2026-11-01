//! Pins for the facade plan: what it carries, when it is unnecessary, and when it is refused.
//! facade 规划的钉子：它携带什么、什么时候不需要它、什么时候被拒绝。

use std::fs;
use std::path::{Path, PathBuf};

use super::plan_facade;
use crate::build_method::crate_plan::{PlannedCrate, PlannedMount, crates_dir};

/// A throwaway host package with the dependency tables a real host has.
/// 一个一次性宿主包，带着真宿主会有的那两张依赖表。
fn host(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let parent = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-facade-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&parent);
    let root = parent.join("control-button");
    fs::create_dir_all(root.join("src")).expect("host dirs");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"control-button\"\nversion = \"0.2.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nxirang-toolchain = { path = \"../../toolchain\" }\n\
         control-button-graft = { path = \"../../control-button-graft\" }\n\n\
         [build-dependencies]\nxirang-toolchain = { path = \"../../toolchain\" }\n",
    )
    .expect("host manifest");
    root
}

/// One planned ghost, next to the host.
/// 一个已规划的幽灵，在宿主旁边。
fn ghost(root: &Path, package: &str) -> PlannedCrate {
    let partition_root = crate::build_method::crate_plan::partition_roots(root).0;
    PlannedCrate {
        name: package.trim_start_matches("control-button-").to_owned(),
        package: package.to_owned(),
        namespace: "control-button".to_owned(),
        // The real helper, not a hand-built path: a fixture that spells the layout itself goes
        // stale the moment the layout moves, and then it answers for the old tree.
        // 调用真助手，而不是手拼路径：自己拼布局的夹具会在布局一挪就过期，然后替旧树作答。
        directory: crates_dir(&partition_root).join(package),
        subtrees: vec!["control::object".to_owned()],
        aliases_rs: String::new(),
        ancestors: Vec::new(),
        mounts: vec![PlannedMount {
            module_path: "control::object::button".to_owned(),
            source: "control/object/button/button.rs".to_owned(),
            spelling: "../control-button/src/control/object/button/button.rs".to_owned(),
        }],
        remap: vec![("../control-button/src/".to_owned(), String::new())],
        lib_rs: "// ghost\n".to_owned(),
        build_rs: "// ghost\n".to_owned(),
        cargo_toml: "// ghost\n".to_owned(),
        config_patch: String::new(),
    }
}

/// The facade sees the host and every ghost by a sibling path, carries the host's namespace, and asks
/// the pipeline for the facade mode.
/// facade 用同级路径看得见宿主与每个幽灵、携带宿主的命名空间、并向管线要 facade 模式。
#[test]
fn a_facade_sees_the_host_and_every_ghost() {
    let root = host("sees");
    let planned = vec![ghost(&root, "control-button-widgets")];
    let facade = plan_facade(
        &root,
        "control-button",
        "control-button",
        "control-button",
        &planned,
        &[],
    )
    .expect("the plan is made")
    .expect("a partition always has a facade");
    assert_eq!(facade.package, "control-button-facade");
    assert_eq!(
        facade.dependencies,
        vec![
            "control-button".to_owned(),
            "control-button-widgets".to_owned()
        ],
        "the printed list names what it is planned to see"
    );
    for expected in [
        "publish = false",
        "control-button = { path = \"../..\" }",
        "control-button-widgets = { path = \"../control-button-widgets\" }",
        // The host's own table, re-spelled for this package's directory: this is where the
        // implementation crates a typed cut names come from, and the facade sits one level down.
        // 宿主自己的表，按本包所在目录重拼：类型化切口点名的实现 crate 就是从这里来的，而 facade 低一层。
        "control-button-graft = { path = \"",
        "[build-dependencies]",
    ] {
        assert!(
            facade.cargo_toml.contains(expected),
            "Cargo.toml carries `{expected}`: {}",
            facade.cargo_toml
        );
    }
    assert!(
        facade
            .lib_rs
            .contains("pub const XIRANG_NAMESPACE: &str = \"control-button\";")
            && facade.lib_rs.contains("generated_lib.rs"),
        "the crate root is the host's namespace plus the generated plan: {}",
        facade.lib_rs
    );
    assert!(
        facade.build_rs.contains("run_for_partition")
            && !facade.build_rs.contains("unsafe")
            // The pipeline takes the **package root**, not the manifest path: passing the manifest made
            // it look for `<…>/Cargo.toml/src`, which the first end-to-end build reported. The root is
            // reached from `CARGO_MANIFEST_DIR` plus a relative walk: the facade is committed, so an
            // absolute path would bind the split to the machine that generated it.
            // 管线收的是**包根**而不是清单路径：传清单会让它去找 `<…>/Cargo.toml/src`，这是第一次端到端
            // 构建报出来的。那个根是从 `CARGO_MANIFEST_DIR` 加一段相对走法到达的：facade 要入库，因此绝对
            // 路径会把这次拆分绑死在产生它的机器上。
            && facade.build_rs.contains("env!(\"CARGO_MANIFEST_DIR\")")
            && !facade.build_rs.contains(&format!("{:?}", root))
            && facade.build_rs.matches("rerun-if-changed").count() == 2,
        "its build script runs the host's manifest in facade mode: {}",
        facade.build_rs
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declaration that hands nothing away has nothing cross-crate to carry: no facade.
/// 什么都没交出去的声明，没有跨 crate 的东西要承载：没有 facade。
#[test]
fn a_declaration_with_no_crates_needs_no_facade() {
    let root = host("empty");
    let facade = plan_facade(
        &root,
        "control-button",
        "control-button",
        "control-button",
        &[],
        &[],
    )
    .expect("no refusal")
    .is_none();
    assert!(facade, "an empty plan has no facade");
    let _ = std::fs::remove_dir_all(&root);
}

/// A facade that would be the host's own package is refused by name: a crate cannot depend on itself.
/// 会成为宿主自己包名的 facade 被点名拒绝：一个 crate 不能依赖它自己。
#[test]
fn a_facade_that_would_be_the_host_is_refused() {
    let root = host("clash");
    let planned = vec![ghost(&root, "control-button-widgets")];
    let refused = plan_facade(&root, "control", "control", "control-facade", &planned, &[])
        .expect_err("refused");
    assert!(
        refused.contains("control-facade") && refused.contains("package_prefix"),
        "the refusal names the clash and the knob: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
