//! Language-neutral ABI facts: declarations, definitions and loads of one boundary symbol.
//! 语言无关的 ABI 事实：一个边界符号的声明、定义与加载点。
//!
//! A call graph that only follows syntax edges cannot cross a language boundary. This module makes
//! the boundary object first-class: an [`AbiSymbol`] plus a normalized [`AbiSignature`] is declared
//! in one language, defined in another, and loaded by a third. Matching is by the **normalized ABI
//! signature**, not by a same-named syntax node; a mismatch is reported as a boundary diagnostic
//! instead of being silently connected.
//! 只沿语法边的调用图跨不过语言边界。本模块把边界对象做成一等：一个 [`AbiSymbol`] 加规范化
//! [`AbiSignature`]，一门语言声明、另一门定义、第三门加载。匹配按**规范化 ABI 签名**，不按同名语法
//! 节点；不匹配会报边界诊断，而不是静默连边。

/// The language one ABI site was read from.
/// 一个 ABI 站点来自哪种语言。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbiLanguage {
    /// Rust `extern "C"` declarations/definitions.
    /// Rust 的 `extern "C"` 声明/定义。
    Rust,
    /// C++ `extern "C"` declarations/definitions.
    /// C++ 的 `extern "C"` 声明/定义。
    Cpp,
    /// A Python `ctypes` loader.
    /// Python 的 `ctypes` 加载点。
    Python,
}

/// What an ABI site is doing at the boundary.
/// 一个 ABI 站点在边界上做什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiRole {
    /// It promises the symbol; the definition may live elsewhere.
    /// 它承诺这个符号；定义可能在别处。
    Declaration,
    /// It supplies the implementation.
    /// 它提供实现。
    Definition,
    /// It loads the symbol from a library at run time.
    /// 它在运行期从库加载这个符号。
    Load,
}

/// A normalized ABI type. The spelling belongs to the source; the canonical value is what can be
/// compared across languages.
/// 规范化的 ABI 类型。拼写属于源码；规范值才能跨语言比较。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AbiType {
    /// An `i8`/`int8_t` value.
    /// `i8`/`int8_t` 值。
    I8,
    /// An `i16`/`int16_t` value.
    /// `i16`/`int16_t` 值。
    I16,
    /// An `i32`/`int32_t` value.
    /// `i32`/`int32_t` 值。
    I32,
    /// An `i64`/`int64_t` value.
    /// `i64`/`int64_t` 值。
    I64,
    /// A `u8`/`uint8_t` value.
    /// `u8`/`uint8_t` 值。
    U8,
    /// A `u16`/`uint16_t` value.
    /// `u16`/`uint16_t` 值。
    U16,
    /// A `u32`/`uint32_t` value.
    /// `u32`/`uint32_t` 值。
    U32,
    /// A `u64`/`uint64_t` value.
    /// `u64`/`uint64_t` 值。
    U64,
    /// A `usize`/`size_t` value.
    /// `usize`/`size_t` 值。
    USize,
    /// A 32-bit float.
    /// 32 位浮点。
    F32,
    /// A 64-bit float.
    /// 64 位浮点。
    F64,
    /// A boolean.
    /// 布尔值。
    Bool,
    /// No return value.
    /// 无返回值。
    Void,
    /// A pointer to another ABI type.
    /// 指向另一个 ABI 类型的指针。
    Pointer(Box<AbiType>),
    /// A spelling this reader could not normalize; it never matches another unknown silently.
    /// 本读取器无法规范化的拼写；它不会静默匹配另一个 unknown。
    Unknown(String),
}

/// A normalized boundary signature.
/// 规范化的边界签名。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbiSignature {
    /// Parameter types in order.
    /// 按顺序的参数类型。
    pub params: Vec<AbiType>,
    /// The return type; `None` for a declaration that writes no arrow (C `void`).
    /// 返回类型；没有写箭头的声明（C 的 `void`）为 `None`。
    pub ret: Option<AbiType>,
}

