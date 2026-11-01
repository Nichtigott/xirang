//! Rust syntax/scoping facts for the shared semantic protocol.
//! 共享语义协议使用的 Rust 语法/作用域事实。
//!
//! This is deliberately narrower than type inference: it resolves **explicit** paths
//! (`Entry::new`), `Self::…`, and method calls whose receiver is `self` or a type path.
//! A method call on a value whose type is not written is returned as **unresolved**, not as a
//! guessed target. The caller of this module must print that distinction.
//! 这有意窄于类型推导：只解析**显式**路径（`Entry::new`）、`Self::…`，以及接收者是 `self` 或类型
//! 路径的方法调用。接收者类型未写出的方法调用返回**未解析**，而不是猜一个目标。调用方必须把这条区别印出来。

use std::collections::{BTreeMap, BTreeSet};

use syn::visit::{self, Visit};

use crate::semantics::SemanticFunction;

/// Parse one Rust file. `None` means the file did not parse; the caller must fall back to the
/// lexical index and label the result as lexical.
/// 解析一份 Rust 文件。`None` 表示文件没解析成功；调用方必须退回词法索引并把结果标成词法。
pub fn semantic_functions(source: &str) -> Option<Vec<SemanticFunction>> {
    let file = syn::parse_file(source).ok()?;
    let mut methods: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    method_index(&file.items, &mut methods);
    let mut collector = Collector {
        methods,
        ..Collector::default()
    };
    collector.items(&file.items, &mut Vec::new());
    let stream = serde_json::Value::Array(collector.nodes).to_string();
    Some(
        crate::languages::kit::ast::walk_nodes(stream.as_bytes(), &KINDS, "syn")
            .expect("the semantic stream this module builds is pre-order"),
    )
}

/// Which of this walker's node kinds mean what.
/// 本遍历器的哪些节点种类意味着什么。
const KINDS: crate::languages::kit::ast::KindMap = crate::languages::kit::ast::KindMap {
    function: &["Function"],
    class: &[],
    call: &["Call"],
};

#[derive(Default)]
struct Collector {
    nodes: Vec<serde_json::Value>,
    methods: BTreeMap<String, BTreeSet<String>>,
}

