//! Build audit manifests.
//! 构建审计清单。
//!
//! The function column is a *declaration* list: it comes from the kernel's lexical
//! scanner (`xirang_kernel::source::function_symbols`), which masks comments, strings and
//! macro text before it looks for `fn`. A symbol is therefore `module::name` — the
//! scanner reports declarations without ownership, so a method carries no `impl`
//! type prefix. This file used to hold a second, line-based scanner that matched
//! `fn ` anywhere on a line and guessed the owner from the previous `impl ` line,
//! which nested `impl` blocks and closures polluted; that column asked the same
//! question the kernel already answers, and removing it is what closes the
//! duplication. No reader inside this tree consumes the file — `grep -rn
//! function_manifest` finds only the writers — so the change is visible to whoever
//! starts trusting it, which is exactly why its format is stated here.
//! 函数列的语义是**声明**清单：它来自内核词法扫描器
//! （`xirang_kernel::source::function_symbols`），该扫描器在寻找 `fn` 之前会屏蔽注释、字符串与宏文本。
//! 因此符号是 `module::name`——扫描器报告的是不带归属的声明，方法因此不带 `impl` 类型前缀。
//! 本文件过去带第二套按行的扫描器，它在行的任意位置匹配 `fn `，并用上一个 `impl ` 行猜归属；
//! 嵌套 impl 与闭包都会污染它。那一列问的正是内核已回答的同一个问题，删掉它正是收掉这份重复。
//! 本树内没有任何读者使用这份文件——`grep -rn function_manifest` 只找到写入侧——因此对将来开始
//! 信任它的人而言这次变化是可见的，这也正是这里写明格式的原因。
//!
//! The pruning column is documented where it is produced: see [`tracked_symbol`] and
//! the three item shapes in [`PruningItem`]. Its rows carry four more columns since
//! 2026-09-29 — `path`, `kind`, `registry_name`, `parent` — holding what the
//! **declaration** spelled (`-` where it named none), which is why a reader can now
//! match a face by logical path without reading the sources.
//! 修剪列的约定写在产出它的地方：见 [`tracked_symbol`] 与 [`PruningItem`] 里的三种条目形状。
//! 自 2026-09-29 起它的行多带四列——`path`、`kind`、`registry_name`、`parent`——内容是**声明**拼出的
//! 词（未声明处为 `-`），因此读者现在可以不读源码就按逻辑路径匹配一个面。
//! 修剪列的约定写在产出它的地方：见 [`tracked_symbol`] 与 [`PruningItem`] 里的三种条目形状。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::Node;
use super::face_view::PruningRow;
use super::registry_identity::NodeId;
use super::registry_syntax::{FaceSyntax, GraftSyntax};
use super::static_plan::source_module_path;
use super::{SourceScope, collect_faces, parsed_face, relative_display, write_if_changed};

