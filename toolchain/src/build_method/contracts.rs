//! Declaration-level registration contract checks.
//! 注册声明层合同检查。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::Node;
use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::{SourceScope, cached_parent_id, node_id, parsed_face, relative_display};

type ParentRules = BTreeMap<super::registry_identity::NodeId, String>;

pub(crate) fn aggregate_contract_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    let mut parent_rules = ParentRules::new();
    // One memo for this pass over the tree, same as the discovery pass: `check_parent_rule` resolves a
    // parent per face, and with no memo each of a thousand faces reads and parses the same folder face.
    // 本趟共用一份备忘，与发现那一趟相同：`check_parent_rule` 每个面解析一次父级，没有备忘时一千个面会各读
    // 各解析同一个文件夹面。
    let _parents = crate::build_method::discovery_cache::ParentLookupScope::open();
    visit_parent_rules(src, nodes, &mut parent_rules);
    visit_contract_errors(
        src,
        nodes,
        include_demo,
        scope,
        false,
        &parent_rules,
        &mut errors,
    );
    errors
}

fn visit_parent_rules(src: &Path, nodes: &[Node], rules: &mut ParentRules) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            // The three values this needs are fields, so the pass reads them from the facts it already
            // has — including, as everywhere in this switch, the reader's own `None` (`stem 2`).
            // 它需要的三个值都是字段，因此本趟从已有的事实里读——包括这次切换中处处遵循的那条：读取器自己的
            // `None` 也算答案（`stem 2`）。
            if super::discovery_cache::face_bool_field(file, &relative, "needs_registry")
                == Some(true)
                && let Some(id) = node_id(src, node)
                && let Some(rule) = declared_rule_source(
                    src,
                    &relative,
                    super::discovery_cache::face_string_field(
                        file,
                        &relative,
                        "registry_rule_path",
                    )
                    .map(|(value, _)| value),
                    super::discovery_cache::face_raw_field(file, &relative, "registry_rule")
                        .map(|(value, _)| value),
                )
            {
                rules.insert(id, rule);
            }
        }
        visit_parent_rules(src, &node.children, rules);
    }
}

fn declared_rule_source(
    src: &Path,
    relative: &str,
    registry_rule_path: Option<String>,
    registry_rule: Option<String>,
) -> Option<String> {
    let explicit = registry_rule_path.map(|path| {
        let relative = path.strip_prefix("src/").unwrap_or(&path);
        src.join(relative)
    });
    let canonical = Path::new(relative)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("registry_rule/registry_rule.rs");
    explicit
        .and_then(|path| fs::read_to_string(path).ok())
        .or_else(|| fs::read_to_string(src.join(canonical)).ok())
        .or(registry_rule)
}

fn visit_contract_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    selected_ancestor: bool,
    parent_rules: &ParentRules,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if node.name == crate::build_method::DEMO_ONLY_DIRECTORY && !include_demo
            || !scope.includes(src, node, selected_ancestor)
        {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
            {
                check_parent_rule(&face, &relative, src, node, parent_rules, errors);
            }
        }
        visit_contract_errors(
            src,
            &node.children,
            include_demo,
            scope,
            selected_here,
            parent_rules,
            errors,
        );
    }
}

fn check_parent_rule(
    face: &FaceSyntax,
    relative: &str,
    src: &Path,
    node: &Node,
    parent_rules: &ParentRules,
    errors: &mut BuildDiagnostics,
) {
    if matches!(face.parent(), Some(ParentSyntax::Root) | None) {
        return;
    }
    let Some(parent_rule) = cached_parent_id(src, face).and_then(|id| parent_rules.get(&id)) else {
        return;
    };
    let id = node_id(src, node).map_or_else(|| "<unknown>".to_owned(), |id| id.to_string());
    let kind = face.path("kind").unwrap_or_else(|| "<unknown>".to_owned());
    let handle = face
        .path("handle")
        .or_else(|| face.path("kind"))
        .unwrap_or_else(|| "<unknown>".to_owned());
    if let Some(required) = rule_method_strings(parent_rule, "require_preset").first()
        && face.path("preset").as_deref() != Some(required.as_str())
    {
        let line = face
            .field_location("preset")
            .map_or(face.location.line, |location| location.line);
        errors.push(
            BuildDiagnostic::new("contract", "required preset is missing")
                .node(id.clone(), kind.clone())
                .at(relative, line)
                .function(handle.clone())
                .field("preset")
                .expected(required),
        );
    }
    for required in rule_method_strings(parent_rule, "require_handle_traits") {
        if !face
            .string_list("handle_traits")
            .unwrap_or_default()
            .iter()
            .any(|value| value == &required)
        {
            let line = face
                .field_location("handle_traits")
                .map_or(face.location.line, |location| location.line);
            errors.push(
                BuildDiagnostic::new("contract", "required handle trait is missing")
                    .node(id.clone(), kind.clone())
                    .at(relative, line)
                    .function(handle.clone())
                    .field("handle_traits")
                    .expected(required),
            );
        }
    }
    for (method, field, label) in [
        ("require_exports", "exports", "export"),
        ("require_part_traits", "part_traits", "part trait"),
    ] {
        for required in rule_method_strings(parent_rule, method) {
            if !face
                .string_list(field)
                .unwrap_or_default()
                .iter()
                .any(|value| value == &required)
            {
                let line = face
                    .field_location(field)
                    .map_or(face.location.line, |location| location.line);
                errors.push(
                    BuildDiagnostic::new("contract", format!("required {label} is missing"))
                        .node(id.clone(), kind.clone())
                        .at(relative, line)
                        .function(handle.clone())
                        .field(field)
                        .expected(required),
                );
            }
        }
    }
}

