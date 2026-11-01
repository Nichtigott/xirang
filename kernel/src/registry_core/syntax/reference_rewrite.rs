//! Pure planners for the explicit Rust references a module move relocates.
//! 模块搬动所迁移的显式 Rust 引用的纯规划器。
//!
//! A move changes a module's crate-relative path, so every hand-written reference
//! that spells the old path — a `use`/`pub use` tree, a typed or expression path,
//! or a file-relative `include_str!` — stops compiling unless it is rewritten. This
//! module plans those rewrites from the parsed syntax alone: it never reads a file,
//! never guesses at a macro expansion, and never touches a comment or an ordinary
//! string. What it cannot decide becomes an [`ReferencePlan::unresolved`] entry
//! instead of a silent success.
//! 一次搬动会改变模块的 crate 相对路径，因此每一处写出旧路径的手写引用——`use`/`pub use`
//! 树、类型或表达式路径，或相对文件的 `include_str!`——不改写就编译不过。本模块只从已解析
//! 的语法规划这些改写：不读文件、不猜宏展开、不动注释或普通字符串。它判不了的东西进入
//! [`ReferencePlan::unresolved`]，而不是静默成功。
//!
//! Scoping is part of "parsed syntax", not an extra: a bare `control::button` is only
//! the crate's own module when nothing nearer declares `control`. An inline
//! `mod local { mod control { … } }` and a function-local
//! `use crate::other as control;` both rebind that head, and a planner that ignored
//! them would rewrite somebody else's reference.
//! 作用域属于"已解析的语法"，不是额外要求：只有当更近的作用域没有声明 `control` 时，裸
//! `control::button` 才是 crate 自己的模块。内联 `mod local { mod control { … } }` 与函数内的
//! `use crate::other as control;` 都会重新绑定那个首段，忽略它们的规划器会改写别人的引用。
//!
//! The byte ranges are planned against the exact source text that was parsed, so a
//! caller must reject the plan when the file changed in between (see the execution
//! surface's bind check). The plan noun and the checked applier live in
//! [`super::text_edit`], which both this planner and the include planner hand their
//! ranges to.
//! 字节区间按被解析的那份源码原文规划，因此文件在此期间变化时调用方必须拒绝该计划（见执行面的绑定
//! 检查）。计划名词与带校验的应用函数住在 [`super::text_edit`]，本规划器与 include 规划器都把自己的
//! 区间交给它。

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use syn::visit::Visit;

use super::FaceSyntaxError;
use super::text_edit::{ReferencePlan, TextEdit};

/// One module path that moved, spelled without a leading `crate::`.
/// 一条被搬动的模块路径，不带前导 `crate::`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleMove {
    /// The module path before the move, e.g. `control::object::button`.
    /// 搬动之前的模块路径，例如 `control::object::button`。
    pub from: String,
    /// The module path after the move, e.g. `control::object::panel::button`.
    /// 搬动之后的模块路径，例如 `control::object::panel::button`。
    pub to: String,
}

/// One path segment as written, with the span that locates it in the source.
/// 一个按原文写出的路径段，以及它在源码中的定位 span。
#[derive(Clone)]
struct Segment {
    name: String,
    span: Span,
}

/// How a file can spell the crate the moved module lives in.
/// 一个文件可以用哪些拼法写出被搬模块所在的 crate。
///
/// The three spellings are not interchangeable and which of them is legal is a
/// property of the **file**, not of the move: inside the crate's own root `crate::…`
/// and a bare `control::…` both reach the module, a module file has only `crate::…`
/// (plus the 2018 self-name), and an integration test or an example has neither —
/// `crate::` there is the *test* crate, so rewriting it would edit an unrelated tree.
/// 三种拼法不能互换，而哪一种合法是**文件**的属性而不是搬动的属性：在 crate 自己的根上，
/// `crate::…` 与裸 `control::…` 都能到达那个模块；模块文件只有 `crate::…`（外加 2018 的自名）；
/// 集成测试或示例两者都没有——那里的 `crate::` 是**测试** crate，改写它等于改一棵无关的树。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathContext {
    /// Whether `crate::…` in this file names the tree the move happens in.
    /// 本文件里的 `crate::…` 是否命名搬动所在的那棵树。
    pub crate_root: bool,
    /// Whether a bare `control::…` in this file resolves to a crate item.
    /// 本文件里的裸 `control::…` 是否解析到 crate 条目。
    pub bare_root: bool,
    /// The crate's own name, when this file may write it (a consumer always may).
    /// crate 自己的名字，本文件可以写出它时给出（消费者文件总是可以）。
    pub crate_name: Option<String>,
}

