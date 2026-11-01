//! Where an approval lives, and why no tool in this bridge can put one there.
//! 批准住在哪里，以及为什么这座桥里没有任何工具能往那里放一条。
//!
//! The maintainer's ruling (2026-10-17): a **local trusted controller** issues authorizations; the agent may
//! apply for a scope and read the state, never approve. This module is the store's side of that split. Two
//! properties carry it, and both are structural rather than documentary.
//! 维护者的裁定（2026-10-17）：由**本地可信控制器**签发授权；agent 可以申请范围、可以读状态，永远不批准。本模块是
//! 那个分工里"存储"的那一半。两条性质承载它，而两条都是结构性的，不是写在文档里的。
//!
//! **It is outside the managed tree.** A store inside the root would be an input of every verification closure
//! and a candidate's own delta — the approval would then be something the candidate could rewrite, which is the
//! defect this replaces. It sits beside the root, keyed by the root's digest so two projects never share one.
//! **它在被管理树之外。** 落在根里的存储会成为每次验证闭包的输入、也是候选自己的改动增量——那样批准就成了候选能改写的
//! 东西，正是这里要替换掉的缺陷。它住在根的旁边，按根的摘要分目录，因此两个项目永远不共用一份。
//!
//! **No tool writes it.** The write entry [`issue`] is called from the interactive approval front end only; the
//! pin (`no_tool_in_this_bridge_can_issue_an_approval`) scans every other source file of this bridge and fails if
//! the name appears. A rule that only lived in prose would be re-broken by the next handler that wanted to be
//! helpful.
//! **没有任何工具写它。** 写入入口 [`issue`] 只被交互式批准前端调用；钉子（`no_tool_in_this_bridge_can_issue_an_approval`）
//! 扫描本桥其余每一个源码文件，名字一出现就红。只活在散文里的规则，会被下一个想"帮个忙"的处理函数重新破坏。
//!
//! What this does **not** pretend to be: a filesystem access control. A process with write access to the store
//! can forge a grant, exactly as it can write the base tree directly. That is why the plain-MCP capability says
//! so out loud instead of claiming that unauthorized writes are prevented.
//! 它**不**假装自己是文件系统访问控制。能写这个存储的进程可以伪造一份授权，正如它能直接写基树。这正是普通 MCP 能力
//! 必须把这件事说出来、而不是宣称已阻止越权写入的原因。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xirang_kernel::maintenance::Grant;

/// The file name one root's active authorization lives under.
/// 一个根的当前授权所住的那个文件名。
const GRANT_FILE: &str = "grant.json";

/// The directory beside the managed root that holds this root's controller state.
/// 被管理根旁边那个存放这个根控制器状态的目录。
///
/// The path is a **sibling** of the managed root and is keyed by the root's portable path digest, so two projects
/// under one parent never share a grant, and a grant can never be mistaken for a file of the project it approves.
/// 该路径是被管理根的**兄弟**，以根的可移植路径摘要作键，因此同一个父目录下的两个项目永不共用授权，而一份授权也永远
/// 不会被误认成它所批准的项目的文件。
///
/// It takes no environment: the ruling put the protection in the **channel** that writes here, not in where the
/// file sits, and a path that moved with `$XDG_STATE_HOME` would make a grant unreadable for no reason a reply
/// could explain.
/// 它不读环境：裁定把保护放在写这里的**通道**上，而不是文件所在的位置；一条随 `$XDG_STATE_HOME` 移动的路径，会让一份
/// 授权没有理由地读不出来，而回复解释不了那个理由。
pub(super) fn directory(root: &Path) -> PathBuf {
    let key = xirang_kernel::sha256_hex(root.to_string_lossy().as_bytes());
    root.parent()
        .unwrap_or(root)
        .join(".xirang-controller")
        .join(key)
}

