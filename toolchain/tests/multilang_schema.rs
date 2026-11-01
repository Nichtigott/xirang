#![cfg(feature = "cli")]
//! `xirang schema` prints a JSON Schema for `xirang.toml`. This test keeps the schema from drifting
//! from the parser: every top-level table and every built-in language table the schema lists must
//! be accepted by the parser, and a key the schema does not list must be refused.
//! `xirang schema` 打印 `xirang.toml` 的 JSON Schema。本测试防 schema 与解析器漂移：schema 列出的每个
//! 顶层表与内置语言表都必须被解析器接受，而 schema 未列出的键必须被拒绝。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-schema-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn minimal_manifest(language: &str) -> String {
    // Two script languages name their standard precisely; every other language takes a free-form
    // standard string, so the table itself is what this test is checking.
    // 两个脚本语言精确点名 standard；其它语言接受自由的 standard 字符串，因此本测试查的是表本身。
    let standard = match language {
        "javascript" => "commonjs",
        "typescript" => "strip",
        _ => "x",
    };
    format!(
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.{language}]\nstandard = \"{standard}\"\n\n[dependencies]\n"
    )
}

#[test]
fn the_schema_names_the_tables_and_languages_the_parser_accepts() {
    let root = scratch("document");
    let printed = invoke(&root, &["schema"]);
    assert!(printed.status.success(), "{}", both(&printed));
    let schema: serde_json::Value =
        serde_json::from_slice(&printed.stdout).expect("`xirang schema` prints JSON");
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    let properties = schema["properties"].as_object().unwrap();
    let mut names: Vec<&str> = properties.keys().map(String::as_str).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "build",
            "dependencies",
            "export",
            "language",
            "overrides",
            "package",
            "schema",
            "sources"
        ]
    );

    // Every built-in language table the schema offers is a table the parser takes: the parser's
    // complaint is about the missing `src/`, not about the language.
    // schema 提供的每个内置语言表都能被解析器接收：解析器抱怨的是缺 `src/`，不是语言本身。
    let languages = schema["properties"]["language"]["properties"]
        .as_object()
        .unwrap();
    let mut ids: Vec<&str> = languages.keys().map(String::as_str).collect();
    ids.sort();
    assert_eq!(ids.len(), 13, "{ids:?}");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    for id in &ids {
        std::fs::write(project.join("xirang.toml"), minimal_manifest(id)).unwrap();
        let plan = invoke(&project, &["plan"]);
        let text = both(&plan);
        assert!(
            !text.contains("no backend is registered for language")
                && !text.contains("unknown table"),
            "`{id}` is in the schema but the parser does not accept that table:\n{text}"
        );
    }

    // A key the schema does not list is refused by the parser.
    // schema 未列出的键被解析器拒绝。
    std::fs::write(
        project.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\
         licence = \"MIT\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    let refused = invoke(&project, &["plan"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(both(&refused).contains("unknown"), "{}", both(&refused));
}

#[test]
fn the_schema_can_be_written_to_a_file_for_an_editor() {
    let root = scratch("output");
    let target = root.join("xirang.schema.json");
    let written = invoke(&root, &["schema", "--output", target.to_str().unwrap()]);
    assert!(written.status.success(), "{}", both(&written));
    let text = std::fs::read_to_string(&target).unwrap();
    let schema: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(schema["title"], "xirang.toml");
    assert!(
        schema["properties"]["language"]["patternProperties"]
            .as_object()
            .unwrap()
            .keys()
            .any(|pattern| pattern.starts_with("^[a-z]"))
    );
}
