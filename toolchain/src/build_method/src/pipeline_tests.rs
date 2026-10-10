//! Tests for the build pipeline's refusal paths.
//! 构建管线拒绝路径的测试。

use std::time::Duration;

use super::{cache_status_line, phase_line};

/// The demo directory and its feature are two constants, and the `cfg` line the
/// renderer writes is built from the feature name. Renaming either would change the
/// generated tree — the identity of every host face rides on strings like this one
/// staying put — so the pin makes a rename visible instead of silent.
/// 演示目录与它的特性是两个常量，而渲染器写出的 `cfg` 行由特性名拼出。改名任何一个都会改变生成树——
/// 每个宿主面的身份都系在这类字符串原地不动上——因此这条钉子让改名可见，而不是静默生效。
#[test]
fn the_demo_constants_spell_the_generated_tree_the_same_way() {
    assert_eq!(
        crate::build_method::DEMO_ONLY_DIRECTORY,
        "compile_error_demo"
    );
    assert_eq!(crate::build_method::DEMO_ONLY_FEATURE, "compile_error_demo");
    assert_eq!(
        format!(
            "#[cfg(feature = {:?})]",
            crate::build_method::DEMO_ONLY_FEATURE
        ),
        "#[cfg(feature = \"compile_error_demo\")]"
    );
}

/// Keeps two tests in the same process apart even when the clock resolution
/// collapses their timestamps into one nanosecond; the workspace suite runs
/// them in parallel and a collision silently mixes two fixtures.
/// 即使时钟分辨率把两个测试的时间戳压进同一纳秒，也把同一进程内的两者分开；
/// workspace 套件并行运行它们，撞名会静默把两套夹具混在一起。
static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[test]
fn successful_build_is_silent_unless_verbose_output_is_requested() {
    assert_eq!(cache_status_line("hit", "abc", false), None);
    assert_eq!(
        cache_status_line("hit", "abc", true).as_deref(),
        Some("xirang discovery cache hit (abc)")
    );
}

/// The phase line is what a reader gets instead of "the build script is 73% of the wall": every
/// phase in the order it ran, then the whole run. The pin keeps the shape a reader (and the
/// measurement in `docs/design-discovery-record.md`) can parse, so a rename or a dropped phase
/// shows up here rather than in a table nobody can reproduce.
/// 相位行是读者在"构建脚本占墙 73%"之外拿到的东西：按运行顺序的每个相位，然后是整次运行。这条钉子守住
/// 读者（以及 `docs/design-discovery-record.md` 里的测量）能解析的形状，因此改名或漏掉一个相位会在这里
/// 现形，而不是在一张没人能复现的表里。
#[test]
fn the_phase_line_lists_every_phase_and_ends_with_the_total() {
    let marks = [
        ("discover", Duration::from_millis(21)),
        ("declare", Duration::from_millis(2631)),
        ("render", Duration::from_millis(193)),
    ];
    assert_eq!(
        phase_line(&marks, Duration::from_millis(1500)),
        "xirang build phases: discover=0.021s declare=2.631s render=0.193s total=1.500s"
    );
}

/// The build-script path has no stdout contract to keep and no generated tree
/// to carry the message, so a missing source tree fails loudly with the
/// rendered diagnostic — not with the bare `expect` it used to hit, which
/// named neither the tree nor the reason.
/// 构建脚本路径没有 stdout 契约要守，也没有生成树可以承载消息，因此源树缺失时带着渲染后的
/// 诊断响亮失败——而不是撞上过去那条裸的 `expect`，它既没说清哪棵树，也没说清原因。
#[test]
#[should_panic(expected = "face-layout")]
fn a_build_script_without_a_source_tree_fails_with_the_layout_diagnostic() {
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-pipeline-missing-src-{}-{sequence}",
            std::process::id()
        ));
    std::fs::create_dir_all(&root).expect("fixture root");
    let input = super::BuildInput::new(root.clone(), root.join("target/xirang/out"), true);
    let _ = super::run(&input);
}

