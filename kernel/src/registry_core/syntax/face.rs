//! Registration-face parser shared by build-time checks and live authoring.
//! 构建期检查与实时编辑共用的注册面解析器。
//!
//! This page owns the face grammar: finding declarations in a Rust file,
//! exposing their fields, and splicing a replacement macro back into the source.
//! The lexical helpers live in `tokens` and the field-value decoders in
//! `fields`; both are re-exported here, so `syntax::*` keeps its one namespace.
//! 本页拥有注册面语法：在 Rust 文件中发现声明、暴露其字段，并把替换用宏拼回源码。
//! 词法辅助函数位于 `tokens`，字段取值解码位于 `fields`；两者都在此再导出，
//! `syntax::*` 因此仍只有一个命名空间。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proc_macro2::{TokenStream, TokenTree};

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

#[path = "tokens.rs"]
mod tokens;
use tokens::end_location;
pub use tokens::{
    compact_tokens, location, path_to_string, split_face_fields, split_top_level, syntax_error,
};

#[path = "fields.rs"]
mod fields;
use super::literal_value::normalized_literal_value;
use fields::parse_face_fields;

/// A position inside a parsed source file, one-based in both axes.
/// 已解析源文件里的位置，两个轴都从 1 开始。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxLocation {
    /// One-based line number.
    /// 从 1 开始的行号。
    pub line: usize,
    /// One-based column number, counted in characters.
    /// 从 1 开始的列号，按字符计。
    pub column: usize,
}

#[derive(Clone, Debug)]
struct FieldSyntax {
    tokens: TokenStream,
    location: SyntaxLocation,
}

/// Parsed registration declaration, independent of the macro that consumes it.
/// 已解析的注册声明，与消费它的宏实现无关。
#[derive(Clone, Debug)]
pub struct FaceSyntax {
    /// The registration macro's name exactly as written (`control_object`,
    /// `root_object`, a generated `*_object!` alias, …).
    /// 注册宏的名字，按源码原文（`control_object`、`root_object`、生成的
    /// `*_object!` 别名等）。
    pub macro_name: String,
    /// The `cfg` gates the declaration carries, if any, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
    pub cfg: Option<String>,
    /// Where the declaration starts — the macro invocation, or the `const` item in the literal form.
    /// 声明的起点——宏调用，或字面量形里的 `const` 条目。
    pub location: SyntaxLocation,
    /// Exclusive end of the macro invocation in the source file.
    /// 宏调用在源码中的排他结束位置。
    pub end: SyntaxLocation,
    macro_span: proc_macro2::Span,
    fields: BTreeMap<String, FieldSyntax>,
    imports: super::imports::Imports,
    expression: Option<(SyntaxLocation, SyntaxLocation)>,
    /// Where the declaration itself starts — the macro invocation in the macro spelling, the `impl` item in
    /// the literal one. Callers that report or rewrite "the declaration" use this rather than `location`,
    /// which the literal spelling never fills (measured: every literal face's anchor landed on line 1).
    /// 声明本身从哪里开始——宏拼法里是那次宏调用，字面量拼法里是那条 `impl` 条目。报告或重写"声明"的调用方
    /// 用它而不是 `location`，后者在字面量拼法下从不被填（实测：每个字面量面的锚点都落到第 1 行）。
    pub declaration_start: SyntaxLocation,
    /// Whether this declaration was written as the **struct-literal form** (`impl X { const
    /// REGISTRATION: Registration = Registration { … } }`) rather than as a macro block.
    /// 这份声明是否写成**结构体字面量形**，而不是宏块。
    ///
    /// The two forms are the same face (same identity, same fields) and every reader takes both — but a
    /// **writer** is not form-agnostic: the macro rewriter splices a macro block, and splicing that into
    /// an `impl` produces code that does not compile. Until the writer learns the new form (H/A2e step
    /// ③), the rewriter refuses to cross forms instead of guessing.
    /// 两形是同一个面（同一身份、同一批字段），每个读者都接受两者——但**写者**不是形无关的：宏重写器拼的是
    /// 宏块，把它拼进 `impl` 会产出编译不过的代码。在写者学会新形之前（H/A2e 第③步），重写器**拒绝跨形**，
    /// 而不是猜。
    struct_literal: bool,
}

