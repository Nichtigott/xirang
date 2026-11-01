//! Build-script input paths and their environment bindings.
//! 构建脚本输入路径及其环境绑定。

use std::env;
use std::path::PathBuf;

use super::{SourceLayout, source_layout};

/// One build run's resolved paths.
/// 一次构建运行解析出的路径。
pub(crate) struct BuildInput {
    pub(crate) manifest: PathBuf,
    /// Where the faces live. A layout problem is carried rather than raised here,
    /// because the two callers report it differently and neither wants a panic
    /// from a constructor.
    /// 注册面住在哪里。布局问题被携带而不是在这里抛出，因为两个调用方报告它的方式不同，而且都
    /// 不想要构造函数 panic。
    pub(crate) layout: Result<SourceLayout, String>,
    pub(crate) out_dir: PathBuf,
    /// Whether to emit `cargo:` directives on stdout. Cargo build scripts set
    /// this; standalone CLI runs leave it off.
    /// 是否在 stdout 输出 `cargo:` 指令。Cargo build script 置位；独立 CLI
    /// 运行时不输出。
    pub(crate) emit_cargo_directives: bool,
    /// The subtrees this package compiles instead of the whole host (`None` = the whole host), and
    /// whether it is the facade — the cross-crate half. Both come from the generator as **arguments**
    /// now: they used to arrive through `XIRANG_SHAPE_*`, which a generated build script can only
    /// set with `unsafe { std::env::set_var(…) }` under Rust 2024.
    /// 这个包编译哪些子树（`None`＝整个宿主），以及它是不是 facade（跨 crate 那一半）。两者现在由生成器
    /// 以**参数**给出：过去经 `XIRANG_SHAPE_*` 传递，而生成的构建脚本在 Rust 2024 下只能靠
    /// `unsafe { std::env::set_var(…) }` 设置它们。
    pub(crate) only: Option<String>,
    pub(crate) facade: bool,
    /// The host entry's graft cuts, when this package is a **generated** one.
    /// 宿主入口的 graft 切口——当本包是**生成的**包时。
    ///
    /// Empty for a host, which reads its own entry. A generated package has no entry of its own, so
    /// the generator hands the cuts over as data (see [`HostCut`]).
    /// 宿主为空——它读自己的入口。生成包没有自己的入口，因此生成器把切口当作数据交过来
    /// （见 [`HostCut`]）。
    pub(crate) host_cuts: Vec<HostCut>,
    /// The host's **library** name, when a generated build script knows it from the plan.
    /// 宿主**库**名，当生成的构建脚本从计划中知道它时。
    pub(crate) host_crate: Option<String>,
}

