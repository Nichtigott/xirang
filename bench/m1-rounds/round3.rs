use control::geometry::{width_with, Ceiling, Floor};

#[test]
fn a_ceiling_is_applied_after_the_floor() {
    assert_eq!(width_with(9, Floor::AtLeast(2)), 9);
    assert_eq!(width_with(9, Ceiling::AtMost(6)), 6, "the ceiling caps what got through");
    assert_eq!(width_with(9, Floor::AtLeast(2).and_then(Ceiling::AtMost(4))), 4);
    assert_eq!(width_with(1, Floor::AtLeast(3).and_then(Ceiling::AtMost(8))), 3);
}
