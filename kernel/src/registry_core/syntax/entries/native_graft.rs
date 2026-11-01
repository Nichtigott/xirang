//! Decode compiler-checked const graft arrays without evaluating user code.
//! 解码由编译器检查的 const graft 数组，不执行用户代码。

use quote::ToTokens;
use syn::{Expr, spanned::Spanned};

use super::graft::{GraftEndpointSpan, GraftExpressions, GraftSpans, GraftSyntax};
use crate::syntax::{FaceSyntaxError, SyntaxLocation, compact_tokens, location, syntax_error};

/// The source span of one endpoint expression.
/// 一个端点表达式的源码跨度。
fn endpoint(expr: &Expr) -> GraftEndpointSpan {
    let end = expr.span().end();
    GraftEndpointSpan {
        start: location(expr.span()),
        end: SyntaxLocation {
            line: end.line,
            column: end.column + 1,
        },
    }
}

pub(super) fn entries(
    item: &syn::ItemConst,
    imports: &super::super::imports::Imports,
) -> Result<Option<Vec<GraftSyntax>>, FaceSyntaxError> {
    // The declared type is the opt-in; an arbitrary constant named GRAFTS is not a plan. Two types opt
    // in, one per wrapper: `&[Graft]` for the array, `Plan` for `Plan::of([ … ])`.
    // 声明类型决定是否加入计划；随意命名为 GRAFTS 的常量不是计划。两种类型可以加入，各对应一种包装：
    // 数组用 `&[Graft]`，`Plan::of([ … ])` 用 `Plan`。
    let last = match item.ty.as_ref() {
        syn::Type::Reference(reference) => match reference.elem.as_ref() {
            syn::Type::Slice(slice) => match slice.elem.as_ref() {
                syn::Type::Path(path) => path
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string()),
                _ => None,
            },
            _ => None,
        },
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    };
    if !matches!(
        last.as_deref(),
        Some("Graft") | Some("Plan") | Some("PlanEntry")
    ) {
        return Ok(None);
    }
    let expression = match item.expr.as_ref() {
        Expr::Reference(reference) => reference.expr.as_ref(),
        other => other,
    };
    // Two wrappers, both at **item** position, both keeping the declared type as the opt-in:
    //   `const PLAN: &[Graft] = &[ … ];`   — the array itself
    //   `const PLAN: Plan = Plan::of([ … ]);` — the readable wrapper (`Plan::of`), whose elements are the
    //   chains `cut(a).graft(b)` / `cut(a).to(b).graft(c)` / `cut(a).graft(b).full()`
    // A function body is still ignored (`visit_item_fn`), because a plan inside one is a runtime
    // expression — the reason that visitor is empty is a past defect: test cuts reached the release table.
    // 两种包装，都在**条目**位置，且都以声明类型作为 opt-in：
    //   `const PLAN: &[Graft] = &[ … ];`   —— 数组本身
    //   `const PLAN: Plan = Plan::of([ … ]);` —— 可读包装（`Plan::of`），元素就是那些链
    //   `cut(a).graft(b)` / `cut(a).to(b).graft(c)` / `cut(a).graft(b).full()`
    // 函数体依旧被忽略（`visit_item_fn`），因为写在里面的计划是运行时表达式——那个访问器留空是有理由的：
    // 过去测试切口真的进过发布表。
    let expression = match (last.as_deref(), expression) {
        (Some("Graft") | Some("PlanEntry"), _) => expression,
        (Some("Plan"), Expr::Call(call)) => {
            let is_of = matches!(
                call.func.as_ref(),
                Expr::Path(path)
                    if path.path.segments.last().is_some_and(|s| s.ident == "of")
            );
            if !is_of || call.args.len() != 1 {
                return Err(syntax_error(
                    item.expr.span(),
                    "a plan declaration is written `Plan::of([ … ])`",
                ));
            }
            match call.args.first() {
                Some(Expr::Array(array)) => {
                    return decode_array(array, imports);
                }
                Some(other) => {
                    return Err(syntax_error(
                        other.span(),
                        "a plan declaration requires a literal array",
                    ));
                }
                None => return Err(syntax_error(item.expr.span(), "`Plan::of` needs one array")),
            }
        }
        _ => return Ok(None),
    };
    let Expr::Array(array) = expression else {
        return Err(syntax_error(
            expression.span(),
            "a static graft declaration requires a literal array",
        ));
    };
    decode_array(array, imports)
}

