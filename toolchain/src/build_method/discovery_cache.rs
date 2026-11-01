//! Discovery/identity cache writing: the observation record the build reuses.
//! 发现/身份缓存写入：构建复用的那份观察记录。
//!
//! The cache is only an observation record: generated Rust still comes from the
//! current source tree, and a corrupt cache is rebuilt instead of trusted. The
//! active-face predicates that used to sit beside it moved to `scope.rs`, whose
//! question they answer (`NAM-04`); the atomic write helper stays here until the
//! cross-crate move gets a task of its own.
//! 缓存只是观察记录：生成的 Rust 仍来自当前源码树，损坏缓存会重建而不会被信任。过去与它放在一起的
//! 活跃注册面判定已搬去 `scope.rs`——它们回答的是作用域自己的问题（`NAM-04`）；原子写入辅助函数
//! 暂留此处，跨 crate 搬迁另开一条。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use super::discovery_node::{Node, relative_display};
use super::face_syntax_check::parsed_face;
use super::identity_cache::{source_unit_fingerprint, valid_cached_unit};
use super::node_id;
use super::registry_identity::{self, NodeId};
use super::registry_syntax::{FaceSyntax, ParentSyntax};

// Bump this whenever the identity input or generated-plan format changes.
// 身份输入或生成计划格式变化时必须递增，避免旧缓存混入新构建。
//
// This is not the identity schema: it versions the discovery cache's own layout,
// and the two change for different reasons.
// 这不是身份 schema：它标记发现缓存自身的版式，两者因不同的理由而变化。
pub(crate) const CACHE_SCHEMA: &str = "8";

/// Write a generated artifact only when its bytes changed — **without** the generation rule.
/// 仅在内容变化时写入生成产物——**不带**代次规则。
///
/// This is the writer for callers that carry no generation (`pipeline`'s file writes, the graph and
/// manifest writers); it keeps last-writer-wins, which [`write_if_changed_at`] is the answer to
/// (W7/A30, report §7e).
/// 这是给"没有代次可带"的调用方用的写入者（`pipeline` 的文件写入、图与清单的写入者）；它保持"最后写的人赢"，
/// 而 [`write_if_changed_at`] 正是针对这一点（W7/A30，报告 §7e）。
pub(crate) fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(());
    }
    publish(path, content)
}

#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
/// The tree's **structural generation**: the highest number under `.xirang/moves/`, or `0` for a
/// tree that has never been moved. A writer carries it so that a stale plan cannot overwrite a
/// fresher artifact (W7/A30; design in the delivery report §7e).
/// 这棵树的**结构代次**：`.xirang/moves/` 下最大的编号，从未搬动过的树为 `0`。写入者带着它，因此一份过期的
/// 计划无法覆盖更新的产物（W7/A30；设计见交付报告 §7e）。
pub(crate) fn generation(root: &Path) -> u64 {
    let directory = root
        .join(xirang_kernel::lexicon::XIRANG_DIR)
        .join(xirang_kernel::lexicon::MOVES_DIR);
    let Ok(entries) = fs::read_dir(&directory) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "generation_line.rs"]
mod generation_line;

