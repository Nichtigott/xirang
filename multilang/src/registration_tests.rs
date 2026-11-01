//! Pins for applying adapter edits: the generic layer, not the adapter, decides whether a plan is safe.
//! 应用适配器改写的钉子：由通用层而不是适配器决定计划是否安全。

use super::*;

/// A non-overlapping edit set is applied to the exact bytes.
/// 不重叠的改写应用到精确字节上。
#[test]
fn edits_apply_to_the_planned_bytes() {
    let source = "abc(x)def";
    let edits = [SourceEdit {
        start: 4,
        end: 5,
        replacement: "y".to_owned(),
    }];
    let applied = apply_source_edits(source, &edits).expect("the edit is in bounds");
    assert_eq!(applied, "abc(y)def");
}

/// An overlapping or reversed edit is refused instead of silently corrupting the source.
/// 重叠或反向的改写被拒绝，而不是静默损坏源码。
#[test]
fn overlapping_or_reversed_edits_are_refused() {
    let source = "abcdef";
    let overlap = [
        SourceEdit {
            start: 1,
            end: 4,
            replacement: "x".to_owned(),
        },
        SourceEdit {
            start: 2,
            end: 5,
            replacement: "y".to_owned(),
        },
    ];
    assert!(apply_source_edits(source, &overlap).is_err());
    let reversed = [SourceEdit {
        start: 4,
        end: 2,
        replacement: "x".to_owned(),
    }];
    assert!(apply_source_edits(source, &reversed).is_err());
}

/// A boundary inside a multi-byte character is refused.
/// 多字节字符内部的边界被拒绝。
#[test]
fn a_split_utf8_boundary_is_refused() {
    let source = "aé";
    let edits = [SourceEdit {
        start: 2,
        end: 3,
        replacement: "x".to_owned(),
    }];
    assert!(apply_source_edits(source, &edits).is_err());
}