impl FaceSyntax {
    /// Whether this declaration is written as the **struct-literal form** (H/A2e) rather than a macro
    /// block. Both are the same face, but a writer must not cross them: splicing a macro block into an
    /// `impl` does not compile, and until the rewriter learns the new form it says so instead of trying.
    /// 这份声明写成**结构体字面量形**（H/A2e）还是宏块。两者是同一个面，但**写者**不该跨形：把宏块拼进
    /// `impl` 编译不过，因此在重写器学会新形之前，它明说这一点而不是去试。
    pub fn is_struct_literal(&self) -> bool {
        self.struct_literal
    }

    /// Take the type-like half a `impl FaceContract` block carries, and refuse a face whose labels
    /// disagree with the traits its `assert_contracts!` line names.
    /// 接下一个 `impl FaceContract` 块携带的类型类那一半，并拒绝"标签与 `assert_contracts!` 点名的 trait
    /// 不一致"的面。
    ///
    /// Only the new spelling needs this: a macro spelling writes those four among its own fields, so
    /// nothing is overwritten there.
    /// 只有新拼法需要这一步：宏那两种拼法把这四项写在自己的字段里，因此那里什么都不覆盖。
    ///
    /// The labels and the assertion are **two statements of one fact**. When they disagree, a rule that
    /// requires a label would find a face that never claimed that trait — so the disagreement is refused
    /// here, which is what turns the convention into something the build checks.
    /// 标签与那条断言是**同一件事实的两种陈述**。两者不一致时，某条要求某个标签的规则会找到一个从未声称过那个
    /// trait 的面——因此在这里拒绝这种不一致，这正是把约定变成构建会检查的东西。
    fn absorb_contract_items(
        &mut self,
        items: &BTreeMap<String, String>,
        asserted: Option<&[String]>,
    ) -> Result<(), FaceSyntaxError> {
        if !self.struct_literal || items.is_empty() {
            return Ok(());
        }
        if let Some(paths) = asserted {
            let asserted: Vec<&str> = paths
                .iter()
                .map(|path| path.rsplit("::").next().unwrap_or(path.as_str()).trim())
                .filter(|label| !label.is_empty())
                .collect();
            for field in ["handle_traits", "part_traits"] {
                let Some(written) = items.get(field) else {
                    continue;
                };
                let labels = quoted_strings(written);
                // An **empty** list claims nothing, so there is nothing for the assertion to match — the
                // rule is about a label that **was** claimed and then left unasserted. Refusing the empty
                // list made every face with one populated contract and one empty one illegal (measured:
                // `handle_traits = ["ControlHandle"]` beside `part_traits = []` refused by name).
                // **空**列表什么都没声称，因此没有东西要断言去匹配——这条规则管的是"**确实声称过**某个标签却没
                // 断言它"。拒绝空列表会让每个"一份合同有值、另一份为空"的面变成非法（实测：`handle_traits =
                // ["ControlHandle"]` 与 `part_traits = []` 并列时被按名拒绝）。
                if labels.is_empty() {
                    continue;
                }
                if !asserted
                    .iter()
                    .any(|label| labels.iter().any(|written| written == label))
                {
                    return Err(super::syntax_error(
                        self.macro_span,
                        format!(
                            "`{field}` and the `assert_contracts!` line name different traits: the                              labels are two statements of one fact, and a rule that requires one of                              them must not find a face that never claimed it"
                        ),
                    ));
                }
            }
        }
        let mut text = String::new();
        for (field, value) in items {
            if self.fields.contains_key(field) {
                continue;
            }
            text.push_str(&format!("{field}: {value}, "));
        }
        if text.is_empty() {
            return Ok(());
        }
        let tokens = text
            .parse::<TokenStream>()
            .map_err(|error| FaceSyntaxError {
                message: format!("cannot read the `FaceContract` block: {error}"),
                location: Some(self.location.clone()),
            })?;
        self.fields
            .extend(parse_face_fields(tokens, self.macro_span)?);
        Ok(())
    }
}

/// The face-field name a type-like associated item carries, if it is one of the four.
/// 某个类型类关联项携带的面字段名，若它是那四个之一。
fn contract_field(name: &str) -> Option<&'static str> {
    match name {
        "Preset" => Some("preset"),
        "Parts" => Some("parts"),
        "HANDLE_TRAITS" => Some("handle_traits"),
        "PART_TRAITS" => Some("part_traits"),
        _ => None,
    }
}