#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
/// Write through the **generation** rule: identical bytes are a no-op, and an older writer refuses
/// rather than overwriting a fresher artifact (W7/A30).
/// 按**代次**规则写入：内容相同即空操作；较老的写入者**拒绝**，而不是覆盖更新的产物（W7/A30）。
///
/// The four judgements this is held to were written before the code (report §7e): a *newer* published
/// generation is refused with both numbers named; an *older* one is published; identical bytes at the
/// same generation publish nothing; the same generation twice is a no-op.
/// 这条实现要满足的四条判定先于代码写成（报告 §7e）：目标代次**更新** ⇒ 拒绝并报出两个编号；目标代次**更旧**
/// ⇒ 发布；**同一代次**下内容相同 ⇒ 什么都不发布；同一代次写两次 ⇒ 空操作。
///
/// **The generation is written into the artifact itself** — a comment line whose syntax is chosen by the
/// file's extension — so the two can never disagree: one `rename` publishes the bytes *and* the number that
/// names them. This is the maintainer's ruling of 2026-10-18 (option A1), taken on the window this file used
/// to describe instead of close: the body and a `<path>.generation` sidecar were two renames, so a crash
/// between them left a current body recorded at an older generation, and a stale writer then passed the
/// `published > generation` gate because that gate only knew what the sidecar said.
/// **代次写进产物本身**——一行注释，注释语法按扩展名选——因此两者永远不会互相矛盾：一次 `rename` 同时发布字节
/// 与命名它们的那个数字。这是维护者 2026-10-18 的裁定（选项 A1），针对的正是这份文件过去只**描述**而不关闭的那个
/// 窗口：正文与 `<path>.generation` 旁注是两次 rename，崩在中间会留下"正文是新的、记在旧代次上"，于是旧写入者
/// 得以通过 `published > generation` 这道闸，因为那道闸只知道旁注说了什么。
///
/// A **leftover sidecar** from the older scheme is read as a floor (an existing tree keeps its guard) and
/// deleted by the write that replaces it, so no number the artifact does not carry can keep guarding it.
/// 旧方案留下的**旁注**被当作下界读（已有树因此不丢守卫），并在替换它的那次写入里删掉，于是产物并不携带的编号
/// 不可能继续守着它。
pub(crate) fn write_if_changed_at(
    path: &Path,
    content: &str,
    generation: u64,
) -> Result<(), String> {
    let body = generation_line::compose(path, content, generation);
    if fs::read_to_string(path).ok().as_deref() == Some(body.as_str()) {
        return Ok(());
    }
    let published = generation_line::published(path);
    if published > generation {
        return Err(format!(
            "refusing {}: it was written by generation {published} and this write carries \
             generation {generation}; a stale plan does not overwrite a fresher artifact",
            path.display()
        ));
    }
    publish(path, &body)?;
    // The pair is one file now. A sidecar left by the older scheme is removed by the write that replaces it,
    // because keeping it would let a number nobody publishes go on guarding this artifact.
    // 现在这一对就是一个文件。旧方案留下的旁注在替换它的这次写入里删掉，因为留着它会让一个没人发布的数字继续守着
    // 这份产物。
    if let Err(error) = fs::remove_file(generation_line::sibling(path))
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!(
            "wrote {}, but the retired generation sidecar cannot be removed: {error}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
/// Write bytes that are **not ours to decorate**: a copy of a host's own file, or a manifest this action only edits.
/// 写那些**不该被我们装饰**的字节：宿主自己文件的副本，或本动作只做编辑的清单。
///
/// The distinction is the point of A1 rather than an exception to it: a generation line belongs on an artifact this
/// writer *authors*, because that is the artifact a stale plan could wrongly overwrite. A copy has no generation of its
/// own — its content is the host's — and decorating it would change bytes other tools compare ("the copy is the host's
/// bytes") and make the release shape differ from the tree it was copied out of.
/// 这个区分是 A1 的要点而不是它的例外：代次行属于这个写入方**创作**的产物，因为那才是过期计划可能错误覆盖的东西。
/// 副本没有自己的代次——它的内容是宿主的——装饰它就会改变别的工具要比较的字节（"副本就是宿主的字节"），并让发布形态
/// 与它被复制出来的那棵树不再一致。
pub(crate) fn write_verbatim_at(path: &Path, content: &str) -> Result<(), String> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(());
    }
    // A copy can land in a directory tree the caller has not created yet (`src/panel/frame/widget/widget.rs`), so the
    // parent is made here rather than at each call site.
    // 副本可能落进调用方还没创建的目录树（`src/panel/frame/widget/widget.rs`），因此父目录在这里建，而不是在每个
    // 调用点各建一次。
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    publish(path, content)
}

/// Put `content` at `path` through a unique sibling, so the target is never half-written.
/// 经唯一的同级文件把 `content` 放到 `path`，因此目标绝不会写了一半。
fn publish(path: &Path, content: &str) -> Result<(), String> {
    let temporary = temporary_sibling(path);
    fs::write(&temporary, content)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| {
        // The half-written sibling is this call's own mess; the target is untouched.
        // 半成品同级文件是本次调用自己的烂摊子；目标文件未被碰过。
        let _ = fs::remove_file(&temporary);
        format!("cannot publish {}: {error}", path.display())
    })
}

/// A unique name beside `path` that no other writer in this process shares.
/// 在 `path` 旁的一个本进程内不与他人共用的唯一名字。
fn temporary_sibling(path: &Path) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path.file_name().map_or_else(
        || "artifact".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!("{name}.tmp-{}-{sequence}", std::process::id()))
}

