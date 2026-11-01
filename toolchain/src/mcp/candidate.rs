//! A candidate tree: the change is made and verified where the base is not, and only a declared
//! scope is ever landed.
//! 候选树：改动在**基树之外**做与验证，而只有声明过的范围才会被落地。
//!
//! The specification separates two capabilities that are easy to blur. An ordinary MCP surface
//! **verifies**; a managed execution environment **keeps a write from escaping**. This module is the
//! second one, and what makes it that is a property of its construction rather than a promise in its
//! description: `open` copies the tree, `verify` runs inside the copy, and the base tree is not
//! written by anything until `land` — which then refuses unless the declared scope covers every
//! changed path, the base has not moved since the copy was taken, and the verification is bound to
//! the candidate's **current** bytes.
//! 规格把两种能力分开，而它们很容易被混淆。普通 MCP 执行面**核验**；托管执行环境**阻止写入外溢**。本模块是
//! 后者，而让它成为后者的是它的**构造性质**，不是它描述里的一句承诺：`open` 复制这棵树，`verify` 在副本里跑，
//! 在 `land` 之前没有任何东西写过基树——而 `land` 会拒绝，除非声明的范围覆盖了每一条改动路径、基树自取副本以来
//! 没有动过、且验证绑定在候选**当前**的字节上。
//!
//! What this does **not** do, said here rather than discovered later: it does not sandbox a process. The
//! boundary is the **landing**, not the command — a process handed the base path could still write it,
//! and `verify` hands it the copy. What it *does* do is make a landing a transaction over files: the old bytes,
//! their digests, the new digests and the operation's identity are written down before the first byte of the base
//! moves, a failure undoes exactly the files this landing had written, and a landing that stopped is finished or
//! undone by the next one rather than by asking a person to clean up. That is a process-level guarantee, not a
//! power-cut one: the record and the rollback bytes are ordinary writes.
//! 它**不**做什么，写在这里而不是等别人发现：它不沙箱化进程。边界在**落地**，不在命令——拿到基树路径的进程照样
//! 能写它，而 `verify` 给它的是副本。它**确实**做的是把落地变成一次跨文件事务：旧字节、它们的摘要、新摘要与这次操作的
//! 身份都在基树的第一个字节移动之前写下来，失败只撤销这次落地已经写下的那些文件，而一次停下的落地由下一次落地完成或撤销，
//! 不需要请人去清场。这是**进程级**保证而不是断电级保证：记录与回滚字节都是普通写入。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The actions this surface takes, and why there is no default.
/// 这个执行面的动作，以及为什么没有默认值。
///
/// A default here would have to be either a write (`open`, `land`) or a read (`status`), and a surface
/// whose **default** is a write must not be reachable by omitting an argument — a caller that mistypes
/// `action` would otherwise get a copy of its tree, or worse, a landing.
/// 这里若有默认值，它要么是写（`open`、`land`）要么是读（`status`），而**默认**是写的执行面绝不能靠省略参数
/// 到达——否则把 `action` 打错的调用方会得到它那棵树的一份副本，或者更糟，一次落地。
pub(super) const ACTIONS: &[&str] = &[
    "open", "verify", "land", "status", "discard", "propose", "widen",
];

/// The policy version this surface adjudicates uses under.
/// 本执行面据以裁定一次使用的政策版本。
///
/// It is a dimension of every use request, so a grant issued under another version of the policy is
/// refused by name instead of being spent as if the rules had not changed.
/// 它是每次使用请求的一个维度，因此按另一版政策签发的授权会**被点名拒绝**，而不是被当作规则没变过一样花掉。
pub(super) const POLICY: &str = "xirang.candidate/v1";

/// The most paths one candidate's scope may name.
/// 一个候选的范围最多点名多少条路径。
const MAX_SCOPE: usize = 64;

/// One candidate's durable record.
/// 一个候选的持久记录。
pub(super) struct Record {
    /// Where the record itself lives.
    /// 记录自己住在哪里。
    path: PathBuf,
    /// The parsed record.
    /// 解析后的记录。
    pub(super) body: Value,
}

