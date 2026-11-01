//! Tests for the published-record reader, and for the claim built on it: that a
//! member which published records is answered without deriving its faces.
//! 已发布记录读取器的测试，以及建立在它之上的那项声称：发布过记录的成员，其答案不推导面。
//!
//! The claim is the load-bearing half. "Read the records" is worth nothing if the
//! old derivation still runs underneath, so every test that says "published" also
//! reads [`derivations`] before and after, and asserts the count it expects. The
//! mutation is the point: restoring "always derive" makes these red rather than
//! making them slower in silence.
//! 那项声称才是承重的一半。如果旧的推导还在底下跑，"读记录"就一文不值，因此每一条说"已发布"的
//! 测试都会在前后读 [`derivations`] 并断言它期望的计数。变异正是要点：把"一律现推"改回来会让这些
//! 测试变红，而不是让它们无声地变慢。

use std::path::{Path, PathBuf};

use xirang_kernel::identity::NodeId;

use super::{Publication, derivations, read};

/// One throwaway package plus whatever published records the test wants under it.
/// 一个一次性包，以及测试想放在它下面的已发布记录。
struct Fixture {
    root: PathBuf,
    namespace: String,
}

impl Drop for Fixture {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A package with two faces under `src/` and no `target/` directory.
/// 一个 `src/` 下有两个面、没有 `target/` 目录的包。
fn fixture(label: &str) -> Fixture {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let namespace = format!("mcp-published-{label}");
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-published-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{namespace}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    for (module, kind) in [("button", "Button"), ("slider", "Slider")] {
        let file = root.join(format!("src/{module}/{module}.rs"));
        std::fs::create_dir_all(file.parent().expect("module directory")).expect("module dir");
        std::fs::write(
            &file,
            format!(
                "pub struct {kind};\n\ncrate::root_object! {{\n    kind: {kind},\n    parent: \
                 crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}}\n"
            ),
        )
        .expect("face");
    }
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("entry");
    Fixture { root, namespace }
}

impl Fixture {
    /// The identity the build would stamp on one of this fixture's faces.
    /// 构建会盖在本夹具某个面上的身份。
    fn id(&self, source: &str, kind: &str) -> NodeId {
        NodeId::from_namespaced_path(&self.namespace, source, kind)
    }

    /// Write one published record file under `target/xirang/out`.
    /// 在 `target/xirang/out` 下写一个已发布记录文件。
    fn publish(&self, name: &str, text: &str) {
        let out = self.root.join("target/xirang/out");
        std::fs::create_dir_all(&out).expect("out directory");
        std::fs::write(out.join(name), text).expect("record");
    }

    /// Publish the two records every build writes: the scope and the face manifest.
    /// 发布每次构建都会写的两份记录：作用域与面清单。
    ///
    /// The fingerprint is not published, so the reader sees records that are
    /// `stale` — which is exactly the state the third nail is about, and it cannot
    /// be faked into `current` because the freshness rule recomputes it from the
    /// sources.
    /// 指纹不被发布，因此读取方看到的是 `stale` 的记录——这正是第三枚钉子所针对的状态，而它无法被
    /// 伪装成 `current`，因为新鲜度规则会从源码重新计算它。
    fn publish_scope_and_faces(&self) {
        let button = self.id("button/button.rs", "Button");
        let slider = self.id("slider/slider.rs", "Slider");
        self.publish(
            "source_scope.tsv",
            &format!(
                "# mode\tauto\n# selected\t1\n# node\tsource\tmodule\n{button}\tbutton/button.rs\t\
                 button\n"
            ),
        );
        self.publish(
            "pruning_manifest.tsv",
            &format!(
                "# node\tsource\tsymbol\n{button}\tbutton/button.rs\t-\n{slider}\t\
                 slider/slider.rs\t-\n"
            ),
        );
    }

