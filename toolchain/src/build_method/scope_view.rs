//! The scope and pruning readers: what the build published in `out_dir`.
//! 作用域与修剪读取方：构建发布在 `out_dir` 中的内容。
//!
//! Split decision: this page only reads the two manifests the build writes
//! (`source_scope.tsv`, `pruning_manifest.tsv`) and the view type derived from
//! them. It re-exports through `super::face_view` so `xirang_build_method`
//! keeps its `face_view::{BuildScopeView, PruningRow, read_build_scope,
//! read_pruning_manifest}` surface unchanged, and it deliberately sits beside
//! the discovery walk (which reads source) because both are the build's own
//! read-only view of a host it cannot link.
//! 拆分决定：本页只读取构建写出的两份清单（`source_scope.tsv`、
//! `pruning_manifest.tsv`）以及由它们派生的视图类型。它经 `super::face_view` 重新
//! 导出，使 `xirang_build_method` 的 `face_view::{BuildScopeView, PruningRow,
//! read_build_scope, read_pruning_manifest}` 表面保持不变；它刻意与发现遍历
//! （读取源码）相邻，因为两者都是构建对无法链接的宿主的只读视图。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::super::registry_identity::NodeId;

/// Whether `out_dir` was published from the sources the package has right now.
/// `out_dir` 是否由该包**此刻**的源码发布而来。
///
/// A reader that answers questions about the build from `out_dir` — `explain` is
/// the one — has to know whether that output still describes this tree. Without
/// this check the same tree gave two different answers depending on whether
/// `check` had happened to run in between: `explain` served the previous build's
/// scope as `known: true`, and it did the same for output left behind by a check
/// that *failed*. The published `discovery.fingerprint` is the token: the pipeline
/// writes it on a clean run and removes it when validation fails, so a missing or
/// different fingerprint means "ask the build again".
/// 从 `out_dir` 回答构建问题的读取方——`explain` 就是——必须知道那份产物是否仍在描述这棵树。
/// 没有这道检查时，同一棵树会因期间是否恰好跑过 `check` 而给出两个不同答案：`explain` 会把
/// 上一次构建的作用域当作 `known: true` 提供，对**失败**的 check 留下的产物也一样。已发布的
/// `discovery.fingerprint` 就是那枚凭据：干净的一次运行写下它，校验失败时移除它，因此指纹缺失或
/// 不同只意味着"再问构建一次"。
pub fn build_output_is_current(root: &Path, out_dir: &Path) -> bool {
    let Ok(stored) = fs::read_to_string(out_dir.join("discovery.fingerprint")) else {
        return false;
    };
    // The same two roots the pipeline computes the fingerprint from: the walk reads
    // `scan_root`, and the fingerprint is taken over the *identity* base
    // (`pipeline.rs` does exactly this). Recomputing both from `root/src` meant a
    // package whose library target lives elsewhere — the layout `source_layout`
    // supports and `build_method/tests/outside_src_layout.rs` fixtures — never
    // matched its own token: every read answered "not current", so `explain` and the
    // MCP evidence stayed `unknown` for good. The direction was conservative, which
    // is why nothing failed loudly, but the answer was permanently unavailable
    // (audit `SUR-S18`).
    // 与管线计算指纹时相同的两个根：遍历读 `scan_root`，指纹按**身份基准**取（`pipeline.rs` 正是
    // 这么做的）。这里过去两者都按 `root/src` 复算，因此库目标住在别处的包——`source_layout`
    // 支持的布局，`build_method/tests/outside_src_layout.rs` 有夹具——永远对不上自己的凭据：每次
    // 读取都答"不是当前的"，`explain` 与 MCP 的 evidence 长期是 `unknown`。这个方向是保守的，因此
    // 没有东西响亮失败，但那个答案永远拿不到（审计 `SUR-S18`）。
    let Ok(layout) = crate::build_method::source_layout(root) else {
        return false;
    };
    let scan = &layout.scan_root;
    if !scan.is_dir() {
        return false;
    }
    let nodes = crate::build_method::source_walk::discover_root(scan);
    stored.trim()
        == crate::build_method::source_walk::discovery_fingerprint(
            &layout.identity_base,
            scan,
            &nodes,
        )
}

