//! Who answers a request made on a virtual workspace root.
//! 在虚拟工作区根上，由谁来回答一次请求。
//!
//! `workspace.rs` is the entrance for the tools that *can* merge a whole workspace:
//! `registry`, `grafts`, `diff` and `search` enumerate the members, give each one its own
//! identity context and answer in sections. The rest of the bridge refused there, with
//! Cargo's own words — `cannot learn the identity namespace of …` — because "package =
//! namespace" is the design and a workspace root is not a package. A refusal is not an
//! answer, and the capability argument cuts the other way: the merged view must be at
//! least as reachable as the single-package one.
//! `workspace.rs` 是那些**能够**合并整个工作区的工具的入口：`registry`、`grafts`、`diff` 与
//! `search` 枚举成员、给每个成员自己的身份上下文并按小节作答。桥里其余工具在那里拒绝，用的是 Cargo
//! 自己的话——`cannot learn the identity namespace of …`——因为"包 = 命名空间"是设计，而工作区根不是
//! 一个包。拒绝不是答案，而能力上的论证指向另一边：合并视图至少要像单包视图一样够得着。
//!
//! This module is the entrance's other half. It classifies each tool by the **subject** its
//! request names, and resolves ownership accordingly:
//! 本模块是那个入口的另一半。它按请求点名的**主体**给每个工具分类，并据此解析归属：
//!
//! - a tool whose request names a **filesystem path** (`read`, `inspect`, `mir`, `unified`,
//!   `callgraph`) is answered by the one member whose directory is a prefix of that path,
//!   under that member's own root; a path no member owns says so instead of reporting a
//!   missing namespace.
//! - 请求点名**文件系统路径**的工具（`read`、`inspect`、`mir`、`unified`、`callgraph`）由那个目录是
//!   该路径前缀的成员、在那成员自己的根之下来回答；没有成员拥有的路径会说出来，而不是报告缺少命名空间。
//! - a tool whose request names a **face** (`explain`, `trace`, `impact`, `usages`,
//!   `converge`, `verify`) asks **every** member and groups the answers by member, with a
//!   census of which members answered and which declined, and a member that could not
//!   answer carries `tree unavailable (reason)` where its body would have been. No
//!   member's result is presented as the global one.
//! - 请求点名**面**的工具（`explain`、`trace`、`impact`、`usages`、`converge`、`verify`）会问
//!   **每一个**成员并按成员分组给出答案，附一份"哪些成员答了、哪些没答"的普查；答不了的成员在它本该有
//!   正文的位置带上 `tree unavailable (原因)`。没有任何成员的结果被当成全局答案。
//! - the **write path** (`apply`, `plugin`) must resolve a unique owner before anything runs,
//!   and refuses with the candidate members when it cannot: writing into the wrong package is
//!   the worst outcome this bridge can produce, so it is never guessed.
//! - **写入路径**（`apply`、`plugin`）必须在运行任何东西之前解析出唯一拥有者；解析不出时就带着候选
//!   成员拒绝：写进错误的包是本桥能造成的最坏结果，因此绝不猜。
//!
//! The per-package body is always the tool's **own single-package implementation**, handed a
//! member's directory — so a merged answer and a single-package answer cannot disagree,
//! because they are the same code (`workspace.rs` states the same rule for its own merge).
//! 逐包主体始终是该工具**自己的单包实现**，只是接收成员的目录——因此合并答案与单包答案不可能不一致，
//! 因为它们是同一段代码（`workspace.rs` 对它自己的合并声明了同一条规则）。
//!
//! online: routing a request needs the faces of **this** tree as it is now, because a member may have been added since the build.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xirang_kernel::NodeId;

use crate::mcp::source_index::portable_path;
use crate::mcp::workspace::{self, Member, Scope};

/// One tool's implementation: the package root plus the `arguments` object.
/// 一个工具的实现：包根加上 `arguments` 对象。
type Handler = fn(&Path, &Value) -> Result<String, String>;

