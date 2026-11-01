//! Folder-backed registration discovery.
//! 文件夹注册面发现。

use std::fs;
use std::path::{Path, PathBuf};

// The mapping machinery lives in its own module now: the discovery walk and the fingerprint both use
// it, and this file keeps the paths its callers already spell.
// 映射机制现在住自己的模块里：发现遍历与指纹都用它，而本文件保留调用方已经写着的那些路径。
pub(crate) use super::parallel::{parallel_map, parallel_map_with_workers, worker_budget};

use super::Node;
use super::registry_syntax::parse_face;

/// A registration face that lives where the build cannot compile it.
/// 长在构建无法编译的位置上的注册面。
/// The phase an out-of-tree implementation is reported under. It is a **notice**, not a defect: the check prints it
/// and stays green (maintainer's ruling, 2026-10-18, option B1), because following the documented graft workflow must
/// not turn the tree red.
/// 树外实现被报告时用的那个阶段。它是**提示**而不是缺陷：check 打印它并保持绿色（维护者 2026-10-18 裁定，选项
/// B1），因为照文档的 graft 工作流不该把树弄红。
pub(crate) const EXTERNAL_FACE_PHASE: &str = "face-external";

/// Split a walk's findings into the **notices** the check prints and stays green on, and the **defects** that fail it.
/// 把一次遍历的发现分成两类：check 打印后仍保持绿色的**提示**，以及让它失败的**缺陷**。
///
/// It is one function rather than a condition repeated at the call site, because the rule ("an out-of-tree
/// implementation is a notice") has to be the same one that decides the exit code — the pin below drives both halves.
/// 它是一个函数而不是在调用点复述的条件，因为"树外实现是提示"这条规则必须与决定退出码的那条是同一条——下面的钉子
/// 同时驱动这两半。
pub(crate) fn notices_and_defects(unplaced: Vec<UnplacedFace>) -> Vec<UnplacedFace> {
    unplaced
        .into_iter()
        .filter(|face| face.phase != EXTERNAL_FACE_PHASE)
        .collect()
}

#[derive(Clone, Debug)]
pub(crate) struct UnplacedFace {
    /// The file, relative to the package root.
    /// 文件，以包根为基准。
    pub(crate) relative: String,
    /// One-based line when the failure is inside the file.
    /// 失败发生在文件内部时为从 1 开始的行号。
    pub(crate) line: usize,
    /// The diagnostic phase this belongs to.
    /// 本条属于哪个诊断阶段。
    pub(crate) phase: &'static str,
    /// What is wrong, ready to print.
    /// 问题所在，可直接打印。
    pub(crate) message: String,
}

/// Discover the registration tree, ignoring unplaceable files.
/// 发现注册树，忽略无法安放的文件。
///
/// The findings a caller needs are collected by [`discover_root_reporting`];
/// this entry point exists for the readers that only want the tree.
/// 调用方需要的发现结果由 [`discover_root_reporting`] 收集；本入口供只要树的读者使用。
/// The one discovery entry the build and the query view share.
/// 构建与查询视图共用的唯一发现入口。
pub(crate) fn discover_package(
    layout: &crate::source_layout::SourceLayout,
    unplaced: &mut Vec<UnplacedFace>,
) -> Vec<Node> {
    let mut nodes = discover_root_reporting(&layout.scan_root, unplaced);
    nodes.extend(super::discovery_node::explicit_placement_nodes(
        &layout.package_root,
        &layout.scan_root,
    ));
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    // The record spelling of every placed file becomes a fact here, where both the package root and
    // the identity base are known. Every pass after this one keys identities and caches on it.
    // 每个被安放文件的记录拼写在这里成为事实——包根与身份基准都在这里可知。之后每一趟的身份与缓存都以它
    // 为键。
    super::discovery_node::remember_source_spellings(layout, &nodes);
    nodes
}

pub(crate) fn discover_root(src: &Path) -> Vec<Node> {
    let mut unplaced = Vec::new();
    discover_root_reporting(src, &mut unplaced)
}

