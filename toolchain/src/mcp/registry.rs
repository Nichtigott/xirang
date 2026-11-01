//! The registration tree this package declares, as the build sees it.
//! 本包声明的注册树，按构建所见。
//!
//! online: a published record of the whole tree cannot carry the faces a fragment crate registers
//! for itself, so when this package declares native sources the answer is derived online and says
//! why — the build's record is complete for the crate that published it and silent about the crate
//! next door.
//! online：整棵树的发布记录带不了碎片 crate 自己登记的面，因此当本包声明了本地源码时，答案在线上推导
//! 并说出原因——构建的记录对它自己那个 crate 是完整的，对隔壁那个 crate 则一言不发。
//!
//! The bridge used to answer registry questions by re-deriving them: an agent
//! grepped for macro names and reconstructed the tree itself, which is the drift
//! the registry exists to remove. The rows here now come from the build's **own
//! published records** (`<package>/target/xirang/out`, read through
//! `build_method`'s readers — `published.rs`), so a workspace-rooted answer costs
//! the records instead of every member's source walk; the derivation
//! (`crate::build_method::face_views`) is the fallback for a member that published
//! nothing, and every answer says which of the two it used.
//! 本桥过去靠重新推导来回答注册问题：代理 grep 宏名，自己重建那棵树——这正是注册树要消除
//! 的漂移。这里的行现在来自构建**自己已发布的记录**（`<package>/target/xirang/out`，经
//! `build_method` 的读取器读取——`published.rs`），因此工作区根上的一次答案花的是记录而不是
//! 每个成员的源码遍历；推导（`crate::build_method::face_views`）是给什么都没发布的成员的回落，
//! 而每一份答案都会说出自己用的是哪一种。
//!
//! What the published rows carry is `node`, `source` and the symbol release
//! pruning tracks, plus the scope verdict — the record's own columns. `path`,
//! `kind`, `registry_name` and `parent` are *derived* facts and are not in it, so
//! an answer built from the record says so and names `xirang.explain` (the one
//! report that derives a per-face projection) as where to read them.
//! 已发布的行携带的是 `node`、`source` 与发布剪枝跟踪的符号，外加作用域结论——也就是记录自己的
//! 列。`path`、`kind`、`registry_name` 与 `parent` 是**推导**出来的事实、不在其中，因此由记录
//! 构成的答案会说出来，并点名 `xirang.explain`（唯一推导逐面投影的报告）为读取它们的地方。

use std::path::Path;

use crate::build_method::{FaceView, PruningRow};
use serde_json::Value;
use xirang_kernel::lexicon;

use crate::mcp::published::{FACES_UNKNOWN, PublishedTree};
use crate::mcp::workspace::{self, Member, Scope, Tree};

/// Report every registration face declared under the package root `root`.
/// 报告 `root` 这个包根下声明的每个注册面。
///
/// `root` is a package root, because that is what an identity needs: the faces'
/// identities are hashed over paths relative to the package's `src/`, and their
/// namespace is the package's own name. A **virtual manifest** names no package,
/// so it is answered as the workspace it is: one section per member, each under its own
/// name, with every member's status in the census above them.
/// `root` 是包根，因为身份需要它：面的身份是对相对该包 `src/` 的路径取散列，而它们的命名空间
/// 就是这个包自己的名字。**虚拟清单**不命名任何包，因此它按它实际的样子——工作区——作答：逐成员
/// 一节，各自在自己的名字之下，而它们上方是每个成员的状态普查。
pub(crate) fn registry(root: &Path) -> Result<String, String> {
    registry_with(root, true)
}

/// The registry's **default** answer at a virtual workspace root: the members and the faces each
/// one declares, not each member's whole tree.
/// 虚拟工作区根上注册树的**默认**答案：成员，以及每个成员声明多少个面——不是每个成员的整棵树。
///
/// The measured reason is what a workspace-root call used to cost: 1,558 characters, most of them
/// the per-member trees of members whose answer the caller had not asked for yet. What a reader
/// actually needs first is the shape — which members there are and which of them declare anything —
/// and `full: true` is one word away. A **package** root is unchanged: there the tree *is* the
/// answer, and there is no member list to summarize.
/// 量到的理由是一次工作区根调用过去的代价：1,558 个字符，其中大多是调用方还没问到的成员的整棵树。
/// 读取方真正先需要的是形状——有哪些成员、其中哪些声明了东西——而 `full: true` 只差一个词。**包**根
/// 逐字节不变：在那里树**就是**答案，也没有成员清单可概括。
pub(crate) fn registry_brief(root: &Path) -> Result<String, String> {
    registry_with(root, false)
}