/// The keys a filesystem-path tool reads its path from, in priority order.
/// 一个文件系统路径工具读取其路径所用的键，按优先级排列。
const PATH_KEYS: &[&str] = &["path", "against"];

/// The key a face-naming tool reads its subject from.
/// 点名面的工具读取其主体的那个键。
const NODE_KEYS: &[&str] = &["node"];

/// A tool that names no subject at all: the package itself is the subject.
/// 根本不点名主体的工具：包本身就是主体。
const NO_KEYS: &[&str] = &[];

/// The keys the write path reads its subject from, in priority order.
/// 写入路径读取其主体所用的键，按优先级排列。
///
/// `fields.parent` is the alias `resolve::parent_id` accepts, so an `add` that puts every
/// value in one object still names a parent here rather than reading as a subject-less
/// request.
/// `fields.parent` 是 `resolve::parent_id` 接受的别名，因此把所有取值放进一个对象的 `add` 在这里
/// 仍然点名了一个父级，而不会被读成没有主体的请求。
const WRITE_KEYS: &[&str] = &["node", "parent", "fields.parent", "selector"];

/// What a tool's request names, which is what decides who answers it on a virtual root.
/// 一个工具的请求点名了什么，而这决定了在虚拟根上由谁回答。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Subject {
    /// The request names a filesystem path; exactly one member owns it.
    /// 请求点名一个文件系统路径；恰好一个成员拥有它。
    Path,
    /// The request may name a filesystem path; without one it is answered **once from the
    /// root**. A name lookup is one question about the whole workspace rather than one question
    /// per member: a caller lives in a member that depends on the definition's own member, so a
    /// per-member fan-out loses every cross-crate caller — measured on the real tree, one symbol
    /// had 36 callers over the workspace and 17 when each member searched only itself.
    /// 请求可以点名一个文件系统路径；没有点名时**从根上一次性作答**。名字查询是关于整个工作区的一个问题，
    /// 而不是每个成员各问一遍：调用者住在依赖定义所在成员的那个成员里，因此逐成员扇出会丢掉每一个跨 crate
    /// 调用者——真树实测：同一个符号在整个工作区上 36 个调用者，而每个成员只搜自己时 17 个。
    WorkspaceOrOwner,
    /// The request names a face; every member's tree is asked and the answers grouped.
    /// 请求点名一个面；问每一个成员的树，并把答案分组。
    EveryMember(&'static [&'static str]),
    /// The request writes, so a unique owner must be resolved before anything runs.
    /// 请求会写入，因此必须在运行任何东西之前解析出唯一拥有者。
    Write,
    /// The tool answers a virtual root itself, or needs no identity at all.
    /// 该工具自己就能回答虚拟根，或者根本不需要身份。
    SelfAnswering,
}

