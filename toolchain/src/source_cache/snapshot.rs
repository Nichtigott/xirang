//! Optional shared syntax snapshots. Every caller still discovers and reads current input bytes.
//! 可选共享语法快照；调用方仍发现并读取当前输入字节。
use super::facts::{CACHE, Unit};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
};

const LIMIT: u64 = 64 * 1024 * 1024;
thread_local! { static ACTIVE: RefCell<Vec<Session>> = const { RefCell::new(Vec::new()) }; }
struct Session {
    path: PathBuf,
    used: HashMap<usize, (String, Arc<Unit>)>,
    dirty: bool,
    bytes: usize,
}
pub(crate) struct Scope;

fn schema() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| {
        format!(
            "syntax-snapshot-2-{}",
            xirang_kernel::source::parser_fingerprint()
        )
    })
}

pub(crate) fn scope(root: &Path) -> Scope {
    let path = root.join("target/xirang/cache/syntax-facts.json");
    static LOADED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    let first = LOADED
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(path.clone());
    if first {
        restore(&path);
    }
    ACTIVE.with(|active| {
        active.borrow_mut().push(Session {
            path,
            used: HashMap::new(),
            dirty: false,
            bytes: 0,
        })
    });
    Scope
}

pub(super) fn used(source: &str, unit: Arc<Unit>, miss: bool) {
    // Tiny call-free leaves cost less to reparse than to persist; keep their memory cache only.
    // 小型无调用叶子重新解析比持久化更便宜；仅保留其内存缓存。
    if source.len() < 1024 && unit.functions.len() < 2 && unit.calls.iter().all(Vec::is_empty) {
        return;
    }
    ACTIVE.with(|active| {
        if let Some(session) = active.borrow_mut().last_mut() {
            let key = Arc::as_ptr(&unit) as usize;
            if session.used.contains_key(&key) {
                return;
            }
            // Snapshot retention is independently bounded; overflow never affects the answer.
            // 快照留存单独有界；超过上限不影响查询答案。
            let bytes = source.len().saturating_mul(8) + unit.bytes();
            if session.used.len() < 131_072 && session.bytes + bytes <= LIMIT as usize {
                session.bytes += bytes;
                session.used.insert(key, (source.to_owned(), unit));
                session.dirty |= miss;
            }
        }
    });
}

fn restore(path: &Path) {
    if !path.metadata().is_ok_and(|meta| meta.len() <= LIMIT) {
        return;
    }
    let Some(value) = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return;
    };
    if value["schema"].as_str() != Some(schema()) {
        return;
    }
    let Some(payload) = value["payload"].as_str() else {
        return;
    };
    if value["digest"].as_str() != Some(xirang_kernel::sha256_hex(payload.as_bytes()).as_str()) {
        return;
    }
    let Some(rows) = serde_json::from_str::<Vec<Value>>(payload).ok() else {
        return;
    };
    for row in rows {
        let Some(source) = row[0].as_str() else {
            continue;
        };
        let Some(unit) = decode(&row) else {
            continue;
        };
        CACHE
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(source, Arc::new(unit));
    }
}
fn decode(row: &Value) -> Option<Unit> {
    let mut functions = Vec::new();
    let mut calls = Vec::new();
    for f in row[1].as_array()? {
        functions.push(xirang_kernel::source::SourceFunction {
            name: f[0].as_str()?.into(),
            signature: f[1].as_str()?.into(),
            body: f[2].as_str()?.into(),
            line: f[3].as_u64()?.try_into().ok()?,
            end_line: f[4].as_u64()?.try_into().ok()?,
        });
        calls.push(
            f[5].as_array()?
                .iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
        );
    }
    Some(Unit {
        functions,
        calls,
        #[cfg(feature = "mcp")]
        branches: OnceLock::new(),
    })
}
impl Drop for Scope {
    fn drop(&mut self) {
        let Some(session) = ACTIVE.with(|active| active.borrow_mut().pop()) else {
            return;
        };
        if !session.dirty {
            return;
        }
        // Serialize borrowed tuples once; avoid per-field JSON allocations and repeated encoding.
        // 借用字段元组只序列化一次，避免逐字段 JSON 分配与重复编码。
        let mut payload = vec![b'['];
        for (source, unit) in session.used.values() {
            let functions: Vec<_> = unit
                .functions
                .iter()
                .zip(&unit.calls)
                .map(|(f, calls)| (&f.name, &f.signature, &f.body, f.line, f.end_line, calls))
                .collect();
            let start = payload.len();
            if start > 1 {
                payload.push(b',');
            }
            if serde_json::to_writer(&mut payload, &(source, functions)).is_err() {
                return;
            }
            if payload.len() > LIMIT as usize / 3 {
                payload.truncate(start);
                break;
            }
        }
        payload.push(b']');
        let payload = String::from_utf8(payload).expect("JSON is UTF-8");
        let data = json!({"schema":schema(),"digest":xirang_kernel::sha256_hex(payload.as_bytes()),"payload":payload}).to_string();
        let Some(parent) = session.path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temporary = parent.join(format!(
            ".syntax-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        if std::fs::write(&temporary, data).is_ok() {
            let _ = std::fs::rename(&temporary, session.path);
        }
        let _ = std::fs::remove_file(temporary);
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