/// One boundary symbol: the name and the calling convention.
/// 一个边界符号：名字与调用约定。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbiSymbol {
    /// The symbol name the linker sees.
    /// 链接器看到的符号名。
    pub name: String,
    /// The calling convention, normalized to the spelling `C`/`system`/a custom string.
    /// 调用约定，规范为 `C`/`system`/自定义字符串。
    pub convention: String,
}

/// One declaration, definition or load of an ABI symbol.
/// 一个 ABI 符号的声明、定义或加载点。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbiSite {
    /// Which language wrote this site.
    /// 这个站点由哪种语言写下。
    pub language: AbiLanguage,
    /// The site's source label (package-relative).
    /// 站点的源码标签（包内相对）。
    pub source: String,
    /// One-based line.
    /// 从 1 开始的行号。
    pub line: usize,
    /// Declaration, definition or load.
    /// 声明、定义或加载。
    pub role: AbiRole,
    /// The boundary symbol.
    /// 边界符号。
    pub symbol: AbiSymbol,
    /// The normalized signature.
    /// 规范化签名。
    pub signature: AbiSignature,
    /// The registration face that owns this source, when the caller knows it.
    /// 拥有这份源码的注册面（调用方知道时）。
    pub owner: Option<String>,
}

/// One source the graph is built from.
/// 构建图的一份源码。
#[derive(Clone, Copy, Debug)]
pub struct AbiSource<'a> {
    /// The language the source is written in.
    /// 源码所用语言。
    pub language: AbiLanguage,
    /// The package-relative label.
    /// 包内相对标签。
    pub label: &'a str,
    /// The source text.
    /// 源码文本。
    pub source: &'a str,
    /// The registration face that owns this source, when known.
    /// 拥有这份源码的注册面（已知时）。
    pub owner: Option<&'a str>,
}

/// Why a declaration and a definition did not become one edge.
/// 声明与定义为什么没有连成一条边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryDiagnostic {
    /// A declaration/load has no definition in the supplied sources.
    /// 声明/加载在所给源码里没有定义。
    MissingDefinition {
        /// The symbol.
        /// 符号。
        symbol: String,
        /// The signature the declaration promised.
        /// 声明承诺的签名。
        signature: AbiSignature,
        /// The sites that promised it.
        /// 承诺它的站点。
        sites: Vec<usize>,
    },
    /// More than one definition supplies the symbol.
    /// 不止一个定义提供这个符号。
    AmbiguousDefinition {
        /// The symbol.
        /// 符号。
        symbol: String,
        /// The competing definitions.
        /// 竞争的定义。
        sites: Vec<usize>,
    },
    /// The declaration and definition normalize to different signatures.
    /// 声明与定义规范化后签名不同。
    SignatureMismatch {
        /// The symbol.
        /// 符号。
        symbol: String,
        /// The declaration's signature.
        /// 声明签名。
        declared: AbiSignature,
        /// The definition's signature.
        /// 定义签名。
        defined: AbiSignature,
    },
}

/// One matched boundary edge: a declaration/load site and its definition.
/// 一条匹配的边界边：声明/加载站点与它的定义。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbiEdge {
    /// Index into [`AbiGraph::sites`] of the declaration or load.
    /// 声明/加载在 [`AbiGraph::sites`] 中的下标。
    pub declaration: usize,
    /// Index into [`AbiGraph::sites`] of the definition.
    /// 定义在 [`AbiGraph::sites`] 中的下标。
    pub definition: usize,
}

/// The boundary graph: every site, the edges that really matched, and the reasons the rest did not.
/// 边界图：每个站点、真正匹配的边，以及其余没有匹配的原因。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbiGraph {
    /// Every site in input order.
    /// 按输入顺序的每个站点。
    pub sites: Vec<AbiSite>,
    /// Matched declaration/load → definition edges.
    /// 匹配的 声明/加载 → 定义 边。
    pub edges: Vec<AbiEdge>,
    /// Unmatched or ambiguous symbols.
    /// 未匹配或有歧义的符号。
    pub diagnostics: Vec<BoundaryDiagnostic>,
}

