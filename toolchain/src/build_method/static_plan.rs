//! Static built-in topology validation.
//! 内置注册树静态拓扑校验。

use std::fs;
use std::path::Path;

use super::Node;
use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_identity;
use super::{SourceScope, cached_parent_id, face_source_is_active, node_id, parsed_face};
use xirang_kernel::{TopologyRecord, lexicon, validate_face_topology};

#[derive(Clone, Debug)]
pub(crate) struct StaticFaceRecord {
    pub(crate) id: registry_identity::NodeId,
    /// Whether the face was declared as a `Registration` literal; see [`PlannedFace`].
    /// 该面是否以 `Registration` 字面量声明；见 [`PlannedFace`]。
    pub(crate) written_as_literal: bool,
    /// Kind, and the two type names an assembly of a literal-form face needs. The macros derive these
    /// from their own arguments; a face that has none needs the build to carry them.
    /// kind，以及拼装一个字面量形的面所需的两个类型名。宏从自己的参数推导它们；而没有宏的面需要构建把它们带上。
    pub(crate) kind: String,
    pub(crate) preset: String,
    pub(crate) parts: String,
    pub(crate) parent: registry_identity::NodeId,
    pub(crate) owns_registry: bool,
    pub(crate) source: String,
    pub(crate) module: String,
}

/// The module path a registration source declares, relative to the crate root.
/// 注册面源码声明的模块路径，相对 crate 根。
///
/// Public so an authoring surface can compare a face with a typed graft cut
/// exactly the way the build does, instead of re-deriving the mapping.
/// 公开它，让创作界面能用与构建完全相同的方式把注册面和类型化 graft 切口对照，
/// 而不必重新推导这套映射。
pub fn source_module_path(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map_or_else(
            || relative.trim_end_matches(".rs").to_owned(),
            |(parent, _)| parent.to_owned(),
        )
        .replace('/', "::")
}

/// What the plan needs to know about one face, from whichever source answered for it.
/// 计划关于一个面需要知道的东西，来自替它作答的那个来源。
///
/// Two sources answer: the parsed face (a miss) and the faces payload a record hit carries
/// (`stem 2`, step 3). Everything downstream of this struct — the `cfg` gate, the error it may raise,
/// the record it may push — is one implementation, so the two paths cannot drift.
/// 有两个来源会作答：解析出的面（未命中）与记录命中携带的逐面载荷（`stem 2` 第③步）。这个结构体下游的一切
/// ——`cfg` 门控、它可能抛出的错误、它可能压入的条目——只有一份实现，因此两条路不会漂移。
pub(crate) struct PlannedFace {
    pub(crate) relative: String,
    /// The Rust module chain that mounts this face, from the discovery tree — the address the
    /// renderer and the generated identity constants use.
    /// 挂载本面的 Rust 模块链，取自发现树——渲染器与生成的标识常量使用的地址。
    ///
    /// It is **not** re-derived from the source path: an explicitly placed face outside the walk
    /// root has a module chain (`reading`) that has no relation to the directories it physically
    /// lives in (`components`), and deriving one from the path produced a module path that was not
    /// even a valid Rust identifier (a leading `::` followed by `tmp`).
    /// 它**不是**从源码路径重新推导的：遍历根之外显式落点的面有一条与它物理所在的目录
    /// （`components`）毫无关系的模块链（`reading`），而从路径推导会产出一个甚至不是合法 Rust
    /// 标识符的模块路径（`::` 开头、后面跟着 `tmp`）。
    pub(crate) module: String,
    /// Which spelling declared this face; the renderer needs it to address the identity without the
    /// module-level `NODE_ID` a macro would have emitted.
    /// 本面由哪种拼法声明；渲染器需要它，才能在没有宏会发射的模块级 `NODE_ID` 时找到身份。
    pub(crate) written_as_literal: bool,
    /// Kind and the two type names an assembly of a literal-form face needs; empty strings when the
    /// declaration names none.
    /// kind，以及拼装一个字面量形的面所需的两个类型名；声明里没写时是空串。
    pub(crate) kind: String,
    pub(crate) preset: String,
    pub(crate) parts: String,
    pub(crate) id: registry_identity::NodeId,
    pub(crate) parent: registry_identity::NodeId,
    pub(crate) owns_registry: bool,
    pub(crate) cfg: Option<String>,
    pub(crate) line: usize,
}