/// Every string literal in a written expression, in order.
/// 一段写下的表达式里的所有字符串字面量，按顺序。
fn quoted_strings(text: &str) -> Vec<String> {
    let Ok(tokens) = text.parse::<TokenStream>() else {
        return Vec::new();
    };
    fn string_literals_in(stream: TokenStream, found: &mut Vec<String>) {
        for tree in stream {
            match tree {
                proc_macro2::TokenTree::Literal(literal) => {
                    if let Ok(value) = syn::parse_str::<syn::LitStr>(&literal.to_string()) {
                        found.push(value.value());
                    }
                }
                proc_macro2::TokenTree::Group(group) => string_literals_in(group.stream(), found),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    string_literals_in(tokens, &mut found);
    found
}

/// How a declaration names its parent, kept as written rather than resolved.
/// 声明如何命名其父级，按原文保留而不在此解析。
///
/// The three shapes exist because the three registrations that accept a parent
/// spell it differently; resolving them is the build-time identity code's job.
/// 之所以有三种形状，是因为接受父级的三类注册写法不同；解析它们是构建期身份代码的事。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParentSyntax {
    /// The package root, written as `crate::root_node_id(crate::XIRANG_NAMESPACE)`.
    /// 包根，写成 `crate::root_node_id(crate::XIRANG_NAMESPACE)`。
    Root,
    /// The parent is named by a path and a kind, as `NodeId::from_path` takes them.
    /// 父级由路径与 kind 命名，即 `NodeId::from_path` 接收的两个参数。
    FromPath {
        /// The `source` literal.
        /// `source` 字面量。
        source: String,
        /// The `kind` literal.
        /// `kind` 字面量。
        kind: String,
    },
    /// The parent is named by an expression path (typically `crate::x::NODE_ID`).
    /// 父级由一个表达式路径命名（通常是 `crate::x::NODE_ID`）。
    NodePath(String),
}

/// Why a declaration could not be parsed, and where when that is known.
/// 声明为何解析不了；位置已知时一并给出。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceSyntaxError {
    /// Human-readable reason.
    /// 人类可读的原因。
    pub message: String,
    /// Where parsing failed; `None` when the failure is about the file as a whole.
    /// 解析失败的位置；失败针对整个文件时为 `None`。
    pub location: Option<SyntaxLocation>,
}

/// Paths one file's executable expressions mention, and how much the scan could
/// actually see.
/// 一个文件的可执行表达式提到的路径，以及这次扫描实际能看到多少。
///
/// A caller that prunes faces from this list must respect `conservative`: the
/// scan is lexical, so anything it cannot follow has to widen the result instead
/// of silently shrinking it.
/// 依据这份清单裁剪注册面的调用方必须尊重 `conservative`：扫描是词法层面的，凡它跟不下去
/// 的东西都必须让结果变宽，而不是悄悄变窄。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceReferences {
    /// Every path reached through an expression, with imports excluded.
    /// 经表达式触达的每个路径，不含导入。
    pub paths: BTreeSet<String>,
    /// Set when the scan met something it cannot follow (macro expansion, a trait
    /// object, `include!`), so the caller must not prune on this list alone.
    /// 扫描遇到跟不下去的东西（宏展开、trait object、`include!`）时置位，调用方不得仅凭
    /// 这份清单裁剪。
    pub conservative: bool,
}

impl fmt::Display for FaceSyntaxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            write!(
                formatter,
                "{}:{}: {}",
                location.line, location.column, self.message
            )
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for FaceSyntaxError {}

/// Parse every XiRang face declaration in one Rust source file.
/// 解析一个 Rust 源文件中的全部 XiRang 注册面声明。
pub fn parse_faces(source: &str) -> Result<Vec<FaceSyntax>, FaceSyntaxError> {
    let file = super::nesting::parse_file(source)?;
    let mut visitor = FaceVisitor {
        faces: Vec::new(),
        error: None,
        imports: Default::default(),
        contract_items: Default::default(),
        asserted_contracts: None,
    };
    visitor.visit_file(&file);
    if let Some(error) = visitor.error {
        Err(error)
    } else {
        for face in &mut visitor.faces {
            face.absorb_contract_items(
                &visitor.contract_items,
                visitor.asserted_contracts.as_deref(),
            )?;
        }
        Ok(visitor.faces)
    }
}