/// Read every source, then match by `(convention, name)` and normalized signature.
/// 读取每份源码，然后按 `(调用约定, 名字)` 与规范化签名匹配。
pub fn build_graph(sources: &[AbiSource<'_>]) -> AbiGraph {
    let mut graph = AbiGraph::default();
    for input in sources {
        let owner = input.owner.map(str::to_owned);
        let mut sites = match input.language {
            AbiLanguage::Rust => read_rust(input.source, input.label),
            AbiLanguage::Cpp => read_cpp(input.source, input.label),
            AbiLanguage::Python => read_python(input.source, input.label),
        };
        for site in &mut sites {
            site.owner = owner.clone();
        }
        graph.sites.extend(sites);
    }
    // Group indexes by the boundary identity.
    let mut by_key: std::collections::BTreeMap<(String, String), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, site) in graph.sites.iter().enumerate() {
        by_key
            .entry((site.symbol.convention.clone(), site.symbol.name.clone()))
            .or_default()
            .push(index);
    }
    for ((_, symbol), indexes) in by_key {
        let definitions: Vec<usize> = indexes
            .iter()
            .copied()
            .filter(|index| graph.sites[*index].role == AbiRole::Definition)
            .collect();
        let promises: Vec<usize> = indexes
            .iter()
            .copied()
            .filter(|index| graph.sites[*index].role != AbiRole::Definition)
            .collect();
        if definitions.len() > 1 {
            graph
                .diagnostics
                .push(BoundaryDiagnostic::AmbiguousDefinition {
                    symbol: symbol.clone(),
                    sites: definitions,
                });
            continue;
        }
        for declaration in promises {
            let Some(&definition) = definitions.first() else {
                graph
                    .diagnostics
                    .push(BoundaryDiagnostic::MissingDefinition {
                        symbol: symbol.clone(),
                        signature: graph.sites[declaration].signature.clone(),
                        sites: vec![declaration],
                    });
                continue;
            };
            if graph.sites[declaration].signature != graph.sites[definition].signature {
                graph
                    .diagnostics
                    .push(BoundaryDiagnostic::SignatureMismatch {
                        symbol: symbol.clone(),
                        declared: graph.sites[declaration].signature.clone(),
                        defined: graph.sites[definition].signature.clone(),
                    });
                continue;
            }
            graph.edges.push(AbiEdge {
                declaration,
                definition,
            });
        }
    }
    graph
}

/// Normalize a type spelling from Rust, C or C++.
/// 规范化来自 Rust、C 或 C++ 的类型拼写。
pub fn normalize_type(spelling: &str) -> AbiType {
    let text = spelling
        .trim()
        .trim_start_matches('&')
        .trim_start_matches("mut ")
        .trim_start_matches("const ")
        .trim();
    let text = text.strip_prefix("std::").unwrap_or(text);
    let text = text.strip_prefix("ctypes.").unwrap_or(text);
    if let Some(pointer) = text.strip_suffix('*') {
        return AbiType::Pointer(Box::new(normalize_type(pointer)));
    }
    if let Some(pointer) = text
        .strip_prefix("*const ")
        .or_else(|| text.strip_prefix("*mut "))
    {
        return AbiType::Pointer(Box::new(normalize_type(pointer)));
    }
    match text {
        "i8" | "int8_t" | "signed char" => AbiType::I8,
        "i16" | "int16_t" | "short" => AbiType::I16,
        "i32" | "int32_t" | "int" => AbiType::I32,
        "i64" | "int64_t" | "long long" => AbiType::I64,
        "u8" | "uint8_t" | "unsigned char" => AbiType::U8,
        "u16" | "uint16_t" | "unsigned short" => AbiType::U16,
        "u32" | "uint32_t" | "unsigned int" => AbiType::U32,
        "u64" | "uint64_t" | "unsigned long long" => AbiType::U64,
        "usize" | "size_t" => AbiType::USize,
        "f32" | "float" => AbiType::F32,
        "f64" | "double" => AbiType::F64,
        "c_int8" => AbiType::I8,
        "c_int16" => AbiType::I16,
        "c_int32" => AbiType::I32,
        "c_int64" => AbiType::I64,
        "c_uint8" => AbiType::U8,
        "c_uint16" => AbiType::U16,
        "c_uint32" => AbiType::U32,
        "c_uint64" => AbiType::U64,
        "c_float" => AbiType::F32,
        "c_double" => AbiType::F64,
        "c_bool" => AbiType::Bool,
        "c_void_p" => AbiType::Pointer(Box::new(AbiType::Void)),
        "bool" | "_Bool" => AbiType::Bool,
        "void" | "()" => AbiType::Void,
        other => AbiType::Unknown(other.to_owned()),
    }
}

