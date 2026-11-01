//! A JSON Schema for `xirang.toml`, for editors. The parser remains the authority: this schema is
//! a hint that catches typos and offers completion. The top-level tables and the built-in language
//! tables are pinned to the parser by tests; an external language's keys cannot be known in
//! advance, so any `language.<id>` table accepts string/integer/boolean values generically.
//! 供编辑器用的 `xirang.toml` JSON Schema。解析器仍是权威：本 schema 是抓拼写错与补全的提示。
//! 顶层表与内置语言表由测试钉在解析器上；外部语言的键无法预先知道，因此任意
//! `language.<id>` 表都按字符串/整数/布尔值泛型接受。

use serde_json::{Value, json};

/// The built-in language ids, in the order this build implements them.
/// 本次构建实现的内置语言 id，按实现顺序。
pub(super) const LANGUAGES: [&str; 13] = [
    "cpp",
    "c",
    "python",
    "rust",
    "go",
    "ruby",
    "csharp",
    "java",
    "php",
    "swift",
    "kotlin",
    "javascript",
    "typescript",
];

/// One built-in language table: `standard` is the only key the parser accepts.
/// 一个内置语言表：解析器只接受 `standard` 这一个键。
fn standard() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["standard"],
        "properties": {"standard": {"type": "string", "minLength": 1}}
    })
}

/// The schema document.
/// schema 文档。
pub(super) fn document() -> Value {
    let mut languages = serde_json::Map::new();
    for id in LANGUAGES {
        languages.insert(id.to_owned(), standard());
    }
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xirang.invalid/xirang.toml",
        "title": "xirang.toml",
        "description": "A XiRang multilang project. The parser is the authority; this schema is an editing hint. An external language's keys are backend-owned and only checked by its backend.",
        "type": "object",
        "additionalProperties": false,
        "required": ["schema", "package"],
        "properties": {
            "schema": {"const": 1},
            "package": {
                "type": "object",
                "additionalProperties": false,
                "required": ["name", "version", "kind"],
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "version": {"type": "string", "minLength": 1},
                    "kind": {"enum": ["library", "binary"]}
                }
            },
            "build": {
                "type": "object",
                "additionalProperties": false,
                "properties": {"profile": {"type": "string", "minLength": 1}}
            },
            "language": {
                "type": "object",
                "additionalProperties": false,
                "properties": languages,
                "patternProperties": {
                    "^[a-z][a-z0-9_-]*$": {
                        "type": "object",
                        "additionalProperties": {"type": ["string", "integer", "boolean"]}
                    }
                }
            },
            "dependencies": {
                "type": "object",
                "additionalProperties": {
                    "oneOf": [
                        {"$ref": "#/$defs/dependency_path"},
                        {"$ref": "#/$defs/dependency_source"}
                    ]
                }
            },
            "sources": {
                "type": "object",
                "additionalProperties": {
                    "oneOf": [
                        {"type": "string"},
                        {"$ref": "#/$defs/source_path"},
                        {"$ref": "#/$defs/source_git"},
                        {"$ref": "#/$defs/source_registry"}
                    ]
                }
            },
            "overrides": {
                "type": "object",
                "propertyNames": {"pattern": "^[^/]+/[^/]+$"},
                "additionalProperties": {"type": "string"}
            },
            "export": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "params", "returns"],
                    "properties": {
                        "name": {"type": "string", "minLength": 1},
                        "params": {"type": "array", "items": {"type": "string"}},
                        "returns": {"type": "string"},
                        "errors": {"type": "array", "items": {"type": "string"}}
                    }
                }
            }
        },
        "$defs": {
            "dependency_path": {
                "type": "object",
                "additionalProperties": false,
                "required": ["path"],
                "properties": {"path": {"type": "string", "minLength": 1}}
            },
            "dependency_source": {
                "type": "object",
                "additionalProperties": false,
                "required": ["source", "package"],
                "properties": {
                    "source": {"type": "string", "minLength": 1},
                    "package": {"type": "string", "minLength": 1}
                }
            },
            "source_path": {
                "type": "object",
                "additionalProperties": false,
                "required": ["path"],
                "properties": {"path": {"type": "string", "minLength": 1}}
            },
            "source_git": {
                "type": "object",
                "additionalProperties": false,
                "required": ["git", "rev"],
                "properties": {
                    "git": {"type": "string", "minLength": 1},
                    "rev": {"type": "string", "minLength": 1}
                }
            },
            "source_registry": {
                "type": "object",
                "additionalProperties": false,
                "required": ["registry"],
                "properties": {"registry": {"type": "string", "minLength": 1}}
            }
        }
    })
}

/// The schema as indented JSON.
/// 缩进后的 schema JSON。
pub(super) fn render() -> Result<String, String> {
    serde_json::to_string_pretty(&document())
        .map_err(|error| format!("cannot render the schema: {error}"))
}