/// Write the pruning manifest and hand back the rows it published (audit `M7`, P1.1).
/// 写剪枝清单，并把它发布出去的行交回来（审计 `M7`，P1.1）。
///
/// The rows are a **result**, not an internal detail: the build-time graph is a second view of
/// exactly these rows, and a caller that had to read the file back to get them would pay a second
/// parse of the same fifty thousand lines — the "computed it and handed over nothing" shape this
/// batch keeps removing, in its other direction.
/// 这些行是**结果**而不是内部细节：构建期的图正是这些行的第二个视图，而一个不得不把文件读回来才能拿到它的
/// 调用方，要付同一批五万行的第二次解析——也就是这一批一直在去掉的"算出来了却不交出来"，只是方向相反。
pub(crate) fn write_pruning_manifest(
    src: &Path,
    nodes: &[Node],
    out_dir: &Path,
) -> Result<Vec<PruningRow>, String> {
    // Pass one: every face's own identity, indexed by the module path a `parent:` declaration
    // spells. The resolution cannot happen while walking, because a child may be visited before the
    // face that owns the module it names.
    // 第一遍：每个面自己的身份，按 `parent:` 声明拼出的模块路径建索引。解析不能在遍历途中做，因为子面
    // 可能先于"拥有它点名的那个模块"的面被访问到。
    let mut modules = std::collections::BTreeMap::new();
    visit_face_modules(src, nodes, &mut modules);
    // A **third** walk, and it has to be one: the triples below carry resolved parent identities, and
    // a child may be visited before the face that owns the module it names — resolving during the
    // indexing walk would answer "unresolved" for exactly the children this column exists for. The
    // pin caught that: the first version of this code did resolve during indexing, and a child whose
    // `parent:` named a later-visited module got `root/child` instead of `root/dial/child`.
    // **第三次**遍历，而且必须是独立的一次：下面的三元组带着解析后的父级身份，而子面可能先于"拥有它点名的
    // 那个模块"的面被访问到——在索引那趟里解析，会对**正是这一列为之存在的那种子面**答"未解析"。钉子抓到了
    // 这一点：这段代码的第一版就是在索引途中解析的，于是 `parent:` 点名了稍后才会被访问的模块的子面拿到
    // `root/child` 而不是 `root/dial/child`。
    let mut names = Vec::new();
    visit_face_names(src, nodes, &modules, &mut names);
    // The logical path is the build's own walk over those triples (audit `W3-2`): the record used to
    // publish only the `path` a declaration spelled, which is `-` for every macro-derived face, so a
    // reader counting levels fell back to source directories and got one level per face.
    // 逻辑路径是构建自己对这些三元组的行走（审计 `W3-2`）：记录过去只发布声明拼出的 `path`，而它对每个
    // 宏派生的面都是 `-`，于是按层级计数的读者退到源码目录，落得"一面一层"。
    let paths =
        super::face_view::logical_paths(super::registry_identity::package_root_node_id(), &names);
    let mut rows = Vec::new();
    visit_pruning_symbols(src, nodes, &modules, &paths, &mut rows);
    write_face_rows(out_dir.join("pruning_manifest.tsv"), rows)
}

impl FaceColumns {
    /// The published row for one face, spelling "the declaration named none" the way the reader reads it.
    /// 一个面的已发布行，"声明里没有它"按读者读到的样子拼写。
    ///
    /// The rule is the reader's own (`read_pruning_manifest`): an empty field and `-` both mean the
    /// declaration named none. It is written here in the other direction so the writer and the reader
    /// cannot drift about what a published row says.
    /// 规则就是读者自己的（`read_pruning_manifest`）：空字段与 `-` 都意为声明里没有它。这里写的是反方向，
    /// 因此写入方与读取方不会对"一条已发布的行说了什么"漂开。
    fn to_row(&self, id: NodeId, source: String, symbol: String) -> PruningRow {
        let named = |value: &str| (!value.is_empty() && value != "-").then(|| value.to_owned());
        PruningRow {
            id,
            source,
            symbol,
            path: named(&self.path),
            kind: named(&self.kind),
            registry_name: named(&self.registry_name),
            parent: named(&self.parent),
            source_hash: named(&self.source_hash),
            fields: named(&self.fields),
            calls: named(&self.calls),
            parent_node: named(&self.parent_node),
            owns_registry: named(&self.owns_registry),
            logical_path: named(&self.logical_path),
        }
    }
}

