//! Host-entry resolution tests.
//! 宿主入口解析测试。
//!
//! A separate page so the module it tests stays inside the size ratchet: a test
//! module is excluded from it, and this one had grown as large as the code.
//! 独立一页，使被测模块留在尺寸棘轮之内：测试模块不受棘轮约束，而这一份已经长到
//! 与被测代码相当。

use std::path::PathBuf;

use super::{
    BuildDiagnostics, HostEntry, application_entry_source, default_entry_source,
    resolve_host_entry, resolve_host_entry_reporting,
};
use crate::build_method::entry_default::calls_host;

/// A throwaway package root for the resolution fixtures, unique per call.
/// 解析夹具使用的临时包根，每次调用唯一。
///
/// Unique because Cargo runs test binaries in parallel: a fixed name would let
/// one binary's cleanup delete another's fixture mid-test.
/// 唯一是必需的：Cargo 并行运行测试二进制，固定名字会让一个二进制的清理删掉另一个
/// 正在使用的夹具。
fn fixture(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-entry-{name}-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    std::fs::create_dir_all(root.join("src")).expect("fixture dir");
    root
}

#[test]
fn warm_validation_keeps_new_entries_and_malformed_ordinary_sources_visible() {
    let root = fixture("warm-entry-validation");
    let src = root.join("src");
    let file = src.join("lib.rs");
    std::fs::write(src.join("engine.rs"), "pub fn run() {}").unwrap();
    let entry = "application!(entry = crate::engine::run);";
    let ordinary = format!("//{}", " ".repeat(entry.len() - 2));
    std::fs::write(&file, &ordinary).unwrap();
    assert!(super::declaring_application_entry(&src).unwrap().is_none());
    let modified = file.metadata().unwrap().modified().unwrap();
    std::fs::write(&file, entry).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(file.metadata().unwrap().len() as usize, ordinary.len());
    assert_eq!(
        super::declaring_application_entry(&src).unwrap(),
        Some(file.clone())
    );
    std::fs::write(&file, &ordinary).unwrap();
    assert!(super::declaring_application_entry(&src).unwrap().is_none());
    let extra = src.join("extra.rs");
    std::fs::write(&extra, entry).unwrap();
    assert_eq!(
        super::declaring_application_entry(&src).unwrap(),
        Some(extra.clone())
    );
    std::fs::write(&extra, "pub fn run() { let x = @; }").unwrap();
    assert!(
        super::declaring_application_entry(&src)
            .unwrap_err()
            .contains("extra.rs")
    );
    std::fs::remove_file(extra).unwrap();
    assert!(super::declaring_application_entry(&src).unwrap().is_none());
    std::fs::remove_dir_all(root).unwrap();
}

/// The layout of one fixture root: these fixtures all keep their sources under
/// `src/`, so the scan root and the identity base are that directory.
/// 一个夹具根的布局：这些夹具都把源码放在 `src/` 下，因此遍历根与身份基准就是该目录。
fn layout(root: &std::path::Path) -> crate::build_method::SourceLayout {
    crate::build_method::source_layout(root).expect("the fixture layout resolves")
}