/// Decode one literal array of graft entries.
/// 解一个 graft 条目的字面量数组。
fn decode_array(
    array: &syn::ExprArray,
    imports: &super::super::imports::Imports,
) -> Result<Option<Vec<GraftSyntax>>, FaceSyntaxError> {
    array
        .elems
        .iter()
        .map(|expr| entry(expr, imports))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// Decode one endpoint: a string literal is a logical path, a Rust path is a typed cut.
/// 解一个端点：字符串字面量是逻辑路径，Rust 路径是类型化切口。
fn decode_chain(
    expr: &Expr,
    imports: &super::super::imports::Imports,
) -> Result<(String, bool), FaceSyntaxError> {
    match expr {
        Expr::Lit(literal) => match &literal.lit {
            syn::Lit::Str(value) => Ok((value.value(), false)),
            _ => Err(syntax_error(
                expr.span(),
                "static graft endpoints must be Rust paths or string literals",
            )),
        },
        Expr::Path(_) => Ok((
            crate::syntax::catalog_reference(&super::super::imports::expand(
                &compact_tokens(expr.to_token_stream()),
                imports,
            )),
            true,
        )),
        _ => Err(syntax_error(
            expr.span(),
            "static graft endpoints must be Rust paths or string literals",
        )),
    }
}

fn call<'a>(expr: &'a Expr, name: &str, count: usize) -> Option<Vec<&'a Expr>> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    (path.path.segments.last()?.ident == name && call.args.len() == count)
        .then(|| call.args.iter().collect())
}

/// The readable chain (C2′): `cut(<target>).graft(<replacement>)`, with an optional `.to(<far>)` for a
/// sibling range or `.full()` for a whole subtree.
/// 可读链（C2′）：`cut(<目标>).graft(<替换>)`，可带 `.to(<远端>)` 表示兄弟区间，或 `.full()` 表示整棵子树。
///
/// The chain's head is the **innermost** receiver — `cut(a).graft(b)` is a method call whose receiver is
/// the `cut` call — so this finds the innermost node first and only then decides which spelling it is.
/// That is the one detail an implementation gets wrong: probing the whole expression for `cut` never
/// matches, and then the branch is dead while every older pin stays green.
/// 链的头是**最内层**的接收者——`cut(a).graft(b)` 是一个方法调用，而它的接收者是那个 `cut` 调用——因此这里
/// 先找到最内层的节点，再判断这是哪种拼法。这正是实现最容易搞错的一处：在整个表达式上探 `cut` 永远匹配不到，
/// 于是那一支是死的，而所有旧钉子依旧全绿。
fn split_chain(expr: &Expr) -> Option<(Option<&Expr>, Option<&Expr>, bool)> {
    let mut innermost = expr;
    while let Expr::MethodCall(method) = innermost {
        innermost = &method.receiver;
    }
    call(innermost, "cut", 1)?;
    let mut tail = expr;
    let mut replacement: Option<&Expr> = None;
    let mut end: Option<&Expr> = None;
    let mut full = false;
    while let Expr::MethodCall(method) = tail {
        // Process **this** link first, and only then stop: the last link before `cut` is the one whose
        // receiver *is* the head, so breaking before the match drops it — a chain written
        // `cut(a).to(b).graft(c)` then loses `.to(b)` silently.
        // 先处理**当前**这一节，再决定停：紧挨着 `cut` 的那一节，其接收者**就是**链头，因此在 match 之前
        // 就 break 会把它丢掉——写成 `cut(a).to(b).graft(c)` 的链会**静默**丢掉 `.to(b)`。
        match method.method.to_string().as_str() {
            "graft" if method.args.len() == 1 && replacement.is_none() => {
                replacement = Some(&method.args[0]);
            }
            "to" if method.args.len() == 1 && end.is_none() => end = Some(&method.args[0]),
            "full" if method.args.is_empty() => full = true,
            _ => return None,
        }
        if std::ptr::eq(method.receiver.as_ref(), innermost) {
            break;
        }
        tail = &method.receiver;
    }
    Some((replacement, end, full))
}