/// Relocate a declaration by editing only its parent expression and optional macro identifier, keeping
/// comments, strings and the rest of the author's source untouched.
/// 仅编辑声明的父表达式与可选的宏标识符，保留注释、字符串和其余业务源码。
pub fn relocate_face_declaration(
    source: &str,
    parent: &str,
    alias: Option<(&str, &str)>,
) -> Result<String, FaceSyntaxError> {
    let face = parse_face(source)?.ok_or_else(|| FaceSyntaxError {
        message: "source has no registration face".to_owned(),
        location: None,
    })?;
    let field = face.fields.get("parent").ok_or_else(|| FaceSyntaxError {
        message: "registration face has no explicit parent".to_owned(),
        location: Some(face.location.clone()),
    })?;
    syn::parse_str::<syn::Expr>(parent)
        .map_err(|error| syntax_error(field.tokens.span(), error.to_string()))?;
    // Spans first, scan as the **fallback**: `field.tokens.span()` is a joined span, which is exact for a
    // single-token value but reports a synthesized position for a multi-token one (measured: `1:50` on a
    // 24-character line, which made every `move` fail). When the span cannot be located, the field's own
    // recorded start plus a bracket-aware scan of the source is what is left — and that path has to stop at
    // the expression, leaving the author's comments around it alone.
    // 先用 span、**回退**才扫描：`field.tokens.span()` 是 joined span，对单 token 的值精确，对多 token 的值会
    // 报合成位置（实测：24 个字符的行上给出 `1:50`，让每一次 `move` 都失败）。span 定位不到时，退而用字段自己
    // 记下的起点 ＋ 对源码做括号感知扫描——那条路必须停在表达式上，把作者写在两侧的注释留下。
    let span_start = location(field.tokens.span());
    let span_end = end_location(field.tokens.span());
    let range = match (
        source_offset(source, &span_start),
        source_offset(source, &span_end),
    ) {
        (Ok(start), Ok(end)) => (start, end),
        _ => {
            let start =
                super::face_region::value_start(source, source_offset(source, &field.location)?);
            (start, super::face_region::expression_end(source, start))
        }
    };
    let mut edits = vec![(range.0, range.1, parent.to_owned())];
    if let Some((before, after)) = alias.filter(|_| !face.is_native()) {
        if face.macro_name != before {
            return Err(syntax_error(
                face.macro_span,
                format!(
                    "expected declaration alias `{before}`, found `{}`",
                    face.macro_name
                ),
            ));
        }
        syn::parse_str::<syn::Ident>(after)
            .map_err(|error| syntax_error(face.macro_span, error.to_string()))?;
        let alias_start = source_offset(source, &location(face.macro_span))?;
        edits.push((
            alias_start,
            alias_start + face.macro_name.len(),
            after.to_owned(),
        ));
    }
    let mut ranges = edits;
    ranges.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut output = source.to_owned();
    for (start, end, text) in ranges {
        output.replace_range(start..end, &text);
    }
    Ok(output)
}

/// Whether `source` declares a registration face.
/// 源码是否声明了注册面。
///
/// A generated-marker line short-circuits the answer: a snapshot this tooling wrote is
/// a face by construction (re-parsing it is wasted work). Without `_object`, none of the
/// recognized macro identifiers can occur; remaining candidates are decided by the parser.
/// 生成标记行会短路答案：本工具写出的快照按构造就是注册面，再解析一遍是白费。其余
/// 文件若不含 `_object`，便不可能含已识别的宏标识符；其余候选一律由解析裁决。
///
/// **The shortcut is about trust, not about proof**: a marker line alone still answers
/// `true` (audit `KRN-K-19`), so "this file really declares a face" must be settled by
/// parsing (`parse_face`), never by reading this answer.
/// 这条短路关乎**信任**而非**证明**：只有标记行也会答 `true`（审计 `KRN-K-19`），因此
/// "这个文件真的声明了注册面"必须由解析（`parse_face`）裁定，而不能把这个答案当成解析结果。
pub fn is_face_source(source: &str, marker: &str) -> bool {
    source.lines().any(|line| line == marker)
        || ((source.contains("_object") || source.contains("soil") || source.contains("face"))
            && matches!(parse_face(source), Ok(Some(_))))
}

