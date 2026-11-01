//! Which siblings are useful references, and **why** — contract first, kind only as a signal.
//! 哪些兄弟姐妹是有用的参考，以及**为什么**——先看合同，kind 只是一个信号。
//!
//! The rule this replaces said "a reference is a sibling with the same parent and the same `kind`", and an
//! independent review measured what that costs (`docs/managed-maintenance/independent-review-2026-10-17.md` §2, P1):
//! the shipped `control-button` example has a Button and a Slider that declare the **same** `ControlHandle`,
//! the same `control.render` export and the same `control.render.v1` flow contract, and the kind comparison
//! excluded Slider as "a different role". The maintainer's ruling says the same thing in one line: 「kind 只作参考
//! 信号」— and adds the other half, that the same kind is not a reliable specimen either.
//! 这条被替换的规则是"同父且同 `kind` 的兄弟就是参考"，而独立复核量出了它的代价
//! （`docs/managed-maintenance/independent-review-2026-10-17.md` §2 P1）：出厂示例 `control-button` 里的 Button 与
//! Slider 声明了**同一个** `ControlHandle`、同一个 `control.render` 导出与同一个 `control.render.v1` flow 合同，而
//! kind 比较把 Slider 当作"不同角色"排除了。维护者的裁定用一句话说了同一件事：「kind 只作参考信号」——并补上另一半：
//! 同 kind 也不是可靠标本。
//!
//! So the comparison reads four signals and gives a **reason** with each verdict: the flow contract (the
//! strongest, read out of the declaration text), the capability labels (`exports` / `provides`), the kind (a
//! signal, never a proof), and whether the workspace already adopted that face as a specimen (the adoption
//! ledger, with its lease verdict — a lapsed lease is reported as lapsed rather than as a specimen).
//! Nothing is excluded on the kind alone, and "not enough information" is said instead of being read as
//! "there is nothing to learn from it".
//! 因此比较读四个信号，并为每个判定给出**理由**：flow 合同（最强，读自声明原文）、能力标签（`exports`/`provides`）、
//! kind（信号，永远不是证明）、以及本工作区是否已把那个面采信为标本（采信台账，连同它的租约判定——失效的租约按失效报，
//! 不按标本报）。没有任何东西只凭 kind 被排除，而"信息不足"会被说出来，而不是被读成"它身上没有可学的东西"。
//!
//! What this page does **not** read yet, so the gap is stated rather than implied: the `handle_traits` /
//! `part_traits` labels (`xirang_toolchain::contracts!(Button: crate::control::ControlHandle)`) are not extracted.
//! A face that matches only through those labels is reported as a weak candidate, not as a match.
//! 本页**尚未**读取的东西，因此这个缺口是明说的而不是暗示的：`handle_traits` / `part_traits` 标签
//! （`xirang_toolchain::contracts!(Button: crate::control::ControlHandle)`）没有被提取。只通过那些标签匹配的面会被报成
//! 弱候选，而不是匹配。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::SourceFile;

/// One face's four signals, read once so every comparison against it costs nothing.
/// 一个面的四个信号，只读一次，于是每一次与它比较都不再花代价。
struct Signals {
    /// The face's logical path.
    /// 面的逻辑路径。
    path: String,
    /// The root-relative source the declaration lives in.
    /// 声明所在的、相对根的源码路径。
    source: String,
    /// The declared `kind`.
    /// 声明的 `kind`。
    kind: String,
    /// The flow contract as `<id>@<version>`, when the declaration names one.
    /// flow 合同，写作 `<id>@<version>`；声明没有点名时为 `None`。
    contract: Option<String>,
    /// The capability labels from `exports` and `provides`, sorted.
    /// 来自 `exports` 与 `provides` 的能力标签，已排序。
    capabilities: Vec<String>,
    /// The adoption entry naming this source, if one does **and** its lease still holds.
    /// 点名这份源码的采信条目（若有一条，**且**它的租约仍然成立）。
    specimen: Option<String>,
    /// The adoption entry naming this source whose lease has lapsed, if any.
    /// 点名这份源码但租约已失效的采信条目（若有）。
    lapsed: Option<String>,
    /// Why the declaration could not be parsed, when it could not.
    /// 声明解析不了时的原因。
    unreadable: Option<String>,
}