/// Keep a reusable discovery snapshot outside Cargo's ephemeral OUT_DIR.
/// 将发现结果保存在 Cargo 临时 OUT_DIR 之外，供后续构建复用。
///
/// This is only an observation cache. Generated Rust still comes from the
/// current source tree, and a corrupt cache is rebuilt instead of trusted.
/// 这只是观察缓存；生成的 Rust 仍来自当前源码树，损坏缓存会重建而不会被信任。
pub(crate) fn update_discovery_cache(
    manifest: &Path,
    src: &Path,
    nodes: &[Node],
    fingerprint: &str,
) -> String {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || manifest.join("target"),
            |path| {
                if path.is_absolute() {
                    path
                } else {
                    manifest.join(path)
                }
            },
        );
    let directory = target.join("xirang").join("cache");
    if fs::create_dir_all(&directory).is_err() {
        return "disabled".to_owned();
    }
    let units = directory.join("units");
    if fs::create_dir_all(&units).is_err() {
        return "disabled".to_owned();
    }
    let mut content =
        format!("# schema\t{CACHE_SCHEMA}\n# fingerprint\t{fingerprint}\n# path\tnode\tunit\n");
    // The proof store for this pass: `visit_discovery_rows` asks every node for its identity, and a
    // declaration-less file answers "no face" only after being parsed. With the store open, a proof a
    // previous process wrote answers that without a parse (report §7i.3).
    // 本趟的证明存储：`visit_discovery_rows` 会问每个节点的身份，而没有声明的文件要解析完才知道"没有面"。
    // 存储打开后，上一个进程写下的证明可以不解析就回答这件事（报告 §7i.3）。
    let _ordinary = crate::source_cache::ordinary_store::Scope::open(&directory.join("ordinary"));
    let mut rows: Vec<(String, PathBuf, NodeId)> = Vec::new();
    visit_discovery_rows(src, nodes, &mut rows);
    rows.sort();
    rows.dedup();
    // One memo for this pass: every path-named parent is read and parsed once, however many faces name it.
    // 本趟共用一份备忘：按路径点名的父级只读解析一次，无论有多少个面点名它。
    let _parents = ParentLookupScope::open();
    // The same pass also hands the facts it reads to the passes after it (`stem 2`).
    // 同一趟还把它读到的事实交给它之后的几趟（`stem 2`）。
    let _facts = FaceFactsScope::open();
    let mut all_units_hit = true;
    let mut unit_hits = 0_usize;
    let mut unit_misses = 0_usize;
    for (path, source_path, id) in rows {
        let unit_fingerprint = source_unit_fingerprint(&source_path);
        let unit_path = units.join(format!("{unit_fingerprint}.tsv"));
        let cached = fs::read_to_string(&unit_path).ok();
        let unit_hit = cached
            .as_deref()
            .is_some_and(|content| valid_cached_unit(content, &path, id));
        if unit_hit && let Some(content) = cached.as_deref() {
            remember_unit(&path, content);
        }
        if !unit_hit {
            // Parse only a new or corrupt unit. A valid fingerprinted unit is
            // consumed as-is by registration_phase::aggregate.
            // 只有新 unit 或损坏 unit 才解析；有效 fingerprint unit 直接由
            // registration_phase::aggregate 消费。
            let source = fs::read_to_string(&source_path).ok();
            let face = source
                .as_deref()
                .and_then(|source| parsed_face(source, &path));
            // `path` first, then `string`: a declaration that names its kind as a bare type (which is
            // what the literal spelling does when the reader synthesizes it) is not a path the reader
            // accepts, and the kind is the plain name either way.
            // 先问 `path`、再问 `string`：把 kind 写成裸类型的声明（字面量拼法被读者合成出来时就是这样）不是
            // 读取器接受的路径，而两种情况下的 kind 都是那个朴素的名字。
            let kind = face
                .as_ref()
                .and_then(|face| face.path("kind").or_else(|| face.string("kind")))
                .unwrap_or_default();
            let mut unit_content = format!(
                "# schema\t{CACHE_SCHEMA}\n# fingerprint\t{unit_fingerprint}\npath\t{path}\nnode\t{id}\nkind\t{kind}\n"
            );
            if let Some(face) = &face {
                if let Some(handle) = face.path("handle") {
                    writeln!(unit_content, "handle\t{handle}").unwrap();
                }
                let parent_id = cached_parent_id(src, face);
                if let Some(parent_id) = parent_id {
                    writeln!(unit_content, "parent\t{parent_id}").unwrap();
                }
                // The declaration's own fields, verbatim, with the line each was written on
                // (`stem 2`): every consumer the census lists asks for a **field** (`kind`, `parent`,
                // `needs_registry`, `plugin`, `stable_name`, `registry_rule`, …) and a few ask for that
                // field's line. Persisting them here is what lets a record hit answer those questions
                // without re-parsing the face — and the cost is paid once per *miss*, on bytes already
                // in hand.
                // 声明自己的字段、逐字保留，并带上每个字段写在**哪一行**（`stem 2`）：普查列出的每个消费者问的都是
                // **字段**（`kind`、`parent`、`needs_registry`、`plugin`、`stable_name`、`registry_rule` …），
                // 少数还问那个字段的行号。把它们持久化在这里，正是让记录命中时不必重解析就能回答这些问题的东西——
                // 而代价只在**未命中**时付一次，用的还是手里已有的字节。
                // `cfg` lives outside the ordered list (that list is the macro front end's accepted
                // order, not this store's schema) and the projection needs it: a record is **tree**-level
                // while a `cfg` is evaluated per **package**, so the hit path carries the text and each
                // package decides. The line column holds the **declaration's** line rather than the
                // field's, because the only diagnostic this path can produce points there.
                // `cfg` 住在有序表之外（那张表是宏前端的**接受顺序**，不是这个存储的模式），而投影需要它：
                // 记录是**树级**的，而 `cfg` 按**包**求值，因此命中路径携带原文、由每个包自己判定。行号那一列
                // 装的是**声明**所在的行而不是字段的行，因为这条路径唯一会产生的诊断指向那里。
                // Which spelling declared this face (H/A2e). It is not a face field — the field order
                // is the macro front end's accepted order, not this store's schema — and the renderer
                // is its consumer: a face written as a `Registration` literal has no module-level
                // `NODE_ID`, so its identity assertion must carry the id literally.
                // 本面由哪种拼法声明（H/A2e）。它不是面字段——那张字段顺序是宏前端的**接受顺序**，不是这个
                // 存储的模式——而它的消费者是渲染器：写成 `Registration` 字面量的面没有模块级 `NODE_ID`，
                // 因此它的身份断言必须直接带字面 id。
                writeln!(
                    unit_content,
                    "form\t{}",
                    if face.is_struct_literal() {
                        "struct-literal"
                    } else {
                        "macro"
                    }
                )
                .unwrap();
                if let Some(cfg) = face.cfg() {
                    writeln!(
                        unit_content,
                        "field\tcfg\t{}\t{}",
                        face.location.line,
                        escape_field(cfg)
                    )
                    .unwrap();
                }
                for name in xirang_kernel::declaration::FACE_FIELD_ORDER {
                    let Some(raw) = face.field(name) else {
                        continue;
                    };
                    let line = face
                        .field_location(name)
                        .map_or(face.location.line, |location| location.line);
                    writeln!(
                        unit_content,
                        "field\t{name}\t{line}\t{}",
                        escape_field(&raw)
                    )
                    .unwrap();
                    // The **decoded** forms, produced by calling the field readers themselves — not by
                    // decoding the raw text here. That distinction is the whole point: `string_list`
                    // *derives* two of its names (`handle_traits`/`part_traits` from the contracts beside
                    // them), so a second decoder in this file would answer differently from the one the
                    // build uses, and the difference would surface as a face that lost a label on a record
                    // hit. A missing row is the accessor's own `None`: the field is absent, or its shape is
                    // one the reader refuses.
                    // **解码后**的形态，由**调用字段读取器本身**产生——不是在这里解码原文。这个区别正是要点：
                    // `string_list` 有两个名字是**推导**出来的（`handle_traits`/`part_traits` 来自它们旁边的
                    // contracts），因此本文件里再写一份解码器会与构建用的那一份给出不同答案，而差别会表现为"记录命中
                    // 时某个面少了一个标签"。缺行就是访问器自己的 `None`：字段不存在，或形状不被它接受。
                    let mut decoded = |kind: &str, value: &str| {
                        writeln!(
                            unit_content,
                            "value\t{name}\t{kind}\t{}",
                            escape_field(value)
                        )
                        .unwrap();
                    };
                    if let Some(value) = face.string(name) {
                        decoded("string", &value);
                    }
                    if let Some(value) = face.boolean(name) {
                        decoded("bool", if value { "true" } else { "false" });
                    }
                    if let Some(value) = face.path(name) {
                        decoded("path", &value);
                    }
                    for (index, item) in face
                        .string_list(name)
                        .unwrap_or_default()
                        .iter()
                        .enumerate()
                    {
                        writeln!(
                            unit_content,
                            "value\t{name}\tlist\t{index}\t{}",
                            escape_field(item)
                        )
                        .unwrap();
                    }
                }
                let line = face
                    .field_location("requires")
                    .map_or(face.location.line, |location| location.line);
                writeln!(unit_content, "line\t{line}").unwrap();
                for (capability, provider) in face.requirements("requires").unwrap_or_default() {
                    writeln!(unit_content, "requires\t{capability}\t{provider}").unwrap();
                }
                for provide in face.string_list("provides").unwrap_or_default() {
                    writeln!(unit_content, "provides\t{provide}").unwrap();
                }
                for name in ["parts", "exports", "handle_traits", "part_traits"] {
                    if let Some(value) = face.field(name) {
                        writeln!(unit_content, "contract\t{name}\t{value}").unwrap();
                    }
                }
            }
            all_units_hit = false;
            unit_misses += 1;
            remember_unit(&path, &unit_content);
            if fs::write(&unit_path, unit_content).is_err() {
                return "disabled".to_owned();
            }
        } else {
            unit_hits += 1;
        }
        writeln!(content, "{path}\t{id}\t{unit_fingerprint}").unwrap();
    }
    let path = directory.join(format!("discovery-{fingerprint}.tsv"));
    if fs::read_to_string(&path).ok().as_deref() == Some(content.as_str()) {
        if all_units_hit {
            format!("hit (units={unit_hits})")
        } else {
            format!("miss (hits={unit_hits}, misses={unit_misses})")
        }
    } else if fs::write(path, content).is_ok() {
        format!("miss (hits={unit_hits}, misses={unit_misses}, discovery=changed)")
    } else {
        "disabled".to_owned()
    }
}

