//! C++ semantic facts from a clang AST dump.
//! 来自 clang AST dump 的 C++ 语义事实。
//!
//! This module is pure: the execution surface runs clang and passes the JSON here. It never
//! invents a target for a virtual or overloaded call; those stay in `unresolved`.
//! 本模块是纯逻辑：执行面运行 clang 并把 JSON 传进来。它绝不为虚调用或重载编造目标；这些留在
//! `unresolved`。

use std::collections::BTreeMap;

use serde_json::Value;

use crate::semantics::SemanticFunction;

#[derive(Clone, Debug)]
struct Decl {
    name: String,
    owner: Option<String>,
    line: usize,
    end_line: usize,
    start: u64,
    calls: Vec<String>,
    unresolved: Vec<String>,
    definition: bool,
    virtual_method: bool,
    /// The template arguments this declaration was instantiated or specialised with, when clang
    /// records them. `pick<int>` and `pick<Widget>` are two identities; collapsing both onto the
    /// primary template `pick` attributes a call to the wrong implementation.
    /// 这条声明被实例化或特化时使用的模板实参，clang 记录时给出。`pick<int>` 与 `pick<Widget>`
    /// 是两个身份；把两者都折叠回主模板 `pick` 会把调用的目标指向错误的实现。
    template_arguments: Option<Vec<String>>,
    /// The parameter types, in order, as clang spells them.
    /// 参数类型，按顺序，按 clang 的拼法。
    parameters: Vec<String>,
    /// The signature appended to the name **only when the owner+name has more than one definition**
    /// — i.e. when the bare name cannot identify the declaration. Two overloads of `Entry::make`
    /// become `Entry::make(int)` and `Entry::make()`, so a call resolves to the one the compiler
    /// picked instead of being refused as "overloaded".
    /// 只在"所属者+名字"有不止一个定义时才追加到名字上的签名——也就是裸名无法标识这条声明的时候。
    /// `Entry::make` 的两个重载变成 `Entry::make(int)` 与 `Entry::make()`，于是调用解析到编译器选中的
    /// 那一个，而不是被当成"重载过多"拒绝。
    signature: Option<String>,
}

fn qualified_name(decl: &Decl) -> String {
    let mut named = base_qualified_name(decl);
    if let Some(signature) = &decl.signature {
        named.push_str(signature);
    }
    named
}

/// The qualified name **without** an overload signature: the key an overload group shares.
/// **不含**重载签名的限定名：一个重载组共用的键。
fn base_qualified_name(decl: &Decl) -> String {
    let base = crate::languages::kit::ast::qualified(decl.owner.as_deref(), &decl.name);
    match &decl.template_arguments {
        Some(arguments) if !arguments.is_empty() => {
            format!("{base}<{}>", arguments.join(", "))
        }
        _ => base,
    }
}

fn call_counts(
    decls: &BTreeMap<String, Decl>,
) -> (BTreeMap<String, usize>, BTreeMap<String, bool>) {
    let mut counts = BTreeMap::new();
    let mut virtuals = BTreeMap::new();
    for decl in decls.values() {
        let name = qualified_name(decl);
        if decl.definition {
            *counts.entry(name.clone()).or_insert(0) += 1;
        }
        if decl.virtual_method {
            virtuals.insert(name, true);
        }
    }
    (counts, virtuals)
}

/// Read the functions and call references from one clang AST JSON document.
/// 从一份 clang AST JSON 文档读取函数与调用引用。
pub fn parse_ast(ast: &Value) -> Vec<SemanticFunction> {
    parse_ast_with_source(ast, None)
}

