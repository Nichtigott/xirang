//! Project scaffolding for new XiRang host crates.
//! 新 XiRang 宿主 crate 的项目脚手架。
//!
//! Renders the manifest, build script, and source entry of a new host, and
//! injects the editor snippets so the first field an author writes is one pick
//! away.
//! 渲染新宿主的清单、构建脚本与源码入口，并注入编辑器 snippet，使作者写下的
//! 第一个字段就能一键得到。

use std::fs;
use std::path::{Path, PathBuf};

use xirang_kernel::lexicon;

use super::snippets::{Editor, write_editor_snippets};

const XIRANG_REPOSITORY: &str = "https://github.com/Nichtigott/xirang";

/// What `xirang new` scaffolds: a binary host or a library host.
/// `xirang new` 生成的宿主类型：二进制宿主或库宿主。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectKind {
    /// A host whose entry is `src/main.rs`.
    /// 入口为 `src/main.rs` 的宿主。
    Binary,
    /// A host whose entry is `src/lib.rs`.
    /// 入口为 `src/lib.rs` 的宿主。
    Library,
}

impl ProjectKind {
    fn entry_file(self) -> &'static str {
        match self {
            ProjectKind::Binary => "src/main.rs",
            ProjectKind::Library => "src/lib.rs",
        }
    }
}

impl std::fmt::Display for ProjectKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            ProjectKind::Binary => "binary",
            ProjectKind::Library => "library",
        })
    }
}

/// Where a generated manifest sources the XiRang crates from.
/// 生成清单中 XiRang crate 的依赖来源。
#[derive(Clone, Debug)]
pub enum DependencySource {
    /// Path dependencies into a local XiRang checkout.
    /// 指向本地 XiRang checkout 的 path 依赖。
    Local {
        /// The checkout the generated manifest points at.
        /// 生成清单指向的检出目录。
        workspace: PathBuf,
    },
    /// The published release, by version requirement alone — what `cargo add xirang-toolchain`
    /// would write. It needs neither this checkout's paths nor a git remote, so a project generated
    /// this way builds offline from a warm registry cache; that is why it, and not `Git`, is the
    /// fallback for a tool that is not running inside a checkout.
    /// 只按版本要求指向**已发布**的那一版——`cargo add xirang-toolchain` 会写下的东西。它既不需要本
    /// 检出的路径、也不需要 git 远端，因此这样生成的项目能靠一份已预热的注册表缓存**离线**构建；这正是不在
    /// 检出内运行的工具回落到这里、而不是回落到 `Git` 的原因。
    Registry,
    /// Git dependencies; Cargo can resolve the same version from crates.io
    /// once a matching release is published.
    /// Git 依赖；发布匹配版本后 Cargo 可将同一版本解析到 crates.io。
    ///
    /// Never chosen for a caller: fetching a repository is a network round trip that a caller asking
    /// for a project did not ask for, so this variant is reachable only by naming it (`--git`, or the
    /// bridge's `dependency: "git"` with its URL).
    /// 从不替调用方选择：拉一个仓库是调用方没要求的网络往返，因此这一支只能**点名**到达（`--git`，或桥的
    /// `dependency: "git"` 加上它的 URL）。
    Git {
        /// The repository the generated manifest pulls from.
        /// 生成清单拉取的仓库地址。
        url: String,
    },
}

/// Detect the dependency source for a running tool binary: path dependencies when the binary sits
/// under some checkout's own `target/` directory; the published release otherwise.
/// 根据运行中的工具二进制位置检测依赖来源：位于某个检出自身 `target/` 目录下时使用 path 依赖，
/// 否则使用已发布的那一版。
///
/// The detection reads the **binary's** path, so a copied checkout answers for itself; the
/// compile-time manifest directory is only a fallback for a binary started from somewhere else
/// (see `checkout_above_target` for the case that made this matter).
/// 检测读的是**二进制自己的**路径，因此被复制的检出会为自己作答；编译期清单目录只留给"从别处启动"
/// 的二进制作为回落（那个让它变成必需的用例见 `checkout_above_target`）。
///
/// The fallback used to be `Git`, which made every installed copy write a manifest that needs the
/// network to resolve — audit `F8`. `Git` is still one of the spellings a caller may name; it is no
/// longer one this function guesses.
/// 过去这里回落到 `Git`，于是每一份装出来的副本都会写下一份需要联网才能解析的清单——审计 `F8`。`Git`
/// 仍是调用方可以点名的一种拼写，只是不再是这个函数会去猜的那一种。
pub fn detected_source(tool_manifest_dir: &Path, current_exe: &Path) -> DependencySource {
    // Which checkout is this binary part of? The answer is where the **binary** is, not where it was
    // compiled. A checkout that was copied or archived elsewhere reuses build artifacts, and those
    // artifacts embed the compile-time path of the directory they were built in — so a binary
    // running from `/tmp/copy/target/…` can carry `/original/toolchain` inside it. Measured: a
    // copied tree with a reused `target/` answered "not a checkout", generated a manifest against
    // the published release, and the scaffolded project then failed to compile because that release
    // predates the API the scaffold writes. The binary's own position is the fact that travels with
    // the copy.
    // 这个二进制属于哪一份检出？答案是**二进制**在哪，不是它在哪编译。被复制或归档到别处的检出会复用
    // 构建产物，而那些产物嵌着当初编译目录的路径——于是从 `/tmp/copy/target/…` 跑的二进制里可能写着
    // `/original/toolchain`。实测：一份复用 `target/` 的复制树回答"不是检出"，按已发布版本生成清单，
    // 随后脚手架出来的项目因为那一版早于脚手架写下的 API 而编译失败。二进制自己的位置才是跟着副本走的
    // 那个事实。
    if let Some(workspace) = checkout_above_target(current_exe) {
        return DependencySource::Local { workspace };
    }
    let workspace = tool_manifest_dir.parent().unwrap_or_else(|| Path::new("."));
    let runs_from_workspace = current_exe.starts_with(workspace.join("target"))
        && workspace.join("kernel").is_dir()
        && workspace.join("toolchain").is_dir();
    if runs_from_workspace {
        DependencySource::Local {
            workspace: workspace.to_path_buf(),
        }
    } else {
        DependencySource::Registry
    }
}

