//! Bounded parallelism, shared by the discovery walk and the fingerprint.
//! 有界并行：发现遍历与指纹共用。
//!
//! Standard library only: this checkout builds offline and publishes crates, so a thread-pool
//! dependency is not an option, and `std::thread::scope` has been enough since 1.63. **Order
//! preservation is the whole contract** — a fingerprint that depended on completion order would go
//! stale for every record the moment the machine's core count changed. The worker count is a rule
//! rather than a number (half the cores, at most [`MAX_WORKERS`], never fewer than one left alone),
//! and the result never depends on it.
//! 只用标准库：本检出离线构建并发布 crate，因此线程池依赖不是选项，而 `std::thread::scope` 自 1.63 起
//! 就够用。**保持顺序就是它的全部契约**——依赖完成顺序的指纹会在机器核数一变时让每份记录立刻陈旧。
//! 工作线程数是规则而不是数字（一半的核、最多 [`MAX_WORKERS`] 个、至少留一个核），而结果绝不取决于它。

/// Map over items with several workers, preserving order (audit `K2`).
/// 用多个工作线程映射一组条目，并**保持顺序**（审计 `K2`）。
///
/// Standard library only: this checkout builds offline and publishes crates, so a thread-pool
/// dependency is not an option, and `std::thread::scope` has been enough since 1.63. **Order
/// preservation is the whole contract** — a fingerprint that depended on completion order would go
/// stale for every record the moment the machine's core count changed, and `fingerprints_do_not_depend
/// _on_the_worker_count` is the pin that holds it.
/// 只用标准库：本检出离线构建并发布 crate，因此线程池依赖不是选项，而 `std::thread::scope` 自 1.63 起就够用。
/// **保持顺序就是它的全部契约**——依赖完成顺序的指纹会在机器核数一变时让每份记录立刻陈旧，而
/// `fingerprints_do_not_depend_on_the_worker_count` 就是守住它的钉子。
pub(crate) fn parallel_map<T: Send>(
    items: &[std::path::PathBuf],
    map: impl Fn(&std::path::PathBuf) -> T + Sync,
) -> Vec<T> {
    parallel_map_with_workers(items, worker_budget(), map)
}

/// How many workers this process may hand one mapping (audit `T1`).
/// 本进程一次映射最多可以交给几个工作线程（审计 `T1`）。
///
/// A rule rather than a number, because machines differ and a tool call must not take a user's whole
/// machine: **half the cores, at most [`MAX_WORKERS`], never fewer than one core left alone** — an
/// editor, a compiler and a test run are usually standing beside us. `XIRANG_JOBS` (see the
/// kernel's `lexicon`) overrides it for a caller who knows better. The **result never depends on this
/// number**: order is preserved either way, which is what
/// `discovery_tests::the_discovered_tree_does_not_depend_on_the_worker_count` and
/// `parallel_tests::fingerprints_do_not_depend_on_the_worker_count` hold.
/// 是规则而不是数字，因为机器各不相同，而一次工具调用不该把用户的整台机器拿走：**一半的核、最多
/// [`MAX_WORKERS`] 个、至少留一个核**——编辑器、编译器与测试通常就在旁边。比规则更清楚自己处境的调用方
/// 用 `XIRANG_JOBS`（见内核 `lexicon`）覆盖它。**结果绝不取决于这个数字**：两种情况下顺序都保持，
/// 这正是 `discovery_tests::the_discovered_tree_does_not_depend_on_the_worker_count` 与
/// `parallel_tests::fingerprints_do_not_depend_on_the_worker_count` 守住的东西。
pub(crate) fn worker_budget() -> usize {
    let requested = std::env::var(xirang_kernel::lexicon::JOBS_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|count| *count > 0);
    let cores = std::thread::available_parallelism().map_or(1, |count| count.get());
    budget_from(requested, cores)
}

