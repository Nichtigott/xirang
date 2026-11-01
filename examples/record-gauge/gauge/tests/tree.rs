//! 构建期从记录文件读进来的面，运行期必须真的出现在树里。
//! The face the build read out of the record file must actually appear in the runtime tree.

#[test]
fn the_record_file_face_is_in_the_runtime_tree() {
    assert_eq!(
        gauge::outline(),
        vec!["root/Gauge kind=Gauge source=native/gauge.java".to_owned()]
    );
}
