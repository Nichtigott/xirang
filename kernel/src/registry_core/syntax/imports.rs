//! Resolve explicit Rust imports in the lexical scope of native declarations.
//! 在原生声明的词法作用域内解析显式 Rust 导入。

use std::collections::BTreeMap;

pub(super) type Imports = BTreeMap<String, String>;

pub(super) fn collect(items: &[syn::Item], imports: &mut Imports) {
    for item in items {
        if let syn::Item::Use(item) = item {
            // Conditional imports cannot be selected without the build's cfg environment.
            // 没有构建的 cfg 环境，不能选择条件导入。
            if !item.attrs.iter().any(|attr| attr.path().is_ident("cfg")) {
                visit_import(&item.tree, "", imports);
            }
        }
    }
}

fn visit_import(tree: &syn::UseTree, prefix: &str, imports: &mut Imports) {
    match tree {
        syn::UseTree::Path(path) => {
            visit_import(&path.tree, &format!("{prefix}{}::", path.ident), imports)
        }
        syn::UseTree::Name(name) => {
            imports.insert(name.ident.to_string(), format!("{prefix}{}", name.ident));
        }
        syn::UseTree::Rename(rename) => {
            imports.insert(
                rename.rename.to_string(),
                format!("{prefix}{}", rename.ident),
            );
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                visit_import(item, prefix, imports);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

pub(super) fn expand(path: &str, imports: &Imports) -> String {
    let mut path = path.to_owned();
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let (head, tail) = path.split_once("::").unwrap_or((&path, ""));
        if !seen.insert(head.to_owned()) {
            break;
        }
        let Some(imported) = imports.get(head) else {
            break;
        };
        path = if tail.is_empty() {
            imported.clone()
        } else {
            format!("{imported}::{tail}")
        };
    }
    path
}
