//! The discovery record: the tree-level products computed once and copied by every package.
//! 发现记录：树级产物算一次，各包复制。
//!
//! Stage 1 of `docs/design-discovery-record.md`. A workspace whose shape splits one host into N packages
//! runs N build scripts over **one tree**, and every one of them re-derives the same tree-level
//! products: the pruning rows (three walks plus parent resolution), the file and function rows (every
//! file read and scanned), the shape table, and the graft plan. Measured on the shipped shape, seven of
//! the eleven products a build writes are byte-identical across the packages of one tree (§1.2 of the
//! design), so five of them are published here as payloads and copied by whoever comes second.
//! `docs/design-discovery-record.md` 的阶段 1。一个把宿主拆成 N 个包的工作区，会对**同一棵树**跑 N 个构建脚本，
//! 而每一个都会重新推导同样的树级产物：修剪行（三趟遍历加父级解析）、文件与函数行（每个文件都要读、都要扫）、
//! 形状表、以及嫁接计划。在出厂形状上实测：一次构建写出的十一份产物里有七份在同一棵树的各包之间逐字节相同
//! （设计的 §1.2），因此其中五份在这里作为载荷发布，后到者复制即可。
//!
//! **Which five, and why not the other six**: `graph_edges.tsv` is re-derived from the two row sets the
//! build already has in hand (skipping that step would mean inventing a `GraphHeader` the record does not
//! carry), `graph.generation` carries the **package root** (`graph.rs:215`), `source_scope.tsv`,
//! `generated_lib.rs` and `xirang_modules/` are shape-related, and `discovery.fingerprint` with
//! `add-crates.lock` belong to the run rather than to the tree.
//! **为什么是这五份、另外六份为什么不是**：`graph_edges.tsv` 由构建手里已有的那两份行重新推导（跳过它就得凭空造出
//! 一份记录并不携带的 `GraphHeader`），`graph.generation` 带着**包根**（`graph.rs:215`），`source_scope.tsv`、
//! `generated_lib.rs` 与 `xirang_modules/` 是形状相关的，而 `discovery.fingerprint` 与 `add-crates.lock`
//! 属于这一次运行、不属于这棵树。
//!
//! **Freshness is by content, never by time** (§4 of the design): the record lives under
//! `<target>/xirang/records/<tree-fingerprint>/`, and the fingerprint is the one the build already
//! computes from the raw file set and every recorded file's bytes. `record.tsv` is written **last** and
//! carries each payload's name, byte count and sha256 — a reader recomputes them, so a truncated,
//! corrupted or half-published record is rebuilt rather than trusted.
//! **新鲜性由内容保证、绝不由时间保证**（设计的 §4）：记录落在 `<target>/xirang/records/<树指纹>/`，而那个指纹
//! 就是构建已经从"原始文件集合 + 每个记录文件的字节"算出来的那一个。`record.tsv` **最后**写，携带每份载荷的名字、
//! 字节数与 sha256——读取方自己重算，因此被截断、损坏或只发布了一半的记录会被重建而不是被信任。

use std::path::{Path, PathBuf};

use super::write_if_changed;