impl Record {
    /// Read one candidate back by id.
    /// 按 id 读回一个候选。
    pub(super) fn read_at(directory: &Path, id: &str) -> Result<Self, String> {
        let path = directory.join(format!("{id}.json"));
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("unknown candidate `{id}` ({}): {error}", path.display()))?;
        let body =
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(Self { path, body })
    }

    /// Publish this record, through the same write-then-rename every durable record here uses.
    /// 发布这条记录，走这里每份持久记录都用的"先写临时再 rename"。
    pub(super) fn publish(&self) -> Result<(), String> {
        super::check_plan::persist(&self.path, &self.body)
    }

    /// The copy's root.
    /// 副本的根。
    pub(super) fn tree(&self) -> Result<PathBuf, String> {
        self.body["tree"]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| "the record names no tree".to_owned())
    }

    /// The declared writable scope.
    /// 声明的可写范围。
    pub(super) fn scope(&self) -> Vec<String> {
        self.body["scope"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    }
}

/// `xirang.candidate`: open, verify, land, inspect or discard one candidate tree.
/// `xirang.candidate`：打开、验证、落地、查看或丢弃一棵候选树。
pub(super) fn candidate(root: &Path, arguments: &Value) -> Result<String, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "candidate requires an explicit action: {}; there is no default because the default would be a write",
                ACTIONS.join(", ")
            )
        })?;
    if action == "review" {
        // A removed action gets its own refusal, so a caller that still passes it learns where the
        // verdict went instead of reading a list of actions it never asked about.
        // 被移除的动作有自己的拒绝，好让仍在传它的调用方知道判定去哪了，而不是读到一串它没问过的动作。
        return Err(
            "recording a review is not a tool action: a tool that writes a verdict can write its own pass. The host review entry prints the packet the reviewer works from — `xirang-mcp --review --root <base> --id <candidate>` — and records `--verdict` against `--packet <digest>`; this surface can only print the packet (`status` does)"
                .to_owned(),
        );
    }
    if !ACTIONS.contains(&action) {
        return Err(format!(
            "candidate action is one of {}, not `{action}`; nothing was done",
            ACTIONS.join(", ")
        ));
    }
    let directory = arguments
        .get("target_dir")
        .and_then(Value::as_str)
        .map(|value| root.join(value))
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/candidates");
    match action {
        "open" => open_candidate(&root, arguments, &directory),
        "verify" => candidate_actions::verify(&root, arguments, &directory),
        "land" => candidate_actions::land(&root, arguments, &directory),
        "status" => candidate_actions::status(arguments, &directory),
        "discard" => candidate_actions::discard(arguments, &directory),
        // Recording a verdict is the host's job, not the agent's: a tool that can write a pass can
        // write its own. The host review entry prints the packet and records against its digest.
        // 记录判定是宿主的职责，不是 agent 的：能写"通过"的工具就能写自己的"通过"。宿主的评审入口
        // 打印材料包，并对照它的摘要记录。
        "review" => Err(
            "recording a review is not a tool action: a tool that writes a verdict can write its own pass. The host review entry prints the packet the reviewer works from — `xirang-mcp --review --root <base> --id <candidate>` — and records `--verdict` against `--packet <digest>` from the same entry; this surface can only print the packet (`status` does)"
                .to_owned(),
        ),
        "propose" => scope::propose(arguments, &directory),
        "widen" => scope::widen(&root, arguments, &directory),
        _ => unreachable!("the action list was checked above"),
    }
}

