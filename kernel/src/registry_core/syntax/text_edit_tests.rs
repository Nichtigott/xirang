//! Pins for the plan vocabulary and the checked applier.
//! 计划词汇与带校验应用函数的钉子。

use super::{TextEdit, apply_text_edits, apply_text_edits_checked, check_text_edits};

/// A plan whose ranges do not fit its own text is refused whole, never half applied,
/// and the refusal is reportable.
/// 与自身文本不匹配的计划被整组拒绝、绝不改一半，而且这次拒绝可以被报出。
#[test]
fn a_malformed_plan_is_refused_whole() {
    let source = "αβγ";
    let inside_a_character = vec![TextEdit {
        start: 1,
        end: 3,
        replacement: "x".to_owned(),
    }];
    assert!(check_text_edits(source, &inside_a_character).is_err());
    assert_eq!(apply_text_edits(source, &inside_a_character), source);
    assert!(apply_text_edits_checked(source, &inside_a_character).is_err());

    let past_the_end = vec![TextEdit {
        start: 0,
        end: 99,
        replacement: "x".to_owned(),
    }];
    assert!(check_text_edits(source, &past_the_end).is_err());
    assert_eq!(apply_text_edits(source, &past_the_end), source);

    let inverted = vec![TextEdit {
        start: 4,
        end: 2,
        replacement: "x".to_owned(),
    }];
    assert!(check_text_edits(source, &inverted).is_err());

    let overlapping = vec![
        TextEdit {
            start: 0,
            end: 3,
            replacement: "x".to_owned(),
        },
        TextEdit {
            start: 2,
            end: 6,
            replacement: "y".to_owned(),
        },
    ];
    assert!(check_text_edits(source, &overlapping).is_err());
    assert_eq!(apply_text_edits(source, &overlapping), source);

    let disjoint = vec![
        TextEdit {
            start: 0,
            end: 2,
            replacement: "A".to_owned(),
        },
        TextEdit {
            start: 2,
            end: 4,
            replacement: "B".to_owned(),
        },
    ];
    assert_eq!(check_text_edits(source, &disjoint), Ok(()));
    assert_eq!(apply_text_edits(source, &disjoint), "ABγ");
}

/// Adjacent edits on a multi-byte string land where they were planned: the accepted
/// half of the boundary above.
/// 多字节字符串上相邻的替换落在规划的位置：上文那条边界的通过一半。
#[test]
fn multi_byte_edits_land_on_their_boundaries() {
    let source = "中::文";
    let edits = vec![
        TextEdit {
            start: 0,
            end: 3,
            replacement: "甲".to_owned(),
        },
        TextEdit {
            start: 5,
            end: 8,
            replacement: "乙".to_owned(),
        },
    ];
    assert_eq!(apply_text_edits(source, &edits), "甲::乙");
}