/// The record shape this module writes and accepts.
/// 本模块写出与接受的记录形状。
/// The record's **shape**, bumped whenever the payload set or a payload's columns change.
/// 记录的**形状**：载荷集合或任何一份载荷的列一变，就推进它。
///
/// The faces payload made this a 2: a record written by 1 has five payloads, and reading it as 2 would
/// report a row with five columns as corruption — while the honest reading is "that record is a previous
/// shape of this cache, ignore it and write the current one". Measured: the example host's `target/`
/// carried a 1, and the clippy gate's example build failed on it (`faces payload row with 5 columns`).
/// 逐面载荷把它推到 2：1 写下的记录只有五份载荷，把它当 2 读会把"五列的行"报成损坏——而诚实的读法是
/// "那份记录是这个缓存的**前一个形状**，忽略它、写下当前这份"。实测：示例宿主的 `target/` 里就躺着一份 1，
/// clippy 门禁里那次示例构建正是被它弄红的（`faces payload row with 5 columns`）。
// Bumped when the **content** of a payload changes shape or meaning: an already-published record
// carries rows derived from whatever the units directory held at that moment, and the freshness
// rule below only governs records this build publishes. A record written before the faces payload
// stopped carrying units the source no longer declares still lists a face nobody compiles, so it
// must not be read at all (measured: such a record made a fragment's tree assert the id of a face
// its source file had replaced).
// 当载荷的**内容**在形状或含义上改变时推一位：已经发布的记录带着"当时 units 目录里有什么"导出的行，
// 而新鲜性规则只管本次构建发布的记录。在"faces 载荷不再携带源文件已不再声明的 unit"之前写下的记录，
// 仍然列着一张没人在编译的面，因此**根本不能被读**（实测：那样一条记录让碎片的树断言了它源文件已经
// 替换掉的那张面的身份）。
pub(crate) const SCHEMA: &str = "xirang-record-5";

/// The tree-level products a record carries, in the order they are published.
/// 记录携带的树级产物，按发布顺序。
pub(crate) const PAYLOADS: [&str; 6] = [
    "pruning_manifest.tsv",
    "file_manifest.tsv",
    "function_manifest.tsv",
    "shape_manifest.tsv",
    "graft_plan.tsv",
    "faces_manifest.tsv",
];

