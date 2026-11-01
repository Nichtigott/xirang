//! Where a package's registration faces live, and how identity names them.
//! 包的注册面住在哪里，以及身份如何命名它们。
//!
//! Two decisions that used to be one hardcoded constant, `package_root/src`:
//!
//! * which directory the discovery walk reads, and
//! * what a face's identity path (the `source` half of
//!   `NodeId::from_namespaced_path`) is relative to.
//!
//! They coincide for every package that lays its sources out under `src/`, which
//! is why one constant worked for so long. A package whose library target lives
//! elsewhere — `[lib] path = "host/lib.rs"`, a legal Cargo layout — separates
//! them, and the declaration macros already answer the second question:
//! `manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!())` takes the file
//! relative to the manifest directory and drops **one leading `src/`**. Measured
//! for that layout, `file!()` is `host/print.rs`, so a face at
//! `<root>/host/control/control.rs` is stamped with the identity path
//! `host/control/control.rs` — not `control/control.rs`.
//! 过去被写死为同一个常量 `package_root/src` 的两个决定：
//!
//! * 发现遍历读哪个目录，以及
//! * 一个面的身份路径（`NodeId::from_namespaced_path` 的 `source` 那一半）相对什么。
//!
//! 对每个把源码铺在 `src/` 之下的包，这两者重合——正因如此，一个常量才能用这么久。库目标住在
//! 别处的包（`[lib] path = "host/lib.rs"`，一种合法的 Cargo 布局）会把它们分开，而声明宏早已
//! 回答了第二个问题：`manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!())` 把文件取
//! 相对清单目录的路径，并去掉**一个前导 `src/`**。对该布局实测，`file!()` 是 `host/print.rs`，
//! 因此 `<root>/host/control/control.rs` 处的面被盖上的身份路径是 `host/control/control.rs`，
//! 不是 `control/control.rs`。
//!
//! The two bases are therefore:
//!
//! * target outside `src/` (or at the package root): walk the target's directory,
//!   take identity relative to the package root;
//! * anything else, including a target *inside* `src/`: walk `src/`, take identity
//!   relative to `src/` — the historical behaviour, and still exactly what the
//!   macros compute, because dropping the leading `src/` is what the walk-relative
//!   form already does.
//!
//! 因此两个基准是：
//!
//! * 目标在 `src/` 之外（或在包根）：遍历目标所在目录，身份相对包根；
//! * 其余情形（包括目标在 `src/` **之内**）：遍历 `src/`，身份相对 `src/`——历史行为，而且仍然
//!   正是宏所计算的值，因为去掉前导 `src/` 正是"相对遍历根"那种写法已经在做的事。
//!
//! Why the manifest is read here and not through `cargo metadata`: this module runs
//! inside a build script, and spawning `cargo` from there would contend for the lock
//! the outer build holds (the same reason [`super::package_name`] is not called by
//! the pipeline). The read is narrow on purpose — one key, in one table — and the
//! answer is verified before it is used: a `path` naming something that is not a
//! file is refused by name, so a misread fails loudly instead of walking a tree
//! nobody asked for.
//! 为什么清单在这里读而不是经 `cargo metadata`：本模块在构建脚本里运行，从那里 spawn `cargo`
//! 会去争外层构建持有的锁（与 [`super::package_name`] 不被管线调用是同一个理由）。这次读取是
//! 有意窄的——一张表里的一个键——而且答案在使用前会被校验：`path` 指到不是文件的东西就按名字拒绝，
//! 因此读错会响亮失败，而不是去遍历一棵没人要的树。

use std::path::{Path, PathBuf};

/// One package's source layout.
/// 一个包的源码布局。
///
/// Public because a *writing* surface needs the same answer the build uses: the
/// bridge authors into the tree the build will read, and guessing `src/` there
/// would write a face the build never sees.
/// 公开是因为**写入**执行面需要与构建相同的答案:桥创作到构建会读的那棵树里,而在那里猜 `src/`
/// 会写下一个构建永远看不见的注册面。
pub struct SourceLayout {
    /// The package root: the base a configured `XIRANG_ENTRY` resolves against,
    /// and the directory a target path is joined to.
    /// 包根：配置的 `XIRANG_ENTRY` 所相对的基准，也是目标路径拼接的起点。
    pub package_root: PathBuf,
    /// The directory the discovery walk reads.
    /// 发现遍历读取的目录。
    pub scan_root: PathBuf,
    /// The base a face's identity path is taken relative to.
    /// 面的身份路径所相对的基准。
    pub identity_base: PathBuf,
    /// The library target the manifest names, when it names one.
    /// 清单命名的库目标；清单没有命名时为空。
    pub target: Option<PathBuf>,
}

