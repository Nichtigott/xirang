//! What a crate-shape declaration turns into: the ghost crates, planned before anything is written.
//! crate 形状声明会变成什么：幽灵 crate，在写下任何东西**之前**先把它们规划出来。
//!
//! A partition needs a crate that compiles a subtree the host no longer compiles (audit `M7`, P3.2).
//! That crate mounts the host's face files with `#[path]` — it does not copy them, because a copy is a
//! second file with the same declarations and a chance for the two to drift. Three facts make the
//! mounting work, and all three are decided here rather than at write time:
//! 一次分区需要一个 crate 去编译宿主不再编译的那棵子树（审计 `M7`，P3.2）。那个 crate 用 `#[path]` 挂载
//! 宿主的**面文件**——不复制它们，因为复制品是第二份带着同样声明的文件，也是一次两边漂移的机会。让挂载成立
//! 的有三件事，而三件都在这里、而不是在写下时才决定：
//!
//! 1. **The namespace is the host's**, so every identity the ghost computes is the id the host's build
//!    baked (a `#![allow]`-free `pub const XIRANG_NAMESPACE` — the constant P3.3 introduced exactly
//!    for this). Without it the ghost would hash its own package name and silently rename every face.
//!    命名空间是**宿主的**，因此幽灵算出的每个身份就是宿主的构建烤进去的那个 id（一个普通
//!    `pub const XIRANG_NAMESPACE`——P3.3 正是为它引入的常量）。没有它，幽灵会散列自己的包名，静默
//!    重命名每一个面。
//! 2. **The source path spelling is remapped back to the host's**, because identity is
//!    `hash(namespace, source path, name)` and `file!()` reports the `#[path]` spelling the ghost used.
//!    Each mount therefore carries the prefix to remap away, leaving `file!()` byte-identical to the
//!    host-relative source the host's records name.
//!    源码路径的拼写要**映射回宿主的**，因为身份是 `hash(命名空间, 源码路径, 名字)`，而 `file!()` 报告的
//!    是幽灵所用的 `#[path]` 拼写。因此每个挂载都带着要映射掉的前缀，让 `file!()` 与宿主记录点名的
//!    宿主相对源码逐字节相同。
//! 3. **Nothing outside the fragment is reachable**, which is the one thing a mount cannot fix: the ghost
//!    has inline container modules and the mounted leaves, and no `control.rs` above them. A face that
//!    says `use crate::control::ControlHandle` would not resolve there — and the honest answer is to
//!    **refuse by name** and name the two ways forward (make the fragment self-contained, or partition a
//!    node that contains the definition) rather than emit a crate that does not compile.
//!
//! **碎片之外的东西都够不到**，而这正是挂载修不了的一件事：幽灵有的是内联容器模块与被挂载的叶子，没有它们
//! 上面的 `control.rs`。写着 `use crate::control::ControlHandle` 的面在那里解析不到——而诚实的做法是**点名
//! 拒绝**并说出两条出路（让碎片自足，或分区一个包含该定义的节点），而不是发射一个编译不过的 crate。
//!
//! This module **writes nothing**. It answers "what would be written, and is it sound?" — which is the
//! `--check` half of the authoring action and the input of its `--write` half.
//! 本模块**什么都不写**。它回答"会写下什么、这样成立吗"——那是创作动作的 `--check` 一半，也是它
//! `--write` 一半的输入。

pub(crate) use super::partition_roots::{compact_path, partition_roots, toml_section, toml_value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::HostCut;
use super::registry_identity::NodeId;
use xirang_kernel::lexicon;

use super::shape_decl::ShapeDeclaration;

/// One ghost crate: the package to create, and the mounts that make it the same faces.
/// 一个幽灵 crate：要创建的包，以及让它成为同一批面的那些挂载。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedCrate {
    /// The declared name inside the host's shape (`widgets`).
    /// 宿主形状里的声明名（`widgets`）。
    pub(crate) name: String,
    /// The package name: `<package_prefix>-<name>`.
    /// 包名：`<package_prefix>-<name>`。
    pub(crate) package: String,
    /// Where the ghost package goes: a sibling of the host package.
    /// 幽灵包的位置：宿主包的**同级**。
    pub(crate) directory: PathBuf,
    /// The identity namespace the ghost must define: the host's, so every face keeps its id.
    /// 幽灵必须定义的身份命名空间：**宿主的**，这样每个面都保住自己的 id。
    pub(crate) namespace: String,
    /// The subtree module paths this crate claims.
    /// 这个 crate 认领的子树模块路径。
    pub(crate) subtrees: Vec<String>,
    /// The crate-local alias module (B2); see [`aliases_rs`].
    /// crate 内的别名模块（B2）；见 [`aliases_rs`]。
    pub(crate) aliases_rs: String,
    pub(crate) remap: Vec<(String, String)>,
    /// The proper prefixes of the claims that are **faces**, with their identities: a shell module
    /// needs them so a fragment's `parent: crate::<ancestor>::NODE_ID` still resolves in a crate that
    /// does not compile the ancestor. Nothing else of the ancestor's is offered — a trait defined up
    /// there stays unreachable, which is the honest boundary this planner refuses on.
    /// 认领的**是注册面**的真前缀，连同它们的身份：壳模块需要它们，好让碎片里的
    /// `parent: crate::<祖先>::NODE_ID` 在一个不编译祖先的 crate 里仍然解析得到。祖先的其它东西一概不提供
    /// ——定义在上面的 trait 仍不可达，那是本规划器据以拒绝的诚实边界。
    pub(crate) ancestors: Vec<(String, NodeId)>,
    /// One mount per face file below those subtrees.
    /// 那些子树之下的每份面文件一项挂载。
    pub(crate) mounts: Vec<PlannedMount>,
    /// The ghost's `src/lib.rs`: the namespace constant and the include of its own generated tree.
    /// 幽灵的 `src/lib.rs`：命名空间常量，以及它自己那棵生成树的 include。
    pub(crate) lib_rs: String,
    /// The ghost's `build.rs`: run the host's manifest through the pipeline for this fragment.
    /// 幽灵的 `build.rs`：把宿主的清单交给管线，只渲染这个碎片。
    pub(crate) build_rs: String,
    /// The ghost's `Cargo.toml`: the host's dependencies, copied verbatim.
    /// 幽灵的 `Cargo.toml`：宿主的依赖，逐字照抄。
    pub(crate) cargo_toml: String,
    /// The `rustflags` entries the **workspace root's** `.cargo/config.toml` has to carry, with the
    /// reason they cannot live in this package's own config (a rustflag is per invocation, and cargo
    /// finds config from the current directory rather than from the package).
    /// **工作区根** `.cargo/config.toml` 必须携带的 `rustflags` 条目，并写明它们为何不能住在这个包自己的
    /// config 里（rustflag 是每次调用的，而 cargo 从当前目录而不是从包出发找 config）。
    pub(crate) config_patch: String,
}

