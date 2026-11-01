//! Reading the external implementation a record's declaration names.
//! 读取记录所声明的外部实现。
//!
//! Split out of `apply_promote.rs` when that file reached the 600-code-line ratchet — the readers
//! below are one concern (find the crate, read its face, spell its fields), and the landing half
//! is another.
//! 当 `apply_promote.rs` 触到 600 代码行的棘轮时拆出来——下面这些读取器是一个关注点（找到 crate、
//! 读它的面、拼出它的字段），而落地那一半是另一个。

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::run_method::AuthoringContext;

/// The crate and module a typed declaration names its implementation with.
/// 类型化声明用来命名其实现的 crate 路径。
///
/// `Some("control_button_graft")` for `control_button_graft::button_fast::NODE_ID`. The first
/// segment is the crate, and the rest is the module inside it — the two halves the host manifest
/// and the external source tree are looked up by.
/// `control_button_graft::button_fast::NODE_ID` 得到 `Some("control_button_graft")`。第一段是 crate，
/// 其余是它内部的模块——宿主清单与外部源码树正是按这两半查找的。
pub(super) fn crate_and_module(
    expressions: &crate::build_method::DeclaredGraftExpressions,
) -> Result<(String, String), String> {
    let text = expressions.graft.trim();
    let crate_name = text
        .split("::")
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty() && *name != "crate" && *name != "self")
        .ok_or_else(|| {
            format!(
                "the declaration names `{text}`, which does not start with the crate that carries \
                 the implementation; pass `implementation: \"<path to the external crate>\"`"
            )
        })?;
    let rest = text.split("::").skip(1).collect::<Vec<_>>().join("::");
    let module = rest
        .strip_suffix("::NODE_ID")
        .unwrap_or(&rest)
        .trim()
        .to_owned();
    if module.is_empty() {
        return Err(format!(
            "the declaration names `{text}`, which does not name a module inside `{crate_name}`; \
             the typed form ends with `<crate>::<module>::NODE_ID`"
        ));
    }
    Ok((crate_name.to_owned(), module))
}

/// The file one module path names inside an external crate.
/// 一个模块路径在外部 crate 里指的那个文件。
///
/// Three layouts, and the middle one is the one this repository's own generator writes: a face whose
/// module is `panel::object::frame` lives at `src/panel/object/frame/frame.rs` — its own directory,
/// named after itself — exactly as the scaffold and `apply add` lay faces out, and exactly as the
/// generated tree mounts them. Only the flat form was searched before, so `promote` could not land an
/// implementation for any face this tool itself creates: it refused, about a file that was right
/// there one directory deeper, with "has neither … nor …" (measured against a scaffolded host,
/// audit `M7`, §M7.39).
/// 三种版式，中间那种正是本仓库自己的生成器写出来的：模块为 `panel::object::frame` 的注册面住在
/// `src/panel/object/frame/frame.rs`——以自己的名字命名的目录里——脚手架与 `apply add` 就是这样摆放注册面
/// 的，生成树也这样挂载它们。此前只找平坦版式，因此 `promote` 对本工具自己创建的**任何**注册面都无法落地：
/// 它对着一个就在那里、只是深了一层的文件说"既没有……也没有……"（对照脚手架宿主实测，审计 `M7`，§M7.39）。
pub(super) fn external_file(crate_root: &Path, module: &str) -> Result<PathBuf, String> {
    let relative = module.replace("::", "/");
    let leaf = module.rsplit("::").next().unwrap_or(module);
    let source = crate_root.join("src");
    for candidate in [
        // The flat form stays first: a crate that carries both spellings keeps the answer it gave
        // before this candidate list grew.
        // 平坦版式仍排在前面：同时带两种拼写的 crate，得到的答案与本清单变长之前一致。
        source.join(format!("{relative}.rs")),
        source.join(&relative).join(format!("{leaf}.rs")),
        source.join(&relative).join("mod.rs"),
    ] {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "the declaration names the module `{module}`, and {} has none of `src/{relative}.rs`, \
         `src/{relative}/{leaf}.rs` or `src/{relative}/mod.rs`; this action lands **file-per-face** \
         implementations",
        crate_root.display()
    ))
}

