//! The graft half of a candidate's record: what a landing would change about the graft plan, and
//! which two declarations would end up claiming one slot.
//! 候选记录里的 graft 那一半：一次落地会改到计划什么，以及哪两条声明会落到同一个槽位上。
//!
//! §8.3 of `docs/managed-maintenance/execution-and-delivery.md` binds a graft candidate to five
//! facts — the **target set**, the **replacement side**, the **contract version**, the
//! **configuration**, and the **effective projection** — and requires the comparison to include
//! **both** the base and the candidate. This module is that binding. It reads the same declarations
//! and the same faces the build reads (no second parser, no Cargo invocation), and it writes them
//! onto the candidate so a later reader can tell "the same graft" from "a different one under the
//! same name".
//! `docs/managed-maintenance/execution-and-delivery.md` 的 §8.3 要求 graft 候选绑定五件事——**目标集合**、
//! **替换端**、**合同版本**、**配置**、**有效投影**——并要求比较**同时**包含基树与候选。本模块就是那份绑定。
//! 它读的是构建读的同一批声明与同一批注册面（没有第二个解析器，也不调 Cargo），并写进候选记录，使后来的读者
//! 分得出"同一次嫁接"与"同名之下的另一次"。
//!
//! Four decisions keep the refusal honest rather than decorative. **A conflict is named by its two
//! parties**, not by a count: "two declarations claim one slot" makes the reader find which two.
//! **Only what the candidate introduces blocks** — a conflict already in the base is reported and
//! allowed through, because refusing an unrelated landing over an old state makes this gate a wall.
//! **A party behind a `cfg` this surface does not evaluate is reported, not refused**: the build
//! decides that combination, and a surface that cannot evaluate a gate must not act as if it had.
//! **The rule version is part of the binding**, so a record written by another version of this rule
//! is not reused as if it were the same contract.
//! 四个决定让这次拒绝诚实而不是装饰。**冲突按它的两方点名**，而不是报个计数："两条声明抢一个槽位"会把"哪两条"
//! 留给读者自己找。**只有候选引入的冲突才阻塞**——已经存在于基树的冲突只上报、照旧放行，因为为一个旧状态拒绝一次
//! 无关的落地，会让这道门变成一堵墙。**落在本执行面不求值的 `cfg` 之后的那一方只上报、不拒**：那个组合由构建
//! 决定，而一个求值不了门控的执行面不许表现得像求过了一样。**规则版本是绑定的一部分**，因此由另一版规则写下的
//! 记录不会被当成同一份合同复用。
//!
//! What this binding deliberately does **not** carry is a face's `flow` contract version. Nothing in
//! this tree derives it into a face view — the discovery walk carries `kind`, `parent`,
//! `needs_registry` and the fields a declaration writes, and a face's flow contract is not one of
//! them — so writing one here would be a fact this tool does not have. The contract version this page
//! can honestly bind is the one it *owns*: [`POLICY`], the rule set that decides which declaration
//! addresses which slot.
//! 这份绑定**有意不**携带的是面的 `flow` 合同版本。本树没有任何东西把它推导进注册面视图——发现遍历携带
//! `kind`、`parent`、`needs_registry` 与声明写下的那些字段，而面的 flow 合同不在其中——因此在这里写一个，
//! 就是一件本工具并不拥有的事实。本页能诚实绑定的合同版本是它**自己的**那一个：[`POLICY`]，也就是决定"哪条
//! 声明指向哪个槽位"的规则集。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

/// The rule set a binding was made under, recorded so a later landing can tell a binding this tool
/// would produce today from one produced by a version that has since changed.
/// 一份绑定所依据的规则集；记录下来，使之后的一次落地分得出"本工具今天会产出的绑定"与"已变更的旧版本产出的绑定"。
///
/// The policy is the **contract version** half of §8.3: the five bound facts are only comparable
/// under one rule for "which declaration addresses which slot", so a record carrying a different
/// `policy` is a different contract and is refused rather than silently reinterpreted.
/// 政策是 §8.3 里**合同版本**的那一半：五个被绑定的事实只有在同一条"哪条声明指向哪个槽位"的规则下才可比，
/// 因此携带不同 `policy` 的记录是另一份合同，会被拒绝而不是被静默重新解释。
pub(super) const POLICY: &str = "xirang.graft.binding/v1";