/// Read functions with an optional source text, used to recover line numbers for template patterns
/// whose AST `loc` omits the line but retains the byte offset.
/// 带可选源码文本读取函数；模板 pattern 的 AST `loc` 省略行号但保留字节偏移时，用它恢复行号。
pub fn parse_ast_with_source(ast: &Value, source: Option<&str>) -> Vec<SemanticFunction> {
    let mut classes: BTreeMap<String, String> = BTreeMap::new();
    class_names(ast, &mut classes);
    let mut decls: BTreeMap<String, Decl> = BTreeMap::new();
    declarations(ast, &classes, &mut decls, None, source);
    deduplicate_template_patterns(&mut decls);
    name_overloads(&mut decls);
    let (counts, virtuals) = call_counts(&decls);
    let mut collected: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for node in nodes(ast) {
        let kind = node.get("kind").and_then(Value::as_str);
        if !matches!(kind, Some("FunctionDecl" | "CXXMethodDecl")) {
            continue;
        }
        let Some(id) = node.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !decls.get(id).is_some_and(|decl| decl.definition) {
            continue;
        }
        // Only the occurrence that carries a body describes what this declaration calls. A
        // decl-only copy that clang lists inside a `FunctionTemplateDecl` has no body, so letting it
        // write an empty call list would erase the real definition's calls whenever the copy is
        // visited last.
        // 只有带函数体的那一次描述这条声明调用了什么。clang 在 `FunctionTemplateDecl` 里列出的纯声明副本
        // 没有函数体；若让它写入空的调用表，一旦副本被最后访问，真正定义的调用就会被抹掉。
        let has_body = node
            .get("inner")
            .and_then(Value::as_array)
            .is_some_and(|inner| {
                inner
                    .iter()
                    .any(|child| child.get("kind").and_then(Value::as_str) == Some("CompoundStmt"))
            });
        if !has_body {
            continue;
        }
        let mut collector = CallCollector {
            defs: &decls,
            counts: &counts,
            virtuals: &virtuals,
            calls: Vec::new(),
            unresolved: Vec::new(),
        };
        if let Some(inner) = node.get("inner").and_then(Value::as_array) {
            for child in inner {
                collector.calls(child);
            }
        }
        collected.insert(id.to_owned(), (collector.calls, collector.unresolved));
    }
    for (id, (calls, unresolved)) in collected {
        if let Some(decl) = decls.get_mut(&id) {
            decl.calls = calls;
            decl.unresolved = unresolved;
        }
    }
    emit::functions(&decls)
}

fn nodes(node: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    gather_nodes(node, &mut out);
    out
}

fn gather_nodes<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if !node.is_object() {
        return;
    }
    out.push(node);
    if let Some(inner) = node.get("inner").and_then(Value::as_array) {
        for child in inner {
            gather_nodes(child, out);
        }
    }
}

/// Record every class's **qualified** name, namespaces included.
/// 记录每个类的**限定**名，含命名空间。
///
/// `namespace shop { struct Entry … }` and `namespace admin { struct Entry … }` are two identities.
/// The old map keyed a class on its bare name, so a call written `shop::Entry::make()` could not be
/// distinguished from the other namespace's — the answer had to call it ambiguous.
/// `namespace shop { struct Entry … }` 与 `namespace admin { struct Entry … }` 是两个身份。旧映射按裸名
/// 记类，因此写成 `shop::Entry::make()` 的调用无法与另一个命名空间区分——答案只能称它歧义。
fn class_names(node: &Value, classes: &mut BTreeMap<String, String>) {
    class_names_in(node, None, classes);
}