/// Parse exactly one face, returning `None` when the file has no declaration.
/// 解析唯一注册面；文件没有声明时返回 `None`。
pub fn parse_face(source: &str) -> Result<Option<FaceSyntax>, FaceSyntaxError> {
    let mut faces = parse_faces(source)?;
    if faces.len() > 1 {
        return Err(FaceSyntaxError {
            message: "expected one registration face in this file".to_owned(),
            location: faces.get(1).map(|face| face.location.clone()),
        });
    }
    Ok(faces.pop())
}

/// Replace only the registration macro, preserving the surrounding Rust code.
/// 只替换注册宏，保留同一文件里的其余 Rust 实现。
pub fn replace_face_macro(source: &str, replacement: &str) -> Result<String, FaceSyntaxError> {
    let current = parse_face(source)?.ok_or_else(|| FaceSyntaxError {
        message: "source has no registration face".to_owned(),
        location: None,
    })?;
    let next = parse_face(replacement)?.ok_or_else(|| FaceSyntaxError {
        message: "replacement has no registration face".to_owned(),
        location: None,
    })?;
    // The two forms are the same face but not interchangeable **as text**, and this function writes
    // text. Crossing them produced code that does not compile (a macro block spliced into an `impl`,
    // measured), so it is refused by name until the writer learns the new form (H/A2e step ③).
    // 两形是同一个面，但**作为文本**不可互换，而本函数写的是文本。跨形会产出编译不过的代码（实测：宏块被拼进
    // `impl`），因此在写者学会新形之前（H/A2e 第③步），跨形**按名拒绝**。
    if current.is_struct_literal() != next.is_struct_literal() {
        return Err(FaceSyntaxError {
            message: format!(
                "cannot rewrite a {} declaration into the {} form yet: the two spellings are the same \
                 face, but the writer for that form has not landed (H/A2e step ③)",
                if current.is_struct_literal() {
                    "struct-literal"
                } else {
                    "macro"
                },
                if next.is_struct_literal() {
                    "struct-literal"
                } else {
                    "macro"
                },
            ),
            location: Some(current.location.clone()),
        });
    }
    // Keep native item identity, visibility, docs, collector and cfg attributes intact.
    // 保留原生条目的名称、可见性、文档、收集器与 cfg 属性。
    if let (Some((start, end)), Some((next_start, next_end))) =
        (&current.expression, &next.expression)
    {
        let mut output = source.to_owned();
        output.replace_range(
            source_offset(source, start)?..source_offset(source, end)?,
            &replacement
                [source_offset(replacement, next_start)?..source_offset(replacement, next_end)?],
        );
        return Ok(output);
    }
    let current_start = source_offset(source, &current.declaration_start)?;
    let current_end = source_offset(source, &current.end)?;
    let replacement_start = source_offset(replacement, &next.location)?;
    let replacement_end = source_offset(replacement, &next.end)?;
    let mut output =
        String::with_capacity(source.len() + replacement_end.saturating_sub(replacement_start));
    output.push_str(&source[..current_start]);
    output.push_str(&replacement[replacement_start..replacement_end]);
    output.push_str(&source[current_end..]);
    Ok(output)
}

pub(crate) fn source_offset(
    source: &str,
    location: &SyntaxLocation,
) -> Result<usize, FaceSyntaxError> {
    let line_start = if location.line <= 1 {
        0
    } else {
        source
            .match_indices('\n')
            .nth(location.line - 2)
            .map(|(index, _)| index + 1)
            .ok_or_else(|| FaceSyntaxError {
                message: "macro span points outside source".to_owned(),
                location: Some(location.clone()),
            })?
    };
    // Parser columns count characters; byte offsets diverge after non-ASCII literals/comments.
    // 解析器的列按字符计数；非 ASCII 字面量或注释之后，列号不等于字节偏移。
    let line = source[line_start..].split('\n').next().unwrap_or_default();
    line.char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(line.len()))
        .nth(location.column.saturating_sub(1))
        .map(|offset| line_start + offset)
        .ok_or_else(|| FaceSyntaxError {
            message: "macro span column points outside source line".to_owned(),
            location: Some(location.clone()),
        })
}

/// Collect paths used by executable expressions, excluding imports, comments,
/// strings, and registration-macro metadata.
/// 收集可执行表达式使用的路径，排除导入、注释、字符串和注册宏元数据。
pub fn source_references(source: &str) -> Result<SourceReferences, FaceSyntaxError> {
    let file = super::nesting::parse_file(source)?;
    Ok(super::reference_scan::scan(&file))
}

