//! Who points at this face: its tree edges, its declared fields, and the
//! capability tokens other faces mention.
//! 谁点名了这个面：树的边、它自己声明的字段，以及别的面提到的能力记号。
//!
//! `xirang.explain` answers about one face in isolation; this answers about its
//! neighbourhood, which is the half an agent needs before changing it: which
//! children hang under it, what it declares (the fields `xirang.apply` accepts
//! as input, read back — the write path could set them and nothing could report
//! them), and which faces mention the same capability tokens. The capability part
//! is deliberately labelled: `requires`/`provides` are manifest text, so the match
//! is on declared tokens rather than a resolved graph, and saying so is the
//! difference between evidence and a guess.
//! `xirang.explain` 孤立地回答一个面；这里回答它的邻域，而那正是代理在改动它之前需要的一半：它名下
//! 挂着哪些子面、它声明了什么（`xirang.apply` 作为输入接受的字段，读回来——写入路径能设置它们，
//! 却没有任何工具能报告它们），以及哪些面提到同一批能力记号。能力那一部分刻意被标明：`requires`/
//! `provides` 是清单文本，因此匹配发生在**声明的记号**上而不是一棵已解析的图；把这句话说出来，就是
//! 证据与猜测之间的区别。
//!
//! online: who points at a face is a property of the sources' declarations as they are now, not of the pruning record.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use xirang_kernel::identity::NodeId;

use serde_json::Value;

use crate::mcp::apply::load_registry;
use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::registry::namespace;
use crate::mcp::resolve::resolve_node;