fn class_names_in(node: &Value, context: Option<&str>, classes: &mut BTreeMap<String, String>) {
    let kind = node.get("kind").and_then(Value::as_str);
    let mut inner_context = context.map(str::to_owned);
    if let Some(name) = node.get("name").and_then(Value::as_str) {
        match kind {
            Some("NamespaceDecl") => {
                inner_context = Some(match context {
                    Some(parent) => format!("{parent}::{name}"),
                    None => name.to_owned(),
                });
            }
            Some(
                "CXXRecordDecl"
                | "ClassTemplateSpecializationDecl"
                | "ClassTemplatePartialSpecializationDecl",
            ) => {
                // clang nests a record inside its own template declaration and repeats the injected
                // class name inside a specialization. Those nodes are the *same* class, not a class
                // named after itself: without this check the pattern's methods came out as
                // `Box::Box::put` and a specialization's as `Box<int>::Box::put`.
                // clang 会把一个记录嵌在自己的模板声明里，并在特化内部重复注入类名。那些节点是**同一个**
                // 类，而不是名字叫自己的类：缺这道检查，pattern 的方法会变成 `Box::Box::put`，
                // 特化的会变成 `Box<int>::Box::put`。
                let same_as_context = context
                    .and_then(|parent| parent.rsplit("::").next())
                    .is_some_and(|last| last.split('<').next() == Some(name));
                let qualified = if same_as_context {
                    context
                        .map(str::to_owned)
                        .unwrap_or_else(|| name.to_owned())
                } else {
                    let named = match template_arguments(node) {
                        Some(arguments) if !arguments.is_empty() => {
                            format!("{name}<{}>", arguments.join(", "))
                        }
                        _ => name.to_owned(),
                    };
                    match context {
                        Some(parent) => format!("{parent}::{named}"),
                        None => named,
                    }
                };
                if let Some(id) = node.get("id").and_then(Value::as_str) {
                    classes.entry(id.to_owned()).or_insert(qualified.clone());
                }
                inner_context = Some(qualified);
            }
            _ => {}
        }
    }
    if let Some(inner) = node.get("inner").and_then(Value::as_array) {
        for child in inner {
            class_names_in(child, inner_context.as_deref(), classes);
        }
    }
}

/// The template arguments clang recorded on one declaration, when it recorded something nameable.
/// clang 在一条声明上记录的模板实参；只在它记下了可命名的实参时返回。
///
/// `None` means "no template arguments here" (a primary template or a plain function) **or** "an
/// argument this reader cannot name". Both keep the declaration's base name: an identity the reader
/// cannot print must not be invented, or two different instantiations would silently collapse.
/// `None` 表示"这里没有模板实参"（主模板或普通函数）**或**"有一个本读取器命名不了的实参"。两者都保留
/// 声明的基础名：读取器打印不出的身份不能编造，否则两个不同实例化会被静默折叠。
fn template_arguments(node: &Value) -> Option<Vec<String>> {
    let inner = node.get("inner").and_then(Value::as_array)?;
    let mut arguments = Vec::new();
    for child in inner {
        if child.get("kind").and_then(Value::as_str) != Some("TemplateArgument") {
            continue;
        }
        if let Some(text) = child
            .get("type")
            .and_then(|typed| typed.get("qualType"))
            .and_then(Value::as_str)
        {
            arguments.push(text.to_owned());
        } else if let Some(value) = child.get("value") {
            // A non-type template argument arrives as a JSON scalar, not as a qualType:
            // `fixed<3>` is `{"kind":"TemplateArgument","value":3}`.
            // 非类型模板实参以 JSON 标量到达，而不是 qualType：`fixed<3>` 是
            // `{"kind":"TemplateArgument","value":3}`。
            match value {
                Value::String(text) => arguments.push(text.clone()),
                Value::Number(number) => arguments.push(number.to_string()),
                Value::Bool(flag) => arguments.push(flag.to_string()),
                _ => return None,
            }
        } else {
            // A pack, a template template argument, or an expression clang did not render: the
            // declaration keeps its base name rather than getting a made-up one.
            // 参数包、模板模板实参或 clang 未渲染的表达式：保留基础名，不编造一个。
            return None;
        }
    }
    if arguments.is_empty() {
        None
    } else {
        Some(arguments)
    }
}

