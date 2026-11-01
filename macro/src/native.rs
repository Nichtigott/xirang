//! Registration glue for a preserved, compiler-checked Rust const.
//! 为保留原文、由编译器检查的 Rust const 生成注册接线。

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let item: syn::ItemConst = syn::parse2(input)?;
    let fields = xirang_kernel::syntax::native_face_fields(&item)
        .map_err(|error| syn::Error::new(item.span(), error.message))?;
    let parsed = xirang_kernel::syntax::parse_face(&format!("external_object! {{ {fields} }}"))
        .map_err(|error| syn::Error::new(item.span(), error.message))?
        .unwrap();
    let field = |name: &str| -> syn::Result<Option<TokenStream>> {
        parsed
            .field(name)
            .map(|text| {
                text.parse()
                    .map_err(|error| syn::Error::new(item.span(), format!("{error}")))
            })
            .transpose()
    };
    let runtime = quote!(::xirang_toolchain);
    let name = &item.ident;
    let kind = field("kind")?.ok_or_else(|| syn::Error::new(item.span(), "missing kind"))?;
    let preset = field("preset")?.unwrap_or_else(|| quote!(#runtime::NoPreset));
    let parts = field("parts")?.unwrap_or_else(|| quote!(#runtime::NoParts));
    let preset_name = if parsed.field("preset").is_some() {
        quote!(stringify!(#preset))
    } else {
        quote!("NoPreset")
    };
    let parts_name = if parsed.field("parts").is_some() {
        quote!(stringify!(#parts))
    } else {
        quote!("NoParts")
    };
    let parent = if parsed.field("parent").is_some() {
        quote!(#name.metadata().parent)
    } else {
        quote!(#runtime::root_node_id(crate::XIRANG_NAMESPACE))
    };
    let source = if parsed.field("source").is_some() {
        quote!(#name.metadata().source.file)
    } else {
        quote!(#runtime::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()))
    };
    let rule_path = if parsed.field("registry_rule_path").is_some() {
        quote!(#name.metadata().registry_rule_path)
    } else {
        source.clone()
    };
    let needs = field("needs_registry")?.unwrap_or_default();
    let rule = field("registry_rule")?.unwrap_or_default();
    let display_name = if parsed.field("name").is_some() {
        quote!(#name.metadata().name)
    } else {
        quote!(#runtime::LocalizedText { en: stringify!(#kind), zh: stringify!(#kind) })
    };
    let flow_provider = match field("flow_provider")? {
        Some(provider) => quote!(Some(stringify!(#provider))),
        None => quote!(None),
    };
    let flow = field("flow")?.unwrap_or_else(|| quote!(#name.metadata().flow));
    let mut checks = Vec::new();
    let mut labels = Vec::new();
    for (path_field, label_field, ty) in [
        ("handle_contracts", "handle_traits", &kind),
        ("part_contracts", "part_traits", &parts),
    ] {
        let paths = parsed
            .path_list(path_field)
            .unwrap_or_default()
            .into_iter()
            .map(|path| syn::parse_str::<syn::Path>(&path))
            .collect::<syn::Result<Vec<_>>>()?;
        let label = syn::Ident::new(label_field, item.span());
        if paths.is_empty() {
            labels.push(quote!(#label: #name.metadata().#label,));
        } else {
            checks.push(quote!(#runtime::__assert_impls!(#ty; [#(#paths),*]);));
            let names = paths
                .iter()
                .map(|path| path.segments.last().unwrap().ident.to_string())
                .collect::<Vec<_>>();
            labels.push(quote!(#label: &[#(#names),*],));
        }
    }
    let mut requirements = 0usize;
    let mut expression = item.expr.as_ref();
    while let syn::Expr::MethodCall(call) = expression {
        requirements += usize::from(call.method == "requires");
        expression = &call.receiver;
    }
    let collector = if args.is_empty() {
        quote!(development)
    } else {
        let meta: syn::MetaNameValue = syn::parse2(args)?;
        if !meta.path.is_ident("collector") {
            return Err(syn::Error::new(
                meta.span(),
                "expected collector = development, linked, or debug",
            ));
        }
        let value = meta.value;
        quote!(#value)
    };
    let cfgs = item
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
        .collect::<Vec<_>>();
    let mut generated = vec![
        quote!(const _: () = #runtime::assert_contract::<#preset, #parts>();),
        quote!(#[doc(hidden)] pub type __Preset = #preset;),
        quote!(#[doc(hidden)] pub type __Parts = #parts;),
        quote!(
            /// The declaration's node identity.
            /// 声明的节点身份。
            pub const NODE_ID: #runtime::NodeId = #runtime::NodeId::from_namespaced_path(crate::XIRANG_NAMESPACE, #source, stringify!(#kind));
        ),
        quote!(
            /// The compiler-checked registration metadata.
            /// 经过编译器检查的注册元数据。
            pub const REGISTRATION: #runtime::RegistrationInfo = #runtime::RegistrationInfo {
                namespace: crate::XIRANG_NAMESPACE, id: NODE_ID, parent: #parent,
                kind: stringify!(#kind), preset: #preset_name, parts: #parts_name,
                params: stringify!(#kind), handle: stringify!(#kind), name: #display_name,
                registry_name: #runtime::last_path_segment(module_path!()),
                registry_rule_path: #rule_path,
                registry_rule: #runtime::__face_rule_or!(#runtime::RegistrationRule::ANY; #needs; #rule),
                requires: &#name.requirement_array::<#requirements>(),
                flow: #flow, flow_provider: #flow_provider, #(#labels)*
                source: #runtime::SourceLocation { file: #source, line: line!(), column: column!(), function: stringify!(#kind) },
                ..#name.metadata()
            };
        ),
        quote!(#runtime::__submit_registration!(#collector; REGISTRATION);),
    ];
    generated.extend(checks);
    let mut output = quote!(
        /// The native registration declaration.
        /// 原生注册声明。
        #item
    );
    for generated in generated {
        output.extend(quote!(#(#cfgs)* #generated));
    }
    Ok(output)
}
