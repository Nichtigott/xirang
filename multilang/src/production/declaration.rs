//! The declaration interchange a non-Rust program can emit: the **identity subset** of a
//! registration face (`name`, `kind`, `contracts`, plus `graft`/`flow` carried as opaque strings).
//! The Rust `#[soil]` parser stays the authority for Rust sources; this document is what another
//! language hands to the same vocabulary, and `xirang declaration check` normalises it so two
//! emitters that say the same thing compare equal.
//! 非 Rust 程序可以发出的声明交换格式：注册面的**身份子集**（`name`、`kind`、`contracts`，外加
//! `graft`/`flow` 作为不解释的字符串）。Rust 源码仍由 `#[soil]` 解析器作权威；这份文档是别的语言交给
//! 同一套词汇的东西，`xirang declaration check` 把它规范化，于是两个说同一件事的发射器比较相等。

use serde_json::{Map, Value, json};

/// One face declaration: the identity subset.
/// 一个注册面声明：身份子集。
pub struct Face {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) contracts: Vec<String>,
    pub(crate) graft: Option<String>,
    pub(crate) flow: Option<String>,
    pub(crate) preset: Option<String>,
    pub(crate) parts: Vec<String>,
    pub(crate) requires: std::collections::BTreeMap<String, String>,
    pub(crate) provides: Vec<String>,
    pub(crate) exports: Vec<String>,
    pub(crate) stable_name: Option<String>,
}

/// Whether `text` is a plain identifier (`[A-Za-z_][A-Za-z0-9_]*`).
/// `text` 是否是普通标识符。
fn identifier(text: &str) -> bool {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Whether `text` is one or more identifiers joined by `::`.
/// `text` 是否是一个或多个用 `::` 连接的标识符。
fn path(text: &str) -> bool {
    !text.is_empty() && text.split("::").all(identifier)
}

/// Parse one declaration document, refusing unknown fields by name.
/// 解析一份声明文档，未知字段按名拒绝。
pub(super) fn parse(text: &str) -> Result<Vec<Face>, String> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| format!("the declaration is not JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or("a declaration document is a JSON object")?;
    for key in object.keys() {
        if key != "protocol" && key != "faces" {
            return Err(format!(
                "unknown key `{key}`; a declaration document carries `protocol` and `faces`"
            ));
        }
    }
    match object.get("protocol") {
        Some(Value::Number(number)) if number.as_u64() == Some(1) => {}
        Some(other) => {
            return Err(format!(
                "this build speaks declaration protocol 1; the document says {other}"
            ));
        }
        None => {
            return Err(
                "the declaration has no `protocol`; this build speaks protocol 1".to_owned(),
            );
        }
    }
    let faces = object
        .get("faces")
        .and_then(Value::as_array)
        .ok_or("the declaration has no `faces` array")?;
    if faces.is_empty() {
        return Err("the declaration declares no face; an empty document says nothing".to_owned());
    }
    let mut parsed = Vec::new();
    for (index, face) in faces.iter().enumerate() {
        let face = face
            .as_object()
            .ok_or_else(|| format!("face {index} is not a JSON object"))?;
        for key in face.keys() {
            if ![
                "name",
                "kind",
                "contracts",
                "graft",
                "flow",
                "preset",
                "parts",
                "requires",
                "provides",
                "exports",
                "stable_name",
            ]
            .contains(&key.as_str())
            {
                return Err(format!(
                    "face {index} has an unknown key `{key}`; a face carries name, kind, contracts and \
                     optionally graft and flow"
                ));
            }
        }
        let name = string_field(face, "name", index)?;
        if !identifier(&name) {
            return Err(format!(
                "face {index} names `{name}`, which is not an identifier (letters, digits and `_`, \
                 starting with a letter or `_`)"
            ));
        }
        let kind = string_field(face, "kind", index)?;
        if !identifier(&kind) {
            return Err(format!(
                "face `{name}` has kind `{kind}`, which is not an identifier"
            ));
        }
        let contracts = face
            .get("contracts")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("face `{name}` has no `contracts` array"))?;
        if contracts.is_empty() {
            return Err(format!(
                "face `{name}` declares no contract; a face without a contract claims nothing"
            ));
        }
        let mut named: Vec<String> = Vec::new();
        for contract in contracts {
            let contract = contract
                .as_str()
                .ok_or_else(|| format!("face `{name}` has a contract that is not a string"))?;
            if !path(contract) {
                return Err(format!(
                    "face `{name}` has contract `{contract}`, which is not a path of identifiers \
                     joined by `::`"
                ));
            }
            if named.iter().any(|seen| seen == contract) {
                return Err(format!("face `{name}` names contract `{contract}` twice"));
            }
            named.push(contract.to_owned());
        }
        named.sort();
        let graft = optional_string(face, "graft", &name)?;
        let flow = optional_string(face, "flow", &name)?;
        let preset = optional_string(face, "preset", &name)?;
        if let Some(preset) = &preset
            && !path(preset)
        {
            return Err(format!(
                "face `{name}` names preset `{preset}`, which is not a path of identifiers joined by `::`"
            ));
        }
        let stable_name = optional_string(face, "stable_name", &name)?;
        if let Some(stable) = &stable_name
            && !identifier(stable)
        {
            return Err(format!(
                "face `{name}` has stable_name `{stable}`, which is not an identifier"
            ));
        }
        let parts = path_list(face, "parts", &name)?;
        let provides = identifier_list(face, "provides", &name)?;
        let exports = identifier_list(face, "exports", &name)?;
        let requires = requirement_map(face, &name)?;
        parsed.push(Face {
            name,
            kind,
            contracts: named,
            graft,
            flow,
            preset,
            parts,
            requires,
            provides,
            exports,
            stable_name,
        });
    }
    for (index, face) in parsed.iter().enumerate() {
        if parsed[..index].iter().any(|seen| seen.name == face.name) {
            return Err(format!("two faces are both named `{}`", face.name));
        }
    }
    Ok(parsed)
}

