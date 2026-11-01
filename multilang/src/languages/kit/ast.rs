//! The shared semantic payload: how a qualified name is assembled, and the row protocol a provider
//! emits when its walk happens outside this process.
//! 共享的语义载荷：限定名怎么拼，以及当遍历发生在本进程之外时提供者发出的行协议。
//!
//! The three walks are different technologies (a clang JSON visitor, an embedded interpreter script,
//! a `syn` visitor), so "one AST walk" is not what is shared. What *is* shared is the payload they
//! produce: a function's name and owner, the qualified spelling that keys it, its line range, and
//! the three lists a reader acts on (definite calls, unresolved calls, caveats). A provider that can
//! emit rows gets the decoder here; a provider that walks in Rust shares [`qualified`].
//! 三种遍历是不同技术（clang JSON 访问器、内嵌解释器脚本、`syn` 访问器），因此共享的不是"一个 AST 遍历"，
//! 而是它们产出的载荷：函数的名字与所属者、作为键的限定拼法、行区间，以及读者会据此行动的三张表（确定调用、
//! 未解析调用、caveat）。能发行协议的提供者用这里的解码器；在 Rust 里遍历的提供者共享 [`qualified`]。

use crate::semantics::SemanticFunction;

/// `Owner::name` for a method, the bare name for a free function.
/// 方法的 `Owner::name`，自由函数的裸名。
///
/// The query layer keys on this spelling (`call_matches` compares it with the owner), so every
/// provider must build it the same way: a language that spells the owner differently would make the
/// same method two different keys.
/// 查询层以这个拼法为键（`call_matches` 用它和所属者比较），因此每个提供者都必须同样地拼它：把所属者拼成
/// 另一种样子的语言，会让同一个方法变成两个不同的键。
pub fn qualified(owner: Option<&str>, name: &str) -> String {
    owner.map_or_else(|| name.to_owned(), |owner| format!("{owner}::{name}"))
}

/// Which node kinds of one provider's AST mean what.
/// 一个提供者的 AST 里哪些节点种类意味着什么。
///
/// The provider emits nodes with **its own** names; the map is the one place a language says
/// "FunctionDef is a function, ClassDef is a class, Call is a call". The walker then owns the
/// traversal, the owner stack, the grouping of calls per function and the qualified spelling.
/// 提供者用它**自己的**名字发节点；映射是语言说"FunctionDef 是函数、ClassDef 是类、Call 是调用"的唯一
/// 地方。遍历、所属者栈、按函数分组调用与限定拼法由遍历器拥有。
pub struct KindMap {
    /// Node kinds that declare a function or method.
    /// 声明函数或方法的节点种类。
    pub function: &'static [&'static str],
    /// Node kinds that declare a class (the owner of the methods inside it).
    /// 声明类的节点种类（其内部方法的所属者）。
    pub class: &'static [&'static str],
    /// Node kinds that name one call site.
    /// 点名一个调用点的节点种类。
    pub call: &'static [&'static str],
}

/// One node, as the provider emitted it.
/// 提供者发出的一条节点。
struct Facts {
    kind: String,
    name: String,
    owner: Option<String>,
    suffix: String,
    line: usize,
    end_line: usize,
    parent: Option<usize>,
    caveats: Vec<String>,
    target: Option<String>,
    unresolved: Option<String>,
}

/// Walk one provider's node stream into the shared facts.
/// 把一个提供者的节点流走成共享事实。
///
/// The stream is **pre-order**: a node's parent has a smaller index, so a walker needs no stack and a
/// broken stream is refused rather than guessed. A function's owner is the nearest class ancestor, its
/// calls are the call nodes whose nearest function ancestor it is, and the qualified spelling comes
/// from [`qualified`].
/// 节点流是**先序**的：父节点下标更小，因此遍历器不需要栈，而坏掉的流会被拒绝而不是猜。函数的所属者是最近的
/// 类祖先；它的调用是"最近的函数祖先是它"的那些调用节点；限定拼法来自 [`qualified`]。
pub fn walk_nodes(
    bytes: &[u8],
    kinds: &KindMap,
    emitter: &str,
) -> Result<Vec<SemanticFunction>, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("{emitter} emitted non-JSON: {error}"))?;
    let stream = value
        .as_array()
        .ok_or_else(|| format!("{emitter} emitted a non-list"))?;
    let mut facts = Vec::with_capacity(stream.len());
    for (index, node) in stream.iter().enumerate() {
        let kind = node
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{emitter} node {index} has no kind"))?;
        let parent = node
            .get("parent")
            .and_then(serde_json::Value::as_u64)
            .map(|parent| parent as usize);
        if parent.is_some_and(|parent| parent >= index) {
            return Err(format!(
                "{emitter} node {index} points at a parent that is not earlier in the stream"
            ));
        }
        facts.push(Facts {
            kind: kind.to_owned(),
            name: node
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            owner: node
                .get("owner")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            suffix: node
                .get("suffix")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            line: node
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as usize,
            end_line: node
                .get("end_line")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as usize,
            parent,
            caveats: strings(node.get("caveats")),
            target: node
                .get("target")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            unresolved: node
                .get("unresolved")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        });
    }
    let kind_is = |kind: &str, wanted: &'static [&'static str]| wanted.contains(&kind);
    let mut functions = Vec::new();
    for (index, fact) in facts.iter().enumerate() {
        if !kind_is(&fact.kind, kinds.function) {
            continue;
        }
        let owner = fact.owner.clone().or_else(|| {
            nearest(&facts, fact.parent, |kind| kind_is(kind, kinds.class))
                .map(|ancestor| facts[ancestor].name.clone())
        });
        let mut calls = Vec::new();
        let mut unresolved = Vec::new();
        for (caller, node) in facts.iter().enumerate() {
            if !kind_is(&node.kind, kinds.call) {
                continue;
            }
            let enclosing = nearest_of(&facts, node.parent, |kind| kind_is(kind, kinds.function));
            if enclosing != Some(index) {
                continue;
            }
            let _ = caller;
            if let Some(target) = &node.target
                && !calls.contains(target)
            {
                calls.push(target.clone());
            }
            if let Some(reason) = &node.unresolved
                && !unresolved.contains(reason)
            {
                unresolved.push(reason.clone());
            }
        }
        functions.push(SemanticFunction {
            name: fact.name.clone(),
            owner: owner.clone(),
            qualified: format!("{}{}", qualified(owner.as_deref(), &fact.name), fact.suffix),
            line: fact.line,
            end_line: fact.end_line.max(fact.line),
            calls,
            unresolved,
            caveats: fact.caveats.clone(),
        });
    }
    Ok(functions)
}

/// The nearest ancestor satisfying a predicate.
/// 满足谓词的最近祖先。
fn nearest(facts: &[Facts], mut at: Option<usize>, wanted: impl Fn(&str) -> bool) -> Option<usize> {
    while let Some(index) = at {
        if wanted(&facts[index].kind) {
            return Some(index);
        }
        at = facts[index].parent;
    }
    None
}

/// The nearest ancestor satisfying a predicate, from a node's parent.
/// 从某节点的父级开始，满足谓词的最近祖先。
fn nearest_of(facts: &[Facts], at: Option<usize>, wanted: impl Fn(&str) -> bool) -> Option<usize> {
    nearest(facts, at, wanted)
}

#[cfg(test)]
#[path = "walk_tests.rs"]
mod walk_tests;

/// One string list in a node; missing or non-list values are empty.
/// 节点里的一张字符串表；缺失或非列表的取值是空表。
fn strings(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
