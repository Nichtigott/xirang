//! One derived `handle`, two fields: a face keeps its function name either way.
//! 派生出的 `handle` 只有一份，供两个字段：两种输入下注册面都保住它的函数名。

use super::FaceManifest;
use crate::run_method::authoring::context::AuthoringContext;
use crate::run_method::registry_core::identity::NodeId;

/// Both spellings of the input answer `source.function` with the same value the
/// `handle` field carries: no `handle` key (what a file-authored face has) and an
/// explicit `handle`.
/// 两种输入拼法都让 `source.function` 与 `handle` 字段同值：没有 `handle` 键（文件创作的注册面
/// 就是这种）与显式写了 `handle`。
///
/// `source.function` is what `registered_functions()` (Studio) and the MCP symbol
/// lists read. The kernel derives `handle` from `kind` and now uses **that value**
/// for both fields; before `N-4` it filled `source.function` from the *raw* key
/// (`core/.../authoring/snapshot.rs:56` vs `:134`), so a face with no
/// `handle` came back with `function=""` and Studio's call-tree tests went from
/// 12 passed to 10 passed / 2 failed. A patch on this side that injected the key
/// hid the drift; one derivation in the kernel is the fix.
/// `source.function` 是 `registered_functions()`（Studio）与 MCP 符号清单读取的字段。内核按
/// `kind` 派生 `handle`，现在把**那个值**同时用于两个字段；`N-4` 之前它用**原始**键填
/// `source.function`（`core/.../authoring/snapshot.rs:56` 对 `:134`），因此不存
/// `handle` 的面回来时 `function=""`，Studio 的调用树测试从 12 通过变成 10 通过 / 2 失败。
/// 在本地这一侧注入该键的补丁把漂移藏了起来；修法是内核里只有一份推导。
#[test]
fn both_spellings_of_the_input_keep_the_function_name() {
    let cases: [(&str, Option<&str>, &str); 2] = [
        ("a file-authored face", None, "Widget"),
        (
            "an explicitly declared handle",
            Some("CustomHandle"),
            "CustomHandle",
        ),
    ];
    for (label, declared, expected) in cases {
        let mut face = FaceManifest::new(
            "widget",
            "Widget",
            NodeId::from_namespaced_path("probe", "root.rs", "root"),
            "<root>",
            "root",
            "widget/widget.rs",
        );
        if let Some(declared) = declared {
            face.values.insert("handle".to_owned(), declared.to_owned());
        }
        let snapshot = AuthoringContext::new("/tmp/xirang-snapshot-probe", "probe")
            .scope(|| face.to_snapshot())
            .expect("a face with every required key converts");
        assert_eq!(snapshot.handle, expected, "{label}: the handle field");
        assert_eq!(
            snapshot.source.function, snapshot.handle,
            "{label}: `source.function` and `handle` are one derivation, not two"
        );
        assert_eq!(
            snapshot.source.function, expected,
            "{label}: the function name a compiled face would carry"
        );
    }
}