/// The file the projection reads: one row per face in the **tree**, before any package filters it.
/// 投影读取的文件：**树**里每个面一行，尚未经过任何包的过滤。
///
/// `source`, `id`, `parent`, `owns_registry` and the `cfg` text — every column read straight out of the
/// face's unit, so writing this costs no parse (`stem 2`, step 3). The order is by source path, so two
/// runs over the same tree write the same bytes and the payload's hash means something.
/// `source`、`id`、`parent`、`owns_registry` 与 `cfg` 原文——每一列都直接从那个面的 unit 里读出，因此写它
/// 不需要解析（`stem 2` 第③步）。顺序按源码路径，因此同一棵树两次运行写下同样的字节，载荷的哈希才有意义。
pub(crate) fn write_faces_manifest(
    layout: &crate::source_layout::SourceLayout,
    units: &std::path::Path,
    out_dir: &std::path::Path,
) -> Result<usize, String> {
    let mut rows: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(units).map_err(|error| error.to_string())?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        let content = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let Some(facts) = crate::build_method::discovery_cache::read_unit(&content) else {
            continue;
        };
        // **Freshness**: the directory is append-only and every unit is named by the fingerprint of
        // the source it describes, so a name that is not the **current** file's fingerprint belongs to
        // a version nobody is compiling. Without this filter an old unit answers for a path whose file
        // has changed, and the payload lists a face the source no longer declares.
        // **新鲜性**：这个目录只追加，而每个 unit 都以它所描述的那份源码的指纹命名，因此名字**不是**当前文件指纹的
        // 那一份属于没人在编译的版本。少了这道过滤，旧 unit 会代替"文件已经改了"的那个路径作答，载荷里就出现
        // 一张源码已不再声明的面。
        let named = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        // The unit names its source by the **record spelling**, and a spelling is not a path this
        // function can join to one root: an explicitly placed face carries a package-relative spelling
        // that has no `src/` under it. Locate it through the one locator, which folds each candidate
        // back, instead of joining a wrong root and fingerprinting a missing file.
        // unit 以**记录拼写**点名它的源码，而拼写不是本函数能拼到一个根上的路径：显式落点的面携带的包内相对
        // 拼写下面没有 `src/`。经唯一的定位器定位它（它会把每个候选折叠回去），而不是拼一个错误的根、再去
        // 给一个不存在的文件取指纹。
        let Ok(source) = xirang_multilang::source_ref::locate_record_spelling(
            &layout.package_root,
            &layout.identity_base,
            &facts.relative,
        ) else {
            continue;
        };
        let live = crate::build_method::identity_cache::source_unit_fingerprint(&source);
        if named != live {
            continue;
        }
        // A face that declares `plugin:` is replaceable plugin surface and the plan leaves it out, so
        // the payload does not describe it either — the absence of a row *is* that filter's answer.
        // 声明了 `plugin:` 的面是可替换的插件面，计划不收录它，载荷也就不描述它——**行不存在**本身就是那道
        // 过滤的答案。
        if facts
            .fields
            .contains_key(xirang_kernel::lexicon::FACE_FIELD_PLUGIN)
        {
            continue;
        }
        // The parent column is always a **concrete** id: no `parent:` field means the package root, and a
        // `parent:` that cannot be resolved means the plan reports an error and keeps no entry — so a tree
        // that published a record (which requires a clean run, §7j.1) contains no such face at all. The
        // projection therefore never has to reconstruct a three-way decision from a missing column.
        // 父级列恒为**具体** id：没有 `parent:` 字段就是包根，而解析不了的 `parent:` 会让计划报错且不留条目——
        // 因此发布过记录的树（前提是干净的一次运行，§7j.1）里根本不存在这样的面。投影因而不必从"列缺失"里
        // 重建一个三岔判断。
        if facts.parent.is_none() && facts.fields.contains_key("parent") {
            continue;
        }
        let parent = facts.parent.clone().unwrap_or_else(|| {
            crate::build_method::registry_identity::package_root_node_id().to_string()
        });
        let owns = facts
            .values
            .get(&("needs_registry".to_owned(), "bool".to_owned()))
            .map_or("false", String::as_str);
        let cfg = facts
            .fields
            .get("cfg")
            .map(|(_, raw)| raw.as_str())
            .unwrap_or("");
        // The last column is the **declaration's** line: the cfg gate is the one thing on this path
        // that can still fail per package, and its diagnostic points there.
        // 最后一列是**声明**所在的行：这条路径上唯一还能按包失败的就是 cfg 门控，而它的诊断指向那里。
        let line = facts
            .fields
            .get("cfg")
            .map(|(line, _)| *line)
            .unwrap_or_default();
        // The last column is the spelling: a hit path has no source text to re-read, so the form has
        // to travel in the payload rather than be re-derived.
        // 最后一列是拼法：命中路径没有源码可重读，因此"形"必须随载荷走，而不能重新推导。
        rows.push(format!(
            "face\t{}\t{}\t{parent}\t{owns}\t{}\t{line}\t{}\t{}\t{}\t{}",
            facts.relative,
            facts.id,
            escape_column(cfg),
            if facts.written_as_literal {
                "literal"
            } else {
                "macro"
            },
            escape_column(&facts.kind),
            escape_column(
                facts
                    .fields
                    .get("preset")
                    .map(|(_, raw)| raw.as_str())
                    .unwrap_or("")
            ),
            escape_column(
                facts
                    .fields
                    .get("parts")
                    .map(|(_, raw)| raw.as_str())
                    .unwrap_or("")
            )
        ));
    }
    rows.sort();
    let mut text =
        String::from("# xirang faces, tree level: one row per face before any package filter\n");
    for row in &rows {
        text.push_str(row);
        text.push('\n');
    }
    crate::build_method::write_if_changed(&out_dir.join("faces_manifest.tsv"), &text)?;
    Ok(rows.len())
}

