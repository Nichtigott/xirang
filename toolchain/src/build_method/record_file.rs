//! Build-time record files: a language that produces no linkable object writes its face records to
//! a file, and the build reads that file as if it had just compiled the sources.
//! 构建期记录文件：不产出可链接目标文件的语言把面记录写进一个文件，构建像刚编译过那些源码一样读取它。
//!
//! The section carrier needs a compiler that can place a custom section (C, C++, Zig). A JVM
//! annotation processor, a `go run` generator or a Roslyn source generator cannot, so it writes the
//! same 1,228-byte version-3 records (layout version 2 plus the source spelling) into a file named by
//! `xirang.toml`'s `[record] file`. One decoder reads both carriers; the only new field is the source
//! the file carrier has to carry, because its writer runs long before the build reads it.
//! 段载体需要能把自定义段放进目标文件的编译器（C、C++、Zig）。JVM 注解处理器、`go run` 生成器或 Roslyn 源
//! 生成器做不到，因此它把同样的 1,228 字节版本 3 记录（版本 2 布局加源码拼写）写进 `xirang.toml` 的
//! `[record] file` 点名的文件。一个解码器读两种载体；唯一的新字段是文件载体必须携带的源码，因为它的写入方
//! 在构建读取之前很久就运行了。
//!
//! A declared file that is missing is refused by name: "the producer has not run" and "this package
//! declares no faces" are different answers, and only one of them means the tree is complete.
//! 声明了却缺失的文件按名拒绝："生产者还没运行"与"这个包没有声明面"是两个答案，而只有后者意味着树是完整的。
//!
//! `[record] command` is how the build runs the producer itself: the command runs with the package
//! root as its directory and receives the destination, the package's root identity and the language
//! SDK locations in its environment. The build still reads the file, so a command that exits zero and
//! writes nothing is a refusal, not an empty tree.
//! `[record] command` 是构建自己运行生产者的方式：命令以包根为工作目录运行，并从环境里收到目标文件、本包的
//! 根身份与语言 SDK 位置。构建仍然读那个文件，因此"退出码 0 但什么都没写"是拒绝，而不是一棵空树。

use std::path::Path;

use xirang_kernel::identity::root_node_id;

use crate::build_method::external_face::{ExternalFaceRecord, records_from_file};

/// What one package's `[record]` table declares.
/// 一个包的 `[record]` 表声明的东西。
pub(crate) struct Declaration {
    /// The package-relative path the records are read from.
    /// 读取记录所用的包内相对路径。
    pub(crate) file: String,
    /// The producer command, when the build is the one that runs it.
    /// 生产者命令（当由构建运行它时）。
    pub(crate) command: Option<Vec<String>>,
    /// The language whose constants file the build writes, when one is declared.
    /// 构建要写常量文件的语言（声明了时）。
    pub(crate) language: Option<String>,
    /// The package-relative constants file the build writes before the producer runs.
    /// 构建在生产运行之前写出的包内相对常量文件。
    pub(crate) constants: Option<String>,
}

/// Read the `[record]` table without running anything.
/// 只读 `[record]` 表，不运行任何东西。
pub(crate) fn declaration(package_root: &Path) -> Result<Option<Declaration>, String> {
    parse_table(package_root)
}

/// The declaration and the constants request, read without running anything.
/// 无副作用地读出的声明与常量请求。
///
/// The pair travels together because they answer one question: does the producer need a pass before
/// it runs? A constants file is written by the build, so it does; a plain record file does not.
/// 两者一起传递，因为它们回答同一个问题：生产者运行前是否需要先有一遍？常量文件由构建写出，因此需要；
/// 普通记录文件不需要。
pub(crate) fn context(
    package_root: &Path,
) -> Result<Option<(Declaration, Option<super::record_constants::Request>)>, String> {
    let Some(declaration) = declaration(package_root)? else {
        return Ok(None);
    };
    let constants = match (&declaration.language, &declaration.constants) {
        (Some(language), Some(path)) => Some(super::record_constants::Request {
            language: language.clone(),
            path: std::path::PathBuf::from(path),
        }),
        _ => None,
    };
    Ok(Some((declaration, constants)))
}

/// Run the producer and read the records, or return none when the package declares no record file.
/// 运行生产者并读取记录；包没有声明记录文件时返回空。
pub(crate) fn records_after_producing(
    package_root: &Path,
    declaration: Option<&Declaration>,
    namespace: &str,
) -> Result<Vec<ExternalFaceRecord>, String> {
    let Some(declaration) = declaration else {
        return Ok(Vec::new());
    };
    produce(package_root, declaration, namespace)?;
    read(package_root, declaration, namespace)
}