/// The checkout whose `target/` holds this executable, when that directory looks like a checkout.
/// 这个可执行文件所在的 `target/` 属于哪份检出——前提是那个目录看起来像一份检出。
fn checkout_above_target(current_exe: &Path) -> Option<PathBuf> {
    for ancestor in current_exe.ancestors() {
        if ancestor.file_name().is_none_or(|name| name != "target") {
            continue;
        }
        let Some(workspace) = ancestor.parent() else {
            continue;
        };
        if workspace.join("kernel").is_dir() && workspace.join("toolchain").is_dir() {
            return Some(workspace.to_path_buf());
        }
    }
    None
}

/// The checkout a request points at, or the detected one (audit `F8` follow-up).
/// 请求指向的检出目录，或检测出来的那一个（审计 `F8` 续）。
///
/// Detection answers "am I running from a checkout", and that question has no answer on a machine
/// where this tool was installed — which is exactly the machine whose generated project could not
/// build offline (`cargo build --offline` → `no matching package named xirang-toolchain`, because
/// the release is unpublished). Naming the checkout makes that machine able to generate a project
/// that **does** build locally, which is the point of the spelling.
/// 检测回答的是"我是不是从检出里跑的"，而这个问题在**装出来的**工具上没有答案——而那正是生成项目无法
/// 离线构建的那台机器（`cargo build --offline` → `no matching package named xirang-toolchain`，
/// 因为那一版还没发布）。**点名检出**让那台机器也能生成**本地确实能构建**的项目，而这正是这个拼写的目的。
///
/// A named directory must look like a checkout: `kernel/` and `toolchain/` beside each other. That
/// check is what keeps a typo from becoming a manifest that fails at its first `cargo build`, and the
/// refusal names both directories it looked for.
/// 点名的目录必须**像**一个检出：`kernel/` 与 `toolchain/` 相邻。这道检查让一个拼写错误不至于变成一份在
/// 第一条 `cargo build` 上失败的清单，而拒绝会点名它找过的两个目录。
fn named_checkout(
    checkout: Option<&str>,
    detected: DependencySource,
) -> Result<DependencySource, String> {
    let Some(named) = checkout.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(detected);
    };
    let workspace = Path::new(named);
    if workspace.join("kernel").is_dir() && workspace.join("toolchain").is_dir() {
        return Ok(DependencySource::Local {
            workspace: workspace.to_path_buf(),
        });
    }
    Err(format!(
        "`path` is `{named}`, which does not look like a XiRang checkout: `kernel/` and \
         `toolchain/` are not both inside it. Point it at a checkout (the directory holding both), \
         or use `dependency: \"registry\"` for the published release. Nothing was created"
    ))
}

/// The dependency source a **request** names, or the detected default when it names none.
/// 一次**请求**点名的依赖来源；一个都没点名时用检测出来的默认值。
///
/// It lives beside [`detected_source`] because the vocabulary is one rule: `path`, `registry` and
/// `git` are the three spellings a request may use, and `git` — the only one that needs the network
/// — is reachable only by naming it. A request that passes a git URL without saying `dependency:
/// "git"` is refused rather than obeyed, because the two ways to read it (a URL the caller wants, or
/// a value meant for another tool) are not distinguishable from the value alone.
/// 它与 [`detected_source`] 摆在一起，因为这套词表是一条规则：`path`、`registry`、`git` 是请求可以用的
/// 三种拼写，而 `git`——唯一需要联网的那一个——只能靠点名到达。给了 git URL 却没说 `dependency:
/// "git"` 的请求会被拒绝而不是照办：光看那个值，两种读法（调用方要的 URL，还是本意给别的工具的值）
/// 分不开。
pub fn requested_source(
    dependency: Option<&str>,
    git: Option<&str>,
    checkout: Option<&str>,
    tool_manifest_dir: &Path,
    current_exe: &Path,
) -> Result<DependencySource, String> {
    let detected = || detected_source(tool_manifest_dir, current_exe);
    let url = git.map(str::trim).filter(|value| !value.is_empty());
    let Some(kind) = dependency.map(str::trim).filter(|value| !value.is_empty()) else {
        // A named checkout is a source by itself: `path` is local, so reading it cannot reach the
        // network, and refusing it would only make the caller spell the same thing twice.
        // 点名检出本身就是一种来源：`path` 是本地拼写，读它到不了网络，拒绝它只会让调用方把同一件事写两遍。
        if checkout
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            return named_checkout(checkout, detected());
        }
        if url.is_some() {
            return Err(
                "`git` names a repository, so the request has to say `dependency: \"git\"` as well; \
                 it is not a spelling this tool picks for you. Nothing was created"
                    .to_owned(),
            );
        }
        return Ok(detected());
    };
    match kind {
        "path" => match named_checkout(checkout, detected())? {
            source @ DependencySource::Local { .. } => Ok(source),
            // Not a fallback to `registry`: the caller asked for this checkout's crates, and
            // quietly handing back a published release would be a different project.
            // 不回落到 `registry`：调用方要的是**本检出**的 crate，而悄悄换成已发布的那一版会是另一个项目。
            _ => Err(
                "`dependency: \"path\"` needs this tool to be running from a XiRang checkout \
                 (a binary under its own `target/`, with `kernel/` and `toolchain/` beside it); this \
                 one is not, so there is no checkout to point at. Use `dependency: \"registry\"` for \
                 the published release. Nothing was created"
                    .to_owned(),
            ),
        },
        "registry" => {
            if checkout.map(str::trim).is_some_and(|value| !value.is_empty()) {
                return Err(
                    "`dependency: \"registry\"` and a named `path` are two different sources: keep \
                     `registry` for the published release, or drop it and let `path` point at a \
                     checkout. Nothing was created"
                        .to_owned(),
                );
            }
            Ok(DependencySource::Registry)
        }
        "git" => Ok(DependencySource::Git {
            url: url.unwrap_or(XIRANG_REPOSITORY).to_owned(),
        }),
        other => Err(format!(
            "`dependency` is `path`, `registry` or `git`, not `{other}`: `path` points at the \
             checkout this tool runs from, `registry` at the published release, and `git` at a \
             repository (its URL comes from `git`, and defaults to this project's own). Nothing was \
             created"
        )),
    }
}

