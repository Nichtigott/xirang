//! Join native provider/consumer obligations to the same executable verification ledger.
//! 将原生提供者/消费者义务连接到同一份可执行验证账本。
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn obligations(root: &Path, files: &Value) -> Result<(Vec<Value>, Vec<Value>), String> {
    let _metadata = crate::cargo_metadata::scope();
    let metadata = crate::cargo_metadata::read(&root.join("Cargo.toml"))?;
    let paths: Vec<_> = match files {
        Value::Array(paths) => paths.iter().filter_map(Value::as_str).collect(),
        Value::String(paths) => paths.split(',').map(str::trim).collect(),
        _ => Vec::new(),
    };
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut evidence = Vec::new();
    let mut obligations = Vec::new();
    for package in metadata["packages"].as_array().into_iter().flatten() {
        let native = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|dependency| {
                dependency["name"] == "xirang-toolchain" && dependency["kind"] == "build"
            });
        if !native {
            continue;
        }
        let Some(manifest) = package["manifest_path"].as_str() else {
            continue;
        };
        let Some(member) = Path::new(manifest).parent() else {
            continue;
        };
        if !member.starts_with(&root) {
            continue;
        }
        let namespace = super::registry::namespace(member)?;
        let answer = super::verify::verify(member, &json!({"limit":200}))?;
        let passed = answer.starts_with("verdict ok") && answer.contains("connector verdict: ok");
        obligations.push(json!({"name":format!("registration:{}",package["name"].as_str().unwrap_or("unknown")),"state":if passed {"passed"} else {"failed"},"expected":"passed","evidence":answer}));
        let faces = match crate::build_method::face_views(member, &namespace) {
            Ok(faces) => faces,
            Err(error) => {
                evidence.push(json!({"package":package["name"],"unknown":error}));
                continue;
            }
        };
        // A face's `source` is its **identity path**, relative to the layout's identity
        // base — `src/` for the historical layout, the package root for a target outside
        // it. Recomputing `member/src/<source>` matched every historical package and no
        // custom one, so a changed face in a `[lib] path = "source/lib.rs"` package left
        // the responsibility set empty while its registration still reported a pass.
        // 面的 `source` 是它的**身份路径**，相对布局的身份基准——历史布局下是 `src/`，目标在它之外时是
        // 包根。复算 `member/src/<source>` 能匹配每一个历史包、匹配不到任何自定义包，因此在
        // `[lib] path = "source/lib.rs"` 的包里改动了面之后，责任集合为空，而它的注册仍然报通过。
        // The layout is the base the record's own spelling is relative to. Without it the changed
        // files cannot be attributed to faces, and re-deriving `member/src/<source>` here is the
        // guessing that used to match every historical package and no custom one.
        // 布局是记录自身拼写所相对的基准。没有它就无法把改动文件归到面上，而在这里复算
        // `member/src/<source>` 正是那种"匹配每个历史包、匹配不到任何自定义包"的猜测。
        let Ok(layout) = crate::build_method::source_layout(member) else {
            continue;
        };
        let seeds: BTreeSet<_> = faces
            .iter()
            .filter(|face| {
                let Ok(source) = face.locate_source(&layout) else {
                    return false;
                };
                paths.iter().any(|path| {
                    let changed = root.join(path);
                    changed == source
                        || source
                            .parent()
                            .is_some_and(|parent| changed.starts_with(parent))
                })
            })
            .map(|face| face.id)
            .collect();
        if seeds.is_empty() {
            continue;
        }
        let packet = super::converge::packet(member, &namespace, &faces, Some(&seeds), None, 200);
        evidence.push(json!({"package":package["name"],"overview":packet.overview,
            "providers_contracts_consumers_grafts":packet.responsibilities,"runtime":packet.runtime,
            "execution":"workspace regression rows verify the selected configurations; individual consumer coverage remains unproven unless a test establishes it"}));
    }
    Ok((obligations, evidence))
}

#[cfg(test)]
#[path = "check_registry_tests.rs"]
mod check_registry_tests;
