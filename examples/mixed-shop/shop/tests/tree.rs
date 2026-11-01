//! 拆出去之后，宿主留下自己那一半：Shop。C++ 声明的 Area 挂在 Panel 下，因此跟着 Panel 走了。
//! After the split the host keeps its half: Shop. The Area the C++ file declares hangs under Panel,
//! so it travelled with Panel.

#[test]
fn the_host_keeps_the_faces_it_did_not_hand_away() {
    let rows = shop::outline();
    assert_eq!(
        rows,
        vec!["root/shop kind=Shop source=shop/shop.rs".to_owned()]
    );
}
