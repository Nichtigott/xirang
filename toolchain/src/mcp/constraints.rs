//! Constraint admission: decide an expectation against what the tree actually declares.
//! 约束准入：拿一条期望去对树真正写下来的声明做判定。
//!
//! This is the narrow, decidable half of W3. It answers one question — **do the declarations this
//! tree writes down carry what the caller expects?** — and it answers it in four words, of which
//! only one is a pass:
//! 这是 W3 里窄而可判定的那一半。它只回答一个问题——**这棵树写下来的声明是否载有调用方期望的东西**
//! ——并且只用四个词回答，其中只有一个是通过：
//!
//! * `met` — a declaration in this node's own file carries exactly the expected value, quoted with
//!   its line so the reader can open it;
//! * `unmet` — the file was read and nothing in it carries the expectation; the reply prints what
//!   the node **does** declare, so the gap is visible rather than asserted;
//! * `unknown` — the expectation cannot be decided here at all (a behavioural claim such as "this
//!   method must call that provider", or a declaration this parser cannot attribute);
//! * `invalid` — the expectation itself is malformed, which is a caller error and not a fact about
//!   the tree.
//! * `met` —— 该节点自己的文件里有一条声明正好载有期望的值，连同行号引出来供读者打开；
//! * `unmet` —— 文件读到了，里面没有任何东西载有该期望；回复印出这个节点**确实**声明了什么，使缺口可见
//!   而不是被断言；
//! * `unknown` —— 这条期望在这里根本无法判定（"这个方法必须调用那个提供者"这类行为性主张，或本解析器
//!   归因不了的声明）；
//! * `invalid` —— 期望本身是畸形的，那是调用方的错误，不是关于树的事实。
//!
//! **What this tool refuses to do is the point of it.** A capability requirement is not a statement
//! that every method must call the provider (`.requires` only expresses a need); a majority of
//! siblings is a candidate and not a verdict; and an expectation the tree does not support is a
//! **conflict between the expectation and the tree** — this tool never edits the business to make
//! the expectation true, and never reports an undecidable claim as satisfied.
//! **这个工具拒绝做的事才是它的要点。** 能力需求不等于"每个方法都必须调用该提供者"（`.requires` 只
//! 表达能力需求）；兄弟的多数是候选而不是判决；而树不支持的期望是**期望与树之间的冲突**——这个工具绝不
//! 为了让期望成真去改业务，也绝不把判不了的主张报成已满足。

use std::path::Path;

use serde_json::Value;

use xirang_kernel::syntax::FaceSyntax;

/// The expectation keys this tool can decide from the declarations alone.
/// 本工具仅凭声明就能判定的那些期望键。
const DECIDABLE: &[&str] = &["declares", "provides", "requires"];