struct FaceVisitor {
    faces: Vec<FaceSyntax>,
    error: Option<FaceSyntaxError>,
    imports: super::imports::Imports,
    /// The type-like half a `impl FaceContract for X` block carries, as written text.
    /// `impl FaceContract for X` 块携带的类型类那一半，按原文文本。
    ///
    /// In the new spelling (H/A2e) these four live **outside** the declaration: the face's type-like
    /// half is an `impl FaceContract` beside it. Downstream every reader expects them among the face's
    /// fields, so they are collected here and absorbed once the file has been visited — order in the
    /// file must not matter.
    /// 在新拼法（H/A2e）里这四项住在声明**之外**：面的类型类那一半是它旁边的 `impl FaceContract`。下游每个
    /// 读者都期望它们在面的字段里，因此在这里收集、等整份文件访问完再注入——文件里的先后不该有影响。
    contract_items: BTreeMap<String, String>,
    /// The trait paths an `assert_contracts!(Face: A, B);` line names, if the file has one.
    /// 文件里若有 `assert_contracts!(Face: A, B);`，这里是它点名的 trait 路径。
    asserted_contracts: Option<Vec<String>>,
}

impl<'ast> Visit<'ast> for FaceVisitor {
    fn visit_file(&mut self, file: &'ast syn::File) {
        super::imports::collect(&file.items, &mut self.imports);
        syn::visit::visit_file(self, file);
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let saved = self.imports.clone();
        if let Some((_, items)) = &item.content {
            super::imports::collect(items, &mut self.imports);
        }
        syn::visit::visit_item_mod(self, item);
        self.imports = saved;
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if self.error.is_some() || !super::has_face_attribute(&item.attrs) {
            return;
        }
        match super::native_face_fields(item)
            .and_then(|tokens| parse_face_fields(tokens, item.span()))
        {
            Ok(fields) => self.faces.push(FaceSyntax {
                expression: Some((location(item.expr.span()), end_location(item.expr.span()))),
                declaration_start: location(item.span()),
                macro_name: super::native_face::face_attribute_name(&item.attrs)
                    .unwrap()
                    .to_string(),
                macro_span: item.ident.span(),
                cfg: item
                    .attrs
                    .iter()
                    .find(|a| a.path().is_ident("cfg"))
                    .and_then(|a| a.parse_args::<TokenStream>().ok())
                    .map(|t| compact(&t)),
                location: location(item.span()),
                end: end_location(item.span()),
                fields,
                imports: self.imports.clone(),
                struct_literal: false,
            }),
            Err(error) => self.error = Some(error),
        }
    }

