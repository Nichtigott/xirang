#![cfg(feature = "cli")]
//! End-to-end: `xirang backends install` records one backend in the user-level record, refuses an
//! installation that cannot answer a handshake, and `xirang languages` merges the project record
//! with the user's and refuses two answers for one language.
//! 端到端：`xirang backends install` 把一个后端记录进用户级记录，拒绝答不了握手的安装，而
//! `xirang languages` 合并项目记录与用户记录，并拒绝同一门语言的两种回答。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory.
/// 一个全新的临时目录。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-backends-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Run `xirang` with an isolated `HOME` (and no `XIRANG_BACKENDS`/`XDG_CONFIG_HOME` leaking in),
/// so an installation lands in this scratch tree and nowhere else.
/// 用隔离的 `HOME` 运行 `xirang`（并清掉会漏进来的 `XIRANG_BACKENDS`/`XDG_CONFIG_HOME`），
/// 使安装只落在这个临时树里。
fn invoke(directory: &Path, home: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .env("HOME", home)
        .env_remove("XIRANG_BACKENDS")
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap()
}

fn both(output: &Output) -> String {
    format!(
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Write the script a backend record points at: it answers a handshake, and for a plan request it
/// answers with an empty plan.
/// 写记录指向的脚本：它回答握手，对计划请求则回答一份空计划。
fn backend_script(root: &Path) -> PathBuf {
    let path = root.join("backend.py");
    std::fs::write(
        &path,
        r#"import json, sys
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test",
                      "language": request["language"], "backend": "fixture-perl",
                      "capabilities": ["discovery", "build-test"],
                      "reasons": {"export": "no export support here"}}))
    sys.exit(0)
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["discovery", "build-test"], "plan": {"id": "fixture",
                  "actions": []}}))
"#,
    )
    .unwrap();
    path
}

/// A project directory with an `xirang.toml` for the language `perl` (no built-in backend).
/// 一个语言为 `perl`（没有内置后端）的项目目录。
fn project(root: &Path) {
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.perl]\nstandard = \"5\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.pl"), "1;\n").unwrap();
}

#[test]
fn an_install_is_written_to_the_user_record_after_the_process_answers() {
    let root = scratch("install");
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    project(&root);
    let script = backend_script(&root);
    let installed = invoke(
        &root,
        &home,
        &[
            "backends",
            "install",
            "--language",
            "perl",
            "--backend",
            "fixture-perl",
            "--command",
            "python3",
            "--command",
            &script.display().to_string(),
            "--capability",
            "discovery",
            "--capability",
            "build-test",
            "--reason",
            "export=no export support here",
        ],
    );
    assert!(installed.status.success(), "{}", both(&installed));
    let record = home.join(".config/xirang/xirang.backends.toml");
    let text = std::fs::read_to_string(&record).expect("the installation writes the user record");
    assert!(text.contains("language = \"perl\""), "{text}");
    assert!(text.contains("fixture-perl"), "{text}");
    // The record reads back: the report merges it with the built-in table and asks it again.
    let reported = invoke(&root, &home, &["languages"]);
    assert!(reported.status.success(), "{}", both(&reported));
    let report = String::from_utf8_lossy(&reported.stdout).into_owned();
    assert!(report.contains("recorded in"), "{report}");
    assert!(report.contains("perl  fixture-perl"), "{report}");
    assert!(
        report.contains(
            "the backend reports: discovery, build-test (asked as a process; matches the record)"
        ),
        "{report}"
    );
}

#[test]
fn an_install_whose_command_cannot_answer_writes_nothing() {
    let root = scratch("no-answer");
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    project(&root);
    let failed = invoke(
        &root,
        &home,
        &[
            "backends",
            "install",
            "--language",
            "perl",
            "--backend",
            "fixture-perl",
            "--command",
            "python3",
            "--command",
            "-c",
            "--command",
            "raise SystemExit(1)",
            "--capability",
            "discovery",
            "--capability",
            "build-test",
        ],
    );
    assert!(!failed.status.success(), "{}", both(&failed));
    assert!(
        !home.join(".config/xirang/xirang.backends.toml").exists(),
        "a failed probe must write nothing"
    );
}

