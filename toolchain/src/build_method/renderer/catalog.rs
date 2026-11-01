//! Compile the logical identity catalog independently of implementation ownership.
//! 编译逻辑身份目录，与实现归属解耦。

use crate::build_method::static_plan::StaticFaceRecord;
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Default)]
struct Branch {
    id: Option<[u8; 16]>,
    children: BTreeMap<String, Branch>,
}

pub(crate) fn render_catalog(
    faces: &[StaticFaceRecord],
    logical: &[(String, [u8; 16])],
) -> Result<String, String> {
    let mut root = Branch::default();
    // The legacy module-path catalog stays: existing declarations name `catalog::<module>::NODE_ID`.
    // 旧的模块路径目录保留：既有声明写的是 `catalog::<模块>::NODE_ID`。
    for face in faces {
        let segments: Vec<&str> = face
            .module
            .trim_start_matches("crate::")
            .split("::")
            .collect();
        insert(&mut root, &segments, face.id.into_bytes(), &face.module)?;
    }
    // The new logical catalog is the same module, keyed by the **object relations** (logical path),
    // so a physical move or crate partition does not change the public spelling.
    // 新逻辑目录仍是同一个模块，但按**对象关系**（逻辑路径）键入，因此物理搬动或 crate 分区不改变公开拼写。
    for (path, id) in logical {
        let segments: Vec<&str> = path
            .trim_start_matches("root/")
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        insert(&mut root, &segments, *id, path)?;
    }
    let mut output = String::from(
        "\n/// Logical identities shared across crate partitions.\n/// 跨 crate 分区共用的逻辑身份。\npub mod catalog {\n",
    );
    render_branch(&mut output, &root, 1);
    output.push_str("}\n");
    Ok(output)
}

fn insert(branch: &mut Branch, segments: &[&str], id: [u8; 16], path: &str) -> Result<(), String> {
    let Some((segment, rest)) = segments.split_first() else {
        if let Some(existing) = branch.id
            && existing != id
        {
            return Err(format!(
                "two logical objects share one catalog path `{path}`; the catalog cannot name both                  (existing {existing:?}, new {id:?})"
            ));
        }
        branch.id = Some(id);
        return Ok(());
    };
    let child = branch.children.entry((*segment).to_owned()).or_default();
    insert(child, rest, id, path)
}

fn render_branch(output: &mut String, branch: &Branch, depth: usize) {
    let indent = "    ".repeat(depth);
    if let Some(id) = branch.id {
        writeln!(output, "{indent}/// Logical node identity.\n{indent}/// 逻辑节点身份。\n{indent}pub const NODE_ID: ::xirang_toolchain::NodeId = ::xirang_toolchain::NodeId::from_raw({id:?});").unwrap();
    }
    for (name, child) in &branch.children {
        let alias = name.trim_start_matches("r#").to_uppercase();
        let unique = branch
            .children
            .keys()
            .filter(|key| key.trim_start_matches("r#").to_uppercase() == alias)
            .count()
            == 1;
        if child.id.is_some()
            && alias != "NODE_ID"
            && unique
            && !branch.children.contains_key(&alias)
        {
            writeln!(output, "{indent}/// Identity of {name}.\n{indent}/// {name} 的身份。\n{indent}pub const {}: ::xirang_toolchain::NodeId = {name}::NODE_ID;", name.trim_start_matches("r#").to_uppercase()).unwrap();
        }
        writeln!(output, "{indent}/// Logical subtree {name}.\n{indent}/// 逻辑子树 {name}。\n{indent}pub mod {name} {{").unwrap();
        render_branch(output, child, depth + 1);
        writeln!(output, "{indent}}}").unwrap();
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod catalog_tests;