/// Discovery skips a directory whose name cannot be a module, so its faces
/// never reach the generated tree. The admission scan reads the same tree,
/// so it must skip them too: a face that can never be compiled must not be
/// able to veto the build.
/// 发现过程会跳过名字无法成为模块的目录，其中的注册面永远进不了生成树。admission
/// 扫描读的是同一棵树，因此也必须跳过它们：永远编译不到的面不该能否决构建。
#[test]
fn an_unrepresentable_directory_cannot_veto_the_build() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let face = "crate::root_object! {\n    kind: Hidden,\n    requires: [\"missing.capability\" => \"MissingProvider\"],\n}\n";
    let mut outcomes = Vec::new();
    for (tag, dir) in [("valid", "hidden"), ("invalid", "bad-name")] {
        let root = std::env::temp_dir()
            .join("xirang-scratch")
            .join(module_path!().replace("::", "-"))
            .join(format!("xirang-hidden-{tag}-{suffix}-{sequence}"));
        let manifest = root.join("host");
        let folder = manifest.join("src").join(dir);
        std::fs::create_dir_all(&folder).expect("face folder");
        std::fs::write(folder.join(format!("{dir}.rs")), face).expect("write face");
        let out = root.join("out");
        outcomes.push(crate::build_method::run_for(&manifest, &out, "test-host").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
    assert_eq!(
        outcomes,
        [true, false],
        "the same face must fail in a representable directory and be ignored in one that cannot be a module"
    );
}

#[test]
fn run_for_validates_outside_cargo_and_reports_diagnostics() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!("xirang-run-for-{suffix}-{sequence}"));
    let manifest = root.join("host");
    std::fs::create_dir_all(manifest.join("src")).expect("src");
    let out = root.join("out");

    // An empty host has no registration faces; validation succeeds.
    assert!(crate::build_method::run_for(&manifest, &out, "test-host").is_ok());

    // Two faces declaring the same stable_name fail with rendered
    // diagnostics instead of `cargo:` directive noise. Faces follow the
    // `<name>/<name>.rs` layout.
    let face = |kind: &str| {
        format!("crate::root_object! {{\n    kind: {kind},\n    stable_name: \"dup\",\n}}\n")
    };
    for (dir, kind) in [("one", "One"), ("two", "Two")] {
        let folder = manifest.join("src").join(dir);
        std::fs::create_dir_all(&folder).expect("face folder");
        std::fs::write(folder.join(format!("{dir}.rs")), face(kind)).expect("write face");
    }
    let error = crate::build_method::run_for(&manifest, &out, "test-host")
        .expect_err("duplicate stable_name must fail");
    assert!(
        error.contains("duplicate stable_name"),
        "unexpected diagnostics: {error}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A host directory for one discovery case, removed by the caller.
/// 为某个发现用例建一个宿主目录，由调用方删除。
fn host_root(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!("xirang-{tag}-{suffix}-{sequence}"));
    let manifest = root.join("host");
    std::fs::create_dir_all(manifest.join("src")).expect("src");
    (root, manifest)
}