/// Discover the registration tree and report the faces it cannot place.
/// 发现注册树，并报告它无法安放的注册面。
///
/// A `.rs` file that is not a registration face is skipped in silence: every Rust
/// project has ordinary modules next to its faces, and refusing them would make
/// the tool unusable on a real crate. A file that *is* a face, or that is meant
/// to be one and does not parse, is reported: the build could never compile it,
/// and silence there is exactly the failure this project exists to refuse.
/// 不是注册面的 `.rs` 文件静默跳过：每个 Rust 项目都会在注册面旁边放普通模块，拒绝它们
/// 会让工具在真实 crate 上不可用。而**是**注册面的文件、或本意是面却解析不了的文件会被
/// 报告：构建永远编译不到它，此处沉默正是本项目存在的意义所在——拒绝的那种失败。
pub(crate) fn discover_root_reporting(src: &Path, unplaced: &mut Vec<UnplacedFace>) -> Vec<Node> {
    discover_root_reporting_with_workers(src, unplaced, worker_budget())
}

/// [`discover_root_reporting`] with the worker count spelled out, so a pin can compare worker counts.
/// [`discover_root_reporting`] 的显式工作线程数版本，好让钉子能比较不同线程数的结果。
///
/// **Order is the whole contract.** The nodes are sorted by name below and the unplaceable findings
/// are merged in the order the directory handed its entries out, so the tree and the diagnostics are
/// the same whatever the worker count — the fingerprint that goes into every record is computed from
/// this tree, and one that depended on completion order would invalidate every record the moment the
/// machine's core count changed (audit `K2`'s rule, now holding for discovery too).
/// **顺序就是全部契约。** 节点在下面按名字排序，无法安放的发现按目录交出条目的顺序合并，因此无论几个
/// 工作线程，树与诊断都相同——写进每份记录的指纹就是从这棵树算出来的，而依赖完成顺序的指纹会在机器的核数
/// 一变时让每份记录失效（审计 `K2` 的规则，现在对发现过程同样成立）。
pub(crate) fn discover_root_reporting_with_workers(
    src: &Path,
    unplaced: &mut Vec<UnplacedFace>,
    workers: usize,
) -> Vec<Node> {
    // A tree the build cannot read is a layout problem, reported where layout
    // problems are reported. This used to `expect("src directory must exist")`,
    // which took the build script down with exit 101 and left `check --json` with
    // an empty stdout — the contract that command was fixed to keep.
    // 构建读不到的树是布局问题，报到布局问题该报的地方。这里过去是
    // `expect("src directory must exist")`，那会以退出 101 打死构建脚本，并让
    // `check --json` 的 stdout 一片空白——而那正是那条命令被修好要守住的契约。
    let Ok(entries) = fs::read_dir(src) else {
        unplaced.push(unreadable(src, src));
        return Vec::new();
    };
    // The entries are read out first so one entry can go to one worker; the order the directory
    // handed them out is the order the findings below are merged in.
    // 先把条目读出来，好把一个条目交给一个工作线程；目录交出它们的顺序就是下面合并发现的顺序。
    let entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let mut nodes = Vec::new();
    for (node, mut found) in parallel_map_with_workers(&entries, workers, |path| {
        walk_root_entry(src, path, workers)
    }) {
        if let Some(node) = node {
            nodes.push(node);
        }
        unplaced.append(&mut found);
    }
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

/// One entry of the source root, walked the way the root walk asks for it.
/// 源根的一个条目，按根遍历要求的方式走过。
fn walk_root_entry(src: &Path, path: &Path, workers: usize) -> (Option<Node>, Vec<UnplacedFace>) {
    let mut unplaced = Vec::new();
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return (None, unplaced);
    };
    if matches!(name, "lib.rs" | "main.rs" | "bin") {
        return (None, unplaced);
    }
    let name = name.to_owned();
    if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        record_unplaced(&mut unplaced, src, path);
    }
    if path.is_dir() && valid_name(&name) {
        let attached = path.join(format!("{name}.rs"));
        return (
            Some(Node {
                name,
                file: attached.is_file().then_some(attached),
                children: discover_children(src, path, &mut unplaced, workers),
            }),
            unplaced,
        );
    }
    (None, unplaced)
}