/// The section one query's repair projection prints for the faces it selected.
/// 一次查询的修复投影为它选中的面打印的那一段。
pub(super) fn section(
    root: &Path,
    sources: &[SourceFile],
    seeds: &BTreeSet<(usize, usize)>,
    faces: &BTreeMap<String, String>,
) -> String {
    let targets: BTreeSet<&str> = seeds
        .iter()
        .filter_map(|&(file, _)| faces.get(&sources[file].relative))
        .map(String::as_str)
        .collect();
    if targets.is_empty() {
        return String::new();
    }
    let views = match crate::mcp::registry::namespace(root)
        .and_then(|namespace| crate::build_method::face_views(root, &namespace))
    {
        Ok(views) => views,
        Err(why) => {
            return format!(
                "references unknown: the registration faces could not be read ({why})\n"
            );
        }
    };
    // A sibling's source comes from the **view**, not from the query's selection: the projection receives the
    // sources it selected, and a sibling that was not selected would otherwise look unreadable — a finding about
    // the query reported as a finding about the tree. `FaceView.source` is relative to the package's identity
    // base, which is the one rule the walk and this page share.
    // 兄弟的源码来自**视图**，而不是这次查询选中的那些：投影收到的是它选中的源码，没被选中的兄弟否则会显得读不出来——
    // 一个关于查询的结论被报成关于树的结论。`FaceView.source` 相对包的身份基准，那是遍历与本页共用的唯一规则。
    let layout = crate::build_method::source_layout(root).ok();
    let signals: BTreeMap<&str, Signals> = views
        .iter()
        .map(|view| (view.path.as_str(), signals_of(root, view, layout.as_ref())))
        .collect();
    let mut output = String::new();
    for target in targets {
        let Some(focus) = signals.get(target) else {
            continue;
        };
        output.push_str(&format!(
            "references {} (scanned {} face(s) for this one section)\n",
            focus.path,
            views.len()
        ));
        let siblings: Vec<&Signals> = signals
            .values()
            .filter(|other| other.path != focus.path && sibling_of(other, focus))
            .collect();
        if siblings.is_empty() {
            output.push_str("  none: no other face shares this face's parent\n");
            continue;
        }
        let mut lines: Vec<(u8, String)> = siblings
            .iter()
            .map(|other| {
                let (rank, label, reason) = verdict(focus, other);
                (rank, format!("  {label}: {} — {reason}\n", other.path))
            })
            .collect();
        lines.sort();
        for (_, line) in &lines {
            output.push_str(line);
        }
        let same_kind = siblings
            .iter()
            .filter(|other| other.kind == focus.kind)
            .count();
        if same_kind > 1 {
            // The ruling's other half: a count is not a role. Several faces sharing one kind is a
            // pattern in the tree, not evidence about what this face must look like.
            // 裁定的另一半：计数不是角色。多个面共用一个 kind 是树里的一种模式，不是关于这个面"应该长成什么样"的证据。
            output.push_str(&format!(
                "  {} face(s) share the kind `{}`; a count is not a role\n",
                same_kind, focus.kind
            ));
        }
        output.push_str(&format!(
            "  signals: the flow contract was read on {} of {} sibling(s); kind is a signal, not a role\n",
            siblings.iter().filter(|other| other.contract.is_some()).count(),
            siblings.len()
        ));
    }
    output
}