// Test-only: stop `publish` after the payloads, before the manifest.
// 仅测试用：让 `publish` 在载荷写完之后、清单之前停下。
//
// **Thread-local on purpose**: the suite runs its tests in parallel, and a process-wide flag would make
// *other* tests' publishes return early — measured: that showed up as one to three unrelated
// "incomplete record" failures in the same run ✗.
// **有意做成线程局部**：测试套件并行跑，进程级的开关会让**别的**测试的发布提前返回——实测：同一次运行里会
// 冒出一到三条与它无关的"记录不完整"失败 ✗。
#[cfg(test)]
thread_local! {
    pub(crate) static PUBLISH_PAUSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// One row of the faces payload: what the projection needs about one face, and nothing else.
/// 逐面载荷的一行：投影关于一个面需要的东西，仅此而已。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FaceRow {
    pub(crate) source: String,
    /// The kind, and the two type names an assembly needs. A hit path cannot re-read the source, so
    /// they travel in the payload like the spelling does.
    /// kind，以及一次拼装需要的两个类型名。命中路径无法重读源码，因此它们与"形"一样随载荷走。
    pub(crate) kind: String,
    pub(crate) preset: String,
    pub(crate) parts: String,
    /// Whether the face was declared as a `Registration` literal (H/A2e), which decides how the
    /// renderer addresses its identity.
    /// 该面是否以 `Registration` 字面量声明（H/A2e）——它决定渲染器如何寻址它的身份。
    pub(crate) written_as_literal: bool,
    pub(crate) id: String,
    pub(crate) parent: String,
    pub(crate) owns_registry: bool,
    pub(crate) cfg: Option<String>,
    pub(crate) line: usize,
}

/// Read the faces payload a hit path runs on.
/// 读取命中路径所依赖的逐面载荷。
pub(crate) fn read_face_rows(out_dir: &std::path::Path) -> Result<Vec<FaceRow>, String> {
    let text = std::fs::read_to_string(out_dir.join("faces_manifest.tsv"))
        .map_err(|error| error.to_string())?;
    let mut rows = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("face\t") else {
            continue;
        };
        let columns: Vec<&str> = rest.split('\t').collect();
        if columns.len() != 10 {
            return Err(format!("faces payload row with {} columns", columns.len()));
        }
        rows.push(FaceRow {
            source: columns[0].to_owned(),
            id: columns[1].to_owned(),
            parent: columns[2].to_owned(),
            owns_registry: columns[3] == "true",
            cfg: (!columns[4].is_empty()).then(|| unescape_column(columns[4])),
            line: columns[5].parse().unwrap_or_default(),
            written_as_literal: columns[6] == "literal",
            kind: unescape_column(columns[7]),
            preset: unescape_column(columns[8]),
            parts: unescape_column(columns[9]),
        });
    }
    Ok(rows)
}

