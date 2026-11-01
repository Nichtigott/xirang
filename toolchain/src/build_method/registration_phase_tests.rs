//! Tests for reading a cached unit through the shared seam.
//! 通过共用那道缝读取缓存单元的测试。

use std::collections::BTreeMap;

use super::super::discovery_cache::{escape_field, read_unit, unescape_field};

/// The seam the census's consumers will read on a record hit: every declaration field the unit carried,
/// verbatim, with the line it was written on.
/// 普查里那些消费者在记录命中时要读的那道缝：单元携带的每个声明字段、逐字，以及它写在哪一行。
///
/// The values are the interesting half. A declaration value is arbitrary source text — a `summary` may
/// well contain a tab or a newline inside its string literal — and a row that corrupted it would hand a
/// consumer a field that no longer matches the source. The pin therefore puts both in and reads them
/// back, and it pins the two escapes in both directions.
/// 取值是有意思的那一半。声明取值是任意源码文本——`summary` 的字符串字面量里完全可能带制表符或换行——而一行把它
/// 弄坏的记录会把一个与源码不再相符的字段交给消费者。因此钉子把两者都放进去再读回来，并在两个方向上钉住那两个转义。
#[test]
fn a_unit_round_trips_every_declaration_field_it_carried() {
    let summary = "plain text\twith a tab\nand a newline \\\\ and a backslash";
    let unit = format!(
        "# schema\t6\n# fingerprint\tabc\npath\tcontrol/control.rs\nnode\tdeadbeef\nkind\tControl\n\
         macro\troot_object\ncfg\tfeature = \"studio\"\nline\t7\n\
         handle\tControlHandle\n\
         field\tkind\t5\tControl\n\
         field\tneeds_registry\t6\ttrue\n\
         field\tsummary\t9\t{}\n\
         value\tkind\tpath\tControl\n\
         value\tneeds_registry\tbool\ttrue\n\
         value\tstable_name\tstring\tdup\n\
         value\thandle_traits\tlist\t0\tControlHandle\n\
         value\thandle_traits\tlist\t1\tFocusable\n\
         requires\trender\tProvider\n\
         provides\tcontrol.render\n",
        escape_field(summary)
    );
    let facts = read_unit(&unit).expect("the unit parses");
    assert_eq!(facts.relative, "control/control.rs");
    assert_eq!(facts.kind, "Control");
    assert_eq!(facts.handle.as_deref(), Some("ControlHandle"));
    assert_eq!(facts.line, Some(7));
    assert_eq!(
        facts.fields.get("needs_registry"),
        Some(&(6, "true".to_owned())),
        "a field the declaration wrote is carried with its own line"
    );
    assert_eq!(
        facts.fields.get("summary").map(|(_, value)| value.as_str()),
        Some(summary),
        "a value with a tab, a newline and a backslash survives the round trip"
    );
    assert_eq!(
        facts.requires,
        vec![("render".to_owned(), "Provider".to_owned())]
    );
    assert_eq!(facts.provides, vec!["control.render".to_owned()]);

    // The **decoded** forms are handed out under the reader's name for them, and a list keeps its order.
    // **解码后**的形态按读取器的名字交出，而列表保持顺序。
    assert_eq!(
        facts
            .values
            .get(&("needs_registry".to_owned(), "bool".to_owned())),
        Some(&"true".to_owned())
    );
    assert_eq!(
        facts
            .values
            .get(&("stable_name".to_owned(), "string".to_owned())),
        Some(&"dup".to_owned()),
        "the quotes belong to the source text, not to the value a consumer reads"
    );
    assert_eq!(
        facts.values.get(&("kind".to_owned(), "path".to_owned())),
        Some(&"Control".to_owned())
    );
    assert_eq!(
        facts
            .lists
            .get(&("handle_traits".to_owned(), "list".to_owned())),
        Some(&vec!["ControlHandle".to_owned(), "Focusable".to_owned()]),
        "a derived label list is carried in the order the reader produced it"
    );
    assert!(
        facts.values.keys().all(|(name, _)| name != "summary"),
        "an absent decoded form is the reader's own `None`, and it is a missing row rather than a guess"
    );

    // Text that is not a unit has no `path` row, so the answer is `None` and every caller falls back to
    // parsing the source — which is the only safe direction for this seam to fail in.
    // 不是单元的文本没有 `path` 行，因此答案是 `None`，而每个调用方都退回解析源码——这是这道缝唯一安全的失败方向。
    assert!(read_unit("# schema\t6\n").is_none());
    assert!(
        read_unit("path\tcontrol/control.rs\nfield\tbroken\tnot-a-number\tvalue\n")
            .expect("a unit with one broken row")
            .fields
            .is_empty(),
        "a row that does not parse is dropped, not guessed at"
    );

    // The escapes are each other's inverse, including the cases that make the pair worth having.
    // 两个转义互为逆运算，包括让这一对值得存在的那些情形。
    for value in [summary, "\\", "a\tb", "a\nb", "\\t is not a tab", ""] {
        assert_eq!(
            unescape_field(&escape_field(value)),
            value,
            "escaping {value:?} is invertible"
        );
        assert!(
            !escape_field(value).contains('\n'),
            "and an escaped value never carries a newline: {value:?}"
        );
    }
    let _: BTreeMap<String, (u64, String)> = read_unit(&unit).expect("the unit parses").fields;
}