impl PathContext {
    /// The context the boolean entry point builds: a crate root, nothing named.
    /// 布尔入口构造的上下文：一个 crate 根，没有可写出的 crate 名。
    pub fn at_crate_root(bare_root: bool) -> Self {
        Self {
            crate_root: true,
            bare_root,
            crate_name: None,
        }
    }
}

/// Which spelling a variant matches.
/// 一种变体匹配的是哪一种拼法。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spelling {
    /// `crate::control::button`.
    /// `crate::control::button`。
    Absolute,
    /// `control::button`, only where a bare path resolves inside the crate.
    /// `control::button`，只在裸路径解析到 crate 内部的地方。
    Bare,
    /// `my_crate::control::button`, the spelling a consumer and a 2018 self-reference use.
    /// `my_crate::control::button`，消费者与 2018 自名所用的拼法。
    Named,
}

/// One accepted `(from, to)` spelling.
/// 一种被接受的 `(from, to)` 拼法。
struct Variant {
    from: Vec<String>,
    to: Vec<String>,
    spelling: String,
    kind: Spelling,
}

/// Plan the explicit Rust re-export, import and path rewrites in one file.
/// 规划一个文件里的显式 Rust 重导出、导入与路径改写。
///
/// `bare_root` says whether this file is a crate root, where a bare `control::…`
/// path resolves inside the crate; anywhere else a bare first segment resolves to
/// an external crate, so it is deliberately left alone. `crate::…` paths are always
/// candidates. Even at a crate root the planner honours the scopes it can see: a
/// nearer `mod control` or `use … as control` makes the bare spelling somebody
/// else's, and a leading `::` makes it ambiguous between an external crate and a
/// 2015-style crate-root path.
/// `bare_root` 表示该文件是否为 crate 根——在根上裸 `control::…` 解析到 crate 内部；其它位置的裸首段
/// 解析到外部 crate，因此有意不改。`crate::…` 路径始终是候选。即使在 crate 根，规划器也遵守它看得见的
/// 作用域：更近的 `mod control` 或 `use … as control` 让裸拼法属于别人，而前导 `::` 让它在外部 crate
/// 与 2015 风格的 crate 根路径之间两可。
pub fn plan_reference_rewrites(
    source: &str,
    moves: &[ModuleMove],
    bare_root: bool,
) -> Result<ReferencePlan, FaceSyntaxError> {
    plan_reference_rewrites_in(source, moves, &PathContext::at_crate_root(bare_root))
}

/// [`plan_reference_rewrites`] for a file whose relationship to the crate is known.
/// [`plan_reference_rewrites`] 的完整形式，供已知文件与 crate 关系的调用方使用。
///
/// A consumer file (an integration test, an example, a build script) reaches the
/// moved module only through the crate's name, so it must be described with
/// `crate_root: false` — `crate::control::button` written there names the *test*
/// crate, and rewriting it would edit a tree the move never touched.
/// 消费者文件（集成测试、示例、构建脚本）只能通过 crate 名到达被搬模块，因此必须以
/// `crate_root: false` 描述——那里写出的 `crate::control::button` 命名的是**测试** crate，改写它
/// 会去动搬动从未碰过的树。
pub fn plan_reference_rewrites_in(
    source: &str,
    moves: &[ModuleMove],
    context: &PathContext,
) -> Result<ReferencePlan, FaceSyntaxError> {
    let file = super::nesting::parse_file(source)?;
    let mut planner = ReferencePlanner {
        variants: move_variants(moves, context),
        moved_paths: moves
            .iter()
            .map(|module_move| split(&module_move.from))
            .collect(),
        context: context.clone(),
        scopes: vec![Scope::default()],
        edits: BTreeMap::new(),
        unresolved: BTreeSet::new(),
    };
    planner.seed_items(&file.items);
    planner.visit_file(&file);
    Ok(planner.finish())
}