/// One graft cut a generated package inherits from its host entry.
/// 生成包从宿主入口继承的一条 graft 切口。
///
/// A generated package has no entry of its own — the release shape carries only the sources it
/// compiles — so its cuts arrive as **data** and are read here exactly as if the author had written
/// them in that package. Without this the release shape's ghosts carried an empty table: measured on a
/// host whose plan replaces a face inside one claimed subtree, the ghost that compiles that face
/// emitted **no** cut for it while the development shape emitted it, so a published crate compiled the
/// copied source and never applied the graft the plan promised (audit `M7`, §M7.55).
/// 生成包没有自己的入口——发布形状只携带它编译的那些源码——因此它的切口以**数据**到来，在这里被读成
/// 作者写在该包里的样子。没有这一条，发布形状的幽灵携带空表：在一份"计划替换了某个被认领子树内的面"的
/// 宿主上实测，编译那个面的幽灵**不为它**发射任何切口，而开发形状会发射；于是发布出去的 crate 编译了
/// 复制过来的源码、却从不应用计划承诺的嫁接（审计 `M7`，§M7.55）。
///
/// The fields are the parser's own view of one `cut(…) graft(…)` clause, so a generated package answers
/// every later question — attribution, the contract assertion, the `cfg` gate — through the same code
/// paths a hand-written entry does.
/// 字段就是解析器对一条 `cut(…) graft(…)` 子句的看法，因此生成包在之后每个问题上——归属、契约断言、
/// `cfg` 门控——都走与手写入口相同的代码路径。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCut {
    /// The cut target, spelled the way the host entry spelled it.
    /// 切口目标，按宿主入口的拼写。
    pub cut: String,
    /// The far endpoint of a sibling range, when the host wrote one.
    /// 兄弟区间的远端端点——当宿主写了区间时。
    pub cut_end: Option<String>,
    /// The replacement selector.
    /// 替换侧的选择器。
    pub graft: String,
    /// Whether the replacement covers the whole subtree at the cut target.
    /// 替换是否覆盖切口目标的整棵子树。
    pub full: bool,
    /// Whether the host wrote `cut(…)`/`graft(…)` (Rust expressions) rather than selector strings.
    /// 宿主写的是 `cut(…)`/`graft(…)`（Rust 表达式）还是选择器字符串。
    pub typed: bool,
    /// The `cfg` gate the declaration carried, if any.
    /// 该声明携带的 `cfg` 门控（若有）。
    pub cfg: Option<String>,
    /// Where the clause sits in the **host's** entry, so a diagnostic can still point at the author's
    /// line rather than at this generated file.
    /// 该子句在**宿主**入口里的位置，因此诊断仍能指向作者那一行，而不是这份生成文件。
    pub line: usize,
    /// The column within [`HostCut::line`].
    /// [`HostCut::line`] 内的列号。
    pub column: usize,
}

impl HostCut {
    /// The parsed declaration this cut stands for.
    /// 这条切口所代表的已解析声明。
    pub(crate) fn syntax(&self) -> xirang_kernel::syntax::GraftSyntax {
        xirang_kernel::syntax::GraftSyntax {
            cut: self.cut.clone(),
            cut_end: self.cut_end.clone(),
            graft: self.graft.clone(),
            full: self.full,
            location: xirang_kernel::syntax::SyntaxLocation {
                line: self.line,
                column: self.column,
            },
            cfg: self.cfg.clone(),
            spans: None,
            expressions: self.typed.then(|| xirang_kernel::syntax::GraftExpressions {
                cut: self.cut.clone(),
                cut_end: self.cut_end.clone(),
                graft: self.graft.clone(),
            }),
        }
    }
}

impl BuildInput {
    pub(crate) fn from_environment() -> Self {
        let manifest =
            PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
        Self::new(manifest, out_dir, true)
    }

    /// The input for one package root, as every non-build-script caller has it.
    /// 从每个非构建脚本调用方都持有的包根构造输入。
    pub(crate) fn new(manifest: PathBuf, out_dir: PathBuf, emit_cargo_directives: bool) -> Self {
        let layout = source_layout(&manifest);
        Self {
            manifest,
            layout,
            out_dir,
            emit_cargo_directives,
            only: None,
            facade: false,
            host_cuts: Vec::new(),
            host_crate: None,
        }
    }

    /// The same input, for a fragment a partition generated: it compiles `only` instead of the whole
    /// host, or it is the facade.
    /// 同一个输入，但用于划分生成的碎片：它编译 `only` 而不是整个宿主，或者它就是 facade。
    pub(crate) fn with_shape(mut self, only: Option<String>, facade: bool) -> Self {
        self.only = only;
        self.facade = facade;
        self
    }

    /// The same input, plus the host's graft cuts as data.
    /// 同一个输入，外加作为数据到来的宿主 graft 切口。
    pub(crate) fn with_host_cuts(mut self, cuts: Vec<HostCut>) -> Self {
        self.host_cuts = cuts;
        self
    }

    pub(crate) fn with_host_crate(mut self, name: Option<String>) -> Self {
        self.host_crate = name;
        self
    }
}