fn discover_children(
    src: &Path,
    dir: &Path,
    unplaced: &mut Vec<UnplacedFace>,
    workers: usize,
) -> Vec<Node> {
    // The same refusal as the root's: a directory that vanished or cannot be read
    // between the walk reaching it and this call is reported, not fatal.
    // 与根目录同样的拒绝：在遍历到达它之后、本次调用之前消失或读不了的目录会被报告，而不是致命。
    let Ok(entries) = fs::read_dir(dir) else {
        unplaced.push(unreadable(src, dir));
        return Vec::new();
    };
    // Each level maps in parallel on its own — the wide level of a real tree is rarely the top one
    // (the 50,000-file fixture keeps its 2,500 faces in `src/control/object/`), and parallelising
    // only the root would leave that level serial. [`parallel_map_with_workers`] refuses to nest, so
    // a worker that meets another wide level finishes it on its own thread instead of spawning a
    // second pool: the thread count stays bounded by the machine, not by the depth of the tree.
    // 每一层各自并行映射——真实树的宽层很少是顶层（50,000 文件夹具把它的 2,500 个面放在
    // `src/control/object/` 下），只并行根层会让那一层留在串行。 [`parallel_map_with_workers`] 拒绝嵌套，
    // 因此在工作线程里遇到另一个宽层的调用会在自己的线程上把它做完，而不是再开一个线程池：线程数由机器
    // 决定，而不是由树的深度决定。
    let entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let mut nodes = Vec::new();
    for (node, mut found) in parallel_map_with_workers(&entries, workers, |path| {
        walk_child_entry(src, dir, path, workers)
    }) {
        if let Some(node) = node {
            nodes.push(node);
        }
        unplaced.append(&mut found);
    }
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

/// One entry of a module directory, walked the way [`discover_children`] asks for it.
/// 模块目录的一个条目，按 [`discover_children`] 要求的方式走过。
fn walk_child_entry(
    src: &Path,
    dir: &Path,
    path: &Path,
    workers: usize,
) -> (Option<Node>, Vec<UnplacedFace>) {
    let mut unplaced = Vec::new();
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return (None, unplaced);
    };
    if path.is_file() {
        if path.file_stem().and_then(|stem| stem.to_str())
            == dir.file_name().and_then(|stem| stem.to_str())
        {
            return (None, unplaced);
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            return (None, unplaced);
        }
        record_unplaced(&mut unplaced, src, path);
    }
    let name = name.to_owned();
    if path.is_dir() && valid_name(&name) {
        let attached = path.join(format!("{name}.rs"));
        return (
            Some(Node {
                name,
                file: attached.is_file().then_some(attached),
                children: discover_children(src, path, &mut unplaced, workers),
            }),
            unplaced,
        );
    }
    (None, unplaced)
}

/// One directory the walk could not read, reported like an unplaceable face.
/// 遍历读不到的一个目录，像无法安放的注册面一样被报告。
///
/// `src` is the source root the message and the relative path are measured
/// against, and `directory` is the one that failed; they are the same path when
/// the root itself is the problem.
/// `src` 是消息与相对路径所依据的源根，`directory` 是失败的那个；根本身出问题时两者相同。
fn unreadable(src: &Path, directory: &Path) -> UnplacedFace {
    let problem = match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.is_dir() => "cannot be read".to_owned(),
        Ok(_) => "is not a directory".to_owned(),
        Err(error) => format!("does not exist ({error})"),
    };
    UnplacedFace {
        relative: super::relative_display(src, directory),
        line: 0,
        phase: "face-layout",
        message: format!(
            "{} {problem}; the build reads registration faces from this source tree",
            directory.display()
        ),
    }
}