/// Decide one expectation against one node's own declaration.
/// 拿一条期望去对这个节点自己的声明做判定。
pub(super) fn constraints(root: &Path, arguments: &Value) -> Result<String, String> {
    let node = arguments
        .get("node")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|node| !node.is_empty())
        .ok_or_else(|| {
            "`node` names the object the expectation is about (a logical path, a module, or an \
             identity); `constraints` decides expectations against one node's own declaration"
                .to_owned()
        })?;
    let expect = arguments
        .get("expect")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "`expect` is required: `{declares: \"<field>\", value: \"<text>\"}`, `{provides: \
             \"<capability>\"}`, or `{requires: \"<capability>\"}`; anything else is reported as \
             unknown rather than met"
                .to_owned()
        })?;
    // Which way this expectation is supposed to go. A **negative** control states something the
    // tree must *not* carry, so the same fact means the opposite thing for it — and running a
    // control is the only way to know whether a check can fail at all.
    // 这条期望本该往哪个方向走。**反控制**声明的是这棵树**不该**载有的东西，因此同一个事实对它意味着
    // 相反的结论——而只有真的跑一次控制，才知道一道检查到底能不能红。
    let negative = match arguments.get("control").and_then(Value::as_str) {
        None | Some("positive") => false,
        Some("negative") => true,
        Some(other) => {
            return Err(format!(
                "`control` is `positive` or `negative`, not `{other}`; nothing was decided"
            ));
        }
    };
    for key in expect.keys() {
        // `value` is the companion of `declares`, not a claim of its own.
        // `value` 是 `declares` 的同伴，不是一条独立主张。
        if key != "value" && !DECIDABLE.contains(&key.as_str()) {
            // An expectation outside the decidable set still gets the control line and the blame
            // line: the one case that must never read as a passing control is the one that cannot
            // be decided at all.
            // 落在可判定集合之外的期望同样拿到控制行与不判罪行：最不该被读成"控制通过"的，正是那条根本
            // 判不了的期望。
            return Ok(undecidable(node, key, expect, negative));
        }
    }
    let namespace = crate::mcp::registry::namespace(root)?;
    let views = crate::build_method::face_views(root, &namespace)?;
    let view = views
        .iter()
        .find(|view| view.path == node || view.module == node || view.id.to_string() == node)
        .ok_or_else(|| {
            format!(
                "no face in this tree has the path, module or identity `{node}`; `registry` lists \
                 them and `explain {{node}}` answers for one"
            )
        })?;
    let layout = crate::build_method::source_layout(root)?;
    let file = view.locate_source(&layout)?;
    let text = std::fs::read_to_string(&file)
        .map_err(|error| format!("{} is not readable: {error}", file.display()))?;
    let face = crate::build_method::parsed_face(&text, &view.source);
    let declared = face.as_ref().map(declared_lines).unwrap_or_default();
    let reply = match &face {
        Some(face) => decide(face, expect),
        None => Verdict::Unknown(format!(
            "{} does not parse as a registration face, so its declarations cannot be attributed",
            view.source
        )),
    };
    let mut out = format!(
        "constraint {} ({})\nnode  {}\nfile  {}\nexpect {}",
        reply.word(),
        reply.kind(),
        view.path,
        view.source,
        expectation_text(expect)
    );
    if !declared.is_empty() {
        out.push_str("\ndeclared");
        for line in &declared {
            out.push_str(&format!("\n  {line}"));
        }
    }
    match &reply {
        Verdict::Unmet { field, found } => out.push_str(&format!(
            "\nnot met because: `{field}` is {found} here, and the expectation asks for something \
             else — the gap is between the expectation and this file, and only a person decides \
             which of the two moves"
        )),
        Verdict::Unknown(reason) => out.push_str(&format!("\nnot decided: {reason}")),
        Verdict::Met => {}
    }
    out.push_str(&control_line(&reply, negative));
    out.push_str(&format!("\ncause  {}", cause(&reply)));
    out.push_str(BLAME);
    out.push_str(&format!(
        "\nnext   `explain {{node:\"{}\"}}` for the whole declaration, `consistency \
         {{parent:\"{}\"}}` for how its siblings differ",
        view.path,
        parent_of(&view.path)
    ));
    Ok(out)
}

/// What one expectation came back as.
/// 一条期望判定回来的结果。
enum Verdict {
    /// A declaration carries exactly the expected value.
    /// 有一条声明正好载有期望的值。
    Met,
    /// The file was read and nothing carries the expectation.
    /// 文件读到了，没有任何东西载有该期望。
    Unmet {
        /// The field the expectation named.
        /// 期望点名的字段。
        field: String,
        /// What that field holds here, or that it is absent.
        /// 该字段在这里的取值，或它并不存在。
        found: String,
    },
    /// The expectation cannot be decided from declarations.
    /// 仅凭声明判不了这条期望。
    Unknown(String),
}

impl Verdict {
    /// The one word a caller branches on.
    /// 调用方据以分支的那一个词。
    fn word(&self) -> &'static str {
        match self {
            Self::Met => "met",
            Self::Unmet { .. } => "unmet",
            Self::Unknown(_) => "unknown",
        }
    }

    /// The kind of rule this verdict was about.
    /// 这条判决针对的是哪一类规则。
    fn kind(&self) -> &'static str {
        match self {
            Self::Met | Self::Unmet { .. } => "structural",
            Self::Unknown(_) => "undecided",
        }
    }
}

