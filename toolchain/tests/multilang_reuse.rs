#![cfg(feature = "cli")]
//! End-to-end: a warm `xirang build` reuses the compile actions whose outputs still hash to what
//! the stamp recorded, and anything missing or edited runs again. Nothing here reads the planner:
//! the evidence file says which actions actually ran.
//! 端到端：热构建复用产物仍与印记摘要相符的编译动作；缺失或被改动的产物会重新运行。这里不读规划器
//! 的内部：证据文件说明哪些动作真的跑过。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory, so two runs never share a plan or an evidence file.
/// 一个全新的临时目录，使两次运行不共享计划或证据文件。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-multilang-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
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

/// Both streams, for a readable failure message.
/// 两路输出，失败时容易读。
fn both(output: &Output) -> String {
    format!(
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The evidence document of the most recent build directory.
/// 最近一次构建目录里的证据文档。
fn evidence(directory: &Path) -> serde_json::Value {
    let builds = directory.join("target/xirang/build");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&builds).unwrap() {
        let path = entry.unwrap().path().join("evidence.json");
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        let modified = metadata.modified().unwrap();
        if newest.as_ref().is_none_or(|(at, _)| modified > *at) {
            newest = Some((modified, path));
        }
    }
    let (_, path) = newest.expect("no evidence.json was written");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Whether every action of the evidence was reused from its stamp.
/// 证据里的每条动作是否都从印记复用了。
fn all_reused(evidence: &serde_json::Value) -> bool {
    evidence["actions"].as_array().is_some_and(|actions| {
        !actions.is_empty()
            && actions
                .iter()
                .all(|action| action["reused"] == serde_json::Value::Bool(true))
    })
}

/// Scaffold a C++ library and return its directory after a successful build.
/// 搭一个 C++ 库，并在一次成功构建之后返回它的目录。
fn built(label: &str) -> (PathBuf, PathBuf, serde_json::Value) {
    let scratch = scratch(label);
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("widget");
    let first = invoke(&project, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let cold = evidence(&project);
    (scratch, project, cold)
}

/// Every regular file under `directory` whose extension is `extension`.
/// `directory` 下扩展名为 `extension` 的每个普通文件。
fn files_with(directory: &Path, extension: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_with(&path, extension, found);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            found.push(path);
        }
    }
}

#[test]
fn a_warm_build_reuses_actions_whose_outputs_still_hold() {
    let (_scratch, project, cold) = built("reuse");
    assert!(
        cold["actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()),
        "a C++ library build must have compile actions: {cold}"
    );
    assert!(
        !all_reused(&cold),
        "nothing can be reused before a stamp exists: {cold}"
    );
    let second = invoke(&project, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let warm = evidence(&project);
    assert_eq!(warm["plan"], cold["plan"], "the inputs did not change");
    assert!(
        all_reused(&warm),
        "a warm build must reuse every action whose outputs still hold: {warm}"
    );
}

#[test]
fn an_edited_output_is_restored_only_from_bytes_that_match_the_stamp() {
    let (_scratch, project, cold) = built("reuse-edited");
    let directory = build_directory(&cold);
    let mut archives = Vec::new();
    files_with(&directory, "a", &mut archives);
    let archive = archives
        .first()
        .expect("a C++ library build writes an archive")
        .clone();
    let recorded = std::fs::read(&archive).unwrap();
    std::fs::write(&archive, b"scribbled over").unwrap();
    let restored = invoke(&project, &["build"]);
    assert!(restored.status.success(), "{}", both(&restored));
    let after = evidence(&project);
    assert!(
        after["actions"].as_array().is_some_and(|actions| actions
            .iter()
            .any(|action| action["restored"] == serde_json::Value::Bool(true))),
        "an edited output must be restored from the content store, and say so: {after}"
    );
    assert_eq!(
        std::fs::read(&archive).unwrap(),
        recorded,
        "the restored archive must be the bytes the stamp recorded"
    );
    // Without the content store the toolchain must fail on bytes it cannot read, not report a
    // successful build.
    remove_tree(&project.join("target/xirang/cas"));
    std::fs::write(&archive, b"scribbled over").unwrap();
    let refused = invoke(&project, &["build"]);
    assert!(
        !refused.status.success(),
        "no stamp and no content store means the action runs; the toolchain must report the failure: {}",
        both(&refused)
    );
}

#[test]
fn a_source_change_lands_in_a_new_plan_and_does_not_reuse() {
    let (_scratch, project, cold) = built("reuse-changed");
    let builds = project.join("target/xirang/build");
    let before = std::fs::read_dir(&builds).unwrap().count();
    let mut sources = Vec::new();
    files_with(&project.join("src"), "cpp", &mut sources);
    let source = sources.first().expect("the scaffold writes a C++ source");
    let mut text = std::fs::read_to_string(source).unwrap();
    text.push_str("\n// one more line\n");
    std::fs::write(source, text).unwrap();
    let rebuilt = invoke(&project, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let after = evidence(&project);
    assert_ne!(
        after["plan"], cold["plan"],
        "a changed source must land in a different plan identity"
    );
    assert!(
        after["actions"].as_array().is_some_and(|actions| actions
            .iter()
            .all(|action| action["reused"] == serde_json::Value::Bool(false))),
        "a new plan identity must not read another plan's stamps: {after}"
    );
    assert_eq!(
        std::fs::read_dir(&builds).unwrap().count(),
        before + 1,
        "the new identity gets its own build directory; the old one stays for comparison"
    );
}

/// Remove a file or a directory tree, ignoring what is already absent.
/// 删除一个文件或目录树，已经不存在的不算错。
fn remove_tree(path: &Path) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            remove_tree(&entry.unwrap().path());
        }
        std::fs::remove_dir(path).unwrap();
    } else if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
}

/// The build directory named by an evidence document.
/// 证据文档点名的构建目录。
fn build_directory(evidence: &serde_json::Value) -> PathBuf {
    PathBuf::from(
        evidence["directory"]
            .as_str()
            .expect("evidence names its build directory"),
    )
}

#[test]
fn outputs_deleted_from_the_build_directory_are_restored_from_the_content_store() {
    let (_scratch, project, cold) = built("reuse-restore");
    let directory = build_directory(&cold);
    let mut archives = Vec::new();
    files_with(&directory, "a", &mut archives);
    let archive = archives
        .first()
        .expect("a C++ library build writes an archive")
        .clone();
    let recorded = std::fs::read(&archive).unwrap();
    remove_tree(&directory.join("obj"));
    remove_tree(&directory.join("lib"));
    let rebuilt = invoke(&project, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let after = evidence(&project);
    assert!(
        all_reused(&after),
        "every action whose stamp survives must be reused: {after}"
    );
    assert!(
        after["actions"].as_array().is_some_and(|actions| actions
            .iter()
            .any(|action| action["restored"] == serde_json::Value::Bool(true))),
        "the missing outputs must be restored from the content store: {after}"
    );
    assert_eq!(
        std::fs::read(&archive).unwrap(),
        recorded,
        "the restored archive must be the same bytes"
    );
}

#[test]
fn a_build_resumes_at_the_first_action_without_a_stamp() {
    let (_scratch, project, cold) = built("reuse-resume");
    let directory = build_directory(&cold);
    let stamps = directory.join("stamps");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&stamps)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    names.sort();
    let last = names
        .last()
        .expect("a successful build writes one stamp per action")
        .clone();
    std::fs::remove_file(&last).unwrap();
    let rebuilt = invoke(&project, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let after = evidence(&project);
    let actions = after["actions"].as_array().expect("actions are listed");
    assert!(
        actions
            .iter()
            .any(|action| action["reused"] == serde_json::Value::Bool(true)),
        "the actions before the missing stamp must be reused: {after}"
    );
    assert!(
        actions
            .iter()
            .any(|action| action["reused"] == serde_json::Value::Bool(false)),
        "the action without a stamp must run again: {after}"
    );
}

/// Every plan directory the project keeps, sorted by name.
/// 项目保存的每个计划目录，按名字排序。
fn plan_ids(project: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(project.join("target/xirang/build")) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = read
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

#[test]
fn a_same_size_same_mtime_rewrite_is_not_a_cache_hit() {
    let (_scratch, project, _) = built("facts-same-size");
    let source = project.join("src/widget.cpp");
    let before = std::fs::metadata(&source).unwrap();
    let original = std::fs::read_to_string(&source).unwrap();
    let rewritten = original.replace("return 42", "return 43");
    assert_eq!(
        rewritten.len(),
        original.len(),
        "the rewrite keeps the size"
    );
    std::fs::write(&source, &rewritten).unwrap();
    std::fs::File::open(&source)
        .unwrap()
        .set_modified(before.modified().unwrap())
        .unwrap();
    let after = std::fs::metadata(&source).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    let ids = plan_ids(&project);
    let rebuilt = invoke(&project, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let grown = plan_ids(&project);
    assert!(
        grown.len() > ids.len(),
        "same size and same mtime is still a different input: {ids:?} -> {grown:?}"
    );
}

#[test]
fn adding_a_source_lands_in_a_new_plan_and_deleting_it_returns_to_the_old_one() {
    let (_scratch, project, _) = built("facts-add-delete");
    let original = plan_ids(&project);
    let extra = project.join("src/extra.cpp");
    std::fs::write(&extra, "namespace widget {\nint extra() { return 1; }\n}\n").unwrap();
    let added = invoke(&project, &["build"]);
    assert!(added.status.success(), "{}", both(&added));
    let with_extra = plan_ids(&project);
    assert!(
        with_extra.len() > original.len(),
        "{original:?} -> {with_extra:?}"
    );
    std::fs::remove_file(&extra).unwrap();
    let deleted = invoke(&project, &["build"]);
    assert!(deleted.status.success(), "{}", both(&deleted));
    let used = evidence(&project)["plan"].as_str().unwrap().to_owned();
    assert!(
        original.contains(&used),
        "the deletion is an input change too, and its plan is the original one: {used} not in {original:?}"
    );
}

#[test]
fn a_corrupt_fact_cache_is_ignored_and_replaced() {
    let (_scratch, project, _) = built("facts-corrupt");
    let cache = project.join("target/xirang/facts.json");
    assert!(cache.is_file(), "the build writes the fact cache");
    std::fs::write(&cache, "{not a cache").unwrap();
    let before = plan_ids(&project);
    let rebuilt = invoke(&project, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    assert_eq!(
        plan_ids(&project),
        before,
        "a cache miss must not change the identity"
    );
    let text = std::fs::read_to_string(&cache).unwrap();
    assert!(
        text.contains("sha256"),
        "the next build writes a readable cache: {text}"
    );
}

/// A source file's path is part of the plan identity: renaming the file (same bytes) must land
/// in a new plan, and no stamp of the old plan may be reused.
/// 源文件路径是计划身份的一部分：改名（字节不变）必须落进新计划，旧计划的任何印记都不得被复用。
#[test]
fn renaming_a_source_file_lands_in_a_new_plan() {
    let (_scratch, project, cold) = built("rename");
    let mut sources = Vec::new();
    files_with(&project.join("src"), "cpp", &mut sources);
    assert_eq!(
        sources.len(),
        1,
        "the scaffold should have one source: {sources:?}"
    );
    std::fs::rename(&sources[0], project.join("src/renamed.cpp")).unwrap();
    let rebuilt = invoke(&project, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let warm = evidence(&project);
    assert_ne!(
        warm["plan"], cold["plan"],
        "a renamed source file is a different input path"
    );
    assert!(
        !all_reused(&warm),
        "no stamp of the old plan may be reused after a rename: {warm}"
    );
}

/// A dependency's directory is part of the plan identity: moving the dependency and repointing
/// the manifest must land in a new plan, not reuse the old one.
/// 依赖目录属于计划身份：把依赖搬走并改写清单里的路径，必须落进新计划而不是复用旧计划。
#[test]
fn moving_a_path_dependency_lands_in_a_new_plan() {
    let scratch = scratch("move-dependency");
    let helper = scratch.join("helper");
    std::fs::create_dir_all(helper.join("src")).unwrap();
    std::fs::write(
        helper.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"helper\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n",
    )
    .unwrap();
    std::fs::write(
        helper.join("src/helper.cpp"),
        "int helper_value() { return 7; }\n",
    )
    .unwrap();
    let app = scratch.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    let manifest = |path: &str| {
        format!(
            "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n\
             helper = {{ path = \"{path}\" }}\n"
        )
    };
    std::fs::write(app.join("xirang.toml"), manifest("../helper")).unwrap();
    std::fs::write(
        app.join("src/app.cpp"),
        "int helper_value();\nint app_value() { return helper_value(); }\n",
    )
    .unwrap();
    let first = invoke(&app, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let before = evidence(&app)["plan"].as_str().unwrap().to_owned();
    std::fs::rename(&helper, scratch.join("moved")).unwrap();
    std::fs::write(app.join("xirang.toml"), manifest("../moved")).unwrap();
    let second = invoke(&app, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let after = evidence(&app)["plan"].as_str().unwrap().to_owned();
    assert_ne!(
        before, after,
        "moving a path dependency must land in a new plan"
    );
}

/// Write one C++ library package at `directory`.
/// 在 `directory` 写一个 C++ 库包。
fn library_package(directory: &Path, name: &str) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        directory.join("src/value.cpp"),
        "int greeter_value() { return 5; }\n",
    )
    .unwrap();
}

/// Write the host project: a library that depends on `greeter` through the `team` source.
/// 写宿主项目：一个经 `team` 源依赖 `greeter` 的库。
fn application(directory: &Path, table: &str) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n{table}\n[dependencies]\ngreeter = {{ source = \"team\", package = \"greeter\" }}\n"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src/lib.cpp"), "int app() { return 1; }\n").unwrap();
}

#[test]
fn switching_the_source_a_dependency_comes_from_lands_in_a_new_plan() {
    let scratch = scratch("source-switch");
    library_package(&scratch.join("team/greeter"), "greeter");
    library_package(&scratch.join("team2/greeter"), "greeter");
    let app = scratch.join("app");
    application(&app, "[sources]\nteam = { path = \"../team\" }\n");
    let first = invoke(&app, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let before = plan_ids(&app);
    assert_eq!(before.len(), 1, "{before:?}");

    application(&app, "[sources]\nteam = { path = \"../team2\" }\n");
    let second = invoke(&app, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let after = plan_ids(&app);
    assert_eq!(after.len(), 2, "{after:?}");
    assert!(after.iter().any(|id| id != &before[0]), "{after:?}");
}

#[test]
fn switching_an_override_target_lands_in_a_new_plan() {
    let scratch = scratch("override-switch");
    library_package(&scratch.join("team/greeter"), "greeter");
    library_package(&scratch.join("pinned/greeter"), "greeter");
    library_package(&scratch.join("pinned2/greeter"), "greeter");
    let app = scratch.join("app");
    application(
        &app,
        "[sources]\nteam = { path = \"../team\" }\n\n[overrides]\nteam/greeter = { path = \"../pinned/greeter\" }\n",
    );
    let first = invoke(&app, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let before = plan_ids(&app);
    assert_eq!(before.len(), 1, "{before:?}");

    application(
        &app,
        "[sources]\nteam = { path = \"../team\" }\n\n[overrides]\nteam/greeter = { path = \"../pinned2/greeter\" }\n",
    );
    let second = invoke(&app, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let after = plan_ids(&app);
    assert_eq!(after.len(), 2, "{after:?}");
}

#[test]
fn a_stamp_recorded_for_another_action_is_not_reused() {
    let (_scratch, project, cold) = built("foreign-stamp");
    let plan = build_directory(&cold);
    let stamps = plan.join("stamps");
    let first = std::fs::read_to_string(stamps.join("0000.json")).unwrap();
    // Take the first action's stamp and name it as the second action's: the recorded bytes are
    // real, but they are not the bytes the second action wrote. It must not be reused.
    let mut document: serde_json::Value = serde_json::from_str(&first).unwrap();
    document["index"] = serde_json::json!(1);
    std::fs::write(
        stamps.join("0001.json"),
        format!("{}\n", serde_json::to_string(&document).unwrap()),
    )
    .unwrap();
    let second = invoke(&project, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let warmed = evidence(&project);
    assert!(!all_reused(&warmed), "{warmed}");
}
