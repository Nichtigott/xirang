//! Common catalogue metadata, including execution effects and freshness policy discovery.
//! 共用工具目录元数据，包含执行副作用与新鲜度策略的发现。

use serde_json::{Value, json};

/// Construct a tool entry with its shared protocol metadata.
/// 构造工具条目并附上共用协议元数据。
pub(super) fn tool(name: &str, description: &str, mut input_schema: Value) -> Value {
    if reads_build_records(name) && input_schema["properties"].get("freshness").is_none() {
        input_schema["properties"]["freshness"] = json!({"type":"string", "enum":["reuse","verify","reuse-only"],
            "description":"Build-record policy: reuse a labelled content verification for at most 30 seconds; verify checks current content; reuse-only returns unknown when no verification is cached."});
    }
    // The evidence directory follows the build's own target directory, and it is declared here for
    // the same reason `freshness` is: an agent can only pass what the catalogue names. A caller who
    // built with `target_dir` names the same value here and the readers follow the writer — without
    // it they read `<package>/target/xirang/out` whatever the build actually used, which is the
    // 2026-10-10 queue item ⑤, reproduced on 2026-10-13 (report §7h).
    // 证据目录跟着构建自己的 target 目录，它在这里声明与 `freshness` 同一个理由：代理只能传目录点过名的
    // 参数。用 `target_dir` 构建过的调用方在这里点同一个值，读取方就跟着写入方走——没有它，无论构建实际用了
    // 哪个目录，读取方都读 `<包>/target/xirang/out`，这正是 2026-10-10 队列第 ⑤ 条、2026-10-13 复现的那一条
    // （报告 §7h）。
    if reads_the_evidence_dir(name) && input_schema["properties"].get("target_dir").is_none() {
        input_schema["properties"]["target_dir"] = json!({"type":"string",
            "description":"Target directory the build used: records are read from `<target_dir>/xirang/out`; omit for `<package>/target/xirang/out`. Pass what the build ran with, or this answers `derived now`."});
    }
    // `title` and `annotations` are built **here**, in the one constructor every entry goes
    // through, so an entry cannot be added without them — the MCP spec's `annotations` are how a
    // client learns which tools make destructive changes (`readOnlyHint`, `destructiveHint`,
    // `idempotentHint`, `openWorldHint`), and a hint that has to be remembered per entry is a
    // hint that goes missing. Verified: all four hints and a non-empty title on every entry.
    // `title` 与 `annotations` 就建在**这里**——每个条目都要经过的唯一构造器，因此新条目不可能漏掉它们：
    // MCP 规范的 `annotations` 是客户端据以知道"哪些工具会做破坏性改动"的东西（`readOnlyHint`、
    // `destructiveHint`、`idempotentHint`、`openWorldHint`），而一条要靠每条自己记得写的提示，就是一条
    // 会缺席的提示。已验：每个条目四个提示齐全、标题非空。
    json!({
        "name": name,
        "title": title(name),
        "description": description,
        "inputSchema": input_schema,
        "annotations": annotations(name),
    })
}

/// Whether a tool answers from the build's published records — the list that decides who is offered
/// the `freshness` policy.
/// 一个工具是否从构建发布的记录作答——这份清单决定谁被提供 `freshness` 策略。
///
/// It is deliberately **not** the list for `target_dir`: that one names the tools whose handlers read
/// the evidence directory ([`reads_the_evidence_dir`]), and the two are not the same list. `explore`
/// carries the policy and does not read `target_dir` at all (its questions are answered from source
/// and from joins), and the advertised frame has roughly twenty characters of headroom — declaring an
/// argument a tool cannot use would spend the reader's first screenful on nothing.
/// 这份清单**有意不**用于 `target_dir`：那个用的是"处理函数真的读证据目录"的清单
/// （[`reads_the_evidence_dir`]），两者不是同一份。`explore` 带着策略却完全不读 `target_dir`（它的问题由
/// 源码与连接作答），而广告帧只剩二十来个字符的余量——为一个工具用不上的参数花掉读者的第一屏是不值的。
fn reads_build_records(name: &str) -> bool {
    matches!(
        name,
        "xirang.explore"
            | "xirang.search"
            | "xirang.registry"
            | "xirang.explain"
            | "xirang.diff"
            | "xirang.trace"
            | "xirang.grafts"
            | "xirang.impact"
            | "xirang.usages"
            | "xirang.converge"
            | "xirang.consistency"
            | "xirang.verify"
    )
}