    /// Publish the record a **current** build writes: the ten columns audit `W3-1` left the
    /// pruning row carrying, so the answer can show the declaration facts instead of denying them.
    /// 发布一份**当前**构建写下的记录：审计 `W3-1` 之后剪枝行携带的那十列，于是答案能显示声明事实，
    /// 而不是否认它们。
    fn publish_current_record(&self) {
        let button = self.id("button/button.rs", "Button");
        let slider = self.id("slider/slider.rs", "Slider");
        self.publish(
            "source_scope.tsv",
            &format!(
                "# mode\tauto\n# selected\t2\n# node\tsource\tmodule\n{button}\tbutton/button.rs\t\
                 button\n{slider}\tslider/slider.rs\tslider\n"
            ),
        );
        self.publish(
            "pruning_manifest.tsv",
            &format!(
                "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls\tparent_node\towns_registry\tlogical_path\n\
                 {button}\tbutton/button.rs\t-\t-\tButton\tbutton\trc\t{hash}\t{fingerprint}\t-\t{parent}\tfalse\troot/button\n\
                 {slider}\tslider/slider.rs\t-\t-\tSlider\tslider\trc\t{hash}\t{fingerprint}\thelper\t{parent}\ttrue\troot/slider\n",
                hash = "a".repeat(64),
                fingerprint = "b".repeat(64),
                parent = "c".repeat(32),
            ),
        );
    }