/// Run the declared producer, so the record file exists for the next read.
/// 运行声明的生产者，让下一次读取时记录文件已经存在。
pub(crate) fn produce(
    package_root: &Path,
    declaration: &Declaration,
    namespace: &str,
) -> Result<(), String> {
    let file = package_root.join(&declaration.file);
    let Some(command) = &declaration.command else {
        return Ok(());
    };
    run_producer(package_root, command, &file, namespace)?;
    if !file.is_file() {
        return Err(format!(
            "`xirang.toml` declares the producer `{command:?}` and the record file `{}`, but the \
             producer exited successfully and wrote nothing; a producer that writes no records is a \
             mistake, not a tree with no faces",
            file.display()
        ));
    }
    Ok(())
}

/// Read the records after the producer has run (or refused because the file is missing).
/// 在生产运行之后读取记录（文件缺失时按名拒绝）。
pub(crate) fn read(
    package_root: &Path,
    declaration: &Declaration,
    namespace: &str,
) -> Result<Vec<ExternalFaceRecord>, String> {
    let file = package_root.join(&declaration.file);
    if !file.is_file() {
        return Err(format!(
            "`xirang.toml` declares the build-time record file `{}`, but {} does not exist; a \
             declared file that is missing means the producer has not run, not that this package \
             declares no faces",
            declaration.file,
            file.display()
        ));
    }
    records_from_file(&file, namespace).map_err(|why| format!("{}: {why}", file.display()))
}

/// The whole declaration→producer→read cycle, for callers that have no pass ordering of their own.
/// 声明→生产→读取的整条链路，供没有自己的 pass 顺序的调用方使用。
#[cfg(all(test, feature = "native-record"))]
pub(crate) fn records(
    package_root: &Path,
    namespace: &str,
) -> Result<Vec<ExternalFaceRecord>, String> {
    let Some(declared) = declaration(package_root)? else {
        return Ok(Vec::new());
    };
    produce(package_root, &declared, namespace)?;
    read(package_root, &declared, namespace)
}

