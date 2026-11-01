//! Recursive walk over the discovered module tree, one render pass.
//! 对已发现模块树的递归遍历，即一次渲染过程。

use std::fmt::Write as _;
use std::path::Path;

use xirang_kernel::lexicon;

use super::modules::emit_face_declaration;
use crate::build_method::{
    Node, SourceScope, module_feature, node_id, relative_display, source_is_active,
};

/// Render every node into the module tree shared by the compiler and editor.
/// 将所有节点渲染到编译器与编辑器共用的模块树中。
pub(super) fn render_nodes(
    output: &mut String,
    src: &Path,
    scope: &SourceScope,
    nodes: &[Node],
    shape: ShapeRender<'_>,
    module_directory: &Path,
    module_files: &mut Vec<(String, String)>,
) {
    let mut pass = RenderPass {
        src,
        scope,
        shape,
        module_directory,
        module_files,
    };
    for node in nodes {
        render_node(output, &mut pass, node, 0, false, "", &node.name);
    }
}

/// The crate shape as a **render** sees it: what this crate does not compile, and what it is the one
/// to compile (audit `M7`, P3.2).
/// crate 形状在**渲染**眼里的样子：本 crate 不编译什么，以及它是唯一的编译者的是什么（审计 `M7`，P3.2）。
#[derive(Clone, Copy, Default)]
pub(crate) struct ShapeRender<'a> {
    /// Subtrees that belong to another crate: this crate emits no module for them, though the records
    /// still carry every face (the tree is unchanged; what changes is which crate compiles it).
    /// 属于另一个 crate 的子树：本 crate 不为它们发射模块，而记录仍带着每一个面（树没变，变的是"由哪个 crate
    /// 编译"）。
    pub(crate) cut_out: &'a [String],
    /// The subtrees this crate is the one to compile, when it is a ghost: every other node is left
    /// out, and the nodes above a claimed subtree become **empty container modules**.
    /// 本 crate 是唯一编译者的那些子树（当它是幽灵时）：其余节点一律不发射，而认领子树**之上**的节点变成
    /// **空的容器模块**。
    pub(crate) only: Option<&'a [String]>,
    /// `(module path, #[path] spelling)` for the faces a ghost mounts from the host.
    ///
    /// A host mounts its own files by their portable path, which is right there because the file *is*
    /// in this package. A ghost's file is not: it must spell a walk out of its own package and into
    /// the host's `src`, and that spelling is what `crate_plan` computed — because `file!()` reports
    /// it, and identity is `hash(namespace, source path, name)`, the spelling *is* the identity.
    /// `(模块路径, #[path] 拼写)` 对，供幽灵挂载宿主的文件。
    ///
    /// 宿主按可移植路径挂载自己的文件，这在那里是对的，因为文件**就在**这个包里。幽灵的文件不在：它必须写出
    /// "走出自己的包、走进宿主 `src`"的拼写，而那个拼写是 `crate_plan` 算出来的——因为 `file!()` 报告的就是
    /// 它，而身份是 `hash(命名空间, 源码路径, 名字)`，**拼写就是身份**。
    pub(crate) mounts: &'a [(String, String)],
    /// Whether this run renders the **facade**: the cross-crate half only (no modules, no faces).
    /// 本次运行是否渲染 **facade**：只渲染跨 crate 那一半（不发模块、不发面）。
    pub(crate) facade: bool,
    /// `(module path, crate name)` for every module in the tree: the crate that compiles it. In facade
    /// mode every `crate::<module>` becomes `<owner>::<module>`, because `crate::` there means the
    /// facade itself (audit `M7`, §M7.33).
    /// `(模块路径, crate 名)` 对，覆盖树里每个模块：编译它的那个 crate。facade 模式里每个
    /// `crate::<模块>` 都变成 `<owner>::<模块>`，因为那里的 `crate::` 指的是 facade 自己（审计 `M7`，§M7.33）。
    pub(crate) owners: &'a [(String, String)],
    /// `(module path, identity bytes)` for the faces a ghost's shell modules stand for.
    ///
    /// A fragment's `parent: crate::<ancestor>::NODE_ID` has to resolve in a crate that does not
    /// compile the ancestor, so the shell carries that one constant and nothing else. It is the id
    /// the host's build baked, so the fragment's parent link is the same node the host sees.
    /// `(模块路径, 身份字节)` 对，供幽灵的壳模块代表的那些面。
    ///
    /// 碎片的 `parent: crate::<祖先>::NODE_ID` 必须在一个不编译祖先的 crate 里解析得到，因此壳只携带那一个
    /// 常量、别的什么都不带。它就是宿主的构建烤进去的那个 id，于是碎片的父级链接与宿主看见的是同一个节点。
    pub(crate) ancestors: &'a [(String, [u8; 16])],
}