/// The active authorization for this root, if the controller has issued one.
/// 这个根当前的授权；如果控制器签发过一份的话。
///
/// Three answers, and they are three different facts: `Ok(None)` is "nobody approved anything yet", `Ok(Some)`
/// is one approval, and `Err` is "there is a store here that cannot be read". Folding the third into the first is
/// how a corrupted or half-written store would read as "no approval" — which is safe for landing but wrong for the
/// reader who wants to know why.
/// 三个答案，三个不同的事实：`Ok(None)` 是"还没有人批准过任何东西"，`Ok(Some)` 是一份批准，`Err` 是"这里有一份读不出来
/// 的存储"。把第三种折进第一种，正是"损坏或写了一半的存储"被读成"没有批准"的方式——对落地来说安全，对想知道原因的读者来说错。
pub(super) fn active(root: &Path) -> Result<Option<Grant>, String> {
    let path = directory(root).join(GRANT_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "cannot read the approval at {}: {error}",
                path.display()
            ));
        }
    };
    let stored: Value = serde_json::from_str(&text).map_err(|error| {
        format!(
            "{}: the approval is not readable JSON ({error})",
            path.display()
        )
    })?;
    let fields: Vec<(String, String)> = stored["fields"]
        .as_array()
        .ok_or_else(|| format!("{}: the approval carries no dimensions", path.display()))?
        .iter()
        .map(|row| {
            let pair = row.as_array().filter(|pair| pair.len() == 2);
            match pair.and_then(|pair| Some((pair[0].as_str()?, pair[1].as_str()?))) {
                Some((name, value)) => Ok((name.to_owned(), value.to_owned())),
                None => Err(format!(
                    "{}: a dimension is not a `[name, value]` pair",
                    path.display()
                )),
            }
        })
        .collect::<Result<_, _>>()?;
    let grant = Grant::issue(fields).map_err(|error| format!("{}: {error}", path.display()))?;
    // The digest is re-derived here rather than trusted from the file: a stored digest is a claim, and the
    // grant's identity has to be the fold of what it actually binds.
    // 摘要在这里重新推导而不是从文件里信任：存下来的摘要是主张，而授权的身份必须是它实际绑定的那些维度的折叠。
    if let Some(recorded) = stored["digest"].as_str()
        && recorded != grant.digest()
    {
        return Err(format!(
            "{}: the stored digest does not describe these dimensions, so this approval was edited after it was issued",
            path.display()
        ));
    }
    Ok(Some(grant))
}

/// Write one approval. **Called by the interactive front end only** — see the module doc and its pin.
/// 写下一条批准。**只被交互式前端调用**——见模块文档与它的钉子。
///
/// The three facts the caller supplies are the three a terminal interaction produces: which dimensions were
/// approved, who said so, and through which channel. Everything else — the digest, the file, the write-then-rename
/// — is the store's job.
/// 调用方给的三样，正是一次终端交互产出的三样：批准了哪些维度、谁说的、经哪个通道。其余（摘要、文件、先写后换名）是
/// 存储的职责。
pub(super) fn issue(
    root: &Path,
    fields: Vec<(String, String)>,
    approver: &str,
    channel: &str,
) -> Result<Grant, String> {
    let mut dimensions = fields;
    dimensions.push(("approver".to_owned(), approver.to_owned()));
    dimensions.push(("channel".to_owned(), channel.to_owned()));
    let grant = Grant::issue(dimensions)?;
    let directory = directory(root);
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let path = directory.join(GRANT_FILE);
    let body = json!({"fields": grant.fields(), "digest": grant.digest()});
    let temporary = path.with_extension("xirang-approval");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("cannot stage {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))?;
    Ok(grant)
}

/// The boundary the host declared for one landing: entities a requirement protects and tests the
/// person allowed to disappear. It lives beside the approval because it is the same kind of record —
/// written by the host's confirmation entry, never by the tool surface.
/// 宿主为一次落地声明的边界：需求保护的实体，以及被允许消失的测试。它放在批准旁边，因为它是同一类记录——
/// 由宿主的确认入口写入，工具面从不写它。
pub(super) const BOUNDARY_SUFFIX: &str = ".boundary.json";

/// The boundary on record, or an empty one. Missing means "the host declared nothing", which the
/// landing entry reports as such rather than as "the candidate declared it".
/// 记录在案的边界，或一份空的。缺失意味着"宿主什么都没声明"，落地入口会照此报告，而不是当成"候选声明了它"。
pub(super) fn boundary(root: &Path, id: &str) -> Result<serde_json::Value, String> {
    let path = directory(root).join(format!("{id}{BOUNDARY_SUFFIX}"));
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"protect": [], "allow_removed_tests": []}));
        }
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let stored: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(json!({
        "protect": stored["protect"].as_array().cloned().unwrap_or_default(),
        "allow_removed_tests": stored["allow_removed_tests"].as_array().cloned().unwrap_or_default(),
    }))
}

/// Record the boundary the host's confirmation entry was shown. Atomic for the same reason the grant is:
/// a half-written boundary would name a boundary nobody approved.
/// 记下宿主确认入口看到的那条边界。原子性与批准同因：写了一半的边界会点名一条没人批准过的边界。
pub(super) fn note_boundary(
    root: &Path,
    id: &str,
    protect: &[String],
    allow_removed_tests: &[String],
) -> Result<(), String> {
    let directory = directory(root);
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let path = directory.join(format!("{id}{BOUNDARY_SUFFIX}"));
    let body = json!({"protect": protect, "allow_removed_tests": allow_removed_tests});
    let temporary = path.with_extension("xirang-boundary");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("cannot stage {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))
}

/// One line a reply can print: what is approved, by whom, through which channel.
/// 回复可以印的一行：批准了什么、谁批的、经哪个通道。
pub(super) fn line(grant: &Grant) -> String {
    format!(
        "approval: {} approved by {} through {}{}",
        grant.field("scope").unwrap_or("nothing"),
        grant.field("approver").unwrap_or("nobody"),
        grant.field("channel").unwrap_or("no channel"),
        match grant.field("budget") {
            Some(budget) => format!(", budget {budget}"),
            None => String::new(),
        },
    )
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod controller_tests;
