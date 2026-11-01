//! The crate shape a host declares, written as ordinary Rust (audit `M7`, P3.1).
//! 宿主声明的 crate 形状，用普通 Rust 写出来（审计 `M7`，P3.1）。
//!
//! A shape is a [`Shape`] value in the host's `add_crates.rs`:
//!
//! ```ignore
//! use xirang_toolchain::run_method::{Crate, Shape};
//!
//! pub const SHAPE: Shape = Shape {
//!     package_prefix: "myapp",
//!     crates: &[
//!         Crate::named("widgets").at(&[crate::control::object::SUBTREE]),
//!     ],
//! };
//! ```
//!
//! It is **ordinary Rust on purpose**, and that is the whole design: the subtree paths are real
//! paths to real items, so an editor completes them, a typo is a compile error, and a wrong kind of
//! item is a type error. A macro taking the same declaration would have been a token tree the editor
//! can only guess at — and the one thing a declaration file must get right is which subtree it names.
//! 它**有意是普通 Rust**，而这正是全部设计：子树路径是指向真实条目的真实路径，因此编辑器能补全、写错是编译
//! 错误、类型不对是类型错误。同样的声明若用宏写，就成了一棵编辑器只能猜的 token 树——而声明文件最不能弄错的
//! 恰恰是"它点名了哪棵子树"。
//!
//! The generated tree writes one [`Subtree`] marker into **every** module it mounts (see
//! `build_method`'s renderer), so `crate::control::object::SUBTREE` exists exactly when that subtree
//! exists. The rule about what a valid shape is lives in the kernel, once, because the build script
//! checks the same declaration as text before anything compiles.
//! 生成树会给它挂载的**每一个**模块写一个 [`Subtree`] 标记（见 `build_method` 的渲染器），因此
//! `crate::control::object::SUBTREE` 存在当且仅当那棵子树存在。什么算合法形状这条规则住在内核里、只有一份，
//! 因为构建脚本在编译之前就把同一份声明当文本查一遍。

use xirang_kernel::registry_core::{DeclaredCrate, validate_shape};

/// One registration subtree, as the generated tree marks it.
/// 一棵注册子树，按生成树给出的标记。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subtree {
    /// The `::`-separated module path the compiler reported for that module.
    /// 编译器为那个模块报出的 `::` 分隔模块路径。
    module: &'static str,
}

impl Subtree {
    /// Mark the module this const is written into.
    /// 标记这个 const 所在的模块。
    ///
    /// Called by generated code with `module_path!()`, which is why a declaration cannot name a
    /// subtree that does not exist: there would be no marker to name.
    /// 由生成代码以 `module_path!()` 调用，这正是声明无法点名一棵不存在的子树的原因：那里没有标记可点。
    pub const fn new(module: &'static str) -> Self {
        Self { module }
    }

    /// The module path this marker stands for.
    /// 这个标记所代表的模块路径。
    pub const fn module(self) -> &'static str {
        self.module
    }
}

/// One crate a host asks for: a name, and what it claims.
/// 宿主要求的一个 crate：一个名字，以及它认领的东西。
///
/// There are **two** ways to name a claim, and the difference is not a matter of taste — it is what
/// each node in the generated tree actually has:
///
/// * a **directory node** (a subtree the build assembled, like `control::object`) has no identity of
///   its own, so the generated tree marks it with [`Subtree`]: `Crate::named("widgets").at(&[…::SUBTREE])`.
/// * a **face** already has an identity — the same `NODE_ID` its own file writes in `parent:` — so it
///   is named with that: `Crate::named("slider").faces(&[…::NODE_ID])`.
///
/// A **file leaf that is not a face** (a registry-rule file, for instance) has neither, so it cannot
/// be named at all yet; the build refuses it by name rather than guessing.
/// 点名有**两种**方式，而差别不是口味问题——它是生成树里每个节点**实际拥有**的东西：
///
/// * **目录节点**（构建拼出来的子树，如 `control::object`）没有自己的身份，因此生成树用 [`Subtree`] 标记它：
///   `Crate::named("widgets").at(&[…::SUBTREE])`。
/// * **面**本来就有身份——就是它自己那份文件在 `parent:` 里写的同一个 `NODE_ID`——所以用它点名：
///   `Crate::named("slider").faces(&[…::NODE_ID])`。
///
/// **不是面的文件叶子**（例如规则文件）两样都没有，因此今天还不能被点名；构建会点名拒绝它，而不是猜。
#[derive(Clone, Copy, Debug)]
pub struct Crate {
    name: &'static str,
    at: &'static [Subtree],
}

impl Crate {
    /// A crate with this name and no subtrees yet.
    /// 一个以此命名的 crate，还没有子树。
    pub const fn named(name: &'static str) -> Self {
        Self { name, at: &[] }
    }

    /// The same crate, claiming these subtrees.
    /// 同一个 crate，认领这些子树。
    pub const fn at(self, at: &'static [Subtree]) -> Self {
        Self {
            name: self.name,
            at,
        }
    }

    /// This crate's name.
    /// 这个 crate 的名字。
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// The subtrees this crate claims.
    /// 这个 crate 认领的子树。
    pub const fn subtrees(self) -> &'static [Subtree] {
        self.at
    }
}

/// A package's whole crate shape.
/// 一个包的整份 crate 形状。
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    /// The prefix every published crate name is built from.
    /// 每个发布包名所依据的前缀。
    pub package_prefix: &'static str,
    /// The crates this package asks for, in declaration order.
    /// 这个包要求的 crate，按声明顺序。
    pub crates: &'static [Crate],
}