/// The one place a face becomes a plan entry or an error.
/// 一个面变成计划条目或错误的**唯一**地方。
pub(crate) fn plan_face(
    face: PlannedFace,
    enabled: &dyn Fn(&str) -> bool,
    records: &mut Vec<StaticFaceRecord>,
    errors: &mut BuildDiagnostics,
) {
    if let Some(cfg) = &face.cfg {
        match face_cfg_enabled(cfg, enabled) {
            Ok(true) => {}
            Ok(false) => return,
            Err(message) => {
                errors.push(
                    BuildDiagnostic::new("face-cfg", message).at(face.relative.clone(), face.line),
                );
                return;
            }
        }
    }
    records.push(StaticFaceRecord {
        id: face.id,
        written_as_literal: face.written_as_literal,
        kind: face.kind,
        preset: face.preset,
        parts: face.parts,
        parent: face.parent,
        owns_registry: face.owns_registry,
        module: face.module,
        source: face.relative,
    });
}
pub(crate) fn static_plan(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    declared_elsewhere: &[(registry_identity::NodeId, bool)],
) -> (Vec<StaticFaceRecord>, BuildDiagnostics) {
    let mut records = Vec::new();
    let mut errors = BuildDiagnostics::default();
    // One memo for this pass: the static plan resolves a parent per face, and the fixtures here are the
    // same shape as everywhere else — a thousand leaves under one folder face.
    // 本趟共用一份备忘：静态计划每个面解析一次父级，而这里的夹具与别处同形——一千个叶子挂在同一个文件夹面下。
    let _parents = crate::build_method::discovery_cache::ParentLookupScope::open();
    visit_static_faces(
        src,
        nodes,
        scope,
        false,
        "",
        &FaceSource::Parse,
        &mut records,
        &mut errors,
    );
    finish_plan(records, errors, declared_elsewhere)
}

/// The plan a **record hit** runs on: same walk, same gates, but every face is answered by the payload
/// instead of by a parse (`stem 2`, step 3). The walk stays here because the scope decision is a
/// property of the **package**, and the payload is a property of the **tree** — so the payload cannot
/// carry the answer, only the facts the answer is computed from.
/// **记录命中**所跑的计划：同一趟遍历、同一批门控，但每个面由载荷作答而不是解析（`stem 2` 第③步）。遍历
/// 留在这里，因为 scope 判定是**包**的属性，而载荷是**树**的属性——因此载荷不能携带答案，只能携带答案据以
/// 计算的事实。
pub(crate) fn static_plan_from_rows(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    rows: &[crate::build_method::discovery_record::FaceRow],
    declared_elsewhere: &[(registry_identity::NodeId, bool)],
) -> Result<(Vec<StaticFaceRecord>, BuildDiagnostics), String> {
    let mut index = std::collections::BTreeMap::new();
    for row in rows {
        index.insert(row.source.clone(), row);
    }
    let mut records = Vec::new();
    let mut errors = BuildDiagnostics::default();
    visit_static_faces(
        src,
        nodes,
        scope,
        false,
        "",
        &FaceSource::Rows(&index),
        &mut records,
        &mut errors,
    );
    Ok(finish_plan(records, errors, declared_elsewhere))
}

/// What both plan paths share after the walk: the topology check, then the pair.
/// 两条计划路径在遍历之后共用的部分：拓扑校验，然后是那一对。
fn finish_plan(
    records: Vec<StaticFaceRecord>,
    mut errors: BuildDiagnostics,
    declared_elsewhere: &[(registry_identity::NodeId, bool)],
) -> (Vec<StaticFaceRecord>, BuildDiagnostics) {
    let mut topology = records
        .iter()
        .map(|record| TopologyRecord {
            id: record.id,
            parent: record.parent,
            owns_registry: record.owns_registry,
            source: record.source.clone(),
        })
        .collect::<Vec<_>>();
    let mut topology_errors = validate_face_topology(
        &mut topology,
        registry_identity::package_root_node_id(),
        declared_elsewhere,
    );
    errors.extend(std::mem::take(&mut topology_errors));
    (records, errors)
}

/// Which source answers for a face during the walk.
/// 遍历期间由哪个来源替一个面作答。
enum FaceSource<'a> {
    /// A miss: read the file and parse the face.
    /// 未命中：读文件、解析面。
    Parse,
    /// A hit: the row the payload carries, keyed by source path.
    /// 命中：载荷携带的那一行，按源码路径索引。
    Rows(
        &'a std::collections::BTreeMap<String, &'a crate::build_method::discovery_record::FaceRow>,
    ),
}

