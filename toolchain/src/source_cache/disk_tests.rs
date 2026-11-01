//! Pins for the disk-backed syntax facts: a record round-trips, and a miss stays a miss.
//! 磁盘语法事实的钉子：记录可往返，未命中仍是未命中。

use super::{fingerprint, load, store};

/// A record survives the round trip, calls included.
/// 记录经往返后不变，调用表也一样。
#[test]
fn a_record_round_trips() {
    let source = &format!(
        "pub fn caller_{}_{}() {{ crate::run(); }}\n",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    assert!(load(source).is_none(), "nothing is stored yet");
    let unit = crate::source_cache::facts::facts(source);
    store(source, &unit);
    let read_back = load(source).expect("the record is there");
    assert_eq!(read_back.functions, unit.functions);
    assert_eq!(read_back.calls, unit.calls);
}

/// The key follows the content, and two lengths of the same prefix are different keys.
/// 键跟着内容走，同前缀不同长度也是不同的键。
#[test]
fn the_key_follows_content() {
    let first = fingerprint("pub fn a() {}\n");
    let same = fingerprint("pub fn a() {}\n");
    let other = fingerprint("pub fn b() {}\n");
    let longer = fingerprint("pub fn a() {}\n// tail\n");
    assert_eq!(first, same);
    assert_ne!(first, other);
    assert_ne!(first, longer);
}