/// One row of `pruning_manifest.tsv`: a symbol release-time pruning tracks for
/// one face.
/// `pruning_manifest.tsv` 的一行：发布期修剪为一个面跟踪的一个符号。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PruningRow {
    /// The face the symbol belongs to.
    /// 该符号所属的注册面。
    pub id: NodeId,
    /// The face's source path, relative to `src/`.
    /// 该面的源码路径，相对 `src/`。
    pub source: String,
    /// The tracked symbol, or `-` when the face has none.
    /// 被跟踪的符号；该面没有时是 `-`。
    pub symbol: String,
    /// The logical path the face declared, when the record carried one.
    /// 该面声明的逻辑路径；记录携带它时才有。
    ///
    /// `None` means the record did not carry the column (the three-column form an older build
    /// wrote) or the declaration named none. Either way the answer is the same for a reader:
    /// derive it from the sources rather than trust a published value.
    /// `None` 意为记录没有携带该列（较早构建写下的三列形式），或声明里没有它。对读者而言两种情形答案
    /// 相同：从源码推导，而不是采信一个已发布的值。
    pub path: Option<String>,
    /// The declared `kind`, when published.
    /// 已发布时，声明的 `kind`。
    pub kind: Option<String>,
    /// The declared `registry_name`, when published.
    /// 已发布时，声明的 `registry_name`。
    pub registry_name: Option<String>,
    /// The parent the declaration spelled, as a Rust path, when published.
    /// 已发布时，声明拼出的父级（Rust 路径）。
    pub parent: Option<String>,
    /// The sha256 of the face's own source bytes, when published (audit `W3-1`).
    /// 该面自己那份源码字节的 sha256，已发布时才有（审计 `W3-1`）。
    ///
    /// `None` means the record predates the column. A reader that needs freshness compares this
    /// against the file it can read now; the build never publishes a hash it did not compute from
    /// the bytes it was looking at.
    /// `None` 意为记录早于该列。需要新鲜度的读者拿它与此刻能读到的文件比对；构建从不发布一个不是从它当时
    /// 看着的那份字节算出的散列。
    pub source_hash: Option<String>,
    /// The declaration's field fingerprint, when published (audit `W3-1`).
    /// 声明字段的指纹，已发布时才有（审计 `W3-1`）。
    pub fields: Option<String>,
    /// The names the face's own file calls directly, when published (audit `W3-1`).
    /// 该面自己那份文件直接调用的名字，已发布时才有（审计 `W3-1`）。
    ///
    /// The reader drops `-`, so a face that calls nothing reads as `None` — the same answer a
    /// derivation gives for a file with no calls, which is what keeps the two paths comparable.
    /// 读取方会滤掉 `-`，因此"什么都没调用"的面读出来也是 `None`——与推导对一份没有调用的文件给出的答案
    /// 相同，正是这一点让两条路可比。
    pub calls: Option<String>,
    /// The **resolved** parent identity, when the record published one (audit `W3-1b`).
    /// **解析后**的父级身份，记录发布过时才有（审计 `W3-1b`）。
    ///
    /// This is the column that lets a reader build a `FaceView` from the record instead of walking
    /// the sources: the `parent` column beside it is the Rust path the **declaration** spelled, and
    /// turning that into an identity is the work this one saves. `None` means the record predates
    /// the column *or* the chain did not resolve — both answer the same way, which is "derive".
    /// 这一列让读者能**从记录**构造 `FaceView`，而不必走一遍源码：它旁边那列 `parent` 是**声明**拼出的
    /// Rust 路径，而把那个变成身份正是这一列省下的工作。`None` 意为记录早于该列，**或**链没解析出来——
    /// 两者答案相同：推导。
    pub parent_node: Option<String>,
    /// Whether the declaration gave this face a registry of its own (audit `W3-1b`).
    /// 这条声明是否给了这个面自己的注册机（审计 `W3-1b`）。
    pub owns_registry: Option<String>,
    /// The **logical** path the build derives by walking the resolved parent chain (audit `W3-2`).
    /// **逻辑**路径，构建沿解析后的父链走出来的（审计 `W3-2`）。
    ///
    /// Distinct from `path` beside it: `path` is what the declaration spelled (`None` for every
    /// macro-derived face), while this is the answer the runtime would give, and it is what the
    /// census counts levels on.
    /// 与旁边的 `path` 是两件事：`path` 是声明拼出的（对每个宏派生的面都是 `None`），而这一列是运行期会
    /// 给出的答案，也是普查用来计层的那一列。
    pub logical_path: Option<String>,
}

