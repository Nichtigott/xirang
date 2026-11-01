//! The read-keys table: which arguments each tool's handler reads for itself.
//! 读键表：每个工具的处理函数为自己读了哪些参数。
//!
//! A **test-support** module with no test of its own, which is a shape this repository already ships
//! (`check_fixtures.rs`): the table is data the pins read, and keeping it in the pin file pushed that
//! file past the test budget for the sake of one import line.
//! 一个**不带测试**的测试支撑模块，这是本仓已经出厂的形状（`check_fixtures.rs`）：这张表是钉子读的数据，
//! 把它留在钉子文件里，只为一行 import 就把那个文件顶过了测试预算。

pub(super) const READ_KEYS: &[(&str, &[&str])] = &[
    (
        "xirang.explore",
        &["query", "max_files", "max_bytes", "root"],
    ),
    ("xirang.search", &["names", "query", "literal", "converge"]),
    ("xirang.digest", &["file"]),
    ("xirang.conformance", &["anchor"]),
    // `files` is read by `adopted`'s renewal path alone (`renew`, adopted.rs), not by the comparison.
    // `files` 只由 `adopted` 的续期路径读（`renew`，adopted.rs），比对那条路不读。
    ("xirang.consistency", &["parent", "full"]),
    // The expectation is the whole input: `node` names the face, `expect` the claim, `root` is the
    // usual base; anything else is *decided against* rather than read.
    // 期望就是全部输入：`node` 点名注册面、`expect` 是那条主张、`root` 是常规基底；其余键都是被**判定**
    // 而不是被读取的参数。
    ("xirang.constraints", &["node", "expect", "control", "root"]),
    // `action` and `id` address the candidate; the rest are the check keys the copy's verification
    // takes, plus `scope`, which only `open` reads.
    // `action` 与 `id` 寻址候选；其余是副本内验证所取的 check 键，外加 `scope`——只有 `open` 读它。
    (
        "xirang.candidate",
        &["action", "id", "reviewer", "scope", "target_dir", "verdict"],
    ),
    // The plan reads `anchor`; `apply` is the one switch that turns the mechanical half from a report
    // into a write. Everything else on the reply is read from the lineage and the tree.
    // 计划读 `anchor`；`apply` 是唯一把机械那一半从报告变成写入的开关。回复里其余内容读自谱系与树。
    ("xirang.refactor", &["anchor", "apply"]),
    // The task's whole input: one action, one id, and — at open — the object it is about, the plan
    // that verified it, and whether an acceptance is required.
    // 任务的全部输入：一个动作、一个 id，以及（open 时）它关于的对象、验证它的计划、是否需要一次接受。
    (
        "xirang.task",
        &[
            "action",
            "task",
            "object",
            "verification",
            "from",
            "target_dir",
            "acceptance",
            "confirm",
            "apply",
        ],
    ),
    (
        "xirang.rules",
        &[
            "action",
            "rule_id",
            "category",
            "selector",
            "requirement",
            "sources",
            "authority",
            "confirm",
            "reason",
        ],
    ),
    ("xirang.why", &["at"]),
    ("xirang.locate", &[]),
    ("xirang.inspect", &[]),
    (
        "xirang.callgraph",
        &["function", "orphans", "path", "source"],
    ),
    ("xirang.read", &["context", "line", "lines", "whole"]),
    ("xirang.status", &[]),
    (
        "xirang.apply",
        &[
            "action", "confirm", "fields", "file", "full", "language", "selector",
        ],
    ),
    (
        "xirang.new_project",
        &[
            "confirm",
            "dependency",
            "directory",
            "faces",
            "git",
            "kind",
            "package",
        ],
    ),
    (
        "xirang.plugin",
        &[
            "checksum",
            "confirm",
            "crate",
            "fingerprint",
            "framework",
            "mode",
            "package",
            "revocations",
            "signature",
            "source",
            "version",
        ],
    ),
    ("xirang.registry", &["full", "offset"]),
    ("xirang.explain", &["node", "overlay"]),
    ("xirang.diff", &["against", "records"]),
    ("xirang.trace", &["query", "values"]),
    ("xirang.mir", &["against", "against_trace", "jsonl"]),
    ("xirang.unified", &["against"]),
    ("xirang.grafts", &[]),
    ("xirang.impact", &["depth"]),
    ("xirang.usages", &["node"]),
    ("xirang.converge", &["trace"]),
    ("xirang.adopted", &["anchor", "apply", "confirm", "files"]),
    ("xirang.affected", &["files"]),
    ("xirang.check", &["census", "target_dir", "verbose"]),
    ("xirang.verify", &[]),
    (
        "xirang.graph",
        &["node", "direction", "depth", "cycles", "limit"],
    ),
    // `crates` reads the action and whether to apply it; `root` is read by every tool's entry.
    // `crates` 读动作与是否写入；`root` 由每个工具的入口读。
    ("xirang.crates", &["action", "apply", "crate", "subtree"]),
    ("xirang.multilang", &["action", "plan"]),
    ("xirang_tools", &["tool", "full"]),
];