/// Report the neighbourhood of one face.
/// 报告一个面的邻域。
pub(crate) fn usages(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let (faces, unparsable) = crate::mcp::resolve::derived_faces(root, &namespace)?;
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "xirang.usages requires node".to_owned())?;
    let id = resolve_node(root, &namespace, target)?;
    let face = faces
        .iter()
        .find(|face| face.id == id)
        .ok_or_else(|| format!("no face in the derived tree has identity {id}"))?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    // The tree edges above need no registry — they come from the source derivation — so
    // they are answered before the connector is consulted. What needs the registry is
    // the field read-back and the capability tokens, and a package whose own faces the
    // kernel refuses has neither.
    // 上面那些树的边不需要注册机——它们来自源码推导——因此在询问连接器之前就作答了。需要注册机的是
    // 字段读回与能力记号，而一个内核拒绝其自身面的包，两样都没有。
    let mut output = format!(
        "namespace {namespace}\n{unparsable}node {}\n  path {}\n  kind {}\n  parent {}{}\n",
        face.id,
        face.path,
        face.kind,
        face.parent,
        faces
            .iter()
            .find(|candidate| candidate.id == face.parent)
            .map_or_else(
                || " (no face declares it)".to_owned(),
                |parent| format!(" {}", parent.path)
            ),
    );
    let children = faces
        .iter()
        .filter(|candidate| candidate.parent == id)
        .collect::<Vec<_>>();
    output.push_str(&format!("children ({})\n", children.len()));
    for child in children.iter().take(limit) {
        output.push_str(&format!("  {} {}\n", child.path, child.kind));
    }
    if children.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            crate::mcp::truncation::withheld(
                children.len() - limit,
                children.len(),
                limit,
                "children",
                "raise `limit`"
            )
        ));
    }
    // The connector's refusal of this package's own faces is a *verdict about the tree*,
    // not a tool failure — `verify` prints the same judgement on its connector line and
    // `converge` prints it in its verdict block, while this tool used to return it as an
    // error. An agent routing on `isError` therefore read the same rejection as a failure
    // here and as an ordinary answer there (audit `A2`). The tree half above is still
    // answered, because it never needed the registry; the half that did is named as
    // unavailable rather than shown empty — an empty field list would be a claim this
    // tree has no fields, which would be a different and wrong answer.
    // 连接器对本包自身面的拒绝是关于**这棵树的判断**，而不是工具故障——`verify` 在它的连接器行上
    // 打印同一个判断，`converge` 在它的判断块里打印它，而本工具过去把它当错误返回。于是按 `isError`
    // 分流的代理，把同一次拒绝在这里读成失败、在那里读成普通答案（审计 `A2`）。上面的树那一半仍然
    // 作答，因为它从不需要注册机；需要它的那一半被点名为不可用，而不是显示成空的——空的字段清单会
    // 断言这棵树没有字段，那是另一个、而且是错误的答案。
    let registry = match load_registry(root, &namespace) {
        Ok(registry) => registry,
        Err(rejection) => {
            output.push_str(&format!("{}\n", crate::mcp::apply::REJECTED_VERDICT));
            for line in rejection.lines() {
                output.push_str(&format!("  {}\n", line.trim_end()));
            }
            output.push_str(
                "fields and capability refs unavailable: both are read back through a registry the \
                 kernel admits, and this tree's own faces are refused; `xirang.verify` reports \
                 both judging surfaces, and `xirang.apply` previews a fix on a copy\n",
            );
            return Ok(output);
        }
    };
    // The read-back resolves the face's generated path, and that resolution is
    // relative to the package root the context carries — not to the directory the
    // process happens to sit in. Calling it outside the context is what once made
    // an edit land in the wrong tree; the unit tests here have no
    // `XIRANG_PACKAGE_ROOT` set, which is exactly why they catch it.
    // 读回会解析该面的生成路径，而那次解析以上下文携带的包根为基准——不是进程碰巧所在的目录。在上下文
    // 之外调用它，曾经让一次编辑落进错误的树；这里的单元测试没有设 `XIRANG_PACKAGE_ROOT`，这正是
    // 它们能抓到它的原因。
    let context = crate::run_method::AuthoringContext::new(root.to_path_buf(), namespace.clone());
    let read_back = |id| context.scope(|| crate::run_method::authored_face(&registry, id));
    // Every face's declared tokens, and which of them could not be read back. A
    // hand-written module has no generated field list, and the executor refuses to
    // invent one, so it is counted rather than silently skipped.
    // 每个面声明的记号，以及哪些读不回来。手写模块没有生成的字段清单，执行器也拒绝凭空造一个，因此
    // 它被计数，而不是被静默跳过。
    let mut declared: BTreeMap<NodeId, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    let mut unreadable = 0usize;
    for candidate in &faces {
        match read_back(candidate.id) {
            Ok(authored) => {
                declared.insert(
                    candidate.id,
                    (tokens(&authored.requires), tokens(&authored.provides)),
                );
            }
            Err(_) => unreadable += 1,
        }
    }
    match read_back(id) {
        Ok(authored) => {
            output.push_str("fields (read back from the generated module)\n");
            // Every field the write path accepts is listed here: `module`, the two summaries
            // and `flow_provider` were missing while the header claimed the fields were read
            // back, so an agent asking "what can I set" got a shorter answer than the tool's
            // own contract.
            // 写入路径接受的每个字段都在这里列出：`module`、两条摘要与 `flow_provider` 过去缺失，
            // 而表头声称字段已被读回——于是一个问"我能设什么"的代理拿到的答案比这个工具自己的契约
            // 更短。
            for (label, value) in [
                ("module", &authored.module),
                ("preset", &authored.preset),
                ("parts", &authored.parts),
                ("name_zh", &authored.name_zh),
                ("name_en", &authored.name_en),
                ("summary_zh", &authored.summary_zh),
                ("summary_en", &authored.summary_en),
                ("stable_name", &authored.stable_name),
                ("exports", &authored.exports),
                ("requires", &authored.requires),
                ("provides", &authored.provides),
                ("handle_traits", &authored.handle_traits),
                ("handle_contracts", &authored.handle_contracts),
                ("part_traits", &authored.part_traits),
                ("part_contracts", &authored.part_contracts),
                ("registration_rule", &authored.registration_rule),
                ("admission", &authored.admission),
                ("flow", &authored.flow),
                ("flow_provider", &authored.flow_provider),
                ("runtime_checks", &authored.runtime_checks),
                (
                    "getting_from_other_registry",
                    &authored.getting_from_other_registry,
                ),
            ] {
                output.push_str(&format!("  {label} {}\n", blank(value)));
            }
            output.push_str(&format!("  needs_registry {}\n", authored.needs_registry));
        }
        Err(error) => output.push_str(&format!("fields unreadable ({error})\n")),
    }
    // Capability references, by declared token. The direction matters: a provider
    // answers a requirement, a consumer asks for what this face provides.
    // 能力引用，按声明的记号。方向很重要：provider 满足需求，consumer 索取这个面提供的东西。
    let (own_requires, own_provides) = declared.get(&id).cloned().unwrap_or_default();
    let mut providers = Vec::new();
    let mut consumers = Vec::new();
    for candidate in &faces {
        if candidate.id == id {
            continue;
        }
        let Some((requires, provides)) = declared.get(&candidate.id) else {
            continue;
        };
        let offered = intersection(provides, &own_requires);
        if !offered.is_empty() {
            providers.push(format!("{} ({})", candidate.path, offered.join(", ")));
        }
        let asked = intersection(requires, &own_provides);
        if !asked.is_empty() {
            consumers.push(format!("{} ({})", candidate.path, asked.join(", ")));
        }
    }
    output.push_str(&format!(
        "capability refs (matched on declared tokens, not resolved)\n  this face requires: {}\n  this face provides: {}\n",
        listed(&own_requires),
        listed(&own_provides),
    ));
    output.push_str(&format!(
        "  providers ({}): {}\n",
        providers.len(),
        listed(&providers.iter().cloned().collect())
    ));
    output.push_str(&format!(
        "  consumers ({}): {}\n",
        consumers.len(),
        listed(&consumers.iter().cloned().collect())
    ));
    if unreadable > 0 {
        output.push_str(&format!(
            "unreadable faces {unreadable} (hand-written modules are not read back)\n"
        ));
    }
    Ok(output)
}

/// The identifier-ish tokens of a manifest text field.
/// 一个清单文本字段里的标识符式记号。
fn tokens(text: &str) -> BTreeSet<String> {
    text.split(|character: char| {
        !(character.is_alphanumeric() || character == '_' || character == '.' || character == ':')
    })
    .filter(|token| token.len() > 1 && token.chars().any(|c| c.is_alphanumeric()))
    .map(str::to_owned)
    .collect()
}

/// The tokens two sets share.
/// 两个集合共有的记号。
fn intersection(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.intersection(right).cloned().collect()
}

/// An empty field reads as `-` rather than as nothing at all.
/// 空字段读作 `-`，而不是什么都没有。
fn blank(value: &str) -> &str {
    if value.trim().is_empty() { "-" } else { value }
}

/// A sorted, comma-joined list, or `-`.
/// 排序后用逗号连接，空则 `-`。
fn listed(items: &BTreeSet<String>) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

#[cfg(test)]
#[path = "usages_tests.rs"]
mod usages_tests;
