//! The Rust builder spelling of a face, normalized into the shared field vocabulary.
//! 注册面的 Rust builder 写法，归一化到共用字段词表。

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, spanned::Spanned};

use super::face::{FaceSyntaxError, syntax_error};

/// Normalize a native const declaration for the compiler and source readers.
/// 为编译器与源码读取器归一化原生 const 声明。
pub fn native_face_fields(item: &syn::ItemConst) -> Result<TokenStream, FaceSyntaxError> {
    let mut fields = Vec::<(String, TokenStream)>::new();
    let mut cursor = item.expr.as_ref();
    while let Expr::MethodCall(call) = cursor {
        let name = call.method.to_string();
        let name = if name == "under" { "parent" } else { &name };
        if !crate::declaration::FACE_FIELD_ORDER.contains(&name) {
            return Err(syntax_error(
                call.method.span(),
                format!("unknown face method `{name}`"),
            ));
        }
        let args = call.args.iter().collect::<Vec<_>>();
        let types = call
            .turbofish
            .as_ref()
            .map(|t| t.args.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let value = match name {
            "preset" | "parts" | "flow_provider" if args.is_empty() && types.len() == 1 => {
                types[0].to_token_stream()
            }
            "handle_contracts" | "part_contracts" if args.len() == 1 && types.is_empty() => {
                let Expr::Macro(mac) = args[0] else {
                    return Err(syntax_error(
                        call.span(),
                        "expected contracts!(Type: Trait, ...)",
                    ));
                };
                if mac
                    .mac
                    .path
                    .segments
                    .last()
                    .is_none_or(|s| s.ident != "contracts")
                {
                    return Err(syntax_error(
                        call.span(),
                        "expected contracts!(Type: Trait, ...)",
                    ));
                }
                let (_, paths) = contract_paths(mac.mac.tokens.clone())
                    .map_err(|e| syntax_error(e.span(), e.to_string()))?;
                quote!([#(#paths),*])
            }
            "name" | "summary" if args.len() == 2 && types.is_empty() => {
                let en = args[0];
                let zh = args[1];
                quote!({zh: #zh, en: #en})
            }
            "requires" if args.len() == 2 && types.is_empty() => {
                let capability = args[0];
                let provider = args[1];
                quote!(#capability => #provider)
            }
            _ if !matches!(
                name,
                "kind"
                    | "preset"
                    | "parts"
                    | "flow_provider"
                    | "name"
                    | "summary"
                    | "requires"
                    | "handle_contracts"
                    | "part_contracts"
            ) && args.len() == 1
                && types.is_empty() =>
            {
                match args[0] {
                    Expr::Reference(reference)
                        if matches!(reference.expr.as_ref(), Expr::Array(_)) =>
                    {
                        reference.expr.to_token_stream()
                    }
                    value => value.to_token_stream(),
                }
            }
            _ => {
                return Err(syntax_error(
                    call.span(),
                    format!("invalid arguments for face method `{name}`"),
                ));
            }
        };
        fields.push((name.to_owned(), value));
        cursor = &call.receiver;
    }
    let Expr::Call(call) = cursor else {
        return Err(syntax_error(cursor.span(), "expected Soil::of::<Kind>()"));
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return Err(syntax_error(call.span(), "expected Soil::of::<Kind>()"));
    };
    let Some(segment) = path.path.segments.last().filter(|s| s.ident == "of") else {
        return Err(syntax_error(path.span(), "expected Soil::of::<Kind>()"));
    };
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syntax_error(segment.span(), "expected one Kind type"));
    };
    if arguments.args.len() != 1 || !call.args.is_empty() {
        return Err(syntax_error(call.span(), "expected Soil::of::<Kind>()"));
    }
    let kind = arguments.args.first().unwrap();
    fields.push(("kind".to_owned(), kind.to_token_stream()));
    fields.reverse();
    let mut output = TokenStream::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut requirements = Vec::new();
    for (name, value) in fields {
        if name == "requires" {
            requirements.push(value);
            continue;
        }
        if !seen.insert(name.clone()) {
            return Err(syntax_error(
                item.span(),
                format!("duplicate face method `{name}`"),
            ));
        }
        let ident = syn::Ident::new(&name, value.span());
        output.extend(quote!(#ident: #value,));
    }
    if !requirements.is_empty() {
        output.extend(quote!(requires: [#(#requirements),*],));
    }
    Ok(output)
}

/// Whether this item carries the native registration attribute.
/// 条目是否携带原生注册属性。
pub fn has_face_attribute(attrs: &[syn::Attribute]) -> bool {
    face_attribute_name(attrs).is_some()
}

pub(crate) fn face_attribute_name(attrs: &[syn::Attribute]) -> Option<&syn::Ident> {
    attrs.iter().find_map(|attr| {
        attr.path()
            .segments
            .last()
            .filter(|segment| segment.ident == "soil" || segment.ident == "face")
            .map(|segment| &segment.ident)
    })
}

/// Render a legacy declaration as a real Rust const, preserving every written field.
/// 将旧声明渲染为真实 Rust const，保留每个已写字段。
pub fn render_native_face(source: &str) -> Result<String, FaceSyntaxError> {
    let face = super::parse_face(source)?
        .ok_or_else(|| syntax_error(proc_macro2::Span::call_site(), "missing face"))?;
    let kind = face
        .field("kind")
        .ok_or_else(|| syntax_error(proc_macro2::Span::call_site(), "missing kind"))?;
    let attribute = face.field("collector").map_or_else(
        || "#[xirang_toolchain::soil]".to_owned(),
        |collector| format!("#[xirang_toolchain::soil(collector = {collector})]"),
    );
    let mut out = format!(
        "{attribute}\npub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<{kind}>()"
    );
    for &field in crate::declaration::FACE_FIELD_ORDER {
        let Some(value) = face.field(field) else {
            continue;
        };
        let method = match field {
            "kind" => continue,
            "parent" => format!("under({value})"),
            "preset" | "parts" | "flow_provider" => format!("{field}::<{value}>()"),
            "name" | "summary" => {
                let en = face.localized(field, "en").ok_or_else(|| {
                    syntax_error(
                        proc_macro2::Span::call_site(),
                        "localized value must be a string",
                    )
                })?;
                let zh = face.localized(field, "zh").ok_or_else(|| {
                    syntax_error(
                        proc_macro2::Span::call_site(),
                        "localized value must be a string",
                    )
                })?;
                format!("{field}({en:?}, {zh:?})")
            }
            "requires" => {
                for (capability, provider) in face.requirements(field).ok_or_else(|| {
                    syntax_error(
                        proc_macro2::Span::call_site(),
                        "requirements must be literal pairs",
                    )
                })? {
                    out.push_str(&format!("\n    .requires({capability:?}, {provider:?})"));
                }
                continue;
            }
            "handle_contracts" | "part_contracts" => {
                let paths = face.path_list(field).unwrap_or_default();
                if paths.is_empty() {
                    continue;
                }
                format!(
                    "{field}(xirang_toolchain::contracts!({}: {}))",
                    if field == "handle_contracts" {
                        kind.clone()
                    } else {
                        face.field("parts")
                            .unwrap_or_else(|| "xirang_toolchain::NoParts".into())
                    },
                    paths.join(", ")
                )
            }
            "exports" | "handle_traits" | "part_traits" | "provides" | "runtime_checks" => {
                format!("{field}(&{value})")
            }
            _ => format!("{field}({value})"),
        };
        out.push_str(&format!("\n    .{method}"));
    }
    out.push(';');
    super::replace_face_macro(source, &out)
}

/// Parse a compiler-checked contract witness, including non-object-safe traits.
/// 解析经过编译器检查的合同见证，支持不能构造 trait object 的 trait。
pub fn contract_paths(tokens: TokenStream) -> syn::Result<(syn::Type, Vec<syn::Path>)> {
    use syn::parse::Parser;
    let parser = |input: syn::parse::ParseStream<'_>| {
        let ty = input.parse::<syn::Type>()?;
        input.parse::<syn::Token![:]>()?;
        let paths =
            syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated(input)?;
        Ok((ty, paths.into_iter().collect()))
    };
    parser.parse2(tokens)
}

/// Resolve a logical catalog reference to its source module identity spelling.
/// 将逻辑目录引用解析为源码模块身份的拼写。
pub fn catalog_reference(path: &str) -> String {
    let Some((prefix, rest)) = path.split_once("catalog::") else {
        return path.to_owned();
    };
    if !prefix.is_empty() && !prefix.ends_with("::") {
        return path.to_owned();
    }
    if rest.ends_with("::NODE_ID") {
        return format!("{prefix}{rest}");
    }
    let (parent, name) = rest.rsplit_once("::").unwrap_or(("", rest));
    if name.is_empty() || name.chars().any(|c| c.is_ascii_lowercase()) {
        return path.to_owned();
    }
    let separator = if parent.is_empty() { "" } else { "::" };
    format!(
        "{prefix}{parent}{separator}{}::NODE_ID",
        name.to_lowercase()
    )
}
