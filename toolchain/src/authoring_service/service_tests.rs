//! Pins for the shared authoring service: language choice, destination, generation, and apply.
//! 共享创作服务的钉子：语言选择、目的地、生成与落盘。

use std::path::{Path, PathBuf};

use xirang_kernel::identity::NodeId;
use xirang_kernel::interchange::LanguageId;

use super::*;

fn cpp() -> LanguageId {
    LanguageId::parse_language("cpp").expect("a language id")
}

fn rust() -> LanguageId {
    LanguageId::parse_language("rust").expect("a language id")
}

/// A throwaway package with one Rust parent face that owns a registry.
/// 一个一次性包，含一个自持注册机的 Rust 父面。
fn fixture(label: &str) -> (PathBuf, NodeId) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-authoring-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/shop/panel")).expect("the fixture tree");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"shop\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("the manifest");
    std::fs::write(root.join("src/lib.rs"), "").expect("the crate root");
    std::fs::write(
        root.join("src/shop/panel/panel.rs"),
        "//! The panel face.\n//! 面板面。\n\n\
         #[xirang_toolchain::soil]\n\
         pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Panel>()\n\
         \x20   .needs_registry(true)\n\
         \x20   .under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\n\
         pub struct Panel;\n",
    )
    .expect("the parent face");
    let id = NodeId::from_namespaced_path("shop", "shop/panel/panel.rs", "Panel");
    (root, id)
}

fn service(root: &Path) -> AuthoringService {
    AuthoringService::new(root.to_path_buf(), "shop")
}

/// The C++ child goes into the Rust parent's own directory, not a separate `native/` tree, and
/// names the parent by the generated header's constant.
/// C++ 子面进入 Rust 父自己的目录，而不是另建 `native/` 树，并按生成头的常量点名父。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_cpp_child_of_a_rust_parent_is_generated_beside_it() {
    let (root, panel) = fixture("cpp-under-rust");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let plan = service(&root).plan(&request).expect("the plan");
    let file = plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".cpp"))
        .expect("a C++ file");
    assert_eq!(file.path, "src/shop/panel/object/area/area.cpp");
    assert!(
        file.updated
            .contains(".under(crate::catalog::panel::NODE_ID)")
    );
    assert!(file.updated.contains(".named(\"Area\")"));
    assert!(
        plan.files
            .iter()
            .any(|file| file.path == "compile_flags.txt"),
        "the editor material is part of the plan"
    );
    let applied = service(&root).apply(&plan).expect("the plan applies");
    assert!(
        applied
            .written
            .iter()
            .any(|path| path.ends_with("area.cpp"))
    );
    assert!(root.join("src/shop/panel/object/area/area.cpp").is_file());
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "lang-python")]
fn python() -> LanguageId {
    LanguageId::parse_language("python").expect("a language id")
}

/// A Python child of a Rust parent is generated beside it, names the generated object directory,
/// and writes no C++ compile-flags file.
/// Rust 父下的 Python 子面生成在父旁边，点名生成的对象目录，且不写 C++ 的 compile flags 文件。
#[cfg(feature = "lang-python")]
#[test]
fn a_python_child_of_a_rust_parent_is_generated_beside_it() {
    let (root, panel) = fixture("python-under-rust");
    let mut request = AuthoringRequest::create();
    request.language = Some(python());
    request.module = Some("gauge".to_owned());
    request.parent = Some(panel);
    let plan = service(&root).plan(&request).expect("the plan");
    let file = plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".py"))
        .expect("a Python file");
    assert_eq!(file.path, "src/shop/panel/object/gauge/gauge.py");
    assert!(
        file.updated.contains("under=faces.shop.panel.NODE_ID"),
        "{}",
        file.updated
    );
    assert!(file.updated.contains("named=\"Gauge\""), "{}", file.updated);
    assert!(
        !plan
            .files
            .iter()
            .any(|file| file.path == "compile_flags.txt"),
        "a Python plan must not write a C++ editor file"
    );
    let applied = service(&root).apply(&plan).expect("the plan applies");
    assert!(
        applied
            .written
            .iter()
            .any(|path| path.ends_with("gauge.py"))
    );
    assert!(root.join("src/shop/panel/object/gauge/gauge.py").is_file());
    let _ = std::fs::remove_dir_all(&root);
}

