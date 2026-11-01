//! The lexical reference scan: which paths a file's executable expressions reach.
//! 词法引用扫描：一个文件的可执行表达式触达了哪些路径。
//!
//! Split out of `face.rs` to keep that module inside the repository's size
//! ratchet. The scan is a self-contained pass over one parsed file, and its only
//! caller is `face::source_references`.
//! 从 `face.rs` 拆出，使那个模块留在仓库的尺寸棘轮之内。本扫描是对一棵已解析文件的自足遍历，
//! 它唯一的调用方是 `face::source_references`。

use std::collections::BTreeSet;

use syn::visit::Visit;

use super::face::{SourceReferences, path_to_string};

/// The references one parsed file reaches, plus how much the walk could see.
/// 一个已解析文件触达的引用，以及这次遍历实际能看到多少。
pub(super) fn scan(file: &syn::File) -> SourceReferences {
    let mut visitor = ReferenceVisitor::default();
    visitor.visit_file(file);
    visitor.references
}

/// Whether a macro name belongs to this build's own wiring, whose contents are
/// read by their own scanners rather than by the lexical reference scan.
/// 该宏名是否属于本构建自己的接线——其内容由各自的扫描器读取，而不是由词法引用扫描读取。
///
/// The distinction is what keeps the scope narrow: a host entry is full of
/// `host!()` and a face is a declaration macro, so widening on every invocation
/// would switch the automatic scope off for every real project — while *not*
/// widening on the others let the build prune a face that only a `vec![…]` named.
/// 这个区分正是收窄得以保留的原因：宿主入口满是 `host!()`，注册面本身就是一个声明宏，因此
/// "任何宏都放宽"会让每个真实工程都失去自动作用域——而对另外那些不放宽，又会让构建剪掉一个
/// 只被 `vec![…]` 命名的面。
fn is_build_macro(name: &str) -> bool {
    // Entry wiring: the generated plan, the entry hint, and the graft plans are
    // read by the entry resolver and `graft_entries`.
    // 入口接线：生成的计划、入口提示与 graft 计划分别由入口解析器与 `graft_entries` 读取。
    matches!(
        name,
        "host"
            | "application"
            | "static_graft_plan"
            | "runtime_graft_plan"
            | "graft_plan"
            | "__graft_plan_cuts"
    ) ||
    // Registration declarations: their field values are registration metadata,
    // which is exactly what the reference scan is documented to ignore
    // (`source_references_ignore_imports_strings_comments_and_registration_data`).
    // 注册声明：它们的字段取值是注册元数据，而引用扫描的契约正是忽略它
    // （见 `source_references_ignore_imports_strings_comments_and_registration_data`）。
    matches!(
        name,
        "control_object"
            | "external_object"
            | "__control_object"
            | "__external_object"
            | "__xirang_object"
            | "__registration_face"
            | "assert_contracts"
            // The hand-written face (H/A2e) writes its own contract check beside itself, and the trait
            // list it names is registration metadata — exactly what this scan ignores. Without this the
            // same face would widen the scope after migration that stayed narrow before it.
            // 手写的面（H/A2e）把自己那条合同检查写在旁边，而它点名的 trait 列表是注册元数据——正是本扫描
            // 忽略的东西。少了这一条，同一个面在迁移后会放宽作用域，而迁移前它是收窄的。
    ) || name.ends_with("_object")
}

