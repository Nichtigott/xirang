//! Tests for the discovery record: what a hit is trusted for, and every way a record stops being one.
//! 发现记录的测试：一次命中被信任到什么程度，以及一份记录以哪些方式不再是记录。

use super::write_faces_manifest;

/// The layout these fixtures describe: a package root with the historical `src/` scan root.
/// 这些夹具所描述的布局：包根加历史 `src/` 扫描根。
fn layout_of(root: &std::path::Path, src: &std::path::Path) -> crate::source_layout::SourceLayout {
    crate::source_layout::SourceLayout {
        package_root: root.to_path_buf(),
        scan_root: src.to_path_buf(),
        identity_base: src.to_path_buf(),
        target: None,
    }
}
use std::path::{Path, PathBuf};

use crate::build_method::test_support::temporary_directory;

use super::{
    PAYLOADS, PUBLISH_PAUSED, SCHEMA, copy_products, directory, load, publish, read_file_rows,
};

/// A tree root and an output directory with the five payloads written as a full run would write them.
/// 一棵树的根，以及一个按全量运行的样子写下五份载荷的输出目录。
fn full_run(label: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-record-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let out_dir = root.join("target/xirang/out");
    std::fs::create_dir_all(&out_dir).expect("the output directory");
    // The root looks like a package so `cache_directory`'s resolution has one to read.
    // 让这个根看起来像一个包，好让 `cache_directory` 的解析有东西可读。
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"record-probe\"\n",
    )
    .expect("the manifest");
    for (index, name) in PAYLOADS.iter().enumerate() {
        std::fs::write(
            out_dir.join(name),
            format!("# header\nrow\t{index}\t{name}\n"),
        )
        .expect("a payload");
    }
    (root, out_dir)
}

/// The namespace every fixture publishes and reads under, spelled once.
/// 每个夹具发布与读取所用的命名空间，只写一次。
const NAMESPACE: &str = "record-probe";

fn fingerprint() -> String {
    xirang_kernel::sha256_hex(b"a tree")
}