/// The face facts the pruning manifest publishes beside each symbol.
/// 剪枝清单在每个符号旁发布的面事实。
///
/// These are the words the **declaration** used, not resolved identities: `parent` is the Rust
/// path the face spelled, so a reader that needs the parent's `NodeId` still resolves it. The
/// three that are already typed (`kind`, `registry_name`, `path`) are published because every
/// table-driven answer used to re-read the sources to get them.
/// 这些是**声明**用的那些词，不是解析后的身份：`parent` 是该面拼出的 Rust 路径，因此需要父级
/// `NodeId` 的读者仍然要解析它。已经带类型的那三个（`kind`、`registry_name`、`path`）之所以发布，
/// 是因为每一份查表面的答案过去都要为此重读源码。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FaceColumns {
    path: String,
    kind: String,
    registry_name: String,
    parent: String,
    /// The hash of the face's own source bytes (audit `W3-1`).
    /// 该面自己那份源码字节的散列（审计 `W3-1`）。
    source_hash: String,
    /// The fingerprint of the declaration's fields (audit `W3-1`).
    /// 声明字段的指纹（审计 `W3-1`）。
    fields: String,
    /// The names this file calls directly, comma-separated, `-` when it calls none.
    /// 这份文件直接调用的名字，逗号分隔，一个都没有时写 `-`。
    calls: String,
    /// The **resolved** parent identity, or `-` when this tree cannot resolve it (audit `W3-1b`).
    /// **解析后**的父级身份；这棵树解析不出时写 `-`（审计 `W3-1b`）。
    parent_node: String,
    /// Whether the declaration gives this face a registry of its own (audit `W3-1b`).
    /// 这条声明是否给了这个面自己的注册机（审计 `W3-1b`）。
    owns_registry: String,
    /// The **logical** path the build derives by walking the resolved parent chain (audit `W3-2`).
    /// **逻辑**路径，构建沿解析后的父链走出来的（审计 `W3-2`）。
    ///
    /// This is a different fact from `path` beside it, and the difference is the point: `path` is
    /// what the **declaration spelled** (`-` for every macro-derived face), while this is the answer
    /// the runtime would give. The census counts levels on this column.
    /// 它与旁边的 `path` 是两件事，而这个区别正是要点：`path` 是**声明拼出的**（对每个宏派生的面都是 `-`），
    /// 而这一列是运行期会给出的答案。普查按这一列计层。
    logical_path: String,
}

impl FaceColumns {
    /// The facts as this face spells them, with `-` where it names none.
    /// 本面拼出的各项事实，未声明的写作 `-`。
    fn declared(
        face: &FaceSyntax,
        kind: &str,
        source: &str,
        modules: &std::collections::BTreeMap<String, NodeId>,
        logical_path: Option<&String>,
    ) -> Self {
        let spelled = |value: Option<String>| value.unwrap_or_else(|| "-".to_owned());
        Self {
            path: spelled(face.string("path")),
            kind: kind.to_owned(),
            registry_name: spelled(face.string("registry_name")),
            parent: spelled(face.path("parent")),
            source_hash: xirang_kernel::sha256_hex(source.as_bytes()),
            fields: field_fingerprint(face, kind),
            calls: called_names(source),
            parent_node: resolved_parent(face, modules),
            owns_registry: match face.boolean("needs_registry") {
                Some(true) => "true".to_owned(),
                _ => "false".to_owned(),
            },
            logical_path: logical_path.cloned().unwrap_or_else(|| "-".to_owned()),
        }
    }
}

/// The parent identity this declaration resolves to, or `-` (audit `W3-1b`).
/// 这条声明解析到的父级身份，或 `-`（审计 `W3-1b`）。
///
/// The resolver is the **build's own** (`face_view::resolve_parent`), so the record publishes the
/// identity the derived tree computes rather than a second opinion about it. `-` is written when
/// the chain does not resolve (a module the tree does not own, or a `parent` that does not parse) —
/// and `-` is honest there: a reader that needs the parent then derives, exactly as it did before
/// the column existed.
/// 解析器用的是**构建自己的**那一个（`face_view::resolve_parent`），因此记录发布的就是推导树算出的身份，
/// 而不是关于它的第二种意见。链解析不出时写 `-`（树不拥有的模块、或解析不了的 `parent`）——而 `-` 在那
/// 里是诚实的：需要父级的读者于是推导，与这一列存在之前一样。
fn resolved_parent(
    face: &FaceSyntax,
    modules: &std::collections::BTreeMap<String, NodeId>,
) -> String {
    let (id, resolved) = resolved_parent_id(face, modules);
    if resolved {
        id.to_string()
    } else {
        "-".to_owned()
    }
}

