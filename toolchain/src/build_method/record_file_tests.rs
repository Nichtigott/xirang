//! Pins for the build-time record file: one decoder, two carriers, and refusals that name the file.
//! 构建期记录文件的钉子：一个解码器、两种载体，以及点名文件的拒绝。

use super::*;
use xirang_multilang::languages::kit::record::{MAGIC, RECORD_BYTES_FILE, VERSION_FILE};

/// One version-3 record, the layout a producer writes into a record file.
/// 一条版本 3 记录，也就是生产者写进记录文件的布局。
fn record(name: &str, source: &str, exports: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES_FILE];
    bytes[..4].copy_from_slice(&MAGIC);
    bytes[4..6].copy_from_slice(&VERSION_FILE.to_le_bytes());
    bytes[8..10].copy_from_slice(&(RECORD_BYTES_FILE as u16).to_le_bytes());
    bytes[28..28 + name.len()].copy_from_slice(name.as_bytes());
    bytes[140..140 + exports.len()].copy_from_slice(exports.as_bytes());
    bytes[1100..1100 + source.len()].copy_from_slice(source.as_bytes());
    bytes
}

fn package(label: &str, manifest: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("xirang-record-file-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("native")).expect("the fixture root");
    std::fs::write(root.join("xirang.toml"), manifest).expect("the manifest");
    root
}

/// A declared record file becomes records whose identities use the carried source.
/// 声明的记录文件变成记录，其身份用记录携带的源码推导。
#[test]
fn a_declared_record_file_is_read_with_its_namespace() {
    let root = package("declared", "[record]\nfile = \"native/gauge.faces\"\n");
    std::fs::write(
        root.join("native/gauge.faces"),
        record("Gauge", "native/gauge/Gauge.java", "gauge"),
    )
    .expect("the record file");
    let records = records(&root, "shop").expect("the records read");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "Gauge");
    assert_eq!(records[0].source, "native/gauge/Gauge.java");
    assert_eq!(
        records[0].id,
        xirang_kernel::identity::NodeId::from_namespaced_path(
            "shop",
            "native/gauge/Gauge.java",
            "Gauge"
        )
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A package with no `[record]` table contributes nothing, and does not fail.
/// 没有 `[record]` 表的包什么也不贡献，也不失败。
#[test]
fn a_package_without_the_table_contributes_no_records() {
    let root = package("absent", "[native]\nlanguage = \"cpp\"\n");
    assert!(
        records(&root, "shop")
            .expect("no table is not a failure")
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declared file that is missing is refused by name, not read as "no faces".
/// 声明了却缺失的文件按名拒绝，而不是读成"没有面"。
#[test]
fn a_declared_but_missing_file_is_refused_by_name() {
    let root = package("missing", "[record]\nfile = \"native/gauge.faces\"\n");
    let why = records(&root, "shop").expect_err("a missing file is not an empty tree");
    assert!(
        why.contains("native/gauge.faces") && why.contains("has not run"),
        "the refusal names the file and the reason: {why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An unknown key is refused, because every field comes from the records themselves.
/// 未知键被拒绝，因为每个字段都来自记录本身。
#[test]
fn an_unknown_key_in_the_table_is_refused() {
    let root = package(
        "unknown-key",
        "[record]\nfile = \"native/gauge.faces\"\nsource = \"java\"\n",
    );
    let why = records(&root, "shop").expect_err("the table describes where, not what");
    assert!(
        why.contains("unknown key `source`") && why.contains("takes `file`, `command`"),
        "the refusal names the key and the shape: {why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A table that names no file is refused rather than silently contributing nothing.
/// 没点名文件的表被拒绝，而不是静默什么都不贡献。
#[test]
fn a_table_without_a_file_is_refused() {
    let root = package("no-file", "[record]\n");
    let why = records(&root, "shop").expect_err("an empty table says nothing");
    assert!(why.contains("names no `file`"), "{why}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The root identity the producer must receive, in the same hex the generated headers spell.
/// 生产者必须收到的根身份，按生成头同样的小写十六进制拼写。
fn root_hex(namespace: &str) -> String {
    let mut text = String::new();
    for byte in xirang_kernel::identity::root_node_id(namespace).into_bytes() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// A declared producer runs before the file is read, with the destination and root identity.
/// 声明的生产者在读取文件之前运行，并拿到目标路径与根身份。
#[test]
fn a_declared_producer_runs_before_the_file_is_read() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        eprintln!("no python3 on PATH; the producer pin was skipped");
        return;
    }
    let root = package(
        "producer",
        "[record]\nfile = \"native/gauge.faces\"\ncommand = [\"python3\", \"make.py\"]\n",
    );
    let hex = root_hex("shop");
    let bytes = record("Gauge", "native/gauge/Gauge.java", "gauge");
    let hex_bytes = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::fs::write(
        root.join("make.py"),
        format!(
            "import binascii, os, sys\n\
             if len(os.environ.get('XIRANG_ROOT_NODE_ID', '')) != 32:\n\
             \x20   sys.exit('root identity was not passed')\n\
             if os.environ['XIRANG_ROOT_NODE_ID'] != '{hex}':\n\
             \x20   sys.exit('root identity is ' + os.environ['XIRANG_ROOT_NODE_ID'])\n\
             if not os.path.isdir(os.environ['XIRANG_PACKAGE_ROOT']):\n\
             \x20   sys.exit('package root was not passed')\n\
             open(os.environ['XIRANG_RECORD_FILE'], 'wb').write(binascii.unhexlify('{hex_bytes}'))\n"
        ),
    )
    .expect("the producer");
    let records = records(&root, "shop").expect("the producer ran and the file reads");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "Gauge");
    let _ = std::fs::remove_dir_all(&root);
}

/// A producer that fails is refused with its status and output, not read as a missing file.
/// 失败的生产者按状态与输出拒绝，而不是被读成缺文件。
#[test]
fn a_failing_producer_is_refused_with_its_output() {
    let root = package(
        "producer-fails",
        "[record]\nfile = \"native/gauge.faces\"\ncommand = [\"sh\", \"fail.sh\"]\n",
    );
    std::fs::write(
        root.join("fail.sh"),
        "#!/bin/sh\necho 'producer exploded' >&2\nexit 3\n",
    )
    .expect("the failing producer");
    let why = records(&root, "shop").expect_err("a failing producer is a build failure");
    assert!(
        why.contains("failed with") && why.contains("producer exploded"),
        "the refusal carries the producer's status and stderr: {why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A producer that exits zero and writes nothing is a refusal, not an empty tree.
/// 退出码 0 但什么都没写的生产者是拒绝，不是一棵空树。
#[test]
fn a_producer_that_writes_nothing_is_refused() {
    let root = package(
        "producer-noop",
        "[record]\nfile = \"native/gauge.faces\"\ncommand = [\"sh\", \"noop.sh\"]\n",
    );
    std::fs::write(root.join("noop.sh"), "#!/bin/sh\nexit 0\n").expect("the no-op producer");
    let why = records(&root, "shop").expect_err("wrote nothing is a producer bug");
    assert!(
        why.contains("wrote nothing") && why.contains("not a tree with no faces"),
        "the refusal separates 'wrote nothing' from 'declares nothing': {why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A command that is not a one-line array of quoted strings is refused by name.
/// 不是一行带引号字符串数组的命令按名拒绝。
#[test]
fn a_command_that_is_not_an_array_is_refused() {
    let root = package(
        "command-shape",
        "[record]\nfile = \"native/gauge.faces\"\ncommand = \"sh build.sh\"\n",
    );
    let why = records(&root, "shop").expect_err("a bare string is not an argv");
    assert!(
        why.contains("[record] command") && why.contains("one line"),
        "the refusal names the shape: {why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A producer without a destination is refused: the build would have nowhere to read.
/// 没有目标的生产者被拒绝：构建将无处读取。
#[test]
fn a_command_without_a_file_is_refused() {
    let root = package(
        "command-no-file",
        "[record]\ncommand = [\"sh\", \"make.sh\"]\n",
    );
    let why = records(&root, "shop").expect_err("a producer needs a destination");
    assert!(why.contains("names no `file`"), "{why}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A `language` without a `constants` path cannot be answered: the build has nowhere to write.
/// 只有 `language` 没有 `constants` 路径无法回答：构建无处可写。
#[test]
fn a_language_without_constants_is_refused() {
    let root = package(
        "language-no-constants",
        "[record]\nfile = \"native/gauge.faces\"\nlanguage = \"go\"\n",
    );
    let why = records(&root, "shop").expect_err("no destination for the constants");
    assert!(
        why.contains("`[record] language` asks for a constants file but no `constants` path"),
        "{why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A `constants` path without a `language` cannot be written: the syntax is unknown.
/// 只有 `constants` 路径没有 `language` 无法写出：语法未知。
#[test]
fn constants_without_a_language_is_refused() {
    let root = package(
        "constants-no-language",
        "[record]\nfile = \"native/gauge.faces\"\nconstants = \"go/src/gauge/faces.go\"\n",
    );
    let why = records(&root, "shop").expect_err("no syntax to write the constants in");
    assert!(
        why.contains("`[record] constants` names a file but no `language`"),
        "{why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A paired `language` + `constants` is read as one declaration (no producer run here).
/// 成对的 `language` + `constants` 读成一条声明（这里不运行生产者）。
#[test]
fn a_language_and_constants_are_read_together() {
    let root = package(
        "language-constants",
        "[record]\nfile = \"native/gauge.faces\"\nlanguage = \"go\"\nconstants = \"go/src/gauge/faces.go\"\n",
    );
    let declared = declaration(&root)
        .expect("the table parses")
        .expect("a table");
    assert_eq!(declared.language.as_deref(), Some("go"));
    assert_eq!(declared.constants.as_deref(), Some("go/src/gauge/faces.go"));
    let _ = std::fs::remove_dir_all(&root);
}
