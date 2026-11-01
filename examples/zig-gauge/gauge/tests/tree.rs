//! 分区之后宿主只留下自己那一半：Shop。Panel 子树带着 Zig 面与它的 Rust 子面走了。
//! After the partition the host keeps its half: Shop. The Panel subtree, with the Zig face and its
//! Rust child, travelled.

#[test]
fn the_host_keeps_the_face_it_did_not_hand_away() {
    assert_eq!(
        gauge::outline(),
        vec!["root/shop kind=Shop source=shop/shop.rs".to_owned()]
    );
}