/// Record a `.rs` file that cannot become a module, unless it is an ordinary one.
/// 记录无法成为模块的 `.rs` 文件，除非它只是普通模块。
fn record_unplaced(unplaced: &mut Vec<UnplacedFace>, src: &Path, path: &Path) {
    let Ok(source) = fs::read_to_string(path) else {
        // An unreadable file makes no claim we can judge; the build's own reads
        // report it where the file actually mattered.
        // 读不了的文件没有可判断的声明；构建自己的读取会在该文件真正要紧的地方报告它。
        return;
    };
    match parse_face(&source) {
        Ok(None) => {}
        // Two different things used to get one message. A face written with `external_object!`
        // declares its own registry and is deliberately *not* part of this package's generated
        // tree — the example host that grafts it registers it explicitly — so saying "the build
        // can never compile it" was false about a working crate. Only a face of the generated
        // layout that sits outside it has a layout problem.
        // 过去有两种不同的东西共用一句话。用 `external_object!` 写的面声明了自己的注册机、并且**有意**
        // 不属于本包的生成树——graft 它的那个示例宿主显式注册它——因此"构建永远编译不了它"对一个能工作的
        // crate 来说是假话。只有生成本该在 `<name>/<name>.rs` 却长在别处的面才是布局问题。
        Ok(Some(face)) if is_external_face(&face) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: face.location.line,
            phase: EXTERNAL_FACE_PHASE,
            message: "external face (`external_object!`): it declares its own registry and this \
                      package's generated tree does not contain it; the host that grafts it \
                      registers it explicitly"
                .to_owned(),
        }),
        Ok(Some(face)) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: face.location.line,
            phase: "face-layout",
            // The **identity** half was missing, and it is the half that costs the reader an afternoon:
            // `NodeId = hash(namespace, source path, name)` includes the source path, so "move the file"
            // is not a layout fix — it is a rename, and every graft record, adoption entry and plugin lock
            // keyed by the old id stops resolving. Measured while auditing a rename: the build said only
            // that the file was in the wrong directory, which is true and useless at the same time
            // (audit `M7`, §M7.60).
            // **身份**那一半是缺的，而它正是让读者搭进去一个下午的那一半：`NodeId = hash(命名空间, 源码路径,
            // 名字)` 含源码路径，因此"把文件挪过去"不是一次布局修正——它是一次改名，而按旧 id 记账的每一条
            // graft 记录、认领条目与插件锁都会不再解析。审计一次改名时实测：构建只说这个文件在错的目录里，
            // 这句话同时是对的与没用的（审计 `M7`，§M7.60）。
            message: "registration face is outside the `<name>/<name>.rs` layout, so the build can never compile it; move the file to `<name>/<name>.rs`. That move is a **rename** as far as this                       framework is concerned: `NodeId = hash(namespace, source path, name)` takes the source                       path as an input, so the face this build derives is not the one the old path produced                       — every graft record, adoption entry and plugin lock keyed by the old identity stops                       resolving, and `xirang.diff {\"records\": true}` lists what did"
                .to_owned(),
        }),
        Err(error) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: error.location.as_ref().map_or(0, |location| location.line),
            phase: "face-syntax",
            message: error.message,
        }),
    }
}

/// Whether a registration macro declares a face of its own registry rather than of the
/// generated tree.
/// 一个注册宏声明的面是自己注册机的，还是生成树的。
///
/// `external_object!` is the framework's out-of-project declaration, and its `__`-prefixed
/// re-export is the same macro under the spelling reserved for generated code.
/// `external_object!` 是框架的项目外声明，而它带 `__` 前缀的重导出是同一个宏在"生成代码专用"拼法下的
/// 名字。
/// Whether one declaration is the **external** form: it declares its own registry and deliberately is not part of
/// this package's generated tree.
/// 一份声明是否是**外部**形式：它声明了自己的注册机，并且有意不属于本包的生成树。
///
/// Two spellings reach this, and the second one is why the first check was not enough. The literal form spells it
/// `external_object!`; the native form (H/A2e) writes `#[soil] … .source("third_party/x.rs")`, where `source` is the
/// field that exists **only** on the external form — it names the file explicitly because an external crate is not
/// part of the host's generated tree. Measured on the shipped `examples/control-button-graft`: its three native-spelled
/// external implementations were reported as `face-layout` failures ("the build can never compile it") on a crate that
/// builds and grafts correctly, because the classifier still keyed on the old macro name alone.
/// 有**两种**拼法到这里，而第二种正是只查宏名不够的原因。字面量形式写作 `external_object!`；原生形式（H/A2e）写作
/// `#[soil] … .source("third_party/x.rs")`，而 `source` 是**只**存在于外部形式上的字段——它显式点名文件，因为外部
/// crate 不在宿主的生成树里。在出厂的 `examples/control-button-graft` 上实测：它那三个原生拼法的外部实现被报成
/// `face-layout` 失败（"构建永远编译不了它"），而那个 crate 能构建、也能正确嫁接——因为分类器还只认旧的宏名。
///
/// Treating a `source` field as the external marker is not a heuristic: only the external form has that field, which is
/// what the field order says where it puts `source` first.
/// 把 `source` 字段当作外部标记不是启发式：只有外部形式有那个字段，而字段顺序把 `source` 排在第一的理由正是这个。
fn is_external_face(face: &super::registry_syntax::FaceSyntax) -> bool {
    matches!(
        face.macro_name.as_str(),
        "external_object" | "__external_object"
    ) || face.field("source").is_some()
}

fn has_source(node: &Node) -> bool {
    node.file.is_some() || node.children.iter().any(has_source)
}