/// The subject of one tool, by the name `tools/list` advertises.
/// 一个工具的主体，按 `tools/list` 声明的名字。
///
/// The classification is a table rather than an inference from the schema, because the two
/// are different facts: what a tool reads is a property of the tool, and a request that
/// happens to carry a `path` says nothing about whether a path is what it means.
/// 分类是一张表，而不是从 schema 推断出来的，因为这是两件事：一个工具读什么是这个工具的属性，而一个
/// 恰好带着 `path` 的请求并不能说明它意指的就是一条路径。
pub(crate) fn subject(tool: &str) -> Subject {
    match tool {
        "xirang.read" | "xirang.inspect" | "xirang.mir" | "xirang.unified" => Subject::Path,
        "xirang.callgraph" => Subject::WorkspaceOrOwner,
        // `crates` writes generated packages and `revert` takes them back, so the ownership
        // question is the same one `apply` asks: is this root the tree being written.
        // `crates` 写下生成包、`revert` 把它们收回去，因此归属问题与 `apply` 问的是同一个：这个根是不是
        // 被写的那棵树。
        "xirang.apply" | "xirang.plugin" | "xirang.crates" | "xirang.candidate"
        | "xirang.rules" | "xirang.refactor" | "xirang.task" => Subject::Write,
        // The scaffold names its own destination and writes a directory that does not exist
        // yet, so the root answers it directly: the ownership question is whether the
        // destination is inside this root, which is a fact about paths rather than about a
        // member's tree, and the tool's own body names both paths when it is not.
        // 脚手架自己点名目的地，而且写的是一个尚不存在的目录，因此由根直接作答：归属问题是目的地是否
        // 落在这个根之内，而这是关于路径的事实而不是关于某个成员的树的事实；不在根内时，该工具自己的
        // 主体会点名两条路径。
        "xirang.new_project" => Subject::SelfAnswering,
        "xirang.explain" | "xirang.impact" | "xirang.usages" | "xirang.converge" => {
            Subject::EveryMember(NODE_KEYS)
        }
        // `trace` and `verify` take no subject: the artifact and the package are the
        // subject, so every member's own answer is the only honest one.
        // `trace` 与 `verify` 不接受主体：artifact 与包本身就是主体，因此每个成员自己的答案才是唯一诚实
        // 的答案。
        "xirang.trace" | "xirang.verify" => Subject::EveryMember(NO_KEYS),
        // `search`, `registry`, `grafts` and `diff` merge the workspace themselves; `status`
        // reads the filesystem and needs no identity at all.
        // `search`、`registry`、`grafts` 与 `diff` 自己合并工作区；`status` 读文件系统，根本不需要身份。
        _ => Subject::SelfAnswering,
    }
}

/// Answer one tool call, resolving ownership first when the root is a virtual manifest.
/// 回答一次工具调用；根是虚拟清单时先解析归属。
///
/// A root that is one package — and a directory with no manifest at all, which these tools
/// read too — goes straight to the handler, so a single-package answer is unchanged byte
/// for byte. Only a virtual manifest is resolved here.
/// 只有一个包的根——以及根本没有清单、而这些工具同样读取的目录——直接交给处理函数，因此单包答案逐字节
/// 不变。只有虚拟清单会在这里被解析。
pub(crate) fn dispatch(
    root: &Path,
    tool: &str,
    arguments: &Value,
    handler: Handler,
) -> Result<String, String> {
    if (tool == "xirang.mir" && arguments.get("capture").and_then(Value::as_bool) == Some(true))
        || (tool == "xirang.read"
            && arguments
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| root.join(path).is_dir()))
    {
        return handler(root, arguments);
    }
    match subject(tool) {
        Subject::SelfAnswering => handler(root, arguments),
        Subject::Path => resolve_owner(root, arguments, handler),
        Subject::WorkspaceOrOwner => {
            if arguments.get("path").and_then(Value::as_str).is_some() {
                return resolve_owner(root, arguments, handler);
            }
            match members(root) {
                Some(members) => {
                    // The answer is built first: handing a body its sources is what marks a
                    // member used, and the census composed afterwards can then say which member
                    // the answer rested on.
                    // 先构成答案：把源码交给主体正是把成员标为已用的动作，随后构成的普查才说得出这份答案
                    // 依托的是哪个成员。
                    let answer = handler(root, arguments)?;
                    // Terse, like the lookup tools it serves: the reason is expanded on the
                    // **face and build** path (`every_member`, which `explain`/`verify` come
                    // through), and nowhere else, which is t1's decision.
                    // 短档，与它所服务的查表工具一致：原因只在**面与构建**那条路径
                    // （`every_member`，`explain`/`verify` 经它进来）展开，别处不展开——这是 t1 的决定。
                    let mut output = workspace::roster(root, &members);
                    output.push_str(&answer);
                    output.push_str(workspace::DETAIL);
                    Ok(output)
                }
                None => handler(root, arguments),
            }
        }
        Subject::EveryMember(keys) => resolve_every_member(root, arguments, handler, keys),
        Subject::Write => resolve_write(root, tool, arguments, handler),
    }
}

