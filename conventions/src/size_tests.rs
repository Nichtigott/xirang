//! Tests for the file-size gate.
//! 文件尺寸门禁的测试。

use super::*;
use crate::workspace_root;
use std::fs;
use std::path::PathBuf;

/// The measured set of oversized files equals the pinned debt exactly.
/// 实测的超标文件集合与钉住的欠账完全一致。
#[test]
fn the_size_ceiling_holds_except_for_the_pinned_debt() {
    let root = workspace_root();
    let actual = oversized(&root);
    let mut expected: Vec<(String, usize)> = BASELINE
        .iter()
        .map(|(path, count)| ((*path).to_owned(), *count))
        .collect();
    expected.sort();
    let new: Vec<_> = actual
        .iter()
        .filter(|entry| !expected.contains(entry))
        .collect();
    let stale: Vec<_> = expected
        .iter()
        .filter(|entry| !actual.contains(entry))
        .collect();
    assert!(
        new.is_empty(),
        "new files over their budget ({CEILING} code lines of source, {TEST_CEILING} for a \
         `#[cfg(test)]` mount); split them: {new:#?}"
    );
    assert!(
        stale.is_empty(),
        "these entries are stale or their file changed size; update BASELINE \
         (an entry that shrank back under the ceiling must be removed): {stale:#?}"
    );
}

/// A throwaway workspace with one member, `probe`, holding the given files.
/// 一个只含一个成员 `probe` 的一次性工作区，成员下含给定文件。
///
/// The manifest deliberately carries a `[workspace.package] name` right after the
/// members array: a reader that took that section for members would build a crate
/// directory out of a borrowed name.
/// 清单刻意在 members 数组之后紧跟一个 `[workspace.package] name`：把那一节当成成员的读者
/// 会用一个借来的名字造出一个 crate 目录。
fn synthetic(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("xirang-size-{}-{sequence}", std::process::id()));
    fs::create_dir_all(root.join("probe/src")).expect("fixture src");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"probe\"]\nresolver = \"2\"\n\n\
         [workspace.package]\nname = \"borrowed\"\nedition = \"2024\"\n",
    )
    .expect("fixture manifest");
    for (relative, contents) in files {
        let path = root.join("probe").join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        fs::write(&path, contents).expect("fixture file");
    }
    root
}

/// Filler that pushes a fixture past [`CEILING`] but not past [`TEST_CEILING`].
/// 把夹具推过 [`CEILING`] 但不超过 [`TEST_CEILING`] 的填充内容。
///
/// The filler is *code* lines rather than comment lines: the budget counts code lines, so a
/// file of `// filler` measures as the empty file it is, and a fixture built from comments
/// would prove nothing. The two budgets differ on purpose, and a fixture between them is how
/// these tests tell which budget a file was measured against.
/// 填充的是**代码**行而不是注释行：预算按代码行计，因此一整文件 `// filler` 会度量成它本来的空
/// 文件，用注释搭出来的夹具什么也证明不了。两份预算有意不同，而落在两者之间的夹具正是这些测试用来
/// 分辨文件按哪份预算度量的手段。
fn over_ceiling() -> String {
    "pub fn filler() {}\n".repeat(CEILING + 10)
}

/// Filler that pushes a fixture past [`TEST_CEILING`] as well.
/// 把夹具也推过 [`TEST_CEILING`] 的填充内容。
fn over_test_ceiling() -> String {
    "pub fn filler() {}\n".repeat(TEST_CEILING + 10)
}