// The eight inputs are the walk's facts (tree, scope, prefix) and its two outputs (records,
// errors) plus the source root and the answer mode; bundling them into a struct would only move the
// same list one level down, and the recursive call passes all of them through unchanged.
// 八个输入是这一趟遍历的事实（树、作用域、前缀）、它的两个输出（记录、错误），加上源码根与作答来源；
// 把它们捆成结构体只是把同一份清单挪到下一层，而递归调用原样传递全部。
#[allow(clippy::too_many_arguments)]
fn visit_static_faces(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    selected_ancestor: bool,
    module_prefix: &str,
    faces: &FaceSource<'_>,
    records: &mut Vec<StaticFaceRecord>,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        // The module chain is the tree's own, one segment per mounting node. `source_module_path`
        // was the path-shaped spelling of the same answer and stays correct only for faces whose
        // physical directories are their modules — which an explicit placement outside the walk
        // root is not.
        // 模块链来自树本身，每个挂载节点一段。`source_module_path` 是同一个答案的路径形状拼法，只对
        // "物理目录就是模块"的面成立——而遍历根之外显式落点的面不是。
        let module = if module_prefix.is_empty() {
            node.name.clone()
        } else {
            format!("{module_prefix}::{}", node.name)
        };
        if node.name == crate::build_method::DEMO_ONLY_DIRECTORY
            || !scope.includes(src, node, selected_ancestor)
        {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if face_source_is_active(src, node, scope, selected_ancestor) {
            let file = node
                .file
                .as_ref()
                .expect("an active registration face has a source file");
            let relative = super::discovery_node::source_spelling(src, file);
            if !lexicon::is_registration_path(&relative) {
                match faces {
                    FaceSource::Parse => {
                        if let Ok(source) = fs::read_to_string(file)
                            && let Some(face) = parsed_face(&source, &relative)
                            && face.field(lexicon::FACE_FIELD_PLUGIN).is_none()
                        {
                            let id = node_id(src, node).expect("a parsed face has an identity");
                            let parent = cached_parent_id(src, &face).or_else(|| {
                                face.field("parent")
                                    .is_none()
                                    .then_some(registry_identity::package_root_node_id())
                            });
                            match parent {
                                Some(parent) => plan_face(
                                    PlannedFace {
                                        relative,
                                        module: module.clone(),
                                        id,
                                        parent,
                                        written_as_literal: face.is_struct_literal(),
                                        // The kind is a bare type name, which the path reader accepts
                                        // and the string reader does not; ask in the same order the unit
                                        // writer does, so both sources answer alike.
                                        // kind 是裸类型名：路径读取器接受它、字符串读取器不接受；按与 unit
                                        // 写入方相同的顺序去问，两个来源才会给出同一个答案。
                                        kind: face
                                            .path("kind")
                                            .or_else(|| face.string("kind"))
                                            .unwrap_or_default(),
                                        // A type-like field holds a **path**, and a string reader does not
                                        // accept one: ask in the same order the unit writer does, so both
                                        // sources answer alike (the `kind` line above does the same).
                                        // 类型类字段装的是**路径**，字符串读取器不接受它：按与 unit 写入方相同的
                                        // 顺序去问，两个来源才会给出同一个答案（上面的 `kind` 就是这么做的）。
                                        preset: face
                                            .path("preset")
                                            .or_else(|| face.string("preset"))
                                            .unwrap_or_default(),
                                        parts: face
                                            .path("parts")
                                            .or_else(|| face.string("parts"))
                                            .unwrap_or_default(),
                                        owns_registry: face
                                            .boolean("needs_registry")
                                            .unwrap_or(false),
                                        cfg: face.cfg().map(str::to_owned),
                                        line: face.location.line,
                                    },
                                    &feature_enabled,
                                    records,
                                    errors,
                                ),
                                None => errors.push(
                                    BuildDiagnostic::new(
                                        "static-plan",
                                        "parent declaration cannot be resolved",
                                    )
                                    .at(relative, 0),
                                ),
                            }
                        }
                    }
                    FaceSource::Rows(index) => {
                        // A row the walk cannot read is reported rather than skipped: a payload this
                        // build cannot honor must not leave it with a silently smaller tree.
                        // 遍历读不出来的行会被上报而不是跳过：本构建兑现不了的载荷，不得让它带着一棵**悄悄变小**
                        // 的树离开。
                        if let Some(row) = index.get(&relative) {
                            match (row.id.parse(), row.parent.parse()) {
                                (Ok(id), Ok(parent)) => plan_face(
                                    PlannedFace {
                                        relative,
                                        module: module.clone(),
                                        id,
                                        parent,
                                        written_as_literal: row.written_as_literal,
                                        kind: row.kind.clone(),
                                        preset: row.preset.clone(),
                                        parts: row.parts.clone(),
                                        owns_registry: row.owns_registry,
                                        cfg: row.cfg.clone(),
                                        line: row.line,
                                    },
                                    &feature_enabled,
                                    records,
                                    errors,
                                ),
                                _ => errors.push(
                                    BuildDiagnostic::new(
                                        "faces-payload",
                                        "a row carries an identity this build cannot read",
                                    )
                                    .at(relative, row.line),
                                ),
                            }
                        }
                    }
                }
            }
        }
        visit_static_faces(
            src,
            &node.children,
            scope,
            selected_ancestor || selected_here,
            &module,
            faces,
            records,
            errors,
        );
    }
}

