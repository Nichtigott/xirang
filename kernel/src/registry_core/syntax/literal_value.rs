//! One field value written in the struct-literal form, normalized to the chain spelling's text.
//! 结构体字面量形写下的一个字段值，归一成链式拼法产出的文本。
//!
//! The chain spelling (`native_face.rs`) rewrites what the author wrote into the shape every reader
//! consumes; this module performs the same rewrites for the spelling that writes the field's **value**
//! (`Registration`'s own type), so both reach the readers with one text instead of two.
//! 链式拼法（`native_face.rs`）把作者的写法整成每个读者消费的形状；本模块为"写字段**值**"（类型就是
//! `Registration` 自己的）的那一形做同样的改写，于是两形到达读者的是同一段文本，而不是两段。
use quote::ToTokens;

/// One field value written in the struct-literal form, normalized to the text the chain spelling
/// produces for the same field.
/// 结构体字面量形写下的一个字段值，归一成链式拼法为同一字段产出的文本。
///
/// The rewrites are the ones `native_face.rs` performs on its own side, so both spellings reach every
/// reader with the same text: the chain spelling passes a bare value where this form writes `Some(x)`,
/// passes the array where this form writes `&[..]`, and writes a brace group where this form names the
/// value's type. Keeping them here — rather than teaching each reader two shapes — is what makes the two
/// forms interchangeable instead of merely compatible.
/// 这些改写就是 `native_face.rs` 在它那一侧所做的，因此两形到达每个读者时文本相同：链式拼法传裸值的地方本形
/// 写 `Some(x)`、传数组的地方本形写 `&[..]`、写花括号组的地方本形点名值的类型。把它们放在这里——而不是教每个
/// 读者认两种形状——才让两形**可互换**，而不仅仅是**兼容**。
pub(super) fn normalized_literal_value(expression: &syn::Expr) -> String {
    match expression {
        // `.stable_name("x")` / `.flow(FlowContract::new(..))` take the bare value.
        // `.stable_name("x")` / `.flow(FlowContract::new(..))` 收的是裸值。
        syn::Expr::Call(call) if is_some_call(call) && call.args.len() == 1 => {
            normalized_literal_value(&call.args[0])
        }
        // `.exports(&["x"])` / `.runtime_checks(&[NON_EMPTY_TEXT])` take the array itself.
        // `.exports(&["x"])` / `.runtime_checks(&[NON_EMPTY_TEXT])` 收的是数组本身。
        syn::Expr::Reference(reference)
            if matches!(reference.expr.as_ref(), syn::Expr::Array(_)) =>
        {
            normalized_literal_value(&reference.expr)
        }
        syn::Expr::Array(array) => {
            let elements = array
                .elems
                .iter()
                .map(normalized_literal_value)
                .collect::<Vec<_>>();
            format!("[{}]", elements.join(", "))
        }
        // `.name("Button", "按钮")` writes a brace group; this form names `LocalizedText`.
        // `.name("Button", "按钮")` 写的是花括号组；本形点名 `LocalizedText`。
        syn::Expr::Struct(literal) if path_ends_with(&literal.path, "LocalizedText") => {
            let fields = literal
                .fields
                .iter()
                .map(|field| {
                    let member = field.member.to_token_stream();
                    let value = normalized_literal_value(&field.expr);
                    format!("{member}: {value}")
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", fields.join(", "))
        }
        // `.requires("render", "Renderer")` writes `capability => provider`.
        // `.requires("render", "Renderer")` 写的是 `capability => provider`。
        syn::Expr::Struct(literal) if path_ends_with(&literal.path, "RequirementSpec") => {
            let named = |wanted: &str| {
                literal
                    .fields
                    .iter()
                    .find(
                        |field| matches!(&field.member, syn::Member::Named(name) if name == wanted),
                    )
                    .map(|field| normalized_literal_value(&field.expr))
            };
            match (named("capability"), named("provider")) {
                (Some(capability), Some(provider)) => format!("{capability} => {provider}"),
                _ => expression.to_token_stream().to_string(),
            }
        }
        value => value.to_token_stream().to_string(),
    }
}

/// Whether a call expression is `Some(..)`.
/// 一个调用表达式是否为 `Some(..)`。
fn is_some_call(call: &syn::ExprCall) -> bool {
    matches!(&*call.func, syn::Expr::Path(path) if path.path.is_ident("Some"))
}

/// Whether a path's last segment is the given name.
/// 一个路径的最后一段是否为给定名字。
fn path_ends_with(path: &syn::Path, name: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}
