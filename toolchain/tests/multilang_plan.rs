#![cfg(feature = "cli")]
//! End-to-end: `xirang plan` previews the actions a build would run, in text or JSON, and writes
//! nothing.
//! 端到端：`xirang plan` 预览一次构建会跑的动作（文本或 JSON），且不写任何文件。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory with one C++ library scaffolded inside.
/// 一个全新的临时目录，里面已生成一个 C++ 库。
fn project(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-plan-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let created = invoke(&root, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", both(&created));
    root.join("widget")
}

/// Run the binary in `directory`.
/// 在 `directory` 里运行该二进制。
fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

/// Both streams, for a failure message.
/// 两条输出流，供失败信息使用。
fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn the_plan_previews_the_actions_and_writes_nothing() {
    let root = project("text");
    let planned = invoke(&root, &["plan"]);
    assert!(planned.status.success(), "{}", both(&planned));
    let text = String::from_utf8_lossy(&planned.stdout).into_owned();
    assert!(text.contains("action(s)"), "{text}");
    assert!(text.contains("nothing was executed"), "{text}");
    assert!(text.contains("plan "), "{text}");
    assert!(
        !root.join("target").exists(),
        "planning must not create the build directory or the fact cache"
    );
}

#[test]
fn the_plan_can_be_read_as_data() {
    let root = project("json");
    let planned = invoke(&root, &["plan", "--json"]);
    assert!(planned.status.success(), "{}", both(&planned));
    let document: serde_json::Value =
        serde_json::from_slice(&planned.stdout).expect("the plan is one JSON document");
    assert_eq!(document["protocol"], 1);
    assert_eq!(document["executed"], false);
    assert!(
        document["actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()),
        "{document}"
    );
    let first = &document["actions"][0];
    assert!(
        first["command"]
            .as_array()
            .is_some_and(|argv| !argv.is_empty()),
        "{first}"
    );
    assert!(first["kind"].is_string(), "{first}");
    assert!(document["plan"].is_string(), "{document}");
    assert!(
        !root.join("target").exists(),
        "planning must not create the build directory even as JSON"
    );
}

#[test]
fn planning_outside_a_project_is_refused_by_name() {
    let root = std::env::temp_dir().join(format!(
        "xirang-plan-none-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let planned = invoke(&root, &["plan"]);
    assert!(!planned.status.success(), "{}", both(&planned));
    let message = both(&planned);
    assert!(message.contains("xirang.toml"), "{message}");
}

#[test]
fn a_second_language_table_is_refused_even_when_it_carries_no_keys() {
    let root = project("two-language-tables");
    let manifest = root.join("xirang.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    // A table declares the unit's language just as much as its `standard` line does, so a second
    // table with no keys at all is still a second declaration — and it used to be the shape that
    // let a unit compile one language while naming two.
    // 表本身就在声明单元的语言，与它的 `standard` 行同理；因此一个键都没写的第二个表仍然是第二次
    // 声明——而过去正是这种形状让一个单元点名两门语言、却只编其中一门。
    let text = format!("{text}\n[language.rust]\n");
    std::fs::write(&manifest, text).unwrap();
    let planned = invoke(&root, &["plan"]);
    assert!(!planned.status.success(), "{}", both(&planned));
    let stderr = String::from_utf8_lossy(&planned.stderr).into_owned();
    assert!(
        stderr.contains("one unit has one source language"),
        "{stderr}"
    );
    assert!(stderr.contains("[language.cpp]"), "{stderr}");
}

#[test]
fn a_language_table_without_a_standard_is_refused_rather_than_defaulting_to_cpp() {
    let root = project("no-standard");
    let manifest = root.join("xirang.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    let text = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("standard"))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("[language.cpp]", "[language.rust]");
    std::fs::write(&manifest, text).unwrap();
    let planned = invoke(&root, &["plan"]);
    assert!(!planned.status.success(), "{}", both(&planned));
    let stderr = String::from_utf8_lossy(&planned.stderr).into_owned();
    assert!(stderr.contains("names no `standard`"), "{stderr}");
}

/// **Two units that fold to one artefact name are refused** — the collision is otherwise silent.
/// **两个折叠到同一产物名的单元会被拒绝**——否则这次碰撞是静默的。
///
/// Measured before the refusal existed: a project holding `math-lib` and `math_lib` built with
/// `6 action(s)`, and `target/xirang/build/<plan>/lib` held a single `libmath_lib.a`; when both units
/// also had `src/lib.cpp`, `obj/` held one `math_lib__lib.o`. Only *equal* package names get a
/// location suffix, so differing names that fold alike are the case nothing else catches.
/// 拒绝存在之前实测：同时放着 `math-lib` 与 `math_lib` 的项目构建出 `6 action(s)`，而
/// `target/xirang/build/<plan>/lib` 里只有一个 `libmath_lib.a`；两个单元还各有 `src/lib.cpp` 时，
/// `obj/` 里只有一个 `math_lib__lib.o`。只有**同名**包才会加位置后缀，因此"名字不同、折叠后相同"
/// 正是别处都拦不住的那一种。
#[test]
fn two_units_that_fold_to_one_artefact_name_are_refused() {
    let root = project("folded-artefact-names");
    for (directory, name) in [
        ("units/math-lib", "math-lib"),
        ("units/math_lib", "math_lib"),
    ] {
        let unit = root.join(directory);
        std::fs::create_dir_all(unit.join("src")).unwrap();
        std::fs::write(
            unit.join("xirang.toml"),
            format!(
                "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
                 [language.cpp]\nstandard = \"17\"\n"
            ),
        )
        .unwrap();
        std::fs::write(
            unit.join("src/lib.cpp"),
            "#include <cstdint>\nextern \"C\" std::int64_t one() { return 1; }\n",
        )
        .unwrap();
    }
    let manifest = root.join("xirang.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        format!(
            "{text}math-lib = {{ path = \"units/math-lib\" }}\n\
             math_lib = {{ path = \"units/math_lib\" }}\n"
        ),
    )
    .unwrap();

    let planned = invoke(&root, &["plan"]);
    assert!(!planned.status.success(), "{}", both(&planned));
    let stderr = String::from_utf8_lossy(&planned.stderr).into_owned();
    assert!(
        stderr.contains("both build artefacts named `math_lib`"),
        "the refusal names the shared stem: {stderr}"
    );
    assert!(
        stderr.contains("units/math-lib") && stderr.contains("units/math_lib"),
        "and both units, so the reader knows what to rename: {stderr}"
    );
}