    /// Publish the record a framework crate writes: every face in scope, and the
    /// build's own reason that there are none.
    /// 发布框架 crate 写的那份记录：每个面都在作用域内，以及构建自己给出的"一个都没有"的原因。
    fn publish_no_faces(&self) {
        self.publish(
            "source_scope.tsv",
            "# mode\tauto\n# result\tall\n# selected\tall\n# reason\tno-registration-face\n",
        );
        self.publish("pruning_manifest.tsv", "# node\tsource\tsymbol\n");
    }
}

/// A readable `source_scope.tsv` is what "this member was built" means, and its
/// rows survive the round trip: identities from the manifest, the scope's own
/// verdict, and the freshness word.
/// 可读的 `source_scope.tsv` 就是"这个成员被构建过"的含义，而它的行经得起往返：来自清单的身份、
/// 作用域自己的结论，以及新鲜度词。
#[test]
fn a_published_package_reads_its_scope_and_face_rows() {
    let fixture = fixture("reads");
    fixture.publish_scope_and_faces();
    let Publication::Published(tree) = read(&fixture.root) else {
        panic!("a readable source_scope.tsv is a published member");
    };
    assert_eq!(tree.faces().len(), 2, "both manifest rows are faces");
    assert!(
        tree.faces()
            .iter()
            .any(|row| row.id == fixture.id("button/button.rs", "Button"))
    );
    assert!(tree.selected(&tree.faces()[0]) || tree.selected(&tree.faces()[1]));
    assert_eq!(
        tree.freshness(),
        "build stale (run `xirang check`)",
        "a record with no published fingerprint does not describe these sources"
    );
    assert!(
        tree.evidence_line()
            .contains("discovery.fingerprint absent")
    );
    let _ = &fixture.namespace;
}

/// The third nail: records exist and the freshness does not match, so the answer
/// says `stale` rather than presenting them as this tree.
/// 第三枚钉子：记录存在而新鲜度不符，因此答案说 `stale`，而不是把它们当成眼前这棵树。
#[test]
fn records_that_do_not_describe_these_sources_are_reported_stale() {
    let fixture = fixture("stale");
    fixture.publish_scope_and_faces();
    let report = crate::mcp::registry::registry(&fixture.root).expect("a published member answers");
    assert!(
        report.contains("tree published from"),
        "the answer names its evidence: {report}"
    );
    assert!(
        report.contains("build stale (run `xirang check`)"),
        "the freshness rule the records already have must be stated: {report}"
    );
    // The note was rewritten on 2026-10-04: it used to say `path`/`kind`/`registry_name`/`parent`
    // "are derived facts and are not in the record", which stopped being true on 2026-09-29 and was
    // a self-description running **behind** the behaviour. What it has to name now is both halves —
    // what the record carries and what a reader still has to derive.
    // 这句话 2026-10-04 重写了：它过去说 `path`/`kind`/`registry_name`/`parent`"是推导事实、不在记录
    // 里"，而那句从 2026-09-29 起就不成立，是一句**落后于**行为的自述。现在它要说两半——记录携带什么、
    // 读者还要推导什么。
    assert!(
        report.contains("note: these are the build's **published** rows")
            && report.contains("the declaration facts the record carries")
            && report.contains("A face added since the build is not in the record at all"),
        "the note names both what the record carries and what it does not: {report}"
    );
}

/// The first nail: a member with published records is answered **without**
/// deriving its faces. The thread-local counter is the observable, and the
/// mutation that restores "always derive" makes this red.
/// 第一枚钉子：发布过记录的成员，其答案**不**推导面。线程局部计数器就是那个可观测量，而把
/// "一律现推"改回来的变异会让它变红。
#[test]
fn a_published_member_is_answered_without_deriving_its_faces() {
    let fixture = fixture("no-derive");
    fixture.publish_scope_and_faces();
    let before = derivations();
    let report = crate::mcp::registry::registry(&fixture.root).expect("a published member answers");
    assert_eq!(
        derivations(),
        before,
        "a member with readable records must not derive its faces: {report}"
    );
    assert!(
        report.contains("tree published from"),
        "the answer must say it came from the published records: {report}"
    );
    assert!(
        !report.contains("root/button"),
        "the record carries no logical path, so the answer must not claim one: {report}"
    );
}

/// The second nail: a member with no records reports `not built` and shows the
/// tree it derived now, rather than an empty tree.
/// 第二枚钉子：没有记录的成员报 `not built` 并展示它现推的树，而不是一棵空树。
#[test]
fn a_member_without_records_reports_not_built_and_still_shows_its_tree() {
    let fixture = fixture("not-built");
    let before = derivations();
    let report =
        crate::mcp::registry::registry(&fixture.root).expect("the derivation still answers");
    assert!(
        report.contains("tree derived now (no published records at"),
        "the answer must say it derived, and why: {report}"
    );
    assert!(
        report.contains("root/button"),
        "the derived tree is the answer here, not an empty one: {report}"
    );
    assert!(
        derivations() > before,
        "nothing was published, so this member really was derived"
    );
}

/// A framework crate whose record says `no-registration-face` reports `no faces`
/// with the build's own reason — the record's answer, not a missing tree.
/// 记录写着 `no-registration-face` 的框架 crate 报 `no faces` 并附构建自己的原因——那是记录给出的
/// 答案，而不是一棵缺失的树。
#[test]
fn a_record_of_no_faces_is_an_answer_rather_than_an_empty_tree() {
    let fixture = fixture("no-faces");
    fixture.publish_no_faces();
    let before = derivations();
    let report =
        crate::mcp::registry::registry(&fixture.root).expect("a record of no faces answers");
    assert_eq!(
        derivations(),
        before,
        "the record already answers, so nothing may be derived: {report}"
    );
    assert!(
        report.contains("faces 0") && report.contains("no registration face is recorded"),
        "{report}"
    );
}

/// The census keeps counting every member, and now says which of them were
/// answered from their own records: one published member, one without records,
/// and exactly one derivation — the unbuilt one.
/// 普查仍然数到每一个成员，并且现在说出其中哪些是由它们自己的记录作答的：一个发布过的成员、一个
/// 没有记录的成员，以及恰好一次推导——没构建过的那个。
#[test]
fn a_merged_answer_names_the_evidence_of_every_member() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-published-census-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    fixture_file(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"built\", \"unbuilt\"]\nresolver = \"2\"\n",
    );
    let built = member(&root, "built", "census-built");
    let unbuilt = member(&root, "unbuilt", "census-unbuilt");
    built.publish_scope_and_faces();