/// The point of the module: what one run publishes, the next one reads and can copy byte for byte.
/// 本模块存在的理由：一次运行发布的，下一次读得到、而且能逐字节复制。
#[test]
fn a_published_record_is_read_back_byte_for_byte() {
    let (root, out_dir) = full_run("roundtrip");
    publish(&root, &fingerprint(), NAMESPACE, &out_dir).expect("the record publishes");
    let record = load(&root, &fingerprint(), NAMESPACE).expect("the record verifies");
    assert_eq!(record, directory(&root, &fingerprint(), NAMESPACE));

    // A second package's output directory, as empty as a fresh build's.
    let second = root.join("second/target/xirang/out");
    std::fs::create_dir_all(&second).expect("the second output directory");
    copy_products(&record, &second).expect("the products copy");
    for name in PAYLOADS {
        let original = std::fs::read(out_dir.join(name)).expect("the original");
        let copied = std::fs::read(second.join(name)).expect("the copy");
        assert_eq!(original, copied, "{name} is copied byte for byte");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The freshness rule: another tree's record is not this tree's, whatever is on disk.
/// 新鲜性规则：别的树的记录不是这棵树的，盘上有什么都不算。
#[test]
fn a_record_for_another_tree_is_not_used() {
    let (root, out_dir) = full_run("other-tree");
    let fingerprint = fingerprint();
    publish(&root, &fingerprint, NAMESPACE, &out_dir).expect("the record publishes");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_some(),
        "its own tree hits"
    );
    assert!(
        load(
            &root,
            &xirang_kernel::sha256_hex(b"another tree"),
            NAMESPACE
        )
        .is_none(),
        "a changed tree means the full path, not this record"
    );
    // A record **copied into this tree's place** — the directory name says this tree, the record's own
    // line says another one. The directory alone would be satisfied by a copy, which is why the line is
    // checked as well; this case is what makes that second check load-bearing.
    // 一份**被复制到这棵树位置上的**记录——目录名说这棵树、记录自己的行说另一棵。只看目录名的话，一次复制就能
    // 满足它，因此那一行也要查；正是这个情形让第二道检查成为承重的。
    let record_tsv = directory(&root, &fingerprint, NAMESPACE).join("record.tsv");
    let copied = std::fs::read_to_string(&record_tsv)
        .expect("the record file")
        .replace(&fingerprint, &xirang_kernel::sha256_hex(b"another tree"));
    std::fs::write(&record_tsv, copied).expect("the misplaced record");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_none(),
        "a record whose own line names another tree is not this tree's"
    );
    let restored = std::fs::read_to_string(&record_tsv)
        .expect("the record file")
        .replace(&xirang_kernel::sha256_hex(b"another tree"), &fingerprint);
    std::fs::write(&record_tsv, restored).expect("the record is restored");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_some(),
        "and its own line is accepted"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Three ways a payload stops being evidence: a flipped byte, a truncation, and a missing payload.
/// 载荷不再是证据的三种方式：改一个字节、截断、以及缺一份。
#[test]
fn a_damaged_or_incomplete_record_is_not_trusted() {
    let (root, out_dir) = full_run("damaged");
    let fingerprint = fingerprint();
    publish(&root, &fingerprint, NAMESPACE, &out_dir).expect("the record publishes");
    let record = directory(&root, &fingerprint, NAMESPACE);
    let payload = record.join("products/pruning_manifest.tsv");
    let original = std::fs::read(&payload).expect("the payload");

    // A flipped byte inside the payload: the size still matches, so only the digest can catch it.
    // 载荷里改一个字节：字节数还对得上，因此只有摘要能抓住它。
    let mut flipped = original.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 0x01;
    std::fs::write(&payload, &flipped).expect("the flipped payload");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_none(),
        "a payload whose digest moved is not a record"
    );

    // A truncated payload: the size differs and the digest differs with it.
    // 被截断的载荷：字节数与摘要一起变了。
    std::fs::write(&payload, &original[..original.len() / 2]).expect("the truncated payload");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_none(),
        "a truncated payload is not a record"
    );

    // A missing payload: a record that would answer "this tree wrote no such file" is not a smaller
    // record, it is a wrong one.
    // 缺一份载荷：一份会答"这棵树没写过那个文件"的记录不是更小的记录，而是错的记录。
    std::fs::write(&payload, &original).expect("the payload is restored");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_some(),
        "restored ⇒ trusted"
    );
    std::fs::remove_file(&payload).expect("the payload is removed");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_none(),
        "a payload the schema promises but the record does not carry is not a record"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A half-published record is not read: the fingerprint line is what a reader trusts, and it is written
/// last — so a run that died before it left payloads nobody will trust.
/// 只发布了一半的记录读不到：读取方信任的是指纹行，而它最后才写——因此在它之前就死掉的运行留下的载荷没人会信。
#[test]
fn a_half_published_record_is_not_read() {
    let (root, out_dir) = full_run("half");
    let fingerprint = fingerprint();
    publish(&root, &fingerprint, NAMESPACE, &out_dir).expect("the record publishes");
    let record = directory(&root, &fingerprint, NAMESPACE);
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_some(),
        "the complete record hits"
    );

    let record_tsv = std::fs::read_to_string(record.join("record.tsv")).expect("the record file");
    // As a run that died mid-write leaves it: payloads on disk, no fingerprint line yet.
    // 一次写到一半就死掉的运行留下的样子：载荷在盘上、还没有指纹行。
    std::fs::write(record.join("record.tsv"), format!("# schema\t{SCHEMA}\n"))
        .expect("the partial record file");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_none(),
        "a record without its tree line is not a record"
    );
    // And a foreign schema version is refused for the same reason: it is another shape of record.
    // 而陌生的 schema 版本同理被拒：那是另一种形状的记录。
    std::fs::write(
        record.join("record.tsv"),
        record_tsv.replace(SCHEMA, "xirang-record-0"),
    )
    .expect("the foreign record file");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_none(),
        "another schema version is another record shape"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The rows read back out of a published payload are the rows the writer wrote.
