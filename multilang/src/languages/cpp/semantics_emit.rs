//! The node stream the C++ walker emits for the shared walker: one node per definition, then the
//! calls each can prove or cannot.
//! C++ 遍历器发给共享遍历器的节点流：每个定义一条节点，然后是它证明得了或证明不了的调用。

use crate::semantics::SemanticFunction;

use crate::languages::kit::ast::{KindMap, walk_nodes};

use super::Decl;

/// Which of clang's AST node kinds mean what.
/// clang AST 的哪些节点种类意味着什么。
const KINDS: KindMap = KindMap {
    function: &["FunctionDecl", "CXXMethodDecl"],
    class: &[],
    call: &["Call"],
};

impl Decl {
    /// The part of the qualified spelling the walker appends after `owner::name`: template
    /// arguments, then the overload signature.
    /// 遍历器在 `owner::name` 之后追加的限定拼法：模板实参，然后是重载签名。
    pub(super) fn suffix_expression(&self) -> String {
        let mut suffix = String::new();
        if let Some(arguments) = &self.template_arguments
            && !arguments.is_empty()
        {
            suffix.push_str(&format!("<{}>", arguments.join(", ")));
        }
        if let Some(signature) = &self.signature {
            suffix.push_str(signature);
        }
        suffix
    }
}

/// The semantic functions for the definitions a file declares, in the order the map iterates.
/// 一份文件声明的定义对应的语义函数，按映射的迭代顺序。
pub(super) fn functions(decls: &std::collections::BTreeMap<String, Decl>) -> Vec<SemanticFunction> {
    let definitions: Vec<&Decl> = decls.values().filter(|decl| decl.definition).collect();
    let mut nodes = Vec::new();
    for decl in &definitions {
        nodes.push(serde_json::json!({
            "kind": "FunctionDecl",
            "name": decl.name,
            "owner": decl.owner,
            "line": decl.line,
            "end_line": decl.end_line,
            "suffix": decl.suffix_expression(),
        }));
    }
    for (index, decl) in definitions.iter().enumerate() {
        for target in &decl.calls {
            nodes.push(serde_json::json!({"kind": "Call", "parent": index, "target": target}));
        }
        for reason in &decl.unresolved {
            nodes.push(serde_json::json!({"kind": "Call", "parent": index, "unresolved": reason}));
        }
    }
    let stream = serde_json::Value::Array(nodes).to_string();
    walk_nodes(stream.as_bytes(), &KINDS, "clang")
        .expect("the semantic stream this module builds is pre-order")
}