    let before = derivations();
    let report = crate::mcp::registry::registry(&root).expect("a workspace root answers");
    assert_eq!(
        derivations(),
        before + 1,
        "only the member with no records may be derived: {report}"
    );
    assert!(
        report.contains("published 1  not built 1"),
        "the census must tell the two apart: {report}"
    );
    assert!(
        report.contains(&format!("== {} (published)\n", built.namespace)),
        "{report}"
    );
    assert!(
        report.contains(&format!("== {} (not built)\n", unbuilt.namespace)),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// One member of a throwaway workspace: a package with one face and no records.
/// 一个一次性工作区的成员：一个有一个面、没有记录的包。
fn member(workspace: &Path, directory: &str, name: &str) -> Fixture {
    let root = workspace.join(directory);
    let fixture = Fixture {
        root,
        namespace: name.to_owned(),
    };
    fixture_file(
        &fixture.root.join("Cargo.toml"),
        &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    );
    fixture_file(
        &fixture.root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: \
         crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    fixture_file(&fixture.root.join("src/lib.rs"), "// host entry\n");
    fixture
}

/// Lay down one fixture file, creating its directory.
/// 放下一份夹具文件并建好它的目录。
///
/// Named after what it makes rather than after the verb: a bare `write` outside an
/// entry position is what the workspace's verb table refuses, and the table is right
/// — this does not write through a `Write`.
/// 按它造出的东西命名而不是按动词命名：入口位之外的裸 `write` 正是本工作区动词表拒绝的东西，
/// 而那张表是对的——这里并不经 `Write` 写出。
fn fixture_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    std::fs::write(path, text).expect("fixture file");
}

/// The fourth nail, and the one with a number in it: on a member that really
/// published records, reading them costs a fraction of deriving the same member.
/// 第四枚钉子，也是带数字的那一枚：在真的发布过记录的成员上，读记录只花掉推导同一个成员的一小部分。
///
/// The member is `examples/control-button`, named relatively to this checkout
/// (`CARGO_MANIFEST_DIR`), because a record set cannot be fabricated into
/// `current` — the freshness rule recomputes the fingerprint from the sources —
/// and the point of the number is the *published* path, not a fixture. A checkout
/// that has never built that example has no records to time, so the test says so
/// and stops rather than inventing one.
/// 这个成员是 `examples/control-button`，相对本检出点名（`CARGO_MANIFEST_DIR`），因为记录集无法被
/// 伪装成 `current`——新鲜度规则会从源码重新计算指纹——而这个数字的意义在于**已发布**那条路径，
/// 不是夹具。从未构建过那个示例的检出没有记录可计，因此本测试说出来并停下，而不是造一份。
#[test]
fn reading_published_records_costs_less_than_deriving_the_same_member() {
    let Some(root) = checkout_member("examples/control-button") else {
        return;
    };
    let namespace = crate::build_method::package_name(&root.join("Cargo.toml"))
        .expect("the example package has a name Cargo reports");
    let Publication::Published(_) = read(&root) else {
        println!(
            "skipped: {} has no published records to time",
            root.display()
        );
        return;
    };

    // Three rounds each, so one scheduling hiccup cannot decide the comparison.
    // 各跑三轮，因此一次调度抖动决定不了这次比较。
    let mut records = std::time::Duration::ZERO;
    let mut derived = std::time::Duration::ZERO;
    let mut faces = 0usize;
    for _ in 0..3 {
        let start = std::time::Instant::now();
        let reading = read(&root);
        records += start.elapsed();
        let start = std::time::Instant::now();
        let tree = crate::mcp::resolve::derived_faces(&root, &namespace).expect("the tree derives");
        derived += start.elapsed();
        faces = tree.0.len();
        assert!(matches!(reading, Publication::Published(_)));
    }
    println!(
        "{}: reading published records {records:?} vs deriving {faces} faces {derived:?}",
        root.display()
    );
    assert!(
        records < derived,
        "reading the build's own records ({records:?}) must cost less than deriving the same \
         member ({derived:?})"
    );
}

/// One member of the checkout this test was compiled in, or `None` when it is not
/// there — a crate can be copied without its examples.
/// 本测试被编译进的那个检出里的一个成员；它不在时返回 `None`——一个 crate 可以被复制到没有示例的
/// 地方。
fn checkout_member(relative: &str) -> Option<PathBuf> {
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
    let root = checkout.join(relative);
    root.join("Cargo.toml").is_file().then_some(root)
}

/// A readable scope with an unreadable face manifest is `faces unknown`, not `no
/// faces`: this member *was* built, and turning a missing file into a claim about
/// the package is the failure the two states are kept apart to prevent.
/// 作用域可读、面清单读不了时是 `faces unknown` 而不是 `no faces`：这个成员**确实**被构建过，而把
/// 一个缺失的文件变成一个关于包的断言，正是区分这两个状态要阻止的失败。
#[test]
fn an_unreadable_face_manifest_is_unknown_rather_than_empty() {
    let fixture = fixture("manifest-gone");
    fixture.publish(
        "source_scope.tsv",
        "# mode\tauto\n# selected\t0\n# node\tsource\tmodule\n",
    );
    let report =
        crate::mcp::registry::registry(&fixture.root).expect("a scope with no manifest answers");
    assert!(
        report.contains("faces unknown (no readable pruning_manifest.tsv"),
        "{report}"
    );
    assert!(
        !report.contains("no registration face is recorded"),
        "an unreadable manifest must not be reported as a package with no faces: {report}"
    );
}

/// The published rows show the declaration facts the record carries, and the census counts levels.
/// 已发布的行显示记录携带的声明事实，而普查按层计数。
///
/// Audit `W2-1`/`W3-1` together: the published path used to print one unbounded row per face and to
/// say in a note that `path`/`kind`/`registry_name`/`parent` were "not in the record" — while the
/// record had published them since 2026-09-29. Both halves are pinned here: the default is a census
/// counted from the **published** logical path, and `full: true` shows those columns.
/// 审计 `W2-1`/`W3-1` 合起来：发布路径过去每个面无上限地印一行，并在注里说
/// `path`/`kind`/`registry_name`/`parent`"不在记录里"——而记录从 2026-09-29 起就发布了它们。两半都在
/// 这里钉住：默认是按**已发布的**逻辑路径计数的普查，`full: true` 显示那几列。
#[test]
fn a_current_record_is_shown_and_its_levels_are_counted() {
    let fixture = fixture("current-record");
    fixture.publish_current_record();
    // `registry_brief` is the **default** the tool answers with (`registry` is the full one).
    // `registry_brief` 是工具**默认**作答的那一支（`registry` 是完整的那一支）。
    let census =
        crate::mcp::registry::registry_brief(&fixture.root).expect("a published member answers");
    // The census prints **level prefixes**, so a fixture whose two faces both hang off the root
    // shows one level holding both of them — the point is that the level came from the published
    // `logical_path` and not from the source directory (which would have been two levels).
    // 普查印的是**层级前缀**，因此两个面都挂在根上的夹具显示一层、两个面——要点是这个层级来自**已发布的**
    // `logical_path`，而不是源码目录（那样会是两层）。
    assert!(
        census.contains("level (logical path prefix)")
            && census.contains("  root  ")
            && census.contains("across 1 level(s)")
            && census.contains("`full: true` prints the rows, 200 per page"),
        "the census counts the published logical paths and names the way to the rows: {census}"
    );
    assert!(
        !census.contains("  selected     "),
        "the default prints no row: {census}"
    );

    let full =
        crate::mcp::registry::registry_page(&fixture.root, 0, 200).expect("the page answers");
    assert!(
        full.contains("rows 1-2 of 2")
            && full.contains("path=-")
            && full.contains("logical_path=root/button")
            && full.contains("kind=Button")
            && full.contains("registry_name=button")
            && full.contains("parent=rc")
            && full.contains("owns_registry=true")
            && full.contains("owns_registry=false"),
        "the rows show the declaration facts the record carries: {full}"
    );
    assert!(
        full.contains("calls=helper") && full.contains("calls=-"),
        "and the direct calls audit `W3-1` added, `-` where a file calls nothing: {full}"
    );
    assert!(
        full.contains("`source_hash`, `fields` and `parent_node` are published **in**")
            && full.contains("not reprinted per row")
            && full.contains("is not in the record at all"),
        "the note names what is published in the file and what the record cannot answer: {full}"
    );
    let _ = &fixture.namespace;
}

/// The facts a `registry` answer carries, with the provenance lines dropped.
/// 一条 `registry` 答案所携带的事实，去掉来源那些行。
///
/// The two paths **cannot** be byte-identical, and the reason is a requirement rather than a defect:
/// the checklist asks a published answer to declare `tree published from …` and a derived one to say
/// it derived. So the M3 gate's "答案一致" is read as the facts agreeing — the total and the level
/// counts — which is what this folds out and what the caller compares.
/// 两条路**不可能**逐字节相同，而原因是要求而不是缺陷：清单要发布答案声明 `tree published from …`、
/// 要推导答案说出它推导了。因此 M3 门禁的"答案一致"读作**事实**一致——总数与逐层计数——也就是这里折出来
/// 交给调用方比对的东西。
fn facts(answer: &str) -> Vec<String> {
    answer
        .lines()
        .filter(|line| line.starts_with("faces ") || line.starts_with("  "))
        .map(str::to_owned)
        .collect()
}

/// A published answer and a derived answer to the same question carry the same facts.
/// 对同一个问题，发布答案与推导答案携带相同的事实。
///
/// This is M3's own gate, taken at the scale a test can afford (`t2` in the checklist's language is
/// the same question with a bigger tree). It is the pin that makes the record path *worth* taking: a
/// fast answer that quietly said something else would be worse than the slow one it replaced.
/// 这就是 M3 自己的门禁，取测试付得起的规模（清单里的 `t2` 是同一件事、树更大）。它是让记录路径**值得**
/// 走的那枚钉子：一个悄悄说了别的事的快答案，比它替换掉的慢答案更糟。
#[test]
fn a_published_answer_and_a_derived_answer_carry_the_same_facts() {
    let fixture = fixture("agreement");
    fixture.publish_current_record();
    let published =
        crate::mcp::registry::registry_brief(&fixture.root).expect("the published answer");
    assert!(
        published.contains("tree published from"),
        "the published answer declares its evidence: {published}"
    );

    // The same tree with its records taken away: the reader derives, which is the fallback the
    // acceptance compares against.
    // 同一棵树、记录拿走：读取方推导——正是验收要比对的那条回退路。
    for record in ["pruning_manifest.tsv", "source_scope.tsv"] {
        std::fs::remove_file(fixture.root.join("target/xirang/out").join(record))
            .expect("the record is removed");
    }
    let derived = crate::mcp::registry::registry_brief(&fixture.root).expect("the derived answer");
    assert!(
        derived.contains("tree derived now"),
        "the derived answer declares that it derived: {derived}"
    );

    assert_eq!(
        facts(&published),
        facts(&derived),
        "the two paths answer the same question the same way:\npublished:\n{published}\nderived:\n{derived}"
    );
    assert_ne!(
        published, derived,
        "and they are not byte-identical, because each has to say where it came from"
    );
    let _ = &fixture.namespace;
}