/// Whether `other` is a sibling of `focus`: the same resolved parent.
/// `other` 是否是 `focus` 的兄弟：解析到同一个父级。
fn sibling_of(other: &Signals, focus: &Signals) -> bool {
    // The parent identity is not carried here, so the comparison is made by the caller's view: two faces
    // whose logical paths share a prefix up to the last `/` are under one parent, which is the same reading
    // the registry's paths give (`root/control/button` and `root/control/slider` are siblings under
    // `root/control`).
    // 父级身份没有带到这里，因此比较按调用方视图的做法：两个面的逻辑路径到最后一段 `/` 之前的公共前缀相同，就同父——
    // 与注册表路径给出的读法一致（`root/control/button` 与 `root/control/slider` 都在 `root/control` 之下）。
    parent_of(&other.path) == parent_of(&focus.path)
}

/// The logical path without its last segment.
/// 去掉最后一段的逻辑路径。
fn parent_of(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

/// Read one face's signals, from its declaration text and the adoption ledger.
/// 读一个面的信号，来自它的声明原文与采信台账。
fn signals_of(
    root: &Path,
    view: &crate::build_method::FaceView,
    layout: Option<&crate::build_method::SourceLayout>,
) -> Signals {
    let mut signals = Signals {
        path: view.path.clone(),
        source: String::new(),
        kind: view.kind.clone(),
        contract: None,
        capabilities: Vec::new(),
        specimen: None,
        lapsed: None,
        unreadable: None,
    };
    let Some(layout) = layout else {
        signals.unreadable = Some(
            "the package's source layout could not be resolved, so no declaration was read"
                .to_owned(),
        );
        return signals;
    };
    let fact = view.source_ref(layout);
    let source = fact.source.clone();
    signals.source = source.clone();
    let text = match view.locate_source(layout) {
        Ok(path) => match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                signals.unreadable = Some(format!("{source} is not readable ({error})"));
                return signals;
            }
        },
        Err(error) => {
            signals.unreadable = Some(error);
            return signals;
        }
    };
    match crate::build_method::parsed_face(&text, &source) {
        Some(face) => {
            signals.contract = face.field("flow").as_deref().and_then(contract_of);
            let mut capabilities = Vec::new();
            for name in ["exports", "provides"] {
                if let Some(field) = face.field(name) {
                    capabilities.extend(quoted_strings(&field));
                }
            }
            capabilities.sort();
            capabilities.dedup();
            signals.capabilities = capabilities;
        }
        None => signals.unreadable = Some(format!("{} is not a registration face", source)),
    }
    // The adoption ledger, with its **lease verdict**: an entry naming a face whose bytes moved is not a
    // specimen, and saying "adopted" about it would turn a lapsed decision into an argument.
    // 采信台账，连同它的**租约判定**：一条点名了某个面、而那些字节已经动过的条目不是标本；把它说成"已采信"，等于把一次
    // 失效的决定变成一条论据。
    if let Ok(entries) = crate::mcp::adopted::entries(root) {
        for entry in entries {
            if !entry.files.contains(&source) {
                continue;
            }
            let holds = crate::mcp::adopted::read_lease(root, &entry)
                .map(|reading| {
                    matches!(
                        reading.state.verdict,
                        xirang_kernel::adoption::AdoptionVerdict::Provisional
                    )
                })
                .unwrap_or(false);
            if holds {
                signals.specimen.get_or_insert(entry.anchor.clone());
            } else {
                signals.lapsed.get_or_insert(entry.anchor.clone());
            }
        }
    }
    signals
}