/// Where the external implementation's crate lives.
/// 外部实现的 crate 住在哪。
///
/// `implementation:` when the caller gives it; otherwise the host manifest's own path dependency
/// whose package name matches the declaration's crate segment. A git or registry dependency is
/// refused rather than searched for: its source is not in this checkout, and a path this action
/// guessed would be a path it cannot read.
/// 调用方给了就用 `implementation:`；否则用宿主清单里包名与声明的 crate 段相符的那条 path 依赖。
/// git 或 registry 依赖会被拒绝而不是去搜：它的源码不在本检出里，而猜出来的路径是它读不到的路径。
pub(super) fn external_root(
    root: &Path,
    arguments: &Value,
    crate_name: &str,
) -> Result<PathBuf, String> {
    if let Some(given) = arguments.get("implementation").and_then(Value::as_str) {
        let given = given.trim();
        if given.is_empty() {
            return Err("`implementation` must not be empty".to_owned());
        }
        let path = PathBuf::from(given);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if !path.is_dir() {
            return Err(format!(
                "`implementation` names {}, which is not a directory",
                path.display()
            ));
        }
        return Ok(path);
    }
    let cargo = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&cargo)
        .map_err(|error| format!("cannot read {}: {error}", cargo.display()))?;
    let wanted = crate_name.replace('-', "_");
    for line in text.lines() {
        let line = line.trim();
        // Only inline tables are considered: `key = { … }` is how this workspace writes a
        // dependency it also carries a `path` for, and a bare `key = "1.0"` has no source to read.
        // 只看行内表：`key = { … }` 是本工作区写"同时带 path 的依赖"的方式，而裸的
        // `key = "1.0"` 没有源码可读。
        let Some((key, rest)) = line.split_once('=') else {
            continue;
        };
        let rest = rest.trim();
        if !rest.starts_with('{') {
            continue;
        }
        let key = key.trim().trim_matches('"').replace('-', "_");
        let package = quoted(rest, "package").unwrap_or_else(|| key.clone());
        let Some(path) = quoted(rest, "path") else {
            if package.replace('-', "_") == wanted {
                return Err(format!(
                    "`{crate_name}` is a dependency of this package but not a path dependency, so \
                     its source is not in this checkout; pass `implementation` with the external \
                     crate's path"
                ));
            }
            continue;
        };
        let resolved = root.join(&path);
        if !resolved.is_dir() {
            return Err(format!(
                "`{package}` is declared at {}, which is not a directory",
                resolved.display()
            ));
        }
        // The typed declaration names the **library**, and a package is free to name its library
        // something else — the shipped example does exactly that
        // (`xirang-example-control-button-graft` with `[lib] name = "control_button_graft"`). So
        // the crate segment is matched against the package name and against the library the
        // dependency's own manifest declares.
        // 类型化声明点名的是**库**，而一个包可以给库起别的名字——出厂示例正是这样。因此 crate 段同时与
        // 包名、以及该依赖自己清单里声明的库名比对。
        if package.replace('-', "_") == wanted || library_name(&resolved) == wanted {
            return Ok(resolved);
        }
    }
    Err(format!(
        "no dependency of this package is named `{crate_name}`, so this action cannot find the \
         external implementation's source; pass `implementation` with the external crate's path"
    ))
}

/// The quoted value of `key` inside one inline TOML table.
/// 一个行内 TOML 表里 `key` 的带引号取值。
///
/// Hand-rolled rather than parsed: this reads one dependency line out of a manifest this action
/// only needs a `path` from, and a full TOML parser here would be a second manifest reader beside
/// the build's own.
/// 手写而不是引入解析器：这里只从一个清单的某一条依赖行里取一个 `path`，而在旁边放一个完整的 TOML
/// 解析器，等于在构建期自己的清单读取器之外再造一个。
fn quoted(table: &str, key: &str) -> Option<String> {
    let at = table
        .find(&format!("{key} "))
        .or_else(|| table.find(&format!("{key}=")))?;
    let after = &table[at + key.len()..];
    let open = after.find('"')?;
    let rest = &after[open + 1..];
    let close = rest.find('"')?;
    Some(rest[..close].to_owned())
}