/// The parent identity a declaration resolves to, and whether the chain resolved at all.
/// 一条声明解析到的父级身份，以及这条链究竟解析出来没有。
///
/// Unresolved parents answer the **package root** with `false`, which is what the derived view does
/// (`face_view::resolve_parent`), so the logical path this feeds is the same one a reader would
/// derive. The published column still writes `-`, because "the root, but the chain was broken" is not
/// the same claim as "the root".
/// 未解析的父级回**包根**加 `false`，与推导视图（`face_view::resolve_parent`）一致，因此它喂给逻辑
/// 路径的东西与读者推导出来的相同。而发布的那一列仍写 `-`，因为"根，但链断了"与"根"不是同一个主张。
fn resolved_parent_id(
    face: &FaceSyntax,
    modules: &std::collections::BTreeMap<String, NodeId>,
) -> (NodeId, bool) {
    match super::face_view::parent_of(face) {
        super::face_view::ParentSpec::Root => {
            (super::registry_identity::package_root_node_id(), true)
        }
        super::face_view::ParentSpec::FromPath { source, kind } => (
            super::registry_identity::package_node_id(&source, &kind),
            true,
        ),
        super::face_view::ParentSpec::NodePath(module) => {
            let key = module.strip_prefix("crate::").unwrap_or(&module);
            match modules.get(key) {
                Some(id) => (*id, true),
                None => (super::registry_identity::package_root_node_id(), false),
            }
        }
        super::face_view::ParentSpec::Unparsed => {
            (super::registry_identity::package_root_node_id(), false)
        }
    }
}

/// The declaration's own fingerprint, over the fields it spells (audit `W3-1`).
/// 声明自己的指纹，取自它写下的那些字段（审计 `W3-1`）。
///
/// The kernel's own field order is the enumerator, so a field the vocabulary does not carry
/// cannot enter the fingerprint, and a field the face does not spell cannot either. `kind` is
/// folded in because it is an identity input and is not always written down: two faces with the
/// same fields and different kinds are different faces, and a record that hashed only what was
/// spelled would call them equal.
/// 以内核自己的字段顺序为枚举器，因此词表不携带的字段进不了这个指纹，面没写的字段同样进不了。`kind` 折在
/// 里面，因为它是身份输入且并不总被写下来：字段相同、kind 不同的两个面是不同的面，而只散列"写下来的东西"的
/// 记录会把它们判成相等。
fn field_fingerprint(face: &FaceSyntax, kind: &str) -> String {
    let mut declared: Vec<String> = xirang_kernel::declaration::FACE_FIELD_ORDER
        .iter()
        .filter_map(|name| face.field(name).map(|raw| format!("{name}={raw}")))
        .collect();
    declared.push(format!("kind={kind}"));
    declared.sort();
    declared.dedup();
    xirang_kernel::sha256_hex(declared.join("\n").as_bytes())
}

/// The names one source file calls directly, sorted and deduplicated (audit `W3-1`).
/// 一份源码文件直接调用的名字，排序去重（审计 `W3-1`）。
///
/// The rule is the kernel's `direct_calls`, the same one every reader in the bridge uses, so a
/// record and a derivation cannot answer differently about what this file calls.
/// 规则用的是内核的 `direct_calls`，也就是桥里每个读者用的那一条，因此记录与推导对"这份文件调用了什么"
/// 不可能给出不同答案。
fn called_names(source: &str) -> String {
    let mut calls: Vec<String> = crate::source_cache::facts(source)
        .calls
        .iter()
        .flatten()
        .cloned()
        .collect();
    calls.sort();
    calls.dedup();
    if calls.is_empty() {
        "-".to_owned()
    } else {
        calls.join(",")
    }
}

/// Write the declaration-shaped facts this build can see (audit `W6-2`, step three).
/// 写下这次构建看得见的"声明形状"事实（审计 `W6-2` 第③步）。
///
/// The bridge's family comparison needs to know **which fields each face declares** before it decides
/// who is the outlier — and today it learns that by lexing every sibling's file. The build already has
/// the same parse in hand, so it publishes the **raw pair** it saw (`field` and the text the
/// declaration spelled) and leaves the interpretation where it belongs: the comparison vocabulary
/// (`SHAPE_FIELDS`, its comparison kinds) stays in the bridge, which is why this record carries
/// declarations rather than verdicts.
/// 桥的家族比对在判定"谁是离群者"之前需要知道**每个面声明了哪些字段**——而今天它是靠词法每个兄弟的文件学到的。
/// 构建手里本来就有同一次解析，因此它发布**看见的那对原始值**（字段名与声明拼出的文本），把解释留在它该在的
/// 地方：比对词汇（`SHAPE_FIELDS` 与它的比较种类）留在桥里——这正是这份记录携带的是**声明**而不是**结论**的
/// 原因。
///
/// Every face gets a row even when it declares none (`field = -`), for the same reason the pruning
/// manifest does: a reader that counts must be able to see the whole family.
/// 每个面都有一行，哪怕它什么都没声明（`field = -`），理由与剪枝清单相同：要计数的读者必须看得见整个家族。
pub(crate) fn write_shape_manifest(
    src: &Path,
    nodes: &[Node],
    out_dir: &Path,
) -> Result<(), String> {
    let mut rows: Vec<(String, String, String)> = Vec::new();
    visit_shape_fields(src, nodes, &mut rows);
    write_shape_rows(out_dir.join("shape_manifest.tsv"), rows)
}

