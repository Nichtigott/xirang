//! The three reuse dimensions nobody used to observe, now read from what was actually recorded.
//! 过去没人观测的那三个复用维度，现在读自真正被记录下来的东西。
//!
//! The ruling (`docs/managed-maintenance/authorization-design-2026-10-17.md` §1.6) settled what these are allowed to
//! be: a dimension nobody observed is `unknown`, and reuse that depends on it is refused — but the unknown is **not** a
//! test failure, and caching is **not** switched off wholesale. So each dimension here answers "what did the plan
//! record", and a plan written before a dimension was recorded answers `Unknown` with the reason a reader needs in
//! order to re-run and get it. A plan that *did* record the dimension can still be reused: that is the other half, and
//! it is why this is not the same as "never reuse".
//! 裁定（`docs/managed-maintenance/authorization-design-2026-10-17.md` §1.6）定了这三者可以是什么：没人观测的维度是
//! `unknown`，依赖它的复用被拒——但未知**不是**测试失败，缓存也**不**整体关掉。因此这里每个维度回答的是"这份计划记录了
//! 什么"；在某个维度被记录之前写下的计划回答 `Unknown`，并带上读者重跑一次就能拿到它的那个理由。**确实记录了**该维度的
//! 计划仍然可复用：那是另一半，也正是"永不重用"与这里的区别。
//!
//! What each dimension is compared against, and why it can be recomputed:
//! · **anomaly** — the obligation's own recorded outcome: skipped tests are the thing a pass does not exercise. A row
//!   that never recorded the skip count cannot say "nothing is undisposed".
//! · **coverage** — the plan records the digest of the protection census it was planned against; a later consumer that
//!   appeared makes the digest differ, which is exactly "the scope this obligation must cover has grown".
//! · **published form** — the build's `pruning_manifest.tsv`, digested when the plan is made and again at reuse. "This
//!   tree publishes no form" is itself an observation, and is recorded as `absent` rather than as a missing value.
//! 每个维度与什么比较，以及为什么可以重算：
//! · **anomaly** —— 义务自己记录下来的结果：被跳过的测试正是通过所没有执行的那部分。从未记录跳过数的行说不出"没有未处置之物"。
//! · **coverage** —— 计划记录它规划时所依据的消费者普查摘要；后来出现的消费者会让摘要不同，那正是"这条义务现在必须覆盖的
//!   范围扩大了"。
//! · **published form** —— 构建的 `pruning_manifest.tsv`，规划时摘要一次、复用时再摘要一次。"这棵树不发布任何形态"本身就是
//!   一次观测，记成 `absent` 而不是记成一个缺失值。

use std::path::Path;

use serde_json::Value;
use xirang_kernel::maintenance::Witness;

/// The dimension's value, read from the obligation's own recorded outcome.
/// 该维度的取值，读自义务自己记录的结果。
pub(super) fn anomaly(rows: &[Value], row: &Value) -> Witness {
    let settled = super::check_resume::settled_row(rows, row);
    let outcome = &settled["outcome"];
    let Some(skipped) = outcome["skipped"].as_u64() else {
        return Witness::Unknown(
            "the run did not record how many tests it skipped; run it again to record them",
        );
    };
    if skipped > 0 {
        return Witness::Known(true);
    }
    let passed = outcome["verdict"].as_str() == Some("passed");
    let executed = outcome["executed"].as_u64().unwrap_or(0);
    let results = outcome["results"].as_u64().unwrap_or(0);
    if passed && executed > 0 && results > 0 {
        Witness::Known(false)
    } else {
        // A row that did not pass everything it ran already has the anomaly attached: that is the fact the dimension
        // records, and reuse is refused earlier anyway.
        // 没把跑过的全部跑通的行，异常已经挂在它上面：那就是这个维度记录的事实，而复用在此之前就会被拒。
        Witness::Known(true)
    }
}

/// Whether the scope this obligation must cover grew since the plan was made.
/// 这条义务必须覆盖的范围在计划制定之后是否扩大了。
pub(super) fn coverage(ledger: &Value, root: &Path) -> Witness {
    let Some(recorded) = ledger["coverage_digest"].as_str() else {
        return Witness::Unknown(
            "the plan did not record the consumer set it was planned against; rerun it to record one",
        );
    };
    match super::check_protection::census(root, &ledger["files"]) {
        Ok(census) => {
            let now = census
                .map(|census| xirang_kernel::sha256_hex(census.summary.to_string().as_bytes()))
                .unwrap_or_else(|| "none".to_owned());
            Witness::Known(now != recorded)
        }
        Err(_) => Witness::Unknown(
            "the protection census could not run now, so the consumer set cannot be compared",
        ),
    }
}

/// Whether the published form gained semantics the run did not verify.
/// 发布形态是否新增了这次运行未验证的语义。
pub(super) fn published(ledger: &Value, root: &Path) -> Witness {
    let Some(recorded) = ledger["published_digest"].as_str() else {
        return Witness::Unknown(
            "the plan did not record the published form it was planned against; rerun it to record one",
        );
    };
    match published_form(root) {
        // `absent` is a value, not a missing one: a tree that published no form when the plan was made and none now is
        // the same observation, and comparing `None` against the word `absent` would read as a change.
        // `absent` 是一个取值而不是缺失：规划时不发布任何形态、现在也不发布，是同一次观测；拿 `None` 去与 `absent`
        // 这个词比较会被读成"变了"。
        Ok(now) => Witness::Known(match (recorded, now) {
            ("absent", None) => false,
            ("absent", Some(_)) => true,
            (_, Some(digest)) => digest != recorded,
            (_, None) => true,
        }),
        Err(_) => Witness::Unknown(
            "a published form exists and could not be read now, so it cannot be compared",
        ),
    }
}