/// Every spelling pair the planner accepts for this file.
/// 本文件所接受的每一种拼法对。
fn move_variants(moves: &[ModuleMove], context: &PathContext) -> Vec<Variant> {
    let mut variants = Vec::new();
    for module_move in moves {
        let from = split(&module_move.from);
        let to = split(&module_move.to);
        // A move spelled `crate::…` already carries its own root, so it is written as
        // it came; otherwise the absolute spelling is added when this file is inside
        // the crate, and the bare one only at a crate root.
        if from.first().map(String::as_str) == Some("crate") {
            variants.push(Variant {
                from,
                to,
                spelling: module_move.from.clone(),
                kind: Spelling::Absolute,
            });
            continue;
        }
        if context.crate_root {
            let mut absolute_from = vec!["crate".to_owned()];
            absolute_from.extend(from.clone());
            let mut absolute_to = vec!["crate".to_owned()];
            absolute_to.extend(to.clone());
            variants.push(Variant {
                from: absolute_from,
                to: absolute_to,
                spelling: format!("crate::{}", module_move.from),
                kind: Spelling::Absolute,
            });
        }
        if context.bare_root {
            variants.push(Variant {
                from: from.clone(),
                to: to.clone(),
                spelling: module_move.from.clone(),
                kind: Spelling::Bare,
            });
        }
        if let Some(name) = &context.crate_name {
            let mut named_from = vec![name.clone()];
            named_from.extend(from.clone());
            let mut named_to = vec![name.clone()];
            named_to.extend(to.clone());
            variants.push(Variant {
                from: named_from,
                to: named_to,
                spelling: format!("{name}::{}", module_move.from),
                kind: Spelling::Named,
            });
        }
    }
    variants
}

/// Split a `::`-joined module path.
/// 切分一条 `::` 连接的模块路径。
fn split(path: &str) -> Vec<String> {
    path.split("::").map(str::to_owned).collect()
}

/// What a name in a lexical scope resolves to, as far as this file can see.
/// 就本文件所能看见的而言，某个词法作用域里的一个名字解析到什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Binding {
    /// The moved module's own head (`mod control` at the crate root).
    /// 被搬模块自己的首段（crate 根上的 `mod control`）。
    Ancestor,
    /// Something else with the same name, so the bare spelling is not ours.
    /// 同名的别的东西，因此裸拼法不是我们的。
    Other,
}

/// One lexical scope's declared path heads.
/// 一个词法作用域里被声明的路径首段。
#[derive(Default)]
struct Scope {
    names: BTreeMap<String, Binding>,
}

struct ReferencePlanner {
    variants: Vec<Variant>,
    /// Every moved module path, spelled as it came, for the unknown-head report.
    /// 每一条被搬模块路径，按传入的拼法，供"未知首段"报告使用。
    moved_paths: Vec<Vec<String>>,
    context: PathContext,
    scopes: Vec<Scope>,
    edits: BTreeMap<(usize, usize), String>,
    unresolved: BTreeSet<String>,
}

impl ReferencePlanner {
    fn finish(mut self) -> ReferencePlan {
        let mut edits: Vec<TextEdit> = Vec::new();
        for ((start, end), replacement) in std::mem::take(&mut self.edits) {
            // Two edits can only be adjacent or disjoint by construction; a genuine
            // overlap means two plan entries claim one range, which is a refusal.
            if let Some(previous) = edits.last()
                && previous.end > start
            {
                self.unresolved.insert(format!(
                    "two rewrites claim overlapping bytes {}..{} and {}..{}",
                    previous.start, previous.end, start, end
                ));
                continue;
            }
            edits.push(TextEdit {
                start,
                end,
                replacement,
            });
        }
        edits.sort_by_key(|edit| (edit.start, edit.end));
        ReferencePlan {
            edits,
            unresolved: self.unresolved.into_iter().collect(),
        }
    }

