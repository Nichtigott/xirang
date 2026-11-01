//! The Studio form's language choice, wired to the shared authoring service.
//! Studio 表单的语言选择，接到共享创作服务。
//!
//! covers `app/state/forms.rs`, `app/overlay/face_form.rs`, `app/mutations.rs`.
//! 覆盖 `app/state/forms.rs`、`app/overlay/face_form.rs`、`app/mutations.rs`。

use super::*;
use crate::run_method::face_field;

/// A throwaway package with one Rust parent face that owns a registry.
/// 一个一次性包，含一个自持注册机的 Rust 父面。
fn fixture(label: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-studio-lang-{label}-{suffix}"));
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
    root
}

/// `l` cycles the languages this build can author, and the rows the selected language does not
/// carry are marked so an author does not type them.
/// `l` 循环本构建可以创作的语言，并标记选中语言不携带的行，免得作者去键入它们。
#[test]
fn the_add_form_cycles_the_languages_and_marks_uncarried_rows() {
    let root = fixture("cycle");
    select_project(root.clone(), root.join("Cargo.toml"), "shop");
    let mut app = App::load_app();
    let panel = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.kind == "Panel")
        .expect("the parent face loaded")
        .id;
    app.overlay = Some(Overlay::Add(AddState::new(panel)));
    let Some(Overlay::Add(before)) = app.overlay.as_ref() else {
        panic!("the add form is open");
    };
    assert_eq!(before.language, "rust");
    assert!(before.is_carried(face_field::PRESET));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('l')));
    let Some(Overlay::Add(after)) = app.overlay.as_ref() else {
        panic!("the add form stays open");
    };
    #[cfg(feature = "lang-cpp")]
    {
        assert_eq!(after.language, "cpp");
        assert!(!after.is_carried(face_field::PRESET));
        assert!(after.is_carried(face_field::KIND));
        assert!(after.language_summary().contains("Rust type"));
    }
    #[cfg(all(not(feature = "lang-cpp"), feature = "lang-python"))]
    {
        assert_eq!(
            after.languages,
            vec!["rust".to_owned(), "python".to_owned()]
        );
        assert_eq!(after.language, "python");
        assert!(after.is_carried(face_field::KIND));
    }
    #[cfg(not(any(feature = "lang-cpp", feature = "lang-python")))]
    {
        assert_eq!(after.languages, vec!["rust".to_owned()]);
        assert_eq!(after.language, "rust");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The form's submit reaches the shared authoring service: a C++ face is generated under the Rust
/// parent and the file exists with its declaration.
/// 表单提交抵达共享创作服务：C++ 面在 Rust 父下生成，文件带着声明存在。
#[cfg(feature = "lang-cpp")]
#[test]
fn submitting_the_add_form_in_cpp_writes_a_real_declaration() {
    let root = fixture("submit");
    select_project(root.clone(), root.join("Cargo.toml"), "shop");
    let mut app = App::load_app();
    let panel = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.kind == "Panel")
        .expect("the parent face loaded")
        .id;
    let mut add = AddState::new(panel);
    add.language = "cpp".to_owned();
    add.language_index = add
        .languages
        .iter()
        .position(|language| language == "cpp")
        .expect("cpp is offered by this build");
    add.carried = {
        let mut rows = std::collections::BTreeSet::new();
        rows.insert(face_field::MODULE);
        rows.insert(face_field::KIND);
        rows.insert(face_field::EXPORTS);
        rows.insert(face_field::NEEDS_REGISTRY);
        rows
    };
    add.values[face_field::MODULE] = "area".to_owned();
    add.values[face_field::KIND] = "Area".to_owned();
    add.values[face_field::EXPORTS] = "area".to_owned();
    app.submit_add(&add);
    let path = root.join("src/shop/panel/object/area/area.cpp");
    assert!(path.is_file(), "{} was not written", path.display());
    let text = std::fs::read_to_string(&path).expect("the generated source");
    assert!(text.contains("XIRANG_SOIL"), "{text}");
    assert!(text.contains("constexpr auto SOIL"), "{text}");
    assert!(text.contains(".named(\"Area\")"), "{text}");
    assert!(
        text.contains(".under(crate::catalog::panel::NODE_ID)"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
