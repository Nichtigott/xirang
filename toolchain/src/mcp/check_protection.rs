//! Consumer protection as a **duty**, not as a note: who consumes a shared implementation, and which
//! of those entries no test calls by name.
//! 把消费者保护做成**义务**而不是一条注记：谁在消费共享实现，以及其中哪些入口没有任何测试按名字调用。
//!
//! A repair can be behaviourally correct and still leave the next change unguarded. The measured
//! instance is in the basic-repair re-acceptance: two of the nine deliverables added an arithmetic
//! unit test and **no** test that covers the two consumers, so the shared fix was protected only by
//! the hidden judge. Nothing in the reply said so, which is the defect this module removes: an
//! uncovered consumer becomes a row in the same obligation ledger the executable checks write into,
//! so a plan that ran every cargo command green still reports `open` while a duty is unmet.
//! 一次修复可以在行为上正确，却让下一次改动毫无遮拦。实测的例子就在基础修复复验里：九份交付里有两份只加了
//! arithmetic 单元测试，**没有**任何覆盖那两个消费者的测试，于是共享修复只被隐藏裁判保护着，而回复里没有
//! 任何一行说明这件事——这就是本模块要消除的缺陷：未被覆盖的消费者成为可执行检查所写入的**同一本义务台账**
//! 里的一行，于是一份把所有 cargo 命令都跑绿的计划，在义务未结清时仍然报 `open`。
//!
//! online: which test files call a consumer is a statement about the sources as they stand now, and a
//! tree that changed since the build has test files the last plan never saw.
//! online：哪些测试文件调用了某个消费者，是关于源码**此刻**如何的陈述；构建之后变过的树里有上一份计划
//! 从未见过的测试文件。
//!
//! What this is **not**: it is not coverage. It is a lexical statement about which test files call
//! which consumer function by name, and the row says so. A test that exists but never executes, a
//! filter that matched nothing, and an ignored test all leave the duty exactly where it was.
//! 它**不是**什么：它不是覆盖率。它是一句词法陈述——哪些测试文件按名字调用了哪个消费者函数，而那一行会
//! 自己说明这一点。存在却从未执行的测试、什么都没匹配到的过滤器、被忽略的测试，都让这条义务原地不动。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

use super::relations::SHARED_FROM;

/// One consumer entry of a shared provider, with the test files that call **it** by name.
/// 共享提供者的一个消费者入口，以及按名字调用**它**的测试文件。
struct Entry {
    provider: String,
    provider_file: String,
    consumer: String,
    consumer_file: String,
    line: usize,
    covered_by: BTreeSet<String>,
}

/// What the census found, for the ledger and the reply.
/// 普查发现了什么，供台账与回复使用。
pub(super) struct Census {
    /// One obligation row per consumer entry, in the shape `check_registry` already uses.
    /// 每个消费者入口一行义务，形状与 `check_registry` 已在用的一致。
    pub(super) duties: Vec<Value>,
    /// The readable summary recorded under `protection`.
    /// 记录在 `protection` 下的可读摘要。
    pub(super) summary: Value,
}

