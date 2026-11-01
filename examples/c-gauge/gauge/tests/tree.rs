//! 构建期折进来的 C 面，运行期必须真的出现在树里。
//! The C face the build folded in must actually appear in the runtime tree.

#[test]
fn the_c_face_is_in_the_runtime_tree() {
    assert_eq!(
        gauge::outline(),
        vec![
            "root/panel kind=Panel source=panel/panel.rs".to_owned(),
            "root/panel/Gauge kind=Gauge source=native/gauge/gauge.c".to_owned(),
        ]
    );
}
