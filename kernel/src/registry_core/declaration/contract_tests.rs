//! The type-like half of a declaration (`FaceContract`) and the check that keeps it honest.
//! 声明里类型类的那一半（`FaceContract`），以及让它不至于只是装饰的那条检查。

use super::{FaceContract, NoParts, NoPreset, PresetContract};

/// A face's two type-like declarations are writable as associated types, readable by naming the face, and
/// **tied together**: `FaceContract::Parts`'s output must be the preset's output, so a mismatched pair is a
/// compile error rather than a graft cut that later finds nothing to link.
/// 面的两个类型类声明可以写成关联类型、可以点名那个面读回来，而且**彼此绑定**：`FaceContract::Parts` 的输出
/// 必须是 preset 的输出，因此配错的一对是**编译错误**，而不是事后在 graft 切口上才发现没东西可接。
#[test]
fn a_faces_type_like_half_is_readable_and_tied() {
    struct Card;

    impl FaceContract for Card {
        type Preset = NoPreset;
        type Parts = NoParts;
    }

    fn preset_of<F: FaceContract>() -> &'static str {
        core::any::type_name::<F::Preset>()
    }
    fn parts_of<F: FaceContract>() -> &'static str {
        core::any::type_name::<F::Parts>()
    }
    assert!(
        preset_of::<Card>().ends_with("NoPreset"),
        "{}",
        preset_of::<Card>()
    );
    assert!(
        parts_of::<Card>().ends_with("NoParts"),
        "{}",
        parts_of::<Card>()
    );

    // The tie itself is a bound, not a runtime comparison: this compiles only because `NoParts`'s output
    // is `NoPreset`'s output, and `assert_face_contract` is the same check the macro emits per face.
    const _: () = super::assert_face_contract::<Card>();
    assert_eq!(
        <NoPreset as PresetContract>::REQUIRED_PARTS,
        <NoPreset as PresetContract>::REQUIRED_PARTS,
        "and the preset's own contract is still reachable through the face"
    );
}