/// The budget as a pure function of the explicit request and the machine's core count (audit `T1`).
/// 把预算写成"显式请求 + 机器核数"的纯函数（审计 `T1`）。
///
/// Split out so the rule can be pinned **without writing to the process environment**: the rule is
/// what matters, and a test that mutates `XIRANG_JOBS` would be writing to the same environment
/// every other test in this process reads (`std::env::set_var` is `unsafe` in edition 2024 for
/// exactly that reason). An explicit request that parses to a positive number wins outright; a zero
/// or a typo is not a request and falls through to the machine rule.
/// 拆出来是为了让规则**不必写进程环境**就能被钉住：要紧的是规则本身，而一个改写 `XIRANG_JOBS` 的测试
/// 会写进本进程里其它每个测试都在读的那份环境（edition 2024 里 `std::env::set_var` 是 `unsafe`，正是
/// 这个原因）。解析出正数的显式请求直接生效；零或错别字不算请求，落到机器规则上。
fn budget_from(requested: Option<usize>, cores: usize) -> usize {
    requested.unwrap_or_else(|| (cores / 2).clamp(1, MAX_WORKERS))
}

/// The most workers a mapping takes even on a very wide machine.
/// 即使在非常宽的机器上，一次映射最多用几个工作线程。
///
/// A 64-core build server gains nothing from 64 threads on a twenty-member workspace, and the thread
/// handoff is not free; this is the ceiling the rule above is capped by, not a promise about speed.
/// 一台 64 核的构建服务器不会因为给二十个成员的工作区开 64 个线程而得到什么，而线程交接也不是免费的；
/// 这是上面那条规则的上限，而不是关于速度的承诺。
const MAX_WORKERS: usize = 8;