fn split_params(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (index, character) in text.char_indices() {
        match character {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if !text[start..].trim().is_empty() {
        parts.push(text[start..].trim());
    }
    parts
}

fn rust_signature(params: &str, ret: Option<&str>) -> AbiSignature {
    AbiSignature {
        params: split_params(params)
            .into_iter()
            .map(|param| normalize_type(param.rsplit_once(':').map_or(param, |(_, ty)| ty)))
            .collect(),
        ret: ret
            .map(normalize_type)
            .filter(|ty| *ty != AbiType::Void)
            .map_or(Some(AbiType::Void), Some),
    }
}

fn c_signature(ret: &str, params: &str) -> AbiSignature {
    AbiSignature {
        params: split_params(params)
            .into_iter()
            .map(|param| {
                let without_name = param
                    .rsplit_once(' ')
                    .filter(|(_, name)| {
                        !name.is_empty()
                            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    })
                    .map_or(param, |(ty, _)| ty);
                normalize_type(without_name)
            })
            .collect(),
        ret: Some(normalize_type(ret))
            .filter(|ty| *ty != AbiType::Void)
            .map_or(Some(AbiType::Void), Some),
    }
}

fn signature_between(line: &str, open: usize) -> Option<(String, String)> {
    let mut depth = 0;
    let start = open + 1;
    for (index, character) in line.char_indices().skip(start) {
        match character {
            '(' => depth += 1,
            ')' if depth == 0 => {
                let rest = &line[index + 1..];
                let ret = rest
                    .split_once("->")
                    .map(|(_, ret)| ret.trim().trim_end_matches(['{', ';', ' ']))
                    .unwrap_or("");
                return Some((line[start..index].to_owned(), ret.to_owned()));
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn name_before(line: &str, open: usize) -> Option<String> {
    let before = line[..open].trim();
    let name = before.split_whitespace().last()?;
    let name = name.trim_matches('*');
    (!name.is_empty()).then(|| name.to_owned())
}

fn read_rust(source: &str, label: &str) -> Vec<AbiSite> {
    let mut sites = Vec::new();
    let mut in_extern = false;
    for (index, line) in source.lines().enumerate() {
        let text = line.trim();
        if text.contains("extern \"C\"") && text.contains('{') {
            in_extern = true;
        }
        let definition = text.contains("extern \"C\" fn");
        let declaration = in_extern && text.starts_with("fn ");
        if !definition && !declaration {
            if in_extern && text.starts_with('}') {
                in_extern = false;
            }
            continue;
        }
        let Some(open) = text.find('(') else { continue };
        let Some(name) = name_before(text, open) else {
            continue;
        };
        let Some((params, ret_from_body)) = signature_between(text, open) else {
            continue;
        };
        let ret = ret_from_body.trim();
        sites.push(AbiSite {
            language: AbiLanguage::Rust,
            source: label.to_owned(),
            line: index + 1,
            role: if definition {
                AbiRole::Definition
            } else {
                AbiRole::Declaration
            },
            symbol: AbiSymbol {
                name,
                convention: "C".to_owned(),
            },
            signature: rust_signature(&params, if ret.is_empty() { None } else { Some(ret) }),
            owner: None,
        });
        if text.starts_with('}') {
            in_extern = false;
        }
    }
    sites
}

fn read_cpp(source: &str, label: &str) -> Vec<AbiSite> {
    let mut sites = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let text = line.trim();
        let Some(extern_at) = text.find("extern \"C\"") else {
            continue;
        };
        let rest = text[extern_at + "extern \"C\"".len()..].trim();
        if rest.starts_with('{') {
            // The block form is read line by line below; the header itself names no symbol.
            continue;
        }
        let Some(open) = rest.find('(') else { continue };
        let Some(name) = name_before(rest, open) else {
            continue;
        };
        let Some((params, _)) = signature_between(rest, open) else {
            continue;
        };
        let ret = rest[..open]
            .trim()
            .strip_suffix(&name)
            .map(str::trim)
            .unwrap_or("");
        let role = if rest.contains('{') {
            AbiRole::Definition
        } else {
            AbiRole::Declaration
        };
        sites.push(AbiSite {
            language: AbiLanguage::Cpp,
            source: label.to_owned(),
            line: index + 1,
            role,
            symbol: AbiSymbol {
                name,
                convention: "C".to_owned(),
            },
            signature: c_signature(ret, &params),
            owner: None,
        });
    }
    sites
}

fn read_python(source: &str, label: &str) -> Vec<AbiSite> {
    // A small, explicit ctypes shape: `<var> = <lib>.<symbol>`, then `<var>.argtypes = [...]` and
    // `<var>.restype = ...`. Anything else is not guessed.
    // 小而明确的 ctypes 形状：`<var> = <lib>.<symbol>`，随后 `<var>.argtypes = [...]` 与
    // `<var>.restype = ...`。其余不猜。
    let mut aliases: std::collections::BTreeMap<String, (String, usize)> =
        std::collections::BTreeMap::new();
    let mut argtypes: std::collections::BTreeMap<String, Vec<AbiType>> =
        std::collections::BTreeMap::new();
    let mut restype: std::collections::BTreeMap<String, AbiType> =
        std::collections::BTreeMap::new();
    for (index, line) in source.lines().enumerate() {
        let text = line.trim();
        if let Some((left, right)) = text.split_once('=')
            && !left.contains('.')
            && !right.contains("ctypes.")
            && !right.contains("CDLL")
            && let Some((_, symbol)) = right.trim().rsplit_once('.')
            && !symbol.is_empty()
            && symbol
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            aliases.insert(left.trim().to_owned(), (symbol.to_owned(), index + 1));
        }
        if let Some((left, right)) = text.split_once(".argtypes")
            && let Some((_, list)) = right.split_once('[')
            && let Some((list, _)) = list.split_once(']')
        {
            argtypes.insert(
                left.trim().to_owned(),
                split_params(list)
                    .into_iter()
                    .map(|item| normalize_type(item.strip_prefix("ctypes.").unwrap_or(item)))
                    .collect(),
            );
        }
        if let Some((left, right)) = text.split_once(".restype")
            && let Some((_, ty)) = right.split_once('=')
        {
            restype.insert(left.trim().to_owned(), normalize_type(ty.trim()));
        }
    }
    aliases
        .into_iter()
        .map(|(variable, (name, line))| {
            let params = argtypes.get(&variable).cloned().unwrap_or_default();
            let ret = restype.get(&variable).cloned();
            AbiSite {
                language: AbiLanguage::Python,
                source: label.to_owned(),
                line,
                role: AbiRole::Load,
                symbol: AbiSymbol {
                    name,
                    convention: "C".to_owned(),
                },
                signature: AbiSignature { params, ret },
                owner: None,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "abi_graph_tests.rs"]
mod abi_graph_tests;