fn visit_shape_fields(src: &Path, nodes: &[Node], rows: &mut Vec<(String, String, String)>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let mut declared = 0usize;
                for name in xirang_kernel::declaration::FACE_FIELD_ORDER {
                    if let Some(raw) = face.field(name) {
                        declared += 1;
                        rows.push((relative.clone(), (*name).to_owned(), raw));
                    }
                }
                if declared == 0 {
                    rows.push((relative, "-".to_owned(), "-".to_owned()));
                }
            }
        }
        visit_shape_fields(src, &node.children, rows);
    }
}

/// Write one row per function each source file declares (audit `T1`).
/// 每份源码文件声明的每个函数写一行（审计 `T1`）。
///
/// `search --query` answers two questions from bytes it reads: "is this a **path**" and "is this a
/// **function name**". Both are facts the build already has in hand, and reading every file in the
/// tree to re-derive them was measured at **96 s** on a 50,000-file workspace (a query that matched
/// nothing paid the whole scan, because a matching one exits early on `limit` — so the earlier
/// "24 s → 14.6 s" figures were partial runs). This manifest is the missing half: paths and names,
/// published, so the query is a record read.
/// `search --query` 用它读到的字节回答两个问题："这是不是一个**路径**"、"这是不是一个**函数名**"。两者都是
/// 构建手里本来就有的事实，而为了重新推出它们去读树里每个文件，在 50,000 文件的工作区上实测 **96 s**（什么
/// 都没命中的查询要付整次扫描，而命中的那个会因 `limit` 提前退出——所以先前那些"24 s → 14.6 s"是**部分**
/// 运行）。这份清单就是缺的那一半：路径与名字，发布出来，让查询变成一次记录读取。
///
/// A file with no function still gets a row (`function = -`): a reader that matches **paths** must see
/// every file, or its answer would silently be about the files that happen to declare something.
/// 没有函数的文件照样有一行（`function = -`）：按**路径**匹配的读者必须看得见每份文件，否则它的答案会静默
/// 地只关于那些恰好声明了东西的文件。
pub(crate) fn write_file_manifest(
    src: &Path,
    nodes: &[Node],
    out_dir: &Path,
) -> Result<Vec<(String, String, String)>, String> {
    let mut rows = Vec::new();
    visit_file_functions(src, nodes, &mut rows);
    write_file_rows(out_dir.join("file_manifest.tsv"), rows)
}

fn visit_file_functions(src: &Path, nodes: &[Node], rows: &mut Vec<(String, String, String)>) {
    for node in nodes {
        if let Some(file) = &node.file
            && let Ok(source) = fs::read_to_string(file)
        {
            let relative = relative_display(src, file);
            let symbols = crate::source_cache::facts(&source).functions.clone();
            if symbols.is_empty() {
                rows.push((relative.clone(), "-".to_owned(), "-".to_owned()));
            }
            for symbol in symbols {
                rows.push((relative.clone(), symbol.name, "-".to_owned()));
            }
        }
        visit_file_functions(src, &node.children, rows);
    }
}

pub(crate) fn write_function_manifest(
    src: &Path,
    nodes: &[Node],
    out_dir: &Path,
) -> Result<(), String> {
    let mut rows = Vec::new();
    visit_function_symbols(src, nodes, &mut rows);
    write_rows(out_dir.join("function_manifest.tsv"), rows)
}

