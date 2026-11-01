//! What this bridge is allowed to claim about the base tree, in one line a reply can print.
//! 这座桥就基树**可以声称**什么，写成回复能印的一行。
//!
//! The maintainer's ruling (§2) names two capabilities and forbids blurring them: a **plain MCP** checks what it
//! lands and *cannot* stop another tool or process from writing the base tree, while **managed execution** is the
//! only shape in which "unauthorized writes are prevented" may be said out loud — and only when the isolation
//! conditions actually hold. This bridge is the plain one: the controller owns the approval store, but nothing here
//! runs the publish loop that would let it own the base tree's writes.
//! 维护者的裁定（§2）点名两种能力，并禁止把它们混为一谈：**普通 MCP** 检查自己落地的东西，但**阻止不了**别的工具或进程写
//! 基树；只有**托管执行**才可以说出"越权写入已被阻止"，而且只在隔离条件真的成立时。这座桥是普通的那一种：控制器拥有批准
//! 存储，但这里没有任何东西跑那条让控制器拥有基树写入的发布环。
//!
//! The line therefore states three things at once — which capability, what it does check, and what it does not
//! protect — because a reader who only gets the first has been told a promotion, and a reader who only gets the
//! third has been told nothing usable.
//! 因此这一行同时说三件事——哪种能力、它确实检查什么、它不保护什么——因为只拿到第一件的读者得到的是宣传，只拿到第三件的
//! 读者得到的是一句没用的话。

use std::path::Path;

/// The capability this bridge runs at. It is a fixed value rather than a flag: a caller-supplied "managed" would be
/// exactly the kind of self-declaration the ruling forbids, and the bridge cannot verify isolation it does not
/// implement.
/// 这座桥运行在哪种能力上。它是一个固定值而不是开关：调用方提供的 "managed" 正是裁定禁止的那种自我声明，而这座桥验证不了
/// 它并未实现的隔离。
pub(super) const LEVEL: &str = "plain-mcp";

/// The one line a reply prints where a reader could otherwise over-read a landing.
/// 回复在读者可能把一次落地读过头的地方印的那一行。
pub(super) fn line(root: &Path) -> String {
    // The **approval store** is named in the same breath as the base tree, because it is the other half of the same
    // boundary and the easier one to over-read: it sits outside the root, it is called "the controller's", and a
    // reader would reasonably assume it is protected. It is not: its digest is a corruption check, not a signature, so
    // a process that can write files can forge a grant exactly as it can write the base tree. Measured 2026-10-18
    // (`controller_tests::a_hand_written_store_is_accepted_because_the_digest_is_not_a_signature`).
    // **批准存储**与基树在同一句里点名，因为它是同一条边界的另一半、而且更容易被读过头：它在根之外、被称为"控制器的"，
    // 读者会合理地以为它受保护。并不是：它的摘要是防损坏检查而不是签名，因此能写文件的进程可以伪造一份授权，正如它能直接写
    // 基树。2026-10-18 实测（`controller_tests::a_hand_written_store_is_accepted_because_the_digest_is_not_a_signature`）。
    format!(
        "capability {LEVEL}: this bridge checks the scope, the verification and the approval of what it lands, and \
         it cannot prevent another tool or process from writing {} directly — nor the approval store beside it, \
         whose digest is a corruption check rather than a signature, and a process that can allocate a \
         pseudo-terminal can answer the approval prompt itself, so an approval records a claim made through a \
         terminal rather than proof that a person was at it — do not read this reply as \"unauthorized \
         writes are prevented\": that claim belongs to managed execution, and this landing does not run inside \
         it — `xirang-mcp --managed --root PATH --writable PATH -- <command>` runs a command in a write domain \
         the kernel enforces",
        root.display()
    )
}

/// Why the stronger capability is **not** claimed, in the terms a reader can check.
/// 为什么**不**声称更强的能力，用读者能核对的说法。
///
/// A rule that only says "we do not claim it" invites the next edit to claim it by accident. Naming the missing piece
/// makes the boundary checkable: a write domain is established around a command started under `--managed`, and a
/// landing is not such a command — so the guarantee the domain provides cannot be claimed for the write that lands.
/// A controller that owned the publish loop would be the other way to make the claim true; the controller owns only
/// the approval store.
/// 只写"我们不声称"的规则，会引来下一次改动不小心替它声称。点名缺的那一块，边界就变成可核对的：写域是围绕在 `--managed`
/// 下启动的命令建立的，而落地不是这样的命令——因此写域提供的保证不能替那次落地声称。让控制器拥有发布环是另一条让那句话成真的路；
/// 今天控制器只拥有批准存储。
pub(super) fn missing_for_managed() -> &'static str {
    "the controller owns the approval store; a write domain exists only for a command started under `--managed`, \
     and a landing runs outside it, so the isolation managed execution claims does not cover the write that lands"
}