impl Collector {
    fn items(&mut self, items: &[syn::Item], module: &mut Vec<String>) {
        for item in items {
            match item {
                syn::Item::Fn(function) => {
                    self.function(&function.sig, &function.block, None);
                }
                syn::Item::Mod(module_item) => {
                    if let Some((_, items)) = &module_item.content {
                        module.push(module_item.ident.to_string());
                        self.items(items, module);
                        module.pop();
                    }
                }
                syn::Item::Impl(implementation) => {
                    let owner = type_name(&implementation.self_ty);
                    for item in &implementation.items {
                        if let syn::ImplItem::Fn(function) = item {
                            self.function(&function.sig, &function.block, owner.clone());
                        }
                    }
                }
                syn::Item::Trait(definition) => {
                    let owner = definition.ident.to_string();
                    for item in &definition.items {
                        if let syn::TraitItem::Fn(function) = item
                            && let Some(block) = &function.default
                        {
                            self.function(&function.sig, block, Some(owner.clone()));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn function(&mut self, signature: &syn::Signature, block: &syn::Block, owner: Option<String>) {
        let name = signature.ident.to_string();
        let mut calls = CallCollector {
            owner: owner.clone(),
            methods: &self.methods,
            locals: BTreeMap::new(),
            calls: Vec::new(),
            unresolved: Vec::new(),
        };
        for input in &signature.inputs {
            if let syn::FnArg::Typed(typed) = input {
                // A parameter's type is written in the signature, so a method call on it has a
                // known owner even when the receiver is spelled as a bare local name.
                // 参数类型写在签名里，因此对它的方法调用有已知所属者——即使接收者只写了一个本地名。
                bind_pattern(&typed.pat, type_name(&typed.ty), &mut calls.locals);
            }
        }
        calls.visit_block(block);
        let index = self.nodes.len();
        self.nodes.push(serde_json::json!({
            "kind": "Function",
            "name": name,
            "owner": owner,
            "line": signature.fn_token.span.start().line,
            "end_line": block.brace_token.span.close().start().line,
        }));
        for target in &calls.calls {
            self.nodes
                .push(serde_json::json!({"kind": "Call", "parent": index, "target": target}));
        }
        for reason in &calls.unresolved {
            self.nodes
                .push(serde_json::json!({"kind": "Call", "parent": index, "unresolved": reason}));
        }
    }
}

struct CallCollector<'a> {
    owner: Option<String>,
    methods: &'a BTreeMap<String, BTreeSet<String>>,
    /// Local bindings, and the type the source **wrote** for each of them.
    ///
    /// `None` is a binding whose type the syntax does not state (a closure, a call to something this
    /// file does not define, a value returned by a function). Keeping the name with no type is what
    /// preserves the function-pointer answer: a call through one is unknown, never a guessed target.
    /// 本地绑定，以及源码为每一个**写出**的类型。`None` 是语法没有陈述类型的绑定（闭包、对本文件没有
    /// 定义的东西的调用、函数返回值）。把名字与"没有类型"一起留着，正是保住函数指针答案的东西：经由它的
    /// 调用是未知，绝不是一个猜出来的目标。
    locals: BTreeMap<String, Option<String>>,
    calls: Vec<String>,
    unresolved: Vec<String>,
}

impl CallCollector<'_> {
    fn push(&mut self, target: String) {
        if !self.calls.contains(&target) {
            self.calls.push(target);
        }
    }

    fn unresolved(&mut self, text: String) {
        if !self.unresolved.contains(&text) {
            self.unresolved.push(text);
        }
    }
}

impl<'ast> Visit<'ast> for CallCollector<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref()
            && path.path.segments.len() == 1
            && let Some(segment) = path.path.segments.first()
            && self.locals.contains_key(&segment.ident.to_string())
        {
            self.unresolved(format!(
                "{} (local binding/function pointer; target is not written)",
                segment.ident
            ));
        } else if let syn::Expr::Path(path) = node.func.as_ref()
            && let Some(target) = call_path(path, self.owner.as_deref())
        {
            self.push(target);
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        let stated = match &node.pat {
            syn::Pat::Type(typed) => type_name(&typed.ty),
            _ => node
                .init
                .as_ref()
                .and_then(|init| initializer_type(&init.expr)),
        };
        bind_pattern(&node.pat, stated.clone(), &mut self.locals);
        if let Some(Some(owner)) = stated.map(Some) {
            // `let fast: Fast = …` and `let fast = Fast::new()` both state the type; `Pat::Type`
            // also binds through the inner pattern, which `bind_pattern` walks.
            // `let fast: Fast = …` 与 `let fast = Fast::new()` 都陈述了类型；`Pat::Type` 也会经内层
            // 模式绑定，`bind_pattern` 会走进去。
            let _ = owner;
        }
        visit::visit_local(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        // `syn` does not expand macros, and calls inside `assert_eq!`/`vec!` are not expressions in
        // its tree. The lexical scanner sees their tokens; these are marked as macro-derived and
        // do not claim an owner, so a same-named method inside a macro stays a candidate rather
        // than a definite target.
        // `syn` 不展开宏，`assert_eq!`/`vec!` 里的调用在它的树中不是表达式。词法扫描器能看见这些
        // token；它们被标成宏派生，不声称所属者，因此宏里的同名方法保持候选而不是确定目标。
        let text = node.tokens.to_string();
        for call in xirang_kernel::source::direct_calls(&text, "") {
            self.push(call);
        }
        visit::visit_macro(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method = node.method.to_string();
        match receiver_owner(&node.receiver, self.owner.as_deref(), &self.locals) {
            Some(owner) => {
                let target = format!("{owner}::{method}");
                if self
                    .methods
                    .get(&method)
                    .is_some_and(|owners| owners.contains(&owner))
                {
                    self.push(target);
                } else {
                    self.unresolved(format!(
                        "{target} (no method definition in this file; receiver type is written)"
                    ));
                }
            }
            None => {
                let candidates = self
                    .methods
                    .get(&method)
                    .map(|owners| owners.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                if candidates.is_empty() {
                    self.unresolved(format!("{method} (receiver type is not written)"));
                } else {
                    self.unresolved(format!(
                        "{method} (receiver type is not written; candidates: {})",
                        candidates
                            .iter()
                            .map(|owner| format!("{owner}::{method}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }
        visit::visit_expr_method_call(self, node);
    }
}

fn method_index(items: &[syn::Item], out: &mut BTreeMap<String, BTreeSet<String>>) {
    for item in items {
        match item {
            syn::Item::Impl(implementation) => {
                if let Some(owner) = type_name(&implementation.self_ty) {
                    for child in &implementation.items {
                        if let syn::ImplItem::Fn(function) = child {
                            out.entry(function.sig.ident.to_string())
                                .or_default()
                                .insert(owner.clone());
                        }
                    }
                }
            }
            syn::Item::Trait(definition) => {
                let owner = definition.ident.to_string();
                for child in &definition.items {
                    if let syn::TraitItem::Fn(function) = child
                        && function.default.is_some()
                    {
                        out.entry(function.sig.ident.to_string())
                            .or_default()
                            .insert(owner.clone());
                    }
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    method_index(items, out);
                }
            }
            _ => {}
        }
    }
}

/// Bind every name a pattern introduces, with the type the statement stated (if any).
/// 绑定一个模式引入的每个名字，并带上语句陈述的类型（如果有）。
fn bind_pattern(
    pattern: &syn::Pat,
    owner: Option<String>,
    out: &mut BTreeMap<String, Option<String>>,
) {
    match pattern {
        syn::Pat::Ident(ident) => {
            out.insert(ident.ident.to_string(), owner);
        }
        syn::Pat::Type(typed) => {
            let stated = type_name(&typed.ty).or(owner);
            bind_pattern(&typed.pat, stated, out);
        }
        syn::Pat::Reference(reference) => bind_pattern(&reference.pat, owner, out),
        syn::Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                bind_pattern(element, owner.clone(), out);
            }
        }
        syn::Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                bind_pattern(element, owner.clone(), out);
            }
        }
        syn::Pat::Struct(structure) => {
            for field in &structure.fields {
                bind_pattern(&field.pat, owner.clone(), out);
            }
        }
        syn::Pat::Slice(slice) => {
            for element in &slice.elems {
                bind_pattern(element, owner.clone(), out);
            }
        }
        syn::Pat::Or(or) => {
            for case in &or.cases {
                bind_pattern(case, owner.clone(), out);
            }
        }
        syn::Pat::Paren(paren) => bind_pattern(&paren.pat, owner, out),
        _ => {}
    }
}

/// The type an initializer expression states, when it states one by construction.
/// 初始化表达式通过构造陈述的类型（当它陈述了时）。
///
/// Only two shapes are read: `Type::new(…)` (and any `Type::assoc`) and a struct literal `Type { … }`.
/// Anything else — a call to a free function, a closure, a method chain — says nothing about the
/// type to a syntax reader, and guessing there is how a dynamic call gets attributed to one
/// implementation.
/// 只读两种形状：`Type::new(…)`（以及任意 `Type::assoc`）与结构体字面量 `Type { … }`。其余——对自由函数
/// 的调用、闭包、方法链——对语法读取器什么都没说，而在那里猜，正是动态调用被归到某一个实现的来路。
fn initializer_type(expression: &syn::Expr) -> Option<String> {
    match expression {
        syn::Expr::Call(call) => match call.func.as_ref() {
            syn::Expr::Path(path) => {
                let segments = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>();
                match segments.len() {
                    0 | 1 => None,
                    _ => type_name_from_segments(&segments),
                }
            }
            _ => None,
        },
        syn::Expr::Struct(structure) => structure
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Expr::Reference(reference) => initializer_type(&reference.expr),
        syn::Expr::Paren(paren) => initializer_type(&paren.expr),
        syn::Expr::Group(group) => initializer_type(&group.expr),
        _ => None,
    }
}

/// The owner a path names in a constructor position: the segment before the last one.
/// 构造位置上路径点名的所属者：最后一段之前的那一段。
fn type_name_from_segments(segments: &[String]) -> Option<String> {
    segments.get(segments.len().saturating_sub(2)).cloned()
}

fn call_path(path: &syn::ExprPath, current: Option<&str>) -> Option<String> {
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let last = segments.last()?;
    let before = segments.get(segments.len().saturating_sub(2));
    if segments.len() >= 2 {
        let owner = before?;
        if owner == "Self" {
            return current.map(|owner| format!("{owner}::{last}"));
        }
        if owner == "self" || owner == "crate" || owner == "super" {
            return Some(last.clone());
        }
        // A path whose penultimate segment is not a type-looking name is a module-qualified
        // function; the answer still names the last two segments so the owner is visible.
        return Some(format!("{owner}::{last}"));
    }
    Some(last.clone())
}

fn receiver_owner(
    expression: &syn::Expr,
    current: Option<&str>,
    locals: &BTreeMap<String, Option<String>>,
) -> Option<String> {
    match expression {
        syn::Expr::Path(path) => {
            let segments: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let last = segments.last()?;
            if last == "self" || last == "Self" {
                return current.map(str::to_owned);
            }
            // A bare local name whose declared type is written in this function: the syntax states
            // the owner, so this is a definite target rather than a candidate.
            // 一个裸本地名，而它的声明类型写在这个函数里：语法陈述了所属者，因此这是确定目标而不是候选。
            if let Some(Some(owner)) = locals.get(last) {
                return Some(owner.clone());
            }
            // A path receiver is a type the author spelled (`Fast::run(&fast)`), which is also an
            // owner. A lowercase bare name that no declaration typed stays unknown.
            // 路径接收者是作者拼出来的类型（`Fast::run(&fast)`），同样是所属者。没有声明类型的裸小写名
            // 保持未知。
            last.chars()
                .next()
                .filter(|character| character.is_uppercase())
                .map(|_| last.clone())
        }
        syn::Expr::Reference(reference) => receiver_owner(&reference.expr, current, locals),
        syn::Expr::Paren(paren) => receiver_owner(&paren.expr, current, locals),
        syn::Expr::Group(group) => receiver_owner(&group.expr, current, locals),
        _ => None,
    }
}

fn type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Type::Reference(reference) => type_name(&reference.elem),
        syn::Type::Paren(paren) => type_name(&paren.elem),
        syn::Type::Group(group) => type_name(&group.elem),
        _ => None,
    }
}

#[cfg(test)]
#[path = "semantics_tests.rs"]
mod semantics_tests;
