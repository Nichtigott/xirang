//! Content-keyed syntax facts shared by build manifests and MCP, never path/ownership answers.
//! 构建清单与 MCP 共用按内容寻址的语法事实，不缓存路径与归属答案。
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(feature = "mcp")]
use xirang_kernel::source::BranchFacts;
use xirang_kernel::source::SourceFunction;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 131_072;

pub(crate) struct Unit {
    pub(crate) functions: Vec<SourceFunction>,
    pub(crate) calls: Vec<Vec<String>>,
    #[cfg(feature = "mcp")]
    pub(super) branches: OnceLock<BranchFacts>,
}
impl Unit {
    #[cfg(feature = "mcp")]
    pub(crate) fn branches(&self, source: &str) -> BranchFacts {
        self.branches
            .get_or_init(|| xirang_kernel::source::branch_facts(source))
            .clone()
    }
    pub(super) fn bytes(&self) -> usize {
        self.functions
            .iter()
            .map(|f| f.name.len() + f.signature.len() + f.body.len() + 128)
            .sum::<usize>()
            + self
                .calls
                .iter()
                .flatten()
                .map(|call| call.len() + 24)
                .sum::<usize>()
    }
}
#[derive(Default)]
pub(super) struct Cache {
    units: HashMap<Arc<str>, Arc<Unit>>,
    order: VecDeque<Arc<str>>,
    bytes: usize,
}
impl Cache {
    pub(super) fn insert(&mut self, source: &str, unit: Arc<Unit>) {
        let bytes = source.len().saturating_mul(8) + unit.bytes();
        if bytes > MAX_BYTES || self.units.contains_key(source) {
            return;
        }
        while self.bytes + bytes > MAX_BYTES || self.units.len() >= MAX_ENTRIES {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            if let Some(unit) = self.units.remove(&old) {
                self.bytes -= old.len().saturating_mul(8) + unit.bytes();
            }
        }
        let source: Arc<str> = source.into();
        self.bytes += bytes;
        self.order.push_back(source.clone());
        self.units.insert(source, unit);
    }
}
pub(super) static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

pub(crate) fn facts(source: &str) -> Arc<Unit> {
    let cache = CACHE.get_or_init(Default::default);
    let hit = cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .units
        .get(source)
        .cloned();
    if let Some(unit) = hit {
        #[cfg(feature = "build")]
        super::snapshot::used(source, unit.clone(), false);
        return unit;
    }
    #[cfg(feature = "mcp")]
    if let Some(unit) = super::disk::load(source) {
        cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(source, unit.clone());
        // A build sharing this process still has to see the unit it would have computed: the
        // snapshot records what this pass used, and skipping it made the build's snapshot file
        // missing (measured red in `snapshot_tests`, again after a restore dropped this line).
        // 共用本进程的构建仍须看到"它本来会算出的"单元：快照记录本趟用过什么，跳过它会让构建的快照文件
        // 缺失（`snapshot_tests` 量到红，一次还原又把这一行弄丢过）。
        #[cfg(feature = "build")]
        super::snapshot::used(source, unit.clone(), true);
        return unit;
    }
    let functions = xirang_kernel::source::function_symbols(source);
    let calls = functions
        .iter()
        .map(|f| xirang_kernel::source::direct_calls(&f.body, &f.name))
        .collect();
    let unit = Arc::new(Unit {
        functions,
        calls,
        #[cfg(feature = "mcp")]
        branches: OnceLock::new(),
    });
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(source, unit.clone());
    #[cfg(feature = "build")]
    super::snapshot::used(source, unit.clone(), true);
    #[cfg(feature = "mcp")]
    super::disk::store(source, &unit);
    unit
}

#[cfg(test)]
#[path = "facts_tests.rs"]
mod facts_tests;

#[cfg(test)]
mod visibility_tests {
    use super::facts;

    /// W7/A28 — **改变可见**，而且这一条是**按构造**成立的：`facts` 的键就是**源码文本本身**
    /// （`units.get(source)`，`facts.rs:66-81`），因此"改过的内容复用旧解析"在结构上不可能发生。
    /// W7/A28 — **a change is visible**, and it holds **by construction**: `facts`'s key *is* the source
    /// text (`units.get(source)`), so a stale parse being reused for edited content cannot happen.
    ///
    /// 两条一起钉：没改 ⇒ 连分配都一样（`Arc::ptr_eq`，不只是"相等"）；改了 ⇒ 拿到的是新内容的解析。
    /// Both halves are pinned: unchanged ⇒ even the allocation is the same (`Arc::ptr_eq`, not merely
    /// equal); changed ⇒ the parse of the new content is what comes back.
    #[test]
    fn an_edit_is_visible_and_an_unchanged_source_reuses_its_unit() {
        let one = "pub fn a() { core::mem::drop(1); }\n";
        let other = "pub fn b() { core::mem::drop(2); }\n";

        let first = facts(one);
        let again = facts(one);
        assert!(
            std::sync::Arc::ptr_eq(&first, &again),
            "the second call is served by the cache, not by a re-parse"
        );

        let edited = facts(other);
        assert!(
            !std::sync::Arc::ptr_eq(&first, &edited),
            "an edit is a different key, so it cannot be served by the old entry"
        );
        let names: Vec<&str> = edited
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert!(
            names.contains(&"b") && !names.contains(&"a"),
            "and what comes back parses the new text: {names:?}"
        );

        // 改回来 ⇒ 又命中那条最初的条目（键是内容，因此回退也能复用）。
        // Edited back ⇒ the original entry is hit again (the key is the content, so going back reuses).
        let restored = facts(one);
        assert!(
            std::sync::Arc::ptr_eq(&first, &restored),
            "the same text is the same key, however it got there"
        );
    }
}
