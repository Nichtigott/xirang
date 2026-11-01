//! Tests for the `xirang.faces` reader.
//! `xirang.faces` 读取器的测试。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::*;

/// The parent identity every fixture hangs from.
/// 每个夹具都挂在它下面的父身份。
const PARENT: [u8; 16] = [
    0xa0, 0x21, 0x84, 0x88, 0x08, 0x04, 0x27, 0x04, 0xe4, 0xc7, 0x46, 0xba, 0xd8, 0x73, 0xe2, 0x26,
];

fn scratch(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xirang-external-face-{}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

fn compiler() -> Option<String> {
    for tool in ["g++", "clang++"] {
        if Command::new(tool)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Some(tool.to_owned());
        }
    }
    None
}

fn compile(directory: &Path, stem: &str, source: &str) -> Option<Output> {
    let tool = compiler()?;
    let file = directory.join(format!("{stem}.cpp"));
    std::fs::write(&file, source).expect("the fixture source");
    let include = crate::sdk::cpp_include_dir();
    Some(
        Command::new(tool)
            .arg("-std=c++20")
            .arg(format!("-I{}", include.display()))
            .arg("-c")
            .arg(&file)
            .arg("-o")
            .arg(directory.join(format!("{stem}.o")))
            .output()
            .expect("the compiler runs"),
    )
}

/// One record built by hand, for the refusals a compiler would not produce.
/// 手写的一条记录，用于编译器不会产生的那些拒绝。
fn record(flags: u16, name: &str, exports: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES_ONE];
    bytes[..4].copy_from_slice(&MAGIC);
    bytes[4..6].copy_from_slice(&VERSION_ONE.to_le_bytes());
    bytes[6..8].copy_from_slice(&flags.to_le_bytes());
    bytes[8..10].copy_from_slice(&(RECORD_BYTES_ONE as u16).to_le_bytes());
    bytes[12..28].copy_from_slice(&PARENT);
    bytes[28..28 + name.len()].copy_from_slice(name.as_bytes());
    bytes[44..44 + exports.len()].copy_from_slice(exports.as_bytes());
    bytes
}

const DECLARATION: &str = r#"
#include "xirang/soil.hpp"
struct Area {};
constexpr ::xirang::Id PARENT{{0xa0,0x21,0x84,0x88,0x08,0x04,0x27,0x04,0xe4,0xc7,0x46,0xba,0xd8,0x73,0xe2,0x26}};
XIRANG_FACE(record_for_area,
            (::xirang::soil<Area>().under(PARENT).named("Area").exports("area,volume")))
"#;