/// The build scope the last pipeline run published in `out_dir`.
/// 上一次管线运行发布在 `out_dir` 的构建作用域。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildScopeView {
    /// `explicit` when the scope came from `XIRANG_SCOPE`, `auto` otherwise.
    /// 作用域来自 `XIRANG_SCOPE` 时为 `explicit`，否则为 `auto`。
    pub mode: String,
    /// True when the manifest recorded `# result all`: every face is in scope.
    /// 清单记录 `# result all` 时为真：每个面都在作用域内。
    pub all: bool,
    /// The build's own one-line reason for the scope.
    /// 构建对作用域给出的一行原因。
    pub reason: Option<String>,
    /// The source paths of the selected faces, as the manifest's second column
    /// records them.
    /// 被选中注册面的源码路径，按清单第二列记录。
    pub selected_sources: BTreeSet<String>,
    /// The identities of the selected faces.
    /// 被选中注册面的身份。
    pub selected_ids: BTreeSet<NodeId>,
    /// The modules of the selected faces, as the manifest's third column
    /// records them.
    /// 被选中注册面的模块，按清单第三列记录。
    pub selected_modules: BTreeSet<String>,
}

impl BuildScopeView {
    /// Whether the published scope keeps a face whose module is `module`.
    /// 已发布的作用域是否保留模块为 `module` 的注册面。
    ///
    /// A narrowed scope records only its *roots* — the faces a declared cut
    /// names — but a parent face survives because a selected child needs it and
    /// because the entry reaches it. Treating "not a root" as "pruned" would
    /// tell an operator that a face present in the shipped tree is gone, so this
    /// test also accepts any selected module nested under `module`. The boundary
    /// is the `::` segment, exactly like the scope's own subtree selector, so
    /// `control` keeps `control::object::button` and never `control_extra`.
    /// 收窄的作用域只记录它的**根**——已声明切口命名的那些面——但父面会因为被选中的
    /// 子级需要它、入口能到达它而存活。把"不是根"当成"被剪掉"会告诉操作者一个仍然
    /// 存在于发布树中的面消失了，因此这里也接受嵌套在 `module` 之下的任何被选中模块。
    /// 边界是 `::` 段，与作用域自己的子树选择器完全一致：`control` 保留
    /// `control::object::button`，绝不保留 `control_extra`。
    pub fn keeps(&self, module: &str) -> bool {
        self.all
            || self.selected_modules.iter().any(|selected| {
                selected == module
                    || (selected.starts_with(module)
                        && selected.as_bytes().get(module.len()) == Some(&b':'))
            })
    }
}