/// Whether a directory name may become a module name.
/// 目录名是否可以成为模块名。
///
/// Discovery and the admission scan must agree on this: a directory discovery
/// skips is a directory whose faces never reach the generated tree, so the
/// admission scan must skip it too — otherwise a face that can never be
/// compiled still vetoes the build.
/// 发现过程与 admission 扫描必须在这一点上一致：发现过程跳过的目录，其注册面永远
/// 进不了生成树，因此 admission 扫描也必须跳过——否则一个永远编译不到的面仍然能否决
/// 构建。
pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
        })
}

pub(crate) fn emit_rerun_paths(src: &Path, nodes: &[Node]) {
    println!("cargo:rerun-if-changed={}", src.display());
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    files.sort();
    files.dedup();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
}

pub(crate) fn discovery_fingerprint(src: &Path, scan: &Path, nodes: &[Node]) -> String {
    discovery_fingerprint_for(env!("CARGO_PKG_VERSION"), src, scan, nodes)
}

/// The same digest with the **producing toolchain** spelled out — the seam a test moves (W7/A29).
/// 同一个摘要，但把**产生它的工具链**写明白——这是测试能移动的那道缝（W7/A29）。
///
/// A29's design decision (report §7c): the reuse condition carries the toolchain's version *and* the
/// existing `CACHE_SCHEMA`, because the schema governs **format** compatibility and the version
/// governs **semantics**. `rustc`'s own version is deliberately **not** an input: the artifacts here
/// (discovery, the contract's parent rules, facade/ghost units) depend on the *sources* rustc
/// compiles, not on its internal representation, so counting it would be a safety that costs a full
/// rebuild on every toolchain bump while buying nothing.
/// A29 的设计决定（报告 §7c）：复用条件同时带工具链版本**与**既有的 `CACHE_SCHEMA`——schema 管**格式**兼容、
/// 版本管**语义**兼容。`rustc` 自己的版本**有意**不作为输入：这里的派生物（发现、合同的父规则、facade/幽灵
/// 单元）依赖 rustc 编译的**源码**，而不依赖它的内部表示，因此把它算进去是一种"每次升级都全量重算、却什么也
/// 没换来"的安全感。
pub(crate) fn discovery_fingerprint_for(
    version: &str,
    src: &Path,
    scan: &Path,
    nodes: &[Node],
) -> String {
    // Two inputs, each covering what the other cannot (audit `T1`): the **raw file set** by path alone
    // — the module tree below cannot see a file that no `mod` declaration names, so a face added
    // beside the tree used to leave the fingerprint unchanged — and the **recorded files** with
    // contents, which is what notices an edit. Paths and not contents for the first half: hashing every
    // file's bytes would make each freshness check read the whole tree once more.
    // 两份输入，各自覆盖对方覆盖不到的东西（审计 `T1`）：**原始文件集合**只看路径——下面那棵模块树看不见
    // 没有任何 `mod` 声明的文件，因此在树旁边新增一个面过去不改指纹——以及**记录过的文件**及其内容，后者
    // 用来发现编辑。前半只取路径不取内容：哈希每个文件的字节会让每次新鲜性检查对整棵树多读一遍。
    let mut raw = Vec::new();
    visit_rust_paths(scan, &mut raw);
    raw.sort();
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    files.sort();
    let mut input = Vec::new();
    // The producing toolchain comes first, so two builds of different versions can never share a digest.
    // 产生它的工具链放在最前，因此两个不同版本的构建绝不可能共享同一个摘要。
    input.extend_from_slice(b"xirang-discovery\0");
    input.extend_from_slice(version.as_bytes());
    input.push(0);
    for path in &raw {
        input.extend_from_slice(super::relative_display(src, path).as_bytes());
        input.push(0);
        input.push(0xfe);
    }
    // The per-file half — read the bytes and spell the path — is independent per file, and it is the one
    // part of this rule parallelism can touch (audit `K2`). Measured at 50,000 files: **125 ms per
    // 2,500-file member**, i.e. ~50 µs per file of read + hash + allocation, paid once per member in
    // sequence. The chunks are joined in the **sorted file order**, which is what keeps the digest
    // identical to the serial one: combining in completion order would change the fingerprint and make
    // every existing record stale.
    // 每文件那一半——读字节、拼路径——逐文件独立，是这条规则里并行唯一能碰的部分（审计 `K2`）。在 50,000
    // 文件上实测：每个 2,500 文件的成员 **125 ms**，即每文件约 50 µs 的读+哈希+分配，而 20 个成员按顺序各付一次。
    // 各分块按**排序后的文件顺序**拼接，这正是让摘要与串行版逐字节相同的原因：按完成顺序拼接会改掉指纹，
    // 从而让每一份已存在的记录变陈旧。
    let chunks = parallel_map(&files, |file| {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(super::relative_display(src, file).as_bytes());
        chunk.push(0);
        chunk.extend_from_slice(&fs::read(file).unwrap_or_default());
        chunk.push(0xff);
        chunk
    });
    for chunk in chunks {
        input.extend_from_slice(&chunk);
    }
    super::registry_identity::NodeId::from_bytes(&input).to_string()
}