pub(crate) fn write_source_scope_manifest(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    out_dir: &Path,
) -> Result<(), String> {
    let Some(selected) = &scope.roots else {
        write_if_changed(
            &out_dir.join("source_scope.tsv"),
            &format!(
                "# mode\t{}\n# result\tall\n# selected\tall\n# reason\t{}\n",
                if scope.reason == "scope-all" {
                    "explicit"
                } else {
                    "auto"
                },
                scope.reason
            ),
        )?;
        return Ok(());
    };
    let mut output = format!(
        "# mode\t{}\n# selected\t{}\n# node\tsource\tmodule\n",
        if scope.reason == "scope-all" {
            "explicit"
        } else {
            "auto"
        },
        selected.len()
    );
    for face in collect_faces(src, nodes) {
        if selected.contains(&face.id) {
            writeln!(
                output,
                "{}\t{}\t{}",
                face.id,
                relative_display(src, &face.source),
                face.module
            )
            .unwrap();
        }
    }
    write_if_changed(&out_dir.join("source_scope.tsv"), &output)
}

/// Persist host graft selectors as data-only build metadata.
/// 将宿主 graft 选择器持久化为只含数据的构建元信息。
pub(crate) fn write_graft_manifest(out_dir: &Path, grafts: &[GraftSyntax]) -> Result<(), String> {
    let mut output = String::from("# cut\tgraft\tfull\tline\tcolumn\n");
    for graft in grafts {
        // The manifest is human-facing audit text, so a range is rendered back
        // to `"start to end"` here through the one rule that owns it.
        // 清单是给人看的审计文本，因此区间在这里经那条唯一的规则渲染回 `"start to end"`。
        let cut = super::graft_view::graft_cut_label(&graft.cut, graft.cut_end.as_deref());
        writeln!(
            output,
            "{cut}\t{}\t{}\t{}\t{}",
            graft.graft, graft.full, graft.location.line, graft.location.column
        )
        .unwrap();
    }
    write_if_changed(&out_dir.join("graft_plan.tsv"), &output)
}

/// Write the face manifest: identity, symbol, and the four facts the declaration spelled.
/// 写面清单：身份、符号，以及声明拼出的四项事实。
fn write_face_rows(
    path: impl AsRef<Path>,
    mut rows: Vec<(NodeId, String, String, FaceColumns)>,
) -> Result<Vec<PruningRow>, String> {
    rows.sort();
    rows.dedup();
    let mut output = String::from(
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls\tparent_node\towns_registry\tlogical_path\n",
    );
    let mut published = Vec::with_capacity(rows.len());
    for (id, source, symbol, columns) in rows {
        writeln!(
            output,
            "{id}\t{source}\t{symbol}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            columns.path,
            columns.kind,
            columns.registry_name,
            columns.parent,
            columns.source_hash,
            columns.fields,
            columns.calls,
            columns.parent_node,
            columns.owns_registry,
            columns.logical_path
        )
        .unwrap();
        published.push(columns.to_row(id, source, symbol));
    }
    write_if_changed(path.as_ref(), &output)?;
    Ok(published)
}

fn write_rows(
    path: impl AsRef<Path>,
    mut rows: Vec<(NodeId, String, String)>,
) -> Result<(), String> {
    rows.sort();
    rows.dedup();
    let mut output = String::from("# node\tsource\tsymbol\n");
    for (id, source, symbol) in rows {
        writeln!(output, "{id}\t{source}\t{symbol}").unwrap();
    }
    write_if_changed(path.as_ref(), &output)
}

/// A record's rows as `(source, name, note)`, in the order the file's own header declares.
/// 一份记录的行，形如 `(source, name, note)`，按文件自己表头声明的顺序。
fn write_file_rows(
    path: impl AsRef<Path>,
    mut rows: Vec<(String, String, String)>,
) -> Result<Vec<(String, String, String)>, String> {
    rows.sort();
    rows.dedup();
    let mut output = String::from("# source\tfunction\tnote\n");
    for (source, name, note) in &rows {
        writeln!(output, "{source}\t{name}\t{note}").unwrap();
    }
    write_if_changed(path.as_ref(), &output)?;
    Ok(rows)
}