thread_local! {
    /// The parent identities **this pass** has already resolved, keyed by the parent file's path.
    /// 本趟已经解析过的父级身份，按父文件路径索引。
    ///
    /// Per pass, not per process: a pass reads a tree that is not changing under it, which is the only
    /// reason reuse is sound here. The scope below is what makes "per pass" true rather than aspirational
    /// — it is opened at the top of a pass and dropped at its end, so nothing survives into the next one.
    /// 按**趟**而不是按进程：一趟读的是一棵在它脚下不变的树，这是复用在这里成立的唯一理由。下面那个作用域
    /// 让"按趟"成为事实而不是愿望——它在趟开始时打开、结束时丢弃，因此没有任何东西留到下一趟。
    static PARENT_LOOKUPS: RefCell<Option<BTreeMap<PathBuf, Option<NodeId>>>> =
        const { RefCell::new(None) };
}

/// Opens the per-pass parent memo for as long as it lives.
/// 在它存续期间打开"按趟"的父级备忘。
///
/// Measured cost of not having it (W7/A33, 2026-10-13): on a 10k-face tree where every leaf names its
/// parent by path, the declared-parent leg was 21 522 ms cold against 15 239 ms for the same tree with
/// parentless leaves — 6 283 ms, **29% of the pass**, spent re-reading and re-parsing one small parent
/// file once per leaf. Resolution is per *unit*, so the number of distinct parents does not enter it.
/// 没有它的实测代价（W7/A33，2026-10-13）：在一棵 10k 面、每个叶子都按路径点名父级的树上，点名那条腿冷
/// 21 522 ms，而同一棵树叶子不点名父级是 15 239 ms——6 283 ms、**占这一趟的 29%**，花在每个叶子各重读重解析
/// 一次那个很小的父文件上。解析是按**单元**做的，因此"有多少个不同的父级"不进这个数。
pub(crate) struct ParentLookupScope {
    previous: Option<BTreeMap<PathBuf, Option<NodeId>>>,
}