/// The two shapes the tool chooses between, so the scope decision below has one spelling.
/// 工具在两种形态之间选择，因此下面的作用域判断只有一处拼法。
fn registry_with(root: &Path, full: bool) -> Result<String, String> {
    match workspace::scope(root)? {
        Scope::Package(namespace) => {
            let member = Member::package(root, namespace);
            registry_body(&member, &serde_json::json!({"full": full}))
        }
        Scope::Workspace(members) if full => {
            let arguments = serde_json::json!({"full": true});
            workspace::merge(root, &members, &arguments, registry_body)
        }
        Scope::Workspace(members) => Ok(members_and_faces(root, &members)),
        Scope::Unresolvable(reason) => Ok(workspace::unresolvable(root, &reason)),
    }
}

/// One page of the list, for the caller that asked for the rows (audit `W2-1`).
/// 清单的一页，给要那些行的调用方（审计 `W2-1`）。
///
/// The workspace case is answered by the merged full view: a page index across members would be a
/// second coordinate system for the same tree, and the 50,000-face case this exists for is a single
/// package.
/// 工作区那一支由合并后的完整视图作答：跨成员的页索引会是同一棵树的第二套坐标，而这条条目为之存在的
/// 五万面情形是单个包。
pub(crate) fn registry_page(root: &Path, offset: usize, limit: usize) -> Result<String, String> {
    match workspace::scope(root)? {
        Scope::Package(namespace) => {
            // One implementation for the rows, page or not: `registry_body` is where "the record is
            // incomplete, so derive and say why" lives, and a page that took its own path would be
            // the second answer to the same question (measured: a page of the built example still
            // listed the record's four rows while the census derived six).
            // 行只有一份实现，分页与否都一样：`registry_body` 才是"记录不完整就推导并说出原因"的住处，
            // 而另走一条路的分页会是同一个问题的第二个答案（实测：已构建示例的分页仍列出记录的 4 行，而普查
            // 推导出 6 行）。
            let member = Member::package(root, namespace);
            registry_body(
                &member,
                &serde_json::json!({"full": true, "offset": offset, "limit": limit}),
            )
        }
        _ => registry(root),
    }
}

/// The short default: the root, one line per member, and the face count that member declares.
/// 短默认档：根，逐成员一行，以及该成员声明多少个面。
///
/// It is **not** built through [`workspace::roster`], and that is a decision rather than an
/// oversight: the roster is a *preamble* — it is allowed to collapse to one line once this session
/// has opened the root — while the member list here **is** the answer. A default that answered
/// `members 2` and no names on the second call would be a different answer to the same question.
/// 它**不**经 [`workspace::roster`] 构成，这是决定而不是疏漏：普查是**前言**——本会话打开过这个根之后
/// 它可以塌成一行——而这里的成员清单**就是**答案。一个在第二次调用时只答 `members 2`、不点名任何成员的
/// 默认档，是对同一个问题的另一个答案。
fn members_and_faces(root: &Path, members: &[Member]) -> String {
    let mut output = format!(
        "workspace {} · members {} · `full: true` for each member's tree\n",
        root.display(),
        members.len(),
    );

    // The round measured the whole corpus: the trees under test **do not host xirang**, so on them
    // `0 faces` is the answer rather than a defect — and a reader who is not told that asks a
    // face-shaped question about a symbol. Saying it once, before the per-member rows, is the whole
    // fix; it costs one line and removes an inference nobody should have to make.
    // 那一轮量遍语料：被测的树**不宿主 xirang**，因此在它们身上 `0 faces` 是答案而不是缺陷 —— 而没被
    // 告知这一点的读者会用一个"面"形状的问题去问一个符号。在逐成员行**之前**说一次就是全部修法：一行
    // 的成本，去掉一次本不该由读者做的推断。
    if !crate::mcp::workspace::hosts_xirang(root) {
        output.push_str(
            "note   this tree does not host xirang (no `build.rs` + `xirang-toolchain` \
             dependency), so it has no registration tree to read and `0 faces` below is the answer \
             rather than a defect. Ask about *symbols*: `search {query}` / `locate {symptom}` / \
             `read {path, line}` / `callgraph {function}`.\n",
        );
    }
    for member in members {
        output.push_str(&format!(
            "  {:<32} {:<14} ({})\n",
            member.name,
            faces_word(member),
            member.status(),
        ));
    }
    output
}