#[test]
fn an_install_without_a_process_or_without_build_test_is_refused_by_name() {
    let root = scratch("rules");
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    project(&root);
    let script = backend_script(&root);
    let no_command = invoke(
        &root,
        &home,
        &[
            "backends",
            "install",
            "--language",
            "perl",
            "--backend",
            "fixture-perl",
            "--capability",
            "discovery",
        ],
    );
    assert!(!no_command.status.success(), "{}", both(&no_command));
    let text = both(&no_command);
    assert!(text.contains("needs a `command`"), "{text}");
    let no_build_test = invoke(
        &root,
        &home,
        &[
            "backends",
            "install",
            "--language",
            "perl",
            "--backend",
            "fixture-perl",
            "--command",
            "python3",
            "--command",
            &script.display().to_string(),
            "--capability",
            "discovery",
        ],
    );
    assert!(!no_build_test.status.success(), "{}", both(&no_build_test));
    let text = both(&no_build_test);
    assert!(text.contains("does not claim `build-test`"), "{text}");
    assert!(!home.join(".config/xirang/xirang.backends.toml").exists());
}

#[test]
fn two_records_answering_one_language_are_refused_by_name() {
    let root = scratch("conflict");
    let home = root.join("home");
    std::fs::create_dir_all(home.join(".config/xirang")).unwrap();
    project(&root);
    let script = backend_script(&root);
    std::fs::write(
        root.join("xirang.backends.toml"),
        "protocol = 1\n\n[[backend]]\nlanguage = \"perl\"\nbackend = \"project-backend\"\n\
         capabilities = [\"discovery\"]\n",
    )
    .unwrap();
    std::fs::write(
        home.join(".config/xirang/xirang.backends.toml"),
        format!(
            "protocol = 1\n\n[[backend]]\nlanguage = \"perl\"\nbackend = \"user-backend\"\n\
             capabilities = [\"discovery\", \"build-test\"]\ncommand = [\"python3\", \"{}\"]\n",
            script.display()
        ),
    )
    .unwrap();
    let reported = invoke(&root, &home, &["languages"]);
    assert!(!reported.status.success(), "{}", both(&reported));
    let text = both(&reported);
    assert!(text.contains("answered by two recorded backends"), "{text}");
    assert!(text.contains("xirang.backends.toml"), "{text}");
    assert!(
        text.contains("config/xirang/xirang.backends.toml"),
        "{text}"
    );
}

#[test]
fn installing_a_builtin_language_is_refused() {
    let root = scratch("builtin");
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    project(&root);
    let script = backend_script(&root);
    let refused = invoke(
        &root,
        &home,
        &[
            "backends",
            "install",
            "--language",
            "cpp",
            "--backend",
            "fixture-perl",
            "--command",
            "python3",
            "--command",
            &script.display().to_string(),
            "--capability",
            "discovery",
            "--capability",
            "build-test",
        ],
    );
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(text.contains("already has a built-in backend"), "{text}");
    assert!(!home.join(".config/xirang/xirang.backends.toml").exists());
}

/// A shell project the scaffold's template can build and test.
/// 脚手架模板能构建与测试的 shell 项目。
fn shell_project(root: &Path) {
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"hello\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.shell]\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(root.join("src/hello.sh"), "#!/bin/sh\necho hello\n").unwrap();
    std::fs::write(
        root.join("tests/hello_test.sh"),
        "#!/bin/sh\nset -e\necho \"test hello_test ... ok\"\n",
    )
    .unwrap();
}