/// The parameter types of one declaration, in order, as clang spells them.
/// 一条声明的参数类型，按顺序，按 clang 的拼法。
fn parameter_types(node: &Value) -> Vec<String> {
    node.get("inner")
        .and_then(Value::as_array)
        .map(|inner| {
            inner
                .iter()
                .filter(|child| child.get("kind").and_then(Value::as_str) == Some("ParmVarDecl"))
                .filter_map(|child| {
                    child
                        .get("type")
                        .and_then(|typed| typed.get("qualType"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Give an overload group a signature suffix, and only an overload group.
/// 只给重载组加签名后缀。
///
/// A bare name is the better spelling when it is unambiguous, and it is what every other reader of
/// this tree already prints. The suffix exists for the one case the bare name cannot answer: two
/// definitions under one owner and name, where a call has to say which one the compiler chose.
/// 裸名在不歧义时是更好的拼法，也是这棵树其他读者已经在打印的形状。后缀只为裸名回答不了的那一种情形
/// 存在：同一个所属者、同一个名字下有两个定义，而调用必须说出编译器选了哪一个。
fn name_overloads(decls: &mut BTreeMap<String, Decl>) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, decl) in decls.iter() {
        if decl.definition {
            groups
                .entry(base_qualified_name(decl))
                .or_default()
                .push(id.clone());
        }
    }
    for ids in groups.into_values() {
        if ids.len() < 2 {
            continue;
        }
        for id in ids {
            if let Some(decl) = decls.get_mut(&id) {
                let arguments = decl.parameters.join(", ");
                decl.signature = Some(format!("({arguments})"));
            }
        }
    }
}

fn declarations(
    node: &Value,
    classes: &BTreeMap<String, String>,
    decls: &mut BTreeMap<String, Decl>,
    current_class: Option<&str>,
    source: Option<&str>,
) {
    let kind = node.get("kind").and_then(Value::as_str);
    if matches!(kind, Some("FunctionDecl" | "CXXMethodDecl"))
        && let (Some(id), Some(name)) = (
            node.get("id").and_then(Value::as_str),
            node.get("name").and_then(Value::as_str),
        )
    {
        let start = start_offset(node).unwrap_or_default();
        let line = line_of(node)
            .or_else(|| source.and_then(|source| line_at_offset(source, start)))
            .unwrap_or_default();
        let end_line = end_line_of(node)
            .or_else(|| {
                source.and_then(|source| line_at_offset(source, end_offset(node).unwrap_or(start)))
            })
            .unwrap_or(line);
        let owner = node
            .get("parentDeclContextId")
            .and_then(Value::as_str)
            .and_then(|parent| classes.get(parent))
            .cloned()
            .or_else(|| current_class.map(str::to_owned));
        let definition = node
            .get("inner")
            .and_then(Value::as_array)
            .is_some_and(|inner| {
                inner
                    .iter()
                    .any(|child| child.get("kind").and_then(Value::as_str) == Some("CompoundStmt"))
            });
        let virtual_method = node
            .get("virtual")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let arguments = template_arguments(node);
        let parameters = parameter_types(node);
        match decls.entry(id.to_owned()) {
            std::collections::btree_map::Entry::Occupied(mut existing) => {
                // clang can list the same declaration twice: a decl-only copy inside a
                // `FunctionTemplateDecl` (no range, no body) and the real definition later in the
                // dump. Keeping the first copy marked it "not a definition", which silently dropped
                // an explicit specialization — and a call to it then resolved to the primary
                // template. The occurrence with a body is the one that describes the declaration.
                // clang 可能把同一条声明列两次：`FunctionTemplateDecl` 里一份只有声明（无 range、无函数体）
                // 的副本，以及转储后面真正的定义。保留第一份会把它标成"不是定义"，于是显式特化被静默丢掉
                // ——而调用它的地方就解析到了主模板。带函数体的那一次才是描述这条声明的那个。
                //
                // The merge below is not only about the body: whichever copy comes **first** may be
                // the context-free one. An instantiated member listed at namespace scope arrives
                // before the nested copy that carries its class, so keeping the first copy alone lost
                // the owner and the definition came out as a bare `put`.
                // 下面的合并不只管函数体：**先**出现的那一份可能是没有上下文的那份。在命名空间作用域列出的
                // 实例化成员会早于带它所在类的嵌套副本出现，因此只留第一份就丢了 owner，定义变成裸 `put`。
                let kept = existing.get_mut();
                let upgraded = definition && !kept.definition;
                if upgraded {
                    // The body-carrying occurrence is the one whose line and range describe the
                    // declaration. The decl-only copy has no range at all, so its line was
                    // backfilled from offset zero and is not evidence of anything.
                    // 带函数体的那一次才是行号与范围描述这条声明的那一次。纯声明副本根本没有 range，
                    // 它的行号是从偏移零回填来的，不构成任何证据。
                    kept.definition = true;
                    kept.line = line;
                    kept.end_line = end_line;
                    kept.start = start;
                } else {
                    if line > 0 && kept.line == 0 {
                        kept.line = line;
                    }
                    if start > 0 && kept.start == 0 {
                        kept.start = start;
                    }
                }
                if kept.owner.is_none() {
                    kept.owner = owner;
                }
                if kept.template_arguments.is_none() {
                    kept.template_arguments = arguments;
                }
                if kept.parameters.is_empty() {
                    kept.parameters = parameters;
                }
            }
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(Decl {
                    name: name.to_owned(),
                    owner,
                    line,
                    end_line,
                    start,
                    calls: Vec::new(),
                    unresolved: Vec::new(),
                    definition,
                    virtual_method,
                    template_arguments: arguments,
                    parameters,
                    signature: None,
                });
            }
        }
    }
    if let Some(inner) = node.get("inner").and_then(Value::as_array) {
        // A specialization nests its members directly under the
        // `ClassTemplateSpecializationDecl` (no injected `CXXRecordDecl` in between), so it has to
        // establish the context exactly as a record does — otherwise an instantiated member comes
        // out with no owner at all.
        // 特化把成员直接嵌在 `ClassTemplateSpecializationDecl` 下（中间没有注入的 `CXXRecordDecl`），
        // 因此它必须像记录节点一样建立上下文——否则实例化成员会完全没有 owner。
        let inner_class = match node.get("kind").and_then(Value::as_str) {
            Some(
                "CXXRecordDecl"
                | "ClassTemplateSpecializationDecl"
                | "ClassTemplatePartialSpecializationDecl",
            ) => node
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| classes.get(id))
                .map(String::as_str)
                .or_else(|| node.get("name").and_then(Value::as_str)),
            _ => current_class,
        };
        for child in inner {
            declarations(child, classes, decls, inner_class, source);
        }
    }
}

fn start_offset(node: &Value) -> Option<u64> {
    node.get("range")
        .and_then(|range| range.get("begin"))
        .and_then(|begin| begin.get("offset"))
        .and_then(Value::as_u64)
}

fn end_offset(node: &Value) -> Option<u64> {
    node.get("range")
        .and_then(|range| range.get("end"))
        .and_then(|end| end.get("offset"))
        .and_then(Value::as_u64)
}

fn line_at_offset(source: &str, offset: u64) -> Option<usize> {
    let offset = usize::try_from(offset).ok()?;
    if offset > source.len() {
        return None;
    }
    Some(
        source[..offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1,
    )
}

/// Collapse clang's duplicate listings of one instantiation, without collapsing distinct ones.
/// 折叠 clang 对同一个实例化的重复列举，但不折叠不同的实例化。
///
/// clang lists the same instantiated declaration more than once (an implicit instantiation and, say,
/// an explicit instantiation declaration share a source start). Two listings of the *same identity*
/// at the same start are one definition; the other stays in the id map so a call reference resolves,
/// but it is not listed twice. A primary template (`pick`) and an instantiation (`pick<int>`) share
/// the start but not the identity, so both stay: that difference is the whole point of naming
/// template arguments.
/// clang 会把同一条实例化声明列不止一次（隐式实例化与显式实例化声明共享同一源码起点）。同一**身份**、
/// 同一起点两次列举算一个定义；另一份仍留在 id 映射里供调用引用解析，但不重复列出。主模板（`pick`）与
/// 实例化（`pick<int>`）共享起点却不同身份，因此两者都保留：这个差异正是命名模板实参的意义。
fn deduplicate_template_patterns(decls: &mut BTreeMap<String, Decl>) {
    let mut seen: BTreeMap<(String, u64, bool), String> = BTreeMap::new();
    for (id, decl) in decls.iter() {
        if !decl.definition {
            continue;
        }
        let key = (qualified_name(decl), decl.start, true);
        seen.entry(key).or_insert_with(|| id.clone());
    }
    let duplicates = decls
        .iter()
        .filter(|(_, decl)| decl.definition)
        .filter_map(|(id, decl)| {
            let key = (qualified_name(decl), decl.start, true);
            seen.get(&key)
                .filter(|kept| *kept != id)
                .map(|_| id.clone())
        })
        .collect::<Vec<_>>();
    for id in duplicates {
        if let Some(decl) = decls.get_mut(&id) {
            decl.definition = false;
        }
    }
}

fn first_decl_ref(node: &Value) -> Option<&Value> {
    if node.get("kind").and_then(Value::as_str) == Some("DeclRefExpr") {
        return Some(node);
    }
    for child in node.get("inner").and_then(Value::as_array)? {
        if let Some(found) = first_decl_ref(child) {
            return Some(found);
        }
    }
    None
}

fn line_of(node: &Value) -> Option<usize> {
    node.get("loc")
        .and_then(|loc| loc.get("line"))
        .or_else(|| {
            node.get("range")
                .and_then(|range| range.get("begin"))
                .and_then(|begin| begin.get("line"))
        })
        .and_then(Value::as_u64)
        .map(|line| line as usize)
}

fn end_line_of(node: &Value) -> Option<usize> {
    node.get("range")
        .and_then(|range| range.get("end"))
        .and_then(|end| end.get("line"))
        .and_then(Value::as_u64)
        .map(|line| line as usize)
}

struct CallCollector<'a> {
    defs: &'a BTreeMap<String, Decl>,
    counts: &'a BTreeMap<String, usize>,
    virtuals: &'a BTreeMap<String, bool>,
    calls: Vec<String>,
    unresolved: Vec<String>,
}

impl<'a> CallCollector<'a> {
    fn calls(&mut self, node: &Value) {
        let kind = node.get("kind").and_then(Value::as_str);

        match kind {
            Some("CallExpr") => {
                if let Some(target) = self.call_target(node)
                    && !self.calls.contains(&target)
                {
                    self.calls.push(target);
                }
            }
            Some("CXXMemberCallExpr") => {
                if let Some(target) = self.member_target(node) {
                    if !self.calls.contains(&target) {
                        self.calls.push(target);
                    }
                } else if let Some(name) = node
                    .get("inner")
                    .and_then(Value::as_array)
                    .and_then(|inner| inner.first())
                    .and_then(|member| member.get("name"))
                    .and_then(Value::as_str)
                {
                    self.unresolved
                        .push(format!("{name} (receiver type is not written)"));
                }
            }
            Some("FunctionDecl" | "CXXMethodDecl") if node.get("id").is_some() => return,
            _ => {}
        }
        if let Some(inner) = node.get("inner").and_then(Value::as_array) {
            for child in inner {
                self.calls(child);
            }
        }
    }

    fn call_target(&mut self, node: &Value) -> Option<String> {
        let reference = first_decl_ref(node)?.get("referencedDecl")?;
        let id = reference.get("id").and_then(Value::as_str)?;
        let decl = self.defs.get(id)?;
        let qualified = qualified_name(decl);
        self.claim(qualified)
    }

    fn claim(&mut self, qualified: String) -> Option<String> {
        let qualified = self.canonical(&qualified)?;
        let count = self.counts.get(&qualified).copied().unwrap_or_default();
        if count > 1 {
            self.unresolved
                .push(format!("{qualified} (overloaded: {count} definitions)"));
            return None;
        }
        if self.virtuals.get(&qualified).copied().unwrap_or(false) {
            let simple = qualified.rsplit("::").next().unwrap_or(&qualified);
            let candidates = self
                .defs
                .values()
                .filter(|decl| decl.definition && decl.name == simple)
                .map(qualified_name)
                .collect::<std::collections::BTreeSet<_>>();
            self.unresolved.push(format!(
                "{qualified} (virtual dispatch is not resolved; candidates: {})",
                candidates.into_iter().collect::<Vec<_>>().join(", ")
            ));
            return None;
        }
        Some(qualified)
    }

    /// Turn the name a call site spells into the identity this translation unit defines.
    /// 把调用点写出的名字换成这个翻译单元定义的**身份**。
    ///
    /// A call inside `namespace shop` may be spelled `Entry::make` while the definition is
    /// `shop::Entry::make`; the namespace is not part of the spelling but it *is* part of the
    /// identity. A unique suffix match therefore resolves; several suffix matches are reported as
    /// ambiguous instead of one being picked, because picking one is exactly how a call gets
    /// attributed to the wrong namespace.
    /// `namespace shop` 内的调用可能写成 `Entry::make`，而定义是 `shop::Entry::make`：命名空间不在拼写里，
    /// 却在身份里。因此唯一后缀匹配可以解析；多个后缀匹配报成歧义而不是挑一个——挑一个正是把调用归错
    /// 命名空间的方式。
    fn canonical(&mut self, qualified: &str) -> Option<String> {
        if self.counts.contains_key(qualified) {
            return Some(qualified.to_owned());
        }
        let suffix = format!("::{qualified}");
        let matches = self
            .counts
            .iter()
            .filter(|(name, count)| **count > 0 && name.ends_with(&suffix))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        match matches.len() {
            0 => {
                self.unresolved.push(format!(
                    "{qualified} (no definition in this translation unit)"
                ));
                None
            }
            1 => matches.into_iter().next(),
            _ => {
                self.unresolved.push(format!(
                    "{qualified} (ambiguous: {} definitions end with this name: {})",
                    matches.len(),
                    matches.join(", ")
                ));
                None
            }
        }
    }

    fn member_target(&mut self, node: &Value) -> Option<String> {
        let member = node
            .get("inner")
            .and_then(Value::as_array)?
            .iter()
            .find(|child| child.get("kind").and_then(Value::as_str) == Some("MemberExpr"))?;
        // Prefer the declaration the call **names**: for a member template, clang resolved the
        // instantiation there, and its template arguments are the only place they are written. The
        // receiver-type spelling below cannot say `put<Widget>` — it only knows the object's type and
        // the method's name — so without this a correct call to an instantiated member template was
        // refused as unresolved.
        // 优先用调用**点名**的那条声明：对成员模板，clang 在那里解析出了实例化，而模板实参只写在那里。
        // 下面的接收者类型拼法说不出 `put<Widget>`——它只知道对象的类型与方法名——因此缺了这一步，一次
        // 正确的"调用实例化的成员模板"会被当成未解析拒绝。
        // clang spells this reference `referencedMemberDecl` (a bare id) on a `MemberExpr`, while
        // other nodes carry `referencedDecl` as an object; both are read here.
        // clang 在 `MemberExpr` 上把这条引用拼成 `referencedMemberDecl`（裸 id），而其他节点带的是对象
        // 形状的 `referencedDecl`；两者这里都读。
        let referenced = member
            .get("referencedMemberDecl")
            .and_then(Value::as_str)
            .or_else(|| {
                member
                    .get("referencedDecl")
                    .and_then(|declared| declared.get("id"))
                    .and_then(Value::as_str)
            });
        if let Some(reference) = referenced
            && let Some(decl) = self.defs.get(reference)
        {
            return self.claim(qualified_name(decl));
        }
        let name = member.get("name").and_then(Value::as_str)?;
        let receiver = member
            .get("inner")
            .and_then(Value::as_array)?
            .iter()
            .find(|child| child.get("kind").and_then(Value::as_str) == Some("DeclRefExpr"))?;
        let owner = receiver
            .get("referencedDecl")?
            .get("type")?
            .get("qualType")?
            .as_str()?
            .trim()
            .trim_start_matches("const ")
            .trim_end_matches(|character: char| {
                character == '&' || character == '*' || character == ' '
            })
            .trim()
            .to_owned();
        self.claim(format!("{owner}::{name}"))
    }
}

#[path = "semantics_emit.rs"]
mod emit;
