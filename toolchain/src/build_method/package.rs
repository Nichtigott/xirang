//! The package identity Cargo reports for a directory.
//! Cargo 为某个目录报告的包身份。
//!
//! A host's package name *is* its `NodeId` namespace: every face identity is a
//! hash over `(namespace, source path, declared name)`, and the declaration
//! macros read `crate::XIRANG_NAMESPACE` as that namespace, and `host!()` defines that constant
//! from `env!("CARGO_PKG_NAME")` (audit `M7`, P3.3). Two surfaces need
//! the same answer — `xirang-toolchain` to name faces from the command line, and the
//! MCP bridge to report the tree it is asked about — so the read lives in one
//! place and the policy is stated once: Cargo answers, and a question Cargo
//! cannot answer is an error rather than a guess.
//! 宿主的包名**就是**它的 `NodeId` 命名空间：每个面的身份都是对
//! `(命名空间, 源码路径, 声明名)` 的散列，而声明宏读的是 crate 根常量
//! `crate::XIRANG_NAMESPACE` 作为该命名空间——那个常量由 `host!()` 用 `env!("CARGO_PKG_NAME")`
//! 定义（审计 `M7`，P3.3）。两个执行面需要同一个答案——`xirang-toolchain` 从命令行命名面，MCP 桥报告被问到的
//! 那棵树——因此这次读取只有一个住址，策略也只声明一次：由 Cargo 作答，而 Cargo 答不出的问题
//! 是错误，不是猜测。
//!
//! Nothing in the build pipeline calls this: a build script receives
//! `CARGO_PKG_NAME` from Cargo, which is the same value, so the subprocess below
//! happens only when a standalone surface asks. That distinction is why this
//! stays a plain function instead of being wired into `pipeline::run`.
//! 构建管线不调用这里：构建脚本从 Cargo 拿到 `CARGO_PKG_NAME`，也就是同一个值，因此下面的
//! 子进程只会在独立执行面提问时发生。正因如此，它是普通函数，而不接进 `pipeline::run`。

use std::path::Path;

/// Ask Cargo for the package name of the package whose manifest is `manifest`.
/// 向 Cargo 询问清单为 `manifest` 的那个包的包名。
///
/// The parameter is the manifest **file**, not its directory: that is the thing
/// the caller actually has — a surface that resolved a project holds a manifest
/// path, possibly one an environment variable named — and it is also what Cargo
/// itself takes. A directory would bake in the assumption that the manifest is
/// called `Cargo.toml`; Cargo refuses any other name anyway ("the manifest-path
/// must be a path to a Cargo.toml file"), so the assumption is not even useful.
/// 参数是清单**文件**而不是它所在目录：那正是调用方实际持有的东西——解析了项目的执行面拿到的是一条
/// 清单路径，还可能来自环境变量——而且这也是 Cargo 自己接受的取值。传目录会把"清单就叫
/// `Cargo.toml`"这个假设烤进接口；Cargo 反正拒绝别的名字（"the manifest-path must be a path to a
/// Cargo.toml file"），所以这个假设连用处都没有。
///
/// The identity namespace a surface must use: the configured override verbatim, else Cargo's
/// answer for the manifest.
/// 一个执行面必须使用的身份命名空间：配置的覆盖原样胜出，否则就是 Cargo 对该清单的回答。
///
/// This is the half every surface shares, and it is shared **here** because the two surfaces that
/// ask it — the MCP bridge and Studio — must not be able to disagree about it: an identity domain
/// that two surfaces compute differently names nodes the host never compiled (audit `S12`,
/// `LGC-LG-13`). What they deliberately do *not* share is the fallback when Cargo has no answer:
/// authoring creates a tree, so Studio falls back to the documented default, while a query about an
/// existing tree refuses rather than invent one. Keeping that difference at each caller's one line
/// is what makes it visible instead of buried.
/// 这是每个执行面共享的那一半，而且**在这里**共享，是因为问它的两个执行面——MCP 桥与 Studio——不能对它
/// 有分歧：两个执行面算出不同答案的身份域，命名的是宿主从未编译过的节点（审计 `S12`、`LGC-LG-13`）。
/// 它们有意**不**共享的是"Cargo 答不出时怎么办"：创作是在创建一棵树，因此 Studio 回落到文档化的默认值；
/// 而对已存在树的查询宁可拒绝也不凭空造一个。把这个差别留在各自调用处的那一行，正是让它可见而不是被埋掉。
///
/// An empty or whitespace-only override counts as absent: `XIRANG_NAMESPACE=` is how a shell
/// clears a variable it inherited, and taking it literally produced a namespace no `NodeId` was ever
/// hashed under — every identity in that session came back as one the tree had never seen.
/// 空或只有空白的覆盖算作没有设置：`XIRANG_NAMESPACE=` 正是 shell 清掉一个继承来的变量的写法，而
/// 照字面采用它会得到一个从未有任何 `NodeId` 在其下散列的命名空间——那个会话里的每个身份都会被报成
/// 这棵树从未见过的。
pub fn identity_namespace(configured: Option<&str>, manifest: &Path) -> Result<String, String> {
    if let Some(configured) = configured.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(configured.to_owned());
    }
    package_name(manifest)
}