    /// Record one replacement, refusing to merge two different texts on one range.
    /// 记录一处替换；同一区间上两种不同文本拒绝合并。
    fn record(&mut self, start: usize, end: usize, replacement: String, spelling: &str) {
        if let Some(existing) = self.edits.get(&(start, end)) {
            if *existing != replacement {
                self.unresolved.insert(format!(
                    "overlapping rewrites for `{spelling}` disagree at bytes {start}..{end}"
                ));
            }
            return;
        }
        self.edits.insert((start, end), replacement);
    }

    /// Declare the path heads one item list introduces in the current scope.
    /// 声明一个条目清单在当前作用域引入的路径首段。
    ///
    /// Module declarations are read first so the crate root's own `mod control` wins
    /// over a same-named item in another namespace; everything else only fills names
    /// nothing has claimed. A file that declares one name twice is not valid Rust, so
    /// there is no third case to decide.
    /// 先读模块声明，使 crate 根自己的 `mod control` 胜过另一个命名空间里的同名条目；其余只填补还没人
    /// 认领的名字。同一名字声明两次的文件本来就不是合法 Rust，因此没有第三种情形要裁定。
    fn seed_items(&mut self, items: &[syn::Item]) {
        for item in items {
            if let syn::Item::Mod(module) = item {
                let binding = self.module_binding(&module.ident.to_string());
                self.declare(module.ident.to_string(), binding);
            }
        }
        for item in items {
            match item {
                syn::Item::Use(item_use) => self.seed_use(&item_use.tree),
                syn::Item::ExternCrate(extern_crate) => {
                    let name = extern_crate.rename.as_ref().map_or_else(
                        || extern_crate.ident.to_string(),
                        |(_, alias)| alias.to_string(),
                    );
                    self.declare(name, Binding::Other);
                }
                syn::Item::Struct(item) => self.declare(item.ident.to_string(), Binding::Other),
                syn::Item::Enum(item) => self.declare(item.ident.to_string(), Binding::Other),
                syn::Item::Union(item) => self.declare(item.ident.to_string(), Binding::Other),
                syn::Item::Trait(item) => self.declare(item.ident.to_string(), Binding::Other),
                syn::Item::Type(item) => self.declare(item.ident.to_string(), Binding::Other),
                _ => {}
            }
        }
    }