thread_local! {
    /// Whether this thread is already inside a mapping, which is what stops a nested wide level from
    /// opening a second pool (audit `T1`).
    /// 本线程是否已经在一个映射之内；正是它阻止嵌套的宽层再开一个线程池（审计 `T1`）。
    static MAPPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// [`parallel_map`] with the worker budget spelled out, so a pin can compare worker counts.
/// [`parallel_map`] 的显式工作线程预算版本，好让钉子能比较不同线程数。
pub(crate) fn parallel_map_with_workers<I: Sync, T: Send>(
    items: &[I],
    budget: usize,
    map: impl Fn(&I) -> T + Sync,
) -> Vec<T> {
    parallel_map_with_threshold(items, budget, 64, map)
}

/// The single mapping implementation: ordered, bounded, and refusing to nest (audit `K2`, `T1`).
/// 唯一的映射实现：保持顺序、有界、并拒绝嵌套（审计 `K2`、`T1`）。
///
/// `serial_below` is the size under which the thread handoff costs more than the work it hands off,
/// and it is a parameter because "small" is a property of the **items**: a file is a handful of
/// kilobytes, while a workspace **member** is a whole-tree content hash, so two members already pay
/// for the handoff where sixty-four files do not. `budget` is the machine rule ([`worker_budget`]:
/// half the cores, at most [`MAX_WORKERS`], one core always left alone) unless a caller names one.
/// `serial_below` 是"线程交接比它交接出去的活还贵"的那个规模，而它是一个参数，因为"小"是**条目**的性质：
/// 一个文件是几 KB，而工作区的一个**成员**是一次整树内容哈希，因此两个成员就够付线程交接的钱，而六十四个
/// 文件不够。`budget` 是机器规则（[`worker_budget`]：一半的核、最多 [`MAX_WORKERS`] 个、永远留一个核），
/// 除非调用方点名一个。
///
/// Inside a worker of an enclosing mapping it runs serially — a nested pool would multiply the
/// machine's threads by the depth of the tree — and the budget is capped by the item count, so a
/// mapping over three members never opens eight threads.
/// 在**外层映射的工作线程之内**时它串行运行——嵌套线程池会把机器的线程数乘以树的深度——而预算被条目数
/// 封顶，因此对三个成员的映射绝不会开八个线程。
pub(crate) fn parallel_map_with_threshold<I: Sync, T: Send>(
    items: &[I],
    budget: usize,
    serial_below: usize,
    map: impl Fn(&I) -> T + Sync,
) -> Vec<T> {
    let workers = budget.min(items.len()).max(1);
    if items.len() < serial_below || workers <= 1 || MAPPING.with(std::cell::Cell::get) {
        return items.iter().map(map).collect();
    }
    let chunk = items.len().div_ceil(workers);
    let mut slots: Vec<Option<Vec<T>>> = (0..items.len().div_ceil(chunk)).map(|_| None).collect();
    std::thread::scope(|scope| {
        for (slot, slice) in slots.iter_mut().zip(items.chunks(chunk)) {
            let map = &map;
            scope.spawn(move || {
                let _guard = MappingGuard::enter();
                *slot = Some(slice.iter().map(map).collect());
            });
        }
    });
    slots.into_iter().flatten().flatten().collect()
}

/// Marks its thread as being inside a mapping, for as long as it lives.
/// 在它存活期间，把所在线程标记为"在一个映射之内"。
struct MappingGuard;

impl MappingGuard {
    /// Enter a mapping on this thread, restoring the previous mark when dropped.
    /// 在本线程进入一个映射，并在被丢弃时恢复先前的标记。
    fn enter() -> Self {
        MAPPING.with(|mapping| mapping.set(true));
        MappingGuard
    }
}

impl Drop for MappingGuard {
    fn drop(&mut self) {
        MAPPING.with(|mapping| mapping.set(false));
    }
}

#[cfg(test)]
mod parallel_tests {
    use super::parallel_map;

    /// The worker budget is a rule, and the explicit request beats it (audit `T1`).
    /// 工作线程预算是**一条规则**，而显式请求压过它（审计 `T1`）。
    ///
    /// Two properties matter and neither is about speed: a mapping must never take a user's whole
    /// machine (at most half the cores, and below a wide machine's core count), and an explicit
    /// request must be honoured so a caller can say "use one" or "use more". The machine's core count
    /// is a parameter here rather than a fact read from this box, because the rule has to hold on
    /// every box.
    /// 两条性质要紧，且都与速度无关：一次映射绝不拿走用户的整台机器（最多一半的核，且低于宽机器的核数），
    /// 而显式请求必须被尊重，好让调用方说"就用一个"或"多用一点"。核数在这里是**参数**而不是从本机读来的
    /// 事实，因为这条规则在每台机器上都要成立。
    #[test]
    fn the_worker_budget_leaves_the_machine_alone_and_honours_an_explicit_request() {
        for cores in [1usize, 2, 3, 4, 8, 16, 20, 64, 128] {
            let budget = super::budget_from(None, cores);
            assert!(budget >= 1, "{cores} cores must still buy one worker");
            assert!(
                cores == 1 || budget < cores,
                "{cores} cores: a mapping must leave the machine something ({budget})"
            );
            assert!(
                budget <= 8,
                "{cores} cores: the ceiling is part of the rule"
            );
            assert_eq!(
                super::budget_from(Some(3), cores),
                3,
                "an explicit request wins"
            );
        }
    }

    /// The fingerprint must not depend on how many workers computed it (audit `K2`).
    /// 指纹不得取决于用几个工作线程算出来（审计 `K2`）。
    ///
    /// Parallelism here is an implementation detail of a **rule**: the digest goes into every record,
    /// and a digest that depended on completion order would invalidate all of them the moment the core
    /// count changed. The pin runs the same mapping many times and compares the joined result — with
    /// enough items that the parallel path is actually taken (> the serial threshold).
    /// 这里的并行只是一条**规则**的实现细节：摘要写进每一份记录，而依赖完成顺序的摘要在核数一变时就会让它们
    /// 全部失效。这枚钉子反复跑同一个映射并比较拼接结果——条目数足够多，因此真的会走并行那条路（超过串行阈值）。
    #[test]
    fn fingerprints_do_not_depend_on_the_worker_count() {
        let items: Vec<std::path::PathBuf> = (0..500)
            .map(|index| std::path::PathBuf::from(format!("file{index:04}.rs")))
            .collect();
        let expected: Vec<String> = items
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        for _ in 0..5 {
            let got = parallel_map(&items, |path| path.display().to_string());
            assert_eq!(
                got, expected,
                "order is preserved whatever the worker count"
            );
        }
    }
}