/// Take a copy of the tree, record the base's identity, and hand back the candidate's id.
/// 取一份树的副本、记下基树的身份，并交回候选的 id。
fn open_candidate(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    let scope = arguments
        .get("scope")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty() && rows.len() <= MAX_SCOPE)
        .ok_or_else(|| {
            format!(
                "candidate open needs `scope`: 1..{MAX_SCOPE} package-relative path prefixes the change may touch; a candidate with no scope could only be discarded"
            )
        })?;
    let mut prefixes = Vec::new();
    for row in scope {
        let prefix = row
            .as_str()
            .filter(|prefix| {
                !prefix.is_empty()
                    && prefix.len() <= 512
                    && !prefix.starts_with('/')
                    && !Path::new(prefix)
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
            })
            .ok_or_else(|| {
                "every scope entry must be a package-relative path prefix without `..`".to_owned()
            })?;
        prefixes.push(prefix.trim_end_matches('/').to_owned());
    }
    let excluded = arguments
        .get("target_dir")
        .and_then(Value::as_str)
        .map(|value| root.join(value));
    let id = super::check_jobs::new_id();
    let slot = directory.join(&id);
    let tree = slot.join("tree");
    std::fs::create_dir_all(&slot).map_err(|error| error.to_string())?;
    super::check_variant::copy_tree(root, &tree, excluded.as_deref())?;
    // EX13: a candidate is a copy, so a relative `path` dependency that pointed beside the base
    // (../external-dep) no longer resolves: from the copy it would point inside the candidate slot.
    // The base's layout is reproduced **outside** the copied tree with links to the same directories,
    // so the build reads the identical bytes and nothing in the base's own manifest is rewritten.
    // EX13：候选是一份副本，因此指向基树旁边的相对 `path` 依赖（../external-dep）不再解析得到：从副本出发它
    // 会指到候选槽内部。这里在**被复制的树之外**用指向同一批目录的链接重建基树的布局，于是构建读到同样的字节，
    // 而基树自己的清单一个字也没有被改写。
    let dependency_links = link_external_dependencies(root, &tree)?;
    // **The scope a candidate is opened with has a person behind it, and that person is read, not
    // asserted** (maintainer, 2026-10-18: 「一开始需要让 ai 依据提示词先选定修改范围，然后人确认后…
    // 越权的不要出现」). `open` therefore takes an optional `authority`: the adoption anchor whose
    // `verifier` confirmed this write set. Absent it the candidate may still be opened — looking at a
    // change is not a write — but it cannot land, and landing is where a write set with nobody's
    // signature would otherwise appear.
    // **开一棵候选所用的范围背后有一个人，而那个人是**读出来的**，不是断言出来的**（维护者 2026-10-18）。
    // `open` 因此接受可选的 `authority`；没有它仍可开候选，但不能落地。
    let confirmed_by: Value = match arguments.get("authority").and_then(Value::as_str) {
        None => Value::Null,
        Some(authority) => {
            let entry = super::adopted::entries(root)?
                .into_iter()
                .find(|entry| entry.anchor == authority)
                .ok_or_else(|| {
                    format!(
                        "no adoption in this tree names the anchor `{authority}`; a scope is confirmed by a person through a lease somebody recorded, not by this request"
                    )
                })?;
            let verifier = entry.verifier.trim().to_owned();
            if verifier.is_empty() {
                return Err(format!(
                    "the adoption entry `{authority}` names nobody as its verifier, so it is not a person's confirmation of this scope"
                ));
            }
            json!(verifier)
        }
    };
    let base_before = super::check_task::Inputs::capture(root, excluded.as_deref())?.digest();
    // §8.3 binds a graft candidate to five facts, and the binding is taken **here**, when the copy is
    // still byte-identical to the base: that is the only moment the two sides of the comparison are
    // known to be the same tree. `land` re-reads both sides; this record is what it compares against.
    // §8.3 要求 graft 候选绑定五件事，而这份绑定在**这里**取——此时副本与基树逐字节相同：那是比较的两端被
    // 确知为同一棵树的唯一时刻。`land` 会重读两端，而这条记录就是它据以比较的东西。
    let configuration = graft::configuration_of(root);
    let graft = json!({
        "policy": graft::POLICY,
        "configuration": configuration,
        "at_open": {
            "base": graft::binding(root, &configuration),
            "candidate": graft::binding(&tree, &configuration),
        },
    });
    // §7: an independent review that the policy (or the person who opened this candidate) **requires**
    // is an obligation, not an option. It is recorded at open — before any work exists — because a
    // requirement added after the fact is a requirement shaped by what the work turned out to be.
    // §7：政策（或开这棵候选的人）**要求**的独立评审是义务而不是选项。它在 `open` 时记下——在任何工作存在之前
    // ——因为事后加上的要求，是被"工作恰好长成什么样"塑造出来的要求。
    let require_review = arguments
        .get("require_review")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // §7: a reviewer who is not told what the work was asked to do can only review the author's
    // account of it, which is the same-source blind spot the review exists to break. So the request
    // (and the criteria, when they are separate) is recorded with the candidate, before the copy is
    // worked on, and every packet carries it.
    // §7：没人告诉评审者这件活被要求做什么时，他只能评审作者的说法——那正是评审要打破的同源盲区。
    // 因此需求（以及单独给出的验收标准）在任何工作开始之前就与候选一起记下，每一份材料包都带着它。
    let requirement = arguments
        .get("requirement")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let acceptance: Vec<&str> = arguments
        .get("acceptance")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if require_review && requirement.is_none() {
        return Err(
            "`require_review: true` needs `requirement`: a required review is judged against the request as the person stated it, and a packet without it leaves the reviewer only the author's account — the blind spot the review exists to break. Pass `requirement` (and `acceptance` when the criteria are separate)"
                .to_owned(),
        );
    }
    // The boundary a person protects can be smaller than a file (EX03), and removing an existing
    // test is the other way to pass by weakening the criteria (EX11). Both are recorded here, before
    // any work exists, and both are judged later from the **actual diff**.
    // 人保护的东西可以比一个文件更小（EX03），而删掉既有测试是另一种"靠削弱判据通过"（EX11）。两者都在这里
    // ——任何工作存在之前——记下，之后都由**实际差异**判定。
    let protected: Vec<Value> = arguments
        .get("protect")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let path = row.get("path").and_then(Value::as_str)?.trim().to_owned();
                    let entity = row.get("entity").and_then(Value::as_str)?.trim().to_owned();
                    (!path.is_empty() && !entity.is_empty())
                        .then(|| json!({"path": path, "entity": entity}))
                })
                .collect()
        })
        .unwrap_or_default();
    if protected.len() > MAX_SCOPE {
        return Err(format!("`protect` names at most {MAX_SCOPE} entities"));
    }
    for row in &protected {
        let path = row["path"].as_str().unwrap_or_default();
        if path.starts_with('/')
            || Path::new(path)
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(
                "every `protect` entry names a package-relative path without `..`".to_owned(),
            );
        }
    }
    // A candidate cannot allow itself to remove a test: that allowance belongs to the host's landing
    // entry (2026-10-19 review). The key is refused rather than ignored, so the caller learns it moved.
    // 候选不能自己允许自己删除测试：那份允许属于宿主的落地入口（2026-10-19 复核）。这里拒绝该键而不是静默
    // 忽略，让调用方知道它搬走了。
    if arguments.get("allow_removed_tests").is_some() {
        return Err(
            "`allow_removed_tests` is not a candidate argument: the allowance belongs to the host's landing entry (`--confirm --land --allow-removed-test <path>::<name>`), because a candidate that can allow itself can weaken its own criteria"
                .to_owned(),
        );
    }
    let record = Record {
        path: directory.join(format!("{id}.json")),
        body: json!({
            "id": id,
            "base_root": root.display().to_string(),
            "tree": tree.display().to_string(),
            "build": slot.join("build").display().to_string(),
            "scope": prefixes,
            "base_at_open": base_before,
            "opened_at": now(),
            "opened_by": super::check_jobs::owner(),
            "scope_confirmed_by": confirmed_by,
            // The three facts a grant is bound to that the candidate itself owns. `proposal` is the scope
            // proposal's revision (a widening raises it), `task` is the task this candidate belongs to, and
            // `grant_revision` is the approval revision this candidate currently relies on.
            // 候选自己拥有的、授权所绑定的三个事实。`proposal` 是范围提案的版本（一次扩大会抬高它），`task` 是这棵候选
            // 所属的任务，`grant_revision` 是这棵候选当前所依赖的批准版本。
            "proposal": "1",
            "task": "candidate",
            "grant_revision": "0",
            "verification": Value::Null,
            "graft": graft,
            "require_review": require_review,
            "requirement": requirement,
            "acceptance": acceptance,
            "corrections": Vec::<String>::new(),
            "protect": protected,
            "dependency_links": dependency_links,
            "state": "open",
        }),
    };
    let graft_line = graft::line(&graft["at_open"]["candidate"], "at open");
    record.publish()?;
    scope::note_history(
        directory,
        &json!({"kind": "authorised", "candidate": id, "scope": prefixes}),
    )?;
    Ok(format!(
        "candidate {id} open\nrecord {}\ntree {}\nscope {}\nbase_at_open {}\n{graft_line}\nthe base tree was copied and not written; `verify` runs inside the copy, and nothing reaches {} until `land` says the scope covers every changed path\n",
        record.path.display(),
        tree.display(),
        prefixes.join(", "),
        base_before,
        root.display(),
    ))
}