/// The library a crate declares, which is what a Rust path names.
/// 一个 crate 声明的库名——Rust 路径点名的就是它。
///
/// `[lib] name` when the manifest has one, otherwise the package name with dashes turned into
/// underscores, which is what cargo defaults to.
/// 清单里有 `[lib] name` 就用它，否则用包名把短横线换成下划线——cargo 的默认值。
fn library_name(crate_root: &Path) -> String {
    let manifest = crate_root.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return String::new();
    };
    let mut in_lib = false;
    let mut package = String::new();
    let mut library = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_lib = line == "[lib]";
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_owned();
        match key.trim() {
            "name" if in_lib => library = value,
            "name" => package = value,
            _ => {}
        }
    }
    if library.is_empty() {
        package.replace('-', "_")
    } else {
        library.replace('-', "_")
    }
}

/// A registry built from the external crate's own source.
/// 从外部 crate 自己的源码建出的注册机。
///
/// It exists for **one** field: `flow`. The declaration's `flow:` carries a Rust *expression*
/// (`FlowContract::new(ContractId::new("x"), 1, "In", "Out")`) while the authoring patch takes the
/// compact `id|version|input|output` spelling, and the kernel has a renderer for the compact form
/// but no reader that goes the other way. The typed snapshot is where the four parts already are,
/// so this reads them from there instead of writing a second parser here.
/// 它是为**一个**字段存在的：`flow`。声明里的 `flow:` 带的是一个 Rust *表达式*
/// （`FlowContract::new(ContractId::new("x"), 1, "In", "Out")`），而创作 patch 收的是紧凑的
/// `id|version|input|output` 拼法；内核把紧凑形式渲染成表达式的方向有，反方向没有读取器。四个部分本来
/// 就在类型化快照里，因此从这里读，而不是在这里再写一个解析器。
pub(super) fn external_registry(root: &Path) -> Result<xirang_kernel::Registry, String> {
    let namespace = crate::mcp::registry::namespace(root)?;
    let mut registry = xirang_kernel::Registry::root_for_namespace(
        xirang_kernel::FrameworkId::new("xirang.mcp"),
        namespace.clone(),
    );
    let layout = crate::build_method::source_layout(root)?;
    let snapshots = AuthoringContext::new(root.to_path_buf(), namespace)
        .scope(|| crate::run_method::generated_snapshots_in(&layout))
        .map_err(|error| format!("the external implementation's source is unreadable: {error}"))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("the external implementation's faces were rejected: {error}"))?;
    Ok(registry)
}

/// The external declaration, as the strings a host declaration is written from.
/// 外部那份声明，写成宿主声明所需的那些字符串。
///
/// Read through the kernel's own face parser (`FaceSyntax::field`), which is what the compile-time
/// macro hands the kernel — so a hand-written `external_object!` and a generated `*_object!` are
/// read by one reader rather than by a scraper written here.
/// 经内核自己的面解析器（`FaceSyntax::field`）读取——正是编译期宏交给内核的那一份，因此手写的
/// `external_object!` 与生成的 `*_object!` 由同一个读取器读出，而不是在这里另写一个刮取器。
#[derive(Debug)]
pub(super) struct External {
    pub(super) kind: String,
    pub(super) preset: String,
    pub(super) parts: String,
    pub(super) name_zh: String,
    pub(super) name_en: String,
    pub(super) summary_zh: String,
    pub(super) summary_en: String,
    pub(super) exports: String,
    pub(super) stable_name: String,
    pub(super) needs_registry: bool,
    pub(super) handle_traits: String,
    pub(super) handle_contracts: String,
    pub(super) part_traits: String,
    pub(super) part_contracts: String,
    pub(super) requires: String,
    pub(super) provides: String,
    pub(super) runtime_checks: String,
    pub(super) flow: String,
    pub(super) flow_provider: String,
}