impl ParentLookupScope {
    /// Open the memo for one pass, keeping whatever an outer pass had.
    /// 为一趟打开备忘，并保留外层那一趟原有的内容。
    ///
    /// Nested scopes are not a hypothesis to be avoided but a thing that happens (one pass calling
    /// another pass): the guard **restores** the previous memo rather than clearing it, so an inner pass
    /// cannot silently empty the outer one's. Same shape as `run_as_package`'s guard, for the same
    /// reason — a guard that puts back what it found is composable, one that writes a constant is not.
    /// 嵌套作用域不是要回避的假设而是会发生的事（一趟调用另一趟）：这个守卫**恢复**上一份备忘而不是清空它，
    /// 因此内层那趟不会静默地清空外层的。与 `run_as_package` 的守卫同形、同一个理由——把找到的东西放回去的
    /// 守卫可以组合，写死一个常量的不可以。
    pub(crate) fn open() -> Self {
        let previous = PARENT_LOOKUPS.with(|slot| slot.borrow_mut().replace(BTreeMap::new()));
        ParentLookupScope { previous }
    }
}

impl Drop for ParentLookupScope {
    fn drop(&mut self) {
        PARENT_LOOKUPS.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

thread_local! {
    /// The identities a Rust declaration may name through the generated header, keyed by the
    /// constant's identifier. The pipeline fills this before the static plan: a Rust face whose
    /// parent is declared in C++ names `crate::xirang_external::<ident>`, and that parent's identity
    /// is decided by the declaration text alone.
    /// Rust 声明经生成头点名的那些身份，键是常量标识符。管道在静态计划之前填好它：一个父面声明在
    /// C++ 里的 Rust 面写的是 `crate::xirang_external::<ident>`，而那个父的身份只由声明文本决定。
    static NATIVE_PARENTS: std::cell::RefCell<Option<std::collections::BTreeMap<String, NodeId>>> =
        const { std::cell::RefCell::new(None) };
}

/// Run `body` with the declared faces' identities visible to the static plan, and put the previous
/// map back afterwards (tests run in threads and builds can nest).
/// 让静态计划看得见声明面的身份，跑完把之前的表放回去（测试跑在线程里，构建也可能嵌套）。
pub(crate) fn with_native_parents<T>(
    parents: &std::collections::BTreeMap<String, NodeId>,
    body: impl FnOnce() -> T,
) -> T {
    let previous = NATIVE_PARENTS.with(|slot| slot.replace(Some(parents.clone())));
    let answer = body();
    NATIVE_PARENTS.with(|slot| *slot.borrow_mut() = previous);
    answer
}

/// The identity of the face this declaration names as its parent.
/// 这条声明点名的父级身份。
///
/// A path-named parent is read and parsed **once per pass** when a [`ParentLookupScope`] is open, and
/// once per call otherwise: the same function, with the only difference being whether somebody has
/// promised that the tree is not moving. Callers outside a pass (the one-off queries) keep today's
/// behaviour exactly.
/// 当一个 [`ParentLookupScope`] 打开时，按路径点名的父级**每趟只读解析一次**，否则每次调用一次：同一个
/// 函数，唯一的区别是"有没有人保证这棵树不动"。趟之外的调用方（一次性查询）行为与今天完全一致。
pub(crate) fn cached_parent_id(src: &Path, face: &FaceSyntax) -> Option<NodeId> {
    match face.parent()? {
        ParentSyntax::Root => Some(registry_identity::package_root_node_id()),
        ParentSyntax::FromPath { source, kind } => {
            Some(registry_identity::package_node_id(&source, &kind))
        }
        ParentSyntax::NodePath(module) => {
            let module = module.strip_prefix("crate::")?;
            // 一个父面声明在别的语言里时，它没有模块文件可读：名字就是生成头里的那个常量。
            // A parent declared in another language has no module file to read: the name is the
            // constant the generated header carries.
            if let Some(ident) = module.strip_prefix("xirang_external::") {
                return NATIVE_PARENTS.with(|slot| {
                    slot.borrow()
                        .as_ref()
                        .and_then(|parents| parents.get(ident).copied())
                });
            }
            // The module chain is the discovery tree's own address; resolving it through that tree
            // is what makes a parent findable under a custom `[lib] path` (whose scan root is not
            // the identity base) and for an explicitly placed parent (whose chain is its stem).
            // 模块链是发现树自己的地址；经那棵树解析它，才让自定义 `[lib] path`（扫描根不是身份基准）
            // 下的父级、以及显式落点的父级（链就是它的 stem）都能被找到。
            let parent_path = match super::discovery_node::module_file(module) {
                Some(path) => path,
                None => {
                    let mut relative_path = module.split("::").collect::<PathBuf>();
                    let name = relative_path.file_name()?.to_owned();
                    relative_path.push(format!("{}.rs", name.to_string_lossy()));
                    src.join(&relative_path)
                }
            };
            let answered = PARENT_LOOKUPS.with(|slot| {
                let memo = slot.borrow();
                memo.as_ref()
                    .and_then(|memo| memo.get(&parent_path).copied())
            });
            if let Some(answered) = answered {
                return answered;
            }
            let resolved = resolve_parent_file(src, &parent_path);
            PARENT_LOOKUPS.with(|slot| {
                if let Some(memo) = slot.borrow_mut().as_mut() {
                    memo.insert(parent_path, resolved);
                }
            });
            resolved
        }
    }
}

/// Read and parse one parent file, answering the identity its declaration carries.
/// 读并解析一个父文件，答出它声明里的身份。
fn resolve_parent_file(src: &Path, parent_path: &Path) -> Option<NodeId> {
    // The identity input is the record spelling the discovery pass recorded; `relative_display` is
    // only the fallback for a file no discovery pass placed (a unit fixture calling this directly).
    // 身份输入是发现那一趟记下的记录拼写；`relative_display` 只是"没有发现趟安放过这个文件"时的兜底
    // （直接调用本函数的单元夹具）。
    let relative = super::discovery_node::remembered_spelling(parent_path)
        .unwrap_or_else(|| relative_display(src, parent_path));
    let parent_source = fs::read_to_string(parent_path).ok()?;
    let parent = parsed_face(&parent_source, &relative)?;
    let kind = parent.path("kind")?;
    Some(registry_identity::package_node_id(&relative, &kind))
}

/// Escape one declaration value so it survives a tab-separated row.
/// 转义一个声明取值，让它能活着穿过制表符分隔的一行。
///
/// Declaration values are arbitrary source text, and a shape field may well contain a tab or a newline
/// inside a string literal. The three escapes are the whole alphabet: a backslash, a tab, a newline.
/// 声明取值是任意源码文本，形态字段的值很可能在字符串字面量里带着制表符或换行。三个转义就是全部字母表：反斜杠、
/// 制表符、换行。
pub(crate) fn escape_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// The inverse of [`escape_field`], for the readers.
/// [`escape_field`] 的逆运算，供读取方使用。
pub(crate) fn unescape_field(value: &str) -> String {
    let mut plain = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            plain.push(character);
            continue;
        }
        match characters.next() {
            Some('t') => plain.push('\t'),
            Some('n') => plain.push('\n'),
            Some('r') => plain.push('\r'),
            Some('\\') => plain.push('\\'),
            Some(other) => plain.push(other),
            None => plain.push('\\'),
        }
    }
    plain
}