/// The configuration a projection was read under, apart from the tree itself: the identity
/// namespace, the toolchain, and the reader.
/// 投影所依据的配置（树本身之外的部分）：身份命名空间、工具链与读取器。
///
/// This is the configuration half of §8.3, and it is recorded rather than compared away because a
/// candidate outlives the process that opened it: the same declarations read by another toolchain or
/// under another namespace are a different projection, and a record that dropped these would compare
/// equal to the wrong one. A namespace that cannot be learned is recorded as an explicit unknown
/// rather than as an empty dimension — "namespace=" would compare equal to a package whose name is
/// the empty string.
/// 这是 §8.3 的配置那一半，而它被记录而不是被抹平，因为候选比打开它的进程活得久：同一批声明在另一条工具链下、
/// 或另一个命名空间下读出来，是另一个投影，而丢掉这些维度的记录会与错误的那一个比较相等。学不到的命名空间被记成
/// 明说的未知，而不是一个空维度——`namespace=` 会与"包名是空字符串"的包比较相等。
///
/// Every dimension here is **deterministic**: the reason a namespace could not be learned is *not*
/// part of it. The failure's text can differ between two reads of the same tree (a tool's error
/// message is not a value), and folding it in would make `land` refuse with "another configuration"
/// when the honest answer is "this plan could not be read" — the reason is already carried by
/// `reading: unknown` and its `why`, which is where a reader looks for it.
/// 这里的每一维都是**确定值**：学不到命名空间的*原因*不属于它。失败文案在同一棵树的两次读取之间可能不同（一个
/// 工具的错误信息不是取值），把它折进来会让 `land` 在正确答案是"这份计划读不出来"时用"另一份配置"拒绝——而那
/// 个原因已经由 `reading: unknown` 与它的 `why` 携带，那才是读者去找它的地方。
pub(super) fn configuration_of(root: &Path) -> String {
    let environment = crate::mcp::check_execution::Environment::observe();
    let namespace = match crate::mcp::registry::namespace(root) {
        Ok(namespace) => format!("namespace={namespace}"),
        Err(_) => "namespace=unknown".to_owned(),
    };
    format!(
        "{namespace}; {}; reader={POLICY} {}",
        environment.toolchain, environment.checker
    )
}