/// What one member's row says about faces, in the one word it can carry.
/// 一个成员那一行对面数说的一句话，用它能带的那个词。
///
/// The states are the ones the roster row already tells apart, said short: a member that declared
/// nothing says `0 faces`, a member whose records do not answer says `faces unknown` rather than
/// borrowing a count, and a member with no tree says `no tree` — its status word beside it is
/// `unresolvable`.
/// 这几种状态就是普查行已经分辨出的那几种，只是用短的说法：什么都没声明的成员写 `0 faces`，记录答不了
/// 的成员写 `faces unknown` 而不是借一个计数，没有树的成员写 `no tree`——它旁边的状态词是 `unresolvable`。
fn faces_word(member: &Member) -> String {
    match member.tree() {
        Ok(Tree::Published(tree)) if tree.records_no_faces() => "0 faces".to_owned(),
        Ok(Tree::Published(tree)) if tree.faces_unknown() => "faces unknown".to_owned(),
        Ok(Tree::Published(tree)) => format!("{} faces", tree.faces().len()),
        Ok(Tree::Derived { faces, .. }) => format!("{} faces", faces.len()),
        Err(_) => "no tree".to_owned(),
    }
}

/// One package's registry report, as the merged view and the single-package view both
/// call it.
/// 一个包的注册树报告；合并视图与单包视图都调用它。
///
/// The evidence is chosen once, here, and the two renderings below share nothing
/// but the namespace line: they answer from different facts and say so. A body
/// that quietly derived over a published member would make the perf note on this
/// page false, so the branch is a `match` a reader can see.
/// 证据在这里被选一次，下面两种渲染只共享命名空间那一行：它们用不同的事实作答，并且说出来。
/// 一个在发布过的成员之上悄悄推导的主体会让本页的性能注记变成假话，因此这个分支是一个读者看得见
/// 的 `match`。
pub(crate) fn registry_body(member: &Member, arguments: &Value) -> Result<String, String> {
    let flag = |key: &str| arguments.get(key).and_then(Value::as_bool) == Some(true);
    let full = flag("full");
    let offset = arguments.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(PAGE_ROWS, |value| (value as usize).clamp(1, PAGE_ROWS));
    match member.tree()? {
        Tree::Published(tree) => {
            // A record published for the whole tree still may not carry every face the sources
            // declare: a partitioned host's fragment crates register their own faces, and those
            // rows land in **that crate's** record, not this one. Answering from this record alone
            // would give a different tree than the runtime and than `status` — three facts for one
            // project. When the record is incomplete and it claimed the whole tree, derive, and say
            // why (the same `because` line the other over-published answers use).
            // 为整棵树发布的记录仍可能不携带源码声明的每个面：分区宿主的碎片 crate 自己登记面，那些行落在
            // **那个 crate 的**记录里，而不是这份。只据这份记录作答，会得到与运行期、与 `status` 不同的
            // 树——同一个工程三份事实。记录不完整且它声称覆盖整棵树时，改为推导，并说出原因（与其它在发布
            // 记录之上推导的答案用同一条 `because` 行）。
            // **Freshness is not coverage**, and the difference is not this tool's private opinion:
            // `workspace::record_covers_producers` owns the question ("may this record answer that a
            // face does not exist?"), and `search`, `status` and this body all ask it there. A
            // record published for the whole tree still may not carry every face the sources
            // declare — a fragment crate registers its own, and a subtree handed to a generated
            // crate registers in that crate — so answering from this record alone would give a
            // different tree than the runtime and than `status`: three facts for one project.
            // **新鲜不是覆盖**，而这个区别不是本工具的私人意见：`workspace::record_covers_producers`
            // 拥有那个问题（"这份记录能否回答某个面不存在？"），`search`、`status` 与这里的主体都去那里问。
            // 为整棵树发布的记录仍可能不携带源码声明的每个面——碎片 crate 自己登记，交给生成 crate 的子树
            // 在那边登记——因此只据这份记录作答会给出与运行期、与 `status` 不同的树：同一个工程三份事实。
            if tree.selects_everything() {
                // The coverage decision hands its own derivation down: the decision had to walk the
                // sources, and deriving again for the replacement is the duplicate work an
                // independent review measured.
                // 覆盖判定把自己那次推导交下来：这个判定已经遍历过源码，而为替代再推一遍正是独立复核量到的
                // 重复工作。
                let coverage = member.producer_coverage()?;
                if !coverage.complete {
                    let (faces, unparsable) = match coverage.derived {
                        Some(derived) => derived,
                        None => member.derived_tree()?,
                    };
                    let because = "the published record does not carry every face the sources \
                                   declare (a fragment crate registers its own)";
                    let evidence = member.evidence_line(because);
                    return if full {
                        Ok(render_page(
                            &member.name,
                            &faces,
                            &unparsable,
                            &evidence,
                            offset,
                            limit,
                        ))
                    } else {
                        Ok(render_census(&member.name, &faces, &unparsable, &evidence))
                    };
                }
            }
            Ok(render_published(&member.name, tree, full, offset, limit))
        }
        // Audit `W2-1`: the census is the default and the rows are bought — by `full: true`, a page
        // at a time (`offset`/`limit`).
        // 审计 `W2-1`：普查是默认，行是**买**来的——`full: true`，一次一页（`offset`/`limit`）。
        Tree::Derived { faces, unparsable } if !full => Ok(render_census(
            &member.name,
            faces,
            unparsable,
            &member.evidence_line(""),
        )),
        Tree::Derived { faces, unparsable } => {
            let offset = arguments.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .map_or(PAGE_ROWS, |value| (value as usize).clamp(1, PAGE_ROWS));
            Ok(render_page(
                &member.name,
                faces,
                unparsable,
                &member.evidence_line(""),
                offset,
                limit,
            ))
        }
    }
}