impl Shape {
    /// The shape a package declares: a prefix and the crates built from it.
    /// 包声明的形状：一个前缀，以及由它构建的那些 crate。
    ///
    /// A constructor rather than a struct literal: a declaration then writes two **method** names an
    /// editor completes (`Shape::of(…)` and `Crate::named(…)`) instead of remembering the field names
    /// `package_prefix:` and `crates:`.
    /// 用构造器而不是结构体字面量：声明里因此只写两个**方法**名（编辑器会补全）——`Shape::of(…)` 与
    /// `Crate::named(…)`——而不必记住 `package_prefix:` 与 `crates:` 这两个字段名。
    pub const fn of(package_prefix: &'static str, crates: &'static [Crate]) -> Self {
        Self {
            package_prefix,
            crates,
        }
    }
}

/// Check a declared shape, and hand it back so a caller can name it once.
/// 校验一份声明好的形状，并把它交回来，好让调用方只点名一次。
///
/// The generated tree calls this in a function that always compiles, which is what turns "the
/// declaration overlaps itself" from a thing nobody notices into a build failure that names the two
/// crates. The refusal text comes from the kernel's one rule.
/// 生成树在一个永远会被编译的函数里调用它，正是这一点把"声明自己跟自己重叠"从没人注意到的事变成一次点名了
/// 两个 crate 的构建失败。拒绝文案来自内核那唯一一条规则。
///
/// # Panics
/// Panics with the kernel's refusal when the shape does not hold together.
/// 形状不成立时，以内核的拒绝文案 panic。
pub fn add_crates(shape: &'static Shape) -> &'static Shape {
    let modules: Vec<Vec<&'static str>> = shape
        .crates
        .iter()
        .map(|krate| {
            krate
                .subtrees()
                .iter()
                .map(|subtree| subtree.module())
                .collect()
        })
        .collect();
    let declared: Vec<DeclaredCrate<'_>> = shape
        .crates
        .iter()
        .zip(&modules)
        .map(|(krate, subtrees)| DeclaredCrate {
            name: krate.name(),
            subtrees,
        })
        .collect();
    if let Err(refusal) = validate_shape(shape.package_prefix, &declared) {
        panic!("{refusal}");
    }
    shape
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod shape_tests;

/// One graft entry of a plan, written as ordinary Rust: `cut(<target>).graft(<replacement>)`, with an
/// optional `.to(<far>)` for a sibling range or `.full()` for a whole subtree.
/// 计划里的一条 graft 切口，写成普通 Rust：`cut(<目标>).graft(<替换>)`，可带 `.to(<远端>)` 表示兄弟区间，
/// 或 `.full()` 表示整棵子树。
///
/// `const fn` throughout, because a plan is a **`const` item**: the build reads the declaration's **text**,
/// and this type exists so that text is also ordinary Rust — the one spelling an editor can complete
/// without any LSP configuration.
/// 全程 `const fn`，因为计划是一个 **`const` 条目**：构建读的是声明的**文本**，而这个类型的存在是为了让那段
/// 文本同时是普通 Rust——也就是编辑器**不必任何 LSP 配置**就能补全的那种拼法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanEntry {
    /// The cut target, as the declaration spells it (a Rust path or a logical path).
    /// 切口目标，按声明所写（Rust 路径或逻辑路径）。
    pub cut: &'static str,
    /// The far endpoint of a sibling range, when `.to(…)` was used.
    /// 兄弟区间的远端端点，用了 `.to(…)` 时才有。
    pub end: Option<&'static str>,
    /// The replacement this cut grafts in.
    /// 这条切口嫁接进来的替换。
    pub graft: &'static str,
    /// Whether `.full()` asked for the whole subtree.
    /// 是否用 `.full()` 要了整棵子树。
    pub whole: bool,
}

impl PlanEntry {
    /// Name the replacement this cut grafts in.
    /// 点名这条切口嫁接进来的替换。
    pub const fn graft(self, replacement: &'static str) -> Self {
        Self {
            graft: replacement,
            ..self
        }
    }

    /// Extend the cut to a sibling range: everything from the target through `far`.
    /// 把切口扩成兄弟区间：从目标到 `far` 的全部内容。
    pub const fn to(self, far: &'static str) -> Self {
        Self {
            end: Some(far),
            ..self
        }
    }

    /// Replace the whole subtree rather than a single face.
    /// 替换整棵子树，而不是单个面。
    pub const fn full(self) -> Self {
        Self {
            whole: true,
            ..self
        }
    }
}

/// Start a graft entry at `target`.
/// 在 `target` 处开始一条 graft 条目。
pub const fn cut(target: &'static str) -> PlanEntry {
    PlanEntry {
        cut: target,
        end: None,
        graft: "",
        whole: false,
    }
}

/// A graft plan: `const PLAN: Plan = Plan::of([cut(a).graft(b), …]);`
/// 一份 graft 计划：`const PLAN: Plan = Plan::of([cut(a).graft(b), …]);`
///
/// It carries nothing on purpose. The plan's **authority is its own text**, which the build reads; the
/// value exists so the declaration is ordinary Rust — completable, type-checked, and refused by the
/// compiler when `.to(…)` and `.full()` are stated together.
/// 它有意不携带任何东西。计划的**权威是它自己的文本**，由构建读取；这个值的存在是为了让声明是普通 Rust——
/// 可补全、过类型检查，并且在 `.to(…)` 与 `.full()` 同时写出时由**编译器**拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan;

impl Plan {
    /// Wrap a plan's entries. The array is read by the build from the source; nothing is stored here.
    /// 包住一份计划的条目。数组由构建从源码读取；这里什么都不存。
    pub const fn of<const N: usize>(_entries: [PlanEntry; N]) -> Self {
        Self
    }
}