/// A file that only mentions `host!()` — in a line comment, a block comment,
/// or a string literal — has no host call. The kernel's lexical scanner masks
/// those regions, so a real call elsewhere still wins over Cargo's `main.rs`
/// preference.
/// 只在行注释、块注释或字符串字面量里提到 `host!()` 的文件没有宿主调用。内核词法
/// 扫描器屏蔽这些区域，因此别处的真实调用仍然胜过 Cargo 对 `main.rs` 的偏好。
#[test]
fn a_host_mention_in_a_comment_or_string_is_not_a_call() {
    let root = fixture("host-mention");
    let src = root.join("src");
    std::fs::write(
        src.join("main.rs"),
        "// host!() in a line comment\n\
             /* host!() in a block comment */\n\
             fn main_entry_tests() { let _ = \"host!()\"; }\n",
    )
    .expect("prose-only main");
    assert!(
        !calls_host(&src.join("main.rs")),
        "a comment or string literal is not a call"
    );
    std::fs::write(src.join("lib.rs"), "crate::host!();\n").expect("host lib");
    assert!(
        calls_host(&src.join("lib.rs")),
        "a real invocation is a call"
    );
    assert_eq!(
        default_entry_source(&layout(&root)),
        src.join("lib.rs"),
        "the file that really calls host!() is the entry"
    );

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A lib+bin host keeps `main.rs` as a stub and calls `host!()` in `lib.rs`.
/// The declared graft plan lives at that call, so the entry must follow it
/// instead of Cargo's `main.rs` preference.
/// 库+二进制宿主把 `main.rs` 留作空壳、在 `lib.rs` 里调用 `host!()`。声明的 graft
/// 计划就在那次调用处，因此入口必须跟随它，而不是 Cargo 对 `main.rs` 的偏好。
#[test]
fn the_entry_is_the_file_that_calls_host() {
    let root = fixture("calls-host");
    let source = root.join("src");
    std::fs::write(source.join("main.rs"), "fn main() {}\n").expect("stub main");
    std::fs::write(
        source.join("lib.rs"),
        "//! docs mentioning host!() in prose\ncrate::host!();\n",
    )
    .expect("host lib");
    assert_eq!(default_entry_source(&layout(&root)), source.join("lib.rs"));

    std::fs::write(source.join("main.rs"), "crate::host!();\nfn main() {}\n").expect("host main");
    assert_eq!(default_entry_source(&layout(&root)), source.join("main.rs"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A configured entry resolves against the package root, and an absolute one
/// is used as written. Both spellings must be able to name the same file,
/// because the documentation presents them as equivalent.
/// 被指定的入口相对包根解析，绝对路径则原样使用。两种写法都必须能指到同一个文件，
/// 因为文档把二者写成等价。
#[test]
fn a_configured_entry_resolves_against_the_package_root() {
    let root = fixture("configured");
    let src = root.join("src");
    std::fs::write(src.join("lib.rs"), "crate::host!();\n").expect("host entry");
    // The configured entry lives in the registry layout discovery accepts:
    // a bare `.rs` next to `src/` is rejected, so a variable can only name a
    // registration source or `lib.rs`/`main.rs`.
    // 被指定的入口位于发现机制接受的注册布局中：`src/` 旁的裸 `.rs` 会被拒绝，
    // 因此变量只能指向一个注册源文件或 `lib.rs`/`main.rs`。
    std::fs::create_dir_all(src.join("preview")).expect("fixture dir");
    std::fs::write(src.join("preview/preview.rs"), "fn preview() {}\n").expect("configured entry");
    let nodes = crate::build_method::source_walk::discover_root(&src);

    let absolute = resolve_host_entry(&src, &nodes, Some(src.join("preview/preview.rs")));
    assert!(matches!(absolute, HostEntry::Configured(_)), "{absolute:?}");
    assert_eq!(absolute.path(), src.join("preview/preview.rs"));

    let relative = resolve_host_entry(&src, &nodes, Some(PathBuf::from("src/preview/preview.rs")));
    assert_eq!(relative.path(), src.join("preview/preview.rs"));
    // A host that named the file owns the absence rule: this entry is required.
    // 自己指定了文件的宿主承担“可否缺失”的规则：这个入口是必需的。
    assert!(relative.is_required());

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A configured entry that names something which is not a file fails the
/// build instead of letting one reader fall back while the other follows the
/// variable. The obvious alternative — return the convention entry — is the
/// bug: pruning would follow the variable while the cut table described
/// `main.rs`.
/// 被指定却指不到文件的入口让构建失败，而不是让一个读取者回退、另一个跟随变量。
/// 显而易见的替代做法——返回约定入口——正是那个 bug：剪枝跟随变量，切口表却描述
/// `main.rs`。
///
/// It fails through a diagnostic rather than a panic. The build still refuses
/// to continue — the rendered diagnostic is what stops it — but the failure
/// now reaches `check --json` as a document, and the rest of the run's
/// diagnostics survive to be reported with it.
/// 它通过诊断而不是 panic 失败。构建仍然拒绝继续——渲染出的诊断正是拦住它的东西——
/// 但这次失败现在以文档形式到达 `check --json`，同一次运行的其余诊断也能活到被一起
/// 报出来。
#[test]
fn a_configured_entry_that_is_not_a_file_is_a_diagnostic() {
    let root = fixture("configured-missing");
    let src = root.join("src");
    std::fs::write(src.join("lib.rs"), "crate::host!();\n").expect("host entry");
    let nodes = crate::build_method::source_walk::discover_root(&src);

    let mut errors = BuildDiagnostics::default();
    let resolved = resolve_host_entry_reporting(
        &layout(&root),
        &nodes,
        Some(PathBuf::from("src/nope.rs")),
        &mut errors,
    );
    assert!(
        matches!(resolved, HostEntry::Convention(_)),
        "the pipeline keeps a usable entry so the remaining diagnostics still flow"
    );
    assert_eq!(errors.len(), 1, "{:#?}", errors.iter().collect::<Vec<_>>());
    let diagnostic = errors.iter().next().expect("one diagnostic");
    assert_eq!(diagnostic.phase, "entry");
    assert!(
        diagnostic.message.contains("XIRANG_ENTRY names"),
        "{}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains("nope.rs"),
        "the message names the path: {}",
        diagnostic.message
    );

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A malformed `application!` declaration is a diagnostic too, and the
/// resolution falls back instead of aborting: the same run then reports every
/// other problem in the host.
/// 畸形的 `application!` 声明同样是诊断，解析改为回退而不是中止：同一次运行随后会报出
/// 宿主里的其他每个问题。
#[test]
fn a_malformed_application_declaration_is_a_diagnostic() {
    let root = fixture("application-malformed");
    let src = root.join("src");
    std::fs::create_dir_all(src.join("host")).expect("face folder");
    std::fs::write(src.join("host/host.rs"), "crate::application!(entry = );\n")
        .expect("malformed declaration");
    let nodes = crate::build_method::source_walk::discover_root(&src);

    let mut errors = BuildDiagnostics::default();
    let resolved = application_entry_source(&src, &nodes, &mut errors);
    assert_eq!(resolved, None, "a malformed declaration names no entry");
    assert!(
        errors.iter().any(|diagnostic| {
            diagnostic.phase == "entry" && diagnostic.message.contains("application!")
        }),
        "{:#?}",
        errors.iter().collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// `application!(entry = …)` resolves every segment against the package tree, and
/// the canonical `dir/dir.rs` layout is one of the spellings it must accept.
/// `application!(entry = …)` 的每一段都对包内目录树解析，而规范的 `dir/dir.rs` 布局是它必须
/// 接受的一种写法。
///
/// It used to look at the first segment alone, so the documented
/// `application!(entry = crate::build_method::app::run)` was refused for `src/app/app.rs` — the
/// layout this workspace's own modules use — while a bogus tail under a flat module
/// passed. Both directions are pinned here.
/// 它过去只看第一段，于是文档里写的 `application!(entry = crate::build_method::app::run)` 在
/// `src/app/app.rs`——本工作区自己的模块用的布局——下被拒，而扁平模块下一个不存在的尾巴却
/// 通过。两个方向都在这里钉住。
#[test]
fn an_application_entry_resolves_every_segment_against_the_tree() {
    let root = fixture("application-entry");
    let src = root.join("src");
    std::fs::create_dir_all(src.join("app")).expect("app module dir");
    std::fs::write(src.join("app/app.rs"), "pub fn run() {}\n").expect("app module");
    std::fs::write(src.join("flat.rs"), "pub fn run() {}\n").expect("flat module");
    std::fs::create_dir_all(src.join("bin")).expect("bin dir");
    std::fs::write(src.join("bin/tool.rs"), "fn main() {}\n").expect("bin root");
    std::fs::write(src.join("lib.rs"), "// host\n").expect("conventional entry");

    // The canonical `<dir>/<dir>.rs` module layout, with and without the function.
    // 规范的 `<dir>/<dir>.rs` 模块布局，带函数名与不带各一次。
    assert!(super::entry_path_exists(&src, "crate::app"));
    assert!(super::entry_path_exists(&src, "crate::app::run"));
    // A flat module, a `src/bin` root, and the conventional `main` → `lib.rs`.
    // 扁平模块、`src/bin` 根，以及约定的 `main` → `lib.rs`。
    assert!(super::entry_path_exists(&src, "crate::flat::run"));
    assert!(super::entry_path_exists(&src, "crate::tool"));
    assert!(super::entry_path_exists(&src, "crate::main"));

    // A segment that does not resolve may only be the trailing name: `bogus` in
    // the middle is a wrong path, not a function.
    // 解析不了的段只能是末段的那个名字：夹在中间的 `bogus` 是写错的路径，不是函数。
    assert!(!super::entry_path_exists(&src, "crate::app::bogus::run"));
    assert!(!super::entry_path_exists(&src, "crate::absent::run"));
    assert!(!super::entry_path_exists(&src, "crate::"));
    let _ = std::fs::remove_dir_all(&root);
}
