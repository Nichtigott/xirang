//! Python semantic facts from the interpreter's AST.
//! 来自 Python 解释器 AST 的语义事实。
//!
//! The execution surface runs `python3` with `SCRIPT`; this module only parses the rows and never
//! invents a target for a dynamic attribute.
//! 执行面用 `SCRIPT` 运行 `python3`；本模块只解析行，绝不为动态属性编造目标。

use crate::semantics::SemanticFunction;

/// The interpreter script that turns a Python file into semantic rows.
/// 把一个 Python 文件变成语义行的解释器脚本。
pub const SCRIPT: &str = r#"
import ast, json, sys

tree = ast.parse(open(sys.argv[1], encoding="utf-8").read())
patched = {}
for node in ast.walk(tree):
    if isinstance(node, ast.Assign):
        for target in node.targets:
            if isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name):
                patched[target.value.id + "::" + target.attr] = ast.unparse(node.value)
            elif isinstance(target, ast.Name):
                patched[target.id] = ast.unparse(node.value)
    elif isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "setattr" and len(node.args) >= 2:
        if isinstance(node.args[0], ast.Name) and isinstance(node.args[1], ast.Constant) and isinstance(node.args[1].value, str):
            replacement = ast.unparse(node.args[2]) if len(node.args) > 2 else "the value set at run time"
            patched[node.args[0].id + "::" + node.args[1].value] = replacement

imports = {}
for node in tree.body:
    if isinstance(node, ast.ImportFrom):
        for alias in node.names:
            imports[alias.asname or alias.name] = alias.name
    elif isinstance(node, ast.Import):
        for alias in node.names:
            imports[alias.asname or alias.name.split(".")[0]] = alias.name

class Calls(ast.NodeVisitor):
    def __init__(self, owner, imports, patched):
        self.owner = owner
        self.imports = imports
        self.patched = patched
        self.calls = []
        self.unresolved = []
    def visit_Call(self, node):
        f = node.func
        if isinstance(f, ast.Call) and isinstance(f.func, ast.Name) and f.func.id == "getattr":
            self.unresolved.append("getattr(...) (dynamic attribute)")
            self.generic_visit(node)
            return
        if isinstance(f, ast.Name) and f.id in ("getattr", "setattr", "vars", "globals", "eval", "exec"):
            self.unresolved.append(f.id + "(...) (dynamic attribute/binding)")
            self.generic_visit(node)
            return
        if isinstance(f, ast.Name):
            self._add(self.imports.get(f.id, f.id))
        elif isinstance(f, ast.Attribute):
            owner = None
            if isinstance(f.value, ast.Name):
                name = f.value.id
                if name in ("self", "cls") and self.owner:
                    owner = self.owner
                elif name in self.imports:
                    owner = self.imports[name]
                elif name[:1].isupper():
                    owner = name
            if owner:
                target = owner + "::" + f.attr
                if target in self.patched:
                    self.unresolved.append(
                        target + " (binding replaced at module level; replacement: " + self.patched[target] + ")"
                    )
                else:
                    self._add(target)
            else:
                self.unresolved.append(f.attr + " (receiver type is not written)")
        self.generic_visit(node)
    def visit_FunctionDef(self, node):
        return
    def visit_AsyncFunctionDef(self, node):
        return
    def visit_Lambda(self, node):
        return
    def _add(self, target):
        if target not in self.calls:
            self.calls.append(target)

# The node stream is pre-order: a node's parent has a smaller index. The walker on the Rust side
# owns the traversal, the class/owner stack, the per-function grouping and the qualified spelling;
# this script only says what the interpreter's AST means (its node kinds) and what each call can
# reach.
nodes = []
def add(node):
    nodes.append(node)
    return len(nodes) - 1

def function(node, owner, parent):
    visitor = Calls(owner, imports, patched)
    for child in node.body:
        visitor.visit(child)
    caveats = [
        "@" + ast.unparse(decorator) + " (decorator may replace this binding)"
        for decorator in node.decorator_list
    ]
    identifier = add({
        "kind": type(node).__name__,
        "name": node.name,
        "line": node.lineno,
        "end_line": getattr(node, "end_lineno", node.lineno),
        "parent": parent,
        "caveats": caveats,
    })
    for target in visitor.calls:
        add({"kind": "Call", "target": target, "parent": identifier})
    for reason in visitor.unresolved:
        add({"kind": "Call", "unresolved": reason, "parent": identifier})

def body(statements, owner, owner_id):
    for statement in statements:
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)):
            function(statement, owner, owner_id)
        elif isinstance(statement, ast.ClassDef):
            identifier = add({"kind": "ClassDef", "name": statement.name, "parent": owner_id})
            body(statement.body, statement.name, identifier)

body(tree.body, None, None)
print(json.dumps(nodes))
"#;

/// Parse one Python file through the interpreter's AST.
/// 通过解释器 AST 解析一份 Python 文件。
pub fn parse_json(bytes: &[u8]) -> Result<Vec<SemanticFunction>, String> {
    crate::languages::kit::ast::walk_nodes(bytes, &KINDS, "python3")
}

/// Which of the interpreter's AST node kinds mean what.
/// 解释器 AST 的哪些节点种类意味着什么。
const KINDS: crate::languages::kit::ast::KindMap = crate::languages::kit::ast::KindMap {
    function: &["FunctionDef", "AsyncFunctionDef"],
    class: &["ClassDef"],
    call: &["Call"],
};

#[cfg(test)]
#[path = "semantics_tests.rs"]
mod semantics_tests;