    /// The **struct-literal form** (`docs/open-work-2026-10-08.md` H, ruling A2e):
    ///
    /// ```ignore
    /// impl Card {
    ///     pub const REGISTRATION: Registration = Registration { parent: crate::board::NODE_ID };
    /// }
    /// ```
    ///
    /// Two things make this the same face as the macro block it replaces, and both are load-bearing:
    /// the **type name is the `kind`** (so it still enters the identity hash — renaming the type is a
    /// different face, exactly as renaming `kind:` was), and the fields are the same ones in the same
    /// meaning. This arm exists so the reader accepts **both** forms while the rest of the landing order
    /// (§H: snippets, executor, migration, mirror retirement) moves at its own pace.
    /// 两件事让这份声明与它取代的宏块是**同一个面**，而两件都是承重的：**类型名就是 `kind`**（因此它照旧进
    /// 身份哈希——改类型名就是换一张面，与改 `kind:` 等价），字段也还是同名字段、同一个含义。这条臂的存在，
    /// 是为了让读者**同时接受两形**，而落地顺序（§H：片段、执行器、迁移、退役镜像）各走各的步子。
    ///
    /// A constant that is not named `REGISTRATION`, or whose type or literal is not `Registration`, is
    /// not read as a face: the form is narrow on purpose, because the reader cannot ask the author what
    /// they meant.
    /// 不叫 `REGISTRATION`、或其类型或字面量不是 `Registration` 的常量不会被读成面：这个形是**有意写窄**的，
    /// 因为读者没法问作者他是什么意思。
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if self.error.is_some() {
            return;
        }
        // The type-like half, wherever it is written: the four names below are the ones every
        // downstream reader asks for by that name.
        // 类型类那一半，无论写在哪：下面四个名字就是下游每个读者按名索取的那四个。
        for entry in &item.items {
            match entry {
                syn::ImplItem::Const(constant) => {
                    let name = constant.ident.to_string();
                    if let Some(field) = contract_field(&name) {
                        self.contract_items.insert(
                            field.to_owned(),
                            constant.expr.to_token_stream().to_string(),
                        );
                    }
                }
                syn::ImplItem::Type(associated) => {
                    let name = associated.ident.to_string();
                    if let Some(field) = contract_field(&name) {
                        self.contract_items.insert(
                            field.to_owned(),
                            associated.ty.to_token_stream().to_string(),
                        );
                    }
                }
                _ => {}
            }
        }
        let Some(kind) = last_segment(&item.self_ty) else {
            return;
        };
        for entry in &item.items {
            let syn::ImplItem::Const(constant) = entry else {
                continue;
            };
            if constant.ident != "REGISTRATION"
                || last_segment(&constant.ty).as_deref() != Some("Registration")
            {
                continue;
            }
            let syn::Expr::Struct(literal) = &constant.expr else {
                continue;
            };
            if last_segment(&syn::Type::Path(syn::TypePath {
                qself: None,
                path: literal.path.clone(),
            }))
            .as_deref()
                != Some("Registration")
            {
                continue;
            }
            // `kind` is not written in this form — it **is** the type name — so it is synthesized here,
            // which is what keeps the two forms indistinguishable to every reader downstream.
            // 这个形里不写 `kind`——它**就是**类型名——因此在这里合成它，这正是让下游每一个读者看不出两形
            // 区别的原因。
            let mut text = format!("kind: {kind}");
            for field in &literal.fields {
                let syn::Member::Named(name) = &field.member else {
                    continue;
                };
                // The chain spelling normalizes every field into the shape the readers consume
                // (`native_face.rs`): `Some(x)` becomes `x`, `&[..]` becomes `[..]`, `LocalizedText { .. }`
                // becomes `{ .. }`, and a `RequirementSpec` pair becomes `capability => provider`. This form
                // writes the field's **value**, whose type is `Registration`'s own, so the same normalization
                // has to happen here — otherwise the two spellings hand the readers different values
                // (measured: `localized` and `string` answered `None` for a migrated face, and `apply edit`
                // refused one with `unknown runtime check `& [NON_EMPTY_TEXT``).
                // 链式拼法把每个字段整成读者消费的形状（`native_face.rs`）：`Some(x)` → `x`、`&[..]` → `[..]`、
                // `LocalizedText { .. }` → `{ .. }`、`RequirementSpec` 对 → `capability => provider`。本形写的是
                // 字段的**值**（类型就是 `Registration` 自己的），因此这里必须做同一层归一——否则两形交给读者
                // 的值不同（实测：已迁的面上 `localized` 与 `string` 答 `None`，且 `apply edit` 拒绝它并报
                // `unknown runtime check `& [NON_EMPTY_TEXT``）。
                let printed = normalized_literal_value(&field.expr);
                text.push_str(&format!(", {name}: {printed}"));
            }
            let parsed = text
                .parse::<TokenStream>()
                .map_err(|error| FaceSyntaxError {
                    message: format!("cannot read the registration literal: {error}"),
                    location: Some(location(constant.span())),
                })
                .and_then(|tokens| parse_face_fields(tokens, constant.span()));
            match parsed {
                Ok(fields) => {
                    let span = constant.span();
                    let cfg =
                        constant
                            .attrs
                            .iter()
                            .chain(item.attrs.iter())
                            .find_map(|attribute| {
                                attribute
                                    .path()
                                    .is_ident("cfg")
                                    .then(|| {
                                        attribute
                                            .parse_args::<TokenStream>()
                                            .ok()
                                            .map(|tokens| compact(&tokens))
                                    })
                                    .flatten()
                            });
                    self.faces.push(FaceSyntax {
                        expression: Some((
                            location(constant.expr.span()),
                            end_location(constant.expr.span()),
                        )),
                        declaration_start: location(span),
                        // No macro is involved. The name is stable and deliberately does not end in
                        // `_object`, so the checks that look for a parent macro stay silent — correct,
                        // because this form does not need one.
                        // 这里没有宏。这个名字是稳定的，且**有意**不以 `_object` 结尾，好让那些找父级宏的检查
                        // 保持沉默——这是对的，因为这个形不需要父级宏。
                        macro_name: "registration".to_owned(),
                        macro_span: constant.ident.span(),
                        cfg,
                        location: location(span),
                        end: end_location(span),
                        fields,
                        imports: self.imports.clone(),
                        struct_literal: true,
                    })
                }
                Err(error) => self.error = Some(error),
            }
        }
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        // `assert_contracts!(Face: A, B);` — the check a hand-written face (H/A2e) writes beside itself.
        // Its trait list is the other statement of what the face's labels say, and the two are compared
        // once the whole file has been read.
        // `assert_contracts!(Face: A, B);`——手写的面（H/A2e）写在自己旁边的那条检查。它的 trait 列表是面
        // 标签那句话的另一种陈述，两者在整份文件读完后再比对。
        // The author writes the macro by its full path (`xirang_toolchain::assert_contracts!`), so the
        // name to compare is the path's last segment.
        // 作者用全路径写这个宏（`xirang_toolchain::assert_contracts!`），因此要比的是路径的最后一段。
        if item
            .mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "assert_contracts")
        {
            let tokens = item.mac.tokens.to_string();
            if let Some((_, traits)) = tokens.split_once(':') {
                self.asserted_contracts = Some(
                    traits
                        .split(',')
                        .map(|path| path.trim().to_owned())
                        .filter(|path| !path.is_empty())
                        .collect(),
                );
            }
        }
        if self.error.is_some() {
            return;
        }
        let Some(segment) = item.mac.path.segments.last() else {
            return;
        };
        let macro_name = segment.ident.to_string();
        let is_face_macro = matches!(macro_name.as_str(), "control_object" | "external_object")
            || macro_name.ends_with("_object");
        if !is_face_macro {
            return;
        }
        match parse_face_fields(item.mac.tokens.clone(), item.mac.span()) {
            Ok(fields) => {
                let span = item.span();
                let cfg = item.attrs.iter().find_map(|attribute| {
                    attribute
                        .path()
                        .is_ident("cfg")
                        .then(|| {
                            attribute
                                .parse_args::<TokenStream>()
                                .ok()
                                .map(|tokens| compact(&tokens))
                        })
                        .flatten()
                });
                self.faces.push(FaceSyntax {
                    expression: None,
                    declaration_start: location(item.mac.path.span()),
                    macro_name,
                    macro_span: segment.ident.span(),
                    cfg,
                    location: location(span),
                    end: end_location(span),
                    fields,
                    imports: self.imports.clone(),
                    struct_literal: false,
                })
            }
            Err(error) => self.error = Some(error),
        }
    }
}