/// The string arguments of every `.{method}(…)` call in a registry rule's source.
/// 注册规则源码里每一处 `.{方法名}(…)` 调用的字符串参数。
///
/// Hand-written rather than parsed, but it still has to be a *lexer*, because the text it
/// reads is Rust source and not every character in it is code:
/// 这里手写而不是解析，但它仍必须是一个**词法器**：它读的是 Rust 源码，而其中并非每个字符都是代码：
///
/// - a `.require_exports("x")` written in a comment is an *example*, not a requirement.
///   Reading it as one makes the build report a violation the author never wrote — the
///   fake-implementation failure this scan was audited for (`SUR-S11`);
/// - every occurrence has to be read, because a rule may require the same trait twice;
/// - the argument list has to be closed by *nesting*, because an argument may itself call
/// - a function whose `)` would otherwise end the list early;
/// - a `.require_exports("x")` shape in **non-comment prose** — a documentation paragraph,
///   pseudo-code that no comment wraps — is textually indistinguishable from a real call:
///   this reader only knows "not inside a comment, not inside a string literal". That is the
///   inherent boundary of a text scan, so never write a `.method(…)` shape in prose; when you
///   need an example, put it in a comment. The real gate is the compile-time
///   `assert_static_registration`, and this scan only makes the build diagnostic sharper.
/// - 写在注释里的 `.require_exports("x")` 是**示例**而不是需求。把它读成需求会让构建报出作者从未
///   写过的违约——那正是这道扫描被审计为"假实现"的失败（`SUR-S11`）；
/// - 每一处出现都必须读到，因为一条规则可以两次要求同一个 trait；
/// - 参数区间必须按**嵌套**配平，因为实参本身可能调用函数，那个函数的 `)` 会提前终止列表。
/// - 写在**非注释散文**里的 `.require_exports("x")` 形态——文档段落、没有被注释包住的伪代码——
///   与真实调用在**文本上无从区分**：本读取器只知道"这一处不在注释里、不在字符串字面量里"。
///   这是文本扫描的固有边界，因此不要在散文里写 `.方法名(…)` 形态；需要举例就写进注释。
///   真正的门是编译期 `assert_static_registration`，这道读取器只是让构建期诊断更准。
///
/// Missing a requirement only makes the diagnostic worse, since the compile-time asserts
/// remain the real gate (`assert_static_registration`); a requirement nobody wrote must
/// not happen at all. That asymmetry is why this scan errs towards reading nothing from
/// source it cannot lex.
/// 漏读一条需求只会让报错变差，因为真正的门仍是编译期断言（`assert_static_registration`）；而
/// 凭空读出一条没人写过的需求是绝不能发生的。这个不对称正是这道扫描对读不懂的源码选择"一无所获"、
/// 而不是猜测的原因。
fn rule_method_strings(source: &str, method: &str) -> Vec<String> {
    let marker = format!(".{method}(");
    let mut found = Vec::new();
    for start in code_occurrences(source, &marker) {
        let arguments = start + marker.len();
        let end = call_end(&source[arguments..]).map_or(source.len(), |offset| arguments + offset);
        found.extend(string_values(&source[arguments..end]));
    }
    found
}

/// The offsets of every occurrence of `needle` in `haystack` that is *code*.
/// `haystack` 里 `needle` 每一处**代码**出现的偏移。
///
/// An occurrence inside a comment, a string or a character literal is text *about* the
/// call, not the call.
/// 落在注释、字符串或字符字面量里的出现，是关于那次调用的文本，而不是那次调用。
fn code_occurrences(haystack: &str, needle: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut index = 0;
    while index < haystack.len() {
        let rest = &haystack[index..];
        if let Some(skip) = skipped(rest) {
            index += skip;
            continue;
        }
        if rest.starts_with(needle) {
            offsets.push(index);
            index += needle.len();
            continue;
        }
        index += rest.chars().next().map_or(1, char::len_utf8);
    }
    offsets
}

