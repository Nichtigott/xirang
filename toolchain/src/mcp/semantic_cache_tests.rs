//! Pins for the semantic-function cache: a record round-trips, and "unparsable" is a record too.
//! 语义函数缓存的钉子：记录可往返，"不可解析"也是一条记录。

use super::{Cached, cached, remember};
use crate::mcp::source_index::Function;

fn source(label: &str) -> String {
    format!(
        "pub fn {label}_{}_{}() {{ crate::run(); }}\n",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

fn function(name: &str) -> Function {
    Function {
        name: name.to_owned(),
        owner: None,
        qualified: name.to_owned(),
        line: 1,
        end_line: 1,
        calls: vec!["crate::run".to_owned()],
        unresolved: vec![],
        caveats: vec!["@decorated".to_owned()],
        semantic: true,
        registration: None,
        binding: None,
    }
}

/// A parsed record round-trips, and an unparsable one is remembered as such.
/// 已解析的记录可往返，不可解析的记录也会被记住。
#[test]
fn records_round_trip_and_remember_refusals() {
    let parsed = source("parsed");
    assert!(cached(&parsed).is_none(), "nothing stored for this content");
    remember(&parsed, Some(&[function("first")]));
    match cached(&parsed) {
        Some(Cached::Parsed(functions)) => {
            assert_eq!(functions.len(), 1);
            assert_eq!(functions[0].name, "first");
            assert_eq!(functions[0].calls, vec!["crate::run".to_owned()]);
            assert_eq!(functions[0].caveats, vec!["@decorated".to_owned()]);
            assert!(functions[0].semantic);
        }
        other => panic!("expected parsed functions, got {}", other.is_some()),
    }
    let unparsable = source("unparsable");
    remember(&unparsable, None);
    assert!(matches!(cached(&unparsable), Some(Cached::Unparsable)));
}
