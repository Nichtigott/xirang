//! The census is the whole-tree half of a `check`: static facts, each with the sentence that says
//! what it did not cover.
//! 总账是 `check` 的全树那一半：静态事实，每条都带着"它没覆盖什么"那一句。

use std::path::{Path, PathBuf};

/// The census as the default `check` reply carries it: sampled rows and the boundary indexes.
/// 默认 `check` 回复携带的总账：抽样的行与边界索引。
fn census(root: &Path) -> Result<Vec<String>, String> {
    super::census(root, false)
}

/// The census as `census: true` carries it: the whole table and the full boundary prose.
/// `census: true` 携带的总账：整表与完整边界散文。
fn whole(root: &Path) -> Result<Vec<String>, String> {
    super::census(root, true)
}

/// A throwaway package with one respelled constant, one unreferenced constant, one production
/// function no test names, and one a test does name.
/// 一个一次性包：一个被重拼的常量、一个没人引用的常量、一个没有任何测试点名的生产函数，以及一个
/// 测试确实点名的函数。
fn package(label: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("xirang-mcp-claims-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"fixture-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A fixture.\n\
         pub const LIMIT: i64 = 1000;\n\
         pub const UNUSED: i64 = 7;\n\
         pub fn named_by_a_test() -> i64 { LIMIT }\n\
         pub fn no_test_names_me() -> i64 { 1 }\n",
    )
    .expect("fixture source");
    std::fs::write(
        root.join("src/other.rs"),
        "//! A second production file that spells the number again.\n\
         pub fn threshold() -> i64 { 1000 }\n",
    )
    .expect("fixture second source");
    std::fs::write(
        root.join("tests/one.rs"),
        "#[test]\nfn names_the_function() { assert_eq!(fixture_facts::named_by_a_test(), 1000); }\n",
    )
    .expect("fixture test");
    root
}

