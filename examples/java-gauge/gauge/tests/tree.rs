//! 构建期从 Java 注解产出的面，运行期必须真的出现在树里。
//! The face the build produced from the Java annotation must actually appear in the runtime tree.

#[test]
fn the_java_declared_face_is_in_the_runtime_tree() {
    assert_eq!(
        gauge::outline(),
        vec![
            "root/panel kind=Panel source=panel/panel.rs".to_owned(),
            "root/panel/Gauge kind=Gauge source=java/src/gauge/Gauge.java".to_owned(),
        ]
    );
}