/// Reproduce the base tree's out-of-root relative `path` dependencies beside the copied tree.
/// 在被复制的树旁边重建基树的根外相对 `path` 依赖。
///
/// Only relative spellings need this: an absolute `path` names the same directory on both sides. A
/// link is created at the location the copy's own manifest spells (`../external-dep` from
/// `<slot>/tree` lands at `<slot>/external-dep`), and the mapping is recorded so the traversal is on
/// the record rather than in somebody's memory. A spelling whose destination would leave the slot is
/// refused: a candidate may not reach outside the slot it was given.
/// 只有相对拼写需要这一步：绝对 `path` 在两端点名同一个目录。链接建在副本自己清单拼写出的位置（从
/// `<slot>/tree` 看 `../external-dep` 落在 `<slot>/external-dep`），映射写进记录，于是这次映射在记录上，
/// 而不在某人的记忆里。目的地会离开槽的拼写被拒绝：候选不得伸到它拿到的槽之外。
fn link_external_dependencies(root: &Path, tree: &Path) -> Result<Vec<Value>, String> {
    let slot = tree
        .parent()
        .ok_or("the candidate tree has no slot beside it")?;
    let mut manifests = Vec::new();
    walk_manifests(tree, &mut manifests)?;
    let mut links = Vec::new();
    let mut unsupported = Vec::new();
    for manifest in manifests {
        let Some(directory) = manifest.parent() else {
            continue;
        };
        let Ok(relative_directory) = directory.strip_prefix(tree) else {
            continue;
        };
        let base_directory = root.join(relative_directory);
        for (spelling, _resolved) in crate::mcp::check_task::path_dependencies(&manifest) {
            if Path::new(&spelling).is_absolute() {
                continue;
            }
            let Ok(original) = base_directory.join(&spelling).canonicalize() else {
                // A dependency that does not resolve is Cargo's refusal to make, not ours.
                continue;
            };
            if original.starts_with(root) {
                // Inside the base tree: the copy already has it at the same relative location.
                continue;
            }
            let destination = normalize(&directory.join(&spelling));
            if !destination.starts_with(slot) {
                // The review's R5 class is "a legal relative dependency". A spelling that climbs past the
                // slot cannot be mirrored inside one candidate's own directory without claiming a
                // neighbour's name, so it is recorded as unsupported instead of refusing the open: the
                // candidate still opens, and the build that needs it fails rather than being silently
                // certified against a dependency it did not read.
                // 复核的 R5 类别是"合法的相对依赖"。越过候选槽的拼法无法在单个候选自己的目录里镜像，否则会占用
                // 邻居的名字；因此记成"不支持"而不在 open 时拒绝：候选仍能打开，需要它的构建会失败，而不是被
                // 静默地拿一条它没读过的依赖来认证。
                unsupported.push(json!({
                    "manifest": manifest.display().to_string(),
                    "spelling": spelling,
                    "reason": "a relative dependency that climbs past the candidate's slot cannot be mirrored inside it",
                }));
                continue;
            }
            if destination.exists() {
                continue;
            }
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(&original, &destination).map_err(|error| {
                format!(
                    "cannot point {} at {}: {error}",
                    destination.display(),
                    original.display()
                )
            })?;
            #[cfg(not(unix))]
            {
                return Err(
                    "a candidate's relative path dependencies need links on this platform"
                        .to_owned(),
                );
            }
            links.push(json!({
                "manifest": manifest.display().to_string(),
                "spelling": spelling,
                "base": original.display().to_string(),
                "link": destination.display().to_string(),
            }));
        }
    }
    if !unsupported.is_empty() {
        links.push(json!({"unsupported": unsupported}));
    }
    Ok(links)
}