/// The answer an expectation outside the decidable set gets, whatever it asks for.
/// 落在可判定集合之外的期望得到的回答，无论它问的是什么。
fn undecidable(
    node: &str,
    key: &str,
    expect: &serde_json::Map<String, Value>,
    negative: bool,
) -> String {
    let reason = format!(
        "`{key}` is not a declaration this tool reads; it decides `declares`/`provides`/`requires` \
         — what the file writes down — and a behavioural claim has no independent oracle here. A \
         capability requirement is not a statement that every method must call the provider, and a \
         majority of siblings is a candidate rather than a verdict, so this comes back unknown \
         instead of met"
    );
    format!(
        "constraint unknown (undecided)\nnode  {node}\nexpect {}\nnot decided: {reason}{}{}{}\n\
         next   `converge {{node:\"{node}\"}}` for the declared neighbourhood, `callgraph \
         {{node:\"{node}\"}}` for the calls this tree spells out (lexical, not coverage)\n",
        expectation_text(expect),
        control_line(&Verdict::Unknown(reason.clone()), negative),
        "\ncause  unknown",
        BLAME
    )
}

/// Decide one expectation against one node, and hand back a **typed** answer.
/// 拿一条期望去对某一个节点做判定，并交回一份**类型化**的答案。
///
/// The tool's reply is prose for a reader; a rule book needs something it can branch on. Both go
/// through `decide`, so the rule book cannot drift from the tool that renders the same verdict —
/// which is the whole reason §2.1 asks a rule to *reference* an existing check.
/// 工具的回复是给读者看的散文；规则簿需要能据以分支的东西。两者都经过 `decide`，因此规则簿不会与渲染同一个
/// 判决的那件工具漂移——这正是 §2.1 要求规则**引用**既有检查的理由。
/// One expectation's answer, with the reason an undecidable one carries.
/// 一条期望的答案，并带上判不了时的原因。
///
/// `decide_expectation` loses the reason because its callers only branch on a word; the rule book
/// needs to print *why* it cannot decide, so both readings come from this one function — the shared
/// decider, not a second interpretation of the same expectation.
/// `decide_expectation` 丢掉原因，因为它的调用方只按一个词分支；规则簿需要印出**为什么**判不了，因此两种
/// 读法都出自这一个函数——共享的判定器，而不是同一期望的第二份解释。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ExpectationAnswer {
    /// The declaration carries exactly what was expected.
    /// 声明正好载有期望之物。
    Satisfied,
    /// The file was read and nothing carries it.
    /// 文件读到了，但没有任何东西载有它。
    NotSatisfied { field: String, found: String },
    /// The expectation cannot be decided from declarations, and this is why.
    /// 仅凭声明判不了这条期望，原因在此。
    Unknown(String),
}

pub(super) fn decide_expectation_detail(
    root: &Path,
    node: &str,
    expect: &serde_json::Map<String, Value>,
) -> Result<ExpectationAnswer, String> {
    if node.trim().is_empty() {
        return Err("a rule that names no node cannot be decided against one".to_owned());
    }
    let namespace = crate::mcp::registry::namespace(root)?;
    let views = crate::build_method::face_views(root, &namespace)?;
    let view = views
        .iter()
        .find(|view| view.path == node || view.module == node || view.id.to_string() == node)
        .ok_or_else(|| {
            format!("no face in this tree has the path, module or identity `{node}`, so the rule cannot be decided")
        })?;
    let layout = crate::build_method::source_layout(root)?;
    let file = view.locate_source(&layout)?;
    let text = std::fs::read_to_string(&file)
        .map_err(|error| format!("{} is not readable: {error}", file.display()))?;
    let face = crate::build_method::parsed_face(&text, &view.source);
    let verdict = match &face {
        Some(face) => decide(face, expect),
        None => Verdict::Unknown(format!(
            "{} does not parse as a registration face, so its declarations cannot be attributed",
            view.source
        )),
    };
    Ok(match verdict {
        Verdict::Met => ExpectationAnswer::Satisfied,
        Verdict::Unmet { field, found } => ExpectationAnswer::NotSatisfied { field, found },
        Verdict::Unknown(reason) => ExpectationAnswer::Unknown(reason),
    })
}