    /// Record one name in the innermost scope unless something already claimed it.
    /// 在最近的作用域里记录一个名字，除非已经有东西认领了它。
    fn declare(&mut self, name: String, binding: Binding) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.names.entry(name).or_insert(binding);
        }
    }

    /// What a `mod <name>` declaration means for the bare spelling of `<name>`.
    /// 对 `<name>` 的裸拼法而言，一条 `mod <name>` 声明意味着什么。
    ///
    /// At the crate root it is the moved module's own ancestor chain, so a bare
    /// `control::…` there is ours. Anywhere else (`mod local { mod control { … } }`)
    /// the bare spelling resolves inside that module and is not.
    /// 在 crate 根它就是被搬模块自己的祖先链，因此那里的裸 `control::…` 是我们的。其它位置
    /// （`mod local { mod control { … } }`）裸拼法解析到那个模块内部，因而不是。
    fn module_binding(&self, name: &str) -> Binding {
        let at_root = self.scopes.len() == 1;
        if at_root && self.context.bare_root && self.bare_heads().contains(name) {
            Binding::Ancestor
        } else {
            Binding::Other
        }
    }

    /// Every head the bare variants can match.
    /// 裸拼法能匹配到的全部首段。
    fn bare_heads(&self) -> BTreeSet<String> {
        self.variants
            .iter()
            .filter(|variant| variant.kind == Spelling::Bare)
            .filter_map(|variant| variant.from.first().cloned())
            .collect()
    }

    /// Declare every name one `use` tree binds in the current scope.
    /// 声明一棵 `use` 树在当前作用域绑定的每一个名字。
    fn seed_use(&mut self, tree: &syn::UseTree) {
        let mut chain = Vec::new();
        self.declare_use_bindings(tree, &mut chain);
    }

    /// Walk one `use` tree, declaring each bound name with what it resolves to.
    /// 遍历一棵 `use` 树，按它解析到的东西声明每个被绑定的名字。
    fn declare_use_bindings(&mut self, tree: &syn::UseTree, chain: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                chain.push(path.ident.to_string());
                self.declare_use_bindings(&path.tree, chain);
                chain.pop();
            }
            syn::UseTree::Name(name) => {
                chain.push(name.ident.to_string());
                let binding = self.use_binding(chain);
                self.declare(chain.last().cloned().unwrap_or_default(), binding);
                chain.pop();
            }
            syn::UseTree::Rename(rename) => {
                chain.push(rename.ident.to_string());
                let binding = self.use_binding(chain);
                self.declare(rename.rename.to_string(), binding);
                chain.pop();
            }
            syn::UseTree::Glob(_) => {}
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.declare_use_bindings(item, chain);
                }
            }
        }
    }

    /// What one fully spelled `use` chain binds, in terms of this planner's question.
    /// 一条完整拼出的 `use` 链绑定了什么，就本规划器所问的问题而言。
    fn use_binding(&self, chain: &[String]) -> Binding {
        let Some((leaf, prefix)) = chain.split_last() else {
            return Binding::Other;
        };
        if !self.bare_heads().contains(leaf) {
            return Binding::Other;
        }
        match prefix {
            // `use crate::control;` / `use self::control;` name the ancestor itself.
            [root] if root == "crate" || root == "self" => Binding::Ancestor,
            // `use control;` is the root's own module in the pre-2018 spelling; away
            // from a crate root the same text is an external crate.
            [] => {
                if self.context.bare_root {
                    Binding::Ancestor
                } else {
                    Binding::Other
                }
            }
            // `use crate::other as control;`, `use crate::other::control;`,
            // `use super::control;` and every deeper chain name something else.
            _ => Binding::Other,
        }
    }

    /// Whether the bare spelling of `head` can be the crate's own module here.
    /// 在此处，`head` 的裸拼法能否是 crate 自己的模块。
    ///
    /// The nearest scope that declares the name decides; nothing in this file
    /// declaring it leaves the caller's `bare_root` contract in force. A name the
    /// crate writes on itself (`my_crate::…`) has no such contract, so any
    /// declaration of that head hides it.
    /// 由最近声明该名字的作用域裁定；本文件没有任何地方声明它时，调用方的 `bare_root` 约定继续有效。
    /// crate 写给自己的名字（`my_crate::…`）没有这条约定，因此只要有任何地方声明该首段就把它遮住。
    fn head_visible(&self, head: &str, kind: Spelling) -> bool {
        match kind {
            Spelling::Bare if !self.context.bare_root => false,
            Spelling::Absolute => true,
            _ => {
                for scope in self.scopes.iter().rev() {
                    if let Some(binding) = scope.names.get(head) {
                        return *binding == Binding::Ancestor;
                    }
                }
                true
            }
        }
    }

    /// Report a path whose tail spells a moved module behind a head we cannot name.
    /// 报告一条尾巴拼出了被搬模块、但首段我们叫不出名字的路径。
    ///
    /// This is what keeps a consumer file from being a silent gap: an integration test
    /// reaches the module as `my_crate::control::button`, and a planner that only knows
    /// `crate::…` would leave it stale without saying so. The tail is what makes it a
    /// candidate; an unrelated `std::…` path never matches one.
    /// 正是它让消费者文件不成为静默的缺口：集成测试经 `my_crate::control::button` 到达那个模块，而只认
    /// `crate::…` 的规划器会把它留在原处且一声不吭。让它成为候选的是那条尾巴；无关的 `std::…` 路径永远
    /// 匹配不上。
    fn report_unknown_head(&mut self, segments: &[Segment], group_base: usize) {
        if group_base != 0 || segments.len() < 2 {
            return;
        }
        // A head this file declares is not unknown: the reference resolves to that
        // declaration, which is simply not the moved module. Only a head nobody here
        // binds leaves a real question.
        // 本文件声明过的首段不是未知的：那条引用解析到那个声明，只是它不是被搬模块。只有这里没有任何地方
        // 绑定的首段才留下真正的问题。
        if self
            .scopes
            .iter()
            .any(|scope| scope.names.contains_key(&segments[0].name))
        {
            return;
        }
        let tail = &segments[1..];
        let matched = self.moved_paths.iter().any(|from| {
            !from.is_empty()
                && tail.len() >= from.len()
                && from
                    .iter()
                    .zip(tail.iter())
                    .all(|(expected, segment)| *expected == segment.name)
        });
        if !matched {
            return;
        }
        let head = &segments[0].name;
        let line = segments[0].span.start().line;
        self.unresolved.insert(format!(
            "`{head}::…` at line {line} has a moved module as its tail, but `{head}` is not a name \
             this planner can resolve here; relocate it explicitly"
        ));
    }

    /// Match one leaf against every move spelling and record the longest candidate.
    /// 把一个叶子与每种搬动拼法匹配，记录最长候选。
    ///
    /// `absolute_leading` is a leading `::`, which in 2018 names an external crate and
    /// in 2015 named the crate root: the planner reports the ambiguity instead of
    /// picking one reading.
    /// `absolute_leading` 是前导 `::`：2018 里它命名外部 crate，2015 里它命名 crate 根；规划器报出
    /// 这个两可，而不是挑一种读法。
    fn consider(&mut self, segments: &[Segment], group_base: usize, absolute_leading: bool) {
        let mut best: Option<(usize, Vec<String>, String)> = None;
        for variant in &self.variants {
            if variant.from.len() > segments.len() {
                continue;
            }
            if !variant
                .from
                .iter()
                .zip(segments.iter())
                .all(|(expected, segment)| *expected == segment.name)
            {
                continue;
            }
            // A leading `::` binds the first segment to an external crate (2018) or to
            // the crate root (2015). Only the second reading is ours, so the reference
            // is reported, never silently rewritten; `crate::…` is never written that
            // way, so the absolute variant is unaffected.
            if absolute_leading && variant.kind == Spelling::Bare {
                if let Some(head) = variant.from.first() {
                    self.unresolved.insert(format!(
                        "`::{head}::…` at byte {} could be an external crate or a 2015 crate-root \
                         path; this planner will not guess which",
                        segments[0].span.byte_range().start
                    ));
                }
                return;
            }
            if let Some(head) = variant.from.first()
                && !self.head_visible(head, variant.kind)
            {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|(length, _, _)| variant.from.len() > *length)
            {
                best = Some((
                    variant.from.len(),
                    variant.to.clone(),
                    variant.spelling.clone(),
                ));
            }
        }
        let Some((matched, to, spelling)) = best else {
            self.report_unknown_head(segments, group_base);
            return;
        };
        // A match entirely inside the chain may be replaced as one contiguous
        // range. A match that starts inside the chain and reaches into a group
        // item replaces only the item's own segments, with the `to` suffix that
        // the group base does not already spell.
        let (start_index, replacement) = if matched <= group_base {
            (0usize, to.join("::"))
        } else {
            if to.len() < group_base
                || !segments[..group_base]
                    .iter()
                    .zip(to.iter())
                    .all(|(segment, expected)| segment.name == *expected)
            {
                self.unresolved.insert(format!(
                    "grouped import of `{spelling}` cannot be rewritten without its group base"
                ));
                return;
            }
            let suffix = to[group_base..].join("::");
            (group_base, suffix)
        };
        let first = &segments[start_index];
        let last = &segments[matched - 1];
        let start = first.span.byte_range().start;
        let end = last.span.byte_range().end;
        self.record(start, end, replacement, &spelling);
    }

    /// The parsed segments of one `syn::Path`, with their spans.
    /// 一条 `syn::Path` 解析出的段，带各自的 span。
    fn path_segments(path: &syn::Path) -> Vec<Segment> {
        path.segments
            .iter()
            .map(|segment| Segment {
                name: segment.ident.to_string(),
                span: segment.ident.span(),
            })
            .collect()
    }

    /// Report a `self::`/`super::` reference whose tail could name a moved module.
    /// 报告尾部可能命名了被搬模块的 `self::`/`super::` 引用。
    fn relative_tail_hint(&mut self, tree: &syn::UseTree, prefix: &[Segment]) {
        let Some(first) = prefix.first().map(|segment| segment.name.as_str()) else {
            return;
        };
        if first != "self" && first != "super" {
            return;
        }
        let tail: Vec<&str> = prefix.iter().map(|segment| segment.name.as_str()).collect();
        let Some(leaf) = self
            .variants
            .first()
            .and_then(|variant| variant.from.last())
        else {
            return;
        };
        if tail.contains(&leaf.as_str()) {
            self.unresolved.insert(format!(
                "a `{first}::…` import at {} may name moved module `{leaf}` and cannot be resolved \
                 lexically; relocate it explicitly",
                tree_span_line(tree)
            ));
        }
    }

    /// Walk one use tree, accumulating the contiguous segment chain.
    /// 遍历一棵 use 树，累积连续的段链。
    ///
    /// `group_base` is the length of the chain at the innermost enclosing brace;
    /// a match that reaches past it replaces only the item's own segments and gets
    /// the `to` suffix the brace does not already spell.
    /// `group_base` 是最内层花括号处的段链长度；越过它的匹配只替换条目自己的段，并取花括号尚未拼出的
    /// `to` 后缀。
    fn walk_use(&mut self, tree: &syn::UseTree, prefix: &mut Vec<Segment>, group_base: usize) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(Segment {
                    name: path.ident.to_string(),
                    span: path.ident.span(),
                });
                self.walk_use(&path.tree, prefix, group_base);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                prefix.push(Segment {
                    name: name.ident.to_string(),
                    span: name.ident.span(),
                });
                self.relative_tail_hint(tree, prefix);
                self.consider(prefix, group_base, false);
                prefix.pop();
            }
            syn::UseTree::Rename(rename) => {
                prefix.push(Segment {
                    name: rename.ident.to_string(),
                    span: rename.ident.span(),
                });
                self.relative_tail_hint(tree, prefix);
                self.consider(prefix, group_base, false);
                prefix.pop();
            }
            syn::UseTree::Glob(_) => {
                self.relative_tail_hint(tree, prefix);
                self.consider(prefix, group_base, false);
            }
            syn::UseTree::Group(group) => {
                // The group's base is the contiguous chain up to the brace; every
                // item is matched against the full path but only its own segments
                // (after the base) may be replaced as one range.
                let base = prefix.len();
                for item in &group.items {
                    self.walk_use(item, prefix, base);
                    prefix.truncate(base);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for ReferencePlanner {
    fn visit_file(&mut self, file: &'ast syn::File) {
        // The crate root is a scope, and the items that declare heads in it were
        // seeded before the walk; visiting must not declare them again.
        for item in &file.items {
            self.visit_item(item);
        }
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if let Some((_, items)) = &module.content {
            self.scopes.push(Scope::default());
            self.seed_items(items);
            for item in items {
                self.visit_item(item);
            }
            self.scopes.pop();
        }
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        // A block is a scope for the `use` items it carries, and an item in a block
        // is visible for the whole block, so the bindings are seeded before the
        // statements are visited — a reference written above its `use` still sees it.
        // 块是它携带的 `use` 条目所在的作用域，而块里的条目对整个块可见，因此绑定在遍历语句之前就播下
        // —— 写在 `use` 上面的引用也看得见它。
        self.scopes.push(Scope::default());
        for statement in &block.stmts {
            if let syn::Stmt::Item(item) = statement {
                match item {
                    syn::Item::Use(item_use) => self.seed_use(&item_use.tree),
                    syn::Item::Mod(module) => {
                        let binding = Binding::Other;
                        self.declare(module.ident.to_string(), binding);
                    }
                    _ => {}
                }
            }
        }
        syn::visit::visit_block(self, block);
        self.scopes.pop();
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if item.leading_colon.is_some() {
            let mut heads = Vec::new();
            visit_use_heads(&item.tree, &mut Vec::new(), &mut heads);
            if heads
                .iter()
                .any(|head| self.bare_heads().contains(head) && self.context.bare_root)
            {
                self.unresolved.insert(format!(
                    "an absolute `::` import at {} cannot be relocated without guessing its root",
                    span_line(item)
                ));
            }
            return;
        }
        let mut prefix = Vec::new();
        self.walk_use(&item.tree, &mut prefix, 0);
    }

    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        let segments = Self::path_segments(&expression.path);
        self.consider(&segments, 0, expression.path.leading_colon.is_some());
        // Nested generic arguments, `QSelf` types and method-call receivers are
        // paths too; the default walk reaches them, and doing it after the top-level
        // match is what keeps a name recorded once instead of not at all.
        // 嵌套泛型实参、`QSelf` 类型与方法调用接收者同样是路径；默认遍历能到达它们，而把它放在顶层匹配
        // 之后，正是"记一次"而不是"一次都不记"的原因。
        syn::visit::visit_expr_path(self, expression);
    }

    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        let segments = Self::path_segments(&ty.path);
        self.consider(&segments, 0, ty.path.leading_colon.is_some());
        syn::visit::visit_type_path(self, ty);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        // A macro body is not Rust this planner owns; a path built inside one is
        // exactly the case the caller has to read as an outstanding obligation. The
        // obligation is raised only when the tokens actually spell a moved module, so
        // an unrelated macro stays out of the list.
        // 宏体不是本规划器拥有的 Rust；其中拼出的路径正是调用方必须读作未完成义务的情形。只有当词法记号
        // 真的拼出了某个被搬模块时才登记该义务，因此无关的宏不会进那张清单。
        let named = item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if matches!(named.as_str(), "include" | "include_str" | "include_bytes")
            && syn::parse2::<syn::LitStr>(item.tokens.clone()).is_err()
        {
            self.unresolved.insert(format!(
                "`{named}!` at line {} builds its path at expansion time, so the reference cannot \
                 be checked",
                span_line(item)
            ));
            return;
        }
        let text = macro_path_text(&item.tokens);
        for variant in &self.variants {
            let spelling = variant.from.join("::");
            let visible = variant
                .from
                .first()
                .is_none_or(|head| self.head_visible(head, variant.kind));
            if visible && text.contains(&spelling) {
                self.unresolved.insert(format!(
                    "macro `{named}!` at line {} mentions `{spelling}`, whose expansion this \
                     planner cannot check; relocate it explicitly if the macro builds a path",
                    span_line(item)
                ));
                break;
            }
        }
    }
}