/// Read `source_scope.tsv` as the build wrote it.
/// 按构建写出的样子读取 `source_scope.tsv`。
///
/// The format is build output, so this reader lives beside the writer instead of
/// in the command; a command that re-parsed it would freeze a second copy of the
/// layout. A missing file means the build has not run yet, which is an error the
/// caller reports with the path that is missing.
/// 该格式是构建产物，因此读取方与写入方放在一起，而不是放进命令里；命令自行解析会
/// 冻结第二份版式。文件缺失意味着构建尚未运行，调用方按缺失路径报错。
pub fn read_build_scope(out_dir: &Path) -> Result<BuildScopeView, String> {
    let path = out_dir.join("source_scope.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut scope = BuildScopeView {
        mode: "auto".to_owned(),
        all: false,
        reason: None,
        selected_sources: BTreeSet::new(),
        selected_ids: BTreeSet::new(),
        selected_modules: BTreeSet::new(),
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# mode\t") {
            scope.mode = rest.to_owned();
        } else if let Some(rest) = line.strip_prefix("# result\t") {
            scope.all = rest == "all";
        } else if let Some(rest) = line.strip_prefix("# reason\t") {
            scope.reason = Some(rest.to_owned());
        } else if line.is_empty() || line.starts_with('#') {
            continue;
        } else {
            let mut fields = line.split('\t');
            if let Some(id) = fields.next().and_then(|value| value.parse::<NodeId>().ok()) {
                scope.selected_ids.insert(id);
            }
            if let Some(source) = fields.next() {
                scope.selected_sources.insert(source.to_owned());
            }
            if let Some(module) = fields.next() {
                scope.selected_modules.insert(module.to_owned());
            }
        }
    }
    Ok(scope)
}

/// Read `pruning_manifest.tsv` as the build wrote it.
/// 按构建写出的样子读取 `pruning_manifest.tsv`。
pub fn read_pruning_manifest(out_dir: &Path) -> Result<Vec<PruningRow>, String> {
    let path = out_dir.join("pruning_manifest.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let Some(id) = fields.next().and_then(|value| value.parse::<NodeId>().ok()) else {
            continue;
        };
        let Some(source) = fields.next() else {
            continue;
        };
        let Some(symbol) = fields.next() else {
            continue;
        };
        // Every column after the symbol is optional: the three-column form an older build wrote
        // parses to four `None`s, and `-` is how the writer spells "the declaration named none".
        // 符号之后的每一列都是可选的：较早构建写下的三列形式解析出四个 `None`，而 `-` 是写入方拼写
        // "声明里没有它"的方式。
        let mut column = || {
            fields
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty() && *value != "-")
                .map(str::to_owned)
        };
        rows.push(PruningRow {
            id,
            source: source.to_owned(),
            symbol: symbol.to_owned(),
            path: column(),
            kind: column(),
            registry_name: column(),
            parent: column(),
            source_hash: column(),
            fields: column(),
            calls: column(),
            parent_node: column(),
            owns_registry: column(),
            logical_path: column(),
        });
    }
    Ok(rows)
}

/// One field a declaration spelled, as the shape record carries it (audit `W6-2`, step three).
/// 一条声明拼出的一个字段，按形状记录携带的样子（审计 `W6-2` 第③步）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeRow {
    /// The face's source path, relative to `src/`.
    /// 该面的源码路径，相对 `src/`。
    pub source: String,
    /// The field's name, or `-` for a face that declares none of the fields the record covers.
    /// 字段名；一个面在记录覆盖的字段里什么都没声明时是 `-`。
    pub field: String,
    /// The text the declaration spelled.
    /// 声明拼出的文本。
    pub value: String,
}

/// One function a source file declares, as the file manifest carries it (audit `T1`).
/// 一份源码文件声明的一个函数，按文件清单携带的样子（审计 `T1`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    /// The file's path, relative to `src/`.
    /// 该文件的路径，相对 `src/`。
    pub source: String,
    /// The function's name, or `-` for a file that declares none.
    /// 函数名；一份什么都没声明的文件写作 `-`。
    pub function: String,
}