/// The workspace a root is, or `None` when it is one package — or a directory with no
/// manifest, which the tools that resolve ownership still read.
/// 根是哪个工作区；当它是一个包——或一个没有清单、而解析归属的工具仍会读取的目录——时为 `None`。
///
/// A separate function so every resolver states the same three-way decision once: a
/// workspace is enumerated, anything else goes to the handler unchanged.
/// 单独一个函数，好让每个解析器只声明同一次三分判断：工作区被枚举，其余一切都原样交给处理函数。
fn members(root: &Path) -> Option<Vec<Member>> {
    match workspace::scope(root) {
        Ok(Scope::Workspace(members)) => Some(members),
        _ => None,
    }
}

/// The path a request names, by the keys a tool reads it from, with its key.
/// 请求点名的路径，按工具读取它的那些键，连同它的键名。
fn named_paths(arguments: &Value, keys: &[&str]) -> Vec<(String, String)> {
    keys.iter()
        .filter_map(|key| {
            arguments
                .get(*key)
                .and_then(Value::as_str)
                .map(|value| ((*key).to_owned(), value.to_owned()))
        })
        .collect()
}

/// The one path a request names by dotted keys, reading `fields.parent` as a path.
/// 请求按点分键点名的那一个路径，把 `fields.parent` 当作路径读取。
fn named_subject<'a>(arguments: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| match key.split_once('.') {
        Some((head, tail)) => arguments
            .get(head)
            .and_then(|value| value.get(tail))
            .and_then(Value::as_str),
        None => arguments.get(*key).and_then(Value::as_str),
    })
}

/// Answer a request that names a filesystem path, from the member that owns it.
/// 从拥有它的那个成员回答一个点名了文件系统路径的请求。
fn resolve_owner(root: &Path, arguments: &Value, handler: Handler) -> Result<String, String> {
    let Some(members) = members(root) else {
        return handler(root, arguments);
    };
    let named = named_paths(arguments, PATH_KEYS);
    if named.is_empty() {
        return handler(root, arguments);
    }
    // Ownership is decided by the member's **root prefix**, which is the only thing that
    // can say which identity domain a path lives in: the derivation hashes source paths
    // relative to a package's own `src/`, so a path belongs to the package whose
    // directory contains it and to no other.
    // 归属由成员的**根前缀**判定，而这是唯一能说出一个路径住在哪个身份域里的东西：推导是对相对包自己
    // `src/` 的源码路径取散列，因此一个路径属于那个目录包含它的包，而不属于别的包。
    let owners: BTreeSet<usize> = named
        .iter()
        .filter_map(|(_, path)| owner_of(&members, root, path))
        .collect();
    // Terse: `read`/`inspect` are lookup tools, and expanding their member list would contradict
    // t1's decision that only the face and build path carries the reason (measured: +324 chars).
    // 短档：`read`/`inspect` 是查表工具，展开它们的成员清单会与 t1"只有面与构建那条路径带原因"的
    // 决定相抵（实测 +324 字符）。
    let mut output = workspace::roster(root, &members);
    let (_, primary_path) = &named[0];
    match owners.len() {
        0 => output.push_str(&no_owner(&members, primary_path)),
        1 => {
            let index = *owners.iter().next().expect("one owner");
            output.push_str(&owned_answer(
                root, &members, index, arguments, &named, handler,
            )?);
        }
        _ => {
            output.push_str(&format!(
                "`{primary_path}` resolves in {} members, so no single member owns this request; \
                 name one with `root`.\n",
                owners.len()
            ));
        }
    }
    output.push_str(workspace::DETAIL);
    Ok(output)
}