/// The rows of a record keyed by source path rather than by identity (audit `W6-2`, step three).
/// 一份按源码路径而不是按身份做键的记录行（审计 `W6-2` 第③步）。
fn write_shape_rows(
    path: impl AsRef<Path>,
    mut rows: Vec<(String, String, String)>,
) -> Result<(), String> {
    rows.sort();
    rows.dedup();
    let mut output = String::from("# source\tfield\tvalue\n");
    for (source, field, value) in rows {
        writeln!(output, "{source}\t{field}\t{value}").unwrap();
    }
    write_if_changed(path.as_ref(), &output)
}

/// List the functions one source file declares, through the kernel's lexical
/// scanner.
/// 通过内核的词法扫描器列出一个源码文件声明的函数。
///
/// The line scanner this replaced matched `fn ` anywhere on a line, so a
/// commented-out function or a doc example was a declaration, and it guessed an
/// `impl` owner from "the previous `impl ` line has no closing brace", which
/// nested `impl` blocks and closures polluted. The kernel scanner masks what is
/// not Rust before it looks, and reports no ownership — so the symbol is
/// `module::name` and nothing invented is published.
/// 它替换掉的按行扫描器在行的任意位置匹配 `fn `，因此被注释掉的函数或文档示例都成了声明；它还用
/// "上一个 `impl ` 行没有右花括号"猜 impl 归属，嵌套 impl 与闭包都会污染它。内核扫描器在寻找之前
/// 先屏蔽非 Rust 文本，且不报告归属——因此符号是 `module::name`，不发布任何编造出来的东西。
fn visit_function_symbols(src: &Path, nodes: &[Node], rows: &mut Vec<(NodeId, String, String)>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let id = super::registry_identity::package_node_id(
                    &relative,
                    &face.path("kind").unwrap_or_else(|| node.name.clone()),
                );
                let module = source_module_path(&relative);
                for function in crate::source_cache::facts(&source).functions.clone() {
                    rows.push((id, relative.clone(), format!("{module}::{}", function.name)));
                }
            }
        }
        visit_function_symbols(src, &node.children, rows);
    }
}

fn visit_pruning_symbols(
    src: &Path,
    nodes: &[Node],
    modules: &std::collections::BTreeMap<String, NodeId>,
    paths: &std::collections::BTreeMap<NodeId, String>,
    rows: &mut Vec<(NodeId, String, String, FaceColumns)>,
) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let kind = face.path("kind").unwrap_or_else(|| node.name.clone());
                let id = super::registry_identity::package_node_id(&relative, &kind);
                let module = source_module_path(&relative);
                let columns = FaceColumns::declared(&face, &kind, &source, modules, paths.get(&id));
                let mut found = false;
                for item in source.lines().filter_map(parse_pruning_item) {
                    found = true;
                    rows.push((
                        id,
                        relative.clone(),
                        tracked_symbol(&module, &kind, item),
                        columns.clone(),
                    ));
                }
                if !found {
                    rows.push((id, relative, "-".to_owned(), columns));
                }
            }
        }
        visit_pruning_symbols(src, &node.children, modules, paths, rows);
    }
}

/// Every registration face's identity, by the module path that face owns (audit `W3-1b`).
/// 每个注册面的身份，按它拥有的模块路径建索引（审计 `W3-1b`）。
///
/// The same walk and the same guards as the row visitor, so the two passes cannot disagree about
/// which files are faces; the map is what turns a `parent:` module path into an identity.
/// 与行遍历同一次遍历、同一批守卫，因此两遍对"哪些文件是面"不可能有分歧；这张映射就是把 `parent:` 的
/// 模块路径变成身份的东西。
fn visit_face_modules(
    src: &Path,
    nodes: &[Node],
    modules: &mut std::collections::BTreeMap<String, NodeId>,
) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let kind = face.path("kind").unwrap_or_else(|| node.name.clone());
                let id = super::registry_identity::package_node_id(&relative, &kind);
                modules.insert(source_module_path(&relative), id);
            }
        }
        visit_face_modules(src, &node.children, modules);
    }
}