#[test]
fn a_record_a_compiler_wrote_is_read_back() {
    let directory = scratch("round-trip");
    let Some(output) = compile(&directory, "area", DECLARATION) else {
        eprintln!("no C++ compiler on PATH; the round trip was not exercised");
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let faces = read_faces(&directory.join("area.o")).expect("the object is readable");
    assert_eq!(faces.len(), 1, "one declaration is one face");
    assert_eq!(faces[0].name, "Area");
    assert_eq!(faces[0].parent, NodeId::from_raw(PARENT));
    assert_eq!(
        faces[0].exports,
        vec!["area".to_owned(), "volume".to_owned()]
    );
    assert!(
        !faces[0].owns_registry,
        "the declaration did not ask to own a registry"
    );
}

#[test]
fn a_face_that_owns_a_registry_says_so() {
    let directory = scratch("owns");
    let source = DECLARATION.replace(".exports(\"area,volume\")", ".owns_registry()");
    let Some(output) = compile(&directory, "panel", &source) else {
        eprintln!("no C++ compiler on PATH; the flag was not exercised");
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let faces = read_faces(&directory.join("panel.o")).expect("the object is readable");
    assert_eq!(faces.len(), 1);
    assert!(
        faces[0].owns_registry,
        "owns_registry() must reach the reader"
    );
}

#[test]
fn two_declarations_in_one_object_are_two_faces() {
    let directory = scratch("two");
    let source = format!(
        "{DECLARATION}\nXIRANG_FACE(record_for_second, \
         (::xirang::soil<Area>().under(PARENT).named(\"Second\").exports(\"second\")))\n"
    );
    let Some(output) = compile(&directory, "pair", &source) else {
        eprintln!("no C++ compiler on PATH; the pair was not exercised");
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut faces: Vec<String> = read_faces(&directory.join("pair.o"))
        .expect("the object is readable")
        .into_iter()
        .map(|face| face.name)
        .collect();
    faces.sort();
    assert_eq!(faces, vec!["Area".to_owned(), "Second".to_owned()]);
}

#[test]
fn an_object_without_the_section_declares_no_faces() {
    let directory = scratch("plain");
    let Some(output) = compile(&directory, "plain", "int answer() { return 42; }\n") else {
        eprintln!("no C++ compiler on PATH; the empty case was not exercised");
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let faces = read_faces(&directory.join("plain.o"))
        .expect("an object without the section is not a failure");
    assert!(faces.is_empty(), "a plain object declares no face");
}

#[test]
fn a_name_longer_than_the_record_allows_is_a_compile_error() {
    let directory = scratch("overlong");
    let source = DECLARATION.replace(
        ".named(\"Area\")",
        ".named(\"ThisNameIsFarTooLongForAThirtyTwoByteField\")",
    );
    let Some(output) = compile(&directory, "too-long", &source) else {
        eprintln!("no C++ compiler on PATH; the refusal was not exercised");
        return;
    };
    assert!(
        !output.status.success(),
        "a name that does not fit must not compile"
    );
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("longer than its record field allows"),
        "the compiler should repeat the refusal this header writes, got: {said}"
    );
}

#[test]
fn a_face_declared_without_a_name_is_refused_when_read() {
    let directory = scratch("anonymous");
    let source = DECLARATION.replace(".named(\"Area\")", "");
    let Some(output) = compile(&directory, "nameless", &source) else {
        eprintln!("no C++ compiler on PATH; the nameless face was not exercised");
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let why =
        read_faces(&directory.join("nameless.o")).expect_err("a nameless face must be refused");
    assert!(
        why.contains("names no face"),
        "the refusal should say what is wrong: {why}"
    );
}

#[test]
fn a_truncated_record_is_refused() {
    let bytes = MAGIC.to_vec();
    let why = decode(&bytes).expect_err("a magic word without a body is not a record");
    assert!(
        why.contains("truncated"),
        "the refusal should say the record is cut short: {why}"
    );
}

#[test]
fn a_layout_version_this_reader_does_not_know_is_refused() {
    let mut bytes = record(0, "Area", "area");
    bytes[4..6].copy_from_slice(&3u16.to_le_bytes());
    let why = decode(&bytes).expect_err("a future layout is not readable");
    assert!(
        why.contains("layout version 3"),
        "the refusal should name the version: {why}"
    );
}

#[test]
fn a_record_whose_own_length_disagrees_is_refused() {
    let mut bytes = record(0, "Area", "area");
    bytes[8..10].copy_from_slice(&64u16.to_le_bytes());
    let why = decode(&bytes).expect_err("a record that misstates its length is not readable");
    assert!(
        why.contains("says it is 64 bytes"),
        "the refusal should quote the claim: {why}"
    );
}

#[test]
fn an_unknown_flag_bit_is_refused() {
    let why = decode(&record(2, "Area", "area")).expect_err("an unknown flag is not ignorable");
    assert!(
        why.contains("unknown flag bits"),
        "the refusal should name the bits: {why}"
    );
}

#[test]
fn a_non_zero_reserved_byte_is_refused() {
    let mut bytes = record(0, "Area", "area");
    bytes[10] = 1;
    let why = decode(&bytes).expect_err("a writer this reader does not know must not pass");
    assert!(
        why.contains("reserved bytes"),
        "the refusal should name the reminder: {why}"
    );
}

#[test]
fn bytes_after_the_terminator_are_refused() {
    let mut bytes = record(0, "Area", "area");
    bytes[28 + "Area".len() + 1] = b'x';
    let why = decode(&bytes).expect_err("a stale byte after the terminator must be refused");
    assert!(
        why.contains("after its terminator"),
        "the refusal should name the tail: {why}"
    );
}

#[test]
fn a_repeated_face_in_one_object_is_refused() {
    let mut bytes = record(0, "Area", "area");
    bytes.extend_from_slice(&record(0, "Area", "area"));
    let why = decode(&bytes).expect_err("the same face twice is a mistake, not two faces");
    assert!(
        why.contains("twice"),
        "the refusal should say the face repeats: {why}"
    );
}

#[test]
fn padding_between_records_is_walked_past() {
    let mut bytes = record(0, "Area", "area");
    bytes.extend_from_slice(&[0u8; 24]);
    bytes.extend_from_slice(&record(0, "Volume", "volume"));
    let faces = decode(&bytes).expect("padding between records is not part of any record");
    let names: Vec<&str> = faces.iter().map(|face| face.name.as_str()).collect();
    assert_eq!(names, vec!["Area", "Volume"]);
}

/// One version-3 record, the layout a build-time record file writes.
/// 一条版本 3 记录，也就是构建期记录文件写下的布局。
fn file_record(name: &str, source: &str, exports: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; RECORD_BYTES_FILE];
    bytes[..4].copy_from_slice(&MAGIC);
    bytes[4..6].copy_from_slice(&VERSION_FILE.to_le_bytes());
    bytes[8..10].copy_from_slice(&(RECORD_BYTES_FILE as u16).to_le_bytes());
    bytes[12..28].copy_from_slice(&PARENT);
    bytes[28..28 + name.len()].copy_from_slice(name.as_bytes());
    bytes[140..140 + exports.len()].copy_from_slice(exports.as_bytes());
    bytes[1100..1100 + source.len()].copy_from_slice(source.as_bytes());
    bytes
}

fn record_file_path(label: &str) -> PathBuf {
    let directory = scratch(label);
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory.join(SECTION)
}

/// A version-3 record is decoded by the same reader, and its source comes back.
/// 版本 3 记录由同一个读取器解码，它的源码也读得回来。
#[test]
fn a_file_record_carries_its_source() {
    let bytes = file_record("Gauge", "native/gauge/Gauge.java", "gauge,reading");
    let faces = decode(&bytes).expect("the shared reader walks the appended layout");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].name, "Gauge");
    assert_eq!(faces[0].source, "native/gauge/Gauge.java");
    assert_eq!(faces[0].exports, vec!["gauge", "reading"]);
}

/// A build-time record file becomes records whose identities use the carried source.
/// 构建期记录文件变成记录，其身份用记录携带的源码推导。
#[test]
fn a_record_file_becomes_records_with_the_carried_source() {
    let path = record_file_path("file-records");
    std::fs::write(
        &path,
        file_record("Gauge", "native/gauge/Gauge.java", "gauge"),
    )
    .expect("the record file");
    let records = records_from_file(&path, "shop").expect("the file reads");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source, "native/gauge/Gauge.java");
    assert_eq!(
        records[0].id,
        NodeId::from_namespaced_path("shop", "native/gauge/Gauge.java", "Gauge"),
        "identity is the same rule the section carrier uses"
    );
    assert_eq!(records[0].parent, NodeId::from_raw(PARENT));
    let _ = std::fs::remove_dir_all(path.parent().expect("a parent"));
}