/// Every `Cargo.toml` under one directory, skipping the build products and the tool caches.
/// 一个目录下的每一份 `Cargo.toml`，跳过构建产物与工具缓存。
fn walk_manifests(directory: &Path, manifests: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        if ["target", ".git", ".codegraph", ".direnv"]
            .iter()
            .any(|skip| name == *skip)
        {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            walk_manifests(&path, manifests)?;
        } else if name == "Cargo.toml" {
            manifests.push(path);
        }
    }
    Ok(())
}

/// Collapse `.`/`..` lexically, without touching the filesystem.
/// 词法折叠 `.`/`..`，不碰文件系统。
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// What happened to one path between the base and the copy.
/// 一条路径在基树与副本之间发生了什么。
pub(super) enum Change {
    /// The copy has the file and the base does not, or they differ.
    /// 副本有这个文件而基树没有，或两者不同。
    Written,
    /// The base has the file and the copy does not.
    /// 基树有这个文件而副本没有。
    Gone,
}

/// Every path the copy changed relative to the base, by content rather than by timestamp.
/// 副本相对基树改动的每一条路径，按内容而不是按时间戳。
///
/// Content is what decides: a restored mtime, a touched file with the same bytes and a same-second edit
/// are all the same fact here, and only the first two are "no change".
/// 决定它的是内容：被恢复的时间戳、改了时间但字节相同的文件、同一秒内的编辑，在这里都是同一个事实，而前两者
/// 是"没有变化"。
pub(super) fn delta(tree: &Path, base: &Path) -> Result<BTreeMap<String, Change>, String> {
    let in_copy = hashed(tree)?;
    let in_base = hashed(base)?;
    let mut delta = BTreeMap::new();
    for (path, digest) in &in_copy {
        if in_base.get(path) != Some(digest) {
            delta.insert(path.clone(), Change::Written);
        }
    }
    for path in in_base.keys() {
        if !in_copy.contains_key(path) {
            delta.insert(path.clone(), Change::Gone);
        }
    }
    Ok(delta)
}