/// The `(id, parent id, registry name)` triples the logical-path walk needs (audit `W3-2`).
/// 逻辑路径行走所需的 `(id, 父级 id, 注册名)` 三元组（审计 `W3-2`）。
///
/// Runs **after** the module index is complete, for the reason the writer's own comment gives: a
/// child may be visited before the module it names.
/// 在模块索引**建完之后**才跑，理由见写入方自己的注：子面可能先于它点名的模块被访问到。
fn visit_face_names(
    src: &Path,
    nodes: &[Node],
    modules: &std::collections::BTreeMap<String, NodeId>,
    names: &mut Vec<(NodeId, NodeId, String)>,
) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let kind = face.path("kind").unwrap_or_else(|| node.name.clone());
                let id = super::registry_identity::package_node_id(&relative, &kind);
                let module = source_module_path(&relative);
                // Use the same parent resolver the static plan uses: it knows a native parent
                // (`crate::xirang_external::…`), while a module-only table would put a Rust child of a
                // C++ face at the root. The logical catalog must agree with the compiled tree.
                // 与静态计划共用父级解析：它知道外语父（`crate::xirang_external::…`），而只有模块表的解析
                // 会把 C++ 面下的 Rust 子面放到根上。逻辑目录必须与编译出的树一致。
                let parent = super::cached_parent_id(src, &face)
                    .unwrap_or_else(|| resolved_parent_id(&face, modules).0);
                names.push((
                    id,
                    parent,
                    super::face_view::resolved_registry_name(&face, &module),
                ));
            }
        }
        visit_face_names(src, &node.children, modules, names);
    }
}

/// One tracked symbol's path, built from the facts that name it.
/// 一条被跟踪符号的路径，由命名它的事实拼出。
///
/// `PRUNING_TABLE` and `pruning_probe` are items of the face's module, so the path
/// is `module::item`. `optional_pruning_probe` is a method on the face's own type,
/// so the path starts at that type — the face's `kind`. The row scanner this
/// replaced returned the literal `Button::optional_pruning_probe` for every face,
/// which is a wrong answer for any face whose `kind` is not `Button`, and nothing
/// in the tree could notice: those three identifiers appear nowhere else in the
/// repository, so no fixture ever contradicted it.
/// `PRUNING_TABLE` 与 `pruning_probe` 是该面模块里的条目，路径即 `module::item`。
/// `optional_pruning_probe` 是该面自己类型上的方法，路径从那个类型——面的 `kind`——开始。它替换掉的
/// 按行扫描器对每个面都返回字面量 `Button::optional_pruning_probe`，对任何 `kind` 不叫 `Button` 的
/// 面都是错答案，而树里没有任何东西能发现：那三个标识符在本仓库别处不再出现，因此没有任何夹具反驳过它。
fn tracked_symbol(module: &str, kind: &str, item: PruningItem) -> String {
    match item {
        PruningItem::OptionalProbe => format!("{kind}::{}", item.name()),
        PruningItem::Table | PruningItem::Probe => format!("{module}::{}", item.name()),
    }
}

/// One of the three item shapes whose symbols the release prunes.
/// 发布期会修剪其符号的三种条目形状之一。
#[derive(Clone, Copy)]
enum PruningItem {
    /// `static PRUNING_TABLE: …` in the face's module.
    /// 面模块里的 `static PRUNING_TABLE: …`。
    Table,
    /// `fn pruning_probe` in the face's module.
    /// 面模块里的 `fn pruning_probe`。
    Probe,
    /// `fn optional_pruning_probe` on the face's own type.
    /// 面自己类型上的 `fn optional_pruning_probe`。
    OptionalProbe,
}

impl PruningItem {
    /// The name the source writes it under.
    /// 源码里书写它的那个名字。
    fn name(self) -> &'static str {
        match self {
            Self::Table => "PRUNING_TABLE",
            Self::Probe => "pruning_probe",
            Self::OptionalProbe => "optional_pruning_probe",
        }
    }
}

fn parse_pruning_item(line: &str) -> Option<PruningItem> {
    let line = line.trim();
    if line.contains("PRUNING_TABLE")
        && (line.starts_with("static ") || line.starts_with("pub static "))
    {
        Some(PruningItem::Table)
    } else if line.contains("pruning_probe") && line.contains("fn pruning_probe") {
        Some(PruningItem::Probe)
    } else if line.contains("optional_pruning_probe") && line.contains("fn optional_pruning_probe")
    {
        Some(PruningItem::OptionalProbe)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "manifests_tests.rs"]
mod manifests_tests;