/// Every column is a static fact, and the closing sentence names the boundary.
/// 每一栏都是静态事实，而结尾那句点出边界。
#[test]
fn the_census_reports_static_facts_and_says_what_it_did_not_cover() {
    let root = package("facts");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains("respelled 1000"), "{lines}");
    assert!(lines.contains("unreferenced `UNUSED`"), "{lines}");
    assert!(
        lines.contains("no test names `no_test_names_me`"),
        "{lines}"
    );
    assert!(
        !lines.contains("no test names `named_by_a_test`"),
        "a test names this one: {lines}"
    );
    assert!(lines.contains("entry plan"), "{lines}");
    assert!(lines.contains("not covered"), "{lines}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A throwaway package whose production side is one chain a test walks (`reachable_a` →
/// `reachable_b`) plus `unreachable` functions nothing reaches.
/// 一个一次性包：生产侧是一条测试走过的链（`reachable_a` → `reachable_b`），外加 `unreachable` 个
/// 没人到达的函数。
fn reachability_package(label: &str, unreachable: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-claims-reach-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"fixture-reach-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    )
    .expect("fixture manifest");
    let mut source = String::from(
        "//! A reachability fixture.\n\
         pub fn reachable_a() -> i64 { reachable_b() }\n\
         pub fn reachable_b() -> i64 { 1 }\n",
    );
    for index in 0..unreachable {
        source.push_str(&format!(
            "pub fn unreachable_{index}() -> i64 {{ {index} }}\n"
        ));
    }
    std::fs::write(root.join("src/lib.rs"), source).expect("fixture source");
    std::fs::write(
        root.join("tests/one.rs"),
        "#[test]\nfn walks_the_chain() { assert_eq!(fixture_reach::reachable_a(), 1); }\n",
    )
    .expect("fixture test");
    root
}

/// The pin: a test walks `reachable_a` → `reachable_b`, so the walk marks both and lists only the
/// function no test can reach.
/// 钉子：测试走过 `reachable_a` → `reachable_b`，因此遍历把两者都标为已到达，只列出没有测试能到达的
/// 那个函数。
#[test]
fn the_walk_lists_only_the_function_no_test_can_reach() {
    let root = reachability_package("chain", 1);
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains("no test reaches `unreachable_0`"), "{lines}");
    assert!(
        !lines.contains("no test reaches `reachable_a`"),
        "a test calls this one: {lines}"
    );
    assert!(
        !lines.contains("no test reaches `reachable_b`"),
        "a test call reaches this one through `reachable_a`: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The other pin: the boundary sentence itself is in the answer, with every invisibility it has to
/// admit, and the column never calls itself coverage.
/// 另一条钉子：边界那句本身就在答案里，带着它必须承认的每一种不可见，而且这一栏从不自称覆盖率。
#[test]
fn the_walk_states_its_own_boundary_in_the_answer() {
    let root = reachability_package("boundary", 1);
    let lines = whole(&root).expect("the census answers").join("\n");
    assert!(lines.contains(super::REACHABILITY_BOUNDARY), "{lines}");
    for bound in [
        "dynamic dispatch",
        "function pointers",
        "macro expansion",
        "trait method",
        "closure",
        "not a coverage measurement",
    ] {
        assert!(
            super::REACHABILITY_BOUNDARY.contains(bound),
            "the boundary sentence has to name `{bound}`: {}",
            super::REACHABILITY_BOUNDARY
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The third pin: both boundary lines are **one-line indexes**. They keep the phrase that stops
/// an inventory reading as a measurement, they name where the prose went (`--list check`), they
/// are really emitted with the census — and a budget keeps them from growing back into the
/// paragraphs they were.
/// 第三枚钉子：两条边界行都是**一行索引**。它们留下那个阻止读者把清单当成量度的短语，点名散文去了哪里
/// （`--list check`），真的随总账输出——而一份预算让它们长不回过去那种段落。
#[test]
fn the_boundaries_are_one_line_indexes_under_a_budget() {
    let root = reachability_package("index", 1);
    let lines = whole(&root).expect("the census answers").join("\n");
    for boundary in [
        super::REACHABILITY_BOUNDARY,
        super::CENSUS_BOUNDARY,
        super::BRANCH_BOUNDARY,
    ] {
        assert!(
            !boundary.contains('\n'),
            "one line, not a paragraph: {boundary}"
        );
        assert!(
            boundary.contains("not a coverage measurement"),
            "a boundary must never stop saying what it is not: {boundary}"
        );
        assert!(
            boundary.contains("--list check"),
            "the prose it indexes has to be named: {boundary}"
        );
        assert!(
            lines.contains(boundary),
            "the boundary is emitted with the census: {boundary}\n{lines}"
        );
    }
    assert!(
        super::CENSUS_BOUNDARY.contains("not covered:"),
        "the closing line says what it does not cover: {}",
        super::CENSUS_BOUNDARY
    );
    assert!(
        super::REACHABILITY_BOUNDARY.contains("not covered by the test-reachability column:"),
        "so does the reachability column's own: {}",
        super::REACHABILITY_BOUNDARY
    );
    assert!(
        super::BRANCH_BOUNDARY.contains("not covered by the branch-level column:"),
        "and the branch column's own: {}",
        super::BRANCH_BOUNDARY
    );
    // The budget is a ratchet rather than a taste: the two lines measured 774 + 331 characters
    // before they became indexes; with the branch column's own boundary (the third line, which the
    // O3 answer key pins verbatim) the three measured 523 + 257 + 725 = 1505, and the repair task
    // t4 added the two truths the independent re-test found missing from that third line (a macro
    // body this tree writes is text; a name imported from another crate is skipped), which brings it
    // to 1137 and the three to 523 + 257 + 1137 = 1917 — the ceiling they must not cross again.
    // 预算是棘轮而不是口味：这两行在变成索引之前量到 774 + 331 个字符；加上分支栏自己的边界（第三条
    // 行，O3 的答案键逐字钉住了它）之后三行量到 523 + 257 + 725 = 1505，而修复任务 t4 把独立复测发现
    // 第三条行里缺的两条真话补上（本树写下的宏体是文本；从另一个 crate 引入的名字会被略过），它于是变成
    // 1137，三行合计 523 + 257 + 1137 = 1917——这就是它们不许再次越过的上限。
    const BUDGET: usize = 1917;
    let total = super::REACHABILITY_BOUNDARY.chars().count()
        + super::CENSUS_BOUNDARY.chars().count()
        + super::BRANCH_BOUNDARY.chars().count();
    assert!(
        total <= BUDGET,
        "the two boundaries must stay within {BUDGET} characters; they are {total}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A throwaway package with the three arms a branch-level read has to separate: a literal-`false`
/// guard and a variant of a private enum that nothing constructs (both decided), and a
/// data-dependent condition (never judged). The variant is named in a comment above its own arm, so
/// a read that counted raw text instead of the kernel's masked text would erase the row.
/// 一个一次性包，含分支级读取必须区分开的三种臂：字面量 `false` 守卫与私有枚举中没人构造的变体
/// （两个都被判定），以及一个数据相关条件（一律不判）。该变体在它自己的臂上方**注释**里被提到一次，
/// 因此拿原始文本而不是内核掩码文本去数的读取会把那一行抹掉。
fn branch_package(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-claims-branch-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"fixture-branch-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A branch fixture.\n\
         enum Band { Small, Frozen }\n\
         \n\
         fn band_word(band: Band, amount: i64, limit: i64) -> &'static str {\n\
         \x20   if false {\n\
         \x20       return \"unreachable\";\n\
         \x20   }\n\
         \x20   if amount > limit {\n\
         \x20       return \"over\";\n\
         \x20   }\n\
         \x20   match band {\n\
         \x20       Band::Small => \"small\",\n\
         \x20       // Band::Frozen is constructed nowhere in this tree.\n\
         \x20       Band::Frozen => \"frozen\",\n\
         \x20   }\n\
         }\n\
         \n\
         pub fn default_band() -> Band { Band::Small }\n",
    )
    .expect("fixture source");
    root
}

/// The sixth column reports the two arms decided by construction, at the lines the design fixes:
/// the guard's own line and the arm's pattern line.
/// 第六栏报出按构造判定的两个臂，行号就是设计钉住的那两个：守卫自己的行与臂的模式行。
#[test]
fn the_branch_column_reports_the_two_constructively_unreachable_arms() {
    let root = branch_package("rows");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(
        lines.contains(
            "branch-level: 2 constructively unreachable arm(s) in this tree (1 `false` guard(s), 1 \
             never-constructed variant(s)"
        ),
        "{lines}"
    );
    assert!(
        lines.contains("`if false` guards an arm in `band_word` at src/lib.rs:5"),
        "the row reports the line of the guard, not of its block: {lines}"
    );
    assert!(
        lines.contains("no construction of `Band::Frozen` is spelled in this tree"),
        "{lines}"
    );
    assert!(
        lines.contains("the arm matching it in `band_word` at src/lib.rs:14"),
        "the row reports the arm's pattern line, not the comment above it: {lines}"
    );
    assert!(
        lines.contains(
            "can never be entered (the enum is private, so a constructor outside this \
                        tree cannot spell the variant either)"
        ),
        "{lines}"
    );
    // The data-dependent arm sits two lines above the match and must not be named at all.
    // 数据相关的那个臂就在 match 上方两行，并且绝不能被点名。
    assert!(
        !lines.contains("src/lib.rs:8") && !lines.contains("`amount > limit`"),
        "a condition whose value depends on data is not judged: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A `pub` enum is never judged: a consumer outside this root can construct a variant this tree
/// never spells, so the arm is not an answer about this tree.
/// `pub` 枚举一律不判：本根之外的使用方可以构造一个本树从未拼出的变体，因此那个臂不是关于这棵树的答案。
#[test]
fn the_branch_column_never_judges_a_public_enum() {
    let root = branch_package("public");
    std::fs::write(
        root.join("src/pub.rs"),
        "//! A public enum nothing here constructs a dormant variant of.\n\
         pub enum State { Open, Dormant }\n\
         \n\
         pub fn state_word(state: State) -> &'static str {\n\
         \x20   match state { State::Open => \"open\", State::Dormant => \"dormant\" }\n\
         }\n",
    )
    .expect("fixture source");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(
        lines.contains("branch-level: 2 constructively unreachable arm(s)"),
        "the `pub` enum adds no row of its own: {lines}"
    );
    assert!(
        !lines.contains("State::Dormant"),
        "a public enum's arm is never judged: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A variant this tree *does* construct is not reported, and a construction in a test file counts:
/// the join is about the tree, not about where the spelling lives — while an arm written in that
/// same test file would still be skipped, because that half *is* about where it lives.
/// 本树**确实**构造的变体不会被报，而测试文件里的构造也算数：这次连接说的是整棵树，而不是拼法住在哪
/// ——而在同一个测试文件里写下的臂仍然会被略过，因为那一半**是**关于它住在哪的。
#[test]
fn a_construction_in_a_test_file_still_counts() {
    let root = branch_package("constructed");
    std::fs::write(
        root.join("src/extra.rs"),
        "//! A test-only file that constructs the reserved band.\n\
         #[test]\n\
         fn builds_the_reserved_band() {\n\
         \x20   let _ = crate::Band::Frozen;\n\
         }\n",
    )
    .expect("fixture source");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(
        lines.contains(
            "branch-level: 1 constructively unreachable arm(s) in this tree (1 `false` guard(s), 0 \
             never-constructed variant(s)"
        ),
        "the constructed variant drops its row and keeps the guard's: {lines}"
    );
    assert!(
        !lines.contains("Band::Frozen"),
        "a variant this tree constructs is not unreachable: {lines}"
    );
    assert!(
        lines.contains("`if false` guards an arm in `band_word` at src/lib.rs:5"),
        "{lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An arm on an enum this tree does not declare is not judged: the variant may be constructed by
/// the crate that declares it, and no scan of this root can see that.
/// 本树没有声明该枚举时，它的臂不被判定：变体可能是声明它的那个 crate 构造的，而任何对本根的扫描都看
/// 不到那件事。
#[test]
fn the_branch_column_never_judges_an_enum_this_tree_does_not_declare() {
    let root = branch_package("foreign");
    std::fs::write(
        root.join("src/elsewhere.rs"),
        "//! An arm on an enum another crate declares.\n\
         use ledger_core::State;\n\
         \n\
         pub fn state_name(state: State) -> &'static str {\n\
         \x20   match state { State::Open => \"open\", State::Dormant => \"dormant\" }\n\
         }\n",
    )
    .expect("fixture source");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(
        lines.contains("branch-level: 2 constructively unreachable arm(s)"),
        "the undeclared enum adds no row of its own: {lines}"
    );
    assert!(
        !lines.contains("State::Dormant"),
        "an enum this tree never declares is never reported: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A two-segment pattern names whatever that name means **in its own file**, so when the file
/// imports the name from another crate the tree's own declaration of the same name is not what the
/// arm named — and no row may claim it.
/// 两段模式点名的是那个名字**在它自己文件里**的含义，因此当该文件从另一个 crate 引入这个名字时，这个臂
/// 点名的并不是本树同名的那个声明——任何一行都不得声称它。
#[test]
fn the_branch_column_never_judges_a_name_imported_from_outside() {
    let root = branch_package("imported");
    std::fs::write(
        root.join("src/elsewhere.rs"),
        "//! The same name, imported: this arm does not name this tree's enum.\n\
         use ledger_core::Band;\n\
         \n\
         pub fn band_name(band: Band) -> &'static str {\n\
         \x20   match band { Band::Small => \"small\", Band::Frozen => \"frozen\" }\n\
         }\n",
    )
    .expect("fixture source");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(
        !lines.contains("the arm matching it in `band_name`"),
        "the imported name is not this tree's enum: {lines}"
    );
    assert!(
        lines.contains("the arm matching it in `band_word` at src/lib.rs:14"),
        "the tree's own arm is still reported: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An arm that sits outside any function — a `macro_rules!` body this tree writes — is still
/// listed, because that body **is** text; but its row must not render a pair of empty backticks.
/// The kernel keeps reporting the empty name (its own pin says so); the rendering drops the segment.
/// 不在任何函数里的臂——本树写下的 `macro_rules!` 体——**仍然**会被列出，因为那个体就是文本；但它的行
/// 绝不能渲染出一对空反引号。内核仍然报告空名字（它自己的钉子钉住这一点），渲染把这一段省掉。
#[test]
fn a_row_without_a_function_name_omits_the_name() {
    let root = branch_package("no-function");
    std::fs::write(
        root.join("src/macros.rs"),
        "//! A macro body the tree writes: its text is read, and no function encloses it.\n\
         macro_rules! m {\n\
         \x20   () => {\n\
         \x20       if false {\n\
         \x20           return;\n\
         \x20       }\n\
         \x20   };\n\
         }\n",
    )
    .expect("fixture source");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(
        lines.contains("`if false` guards an arm at src/macros.rs:4 that no run can enter"),
        "the row drops the name instead of printing an empty pair of backticks: {lines}"
    );
    assert!(
        !lines.contains("an arm in `` at"),
        "an empty function name is never rendered as empty backticks: {lines}"
    );
    assert!(
        !lines.contains("matching it in `` at"),
        "and neither is the variant row's: {lines}"
    );
    // The behaviour is stated where a reader meets it, so the row is not the only place saying so.
    // 这个行为在读者遇到它的地方就有说明，因此那一行不是唯一说出这件事的地方。
    assert!(
        super::BRANCH_BOUNDARY.contains("its row names no function"),
        "the boundary says the row names no function: {}",
        super::BRANCH_BOUNDARY
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The sixth column's boundary sentence is emitted with the column, names every invisibility it has
/// to admit, and says out loud that the read is not a coverage measurement.
/// 第六栏的边界句随那一栏输出，点名它必须承认的每一种不可见，并明说这次读取不是覆盖率量度。
#[test]
fn the_branch_column_states_its_own_boundary_in_the_answer() {
    let root = branch_package("boundary");
    let lines = whole(&root).expect("the census answers").join("\n");
    assert!(lines.contains(super::BRANCH_BOUNDARY), "{lines}");
    for bound in [
        "a condition whose value depends on data",
        "`false` is the only guard literal decided",
        "macro expansion, dynamic dispatch, function pointers and FFI are invisible",
        "would falsify a row",
        "a `pub` enum is never judged",
        "wildcard or a binding",
        "not a coverage measurement",
        // The two truths t3's independent re-test found missing from this sentence: a macro body
        // this tree writes is text (so its arms are read), and a name imported from another crate
        // is skipped rather than judged.
        // t3 独立复测发现这句里缺的两条真话：本树写下的宏体是文本（因此其中的臂会被读到），以及从另
        // 一个 crate 引入的名字会被略过而不是被判定。
        "a `macro_rules!` body this tree writes **is** text",
        "an arm that only exists after expansion is invisible",
        "its row names no function",
        "conservatively skipped",
        "a same-named foreign enum's arms are a miss here rather than a false row",
    ] {
        assert!(
            super::BRANCH_BOUNDARY.contains(bound),
            "the boundary sentence has to name `{bound}`: {}",
            super::BRANCH_BOUNDARY
        );
    }
    assert!(
        super::BRANCH_BOUNDARY.contains("not covered by the branch-level column:"),
        "the boundary says what column it indexes: {}",
        super::BRANCH_BOUNDARY
    );
    assert!(
        !lines.contains("if amount > limit"),
        "the data-dependent arm is invisible, not reported: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The cap: five rows, then the one truncation sentence with what it withheld.
/// 上限：五行，然后是那句唯一的截断说明与被扣下的数量。
#[test]
fn the_walk_lists_five_rows_and_says_how_many_it_withheld() {
    let root = reachability_package("cap", 8);
    let lines = census(&root).expect("the census answers").join("\n");
    assert_eq!(
        lines.matches("no test reaches `").count(),
        5,
        "the cap is five rows: {lines}"
    );
    assert!(
        lines.contains("test-unreachable functions"),
        "the withheld count names what was counted: {lines}"
    );
    assert!(
        lines.contains(crate::mcp::truncation::PHRASE),
        "the cap goes through the one truncation outlet: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The budget is not decoration: a tree over it is reported as skipped with its own count instead
/// of being walked, and no reachability row is invented for it.
/// 阈值不是装饰：超过它的树会带着自身计数被报成跳过，而不是被硬走一遍，也不会为它编出任何可达性行。
#[test]
fn a_tree_over_the_function_budget_is_reported_as_skipped() {
    let root = reachability_package("over", super::REACHABILITY_BUDGET + 1);
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains("test-reachable: skipped ("), "{lines}");
    assert!(
        lines.contains(&format!("over the limit of {}", super::REACHABILITY_BUDGET)),
        "{lines}"
    );
    assert!(
        !lines.contains("no test reaches `"),
        "no rows are computed for a skipped tree: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A package whose unreachable functions sit in two different directories, and whose root has none.
/// 一个包：够不着的函数分住在两个目录里，而根上一个都没有。
fn split_package(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-claims-split-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for directory in ["src", "src/alpha", "src/beta", "tests"] {
        std::fs::create_dir_all(root.join(directory)).expect("fixture dirs");
    }
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"fixture-split-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A split fixture.\npub mod alpha;\npub mod beta;\n\
         pub fn entry() -> i64 { alpha::alpha_reachable() }\n",
    )
    .expect("fixture root");
    std::fs::write(
        root.join("src/alpha/alpha.rs"),
        "pub fn alpha_reachable() -> i64 { 1 }\npub fn alpha_orphan() -> i64 { 2 }\n",
    )
    .expect("fixture alpha");
    std::fs::write(
        root.join("src/beta/beta.rs"),
        "pub fn beta_orphan() -> i64 { 3 }\n",
    )
    .expect("fixture beta");
    std::fs::write(
        root.join("tests/one.rs"),
        "#[test]\nfn walks() { assert_eq!(fixture_split::entry(), 1); }\n",
    )
    .expect("fixture test");
    root
}

/// The whole table breaks the test-unreachable column down per directory, and the sample does not.
/// 整表把"没有测试能到达"那一栏按目录拆开，而样本不拆。
///
/// Measured need: the column was a count plus five rows, so "which parts of this tree are the
/// unreachable ones in" cost one call per directory (the round that produced `s3` spent 13). Two
/// numbers per directory answer it once, and a directory with nothing unreachable is left out
/// rather than printed as a zero.
/// 量出来的需求：这一栏过去是一个计数加五行，于是"这棵树里够不着的是哪几块"要一个目录一次调用（产出 `s3`
/// 的那轮花了 13 次）。每个目录两个数字一次答完，而没有够不着的函数的目录不印成一个零。
#[test]
fn the_whole_table_splits_the_unreachable_column_per_directory() {
    let root = split_package("split");
    let full = whole(&root).expect("the census answers").join("\n");
    assert!(
        full.contains("by directory: src/alpha 1 of 2 · src/beta 1 of 1"),
        "each directory carries its own two numbers, alphabetically, and `src` has no row: {full}"
    );
    assert!(
        full.contains("no test reaches `alpha_orphan`")
            && full.contains("no test reaches `beta_orphan`"),
        "the rows are still the answer: {full}"
    );
    let sampled = census(&root).expect("the census answers").join("\n");
    assert!(
        !sampled.contains("by directory"),
        "the sample stays a sample: {sampled}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A dead arm's row carries the contract of the function it sits in — the join, not just the fact.
/// 死臂那一行带上它所在函数的契约——是**合并**，不只是那个事实。
///
/// Measured (W8, h4): the census named the dead arm in one answer and `digest` printed the contract's
/// first line in the next one, and the agent spent 31,397 characters of reasoning — 20% of that arm's
/// whole chain — comparing implementations against their documentation by hand. Both halves were
/// already computed here; only the join was missing.
/// 量出来的（W8 的 h4）：总账在一次答案里点名死臂，`digest` 在下一次里印契约首行，而 agent 为此写掉
/// 31,397 字符的推理——占该臂整条链的 20%——手工把实现与文档逐条比对。两半本来都在这里算出来了，
/// 缺的只是合并。
fn documented_dead_arm_package(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-claims-docdebt-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"fixture-docdebt-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A documented dead arm.\n\
         enum Mode { Live, Dead }\n\
         \n\
         /// Which mode decides the answer.\n\
         ///\n\
         /// The second line carries the promise.\n\
         fn decide(mode: Mode) -> bool {\n\
         \x20   match mode {\n\
         \x20       Mode::Live => true,\n\
         \x20       Mode::Dead => false,\n\
         \x20   }\n\
         }\n\
         \n\
         fn only_live() -> bool {\n\
         \x20   decide(Mode::Live)\n\
         }\n",
    )
    .expect("fixture source");
    root
}

#[test]
fn a_dead_arms_row_carries_the_contract_of_its_function() {
    let root = documented_dead_arm_package("contract-note");
    let lines = whole(&root).expect("the census answers").join("\n");
    let row = lines
        .lines()
        .find(|line| line.contains("can never be entered"))
        .unwrap_or_else(|| panic!("no dead-arm row in:\n{lines}"));
    assert!(
        row.contains("the contract above it says"),
        "the arm's row carries the contract of its function: {row}"
    );
    assert!(
        row.contains("Which mode decides the answer."),
        "and the contract text is the function's own, without its `///`: {row}"
    );
    // The note is bounded and says so, so a long doc block cannot push the row past its budget.
    // 注是有界的、并且自己说明了，因此一长段文档不会把这一行撑爆。
    assert!(
        !row.contains("///"),
        "the rendered contract text must not keep its markers, indented or not: {row}"
    );
    // Coupling pin (T-01), and the first version of it was worthless: it computed the expected
    // value by calling `note::numbered` itself, so mutating the outlet moved both sides at once and
    // the assertion stayed green. A **self-referential pin measures nothing**. What has teeth is the
    // literal below (mutate the outlet's joiner and this fails) plus the structural half (the note
    // is rendered by the outlet rather than by a second joiner in this file).
    // 耦合钉子（T-01），而它的第一版毫无价值：它自己调 `note::numbered` 算出期望值，于是变异共享出口时两边
    // 一起变、断言照样绿。**自指的钉子什么也没量。** 有牙的是下面这个**字面量**（把出口的接法变异掉它就会
    // 失败），加上结构那一半（这条注由出口渲染，而不是本文件里第二个拼接器）。
    assert!(
        row.contains(
            "the contract above it says 4: Which mode decides the answer. / 6: The second line \
             carries the promise."
        ),
        "the note is the outlet's output, verbatim: {row}"
    );
    let source = include_str!("claims.rs");
    assert!(
        source.contains("crate::mcp::note::numbered("),
        "this consumer renders the note through the shared outlet"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Per-item census rows speak the layer vocabulary, and one fact about one layer shares a line.
/// 总账的逐项行说同一套层级词表，而**同一层**的同形事实共用一行。
///
/// Measured (audit T-10): the other answers already tag their rows (`face`/`file`/`fn` in `search`
/// and `digest`, the definition's line number in `read`), while the census's per-item rows began with
/// the fact — so one tree came back in two vocabularies, and only one of them said which layer a row
/// was about. And nine `no test reaches \`paint\`` rows repeated a full path each, differing only in
/// the file: the layer's facts now share a line whose prefix is stated once.
/// 量到的（审计 T-10）：别的答案已经给行打了标签（`search`/`digest` 的 `face`/`file`/`fn`，`read` 的定义
/// 行号），而总账的逐项行以事实本身开头——于是同一棵树用两套词汇回来，而只有一套说了这一行讲的是哪一层。
/// 另外九行 `no test reaches \`paint\`` 每行重复一整条路径、只有文件名不同：现在这一层的事实共用一行，
/// 前缀只说一次。
#[test]
fn census_rows_speak_the_layer_vocabulary_and_share_a_line_per_layer() {
    let root = package("layers");
    let lines = whole(&root).expect("the census answers");
    let joined = lines.join("\n");
    for line in &lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("no test names") || trimmed.starts_with("no test reaches") {
            panic!("a per-item row with no layer tag: {line}");
        }
    }
    // A **summary** row names a column (`declarations: 2 production …`) and keeps its prose; a
    // per-item row is one that begins with the fact itself, so the vocabulary is asserted on the
    // tagged rows and the untagged spelling is refused by the loop above.
    // **汇总**行点名一栏（`declarations: 2 production …`）并保留散文；逐项行以事实本身开头——因此这里
    // 断言的是"带标签的行确实存在"，而未带标签的拼法由上面那个循环拒绝。
    let tagged = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("decl ") || trimmed.starts_with("fn ")
        })
        .count();
    assert!(tagged > 0, "the layer vocabulary is in use: {joined}");
    assert!(
        joined.contains("no test reaches"),
        "the column is still reported: {joined}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The 2026-10-10 defect queue, item ①: the entry-plan column counts **code**, so a comment that
/// merely mentions `cut(…)` cannot move it. Measured before the fix, on the real bridge: one two-line
/// prose comment took the column from `8 cut( / 12 graft(` to `10 / 14` with no declaration changed
/// (delivery report §7h).
/// 2026-10-10 缺陷队列第 ① 条：入口计划那一栏数的是**代码**，因此一条只是提到 `cut(…)` 的注释挪不动它。
/// 修之前用真桥实测：一条两行的散文注释让那一栏从 `8 个 cut( / 12 个 graft(` 变成 `10 / 14`，而声明一个
/// 都没变（交付报告 §7h）。
#[test]
fn a_comment_that_mentions_a_cut_does_not_move_the_entry_plan_column() {
    let quiet = package("entry-plan-quiet");
    let loud = package("entry-plan-loud");
    let original = std::fs::read_to_string(loud.join("src/lib.rs")).expect("the fixture source");
    std::fs::write(
        loud.join("src/lib.rs"),
        format!(
            "{original}\n// prose, not a declaration: cut(root/x) and graft(other)\n\
             // 散文而不是声明：这里的 cut(root/x) 与 graft(other) 只是词\n"
        ),
    )
    .expect("the commented fixture");
    let column = |root: &Path| {
        whole(root)
            .expect("the census reads")
            .into_iter()
            .find(|line| line.contains("entry plan:"))
            .expect("the entry-plan column is there")
    };
    assert_eq!(column(&quiet), column(&loud), "a comment is not a cut site");
    assert!(
        column(&quiet).contains("0 `cut(`") && column(&quiet).contains("0 `graft(`"),
        "and this fixture declares none, so the number is the code's: {}",
        column(&quiet)
    );
    let _ = std::fs::remove_dir_all(&quiet);
    let _ = std::fs::remove_dir_all(&loud);
}