/// One read of the graft plan: the declarations the entry carries, the objects they cover, the slots
/// two of them would compete for, and the declarations that name no face at all.
/// 一次 graft 计划读取：入口携带的声明、它们覆盖的对象、其中两条会争的槽位，以及完全没有点名任何面的声明。
///
/// `reading` is `read`, `none` or `unknown`, and the three are different facts. `none` means the tree
/// has no host entry, so there is no graft surface to bind — that is a clean state. `unknown` means
/// there **is** a surface this read could not establish, and nothing may claim the landing introduces
/// no conflict on the strength of a read that failed.
/// `reading` 是 `read`、`none` 或 `unknown`，三者是三个不同的事实。`none` 表示树里没有宿主入口，因此没有
/// graft 面可绑——那是干净状态。`unknown` 表示**存在**一个这次读取没能确立的面，于是没有任何东西可以借一次
/// 失败的读取来声称"这次落地没有引入冲突"。
pub(super) fn binding(root: &Path, configuration: &str) -> Value {
    // The entry is checked **first**, and that order is the difference between two answers: a tree
    // with no host entry is a clean "no graft surface", while a tree that has one this read cannot
    // establish is an unknown. Asking for the identity namespace first would have made every tree
    // without an entry report `unknown` — cargo refuses to name a package with no target, so the
    // namespace read fails and its failure was being reported as a failed *graft* read.
    // 入口**先**检查，而这个顺序正是两个答案的区别：没有宿主入口的树是干净的"没有 graft 面"，而有一个却读不
    // 出来的树是未知。先问身份命名空间，会让每棵没有入口的树都报 `unknown`——cargo 拒绝为一个没有 target 的包
    // 命名，于是命名空间读取失败，而它的失败被报成了一次失败的 **graft** 读取。
    if let Err(why) = crate::build_method::host_entry_source(root) {
        // No entry is the clean "no graft surface" answer; anything else is a failure to read a
        // tree that does have one.
        // 没有入口就是干净的"没有 graft 面"；其余都是"树有这个面但读不出来"。
        if why.starts_with("no host entry at") {
            return json!({
                "reading": "none",
                "configuration": configuration,
                "why": why,
                "declarations": [],
                "objects": [],
                "conflicts": [],
                "unresolved": [],
            });
        }
        return unread(configuration, &why);
    }
    let namespace = match crate::mcp::registry::namespace(root) {
        Ok(namespace) => namespace,
        Err(why) => {
            return unread(
                configuration,
                &format!("the identity namespace could not be learned ({why})"),
            );
        }
    };
    let declared = match crate::build_method::declared_grafts(root) {
        Ok(declared) => declared,
        Err(why) => {
            return unread(
                configuration,
                &format!("the host entry's graft declarations could not be read ({why})"),
            );
        }
    };
    let views = match crate::build_method::face_views(root, &namespace) {
        Ok(views) => views,
        Err(why) => {
            return unread(
                configuration,
                &format!("the registration faces could not be read ({why})"),
            );
        }
    };
    let entry = declared
        .entry
        .strip_prefix(root)
        .unwrap_or(&declared.entry)
        .display()
        .to_string();
    let mut declarations: Vec<Value> = Vec::new();
    let mut unresolved: Vec<Value> = Vec::new();
    // The slot a declaration addresses is every face it covers, not only the face it names: a `full`
    // cut replaces the whole subtree below its target, so a second declaration naming one face inside
    // that subtree competes for a slot the first one already owns.
    // 一条声明所指的槽位是它覆盖的**每一个**面，而不只是它点名的那个：`full` 切口替换它目标之下的整棵子树，
    // 因此另一条点名该子树里某个面的声明，争的是第一条已经占住的槽位。
    let mut slots: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut objects: BTreeSet<String> = BTreeSet::new();
    let mut paths: Vec<String> = Vec::new();
    for (index, cut) in declared.cuts.iter().enumerate() {
        let mut covers: BTreeSet<String> = views
            .iter()
            .filter(|view| cut.names_face(&view.path, Some(&view.module)))
            .map(|view| view.path.clone())
            .collect();
        let named = covers.len();
        if cut.full {
            for view in &views {
                if covers_below(&view.path, &covers) {
                    covers.insert(view.path.clone());
                }
            }
        }
        for path in &covers {
            slots.entry(path.clone()).or_default().push(index);
            objects.insert(path.clone());
        }
        if named == 0 {
            unresolved.push(json!({
                "line": cut.line,
                "cut": cut.cut_label(),
                "why": "names no face in this tree",
            }));
        }
        paths.push(covers.iter().cloned().collect::<Vec<_>>().join(", "));
        declarations.push(json!({
            "line": cut.line,
            "cut": cut.cut_label(),
            "graft": cut.graft,
            "form": cut.form(),
            "full": cut.full,
            "cfg": cut.cfg,
            "targets": covers.iter().cloned().collect::<Vec<_>>(),
        }));
    }
    let mut conflicts: Vec<Value> = Vec::new();
    for (path, claimants) in &slots {
        if claimants.len() < 2 {
            continue;
        }
        let parties: Vec<Value> = claimants
            .iter()
            .map(|&index| {
                let declaration = &declarations[index];
                json!({
                    "line": declaration["line"],
                    "cut": declaration["cut"],
                    "graft": declaration["graft"],
                    "gated": !declaration["cfg"].is_null(),
                })
            })
            .collect();
        // A conflict with a gated party is reported and not refused: which features are on is the
        // build's decision, and this surface does not evaluate `cfg` at all.
        // 有一方带门控的冲突只上报、不拒：打开哪些特性由构建决定，而本执行面**完全不**对 `cfg` 求值。
        let blocking = parties
            .iter()
            .all(|party| party["gated"].as_bool() != Some(true));
        conflicts.push(json!({
            "path": path,
            "parties": parties,
            "blocking": blocking,
        }));
    }
    json!({
        "reading": "read",
        "configuration": configuration,
        "entry": entry,
        "declarations": declarations,
        "objects": objects.iter().cloned().collect::<Vec<_>>(),
        "conflicts": conflicts,
        "unresolved": unresolved,
    })
}

