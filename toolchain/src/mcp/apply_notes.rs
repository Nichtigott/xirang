//! What a write tells its caller about the change it made (audits `W5-3`, `W5-6`, `W5-7`).
//! 一次写入告诉调用方它做了什么改动（审计 `W5-3`、`W5-6`、`W5-7`）。
//!
//! Split out of `apply.rs` when that file crossed the 600-code-line ratchet. The report's job is to
//! render the executor's facts; these are the facts that are *about the caller's decision* rather than
//! about the change itself, and they belong together because they are read in one place.
//! 在 `apply.rs` 越过 600 行代码行棘轮时拆出来。报告的工作是渲染执行器的事实；而这些事实是**关于调用方
//! 那个决定**的、不是关于这次改动本身的，它们应当放在一起，因为读者在一个地方读它们。

use super::Outcome;

/// Every note that rides on a write reply, in the order they are read.
/// 搭在写入回复上的全部注记，按读者阅读的顺序。
pub(super) fn notes(outcome: &Outcome, applied: bool) -> String {
    let mut text = String::new();
    // `W5-6`'s inheritance and `W5-3`'s consumer story ride in **both** modes: a preview that told a
    // different story from the one that lands would be an advertisement rather than a preview.
    // `W5-6` 的继承与 `W5-3` 的消费方说法**两种模式都带**：一个与落盘说法不同的预览是广告而不是预览。
    for line in outcome.inherited.iter().chain(outcome.consumers.iter()) {
        text.push_str(line);
        text.push('\n');
    }
    if outcome.alternative && applied {
        // Audit `W5-7`: the two blocks below are a **decision aid** — they belong where the decision is
        // still open. Once the write has happened the caller has already chosen, so repeating the whole
        // pricing (and the unit-struct note, which the diff already shows) is payload the round
        // measured arriving in every deepen reply for nothing.
        // 审计 `W5-7`：下面两块是**决策辅助**——它们属于"决定还没做"的那一刻。写入一旦发生，调用方已经
        // 选过了，因此把整段价钱（以及 diff 已经显示的单位结构体提示）再念一遍，正是那一轮量到的"每次
        // deepen 回复里白来的载荷"。
        text.push_str(
            "alternative was stated in preview; the layer above leaves the tree, the public path, \
             the slot and the factory pins alone\n",
        );
    }
    if outcome.alternative && !applied {
        // The other reading of "make it deeper", priced. Family b of the round measured that this
        // decision is what costs the iterations — the tree, the public path, the slot and four families
        // of pins all move together — so the write path states both readings and lets the caller choose
        // instead of letting a red run do the arithmetic.
        // "把它做深"的另一种读法，连同它的价钱。五族 b 量到的正是：这个决定才是迭代成本所在——树、公开
        // 路径、槽位与四类钉子是一起动的——因此写入路径把两种读法都摆出来让调用方选，而不是让一次红运行去做
        // 这道算术。
        text.push_str(
            "alternative (the other reading of `make it deeper`): hang another face **under** this \
             one — that needs `needs_registry: true` plus its own `registration_rule`, adds a row \
             per child to the registration tree, adds a segment to this face's public module path, \
             forces its graft slot to `full graft`, and moves the factory-shape assertions that \
             count faces, scope rows and slots; the layer above was measured to leave all of them \
             alone\n",
        );
        // The one shape change this action *does* make, said before the write: the face stops being a
        // unit struct. An independent review of the first cut listed it as a boundary, and a consequence
        // a caller has to discover by compiling is exactly what this block exists to prevent.
        // 这个动作**确实**会带来的一处形状变化，写在写之前：这个面不再是单位结构体。第一版的独立复核把它
        // 列为一条边界，而"要编译一次才知道的后果"正是这一节要消灭的东西。
        text.push_str(
            "note   this face stops being a unit struct: anything that used the type as a value \
             (`let _ = Kind;`, a literal pattern) must be updated by hand; this action writes the \
             face's own file and not its callers\n",
        );
    }
    text
}
