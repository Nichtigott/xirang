//! The candidate actions that touch the base tree or run something: verify, land, status, discard.
//! 会碰基树或会运行东西的候选动作：verify、land、status、discard。
//!
//! They live apart from [`super`] so that the dispatch, the record vocabulary and the tree arithmetic stay
//! readable in one file, and so that one action's fault-injection seam (`landing`) does not have to be
//! read through four neighbours.
//! 它们与 [`super`] 分开，是为了让分派、记录词汇与树算术留在同一个可读文件里，也让某个动作的故障注入缝
//! (`landing`) 不必穿过四个邻居才读得到。
//!
//! Nothing here is reachable from a tool call by itself: [`super::candidate`] is the only dispatch, and it
//! passes the arguments it already validated.
//! 这里的任何东西都不单独从工具调用可达：[`super::candidate`] 是唯一分派处，它传下自己已经校验过的参数。

use super::*;
use crate::mcp::candidate::{delta, now};

/// §8.2 step 5: does the verdict this candidate carries still describe what the base holds **now**?
/// §8.2 第 5 步：这份候选携带的判定，还描述着基树**现在**持有的东西吗？
///
/// The verdict was bound to a verification identity — files, environment, whole environment — computed over the copy.
/// After landing, the base is captured the same way: if the two identities are equal, the verdict covers what landed,
/// and that is a fact rather than a promise about filenames. If they are not, the record keeps the obligation
/// outstanding and names the command that would discharge it, instead of letting a landing read as verified.
/// 判定绑定在一个验证身份上——文件、环境、整个环境——它是在副本上算出来的。落地之后，基树以同样方式取一次：两个身份
/// 相等，判定就覆盖了实际落地的东西，而这是一条事实，不是关于文件名的承诺。不相等时，记录把义务留成未完成并点名能结清
/// 它的命令，而不是让一次落地被读成已验证。
pub(super) fn landing_evidence(
    inputs: &crate::mcp::check_task::Inputs,
    record: &mut Record,
) -> String {
    let identity = inputs.verification_digest();
    let recorded = record.body["verification"]["candidate"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let reused = !recorded.is_empty() && recorded == identity;
    record.body["post_landing"] = json!({
        "at": now(),
        "base_verification": identity,
        "candidate_verification": recorded,
        "reused_candidate_evidence": reused,
        "obligation": if reused {
            "none: the landed tree's verification identity is the identity the verdict was bound to"
        } else {
            "run `xirang.check` on the base tree with the request this candidate verified; the verdict on record was bound to another identity"
        },
    });
    if reused {
        format!(
            "post-landing: the base now has the verification identity the verdict was bound to ({identity}), so the candidate's verdict covers what landed"
        )
    } else {
        format!(
            "post-landing: the base's verification identity is {identity}, and the verdict was bound to {recorded}; the evidence does not cover what landed — run `xirang.check` on the base tree with the request this candidate verified"
        )
    }
}

/// Run a verification **inside** the candidate and bind its verdict to the copy's current bytes.
/// 在候选**内部**运行一次验证，并把判定绑定到副本当前的字节上。
pub(super) fn verify(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or("candidate verify needs `id`")?;
    let mut record = Record::read_at(directory, id)?;
    if record.body["state"] == "landed" {
        return Err("this candidate already landed; open a new one for a further change".into());
    }
    let tree = record.tree()?;
    // The verification request is the ordinary check request, minus the keys this surface owns.
    let mut request = arguments.clone();
    if let Some(object) = request.as_object_mut() {
        for key in ["action", "id", "scope", "root", "target_dir"] {
            object.remove(key);
        }
    }
    request["census"] = json!(false);
    request["target_dir"] = json!(record.body["build"]);
    crate::mcp::check_request::validate(&request)?;
    // R3: a candidate's verdict has to be a **test** verdict over the whole delta. `mode: build|check|
    // clippy`, a `filter`, or a `test` name would let one passing build (or one selected test) stand in
    // for behaviour that never ran, which is exactly the bypass the 2026-10-19 review reproduced.
    // R3：候选的判定必须是覆盖整份差异的**测试**判定。`mode: build|check|clippy`、`filter` 或点名一个
    // `test` 会让一次构建通过（或一条被选中的测试）冒充从未运行的行为——正是 2026-10-19 复核复现的绕过。
    if let Some(mode) = request.get("mode").and_then(Value::as_str)
        && mode != "test"
    {
        return Err(format!(
            "a candidate's verdict must run the tests, not `{mode}`; leave `mode` out or name `test`"
        ));
    }
    for key in ["filter", "test", "package"] {
        if request.get(key).is_some_and(|value| !value.is_null()) {
            return Err(format!(
                "a candidate's verification does not take `{key}`: a narrowed run would leave obligations that this candidate's delta creates unexecuted"
            ));
        }
    }
    // The same narrowing can hide behind the build-shaped switches, which is how the 2026-10-19
    // re-review landed a delta whose own test never ran: the suite that "passed" was the default
    // feature set while the changed module was gated behind it.
    // 同样的收窄可以藏在构建形状的开关后面，2026-10-19 复审正是这样落地了一份自己的测试从未运行的差异：
    // "通过"的那套是默认特性，而被改的模块就在那之后门控着。
    if request.get("no_default_features").and_then(Value::as_bool) == Some(true) {
        return Err(
            "a candidate's verification runs the default features: `no_default_features: true` would execute a narrower suite and leave the delta's default-feature behaviour unexecuted"
                .to_owned(),
        );
    }
    if request.get("workspace").and_then(Value::as_bool) == Some(false) {
        return Err(
            "a candidate's verification runs the whole workspace: `workspace: false` would execute only the root package, so a change in a member would be certified by tests that never compiled it"
                .to_owned(),
        );
    }
    // R1: the verdict is bound to the inputs the run **started** on. The 2026-10-19 review reproduced
    // a test that read `data.txt = good`, passed, and then rewrote it to `bad`: capturing after the
    // run certified bytes the run never read, and `land` published them.
    // R1：判定绑定到运行**开始**时的输入。2026-10-19 复核复现了一条测试：读 `data.txt = good` 并通过，随后
    // 把它写成 `bad`——在运行之后捕获，认证的是这次运行从没读过的字节，而 `land` 把它们发布了出去。
    let before = crate::mcp::check_task::Inputs::capture(&tree, None)?;
    let outcome = crate::mcp::check::outcome(&tree, &request)?;
    let after = crate::mcp::check_task::Inputs::capture(&tree, None)?;
    // A closure that could not bind everything cannot certify a reuse. The rule was already written on
    // `Inputs::incomplete` ("never promoted to a reusable credential"); the 2026-10-19 review found that
    // only `check resume` enforced it, while a candidate verified and landed over unbound inputs.
    // 绑定不全的闭包不能认证复用。规则本来就写在 `Inputs::incomplete` 上（"永不升格为可复用凭据"）；
    // 2026-10-19 复核发现只有 `check resume` 执行了它，而候选会在未绑定的输入上验证并落地。
    if let Some(reason) = after.incomplete_reason() {
        record.body["verification"] = json!({
            "candidate": after.verification_digest(),
            "verdict": "unknown",
            "executed": outcome.observation.executed,
            "reason": format!("the input closure is incomplete: {reason}"),
            "at": now(),
        });
        record.publish()?;
        return Err(format!(
            "the candidate's input closure is incomplete: {reason}. Nothing is reusable until every input is bound"
        ));
    }
    // Cargo writes its lockfile as part of the build; that is the run's own product, not a source edit.
    // Cargo 在构建时写它自己的锁文件；那是运行自己的产物，不是源码编辑。
    let during: Vec<String> = before
        .changes(&after)
        .into_iter()
        .filter(|line| !line.ends_with("Cargo.lock"))
        .collect();
    if !during.is_empty() {
        record.body["verification"] = json!({
            "candidate": after.verification_digest(),
            "verdict": "unknown",
            "executed": outcome.observation.executed,
            "reason": "inputs changed while the verification ran",
            "changed_during_run": during,
            "at": now(),
        });
        record.publish()?;
        return Err(format!(
            "the candidate's inputs changed while its verification was running: {}. A run that read one set of bytes and ended on another does not certify the bytes now on disk, so nothing is reusable — run `verify` again on the inputs as they are now",
            during.join(", "),
        ));
    }
    let candidate = after.verification_digest();
    // R3: the consumer obligations `check {files}` reports must bind this verdict too. A candidate
    // that passes its own Cargo test while a consumer entry still has no test is the "the tool can
    // report the gap, the delivery is not bound by it" failure the 2026-10-19 review reproduced.
    // R3：`check {files}` 报出的消费者义务也必须约束这份判定。候选自己那条 Cargo 测试通过、而某个消费者入口
    // 仍没有测试，正是 2026-10-19 复核复现的"工具能报缺口、交付却不受它约束"。
    let delta = delta(&tree, &tree_root_of(&record)?)?;
    let changed: Vec<String> = delta.keys().cloned().collect();
    // R3 (the other half): a delta whose own tests never ran is not covered by a suite that passed
    // elsewhere. A file that declares `#[test]` items must have at least one of them in the
    // executed set, which is the difference between "the build passed" and "the change was tested".
    // R3（另一半）：差异自己的测试从未运行，不能被别处的通过覆盖。声明了 `#[test]` 条目的文件，至少要有
    // 一条出现在已执行集合里——这正是"构建通过"与"改动被测试过"的差别。
    // Only lines the harness printed as `test <name> ... ok|FAILED` describe what **ran**: the
    // stream also carries `... ignored` rows, and treating those as executed is how a skipped test
    // would certify the change it was written for.
    // 只有测试框架印成 `test <name> ... ok|FAILED` 的行才说明**运行过**：流里也带着 `... ignored`
    // 的行，把它们当成已执行，正是"被跳过的测试去认证它本该覆盖的改动"的来路。
    let unexecuted = super::guard::unexecuted(
        &tree,
        &changed,
        &outcome.run.log,
        &outcome.observation.targets,
    );
    if !unexecuted.is_empty() {
        record.body["verification"] = json!({
            "candidate": candidate,
            "verdict": "unknown",
            "executed": outcome.observation.executed,
            "reason": "the delta's own tests did not run",
            "unexecuted": unexecuted,
            "at": now(),
        });
        record.publish()?;
        return Err(format!(
            "the delta changes {}, and none of the tests those files declare ran in this execution; a suite that passed elsewhere does not cover them. Nothing is reusable",
            unexecuted.join(", "),
        ));
    }
    let mut obligations = Vec::new();
    if !changed.is_empty() {
        match crate::mcp::check_protection::census(&tree, &json!(changed)) {
            Ok(Some(census)) => obligations = census.duties,
            Ok(None) => {}
            Err(error) => {
                record.body["verification"] = json!({
                    "candidate": candidate,
                    "verdict": "unknown",
                    "executed": outcome.observation.executed,
                    "reason": format!("the consumer census could not run: {error}"),
                    "at": now(),
                });
                record.publish()?;
                return Err(format!(
                    "the consumer census could not run, so the obligations of this change are unknown: {error}. Nothing is reusable until they can be computed"
                ));
            }
        }
    }
    let obligations_open = obligations
        .iter()
        .filter(|row| row["state"].as_str() != Some("passed"))
        .count();
    // R4: the boundaries that are smaller than a file. Both are judged from the diff; an entity the
    // base declares but the copy no longer has is `unknown`, not a pass.
    // R4：比文件更小的边界。两者都由实际差异判定；基树声明过而副本没有的实体是 `unknown`，不是通过。
    let (host_protect, allowed_removals) = super::guard::host_boundary(root, id)?;
    let touched = super::guard::protected(&record, &tree, &host_protect)?;
    if !touched.is_empty() {
        record.body["verification"] = json!({
            "candidate": candidate,
            "verdict": "unknown",
            "executed": outcome.observation.executed,
            "reason": "protected content changed",
            "protected": touched,
            "at": now(),
        });
        record.publish()?;
        return Err(format!(
            "the change moves protected content: {}. The boundary is unchanged: restore those bytes, or withdraw the protection through the person who asked for it. Nothing is reusable",
            touched.join("; "),
        ));
    }
    let removed = super::guard::changed_tests(&record, &tree, &changed, &allowed_removals)?;
    if !removed.is_empty() {
        record.body["verification"] = json!({
            "candidate": candidate,
            "verdict": "unknown",
            "executed": outcome.observation.executed,
            "reason": "existing tests were removed or weakened",
            "removed_tests": removed,
            "at": now(),
        });
        record.publish()?;
        return Err(format!(
            "the change removes or weakens test(s) that were part of the existing criteria: {}. Adding a regression test and weakening a criterion are different acts; if it is intended, the person who asked for the work names it through the host's landing entry (`--allow-removed-test`). Nothing is reusable",
            removed.join("; "),
        ));
    }
    let verdict = if obligations_open == 0 {
        outcome.verdict.label()
    } else {
        "unmet"
    };
    let rendered = crate::mcp::check::render(&request, &outcome);
    record.body["verification"] = json!({
        "candidate": candidate,
        "verdict": verdict,
        "executed": outcome.observation.executed,
        "obligations_open": obligations_open,
        "obligations": obligations,
        "at": now(),
    });
    record.publish()?;
    Ok(format!(
        "candidate {id} verification {verdict} ({} test(s) executed, {obligations_open} consumer obligation(s) open)\ncandidate {candidate}\n{rendered}\nnothing was written to {}; a land requires this verdict, this candidate digest, and every consumer obligation it reports to still hold\n",
        outcome.observation.executed,
        record.body["base_root"].as_str().unwrap_or("the base tree"),
    ))
}

/// Publish the candidate's in-scope changes into the base tree, or refuse and say which rule refused.
/// 把候选在范围内的改动发布进基树；拒绝时说明是哪条规则拒绝的。
pub(super) fn land(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or("candidate land needs `id`")?;
    let mut record = Record::read_at(directory, id)?;
    let tree = record.tree()?;
    let excluded = arguments
        .get("target_dir")
        .and_then(Value::as_str)
        .map(|value| root.join(value));
    // §8.2: a landing is a transaction over files, so the first thing it does is look at what the **previous**
    // attempt left. This has to happen before the baseline check, because our own part-written change is exactly
    // what that check would read as somebody else's edit — the measured 2026-10-18 failure: the first file
    // published, the second did not, and every retry was refused for a baseline this surface had moved itself.
    // §8.2：落地是一次跨文件事务，因此它做的第一件事是看**上一次**尝试留下了什么。这必须发生在基线检查之前，因为
    // 我们自己写了一半的改动，正是那道检查会读成"别人改的"东西——2026-10-18 实测的失败：第一个文件发布、第二个失败，
    // 之后每次重试都被"基线移动了"拒绝，而移动它的正是这个执行面自己。
    let _holding = landing::lock(directory, id)?;
    let recovered = landing::recover(
        directory,
        id,
        root,
        record.body["base_at_open"].as_str().unwrap_or_default(),
    )?;
    if let Some(landing::Recovered::Published(note)) = &recovered {
        let landed_inputs = crate::mcp::check_task::Inputs::capture(root, excluded.as_deref())?;
        let base_after = landed_inputs.digest();
        let post_landing = landing_evidence(&landed_inputs, &mut record);
        landing::close(directory, id);
        graft::note_write(&mut record.body["graft"], "recovered");
        record.body["state"] = json!("landed");
        record.body["review_at_land"] = record.body["review"].clone();
        record.body["landed_at"] = json!(now());
        record.body["base_at_land"] = json!(base_after);
        record.publish()?;
        return Ok(format!(
            "candidate {id} landed (recovered)\n{note}\nbase {} -> {base_after}\n{post_landing}\nthe landing record said every file was written; nothing was written twice\n",
            record.body["base_at_open"].as_str().unwrap_or("unknown"),
        ));
    }
    let recovered_line = match &recovered {
        Some(landing::Recovered::RolledBack(note)) => format!("\n{note}"),
        _ => String::new(),
    };
    // Three refusals, each naming the rule rather than the symptom, and each checked **before** the
    // first byte of the base is written.
    // 三条拒绝，每条点名规则而不是症状，而且每条都在基树的第一个字节被写之前检查。
    let base_now = crate::mcp::check_task::Inputs::capture(root, excluded.as_deref())?.digest();
    if base_now != record.body["base_at_open"].as_str().unwrap_or_default() {
        return Err(format!(
            "the base tree moved since this candidate was opened ({} -> {base_now}); landing would overwrite work this candidate never saw. Open a new candidate on the current tree",
            record.body["base_at_open"].as_str().unwrap_or("unknown"),
        ));
    }
    let candidate_now = crate::mcp::check_task::Inputs::capture(&tree, None)?.verification_digest();
    let verified = record.body["verification"]["candidate"].as_str();
    if verified != Some(candidate_now.as_str()) {
        return Err(format!(
            "this candidate is not verified at its current bytes: recorded {}, now {candidate_now}. A candidate that changed after its verification is not verified; run `verify` again",
            verified.unwrap_or("nothing"),
        ));
    }
    if record.body["verification"]["verdict"].as_str() != Some("passed") {
        return Err(format!(
            "the last verification of this candidate was `{}`; only a passed verification lands",
            record.body["verification"]["verdict"]
                .as_str()
                .unwrap_or("unknown"),
        ));
    }
    // §7: the review gate is one function for both halves — a failed review blocks, and a required
    // one must be a `pass` on the packet **as it stands**. The packet is the identity, not the
    // verification digest: a verdict is about a review packet (request, criteria, corrections, delta,
    // evidence), so work that moves without moving the `.rs` bytes still leaves the old pass behind.
    // §7：评审门是一道函数管两半——失败的评审阻断，必需的评审必须是**当前这份**材料包上的 `pass`。材料包
    // 才是身份而不是验证摘要：判定针对的是一份材料包（需求、标准、纠正、差异、证据），因此哪怕 `.rs` 字节没动、
    // 只要工作内容动了，旧"通过"就留在原地覆盖不到。
    review::gate(&record)?;
    let delta = delta(&tree, &tree_root_of(&record)?)?;
    let changed: Vec<String> = delta.keys().cloned().collect();
    // R4: the two boundaries smaller than a file, judged from the same diff `land` is about to publish,
    // against the **host's** boundary. A candidate's own `protect` may add protection; it may never
    // remove a protection the host declared, and a candidate's own `allow_removed_tests` is not read.
    // R4：比文件更小的两条边界，用 `land` 即将发布的同一份差异判定，并对照**宿主的**边界。候选自己的
    // `protect` 可以增加保护，但永远不能减少宿主声明的保护，而候选自己的 `allow_removed_tests` 根本不会被读。
    let (host_protect, host_allow) = super::guard::host_boundary(root, id)?;
    let touched = super::guard::protected(&record, &tree, &host_protect)?;
    if !touched.is_empty() {
        return Err(format!(
            "the change moves protected content: {}. The boundary is unchanged: restore those bytes, or withdraw the protection through the person who asked for it. Nothing landed",
            touched.join("; "),
        ));
    }
    let removed = super::guard::changed_tests(&record, &tree, &changed, &host_allow)?;
    if !removed.is_empty() {
        return Err(format!(
            "the change removes or weakens test(s) that were part of the existing criteria: {}. If it is intended, the person who asked for the work names it through the host's landing entry (`--allow-removed-test <path>::<name>`); a candidate's own list cannot authorise itself. Nothing landed",
            removed.join("; "),
        ));
    }
    // R3: a verdict is not a Cargo pass; it is that pass **plus** every consumer obligation of the
    // actual delta. A record written before obligations were part of the verdict is not promoted.
    // R3：判定不是一次 Cargo 通过，而是那次通过**加上**实际差异的每一条消费者义务。在义务进入判定之前写下的
    // 记录不被升级。
    match record.body["verification"]["obligations_open"].as_u64() {
        Some(0) => {}
        Some(open) => {
            return Err(format!(
                "the verification on record leaves {open} consumer protection obligation(s) open; `land` requires every obligation of the actual change to be settled. Nothing landed"
            ));
        }
        None => {
            return Err(
                "the verification on record predates consumer obligations; it cannot say they were settled. Run `verify` again. Nothing landed"
                    .to_owned(),
            );
        }
    }
    // The census is recomputed from the diff `land` is about to publish, so a record that was edited
    // after `verify` (the record lives beside the candidate, inside `target/`, which is excluded from
    // the closure's rows) cannot outvote the actual change. The record's own count is still checked
    // above for the "old record" case; this is the second, independent reading.
    // 消费者普查按 `land` 即将发布的差异**重算**，因此 `verify` 之后被改过的记录（记录住在候选旁边、
    // 在 `target/` 里，而该目录被排除在闭包行之外）无法压过实际改动。上面仍检查记录自己的计数以覆盖
    // "旧记录"的情形；这里是第二次、独立的读数。
    if !changed.is_empty() {
        let duties = match crate::mcp::check_protection::census(&tree, &json!(changed)) {
            Ok(Some(census)) => census.duties,
            Ok(None) => Vec::new(),
            Err(error) => {
                return Err(format!(
                    "the consumer census could not be recomputed at landing, so the obligations of the actual change are unknown: {error}. Nothing landed"
                ));
            }
        };
        let open = duties
            .iter()
            .filter(|row| row["state"].as_str() != Some("passed"))
            .count();
        if open > 0 {
            return Err(format!(
                "the actual change leaves {open} consumer protection obligation(s) open; `land` recomputes them from the diff it is about to publish, so a record cannot outvote the census. Nothing landed"
            ));
        }
        if record.body["verification"]["executed"]
            .as_u64()
            .unwrap_or(0)
            == 0
        {
            return Err(
                "the verification on record executed no test, and this change is not empty: a build, a compile check, or a filtered run cannot stand in for the behaviour this candidate changes. Nothing landed"
                    .to_owned(),
            );
        }
    }
    let scope = record.scope();
    let outside: Vec<&String> = delta
        .keys()
        .filter(|path| !in_scope(path, &scope))
        .collect();
    if !outside.is_empty() {
        // A refusal that only says "no" forces the caller to guess the scope, and a guessed scope is
        // how a change nobody authorized gets landed under a header that looks authorized. The
        // maintainer's correction of 2026-10-18 turned this from a rejection into a **request**: the
        // surface says what is outside, what changed there, what else shares those paths, and what the
        // alternatives are, and it stays `pending` until the host's confirmation entry decides it.
        // 只会说"不"的拒绝逼调用方去猜范围，而猜出来的范围，正是"没人授权过的改动"顶着一个看起来被授权的抬头
        // 落地的方式。维护者 2026-10-18 的纠正把它从拒绝变成一份**请求**：界面说明什么在范围之外、那里改了什么、
        // 还有谁共享这些路径、以及替代方案，并且在宿主确认入口作出决定之前一直保持 `pending`。
        scope::note_history(
            directory,
            &json!({"kind": "attempted", "candidate": id, "refused": outside}),
        )?;
        let request = scope::file_request(&mut record, &tree, &delta)?;
        return Err(format!(
            "{} changed path(s) are outside the declared scope ({}): {}. Nothing landed. This is a request, not a refusal:\n\n{}",
            outside.len(),
            scope.join(", "),
            outside
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            scope::describe(&request, root, id),
        ));
    }
    // The gate the ruling puts here: the write set needs an approval the **controller** issued. Everything a
    // caller can put in this request — the adoption anchor, a `confirm` flag, an environment variable — is a
    // declaration or a locator, and none of them can manufacture a grant. A candidate can therefore be opened
    // and verified by an agent, and only a person at a terminal (or the host's confirmation surface) can move
    // the last step. See `docs/managed-maintenance/authorization-design-2026-10-17.md` §1.
    // 裁定要求的门在这里：这份写集合需要一份**控制器**签发的批准。调用方能写进这次请求的每一样东西——采信锚点、
    // `confirm` 开关、环境变量——都是声明或定位信息，没有一样能造出授权。因此 agent 可以开候选、可以验证，而只有终端前
    // 的人（或宿主的确认界面）能推动最后一步。
    //
    // **2026-10-18 维护者纠正后**，这条拒绝指的不再是"有终端就能批准"：终端与手输姓名都不是有人批准的证明，那条入口
    // 已改成墓碑。现在指的入口是宿主的确认界面（`--confirm`），它展示项目、范围、操作与依据，而不是要求输一个名字。
    let approval = crate::mcp::controller::active(root)?;
    let granted = match &approval {
        None => {
            return Err(format!(
                "no approval is on record for this tree (the controller's store is {}), so nothing lands. The agent that opened this candidate cannot issue one: the host's confirmation entry is `xirang-mcp --confirm --root <path> --id <candidate> --land`, and no tool in this bridge reaches it",
                crate::mcp::controller::directory(root).display()
            ));
        }
        Some(grant) => grant,
    };
    let written: Vec<String> = delta.keys().cloned().collect();
    let project = crate::mcp::registry::namespace(root).unwrap_or_else(|_| "unknown".to_owned());
    let requested = xirang_kernel::maintenance::UseRequest {
        project: &project,
        task: record.body["task"].as_str().unwrap_or("candidate"),
        candidate: id,
        proposal: record.body["proposal"].as_str().unwrap_or("0"),
        revision: record.body["grant_revision"].as_str().unwrap_or("0"),
        operation: "land",
        paths: &written,
        policy: POLICY,
        cost: xirang_kernel::maintenance::Budget {
            files: Some(written.len() as u64),
            executions: Some(1),
        },
    };
    match granted.covers(&requested) {
        xirang_kernel::maintenance::Allowance::Covered => {}
        xirang_kernel::maintenance::Allowance::Refused(why) => {
            return Err(format!(
                "{why}. Nothing lands: an approval has to be issued by the controller for this candidate and this scope, and this request cannot supply one"
            ));
        }
    }
    record.body["approval_at_land"] = json!({
        "digest": granted.digest(),
        "approver": granted.field("approver"),
        "channel": granted.field("channel"),
        "budget": granted.field("budget"),
    });
    let approval_line = crate::mcp::controller::line(granted);
    // The one place a reader is most likely to over-read: a landing that checked scope, verification and approval
    // still did not prevent anybody else from writing the base tree.
    // 读者最可能读过头的那一处：一次检查过范围、验证与批准的落地，仍然阻止不了任何人写基树。
    let capability = crate::mcp::capability::line(root);
    // §8.3's comparison, taken on **both** sides and taken before the first write: a landing that
    // would leave two declarations competing for one slot is refused here, and the record says what
    // was found — which two parties, and that nothing was written. Refusals that happen before any
    // write are the only kind a reader can trust without diffing the tree first.
    // §8.3 的比较，两端都取，而且都在第一次写之前：一次会留下两条声明争同一个槽位的落地在这里被拒，记录里写下
    // 发现了什么——哪两方，以及什么都没写。发生在任何写之前的拒绝，是读者不必先对比树就能相信的唯一一种。
    let grafted = graft::gate(root, &tree, &record.body["graft"]);
    let graft_report = grafted.report.join("\n");
    record.body["graft"] = grafted.record;
    if let Some(refusal) = grafted.refusal {
        record.publish()?;
        return Err(refusal);
    }
    if delta.is_empty() {
        let landed_inputs = crate::mcp::check_task::Inputs::capture(root, excluded.as_deref())?;
        let post_landing = landing_evidence(&landed_inputs, &mut record);
        graft::note_write(&mut record.body["graft"], "nothing");
        record.body["state"] = json!("landed");
        record.body["review_at_land"] = record.body["review"].clone();
        record.body["landed_at"] = json!(now());
        record.body["base_at_land"] = json!(base_now);
        record.publish()?;
        return Ok(format!(
            "candidate {id} landed nothing: the copy is byte-identical to the base. No file was written\n{graft_report}\n{post_landing}\n"
        ));
    }
    // The transaction is written down **before** the first byte of the base moves: the old bytes (so it can undo
    // itself), their digests, the new digests, and the identity of the operation that is about to happen. The
    // journal line stays as the append-only history of attempts; the landing record is the state machine.
    // 事务在基树的第一个字节移动**之前**写下：旧字节（好让它能撤销自己）、它们的摘要、新摘要，以及即将发生的那次操作
    // 的身份。journal 行保留为只追加的尝试历史；落地记录才是状态机。
    let journal = directory.join(format!("{id}.journal.jsonl"));
    crate::mcp::check_task::note(
        &journal,
        json!({"at": now(), "action": "land", "base_at_open": base_now, "paths": delta.keys().collect::<Vec<_>>()}),
    )?;
    let mut transaction = landing::prepare(
        directory,
        id,
        root,
        &tree,
        &delta,
        json!({
            "tool": "candidate",
            "action": "land",
            "candidate": id,
            "task": record.body["task"],
            "proposal": record.body["proposal"],
            "revision": record.body["grant_revision"],
            "approval": record.body["approval_at_land"]["digest"],
            "base_at_open": base_now,
        }),
    )?;
    let (landed, removed) = match landing::publish(&mut transaction, root, &tree) {
        Ok(written) => written,
        Err(refusal) => {
            crate::mcp::check_task::note(
                &journal,
                json!({"at": now(), "action": "land-refused", "stage": "rollback", "why": refusal}),
            )?;
            return Err(refusal);
        }
    };
    let landed_inputs = crate::mcp::check_task::Inputs::capture(root, excluded.as_deref())?;
    let base_after = landed_inputs.digest();
    let post_landing = landing_evidence(&landed_inputs, &mut record);
    // What was actually written is recorded on the graft section too, because "recovery either landed
    // or wrote nothing" is only decidable if the record says which of the two happened.
    // 实际写了什么也记进 graft 段，因为"恢复要么落地要么什么都没写"只有记录说出是哪一种时才可判定。
    graft::note_write(
        &mut record.body["graft"],
        &if landed.is_empty() && removed.is_empty() {
            "nothing".to_owned()
        } else {
            [landed.clone(), removed.clone()].concat().join(", ")
        },
    );
    // The record says `landed` first; only then does the transaction stop being open. A crash between the two
    // leaves a `published` landing record, which the next run finishes rather than repeating.
    // 记录先写 `landed`，之后事务才不再开着。两步之间崩溃会留下 `published` 的落地记录，下一次运行完成它而不是重做。
    record.body["state"] = json!("landed");
    record.body["review_at_land"] = record.body["review"].clone();
    record.body["landed_at"] = json!(now());
    record.body["base_at_land"] = json!(base_after);
    record.publish()?;
    landing::close(directory, id);
    Ok(format!(
        "candidate {id} landed{recovered_line}\nreview {}\nscope {}\n{approval_line}\n{capability}\n{graft_report}\nwrote {}\nremoved {}\nbase {base_now} -> {base_after}\n{post_landing}\neach file landed through a write-then-rename under a landing lock; this was a transaction over {} file(s), recorded at {} before the first byte moved, and a failure rolls back the files it had already written\n",
        match record.body["review_at_land"]["verdict"].as_str() {
            Some(verdict) => format!(
                "{verdict} by {} (a declaration, not proof of a separate session)",
                record.body["review_at_land"]["reviewer"]
                    .as_str()
                    .unwrap_or("an unnamed reviewer")
            ),
            None => "none on record; this landing says nothing about whether anyone reviewed it"
                .to_owned(),
        },
        match record.body["scope_confirmed_by"].as_str() {
            Some(name) => format!(
                "confirmed by {name} as read from the adoption ledger (a declaration, not proof: this \
                 surface can also write that ledger, so the entry saying a person confirmed the scope is \
                 something this process could have written itself)"
            ),
            None => "no confirmation on record".to_owned(),
        },
        if landed.is_empty() {
            "nothing".to_owned()
        } else {
            landed.join(", ")
        },
        if removed.is_empty() {
            "nothing".to_owned()
        } else {
            removed.join(", ")
        },
        transaction.files(),
        transaction.path().display(),
    ))
}

/// Read one candidate's record back without touching anything.
/// 只读地读回一个候选的记录。
pub(super) fn status(arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or("candidate status needs `id`")?;
    let record = Record::read_at(directory, id)?;
    let tree = record.tree()?;
    // A discarded candidate has no bytes to identify, and saying "unknown" would be worse than
    // saying "gone": the record is still readable, and what it has to report is that there is no
    // copy left to compare a verification against.
    // 被丢弃的候选没有字节可供标识，而说"unknown"比说"没了"更糟：记录仍然可读，而它要报的正是"没有副本
    // 可以拿来与验证对照了"。
    let now_identity = if tree.exists() {
        Some(crate::mcp::check_task::Inputs::capture(&tree, None)?.verification_digest())
    } else {
        None
    };
    let verified = record.body["verification"]["candidate"].as_str();
    // The review belongs on the **read** surface, not only on the landing's reply: a candidate can be
    // reviewed and then sit there, and a reader who only asks `status` would otherwise see a
    // verification with no word about whether anyone looked at it. `none` is stated rather than
    // omitted, for the same reason the landing says it: silence must not be readable as approval.
    // 评审属于**读**面，而不只属于落地那次回复：候选可以被评审完然后搁着，而只问 `status` 的读者，否则会看到一次
    // 验证却听不到有没有人看过它。`none` 是写出来的而不是省略的，理由与落地那句相同：沉默不许被读成批准。
    // §7: the packet is what a reviewer works from, so the read surface prints it — the request as
    // the person stated it, the criteria, the corrections, the delta and the evidence that the
    // verdict binds to — instead of the author's summary. A reviewer who has to ask the author for
    // these facts is reviewing the author's account of the work.
    // §7：材料包是评审者据以工作的东西，因此读面印它——按本人措辞记录的需求、验收标准、纠正、差异，以及判定
    // 所绑定的证据——而不是作者的综述。还得向作者索取这些事实的评审者，评的是作者的说法。
    let full = review::status(&record)?;
    let review_block = if record.body["require_review"].as_bool().unwrap_or(false)
        || record.body["review"]["verdict"].as_str().is_some()
    {
        full
    } else {
        full.lines()
            .next()
            .unwrap_or("review none on record; nothing here says anyone reviewed it")
            .to_owned()
    };
    // Built outside the reply's `format!` so the two lines stay one value (clippy's
    // `format_in_format_args` is right: nesting makes the reply harder to read and to change).
    // 在回复的 `format!` 之外构造，好让那两行是一个值（clippy 的 `format_in_format_args` 是对的：嵌套让回复更难读、更难改）。
    let capability_block = format!(
        "{}\n{}",
        crate::mcp::capability::line(Path::new(
            record.body["base_root"].as_str().unwrap_or("the base tree")
        )),
        crate::mcp::capability::missing_for_managed()
    );
    // Ruling §1: the agent may **see** the state, not only request a scope. Until this line existed the authorization
    // only appeared in the reply of the action that used it, so "am I allowed to land this?" could only be answered by
    // attempting a write — which is the one thing a reader must not have to do to find out.
    // 裁定 §1：agent 可以**查看状态**，不只是提出范围申请。在这一行之前，授权只出现在用到它的那次动作的回复里，于是
    // "这棵候选我能落地吗"只能靠**试一次写**来回答——而那恰恰是读者不该为了知道答案去做的事。
    let approval_block = match record.body["base_root"].as_str() {
        Some(base) => approval_view::state(Path::new(base), &record, id),
        None => {
            "approval unreadable: the record names no base root, so the controller cannot be asked"
                .to_owned()
        }
    };
    // A pending widening request belongs on the read surface for the same reason the review does: a
    // request nobody can find is a request nobody answers. `pending` prints the whole thing — what is
    // outside, why, who else shares it, and the two ways to decide it — because the entry that decides
    // it is not reachable from any tool, and the reader has to be told where it is.
    // 待确认的扩权请求与评审同理，属于读面：找不到的请求等于没人会答的请求。`pending` 会印出整份请求——什么在范围外、
    // 为什么、还有谁共享它、以及决定它的两条路——因为决定它的入口从任何工具都到不了，必须告诉读者它在哪。
    let broaden = match record.body["broaden"]["state"].as_str() {
        Some("pending") => format!(
            "\n{}",
            scope::describe(
                &record.body["broaden"],
                Path::new(record.body["base_root"].as_str().unwrap_or("the base tree")),
                id,
            )
        ),
        Some(state) => format!(
            "\nlast widening request {} was decided `{state}`; the boundary is the one printed above unless it was accepted",
            record.body["broaden"]["version"].as_str().unwrap_or("?"),
        ),
        None => String::new(),
    };
    let mut status = format!(
        "candidate {}\nstate {}\ntree {}\nscope {}\nverification {}\n{review_block}{broaden}\n{}\n{}\n{}\ncandidate now {}\n{}\nstatus only reads: nothing was run and nothing was written\n",
        record.body["id"].as_str().unwrap_or(id),
        record.body["state"].as_str().unwrap_or("unknown"),
        tree.display(),
        record.scope().join(", "),
        now_identity
            .clone()
            .unwrap_or_else(|| "gone (discarded)".to_owned()),
        match verified {
            Some(candidate) => format!(
                "{} at {candidate}",
                record.body["verification"]["verdict"]
                    .as_str()
                    .unwrap_or("unknown")
            ),
            None => "none".to_owned(),
        },
        capability_block,
        approval_block,
        graft::status(&record.body["graft"]),
        match &now_identity {
            None => "the copy is gone, so there is nothing left to verify or land",
            Some(now) if verified == Some(now.as_str()) => {
                "the verification is bound to the candidate's current bytes"
            }
            Some(_) =>
                "the verification is NOT bound to the candidate's current bytes: a land would be refused until `verify` runs again",
        },
    );
    status.push_str(&match record.body["post_landing"]["obligation"].as_str() {
        Some(obligation) => format!("post-landing evidence: {obligation}\n"),
        None => "post-landing evidence: this candidate has not landed, so there is nothing the verdict could fail to cover\n".to_owned(),
    });
    Ok(status)
}

/// Remove a candidate's copy. The record stays, marked discarded, so "it was thrown away" and "it was
/// never opened" remain different facts on disk.
/// 删掉一个候选的副本。记录保留并标为 discarded，因此"它被丢掉了"与"它从没被打开过"在磁盘上仍是两件事。
pub(super) fn discard(arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or("candidate discard needs `id`")?;
    let mut record = Record::read_at(directory, id)?;
    let tree = record.tree()?;
    let slot = tree.parent().map(Path::to_path_buf).unwrap_or(tree.clone());
    std::fs::remove_dir_all(&slot).map_err(|error| format!("cannot discard: {error}"))?;
    record.body["state"] = json!("discarded");
    record.body["discarded_at"] = json!(now());
    record.publish()?;
    Ok(format!(
        "candidate {id} discarded\nthe copy under {} is gone; the record stays so the discard is on the record rather than silent\n",
        slot.display()
    ))
}