/// An ordinary module beside the faces is not a registration source. Any flat
/// `.rs` under `src/` used to panic with a layout message, which made the tool
/// unusable on a real crate: `src/helpers.rs` is not a mistake.
/// 注册面旁边的普通模块不是注册源。`src/` 下任何平铺 `.rs` 过去都会以布局消息
/// panic，使工具在真实 crate 上不可用：`src/helpers.rs` 并不是错误。
#[test]
fn an_ordinary_flat_module_is_not_a_registration_source() {
    let (root, manifest) = host_root("flat-module");
    std::fs::write(
        manifest.join("src/helpers.rs"),
        "pub fn helper() -> u32 { 1 }\n",
    )
    .expect("write module");
    let outcome = crate::build_method::run_for(&manifest, &root.join("out"), "test-host");
    assert!(
        outcome.is_ok(),
        "an ordinary module must be skipped, not refused: {outcome:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A file that *is* a registration face but sits outside `<name>/<name>.rs`
/// is reported with its path. The build can never compile it, so silence
/// would be a face that silently never registers.
/// 确实是注册面、却长在 `<name>/<name>.rs` 之外的文件会带着路径被报告。构建永远编译
/// 不到它，沉默就意味着一个静默地从未注册的面。
#[test]
fn a_face_outside_the_layout_is_reported_with_its_path() {
    let (root, manifest) = host_root("misplaced-face");
    std::fs::write(
        manifest.join("src/flat.rs"),
        "crate::root_object! {\n    kind: Flat,\n}\n",
    )
    .expect("write face");
    let error = crate::build_method::run_for(&manifest, &root.join("out"), "test-host")
        .expect_err("a face outside the layout must fail the build");
    assert!(error.contains("flat.rs"), "the file must be named: {error}");
    assert!(
        error.contains("face-layout")
            // The half a reader acts on: the move is an identity change, not a layout fix.
            // 读者会照做的那一半：这次搬动是身份变化，不是布局修正。
            && error.contains("NodeId = hash(namespace, source path, name)")
            && error.contains("stops") && error.contains("resolving"),
        "the diagnostic must say what is wrong: {error}"
    );
    assert!(
        error.contains("<name>/<name>.rs"),
        "the diagnostic must say where the file belongs: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A generated tree that cannot be written is a diagnostic, not a panic: a
/// structured caller reports it with the rest, while a build script stops
/// with the reason, having nowhere to render one.
/// 写不成的生成树是诊断而不是 panic：结构化调用方把它与其余诊断一起报出，而构建脚本
/// 带着原因停下——它没有地方渲染诊断。
#[test]
fn an_unwritable_generated_tree_is_reported_not_fatal() {
    let (root, manifest) = host_root("unwritable-out");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("out directory");
    // A directory where the fingerprint file belongs: the path exists, so this
    // is a write failure rather than a missing parent.
    // 指纹文件的位置放一个目录：路径存在，因此这是写失败而不是父目录缺失。
    std::fs::create_dir_all(out.join("discovery.fingerprint")).expect("blocking directory");

    let error = crate::build_method::run_for(&manifest, &out, "test-host")
        .expect_err("an unwritable generated tree must fail the run");
    assert!(
        error.contains("out-dir"),
        "the failure is a diagnostic: {error}"
    );
    // The artifact is written to a unique sibling and then renamed over the target,
    // so a target that cannot be replaced fails at the rename: "cannot publish"
    // names that step, and the path still names the file the reader was expecting.
    // 产物先写到一个唯一的同级文件，再 rename 覆盖目标，因此无法被替换的目标会在 rename 处失败：
    // "cannot publish" 点名的就是这一步，而路径仍然点名读取方期待的那个文件。
    assert!(error.contains("cannot publish"), "{error}");
    assert!(error.contains("discovery.fingerprint"), "{error}");

    let _ = std::fs::remove_dir_all(&root);
}

/// A payload that cannot be written publishes no fingerprint. The fingerprint is
/// the token `build_output_is_current` reads, so a run that could not publish its
/// manifests must not leave one behind: writing the token first produced exactly the
/// mixed generation it is supposed to rule out — a new token beside an old or
/// missing manifest — and the next reader called that `current`.
/// 写不成的载荷不发布指纹。指纹是 `build_output_is_current` 读取的那枚凭据，因此发布不了清单的一次
/// 运行绝不能把它留下：先写凭据恰好产生了这枚凭据本该排除的混代状态——新凭据配旧或缺的清单——而下一个
/// 读取方会把它称作 `current`。
#[test]
fn a_payload_that_cannot_be_written_publishes_no_fingerprint() {
    let (root, manifest) = host_root("payload-failure");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("out directory");
    // A directory where the pruning manifest belongs: the path exists, so this is a
    // write failure rather than a missing parent.
    // 修剪清单的位置放一个目录：路径存在，因此这是写失败而不是父目录缺失。
    std::fs::create_dir_all(out.join("pruning_manifest.tsv")).expect("blocking directory");

    let error = crate::build_method::run_for(&manifest, &out, "test-host")
        .expect_err("an unwritable payload must fail the run");
    assert!(error.contains("out-dir"), "{error}");
    assert!(
        !out.join("discovery.fingerprint").exists(),
        "a run that could not publish its payloads must not publish the token \
         that calls them current"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A face file that does not parse is reported, not fatal: the same run also
/// reports what it costs the entry reader, and `check --json` gets a document.
/// Before this the entry reader panicked first, so the build died with exit
/// 101 and an empty stdout.
/// 解析不了的注册面文件被报告，而不是致命：同一次运行还会报出它给入口读取器带来的
/// 代价，而 `check --json` 拿到文档。在这之前入口读取器先 panic，于是构建以退出 101
/// 与空白 stdout 死掉。
#[test]
fn a_malformed_face_is_reported_instead_of_aborting() {
    let (root, manifest) = host_root("malformed-face");
    std::fs::create_dir_all(manifest.join("src/broken")).expect("face folder");
    std::fs::write(
        manifest.join("src/broken/broken.rs"),
        "crate::root_object! {\n    kind: Broken\n",
    )
    .expect("write broken face");
    let error = crate::build_method::run_for(&manifest, &root.join("out"), "test-host")
        .expect_err("a malformed face must fail the build");
    assert!(
        error.contains("face-syntax"),
        "the face diagnostic must be there: {error}"
    );
    assert!(
        error.contains("broken.rs"),
        "the file must be named: {error}"
    );
    assert!(
        error.contains("entry"),
        "the entry reader must report it too, not abort: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// Pins for the two halves of one defect: what a package **emits**, and what it **says** it emits
// (audit `M7`, §M7.55).
// 一条缺陷的两半各自的钉子：一个包**发射**什么，以及它**说**自己发射什么（审计 `M7`，§M7.55）。
//
// The defect was measured on a real host in both partition shapes. In the development shape every
// package's `graft_plan.tsv` listed the **host's whole plan** while the table it compiled carried one
// entry or none, so two readers concluded the shape "carries every cut everywhere"; in the release shape
// the ghost that compiled the cut face emitted **no** cut at all, so the published crate compiled the
// copied source and never applied the graft the plan promised. The three cases below are the three
// answers that have to agree: the emitted table, the audit text, and the cuts a package receives when its
// own tree carries no declaration.

use std::path::{Path, PathBuf};

use crate::build_method::{HostCut, check_for_shape_with_cuts};

/// A throwaway host package with one registration face, and its output directory.
/// 一个含单个注册面的临时宿主包，以及它的输出目录。
///
/// `entry` is written verbatim as `src/lib.rs`, because the pipeline **parses** the entry rather than
/// compiling it: a fixture that spelled a real `FRAMEWORK` constant would be testing rustc, not this.
/// `entry` 逐字写成 `src/lib.rs`，因为管线**解析**入口而不是编译它：把 `FRAMEWORK` 常量写成真的，测的是
/// rustc 而不是本模块。
fn fixture(label: &str, entry: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-cuts-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"cuts-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), entry).expect("entry");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n",
    )
    .expect("face");
    let out = root.join("out");
    (root, out)
}

/// The one cut every fixture host's face is replaced by, as the generator would hand it over.
/// 每个夹具宿主的面都被它替换的那一条切口，按生成器交过来的样子。
fn the_cut() -> HostCut {
    HostCut {
        cut: "crate::button::NODE_ID".to_owned(),
        cut_end: None,
        graft: "fast_button::fast::NODE_ID".to_owned(),
        full: false,
        typed: true,
        cfg: None,
        line: 3,
        column: 8,
    }
}

/// How many entries the generated table carries, and how many rows the audit text has.
/// 生成的表里有多少条，以及审计文本有多少行。
fn read_both(out: &Path) -> (usize, usize) {
    let generated =
        std::fs::read_to_string(out.join("generated_lib.rs")).expect("the generated tree");
    let table = generated
        .split_once("pub static BUILTIN_GRAFT_CUTS")
        .and_then(|(_, rest)| rest.split_once("];"))
        .map(|(body, _)| body.matches("StaticGraftCut::").count())
        .expect("the cut table is in the generated tree");
    let audit = std::fs::read_to_string(out.join("graft_plan.tsv")).expect("the audit text");
    let rows = audit.lines().filter(|line| !line.starts_with('#')).count();
    (table, rows)
}

/// A package that carries its host's cuts as **data** emits them — and says so.
/// 把宿主的切口当作**数据**携带的包会发射它们——并且如实说出来。
///
/// This is the release shape's ghost: its own tree has copied faces and no entry, so the empty-table
/// answer it used to give was the published crate never applying the graft the plan promised.
/// 这就是发布形状的幽灵：它自己的树里只有复制过来的面、没有入口，因此它过去给出的"空表"答案，就是发布
/// 出去的 crate 从不应用计划承诺的嫁接。
#[test]
fn a_package_carrying_host_cuts_emits_them_and_records_the_same_rows() {
    let (root, out) = fixture(
        "carried",
        "// a generated package: no declaration of its own\n",
    );
    check_for_shape_with_cuts(&root, &out, "cuts-host", None, false, false, &[the_cut()])
        .expect("the pipeline accepts the host");
    let (table, rows) = read_both(&out);
    assert_eq!(table, 1, "the cut the host declared is emitted");
    assert_eq!(
        rows, table,
        "the audit text lists exactly the table this package compiles"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A package with neither a declaration nor inherited cuts emits none — and the audit text agrees.
/// 既没有声明、也没有继承切口的包，一条都不发射——而审计文本与此一致。
///
/// The negative half is what the development shape failed: the audit text carried the host's whole plan
/// into packages whose compiled table was empty.
/// 否定的那一半正是开发形状失败的地方：审计文本把宿主的整份计划带进了那些编译表为空的包。
#[test]
fn a_package_with_no_cuts_emits_none_and_records_none() {
    let (root, out) = fixture("bare", "// nothing declared here\n");
    check_for_shape_with_cuts(&root, &out, "cuts-host", None, false, false, &[])
        .expect("the pipeline accepts the host");
    let (table, rows) = read_both(&out);
    assert_eq!(table, 0, "nothing is emitted");
    assert_eq!(rows, 0, "and nothing is claimed: {rows} row(s)");
    let _ = std::fs::remove_dir_all(&root);
}

/// A hand-written entry is still the authority: its cuts are emitted, and the audit text matches.
/// 手写入口仍是权威：它的切口会被发射，审计文本与之一致。
///
/// This is the half that already worked, kept here so the two paths cannot drift apart silently.
/// 这是本来就没坏的那一半，留在这里，好让两条路不会无声地分道扬镳。
#[test]
fn a_hand_written_entry_still_answers_for_itself() {
    let (root, out) = fixture(
        "entry",
        "pub const FRAMEWORK: FrameworkId = FrameworkId::new(\"cuts.fixture\");\n\
         xirang_toolchain::run_method::static_graft_plan!(\n\
         \x20   FRAMEWORK,\n\
         \x20   cut(crate::button::NODE_ID) graft(fast_button::fast::NODE_ID),\n\
         );\n",
    );
    check_for_shape_with_cuts(&root, &out, "cuts-host", None, false, false, &[]).expect("accepted");
    let (table, rows) = read_both(&out);
    assert_eq!(table, 1, "the entry's cut is emitted");
    assert_eq!(rows, table, "and the audit text lists the same one");
    let _ = std::fs::remove_dir_all(&root);
}