/// The offset of the `)` that closes the argument list `arguments` starts with.
/// 关闭 `arguments` 开头那个参数列表的 `)` 的偏移。
///
/// `None` when the parentheses never balance, which leaves the caller reading to the end
/// of the source instead of to a guessed boundary.
/// 括号始终配不平时返回 `None`：调用方会读到底，而不是读到一个猜出来的边界。
fn call_end(arguments: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = 0;
    while index < arguments.len() {
        let rest = &arguments[index..];
        if let Some(skip) = skipped(rest) {
            index += skip;
            continue;
        }
        let character = rest.chars().next()?;
        match character {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
        index += character.len_utf8();
    }
    None
}

/// Every string literal in `text`, in order, with its escapes decoded.
/// `text` 里的每个字符串字面量，按顺序，转义已解码。
fn string_values(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        if let Some((length, content)) = literal(rest) {
            if let Some(value) = content {
                found.push(value);
            }
            index += length;
            continue;
        }
        if let Some(skip) = skipped(rest) {
            index += skip;
            continue;
        }
        index += rest.chars().next().map_or(1, char::len_utf8);
    }
    found
}

/// How far to skip when `text` opens a comment or a literal; `None` when it opens code.
/// `text` 开头是注释或字面量时要跳过多远；开头是代码时返回 `None`。
fn skipped(text: &str) -> Option<usize> {
    if let Some(tail) = text.strip_prefix("//") {
        return Some(2 + tail.find('\n').unwrap_or(tail.len()));
    }
    if let Some(tail) = text.strip_prefix("/*") {
        // An unterminated block comment runs to the end: what follows is not code, and a
        // rule source that does not parse is refused by the compiler long before this.
        // 未闭合的块注释一直到结尾：其后不是代码，而解析不过的规则源码早在编译器那里就被拒了。
        return Some(2 + tail.find("*/").map_or(tail.len(), |offset| offset + 2));
    }
    literal(text).map(|(length, _)| length)
}

/// The literal `text` opens: its length, and its contents when it is a *string*.
/// `text` 开头的字面量：它的长度，以及它是**字符串**时它的内容。
///
/// A character or byte-character literal has no contents here — `'x'` carries no
/// requirement, and returning its character would put a phantom capability in the list.
/// 字符或字节字符字面量在这里没有内容——`'x'` 不带需求，把它的字符返回去等于往清单里塞一条幽灵能力。
fn literal(text: &str) -> Option<(usize, Option<String>)> {
    let body = text.strip_prefix('b').unwrap_or(text);
    let prefix = text.len() - body.len();
    if let Some(raw) = body.strip_prefix('r') {
        let hashes = raw
            .chars()
            .take_while(|character| *character == '#')
            .count();
        let inner = raw[hashes..].strip_prefix('"')?;
        let terminator = format!("\"{}", "#".repeat(hashes));
        let end = inner.find(&terminator)?;
        let length = prefix + 1 + hashes + 1 + end + terminator.len();
        return Some((length, Some(inner[..end].to_owned())));
    }
    let quote = body.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let quotes = escaped_len(body, quote)?;
    let length = quotes + prefix;
    if quote == '\'' {
        return Some((length, None));
    }
    // The contents live between the quotes, which is why the slice ends at `quotes - 1`
    // and not at the end of the remaining text.
    // 内容在两个引号之间，因此切片的终点是 `quotes - 1`，而不是剩余文本的结尾。
    let inner = &body[1..quotes - 1];
    Some((length, Some(unescaped(inner))))
}

/// The length of a quoted literal, whose closing quote is the first unescaped one.
/// 带引号字面量的长度：它的结束引号是第一个未被转义的引号。
fn escaped_len(text: &str, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (offset, character) in text.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            found if found == quote => return Some(offset + found.len_utf8()),
            _ => {}
        }
    }
    None
}

