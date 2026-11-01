//! covers `app/source_index.rs`, `app/search_queries.rs`.
//! 覆盖 `app/source_index.rs`、`app/search_queries.rs`。
//!
//! Source index, call scanning, symbol search, and MIR evidence tests.
//! 源码索引、调用扫描、符号搜索与 MIR 证据测试。

use super::*;

#[test]
fn function_bodies_capture_calls_but_ignore_use_and_registration_text() {
    let bodies = function_bodies(
        "use crate::Thing;\n\nfn outer() { inner(); crate::control_object!(); }\nfn inner() {}",
    );
    assert_eq!(
        bodies
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["outer", "inner"]
    );
    assert!(bodies[0].1.contains("inner()"));
    assert!(!body_calls("control_object!(inner());", "inner"));
    assert!(!body_calls("// inner()\nlet text = \"inner()\";", "inner"));
    assert!(body_calls("crate::inner ();", "inner"));
}

#[test]
fn function_index_accepts_qualified_visibility_and_impl_methods() {
    let source = r#"
            // fn ignored() {}
            pub(crate) async fn load(value: usize)
            where
                usize: Copy,
            {
                self.render::<usize>(value);
            }

            impl Widget {
                unsafe fn render(&self, value: usize) {
                    Type::paint(value);
                }
            }
        "#;
    let functions = function_symbols(source);
    assert_eq!(
        functions
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["load", "render"]
    );
    assert!(functions[0].signature.contains("pub(crate) async fn load"));
    assert!(functions[0].body.contains("self.render::<usize>(value)"));
    assert!(body_calls(&functions[0].body, "render"));
    assert!(body_calls(&functions[0].body, "render"));
    assert!(body_calls(&functions[1].body, "paint"));
    assert!(!body_calls(&functions[1].body, "load"));
}

#[test]
fn source_preview_range_is_limited_to_the_selected_function() {
    let source = [
        "fn first() {",
        "    one();",
        "}",
        "",
        "pub(crate) fn second() {",
        "    two();",
        "}",
    ];
    let range = app_function_source_range(&source, "second").expect("function range");
    assert_eq!(range, (4, 6));
}