/// It runs the **host's** manifest through the same pipeline and writes this crate's `OUT_DIR`, asking
/// for the fragment only (`XIRANG_SHAPE_ONLY`): the sources are the host's, the identities are the
/// host's, and what lands here is the part of the tree this crate is the one to compile.
/// 它把**宿主的**清单交给同一条管线、写本 crate 的 `OUT_DIR`，并且只要那个碎片（`XIRANG_SHAPE_ONLY`）：
/// 源码是宿主的、身份是宿主的，落在这里的是这棵树中由本 crate 负责编译的那部分。
/// One `#[path] pub mod …;` in the ghost's generated tree.
/// 幽灵生成树里的一条 `#[path] pub mod …;`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedMount {
    /// The module path the face must keep (`control::object::button`).
    /// 该面必须保持的模块路径（`control::object::button`）。
    pub(crate) module_path: String,
    /// The host-relative source (`control/object/button/button.rs`) — what the records name.
    /// 宿主相对源码（`control/object/button/button.rs`）——记录点名的那个。
    pub(crate) source: String,
    /// The `#[path]` spelling, written relative to the directory the inline module sits in.
    /// `#[path]` 拼写，写成相对内联模块所在目录的形式。
    pub(crate) spelling: String,
}
/// The line every generated ghost file carries, so a revert can tell its own work from a hand-written
/// package. One home, because the writer that puts it there and the reader that checks it must agree.
/// 每份生成的幽灵文件都带着的那一行，好让撤回能把自己的产物与手写的包区分开。只有一处住址，因为写下它的写入方
/// 与检查它的读取方必须一致。
pub(crate) const GENERATED_MARKER: &str = "Generated by XiRang";
/// The `&[HostCut { … }]` literal a generated build script carries its host's cuts in.
/// 生成的构建脚本用来携带宿主切口的 `&[HostCut { … }]` 字面量。
///
/// Rendered rather than re-read from the host at build time because the release shape has no host tree
/// to read — the shipped crate carries copied faces and nothing else. The development shape carries it
/// too, and there the entry still wins at build time: a host that edits its plan must not keep answering
/// from the snapshot taken when the package was generated (see `pipeline`).
/// 渲染出来而不是在构建期回读宿主，是因为发布形状没有宿主树可读——发出去的 crate 只带着复制过来的面。
/// 开发形状也带着它，而在那里入口在构建期仍然赢：改了计划的宿主不该继续用生成那个时刻的快照作答
/// （见 `pipeline`）。
pub(crate) fn host_cuts_literal(cuts: &[HostCut]) -> String {
    if cuts.is_empty() {
        return "&[]".to_owned();
    }
    let mut out = String::from("&[\n");
    for cut in cuts {
        out.push_str("            xirang_toolchain::build_method::HostCut {\n");
        out.push_str(&format!("                cut: {:?}.to_owned(),\n", cut.cut));
        out.push_str(&format!(
            "                cut_end: {},\n",
            option_literal(&cut.cut_end)
        ));
        out.push_str(&format!(
            "                graft: {:?}.to_owned(),\n",
            cut.graft
        ));
        out.push_str(&format!("                full: {},\n", cut.full));
        out.push_str(&format!("                typed: {},\n", cut.typed));
        out.push_str(&format!(
            "                cfg: {},\n",
            option_literal(&cut.cfg)
        ));
        out.push_str(&format!("                line: {},\n", cut.line));
        out.push_str(&format!("                column: {},\n", cut.column));
        out.push_str("            },\n");
    }
    out.push_str("        ]");
    out
}
/// The partition call a generated build script makes: the function name and the cuts argument.
/// 生成的构建脚本所做的分区调用：函数名与切口参数。
///
/// One home for the three shapes that carry these cuts — the development ghost, the release ghost and
/// the two facades — because a call site that spelled the argument differently would be a fourth answer
/// to "how does a generated package receive its host's plan".
/// 携带这些切口的三种形状——开发幽灵、发布幽灵与两种 facade——共用一处住址：某个调用点若把参数拼成另
/// 一种样子，就是"生成包如何收到宿主的计划"的第四个答案。
pub(crate) fn partition_call(cuts: &[HostCut]) -> (String, String) {
    if cuts.is_empty() {
        ("run_for_partition".to_owned(), String::new())
    } else {
        (
            "run_for_partition_with_cuts".to_owned(),
            format!("        {},\n", host_cuts_literal(cuts)),
        )
    }
}
/// One `Option<String>` field of a `HostCut` literal.
/// `HostCut` 字面量的一个 `Option<String>` 字段。
fn option_literal(value: &Option<String>) -> String {
    match value {
        Some(text) => format!("Some({text:?}.to_owned())"),
        None => "None".to_owned(),
    }
}
/// The ghost's `src/lib.rs`.
/// 幽灵的 `src/lib.rs`。
///
/// Two items, and the first one is the whole point of P3.3: the ghost declares the **host's** identity
/// namespace, so every face it compiles keeps the id the host's build baked. Its own generated tree
/// carries the mounts; its own `src/` has no faces at all.
/// 两项，而第一项正是 P3.3 的全部意义：幽灵声明的是**宿主的**身份命名空间，因此它编译的每个面都保住宿主构建
/// 烤进去的那个 id。它自己的生成树携带挂载；它自己的 `src/` 一个面都没有。
fn ghost_lib_rs(namespace: &str) -> String {
    format!(
        "//! {marker}: the crate that compiles one fragment of a host's registration\n\
         //! tree. Do not edit — the declaration lives in the host's `add_crates.rs`.\n\
         //! 由 XiRang 生成：编译宿主注册树某一个碎片的 crate。请勿手工修改——声明住在宿主的\n\
         //! `add_crates.rs` 里。\n\n\
         pub const XIRANG_NAMESPACE: &str = {namespace:?};\n\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));\n",
        marker = GENERATED_MARKER
    )
}
/// The crate-local alias module (B2): one `as`-name per claimed subtree, written into the package's own
/// `src/` so the declaration's paths are ones an editor can complete. Inside the crate the same paths
/// resolve through the generated tree — but a path that exists only in generated code is not something a
/// completion engine will offer, and the declaration is a surface people write by hand.
/// crate 内的别名模块（B2）：每条被认领的子树一个 `as` 名，写进本包自己的 `src/`，于是声明里的路径是编辑器
/// 补得出来的。在 crate 内部同样的路径经生成树也能解析——可只存在于生成代码里的路径，补全引擎不会给你，
/// 而声明是**人总会手写**的那种表面。
pub(crate) fn aliases_rs(subtrees: &[String]) -> String {
    let mut text = String::from(
        "//! The subtree markers this crate claims, as paths an editor can complete.\n\
         //! Generated by XiRang; the markers themselves belong to the host's tree.\n\
         //! 本 crate 认领的子树标记，写成编辑器能补全的路径。由 XiRang 生成；标记本身属于宿主的树。\n\n",
    );
    for subtree in subtrees {
        let name = subtree
            .rsplit("::")
            .next()
            .unwrap_or(subtree)
            .to_uppercase();
        text.push_str(&format!("pub use crate::{subtree}::SUBTREE as {name};\n"));
    }
    text
}