/// The XiRang release this tool belongs to, as the requirement a generated
/// manifest writes for the crates it depends on.
/// 本工具所属的 XiRang 发布版本，也就是生成清单为它依赖的 crate 写下的要求。
///
/// A literal here drifts: every scaffolded manifest used to require `0.1.0` forever,
/// which caret semantics happened to satisfy until the line reached `0.2.0` — at
/// which point a freshly generated host would stop resolving. The generating tool's
/// own version is the one value that cannot go stale, and `a_generated_project_can_
/// be_published_and_built_in_a_workspace` pins the requirement against it instead of
/// against a literal, so the next release moves it for free.
/// 这里的字面量会腐化：生成的清单过去永远要求 `0.1.0`，而 caret 语义恰好一直满足它，直到版本线走到
/// `0.2.0`——那时刚生成的宿主就会解析失败。生成工具自己的版本是唯一不会陈旧的取值，而
/// `a_generated_project_can_be_published_and_built_in_a_workspace` 把要求钉在它上面而不是钉在
/// 字面量上，因此下一次发布自动带上它。
const RELEASE_REQUIREMENT: &str = env!("CARGO_PKG_VERSION");

/// Whether this release could resolve from the **local** registry cache.
/// 这一版能否从**本地**注册表缓存解析出来。
///
/// This is the question that decides whether `cargo build --offline` works for a generated host whose
/// manifest names the published release, and it is answerable without a network: cargo resolves an
/// offline version requirement from the unpacked sources (`registry/src/<index>/<name>-<version>/`)
/// or the downloaded archive (`registry/cache/<index>/<name>-<version>.crate`). Asking the cache
/// rather than crates.io keeps this check deterministic, instant, and free of a probe that could hang
/// on an air-gapped machine — which is exactly the machine the warning exists for.
/// 这正是决定"清单指向已发布那一版的生成宿主能否 `cargo build --offline`"的那个问题，而它**不需要网络**
/// 就能回答：离线时 cargo 从解包源码（`registry/src/<index>/<name>-<version>/`）或已下载的存档
/// （`registry/cache/<index>/<name>-<version>.crate`）解析版本要求。问缓存而不是问 crates.io，让这项检查
/// 确定、瞬时，并且不会在气隙机器上挂住——而气隙机器正是这条警告存在的理由。
pub fn registry_release_present(version: &str, cargo_home: &Path) -> bool {
    // The **index cache** is the decisive one, and that is a measured fact rather than a reading of
    // cargo's docs: this machine holds `registry/cache/…/xirang-toolchain-0.2.0.crate` (the release
    // was published and then deleted) while `registry/index/…/.cache/ni/ch/xirang-toolchain` does
    // not exist, and `cargo build --offline` on a generated project fails with `no matching package
    // named xirang-toolchain found`. An archive without an index entry does not resolve.
    // **索引缓存**才是决定性的那一个，而这是实测事实、不是对 cargo 文档的解读：这台机器上有
    // `registry/cache/…/xirang-toolchain-0.2.0.crate`（那一版发布过又被删除），却没有
    // `registry/index/…/.cache/ni/ch/xirang-toolchain`，而生成的项目上 `cargo build --offline` 报
    // `no matching package named xirang-toolchain found`。**没有索引条目的存档解析不了。**
    let wanted = format!("\"vers\":\"{version}\"");
    let indexes = cargo_home.join("registry").join("index");
    let Ok(indexes) = std::fs::read_dir(&indexes) else {
        return false;
    };
    for index in indexes.flatten() {
        // Cargo's index cache path for a crate: the first two characters, then the next two.
        // cargo 为某个 crate 缓存的索引路径：前两个字符，再接后两个字符。
        let entry = index
            .path()
            .join(".cache")
            .join("ni")
            .join("ch")
            .join("xirang-toolchain");
        let Ok(text) = std::fs::read(&entry) else {
            continue;
        };
        if String::from_utf8_lossy(&text).contains(&wanted) {
            return true;
        }
    }
    false
}