/// Run the declared producer with the context its language toolchain needs.
/// 用语言工具链需要的上下文运行声明的生产者。
///
/// The environment carries what the producer cannot compute itself without duplicating a rule:
/// the destination it must write, the package root, the package's **root identity** (the same
/// `root_node_id(namespace)` the generated C header carries), and the Java SDK sources. A producer
/// that needs more can read its own language's SDK; the build does not guess.
/// 环境携带生产者无法在不重复一条规则的前提下自己算出的东西：它必须写的目标、包根、包的**根身份**（与生成的
/// C 头携带的同一个 `root_node_id(namespace)`），以及 Java SDK 源码。需要更多的生产者可以读自己语言的 SDK；
/// 构建不去猜。
fn run_producer(
    package_root: &Path,
    command: &[String],
    file: &Path,
    namespace: &str,
) -> Result<(), String> {
    let (program, arguments) = command
        .split_first()
        .expect("the parser refuses an empty command");
    let output = std::process::Command::new(program)
        .args(arguments)
        .current_dir(package_root)
        .env("XIRANG_RECORD_FILE", file)
        .env("XIRANG_PACKAGE_ROOT", package_root)
        .env("XIRANG_ROOT_NODE_ID", root_node_id(namespace).to_string())
        .env("XIRANG_JAVA_SDK", xirang_multilang::sdk::java_include_dir())
        .env("XIRANG_GO_SDK", xirang_multilang::sdk::go_include_dir())
        .env(
            "XIRANG_CSHARP_SDK",
            xirang_multilang::sdk::csharp_include_dir(),
        )
        .output()
        .map_err(|error| {
            format!(
                "cannot run the `[record] command` `{program}` ({}): {error}",
                package_root.display()
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "the `[record] command` `{program}` failed with {}; stdout: {}; stderr: {}",
        output.status,
        excerpt(&output.stdout),
        excerpt(&output.stderr)
    ))
}

/// A bounded, single-spaced excerpt of a producer's output.
/// 生产者输出的有界、单行摘要。
fn excerpt(bytes: &[u8]) -> String {
    const LIMIT: usize = 1_200;
    let text = String::from_utf8_lossy(bytes);
    let mut trimmed = text.trim().to_owned();
    if trimmed.len() > LIMIT {
        trimmed.truncate(LIMIT);
        trimmed.push_str("… (truncated)");
    }
    if trimmed.is_empty() {
        "(no output)".to_owned()
    } else {
        trimmed.replace(['\n', '\r'], " | ")
    }
}

/// The `[record]` table a package declares.
/// 一个包声明的 `[record]` 表。
fn parse_table(package_root: &Path) -> Result<Option<Declaration>, String> {
    let manifest = package_root.join("xirang.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return Ok(None);
    };
    let Some(section) = text.split("[record]").nth(1) else {
        return Ok(None);
    };
    let mut file: Option<String> = None;
    let mut command: Option<Vec<String>> = None;
    let mut language: Option<String> = None;
    let mut constants: Option<String> = None;
    for line in section.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            break;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "`[record]` has the line `{line}`, which is not a `key = value` entry; the table \
                 takes `file = \"<path>\"` and optionally `command = [\"<program>\", …]`"
            ));
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "file" => {
                if file.is_some() {
                    return Err(
                        "`[record]` names `file` twice; a package has one record file per shape"
                            .to_owned(),
                    );
                }
                let path = value.trim_matches('"');
                if path.is_empty() {
                    return Err(
                        "`[record] file` is empty; name the file the producer writes".to_owned(),
                    );
                }
                file = Some(path.to_owned());
            }
            "command" => {
                if command.is_some() {
                    return Err("`[record]` names `command` twice".to_owned());
                }
                command = Some(parse_command(value)?);
            }
            "language" => {
                if language.is_some() {
                    return Err("`[record]` names `language` twice".to_owned());
                }
                let name = value.trim_matches('"');
                if name.is_empty() {
                    return Err("`[record] language` is empty; name the language whose constants                                 file the build writes"
                        .to_owned());
                }
                language = Some(name.to_owned());
            }
            "constants" => {
                if constants.is_some() {
                    return Err("`[record]` names `constants` twice".to_owned());
                }
                let path = value.trim_matches('"');
                if path.is_empty() {
                    return Err(
                        "`[record] constants` is empty; name the file the build writes".to_owned(),
                    );
                }
                constants = Some(path.to_owned());
            }
            other => {
                return Err(format!(
                    "`[record]` has an unknown key `{other}`; the table takes `file`, `command`, \
                     and optionally `language` + `constants`, because every field is read from the \
                     records themselves"
                ));
            }
        }
    }
    let Some(file) = file else {
        return Err(
            "`[record]` names no `file`; write `[record]` followed by `file = \"<path>\"`, or drop \
             the table"
                .to_owned(),
        );
    };
    match (&language, &constants) {
        (Some(_), Some(_)) => {}
        (None, None) => {}
        (Some(_), None) => {
            return Err(
                "`[record] language` asks for a constants file but no `constants` path was named;                  write both, or neither"
                    .to_owned(),
            );
        }
        (None, Some(_)) => {
            return Err(
                "`[record] constants` names a file but no `language` says what syntax to write it                  in; write both, or neither"
                    .to_owned(),
            );
        }
    }
    Ok(Some(Declaration {
        file,
        command,
        language,
        constants,
    }))
}

/// One TOML array of quoted strings, on one line.
/// 一行里的 TOML 带引号字符串数组。
///
/// The workspace has no TOML parser, and `[record]` is deliberately too small to need one: it is a
/// destination and an argv, and everything else comes from the records. A malformed array is
/// refused by name rather than guessed at.
/// 工作区没有 TOML 解析器，而 `[record]` 有意小到不需要一个：它是一个目的地加一个 argv，其余一切来自记录。
/// 畸形数组按名拒绝，而不是靠猜。
fn parse_command(value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| {
            format!("`[record] command` is `{value}`; it is one line like `[\"sh\", \"build.sh\"]`")
        })?;
    let mut arguments = Vec::new();
    let mut rest = inner;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let Some(after_quote) = rest.strip_prefix('"') else {
            return Err(format!(
                "`[record] command` has `{rest}` where a quoted argv element belongs"
            ));
        };
        let Some(end) = after_quote.find('"') else {
            return Err("`[record] command` has an unterminated quoted element".to_owned());
        };
        arguments.push(after_quote[..end].to_owned());
        rest = after_quote[end + 1..].trim_start();
        if rest.is_empty() {
            break;
        }
        let Some(after_comma) = rest.strip_prefix(',') else {
            return Err(format!(
                "`[record] command` has `{rest}` between argv elements; they are comma-separated"
            ));
        };
        rest = after_comma;
    }
    if arguments.is_empty() {
        return Err("`[record] command` is empty; name the program to run".to_owned());
    }
    Ok(arguments)
}

#[cfg(all(test, feature = "native-record"))]
#[path = "record_file_tests.rs"]
mod record_file_tests;