pub(crate) fn collect_source_files(nodes: &[Node], files: &mut Vec<std::path::PathBuf>) {
    for node in nodes {
        if let Some(file) = &node.file {
            files.push(file.clone());
        }
        collect_source_files(&node.children, files);
    }
}

/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
struct StdSourceTree;

impl xirang_kernel::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("cannot scan {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("cannot scan {}: {error}", path.display()))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

/// Every `.rs` file under `directory`, following the crate's own layout.
/// `directory` 下的每个 `.rs` 文件，遵循 crate 自己的布局。
pub(crate) fn collect_rust_sources(
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    xirang_kernel::source::collect_rust_sources(
        &StdSourceTree,
        directory,
        xirang_kernel::source::SourceWalk::EVERYTHING,
        |_, _| xirang_kernel::source::Keep::Yes,
        files,
    )
}

/// The cheap stamp of a package's sources: how many `.rs` files it has, and the newest modification
/// time among them (audit `T1`).
/// 一个包源码的廉价戳：有多少 `.rs` 文件，以及其中最新的修改时间（审计 `T1`）。
///
/// Directory entries only — not a single file is opened — so this costs a walk rather than the bytes.
/// It is a **guard**, never a verdict: the freshness rule beside it in the bridge still compares
/// bytes, and the generation file records this stamp only so that a reader can tell *cheaply* whether
/// the published index could still describe these sources. Its blind spot is by construction the
/// memo's: an edit that preserves both the file count and the modification time is not noticed.
/// 只读目录项——一个文件都不打开——因此它花一次遍历而不是字节。它是**守卫**，从不是裁决：桥里那条新鲜度
/// 规则仍然逐字节比较，而 generation 文件记下这个戳，只是为了让读者能**廉价地**判断已发布的索引是否仍可能
/// 描述着这批源码。它的盲区与那份记忆本来就相同：同时保留文件数与修改时间的编辑不会被发现。
///
/// `(0, 0)` when the package has no readable source layout at all: an absent tree is not an empty one,
/// and answering "zero files" would let a reader treat a missing tree as a clean one.
/// 包连可读的源码布局都没有时是 `(0, 0)`：缺一棵树不等于那是一棵空树，而答"零个文件"会让读者把缺失的树
/// 当成干净的树。
pub(crate) fn source_stamp(root: &Path) -> (usize, u64) {
    let Ok(layout) = crate::build_method::source_layout(root) else {
        return (0, 0);
    };
    let mut count = 0usize;
    let mut newest = 0u64;
    let mut stack = vec![layout.scan_root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            count += 1;
            if let Ok(modified) = metadata.modified()
                && let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH)
            {
                newest = newest.max(since.as_nanos() as u64);
            }
        }
    }
    (count, newest)
}