/// The line a creation reply owes a caller whose manifest will not resolve offline (audit `F8`).
/// 生成回复欠调用方的那句话：当清单在离线时解析不了时（审计 `F8`）。
///
/// `Registry` is the right default for a tool that is not inside a checkout — but only for a release
/// that **is published**. The round that found this measured `cargo build --offline` failing with
/// `no matching package named xirang-toolchain` on a freshly generated project, i.e. the first
/// impression scenario failing at its first command, silently. The manifest cannot fix itself; the
/// reply can say what happened, name the check it ran, and give the two ways out.
/// 对不在检出内的工具，`Registry` 是对的默认值——但只对**已发布**的那一版成立。发现这条缺陷的那一轮实测：
/// 刚生成的项目上 `cargo build --offline` 报 `no matching package named xirang-toolchain`，也就是
/// **第一印象场景在第一条命令上悄悄失败**。清单自己无法补救；回复可以说明发生了什么、点名它做的检查，
/// 并给出两条出路。
pub fn offline_source_warning(
    source: &DependencySource,
    version: &str,
    cargo_home: &Path,
) -> Option<String> {
    let DependencySource::Registry = source else {
        return None;
    };
    if registry_release_present(version, cargo_home) {
        return None;
    }
    Some(format!(
        "offline     xirang-toolchain {version} is not in the local registry cache \
         ({}), so `cargo build --offline` here cannot resolve it yet — build once online, or \
         regenerate with `dependency: \"git\"` (`git: \"<url>\"` for another repository), or run \
         this tool from a XiRang checkout to get `path` dependencies instead",
        cargo_home.display()
    ))
}

/// The `(runtime, build)` dependency requirement strings for a generated
/// manifest, in the two lines `Cargo.toml` needs.
/// 生成清单所需的 `(runtime, build)` 两行依赖声明，即 `Cargo.toml` 要的两条。
pub fn dependency_specs(source: &DependencySource) -> (String, String) {
    match source {
        DependencySource::Local { workspace } => {
            // Both halves now name the same directory: batch 2 merged the nine
            // execution-surface crates into `toolchain/`, and the generated manifest
            // depends on that one crate from both tables.
            // 两半现在指向同一个目录：批 2 把九个执行面 crate 合并成了 `toolchain/`，而生成的
            // 清单在两个表里依赖的是同一个 crate。
            let runtime = toml_path(&workspace.join("toolchain"));
            let build = toml_path(&workspace.join("toolchain"));
            (
                format!(
                    "xirang-toolchain = {{ path = \"{runtime}\", version = \"{RELEASE_REQUIREMENT}\" }}"
                ),
                format!(
                    "xirang-toolchain = {{ path = \"{build}\", version = \"{RELEASE_REQUIREMENT}\" }}"
                ),
            )
        }
        DependencySource::Registry => (
            format!("xirang-toolchain = {{ version = \"{RELEASE_REQUIREMENT}\" }}"),
            format!("xirang-toolchain = {{ version = \"{RELEASE_REQUIREMENT}\" }}"),
        ),
        DependencySource::Git { url } => (
            format!(
                "xirang-toolchain = {{ git = \"{url}\", branch = \"main\", version = \"{RELEASE_REQUIREMENT}\" }}"
            ),
            format!(
                "xirang-toolchain = {{ git = \"{url}\", branch = \"main\", version = \"{RELEASE_REQUIREMENT}\" }}"
            ),
        ),
    }
}