impl<'a> ShapeRender<'a> {
    /// The whole tree, nothing handed away: what a host without a declaration renders.
    /// 整棵树、什么都不交出去：没有声明的宿主渲染的东西。
    pub(crate) fn whole() -> Self {
        Self {
            cut_out: &[],
            only: None,
            mounts: &[],
            ancestors: &[],
            facade: false,
            owners: &[],
        }
    }
}

/// Mutable state threaded through one render pass.
/// 一次渲染过程中传递的可变状态。
struct RenderPass<'a> {
    src: &'a Path,
    scope: &'a SourceScope,
    shape: ShapeRender<'a>,
    /// Where the generated containers are materialized, and what was written there.
    /// 生成的容器落盘的位置，以及往那里写了什么。
    ///
    /// The walk **emits** each container into its own file as it descends, instead of writing one big
    /// source and parsing that source again to split it (`stem 2`, step 3). The round trip cost
    /// **3.710 s on a 1,000-face host** — measured by short-circuiting it and reverting — and it existed
    /// only because the splitter could not see what the walk already knew.
    /// 这次遍历**在下行的同时**把每个容器发射进它自己的文件，而不是先写一大份源码、再解析那份源码把它切开
    /// （`stem 2` 第③步）。那趟往返在 1,000 面的宿主上要 **3.710 s**——用"短路再还原"量出来的——而它存在的
    /// 唯一原因是：切分器看不见这次遍历本来就知道的东西。
    module_directory: &'a Path,
    module_files: &'a mut Vec<(String, String)>,
}