/// The identity namespace this package's faces were stamped with.
/// 本包的注册面被盖下的身份命名空间。
///
/// Shared with the write path: an edit must author under the same namespace the
/// query reports, or the face it writes lands in an identity domain the host never
/// compiled.
/// 与写入路径共用：编辑必须在查询所报告的同一个命名空间之下创作，否则它写下的面会落在宿主从未
/// 编译过的身份域里。
///
/// `XIRANG_NAMESPACE` wins verbatim when it is set, and otherwise the package name Cargo reports
/// is the namespace, because that is exactly what the declaration macros bake in as
/// `crate::XIRANG_NAMESPACE`, which `host!()` defines from the package name (audit `M7`, P3.3).
/// `XIRANG_NAMESPACE` 一旦设置就原样胜出，否则 Cargo 报告的包名就是命名空间，因为声明宏烤进去的
/// `crate::XIRANG_NAMESPACE` 正是它，而 `host!()` 用包名定义它（审计 `M7`，P3.3）。
///
/// What this is **not**: a rule every surface reads. Studio reads the same override, the CLI reads
/// none of it (it always asks Cargo for the package name), and no build-side code reads it either —
/// the namespace is baked in at compile time. So with the variable set this query reports a
/// namespace, and `NodeId`s, that `xirang explain --json` and the built host do not use. The
/// variable's documented purpose is a *reader's* override for trace artifacts; whether it should
/// keep applying to host identity is the maintainer's decision, and this note exists so the
/// divergence is a known one rather than a claim of agreement (audit `S12`).
/// 这**不是**什么：不是每个执行面都读的规则。Studio 读同一个覆盖，CLI 完全不读（它始终问 Cargo 要
/// 包名），构建侧也不读——命名空间在编译期就烤好了。因此设置该变量后，这条查询报告的命名空间与
/// `NodeId`，正是 `xirang explain --json` 与已构建宿主**不用的**那一套。该变量文档化的用途是 trace
/// artifact 的**读取者覆盖**；它是否应继续作用于宿主身份由维护者决定，而这段注记的存在是为了让这处分歧
/// 成为已知事项，而不是一句"彼此一致"的声称（审计 `S12`）。
pub(crate) fn namespace(root: &Path) -> Result<String, String> {
    namespace_from(std::env::var(lexicon::NAMESPACE_ENV).ok().as_deref(), root)
}