/// Resolve where one package's faces live.
/// 解析一个包的注册面住在哪里。
///
/// A manifest that cannot be read, or that names no library target, keeps the
/// historical `src/` root: this is not the place to report a missing manifest,
/// because the walk's own "not a source directory" diagnostic already names the
/// tree it looked for and is the one a reader can act on.
/// 读不了的清单，或没有命名库目标的清单，沿用历史上的 `src/` 根：这里不是报告清单缺失的地方，
/// 因为遍历自己那条"不是源码目录"的诊断已经点名它找的是哪棵树，而那条才是读者能据以行动的诊断。
///
/// A library target that is **not a file** is an `Err` naming both the manifest key
/// and the path it produced. That is the verify step for the narrow read above: a
/// misread path cannot quietly point the walk at some other tree, because a path
/// that names nothing is refused before anything is walked.
/// 库目标**不是文件**时返回 `Err`，同时点名清单里的键与它产生的路径。这就是上面那次窄读的校验步：
/// 读错的路径无法悄悄把遍历指向别的树，因为指向空处的路径在遍历任何东西之前就被拒绝。
pub fn source_layout(package_root: &Path) -> Result<SourceLayout, String> {
    let src = package_root.join("src");
    let named = std::fs::read_to_string(package_root.join("Cargo.toml"))
        .ok()
        .and_then(|text| lib_path(&text));
    let Some(relative) = named else {
        return Ok(SourceLayout {
            package_root: package_root.to_path_buf(),
            scan_root: src.clone(),
            identity_base: src,
            target: None,
        });
    };
    let target = package_root.join(&relative);
    if !target.is_file() {
        return Err(format!(
            "{} names `{relative}` as the library target, and {} is not a file",
            package_root.join("Cargo.toml").display(),
            target.display()
        ));
    }
    let inside_src = target.starts_with(&src);
    let scan_root = if inside_src {
        src.clone()
    } else {
        target
            .parent()
            .map_or_else(|| package_root.to_path_buf(), Path::to_path_buf)
    };
    let identity_base = if scan_root == src {
        src
    } else {
        package_root.to_path_buf()
    };
    Ok(SourceLayout {
        package_root: package_root.to_path_buf(),
        scan_root,
        identity_base,
        target: Some(target),
    })
}

impl SourceLayout {
    /// The Rust module chain one face file is mounted at.
    /// 一个面文件被挂载到的 Rust 模块链。
    ///
    /// Two shapes, and they are the discovery walk's own:
    ///
    /// * a file **below the scan root** is mounted at its directory chain, so
    ///   `src/panel/object/button/button.rs` is `panel::object::button`;
    /// * a file **outside it** — an explicit placement, which is a root-level node named by its
    ///   stem — is mounted at that stem alone: `components/reading/reading.rs` is `reading`, even
    ///   though it sits under two directories.
    ///
    /// The chain is not the physical directory chain, and deriving it from the source spelling is
    /// how an explicitly placed face once produced `::tmp::f3-e2e::…` as its module path (a
    /// leading `::` and a package name that is not an identifier). It is also not the registration
    /// path: a face may hang under a parent it is not a Rust child of.
    /// 两种形状，都是发现遍历自己的：
    ///
    /// * **扫描根之下**的文件挂在其目录链上，因此 `src/panel/object/button/button.rs` 是
    ///   `panel::object::button`；
    /// * **扫描根之外**的文件——显式落点，一个以 stem 命名的根级节点——只挂在那个 stem 上：
    ///   `components/reading/reading.rs` 是 `reading`，尽管它落在两层目录下。
    ///
    /// 模块链不是物理目录链，而从源码拼写推导它，正是显式落点的面曾经把 `::tmp::f3-e2e::…` 当模块
    /// 路径产出的原因（开头的 `::` 与一个不是合法标识符的包名）。它也不是注册路径：一个面可以挂在
    /// 它并非其 Rust 子节点的父级下。
    pub fn module_chain(&self, file: &Path) -> Option<String> {
        if let Ok(relative) = file.strip_prefix(&self.scan_root) {
            let directory = relative.parent()?;
            let mut segments = Vec::new();
            for component in directory.components() {
                match component {
                    std::path::Component::Normal(segment) => {
                        segments.push(segment.to_string_lossy().into_owned())
                    }
                    _ => return None,
                }
            }
            if segments.is_empty() {
                return None;
            }
            return Some(segments.join("::"));
        }
        file.file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
    }
}