fn render_node(
    output: &mut String,
    pass: &mut RenderPass<'_>,
    node: &Node,
    depth: usize,
    selected_ancestor: bool,
    parent_chain: &str,
    module_path: &str,
) {
    // A subtree another crate owns is not rendered here — and returning skips its whole subtree,
    // which is what "the same face is not compiled twice" means (audit `M7`, P3.2).
    // 另一个 crate 拥有的子树不在这里渲染——而直接返回会连整棵子树一起跳过，这正是"同一个面不编译两遍"的
    // 含义（审计 `M7`，P3.2）。
    if pass.shape.facade {
        // A facade compiles no faces: no module is rendered into it at all. What it carries is the
        // cross-crate half, which `pass.rs` emits after this walk.
        // facade 不编译任何面：一个模块都不渲染进它。它携带的是跨 crate 那一半，由 `pass.rs` 在这次遍历之后
        // 发射。
        return;
    }
    if pass.shape.cut_out.iter().any(|cut| cut == module_path) {
        return;
    }
    // A ghost compiles one fragment and nothing else: keep the claimed subtrees, keep the nodes above
    // them (as shells), and leave the rest out.
    // 幽灵只编译一个碎片：保留认领的子树、保留它们**之上**的节点（作为壳），其余一律不发射。
    let below_a_claim = pass.shape.only.is_some_and(|only| {
        only.iter()
            .any(|claim| claim.starts_with(&format!("{module_path}::")))
    });
    let inside_a_claim = pass.shape.only.is_some_and(|only| {
        only.iter()
            .any(|claim| claim == module_path || module_path.starts_with(&format!("{claim}::")))
    });
    if pass.shape.only.is_some() && !below_a_claim && !inside_a_claim {
        return;
    }
    // A node **above** a claim is a shell: it keeps the module path so the claimed faces resolve
    // through it, and mounts no file of its own — mounting an ancestor's file would register that
    // face a second time, in this crate's registry (audit `M7`, P3.2).
    // 认领**之上**的节点是壳：它保住模块路径好让认领的面经它解析，而不挂载自己的文件——挂载祖先的文件会把那个
    // 面第二次注册进本 crate 的注册机（审计 `M7`，P3.2）。
    let shell = below_a_claim;
    if node
        .file
        .as_ref()
        .is_some_and(|file| relative_display(pass.src, file) == "registry_core/macros/macros.rs")
    {
        return;
    }
    let selected_here = selected_ancestor
        || node_id(pass.src, node).is_some_and(|id| {
            pass.scope
                .roots
                .as_ref()
                .is_some_and(|roots| roots.contains(&id))
        });
    if !pass.scope.includes(pass.src, node, selected_ancestor) {
        return;
    }
    let include_source = !shell && source_is_active(pass.src, node, pass.scope, selected_ancestor);
    let indent = "    ".repeat(depth);
    if depth == 0 && node.name == crate::build_method::DEMO_ONLY_DIRECTORY {
        let cfg = format!(
            "#[cfg(feature = {:?})]",
            crate::build_method::DEMO_ONLY_FEATURE
        );
        writeln!(output, "{indent}{cfg}").unwrap();
    }
    if let Some(feature) = module_feature(pass.src, node) {
        writeln!(output, "{indent}#[cfg(feature = {feature:?})]").unwrap();
    }
    writeln!(
        output,
        "{indent}#[allow(dead_code, unused_imports, ambiguous_glob_reexports, clippy::items_after_test_module, clippy::module_inception)]"
    )
    .unwrap();
    // A face file is loaded as a real module through `#[path]`, never through
    // `include!`, and never as a copy: that is what preserves a face file's
    // `//!` header, since an `include!` expansion cannot introduce inner
    // attributes.
    // 注册面文件通过 `#[path]` 以真实模块载入，不走 `include!`，也不是副本：
    // 这既保住了面文件的 `//!` 头（`include!` 展开无法引入 inner attribute），
    // 也让编译器直接读取作者正在编辑的那个文件。
    //
    // Generated containers become real module files before compilation, so rustc and
    // rust-analyzer follow the same paths without shadow declarations.
    // 生成容器在编译前成为真实模块文件，因此 rustc 与 rust-analyzer 沿同一路径读取，不需要影子声明。
    // The declaration macros derive `source` from `file!()` and the registry
    // name from `module_path!()`, so no per-face constants are injected here.
    // For a leaf face the file is loaded under its own name, which keeps the
    // public module path unchanged. A face that owns child registries must also
    // be their container, so it loads into a same-named child module and
    // re-exports; only that case gains a module-path segment.
    // 声明宏从 `file!()` 推导 `source`、从 `module_path!()` 推导注册机名，因此
    // 这里不再注入逐面常量。叶子面以自身名字载入，公开模块路径保持不变；拥有
    // 子注册机的面必须同时充当容器，于是载入到同名子模块再重导出——只有这种
    // 情况会多出一段模块路径。
    // A ghost mounts the host's file through the spelling the plan computed; a host mounts its own
    // file by its portable path. The lookup is by module path because that is what both sides agree
    // on, and a face this run renders without a planned mount keeps the portable path — a wrong
    // identity cannot pass silently, because the build bakes one id and the compiler computes the
    // other (audit `M7`, §M7.15).
    // 幽灵经规划算出的拼写挂载宿主的文件；宿主按可移植路径挂载自己的文件。查表用模块路径，因为那是两边一致
    // 的东西；本次渲染到、却没有规划挂载的面保留可移植路径——错的身份不可能悄悄通过，因为构建烤进一个 id、
    // 编译器算出另一个（审计 `M7`，§M7.15）。
    let mounted = pass
        .shape
        .mounts
        .iter()
        .find(|(path, _)| path == module_path)
        .map(|(_, spelling)| spelling.clone());
    let absolute = match (mounted, node.file.as_ref()) {
        (Some(spelling), _) => Some(spelling),
        (None, Some(file)) => Some(xirang_kernel::declaration::portable_path(
            &file.to_string_lossy(),
        )),
        (None, None) => None,
    };
    // The container key, spelled exactly as the splitter spelled it: `{parent_key}{len}_{name}`, so a
    // name containing `_` cannot collide with a deeper one (`a_b/c` vs `a/b_c`) and so the file names in
    // `xirang_modules/` stay the ones every earlier build wrote.
    // 容器键，与切分器的拼法逐字一致：`{父键}{名字长度}_{名字}`，这样含 `_` 的名字不会与更深一层撞上
    // （`a_b/c` 对 `a/b_c`），而 `xirang_modules/` 里的文件名仍是此前每一次构建写下的那些。
    let key = format!("{parent_chain}{}_{}", node.name.len(), node.name);
    if let (true, Some(absolute)) = (include_source, absolute.as_ref())
        && node.children.is_empty()
    {
        // A leaf face is declared at its own indent, so only a nested one needs
        // the IDE to be given a second, crate-root view of the same file.
        // 叶子面声明在自己的缩进处，因此只有嵌套的那些才需要给 IDE 补一份
        // crate 根视角的同一文件。
        emit_face_declaration(output, node, absolute, depth);
        return;
    }
    // A host that turns on `#![warn(missing_docs)]` sees this generated file as
    // its own source, so inline container submodules need docs too: a leaf face's
    // module takes its docs from the face file's `//!` header, but a container
    // module has no file of its own to take them from.
    // 开启 `#![warn(missing_docs)]` 的宿主会把这份生成文件当成自己的源码，因此内联容器
    // 子模块也需要文档：叶子面的模块文档来自面文件的 `//!` 头，而容器模块没有自己的文件
    // 可依托。
    writeln!(
        output,
        "{indent}/// The `{}` registration subtree this build generated from the folder layout.",
        node.name
    )
    .unwrap();
    writeln!(
        output,
        "{indent}/// 本次构建由文件夹布局生成的 `{}` 注册子树。",
        node.name
    )
    .unwrap();
    // The container's own text goes to its own file, and the parent keeps the two lines that mount it.
    // Byte-for-byte this is what the old splitter produced: it inserted `#[path = …]` at the
    // declaration's first token — i.e. **after** the indentation that precedes it — and replaced the
    // braces with `;`, which left `pub mod X;` un-indented on its own line.
    // 容器自己的文本进它自己的文件，父级只留挂载它的那两行。逐字节地说，这正是旧切分器产出的东西：它把
    // `#[path = …]` 插在声明的第一个 token 处——也就是**缩进之后**——并把花括号换成 `;`，于是 `pub mod X;`
    // 独占一行且没有缩进。
    writeln!(
        output,
        "{indent}#[path = {:?}]",
        pass.module_directory.join(format!("{key}.rs"))
    )
    .unwrap();
    writeln!(output, "pub mod {};", node.name).unwrap();
    let mut container = String::new();
    let output = &mut container;
    // Every mounted module carries a `Subtree` marker, so a host's `add_crates.rs` can name this
    // subtree as a real path: that is what makes the declaration complete in an editor, a typo a
    // compile error, and the shape the build reads the same shape the compiler checked.
    // 每个挂载的模块都带一个 `Subtree` 标记，因此宿主的 `add_crates.rs` 能把这棵子树当真实路径点名：这正是
    // 让声明在编辑器里可补全、写错就是编译错误，并让构建读到的形状与编译器查过的形状是同一份的原因。
    writeln!(
        output,
        "{inner}pub const SUBTREE: ::xirang_toolchain::run_method::Subtree = \
         ::xirang_toolchain::run_method::Subtree::new(module_path!());",
        inner = "    ".repeat(depth + 1)
    )
    .unwrap();
    // A shell stands in for a face this crate does not compile: it carries that face's identity so a
    // fragment's `parent:` still resolves, and carries nothing else (audit `M7`, P3.2).
    // 壳代替的是本 crate 不编译的那个面：它携带那个面的身份，好让碎片的 `parent:` 仍然解析，别的什么都不带
    // （审计 `M7`，P3.2）。
    if shell
        && let Some((_, bytes)) = pass
            .shape
            .ancestors
            .iter()
            .find(|(path, _)| path == module_path)
    {
        writeln!(
            output,
            "{inner}/// The identity of the registration face this module stands for in the host's tree.\n\
             {inner}/// 本模块在宿主的树里所代表的注册面的身份。\n\
             {inner}pub const NODE_ID: ::xirang_toolchain::run_method::registry_core::NodeId = \
             ::xirang_toolchain::run_method::registry_core::NodeId::from_raw({bytes:?});",
            inner = "    ".repeat(depth + 1)
        )
        .unwrap();
    }
    if let (true, Some(absolute)) = (include_source, absolute.as_ref()) {
        // A container face owns child registries, so its file always loads into
        // a child module one level deeper: it always needs the IDE view.
        // 容器面同时承载子注册机，其文件总是载入到深一层的子模块：它始终需要
        // IDE 视角。
        emit_face_declaration(output, node, absolute, depth + 1);
    }
    for child in &node.children {
        let child_path = format!("{module_path}::{}", child.name);
        render_node(
            output,
            pass,
            child,
            depth + 1,
            selected_ancestor || selected_here,
            &key,
            &child_path,
        );
    }
    if depth == 0 && node.name == lexicon::SCOPE_REGISTRATION_MODULE {
        let inner = "    ".repeat(depth + 1);
        for child in node.children.iter().filter(|child| child.name != "macros") {
            if let Some(feature) = module_feature(pass.src, child) {
                writeln!(output, "{inner}#[cfg(feature = {feature:?})]").unwrap();
            }
            writeln!(output, "{inner}pub use {}::*;", child.name).unwrap();
        }
    }
    // The file holds the text **between** the braces — which ends with the indentation that preceded
    // the closing brace, and not with the brace itself, exactly as the old splitter sliced it.
    // 文件装的是两个花括号**之间**的文本——它以闭括号之前的缩进结尾，而不是以闭括号结尾，与旧切分器切出的
    // 一模一样。
    write!(container, "{indent}").unwrap();
    pass.module_files.push((key, container));
}