/// Decide a parsed face against a decidable expectation.
/// 拿一个已解析的注册面去对一条可判定的期望做判定。
fn decide(face: &FaceSyntax, expect: &serde_json::Map<String, Value>) -> Verdict {
    let Some((key, wanted)) = expect.iter().next() else {
        return Verdict::Unknown("the expectation is empty".to_owned());
    };
    let wanted = match wanted.as_str() {
        Some(text) if !text.trim().is_empty() => text.trim(),
        _ => {
            return Verdict::Unknown(format!(
                "`{key}` must be a non-empty string; nothing was decided"
            ));
        }
    };
    let field = match key.as_str() {
        "declares" => {
            // `declares` names the field; its value rides beside it, so a missing one is a
            // malformed expectation rather than a fact about the tree.
            // `declares` 点名的是字段；它的取值就在旁边，因此缺了它是期望畸形，而不是关于树的事实。
            let Some(name) = expect.get("value").and_then(Value::as_str) else {
                return Verdict::Unknown(
                    "`declares` needs a sibling `value` naming what the field should hold"
                        .to_owned(),
                );
            };
            return match declared_value(face, wanted) {
                None => Verdict::Unmet {
                    field: wanted.to_owned(),
                    found: "absent".to_owned(),
                },
                Some(found) if holds(face, wanted, name) => {
                    let _ = found;
                    Verdict::Met
                }
                Some(found) => Verdict::Unmet {
                    field: wanted.to_owned(),
                    found: format!("`{found}`"),
                },
            };
        }
        other => other,
    };
    match declared_value(face, field) {
        None => Verdict::Unmet {
            field: field.to_owned(),
            found: "absent".to_owned(),
        },
        Some(_) if holds(face, field, wanted) => Verdict::Met,
        Some(found) => Verdict::Unmet {
            field: field.to_owned(),
            found: format!("`{found}`"),
        },
    }
}

/// One field as a reader would say it: the list entries when it is list-valued, else the scalar.
/// 一个字段按读者会说出来的样子：列表值给条目，否则给标量。
fn declared_value(face: &FaceSyntax, field: &str) -> Option<String> {
    face.string_list(field)
        .map(|values| values.join(", "))
        .or_else(|| face.field(field).map(|value| value.trim().to_owned()))
}

/// Whether the field carries exactly this entry.
/// 该字段是否正好载有这一条。
///
/// List-valued fields are matched per entry, so `cap.render=>Button` is one value and not four.
/// 列表值按条目匹配，因此 `cap.render=>Button` 是一个取值而不是四个。
fn holds(face: &FaceSyntax, field: &str, wanted: &str) -> bool {
    match face.string_list(field) {
        Some(values) => values.iter().any(|entry| entry.trim() == wanted.trim()),
        None => face
            .field(field)
            .is_some_and(|value| value.trim() == wanted.trim()),
    }
}

/// The declaration lines a reader needs to see the gap.
/// 读者看懂缺口所需的那些声明行。
fn declared_lines(face: &FaceSyntax) -> Vec<String> {
    let mut lines = Vec::new();
    for name in ["kind", "provides", "requires", "exports"] {
        if let Some(value) = declared_value(face, name) {
            lines.push(format!("{name:<13} {value}"));
        }
    }
    lines
}

/// The expectation as the reply quotes it.
/// 回复引用的那条期望。
fn expectation_text(expect: &serde_json::Map<String, Value>) -> String {
    let mut parts = Vec::new();
    for (key, value) in expect {
        parts.push(format!(
            "{key} {}",
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned)
        ));
    }
    parts.join(" ")
}