/// Read the file/function facts the last build published (audit `T1`).
/// 读取上一次构建发布的"文件/函数"事实（审计 `T1`）。
pub fn read_file_manifest(out_dir: &Path) -> Result<Vec<FileRow>, String> {
    let path = out_dir.join("file_manifest.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(source), Some(function)) = (fields.next(), fields.next()) else {
            continue;
        };
        rows.push(FileRow {
            source: source.to_owned(),
            function: function.to_owned(),
        });
    }
    Ok(rows)
}

/// Read the declaration-shaped facts the last build published.
/// 读取上一次构建发布的"声明形状"事实。
pub fn read_shape_manifest(out_dir: &Path) -> Result<Vec<ShapeRow>, String> {
    let path = out_dir.join("shape_manifest.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(source), Some(field), Some(value)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        rows.push(ShapeRow {
            source: source.to_owned(),
            field: field.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Both forms of the face manifest read: the three-column one an older build wrote leaves the
    /// four published facts absent, and the seven-column one fills them, with `-` still meaning
    /// "the declaration named none".
    /// 两种面清单形式都能读：较早构建写下的三列形式让四项已发布事实缺席，七列形式填上它们，而 `-` 仍
    /// 意为"声明里没有它"。
    #[test]
    fn the_pruning_reader_accepts_the_three_and_the_seven_column_forms() {
        let out = std::env::temp_dir().join(format!(
            "xirang-pruning-forms-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&out).expect("out dir");
        let id = "bdb4427ce81c9bc51e56bee7667fd2be";
        let text = format!(
            "# node\tsource\tsymbol\n\
             {id}\tcontrol/control.rs\t-\n\
             # node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\n\
             {id}\tcontrol/control.rs\t-\tcontrol::object::button\tButton\tbutton\tcrate::root_node_id(env!(\"CARGO_PKG_NAME\"))\n"
        );
        std::fs::write(out.join("pruning_manifest.tsv"), text).expect("manifest");
        let rows = read_pruning_manifest(&out).expect("both forms read");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path, None);
        assert_eq!(rows[0].kind, None);
        assert_eq!(rows[1].path.as_deref(), Some("control::object::button"));
        assert_eq!(rows[1].kind.as_deref(), Some("Button"));
        assert_eq!(rows[1].registry_name.as_deref(), Some("button"));
        assert!(
            rows[1]
                .parent
                .as_deref()
                .unwrap_or_default()
                .contains("root_node_id"),
            "{:?}",
            rows[1].parent
        );
        let _ = std::fs::remove_dir_all(&out);
    }

    /// A minimal host: one valid root face and the conventional entry.
    /// 最小宿主：一个合法的根面与约定入口。
    fn host(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-{label}-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src/alpha")).expect("src tree");
        fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        )
        .expect("manifest");
        fs::write(root.join("src/lib.rs"), "// host entry\n").expect("entry");
        fs::write(
            root.join("src/alpha/alpha.rs"),
            "crate::root_object! {\n    kind: Alpha,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("face");
        root
    }

    /// Published output describes the sources it was built from, and stops being
    /// trusted the moment they change.
    /// 已发布的产物描述它构建时的那批源码，并在源码一变就不再被信任。
    #[test]
    fn published_output_is_current_until_the_sources_change() {
        let root = host("scope-view-current");
        let out = root.join("target/xirang/out");
        crate::build_method::check_for(&root, &out, "scope-view-current")
            .expect("a valid host checks clean");
        assert!(
            build_output_is_current(&root, &out),
            "a clean run publishes output that describes these sources"
        );

        // A content change that stays valid: the fingerprint hashes bytes, so the
        // pin is about the token, not about a semantic edit.
        // 一次仍然合法的内容变化：指纹哈希的是字节，因此这条钉子关乎那枚凭据，而不是语义改动。
        fs::write(
            root.join("src/alpha/alpha.rs"),
            "// edited\ncrate::root_object! {\n    kind: Alpha,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("edited face");
        assert!(
            !build_output_is_current(&root, &out),
            "the same tree must not report a previous build as current"
        );

        crate::build_method::check_for(&root, &out, "scope-view-current")
            .expect("the edited host still checks");
        assert!(
            build_output_is_current(&root, &out),
            "a clean run makes the output current again"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A failed run publishes no token, so a reader asks the build instead of
    /// trusting what the failed run left behind.
    /// 失败的一次运行不发布任何凭据，读取方因此去问构建，而不是相信那次运行留下的东西。
    #[test]
    fn a_failed_check_publishes_no_trusted_output() {
        let root = host("scope-view-failed");
        let out = root.join("target/xirang/out");
        fs::write(
            root.join("src/alpha/alpha.rs"),
            "crate::root_object! {\n    kind:\n}\n",
        )
        .expect("broken face");

        let outcome = crate::build_method::check_for(&root, &out, "scope-view-failed");
        assert!(outcome.is_err(), "a malformed face must fail the check");
        assert!(
            !out.join("discovery.fingerprint").exists(),
            "a failed run must leave no fingerprint behind"
        );
        assert!(
            !build_output_is_current(&root, &out),
            "and the output it left must not read as current"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A package whose library target lives outside `src/` matches its own
    /// fingerprint: the reader resolves the same two roots the writer used.
    /// 库目标住在 `src/` 之外的包能对上自己的指纹：读取方解析出与写入方相同的两个根。
    ///
    /// The reader used to recompute both from `root/src`, so such a package never saw
    /// a matching token: every read answered "not current" and the scope/pruning
    /// evidence stayed `unknown` for good. The direction was conservative — it never
    /// called an old artifact current — which is why nothing failed loudly (audit
    /// `SUR-S18`).
    /// 读取方过去两者都按 `root/src` 复算，因此这样的包永远看不到匹配的凭据：每次读取都答"不是当前的"，
    /// 作用域与修剪证据长期是 `unknown`。这个方向是保守的——它从不把旧产物当成当前的——因此没有东西
    /// 响亮失败（审计 `SUR-S18`）。
    #[test]
    fn a_library_target_outside_src_matches_its_own_fingerprint() {
        let root = host_outside_src("scope-view-outside-src");
        let out = root.join("target/xirang/out");
        crate::build_method::check_for(&root, &out, "scope-view-outside-src")
            .expect("the outside-src layout is supported");
        assert!(
            build_output_is_current(&root, &out),
            "the reader must resolve the same roots the pipeline wrote with"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A host whose library target is `host/lib.rs`, with one face next to it.
    /// 库目标是 `host/lib.rs` 的宿主，旁边有一个注册面。
    ///
    /// The face owns a registry, so the canonical rule module has to sit beside it: a face that
    /// declares `needs_registry: true` and names no rule reads `super::registry_rule`, and the build
    /// refuses a tree where that module is missing (measured before the refusal existed:
    /// `error[E0433]: cannot find registry_rule in super`). This fixture used to leave the file out —
    /// harmless while nothing judged it, and exactly the tree a real `cargo build` cannot compile.
    /// 该面拥有注册机，因此规范规则模块必须在它旁边：声明 `needs_registry: true` 又不点名规则的注册面读的是
    /// `super::registry_rule`，而构建会拒绝那棵树（在那条拒绝存在之前实测为
    /// `error[E0433]: cannot find registry_rule in super`）。本夹具过去没有这个文件——在没人判它时无害，
    /// 而那正是真 `cargo build` 编译不过的树。
    fn host_outside_src(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("xirang-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("host/control/registry_rule")).expect("face directory");
        fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [lib]\npath = \"host/lib.rs\"\n"
            ),
        )
        .expect("manifest");
        fs::write(root.join("host/lib.rs"), "// host entry\n").expect("entry");
        fs::write(
            root.join("host/control/registry_rule/registry_rule.rs"),
            "use crate::RegistrationRule;\n\n\
             pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();\n",
        )
        .expect("registration rule");
        fs::write(
            root.join("host/control/control.rs"),
            "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n    \
             parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("face");
        root
    }
}