/// The library target path one manifest declares, as written.
/// 一份清单声明的库目标路径，按原样。
///
/// Narrow on purpose: TOML sections, quoted values, comments, and the dotted
/// `lib.path = "…"` spelling — the forms a manifest this workspace can hand a
/// build actually uses. Anything else answers `None`, which keeps the `src/`
/// default rather than guessing a tree.
/// 有意收窄：TOML 区段、带引号的值、注释，以及点式 `lib.path = "…"` 写法——本工作区可能交给
/// 构建的清单实际会用的形式。其余写法一律答 `None`，即保留 `src/` 默认值，而不是猜一棵树。
fn lib_path(manifest: &str) -> Option<String> {
    let mut in_lib = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(header) = trimmed.strip_prefix('[') {
            // `[[bin]]` strips to `[bin]` here, so it never reads as the library
            // table.
            // `[[bin]]` 在这里剥成 `[bin]`，因此绝不会被读成库表。
            in_lib = header.trim_end_matches(']').trim() == "lib";
            continue;
        }
        let (key, value) = match trimmed.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let key = key.trim().strip_prefix("lib.").unwrap_or(key.trim());
        // The dotted form is a top-level `lib.path = "…"`; inside the table the
        // key is bare `path`.
        // 点式写法是顶层的 `lib.path = "…"`；在表内键是裸的 `path`。
        let dotted = trimmed.starts_with("lib.");
        if key != "path" || (!in_lib && !dotted) {
            continue;
        }
        if let Some(value) = quoted(value.trim()) {
            return Some(value);
        }
    }
    None
}

/// The value of a TOML scalar, with its quotes removed.
/// 一个 TOML 标量的取值，去掉引号。
///
/// Single and double quotes both count, and a trailing comment is not part of the
/// value: `"host/lib.rs" # the layout` names `host/lib.rs`.
/// 单引号与双引号都算，行尾注释不属于取值：`"host/lib.rs" # the layout` 命名的是 `host/lib.rs`。
fn quoted(value: &str) -> Option<String> {
    let quote = value.chars().next()?;
    let rest = if quote == '"' || quote == '\'' {
        &value[1..]
    } else {
        value
    };
    let end = if quote == '"' || quote == '\'' {
        rest.find(quote)?
    } else {
        rest.find(['#', ' ', '\t']).unwrap_or(rest.len())
    };
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::lib_path;

    /// The forms a manifest may use, and the ones this reader refuses to guess
    /// from. The dotted spelling matters most: it is the same table with no
    /// `[lib]` header, so a section-only reader answers `None` and the package
    /// silently keeps the `src/` root.
    /// 清单可能使用的形式，以及本读取方拒绝据以猜测的那些。点式写法最重要：它是同一张表却没有
    /// `[lib]` 表头，因此只看区段的读取方会答 `None`，包就静默地保留 `src/` 根。
    #[test]
    fn the_library_path_is_read_from_its_own_table_or_the_dotted_form() {
        let cases: &[(&str, Option<&str>)] = &[
            ("[lib]\npath = \"host/lib.rs\"\n", Some("host/lib.rs")),
            ("[lib]\npath = 'host/lib.rs'\n", Some("host/lib.rs")),
            (
                "[lib]\npath = \"host/lib.rs\" # the layout\n",
                Some("host/lib.rs"),
            ),
            ("lib.path = \"host/lib.rs\"\n", Some("host/lib.rs")),
            (
                "package.name = \"app\"\nlib.path = \"host/lib.rs\"\n",
                Some("host/lib.rs"),
            ),
            (
                "[package]\nname = \"app\"\n\n[lib]\nname = \"other\"\npath = \"host/lib.rs\"\n",
                Some("host/lib.rs"),
            ),
            ("[lib]\nname = \"other\"\n", None),
            ("[[bin]]\npath = \"host/main.rs\"\n", None),
            ("[dependencies]\npath = \"host/lib.rs\"\n", None),
            ("# [lib]\n# path = \"host/lib.rs\"\n", None),
            (
                "[package]\nname = \"app\"\n\n[lib]\npath = \"src/host.rs\"\n",
                Some("src/host.rs"),
            ),
        ];
        for (manifest, expected) in cases {
            assert_eq!(lib_path(manifest).as_deref(), *expected, "{manifest:?}");
        }
    }
}
