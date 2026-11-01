//! The backends this checkout registers, and the handshake each one declares.
//! 本检出注册的后端，以及每个后端声明的握手。
//!
//! A language is only implemented when a backend is registered for it and says, ability by
//! ability, what it does. The list below is the truth a refusal quotes, so it must not name a
//! language whose chain has not been run.
//! 一门语言只有在有后端注册、并逐项说出它能做什么时才算实现。下面的清单是拒绝语所引用的真相，
//! 因此它绝不能点名一条尚未跑通的链路。

use std::path::Path;

use xirang_kernel::interchange::{Capability, Handshake, LanguageId};

/// One registered backend.
/// 一个已注册的后端。
pub(super) struct Registered {
    /// The language identity the backend answers for.
    /// 该后端负责的语言身份。
    pub language: &'static str,
    /// The programs that carry the backend out.
    /// 执行该后端的程序。
    pub backend: &'static str,
    /// What the backend declares it can do.
    /// 后端声明它能做的事。
    pub capabilities: &'static [Capability],
}

/// The languages this checkout registers, in the order a report lists them.
/// 本检出注册的语言，按报告列出的顺序。
const REGISTERED: &[Registered] = &[
    Registered {
        language: "cpp",
        backend: "g++ / clang++",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Export,
        ],
    },
    Registered {
        language: "c",
        backend: "gcc / clang",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "python",
        backend: "python3",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "javascript",
        backend: "node",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "typescript",
        backend: "node",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "go",
        backend: "go",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "ruby",
        backend: "ruby",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "java",
        backend: "javac / java",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "php",
        backend: "php",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "kotlin",
        backend: "kotlinc / java",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "swift",
        backend: "swiftc",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "csharp",
        backend: "dotnet",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
    Registered {
        language: "rust",
        backend: "rustc",
        capabilities: &[
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
            Capability::Consumer,
        ],
    },
];

/// The languages the task names but this checkout has no backend for.
/// 任务点名、但本检出没有后端的语言。
const NAMED: &[&str] = &[];

/// The handshake for a language, or the refusal naming what is registered.
/// 某语言的握手；没有时返回点名已注册内容的拒绝。
pub(super) fn handshake(language: &str) -> Result<Handshake, String> {
    let Some(entry) = REGISTERED.iter().find(|entry| entry.language == language) else {
        return Err(format!(
            "no backend is registered for language `{language}`; this checkout registers {}. A \
             language is not implemented until a backend declares what it does",
            registered_list()
        ));
    };
    let identity = LanguageId::parse_language(entry.language)
        .expect("a registered language identity is valid by construction");
    Ok(Handshake::new(
        identity,
        entry.backend,
        entry.capabilities.iter().copied(),
    ))
}

/// Ask one recorded backend whether it can own a language in this build: handshake first, then a
/// real plan request against `root`, and check the answer has the shape this build executes.
/// 问一个记录后端能否在本构建里拥有一门语言：先握手，再对 `root` 发一次真实计划请求，
/// 并检查应答的形状是不是本构建能执行的那种。
pub(super) fn check(root: &Path, language: &str) -> Result<String, String> {
    let probe = layout(root, language)?;
    let capabilities: Vec<&str> = probe
        .capabilities
        .iter()
        .map(|capability| capability.name())
        .collect();
    let mut report = format!(
        "handshake: ok — `{}` answers for `{language}` (capabilities: {}), \
         {} source extension(s) [{}], test files end with `{}`\n",
        probe.backend,
        capabilities.join(", "),
        probe.extensions.len(),
        probe.extensions.join(", "),
        probe.test_suffix
    );
    let plan = crate::production::plan_json(root)?;
    let document: serde_json::Value = serde_json::from_str(&plan)
        .map_err(|error| format!("the plan this build produced is not JSON: {error}"))?;
    let actions = document["actions"]
        .as_array()
        .ok_or("the plan this build produced has no `actions` array")?;
    if actions.is_empty() {
        return Err(format!(
            "the backend for `{language}` planned no actions for this project; a backend that \
             plans nothing owns nothing"
        ));
    }
    for action in actions {
        let kind = action["kind"].as_str().unwrap_or("<missing>");
        if !matches!(kind, "compile" | "archive" | "link" | "test") {
            return Err(format!(
                "the plan names a `{kind}` action; this build executes `compile`, `archive`, \
                 `link` and `test` actions"
            ));
        }
        let program = action["command"]
            .as_array()
            .and_then(|command| command.first())
            .and_then(|program| program.as_str())
            .unwrap_or("");
        if program.is_empty() {
            return Err(format!(
                "a `{kind}` action has no program: an empty argv is not a command this build can run"
            ));
        }
    }
    report.push_str(&format!(
        "plan: ok — {} action(s), plan `{}`\n",
        actions.len(),
        document["plan"].as_str().unwrap_or("?")
    ));
    report.push_str("the backend answers this build's protocol 2; it can own this language.\n");
    Ok(report)
}

/// The record that answers for an external language, if it is executable.
/// 回答某外部语言的记录（当它可执行时）。
pub(super) fn external(root: &Path, language: &str) -> Result<(), String> {
    let sources = super::record::discover(root, &registered_languages())?;
    super::record::conflicts(&sources)?;
    let named: Vec<_> = sources
        .iter()
        .flat_map(|(_, record)| record.entries.iter())
        .filter(|entry| entry.language == language)
        .collect();
    if named.iter().any(|entry| entry.command.is_some()) {
        return Ok(());
    }
    if let Some(entry) = named.first() {
        return Err(format!(
            "the record declares `{language}` through `{}` and names no `command`; a \
             declaration is not an executor",
            entry.backend
        ));
    }
    Err(format!(
        "no backend is registered for language `{language}`, and no record (the project's or the \
         user's) names an executable backend for it; this checkout registers {}. A language is \
         not implemented until a backend declares what it does",
        registered_list()
    ))
}