fn entry(
    expr: &Expr,
    imports: &super::super::imports::Imports,
) -> Result<GraftSyntax, FaceSyntaxError> {
    let error = || {
        syntax_error(
            expr.span(),
            "expected graft(node | range(start, end) | subtree(...)).with(replacement)",
        )
    };
    // Two element spellings state the same three shapes, and the reader takes both so a host can
    // move from one to the other without a flag day:
    //   1. `graft(<a>).with(<replacement>)` / `graft(range(<a>, <b>)).with(...)` / `graft(subtree(<a>)).with(...)`
    //   2. `cut(<a>).graft(<replacement>)` / `cut(<a>).to(<b>).graft(...)` / `cut(<a>).graft(...).full()`
    // The second is the readable chain the plan ruling asked for (C2′): the target comes first, the
    // replacement second, and the shape is a method rather than a wrapper — so `.to(…)` and `.full()`
    // read as what they mean. A chain that uses both is refused by name, exactly as the first spelling
    // refuses `subtree(range(…))`.
    // 两种元素拼法陈述同样三种形状，读法两者都收，于是宿主不必等一个"改旗之日"：
    //   1. `graft(<a>).with(<替换>)` / `graft(range(<a>, <b>)).with(...)` / `graft(subtree(<a>)).with(...)`
    //   2. `cut(<a>).graft(<替换>)` / `cut(<a>).to(<b>).graft(...)` / `cut(<a>).graft(...).full()`
    // 第二种就是计划裁定（C2′）要的可读链：目标在前、替换在后，形状是方法而不是包装——于是
    // `.to(…)` 与 `.full()` 读起来就是它们的意思。两者同时用会**按名拒绝**，与第一种拼法拒绝
    // `subtree(range(…))` 完全对称。
    // The chain spelling states the target first and the shape as a method (`cut(a).graft(b).to(c)`),
    // which reads the way it means; the older spelling wraps it (`graft(range(a, c)).with(b)`). Both are
    // accepted, and they decode to the same `GraftSyntax` — pinned field for field.
    // 链形把目标写在前、形状写成方法（`cut(a).graft(b).to(c)`），读起来就是它的意思；旧拼法是包装式的
    // （`graft(range(a, c)).with(b)`）。两者都收，且解出同一个 `GraftSyntax`——逐字段钉住。
    if let Some((replacement, end, full)) = split_chain(expr) {
        let target = call(
            {
                let mut innermost = expr;
                while let Expr::MethodCall(method) = innermost {
                    innermost = &method.receiver;
                }
                innermost
            },
            "cut",
            1,
        )
        .ok_or_else(error)?[0];
        if full && end.is_some() {
            return Err(syntax_error(
                expr.span(),
                "`to(…)` and `full()` name two different shapes; use one of them",
            ));
        }
        let replacement = replacement.ok_or_else(error)?;
        let (cut, typed) = decode_chain(target, imports)?;
        let cut_end = end.map(|e| decode_chain(e, imports)).transpose()?;
        let (graft, replacement_typed) = decode_chain(replacement, imports)?;
        if replacement_typed != typed || cut_end.as_ref().is_some_and(|(_, t)| *t != typed) {
            return Err(syntax_error(
                expr.span(),
                "graft endpoints must all use identities or all use strings",
            ));
        }
        let cut_end = cut_end.map(|(value, _)| value);
        return Ok(GraftSyntax {
            expressions: typed.then(|| GraftExpressions {
                cut: cut.clone(),
                cut_end: cut_end.clone(),
                graft: graft.clone(),
            }),
            cut,
            cut_end,
            graft,
            full,
            location: location(expr.span()),
            cfg: None,
            spans: typed.then(|| GraftSpans {
                cut: endpoint(target),
                cut_end: end.map(endpoint),
                graft: endpoint(replacement),
            }),
        });
    }
    let Expr::MethodCall(method) = expr else {
        return Err(error());
    };
    if method.method != "with" || method.args.len() != 1 || method.turbofish.is_some() {
        return Err(error());
    }
    let target = call(&method.receiver, "graft", 1).ok_or_else(error)?[0];
    let replacement = &method.args[0];
    let full = call(target, "subtree", 1);
    let target = full.as_ref().map_or(target, |args| args[0]);
    let endpoints = call(target, "range", 2);
    if full.is_some() && endpoints.is_some() {
        return Err(syntax_error(
            expr.span(),
            "subtree(range(...)) has no defined static graft meaning; use one subtree per face",
        ));
    }
    let (start, end) = endpoints
        .as_ref()
        .map_or((target, None), |args| (args[0], Some(args[1])));
    let decode = |expr: &Expr| -> Result<(String, bool), FaceSyntaxError> {
        match expr {
            Expr::Lit(literal) => match &literal.lit {
                syn::Lit::Str(value) => Ok((value.value(), false)),
                _ => Err(error()),
            },
            Expr::Path(_) => Ok((
                crate::syntax::catalog_reference(&super::super::imports::expand(
                    &compact_tokens(expr.to_token_stream()),
                    imports,
                )),
                true,
            )),
            _ => Err(syntax_error(
                expr.span(),
                "static graft endpoints must be Rust paths or string literals",
            )),
        }
    };
    let (cut, typed) = decode(start)?;
    let cut_end = end.map(decode).transpose()?;
    let (graft, replacement_typed) = decode(replacement)?;
    if replacement_typed != typed || cut_end.as_ref().is_some_and(|(_, t)| *t != typed) {
        return Err(syntax_error(
            expr.span(),
            "graft endpoints must all use identities or all use strings",
        ));
    }
    let cut_end = cut_end.map(|(value, _)| value);
    Ok(GraftSyntax {
        expressions: typed.then(|| GraftExpressions {
            cut: cut.clone(),
            cut_end: cut_end.clone(),
            graft: graft.clone(),
        }),
        cut,
        cut_end,
        graft,
        full: full.is_some(),
        location: location(expr.span()),
        cfg: None,
        spans: typed.then(|| GraftSpans {
            cut: endpoint(start),
            cut_end: end.map(endpoint),
            graft: endpoint(replacement),
        }),
    })
}

