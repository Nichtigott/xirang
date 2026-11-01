//! What a `crate::build_method::…` path means in the package tree.
//! 一条 `crate::build_method::…` 路径在包内目录树里意味着什么。
//!
//! Split out of `entry.rs` to keep that module inside the repository's size
//! ratchet. Two callers ask these questions: the entry resolver (does the declared
//! entry name a module that exists?) and the scope walker (does a referenced path
//! name this face's module?).
//! 从 `entry.rs` 拆出，使那个模块留在仓库的尺寸棘轮之内。有两个调用方问这两个问题：入口
//! 解析器（声明的入口命名了存在的模块吗？）与作用域遍历（被引用的路径命名了这个面的模块
//! 吗？）。

use std::path::Path;

/// Whether an `application!(entry = …)` path names a module that exists under the
/// source root — `src/` for a conventional package, and the library target's own
/// directory for one whose target lives elsewhere.
/// `application!(entry = …)` 的路径是否命名了源码根下存在的模块——对约定布局的包就是 `src/`，
/// 而库目标住在别处的包则是目标自己所在的目录。
///
/// Every segment is resolved against the tree, not only the first. The spelling may
/// end in the function the host wants (`crate::build_method::app::run`) or in the module itself
/// (`crate::build_method::app`), and a segment in the middle that resolves to nothing is a wrong
/// path rather than a function name. Three layouts count as a module: the flat
/// `<name>.rs`, this workspace's canonical `<name>/<name>.rs`, and an ordinary
/// module tree's `<name>/mod.rs`; the first segment may also be a `src/bin` root or
/// the conventional `main`, which names the crate root's `lib.rs`.
/// 每一段都对目录树解析，而不是只看第一段。写法可能以宿主想要的函数结尾（`crate::build_method::app::run`），
/// 也可能以模块本身结尾（`crate::build_method::app`），而夹在中间解析不出任何东西的那一段是写错的路径，不是
/// 函数名。三种布局算模块：扁平的 `<name>.rs`、本工作区规范的 `<name>/<name>.rs`，以及普通模块
/// 树的 `<name>/mod.rs`；第一段还可以是 `src/bin` 的根，或约定写法 `main`（它命名 crate 根的
/// `lib.rs`）。
///
/// Looking at the first segment alone refused the documented
/// `application!(entry = crate::build_method::app::run)` whenever the module used the canonical
/// layout (`src/app/app.rs`, which is how this workspace mounts its own modules) and
/// accepted a bogus tail under a flat one.
/// 只看第一段会让文档里写的 `application!(entry = crate::build_method::app::run)` 在模块使用规范布局
/// （`src/app/app.rs`，也就是本工作区挂载自己模块的方式）时被拒，却接受扁平模块下一个不存在的
/// 尾巴。
pub(crate) fn entry_path_exists(src: &Path, path: &str) -> bool {
    let segments = path
        .strip_prefix("crate::")
        .unwrap_or_default()
        .split("::")
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return false;
    }

    let mut directory = src.to_path_buf();
    let mut resolved = 0usize;
    for (index, segment) in segments.iter().enumerate() {
        if index == 0 {
            if *segment == "main" && src.join("lib.rs").is_file() {
                resolved += 1;
                continue;
            }
            if src.join("bin").join(segment).with_extension("rs").is_file() {
                resolved += 1;
                directory = src.join("bin").join(segment);
                continue;
            }
        }
        let module = directory.join(segment);
        if module.with_extension("rs").is_file()
            || module.join(segment).with_extension("rs").is_file()
            || module.join("mod.rs").is_file()
        {
            resolved += 1;
            directory = module;
            continue;
        }
        break;
    }

    // Everything resolved, or everything but the trailing function name.
    // 全部解析成功，或者除末尾那个函数名之外全部解析成功。
    resolved >= 1 && resolved + 1 >= segments.len()
}

pub(crate) fn path_mentions_module(path: &str, module: &str) -> bool {
    let path = path
        .strip_prefix("crate::")
        .or_else(|| path.strip_prefix("xirang_kernel::"))
        .unwrap_or(path);
    path == module || path.starts_with(&format!("{module}::"))
}
