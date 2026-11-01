//! Process-local validation reuse owned by the execution surfaces.
//! 声明发现读取方共用的、有界且按完整内容寻址的语法校验。
//!
//! This module owns only the reusable proof that bytes are valid Rust without
//! face or application identifiers. It never stores paths, identities, directory
//! inventories or answers: callers still read current sources on every walk.
//! The kernel remains a pure parser; this execution adapter owns cache lifetime
//! and preserves syntax errors that a plain substring prefilter would hide.
//! 本模块仅持有可复用的证明：这些字节是合法 Rust，且不含注册面或入口标识符。
//! 它不存路径、身份、目录清单或答案：调用方每轮遍历仍读取当前源码。
//! 内核保持纯解析；执行适配器持有缓存生命周期，并保留纯子串预筛会掩盖的语法错误。

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use xirang_kernel::syntax::FaceSyntaxError;

const MAX_BYTES: usize = 16 * 1024 * 1024;
// 100k-file scans must fit when their source payload fits the unchanged byte cap.
// 十万文件扫描在源码负载未超出不变的字节上限时，应能容纳其校验记录。
const MAX_ENTRIES: usize = 131_072;

#[derive(Default)]
struct Validated {
    sources: HashSet<Arc<str>>,
    order: VecDeque<Arc<str>>,
    bytes: usize,
}

impl Validated {
    fn insert(&mut self, source: &str, max_bytes: usize, max_entries: usize) {
        if source.len() > max_bytes || max_entries == 0 || self.sources.contains(source) {
            return;
        }
        while self.bytes + source.len() > max_bytes || self.sources.len() >= max_entries {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.bytes -= oldest.len();
            self.sources.remove(&oldest);
        }
        let source: Arc<str> = source.into();
        self.bytes += source.len();
        self.sources.insert(Arc::clone(&source));
        self.order.push_back(source);
    }
}

/// `None` requires declaration parsing; `Some` preserves ordinary Rust validation.
/// `None` 要求解析声明；`Some` 保留普通 Rust 的语法校验结果。
pub(super) fn validate(source: &str) -> Option<Result<(), FaceSyntaxError>> {
    // Match the attribute identifier, so words such as `surface` keep their cache path.
    // 匹配属性标识符，让 `surface` 等普通单词仍能走缓存路径。
    let native_face = ["soil", "face"].iter().any(|name| {
        source.match_indices(name).any(|(at, _)| {
            let identifier = |c: char| c.is_alphanumeric() || c == '_';
            !source[..at].chars().next_back().is_some_and(identifier)
                && !source[at + name.len()..]
                    .chars()
                    .next()
                    .is_some_and(identifier)
        })
    });
    if source.contains("_object") || native_face || source.contains("application") {
        return None;
    }
    static VALIDATED: OnceLock<Mutex<Validated>> = OnceLock::new();
    let cache = VALIDATED.get_or_init(Mutex::default);
    // Hash lookup includes exact byte equality; timestamps and hash collisions
    // cannot turn a changed file into a hit. Poisoning only disables reuse.
    // 散列查找包括精确字节比较；时间戳和散列碰撞都不能让已修改文件命中。锁中毒仅禁用复用。
    if cache
        .lock()
        .is_ok_and(|index| index.sources.contains(source))
    {
        return Some(Ok(()));
    }
    // A previous **process** may have proved these exact bytes already: without this the proof dies
    // with the process and every declaration-less file is parsed again on every build — measured at
    // 10 250 ms per fresh process on a 1,000-face + 1 000-neighbour tree (`W7`/`A33`, report §7i.3).
    // 上一个**进程**可能已经证明过这串字节：没有这一半，证明会随进程消失，而每个没有声明的文件在每次构建里都
    // 被重新解析——在 1 000 面加 1 000 邻居的树上实测为每个新进程 10 250 ms（`W7`/`A33`，报告 §7i.3）。
    if super::ordinary_store::proved(source) {
        if let Ok(mut index) = cache.lock() {
            index.insert(source, MAX_BYTES, MAX_ENTRIES);
        }
        return Some(Ok(()));
    }
    let result = xirang_kernel::syntax::parse_face(source).map(|_| ());
    if result.is_ok()
        && let Ok(mut index) = cache.lock()
    {
        index.insert(source, MAX_BYTES, MAX_ENTRIES);
    }
    if result.is_ok() {
        super::ordinary_store::remember(source);
    }
    Some(result)
}

#[cfg(all(test, feature = "build"))]
#[path = "ordinary_tests.rs"]
mod tests;
