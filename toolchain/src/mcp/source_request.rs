//! Reuse one current inventory within a read request; never carry raw bytes across requests.
//! 在一次读取请求内复用当前清单；原文绝不跨请求保留。
use super::source_index::SourceFile;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[derive(Default)]
struct Request {
    sources: BTreeMap<(PathBuf, Vec<String>), Vec<SourceFile>>,
    bytes: usize,
}
thread_local! { static CURRENT: RefCell<Option<Request>> = const { RefCell::new(None) }; }
pub(super) struct Scope;
pub(super) fn scope() -> Scope {
    CURRENT.with(|current| *current.borrow_mut() = Some(Request::default()));
    Scope
}
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = None);
    }
}
pub(super) fn active() -> bool {
    CURRENT.with(|current| current.borrow().is_some())
}
pub(super) fn get(root: &Path, scopes: &[String]) -> Option<Vec<SourceFile>> {
    CURRENT.with(|current| {
        current.borrow().as_ref().and_then(|request| {
            request
                .sources
                .get(&(root.to_path_buf(), scopes.to_vec()))
                .cloned()
        })
    })
}
pub(super) fn put(root: &Path, scopes: &[String], sources: &[SourceFile]) {
    CURRENT.with(|current| {
        if let Some(request) = current.borrow_mut().as_mut() {
            let bytes = sources
                .iter()
                .map(|file| file.source.len() + file.relative.len())
                .sum::<usize>();
            if request.bytes + bytes <= 32 * 1024 * 1024 {
                request.bytes += bytes;
                request
                    .sources
                    .insert((root.to_path_buf(), scopes.to_vec()), sources.to_vec());
            }
        }
    });
}