/// The namespace decision, taking the configured value as a parameter so the
/// override and the fallback can be tested without the process environment.
/// 命名空间的判断；配置值作为参数传入，因此覆盖与回落都能不依赖进程环境地测试。
///
/// It is `pub(crate)` because the workspace entrance asks the same question first: a
/// configured override names one tree, so a virtual root under it is that package rather
/// than a workspace (`workspace::scope`).
/// 它是 `pub(crate)`，因为工作区入口先问同一个问题：配置的覆盖命名一棵树，因此它之下的虚拟根是那个
/// 包而不是一个工作区（`workspace::scope`）。
///
/// The documented default is deliberately **not** a fallback here, and the
/// asymmetry with authoring is the point: authoring *creates* a tree, so
/// `xirang.default` is a real answer for a project nobody has built yet, while
/// this tool *reports* a tree that already exists. Answering under
/// `xirang.default` would publish identities the host never compiled — every
/// `NodeId` is a hash over the namespace — and an agent would carry them into a
/// graft record or a trace lookup that cannot resolve. A refusal it can act on
/// (`set XIRANG_NAMESPACE`) beats an identity that is wrong everywhere.
/// 这里有意**不**回落到文档化的默认值，而与创作侧的不对称正是关键：创作是在**创建**一棵树，
/// 对一个还没人构建过的项目，`xirang.default` 是真实答案；而本工具报告的是**已经存在**的
/// 树。在 `xirang.default` 之下作答会发布宿主从未编译过的身份——每个 `NodeId` 都是对命名
/// 空间的散列——而代理会带着它们去做无法解析的 graft 记录或 trace 查找。一个它能据以行动的
/// 拒绝（`set XIRANG_NAMESPACE`）胜过到处都错的身份。
pub(crate) fn namespace_from(configured: Option<&str>, root: &Path) -> Result<String, String> {
    crate::build_method::identity_namespace(configured, &root.join("Cargo.toml")).map_err(|error| {
        format!(
            "cannot learn the identity namespace of {}: {error}; \
             set {} to name it explicitly",
            root.display(),
            lexicon::NAMESPACE_ENV
        )
    })
}

/// How many level rows the census prints before withholding the rest.
/// 普查在扣下其余之前印多少行层级。
///
/// Measured (2026-10-04) on a built 101-face host: each face in its own source directory makes the
/// published census's fallback coordinate one level per face, and twenty level rows plus the notes
/// came to 2,824 bytes — over the `W2-1` budget of 2 KB for the 50,000-face tier. Twelve rows keep
/// the worst case inside it while still showing the shape a reader needs (where the mass sits).
/// 实测（2026-10-04，一个已构建的 101 面宿主）：每个面各占一个源码目录时，发布普查的回退坐标就是
/// "一面一层"，二十行层级加注是 2,824 字节——超过 `W2-1` 给五万面档定的 2 KB。十二行把最坏情形压回
/// 预算内，同时仍显示读者需要的形状（质量集中在哪儿）。
const CENSUS_LEVELS: usize = 10;

/// How many face rows one page of the list carries.
/// 清单的一页带多少行面。
pub(crate) const PAGE_ROWS: usize = 200;