/// Reading `[package] name` by hand was wrong three ways at once: it did not
/// know TOML sections, so a `[lib]` or `[[bin]]` name key could answer first; it
/// did not know single-quoted strings; and when it found nothing it silently
/// returned the directory name. The package name is the `NodeId` namespace, so a
/// wrong answer makes the same crate report one identity under `xirang check`
/// and another under a real build. Cargo is the authority, and a question it
/// cannot answer is an error rather than a guess.
/// 手写读 `[package] name` 同时错了三处：它不认识 TOML 区段，于是 `[lib]` 或
/// `[[bin]]` 的 name 键可能抢先作答；它不认识单引号字符串；找不到时还会静默返回目录
/// 名。包名是 `NodeId` 的命名空间，答错就会让同一个 crate 在 `xirang check` 与真实
/// 编译下报告两种身份。Cargo 才是权威；它答不出的问题一律报错，而不是猜。
/// The `[lib] name` a manifest declares, read from its text (no cargo metadata).
/// 清单声明的 `[lib] name`，从文本读取（不跑 cargo metadata）。
///
/// The build script of a generated facade runs **inside** cargo's build phase; a nested metadata
/// call there is both slow and order-sensitive. The host's library name is a stable fact of the
/// committed partition plan, so it is read where the plan is made and carried into the generated
/// script as an input.
/// 生成 facade 的构建脚本运行在 cargo 的构建阶段**之内**；在那里再跑一次 metadata 又慢又依赖顺序。
/// 宿主库名是已提交分区计划里的稳定事实，因此在制定计划处读取，并作为输入带进生成的脚本。
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub fn declared_library_name(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let mut in_lib = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_lib = trimmed == "[lib]";
            continue;
        }
        if in_lib
            && let Some(rest) = trimmed.strip_prefix("name")
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// The Rust crate name a manifest's library target is imported as.
/// 清单里库目标被导入时使用的 Rust crate 名。
///
/// A package may set `[lib] name = "control_button"` while the package is called
/// `xirang-example-control-button`; generated cross-crate code must name the **library**, not the
/// package, or every reference into it is an unresolved crate.
/// 包可以叫 `xirang-example-control-button` 而 `[lib] name = "control_button"`；生成的跨 crate 代码必须
/// 点名**库**而不是包，否则每一条指向它的引用都是解析不到的 crate。
pub fn library_name(manifest: &Path) -> Result<String, String> {
    let metadata = crate::cargo_metadata::read(manifest)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata reported no packages".to_owned())?;
    let package = packages
        .iter()
        .find(|package| {
            package["manifest_path"].as_str().is_some_and(|path| {
                Path::new(path).parent().is_some_and(|parent| {
                    same_directory(parent, manifest.parent().unwrap_or_else(|| Path::new(".")))
                })
            })
        })
        .ok_or_else(|| format!("cargo metadata does not list {}", manifest.display()))?;
    let library = package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("lib")))
        });
    match library.and_then(|target| target["name"].as_str()) {
        Some(name) => Ok(name.to_owned()),
        None => package_name(manifest),
    }
}

