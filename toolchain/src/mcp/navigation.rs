//! Bounded directory navigation and byte-cursor readback for large immutable logs.
//! 有界目录导航与大型不可变日志的字节游标续读。
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) fn read(root: &Path, args: &Value) -> Result<Option<String>, String> {
    let relative = super::source_index::required_path(args)?;
    let base = root.canonicalize().map_err(|e| e.to_string())?;
    let path = root
        .join(&relative)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(&base) {
        return Err("path escapes project root".into());
    }
    if path.is_dir() {
        let offset = number(args, "offset", 0, usize::MAX)?;
        let limit = number(args, "limit", 80, 200)?.max(1);
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let child = entry.path();
            let Ok(resolved) = child.canonicalize() else {
                continue;
            };
            if !resolved.starts_with(&base) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            entries.push(format!(
                "{name}{}",
                if resolved.is_dir() { "/" } else { "" }
            ));
        }
        entries.sort();
        let mut output = format!("directory {relative}; {} entries\n", entries.len());
        let mut next = offset;
        for entry in entries.iter().skip(offset).take(limit) {
            if output.len() + entry.len() > 23_000 {
                break;
            }
            output.push_str(entry);
            output.push('\n');
            next += 1;
        }
        if next < entries.len() {
            output.push_str(&format!(
                "withheld {} entries; next offset {next}\n",
                entries.len() - next
            ));
        }
        return Ok(Some(output));
    }
    if args.get("offset_bytes").is_none() {
        return Ok(None);
    }
    let offset = number(args, "offset_bytes", 0, usize::MAX)?;
    let budget = number(args, "max_bytes", 12_000, 24_000)?.max(4);
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let total = file.metadata().map_err(|e| e.to_string())?.len();
    file.seek(SeekFrom::Start(offset as u64))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(budget as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let mut end = bytes.len();
    let text = loop {
        match std::str::from_utf8(&bytes[..end]) {
            Ok(text) => break text,
            Err(error) if error.error_len().is_none() && error.valid_up_to() > 0 => {
                end = error.valid_up_to()
            }
            Err(_) => {
                return Err("byte cursor is not a UTF-8 boundary or file is not UTF-8 text".into());
            }
        }
    };
    Ok(Some(format!(
        "file {relative}; bytes {offset}-{} of {total}; next offset_bytes {}\n{text}",
        offset + end,
        offset + end
    )))
}
fn number(args: &Value, key: &str, default: usize, max: usize) -> Result<usize, String> {
    match args.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|n| *n <= max as u64)
            .map(|n| n as usize)
            .ok_or_else(|| format!("invalid {key}; maximum {max}")),
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod navigation_tests;