/// The **census**: the tree's shape without its rows (audit `W2-1`).
/// **普查**：树的形状，不带它的行（审计 `W2-1`）。
///
/// The measured reason: a package root's `registry` printed one line per face, so a 50,000-face tree
/// answered with a 5 MB list whose first screen already said everything a reader needed to choose the
/// next call. What stays here is the shape — how many faces, and how many sit at each level — and the
/// rows are one word away (`full: true`), paged at [`PAGE_ROWS`].
/// 量到的理由：包根的 `registry` 每个面印一行，因此五万面的树回的是 5 MB 的清单，而它第一屏就已经说完了
/// 读者选择下一个调用所需的一切。留在这里的是形状——多少个面、每一层各有多少——而行离一个词
/// （`full: true`），按 [`PAGE_ROWS`] 分页。
fn render_census(namespace: &str, faces: &[FaceView], unparsable: &str, evidence: &str) -> String {
    let mut output = header(namespace, faces.len(), unparsable, evidence);
    if faces.is_empty() {
        output.push_str("no registration face is declared under the package's src/\n");
        return output;
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for face in faces {
        let level = face
            .path
            .rsplit_once('/')
            .map_or("<root>", |(above, _)| above);
        *counts.entry(level).or_default() += 1;
    }
    output.push_str("level (path prefix) and how many faces sit directly under it:\n");
    for (level, count) in counts.iter().take(CENSUS_LEVELS) {
        output.push_str(&format!("  {level:<44} {count} face(s)\n"));
    }
    if counts.len() > CENSUS_LEVELS {
        output.push_str(&crate::mcp::truncation::withheld(
            counts.len() - CENSUS_LEVELS,
            counts.len(),
            CENSUS_LEVELS,
            "level(s)",
            "`full: true` prints the rows themselves; a deeper level is also `search --root`'s business",
        ));
        output.push('\n');
    }
    output.push_str(&format!(
        "faces {} across {} level(s); `full: true` prints the rows, {PAGE_ROWS} per page and \
         `offset: <n>` for the next\n",
        faces.len(),
        counts.len()
    ));
    output
}

/// The header every registry answer opens with: the namespace the identities live in, the evidence
/// line, the files the derivation could not read, and the face count.
/// 每条注册树答案开头的标头：身份所属的命名空间、证据行、推导读不了的文件、以及面的数量。
fn header(namespace: &str, faces: usize, unparsable: &str, evidence: &str) -> String {
    format!("namespace {namespace}\n{evidence}{unparsable}faces {faces}\n")
}

/// One page of the face list, for the reader that asked for the rows.
/// 面清单的一页，给要那些行的读者。
///
/// Audit `W2-1`: the list is bought explicitly, and it arrives a page at a time so the answer stays a
/// screenful even when the tree is 50,000 faces. Every page says which slice it is and names the call
/// that prints the next one.
/// 审计 `W2-1`：清单是显式购买的，而且一次一页，因此即使树有五万面，答案也还是一屏。每一页都说出它是哪
/// 一段，并点名印出下一页的那次调用。
fn render_page(
    namespace: &str,
    faces: &[FaceView],
    unparsable: &str,
    evidence: &str,
    offset: usize,
    limit: usize,
) -> String {
    let mut output = header(namespace, faces.len(), unparsable, evidence);
    if faces.is_empty() {
        output.push_str("no registration face is declared under the package's src/\n");
        return output;
    }
    let start = offset.min(faces.len());
    let end = start.saturating_add(limit.max(1)).min(faces.len());
    output.push_str(&format!("rows {}-{} of {}\n", start + 1, end, faces.len()));
    for face in &faces[start..end] {
        output.push_str(&face_row(face));
    }
    if end < faces.len() {
        output.push_str(&crate::mcp::truncation::withheld(
            faces.len() - end,
            faces.len(),
            limit.max(1),
            "face(s)",
            &format!(
                "pass `offset: {end}` for the next page, or `registry --all-slim`-style narrowing \
                 by `search --root`"
            ),
        ));
        output.push('\n');
    }
    output
}

/// One row of the list, as the whole list and every page spell it.
/// 清单的一行，整份清单与每一页都这样拼。
fn face_row(face: &FaceView) -> String {
    // An unresolved parent is named rather than hidden: the face is real,
    // and the fact that its parent is not is the answer to "why is this node
    // not in my tree".
    // 未解析的父级被点名而不是藏起来：这个面是真的，而"它的父级不是"正是"为什么这个节点不在
    // 我的树里"的答案。
    let unresolved = if face.parent_resolved {
        ""
    } else {
        "  parent-unresolved"
    };
    format!(
        "{:<40} {:<14} {:<38} {}{}\n",
        face.path, face.kind, face.source, face.id, unresolved
    )
}

/// The record's own rows: an identity, a source, a symbol, and the scope's verdict.
/// 记录自己的行：一个身份、一条源码、一个符号，以及作用域的结论。
///
/// The face list is `pruning_manifest.tsv`'s, which the build writes for **every**
/// face it found — not `source_scope.tsv`'s, which lists only the *roots* a
/// narrowed scope selected (two rows for a three-face package, measured on
/// `examples/control-button`). Reading the scope's list as the tree would
/// under-report it, and the last line says where the derived projection lives so
/// nobody has to guess why a `path` is missing.
/// 面清单是 `pruning_manifest.tsv` 的，构建为它找到的**每个**面都写这一份——而不是
/// `source_scope.tsv` 的，后者只列收窄作用域选中的**根**（在 `examples/control-button` 上实测：
/// 三个面的包只有两行）。把作用域那份清单读成树会少报，而最后一行说出推导投影住在哪里，因此没人
/// 需要猜为什么没有 `path`。
fn render_published(
    namespace: &str,
    tree: &PublishedTree,
    full: bool,
    offset: usize,
    limit: usize,
) -> String {
    let mut output = format!(
        "namespace {namespace}\n{}{}\n",
        tree.evidence_line(),
        tree.freshness()
    );
    output.push_str(&format!(
        "scope mode={} all={} reason={} selected_ids={} selected_sources={} selected_modules={}\n",
        tree.scope.mode,
        tree.scope.all,
        tree.scope.reason.as_deref().unwrap_or("-"),
        tree.scope.selected_ids.len(),
        tree.scope.selected_sources.len(),
        tree.scope.selected_modules.len(),
    ));
    let rows = tree.faces();
    if tree.faces_unknown() {
        output.push_str(FACES_UNKNOWN);
        output.push('\n');
        output.push_str(PUBLISHED_NOTE);
        return output;
    }
    output.push_str(&format!("faces {}\n", rows.len()));
    if full {
        let start = offset.min(rows.len());
        let end = start.saturating_add(limit.max(1)).min(rows.len());
        output.push_str(&format!("rows {}-{} of {}\n", start + 1, end, rows.len()));
        for row in &rows[start..end] {
            output.push_str(&published_row(tree, row));
        }
        if end < rows.len() {
            output.push_str(&crate::mcp::truncation::withheld(
                rows.len() - end,
                rows.len(),
                limit.max(1),
                "row(s)",
                &format!("pass `offset: {end}` for the next page"),
            ));
            output.push('\n');
        }
    } else {
        // Audit `W2-1` reaches the published path too: a built tree's `registry` used to print one
        // row per face with no bound at all, which is the same defect the derived path had. The
        // census is the same shape on both paths — the count and how many sit at each level — and it
        // is counted from the **published** `path` column when the record carries one.
        // 审计 `W2-1` 同样落到发布路径上：已构建的树的 `registry` 过去每个面印一行、毫无上限，与推导路径
        // 当初的缺陷是同一个。普查在两条路上是同一个形状——总数与逐层计数——而记录携带 `path` 列时就用
        // **已发布的**那一个来数。
        // The **logical** path is what the census counts on (audit `W3-2`): it is the answer the
        // runtime gives, while `path` is only what a declaration spelled and is empty for every
        // macro-derived face. The fallback chain is logical path → declared path → source directory,
        // and whichever one answered is named in the line above the counts.
        // 普查按**逻辑**路径计数（审计 `W3-2`）：它是运行期给出的答案，而 `path` 只是声明拼出的东西、
        // 对每个宏派生的面都空着。回退链是 逻辑路径 → 声明路径 → 源码目录，而究竟是哪一个答的，由计数
        // 上方那一行说出来。
        let by_logical = rows.iter().any(|row| row.logical_path.is_some());
        let by_path = by_logical || rows.iter().any(|row| row.path.is_some());
        output.push_str(&format!(
            "level ({}) and how many recorded faces sit under it:\n",
            if by_logical {
                "logical path prefix"
            } else if by_path {
                "declared path prefix — this record carries no logical path"
            } else {
                "source directory — this record carries no path"
            }
        ));
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for row in rows {
            let level = match (&row.logical_path, by_logical) {
                (Some(path), true) => path
                    .rsplit_once('/')
                    .map_or("<root>", |(above, _)| above)
                    .to_owned(),
                _ => match (&row.path, by_path) {
                    (Some(path), true) => path
                        .rsplit_once('/')
                        .map_or("<root>", |(above, _)| above)
                        .to_owned(),
                    _ => row
                        .source
                        .rsplit_once('/')
                        .map_or("<src>", |(dir, _)| dir)
                        .to_owned(),
                },
            };
            *counts.entry(level).or_default() += 1;
        }
        for (level, count) in counts.iter().take(CENSUS_LEVELS) {
            output.push_str(&format!("  {level:<44} {count} face(s)\n"));
        }
        if counts.len() > CENSUS_LEVELS {
            output.push_str(&crate::mcp::truncation::withheld(
                counts.len() - CENSUS_LEVELS,
                counts.len(),
                CENSUS_LEVELS,
                "level(s)",
                "`full: true` prints the rows themselves",
            ));
            output.push('\n');
        }
        output.push_str(&format!(
            "faces {} across {} level(s); `full: true` prints the rows, {PAGE_ROWS} per page and \
             `offset: <n>` for the next\n",
            rows.len(),
            counts.len()
        ));
    }
    if rows.is_empty() {
        output.push_str("no registration face is recorded for this package — the sources may still declare `external_object!` faces, which this package's generated tree deliberately does not contain; `xirang.search` derives and names them\n");
    }
    output.push_str(PUBLISHED_NOTE);
    output
}

/// One published row, with the declaration facts the record carries when it carries them.
/// 一条已发布的行，记录携带声明事实时就带上它们。
fn published_row(tree: &PublishedTree, row: &PruningRow) -> String {
    let verdict = if tree.selected(row) {
        "selected"
    } else {
        "not-selected"
    };
    let mut line = format!(
        "  {:<13} {:<38} {:<44} {}",
        verdict, row.id, row.source, row.symbol
    );
    fn spelled(value: &Option<String>) -> &str {
        value.as_deref().unwrap_or("-")
    }
    line.push_str(&format!(
        "  path={} logical_path={} kind={} registry_name={} parent={} calls={} owns_registry={}",
        spelled(&row.path),
        spelled(&row.logical_path),
        spelled(&row.kind),
        spelled(&row.registry_name),
        spelled(&row.parent),
        spelled(&row.calls),
        spelled(&row.owns_registry)
    ));
    line.push('\n');
    line
}

/// What a published answer is, and what a reader still has to derive.
/// 一条已发布答案是什么，以及读者还需要推导什么。
///
/// The sentence here used to say `path`, `kind`, `registry_name` and `parent` "are derived facts and
/// are not in the record" — untrue since 2026-09-29, when the pruning row began publishing them
/// (audit `W3-1` then added the source hash, the declaration fingerprint and the direct calls). It
/// is a self-description that ran **behind** the behaviour, which is the same defect as one that
/// runs ahead of it: a reader believed the columns were not there and derived them by hand.
/// 这句话过去说 `path`、`kind`、`registry_name`、`parent`"是推导事实、不在记录里"——从 2026-09-29 剪枝行
/// 开始发布它们起就不成立（审计 `W3-1` 又加了源码哈希、声明指纹与直接调用名）。这是一句**落后于**行为的
/// 自述，与"跑在行为前面"的自述是同一个缺陷：读者以为那几列不在，于是手工推导。
const PUBLISHED_NOTE: &str = "note: these are the build's **published** rows. A row shows the \
     declaration facts the record carries (`path`, `logical_path`, `kind`, `registry_name`, `parent`, \
     `owns_registry`) and what its file calls (`calls`); `-` means the declaration named none. \
     `source_hash`, `fields` and `parent_node` are published **in** `pruning_manifest.tsv` and are \
     not reprinted per row. A face added since the build is not in the record at all — that one is \
     `xirang.search`'s to derive.\n";

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