/// The base root a record was opened against, so a delta compares the copy with what it was copied from.
/// 一条记录当初据以打开的基树根，使差异是与"它从哪复制来"比较。
/// How many files a landing of this candidate would publish, when both trees are still readable.
/// 这份候选落地会发布多少个文件（两棵树都仍可读时）。
///
/// `None` means the count could not be measured: a caller must then leave the file limit unchecked rather than compare
/// it against zero, because a limit compared against a number nobody measured is not a limit.
/// `None` 表示数不出来：调用方此时必须**不检查**文件上限，而不是拿零去比——拿没人量过的数去比的上限不是上限。
pub(super) fn changed_files(record: &Record) -> Option<u64> {
    let tree = record.tree().ok()?;
    let base = tree_root_of(record).ok()?;
    Some(delta(&tree, &base).ok()?.len() as u64)
}

pub(super) fn tree_root_of(record: &Record) -> Result<PathBuf, String> {
    record.body["base_root"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| "the record names no base root".to_owned())
}

/// Every regular file under one root, as relative path → content digest, skipping build directories.
/// 一个根之下的每个常规文件，按相对路径 → 内容摘要，跳过构建目录。
fn hashed(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if ["target", ".git", ".codegraph", ".direnv"]
                .iter()
                .any(|skip| name == *skip)
            {
                continue;
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| "a file escaped the root it was read under".to_owned())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
                out.insert(relative, xirang_kernel::sha256_hex(&bytes));
            }
        }
    }
    Ok(out)
}