thread_local! {
    /// The per-face facts **this pass** has in hand, keyed by relative path.
    /// 本趟手里已有的逐面事实，按相对路径索引。
    ///
    /// The units *are* the per-face facts store — they already carry `kind`, `parent`, `handle`,
    /// `line`, `requires`, `provides` and, since `stem 2`, every declaration field verbatim with its
    /// line. This is the layer that lets the consumers the census lists read them instead of parsing the
    /// face again: the discovery pass has each unit's text in hand on **both** of its branches (the
    /// cached text on a hit, the text it just wrote on a miss), so remembering it costs nothing and the
    /// passes after it can stop re-parsing what it already read.
    /// unit **就是**逐面事实存储——它们已经带着 `kind`、`parent`、`handle`、`line`、`requires`、
    /// `provides`，而自从 `stem 2` 起还带着每个声明字段的逐字取值与行号。这一层让普查列出的消费者读它们、
    /// 而不是再解析一次面：发现那一趟在**两个分支**上都拿着单元文本（命中有缓存文本、未命中有刚写出的文本），
    /// 因此记住它不花任何代价，而它之后的几趟就不必重解析已经读过的东西。
    static FACE_FACTS: RefCell<Vec<BTreeMap<String, std::sync::Arc<UnitFacts>>>> =
        const { RefCell::new(Vec::new()) };
}

/// Opens the per-pass face-facts store for as long as it lives, restoring whatever it found.
/// 在它存续期间打开"按趟"的逐面事实存储，并恢复它找到的东西。
///
/// Nesting-safe for the same reason the parent memo is: passes call passes here.
/// 可嵌套，理由与父级备忘相同：这里一趟会调用另一趟。
pub(crate) struct FaceFactsScope;

impl FaceFactsScope {
    /// Open the store for one pass.
    /// 为一趟打开存储。
    ///
    /// A **stack**, and `remember_unit` writes into every layer — because the pass that opens a store
    /// around a whole run (the build) must still see what a nested pass learned. Measured failure this
    /// exists against: with a single slot, the nested pass's `open` *replaced* the outer map, every fact
    /// landed in the inner one, and the outer — the only one the consumers could see — stayed empty.
    /// The symptom was the counter, not the clock: `facts=0` on a 1,000-face build (`stem 2`).
    /// 是**栈**，而 `remember_unit` 写进每一层——因为"在整次运行外面开存储"的那一趟（构建）必须仍然看得见内层
    /// 学到的东西。它针对的实测失败：单槽位时，内层那一趟的 `open` **换掉**了外层那张表，所有事实落进内层，而外层
    /// ——消费者唯一能看见的那张——始终是空的。症状出现在**计数器**上而不是时钟上：1,000 面的构建里 `facts=0`
    /// （`stem 2`）。
    pub(crate) fn open() -> Self {
        FACE_FACTS.with(|slot| slot.borrow_mut().push(BTreeMap::new()));
        FaceFactsScope
    }
}

impl Drop for FaceFactsScope {
    fn drop(&mut self) {
        FACE_FACTS.with(|slot| {
            slot.borrow_mut().pop();
        });
    }
}