/// The inverse of [`escape_column`].
/// [`escape_column`] 的逆。
fn unescape_column(text: &str) -> String {
    text.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

/// A tab or newline inside a column would move the columns, so both are escaped.
/// 列里的制表符或换行会挪动列，因此两者都转义。
fn escape_column(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

/// Where a tree's record lives: `<target>/xirang/records/<tree-fingerprint>/<namespace>/`.
/// 一棵树的记录所在：`<target>/xirang/records/<树指纹>/<namespace>/`。
///
/// The namespace is a **level of the key, not a field of the payload**, and it is the second thing this
/// module got wrong before its pins caught it: identities are a function of the namespace, so a record
/// published under one and read under another hands over rows whose every face has "moved" (measured:
/// `verify` with an `XIRANG_NAMESPACE` override read the default run's record and reported
/// `re-identified 1` for a tree nothing had touched). `graph.generation` stamps the same input for the
/// same reason (audit 2026-10-06, §M7.18).
/// 命名空间是**键的一层，而不是载荷里的一个字段**，而且它是本模块被自己的钉子抓住之前弄错的第二件事：身份是
/// 命名空间的函数，因此在一种命名空间下发布、在另一种下读取的记录，会把一批"每个面都搬过"的行交出去（实测：
/// 带 `XIRANG_NAMESPACE` 覆盖的 `verify` 读到了默认那次运行的记录，对一棵没人碰过的树报 `re-identified 1`）。
/// `graph.generation` 出于同一个理由盖的是同一个输入（审计 2026-10-06，§M7.18）。
///
/// `root` is the **package root directory** (the same argument `identity_cache::cache_directory`
/// takes), and the target resolution is that function's, so a build with `CARGO_TARGET_DIR` finds its
/// own records and no other build's.
/// `root` 是**包根目录**（与 `identity_cache::cache_directory` 收的是同一个参数），target 的解析就是那个函数
/// 的，因此带 `CARGO_TARGET_DIR` 的构建只会找到自己的记录。
/// 与发现缓存同一套 target 解析（`identity_cache::cache_directory`），因此带 `CARGO_TARGET_DIR` 的构建只会找到
/// 自己的记录。
pub(crate) fn directory(root: &Path, fingerprint: &str, namespace: &str) -> PathBuf {
    super::identity_cache::cache_directory(root)
        .parent()
        .map(|xirang| xirang.join("records"))
        .unwrap_or_else(|| PathBuf::from("xirang/records"))
        .join(fingerprint)
        .join(namespace)
}

/// The verified record for this tree, or `None` when there is none to trust.
/// 这棵树经过校验的记录；没有可信的记录时返回 `None`。
///
/// Every refusal answers `None`, and `None` means "run the full path": a record that does not carry this
/// schema, names another tree, misses a payload, or whose payload no longer has the byte count and
/// sha256 `record.tsv` recorded is not evidence of anything.
/// 每一次拒绝都答 `None`，而 `None` 意为"走全量路径"：不带本 schema 的、点名另一棵树的、缺载荷的、或载荷的字节数与
/// sha256 已与 `record.tsv` 记下的不符的记录，都不能作为任何事情的证据。
pub(crate) fn load(root: &Path, fingerprint: &str, namespace: &str) -> Option<PathBuf> {
    let directory = directory(root, fingerprint, namespace);
    let text = std::fs::read_to_string(directory.join("record.tsv")).ok()?;
    let mut lines = text.lines();
    if lines.next()? != format!("# schema\t{SCHEMA}") {
        return None;
    }
    if lines.next()? != format!("# tree\t{fingerprint}") {
        return None;
    }
    // The directory already separates namespaces; the line is the second half of the same check, for the
    // record that was copied out of its own place (the case the pin for this line carries).
    // 目录已经把命名空间分开了；这一行是同一个检查的另一半，针对被从原位复制走的记录（这条钉子的用例正是它）。
    if lines.next()? != format!("# namespace\t{namespace}") {
        return None;
    }
    let mut listed = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let mut fields = line.split('\t');
        let (Some(name), Some(bytes), Some(digest)) = (fields.next(), fields.next(), fields.next())
        else {
            return None;
        };
        let bytes: u64 = bytes.parse().ok()?;
        let content = std::fs::read(directory.join("products").join(name)).ok()?;
        if content.len() as u64 != bytes {
            return None;
        }
        if xirang_kernel::sha256_hex(&content) != digest {
            return None;
        }
        listed.push(name.to_owned());
    }
    // Every payload this schema promises has to be there: a record missing one is not a smaller record,
    // it is a record that would answer "this tree wrote no such file" ✗.
    // 本 schema 承诺的每一份载荷都必须在：缺一份的记录不是"更小的记录"，而是一份会答"这棵树没写过那个文件"的记录 ✗。
    for name in PAYLOADS {
        if !listed.iter().any(|listed| listed == name) {
            return None;
        }
    }
    Some(directory)
}

/// Copy the record's payloads into `out_dir`, byte for byte.
/// 把记录的载荷逐字节复制进 `out_dir`。
///
/// `write_if_changed` is what keeps a hit cheap: an out directory that already holds the same bytes is
/// left alone, so a rebuild does not touch its mtime and cargo does not treat the tree as dirtied.
/// `write_if_changed` 是让命中变便宜的那一半：已经持有同样字节的输出目录不被触碰，因此重建不会改它的 mtime，
/// cargo 也就不会把这棵树当成被动过。
pub(crate) fn copy_products(directory: &Path, out_dir: &Path) -> Result<(), String> {
    for name in PAYLOADS {
        let source = directory.join("products").join(name);
        let content = std::fs::read(&source)
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
        write_if_changed(&out_dir.join(name), &String::from_utf8_lossy(&content))?;
    }
    Ok(())
}

/// Publish this tree's record from the products a full run just wrote.
/// 用一次全量运行刚写出的产物发布这棵树的记录。
///
/// The payloads are copied first and `record.tsv` (the thing a reader trusts) is written last, which is
/// the same discipline the fingerprint line follows among the products themselves: a half-published
/// record cannot be read as a complete one.
/// 先复制载荷、最后写 `record.tsv`（读取方信任的那样东西），这与指纹行在产物之间遵循的是同一条纪律：只发布了一半
/// 的记录不可能被读成完整的一份。
pub(crate) fn publish(
    root: &Path,
    fingerprint: &str,
    namespace: &str,
    out_dir: &Path,
) -> Result<(), String> {
    let directory = directory(root, fingerprint, namespace);
    let products = directory.join("products");
    std::fs::create_dir_all(&products)
        .map_err(|error| format!("cannot create {}: {error}", products.display()))?;
    let mut record =
        format!("# schema\t{SCHEMA}\n# tree\t{fingerprint}\n# namespace\t{namespace}\n");
    for name in PAYLOADS {
        let content = std::fs::read(out_dir.join(name))
            .map_err(|error| format!("cannot read {}: {error}", out_dir.join(name).display()))?;
        write_if_changed(&products.join(name), &String::from_utf8_lossy(&content))?;
        record.push_str(&format!(
            "{name}\t{}\t{}\n",
            content.len(),
            xirang_kernel::sha256_hex(&content)
        ));
    }
    // A test-only stop between the payloads and the manifest. Without it the "written last" rule is only
    // *structural*: a pin that runs after two publishers sees a whole record either way, so flipping the
    // order would not turn anything red (measured — both concurrency pins stayed green under exactly that
    // flip ✗). This hook makes the window observable, and therefore makes the rule checkable.
    // 一个仅测试用的暂停点，落在载荷与清单之间。没有它，"最后写"就只是**结构上**的：在两个发布者跑完之后
    // 检查的钉子两种情况都会看到完整记录，因此把顺序翻过来不会让任何东西变红（实测——两颗并发钉子在**正是**
    // 那次翻转下都是绿的 ✗）。这个钩子让那个窗口可观察，于是这条规则可被检查。
    #[cfg(test)]
    if PUBLISH_PAUSED.with(std::cell::Cell::get) {
        return Ok(());
    }
    write_if_changed(&directory.join("record.tsv"), &record)
}

/// The file rows a published `file_manifest.tsv` carries, read back the way the writer wrote them.
/// 已发布的 `file_manifest.tsv` 携带的文件行，按写入方写出的样子读回。
///
/// A record hit does not re-derive these; it reads them from the copy the record handed it, which is
/// why the reader lives next to the writer's contract rather than in the consumer.
/// 记录命中时不再重新推导它们，而是从记录交过来的那份复制里读——这也是这个读取方住在写入方契约旁边、而不在消费者
/// 那里的原因。
pub(crate) fn read_file_rows(out_dir: &Path) -> Result<Vec<(String, String, String)>, String> {
    let path = out_dir.join("file_manifest.tsv");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(relative), Some(symbol), Some(handle)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        rows.push((relative.to_owned(), symbol.to_owned(), handle.to_owned()));
    }
    Ok(rows)
}

#[cfg(test)]
#[path = "discovery_record_tests.rs"]
mod discovery_record_tests;