/// The section for the one member that owns the request, and the tool's own answer from it.
/// 拥有该请求的那个成员的小节，以及来自它的该工具自己的答案。
fn owned_answer(
    root: &Path,
    members: &[Member],
    index: usize,
    arguments: &Value,
    named: &[(String, String)],
    handler: Handler,
) -> Result<String, String> {
    let member = &members[index];
    // Every path the same member owns is rewritten to be relative to that member's own
    // root, because that is the root the body is handed. A path another member owns — or
    // that no member owns — is left exactly as the caller wrote it, so the tool's own
    // message about it stays the message the caller can act on.
    // 同一个成员拥有的每一条路径都被改写成相对该成员自己根的形式，因为交给主体的就是这个根。别的成员
    // 拥有的路径——或没有成员拥有的路径——原样保留调用方写下的样子，因此该工具自己关于它的消息仍是
    // 调用方能据以行动的那一条。
    let mut redirected = arguments.clone();
    if let Some(object) = redirected.as_object_mut() {
        for (key, path) in named {
            if owner_of(members, root, path) == Some(index) {
                object.insert(key.clone(), json!(relative_to(member, root, path)));
            }
        }
    }
    let (_, primary_path) = &named[0];
    let mut output = format!(
        "owner {} owns `{primary_path}` (its root is {})\n== {} ({})\n",
        member.name,
        member.dir.display(),
        member.name,
        member.status(),
    );
    // The body is the tool's own single-package implementation. A member whose tree cannot
    // be read gets no body: it says so, because the reader is looking here.
    // 主体就是该工具自己的单包实现。树读不了的成员没有主体：它说出来，因为读者正是在看这里。
    match member.tree() {
        Ok(_) => match handler(&member.dir, &redirected) {
            Ok(text) => output.push_str(&text),
            Err(reason) => output.push_str(&format!(
                "tree unavailable ({})\n",
                workspace::one_line(&reason)
            )),
        },
        Err(reason) => {
            output.push_str(&format!(
                "tree unavailable ({})\n",
                workspace::one_line(&reason)
            ));
        }
    }
    Ok(output)
}

/// What a path no member owns has to say, and the directories that were compared.
/// 一条没有成员拥有的路径必须说的话，以及被比较过的那些目录。
fn no_owner(members: &[Member], path: &str) -> String {
    let mut output = format!(
        "no member owns `{path}`: no member directory is a prefix of it, so no member has an \
         identity context for this path. Members and their directories:\n"
    );
    for member in members {
        output.push_str(&format!("  {:<32} {}\n", member.name, member.dir.display()));
    }
    output
}