/// Whether the control this expectation was run as passed, and what that means.
/// 这条期望按当前方向跑出来的控制是通过还是失败，以及它意味着什么。
///
/// The fact and its direction stay separate on purpose: `met` is a fact about the file, and
/// whether that is good news depends on which way the expectation was aimed. A direction the tool
/// cannot decide from declarations is `unknown` under **both** directions, never a passing control.
/// 事实与方向有意分开：`met` 是关于文件的事实，而它算不算好消息取决于期望瞄准的是哪一边。仅凭声明判不了
/// 的方向，在**两个**方向下都是 `unknown`，绝不算控制通过。
fn control_line(verdict: &Verdict, negative: bool) -> String {
    let direction = if negative { "negative" } else { "positive" };
    match verdict {
        Verdict::Unknown(_) => format!(
            "\ncontrol {direction} unknown — not a control: an undecidable expectation cannot pass \
             in either direction"
        ),
        Verdict::Met | Verdict::Unmet { .. } => {
            let holds = matches!(verdict, Verdict::Met);
            let passed = holds != negative;
            format!(
                "\ncontrol {direction} {} — control {}",
                verdict.word(),
                if passed { "passed" } else { "failed" }
            )
        }
    }
}

/// What this tool refuses to decide, printed on every answer.
/// 这个工具拒绝判定什么，印在每一条答复上。
///
/// A08's rule, made part of the promise instead of a footnote: a legality check and an
/// implementation defect are **different causes**, and which one a disagreement is cannot be read
/// off who wrote the test — nor off this tool, which reads declarations and not test intent. Saying
/// so on every answer is what keeps a reader from turning `unmet` into "your business is wrong".
/// A08 的规则，写进承诺而不是脚注：合法性检查与实现缺陷是**两种不同的原因**，而一次分歧属于哪一种，
/// 既读不出"谁写的测试"，也读不出这个工具——它读声明，不读测试的意图。每一条答复都这么说，读者才不会把
/// `unmet` 读成"你的业务错了"。
const BLAME: &str = "\nblame  undecided here — this reads declarations, not test intent: a failed \
                     expectation means the test and the tree disagree, and which of the two is out \
                     of date is not something this tool decides";

/// The parent path of a logical path, for the "what next" line.
/// 逻辑路径的父路径，供"下一步"那行使用。
fn parent_of(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| "root".to_owned(), |(parent, _)| parent.to_owned())
}

#[cfg(test)]
#[path = "constraints_tests.rs"]
mod constraints_tests;

/// Which of the two causes a disagreement **could** be, named as a candidate.
/// 一次分歧**可能**属于两种原因里的哪一种，作为候选点名。
///
/// A08 asks for `constraint_invalid` (a legitimate implementation paired with a wrong test) and
/// `implementation_violation` (a correct test against a wrong implementation) to be reported
/// separately — and for the tool never to decide which from who wrote the test. What this tool can
/// read is narrower and enough: whether the tree's own declarations carry the expectation. It does
/// **not** carry it ⇒ the test and the tree disagree about the contract; it does carry it ⇒ the
/// disagreement lies on the implementation side. Both are printed as `(candidate)` for the same
/// reason the blame line exists: a reader who takes either for a verdict has been handed a decision
/// this tool never made.
/// A08 要求把 `constraint_invalid`（合法实现配了错的测试）与 `implementation_violation`（正确的测试配了错的
/// 实现）**分别**报出，并且绝不由"谁写的测试"来定案。这个工具能读到的更窄，却也够用：这棵树自己的声明是否载有
/// 该期望。**不载有** ⇒ 测试与树对合同的看法不一致；**载有** ⇒ 分歧落在实现那一侧。两者都印成 `(candidate)`，
/// 理由与那行"不判罪"相同：把它当成判决的读者，等于被交给了一个这个工具从没做过的决定。
fn cause(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Met => "implementation_violation (candidate)",
        Verdict::Unmet { .. } => "constraint_invalid (candidate)",
        Verdict::Unknown(_) => "unknown",
    }
}