/// Write a native typed graft expression, preserving the existing static selection rules.
/// 写出原生类型化 graft 表达式，保留既有静态选择规则。
pub fn render_native_graft_expression(
    cut: &str,
    end: Option<&str>,
    full: bool,
    replacement: &str,
) -> Result<String, FaceSyntaxError> {
    if full && end.is_some() {
        return Err(syntax_error(
            proc_macro2::Span::call_site(),
            "subtree(range(...)) has no defined static graft meaning",
        ));
    }
    for path in [Some(cut), end, Some(replacement)].into_iter().flatten() {
        syn::parse_str::<syn::Expr>(path)
            .and_then(|expr| {
                if matches!(
                    expr,
                    Expr::Path(_)
                        | Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(_),
                            ..
                        })
                ) {
                    Ok(expr)
                } else {
                    Err(syn::Error::new(
                        expr.span(),
                        "expected an identity path or string literal",
                    ))
                }
            })
            .map_err(|error| syntax_error(error.span(), "expected a Rust identity path"))?;
    }
    let logical = |path: &str| {
        if path.starts_with("crate::")
            && path.ends_with("::NODE_ID")
            && !path.starts_with("crate::catalog::")
        {
            path.replacen("crate::", "crate::catalog::", 1)
        } else {
            path.to_owned()
        }
    };
    let mut target = end.map_or_else(
        || logical(cut),
        |end| {
            format!(
                "xirang_toolchain::range({}, {})",
                logical(cut),
                logical(end)
            )
        },
    );
    if full {
        target = format!("xirang_toolchain::subtree({target})");
    }
    Ok(format!(
        "xirang_toolchain::graft({target}).with({})",
        logical(replacement)
    ))
}