/// The last segment of a type or path, which is the name this parser compares against.
/// 类型或路径的最后一段——本解析器用来比较的那个名字。
fn last_segment(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    Some(path.path.segments.last()?.ident.to_string())
}

/// Render a token stream as compact Rust source text.
/// 把 token 流渲染成紧凑的 Rust 源码文本。
///
/// The by-reference twin of [`compact_tokens`], kept for the field reader.
/// [`compact_tokens`] 的按引用版本，供字段读取器使用。
pub(super) fn compact(tokens: &TokenStream) -> String {
    compact_tokens(tokens.clone())
}

/// Split a typed cut's tokens on a top-level `to`, returning both sides as
/// compact source text: `cut(a to b)` names a sibling range verbatim.
/// 在类型化切口的 token 里按顶层 `to` 切分，返回两侧的紧凑源码文本：
/// `cut(a to b)` 就是用原样 Rust 表达兄弟区间。
pub(super) fn split_typed_range(tokens: Vec<TokenTree>) -> Option<(String, String)> {
    let position = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Ident(value) if value == "to"))?;
    let start = tokens[..position].iter().cloned().collect::<TokenStream>();
    let finish = tokens[position + 1..]
        .iter()
        .cloned()
        .collect::<TokenStream>();
    if start.is_empty() || finish.is_empty() {
        return None;
    }
    Some((compact_tokens(start), compact_tokens(finish)))
}
#[cfg(test)]
#[path = "face_tests.rs"]
mod face_tests;

#[cfg(test)]
#[path = "face_relocation_tests.rs"]
mod relocation_tests;