/// The flow contract a declaration names, as `<id>@<version>`.
/// 声明点名的 flow 合同，写作 `<id>@<version>`。
///
/// It is read out of the declaration **text** rather than evaluated: the workspace has no evaluator that would
/// resolve `FlowContract::new(ContractId::new("…"), 1, …)`, and inventing one here would be a second rule. The id is
/// the first string literal in the field; the version is the first number after it. A declaration whose field is
/// shaped differently yields `None`, which the verdict reports as "not read" instead of guessing.
/// 它是从声明**原文**读的而不是求值：本工作区没有能解析 `FlowContract::new(ContractId::new("…"), 1, …)` 的求值器，
/// 在这里发明一个就是第二条规则。id 是字段里第一个字符串字面量；版本是它之后的第一个数字。字段形状不同的声明得到
/// `None`，判定会把它报成"没读到"而不是去猜。
fn contract_of(field: &str) -> Option<String> {
    let open = field.find('"')? + 1;
    let close = field[open..].find('"')? + open;
    let id = field[open..close].trim();
    if id.is_empty() {
        return None;
    }
    let version: String = field[close + 1..]
        .chars()
        .skip_while(|character| !character.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    Some(if version.is_empty() {
        id.to_owned()
    } else {
        format!("{id}@{version}")
    })
}

/// Every double-quoted string in one field's text, in order.
/// 某个字段文本里的每一个双引号字符串，按顺序。
fn quoted_strings(field: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut rest = field;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else {
            break;
        };
        let value = after[..close].trim();
        if !value.is_empty() {
            strings.push(value.to_owned());
        }
        rest = &after[close + 1..];
    }
    strings
}

/// One sibling's verdict: its rank (for ordering), the label a reader sees, and the reason.
/// 一个兄弟的判定：它的序号（用于排序）、读者看到的标签，以及理由。
///
/// The ranks keep the strongest signal first and the refusals last, so the reader's eye lands on what the ruling
/// says matters (`same contract`) before the weaker ones.
/// 序号把最强的信号排在前面、把排除排在最后，于是读者的眼睛先落在裁定说重要的那个信号（`same contract`）上。
fn verdict(focus: &Signals, other: &Signals) -> (u8, &'static str, String) {
    if let Some(why) = &other.unreadable {
        return (
            4,
            "unknown",
            format!("{why}, so there is no reason for or against it"),
        );
    }
    let capabilities: BTreeSet<&str> = focus
        .capabilities
        .iter()
        .map(String::as_str)
        .filter(|label| other.capabilities.iter().any(|own| own == label))
        .collect();
    match (&focus.contract, &other.contract) {
        (Some(mine), Some(theirs)) if mine == theirs => {
            let mut reason = format!("same flow contract `{mine}`");
            if !capabilities.is_empty() {
                reason.push_str(&format!(
                    "; same capability labels ({})",
                    capabilities.iter().copied().collect::<Vec<_>>().join(", ")
                ));
            }
            if let Some(anchor) = &other.specimen {
                reason.push_str(&format!("; an adopted specimen ({anchor})"));
            }
            (0, "candidate: same contract", reason)
        }
        (Some(mine), Some(theirs)) => {
            let mut reason = format!("the flow contracts differ (`{mine}` vs `{theirs}`)");
            if focus.kind == other.kind {
                reason.push_str("; the kind is the same, and a kind is not a role");
            }
            if !capabilities.is_empty() {
                reason.push_str(&format!(
                    "; they do share the labels ({}) — check whether the difference is intended",
                    capabilities.iter().copied().collect::<Vec<_>>().join(", ")
                ));
            }
            (3, "not a candidate as written", reason)
        }
        _ => {
            let mut reason = if focus.kind == other.kind {
                "same kind, and neither declaration names a flow contract: a signal, not a proof of role"
                    .to_owned()
            } else {
                "different kind, and the contract is not in the declaration: kind alone does not decide the role"
                    .to_owned()
            };
            if !capabilities.is_empty() {
                reason.push_str(&format!(
                    "; the capability labels match ({})",
                    capabilities.iter().copied().collect::<Vec<_>>().join(", ")
                ));
            }
            if let Some(anchor) = &other.specimen {
                reason.push_str(&format!("; an adopted specimen ({anchor})"));
            }
            if let Some(anchor) = &other.lapsed {
                reason.push_str(&format!(
                    "; an adoption entry names it ({anchor}) but that lease has lapsed, so it is not a specimen"
                ));
            }
            (1, "candidate (weak)", reason)
        }
    }
}

#[cfg(test)]
#[path = "explore_references_tests.rs"]
mod explore_references_tests;