/// Whether `text` is declared by a member's derived tree, and how many members declare it.
/// `text` 是否被某个成员的推导树声明，以及有多少成员声明它。
///
/// A logical path is spelled the same in every member — `root/button` means "the `button`
/// child of whichever tree" — so "which member owns it" is a question about the members'
/// trees rather than about the string. `root` (and the empty string) is a member's own
/// root, which every member has, so it names every member and therefore no unique owner.
/// 逻辑路径在每个成员里的拼法是相同的——`root/button` 的意思是"无论哪棵树里的 `button` 子节点"——
/// 因此"哪个成员拥有它"是关于成员树的问题，而不是关于这个字符串的问题。`root`（以及空串）是成员自己的
/// 根，每个成员都有，因此它点名每一个成员、也就没有唯一拥有者。
fn owners_of(members: &[Member], target: &str) -> Vec<usize> {
    let target = target.trim_start_matches('/');
    // An identity the build published is a hit: the manifest pairs it with this
    // package and no other package can hold it — every id is a hash over the
    // namespace — so the search stops at the member the build wrote it into
    // instead of deriving every member's tree. Whether that member still declares
    // it is a question only that member's own answer can settle, and it is asked
    // there: a face deleted since the build comes back as that member's own
    // refusal rather than as "no member owns it". A **miss** proves nothing — a
    // face added since the build lives in the sources only — so the loop below
    // still derives every member, exactly as this resolver always did.
    // 构建发布过的身份就是命中：清单把它与这个包配对，别的包不可能持有它——每个 id 都是对命名空间
    // 的散列——因此搜索停构建把它写进的那个成员，而不是推导每个成员的树。那个成员是否仍然声明它，
    // 只有它自己的答案能裁定，而问题就在那里问：构建之后被删掉的面会以该成员自己的拒绝回来，而不是
    // "没有成员拥有它"。**未命中**什么也证明不了——构建之后新增的面只住在源码里——因此下面的循环
    // 仍然推导每个成员，与本解析器一直以来的做法完全相同。
    if let Ok(id) = target.parse::<NodeId>() {
        let published: Vec<usize> = members
            .iter()
            .enumerate()
            .filter_map(|(index, member)| member.publishes(id).then_some(index))
            .collect();
        if !published.is_empty() {
            return published;
        }
    }
    members
        .iter()
        .enumerate()
        .filter_map(|(index, member)| {
            let Ok((faces, _)) = member.derived_tree() else {
                return None;
            };
            if target.is_empty() || target == "root" {
                return Some(index);
            }
            if let Ok(id) = target.parse::<NodeId>() {
                // An identity is checked against the member's own faces rather than taken
                // at its word: `resolve_node` returns any parsable identity as-is, so
                // without this every member would claim it.
                // 身份是拿成员自己的面来核对的，而不是照单全收：`resolve_node` 会把任何能解析的身份原样
                // 返回，没有这一道，每个成员都会声称拥有它。
                return faces.iter().any(|face| face.id == id).then_some(index);
            }
            faces
                .iter()
                .any(|face| face.path == target)
                .then_some(index)
        })
        .collect()
}

/// Answer a request that names a face from **every** member, grouped, with a census.
/// 从**每一个**成员回答一个点名了面的请求，分组并附普查。
fn resolve_every_member(
    root: &Path,
    arguments: &Value,
    handler: Handler,
    keys: &'static [&'static str],
) -> Result<String, String> {
    let Some(members) = members(root) else {
        return handler(root, arguments);
    };
    every_member(root, &members, arguments, handler, keys)
}