#[test]
fn call_scanner_ignores_use_and_macro_but_accepts_qualified_calls() {
    let body = r#"
            use crate::paint;
            control_object!(paint());
            self.paint::<Color>();
            Widget::layout();
            // paint()
            let text = "layout()";
        "#;
    assert!(body_calls(body, "paint"));
    assert!(body_calls(body, "layout"));
    assert!(!body_calls("use crate::paint;", "paint"));
    assert!(!body_calls("control_object!(paint());", "paint"));
    assert!(!body_calls("let text = \"paint()\";", "paint"));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn call_relations_report_real_cross_file_function_calls() {
    let Some(app) = fixture_app() else {
        return;
    };
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let (callers, callees) = app.call_relations(node_editor.id, "preview_canvas_width");
    assert!(
        callers
            .iter()
            .any(|item| item.function == "preview_canvas_width_traced")
    );
    assert!(
        callees
            .iter()
            .any(|item| item.function == "clamp_canvas_width")
    );
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("Canvas call");
    let (reverse_callers, _) = app.call_relations(canvas.node, "clamp_canvas_width");
    assert!(
        reverse_callers
            .iter()
            .any(|item| item.function == "preview_canvas_width")
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn mir_candidates_are_optional_and_keep_live_evidence_distinct() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.mir_graph = Some(
            MirGraph::from_jsonl(
                "{\"kind\":\"call\",\"caller\":\"preview_canvas_width\",\"callee\":\"clamp_canvas_width\",\"mir_line\":7}\n",
            )
            .unwrap(),
        );
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let (_, callees) = app.call_relations(node_editor.id, "preview_canvas_width");
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("MIR candidate should resolve to a known function");
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.to_owned(),
    };
    // Live evidence comes from a trace, MIR from the graph: install the trace
    // that actually observed this call, so the assertion below distinguishes the
    // two instead of comparing against the standalone demo sample.
    // 实测证据来自追踪，MIR 来自图：安装一条真正观测到这次调用的追踪，使下面的断言区分
    // 两者，而不是拿独立演示样本来比较。
    app.runtime_trace = super::fixtures::fixture_live_trace(&center, canvas);
    assert_eq!(app.call_evidence(&center, canvas), CallEvidence::Live);
    assert_eq!(app.mir_candidates_for("preview_canvas_width").len(), 1);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn directory_style_source_paths_resolve_to_attached_files() {
    let Some(_fixture) = fixture_project() else {
        return;
    };
    let path = source_path_for("control/object/node_editor/object/");
    assert!(path.is_file());
    assert!(path.ends_with("control/object/node_editor/object/object.rs"));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_file_adds_compact_source_symbols() {
    let Some(app) = fixture_app() else {
        return;
    };
    let rows = app.search_rows("control/object/node_editor/node_editor.rs");
    assert!(rows.iter().any(|row| row.function == "NodeEditor"));
    assert!(rows.iter().all(|row| !row.text.contains("declared-at=")));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_function_name_finds_the_source_symbol() {
    let Some(app) = fixture_app() else {
        return;
    };
    let rows = app.search_rows("accept_canvas");
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
    assert!(
        rows.iter()
            .any(|row| row.signature.contains("accept_canvas"))
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_parameter_name_finds_its_function() {
    let Some(app) = fixture_app() else {
        return;
    };
    let rows = app.search_rows("canvas_name");
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
}

/// A declaration placed in an island resolves to the file the **build** reads, not to
/// `src/<its path>`: the object ledger is the only place that spelling is written down, so the
/// editor asks it first. The Rust half is the control — an identity spelling still resolves under
/// the scan root, so this is not "everything is package-relative now".
/// 放在孤岛里的声明解析到**构建**读的那个文件，而不是 `src/<那个路径>`：对象台账是那个拼写唯一的
/// 落笔处，因此编辑器先问它。Rust 那一半是控制组——身份拼写仍按扫描根解析，所以这不是"从此一切都
/// 是包内相对"。
#[test]
fn an_island_declaration_resolves_where_the_build_reads_it() {
    // The shared prelude exports `source_path_for` only with the fixture feature, and this pin
    // deliberately does not need a fixture project: it builds its own two-file tree, so it must
    // run in the default and `mcp,cli` faces too.
    // 共享前导只在夹具特性下导出 `source_path_for`，而这条钉子有意不需要夹具工程：它自己搭一棵两文件
    // 的树，因此必须在默认与 `mcp,cli` 面上也跑。
    use crate::studio::studio::app::project_context::source_path_for;

    let root = std::env::temp_dir().join(format!(
        "xirang-studio-island-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/panel")).expect("scan root");
    std::fs::create_dir_all(root.join("components/area")).expect("island");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"island\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// entry\n").expect("entry");
    std::fs::write(root.join("src/panel/panel.rs"), "// panel\n").expect("panel");
    std::fs::write(root.join("components/area/area.cpp"), "// area\n").expect("area");
    std::fs::write(
        root.join("xirang.objects.tsv"),
        format!(
            "# xirang objects v1\n{}\troot/panel/Area\tcomponents/area/area.cpp\tcomponents/area\t-\n",
            "0".repeat(32)
        ),
    )
    .expect("ledger");

    select_project(root.clone(), root.join("Cargo.toml"), "island");
    let island = source_path_for("components/area/area.cpp");
    assert_eq!(
        island,
        root.join("components/area/area.cpp"),
        "the editor opens the file the build reads"
    );
    assert!(island.is_file(), "and that file exists");
    let rust = source_path_for("panel/panel.rs");
    assert_eq!(
        rust,
        root.join("src/panel/panel.rs"),
        "a Rust face's identity spelling still resolves under the scan root"
    );
    assert!(rust.is_file());

    let _ = std::fs::remove_dir_all(&root);
    clear_project_context();
}