/// Location is not proof of testhood, and a proof of testhood is not an exemption: a file
/// under `src/tests/` with no `#[cfg(test)]` mount is measured against [`CEILING`], the same
/// file mounted behind `#[cfg(test)]` is measured against the wider [`TEST_CEILING`], and a
/// file past *that* is debt like any other. Audit G-05 stopped location from being proof; the
/// budget model keeps the mount from being an escape.
/// 位置不是"仅测试"的证明，而"仅测试"的证明也不是豁免：`src/tests/` 下没有 `#[cfg(test)]`
/// 挂载的文件按 [`CEILING`] 度量，同一个文件挂在 `#[cfg(test)]` 之后则按更宽的 [`TEST_CEILING`]
/// 度量，而越过**后者**的文件与别的文件一样是欠账。审计 G-05 让位置不再是证明；预算模型则让挂载
/// 不再是逃生口。
#[test]
fn a_file_under_src_tests_is_measured_against_the_budget_its_mount_names() {
    // 610 code lines: over `CEILING`, under `TEST_CEILING`.
    let root = synthetic(&[
        ("src/lib.rs", "pub mod tests;\n"),
        (
            "src/tests.rs",
            "#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_ceiling()),
    ]);
    assert!(
        oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "nothing says this file is a test, so the source budget measures it"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/lib.rs", "pub mod tests;\n"),
        (
            "src/tests.rs",
            "#[cfg(test)]\n#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_ceiling()),
    ]);
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "a `#[cfg(test)]` declaration moves it to the wider test budget"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/lib.rs", "pub mod tests;\n"),
        (
            "src/tests.rs",
            "#[cfg(test)]\n#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_test_ceiling()),
    ]);
    assert!(
        oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "a test file is budgeted, not exempt: past `TEST_CEILING` it is reported"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/lib.rs", "#[cfg(test)]\npub mod tests;\n"),
        (
            "src/tests.rs",
            "#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_ceiling()),
    ]);
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "the attribute is inherited down the mount chain"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The recursion bound is a cycle stop with headroom, not a description of this tree's depth.
/// The deepest chain the tree ships is six files (five mount edges): `kernel/src/lib.rs` →
/// `registry_core.rs` → `authoring/authoring.rs` → `authoring/parse/parse.rs` →
/// `authoring/parse/flow.rs` → `authoring/parse/flow_tests.rs`. An eleven-file chain below one
/// `#[cfg(test)]` mount must still be read as test-only — measured against the test budget —
/// because a bound that merely fits today's tree reclassifies a test file as measured source
/// the next time someone nests two more modules.
/// 递归上限是带余量的防环，而不是对本树深度的描述。本树出厂的最深链有 6 个文件（5 条挂载边）：
/// `kernel/src/lib.rs` → `registry_core.rs` → `authoring/authoring.rs` → `authoring/parse/parse.rs`
/// → `authoring/parse/flow.rs` → `authoring/parse/flow_tests.rs`。一条挂在单个 `#[cfg(test)]`
/// 之下的十一个文件的链必须仍被读成仅测试——按测试预算度量——因为"刚好装下今天的树"的上限会在下次
/// 有人再嵌两层模块时，把一个测试文件重新归类成被测源码。
#[test]
fn a_deep_mount_chain_is_still_test_only() {
    let mut files: Vec<(String, String)> = vec![(
        "src/lib.rs".to_owned(),
        "#[cfg(test)]\npub mod a;\n".to_owned(),
    )];
    for letter in b'a'..=b'j' {
        let name = (letter as char).to_string();
        let body = if letter == b'j' {
            over_ceiling()
        } else {
            let next = (letter + 1) as char;
            format!("#[path = \"{next}.rs\"]\npub mod {next};\n")
        };
        files.push((format!("src/{name}.rs"), body));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect();
    let root = synthetic(&borrowed);
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("j.rs")),
        "a ten-level chain below a `#[cfg(test)]` mount is still a test file"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Test-only files are recognised by where they sit, or by their name together
/// with a `#[cfg(test)]` mount.
/// 仅测试文件按所在位置识别，或按名字加 `#[cfg(test)]` 挂载识别。
#[test]
fn test_only_files_are_recognised() {
    assert!(is_test_shaped(Path::new("toolchain/src/cli/lib_tests.rs")));
    assert!(is_test_shaped(Path::new(
        "kernel/src/registry_core/syntax/face_tests.rs"
    )));
    assert!(is_test_shaped(Path::new(
        "toolchain/src/studio/studio/app/tests.rs"
    )));
    assert!(!is_test_shaped(Path::new(
        "kernel/src/registry_core/syntax/face.rs"
    )));
    assert!(is_test_by_location(Path::new(
        "toolchain/src/studio/tests/graph.rs"
    )));
    assert!(!is_test_by_location(Path::new(
        "toolchain/src/studio/studio/app/tests.rs"
    )));
}

/// A name is not proof: real code renamed to `x_tests.rs` and never mounted is
/// measured like anything else, which is the hole this closed.
/// 名字不是证明：把真实代码改名成 `x_tests.rs` 却从不挂载，会像别的文件一样被度量——这正是
/// 被堵上的那个洞。
#[test]
fn an_unmounted_test_shaped_file_is_measured() {
    let root = synthetic(&[("src/probe_tests.rs", &over_ceiling())]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "an unmounted `_tests` file is source like any other: {found:#?}"
    );
    assert!(found[0].0.ends_with("src/probe_tests.rs"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// Mounted behind `#[cfg(test)]`, the same file moves to the wider test budget — under both
/// spellings this tree uses — and past that budget it is reported like anything else.
/// 挂在 `#[cfg(test)]` 之后，同一个文件转到更宽的测试预算——本树使用的两种拼法都算——而越过
/// 那份预算时它和别的文件一样会被报出来。
#[test]
fn a_mounted_test_file_is_measured_against_the_test_budget() {
    let root = synthetic(&[
        (
            "src/probe.rs",
            "#[cfg(test)]\n#[path = \"probe_tests.rs\"]\nmod probe_tests;\n",
        ),
        ("src/probe_tests.rs", &over_ceiling()),
    ]);
    assert_eq!(
        oversized(&root),
        Vec::new(),
        "a mounted `#[path]` test module fits the wider test budget"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/tests.rs", &over_ceiling()),
        ("src/lib.rs", "#[cfg(test)]\nmod tests;\n"),
    ]);
    assert_eq!(
        oversized(&root),
        Vec::new(),
        "the bare `mod tests;` form uses that budget too"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        (
            "src/probe.rs",
            "#[cfg(test)]\n#[path = \"probe_tests.rs\"]\nmod probe_tests;\n",
        ),
        ("src/probe_tests.rs", &over_test_ceiling()),
    ]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "a test file past `TEST_CEILING` is debt, not an exemption: {found:#?}"
    );
    assert!(found[0].0.ends_with("src/probe_tests.rs"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The crate-root `build.rs` is measured like `src/`.
/// crate 根的 `build.rs` 与 `src/` 一同被度量。
#[test]
fn the_crate_root_build_script_is_measured() {
    let root = synthetic(&[("build.rs", &over_ceiling())]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "an oversized build script is debt like any other: {found:#?}"
    );
    assert!(found[0].0.ends_with("build.rs"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The ceilings are the maintainer's decision, not a drift target: 500 code lines is accepted
/// under both, a source file past 600 is debt, and a test file is accepted up to the wider
/// number. Pinned so the 2026-09-28 relaxation from 450 to 600 and the separate test budget
/// cannot be undone or exceeded silently.
/// 上限是维护者的决定，不是可以漂移的目标：500 代码行在两份预算下都被接受，超过 600 的源码文件
/// 是欠账，而测试文件被接受到那个更宽的数字为止。钉住它，使 2026-09-28 从 450 到 600 的放宽与
/// 独立的测试预算不会被静默撤销或被静默越界。
#[test]
fn the_ceilings_are_the_numbers_the_maintainer_set() {
    assert_eq!(
        CEILING, 600,
        "the ratchet's ceiling is the maintainer's decision of 2026-09-28"
    );
    assert_eq!(
        TEST_CEILING, 800,
        "the test budget is its own, wider number rather than the source ceiling or an exemption"
    );
    let accepted = "pub fn filler() {}\n".repeat(CEILING - 100);
    let root = synthetic(&[("src/probe.rs", &accepted)]);
    assert_eq!(
        oversized(&root),
        Vec::new(),
        "500 code lines sits under the 600-line ceiling"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[("src/probe.rs", &over_ceiling())]);
    assert_eq!(
        oversized(&root).len(),
        1,
        "past the ceiling is still measured debt"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The budget counts code lines, not stored lines: a blank line and a comment-only line are
/// not code, a line of code is, and the interior of a multi-line string is the one boundary
/// the mask does not cover.
/// 预算按代码行而不是存储行计：空行与纯注释行不是代码，代码行是，而多行字符串的内部是掩码不覆盖
/// 的那一处边界。
#[test]
fn the_budget_counts_code_lines() {
    let text = "// comment\n/// doc\n//! module\n\nfn code() {}\n/* block\n * inside\n */\n";
    assert_eq!(
        code_lines(text),
        1,
        "only `fn code() {{}}` is code in this fixture"
    );

    let trailing = "let x = 1; // trailing comment\nlet y = 2;\n";
    assert_eq!(code_lines(trailing), 2, "a trailing comment is not a line");

    // The documented boundary: the mask blanks string literals, so the interior lines of a
    // multi-line string are neither code nor comment.
    // 记录在案的边界：掩码会抹掉字符串字面量，因此多行字符串的内部行既不是代码也不是注释。
    let string = "let s = \"first\n// not a comment\nsecond\";\n";
    assert_eq!(
        code_lines(string),
        2,
        "the string's opening and closing lines are code; its interior is neither"
    );
}

/// Every spelling rustc accepts for a `#[cfg(test)]` mount is the same mount, and the
/// budget has to read all of them. Comparing the declaration against two literal
/// strings read `#[cfg(all(test))]` and `pub(crate) mod x;` as *not* test mounts, so a
/// test file under either spelling was measured as source — a debt report the author
/// cannot act on, on a file the gate had already agreed uses the test budget under the
/// orthodox spelling. Audit `G-04`.
/// rustc 接受的每一种 `#[cfg(test)]` 挂载拼法都是同一个挂载，而预算必须都能读出来。把声明与
/// 两个字面串比较，会把 `#[cfg(all(test))]` 与 `pub(crate) mod x;` 读成**不是**测试挂载，于是
/// 这两种拼法下的测试文件被当作源码度量——在门禁已按正统拼法同意使用测试预算的同一个文件上，报出
/// 一笔作者无法处理的欠账。审计 `G-04`。
#[test]
fn every_spelling_of_a_test_mount_uses_the_test_budget() {
    let filler = over_ceiling();
    let root = synthetic(&[
        (
            "src/lib.rs",
            "#[cfg(test)]\n#[path = \"big_tests.rs\"]\nmod big_tests;\n\
             #[cfg(all(test))]\n#[path = \"big2_tests.rs\"]\nmod big2_tests;\n\
             #[cfg(test)]\npub(crate) mod big3_tests;\n",
        ),
        ("src/big_tests.rs", &filler),
        ("src/big2_tests.rs", &filler),
        ("src/big3_tests.rs", &filler),
    ]);
    let found = oversized(&root);
    assert_eq!(
        found,
        Vec::new(),
        "all three spellings are the same mount: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A `#[cfg(not(test))]` mount is the opposite of a test mount, and `all`/`any` are
/// not interchangeable: `all(test, feature = "x")` is only compiled in a test build,
/// while `any(test, feature = "x")` is compiled in an ordinary one too.
/// `#[cfg(not(test))]` 挂载是测试挂载的反面，而 `all`/`any` 不可互换：
/// `all(test, feature = "x")` 只在测试构建里编译，而 `any(test, feature = "x")` 在普通构建里
/// 也会编译。
#[test]
fn a_negative_or_conditional_cfg_is_not_a_test_mount() {
    let filler = over_ceiling();
    for (spelling, under_the_test_budget) in [
        ("#[cfg(all(test))]", true),
        ("#[cfg(all(test, feature = \"probe\"))]", true),
        ("#[cfg(test)]", true),
        ("#[cfg(not(test))]", false),
        ("#[cfg(any(test, feature = \"probe\"))]", false),
        ("#[cfg(feature = \"probe\")]", false),
    ] {
        let root = synthetic(&[
            ("src/lib.rs", &format!("{spelling}\nmod probe;\n")),
            ("src/probe.rs", &filler),
        ]);
        let found = oversized(&root);
        assert_eq!(
            found.len(),
            if under_the_test_budget { 0 } else { 1 },
            "`{spelling}` must {} the test budget: {found:#?}",
            if under_the_test_budget {
                "use"
            } else {
                "not use"
            }
        );
        let _ = fs::remove_dir_all(&root);
    }
}

/// The ratchet measures the file rustfmt writes, not the bytes on disk, because those
/// two are not the same file and the one that is shorter is the one that hides debt.
/// rustfmt splits the long lines it is handed, so a file written with several statements
/// per line is *under* the ceiling as stored and *over* it as read: the B1 round measured
/// `mutations.rs` at 586 lines while rustfmt makes it 616.
/// 棘轮量的是 rustfmt 写出的文件，而不是磁盘上的字节，因为两者不是同一个文件，而更短的那个正是
/// 藏住欠账的那个。rustfmt 会切开交给它的长行，因此一行里塞了几条语句的文件在**存储形态**下位于
/// 上限之内、在**阅读形态**下越过上限：B1 轮实测 `mutations.rs` 是 586 行，而 rustfmt 会把它
/// 写成 616 行。
#[test]
fn an_unformatted_file_does_not_hide_the_ratchet() {
    // Statements packed onto one line, which is the shape rustfmt rewrites.
    // 挤在一行上的语句，正是 rustfmt 会重写的形状。
    let mut packed = String::from("fn probe() {\n    ");
    for index in 0..700 {
        packed.push_str(&format!("let v{index} = {index}; "));
    }
    packed.push_str("\n}\n");
    assert!(
        packed.lines().count() < CEILING,
        "the fixture is under the ceiling as written, which is what hides it"
    );
    let root = synthetic(&[("src/probe.rs", &packed)]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "the file rustfmt writes is over the ceiling: {found:#?}"
    );
    assert!(
        found[0].1 > CEILING,
        "the reported size is the formatted one: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The claim the comment above [`declared_stem`] makes, held to the code.
/// [`declared_stem`] 上面那句注释所做的断言，与实现对齐。
///
/// The branch exists for spellings rustc accepts, and the failure it guards is a *test* file
/// measured as source: `mod  x ;` did that, because the token join left the space before the `;`
/// in place (audit `G-07`, finding `F-2`). The pin lives here rather than in `size.rs` because a
/// test-only file is measured against the test budget this gate enforces — the assertion is about
/// a private function, and a sibling module sees it.
/// 这条分支是为 rustc 接受的拼法而存在的，而它守的失败是**测试**文件被当作源码度量：
/// `mod  x ;` 会这样，因为那次词元连接把 `;` 前的空格留在了原地（审计 `G-07`，发现 `F-2`）。
/// 钉子放在这里而不是 `size.rs`：仅测试文件按本门禁执行的测试预算度量——断言的对象是私有函数，
/// 而兄弟模块能看见它。
#[test]
fn a_space_before_the_semicolon_is_the_same_declaration() {
    assert_eq!(declared_stem("mod x;").as_deref(), Some("x"));
    assert_eq!(declared_stem("mod  x ;").as_deref(), Some("x"));
    assert_eq!(declared_stem("pub(crate) mod  x ;").as_deref(), Some("x"));
    assert_eq!(declared_stem("pub(super) mod x;").as_deref(), Some("x"));
}

/// A longer identifier that shares the keyword's prefix is not a declaration, and neither is an
/// inline `mod x { … }`, which mounts no file.
/// 仅仅与关键字同前缀的更长标识符不是声明，内联的 `mod x { … }` 也不是——它不挂载任何文件。
#[test]
fn a_longer_identifier_is_not_the_declaration() {
    assert_eq!(declared_stem("modx;"), None);
    assert_eq!(declared_stem("module x;"), None);
    assert_eq!(declared_stem("mod x {}"), None);
}

/// The declaration index is a cache, and a cache that answered for a tree somebody has already rewritten would be a
/// gate that measures a tree no longer on disk. Fixture crates are therefore rebuilt on every question: changing a
/// fixture's mount and asking again gets the new answer.
/// 声明索引是一份缓存，而"替一棵已经被改写过的树作答"的缓存，等于一道在量一棵已不在磁盘上的树的门禁。因此夹具
/// crate 每次都重建：改掉夹具的挂载再问一次，拿到的就是新答案。
#[test]
fn a_fixture_crates_answer_follows_its_current_bytes() {
    let root = synthetic(&[
        ("src/lib.rs", "pub mod plain;\n"),
        ("src/plain.rs", &over_ceiling()),
    ]);
    // A plain `mod` is source, so the oversized file is measured.
    // 普通的 `mod` 是源码，因此那个超限文件会被量到。
    assert!(
        oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("plain.rs")),
        "a plain module is measured"
    );
    // Mount it under `#[cfg(test)]` and ask again: the same path, new bytes, new answer.
    // 把它挂到 `#[cfg(test)]` 之下再问一次：同一个路径、新的字节、新的答案。
    std::fs::write(
        root.join("probe/src/lib.rs"),
        "#[cfg(test)]\npub mod plain;\n",
    )
    .expect("rewrite the mount");
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("plain.rs")),
        "the answer follows the current bytes, not a cached one"
    );
    let _ = std::fs::remove_dir_all(root);
}
