use control::geometry::{width, width_with, Floor};

#[test]
fn the_floor_can_be_configured_per_control() {
    assert_eq!(width_with(1, Floor::AtLeast(4)), 4);
    assert_eq!(width_with(5, Floor::AtLeast(4)), 5);
    assert_eq!(width(3, 4), 4, "the old entry point keeps the old behaviour");
}