/// Whether one changed path is covered by a declared scope.
/// 一条改动路径是否落在声明的范围内。
///
/// A prefix names a directory when it matches, or itself: `src` covers `src/lib.rs` and `src` but not
/// `src-other/lib.rs`, which is the difference between a scope and a string prefix.
/// 前缀在相等或以它开头并跟一个分隔符时纳入了它：`src` 覆盖 `src/lib.rs` 与 `src`，但不覆盖
/// `src-other/lib.rs`——这正是"范围"与"字符串前缀"的区别。
pub(super) fn in_scope(path: &str, scope: &[String]) -> bool {
    scope
        .iter()
        .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")))
}

/// The wall clock a record stamps, in nanoseconds since the epoch.
/// 记录盖下的墙上时钟，自纪元起的纳秒数。
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// The tool's own description, kept next to the surface it describes rather than in the catalogue
/// table: a description that lives away from the handler is one of the two things that drift.
/// 工具自己的描述，放在它所描述的执行面旁边而不是目录表里：离开处理函数的描述正是会漂移的两样东西之一。
pub(super) const DESCRIPTION: &str = "Managed candidate: make a change and verify it where the base tree is not, then land only what a declared scope covers. \
             `action` is required, because the default would have to be either a write or a read and a write must not be reachable by omitting an argument. \
             `open` copies the tree to `<target_dir>/xirang/candidates/<id>/tree` and records the base's own input identity; nothing in the base is written. \
             `verify` runs a check request **inside the copy** — the same arguments `xirang.check` takes, minus `files`/`cases`/`plan_id` — and binds the verdict to the copy's current bytes. \
             `land` publishes the changed, in-scope files into the base and refuses when any changed path is outside `scope`, when the base moved since `open`, or when the verification is not bound to the candidate's current bytes; the refusal names every outside path and writes nothing. \
             `status` reads the record back without running anything; `discard` removes the copy and leaves the record marked discarded, so a discard is never silent. \
             `open` also binds the graft plan (targets, replacement, rule version, configuration, effective projection) and `land` re-reads it on **both** sides: a landing that would leave two declarations competing for one slot is refused with both of them named and writes nothing, and a projection that moved is printed as an old→new object set. \
             `land` needs an **approval the controller issued** (a grant bound to this candidate, this proposal revision, these paths and this operation); `widen` needs the same approval with `widen` in its operation set, applies the widened scope to the candidate and clears the verification because the obligations grew. \
             Every landing and status reply prints the capability it runs at: **plain-mcp** checks what it lands and cannot prevent another tool or process from writing the base tree — the stronger claim belongs to managed execution, which this bridge does not implement. \
             **This bounds what this tool lands, not what a process can reach**: `verify` runs the same Cargo command as `check`, with the copy as its root — it is not a sandbox. A landing is a transaction over files: the old bytes and their digests are recorded before the first byte moves, a failure rolls back exactly what this landing wrote, and a stopped landing is finished or undone by the next one under a per-candidate lock. The guarantee is about a process that stops, not about a machine that does.";

