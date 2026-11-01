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
    let line = phase_line(&marks, Duration::from_millis(1500));
    assert!(
        line.starts_with("xirang build phases: discover=0.021s declare=2.631s render=0.193s "),
        "every phase in order: {line}"
    );
    assert!(
        line.ends_with(" total=1.500s"),
        "and the total still ends the line: {line}"
    );
    assert!(
        line.contains(" parses=") && line.contains(" facts="),
        "the mechanism's own counters ride along, because a duration cannot resolve them: {line}"
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
    let _ = super::run(&input, &[], &[], &[], &[], None);
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

/// §6.1 of the discovery-record design, on the pipeline itself: the same tree built twice — the first
/// time publishing a record, the second reading one — writes the same products, and a tree that has a
/// diagnostic produces it **and publishes no record**, so a record can never hide one.
/// 发现记录设计的 §6.1，落在管线上：同一棵树建两次——第一次发布记录、第二次读记录——写出的产物相同；而一棵有
/// 诊断的树会给出那条诊断**并且不发布记录**，因此记录永远遮不住一条诊断。
///
/// That last half is the design's own warning ("the easiest half to break silently"): a record published
/// by a clean run cannot be read by a defective one, because a defect changes bytes, bytes are the key,
/// and a run that has diagnostics never reaches the publish step at all. The pin makes both true at once,
/// and the tamper step is what gives it teeth: a record whose payload no longer matches its digest must
/// be rebuilt rather than copied, or the third run would publish the tampered bytes.
/// 后一半正是设计自己的警告（"最容易悄悄坏掉的那一半"）：干净运行发布的记录不可能被有缺陷的一次读到——缺陷会改
/// 字节、字节就是键，而有诊断的一次根本走不到发布那一步。钉子让两件事同时成立，而"篡改"那一步给它装上牙：载荷
/// 与摘要不再相符的记录必须被重建而不是被复制，否则第三次运行会把篡改过的字节发布出去。
#[test]
fn a_record_hit_writes_the_same_products_and_diagnostics_as_the_full_path() {
    let (root, manifest) = host_root("record-equivalence");
    let out = root.join("out");
    // `stable_name` is spelled on both faces below so the pair collides: that diagnostic is about two
    // faces claiming one name, which is the shape the existing duplicate-name test uses.
    // 下面两个面都写 `stable_name`，让它们撞在一起：那条诊断讲的是两个面认领同一个名字，也是既有重名测试的形状。
    // `handle_contracts` is here on purpose: `handle_traits` is **derived** from it by the field reader,
    // so this fixture walks the one path a second decoder inside the unit writer would get wrong — the
    // derived label would silently go missing on a record hit.
    // 这里带上 `handle_contracts` 是有意的：`handle_traits` 是字段读取器从它**推导**出来的，因此这条夹具
    // 走到的正是"unit 写入方另写一份解码器就会弄错"的那条路——记录命中时那个推导标签会静默消失。
    let face = |kind: &str, stable: &str| {
        format!(
            "crate::root_object! {{\n    kind: {kind},\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n    stable_name: \"{stable}\",\n    handle_contracts: [crate::ControlHandle],\n}}\n"
        )
    };
    let folder = manifest.join("src").join("button");
    std::fs::create_dir_all(&folder).expect("face folder");
    std::fs::write(folder.join("button.rs"), face("Button", "dup")).expect("write face");
    std::fs::write(manifest.join("src/lib.rs"), "// host entry\n").expect("write entry");

    crate::build_method::run_for(&manifest, &out, "record-host").expect("the first build succeeds");
    let first = product_bytes(&out);
    assert!(
        !first.is_empty(),
        "a successful build writes its products: {:?}",
        first.keys().collect::<Vec<_>>()
    );
    let records = manifest.join("target/xirang/records");
    assert!(
        records.is_dir(),
        "a clean run publishes this tree's record at {}",
        records.display()
    );

    // Second run: the record is there and verifies, so the tree-level products are copied.
    // 第二次运行：记录在、且校验通过，因此树级产物来自复制。
    crate::build_method::run_for(&manifest, &out, "record-host")
        .expect("the second build succeeds");
    assert_eq!(
        first,
        product_bytes(&out),
        "a record hit writes the same products as the full path"
    );

    // Tampering: the payload no longer matches the digest `record.tsv` carries, so the run rebuilds.
    // 篡改：载荷与 `record.tsv` 携带的摘要不再相符，因此这次运行重建。
    let tampered = std::fs::read_dir(
        records.join("").join(
            std::fs::read_dir(&records)
                .expect("the records directory")
                .flatten()
                .map(|entry| entry.file_name())
                .next()
                .expect("a tree directory"),
        ),
    )
    .expect("the namespace directory")
    .flatten()
    .map(|entry| entry.path().join("products/pruning_manifest.tsv"))
    .find(|path| path.is_file())
    .expect("a pruning payload");
    let mut bytes = std::fs::read(&tampered).expect("the payload");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(&tampered, &bytes).expect("the tampered payload");
    crate::build_method::run_for(&manifest, &out, "record-host").expect("the third build succeeds");
    assert_eq!(
        first,
        product_bytes(&out),
        "a damaged record is rebuilt, not copied into the tree"
    );

    // A tree with a diagnostic: the run reports it and publishes no record for that tree.
    // 一棵有诊断的树：这次运行报出它，并且不为那棵树发布记录。
    // Both faces now claim `dup`, and the first one's bytes changed with it, so this tree is a
    // different key than the one the record was published for.
    // 两个面现在都认领 `dup`，而第一个面的字节也随之变了，因此这棵树与记录发布时的那棵是不同的键。
    std::fs::write(folder.join("button.rs"), face("Button", "dup")).expect("write face");
    let duplicate = manifest.join("src").join("second");
    std::fs::create_dir_all(&duplicate).expect("second face folder");
    std::fs::write(duplicate.join("second.rs"), face("Second", "dup")).expect("write second face");
    let error = crate::build_method::run_for(&manifest, &out, "record-host")
        .expect_err("a duplicate stable_name is a diagnostic");
    assert!(error.contains("button"), "the diagnostic names it: {error}");
    let trees = std::fs::read_dir(&records)
        .expect("the records directory")
        .flatten()
        .count();
    assert_eq!(
        trees, 1,
        "the defective tree published no record of its own"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Every product in an out directory that is **not** per-run, as name ⇒ bytes.
/// 一个输出目录里**不属于这次运行**的每一份产物，按名字 ⇒ 字节。
///
/// `graph.generation` is excluded on purpose and by design: it carries the generation counter and the
/// wall-clock stamp of the run that wrote it.
/// `graph.generation` 是有意、按设计排除的：它带着写下它的那次运行的代数计数与墙钟戳。
fn product_bytes(out: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut products = std::collections::BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(out) else {
        return products;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() || name == "graph.generation" {
            continue;
        }
        if let Ok(bytes) = std::fs::read(entry.path()) {
            products.insert(name, bytes);
        }
    }
    products
}

/// **Ownership** (`docs/design-discovery-record.md` §6): the record is a property of the **tree**, the
/// filter is a property of the **package** — so a fragment's record hit must still render only the
/// subtree it claimed, while the payload it copied keeps listing every face.
/// **归属**（`docs/design-discovery-record.md` §6）：记录是**树**的属性、过滤是**包**的属性——因此一个碎片的
/// 记录命中仍必须只渲染它认领的子树，而它复制过来的载荷照旧列出每一个面。
///
/// This is the nail the projection could most easily get wrong: the payload it reads is deliberately
/// **unfiltered**, and a projection that forgot to apply the scope would compile another package's faces
/// into this one — which is exactly what `add_crates.rs` exists to prevent (audit `M7`, P3.2).
/// 这是投影最容易弄错的一颗钉子：它读的载荷**有意**未过滤，而忘了施加 scope 的投影会把别的包的面编译进
/// 这一个——正是 `add_crates.rs` 存在要防的事（审计 `M7`，P3.2）。
#[test]
fn a_fragment_projects_only_its_own_claim_even_on_a_record_hit() {
    let (root, manifest) = host_root("fragment-ownership");
    let out = root.join("out");
    let face = |kind: &str| {
        format!(
            "crate::root_object! {{\n    kind: {kind},\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}}\n"
        )
    };
    for (folder, kind) in [("alpha", "Alpha"), ("beta", "Beta")] {
        let directory = manifest.join("src").join(folder);
        std::fs::create_dir_all(&directory).expect("face folder");
        std::fs::write(directory.join(format!("{folder}.rs")), face(kind)).expect("write face");
    }
    std::fs::write(manifest.join("src/lib.rs"), "// fragment host entry\n").expect("write entry");

    // The host writes the tree's record, with every face in the payload.
    // 宿主写下这棵树的记录，载荷里带着每一个面。
    crate::build_method::run_for(&manifest, &out, "fragment-host")
        .expect("the host build succeeds");
    let copied = std::fs::read_to_string(out.join("faces_manifest.tsv")).expect("the payload");
    assert!(
        copied.contains("alpha/alpha.rs") && copied.contains("beta/beta.rs"),
        "the record is tree level: its payload lists faces this package, and every package, may not compile:\n{copied}"
    );

    // One fragment, on a record hit: it renders its claim and nothing else.
    // 一个碎片，在记录命中时：只渲染它的认领。
    let claimed = root.join("claimed");
    crate::build_method::run_for_partition(
        &manifest,
        &claimed,
        "fragment-ghost",
        Some("alpha"),
        false,
    )
    .expect("the fragment build succeeds");
    let hit = product_bytes(&claimed);
    let host = product_bytes(&out);
    // The plan decides how many faces are **published**, and the catalog in `generated_lib.rs` is where
    // that number becomes text: one `StaticFace::new(…)` per planned face. Counting it is what makes this
    // pin bite — the module walk would filter the claim even with a broken plan, and the catalog carries
    // identities rather than source paths, so a substring search for the other subtree would prove
    // nothing (its first version searched for "beta" and passed under a projection that ignored the scope
    // entirely ✗).
    // 计划决定**发布**多少个面，而 `generated_lib.rs` 里的目录正是那个数字变成文本的地方：每个计划中的面一行
    // `StaticFace::new(…)`。数它才让这条钉子有牙——模块遍历即使计划坏了也会过滤认领，而目录携带的是身份而不是
    // 源码路径，因此"搜另一个子树的名字"什么也证明不了（它的第一版就是搜 "beta"，在一个完全无视 scope 的投影下
    // 也照样通过 ✗）。
    let published = |products: &std::collections::BTreeMap<String, Vec<u8>>| {
        String::from_utf8_lossy(
            products
                .get("generated_lib.rs")
                .expect("every build writes the generated tree"),
        )
        .matches("StaticFace::new(")
        .count()
    };
    assert!(
        published(&hit) < published(&host),
        "a fragment publishes only its claim: {} against the host's {}",
        published(&hit),
        published(&host)
    );
    assert!(
        published(&hit) > 0,
        "and its own claim is published: {:?}",
        hit.keys().collect::<Vec<_>>()
    );

    // The same shape without a record: the fragment must come out byte-identical, or the hit path is a
    // different build wearing the same name.
    // 同形状但没有记录：碎片必须逐字节相同，否则命中路径就是另一个同名构建。
    std::fs::remove_dir_all(manifest.join("target/xirang/records")).expect("drop the record");
    let full = root.join("full");
    crate::build_method::run_for_partition(
        &manifest,
        &full,
        "fragment-ghost",
        Some("alpha"),
        false,
    )
    .expect("the fragment build succeeds without a record");
    assert_eq!(
        product_bytes(&claimed),
        product_bytes(&full),
        "a fragment's record hit writes the same products as its full path"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **Concurrency** (`docs/design-discovery-record.md` §6): one package built twice at once — the only
/// shape that actually races, because a record lives under the tree **and its namespace**
/// (`<target>/xirang/records/<tree>/<namespace>/`), so two *different* packages publish into two different
/// directories and never meet. `check` followed by `cargo build` in one session is the everyday case.
/// **并发**（`docs/design-discovery-record.md` §6）：同一个包同时构建两次——这才是真正会撞的形状，因为记录住在
/// 树**与它的命名空间**之下（`<target>/xirang/records/<树>/<命名空间>/`），两个**不同**的包写的是两个不同目录、
/// 根本不会相遇。一个会话里先 `check` 再 `cargo build` 就是日常那一次。
///
/// A record is content-addressed and its `record.tsv` is written **last** (§7j), so the race must end with
/// one valid record and two successful builds — never with a half record the next run would read.
/// 记录是内容寻址的、`record.tsv` **最后**写（§7j），因此这场竞争必须收在"一份有效记录 + 两次成功构建"上——
/// 绝不能留下半份供下一次运行去读。
#[test]
fn one_package_building_twice_at_once_leaves_a_whole_record() {
    let (root, manifest) = host_root("two-publishers");
    let face = "crate::root_object! {\n    kind: Alpha,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n";
    let folder = manifest.join("src").join("alpha");
    std::fs::create_dir_all(&folder).expect("face folder");
    std::fs::write(folder.join("alpha.rs"), face).expect("write face");
    std::fs::write(manifest.join("src/lib.rs"), "// two-publisher host\n").expect("write entry");

    let first_out = root.join("first-out");
    let second_out = root.join("second-out");
    let outcomes = std::thread::scope(|scope| {
        let first =
            scope.spawn(|| crate::build_method::run_for(&manifest, &first_out, "publisher"));
        let second =
            scope.spawn(|| crate::build_method::run_for(&manifest, &second_out, "publisher"));
        (
            first.join().expect("the first thread"),
            second.join().expect("the second thread"),
        )
    });
    assert!(
        outcomes.0.is_ok(),
        "the first build published: {:?}",
        outcomes.0
    );
    assert!(
        outcomes.1.is_ok(),
        "the second build published: {:?}",
        outcomes.1
    );
    assert_eq!(
        product_bytes(&first_out),
        product_bytes(&second_out),
        "and both wrote the same products"
    );

    // What is on disk must be a **whole** record: the manifest names every payload and every payload file
    // is there. `record.tsv` is written last for exactly this reason, so its presence — and its
    // completeness — is the invariant this race is about.
    // 磁盘上留下的必须是一份**完整**记录：清单点名每一份载荷，而每份载荷文件都在。`record.tsv` 最后写正是为了
    // 这件事，因此它的存在——以及它的完整——就是这场竞争要守的不变量。
    let records = manifest.join("target/xirang/records");
    let manifests = std::fs::read_dir(&records)
        .expect("the record directory exists")
        .flatten()
        .flat_map(|tree| std::fs::read_dir(tree.path()).into_iter().flatten())
        .flatten()
        .map(|namespace| namespace.path().join("record.tsv"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    assert_eq!(
        manifests.len(),
        1,
        "one tree, one namespace, one record: {manifests:?}"
    );
    let manifest_text = std::fs::read_to_string(&manifests[0]).expect("the record manifest");
    for payload in crate::build_method::discovery_record::PAYLOADS {
        assert!(
            manifest_text.contains(payload),
            "the record names every payload, {payload} included:\n{manifest_text}"
        );
        assert!(
            manifests[0]
                .parent()
                .expect("the record directory")
                .join("products")
                .join(payload)
                .is_file(),
            "and the payload itself is on disk: {payload}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Two cuts naming the **same slot** leave exactly one row in the plan — the winner's.
/// 两条点名**同一个槽位**的切口在计划里只留下一行——赢家的那一行。
///
/// This is a regression pin for a defect measured 2026-10-18: the overlap decision names the later entry as the
/// winner and the earlier one as ignored, and the drop was keyed by **subtree**. Two cuts for one slot share their
/// subtree, so the drop removed the winner together with the entry it beat and the published plan carried **no row at
/// all** — `check` said "the later entry answers" and the plan said none did. The declaration is the only place a
/// reader learns a slot is grafted, so a plan that silently loses it ships an ungrafted slot while reporting success.
/// 这是 2026-10-18 实测缺陷的回归钉子：重叠判定点名后一条为赢家、前一条被忽略，而丢弃是**按子树**作键的。同一槽位的
/// 两条切口子树相同，于是丢弃把赢家连同它击败的那条一起删掉，发布的计划**一行都没有**——`check` 说"后一条作数"，计划
/// 说一条也不作数。声明是读者唯一得知某槽位被嫁接的地方，因此悄悄丢掉它的计划会让一个未嫁接的槽位带着"成功"发布出去。
#[test]
fn two_cuts_for_one_slot_leave_exactly_the_winners_row() {
    let (root, out) = fixture(
        "identical",
        "// the host entry\n\n\
         pub const GRAFTS: &[Graft] = &[\n\
         \x20   graft(\"root/button\").with(\"button_fast\"),\n\
         \x20   graft(\"root/button\").with(\"slider_fast\"),\n\
         ];\n",
    );
    check_for_shape_with_cuts(&root, &out, "cuts-host", None, false, false, &[])
        .expect("the pipeline accepts the host");
    let audit = std::fs::read_to_string(out.join("graft_plan.tsv")).expect("the audit text");
    let rows: Vec<&str> = audit
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "one slot, one row: the winner answers and the plan says so: {audit}"
    );
    assert!(
        rows[0].starts_with("root/button\t") && rows[0].contains("slider_fast"),
        "and the surviving row is the **later** declaration's: {audit}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A Rust face an explicit placement put outside `src/` is planned with the **record spelling** the
/// declaration macro hashes and mounted at the module chain the discovery tree gives it.
/// 显式落点放在 `src/` 之外的 Rust 面，用声明宏所散列的**记录拼写**进入计划，并挂在发现树给出的模块链上。
///
/// Two defects are pinned at once, because they were one wrong answer: the plan derived the source
/// from the identity base alone (a file outside it came back as its **absolute path**), and it
/// derived the module path from that same spelling (`::tmp::f3-e2e::…`, which is not even a valid
/// Rust path). The identity then disagreed with the compiled declaration, so the generated
/// `assert_static_identity` refused the build, and the module mount pointed at a path that does not
/// resolve.
/// 同时钉住两个缺陷，因为它们是同一个错误答案：计划只从身份基准推导源码（基准之外的文件回成**绝对路径**），
/// 又从同一份拼写推导模块路径（`::tmp::f3-e2e::…`，甚至不是合法的 Rust 路径）。身份于是与编译出的声明
/// 不一致，生成的 `assert_static_identity` 会拒绝构建，而模块挂载指向一个解析不到的路径。
#[test]
fn an_explicitly_placed_face_is_planned_with_its_record_spelling_and_module_chain() {
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-island-plan-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/panel/registry_rule")).expect("source tree");
    std::fs::create_dir_all(root.join("components/reading")).expect("island tree");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"island-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("entry");
    std::fs::write(
        root.join("src/panel/panel.rs"),
        "// generated-by=XiRang\n\
         #[xirang_toolchain::soil]\n\
         pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Panel>()\n\
         \x20   .needs_registry(true)\n\
         \x20   .under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\
         pub struct Panel;\n",
    )
    .expect("panel");
    std::fs::write(
        root.join("src/panel/registry_rule/registry_rule.rs"),
        "//! Registry rule.\nuse crate::RegistrationRule;\n\n\
         pub const REGISTRATION_RULE: RegistrationRule = crate::RegistrationRule::ANY;\n",
    )
    .expect("registry rule");
    std::fs::write(
        root.join("components/reading/reading.rs"),
        "// generated-by=XiRang\n\
         #[xirang_toolchain::soil]\n\
         pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Reading>()\n\
         \x20   .under(crate::panel::NODE_ID);\n\
         pub struct Reading;\n",
    )
    .expect("island");
    // The ledger is how the walk learns about a file outside its root.
    // 台账是遍历得知根之外那个文件的途径。
    std::fs::write(
        root.join("xirang.objects.tsv"),
        "# xirang objects v1\n\
         0123456789abcdef0123456789abcdef\troot/panel/reading\tcomponents/reading/reading.rs\tcomponents/reading\t-\n",
    )
    .expect("ledger");

    let out = root.join("out");
    crate::build_method::run_for(&root, &out, "island-host").expect("the host build succeeds");
    let generated =
        std::fs::read_to_string(out.join("generated_lib.rs")).expect("the generated tree");

    assert!(
        generated.contains("#[path = \"") && generated.contains("components/reading/reading.rs\"]"),
        "the island is mounted by its materialized path:\n{generated}"
    );
    assert!(
        generated.contains("pub mod reading;"),
        "it is mounted at its stem, the root-level node the walk placed:\n{generated}"
    );
    assert!(
        generated.contains(
            "assert_static_registration(panel::REGISTRATION.registry_rule, reading::REGISTRATION)"
        ) && generated.contains("assert_static_identity(crate::reading::NODE_ID"),
        "the assertions address the module chain, not the physical directories:\n{generated}"
    );
    assert!(
        !generated.contains("::tmp::") && !generated.contains("components::reading"),
        "no module path is derived from the absolute path or from the directories:\n{generated}"
    );
    assert_eq!(
        generated.matches("StaticFace::new(").count(),
        2,
        "the plan carries both faces:\n{generated}"
    );

    let expected = crate::build_method::registry_identity::NodeId::from_namespaced_path(
        "island-host",
        "components/reading/reading.rs",
        "Reading",
    );
    let bytes = expected.into_bytes();
    assert!(
        generated.contains(&format!("{bytes:?}")),
        "the island's identity is the one the declaration macro hashes for that spelling \
         (expected {expected}):\n{generated}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