/// The consumer entries of every **shared** provider the named files define.
/// 被点名文件所定义的每个**共享**提供者的消费者入口。
///
/// `None` means there is nothing to say — no named file defines a function that two other files call —
/// and that is different from "everything is protected": the summary says which of the two it is.
/// `None` 意味着无话可说——没有任何被点名的文件定义了被另外两个文件调用的函数——而它与"全都被保护"不同：
/// 摘要会说明是这两者中的哪一种。
pub(super) fn census(root: &Path, files: &Value) -> Result<Option<Census>, String> {
    let named: Vec<String> = match files {
        Value::Array(paths) => paths
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Value::String(paths) => paths
            .split(',')
            .map(|path| path.trim().to_owned())
            .collect(),
        _ => Vec::new(),
    };
    if named.is_empty() {
        return Ok(None);
    }
    // One walk, over the same index `explore` reads, so the census and the repair projection answer
    // from one lexical view rather than from two scans that can disagree.
    // 一趟遍历，走的是 `explore` 读的同一份索引，因此普查与修复投影从同一个词法视图作答，而不是从两次可能
    // 互相矛盾的扫描。
    let sources = super::source_index::load_sources(root)?;
    let named_matches = |relative: &str| {
        named.iter().any(|path| {
            let path = path.strip_prefix("./").unwrap_or(path);
            relative == path
                || relative.ends_with(&format!("/{path}"))
                || relative == path.trim_start_matches('/')
        })
    };
    let provider_files: BTreeSet<&str> = sources
        .iter()
        .filter(|file| named_matches(&file.relative))
        .map(|file| file.relative.as_str())
        .collect();
    if provider_files.is_empty() {
        return Ok(Some(Census {
            duties: Vec::new(),
            summary: json!({
                "scope": "the named files are not in this tree's source index",
                "shared_providers": Vec::<String>::new(),
                "note": "no consumer duty was derived; this is not evidence that consumers are covered",
            }),
        }));
    }
    // Callers of every name, then the tests that call each caller by name. Both directions use the
    // shared call-shape rule, so a qualified call and a bare call are the same relation here.
    // 每个名字的调用者，然后是按名字调用各个调用者的测试。两个方向都用共用的调用形状规则，因此限定调用与
    // 裸名调用在这里是同一条关系。
    let mut callers: BTreeMap<&str, Vec<(&str, &str, usize)>> = BTreeMap::new();
    for file in &sources {
        for function in &file.functions {
            for call in &function.calls {
                let name = call.rsplit("::").next().unwrap_or(call);
                callers.entry(name).or_default().push((
                    file.relative.as_str(),
                    function.name.as_str(),
                    function.line,
                ));
            }
        }
    }
    let test_files: BTreeSet<&str> = sources
        .iter()
        .filter(|file| {
            super::callgraph::looks_like_a_test(&file.relative, &file.source)
                || file.relative.starts_with("tests/")
        })
        .map(|file| file.relative.as_str())
        .collect();
    let mut covered: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for file in &sources {
        if !test_files.contains(file.relative.as_str()) {
            continue;
        }
        for function in &file.functions {
            for call in &function.calls {
                let name = call.rsplit("::").next().unwrap_or(call);
                covered
                    .entry(name)
                    .or_default()
                    .insert(file.relative.clone());
            }
        }
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut shared: BTreeSet<String> = BTreeSet::new();
    for file in sources
        .iter()
        .filter(|file| provider_files.contains(file.relative.as_str()))
    {
        for function in &file.functions {
            let consumers: Vec<_> = callers
                .get(function.name.as_str())
                .into_iter()
                .flatten()
                // A **test** file calling the provider is evidence about the provider, not a consumer
                // entry: counting it would let "a test exists" both make the provider look shared and
                // then immediately satisfy the duty it created.
                // **测试**文件调用提供者是关于提供者的证据，而不是消费者入口：把它算进来，会让"存在一个
                // 测试"既把提供者显得像共享的、又立刻结清它自己造出来的那条义务。
                .filter(|(consumer_file, _, _)| {
                    *consumer_file != file.relative.as_str() && !test_files.contains(*consumer_file)
                })
                .map(|(consumer_file, consumer, line)| (*consumer_file, *consumer, *line))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let independent: BTreeSet<&str> = consumers.iter().map(|(file, _, _)| *file).collect();
            if independent.len() < SHARED_FROM {
                continue;
            }
            shared.insert(format!("{}::{}", file.relative, function.name));
            for (consumer_file, consumer, line) in consumers {
                entries.push(Entry {
                    provider: function.name.clone(),
                    provider_file: file.relative.clone(),
                    consumer: consumer.to_owned(),
                    consumer_file: consumer_file.to_owned(),
                    line,
                    covered_by: covered
                        .get(consumer)
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect(),
                });
            }
        }
    }
    if entries.is_empty() {
        return Ok(Some(Census {
            duties: Vec::new(),
            summary: json!({
                "scope": "no function defined in the named files is called from two or more other files",
                "shared_providers": Vec::<String>::new(),
                "note": "nothing was left open, because nothing is shared by this rule; a provider called from one file is not a shared root cause",
            }),
        }));
    }
    let mut duties = Vec::new();
    let mut uncovered = Vec::new();
    let mut rows = Vec::new();
    for entry in &entries {
        let protected = !entry.covered_by.is_empty();
        let name = format!(
            "consumer protection: {}::{}",
            entry.consumer_file, entry.consumer
        );
        if protected {
            rows.push(json!({
                "consumer": format!("{}:{} {}", entry.consumer_file, entry.line, entry.consumer),
                "provider": format!("{}::{}", entry.provider_file, entry.provider),
                "state": "protected",
                "covered_by": entry.covered_by.iter().cloned().collect::<Vec<_>>(),
            }));
        } else {
            uncovered.push(format!("{}::{}", entry.consumer_file, entry.consumer));
            rows.push(json!({
                "consumer": format!("{}:{} {}", entry.consumer_file, entry.line, entry.consumer),
                "provider": format!("{}::{}", entry.provider_file, entry.provider),
                "state": "needs_evidence",
                "covered_by": Vec::<String>::new(),
            }));
        }
        duties.push(json!({
            "name": name,
            "expected": "passed",
            "state": if protected { "passed" } else { "unknown" },
            "evidence": if protected {
                format!(
                    "a test file calls `{}` by name: {}; this is a lexical statement, not executed coverage",
                    entry.consumer,
                    entry.covered_by.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            } else {
                format!(
                    "`{}` in {} calls the shared implementation `{}` (defined in {}), and no test file calls `{}` by name — so this consumer's own delegation is unproven. A test that calls `{}` directly protects the implementation, not this consumer.",
                    entry.consumer, entry.consumer_file, entry.provider, entry.provider_file, entry.consumer, entry.provider
                )
            },
        }));
    }
    Ok(Some(Census {
        duties,
        summary: json!({
            "scope": "lexical: which test files call which consumer function by name",
            "shared_providers": shared.iter().cloned().collect::<Vec<_>>(),
            "consumers": rows,
            "uncovered": uncovered,
            "note": "a duty here is closed by a test that calls the consumer by name; a test that exists but never executes, a filter that matched nothing and an ignored test leave it open",
        }),
    }))
}

#[cfg(test)]
#[path = "check_protection_tests.rs"]
mod check_protection_tests;