/// Record the three observations the plan is in a position to make, so a later reuse can compare them.
/// 记下这份计划**有资格**做的那三次观测，好让之后的复用能比对它们。
///
/// It writes the inputs alongside the digests: without the file list a later comparison would have nothing to recompute
/// the census from, and a digest that cannot be recomputed is exactly the "recorded but unverifiable" credential this
/// area keeps refusing.
/// 它把输入与摘要一起写下：没有文件清单，之后的比较就没有东西可以重算普查，而一份重算不出来的摘要，正是这个领域一直在拒的
/// "记录了但核实不了"的凭据。
pub(super) fn record(ledger: &mut Value, root: &Path, files: &Value) {
    ledger["files"] = files.clone();
    ledger["coverage_digest"] = json!(match super::check_protection::census(root, files) {
        Ok(census) => census
            .map(|census| xirang_kernel::sha256_hex(census.summary.to_string().as_bytes()))
            .unwrap_or_else(|| "none".to_owned()),
        // A census that could not run is recorded as **not recorded**, so reuse refuses it instead of comparing
        // against a value that was never observed.
        // 跑不起来的普查被记成**没记录**，于是复用会拒绝它，而不是拿一个从未被观测的值去比较。
        Err(_) => return,
    });
    if let Ok(published) = published_form(root) {
        ledger["published_digest"] = json!(published.unwrap_or_else(|| "absent".to_owned()));
    }
}

/// Why the newest plan for this task could not be reused, in the reader's words — or `None` when nothing refused it.
/// 这个任务最新的那份计划为什么不能复用，用读者的话说——没有任何东西拒绝它时为 `None`。
///
/// Without this, a `reuse: true` call that re-runs looks identical to one where the caller simply had no plan: the
/// ruling asks for the unknown to be **stated**, not merely enforced, and a refusal a reader cannot see is the kind of
/// silence this workspace keeps replacing.
/// 没有它，一次重跑的 `reuse: true` 与"调用方本来就没有计划"看起来完全一样：裁定要求未知被**说出来**而不只是被执行，
/// 而读者看不见的拒绝，正是这个工作区一直在替换掉的那种沉默。
pub(super) fn refusal(ledger: &Value, root: &Path) -> Option<String> {
    // Only the two dimensions that belong to the **plan** are explained here: the anomaly dimension is per obligation,
    // and its refusal is already visible in the obligation table a resume prints.
    // 这里只解释属于**计划**的那两个维度：异常维度是逐义务的，它的拒绝在 resume 印的义务表里已经看得见。
    for (name, witness) in [
        ("scope", coverage(ledger, root)),
        ("published form", published(ledger, root)),
    ] {
        if let Witness::Unknown(why) = witness {
            return Some(format!("the {name} dimension is unknown: {why}"));
        }
        if witness == Witness::Known(true) {
            return Some(format!("the {name} dimension is recorded as changed"));
        }
    }
    None
}

/// The line a plan prints when `reuse` was asked for and not taken, so a silent re-run cannot be read as
/// "there was nothing to reuse". `None` when no plan exists for this task or nothing refused it.
/// `reuse` 被要求却没拿到计划时，计划印的那一行，好让一次沉默的重跑不被读成"本来就没有可复用的东西"。这个任务没有
/// 计划、或没有任何东西拒绝它时为 `None`。
pub(super) fn reuse_note(root: &Path, directory: &Path, args: &Value) -> String {
    if args.get("reuse").and_then(Value::as_bool) != Some(true) {
        return String::new();
    }
    let note = || {
        let record =
            super::check_task::newest_for_task(directory, &super::check_task::task_key(args))?;
        let ledger: Value = serde_json::from_str(&std::fs::read_to_string(record).ok()?).ok()?;
        refusal(&ledger, root)
    };
    match note() {
        Some(reason) => format!(
            "reuse not taken: {reason}; run again (this plan did) or rerun the older one after recording the dimension\n"
        ),
        None => String::new(),
    }
}

/// The digest of the build's pruning manifest, or `None` when this tree publishes no form.
/// 构建修剪清单的摘要；这棵树不发布任何形态时为 `None`。
fn published_form(root: &Path) -> Result<Option<String>, String> {
    let target = root.join("target");
    let Ok(profiles) = std::fs::read_dir(&target) else {
        return Ok(None);
    };
    let mut manifests = Vec::new();
    for profile in profiles.flatten() {
        let Ok(units) = std::fs::read_dir(profile.path().join("build")) else {
            continue;
        };
        for unit in units.flatten() {
            let candidate = unit.path().join("out/pruning_manifest.tsv");
            if candidate.is_file() {
                manifests.push(candidate);
            }
        }
    }
    manifests.sort();
    let Some(first) = manifests.first() else {
        return Ok(None);
    };
    let bytes = std::fs::read(first)
        .map_err(|error| format!("cannot read {}: {error}", first.display()))?;
    Ok(Some(xirang_kernel::sha256_hex(&bytes)))
}

use serde_json::json;