/// The advertised input contract.
/// 广告出去的输入契约。
pub(super) fn schema() -> Value {
    json!({"type":"object","properties":{
                "action":{"type":"string","enum":["open","verify","propose","widen","land","status","discard"],"description":"Required. `open`/`land`/`discard` write; `verify` runs Cargo in the copy; `status`/`propose` only read. There is no `review` action: a verdict comes from the host review entry, against the packet `status` prints."},
                "scope":{"type":"array","maxItems":64,"items":{"type":"string"},"description":"`open`: the package-relative path prefixes the change may touch, 1..64 entries. A changed path outside all of them refuses the landing by name; `src` covers `src/lib.rs` and `src`, but not `src-other/lib.rs`."},
                "id":{"type":"string","description":"The candidate id `open` printed; required by verify/land/status/discard."},
                "authority":{"type":"string","description":"`open`: the adoption anchor whose verifier (a person) confirmed this write set. Without it a candidate can be opened and verified but not landed."},
                "require_review":{"type":"boolean","description":"`open`: make an independent review a blocking obligation — `land` refuses until the host review entry records a `pass` against the packet as it stands (§7). Needs `requirement`."},
                "requirement":{"type":"string","description":"`open`: the request as the person stated it — the review packet carries it verbatim, and it is required when `require_review` is true."},
                "acceptance":{"type":"array","items":{"type":"string"},"description":"`open`: the acceptance criteria when they are separate from the request; every packet lists them."},
                "protect":{"type":"array","maxItems":64,"items":{"type":"object","properties":{"path":{"type":"string"},"entity":{"type":"string"}},"required":["path","entity"]},"description":"`open`: entities that must keep their bytes even when their file is in scope (EX03). `verify`/`land` judge the actual diff: a changed protected entity refuses, and an entity locatable on only one side is `unknown` rather than a pass."},
                "allow_removed_tests":{"type":"array","items":{"type":"string"},"description":"Not a `open` argument: see the host landing entry. A candidate that could allow itself to remove a test could weaken its own criteria, so the allowance is recorded through `--confirm --land --allow-removed-test`."},
                "verdict":{"type":"string","enum":["pass","fail"],"description":"`review`: what the reviewer concluded."},
                "reviewer":{"type":"string","description":"`review`: who reviewed it. A declaration, not proof of a separate session."},
                "mode":{"type":"string","enum":["test","check","build","clippy"],"default":"test"},
                "profile":{"type":"string","enum":["debug","release"],"default":"debug"},
                "target":{"type":"string","description":"Cargo target triple."},
                "all_targets":{"type":"boolean"},
                "deny_warnings":{"type":"boolean","description":"Clippy -D warnings."},
                "workspace":{"type":"boolean"},
                "package":{"type":"string"},
                "test":{"type":"string"},
                "filter":{"type":"string"},
                "no_default_features":{"type":"boolean"},
                "face":{"type":"string","description":"`default`, `all`, or Cargo features; passed to the verification inside the copy."},
                "timeout_ms":{"type":"integer","minimum":1000,"maximum":3600000},
                "target_dir":{"type":"string","description":"Base for the candidate slots and the copy's build directory; the base tree's own identity is taken with this directory excluded."},
                "root":{"type":"string"}
            },"required":["action"]})
}

#[path = "candidate_scope.rs"]
pub(super) mod scope;

#[path = "candidate_review.rs"]
pub(super) mod review;

#[path = "candidate_graft.rs"]
pub(super) mod graft;

#[path = "candidate_actions.rs"]
mod candidate_actions;

#[cfg(test)]
#[path = "candidate_tests.rs"]
mod candidate_tests;

#[cfg(test)]
#[path = "candidate_faults_tests.rs"]
mod candidate_faults_tests;

#[path = "candidate_landing.rs"]
mod landing;

#[path = "candidate_guard.rs"]
mod guard;

#[path = "candidate_approval.rs"]
mod approval_view;

#[cfg(test)]
#[path = "candidate_approval_tests.rs"]
mod candidate_approval_tests;

#[cfg(test)]
#[path = "candidate_guard_tests.rs"]
mod candidate_guard_tests;