/// 从已发布载荷里读回的行，就是写入方写下的那些行。
#[test]
fn the_file_rows_read_back_are_the_rows_written() {
    let (root, out_dir) = full_run("rows");
    std::fs::write(
        out_dir.join("file_manifest.tsv"),
        "# header\ncontrol/control.rs\tControl::render\t-\ncontrol/control.rs\t-\t-\n",
    )
    .expect("the manifest");
    let rows = read_file_rows(&out_dir).expect("the rows read");
    assert_eq!(
        rows,
        vec![
            (
                "control/control.rs".to_owned(),
                "Control::render".to_owned(),
                "-".to_owned()
            ),
            (
                "control/control.rs".to_owned(),
                "-".to_owned(),
                "-".to_owned()
            ),
        ],
        "the header is skipped and every column survives the round trip"
    );
    assert!(
        read_file_rows(Path::new("/nonexistent")).is_err(),
        "a missing manifest is an error, not an empty row set"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Two packages publishing the same tree's record at once: neither may leave a half record behind.
/// 两个包同时发布同一棵树的记录：谁都不许留下半份记录。
///
/// The design asks for this one by name (§6.4). What makes it true is the order — payloads first,
/// `record.tsv` last — plus one file at a time being written through a process-unique temporary and
/// renamed. A reader therefore sees either no record or a complete one, and two publishers of the same
/// tree are writing identical bytes anyway, so the later one cannot damage the earlier one's payloads.
/// 设计要求点名了这一条（§6.4）。让它成立的是那个顺序——先载荷、后 `record.tsv`——加上每份文件都经进程唯一的
/// 临时名再改名。因此读取方要么看不到记录、要么看到完整的一份；而同一棵树的两个发布方写的是同样的字节，后到的
/// 那个也伤不到先到的载荷。
#[test]
fn two_publishers_at_once_leave_no_half_record() {
    let (root, first) = full_run("concurrent-a");
    let second = root.join("second/target/xirang/out");
    std::fs::create_dir_all(&second).expect("the second output directory");
    for name in PAYLOADS {
        std::fs::copy(first.join(name), second.join(name)).expect("the same payloads land twice");
    }
    let fingerprint = fingerprint();
    std::thread::scope(|scope| {
        for out_dir in [&first, &second] {
            let root = &root;
            let fingerprint = &fingerprint;
            scope.spawn(move || {
                for _ in 0..8 {
                    publish(root, fingerprint, NAMESPACE, out_dir).expect("the record publishes");
                }
            });
        }
    });
    let record = load(&root, &fingerprint, NAMESPACE).expect("a complete record is left behind");
    for name in PAYLOADS {
        let published = std::fs::read(record.join("products").join(name)).expect("a payload");
        let expected = std::fs::read(first.join(name)).expect("the same payload");
        assert_eq!(
            published, expected,
            "{name} survived two publishers without being damaged"
        );
    }
    // And no temporary files are left claiming to be records.
    // 而且没有临时文件留下来冒充记录。
    let strays = std::fs::read_dir(record.join("products"))
        .expect("the products directory")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
        .count();
    assert_eq!(strays, 0, "the temporaries are cleaned up");
    let _ = std::fs::remove_dir_all(&root);
}

/// Two namespaces over one tree do not share a record: identities are a function of the namespace.
/// 同一棵树上的两个命名空间不共用记录：身份是命名空间的函数。
///
/// This is the pin for the defect the record's first version had, and an existing test found it before
/// this pin existed: `mcp::verify`'s namespace-override pin read the default run's record under an
/// override and reported `re-identified 1` for a tree nothing had touched — every identity in the
/// handed-over rows "moved" ✗. The namespace is therefore part of the key **and** stamped in the record
/// line, for the same reason `graph.generation` stamps it.
/// 这是记录第一版那个缺陷的钉子，而在它存在之前是一条既有测试先抓到的：`mcp::verify` 的命名空间覆盖钉子
/// 在覆盖之下读到了默认那次运行的记录，对一棵没人碰过的树报 `re-identified 1`——交出去的行里每个身份都"搬"了 ✗。
/// 因此命名空间既是键的一部分、又盖在记录行里，理由与 `graph.generation` 盖它相同。
#[test]
fn another_namespace_is_another_record() {
    let (root, out_dir) = full_run("namespace");
    let fingerprint = fingerprint();
    publish(&root, &fingerprint, NAMESPACE, &out_dir).expect("the record publishes");
    assert!(
        load(&root, &fingerprint, NAMESPACE).is_some(),
        "the publishing namespace hits"
    );
    assert!(
        load(&root, &fingerprint, "another-package").is_none(),
        "and every other namespace runs the full path"
    );
    // A record **moved into** another namespace's place is refused by its own namespace line too.
    // 一份被**搬进**别的命名空间位置的记录，同样被它自己的命名空间行拒绝。
    let moved = directory(&root, &fingerprint, "another-package");
    std::fs::create_dir_all(&moved).expect("the other namespace directory");
    copy_products(&directory(&root, &fingerprint, NAMESPACE), &moved).expect("the payloads copy");
    std::fs::copy(
        directory(&root, &fingerprint, NAMESPACE).join("record.tsv"),
        moved.join("record.tsv"),
    )
    .expect("the record file copies");
    assert!(
        load(&root, &fingerprint, "another-package").is_none(),
        "its namespace line says whose it is"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The faces payload is built **without parsing a face**: every column is read out of the unit, and the
/// two things a projection must not get wrong are pinned here — a face that declares `plugin:` has no
/// row (the plan leaves it out, so the absence *is* that filter's answer), and a column that contains a
/// tab or a newline cannot move the columns (`stem 2`, step 3).
/// 逐面载荷**不解析任何面**就能写出：每一列都从 unit 读出，而投影绝不能弄错的两件事钉在这里——声明了
/// `plugin:` 的面没有行（计划不收录它，因此"行不存在"就是那道过滤的答案），以及含制表符或换行的列不能
/// 挪动列（`stem 2` 第③步）。
#[test]
fn the_faces_payload_is_read_out_of_the_units_and_keeps_its_columns() {
    let root = temporary_directory("faces-payload");
    let units = root.join("units");
    std::fs::create_dir_all(&units).expect("fixture units directory");
    let src = root.join("src");
    std::fs::create_dir_all(&src).expect("fixture source directory");
    // A unit file is named by its source's fingerprint — that is how the append-only directory keeps
    // old versions of one path apart, and what the freshness rule reads back.
    // unit 文件以它那份源码的指纹命名——只追加的目录正是靠这个把同一路径的旧版本分开，而新鲜性规则读的也是它。
    let unit = |relative: &str, body: &str| {
        let source = src.join(relative);
        std::fs::create_dir_all(source.parent().expect("a parent")).expect("fixture source tree");
        std::fs::write(&source, format!("// {relative}\n")).expect("fixture source");
        let name = crate::build_method::identity_cache::source_unit_fingerprint(&source);
        std::fs::write(units.join(format!("{name}.tsv")), body).expect("fixture unit");
    };
    unit(
        "control/button/button.rs",
        "# schema\t6\npath\tcontrol/button/button.rs\nnode\tabc\nkind\tButton\n\
         parent\tdef\nvalue\tneeds_registry\tbool\ttrue\n\
         field\tcfg\t7\tfeature = \"one\\ttwo\"\n",
    );
    unit(
        "control/shelf/shelf.rs",
        "# schema\t6\npath\tcontrol/shelf/shelf.rs\nnode\tghi\nkind\tShelf\n\
         field\tplugin\t3\tcrate::Slot\n",
    );
    let layout = layout_of(&root, &src);
    let written = write_faces_manifest(&layout, &units, &root).expect("the payload is written");
    assert_eq!(written, 1, "the plugin face contributes no row");
    let text = std::fs::read_to_string(root.join("faces_manifest.tsv")).expect("the payload");
    let row = text
        .lines()
        .find(|line| line.starts_with("face\t"))
        .expect("one face row");
    let columns: Vec<&str> = row.split('\t').collect();
    assert_eq!(
        columns.len(),
        11,
        "eleven fields — the `face` marker plus relative, id, parent, owns, cfg, line, spelling, \
         kind, preset and parts — whatever the cfg text holds: {row}"
    );
    assert_eq!(
        columns[6], "7",
        "the declaration's line rides along, because the cfg gate's diagnostic points at it: {row}"
    );
    assert_eq!(columns[1], "control/button/button.rs");
    assert_eq!(columns[2], "abc");
    assert_eq!(columns[3], "def");
    assert_eq!(columns[4], "true", "owns_registry rides along");
    assert_eq!(
        columns[5], "feature = \"one\\ttwo\"",
        "a tab inside the cfg text is escaped, not a column break: {row}"
    );
    std::fs::remove_dir_all(&root).expect("temporary fixture cleanup");
}

/// The manifest is written **last**, and that rule is now checkable rather than merely structural: with the
/// publish paused between the payloads and `record.tsv`, a reader must answer `None` — not a record it
/// would then trust without its payloads. Flipping the order in `publish` turns this red (measured).
/// 清单**最后**写，而这条规则现在**可被检查**、不再只是结构上的：把发布暂停在载荷与 `record.tsv` 之间，
/// 读取方必须答 `None`——而不是一份它随后会连载荷都没有就信任的记录。把 `publish` 里的顺序翻过来，这条就红
/// （实测过）。
#[test]
fn a_reader_inside_the_publish_window_sees_no_record_at_all() {
    let root = temporary_directory("publish-window");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("out dir");
    for payload in PAYLOADS {
        std::fs::write(out.join(payload), format!("{payload} contents\n")).expect("payload");
    }
    PUBLISH_PAUSED.with(|paused| paused.set(true));
    let published = publish(&root, "fingerprint", "namespace", &out);
    let during = load(&root, "fingerprint", "namespace");
    PUBLISH_PAUSED.with(|paused| paused.set(false));
    published.expect("the paused publish still reports success");
    assert!(
        during.is_none(),
        "inside the window there is no record to read: {during:?}"
    );

    // And once it finishes, the same reader finds one — so the pause is a window, not a failure mode.
    // 而它写完之后，同一个读取方就能找到一份——因此这个暂停是一个**窗口**，不是一种失败模式。
    let after = publish(&root, "fingerprint", "namespace", &out).and_then(|()| {
        load(&root, "fingerprint", "namespace").ok_or_else(|| "no record".to_owned())
    });
    assert!(
        after.is_ok(),
        "the record verifies after the manifest lands: {after:?}"
    );
    std::fs::remove_dir_all(&root).expect("temporary fixture cleanup");
}

/// The freshness rule reaches the units directory too: a unit whose name is not the **current**
/// fingerprint of the source it names describes a version of that file nobody is compiling, and the
/// payload must not carry it. Without this, the append-only directory lets an old unit answer for a
/// path whose file has changed — measured: a promote rewrote one face file, `Widget` (the pre-promote
/// face) and `WidgetFast` (the current one) were both read out of `units/`, and the fragment's tree
/// then asserted the id of a face its source file no longer declares (`E0080`).
/// 新鲜性规则同样要够到 units 目录：一个文件名**不是**它所声明的那份源码**当前**指纹的 unit，描述的是没人
/// 在编译的那个版本，载荷不得带上它。没有这一条，只追加的目录就会让旧 unit 代替"文件已经改了"的那个路径作答
/// ——实测：一次 promote 重写了某个面文件，`Widget`（promote 之前那张面）与 `WidgetFast`（当前那张）双双被从
/// `units/` 读出，碎片的树于是断言了一张它源文件里已不存在的面的身份（`E0080`）。
#[test]
fn a_stale_unit_does_not_answer_for_a_path_whose_file_changed() {
    let root = temporary_directory("stale-unit");
    let units = root.join("units");
    let src = root.join("src");
    std::fs::create_dir_all(&units).expect("fixture units directory");
    std::fs::create_dir_all(&src).expect("fixture source directory");
    let relative = "panel/object/frame/object/widget/widget.rs";
    let source = src.join(relative);
    std::fs::create_dir_all(source.parent().expect("a parent")).expect("fixture source tree");
    std::fs::write(&source, "// the current file declares WidgetFast\n").expect("fixture source");
    let current = crate::build_method::identity_cache::source_unit_fingerprint(&source);
    // The unit the current file deserves.
    std::fs::write(
        units.join(format!("{current}.tsv")),
        format!("# schema\t6\npath\t{relative}\nnode\tcurrent\tkind\tWidgetFast\n"),
    )
    .expect("the current unit");
    // And the one left behind by the version this file used to be.
    std::fs::write(
        units.join("00000000000000000000000000000000.tsv"),
        format!("# schema\t6\npath\t{relative}\nnode\tstale\tkind\tWidget\n"),
    )
    .expect("the stale unit");
    let layout = layout_of(&root, &src);
    let written = write_faces_manifest(&layout, &units, &root).expect("the payload is written");
    assert_eq!(
        written, 1,
        "only the unit that matches the current file answers"
    );
    let text = std::fs::read_to_string(root.join("faces_manifest.tsv")).expect("the payload");
    assert!(
        text.contains("WidgetFast") && !text.contains("Widget\t"),
        "the payload lists the face the source file declares, not the one whose unit outlived it: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