/// The package name Cargo reports for one manifest.
/// Cargo 为一个清单报告的包名。
pub fn package_name(manifest: &Path) -> Result<String, String> {
    let metadata = crate::cargo_metadata::read(manifest)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata reported no packages".to_owned())?;
    // `--no-deps` lists the whole workspace, so the package has to be the one
    // whose manifest this is; taking the first entry would answer for a sibling
    // member.
    // `--no-deps` 会列出整个 workspace，因此必须挑出 manifest 正是这一个的包；取第一
    // 项会替同工作区的另一个成员作答。
    let package = packages
        .iter()
        .find(|package| {
            package["manifest_path"].as_str().is_some_and(|path| {
                Path::new(path).parent().is_some_and(|parent| {
                    same_directory(parent, manifest.parent().unwrap_or_else(|| Path::new(".")))
                })
            })
        })
        .ok_or_else(|| {
            format!(
                "{} is not a package; cargo metadata listed {} workspace member(s)",
                manifest.display(),
                packages.len()
            )
        })?;
    package["name"].as_str().map(str::to_owned).ok_or_else(|| {
        format!(
            "cargo metadata reported no package name for {}",
            manifest.display()
        )
    })
}

/// Whether two paths name the same directory, following symlinks when possible.
/// 两个路径是否指向同一目录，能跟随符号链接时跟随。
fn same_directory(left: &Path, right: &Path) -> bool {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::package_name;

    /// A throwaway directory, unique per call.
    /// 每次调用唯一的临时目录。
    fn temporary_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-package-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("fixture directory");
        root
    }

    /// A package name the old hand-rolled scan got wrong: a single-quoted name
    /// with a later `[[bin]]` name, and a `name` key with no space around `=`.
    /// 旧的手写扫描会读错的包名：单引号名字后面还跟着 `[[bin]]` 的 name；以及 `name`
    /// 键等号两侧没有空格。
    #[test]
    fn the_package_name_comes_from_cargo_not_from_a_manifest_scan() {
        let root = temporary_root("name");
        let fixtures = [
            (
                "single-quoted",
                "[package]\nname = 'single-quoted'\nversion = \"0.1.0\"\n\n[[bin]]\nname = \"other-bin\"\npath = \"src/main.rs\"\n",
                "single-quoted",
            ),
            (
                "tight",
                "[package]\nname=\"tight\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"different_lib\"\npath = \"src/lib.rs\"\n",
                "tight",
            ),
        ];
        for (directory, manifest, expected) in fixtures {
            let manifest_dir = root.join(directory);
            std::fs::create_dir_all(manifest_dir.join("src")).expect("source directory");
            std::fs::write(manifest_dir.join("Cargo.toml"), manifest).expect("manifest");
            std::fs::write(manifest_dir.join("src/main.rs"), "").expect("binary source");
            std::fs::write(manifest_dir.join("src/lib.rs"), "").expect("library source");
            assert_eq!(
                package_name(&manifest_dir.join("Cargo.toml")).expect("cargo answers"),
                expected,
                "{directory}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A virtual manifest names no package, so the identity cannot be guessed:
    /// the caller has to say so instead of falling back to the directory name.
    /// 虚拟 manifest 不命名任何包，身份因此无从猜测：调用方必须说出来，而不是回退到
    /// 目录名。
    #[test]
    fn a_manifest_without_a_package_is_an_error() {
        let root = temporary_root("virtual");
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = []\nresolver = \"2\"\n",
        )
        .expect("manifest");
        let error = package_name(&root.join("Cargo.toml"))
            .expect_err("a virtual manifest names no package");
        assert!(error.contains("not a package"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A path that is not a manifest is an error too, and the message names the
    /// path: this is the case a caller reaches by passing a directory instead of
    /// the manifest file.
    /// 不是清单的路径同样是错误，而且消息点名该路径：调用方传目录而不是清单文件时走到的正是这一种。
    #[test]
    fn a_path_that_is_not_a_manifest_is_an_error() {
        let root = temporary_root("no-manifest");
        let error = package_name(&root).expect_err("a directory is not a manifest");
        assert!(error.contains("cargo metadata failed"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A configured namespace wins **verbatim** and without asking Cargo, and an empty one counts
    /// as absent: `XIRANG_NAMESPACE=` is how a shell clears an inherited variable, and taking it
    /// literally would name an identity domain no `NodeId` was ever hashed under.
    /// 配置的命名空间**原样**胜出、且不问 Cargo；空的算作没设置：`XIRANG_NAMESPACE=` 正是 shell 清掉
    /// 继承变量的写法，照字面采用它会得到一个从未有任何 `NodeId` 在其下散列的命名域。
    #[test]
    fn a_configured_namespace_wins_verbatim_and_an_empty_one_is_absent() {
        // A directory rather than a manifest: any Cargo answer at all would fail, so a `Some`
        // override proves it never asked.
        // 给的是目录而不是清单：任何 Cargo 回答都会失败，因此有覆盖时能成功就证明它根本没问。
        let root = std::env::temp_dir().join(format!("xirang-namespace-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("scratch");
        assert_eq!(
            super::identity_namespace(Some("trace-recorded-host"), &root)
                .expect("the override is the answer"),
            "trace-recorded-host"
        );
        for blank in ["", "   "] {
            let error = super::identity_namespace(Some(blank), &root)
                .expect_err("an empty override is absent, so Cargo is asked and cannot answer");
            assert!(error.contains("cargo metadata failed"), "{error}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Every copy of this workspace's own crates in a project's dependency graph, grouped by name.
/// 一个项目的依赖图里属于本工作区那些 crate 的每一份拷贝，按名字分组。
///
/// Two copies of `xirang-toolchain` are **two different types** to rustc, and the generated tree names
/// both: the host resolves its own copy, the graft implementation resolves the one it was built against.
/// The failure then lands inside a file the author cannot edit — measured on a hand-built host whose own
/// manifest took the toolchain from a checkout while its external implementation took it from the
/// registry:
/// `xirang-toolchain` 的两份拷贝对 rustc 是**两个不同的类型**，而生成树把两者都点名：宿主解析自己那一份，
/// 嫁接实现解析它编译时用的那一份。失败随后落在作者无法编辑的文件里——在一个手工宿主上实测，它的清单从检出取
/// toolchain，而它的项目外实现从注册局取：
///
/// ```text
/// error[E0308]: mismatched types
///    --> …/build/dash-dash-board-…/out/generated_lib.rs:218:144
///     |     expected `NodeId`, found a different `NodeId`
/// error[E0277]: the trait bound `…::NoParts: PartsContract` is not satisfied
///    --> …/generated_lib.rs:223:114
/// ```
///
/// Read from `cargo metadata` rather than from the manifests: a `path` dependency three crates deep is
/// still the copy that gets linked, and only cargo's own graph knows which copies a build will contain.
/// 从 `cargo metadata` 读，而不是从清单读：三层深的一个 `path` 依赖仍然是会被链接的那一份，而只有 cargo
/// 自己的图知道一次构建会包含哪些拷贝。
///
/// Returns `(name, spelling of each copy)` so the refusal can name both, and an empty list when there is
/// nothing to say — which is the normal case, and the reason this is not a warning.
/// 返回 `(名字, 每一份的拼写)` 使拒绝能同时点名两者；没有可说的时候返回空表——那是常态，也是这里不做成
/// 警告的原因。
pub fn duplicated_own_crates(manifest: &Path) -> Result<Vec<(String, Vec<String>)>, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .arg("--manifest-path")
        .arg(manifest)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot read cargo metadata output: {error}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata reported no packages".to_owned())?;
    Ok(duplicated_in(packages))
}

/// The grouping itself: which of this workspace's own crates arrive more than once, and as what.
/// 分组本身：本工作区自己的 crate 里哪些来了不止一次，以及各自以什么形式。
///
/// A pure function of one `cargo metadata` package list, so the rule can be pinned without a cargo run —
/// the same split `package_name` uses for its own question.
/// 它是**一份** `cargo metadata` 包清单的纯函数，因此这条规则不需要跑 cargo 就能被钉住——与 `package_name`
/// 为自己那个问题所做的拆分相同。
fn duplicated_in(packages: &[serde_json::Value]) -> Vec<(String, Vec<String>)> {
    let mut by_name: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for package in packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if !name.starts_with("xirang-") {
            continue;
        }
        let spelling = match package["source"].as_str() {
            Some(source) => format!(
                "{source} ({name} {})",
                package["version"].as_str().unwrap_or("?")
            ),
            None => package["manifest_path"]
                .as_str()
                .map(|path| format!("this checkout ({path})"))
                .unwrap_or_else(|| "a path dependency".to_owned()),
        };
        by_name.entry(name.to_owned()).or_default().push(spelling);
    }
    by_name
        .into_iter()
        .filter(|(_, copies)| copies.len() > 1)
        .collect()
}

#[cfg(test)]
mod duplicated_tests {
    use super::duplicated_in;

    /// One copy of each of this workspace's crates is the normal case, and the gate says nothing.
    /// 本工作区每个 crate 各一份是常态，门禁什么都不说。
    #[test]
    fn one_copy_each_is_silent() {
        let packages = vec![
            serde_json::json!({"name": "xirang-kernel", "version": "0.2.2", "source": null,
                               "manifest_path": "/w/kernel/Cargo.toml"}),
            serde_json::json!({"name": "serde", "version": "1.0.0", "source": "registry+x",
                               "manifest_path": "/r/serde/Cargo.toml"}),
        ];
        assert!(duplicated_in(&packages).is_empty());
    }

    /// A third party appearing twice is not this gate's business: two versions of a foreign crate are a
    /// normal dependency graph, and refusing them would refuse the ecosystem.
    /// 第三方出现两次不归本门禁管：外来 crate 的两个版本是正常的依赖图，拒绝它等于拒绝整个生态。
    #[test]
    fn a_foreign_crate_twice_is_left_alone() {
        let packages = vec![
            serde_json::json!({"name": "serde", "version": "1.0.0", "source": "registry+x",
                               "manifest_path": "/r/serde-1/Cargo.toml"}),
            serde_json::json!({"name": "serde", "version": "1.0.1", "source": "registry+x",
                               "manifest_path": "/r/serde-2/Cargo.toml"}),
        ];
        assert!(duplicated_in(&packages).is_empty());
    }

    /// The measured shape: the host takes the toolchain from a checkout, the graft implementation takes
    /// it from the registry, and the two copies are distinct types inside one build.
    /// 实测的形状：宿主从检出取 toolchain，嫁接实现从注册局取，而两份拷贝在同一次构建里是两个不同的类型。
    #[test]
    fn a_checkout_copy_beside_a_registry_copy_is_reported() {
        let packages = vec![
            serde_json::json!({"name": "xirang-toolchain", "version": "0.2.2", "source": null,
                               "manifest_path": "/w/toolchain/Cargo.toml"}),
            serde_json::json!({"name": "xirang-toolchain", "version": "0.2.2",
                               "source": "registry+https://github.com/rust-lang/crates.io-index",
                               "manifest_path": "/r/xirang-toolchain/Cargo.toml"}),
        ];
        let found = duplicated_in(&packages);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, "xirang-toolchain");
        assert_eq!(found[0].1.len(), 2);
        assert!(
            found[0].1[0].contains("this checkout") && found[0].1[1].contains("registry+"),
            "both spellings are named, so the reader knows which two to unify: {:?}",
            found[0].1
        );
    }
}