/// A read that established nothing about the graft surface.
/// 一次没能就 graft 面确立任何东西的读取。
fn unread(configuration: &str, why: &str) -> Value {
    json!({
        "reading": "unknown",
        "configuration": configuration,
        "why": why,
        "declarations": [],
        "objects": [],
        "conflicts": [],
        "unresolved": [],
    })
}

/// Whether `path` lies strictly below one of the covered paths.
/// `path` 是否严格位于某个被覆盖路径之下。
///
/// The separator bound is the point: `root/control` covers `root/control/left` but not
/// `root/controls`, which is the difference between a subtree and a string prefix.
/// 分隔符边界是要点：`root/control` 覆盖 `root/control/left`，但不覆盖 `root/controls`——这正是"子树"与
/// "字符串前缀"的区别。
fn covers_below(path: &str, covered: &BTreeSet<String>) -> bool {
    covered.iter().any(|ancestor| {
        path.strip_prefix(ancestor.as_str())
            .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// The conflicts `candidate` has that `base` does not, which is what a landing would introduce.
/// `candidate` 有而 `base` 没有的那些冲突，也就是一次落地会引入的冲突。
///
/// The comparison is by **parties** and not by count: a slot that was already claimed by two
/// declarations, and is still claimed by two but by a different pair, is a new conflict — the old
/// pair's disagreement is gone and a different one took its place.
/// 比较按**两方**而不是按计数：一个已经被两条声明占住的槽位，仍然被两条占住、但换了一对，仍然是一个新冲突——
/// 旧那一对的分歧结束了，另一对顶了上来。
pub(super) fn introduced(base: &Value, candidate: &Value) -> Vec<Value> {
    let existing: BTreeSet<String> = base["conflicts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(conflict_key)
        .collect();
    candidate["conflicts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|conflict| !existing.contains(&conflict_key(conflict)))
        .cloned()
        .collect()
}

/// A conflict's identity: the slot and the parties, independent of the order they were found in.
/// 一个冲突的身份：槽位加上两方，与它们被发现的顺序无关。
fn conflict_key(conflict: &Value) -> String {
    let mut parties: Vec<String> = conflict["parties"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|party| {
            format!(
                "{}:{} `{}` `{}`",
                party["line"],
                party["cut"].as_str().unwrap_or(""),
                party["graft"].as_str().unwrap_or(""),
                party["gated"],
            )
        })
        .collect();
    parties.sort();
    format!(
        "{} | {}",
        conflict["path"].as_str().unwrap_or(""),
        parties.join(" + ")
    )
}

/// One conflict rendered as the sentence a refusal needs: **both** parties, at their own lines.
/// 把一个冲突渲染成一次拒绝所需要的那句话：**两方**都在，各自带着自己的行号。
pub(super) fn describe(conflict: &Value, entry: &str) -> String {
    let parties: Vec<String> = conflict["parties"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|party| {
            format!(
                "{entry}:{} `cut {}` graft `{}`{}",
                party["line"],
                party["cut"].as_str().unwrap_or(""),
                party["graft"].as_str().unwrap_or(""),
                if party["gated"].as_bool() == Some(true) {
                    " (behind a cfg this surface does not evaluate)"
                } else {
                    ""
                },
            )
        })
        .collect();
    format!(
        "`{}` is claimed by {}",
        conflict["path"].as_str().unwrap_or("a slot"),
        parties.join(" and "),
    )
}

/// What a landing would change about the effective object set, as lines a reader can compare.
/// 一次落地会把有效对象集合改成什么，写成读者能逐行对照的样子。
///
/// §8.3 makes this a **semantic** change rather than a textual rebinding: the reader is told which
/// objects appear, which disappear, and which declaration now covers a different set, so "the graft
/// still parses" cannot be read as "the graft still means the same thing".
/// §8.3 让这件事成为一次**语义**变化而不是一次文字重绑：读者会看到哪些对象出现、哪些消失、哪条声明现在覆盖了
/// 另一批对象，因此"这行嫁接还能解析"不会被读成"这次嫁接还是同一个意思"。
pub(super) fn changes(base: &Value, candidate: &Value) -> Vec<String> {
    let before = by_spelling(base);
    let after = by_spelling(candidate);
    let mut lines = Vec::new();
    for (spelling, covers) in &after {
        match before.get(spelling) {
            None => lines.push(format!("+ {spelling} now covers [{}]", join(covers))),
            Some(previous) if previous != covers => lines.push(format!(
                "~ {spelling}: [{}] -> [{}]",
                join(previous),
                join(covers)
            )),
            Some(_) => {}
        }
    }
    for (spelling, covers) in &before {
        if !after.contains_key(spelling) {
            lines.push(format!("- {spelling} no longer covered [{}]", join(covers)));
        }
    }
    let objects_before = objects(base);
    let objects_after = objects(candidate);
    if objects_before != objects_after {
        lines.push(format!(
            "effective objects [{}] -> [{}]",
            join(&objects_before),
            join(&objects_after)
        ));
    }
    lines
}

/// Every declaration, keyed by the spelling that decides what it means.
/// 每一条声明，按决定它含义的那个拼法作键。
fn by_spelling(binding: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for declaration in binding["declarations"].as_array().into_iter().flatten() {
        let spelling = format!(
            "{} graft `{}`{}",
            declaration["cut"].as_str().unwrap_or(""),
            declaration["graft"].as_str().unwrap_or(""),
            if declaration["full"].as_bool() == Some(true) {
                " full"
            } else {
                ""
            },
        );
        let covers: BTreeSet<String> = declaration["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        map.entry(spelling).or_default().extend(covers);
    }
    map
}

/// The effective object set of one read.
/// 一次读取的有效对象集合。
pub(super) fn objects(binding: &Value) -> BTreeSet<String> {
    binding["objects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// Render a set for one line, or `nothing` when it is empty.
/// 把集合渲染成一行；为空时写 `nothing`。
fn join(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        "nothing".to_owned()
    } else {
        set.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

/// What the gate decided, and what the record should say afterwards.
/// 这道门判定了什么，以及记录之后应当怎么写。
pub(super) struct Gate {
    /// The replacement for the record's `graft` object.
    /// 记录里 `graft` 对象的替代值。
    pub(super) record: Value,
    /// Lines for the landing's reply, when the landing goes ahead.
    /// 落地继续时写进落地回复的行。
    pub(super) report: Vec<String>,
    /// The refusal, when the landing would introduce a conflict. Nothing has been written when this
    /// is set.
    /// 当落地会引入冲突时的拒绝文案。它被设置时，什么都还没写。
    pub(super) refusal: Option<String>,
}

/// Decide whether this landing may proceed, from both sides of the comparison.
/// 从比较的两端判定这次落地能否继续。
///
/// `opened` is what `open` stored: the policy, the configuration, and the two reads taken then.
/// Everything here happens **before** the first byte of the base is written, so a refusal leaves the
/// base exactly as it was — and the record says so, because a refusal that is not on the record is
/// indistinguishable from a landing that never happened.
/// `opened` 是 `open` 存下的东西：政策、配置，以及当时取的两次读取。这里的一切都发生在基树的第一个字节被写
/// **之前**，因此一次拒绝会让基树原封不动——而记录会这么写，因为没有记进记录的拒绝，与一次从未发生的落地
/// 在磁盘上分不出来。
pub(super) fn gate(root: &Path, tree: &Path, opened: &Value) -> Gate {
    let configuration = configuration_of(root);
    let base = binding(root, &configuration);
    let candidate = binding(tree, &configuration);
    let mut report = Vec::new();
    // A binding is only comparable under the rule set and the configuration that produced it. The
    // refusal names both sides instead of quietly recomputing: a record from another version is not
    // wrong, it is a different contract.
    // 一份绑定只在产出它的规则集与配置下可比。拒绝会点名两端，而不是静默重算：另一个版本写下的记录不是错的，
    // 它是另一份合同。
    let recorded_policy = opened["policy"].as_str().unwrap_or("nothing");
    let recorded_configuration = opened["configuration"].as_str().unwrap_or("nothing");
    if recorded_policy != POLICY || recorded_configuration != configuration {
        let refusal = format!(
            "this candidate's graft binding was read under `{recorded_policy}` / `{recorded_configuration}`, and this tree is now read under `{POLICY}` / `{configuration}`; a binding is only comparable under the rule set that produced it, so nothing lands. Open a new candidate",
        );
        return Gate {
            record: attempt(
                opened,
                &configuration,
                &base,
                &candidate,
                "nothing",
                "the binding was read under a different rule set or configuration",
            ),
            report,
            refusal: Some(refusal),
        };
    }
    // A read that established nothing cannot vouch for anything: an `unknown` on either side means
    // "this landing says nothing about the graft plan", so it does not land.
    // 一次什么都没确立的读取保不了任何东西：任意一端是 `unknown`，都意味着"这次落地对 graft 计划什么都不说"，
    // 因此它不落地。
    for (side, read) in [("the base tree", &base), ("the candidate copy", &candidate)] {
        if read["reading"].as_str() != Some("read") && read["reading"].as_str() != Some("none") {
            let refusal = format!(
                "the graft plan of {side} could not be read ({}), so nothing lands: a landing that cannot read the plan cannot say what it would change about it",
                read["why"].as_str().unwrap_or("no reason recorded"),
            );
            return Gate {
                record: attempt(
                    opened,
                    &configuration,
                    &base,
                    &candidate,
                    "nothing",
                    "the graft plan could not be read",
                ),
                report,
                refusal: Some(refusal),
            };
        }
    }
    let entry = base["entry"]
        .as_str()
        .or_else(|| candidate["entry"].as_str())
        .unwrap_or("the host entry");
    let blocking: Vec<Value> = introduced(&base, &candidate)
        .into_iter()
        .filter(|conflict| conflict["blocking"].as_bool() == Some(true))
        .collect();
    if !blocking.is_empty() {
        let mut sentences = Vec::new();
        for conflict in &blocking {
            sentences.push(describe(conflict, entry));
            report.push(format!("graft conflict: {}", describe(conflict, entry)));
        }
        let refusal = format!(
            "{} — this landing would leave two declarations competing for one slot. Nothing was written to {}; the conflict is on the record, and the copy is still there to fix",
            sentences.join("; "),
            root.display(),
        );
        return Gate {
            record: attempt(
                opened,
                &configuration,
                &base,
                &candidate,
                "nothing",
                "graft conflict",
            ),
            report,
            refusal: Some(refusal),
        };
    }
    // What the landing changes is printed whether or not anything competes: an effective object set
    // that moved is a semantic change, and the old and new sets are the only way a reader sees it.
    // 落地改了什么一律印出来，无论有没有竞争：有效对象集合移动了就是一次语义变化，而旧/新集合是读者看见它的唯一
    // 途径。
    let changed = changes(&base, &candidate);
    if !changed.is_empty() {
        report.push("graft projection changed:".to_owned());
        for line in &changed {
            report.push(format!("  {line}"));
        }
    } else {
        report.push(format!(
            "graft projection unchanged: {} declaration(s) over [{}]",
            candidate["declarations"]
                .as_array()
                .map_or(0, std::vec::Vec::len),
            join(&objects(&candidate)),
        ));
    }
    for conflict in introduced(&base, &candidate) {
        if conflict["blocking"].as_bool() != Some(true) {
            report.push(format!(
                "graft conflict not refused (a cfg decides it, and this surface does not evaluate cfg): {}",
                describe(&conflict, entry)
            ));
        }
    }
    for conflict in candidate["conflicts"].as_array().into_iter().flatten() {
        if conflict["blocking"].as_bool() == Some(true)
            && base["conflicts"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|existing| conflict_key(existing) == conflict_key(conflict))
        {
            report.push(format!(
                "graft conflict already in the base (this landing neither creates nor settles it): {}",
                describe(conflict, entry)
            ));
        }
    }
    for declaration in candidate["unresolved"].as_array().into_iter().flatten() {
        if !base["unresolved"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|existing| existing == declaration)
        {
            report.push(format!(
                "graft declaration names no face: `{}` at {entry}:{} — it addresses nothing in this tree",
                declaration["cut"].as_str().unwrap_or(""),
                declaration["line"],
            ));
        }
    }
    Gate {
        record: attempt(
            opened,
            &configuration,
            &base,
            &candidate,
            "unknown (this attempt reached the gate but recorded no write)",
            "",
        ),
        report,
        refusal: None,
    }
}

/// The record one landing attempt leaves behind: both reads, what was introduced, and whether
/// anything was written — **appended** to the attempts before it, never replacing them.
/// 一次落地尝试留下的记录：两次读取、引入了什么，以及有没有写下任何东西——**追加**在之前的尝试之后，绝不替换它们。
///
/// Appending is the point. "There was a conflict and then it was fixed" and "the landing was accepted"
/// are two different histories, and a record that kept only the last attempt would make them one: the
/// conflict would vanish the moment the recovery succeeded, which is exactly the fact the criterion
/// asks a reader to be able to check afterwards.
/// 追加是要点。"曾经有冲突、之后被修好"与"落地被接受"是两段不同的历史，而只留最后一次尝试的记录会把它们变成一段：
/// 恢复成功的那一刻冲突就消失了，而那正是判据要求读者事后能核到的事实。
fn attempt(
    opened: &Value,
    configuration: &str,
    base: &Value,
    candidate: &Value,
    wrote: &str,
    refused: &str,
) -> Value {
    let mut attempts: Vec<Value> = opened["attempts"].as_array().cloned().unwrap_or_default();
    attempts.push(json!({
        "base": base,
        "candidate": candidate,
        "introduced": introduced(base, candidate),
        "changed": changes(base, candidate),
        "wrote": wrote,
        "refused": if refused.is_empty() { Value::Null } else { json!(refused) },
    }));
    json!({
        "policy": POLICY,
        "configuration": configuration,
        "at_open": opened["at_open"].clone(),
        "attempts": attempts,
    })
}

/// Record what the landing actually wrote on the newest attempt.
/// 把这次落地实际写了什么记在最新那次尝试上。
///
/// The gate cannot know this — it runs before the first write — so the landing fills it in afterwards.
/// Until it does, the attempt says `unknown` rather than `nothing`, because "the write has not happened
/// yet" and "the write wrote nothing" are different facts and only one of them is ever true here.
/// 这道门算不出这个数——它跑在第一次写之前——因此由落地在事后填进去。在填进去之前，那次尝试写的是 `unknown`
/// 而不是 `nothing`，因为"写还没发生"与"写等于什么都没写"是两个不同的事实，而这里只可能有一个成立。
pub(super) fn note_write(graft: &mut Value, wrote: &str) {
    if let Some(attempts) = graft["attempts"].as_array_mut()
        && let Some(last) = attempts.last_mut()
    {
        last["wrote"] = json!(wrote);
    }
}

/// One line for a read: what it found, in the numbers a reader can check against `entry`.
/// 一次读取的一行：它发现了什么，写成读者能拿去与 `entry` 对照的数字。
pub(super) fn line(binding: &Value, label: &str) -> String {
    match binding["reading"].as_str() {
        Some("read") => {
            let declarations = binding["declarations"]
                .as_array()
                .map_or(0, std::vec::Vec::len);
            let mut text = format!(
                "graft {label}: {declarations} declaration(s) in {} over [{}]",
                binding["entry"].as_str().unwrap_or("the host entry"),
                join(&objects(binding)),
            );
            for conflict in binding["conflicts"].as_array().into_iter().flatten() {
                text.push_str(&format!(
                    "\n  conflict{}: {}",
                    if conflict["blocking"].as_bool() == Some(true) {
                        ""
                    } else {
                        " (a cfg decides it)"
                    },
                    describe(
                        conflict,
                        binding["entry"].as_str().unwrap_or("the host entry")
                    )
                ));
            }
            text
        }
        Some("none") => format!(
            "graft {label}: none — {}",
            binding["why"]
                .as_str()
                .unwrap_or("this tree has no host entry")
        ),
        _ => format!(
            "graft {label} unknown: {}",
            binding["why"]
                .as_str()
                .unwrap_or("the read failed without a reason")
        ),
    }
}

/// The whole graft section of a record, for the **read** surface.
/// 记录里整个 graft 段，供**读**面使用。
///
/// It belongs on `status` and not only on the landing's reply for the reason the review line does: a
/// candidate can be refused and then sit there, and a reader who only asks `status` must be able to
/// read back **which two parties** the conflict was between and whether anything was written.
/// 它属于 `status` 而不只属于落地那次回复，理由与评审那一行相同：候选可以被拒之后搁在那里，而只问 `status`
/// 的读者必须能读回**冲突在哪两方之间**、以及有没有写下任何东西。
pub(super) fn status(graft: &Value) -> String {
    if graft.is_null() {
        return "graft not bound on this record (it was opened before this surface bound one)"
            .to_owned();
    }
    let mut lines = vec![format!(
        "graft bound under `{}`; {}",
        graft["policy"].as_str().unwrap_or("no policy recorded"),
        graft["configuration"]
            .as_str()
            .unwrap_or("no configuration recorded"),
    )];
    match graft["at_open"]["candidate"]["reading"].as_str() {
        Some(_) => lines.push(line(&graft["at_open"]["candidate"], "at open")),
        None => lines.push("graft at open: not recorded".to_owned()),
    }
    let attempts: Vec<&Value> = graft["attempts"].as_array().into_iter().flatten().collect();
    if attempts.is_empty() {
        return lines.join("\n");
    }
    lines.push(format!(
        "graft {} landing attempt(s) on record",
        attempts.len()
    ));
    for (index, landed) in attempts.iter().enumerate() {
        let number = index + 1;
        lines.push(format!(
            "graft attempt {number}: {}; wrote {}",
            match landed["refused"].as_str() {
                Some(reason) => format!("refused ({reason})"),
                None => "accepted".to_owned(),
            },
            landed["wrote"]
                .as_str()
                .unwrap_or("nothing (no record of a write)"),
        ));
        let entry = landed["candidate"]["entry"]
            .as_str()
            .or_else(|| landed["base"]["entry"].as_str())
            .unwrap_or("the host entry");
        for conflict in landed["introduced"].as_array().into_iter().flatten() {
            lines.push(format!(
                "  conflict introduced{}: {}",
                if conflict["blocking"].as_bool() == Some(true) {
                    ""
                } else {
                    " (a cfg decides it, and this surface does not evaluate cfg)"
                },
                describe(conflict, entry),
            ));
        }
        for change in landed["changed"].as_array().into_iter().flatten() {
            lines.push(format!(
                "  change: {}",
                change.as_str().unwrap_or("unreadable entry")
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
#[path = "candidate_graft_tests.rs"]
mod candidate_graft_tests;