/// Remember what one unit says about one face, for the rest of this pass.
/// 记住一份单元关于一个面说了什么，供这一趟余下的部分使用。
///
/// Text that is not a unit is ignored: `read_unit` answers `None` and nothing is stored, which is the
/// direction that keeps a malformed cache from being read as facts.
/// 不是单元的文本被忽略：`read_unit` 答 `None`、什么都不存——这个方向保证畸形的缓存不会被读成事实。
pub(crate) fn remember_unit(relative: &str, content: &str) {
    let Some(facts) = read_unit(content) else {
        return;
    };
    let facts = std::sync::Arc::new(facts);
    FACE_FACTS.with(|slot| {
        for store in slot.borrow_mut().iter_mut() {
            store.insert(relative.to_owned(), std::sync::Arc::clone(&facts));
        }
    });
}

/// What this pass knows about one face, if it knows anything yet.
/// 这一趟关于某个面已经知道的东西（若已知道）。
/// How many lookups this process answered from the facts instead of parsing the source.
/// 本进程有多少次查找是由事实作答、而不是去解析源码。
///
/// The other half of the mechanism's dependent variable: `face_parses` says how much parsing is left,
/// this says how much was avoided. Both are **counted** rather than timed, because a duration on a
/// 1,000-face tree cannot resolve the difference (`stem 2`).
/// 机制因变量的另一半：`face_parses` 说还剩多少解析，这个说避掉了多少。两者都**计数**而不是计时，因为在 1,000 面
/// 的树上时长的分辨率不够（`stem 2`）。
pub(crate) fn facts_answered() -> u64 {
    FACTS_ANSWERED.load(std::sync::atomic::Ordering::Relaxed)
}

static FACTS_ANSWERED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn facts_of(relative: &str) -> Option<std::sync::Arc<UnitFacts>> {
    FACE_FACTS.with(|slot| {
        slot.borrow()
            .last()
            .and_then(|store| store.get(relative).cloned())
    })
}

/// One decoded **string** field and the line it was written on: the facts this pass already read, or
/// the face parsed from the source.
/// 一个解码后的**字符串**字段与它写在哪一行：本趟已读过的事实，或从源码解析出的面。
///
/// The two branches are not interchangeable spellings of one lookup — they are two **sources**, and the
/// rule between them is the whole consumer switch: **when the facts are there, they answer, including
/// "the reader said `None`"**. Falling back to parsing after a present-but-silent fact row would put the
/// parse back on the hot path and, worse, let the record-hit path disagree with the full path about a
/// face (the unit's rows were produced by the same reader, so a missing row *is* the reader's `None`).
/// 两个分支不是同一次查找的两种写法——它们是两个**来源**，而它们之间的规则就是整个消费者切换：**事实在，就由事实
/// 作答，包括"读取器说了 `None`"**。若在"事实存在但没有那一行"时回退去解析，就会把解析放回热路径；更糟的是，
/// 记录命中路径会与全量路径对同一个面给出不同答案（unit 的行由同一个读取器产生，因此缺行**就是**读取器的 `None`）。
pub(crate) fn face_string_field(
    file: &Path,
    relative: &str,
    name: &str,
) -> Option<(String, usize)> {
    if let Some(facts) = facts_of(relative) {
        FACTS_ANSWERED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let value = facts
            .values
            .get(&(name.to_owned(), "string".to_owned()))?
            .clone();
        let line = facts.fields.get(name).map_or(0, |(line, _)| *line);
        return usize::try_from(line).ok().map(|line| (value, line));
    }
    let source = fs::read_to_string(file).ok()?;
    let face = parsed_face(&source, relative)?;
    let value = face.string(name)?;
    let line = face
        .field_location(name)
        .map_or(face.location.line, |location| location.line);
    Some((value, line))
}

/// One decoded **boolean** field, from the facts or from the source.
/// 一个解码后的**布尔**字段，取自事实或取自源码。
pub(crate) fn face_bool_field(file: &Path, relative: &str, name: &str) -> Option<bool> {
    if let Some(facts) = facts_of(relative) {
        FACTS_ANSWERED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return match facts
            .values
            .get(&(name.to_owned(), "bool".to_owned()))
            .map(String::as_str)
        {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        };
    }
    let source = fs::read_to_string(file).ok()?;
    parsed_face(&source, relative)?.boolean(name)
}

/// One field's **verbatim text**, from the facts or from the source.
/// 一个字段的**逐字原文**，取自事实或取自源码。
pub(crate) fn face_raw_field(file: &Path, relative: &str, name: &str) -> Option<(String, usize)> {
    if let Some(facts) = facts_of(relative) {
        FACTS_ANSWERED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (line, raw) = facts.fields.get(name)?;
        return Some((raw.clone(), usize::try_from(*line).ok()?));
    }
    let source = fs::read_to_string(file).ok()?;
    let face = parsed_face(&source, relative)?;
    let raw = face.field(name)?;
    let line = face
        .field_location(name)
        .map_or(face.location.line, |location| location.line);
    Some((raw, line))
}