/// Append a native plan declaration without modifying existing legacy or cfg-gated plans.
/// 追加原生计划声明，不修改既有旧语法或带 cfg 的计划。
pub fn append_native_graft(source: &str, expression: &str) -> Result<String, FaceSyntaxError> {
    let file = super::super::nesting::parse_file(source)?;
    let mut index = 0;
    let name = loop {
        let name = format!("XIRANG_GRAFTS_{index}");
        if !file
            .items
            .iter()
            .any(|item| matches!(item, syn::Item::Const(item) if item.ident == name))
        {
            break name;
        }
        index += 1;
    };
    let addition = format!(
        "\n/// Graft declaration.\n/// Graft 声明。\npub const {name}: &[xirang_toolchain::Graft] = &[\n    {expression},\n];\n"
    );
    let entries = super::graft::graft_entries(&addition)?;
    if entries.len() != 1 {
        return Err(syntax_error(
            proc_macro2::Span::call_site(),
            "expected one new graft",
        ));
    }
    Ok(format!("{source}{addition}"))
}

/// Rebind one native graft to its own target, preserving aliases, cfgs and surrounding source.
/// 将一条原生 graft 重新绑定到自身目标，保留别名、cfg 与周围源码。
pub fn repoint_native_graft(
    source: &str,
    cut: &str,
    graft: &str,
) -> Result<Option<String>, FaceSyntaxError> {
    use syn::visit::Visit;
    let entries = super::graft::graft_entries(source)?;
    let Some(selected) = entries
        .iter()
        .find(|entry| entry.cut == cut && entry.graft == graft)
    else {
        return Ok(None);
    };
    struct Repoint<'a> {
        location: &'a crate::syntax::SyntaxLocation,
        spans: Option<(proc_macro2::Span, proc_macro2::Span)>,
    }
    impl<'ast> Visit<'ast> for Repoint<'_> {
        fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
            if location(method.span()) == *self.location
                && method.method == "with"
                && method.args.len() == 1
                && let Some(args) = call(&method.receiver, "graft", 1)
            {
                let target = args[0];
                let subtree = call(target, "subtree", 1);
                let target = subtree.as_ref().map_or(target, |args| args[0]);
                let range = call(target, "range", 2);
                let start = range.as_ref().map_or(target, |args| args[0]);
                self.spans = Some((start.span(), method.args[0].span()));
            }
            syn::visit::visit_expr_method_call(self, method);
        }
    }
    let file = super::super::nesting::parse_file(source)?;
    let mut visitor = Repoint {
        location: &selected.location,
        spans: None,
    };
    visitor.visit_file(&file);
    let Some((target, replacement)) = visitor.spans else {
        return Ok(None);
    };
    let range = |span: proc_macro2::Span| -> Result<std::ops::Range<usize>, FaceSyntaxError> {
        use super::super::face::source_offset;
        Ok(source_offset(source, &location(span))?
            ..source_offset(
                source,
                &crate::syntax::SyntaxLocation {
                    line: span.end().line,
                    column: span.end().column + 1,
                },
            )?)
    };
    let mut output = source.to_owned();
    output.replace_range(range(replacement)?, &source[range(target)?]);
    Ok(Some(output))
}

#[cfg(test)]
#[path = "native_graft_tests.rs"]
mod native_graft_tests;