/// Whether a tool's handler reads `build_evidence::out_dir` — the list that decides who is offered
/// `target_dir`.
/// 一个工具的处理函数是否读 `build_evidence::out_dir`——这份清单决定谁被提供 `target_dir`。
///
/// Every name here also appears in [`reads_build_records`]: a tool cannot be told where the records
/// are without also being told how much verification to pay for. The reverse does not hold
/// (`explore`), which is why they are two lists rather than one.
/// 这里每个名字也出现在 [`reads_build_records`] 里：一个工具不可能被告知记录在哪、却不知道愿意为核验付
/// 多少。反过来不成立（`explore`），因此这是两份清单而不是一份。
fn reads_the_evidence_dir(name: &str) -> bool {
    matches!(
        name,
        "xirang.search"
            | "xirang.registry"
            | "xirang.explain"
            | "xirang.diff"
            | "xirang.trace"
            | "xirang.grafts"
            | "xirang.impact"
            | "xirang.usages"
            | "xirang.converge"
            | "xirang.consistency"
            | "xirang.verify"
    )
}

/// What a tool does to the tree, as `annotations` discloses it.
/// 一个工具对这棵树做了什么，按 `annotations` 披露的那样。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Effect {
    /// Reads only.
    /// 只读。
    Read,
    /// Adds something that was not there, and removes nothing.
    /// 增添原本不存在的东西，且不移除任何东西。
    Add,
    /// Rewrites or removes what is there.
    /// 改写或移除已有的东西。
    Rewrite,
}

/// The effect of one tool.
/// 一个工具的效果。
///
/// Shared effect classification includes test execution and generated build records.
/// 共用副作用分类包含测试执行与构建记录生成，不把验证误标为只读。
fn effect(tool: &str) -> Effect {
    match tool {
        // `plugin` appends (and promotes) a catalog record: it adds, and removes nothing.
        // `plugin` 追加（并升级）一条目录记录：它只增添，不移除任何东西。
        "xirang.new_project" | "xirang.plugin" | "xirang.adopted" => Effect::Add,
        // `apply` can move a subtree into the trash (`delete`), which is the one action here
        // that takes something away from the caller.
        // `apply` 能把一棵子树移进回收目录（`delete`）——这是这里唯一一个把东西从调用方手里拿走的动作。
        "xirang.apply" | "xirang.crates" | "xirang_tools" | "xirang.check" | "xirang.verify"
        | "xirang.mir" | "xirang.candidate" | "xirang.rules" | "xirang.refactor"
        | "xirang.task" => Effect::Rewrite,
        _ => Effect::Read,
    }
}

/// Only read tools may reuse request-local filesystem and Cargo metadata snapshots.
/// 只有只读工具可复用请求内文件系统及 Cargo 元数据快照。
pub(super) fn shares_reads(tool: &str) -> bool {
    effect(tool) == Effect::Read
}

/// The MCP annotations for one tool.
/// 一个工具的 MCP `annotations`。
///
/// `destructiveHint` is set **explicitly on every tool** rather than left to the protocol's
/// default (which is `true`): a read tool that omits it is telling a client the opposite of the
/// truth, and `apply` really does move subtrees into the trash, so it says `true` while
/// `new_project` says `false` — additive and destructive are not the same permission.
/// `destructiveHint` **每个工具都显式写出**，而不是留给协议默认值（默认是 `true`）：一个省略它的读工具
/// 等于对客户端说了反话；而 `apply` 确实会把子树移进回收目录，因此它写 `true`，`new_project` 写
/// `false`——"新增"与"破坏"不是同一种许可。
fn annotations(tool: &str) -> Value {
    let read_only = effect(tool) == Effect::Read;
    json!({
        "readOnlyHint": read_only,
        // `destructiveHint` is about taking something away, not about writing. Among the writers only
        // `apply` can do that — its `delete` moves a subtree into the trash. `refactor` rewrites
        // declaration fields and removes nothing, so saying `true` for it would overstate the
        // permission in the one direction a client acts on.
        // `destructiveHint` 说的是"把东西拿走"，不是"会写"。写入方里只有 `apply` 能那样——它的 `delete`
        // 会把整棵子树移进回收目录。`refactor` 改写声明字段、不移除任何东西，对它说 `true` 会在客户端唯一
        // 据以行动的方向上夸大许可。
        "destructiveHint": matches!(effect(tool), Effect::Rewrite)
            && !matches!(tool, "xirang.refactor" | "xirang.task"),
        "idempotentHint": read_only,
        "openWorldHint": matches!(
            tool,
            "xirang.check" | "xirang.mir" | "xirang_tools" | "xirang.candidate"
        ),
    })
}