/// Render the manifest, build script, and source entry for a new project.
/// 渲染新项目的 manifest、构建脚本和源码入口。
pub fn project_files(
    package: &str,
    kind: ProjectKind,
    source: &DependencySource,
) -> Vec<(&'static str, String)> {
    let (runtime_dependency, build_dependency) = dependency_specs(source);
    // A binary host demonstrates the whole runtime-evidence chain, because that is
    // the half nothing else in this workspace does: record under the mode the
    // environment asks for, then write the artifact where every reader looks
    // (`XIRANG_TRACE_FILE`, else `.xirang/traces/xirang.trace`). It is inert
    // until someone opts in — `CallTrace::runtime()` is `off` in a release build and
    // `errors-only` in a debug one — so the release path collects nothing; the point
    // is that the chain is visible and runnable, not that evidence always exists.
    // 二进制宿主演示整条运行期证据链，因为这是本工作区里别的任何东西都不做的那一半：按环境要求的模式
    // 记录，再把 artifact 写到所有读取方都看的地方（`XIRANG_TRACE_FILE`，否则
    // `.xirang/traces/xirang.trace`）。在有人 opt-in 之前它是惰性的——`CallTrace::runtime()`
    // 在 release 构建里是 `off`、在 debug 里是 `errors-only`——因此发布路径什么都不收集；意义在于那条
    // 链可见且可跑，而不是永远存在证据。
    // Every XiRang path in the generated source is spelled from the crate the
    // generated manifest depends on. The host crate does re-export the kernel's
    // `registry_core` at its root — that is what `host!()` emits, and it is why the
    // bare `lexicon`/`root_node_id` spellings below resolve — but the trace APIs are
    // not part of that re-export, so `crate::CallTrace` was a mis-anchor: it made every
    // scaffolded binary host fail to compile. The crate name itself comes from the same
    // kernel constant the build-time renderer spells its generated code with, so a
    // rename moves it in one place.
    // 生成源码里每一处 XiRang 路径都从生成清单所依赖的那个 crate 写起。宿主 crate 确实在根上
    // 重导出了内核的 `registry_core`——`host!()` 发射的就是它，也正是下面裸写 `lexicon` /
    // `root_node_id` 能解析的原因——但 trace 那组 API 不在那份重导出里，因此 `crate::CallTrace`
    // 是一处失锚：它让每个脚手架生成的二进制宿主都编译不过。crate 名本身取自构建期渲染器发射生成
    // 代码时所用的同一个内核常量，一次改名只动一处。
    let toolchain = lexicon::RUN_METHOD_CRATE;
    let binary_body = format!(
        "\nfn main() {{\n    \
         // Evidence is opt-in: neither `XIRANG_TRACE` (the collection mode) nor\n    \
         // `XIRANG_TRACE_FILE` (its path) is set by default, and without one of them this\n    \
         // host records and writes nothing at all.\n    \
         let asked = std::env::var_os({toolchain}::lexicon::TRACE_FILE_ENV).is_some()\n        \
         || std::env::var_os({toolchain}::lexicon::TRACE_MODE_ENV).is_some();\n    \
         let mut trace = {toolchain}::CallTrace::runtime();\n    \
         let root = {toolchain}::root_node_id(env!(\"CARGO_PKG_NAME\"));\n    \
         let faces = trace.with(root, \"main\", |_| builtin_static_plan().len());\n    \
         println!(\"registered faces: {{faces}}\");\n    \
         if !asked {{\n        return;\n    }}\n    \
         let path = {toolchain}::trace_artifact_path(std::path::Path::new(env!(\n        \
         \"CARGO_MANIFEST_DIR\",\n    )));\n    \
         match {toolchain}::write_trace_artifact(&trace, &path, env!(\"CARGO_PKG_NAME\")) {{\n        \
         Ok(()) => println!(\"trace written: {{}}\", path.display()),\n        \
         Err(error) => eprintln!(\"xirang: trace not written: {{error}}\"),\n    }}\n}}\n"
    );
    let prelude = format!(
        "{toolchain}::run_method::host!();\n{}",
        if kind == ProjectKind::Library {
            "\n// A library host has no `main` to write at the end of: record around the work you\n\
             // actually run (`CallTrace::runtime`, `trace_call!`), then write it with\n\
             // `trace_artifact_path` + `write_trace_artifact`, or scaffold a binary for the demo.\n"
        } else {
            binary_body.as_str()
        }
    );
    // The inner `[workspace]` table makes the generated manifest its own workspace root.
    // Without it, scaffolding inside an existing workspace fails `cargo metadata`,
    // `check` and `build` with "current package believes it's in a workspace when it's
    // not" — the repository's own rehearsal script had to append the table by hand.
    // 内部的 `[workspace]` 表让生成的清单成为它自己的工作区根。没有它，在已有工作区内脚手架会
    // 让 `cargo metadata`、`check` 与 `build` 报 "current package believes it's in a workspace
    // when it's not"——本仓库自己的演练脚本不得不手工追加这张表。
    let cargo = format!(
        "[workspace]\n\n[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n\n[dependencies]\n{runtime_dependency}\n\n[build-dependencies]\n{build_dependency}\n\n"
    );
    vec![
        ("Cargo.toml", cargo),
        (
            "build.rs",
            format!("fn main() {{\n    {toolchain}::build_method::run();\n}}\n"),
        ),
        (kind.entry_file(), prelude),
    ]
}

/// Validate the package name and target directory, then write the project.
/// 校验包名与目标目录后写入项目文件。
pub fn create_project(
    root: &Path,
    package: &str,
    kind: ProjectKind,
    source: &DependencySource,
) -> Result<(), String> {
    if package.is_empty() {
        return Err("package name is required".to_owned());
    }
    if !package
        .chars()
        .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
    {
        return Err("package must use letters, digits, '_' or '-'".to_owned());
    }
    if root.exists()
        && fs::read_dir(root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true)
    {
        return Err(format!("{} is not empty", root.display()));
    }
    // A half-scaffolded project is worse than none: the manifest may name modules
    // whose files never arrived, and the next attempt refuses the directory as
    // non-empty. The emptiness check above means everything under `root` is ours,
    // so a failure removes what we wrote — unless the directory already existed,
    // in which case the error says what was left behind rather than deleting a
    // directory the reader made.
    // 半成品的项目比没有更糟：清单可能引用从未出现的模块文件，而下一次尝试会以"非空"拒绝
    // 该目录。上面的空目录检查意味着 `root` 下的一切都是我们写的，因此失败时移除我们写下的
    // 内容——除非该目录本来就存在，那种情况下错误会说明留下了什么，而不是删掉读者建的目录。
    let existed = root.exists();
    write_project(root, &project_files(package, kind, source), existed)
}

/// Write every part of a project, removing it again when any step fails.
/// 写下一个项目的每个部分；任何一步失败时再把它移除。
///
/// The emptiness check in `create_project` means everything under `root` is ours,
/// so a failure removes what we wrote — unless the directory already existed, in
/// which case the error says what was left behind rather than deleting a
/// directory the reader made.
/// `create_project` 的空目录检查意味着 `root` 下的一切都是我们写的，因此失败时移除我们
/// 写下的内容——除非该目录本来就存在，那种情况下错误会说明留下了什么，而不是删掉读者建的
/// 目录。
fn write_project(root: &Path, files: &[(&str, String)], existed: bool) -> Result<(), String> {
    let outcome = write_project_files(root, files).and_then(|()| {
        // A new project gets the face-field snippets too, so `kind: ` is one pick
        // away from the first field the author writes.
        // 新项目同时得到注册面字段 snippet，因此作者写下的第一个字段就能一键得到 `kind: `。
        write_editor_snippets(root, Editor::Vscode)
    });
    if let Err(error) = outcome {
        if existed {
            return Err(format!(
                "{error}; a partial project was left in {}",
                root.display()
            ));
        }
        let _ = fs::remove_dir_all(root);
        return Err(format!("{error}; the partial project was removed"));
    }
    Ok(())
}