/// Whether a declaration's `cfg` gate is satisfied for this build.
/// 声明的 `cfg` 门控在本次构建中是否成立。
///
/// `enabled` answers for one feature name, so the decision is testable without
/// touching the process environment.
/// `enabled` 回答单个特性名，因此该判断无需触碰进程环境即可测试。
pub(crate) fn face_cfg_enabled(cfg: &str, enabled: &dyn Fn(&str) -> bool) -> Result<bool, String> {
    let meta = syn::parse_str::<syn::Meta>(cfg)
        .map_err(|error| unsupported_cfg(cfg, &error.to_string()))?;
    evaluate_cfg(&meta, cfg, enabled)
}

fn evaluate_cfg(
    meta: &syn::Meta,
    original: &str,
    enabled: &dyn Fn(&str) -> bool,
) -> Result<bool, String> {
    if let syn::Meta::NameValue(pair) = meta
        && pair.path.is_ident("feature")
    {
        return match &pair.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(name),
                ..
            }) => Ok(enabled(&name.value())),
            _ => Err(unsupported_cfg(
                original,
                "`feature` needs a string literal",
            )),
        };
    }
    if let syn::Meta::List(list) = meta
        && list.path.is_ident("not")
    {
        let inner = syn::parse2::<syn::Meta>(list.tokens.clone())
            .map_err(|error| unsupported_cfg(original, &error.to_string()))?;
        return Ok(!evaluate_cfg(&inner, original, enabled)?);
    }
    Err(unsupported_cfg(
        original,
        "only `feature = \"…\"` and `not(feature = \"…\")` can be followed",
    ))
}

fn unsupported_cfg(cfg: &str, reason: &str) -> String {
    format!(
        "unsupported `cfg` on a registration face: `{cfg}` ({reason}); the generated plan \
         cannot follow it, so the plan and the compiled crate would disagree"
    )
}

/// Whether a Cargo feature is enabled for this build script run.
/// 本次 build script 运行中某个 Cargo 特性是否启用。
pub(crate) fn feature_enabled(name: &str) -> bool {
    let key = format!(
        "CARGO_FEATURE_{}",
        name.to_uppercase().replace(['-', '.'], "_")
    );
    std::env::var_os(key).is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod face_cfg_tests {
    use super::face_cfg_enabled;

    /// Feature gates are followed; anything this build cannot evaluate is
    /// refused rather than guessed.
    /// 特性门控会被跟随；本次构建无法求值的写法一律拒绝，而不是猜测。
    #[test]
    fn feature_gates_are_followed_and_others_refused() {
        let enabled = |name: &str| name == "optional-face";
        assert_eq!(
            face_cfg_enabled(r#"feature = "optional-face""#, &enabled),
            Ok(true)
        );
        assert_eq!(
            face_cfg_enabled(r#"feature = "other""#, &enabled),
            Ok(false)
        );
        assert_eq!(
            face_cfg_enabled(r#"not(feature = "other")"#, &enabled),
            Ok(true)
        );
        assert_eq!(
            face_cfg_enabled(r#"not(feature = "optional-face")"#, &enabled),
            Ok(false)
        );
        let error = face_cfg_enabled("test", &enabled).expect_err("unsupported gate");
        assert!(error.contains("unsupported"), "{error}");
    }
}