/// An optional array of identifier paths (`parts`), required to be non-empty when present.
/// 可选的标识符路径数组（`parts`）：出现时必须非空。
fn path_list(face: &Map<String, Value>, key: &str, name: &str) -> Result<Vec<String>, String> {
    let Some(value) = face.get(key) else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or_else(|| format!("face `{name}` has a `{key}` that is not an array"))?;
    if entries.is_empty() {
        return Err(format!(
            "face `{name}` has an empty `{key}` array; omit the key rather than say nothing"
        ));
    }
    let mut named: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry
            .as_str()
            .ok_or_else(|| format!("face `{name}` has a `{key}` entry that is not a string"))?;
        if !path(entry) {
            return Err(format!(
                "face `{name}` has `{key}` entry `{entry}`, which is not a path of identifiers joined by `::`"
            ));
        }
        if named.iter().any(|seen| seen == entry) {
            return Err(format!("face `{name}` names `{entry}` twice in `{key}`"));
        }
        named.push(entry.to_owned());
    }
    named.sort();
    Ok(named)
}

/// An optional array of plain identifiers (`provides`/`exports`).
/// 可选的普通标识符数组（`provides`/`exports`）。
fn identifier_list(
    face: &Map<String, Value>,
    key: &str,
    name: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = face.get(key) else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or_else(|| format!("face `{name}` has a `{key}` that is not an array"))?;
    if entries.is_empty() {
        return Err(format!(
            "face `{name}` has an empty `{key}` array; omit the key rather than say nothing"
        ));
    }
    let mut named: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry
            .as_str()
            .ok_or_else(|| format!("face `{name}` has a `{key}` entry that is not a string"))?;
        if !identifier(entry) {
            return Err(format!(
                "face `{name}` has `{key}` entry `{entry}`, which is not an identifier"
            ));
        }
        if named.iter().any(|seen| seen == entry) {
            return Err(format!("face `{name}` names `{entry}` twice in `{key}`"));
        }
        named.push(entry.to_owned());
    }
    named.sort();
    Ok(named)
}

/// Whether `text` is a capability name (`[A-Za-z0-9_.:-]+`).
/// `text` 是否是能力名（`[A-Za-z0-9_.:-]+`）。
fn capability(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._:-".contains(character))
}