fn write_project_files(root: &Path, files: &[(&str, String)]) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for (relative, content) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        if let Err(error) = fs::write(&path, content) {
            return Err(format!("cannot write {}: {error}", path.display()));
        }
    }
    Ok(())
}

fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        DependencySource, ProjectKind, XIRANG_REPOSITORY, create_project, dependency_specs,
        detected_source, project_files, requested_source, toml_path,
    };
    use crate::build_method::scaffold::{Editor, SNIPPET_FILE, editor_snippets};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A copied checkout answers for itself, even when its build artifacts carry another path.
    /// 被复制的检出为自己作答，即使它的构建产物里带着另一个路径。
    ///
    /// This is the case that made the detection read the binary's path: a checkout copied to a new
    /// directory reuses `target/`, the reused artifacts embed the **original** compile-time path,
    /// and the old detection therefore answered "not a checkout" — the generated project then
    /// depended on the published release and failed to compile against the API the scaffold writes.
    /// Measured on a frozen copy: three test faces failed on exactly this.
    /// 正是这个用例让检测改读二进制路径：检出被复制到新目录时会复用 `target/`，而复用的产物嵌着**原来**
    /// 的编译期路径，于是旧检测回答"不是检出"——生成的项目依赖已发布版本，并因为与该脚手架写下的 API
    /// 不符而编译失败。在一份冻结副本上实测：三个测试面正是这样失败的。
    #[test]
    fn a_copied_checkout_is_detected_from_its_binary() {
        let base =
            std::env::temp_dir().join(format!("xirang-copied-checkout-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let original = base.join("original");
        let copy = base.join("copy");
        for root in [&original, &copy] {
            std::fs::create_dir_all(root.join("kernel")).expect("kernel sibling");
            std::fs::create_dir_all(root.join("toolchain")).expect("toolchain sibling");
        }
        std::fs::create_dir_all(copy.join("target/debug")).expect("binary directory");
        let source = detected_source(
            &original.join("toolchain"),
            &copy.join("target/debug/xirang"),
        );
        let (runtime, build) = dependency_specs(&source);
        assert!(
            runtime.contains(&toml_path(&copy.join("toolchain"))),
            "the copied checkout is the one the binary belongs to: {runtime}"
        );
        assert!(
            !runtime.contains(&toml_path(&original.join("toolchain"))),
            "the compile-time path must not win over the binary's own: {runtime}"
        );
        assert!(
            !build.contains(&toml_path(&original.join("toolchain"))),
            "nor may the build half: {build}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A new binary host demonstrates the runtime-evidence chain, because no
    /// other project in this repository does: record under the mode the
    /// environment asks for, then write the artifact where every reader looks.
    /// It stays inert until someone opts in, so the release path collects
    /// nothing — the point of the demo is that the *chain* is visible and
    /// runnable, not that evidence is always collected.
    /// 新的二进制宿主演示运行期证据链，因为本仓库里没有别的项目做这件事：按环境要求的模式记录，再把
    /// artifact 写到所有读取方都看的地方。在有人 opt-in 之前它保持惰性，因此发布路径什么都不收集——
    /// 这个演示的意义在于那条**链**可见且可跑，而不是永远在收集证据。
    #[test]
    fn a_binary_host_records_and_writes_a_trace() {
        let source = DependencySource::Local {
            workspace: PathBuf::from("/checkout"),
        };
        let entry = project_files("probe", ProjectKind::Binary, &source)
            .into_iter()
            .find(|(name, _)| *name == "src/main.rs")
            .expect("the binary entry is generated")
            .1;
        for needed in [
            "CallTrace::runtime()",
            "trace_artifact_path",
            "write_trace_artifact",
            "TRACE_FILE_ENV",
            "TRACE_MODE_ENV",
        ] {
            assert!(
                entry.contains(needed),
                "the demo must contain {needed}: {entry}"
            );
        }
        // A library host has no `main` to write at the end of, so it gets the pointer
        // instead of a demo that cannot run. The pointer names both APIs, which is why
        // the assertion is about the absent `main` rather than about a name appearing
        // in a comment.
        // 库宿主没有可在结尾写入的 `main`，因此它拿到的是指引，而不是一段跑不起来的演示。指引里两个
        // API 都会被点名，所以断言落在"没有 `main`"上，而不是落在"某个名字出现在注释里"。
        let library = project_files("probe", ProjectKind::Library, &source)
            .into_iter()
            .find(|(name, _)| *name == "src/lib.rs")
            .expect("the library entry is generated")
            .1;
        assert!(!library.contains("fn main"), "{library}");
        assert!(library.contains("trace_artifact_path"), "{library}");
        assert!(library.contains("CallTrace::runtime"), "{library}");
    }

    /// Every XiRang path a scaffold writes is spelled from the crate the generated
    /// manifest depends on, and each old spelling is asserted **absent**: the templates
    /// said `crate::host!()`, `crate::run()` and `crate::CallTrace` for two batches while
    /// nothing compiled a generated project, so every scaffolded host was dead on
    /// arrival. Listing only the new spelling would let a later edit add the old one back
    /// beside it — the same "a description that reads well in both directions" trap the
    /// `apply cut` wording fell into.
    /// 脚手架写下的每一处 XiRang 路径都从生成清单所依赖的那个 crate 写起，并且每一处旧拼法都被
    /// **反向断言不许出现**：模板用 `crate::host!()`、`crate::run()` 与 `crate::CallTrace`
    /// 写了两批，而没有任何东西去编译生成物，于是每个生成的宿主一出生就是坏的。只列出新拼法的钉子
    /// 会让后来的改动把旧拼法加回它旁边——正是 `apply cut` 的措辞栽进去的那个"两个方向读起来都通顺"的坑。
    #[test]
    fn a_generated_host_spells_every_xirang_path_from_its_dependency() {
        let crate_name = xirang_kernel::lexicon::RUN_METHOD_CRATE;
        let source = DependencySource::Local {
            workspace: PathBuf::from("/checkout"),
        };
        let content = |kind, name: &str| {
            project_files("probe", kind, &source)
                .into_iter()
                .find(|(file, _)| *file == name)
                .unwrap_or_else(|| panic!("{name} is generated"))
                .1
        };
        let build = content(ProjectKind::Binary, "build.rs");
        assert!(
            build.contains(&format!("{crate_name}::build_method::run()")),
            "the build script calls the merged crate by its full path: {build}"
        );
        for kind in [ProjectKind::Binary, ProjectKind::Library] {
            let entry = content(kind, kind.entry_file());
            assert!(
                entry.contains(&format!("{crate_name}::run_method::host!()")),
                "the entry pulls in the generated plan by its full path: {entry}"
            );
            for stale in [
                "crate::host!",
                "crate::run()",
                "crate::CallTrace",
                "crate::trace_artifact_path",
                "crate::write_trace_artifact",
            ] {
                assert!(
                    !entry.contains(stale) && !build.contains(stale),
                    "`{stale}` is the old spelling and does not resolve in a host: {entry}"
                );
            }
        }
        // The local source points both tables at the one crate the merge produced.
        // 本地来源把两个表都指向合并之后的那一个 crate。
        let (runtime, build_dependency) = dependency_specs(&source);
        for spec in [&runtime, &build_dependency] {
            // The spec is a TOML **basic string**, so a Windows path appears with its backslashes
            // escaped (`toml_path` does that escape) and TOML is what would unescape them: undo the
            // escape first, then compare separators. Doing it the other way round turns one
            // backslash into two slashes and fails on a path that is spelled correctly.
            // 这条 spec 是一个 TOML **基本字符串**，因此 Windows 路径的反斜杠是转义过的（`toml_path` 负责
            // 那次转义），而要解开它的是 TOML：先还原转义，再统一分隔符。反过来做会把一个反斜杠变成两个斜杠，
            // 让一条拼写正确的路径判为失败。
            let spelling = spec.replace("\\\\", "\\").replace('\\', "/");
            assert!(spelling.contains("/checkout/toolchain"), "{spec}");
            for stale in ["run_method", "build_method"] {
                assert!(!spec.contains(stale), "{stale} no longer exists: {spec}");
            }
        }
    }

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xirang-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    /// A failure part-way through leaves no half-project behind, and a directory
    /// the reader already had is reported rather than deleted.
    /// 进行到一半时的失败不会留下半个项目，而读者本来就有的目录会被如实报告而不是删掉。
    #[test]
    fn a_failed_scaffold_removes_what_it_wrote() {
        let root = temporary_directory("project-partial");
        // The first entry creates `a` as a directory, so the second cannot write
        // `a` as a file: the failure happens after something was written.
        // 第一条把 `a` 建成目录，因此第二条无法把 `a` 当文件写入：失败发生在已经写下东西之后。
        let files = vec![
            ("a/b.txt", "written first".to_owned()),
            ("a", "cannot be written".to_owned()),
        ];

        let error =
            super::write_project(&root, &files, false).expect_err("the second write must fail");
        assert!(error.contains("the partial project was removed"), "{error}");
        assert!(
            !root.exists(),
            "nothing of the failed scaffold is left behind"
        );

        fs::create_dir_all(&root).expect("a directory the reader already had");
        let error =
            super::write_project(&root, &files, true).expect_err("the second write must fail");
        assert!(error.contains("a partial project was left in"), "{error}");
        assert!(
            root.join("a/b.txt").is_file(),
            "a pre-existing directory is reported, not deleted"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A scaffolded project already carries the file, so the author never runs
    /// the command by hand.
    /// 脚手架生成的项目自带该文件，作者无需手动执行命令。
    #[test]
    fn a_new_project_carries_the_editor_snippets() {
        let root = temporary_directory("project-snippets").join("app");
        create_project(
            &root,
            "app",
            ProjectKind::Binary,
            &DependencySource::Git {
                url: "https://example.invalid/xirang".to_owned(),
            },
        )
        .expect("project");
        let snippets = fs::read_to_string(root.join(SNIPPET_FILE)).expect("snippet file");
        assert_eq!(snippets, editor_snippets(Editor::Vscode));
        fs::remove_dir_all(root.parent().expect("project parent")).expect("cleanup");
    }
    /// Every internal requirement in a generated manifest names the release this tool
    /// belongs to, from both dependency sources, and the manifest is its own workspace
    /// root.
    /// 生成的清单里每处内部要求都点名本工具所属的发布版本（两种依赖来源都算），且该清单是自己的根工作区。
    ///
    /// `cargo publish --dry-run` refuses a manifest whose path dependency has no
    /// `version`, and a manifest without its own `[workspace]` table cannot be built
    /// inside an existing workspace ("current package believes it's in a workspace when
    /// it's not"). Both were measured on a scaffolded project. The requirement is pinned
    /// against `env!("CARGO_PKG_VERSION")` rather than a literal because the old literal
    /// (`0.1.0`) was stale for four releases: caret semantics hid it, and it would have
    /// started failing the moment the line reached `0.2.0`.
    /// `cargo publish --dry-run` 会拒绝带无版本路径依赖的清单，而缺少自己的 `[workspace]`
    /// 表的清单无法在已有工作区里构建（"current package believes it's in a workspace when
    /// it's not"）。两者都在脚手架产物上实测过。这里把要求钉在 `env!("CARGO_PKG_VERSION")`
    /// 上而不是字面量上，因为旧字面量（`0.1.0`）已经陈旧了四个发布：caret 语义把它藏住了，而版本线
    /// 一到 `0.2.0` 它就会开始失败。
    #[test]
    fn a_generated_project_can_be_published_and_built_in_a_workspace() {
        let local = DependencySource::Local {
            workspace: std::path::PathBuf::from("/tmp/xirang"),
        };
        let git = DependencySource::Git {
            url: "https://github.com/Nichtigall/xirang".to_owned(),
        };
        let expected = format!("version = \"{}\"", env!("CARGO_PKG_VERSION"));
        for source in [&local, &git] {
            let (runtime, build) = dependency_specs(source);
            for spec in [&runtime, &build] {
                assert!(
                    spec.contains(&expected),
                    "a generated manifest must require this release, not a literal: {spec}"
                );
            }
            let files = project_files("probe", ProjectKind::Binary, source);
            let cargo = &files
                .iter()
                .find(|(name, _)| *name == "Cargo.toml")
                .expect("the manifest is generated")
                .1;
            assert!(
                cargo.starts_with("[workspace]"),
                "the generated manifest is its own workspace root: {cargo}"
            );
        }
    }

    /// The registry source is a version requirement alone: no path, no repository to fetch.
    /// registry 那一支只写版本要求：没有 path，也没有要拉的仓库。
    ///
    /// Audit `F8`: the generated manifest's dependency source is what decides whether the project
    /// builds at all without a network. `path` needs a checkout on this machine, `git` needs a fetch;
    /// the published release needs neither, so those two lines must carry **only** the version.
    /// 审计 `F8`：生成清单的依赖来源决定了这个项目**能不能离线**构建。`path` 需要本机有检出、`git` 需要
    /// 一次拉取，而已发布的那一版两者都不需要，因此那两行只能带版本要求。
    #[test]
    fn the_registry_source_needs_neither_a_checkout_nor_a_fetch() {
        let (runtime, build) = dependency_specs(&DependencySource::Registry);
        for spec in [&runtime, &build] {
            assert!(
                spec.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))),
                "the released requirement is what a registry line states: {spec}"
            );
            assert!(
                !spec.contains("path =") && !spec.contains("git ="),
                "and nothing that has to be resolved outside the registry cache: {spec}"
            );
        }
        assert!(
            !runtime.contains("package = \"xirang-toolchain\"")
                && runtime.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))),
            "both tables name the one crate the merge produced: {runtime}"
        );
    }

    /// The request vocabulary: detected by default, git only when named, and one way to be wrong.
    /// 请求词表：默认用检测值；git 只在**点名**时使用；错的方式只有一种。
    #[test]
    fn a_request_names_the_source_and_git_is_never_guessed() {
        let manifest = Path::new("/nowhere/xirang/toolchain");
        let exe = Path::new("/usr/local/bin/xirang-mcp");
        // Nothing named: the detected default, and here that is the registry — not a git fetch.
        // 什么都没点名：用检测出来的默认值，而这里它是 registry——不是一次 git 拉取。
        let detected =
            requested_source(None, None, None, manifest, exe).expect("the default answers");
        assert!(
            matches!(detected, DependencySource::Registry),
            "an installed binary must not write a manifest that needs a fetch"
        );
        // A URL without the word `git`: refused, because the value alone cannot say what it meant.
        // 给了 URL 却没说 `git`：拒绝，因为光看那个值说不清它是什么意思。
        let unnamed =
            requested_source(None, Some("https://example.invalid/x"), None, manifest, exe)
                .expect_err("a URL alone is not a decision");
        assert!(unnamed.contains("dependency: \"git\""), "{unnamed}");
        // Named: the URL is used, and with no URL the project's own repository is.
        // 点名了：用给的 URL；没给 URL 时用本项目自己的仓库。
        let named = requested_source(
            Some("git"),
            Some("https://example.invalid/x"),
            None,
            manifest,
            exe,
        )
        .expect("a named git source answers");
        assert!(
            matches!(&named, DependencySource::Git { url } if url == "https://example.invalid/x"),
            "{named:?}"
        );
        let default_url =
            requested_source(Some("git"), None, None, manifest, exe).expect("a default URL");
        assert!(
            matches!(&default_url, DependencySource::Git { url } if url == XIRANG_REPOSITORY),
            "{default_url:?}"
        );
        // `path` without a checkout is refused rather than silently turned into the registry.
        // 没有检出时 `path` 被拒绝，而不是悄悄换成 registry。
        let no_checkout = requested_source(Some("path"), None, None, manifest, exe)
            .expect_err("an installed binary has no checkout to point at");
        assert!(
            no_checkout.contains("dependency: \"registry\"")
                && no_checkout.contains("Nothing was created"),
            "{no_checkout}"
        );
        // And an unknown spelling names the three that exist.
        // 未知拼写会点名存在的三种。
        let unknown = requested_source(Some("crates-io"), None, None, manifest, exe)
            .expect_err("only three spellings exist");
        for spelling in ["path", "registry", "git"] {
            assert!(unknown.contains(spelling), "{unknown}");
        }
    }
}