/// `text` with Rust's escape sequences decoded.
/// 解码 Rust 转义序列后的 `text`。
///
/// An escape this does not know keeps its backslash, so a rule that used one still reads
/// as itself rather than as something shorter and different.
/// 不认识的转义保留反斜杠，因此用过它的规则读出来仍是它自己，而不是少了一截的别的东西。
fn unescaped(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => decoded.push('\n'),
            Some('r') => decoded.push('\r'),
            Some('t') => decoded.push('\t'),
            Some('0') => decoded.push('\0'),
            Some(other) => decoded.push(other),
            None => decoded.push('\\'),
        }
    }
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_method::test_support::temporary_directory;

    /// Every `require_*` call is read, and neither a comment nor a nested call can
    /// invent or hide a requirement.
    /// 每一处 `require_*` 调用都会被读到，而注释或嵌套调用都不能凭空造出、也不能藏起一条需求。
    ///
    /// Three defects live in this one input, which is why the pin is one assertion:
    /// the rule calls `require_exports` twice (a text scan that reads only the first
    /// occurrence drops the second), the second call passes a function call whose `)`
    /// would end the argument list early, and the comment above is an *example* of the
    /// call — read as code it becomes a requirement no face was asked for, so the build
    /// reports a violation the author never wrote (audit `SUR-S11`).
    /// 这一条输入里住着三个缺陷，因此钉子只有一条断言：规则里 `require_exports` 被调用两次
    /// （只读第一次出现的文本扫描会丢掉第二处），第二处实参里有函数调用，它的 `)` 会提前终止参数
    /// 区间，而上方的注释是那次调用的**示例**——被当成代码读就成了一条没有任何面被要求满足的需求，
    /// 于是构建报出作者从未写过的违约（审计 `SUR-S11`）。
    #[test]
    fn every_rule_call_is_read_and_a_commented_example_is_not_one() {
        let source = "// Example: .require_exports(\"ghost.export\")\n\
                      RegistrationRule::new()\n    \
                      .require_exports(&[\"control.render\"])\n    \
                      .require_exports(&[pick(\"control.preview\"), \"control.extra\"]);";
        assert_eq!(
            rule_method_strings(source, "require_exports"),
            ["control.render", "control.preview", "control.extra"],
            "both calls are read, the comment is not, and the nested `)` does not cut the list"
        );

        // The literal spellings a rule may use around the same call: a raw string, a
        // character literal holding a quote (which must not be read as an opening one),
        // and an escape inside the value.
        // 同一次调用周围可能出现的几种字面量写法：原始字符串、内容含引号的字符字面量（它不得被当成
        // 开引号）、以及值里的转义。
        let spellings = "RegistrationRule::new()\
            .require_preset(r#\"Action\"Parts\"#)\
            .require_exports(&[\"control.quote\\\"inside\"], \"control.escaped\\n\")\
            .require_exports(&[\"control.plain\"]);";
        assert_eq!(
            rule_method_strings(spellings, "require_preset"),
            ["Action\"Parts"]
        );
        assert_eq!(
            rule_method_strings(spellings, "require_exports"),
            [
                "control.quote\"inside",
                "control.escaped\n",
                "control.plain"
            ]
        );
    }

    #[test]
    fn child_errors_are_checked_against_the_parent_registry_rule() {
        let src = temporary_directory("parent-contract");
        let control = src.join("control/control.rs");
        let button = src.join("control/object/button/button.rs");
        let rule = src.join("control/registry_rule/registry_rule.rs");
        write_contracts(
            &control,
            r#"crate::root_object! {
    kind: Control,
    needs_registry: true,
    parent: crate::ROOT_NODE_ID,
    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,
}"#,
        );
        write_contracts(
            &button,
            r#"crate::control_object! {
    kind: BrokenButton,
    preset: WrongPreset,
    parts: BrokenParts,
    parent: crate::control::NODE_ID,
    exports: ["control.preview"],
}"#,
        );
        write_contracts(
            &rule,
            r#"RegistrationRule::new()
    .require_preset("ActionParts")
    .require_exports(&["control.render"])
    .require_handle_traits(&["ControlHandle"])
    .require_part_traits(&["ActionPartsContract"]);"#,
        );
        let nodes = vec![Node {
            name: "control".to_owned(),
            file: Some(control),
            children: vec![Node {
                name: "button".to_owned(),
                file: Some(button),
                children: Vec::new(),
            }],
        }];
        let diagnostics = aggregate_contract_errors(
            &src,
            &nodes,
            false,
            &SourceScope {
                roots: None,
                reason: "test",
            },
        )
        .render_build_diagnostics();
        for expected in [
            "required preset is missing",
            "required export is missing",
            "required handle trait is missing",
            "required part trait is missing",
        ] {
            assert!(diagnostics.contains(expected), "{diagnostics}");
        }
        fs::remove_dir_all(src).expect("temporary fixture cleanup");
    }

    fn write_contracts(path: &Path, source: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("temporary fixture directory");
        fs::write(path, source).expect("temporary fixture source");
    }
}