/// Run the tool's own body for every member and group what came back.
/// 为每个成员跑一次该工具自己的主体，并把回来的东西分组。
///
/// A member that derived its tree but whose body refused keeps its refusal **here**, where
/// its body would have been, instead of failing the whole call: one member that cannot
/// resolve the named face says nothing about whether another can, and a bridge-wide error
/// would hide both the members that did answer and the ones that did not.
/// 推导出了树、而主体拒绝的成员把它的拒绝留在**这里**、也就是它本该有正文的位置，而不是让整次调用失败：
/// 一个成员解析不了点名的面，说明不了另一个成员能不能，而一条桥级错误会把答了的成员与没答的成员一起
/// 藏掉。
fn every_member(
    root: &Path,
    members: &[Member],
    arguments: &Value,
    handler: Handler,
    keys: &'static [&'static str],
) -> Result<String, String> {
    let mut sections = String::new();
    let mut answered = 0usize;
    let mut declined: Vec<(String, String)> = Vec::new();
    let mut no_tree: Vec<String> = Vec::new();
    for member in members {
        sections.push_str(&format!("\n== {} ({})\n", member.name, member.status()));
        match member.tree() {
            Ok(_) => match handler(&member.dir, arguments) {
                Ok(text) => {
                    answered += 1;
                    sections.push_str(&text);
                }
                Err(reason) => {
                    declined.push((member.name.clone(), workspace::one_line(&reason)));
                    sections.push_str(&format!(
                        "tree unavailable ({})\n",
                        workspace::one_line(&reason)
                    ));
                }
            },
            Err(reason) => {
                no_tree.push(member.name.clone());
                sections.push_str(&format!(
                    "tree unavailable ({})\n",
                    workspace::one_line(&reason)
                ));
            }
        }
    }
    // `explain` and `verify` come through here, and their rows are the ones t1's acceptance
    // named: the reason is expanded like `registry`/`diff`/`grafts`, so the face and build tools
    // spell their member lists the same way.
    // `explain` 与 `verify` 经这里进来，而它们那几行正是 t1 验收点名的：原因像
    // `registry`/`diff`/`grafts` 一样展开，因此面与构建类工具的成员清单拼法一致。
    let mut output = workspace::roster_expanded(root, members);
    output.push_str(&format!(
        "answers: {} of {} members answered this request  declined {}  no tree {}\n",
        answered,
        members.len(),
        declined.len(),
        no_tree.len()
    ));
    // The census names the members that did **not** answer, because "2 of 3 answered" is
    // only actionable once the reader knows which one did not and why; the reason is the
    // one that member's own body gave, folded onto one line.
    // 普查点名**没有**作答的成员，因为"三个里答了两个"只有在读者知道是哪一个、以及为什么之后才可据以
    // 行动；原因是那个成员自己的主体给出的那一个，折成一行。
    for (name, reason) in &declined {
        output.push_str(&format!("declined {name}: {reason}\n"));
    }
    for name in &no_tree {
        output.push_str(&format!("no tree {name}\n"));
    }
    let target = named_subject(arguments, keys);
    if answered == 0 && !members.is_empty() {
        match target {
            Some(target) => output.push_str(&format!(
                "no member owns `{target}`: every member's own answer declined it, and each \
                 reason is above.\n"
            )),
            None => output
                .push_str("no member answered this request; each member's own reason is above.\n"),
        }
    }
    output.push_str(&sections);
    output.push_str(workspace::DETAIL);
    Ok(output)
}

/// Answer the write path, which runs only against a member it is the unique owner of.
/// 回答写入路径；它只会对它唯一拥有的那个成员运行。
fn resolve_write(
    root: &Path,
    tool: &str,
    arguments: &Value,
    handler: Handler,
) -> Result<String, String> {
    // A root Cargo cannot name is refused where it always was, with that tool's own
    // message: the write path has nothing to add to it.
    // Cargo 说不出名字的根仍在它一贯被拒绝的地方被拒绝，用的是那个工具自己的消息：写入路径对它没有
    // 可补充的。
    let scope = workspace::scope(root)?;
    let members = match scope {
        Scope::Workspace(members) => members,
        Scope::Package(_) => return handler(root, arguments),
        Scope::Unresolvable(reason) => return Ok(workspace::unresolvable(root, &reason)),
    };
    let target = named_subject(arguments, WRITE_KEYS);
    let owners = target.map_or_else(Vec::new, |target| owners_of(&members, target));
    // Exactly one owner is the precondition. Zero owners means nothing in the request
    // names a member's tree (an `add` with no parent names every tree's root); more than
    // one means the name is spelled the same in several trees. Both are refusals that
    // list the candidates rather than a guess followed by a write.
    // 唯一拥有者是前提。零个拥有者意味着请求里没有任何东西点名某个成员的树（没有父级的 `add` 点名了
    // 每一棵树的根）；多于一个意味着这个名词在好几棵树里拼法相同。两者都是列出候选的拒绝，而不是"先猜
    // 再写"。
    match owners.as_slice() {
        [index] => {
            let member = &members[*index];
            let mut output = format!(
                "owner {} owns `{}` — this write runs in that member\n== {} ({})\n",
                member.name,
                target.unwrap_or_default(),
                member.name,
                member.status()
            );
            output.push_str(&handler(&member.dir, arguments)?);
            Ok(output)
        }
        _ => Ok(refused_write(root, tool, &members, target, owners.len())),
    }
}