/// A human-readable title for one tool, derived from its name.
/// 一个工具的人类可读标题，由它的名字推导。
///
/// Derived rather than written per entry: a title is a display name, and a second list of
/// eighteen names is a second list that drifts.
/// 推导而不是逐条写：标题是显示名，而第二份十八个名字的清单就是一份会漂移的清单。
fn title(tool: &str) -> String {
    let bare = tool.strip_prefix("xirang.").unwrap_or(tool);
    let mut spaced = bare.replace('_', " ");
    if let Some(first) = spaced.get(0..1) {
        spaced.replace_range(0..1, &first.to_uppercase());
    }
    spaced
}

#[cfg(test)]
mod metadata_tests {
    use super::tool;
    use serde_json::json;

    /// The two request-level arguments a build-record reader takes travel together.
    /// 读取构建记录的工具所接受的两个请求级参数是成对的。
    ///
    /// `freshness` says how much content verification this call may pay for; `target_dir` says which
    /// directory the build published into. A reader with the first and not the second reads
    /// `<package>/target/xirang/out` whatever directory the build actually used and silently answers
    /// `derived now` forever — the 2026-10-10 queue item ⑤, reproduced on 2026-10-13 through the
    /// bridge (report §7h) — while a reader with neither has no build records at all. Both are
    /// decided by one list, [`super::reads_build_records`], so this pin is about that list as much as
    /// about the constructor.
    /// `freshness` 说这次调用可以为内容核验付多少；`target_dir` 说构建发布到了哪个目录。只有前者、
    /// 没有后者的读取方，无论构建实际用了哪个目录都去读 `<包>/target/xirang/out`，并永远静默地回答
    /// `derived now`——2026-10-10 队列第 ⑤ 条，2026-10-13 用真桥复现（报告 §7h）——而两者都没有的读取方
    /// 根本没有构建记录。两者由同一份清单决定，即 [`super::reads_build_records`]，因此这条钉子既关于那个
    /// 构造器，也关于那份清单。
    #[test]
    fn the_evidence_arguments_travel_together_and_only_where_they_are_read() {
        let schema = || json!({"type": "object", "properties": {}});
        let reads = tool("xirang.registry", "a reader", schema());
        let properties = &reads["inputSchema"]["properties"];
        assert!(
            properties.get("freshness").is_some() && properties.get("target_dir").is_some(),
            "an evidence reader advertises both: {reads}"
        );
        // `explore` takes the policy and no path: it reads no evidence directory, and the advertised
        // frame is a screenful with about twenty characters to spare.
        // `explore` 只要策略、不要路径：它不读任何证据目录，而广告帧是一屏、只剩二十来个字符的余量。
        let explore = tool("xirang.explore", "a reader of sources", schema());
        let properties = &explore["inputSchema"]["properties"];
        assert!(
            properties.get("freshness").is_some() && properties.get("target_dir").is_none(),
            "explore takes the policy and no path: {explore}"
        );
        let other = tool("xirang.check", "not an evidence reader", schema());
        let properties = &other["inputSchema"]["properties"];
        assert!(
            properties.get("freshness").is_none() && properties.get("target_dir").is_none(),
            "and a tool that reads no records advertises neither: {other}"
        );
    }
}
