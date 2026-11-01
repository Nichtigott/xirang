//! The `xirang.faces` decoder is language-neutral: a build with no language must still read it.
//! `xirang.faces` 解码器是语言中立的：不带任何语言的构建也必须能读它。
//!
//! This target runs in the `native-record` face, which enables the decoder **without** `lang-cpp`.
//! If the decoder moves back under a language feature, the target stops compiling — that is the
//! pin. The bytes are built by hand so the test needs no compiler.
//! 本 target 在 `native-record` 面里跑，该面打开解码器而**不**打开 `lang-cpp`。若解码器被挪回某个
//! 语言特性之下，这个 target 就编译不过——那就是钉子。字节是手写的，因此测试不需要编译器。

use xirang_multilang::languages::kit::record::{MAGIC, RECORD_BYTES_ONE, VERSION_ONE, decode};

/// The parent identity the hand-built record names.
/// 手写记录点名的父身份。
const PARENT: [u8; 16] = [
    0xa0, 0x21, 0x84, 0x88, 0x08, 0x04, 0x27, 0x04, 0xe4, 0xc7, 0x46, 0xba, 0xd8, 0x73, 0xe2, 0x26,
];

/// One version-1 record, the layout the SDK header writes.
/// 一条版本 1 记录，即 SDK 头文件写出的布局。
fn record(name: &str, exports: &str, flags: u16) -> Vec<u8> {
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

#[test]
fn a_record_decodes_without_any_language_feature() {
    let faces = decode(&record("Area", "area,volume", 0)).expect("the record decodes");
    assert_eq!(faces.len(), 1, "one record is one face");
    assert_eq!(faces[0].name, "Area");
    assert_eq!(
        faces[0].exports,
        vec!["area".to_owned(), "volume".to_owned()]
    );
    assert!(
        !faces[0].owns_registry,
        "the flags word did not ask to own a registry"
    );
}

/// A section with no record magic carries no face; it is not an error, and it is not a face.
/// 没有记录魔数的段不携带面；它不是错误，也不是一个面。
#[test]
fn a_stream_without_the_magic_yields_no_face() {
    let mut bytes = record("Area", "", 0);
    bytes[..4].copy_from_slice(b"NOPE");
    let faces = decode(&bytes).expect("a stream with no record magic decodes to no faces");
    assert!(faces.is_empty(), "a foreign byte stream is not a face");
}