/// One `rerun-if-changed` line per file **and per directory** below `src`, indented like the rest of
/// the generated script. Directories are included because a new file inside one changes that
/// directory's mtime, which is how a face that appears later is noticed at all.
/// `src` 之下**每个文件与每个目录**一行 `rerun-if-changed`，缩进与生成脚本的其余部分一致。目录也要
/// 列上：新建的文件会改变它所在目录的 mtime，这正是"后来才出现的面"能被注意到的唯一途径。
///
/// Every line is spelled as `host.join(<path relative to the host>)` rather than as the absolute path
/// the planner saw. A fragment is **committed** (`xirang crates --write` is the submit step), so an
/// absolute path would bind the split to the machine that produced it: a fresh checkout elsewhere
/// would watch paths that do not exist and rebuild against a host directory it does not have.
/// The relative spelling is what makes the committed fragment portable, and it is also what makes
/// `xirang crates --write` byte-for-byte reproducible on every machine.
/// 每一行都写成 `host.join(<相对宿主的路径>)`，而不是规划器看到的绝对路径。碎片是要**入库**的
/// （`xirang crates --write` 就是提交那一步），绝对路径会把这次拆分绑死在产生它的机器上：换一台机器
/// 全新检出的副本会去监视不存在的路径、对着一个它没有的宿主目录重建。相对写法才是入库碎片可移植的原因，
/// 也是 `xirang crates --write` 在每台机器上逐字节可复现的原因。
fn watch_lines(src: &Path, host: &Path) -> String {
    let mut paths = Vec::new();
    subtree_paths(src, &mut paths);
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let spelled = path
                .strip_prefix(host)
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| path.display().to_string());
            format!("println!(\"cargo:rerun-if-changed={{}}\", host.join({spelled:?}).display());")
        })
        .collect::<Vec<_>>()
        .join("\n    ")
}

fn subtree_paths(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    out.push(dir.to_path_buf());
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subtree_paths(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The expression a committed fragment uses to reach its host: `CARGO_MANIFEST_DIR` plus a walk
/// spelled with `.parent()`/`.join()`, never an absolute path and never a surviving `..`.
/// 入库碎片用来到达宿主的表达式：`CARGO_MANIFEST_DIR` 加一段用 `.parent()`/`.join()` 写成的走法，
/// 绝不是绝对路径，也绝不留下 `..`。
pub(super) fn host_binding(fragment_dir: &Path, host_root: &Path, indent: &str) -> String {
    // The walk from the fragment to its host, spelled as `PathBuf` steps: `pop()` walks up without
    // leaving a `..` component behind, and `push()` names the host. The committed fragment therefore
    // says "two levels up, then `shop`" instead of naming this machine's checkout.
    // 从碎片走到宿主的走法，写成 `PathBuf` 的几步：`pop()` 向上走却不留下 `..` 成分，`push()` 点名宿主。
    // 于是入库的碎片说的是"上两级、再进 `shop`"，而不是点名本机的检出目录。
    //
    // Why `pop()` rather than `.join("../..")`: a spelling that still contains `..` no longer matches the
    // `--remap-path-prefix` the partition wrote (`<host>/src/` → nothing), so every mounted face would
    // compile a `file!()` the plan never baked — measured as `static identity failed: this face compiles
    // an id the build did not bake` while making the committed example portable.
    // 为什么用 `pop()` 而不是 `.join("../..")`：仍带 `..` 的拼写不再匹配拆分写下的 `--remap-path-prefix`
    // （`<宿主>/src/` → 空），于是每个被挂载面编译出的 `file!()` 都不是计划烘焙过的那个——让入库示例可移植时
    // 实测到的报错正是 `static identity failed: this face compiles an id the build did not bake`。
    //
    // Short statements on purpose: the fragments are committed, and `cargo fmt` must not rewrite them, or
    // the "tool output equals the committed bytes" gate fails on a formatted tree.
    // 有意写成短语句：碎片要入库，而 `cargo fmt` 不得改写它们，否则"工具输出等于入库字节"那道门禁在格式化过的
    // 树上就会红。
    let mut steps =
        format!("{indent}let mut host = std::path::PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));\n");
    match relative_walk(fragment_dir, host_root) {
        Some(walk) if !walk.starts_with('/') => {
            for component in walk.split('/') {
                if component == ".." {
                    steps.push_str(&format!("{indent}host.pop();\n"));
                } else {
                    steps.push_str(&format!("{indent}host.push({component:?});\n"));
                }
            }
        }
        _ => {
            steps = format!(
                "{indent}let mut host = std::path::PathBuf::from({:?});\n",
                host_root.display().to_string()
            )
        }
    }
    // `host` need not be mutable when no step follows the binding, and `cargo clippy -- -D warnings`
    // refuses an unused `mut`.
    // 后面没有步子时 `host` 不必可变，而 `cargo clippy -- -D warnings` 会拒绝多余的 `mut`。
    if !steps.contains("host.pop();") && !steps.contains("host.push(") {
        steps = steps.replace("let mut host =", "let host =");
    }
    steps
}

fn ghost_build_rs(
    package_root: &Path,
    fragment_dir: &Path,
    namespace: &str,
    subtrees: &[String],
    cuts: &[HostCut],
) -> String {
    let src = package_root.join("src");
    let declaration = lexicon::ADD_CRATES_FILE;
    // Cargo reads `rerun-if-changed=<dir>` as the **directory's own** mtime, not as a recursive
    // watch, so a fragment that watches only `<host>/src` misses every edit made to a face file
    // below it — measured: appending to the host's `lib.rs` left the fragment's tree untouched, and
    // a `promote` that deepens a face (moving it to `…/object/widget/widget.rs`) never made the
    // fragment rebuild, so the tree kept asserting the identity the face had before it moved.
    // The host's own script already watches every file (`source_walk::emit_rerun_paths`); this is
    // the same list for the package that compiles part of that tree.
    // cargo 把 `rerun-if-changed=<目录>` 读作**目录自身**的 mtime，而不是递归监视，因此只盯
    // `<宿主>/src` 的碎片会漏掉它下面任何一个面文件的改动——实测：往宿主的 `lib.rs` 追加内容，
    // 碎片的树一动不动；而一个把面**加深**的 `promote`（把它挪到 `…/object/widget/widget.rs`）也从没
    // 让碎片重建，于是树里一直断言着那个面搬迁前的身份。宿主自己的脚本本来就逐文件监视
    // （`source_walk::emit_rerun_paths`）；这里给那个编译这棵树一部分的包补上同一张清单。
    let watch = watch_lines(&src, package_root); // The entry is the authority while this package can see the host's tree (the development shape);
    // the literal is what a released package has instead. A host that declares no cuts passes the old
    // five-argument call, so a generated script says exactly as much as it needs to.
    // 本包看得见宿主树时（开发形状）权威是入口；字面量是发布出去的包所拥有的替代品。宿主没有声明任何切口时
    // 走旧的五参数调用，因此生成的脚本只说它必须说的那些。
    let (function, cuts_argument) = partition_call(cuts);
    format!(
        "//! {marker}: build this fragment out of the host's sources.\n\
         //! 由 XiRang 生成：从宿主的源码构建这个碎片。\n\n\
         #[rustfmt::skip]\n\
         fn main() {{\n\
         {host_binding}\
         {i}println!(\"cargo:rerun-if-changed={{}}\", host.join(\"src\").display());\n\
         {i}println!(\"cargo:rerun-if-changed={{}}\", host.join({declaration:?}).display());\n\
         {i}{watch}\n\
         {i}let out = std::path::PathBuf::from(std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\"));\n\
         {i}xirang_toolchain::build_method::{function}(\n\
         {i}    &host,\n\
         {i}    &out,\n\
         {i}    {namespace:?},\n\
         {i}    Some({claims:?}),\n\
         {i}    false,\n\
         {cuts_argument}\
         {i})\n\
         {i}.expect(\"xirang\");\n\
         }}\n",
        i = "    ",
        watch = watch,
        declaration = declaration,
        host_binding = host_binding(fragment_dir, package_root, "    "),
        claims = subtrees.join(","),
        namespace = namespace,
        marker = GENERATED_MARKER,
    )
}

/// The ghost's `Cargo.toml`: the host's dependencies, **copied verbatim**.
/// 幽灵的 `Cargo.toml`：宿主的依赖，**逐字照抄**。
///
/// Copied rather than re-derived because the mounted files are the host's: whatever they import has to
/// resolve here exactly as it resolves there, and a hand-written dependency list is a second answer to
/// "what does this source need". `publish = false`: a ghost is a workspace-local build shape, and what
/// a published partition looks like is P3.5's question, not this planner's.
/// 照抄而不是重新推导，因为被挂载的文件就是宿主的：它们 import 什么，在这里就必须和在那里一样解析得到，而
/// 手写的依赖清单是"这份源码需要什么"的第二个答案。`publish = false`：幽灵是工作区本地的构建形状，而发布出去
/// 的分区长什么样是 P3.5 的问题、不是本规划器的。
fn ghost_cargo_toml(
    package_root: &Path,
    package: &str,
    directory: &Path,
    generated: &[String],
) -> Result<String, String> {
    let manifest = package_root.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "add_crates: cannot read {} to copy its dependencies: {error}",
            manifest.display()
        )
    })?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "0.1.0".to_owned());
    let edition = toml_value(&text, "edition").unwrap_or_else(|| "2024".to_owned());
    let mut output = format!(
        "# Generated by XiRang: the dependencies are the host's, with their relative paths\n\
         # re-spelled for this crate's directory.\n\
         # 由 XiRang 生成：依赖是宿主的，相对路径已按本 crate 所在目录重拼。\n\
         [package]\nname = {package:?}\nversion = {version:?}\nedition = {edition:?}\n\
         publish = false\n"
    );
    for section in ["dependencies", "build-dependencies"] {
        if let Some(block) = toml_section(&text, section) {
            let block = respell_dependency_paths(&block, package_root, directory);
            // The host now depends on the generated crates (so its own faces can name the types they
            // took over). Copying that edge into a ghost would make the ghost depend on itself or on
            // its sibling a second time; the partition plan owns those edges.
            // 宿主现在依赖生成 crate（好让宿主自己的面点名被接管的类型）。把这条边抄进幽灵会让它依赖自己、
            // 或把兄弟依赖加第二遍；这些边归分区计划所有。
            let block = strip_generated_dependencies(&block, generated);
            output.push_str(&format!("\n[{section}]\n{block}"));
        }
    }
    Ok(output)
}