#[test]
fn a_scaffolded_backend_is_recorded_and_runs_a_plan() {
    let root = scratch("scaffold");
    let home = scratch("scaffold-home");
    let made = invoke(
        &root,
        &home,
        &[
            "backends",
            "new",
            "--language",
            "shell",
            "--backend",
            "template-shell",
            "--directory",
            "backend",
        ],
    );
    assert!(made.status.success(), "{}", both(&made));
    let script = root.join("backend/backend.py");
    let text = std::fs::read_to_string(&script).unwrap();
    assert!(text.contains("LANGUAGE = \"shell\""), "{text}");
    assert!(text.contains("\"build-test\""), "{text}");
    let installed = invoke(
        &root,
        &home,
        &[
            "backends",
            "install",
            "--language",
            "shell",
            "--backend",
            "template-shell",
            "--command",
            "python3",
            "--command",
            script.to_str().unwrap(),
            "--capability",
            "discovery",
            "--capability",
            "build-test",
        ],
    );
    assert!(installed.status.success(), "{}", both(&installed));
    shell_project(&root);
    let built = invoke(&root, &home, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&root, &home, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let output = String::from_utf8_lossy(&tested.stdout);
    assert!(output.contains("hello_test"), "{output}");
    // Red control: a failing test must fail, so the test action really ran.
    std::fs::write(root.join("tests/hello_test.sh"), "#!/bin/sh\nexit 1\n").unwrap();
    let failed = invoke(&root, &home, &["test"]);
    assert!(
        !failed.status.success(),
        "a failing test was recorded as a pass\n{}",
        both(&failed)
    );
}

#[test]
fn the_scaffold_refuses_a_language_this_checkout_implements() {
    let root = scratch("scaffold-builtin");
    let home = scratch("scaffold-builtin-home");
    let made = invoke(
        &root,
        &home,
        &[
            "backends",
            "new",
            "--language",
            "cpp",
            "--backend",
            "template-cpp",
            "--directory",
            "backend",
        ],
    );
    assert!(!made.status.success());
    assert!(
        String::from_utf8_lossy(&made.stderr).contains("built-in"),
        "{}",
        both(&made)
    );
    assert!(
        !root.join("backend").exists(),
        "the scaffold wrote a directory anyway"
    );
}

#[test]
fn the_scaffold_refuses_a_directory_that_already_holds_files() {
    let root = scratch("scaffold-nonempty");
    let home = scratch("scaffold-nonempty-home");
    std::fs::create_dir_all(root.join("backend")).unwrap();
    std::fs::write(root.join("backend/keep.txt"), "keep").unwrap();
    let made = invoke(
        &root,
        &home,
        &[
            "backends",
            "new",
            "--language",
            "shell",
            "--backend",
            "template-shell",
            "--directory",
            "backend",
        ],
    );
    assert!(!made.status.success());
    assert!(
        String::from_utf8_lossy(&made.stderr).contains("not empty"),
        "{}",
        both(&made)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("backend/keep.txt")).unwrap(),
        "keep"
    );
}

/// A script that answers a handshake under `name`, so two installations are distinguishable.
/// 一个以 `name` 回答握手的脚本，使两次安装可以区分。
fn named_script(root: &Path, file: &str, name: &str) -> PathBuf {
    let path = root.join(file);
    std::fs::write(
        &path,
        format!(
            r#"import json, sys
request = json.load(sys.stdin)
print(json.dumps({{"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": request["language"], "backend": "{name}",
                  "capabilities": ["discovery", "build-test"]}}))
"#
        ),
    )
    .unwrap();
    path
}

#[test]
fn installing_again_replaces_the_entry_and_removing_drops_it() {
    let root = scratch("upgrade");
    let home = scratch("upgrade-home");
    let first = named_script(&root, "first.py", "first-backend");
    let second = named_script(&root, "second.py", "second-backend");
    let install = |script: &Path| {
        invoke(
            &root,
            &home,
            &[
                "backends",
                "install",
                "--language",
                "shell",
                "--backend",
                "template-shell",
                "--command",
                "python3",
                "--command",
                script.to_str().unwrap(),
                "--capability",
                "discovery",
                "--capability",
                "build-test",
            ],
        )
    };
    let installed = install(&first);
    assert!(installed.status.success(), "{}", both(&installed));
    let replaced = install(&second);
    assert!(replaced.status.success(), "{}", both(&replaced));
    let output = String::from_utf8_lossy(&replaced.stdout);
    assert!(output.contains("replaces `template-shell`"), "{output}");
    let record = home.join(".config/xirang/xirang.backends.toml");
    let text = std::fs::read_to_string(&record).unwrap();
    assert_eq!(text.matches("[[backend]]").count(), 1, "{text}");
    assert!(text.contains("second.py"), "{text}");
    assert!(!text.contains("first.py"), "{text}");
    let reported = invoke(&root, &home, &["languages"]);
    assert!(reported.status.success(), "{}", both(&reported));
    let listing = String::from_utf8_lossy(&reported.stdout);
    assert!(listing.contains("second-backend"), "{listing}");
    assert!(!listing.contains("first-backend"), "{listing}");
    let removed = invoke(&root, &home, &["backends", "remove", "--language", "shell"]);
    assert!(removed.status.success(), "{}", both(&removed));
    let output = String::from_utf8_lossy(&removed.stdout);
    assert!(
        output.contains("removed the `shell` entry (`template-shell`)"),
        "{output}"
    );
    let after = std::fs::read_to_string(&record).unwrap();
    assert!(!after.contains("[[backend]]"), "{after}");
    let again = invoke(&root, &home, &["backends", "remove", "--language", "shell"]);
    assert!(!again.status.success());
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("records no `shell`"),
        "{}",
        both(&again)
    );
    assert_eq!(std::fs::read_to_string(&record).unwrap(), after);
}