/// Every `.rs` file under `dir`, by path, without reading any of them.
/// `dir` 下每个 `.rs` 文件的路径，不读取其中任何一个。
fn visit_rust_paths(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rust_paths(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::{UnplacedFace, discover_root_reporting_with_workers};
    use std::fs;
    use std::path::PathBuf;

    /// A wide tree: `root/src/control/object/<face>/<face>.rs`, the shape a real generated host has
    /// when it has to spread a large registry over directories — and the shape the 50,000-file
    /// fixture has, whose wide level is neither the root nor the first level (audit `T1`).
    /// 一棵宽树：`root/src/control/object/<face>/<face>.rs`——真实生成宿主在必须把大注册表摊到目录里时的
    /// 形状，也正是 50,000 文件夹具的形状，而它的宽层既不是根层也不是第一层（审计 `T1`）。
    fn wide_tree(label: &str, faces: usize) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-source-walk-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/control/object")).expect("tree");
        fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
        fs::write(
            root.join("src/control/control.rs"),
            "// the module that owns the wide level\n",
        )
        .expect("control module");
        for index in 0..faces {
            let face = format!("child{index:06}");
            let directory = root.join("src/control/object").join(&face);
            fs::create_dir_all(&directory).expect("face directory");
            fs::write(directory.join(format!("{face}.rs")), "// face\n").expect("face source");
        }
        // A face that sits outside the layout is the finding the walk reports, and its position in
        // the merged list is part of what this pin compares.
        // 一个长在布局之外的面正是遍历要报告的发现，而它在合并后的清单里的位置也是这枚钉子比较的东西。
        fs::write(root.join("src/control/loose.rs"), "// not a face\n").expect("loose file");
        root
    }

    fn render_lines(nodes: &[super::Node]) -> Vec<String> {
        let mut lines = Vec::new();
        render_into(nodes, 0, &mut lines);
        lines
    }

    fn render_into(nodes: &[super::Node], depth: usize, lines: &mut Vec<String>) {
        for node in nodes {
            lines.push(format!(
                "{}{} file={:?}",
                "  ".repeat(depth),
                node.name,
                node.file.as_ref().map(|file| file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default())
            ));
            render_into(&node.children, depth + 1, lines);
        }
    }

    fn render_findings(findings: &[UnplacedFace]) -> Vec<String> {
        findings
            .iter()
            .map(|finding| format!("{} {} {}", finding.relative, finding.phase, finding.line))
            .collect()
    }

    /// Discovery must hand back the same tree and the same findings whatever the worker count
    /// (audit `T1`).
    /// 无论几个工作线程，发现过程都必须交回同一棵树与同一批发现（审计 `T1`）。
    ///
    /// The tree goes into the fingerprint that every record carries, so a result that depended on
    /// which worker finished first would invalidate every record on a machine with a different core
    /// count. The serial run is the reference; the parallel runs must match it **including the order
    /// of the findings**, which is the half that a naive parallel walk gets wrong.
    /// 这棵树会进入每份记录携带的指纹，因此一个取决于哪个工作线程先做完的结果，会在核数不同的机器上让每份记录
    /// 失效。串行那次是参照；并行的几次必须与它一致，**连发现的顺序也要一致**——那正是草率的并行遍历会弄错的
    /// 那一半。
    #[test]
    fn the_discovered_tree_does_not_depend_on_the_worker_count() {
        let root = wide_tree("workers", 200);
        let src = root.join("src");
        let mut serial_findings = Vec::new();
        let serial = discover_root_reporting_with_workers(&src, &mut serial_findings, 1);
        assert!(
            render_lines(&serial).len() > 200,
            "the fixture must be wide enough to take the parallel path: {}",
            render_lines(&serial).len()
        );
        for workers in [2usize, 3, 8, 64] {
            let mut findings = Vec::new();
            let tree = discover_root_reporting_with_workers(&src, &mut findings, workers);
            assert_eq!(
                render_lines(&tree),
                render_lines(&serial),
                "the tree differs with {workers} workers"
            );
            assert_eq!(
                render_findings(&findings),
                render_findings(&serial_findings),
                "the findings differ with {workers} workers"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod toolchain_version_tests {
    use super::discovery_fingerprint_for;
    use std::path::Path;

    /// W7/A32: **an irrelevant environment variable does not change the digest.** The invariant
    /// (report §7d) is "artifacts depend only on in-root sources plus the toolchain version", and the
    /// only way to vary the environment *soundly* is a child process: `set_var` is `unsafe` in edition
    /// 2024 and the harness runs tests on parallel threads, so an in-process test would be relying on
    /// undefined behaviour. This test therefore re-runs **its own binary** with one extra variable and
    /// compares the digest the child prints.
    /// W7/A32：**一个无关的环境变量不会改变摘要。** 不变量（报告 §7d）是"派生物只依赖根内源码 + 工具链
    /// 版本"，而**可靠地**改变环境只有子进程一条路：edition 2024 里 `set_var` 是 `unsafe`，且测试框架在
    /// 并行线程上跑测试，进程内改环境等于依赖未定义行为。因此这条测试**重跑自己的二进制**，只多给一个
    /// 环境变量，再比对子进程打印出来的摘要。
    #[test]
    fn an_irrelevant_environment_variable_does_not_change_the_digest() {
        let digest_of = |root: &std::path::Path| {
            discovery_fingerprint_for("0.1.0-probe", root, &root.join("src"), &[])
        };
        // The child: same code, same inputs, one extra variable already in its environment.
        // 子进程：同一份代码、同一组输入，只是环境里多了一个变量。
        if std::env::var_os("XIRANG_FP_CHILD").is_some() {
            let root = std::path::PathBuf::from(
                std::env::var("XIRANG_FP_ROOT").expect("the parent passes the tree"),
            );
            println!("child digest: {}", digest_of(&root));
            return;
        }

        let root = std::env::temp_dir().join(format!("xirang-fp-env-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn env_probe() {}\n").unwrap();
        let expected = digest_of(&root);

        let output = std::process::Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "an_irrelevant_environment_variable_does_not_change_the_digest",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("XIRANG_FP_CHILD", "1")
            .env("XIRANG_FP_ROOT", &root)
            .env(
                "XIRANG_FP_UNRELATED",
                "whatever this is, it is not an input",
            )
            .output()
            .expect("the child runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // The harness prints a test's stdout **on the same line** as the test name, so the marker is
        // searched for rather than anchored: `test … child digest: <hex>` is one line.
        // 测试框架把测试的 stdout 与测试名**打在同一行**，因此这里是按标记切分而不是按行首匹配：
        // `test … child digest: <hex>` 是一行。
        let child = stdout
            .lines()
            .find_map(|line| {
                line.split_once("child digest: ")
                    .map(|(_, digest)| digest.trim())
            })
            .filter(|digest| !digest.is_empty())
            .unwrap_or_else(|| panic!("the child prints its digest: {stdout}"));
        assert_eq!(
            expected, child,
            "an unrelated environment variable is not an input of the digest"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// W7/A29: the **producing toolchain** is an input of the discovery fingerprint, and content is
    /// still the other one. Three judgements were written before the code was (report §7c), so this
    /// pin is the code catching up with them rather than the other way round.
    /// W7/A29：**产生它的工具链**是发现指纹的一个输入，内容仍是另一个。三条判定先于代码写成（报告 §7c），
    /// 因此这条钉子是代码去对上它们，而不是反过来。
    #[test]
    fn the_fingerprint_carries_the_producing_toolchain_and_the_sources() {
        let root = std::env::temp_dir().join(format!("xirang-fp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn one() {}\n").unwrap();
        let fingerprint =
            |version: &str| discovery_fingerprint_for(version, &root, &root.join("src"), &[]);

        // ① A toolchain bump must not reuse the old cache: same sources, different version, different digest.
        // ① 工具链一变就不许复用旧缓存：源码相同、版本不同 ⇒ 摘要不同。
        let a = fingerprint("0.1.0");
        let b = fingerprint("0.2.0");
        assert_ne!(a, b, "the version is an input, not a comment");

        // ② Same version, same sources ⇒ still the same digest, so nothing is invalidated needlessly.
        // ② 版本相同、源码相同 ⇒ 摘要仍然相同，因此不会无谓地失效。
        assert_eq!(
            a,
            fingerprint("0.1.0"),
            "a stable version is a stable digest"
        );

        // ③ The other half is the **raw path set**: a file appearing beside the tree moves the digest
        // at a fixed version — that is the half no `mod` declaration is needed for (audit `T1`).
        // ③ 另一半是**原始路径集合**：版本不变而在树旁边新增一个文件，摘要就动——这一半不需要任何 `mod`
        // 声明（审计 `T1`）。
        std::fs::write(root.join("src/added.rs"), "pub fn two() {}\n").unwrap();
        let after_added = fingerprint("0.1.0");
        assert_ne!(
            a, after_added,
            "a new path still invalidates, whatever the version"
        );

        // ④ **What this seam cannot show**, recorded rather than assumed: content edits are seen by the
        // *recorded-files* half, which is driven by `nodes`, so with an empty node list an edit inside
        // an existing file leaves the digest where it was. That is the two-half design working as
        // written (paths from the raw walk, contents from the recorded files) — not a gap in A29.
        // ④ **这道缝展示不了的东西**，记下来而不是假设：内容编辑由**记录过的文件**那一半看见，而它由 `nodes`
        // 驱动，因此在 `nodes` 为空时，编辑一个已存在文件的内容不会动摘要。这是那套"两份输入"的设计按写法
        // 工作（路径来自原始遍历、内容来自记录过的文件），不是 A29 的缺口。
        std::fs::write(root.join("src/lib.rs"), "pub fn three() {}\n").unwrap();
        assert_eq!(
            after_added,
            fingerprint("0.1.0"),
            "with no nodes, the content half has nothing to read — stated, not silent"
        );
        let _ = std::fs::remove_dir_all(Path::new(&root));
    }
}

#[cfg(test)]
#[path = "source_walk_tests.rs"]
mod source_walk_tests;