/// Re-spell every `path = "…"` in a copied dependency block so it still points at the same package
/// from the *ghost's* directory.
/// 把照抄来的依赖块里每个 `path = "…"` 重拼成"从**幽灵自己的**目录出发仍指向同一个包"。
///
/// The host's relative paths are written relative to the host, and the ghost is not always its
/// sibling: a partition puts it under `crates/`, one level down. Copying them verbatim then points at
/// a package that is not there — measured, this is what broke the committed example's build the
/// moment the generated packages moved. The facade has always done this walk
/// (`crate_facade::sibling_dependencies`); this is the same walk, applied to the ghost's copy.
/// 宿主的相对路径是相对宿主写的，而幽灵并不总是它的同级：划分把它放在 `crates/` 之下、低一层。逐字照抄会指向
/// 一个不存在的包——实测：生成包一挪位置，入库示例的构建就是这样坏的。facade 一直做这个走法
/// （`crate_facade::sibling_dependencies`）；这里是同一个走法，用在幽灵那份拷贝上。
pub(crate) fn respell_dependency_paths(
    block: &str,
    package_root: &Path,
    directory: &Path,
) -> String {
    let mut respelled = block
        .lines()
        .map(|line| {
            let Some((head, rest)) = line.split_once("path = \"") else {
                return line.to_owned();
            };
            let Some((path, tail)) = rest.split_once('"') else {
                return line.to_owned();
            };
            let target = compact_path(&package_root.join(path));
            match relative_walk(directory, &target.join("Cargo.toml")) {
                Some(walk) => format!(
                    "{head}path = {:?}{tail}",
                    walk.trim_end_matches("/Cargo.toml")
                ),
                None => line.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    // The trailing newline belongs to the block: the ghost appends a section header after it and the
    // facade appends sibling entries, so a re-spelled block that lost it glues two tables together
    // (measured: `…graft = { path = "../../graft" }xirang-example-… = { … }`, which cargo refuses).
    // 尾换行属于这个块：幽灵在它之后接一节表头、facade 在它之后接同级条目，所以重拼后若丢掉它，两张表会被
    // 粘在一起（实测：`…graft = { path = "../../graft" }xirang-example-… = { … }`，cargo 直接拒绝）。
    if block.ends_with('\n') {
        respelled.push('\n');
    }
    respelled
}

/// The pair `--remap-path-prefix` receives: the spelling minus the source itself, mapped to nothing.
/// `--remap-path-prefix` 收到的那一对：拼写里去掉源码本身的那部分，映射为空。
fn remap_for(spelling: &str, source: &str) -> (String, String) {
    match spelling.strip_suffix(source) {
        Some(prefix) => (prefix.to_owned(), String::new()),
        None => (spelling.to_owned(), String::new()),
    }
}

/// Refuse a face that reaches **outside** the fragment: the ghost has no `control.rs` above it, so
/// such a path cannot resolve, and a crate that does not compile is worse than a named refusal.
/// 拒绝一个**够到碎片之外**的面：幽灵在它上面没有 `control.rs`，因此那样的路径解析不到，而一个编译不过的
/// crate 比一句点名拒绝更坏。
fn check_reachability(
    package_root: &Path,
    crate_name: &str,
    source: &str,
    subtrees: &[String],
    ancestors: &[String],
    modules: &[String],
) -> Result<(), String> {
    let path = package_root.join("src").join(source);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        for (at, _) in line.match_indices("crate::") {
            let rest = &line[at + "crate::".len()..];
            // A grouped import (`use crate::{NoParts, NoPreset};`) names several things at once, and
            // the group is not itself a path: every name in it is checked on its own. Reading the group
            // as one path produced an **empty** head, so a host built by `new_project` could not be
            // partitioned at all — each generated face opens with exactly that line (measured, §M7.39).
            // 分组导入（`use crate::{NoParts, NoPreset};`）一次点名好几样东西，而分组本身不是路径：其中每个
            // 名字各自受检。把分组当成一条路径会得到一个**空**的头，于是 `new_project` 建出来的宿主**完全无法
            // 分区**——每个生成的注册面都以那一行开头（实测，§M7.39）。
            if let Some(group) = rest.strip_prefix('{') {
                let Some((names, _)) = group.split_once('}') else {
                    continue;
                };
                for name in names
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                {
                    // The first segment is what has to resolve; `crate::{panel::NODE_ID}` names the
                    // same module a bare `crate::panel` does.
                    // 需要解析的是第一段；`crate::{panel::NODE_ID}` 点名的模块与裸的 `crate::panel` 相同。
                    let head = name.split("::").next().unwrap_or(name).trim();
                    if !reachable(head, subtrees, ancestors, modules) {
                        return Err(unreachable(crate_name, source, index + 1, head, subtrees));
                    }
                }
                continue;
            }
            // The trailing `::` of a braced path (`crate::panel::{Gauge}`) is not part of the module
            // the reader has to see, and a message that ends in one reads like a cut-off sentence.
            // 花括号路径（`crate::panel::{Gauge}`）末尾的 `::` 不属于读者需要看见的那个模块，而以此结尾的
            // 消息读起来像被截断的句子。
            let head: String = rest
                .chars()
                .take_while(|character| {
                    character.is_alphanumeric() || *character == '_' || *character == ':'
                })
                .collect::<String>()
                .trim_end_matches(':')
                .to_owned();
            // A `crate::…!` path is a **macro invocation**, not a module lookup: the declaration macro
            // aliases (`crate::root_object!`, `crate::control_object!`) are emitted into every crate's
            // own generated tree, so the ghost has them exactly as the host does. They are also the
            // reason this check cannot be "does the path resolve" — text cannot answer that; what it
            // can answer is "does this reach a *module* outside the fragment".
            // `crate::…!` 路径是**宏调用**，不是模块查找：声明宏别名（`crate::root_object!`、
            // `crate::control_object!`）会发射进每个 crate 自己的生成树，因此幽灵和宿主一样拥有它们。
            // 它们也正是这条检查不能写成"路径能不能解析"的原因——文本答不了那个问题；它能答的是"这里够到了
            // 碎片之外的**模块**吗"。
            if rest[head.len()..].trim_start().starts_with('!') {
                continue;
            }
            if reachable(&head, subtrees, ancestors, modules) {
                continue;
            }
            return Err(unreachable(crate_name, source, index + 1, &head, subtrees));
        }
    }
    Ok(())
}

/// The refusal for one `crate::` path the ghost cannot resolve, with the ways forward.
/// 对一条幽灵解析不了的 `crate::` 路径的拒绝，带上出路。
fn unreachable(
    crate_name: &str,
    source: &str,
    line: usize,
    head: &str,
    subtrees: &[String],
) -> String {
    format!(
        "add_crates: `{crate_name}` would not compile: {source}:{line} reaches `crate::{head}`, \
         which lives outside the subtree(s) this crate claims ({}), and a ghost crate mounts \
         only those files — there is no module above them to resolve it against.\n\
         way forward: partition a node that contains the definition (a crate is a subtree, so \
         a larger subtree is a legal answer), or make the fragment self-contained by moving \
         the item into it, or name it through the crate that defines it \
         (`xirang_toolchain::run_method::registry_core::…` for a kernel name), which every \
         crate can name",
        subtrees.join(", ")
    )
}

/// The names of this package's own top-level modules, read from `src/`.
/// 本包自己的顶层模块名，从 `src/` 读出。
///
/// Rust's module↔path mapping is what makes this decidable: a top-level module is a directory or a
/// `.rs` file beside `lib.rs`/`main.rs`. A `crate::` path whose first segment is one of these is a
/// **module path**, so the ghost must mount it; anything else is a crate-root item, and the preamble
/// every generated tree emits decides what those are.
/// 让这件事可判定的是 Rust 的模块↔路径映射：顶层模块就是 `lib.rs`/`main.rs` 旁边的目录或 `.rs` 文件。
/// 首段是其中之一的 `crate::` 路径是**模块路径**，幽灵必须挂载它；其余是 crate 根上的条目，而它们是什么由
/// 每个生成树都会发射的前言决定。
/// The directory a partition's generated packages live in, under the partition's build root.
/// 一次拆分的生成包所在的目录，位于该拆分的构建根之下。
///
/// One implementation for the ghost, the facade and both release shapes: three copies of this rule
/// would be three chances for one of them to write a package where the workspace member list, the
/// revert path and the reader's eyes do not look.
/// 幽灵、facade 与两种发布形状共用一份实现：这条规则有三份拷贝，就是三次"其中一个把包写在成员清单、
/// 撤回路径与读者目光都不看的地方"的机会。
pub(crate) fn crates_dir(root: &Path) -> PathBuf {
    root.join(lexicon::CRATES_DIR)
}

/// Every module path this host's `src/` declares, to any depth — the authority for "does this claim
/// name something in the tree".
/// 这个宿主的 `src/` 在任何深度上声明的模块路径全集——它是"这个认领点名的东西在不在树里"的权威。
///
/// `source_modules` above answers a different question (which top-level names a `crate::` path may
/// start with) and only reads one level; a claim can name any depth (`control::left`), so this walks.
/// 上面的 `source_modules` 回答的是另一个问题（`crate::` 路径可以以哪些顶层名字开头），而且只读一层；
/// 认领可以点名任意深度（`control::left`），所以这里要递归。
fn module_paths(package_root: &Path) -> Vec<String> {
    fn walk_paths(base: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(base) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if path.is_dir() {
                let module = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}::{name}")
                };
                out.push(module.clone());
                walk_paths(&path, &module, out);
            } else if let Some(stem) = name.strip_suffix(".rs") {
                if prefix.is_empty() && (stem == "lib" || stem == "main") {
                    continue;
                }
                out.push(if prefix.is_empty() {
                    stem.to_owned()
                } else {
                    format!("{prefix}::{stem}")
                });
            }
        }
    }
    let mut modules = Vec::new();
    walk_paths(&package_root.join("src"), "", &mut modules);
    modules.sort();
    modules.dedup();
    modules
}