/// Every `::`-joined path spelled inside a macro invocation's tokens.
/// 一次宏调用的 token 里写出的每个 `::` 连接路径。
///
/// A macro body is not Rust this parser owns, but a node identity is still
/// spelled out in it — `vec![crate::control::object::dial::NODE_ID]` — and a
/// `::` path is the only shape the scope matcher can use. Collecting those keeps
/// the named face *and* keeps the scope narrow, which is what the audit found
/// missing: the build pruned a face that a macro named, `check` reported ok, and
/// the host's `cargo check` failed with `E0433`.
///
/// Residual, stated rather than assumed: a macro that *builds* a path from a bare
/// identifier at expansion time (`paste!`-style) spells no path in its tokens, so
/// this scan cannot see it. Widening on every identifier instead would switch the
/// automatic scope off for any host that writes `format!` or `println!` in a face,
/// which is the feature's whole point; the honest reading is that the scan follows
/// path-shaped references, and a bare identifier is not one — inside a macro or
/// outside it, the matcher cannot use it either.
/// 宏的内容不是本解析器拥有的 Rust，但节点身份仍然写在里面——
/// `vec![crate::control::object::dial::NODE_ID]`——而 `::` 路径是作用域匹配器唯一能用的形状。
/// 收集它们既保住被命名的注册面，又保住作用域的收窄；这正是审计发现的缺口：构建剪掉了一个
/// 宏命名的面，`check` 报 ok，而宿主的 `cargo check` 以 `E0433` 失败。
///
/// 残留（明说而不是默认）：在展开期**从裸标识符构造**路径的宏（`paste!` 那一类）在它的 token
/// 里没有写出路径，本扫描看不到它。反过来"凡有标识符就放宽"会让任何在注册面里写 `format!`
/// 或 `println!` 的宿主失去自动作用域，而那正是这个功能的意义；诚实的读法是：本扫描跟随**路径
/// 形状**的引用，而裸标识符不是——无论宏内宏外，匹配器都用不上它。
fn visit_macro_paths(tokens: &proc_macro2::TokenStream, paths: &mut BTreeSet<String>) {
    let mut segment = String::new();
    let mut colons = 0usize;
    fn flush(segment: &mut String, paths: &mut BTreeSet<String>) {
        if segment.contains("::") {
            paths.insert(std::mem::take(segment));
        } else {
            segment.clear();
        }
    }
    for tree in tokens.clone() {
        match tree {
            proc_macro2::TokenTree::Ident(ident) => {
                if colons >= 2 && !segment.is_empty() {
                    segment.push_str("::");
                    segment.push_str(&ident.to_string());
                } else {
                    flush(&mut segment, paths);
                    segment.push_str(&ident.to_string());
                }
                colons = 0;
            }
            proc_macro2::TokenTree::Punct(punct) if punct.as_char() == ':' => colons += 1,
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {
                flush(&mut segment, paths);
                colons = 0;
            }
            proc_macro2::TokenTree::Group(group) => {
                flush(&mut segment, paths);
                colons = 0;
                visit_macro_paths(&group.stream(), paths);
            }
        }
    }
    flush(&mut segment, paths);
}

#[derive(Default)]
struct ReferenceVisitor {
    references: SourceReferences,
}

impl<'ast> Visit<'ast> for ReferenceVisitor {
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if !super::has_face_attribute(&item.attrs) {
            syn::visit::visit_item_const(self, item);
        }
    }

    /// The struct-literal form (H/A2e) keeps its declaration in an `impl`, so the "registration metadata
    /// is not a reference" rule has to reach it there: a face's own `parent:`, `flow:` and labels are the
    /// same metadata the macro block carried, and reading them as references would widen the build's
    /// source scope the moment a face is migrated — while `graft_plan.tsv` stayed byte-identical
    /// (measured: `source_scope.tsv` grew a row for the parent face).
    /// 字面量形（H/A2e）把声明放在 `impl` 里，因此"注册元数据不算引用"这条规则必须够到那里：面自己的
    /// `parent:`、`flow:` 与标签就是宏块携带的同一份元数据，把它们读成引用会让构建的源码作用域在面迁移的那一刻
    /// 放宽——而 `graft_plan.tsv` 是逐字不变的（实测：`source_scope.tsv` 多出父面那一行）。
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let is_face_declaration = item.items.iter().any(|entry| {
            matches!(entry, syn::ImplItem::Const(constant) if constant.ident == "REGISTRATION")
        }) || item.trait_.as_ref().is_some_and(|(_, path, _)| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == "FaceContract")
        });
        if !is_face_declaration {
            syn::visit::visit_item_impl(self, item);
        }
    }

    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        self.references
            .paths
            .insert(path_to_string(&expression.path));
        syn::visit::visit_expr_path(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'ast syn::ExprCall) {
        if !matches!(expression.func.as_ref(), syn::Expr::Path(_)) {
            self.references.conservative = true;
        }
        syn::visit::visit_expr_call(self, expression);
    }

    fn visit_type_trait_object(&mut self, object: &'ast syn::TypeTraitObject) {
        self.references.conservative = true;
        syn::visit::visit_type_trait_object(self, object);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.references.conservative = true;
        syn::visit::visit_item_extern_crate(self, item);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        let name = item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        // Anything this build does not read itself is an expansion the lexical
        // walk cannot enter, so the paths it spells are collected from the tokens
        // instead of being lost. Leaving them uncollected let the build prune a
        // face a macro still referenced, and the damage appeared only in the
        // host's `cargo check` (`error[E0433]: cannot find \`dial\` in \`object\``)
        // while `xirang check` reported ok.
        // 凡本构建自己读不了的宏，都是词法遍历进不去的展开，因此它写出的路径要从 token 里收集，
        // 而不是丢掉。过去不收曾让构建剪掉一个宏仍引用着的面，而损害只在宿主的 `cargo check` 里
        // 以 `error[E0433]: cannot find \`dial\` in \`object\`` 出现，`xirang check` 却报 ok。
        if !name.as_deref().is_some_and(is_build_macro) {
            visit_macro_paths(&item.tokens, &mut self.references.paths);
        }
    }
}
