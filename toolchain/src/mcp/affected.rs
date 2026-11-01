//! Which tests a set of changed files reaches — the question a reader asks before running
//! anything, answered from the same source index every other tool reads.
//! 一组改动过的文件能触到哪些测试——读者在"跑什么"之前要问的问题，用与其它工具同一份源码索引作答。
//!
//! Reverse lexical calls nominate regression candidates, never coverage. Unknown dispatch,
//! aliases and deleted definitions require a conservative workspace regression.
//! 反向词法调用给出回归候选，不代表覆盖率。未知派发、别名和已删除定义需要保守的工作区回归。
//!
//! online: the build publishes faces, not which test reaches which source, so this answer is about the sources as they are now.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::mcp::source_index::{display_list, load_sources};
use crate::mcp::truncation::withheld;
use xirang_kernel::lexicon;

/// How many tests one changed file may name before the reply says how many it withheld.
/// 一个改动过的文件最多点名多少个测试，超出后回复要说出扣下了多少。
const TESTS_PER_FILE: usize = 10;

pub(crate) fn affected(root: &Path, arguments: &Value) -> Result<String, String> {
    // One path and a list of paths are the same question asked twice, so both spellings are
    // accepted: a single `--files <path>` became a string on the command line, and refusing it was
    // the round's three-call detour through `--json`.
    // 一个路径与一串路径是同一个问题问两次，因此两种写法都收：命令行上的单个 `--files <路径>` 是字符串，
    // 拒绝它正是那轮经 `--json` 绕行的三次调用。
    let files = match arguments.get("files") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("files entries must be strings")
            })
            .collect::<Result<Vec<_>, _>>()?,
        // A comma-separated string is the third spelling of "several files", and the round measured
        // it failing: `--files "a.rs,b.rs"` reached the tool as **one** path named `a.rs,b.rs` and
        // answered `not in the index`, while the same two paths in a JSON array worked. Nothing
        // about the argument's type says "one path" — its spelling does, and a comma is how a list
        // is written on a command line.
        // 逗号分隔的字符串是"多个文件"的第三种拼法，而那一轮量到它失败：`--files "a.rs,b.rs"` 以**一个**
        // 名为 `a.rs,b.rs` 的路径到达，答 `not in the index`，而同样两个路径写成 JSON 数组就成功。参数的
        // **类型**并没有说"这是一个路径"—— 是它的**拼法**在说，而逗号正是命令行上写列表的方式。
        Some(Value::String(spelled)) => spelled
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        _ => {
            return Err(
                "xirang.affected requires `files`: one path, a list of paths, or repeated \
                 `--files` flags from the one-shot client"
                    .to_owned(),
            );
        }
    };
    let files = files
        .into_iter()
        .map(|path| path.trim().to_owned())
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err("`files` must name at least one path".to_owned());
    }
    let limit = match arguments.get("limit") {
        None => TESTS_PER_FILE,
        Some(value) => value
            .as_u64()
            .filter(|n| (1..=200).contains(n))
            .ok_or("limit must be an integer between 1 and 200")? as usize,
    };
    let files: BTreeSet<_> = files.into_iter().collect();
    if files.len() > 128 {
        return Err("files accepts at most 128 distinct paths".into());
    }
    let mut plan_cache = BTreeMap::new();
    let mut output = String::from(
        "evidence: syntax-resolved where the Rust source parses; lexical fallback otherwise\n",
    );
    if let Some(above) = crate::mcp::workspace::enclosing_workspace(root) {
        // The cross-member half of this answer is the whole point of the tool, so when the root is a
        // member the caller has to know the other members are out of frame.
        // 这个答案的"跨成员"那一半正是这个工具的意义，因此当根是成员时，调用方必须知道别的成员在画面外。
        output.push_str(&format!(
            "note  this root is a member of the workspace at {}; other members are out of frame \
             here\n",
            above.display()
        ));
    }
    // Read one current inventory and build one reverse index for all changed files.
    // 全部改动文件共用一次当前清单读取与一份反向索引。
    let sources = load_sources(root)?;
    let relations = super::relations::Relations::new(&sources);
    output.push_str(
        "scope: current Rust source under root, including uncompiled files; calls carry their \
         owner where the syntax writes one, unresolved receivers and bare-name collisions remain \
         unknown; this is transitive candidates, not coverage\n",
    );
    for path in &files {
        let Some(touched) = sources.iter().find(|file| file.relative == *path) else {
            output.push_str(&format!(
                "{path}: not in the index; deleted/generated files require workspace regression\n"
            ));
            continue;
        };
        let names = touched
            .functions
            .iter()
            .map(|function| function.qualified.clone())
            .collect::<Vec<_>>();
        let tests = relations.tests_reaching(&names, touched);
        let owner = owner_of(root, path);
        output.push_str(&format!(
            "{}: {} definition(s)\n",
            touched.relative,
            names.len()
        ));
        let unresolved: usize = touched
            .functions
            .iter()
            .map(|function| function.unresolved.len())
            .sum();
        if unresolved > 0 {
            output.push_str(&format!(
                "  unresolved: {unresolved} call site(s) kept as unknown; no caller/callee edge is claimed from them\n"
            ));
        }
        // Audit `W5-2`'s plan layer: a symbol's reach is not only the tests that call it, it is also
        // the **graft plan entries** that name the face the symbol lives on. A reader of "what does
        // this change touch" who is not told about those entries misses the half of the answer that
        // is not Rust code.
        // 审计 `W5-2` 的计划层：一个符号的波及面不只是调用它的测试，还包括点名"它所在的那个面"的
        // **graft 计划条目**。问"这次改动波及什么"的读者如果不知道那些条目，就漏掉了答案里不是 Rust
        // 代码的那一半。
        for line in plan_layer(
            &mut plan_cache,
            &owner,
            &root
                .join(&touched.relative)
                .strip_prefix(&owner)
                .unwrap_or(Path::new(&touched.relative))
                .to_string_lossy(),
        ) {
            output.push_str(&format!("  {line}\n"));
        }
        if tests.is_empty() {
            output.push_str(
                "  tests: none reference these definitions through indexed calls; coverage unknown; run workspace regression\n",
            );
        } else {
            output.push_str(&format!(
                "  tests: {}\n",
                display_list(&tests[..tests.len().min(limit)])
            ));
            if tests.len() > limit {
                output.push_str(&format!(
                    "  {}\n",
                    withheld(
                        tests.len() - limit,
                        tests.len(),
                        limit,
                        "test files",
                        "raise limit (up to 200); beyond that run workspace regression"
                    )
                ));
            }
        }
    }
    output.push_str("run: `cargo test --workspace` for the conservative regression; listed candidates alone do not prove complete coverage\n");
    Ok(output)
}