fn source_modules(package_root: &Path) -> Vec<String> {
    let mut modules = Vec::new();
    let Ok(entries) = fs::read_dir(package_root.join("src")) else {
        return modules;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() {
            modules.push(name);
        } else if let Some(stem) = name.strip_suffix(".rs")
            && stem != "lib"
            && stem != "main"
        {
            modules.push(stem.to_owned());
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

/// Whether a `crate::` path resolves in the ghost as well as in the host.
/// 一条 `crate::` 路径在幽灵里是否与在宿主里一样解析得到。
fn reachable(head: &str, subtrees: &[String], ancestors: &[String], modules: &[String]) -> bool {
    // Only the **first** segment decides whether this is a module path: `crate::FlowContract::new(…)`
    // and `crate::NoParts` both begin with an item, and an associated function is not a module. The
    // claim and ancestor comparisons below still read the whole path, because that is what a shell
    // carries (`<ancestor>::NODE_ID`) and what a claim names.
    // 决定它是不是模块路径的只有**第一段**：`crate::FlowContract::new(…)` 与 `crate::NoParts` 都以条目开头，
    // 而关联函数不是模块。下面的认领与祖先比较仍然读整条路径，因为壳携带的（`<祖先>::NODE_ID`）与认领点名的
    // 就是整条。
    let first = head.split("::").next().unwrap_or(head);
    if subtrees
        .iter()
        .any(|subtree| head == subtree || head.starts_with(&format!("{subtree}::")))
    {
        return true;
    }
    // An ancestor is reachable as a **module** and for its `NODE_ID` alone: the shell carries that
    // one constant so a fragment's `parent:` still resolves, and carries nothing else — a trait
    // defined above the fragment is exactly what this planner refuses on.
    // 祖先作为**模块**、以及仅为它的 `NODE_ID` 可达：壳携带那一个常量，好让碎片的 `parent:` 仍然解析，别的
    // 什么都不带——定义在碎片之上的 trait 正是本规划器据以拒绝的东西。
    if ancestors
        .iter()
        .any(|ancestor| head == ancestor || head == format!("{ancestor}::NODE_ID"))
    {
        return true;
    }
    // Everything left whose first segment is **not** a module of this package is a crate-root item,
    // and the ghost's
    // crate root carries the same preamble the host's does — `pub use <registry_core>::*;` — so the
    // kernel vocabulary a face imports resolves in both. Measured before this rule: `new_project`'s
    // own faces (`use crate::{NoParts, NoPreset};`, `use crate::RegistrationRule;`) and the faces
    // `promote` lands (`crate::FlowContract`, `crate::ContractId`) were all refused as "outside the
    // fragment", which made a scaffolded host impossible to partition at all — a false "would not
    // compile" about our own output, and a hand-maintained list of allowed names is what that
    // falsehood would have to be re-guessed into on every new field (audit `M7`, §M7.39).
    // 剩下的、**不是**本包模块的一切都是 crate 根上的条目，而幽灵的 crate 根与宿主的一样带着同一段前言
    // （`pub use <registry_core>::*;`），因此注册面导入的内核词汇在两边都解析得到。这条规则出现之前实测：
    // `new_project` 自己的注册面（`use crate::{NoParts, NoPreset};`、`use crate::RegistrationRule;`）与
    // `promote` 落地的注册面（`crate::FlowContract`、`crate::ContractId`）全都被当成"够到碎片之外"拒绝，
    // 于是一个脚手架建出来的宿主**根本没法分区**——那是一句关于我们自己产物的假"编译不过"；而要想把它继续
    // 说成真的，就得为每个新字段维护一份"允许的名字"清单（审计 `M7`，§M7.39）。
    //
    // The residual is honest and left to rustc: an item a host defines in its **own crate root**
    // (and not a module) is one no ghost has. Deciding that from text means guessing what a Rust
    // item is; the compiler says it exactly, once, on the crate that is wrong.
    // 残留部分是诚实的、交给 rustc：宿主在**自己的 crate 根**上定义（且不是模块）的条目，任何幽灵都没有。
    // 用文本判定那件事等于猜"什么算一个 Rust 条目"；编译器会在出错的那个 crate 上准确地说一次。
    !first.is_empty() && !modules.iter().any(|module| module == first)
}

#[cfg(test)]
#[path = "crate_plan_tests.rs"]
mod crate_plan_tests;

/// Remove dependency lines naming crates this partition generates.
/// 去掉点名本次分区生成的 crate 的依赖行。
pub(crate) fn strip_generated_dependencies(block: &str, generated: &[String]) -> String {
    block
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !generated.iter().any(|name| {
                trimmed.starts_with(&format!("{name} ="))
                    || trimmed.starts_with(&format!("{name}="))
            })
        })
        .map(|line| format!("{line}\n"))
        .collect()
}

fn config_patch(remap: &[(String, String)]) -> String {
    let mut output = String::from(
        "# Add these to the **workspace root's** `.cargo/config.toml` (merge into an existing\n\
         # `rustflags` array; do not overwrite the file). A rustflag is per **invocation**, and cargo\n\
         # finds config from the current directory rather than from the package, so a config inside a\n\
         # ghost package only applies when cargo is run from that directory.\n\
         # 把下面这些**合并**进**工作区根**的 `.cargo/config.toml`（合并到已有的 `rustflags` 数组里，\n\
         # 不要覆盖整个文件）。rustflag 是每次调用生效的，而 cargo 从当前目录而不是从包出发找 config，\n\
         # 因此放在幽灵包里的 config 只在从那个目录跑 cargo 时才生效。\n\
         [build]\n",
    );
    let entries: Vec<String> = remap
        .iter()
        .map(|(from, to)| format!("{from}={to}"))
        .map(|pair| {
            format!(
                "\"--remap-path-prefix={pair}\"",
                pair = pair.replace('"', "")
            )
        })
        .collect();
    output.push_str(&format!("rustflags = [{}]\n", entries.join(", ")));
    output
}

/// Which subtrees **this** run must not render: the ones the declaration hands to another crate.
/// 本次运行**不得**渲染哪些子树：声明交给另一个 crate 的那些。
///
/// A host hands its claimed subtrees away and renders the rest; a **ghost** is the crate those
/// subtrees were handed to, so it renders exactly them and hands nothing away — clearing the list is
/// what keeps the ghost from skipping the fragment it exists to compile (found by the end-to-end
/// demonstration: the render, not the plan, was where the two modes disagreed).
/// 宿主把它认领的子树交出去、渲染其余；**幽灵**正是那些子树被交给的 crate，因此它渲染的恰好是它们、什么都不
/// 交出去——清空这份清单，才不会让幽灵跳过它存在的理由（由端到端演示发现：两种模式分歧的地方在渲染，不在规划）。
pub(crate) fn cut_out_for(declaration: Option<&ShapeDeclaration>, ghost: bool) -> Vec<String> {
    match (ghost, declaration) {
        (true, _) | (false, None) => Vec::new(),
        (false, Some(declaration)) => declaration.cut_subtrees(),
    }
}

/// Plan every crate a declaration asks for, or refuse by name.
/// 规划声明要求的每个 crate，或者点名拒绝。
///
/// `faces` is the tree's face list as `(host-relative source, module path)` pairs, handed in rather
/// than re-derived: the caller already holds it (the pipeline's `static_faces`, computed **before**
/// the render, which is where the mounts are needed), and a second derivation is a second chance to
/// disagree about which module a file declares.
/// `faces` 是这棵树的面清单，形如 `(宿主相对源码, 模块路径)` 对，由调用方交进来而不是重新推导：调用方手里
/// 已经有了（管线的 `static_faces`，在**渲染之前**算好，而渲染正是需要挂载的地方），而第二次推导就是第二次
/// 对"某文件声明了哪个模块"产生分歧的机会。
pub(crate) fn plan(
    package_root: &Path,
    namespace: &str,
    declaration: &ShapeDeclaration,
    faces: &[(String, String, NodeId)],
    cuts: &[HostCut],
) -> Result<Vec<PlannedCrate>, String> {
    // The same placement decision the writer uses: a standalone host is its own workspace root, so
    // its generated crates go **under** it; a host inside a workspace puts them under that root.
    // 与写入方同一个落点决定：独立宿主自己是工作区根，生成包因此落在它**之内**；工作区里的宿主则落在工作区根下。
    let partition_root = partition_roots(package_root).0;
    // Read once: the names every `crate::` path is judged against, for every declared crate.
    // 只读一次：每条 `crate::` 路径据以判定的那些名字，对所有已声明的 crate 相同。
    let modules = source_modules(package_root);
    let generated_names: Vec<String> = declaration
        .crates
        .iter()
        .map(|(name, _)| format!("{}-{name}", declaration.package_prefix))
        .chain(std::iter::once(format!(
            "{}-facade",
            declaration.package_prefix
        )))
        .collect();
    let mut planned = Vec::new();
    // Read once per plan: the module tree this run can see, and every claim the declaration makes.
    // 每次规划读一次：本次运行看得见的模块树，以及声明里写下的每一条认领。
    let all_modules = module_paths(package_root);
    let all_claims: Vec<String> = declaration
        .crates
        .iter()
        .flat_map(|(_, subtrees)| subtrees.iter().cloned())
        .collect();
    for (name, subtrees) in &declaration.crates {
        let package = format!("{}-{name}", declaration.package_prefix);
        let directory = crates_dir(&partition_root).join(&package);
        let mut mounts = Vec::new();
        let mut files = Vec::new();
        for subtree in subtrees {
            // The claim names a subtree **rooted at** that node, so the face at the node itself is
            // part of the fragment too: the host hands the whole module away (its render skips the
            // module and everything under it), and a fragment whose root face went to neither crate
            // would be a face nobody compiles.
            // 认领点名的是**以该节点为根**的子树，因此该节点自己的面也属于碎片：宿主把整个模块交出去（它的渲染
            // 跳过该模块及其下的一切），而根面若不归任何一方，就是一个没人编译的面。
            let mut claimed = 0usize;
            for (source, module_path, _) in faces.iter().filter(|(_, module, _)| {
                *module == *subtree || module.starts_with(&format!("{subtree}::"))
            }) {
                claimed += 1;
                mounts.push(PlannedMount {
                    spelling: spelling_for(package_root, source),
                    module_path: module_path.clone(),
                    source: source.clone(),
                });
                files.push(source.clone());
            }
            // A claim that matches nothing is never a quiet empty crate: either the path is misspelled,
            // or the plan was built from an earlier declaration. Both are worth stopping for — a crate
            // with no sources compiles, ships, and answers every later question wrongly.
            // 匹配不到任何东西的认领，绝不能安静地变成一个空 crate：要么路径写错了，要么这份计划是上一版声明
            // 构建的。两者都值得当场停下——没有源码的 crate 会编译、会发布，并在之后每个问题上都答错。
            // The authority for "does this claim name something" is the **module tree**, not the face
            // list: the faces this planner sees have already been narrowed to what the host keeps, so a
            // legitimate claim on a deeper subtree (`control::left`) matches nothing there — measured,
            // that false refusal is what this test first did. A claim that names no module at all is a
            // misspelling or a stale plan; one that names a module with no faces is empty but real.
            // "这个认领点名了东西没有"的权威是**模块树**，不是面表：本规划器看到的面已经被收窄成宿主保留的
            // 那些，因此对更深子树（`control::left`）的合法认领在那里一个也匹配不到——实测：这条误拒是本判据
            // 一开始的行为。点名不到任何**模块**的认领是写错或计划过期；点名到模块但没有面的，是空但真实。
            let names_a_module = all_modules
                .iter()
                .any(|module| module == subtree || module.starts_with(&format!("{subtree}::")));
            // Scope of this refusal: the **whole declaration**, not this one claim. A partition's
            // fragment compiles one subtree, so the other claims legitimately name modules that are
            // not in *its* tree — measured: per-claim refusal failed every multi-claim partition
            // (`control` "names no module in this host's tree" while building the `panel::frame`
            // crate). A declaration that names nothing here is still refused, with the nearest module.
            // 拒绝的适用范围是**整份声明**，不是这一条认领。划分出来的片段只编译一棵子树，因此其余认领合法地点名
            // 不在**它**树里的模块——实测：逐条拒绝让每一个多认领的划分都失败（构建 `panel::frame` 那个 crate 时
            // `control` 被报"在本宿主的树里没有对应模块"）。整份声明一条都点不到模块时仍然拒绝，并给出最近的模块。
            let any_claim_names_a_module = all_claims.iter().any(|claim| {
                all_modules
                    .iter()
                    .any(|module| module == claim || module.starts_with(&format!("{claim}::")))
            });
            if claimed == 0 && !names_a_module && !any_claim_names_a_module {
                let nearest =
                    super::shape_decl::closest(subtree, modules.iter().map(String::as_str))
                        .map(|module| format!("; the nearest module is `{module}`"))
                        .unwrap_or_default();
                return Err(format!(
                    "add_crates: `{subtree}` names no module in this host's tree, so `{name}` would \
                     carry nothing{nearest}. Either the path is misspelled, or the plan predates this \
                     declaration — run `xirang check` and read what it says"
                ));
            }
        }
        mounts.sort_by(|left, right| left.module_path.cmp(&right.module_path));
        files.sort();
        files.dedup();
        // The proper prefixes of the claims that are **faces**: their identities are what a shell can
        // carry so a fragment's `parent:` still resolves (a non-face prefix is a plain directory and
        // has no identity for anyone to name).
        // 认领里**是注册面**的真前缀：它们的身份正是壳可以携带、好让碎片的 `parent:` 仍然解析的东西（不是面的
        // 前缀只是普通目录，没有任何身份可被点名）。
        let mut ancestors: Vec<(String, NodeId)> = Vec::new();
        for subtree in subtrees {
            let mut prefix = String::new();
            for segment in subtree.split("::") {
                if !prefix.is_empty()
                    && let Some((_, _, id)) = faces.iter().find(|(_, module, _)| *module == prefix)
                {
                    ancestors.push((prefix.clone(), *id));
                }
                prefix = if prefix.is_empty() {
                    segment.to_owned()
                } else {
                    format!("{prefix}::{segment}")
                };
            }
        }
        ancestors.sort_by(|left, right| left.0.cmp(&right.0));
        ancestors.dedup();
        // The reachability check comes **after** both, so a refusal can name every file it read and
        // speak about the crate as a whole (a reference between two subtrees of one crate is fine, and
        // so is an ancestor's `NODE_ID`, which the shell carries).
        // 可及性检查排在两者之后，这样拒绝能点名它读过的每份文件、并就整个 crate 说话（同一个 crate 的两棵子树
        // 之间的引用是允许的，祖先的 `NODE_ID` 也允许——壳携带它）。
        let ancestor_modules: Vec<String> =
            ancestors.iter().map(|(module, _)| module.clone()).collect();
        for file in &files {
            check_reachability(
                package_root,
                name,
                file,
                subtrees,
                &ancestor_modules,
                &modules,
            )?;
        }
        let mut remap: Vec<(String, String)> = mounts
            .iter()
            .map(|mount| remap_for(&mount.spelling, &mount.source))
            .collect();
        remap.sort();
        remap.dedup();
        let lib_rs = ghost_lib_rs(namespace);
        let aliases_rs = aliases_rs(subtrees);
        let build_rs = ghost_build_rs(package_root, &directory, namespace, subtrees, cuts);
        let cargo_toml = ghost_cargo_toml(package_root, &package, &directory, &generated_names)?;
        let config_patch = config_patch(&remap);
        planned.push(PlannedCrate {
            name: name.clone(),
            package,
            namespace: namespace.to_owned(),
            directory,
            subtrees: subtrees.clone(),
            ancestors,
            mounts,
            remap,
            lib_rs,
            aliases_rs,
            build_rs,
            cargo_toml,
            config_patch,
        });
    }
    Ok(planned)
}

/// `(module path, crate name)` for every module in the tree: the crate that compiles it.
/// `(模块路径, crate 名)` 对，覆盖树里每个模块：编译它的那个 crate。
///
/// The facade needs this to rewrite `crate::<module>` into `<owner>::<module>`, because `crate::` inside
/// a facade means the facade itself. One implementation, two callers (the render pass and the facade
/// planner): a claim, its **root face** and its ancestor shells all belong to that ghost's package, and
/// every other module — including the host's own faces — belongs to the host.
/// facade 需要它来把 `crate::<模块>` 改写成 `<属主>::<模块>`，因为 facade 里的 `crate::` 指的是 facade
/// 自己。一处实现、两个调用方（渲染 pass 与 facade 规划器）：一条认领、它的**根面**与它的祖先壳都归那个
/// 幽灵的包，其余每个模块——**包括宿主自己的面**——归宿主。
pub(crate) fn owners(
    host_package: &str,
    faces: &[(String, String, NodeId)],
    planned: &[PlannedCrate],
) -> Vec<(String, String)> {
    let host_crate = host_package.replace('-', "_");
    let mut owner_map: BTreeMap<String, String> = faces
        .iter()
        .map(|(_, module, _)| (module.clone(), host_crate.clone()))
        .collect();
    for planned in planned {
        let owner = planned.package.replace('-', "_");
        for mount in &planned.mounts {
            owner_map.insert(mount.module_path.clone(), owner.clone());
        }
        // **Ancestor shells do not go in the owner table.** A shell is a module the ghost builds to carry
        // the ancestor's `NODE_ID`, but the *face* at that module is compiled by whoever keeps it — the
        // host — so face-level items (`REGISTRATION`, its rule) must be spelled with the host's crate. The
        // first facade build measured the alternative: it emitted `app_widgets::panel::REGISTRATION`, a
        // shell that carries no `REGISTRATION` (`cannot find value REGISTRATION in module app_widgets::panel`).
        // **祖先壳不进属主表。** 壳是幽灵为携带祖先 `NODE_ID` 而搭的模块，但那个模块上的**面**是由留住它的
        // 那个 crate（宿主）编译的，因此面级别的项（`REGISTRATION` 及其规则）必须用宿主的 crate 拼。第一次
        // facade 构建量到了另一种做法：它发 `app_widgets::panel::REGISTRATION`——一个不携带 `REGISTRATION`
        // 的壳（`cannot find value REGISTRATION in module app_widgets::panel`）。
        for _ in &planned.ancestors {}
    }
    owner_map.into_iter().collect()
}

/// The `#[path]` spelling one mounted face file needs: the host's **absolute** path.
/// 一份被挂载的面文件所需的 `#[path]` 拼写：宿主的**绝对**路径。
///
/// It used to be a relative walk from the ghost's own directory, and that cannot work: the generated
/// tree lives in the build script's `OUT_DIR`, which cargo chooses, so `#[path]` resolves relative to a
/// directory the planner never sees. The first end-to-end build measured exactly that
/// (`<out>/panel/frame/../../../app/src/…`: three `../` short of a file that is seven levels away).
/// The host's own generated tree already spells its mounts absolutely, for the same reason. Identity is
/// unaffected because the remap strips the host's `src` prefix back off, so `file!()` still reads as the
/// host-relative source the records name (audit `M7`, P3.2/§M7.15).
/// 它过去是"从幽灵自己目录出发的相对走法"，而那条路不成立：生成树住在构建脚本的 `OUT_DIR`（由 cargo 决定），
/// 因此 `#[path]` 相对一个规划器根本看不见的目录解析。第一次端到端构建量到的正是这个
/// （`<out>/panel/frame/../../../app/src/…`：离那个文件差了七层里的三层）。宿主自己的生成树早就用绝对路径
/// 拼它的挂载，原因相同。身份不受影响，因为 remap 会把宿主的 `src` 前缀摘回去，于是 `file!()` 读起来仍是记录
/// 点名的那个宿主相对源码（审计 `M7`，P3.2/§M7.15）。
fn spelling_for(host_dir: &Path, source: &str) -> String {
    host_dir.join("src").join(source).display().to_string()
}

/// The walk from one directory to one file, both absolute, as a `/`-separated relative path.
/// 从一个目录走到一个文件的相对路径（两者都是绝对路径），用 `/` 分隔。
// Only the authoring CLI face needs this (the facade spells its sibling dependencies relatively);
// the build path no longer walks at all, because a mount is an absolute path (audit `M7`, §M7.37).
// 生成包自己的清单里每一条 `path` 都要相对**它自己**的目录重拼（幽灵与 facade 都走这条），而清单是构建面
// 也算出来的计划的一部分 ⇒ 构建面同样需要它。挂载本身仍是绝对路径（审计 `M7`，§M7.37），这条只关乎依赖表。
#[cfg(any(
    feature = "build",
    feature = "cli",
    feature = "studio",
    feature = "mcp"
))]
pub(crate) fn relative_walk(from_dir: &Path, to_file: &Path) -> Option<String> {
    let from: Vec<_> = from_dir.components().collect();
    let to: Vec<_> = to_file.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut walk: Vec<String> = Vec::new();
    for _ in common..from.len() {
        walk.push("..".to_owned());
    }
    for component in &to[common..] {
        walk.push(component.as_os_str().to_string_lossy().into_owned());
    }
    (!walk.is_empty()).then(|| walk.join("/"))
}