impl External {
    pub(super) fn from_syntax(
        face: &xirang_kernel::syntax::FaceSyntax,
        file: &Path,
        flow: String,
    ) -> Result<Self, String> {
        let text = |name: &str| face.field(name).unwrap_or_default();
        let list = |name: &str| {
            face.string_list(name)
                .map(|items| items.join(","))
                .unwrap_or_default()
        };
        // Two fields name a path *inside the tree that declared them*, and the external tree's
        // spelling of them points at files the host does not have. Refusing by name beats copying a
        // path that compiles nowhere.
        // 有两个字段命名的是*声明它们的那棵树内部*的路径，而外部树的拼法指的文件宿主没有。按名拒绝胜过
        // 复制一条在哪里都编译不过的路径。
        let admission = text("admission");
        if !admission.is_empty() {
            return Err(format!(
                "the external declaration sets `admission: {admission}`, which names paths inside \
                 *that* crate; this action will not copy a path that does not exist here. Land it \
                 by hand, or declare the admission on the host side first"
            ));
        }
        for name in ["handle_contracts", "part_contracts"] {
            let value = text(name);
            if value.contains("crate::") {
                return Err(format!(
                    "the external declaration's `{name}` is `{value}`, a path inside that crate; \
                     this action will not copy a path that does not resolve here"
                ));
            }
        }
        // The registration rule is the one field whose spelling is host-relative: a generated
        // declaration writes `crate::RegistrationRule::ANY`, while an external one writes
        // `RegistrationRule::ANY` because that is what it imports. Anything richer than `ANY`
        // names a rule module inside the external crate, which the host does not have.
        // 注册规则是唯一一个拼法属于宿主的字段：生成的声明写 `crate::RegistrationRule::ANY`，而外部的
        // 写 `RegistrationRule::ANY`，因为那是它导入的名字。比 `ANY` 更复杂的取值命名的是外部 crate 内部
        // 的规则模块，而宿主没有那个模块。
        let rule = text("registry_rule");
        // The *patch* takes the compact clause spelling (`ANY`, or `preset:…; exactly:…`), not a
        // Rust path — `render.rs` is what turns an empty rule into `crate::RegistrationRule::ANY`
        // in the file. So the host-agnostic value here is the compact `ANY`.
        // *patch* 收的是紧凑的子句拼法（`ANY`，或 `preset:…; exactly:…`），不是 Rust 路径——把空规则写成
        // 文件里的 `crate::RegistrationRule::ANY` 的是 `render.rs`。因此这里宿主无关的取值是紧凑的 `ANY`。
        // Checked, not carried: the landing keeps the **host's** `needs_registry` and rule path, because
        // both describe the position the face occupies in the host's tree rather than the crate its fields
        // came from (see `overlay`). What this still has to refuse is a rule too rich for any host to own:
        // it names a module inside the external crate, and no tree here mounts it.
        // 只校验，不携带：落地保留**宿主的** `needs_registry` 与规则路径，因为两者描述的是这个面在宿主那棵树里
        // 占据的位置，而不是它字段的来源 crate（见 `overlay`）。这里仍必须拒绝的是"任何宿主都拥有不了的"富规则：
        // 它点名外部 crate 内部的一个模块，而这里的任何树都没有挂载它。
        if !(rule.is_empty() || rule.ends_with("::ANY") || rule == "ANY") {
            return Err(format!(
                "the external declaration's `registry_rule` is `{rule}`, which names a rule module \
                 inside that crate; this action lands declarations whose rule is `ANY`, and a \
                 richer rule has to be declared on the host side first"
            ));
        }
        let _ = file;
        Ok(Self {
            kind: text("kind"),
            preset: text("preset"),
            parts: text("parts"),
            name_zh: face.localized("name", "zh").unwrap_or_default(),
            name_en: face.localized("name", "en").unwrap_or_default(),
            summary_zh: face.localized("summary", "zh").unwrap_or_default(),
            summary_en: face.localized("summary", "en").unwrap_or_default(),
            exports: list("exports"),
            stable_name: face.string("stable_name").unwrap_or_default(),
            needs_registry: face.boolean("needs_registry").unwrap_or(false),
            handle_traits: list("handle_traits"),
            handle_contracts: text("handle_contracts").trim_matches(['[', ']']).to_owned(),
            part_traits: list("part_traits"),
            part_contracts: text("part_contracts").trim_matches(['[', ']']).to_owned(),
            requires: list("requires"),
            provides: list("provides"),
            runtime_checks: text("runtime_checks").trim_matches(['[', ']']).to_owned(),
            flow,
            flow_provider: text("flow_provider"),
        })
    }
}