/// A version-2 record has no source field, so a file carrying one is refused by name.
/// 版本 2 记录没有源码字段，因此携带它的文件按名拒绝。
#[test]
fn a_file_record_without_a_source_is_refused_by_name() {
    let path = record_file_path("file-v2");
    std::fs::write(&path, record(0, "Gauge", "gauge")).expect("the record file");
    let why = read_file_faces(&path).expect_err("a file record must name its source");
    assert!(
        why.contains("version 3") && why.contains(SECTION),
        "the refusal names the layout the file needs: {why}"
    );
    let _ = std::fs::remove_dir_all(path.parent().expect("a parent"));
}

/// An empty record file is a producer bug, not a tree with no faces.
/// 空记录文件是生产者的错误，而不是一棵没有面的树。
#[test]
fn an_empty_record_file_is_refused() {
    let path = record_file_path("file-empty");
    std::fs::write(&path, []).expect("the empty file");
    let why = read_file_faces(&path).expect_err("an empty file says nothing");
    assert!(
        why.contains("empty") && why.contains("producer bug"),
        "the refusal separates 'nothing was written' from 'no faces': {why}"
    );
    let _ = std::fs::remove_dir_all(path.parent().expect("a parent"));
}

/// A truncated version-3 record is refused where it ends, not read past.
/// 截断的版本 3 记录在它结束的地方被拒绝，而不是被读过界。
#[test]
fn a_truncated_file_record_is_refused() {
    let mut bytes = file_record("Gauge", "native/gauge/Gauge.java", "gauge");
    bytes.truncate(RECORD_BYTES_FILE - 3);
    let why = decode(&bytes).expect_err("a short record is not a face");
    assert!(why.contains("truncated"), "{why}");
}
