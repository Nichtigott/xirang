//! The backend scaffold: a runnable recorded backend, the README that says what to replace, and
//! the exact install command that records it. A scaffold is how a language outside this checkout
//! stops being a blank page; it is deliberately a working example rather than a skeleton that
//! only answers a handshake.
//! 后端脚手架：一个可运行的记录型后端、说明该替换哪里的 README，以及记录它的确切安装命令。
//! 脚手架是本检出之外的语言不必从空白开始的方式；它有意做成可运行的例子，而不是只会答握手的骨架。

use std::path::Path;

/// One generated backend template, with the install command that records it.
/// 一份生成的后端模板，以及记录它的安装命令。
pub fn scaffold(language: &str, backend: &str, directory: &Path) -> Result<String, String> {
    if !identifier(language) {
        return Err(format!(
            "`{language}` is not usable as a language id; ids are ascii lowercase letters, digits, `-` and `_`"
        ));
    }
    if !identifier(backend) {
        return Err(format!(
            "`{backend}` is not usable as a backend name; names are ascii lowercase letters, digits, `-` and `_`"
        ));
    }
    if super::backends::registered_languages().contains(&language) {
        return Err(format!(
            "`{language}` already has a built-in backend in this checkout; a record cannot replace \
             it. The scaffold answers for a language this checkout does not implement, such as {}",
            super::backends::named_languages().join(", ")
        ));
    }
    if directory.exists() {
        let empty = std::fs::read_dir(directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
            .next()
            .is_none();
        if !empty {
            return Err(format!(
                "{} is not empty; the scaffold writes a new directory and will not overwrite files",
                directory.display()
            ));
        }
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let script = directory.join("backend.py");
    std::fs::write(&script, template(language, backend))
        .map_err(|error| format!("cannot write {}: {error}", script.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("cannot make {} executable: {error}", script.display()))?;
    }
    let readme = directory.join("README.md");
    std::fs::write(&readme, readme_text(language, backend))
        .map_err(|error| format!("cannot write {}: {error}", readme.display()))?;
    let absolute = script.canonicalize().unwrap_or(script.clone());
    Ok(format!(
        "wrote {}\nwrote {}\nnext: xirang backends install --language {language} --backend {backend} \
         --command python3 --command {} --capability discovery --capability build-test",
        script.display(),
        readme.display(),
        absolute.display()
    ))
}

/// Whether one spelling can name a language or a backend.
/// 一个拼法能否用来命名语言或后端。
fn identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 40
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

/// The runnable template. Its `plan()` compiles and tests shell scripts; everything else — the
/// handshake, the answer shape, the refusal when there is nothing to do — is what a backend of any
/// language keeps.
/// 可运行模板。它的 `plan()` 编译并测试 shell 脚本；其余部分——握手、应答形状、无事可做时的拒绝——
/// 是任何语言的后端都保留的。
fn template(language: &str, backend: &str) -> String {
    TEMPLATE
        .replace("__LANGUAGE__", language)
        .replace("__BACKEND__", backend)
}

/// The README that explains what a backend owns.
/// 说明一个后端拥有什么的 README。
fn readme_text(language: &str, backend: &str) -> String {
    format!(
        "# A recorded XiRang backend for `{language}`\n\n\
         `backend.py` speaks protocol 2: one JSON request on stdin, one JSON answer on stdout. It\n\
         is a working example for shell scripts — replace `plan()` with the compiler your language\n\
         owns; the handshake and the answer shape stay as they are.\n\n\
         Record it (the command the scaffold printed):\n\n\
         ```sh\n\
         xirang backends install --language {language} --backend {backend} \\\n    \
         --command python3 --command \"$PWD/backend.py\" \\\n    \
         --capability discovery --capability build-test\n\
         ```\n\n\
         The rules the executor enforces:\n\n\
         - one request, one answer; a plan answer's keys are `protocol`, `language`, `backend`,\n  \
         `capabilities`, `inputs` and `actions`, and `inputs` must echo the digest the request\n  \
         carried (the executor refuses an answer that names another input closure);\n\
         - the handshake also declares `source_extensions` (the extensions `src/` uses) and\n  \
         `test_suffix` (the word `tests/` files end with before their extension), so a project\n  \
         in this language is discovered like any other;\n\
         - every action is `kind` (`compile`, `archive`, `link` or `test`) plus `program` and\n  \
         `args`; an action may also carry `name`, `outputs`, `env` and `cwd`, and a `test`\n  \
         action runs the program it names, so it is the test itself;\n\
         - `[language.<id>]` keys other than the built-in languages reach the backend as\n  \
         `options`; `xirang add` refuses to edit an external-language project, because its\n  \
         backend owns the dependency graph;\n\
         - a `test` answer needs at least one `test` action;\n
         - the backend runs with the project root as its working directory and keeps the rights\n  \
         of the user who started it — this build provides no sandbox for recorded backends.\n"
    )
}

/// The template itself. Placeholders are substituted so the generated file has no `format!` braces
/// to escape.
/// 模板本体。占位符被替换，因此生成的文件里没有需要转义的 `format!` 花括号。
const TEMPLATE: &str = r#"#!/usr/bin/env python3
"""A recorded XiRang backend, generated by `xirang backends new`.

It speaks protocol 2: one JSON request on stdin, one JSON answer on stdout. Replace `plan()` with
the compiler this language owns; the handshake and the answer shape are all the executor requires.
"""

import json
import sys
from pathlib import Path

PROTOCOL = 2
LANGUAGE = "__LANGUAGE__"
BACKEND = "__BACKEND__"


def answer(payload):
    json.dump(payload, sys.stdout)
    sys.stdout.write("\n")


def plan(action):
    actions = []
    if action == "build":
        for source in sorted(Path("src").glob("*.sh")):
            actions.append({"kind": "compile", "program": "sh", "args": ["-n", str(source)]})
    else:
        for test in sorted(Path("tests").glob("*_test.sh")):
            actions.append(
                {
                    "kind": "test",
                    "program": "sh",
                    "args": [str(test)],
                    "name": test.stem,
                }
            )
    return actions


def main():
    request = json.load(sys.stdin)
    if request.get("request") == "handshake":
        answer(
            {
                "protocol": PROTOCOL,
                "language": LANGUAGE,
                "backend": BACKEND,
                "capabilities": ["discovery", "resolution", "build-test"],
                "source_extensions": ["sh"],
                "test_suffix": "_test",
            }
        )
        return 0
    action = request.get("action")
    actions = plan(action)
    if not actions:
        print(f"nothing to do for `{action}`: no matching files under src/ or tests/", file=sys.stderr)
        return 2
    answer(
        {
            "protocol": PROTOCOL,
            "language": request.get("language", LANGUAGE),
            "backend": BACKEND,
            "capabilities": ["build-test"],
            "inputs": request.get("inputs"),
            "actions": actions,
        }
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
"#;