/// The graft plan entries that name the face a changed file declares (audit `W5-2`).
/// 点名"某个改动文件所声明的那个面"的那些 graft 计划条目（审计 `W5-2`）。
///
/// **A member with no plan directory pays nothing**: the existence check comes first, so a tree that
/// has never had an external graft gets no derivation, no rows and no extra line. That ordering is
/// the whole reason this can sit in `affected` without making it a tree walk.
/// **没有计划目录的成员不付任何代价**：先做存在性判断，因此从未有过外部 graft 的树不会多出一次推导、
/// 不会多出行、也不会多出一行字。这个次序正是它能待在 `affected` 里而不把它变成一次全树遍历的原因。
///
/// A plan entry is reported by the **selector** that names it and the **logical path** it targets, not
/// by the implementation it selects: `affected` answers "what does this change touch", and the
/// replacement is the build's question (`grafts` answers it).
/// 一条计划条目按点名它的**选择器**与它针对的**逻辑路径**报告，而不是按它选中的实现：`affected` 回答的是
/// "这次改动波及什么"，而替换件是构建的问题（由 `grafts` 回答）。
type Plans = Result<
    (
        Vec<crate::build_method::FaceView>,
        Vec<crate::build_method::GraftPlanRow>,
    ),
    String,
>;
fn plan_layer(cache: &mut BTreeMap<PathBuf, Plans>, member: &Path, source: &str) -> Vec<String> {
    let plans = member
        .join(lexicon::XIRANG_DIR)
        .join(lexicon::EXTERNAL_GRAFT_DIR);
    if !plans.is_dir() {
        return Vec::new();
    }
    let data = cache.entry(member.to_path_buf()).or_insert_with(|| {
        let namespace = crate::mcp::registry::namespace(member)?;
        let (faces, _) = crate::mcp::resolve::derived_faces(member, &namespace)?;
        let declared = crate::build_method::declared_grafts(member)?;
        let rows = crate::build_method::graft_plan_rows(member, &faces, Some(&declared))?;
        Ok((faces, rows))
    });
    let (faces, rows) = match data {
        Ok(data) => data,
        Err(error) => {
            return vec![format!(
                "plan: unknown ({error}); graft obligations remain open"
            )];
        }
    };
    if rows.is_empty() {
        return vec![format!(
            "plan: no entry under {} names any face (nothing to touch there)",
            plans.display()
        )];
    }
    // Two coordinate systems meet here, and the record settles which is which: `FaceView.source` is
    // relative to `src/` (the build's own layout) while `affected`'s changed paths come from the
    // source index and carry the `src/` prefix. Comparing them raw makes every face look unnamed,
    // which is a **silent** miss — the answer would say "no plan names this face" for a face a plan
    // does name. Both spellings are therefore accepted, and so is an absolute path's tail.
    // 这里两套坐标相遇，而记录决定了哪套是哪套：`FaceView.source` 相对 `src/`（构建自己的布局），而
    // `affected` 的改动路径来自源码索引、带 `src/` 前缀。原样比较会让每个面都像"没有名字"，而那是**静默**
    // 漏报——对一个确实被计划点名的面，答案会说"没有计划点名它"。因此两种拼法都收，绝对路径的尾段也收。
    let bare = source.strip_prefix("src/").unwrap_or(source);
    let named: Vec<String> = faces
        .iter()
        .filter(|face| face.source == bare || face.source == source)
        .map(|face| face.id.to_string())
        .collect();
    let mut hits: Vec<String> = rows
        .iter()
        .filter(|row| {
            row.target
                .is_some_and(|target| named.iter().any(|id| id == &target.to_string()))
        })
        .map(|row| {
            format!(
                "{} targets {}",
                row.selector,
                row.target_path.as_deref().unwrap_or("-")
            )
        })
        .collect();
    hits.sort();
    hits.dedup();
    if hits.is_empty() {
        vec![format!(
            "plan: none of the {} entry(ies) here names this face",
            rows.len()
        )]
    } else {
        vec![format!(
            "plan: {} entry(ies) name this face — {}",
            hits.len(),
            display_list(&hits[..hits.len().min(6)])
        )]
    }
}

/// Find the closest package manifest without deriving every member's registration tree.
/// 查找最近的包清单，不为源码问题推导全部成员的注册树。
fn owner_of(root: &Path, path: &str) -> PathBuf {
    let source = root.join(path);
    source
        .ancestors()
        .skip(1)
        .take_while(|parent| parent.starts_with(root))
        .find(|parent| parent.join("Cargo.toml").is_file())
        .unwrap_or(root)
        .to_path_buf()
}

#[cfg(test)]
#[path = "affected_tests.rs"]
mod affected_tests;