/// Every fact one cached unit carries, read from its text.
/// 一份缓存单元携带的全部事实，从它的文本读出。
///
/// The consumers the census lists ask for **fields**, so this is the seam they will read on a record
/// hit: one parse of the unit text, no source, no face parser. `None` means the text is not shaped like
/// a unit at all (no `path` row), which every caller treats as "parse the source instead".
/// 普查列出的消费者问的是**字段**，因此这就是记录命中时它们要读的那道缝：解析一次单元文本，不要源码、不要面解析器。
/// `None` 表示这段文本根本不像一份单元（没有 `path` 行），所有调用方都把它当作"去解析源码"。
pub(crate) struct UnitFacts {
    pub(crate) relative: String,
    /// Whether the declaration was written as a `Registration` literal rather than a macro block.
    /// 该声明写成 `Registration` 字面量还是宏块。
    pub(crate) written_as_literal: bool,
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) parent: Option<String>,
    pub(crate) handle: Option<String>,
    pub(crate) line: Option<u64>,
    /// Field name ⇒ (line, verbatim text).
    /// 字段名 ⇒（行号，逐字文本）。
    pub(crate) fields: BTreeMap<String, (u64, String)>,
    /// (field name, reader kind) ⇒ decoded value — `string`, `bool` or `path`. A missing entry is the
    /// field reader's own `None`.
    /// （字段名，读取器种类）⇒ 解码后的取值——`string`、`bool` 或 `path`。缺项就是字段读取器自己的 `None`。
    pub(crate) values: BTreeMap<(String, String), String>,
    /// (field name, reader kind) ⇒ ordered list items, keyed the same way.
    /// （字段名，读取器种类）⇒ 有序的列表项，键的写法相同。
    pub(crate) lists: BTreeMap<(String, String), Vec<String>>,
    pub(crate) requires: Vec<(String, String)>,
    pub(crate) provides: Vec<String>,
    pub(crate) contract: Vec<(String, String)>,
}

pub(crate) fn read_unit(content: &str) -> Option<UnitFacts> {
    let row = |name: &str| {
        content
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}\t")))
    };
    let mut facts = UnitFacts {
        relative: row("path")?.to_owned(),
        written_as_literal: false,
        id: row("node").unwrap_or_default().to_owned(),
        kind: row("kind").unwrap_or_default().to_owned(),
        parent: row("parent").map(str::to_owned),
        handle: row("handle").map(str::to_owned),
        line: row("line").and_then(|value| value.parse().ok()),
        fields: BTreeMap::new(),
        values: BTreeMap::new(),
        lists: BTreeMap::new(),
        requires: Vec::new(),
        provides: Vec::new(),
        contract: Vec::new(),
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("form\t") {
            facts.written_as_literal = rest.trim() == "struct-literal";
            continue;
        }
        if let Some(rest) = line.strip_prefix("field\t") {
            let mut fields = rest.splitn(3, '\t');
            let (Some(name), Some(number), Some(value)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let Ok(number) = number.parse::<u64>() else {
                continue;
            };
            facts
                .fields
                .insert(name.to_owned(), (number, unescape_field(value)));
        } else if let Some(rest) = line.strip_prefix("value\t") {
            // `value\t<name>\t<string|bool|path>\t<value>` or
            // `value\t<name>\tlist\t<index>\t<item>` — split into four so the list form's index and item
            // stay distinguishable from a value that happens to contain a tab (unlikely, but the escape
            // already handles that; the index does not).
            // 两种写法：`value\t名\t<string|bool|path>\t值>`，或 `value\t名\tlist\t序号\t项`——分成四段，
            // 好让列表形态的序号与项区别于"值里恰好带制表符"（不太可能，但转义已经管住了那种情况；序号不管）。
            let mut parts = rest.splitn(4, '\t');
            let (Some(name), Some(kind), Some(third)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            match parts.next() {
                Some(item) => {
                    let Ok(index) = third.parse::<usize>() else {
                        continue;
                    };
                    let entry = facts
                        .lists
                        .entry((name.to_owned(), kind.to_owned()))
                        .or_default();
                    if entry.len() == index {
                        entry.push(unescape_field(item));
                    }
                }
                None => {
                    facts
                        .values
                        .insert((name.to_owned(), kind.to_owned()), unescape_field(third));
                }
            }
        } else if let Some(rest) = line.strip_prefix("requires\t") {
            if let Some((capability, provider)) = rest.split_once('\t') {
                facts
                    .requires
                    .push((capability.to_owned(), provider.to_owned()));
            }
        } else if let Some(provide) = line.strip_prefix("provides\t") {
            facts.provides.push(provide.to_owned());
        } else if let Some(rest) = line.strip_prefix("contract\t")
            && let Some((name, value)) = rest.split_once('\t')
        {
            facts.contract.push((name.to_owned(), value.to_owned()));
        }
    }
    Some(facts)
}

fn visit_discovery_rows(src: &Path, nodes: &[Node], rows: &mut Vec<(String, PathBuf, NodeId)>) {
    for node in nodes {
        if let Some(file) = &node.file
            && let Some(id) = node_id(src, node)
        {
            // The row is keyed by the **record spelling** the discovery pass recorded, because the
            // plan and every unit reader ask for facts by that key. The materialized path travels
            // beside it so nothing has to re-join a spelling to a root it may not belong to.
            // 行以发现那一趟记下的**记录拼写**为键，因为计划与每个单元读取方都按那个键问事实。物化路径
            // 随行携带，免得任何东西把拼写拼回一个它未必属于的根。
            rows.push((
                super::discovery_node::source_spelling(src, file),
                file.clone(),
                id,
            ));
        }
        visit_discovery_rows(src, &node.children, rows);
    }
}

#[cfg(all(test, any(feature = "cli", feature = "studio", feature = "mcp")))]
mod discovery_cache_tests;

#[cfg(all(test, any(feature = "cli", feature = "studio", feature = "mcp")))]
#[path = "discovery_cache_fault_tests.rs"]
mod discovery_cache_fault_tests;