/// The path-like text of a macro body, with string literals blanked out.
/// 宏体中像路径的那部分文本，字符串字面量被抹掉。
///
/// A string literal is a value, not a path: `format!("control::button")` must not be
/// reported as a reference to the moved module, while `control::button!()` must be.
/// 字符串字面量是值而不是路径：`format!("control::button")` 不得被报成对搬动模块的引用，而
/// `control::button!()` 必须被报出来。
fn macro_path_text(tokens: &proc_macro2::TokenStream) -> String {
    let mut text = String::new();
    for tree in tokens.clone() {
        match tree {
            proc_macro2::TokenTree::Literal(_) => text.push(' '),
            other => text.push_str(&other.to_string()),
        }
    }
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Every bound name a `use` tree introduces, for the absolute-import check.
/// 一棵 `use` 树引入的全部绑定名，供绝对导入检查使用。
fn visit_use_heads(tree: &syn::UseTree, chain: &mut Vec<String>, heads: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            chain.push(path.ident.to_string());
            visit_use_heads(&path.tree, chain, heads);
            chain.pop();
        }
        syn::UseTree::Name(name) => {
            chain.push(name.ident.to_string());
            heads.extend(chain.iter().cloned());
            chain.pop();
        }
        syn::UseTree::Rename(rename) => {
            chain.push(rename.ident.to_string());
            heads.extend(chain.iter().cloned());
            chain.pop();
        }
        syn::UseTree::Glob(_) => heads.extend(chain.iter().cloned()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                visit_use_heads(item, chain, heads);
            }
        }
    }
}

/// The line number of an item's span, for a refusal a reader can act on.
/// 条目 span 的行号，供读者可据以行动的拒绝使用。
fn span_line(spanned: &impl syn::spanned::Spanned) -> usize {
    spanned.span().start().line
}

/// The line number of a use tree's span.
/// 一棵 use 树 span 的行号。
fn tree_span_line(tree: &syn::UseTree) -> usize {
    use syn::spanned::Spanned;
    tree.span().start().line
}

#[cfg(test)]
#[path = "reference_rewrite_tests.rs"]
mod reference_rewrite_tests;