/// The refusal the write path gives on a virtual root, with the members it could name.
/// 写入路径在虚拟根上给出的拒绝，连同它可以点名的那些成员。
fn refused_write(
    root: &Path,
    tool: &str,
    members: &[Member],
    target: Option<&str>,
    owners: usize,
) -> String {
    // Terse: a refusal is not the build/face picture. The `REFUSED:` line itself is untouched
    // (the shape line is a red line).
    // 短档：拒绝不是面/构建的画面。`REFUSED:` 那一行本身未动（形状行是红线）。
    let mut output = workspace::roster(root, members);
    output.push_str(&format!(
        "\nREFUSED: {tool} needs one package, and this root is a virtual manifest that names \
         none; nothing was copied and nothing was written.\n"
    ));
    match (target, owners) {
        (Some(target), 0) => output.push_str(&format!(
            "no member's tree declares `{target}`, so no member owns this write.\n"
        )),
        (Some(target), count) => output.push_str(&format!(
            "`{target}` is declared in {count} members, so no unique owner can be resolved for \
             this write.\n"
        )),
        (None, _) => output.push_str(
            "this request names no face that could identify a member among these, and the root \
             names none either.\n",
        ),
    }
    output.push_str(
        "candidates — pass `root` as one of these member directories (relative to this root) to \
         write there:\n",
    );
    for member in members {
        output.push_str(&format!(
            "  {:<32} ({}, {})\n",
            relative_dir(root, member),
            member.status(),
            member.name
        ));
    }
    output.push_str(workspace::DETAIL);
    output
}

/// The member whose directory contains `path`, when exactly one does.
/// 目录包含 `path` 的那个成员，当且仅当只有一个时。
///
/// The longest matching prefix wins, so a package nested inside another member's directory
/// is not shadowed by its ancestor. The comparison is canonical on both sides, because a
/// root that reaches the members through a symbolic link would otherwise compare two
/// spellings of one directory.
/// 匹配最长前缀者胜出，因此嵌在另一个成员目录里的包不会被它的祖先遮住。两侧都比较规范形式，否则一个
/// 经符号链接到达成员的根会拿同一个目录的两种拼法去比较。
fn owner_of(members: &[Member], root: &Path, path: &str) -> Option<usize> {
    let candidate = canonical(&absolute(root, path));
    members
        .iter()
        .enumerate()
        .filter(|(_, member)| candidate.starts_with(canonical(&member.dir)))
        .max_by_key(|(_, member)| canonical(&member.dir).components().count())
        .map(|(index, _)| index)
}

/// `path` as an absolute path, taken against `root` when it is relative.
/// `path` 的绝对形式；它是相对路径时以 `root` 为基准。
fn absolute(root: &Path, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

/// A path's canonical form, or the path itself when it does not resolve.
/// 路径的规范形式，或者在解析不了时就是它本身。
///
/// A path that does not exist yet — a MIR artifact about to be produced, a file this
/// request only names — still has to be attributable to a member, and the lexical prefix
/// answers that.
/// 尚不存在的路径——一份即将产出的 MIR artifact、这次请求只是点名的文件——仍然必须能归属到某个成员，
/// 而词法前缀回答得了这个。
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A path rewritten relative to the member's own root, which is the root its body gets.
/// 一条被改写成相对该成员自己根的路径，也就是它主体得到的那个根。
fn relative_to(member: &Member, root: &Path, path: &str) -> String {
    absolute(root, path)
        .strip_prefix(&member.dir)
        .map_or_else(|_| path.to_owned(), portable_path)
}

/// A member's directory as the caller would pass it back, relative to the workspace root.
/// 成员的目录，按调用方会传回来的样子——相对工作区根。
fn relative_dir(root: &Path, member: &Member) -> String {
    member
        .dir
        .strip_prefix(root)
        .map_or_else(|_| member.dir.display().to_string(), portable_path)
}

#[cfg(test)]
#[path = "ownership_tests.rs"]
mod ownership_tests;
