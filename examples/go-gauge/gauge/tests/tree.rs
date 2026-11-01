//! 构建期从 Go 声明产出的面，运行期必须真的出现在树里。
//! The face the build produced from the Go declaration must actually appear in the runtime tree.

#[test]
fn the_go_declared_face_is_in_the_runtime_tree() {
    assert_eq!(
        gauge::outline(),
        vec![
            "root/panel kind=Panel source=panel/panel.rs".to_owned(),
            "root/panel/Gauge kind=Gauge source=go/src/gauge/gauge.go".to_owned(),
        ]
    );
}
