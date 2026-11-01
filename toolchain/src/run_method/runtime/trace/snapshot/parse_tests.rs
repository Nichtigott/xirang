//! The intern table's ceiling: the parser may not grow the process without one.
//! 驻留表的天花板：解析器不得让进程没有天花板地变大。

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{INTERN_LIMIT, InternTable, TraceArtifact, intern_into, interned_len};
use crate::run_method::registry_core::identity::NodeId;

/// A name no other test in this process has interned.
/// 本进程里没有别的测试驻留过的名字。
///
/// The table is process-level and `cargo test` runs tests in threads, so a test
/// that asserted on the table's *size* would race every other test that parses an
/// artifact. Freshness is what these tests assert on, never an absolute count.
/// 这张表是进程级的，而 `cargo test` 用线程跑测试，因此对表**大小**的断言会与任何其它解析
/// artifact 的测试相互竞争。这些测试断言的是"新鲜"，绝不是绝对计数。
fn fresh(label: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "t28-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// An empty table a test may fill without touching the process's own.
/// 一张空表，测试可以填它而不碰进程自己的那张。
fn private_table() -> InternTable {
    InternTable::new(BTreeSet::new())
}

fn size(table: &InternTable) -> usize {
    table
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .len()
}

/// A document with one frame per name, so each name reaches the intern table.
/// 每个名字一帧的文档，因此每个名字都会到达驻留表。
fn document(names: &[String]) -> String {
    let node = NodeId::from_namespaced_path("t28-intern", "frame.rs", "Probe");
    let mut text = format!("version=1\nnamespace=t28\nroot={node}\nmode=full\n");
    for (index, name) in names.iter().enumerate() {
        text.push_str(&format!("frame={index}\t-\t{node}\t{name}\t-\t-\t-\n"));
    }
    text
}

/// The ceiling is a refusal, not a bigger process: an equal string is still
/// served once the table is full, a new one is refused by name, and the refused
/// string is *not* inserted.
/// 天花板是一次拒绝，而不是一个更大的进程：表满之后相同字符串仍然被服务，新字符串按名被拒，
/// 且被拒的字符串**没有**被插入。
///
/// Red before the fix: `intern` had no ceiling at all — every distinct string was
/// leaked and inserted forever, which a release probe measured as about 83 bytes
/// per name that never came back.
/// 修前为红：`intern` 根本没有天花板——每个不同字符串都会被永久泄漏并插入，release 探针实测到
/// 每个名字约 83 字节、从不回落。
#[test]
fn the_budget_refuses_a_new_string_instead_of_growing() {
    let table = private_table();
    let limit = 3;
    let names: Vec<String> = (0..limit).map(|_| fresh("kept")).collect();
    for name in &names {
        intern_into(&table, name, limit).expect("within the budget");
    }
    assert_eq!(
        intern_into(&table, &names[0], limit).expect("a known string"),
        names[0].as_str(),
        "interning is idempotent, not blocked once the table is full"
    );
    let refused = fresh("refused");
    let error = intern_into(&table, &refused, limit).expect_err("past the ceiling");
    assert!(
        error.to_string().contains(&limit.to_string()),
        "the refusal must name the ceiling: {error}"
    );
    assert!(
        error.to_string().contains("restart"),
        "the refusal must say what clears the table: {error}"
    );
    assert!(
        intern_into(&table, &refused, limit).is_err(),
        "a refused string is not inserted, so asking again refuses again"
    );
    assert_eq!(
        size(&table),
        limit,
        "the ceiling is a hard ceiling, not a threshold a refusal crosses"
    );
}

/// A parse interns its vocabulary into the process table, and a repeated string
/// is served rather than added.
/// 解析会把词表驻留进进程表，而重复的字符串是被服务而不是被加入。
#[test]
fn a_parse_interns_its_vocabulary_and_a_repeat_costs_nothing() {
    let before = interned_len();
    let names: Vec<String> = (0..3).map(|_| fresh("parsed")).collect();
    let artifact =
        TraceArtifact::parse_trace_artifact(&document(&names)).expect("a well-formed document");
    assert_eq!(artifact.frames.len(), 3);
    assert!(
        interned_len() >= before + 3,
        "every distinct function name reaches the process table"
    );

    let table = private_table();
    for name in &names {
        intern_into(&table, name, INTERN_LIMIT).expect("within the budget");
    }
    let after_first = size(&table);
    for name in &names {
        intern_into(&table, name, INTERN_LIMIT).expect("a known string");
    }
    assert_eq!(
        size(&table),
        after_first,
        "the same vocabulary again interns nothing new — the reason a repeated artifact costs nothing"
    );
}

/// The production ceiling itself refuses and names itself.
/// 生产天花板本身会拒绝，并点出自己的值。
///
/// This fills a private table with the production ceiling, so the number a real
/// process would hit is exercised here rather than assumed: 131 072 distinct
/// strings is a few megabytes and under a second, and the leak lands in the test
/// process, which exits.
/// 这里用生产天花板填一张私有表，因此真实进程会撞到的那个数字是在这里**被执行**的，而不是被
/// 假设的：131 072 个不同字符串是几兆字节、不到一秒，而泄漏落在测试进程里，测试结束即释放。
#[test]
fn the_production_ceiling_refuses_and_names_itself() {
    let table = private_table();
    for _ in 0..INTERN_LIMIT {
        intern_into(&table, &fresh("fill"), INTERN_LIMIT).expect("within the budget");
    }
    let error = intern_into(&table, &fresh("over"), INTERN_LIMIT).expect_err("past the ceiling");
    assert!(
        error.to_string().contains(&INTERN_LIMIT.to_string()),
        "the production refusal names the production ceiling: {error}"
    );
    assert!(size(&table) == INTERN_LIMIT);
}

/// A real artifact vocabulary has to fit under the ceiling. This is a property of
/// the constant, so it is asserted where the constant is: at compile time. Written
/// as a runtime `assert!` it was a constant condition, and clippy's
/// `assertions_on_constants` refused it.
/// 真实 artifact 的词表必须装得下这个天花板。这是常量的性质，因此在常量所在处断言：编译期。写成
/// 运行期 `assert!` 时它是一个常量条件，clippy 的 `assertions_on_constants` 会拒绝它。
const _: () = assert!(
    INTERN_LIMIT >= 0x1_0000,
    "the intern ceiling must leave room for a real vocabulary"
);

/// A record may be written before what it references: the document's order is
/// free, and a local's function or an edge's function is resolved after the whole
/// document has been read.
/// 记录可以写在它引用的东西之前：文档顺序自由，而局部值的函数名或边的函数名在整份文档读完后补全。
///
/// Red before the fix: a `local` line ahead of its `frame` kept
/// `source.function == "<local>"` (and an `edge` ahead of its `local` kept
/// `<runtime>`) because resolution assumed the canonical order and only the
/// canonical render writes that order. Nothing downstream catches it — the
/// `into_trace` integrity check does not look at `function` — so the wrong name
/// stayed in the evidence.
/// 修前为红：`local` 行写在其 `frame` 之前时 `source.function` 会是 `"<local>"`（`edge` 写在其
/// `local` 之前时是 `"<runtime>"`），因为解析假定了规范顺序，而只有规范渲染才写那个顺序。
/// 下游没有任何东西能发现它——`into_trace` 的完整性检查不看 `function`——因此错误的函数名留在了
/// 证据里。
#[test]
fn a_record_may_reference_what_follows_it() {
    let node = NodeId::from_namespaced_path("t28-order", "frame.rs", "Probe");
    let shuffled = format!(
        "version=1\nnamespace=t28\nroot={node}\nmode=full\n\
         local=0\t7\tlet\tobserved\tvar\tu32\t7\tprobe.rs\t10\t4\n\
         edge=0\t0\tlabel\tprobe.rs\t10\t4\n\
         frame=7\t-\t{node}\tProbe\tprobe.rs\t10\t4\n"
    );
    let canonical = format!(
        "version=1\nnamespace=t28\nroot={node}\nmode=full\n\
         frame=7\t-\t{node}\tProbe\tprobe.rs\t10\t4\n\
         local=0\t7\tlet\tobserved\tvar\tu32\t7\tprobe.rs\t10\t4\n\
         edge=0\t0\tlabel\tprobe.rs\t10\t4\n"
    );
    let shuffled = TraceArtifact::parse_trace_artifact(&shuffled).expect("order is free");
    let canonical = TraceArtifact::parse_trace_artifact(&canonical).expect("the canonical order");

    let local = shuffled.locals.first().expect("the local was read");
    assert_eq!(
        local.source.function, "Probe",
        "a local resolves its function from a frame written below it"
    );
    let edge = shuffled.edges.first().expect("the edge was read");
    assert_eq!(
        edge.source.as_ref().map(|source| source.function),
        Some("Probe"),
        "an edge resolves its function from a local written below it"
    );
    assert_eq!(
        shuffled.frames.len(),
        canonical.frames.len(),
        "both orders read the same document"
    );
    assert_eq!(
        serde_free_debug(&shuffled),
        serde_free_debug(&canonical),
        "the two orders produce the same artifact, field by field"
    );
}

/// A `Debug` rendering cheap enough to compare two artifacts without making them
/// carry `PartialEq` for a test's sake.
/// 一种便宜的 `Debug` 渲染，用来比较两份 artifact，而不必为了测试给它们加 `PartialEq`。
fn serde_free_debug(artifact: &TraceArtifact) -> String {
    format!(
        "{:?}\nlocals={:?}\nedges={:?}",
        artifact.frames, artifact.locals, artifact.edges
    )
}