/// How an external language lays out its sources and tests: asked of its recorded backend, because
/// the project reader cannot guess another language's file names.
/// 外部语言如何放置源码与测试：询问记录里的后端，因为项目读取器猜不出另一门语言的文件名。
pub(super) fn layout(root: &Path, language: &str) -> Result<super::delegate::Probe, String> {
    let sources = super::record::discover(root, &registered_languages())?;
    super::record::conflicts(&sources)?;
    let entry = sources
        .iter()
        .flat_map(|(_, record)| record.entries.iter())
        .find(|entry| entry.language == language)
        .ok_or_else(|| {
            format!(
                "no record declares `{language}`; a project in that language has no backend to \
                 discover its sources"
            )
        })?;
    let argv = entry.command.clone().ok_or_else(|| {
        format!(
            "the record declares `{language}` through `{}` but names no `command`",
            entry.backend
        )
    })?;
    super::delegate::handshake(root, &argv, language, entry.planner_sandbox)
}

/// The registered languages, comma separated.
/// 已注册语言，以逗号分隔。
fn registered_list() -> String {
    REGISTERED
        .iter()
        .map(|entry| format!("`{}`", entry.language))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The language identities this build registers.
/// 本构建注册的语言身份。
pub(super) fn registered_languages() -> Vec<&'static str> {
    REGISTERED.iter().map(|entry| entry.language).collect()
}

/// The languages this checkout names but does not implement: the ones a recorded backend can
/// answer for.
/// 本检出点名但未实现的语言：记录型后端可以回答的那些。
pub(super) fn named_languages() -> Vec<&'static str> {
    NAMED.to_vec()
}

/// The `xirang languages` report: what is registered, what the checkout's record only declares, and
/// what is only named.
/// `xirang languages` 报告：注册了什么、检出记录里只是声明了什么，以及什么只是被点名。
pub(crate) fn report(root: &Path) -> Result<String, String> {
    let mut out = String::from("language  backend  capabilities\n");
    for entry in REGISTERED {
        out.push_str(&format!(
            "{}  {}  {}\n",
            entry.language,
            entry.backend,
            entry
                .capabilities
                .iter()
                .map(|capability| capability.name())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push_str(&format!(
        "\nnamed by the task, no backend registered here: {}\n",
        NAMED.join(", ")
    ));
    out.push_str(
        "A name in that list is not support: it says what the task asks for, not what a backend \
         declares. `xirang build` in such a project refuses by naming this report.\n",
    );
    let sources = super::record::discover(root, &registered_languages())?;
    super::record::conflicts(&sources)?;
    if sources.is_empty() {
        out.push_str(
            "\nthis checkout has no backend record: only the built-in backends above are \
             registered\n",
        );
        return Ok(out);
    }
    for (path, record) in &sources {
        out.push_str(&format!("\nrecorded in {}:\n", path.display()));
        for entry in &record.entries {
            let state = match (entry.command.is_some(), entry.sandbox) {
                (true, true) => "(executable, sandboxed)",
                (true, false) => "(executable)",
                (false, _) => "(declared)",
            };
            out.push_str(&format!(
                "{}  {}  {}  {state}\n",
                entry.language,
                entry.backend,
                entry
                    .capabilities
                    .iter()
                    .map(|capability| capability.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            for (capability, reason) in &entry.reasons {
                out.push_str(&format!("  no {capability}: {reason}\n"));
            }
            if entry.command.is_some() {
                // A record is the project's choice; the process is the fact. Ask it and print both, so
                // a record that overclaims is visible without trusting either side silently.
                // 记录是项目的选择，进程才是事实。问它并把两边都印出来，于是过度声明的记录会被看见。
                let argv = entry.command.clone().unwrap_or_default();
                match super::delegate::handshake(
                    root,
                    &argv,
                    &entry.language,
                    entry.planner_sandbox,
                ) {
                    Ok(report) => {
                        let reported = report
                            .capabilities
                            .iter()
                            .map(|capability| capability.name())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let claimed = entry
                            .capabilities
                            .iter()
                            .map(|capability| capability.name())
                            .collect::<Vec<_>>()
                            .join(", ");
                        if reported == claimed {
                            out.push_str(&format!(
                            "  the backend reports: {reported} (asked as a process; matches the \
                             record)\n"
                        ));
                        } else {
                            out.push_str(&format!(
                            "  the backend reports: {reported}; the record claims {claimed} — the \
                             backend's answer is the fact\n"
                        ));
                        }
                        if report.backend != entry.backend {
                            out.push_str(&format!(
                                "  the backend names itself `{}`; the record names `{}`\n",
                                report.backend, entry.backend
                            ));
                        }
                    }
                    Err(error) => {
                        return Err(format!(
                            "the recorded backend `{}` for `{}` did not answer its handshake: {error}",
                            entry.backend, entry.language
                        ));
                    }
                }
            }
        }
    }
    out.push_str(
        "`(executable)` means the entry names a `command`: `xirang build` and `xirang test` ask it \
         for a plan and run that plan here, and this report asks it for a handshake and prints \
         what it answered. `(declared)` means the entry names no command, and `xirang build` still \
         refuses a project whose language has no executor.\n",
    );
    Ok(out)
}