/// The optional `requires` map: capability name to provider name.
/// 可选的 `requires` 映射：能力名到提供者名。
fn requirement_map(
    face: &Map<String, Value>,
    name: &str,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let Some(value) = face.get("requires") else {
        return Ok(std::collections::BTreeMap::new());
    };
    let entries = value
        .as_object()
        .ok_or_else(|| format!("face `{name}` has a `requires` that is not an object"))?;
    if entries.is_empty() {
        return Err(format!(
            "face `{name}` has an empty `requires` object; omit the key rather than say nothing"
        ));
    }
    let mut map = std::collections::BTreeMap::new();
    for (capability_name, provider) in entries {
        if !capability(capability_name) {
            return Err(format!(
                "face `{name}` requires capability `{capability_name}`, which is not a capability name"
            ));
        }
        let provider = provider
            .as_str()
            .ok_or_else(|| format!("face `{name}` has a `requires` value that is not a string"))?;
        if !capability(provider) {
            return Err(format!(
                "face `{name}` requires `{capability_name}` from `{provider}`, which is not a capability name"
            ));
        }
        map.insert(capability_name.clone(), provider.to_owned());
    }
    Ok(map)
}

/// One required string field.
/// 一个必填字符串字段。
fn string_field(face: &Map<String, Value>, key: &str, index: usize) -> Result<String, String> {
    face.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("face {index} has no string `{key}`"))
}

/// One optional string field.
/// 一个可选字符串字段。
fn optional_string(
    face: &Map<String, Value>,
    key: &str,
    name: &str,
) -> Result<Option<String>, String> {
    match face.get(key) {
        None => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        Some(_) => Err(format!(
            "face `{name}` has a `{key}` that is not a non-empty string; this subset carries it \
             as text and does not interpret it"
        )),
    }
}

/// Render the canonical form: sorted keys, sorted contracts, one line.
/// 渲染规范形式：键排序、契约排序、一行。
pub(super) fn canonical(faces: &[Face]) -> Result<String, String> {
    let faces: Vec<Value> = faces
        .iter()
        .map(|face| {
            let mut object = Map::new();
            object.insert("name".to_owned(), json!(face.name));
            object.insert("kind".to_owned(), json!(face.kind));
            object.insert("contracts".to_owned(), json!(face.contracts));
            if let Some(graft) = &face.graft {
                object.insert("graft".to_owned(), json!(graft));
            }
            if let Some(flow) = &face.flow {
                object.insert("flow".to_owned(), json!(flow));
            }
            if let Some(preset) = &face.preset {
                object.insert("preset".to_owned(), json!(preset));
            }
            if !face.parts.is_empty() {
                object.insert("parts".to_owned(), json!(face.parts));
            }
            if !face.requires.is_empty() {
                object.insert("requires".to_owned(), json!(face.requires));
            }
            if !face.provides.is_empty() {
                object.insert("provides".to_owned(), json!(face.provides));
            }
            if !face.exports.is_empty() {
                object.insert("exports".to_owned(), json!(face.exports));
            }
            if let Some(stable) = &face.stable_name {
                object.insert("stable_name".to_owned(), json!(stable));
            }
            Value::Object(object)
        })
        .collect();
    serde_json::to_string(&json!({"protocol": 1, "faces": faces}))
        .map_err(|error| format!("cannot render the canonical declaration: {error}"))
}

/// Read a declaration document, normalise it, and print the canonical form.
/// 读取一份声明文档，规范化，并打印规范形式。
pub fn check_document(path: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let faces = parse(&text)?;
    canonical(&faces)
}

/// The starter sources for one face in one language. The program prints the declaration.
/// 一门语言里一个面的起点源码。程序会打印那份声明。
pub fn starter(language: &str, face: &Face) -> Result<(&'static str, String), String> {
    if !identifier(&face.name) || !identifier(&face.kind) {
        return Err(format!(
            "`{}`/`{}` are not identifiers; a starter is named after the face it declares",
            face.name, face.kind
        ));
    }
    let document = canonical(std::slice::from_ref(face))?;
    match language {
        "python" => Ok((
            "py",
            format!(
                "# Generated by `xirang declaration new --language python`. Replace the contracts and\n\
                 # add `graft`/`flow` where the face needs them; `xirang declaration check` normalises it.\n\
                 import json\n\n\
                 print(json.dumps({document}, sort_keys=True, separators=(\",\", \":\")))\n"
            ),
        )),
        "cpp" => Ok((
            "cpp",
            format!(
                "// Generated by `xirang declaration new --language cpp`. Replace the contracts and add\n\
                 // `graft`/`flow` where the face needs them; `xirang declaration check` normalises it.\n\
                 #include <iostream>\n\n\
                 int main() {{\n    std::cout << R\"json({document})json\" << '\\n';\n    return 0;\n}}\n"
            ),
        )),
        other => Err(format!(
            "no declaration starter for `{other}`; this slice writes python and cpp"
        )),
    }
}