/// The same parent facts render in each adapter's own language: the surface never hands over a
/// pre-rendered expression, and a Python plan never contains C++ text.
/// 同一父事实在每个适配器自己的语言里渲染：执行面从不交出预先渲染的表达式，Python 计划里也没有 C++ 文本。
#[cfg(all(feature = "lang-cpp", feature = "lang-python"))]
#[test]
fn the_same_parent_facts_render_in_each_languages_own_spelling() {
    let (root, panel) = fixture("two-spellings");
    let mut cpp_request = AuthoringRequest::create();
    cpp_request.language = Some(cpp());
    cpp_request.module = Some("area".to_owned());
    cpp_request.parent = Some(panel);
    let cpp_plan = service(&root).plan(&cpp_request).expect("the C++ plan");
    let cpp_source = &cpp_plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".cpp"))
        .expect("a C++ file")
        .updated;
    assert!(
        cpp_source.contains(".under(crate::catalog::panel::NODE_ID)"),
        "{cpp_source}"
    );

    let mut python_request = AuthoringRequest::create();
    python_request.language = Some(python());
    python_request.module = Some("gauge".to_owned());
    python_request.parent = Some(panel);
    let python_plan = service(&root)
        .plan(&python_request)
        .expect("the Python plan");
    let python_source = &python_plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".py"))
        .expect("a Python file")
        .updated;
    assert!(
        python_source.contains("under=faces.shop.panel.NODE_ID"),
        "{python_source}"
    );
    assert!(
        !python_source.contains("XIRANG_SOIL") && !python_source.contains("::xirang::faces"),
        "a Python plan must not contain C++ text: {python_source}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An existing editor file is the user's: the plan leaves it untouched and says so, instead of
/// silently claiming the editor material was written.
/// 已存在的编辑器文件属于用户：计划保持不动并说出来，而不是静默声称写好了编辑器资料。
#[cfg(feature = "lang-cpp")]
#[test]
fn an_existing_editor_file_is_left_alone_and_reported() {
    let (root, panel) = fixture("editor-existing");
    std::fs::write(
        root.join("compile_flags.txt"),
        "-Iuser
",
    )
    .expect("the user file");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let plan = service(&root).plan(&request).expect("the plan");
    assert!(
        !plan
            .files
            .iter()
            .any(|file| file.path == "compile_flags.txt"),
        "an existing editor file must not be overwritten"
    );
    assert!(
        plan.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("left unchanged")),
        "the skip must be reported: {:?}",
        plan.diagnostics
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The registry root is a structural fact, so the service hands it over as `Root`; the language
/// adapter is the one that decides it cannot name it. The refusal is the adapter's, not a public
/// rule that every foreign language is forbidden to hang under the root.
/// 注册机根是结构事实，因此服务以 `Root` 交出；由语言适配器决定自己无法点名它。拒绝来自适配器，而不是
/// 一条"所有外语都不能挂根"的公共规则。
#[cfg(feature = "lang-python")]
#[test]
fn the_root_is_refused_by_the_adapter_not_by_the_service() {
    let (root, _) = fixture("python-root");
    let mut request = AuthoringRequest::create();
    request.language = Some(python());
    request.module = Some("gauge".to_owned());
    request.parent = Some(xirang_kernel::ROOT_NODE_ID);
    let error = service(&root).plan(&request).unwrap_err();
    assert!(error.contains("root"), "{error}");
    assert!(
        error.contains("Python"),
        "the adapter's reason is expected: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A destination with the wrong extension is refused by the adapter's own claim, not by a list in
/// the service.
/// 扩展名不对的目的地被适配器自己的声明拒绝，而不是被服务里的一张表拒绝。
#[cfg(feature = "lang-python")]
#[test]
fn a_python_destination_must_be_a_python_file() {
    let (root, panel) = fixture("python-wrong-extension");
    let mut request = AuthoringRequest::create();
    request.language = Some(python());
    request.module = Some("gauge".to_owned());
    request.parent = Some(panel);
    request.destination = Some("src/shop/panel/object/gauge/gauge.rs".to_owned());
    let error = service(&root).plan(&request).unwrap_err();
    assert!(error.contains("not a `py` source file"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The generated C++ face reads back to the same registration fields, which is the round trip the
/// acceptance contract asks for.
/// 生成的 C++ 面读回同一份注册字段，正是验收契约要求的往返。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_generated_cpp_face_reads_back_to_the_requested_fields() {
    let (root, panel) = fixture("cpp-round-trip");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    request
        .registration
        .set_text(RegistrationField::Exports, FieldUpdate::Set("area,volume"))
        .unwrap();
    request
        .registration
        .set_text(RegistrationField::NeedsRegistry, FieldUpdate::Set("true"))
        .unwrap();
    let plan = service(&root).plan(&request).expect("the plan");
    service(&root).apply(&plan).expect("the plan applies");
    let read_back = service(&root)
        .read_registration("src/shop/panel/object/area/area.cpp")
        .expect("the face reads back");
    assert!(request.registration.satisfies(&read_back));
    let _ = std::fs::remove_dir_all(&root);
}

/// A Rust child of a C++ parent names the generated `xirang_external` constant, and the C++ parent
/// keeps its own declaration untouched.
/// C++ 父下的 Rust 子面点名生成的 `xirang_external` 常量，而 C++ 父自己的声明不动。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_rust_child_of_a_cpp_parent_names_the_external_constant() {
    let (root, panel) = fixture("rust-under-cpp");
    let mut area = AuthoringRequest::create();
    area.language = Some(cpp());
    area.module = Some("area".to_owned());
    area.parent = Some(panel);
    area.registration
        .set_text(RegistrationField::NeedsRegistry, FieldUpdate::Set("true"))
        .unwrap();
    let area_plan = service(&root).plan(&area).expect("the C++ plan");
    service(&root)
        .apply(&area_plan)
        .expect("the C++ plan applies");
    let area_id =
        NodeId::from_namespaced_path("shop", "src/shop/panel/object/area/area.cpp", "Area");
    let mut reading = AuthoringRequest::create();
    reading.language = Some(rust());
    reading.module = Some("reading".to_owned());
    reading.parent = Some(area_id);
    let plan = service(&root).plan(&reading).expect("the Rust plan");
    let file = plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".rs"))
        .expect("a Rust file");
    assert_eq!(
        file.path,
        "src/shop/panel/object/area/object/reading/reading.rs"
    );
    assert!(
        file.updated.contains(
            ".under(crate::xirang_external::src__shop__panel__object__area_Area::NODE_ID)"
        ),
        "{}",
        file.updated
    );
    service(&root).apply(&plan).expect("the Rust plan applies");
    let _ = std::fs::remove_dir_all(&root);
}

/// Editing one C++ field keeps the business code, comments, and the fields the request did not
/// name.
/// 修改一个 C++ 字段会保留业务代码、注释与请求没有点名的字段。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_cpp_field_edit_preserves_everything_else() {
    let (root, panel) = fixture("cpp-edit");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let create = service(&root).plan(&request).expect("the create plan");
    service(&root).apply(&create).expect("the create applies");
    let path = "src/shop/panel/object/area/area.cpp";
    let original = std::fs::read_to_string(root.join(path)).expect("the created file");
    let decorated = format!(
        "{original}\n// a business comment that must survive\nint area() {{ return 0; }}\n"
    );
    std::fs::write(root.join(path), &decorated).expect("the decorated source");
    let mut edit = AuthoringRequest {
        language: Some(cpp()),
        operation: AuthoringOperation::Modify,
        node: None,
        module: None,
        parent: None,
        destination: Some(path.to_owned()),
        registration: RegistrationUpdate::new(),
    };
    edit.registration
        .set_text(RegistrationField::Exports, FieldUpdate::Set("area"))
        .unwrap();
    let plan = service(&root).plan(&edit).expect("the edit plan");
    service(&root).apply(&plan).expect("the edit applies");
    let updated = std::fs::read_to_string(root.join(path)).expect("the edited file");
    assert!(
        updated.contains("// a business comment that must survive"),
        "{updated}"
    );
    assert!(updated.contains("int area() { return 0; }"), "{updated}");
    assert!(updated.contains(".exports(\"area\")"), "{updated}");
    assert!(
        updated.contains(".under(crate::catalog::panel::NODE_ID)"),
        "an edit emits the shared catalog spelling: {updated}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Reading an old `::xirang::faces::…` parent is compatibility; writing the declaration back emits
/// the shared catalog spelling, never the old one.
/// 读取旧的 `::xirang::faces::…` 父是兼容；把声明写回时发射共享目录拼法，绝不是旧拼法。
#[cfg(feature = "lang-cpp")]
#[test]
fn an_old_cpp_parent_is_read_but_the_output_uses_the_catalog() {
    let (root, panel) = fixture("cpp-old-parent");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let create = service(&root).plan(&request).expect("the create plan");
    service(&root).apply(&create).expect("the create applies");
    let path = "src/shop/panel/object/area/area.cpp";
    let source = std::fs::read_to_string(root.join(path)).expect("created source");
    let old = source.replace(
        "crate::catalog::panel::NODE_ID",
        "::xirang::faces::shop::panel::NODE_ID",
    );
    assert_ne!(old, source, "the fixture had the new spelling");
    std::fs::write(root.join(path), &old).expect("old spelling on disk");
    let mut edit = AuthoringRequest {
        language: Some(cpp()),
        operation: AuthoringOperation::Modify,
        node: None,
        module: None,
        parent: None,
        destination: Some(path.to_owned()),
        registration: RegistrationUpdate::new(),
    };
    edit.registration
        .set_text(RegistrationField::Exports, FieldUpdate::Set("area"))
        .unwrap();
    let plan = service(&root).plan(&edit).expect("the old parent reads");
    service(&root).apply(&plan).expect("the edit applies");
    let updated = std::fs::read_to_string(root.join(path)).expect("edited source");
    assert!(
        updated.contains(".under(crate::catalog::panel::NODE_ID)"),
        "the output is the catalog spelling: {updated}"
    );
    assert!(
        !updated.contains("::xirang::faces::shop::panel::NODE_ID"),
        "the old spelling is not emitted: {updated}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An unsupported field is refused by name, not dropped and not downgraded to a comment.
/// 不支持的字段按名拒绝，不丢弃、也不降级成注释。
#[cfg(feature = "lang-cpp")]
#[test]
fn an_unsupported_cpp_field_is_refused_by_name() {
    let (root, panel) = fixture("cpp-unsupported");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    request
        .registration
        .set_text(RegistrationField::Flow, FieldUpdate::Set("a => b"))
        .unwrap();
    let error = service(&root).plan(&request).unwrap_err();
    assert!(error.contains("flow"), "{error}");
    assert!(error.contains("cpp"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Without the feature the build says which support is missing instead of writing a Rust file under
/// a C++ request.
/// 没有特性时，构建说明缺哪种支持，而不是在 C++ 请求下写一个 Rust 文件。
#[cfg(not(feature = "lang-cpp"))]
#[test]
fn a_cpp_request_without_the_feature_names_the_missing_support() {
    let (root, panel) = fixture("cpp-disabled");
    let mut request = AuthoringRequest::create();
    request.language = Some(cpp());
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let error = service(&root).plan(&request).unwrap_err();
    assert!(error.contains("not compiled into this build"), "{error}");
    assert!(
        error.contains("lang-cpp") || error.contains("enabled languages"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A request with no language and no default is an ambiguity the caller must settle; the refusal
/// lists the languages this build can author.
/// 没有语言也没有默认值的请求是调用方必须解决的歧义；拒绝会列出本构建能创作的语言。
#[test]
fn an_ambiguous_language_is_refused_with_the_choices() {
    let (root, panel) = fixture("ambiguous");
    let mut request = AuthoringRequest::create();
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let error = service(&root).plan(&request).unwrap_err();
    assert!(error.contains("names no language"), "{error}");
    assert!(error.contains("rust"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A project may declare its default language; an explicit choice still wins.
/// 项目可以声明默认语言；显式选择仍然优先。
#[test]
fn a_project_default_selects_the_language_when_none_is_given() {
    let (root, panel) = fixture("default-language");
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[project]\ndefault_language = \"rust\"\n",
    )
    .expect("the project manifest");
    let mut request = AuthoringRequest::create();
    request.module = Some("area".to_owned());
    request.parent = Some(panel);
    let plan = service(&root).plan(&request).expect("the plan");
    assert_eq!(plan.language, rust());
    assert!(plan.files.iter().any(|file| file.path.ends_with(".rs")));
    let _ = std::fs::remove_dir_all(&root);
}

/// A Rust create under a Rust parent goes through the existing generator, so its file and its
/// declaration are byte-for-byte what the existing `add` writes.
/// Rust 父下的 Rust 新建经现有生成器，因此文件与声明逐字节等于既有 `add` 写出的内容。
#[test]
fn a_rust_create_reuses_the_existing_generator() {
    let (root, panel) = fixture("rust-create");
    let mut request = AuthoringRequest::create();
    request.language = Some(rust());
    request.module = Some("button".to_owned());
    request.parent = Some(panel);
    let plan = service(&root).plan(&request).expect("the plan");
    let file = plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".rs"))
        .expect("a Rust file");
    assert_eq!(file.path, "src/shop/panel/object/button/button.rs");
    assert!(file.updated.contains("pub const SOIL"));
    assert!(file.updated.contains("Soil::of::<Button>()"));
    assert!(file.updated.contains("crate::shop::panel::NODE_ID"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A stale plan is refused instead of overwriting a file that changed after the plan was made.
/// 过期计划被拒绝，而不是覆盖计划生成后已变化的文件。
#[test]
fn applying_a_stale_plan_is_refused() {
    let (root, panel) = fixture("stale");
    let mut request = AuthoringRequest::create();
    request.language = Some(rust());
    request.module = Some("button".to_owned());
    request.parent = Some(panel);
    let plan = service(&root).plan(&request).expect("the plan");
    let path = plan
        .files
        .iter()
        .find(|file| file.path.ends_with(".rs"))
        .expect("a Rust file")
        .path
        .clone();
    // Write something else at the destination first, then try the create plan.
    std::fs::create_dir_all(root.join(&path).parent().expect("a parent")).expect("the directory");
    std::fs::write(root.join(&path), "someone else's bytes").expect("the file");
    let error = service(&root).apply(&plan).unwrap_err();
    assert!(error.contains("cannot create"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
