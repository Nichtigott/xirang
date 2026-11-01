//! Pins for the durable verification-task store: the manifest, the node fold and the journal.
//! 持久验证任务存储的钉子：清单、摘要折叠与 journal。

use std::path::Path;

use super::{Inputs, files, newest_for_task, phase, tail, task_key, writer_alive};

/// A throwaway directory with the given files.
/// 一个带给定文件的一次性目录。
fn fixture(label: &str, entries: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-task-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (relative, text) in entries {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// The manifest is the durable half of the identity, so it must round-trip **and** fold to the
/// same digest it had in memory — a manifest that reads back differently would silently
/// invalidate every plan written before the process restarted.
/// 清单是身份中持久的那一半，因此它必须能原样往返，**并且**折叠回与内存中相同的摘要——读回来就变样的
/// 清单会静默作废进程重启之前写下的每一份计划。
#[test]
fn a_manifest_round_trips_and_keeps_its_digest() {
    let root = fixture(
        "roundtrip",
        &[
            ("src/lib.rs", "pub fn a() {}\n"),
            ("Cargo.toml", "[package]\nname=\"x\"\n"),
            ("notes.md", "a file like any other\n"),
        ],
    );
    let inputs = Inputs::capture(&root, None).unwrap();
    assert_eq!(
        inputs.count(),
        3,
        "every regular file is an input: there is no extension whitelist (BR-A)"
    );
    let path = root.join("manifest.tsv");
    inputs.write(&path).unwrap();
    let read = Inputs::read(&path).unwrap();
    assert_eq!(read.count(), inputs.count());
    assert_eq!(
        read.digest(),
        inputs.digest(),
        "a manifest read back from disk describes the same bytes"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A missing manifest is an error, never "no inputs": treating absence as emptiness would let
/// a plan claim its inputs still match when it has nothing to compare against.
/// 清单缺失是错误，绝不是"没有输入"：把缺席当成空会让一份计划在无从比较时声称输入仍然匹配。
#[test]
fn a_missing_manifest_is_an_error() {
    let root = fixture("missing", &[("src/lib.rs", "\n")]);
    let error = Inputs::read(&root.join("nothing.tsv")).expect_err("absence is not emptiness");
    assert!(error.contains("cannot read"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// The diff names added, removed and modified inputs — that is what turns "the digest differs"
/// into "these bytes changed".
/// 差分点名新增、删除与改动的输入——正是它把"摘要不同"变成"这些字节变了"。
#[test]
fn the_diff_names_what_changed() {
    let root = fixture("diff", &[("src/lib.rs", "a\n"), ("src/gone.rs", "b\n")]);
    let before = Inputs::capture(&root, None).unwrap();
    std::fs::write(root.join("src/lib.rs"), "changed\n").unwrap();
    std::fs::remove_file(root.join("src/gone.rs")).unwrap();
    std::fs::write(root.join("src/new.rs"), "c\n").unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    let changes = before.changes(&after);
    assert!(
        changes.iter().any(|line| line == "modified src/lib.rs"),
        "{changes:?}"
    );
    assert!(
        changes.iter().any(|line| line == "removed  src/gone.rs"),
        "{changes:?}"
    );
    assert!(
        changes.iter().any(|line| line == "added    src/new.rs"),
        "{changes:?}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// An excluded output directory is not an input, even when the run put files there.
/// 被排除的输出目录不是输入，即使这次运行往那里放了文件。
#[test]
fn the_excluded_directory_is_not_an_input() {
    let root = fixture("excluded", &[("src/lib.rs", "a\n")]);
    std::fs::create_dir_all(root.join("build/xirang")).unwrap();
    std::fs::write(root.join("build/xirang/ledger.json"), "{}\n").unwrap();
    let all = Inputs::capture(&root, None).unwrap();
    let excluded = Inputs::capture(&root, Some(&root.join("build"))).unwrap();
    assert_eq!(excluded.count(), all.count() - 1);
    assert_eq!(
        excluded.changes(&all),
        vec!["added    build/xirang/ledger.json".to_owned()],
        "the excluded directory is the whole difference, and it is named"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The task key covers what decides the work and ignores what only shapes the reply: a second
/// call with more verbosity finds the same plan, a different face does not.
/// 任务键覆盖决定工作的参数、忽略只影响回复形状的参数：更啰嗦的第二次调用找到同一份计划，换一个面则不然。
#[test]
fn the_task_key_ignores_presentation_and_keeps_the_decision() {
    let base = serde_json::json!({"files": ["src/lib.rs"], "face": "default"});
    let same = serde_json::json!({
        "files": ["src/lib.rs"], "face": "default",
        "verbose": true, "timeout_ms": 1000, "target_dir": "elsewhere"
    });
    let other = serde_json::json!({"files": ["src/lib.rs"], "face": "all"});
    assert_eq!(task_key(&base), task_key(&same));
    assert_ne!(task_key(&base), task_key(&other));
}

/// The journal is append-only and its tail is the last thing written, which is what a resume
/// reads to decide whether anyone is still inside the plan.
/// journal 是只追加的，而它的尾巴是最后写下的东西——恢复正是读它来判断还有没有人在这份计划里。
#[test]
fn the_journal_tail_is_the_last_record() {
    let root = fixture("journal", &[("src/lib.rs", "\n")]);
    let plan = files(&root, "7-8-9");
    assert!(tail(&plan.journal).is_none());
    note(&plan.journal, phase("7-8-9", "planned"));
    note(&plan.journal, phase("7-8-9", "completed"));
    let last = tail(&plan.journal).unwrap();
    assert_eq!(last["phase"], "completed");
    assert_eq!(last["pid"], std::process::id());
    let _ = std::fs::remove_dir_all(root);
}

fn note(journal: &Path, record: serde_json::Value) {
    super::note(journal, record).expect("the journal appends");
}

/// A record's writer is alive only when the process directory says so; this process is a
/// known-alive pid and a very high one is a known-dead one on Linux.
/// 一条记录的写者只有在进程目录这么说时才算活着；在 Linux 上本进程是已知活着的 pid，而一个极大的 pid
/// 是已知死掉的。
#[test]
fn writer_liveness_reads_the_process_table() {
    assert_eq!(writer_alive(std::process::id() as u64), Some(true));
    #[cfg(target_os = "linux")]
    {
        assert_eq!(writer_alive(1), Some(true), "init always exists");
        assert_eq!(writer_alive(999_999), Some(false));
    }
}

/// The newest ledger for a task key wins, so a resumed task continues the plan its own last run
/// wrote rather than an older one.
/// 同一任务键下最新的台账胜出，因此被恢复的任务继续的是它自己最后一次运行写下的计划，而不是更早的那份。
#[test]
fn the_newest_plan_for_a_task_wins() {
    let root = fixture("newest", &[("src/lib.rs", "\n")]);
    let directory = root.join("target/xirang/verification");
    std::fs::create_dir_all(&directory).unwrap();
    for (id, key) in [("1-1-1", "aaaa"), ("2-2-2", "aaaa"), ("3-3-3", "bbbb")] {
        std::fs::write(
            directory.join(format!("{id}.json")),
            serde_json::to_vec(&serde_json::json!({"id": id, "task": key})).unwrap(),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let found = newest_for_task(&directory, "aaaa").unwrap();
    assert_eq!(found.file_name().unwrap(), "2-2-2.json");
    assert_eq!(
        newest_for_task(&directory, "cccc").map(|path| path.file_name().unwrap().to_owned()),
        None
    );
    let _ = std::fs::remove_dir_all(root);
}

/// BR-A: a `.txt` input is an input. This closure used to bind only `rs|toml|lock|tsv|json`, so editing a text
/// fixture left the identity untouched and a stale verdict looked reusable — the false green this path exists to
/// prevent (spec `basic-repair-efficiency.md` §5 BR-A forbids issuing a credential from an extension whitelist).
/// BR-A：`.txt` 输入就是输入。这个闭包过去只绑 `rs|toml|lock|tsv|json`，于是改一份文本夹具时身份纹丝不动、
/// 陈旧判定看起来仍可复用——正是这条路径存在的目的所要防的假绿（规格 §5 BR-A 禁止用扩展名白名单签发凭据）。
#[test]
fn a_txt_input_changes_the_closure() {
    let root = fixture("txt", &[("src/lib.rs", "a\n"), ("cases/case.txt", "one\n")]);
    let before = Inputs::capture(&root, None).unwrap();
    std::fs::write(root.join("cases/case.txt"), "two\n").unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    assert_ne!(
        before.digest(),
        after.digest(),
        "a text fixture is part of the closure"
    );
    assert!(
        before
            .changes(&after)
            .iter()
            .any(|line| line == "modified cases/case.txt"),
        "and the diff names it"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// BR-A: an input with **no** extension is an input — scripts, `include!`d inputs and generated files are
/// exactly the shapes an extension whitelist drops.
/// BR-A：**没有**扩展名的输入也是输入——脚本、被 `include!` 的输入与生成文件，恰恰是扩展名白名单会丢掉的形状。
#[test]
fn an_extensionless_input_changes_the_closure() {
    let root = fixture("noext", &[("src/lib.rs", "a\n"), ("scripts/gen", "one\n")]);
    let before = Inputs::capture(&root, None).unwrap();
    assert_eq!(before.count(), 2, "the extensionless file is bound");
    std::fs::write(root.join("scripts/gen"), "two\n").unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    assert_ne!(before.digest(), after.digest(), "its bytes are bound");
    let _ = std::fs::remove_dir_all(root);
}

/// BR-A: **every** environment variable is an input to a verdict, not only the declared prefixes.
/// BR-A：**每个**环境变量都是判定的输入，不只是声明过的那些前缀。
///
/// This pin was written the other way round first, and that version is what let the false green in: it
/// asserted that a variable outside the prefix list "is not bound, so it does not invalidate a verdict".
/// Measured against the product (`docs/managed-maintenance/review-2026-10-17/freshness-results.json`): a
/// `#[test]` reading `XR_REVIEW_MODE` passed under `good`, and `check {reuse:true}` under `bad` answered
/// `passed (reused; nothing was run)` while the same command run directly failed. So the two digests now
/// answer two questions, and this pin holds both of them: the **tree** identity keeps ignoring the shell
/// (a tree does not move because a variable did), and the **verification** identity binds the whole
/// environment.
/// 这道钉子第一版写的是反的，而正是那一版放进了那次假绿：它断言清单之外的变量"不被绑定，因此不会让判定失效"。
/// 对着产品实测（`docs/managed-maintenance/review-2026-10-17/freshness-results.json`）：一条读 `XR_REVIEW_MODE`
/// 的 `#[test]` 在 `good` 下通过，而在 `bad` 下 `check {reuse:true}` 回答 `passed (reused; nothing was run)`，
/// 而同一条命令直接执行却失败。因此现在两个摘要回答两个问题，这道钉子把两个都钉住：**树**身份照旧不受 shell
/// 影响（一棵树不会因为一个变量而移动），而**验证**身份绑定整个环境。
#[test]
fn an_environment_input_changes_the_closure() {
    let root = fixture("env", &[("src/lib.rs", "a\n")]);
    let before =
        Inputs::capture_with_env(&root, None, [("RUSTFLAGS".to_owned(), "".to_owned())]).unwrap();
    let same =
        Inputs::capture_with_env(&root, None, [("RUSTFLAGS".to_owned(), "".to_owned())]).unwrap();
    let changed = Inputs::capture_with_env(
        &root,
        None,
        [("RUSTFLAGS".to_owned(), "-Cdebug".to_owned())],
    )
    .unwrap();
    assert_eq!(
        before.digest(),
        same.digest(),
        "the legal control: the same environment still compares equal"
    );
    assert_ne!(
        before.digest(),
        changed.digest(),
        "a different RUSTFLAGS is a different closure"
    );
    assert!(
        before
            .changes(&changed)
            .iter()
            .any(|line| line == "modified env:RUSTFLAGS"),
        "and the diff says which variable moved"
    );
    let unrelated = Inputs::capture_with_env(
        &root,
        None,
        [
            ("RUSTFLAGS".to_owned(), "".to_owned()),
            ("HOME".to_owned(), "/tmp/one".to_owned()),
        ],
    )
    .unwrap();
    // The tree did not move: a `HOME` the shell happens to carry is not a file in this package.
    // 树没有移动：shell 恰好带着的 `HOME` 不是这个包里的文件。
    assert_eq!(
        before.digest(),
        unrelated.digest(),
        "the tree identity binds bytes, not the shell that ran the command"
    );
    // The verdict did move: any variable may be read by a build script or a test, so the verification
    // identity must change. This is the half the first version of this pin got wrong.
    // 判定移动了：任何变量都可能被构建脚本或测试读到，因此验证身份必须变。这正是这道钉子第一版弄错的那一半。
    assert_ne!(
        before.verification_digest(),
        unrelated.verification_digest(),
        "every environment variable is an input to a verdict, including one the manifest does not name"
    );
    assert_eq!(
        before.verification_digest(),
        same.verification_digest(),
        "the legal control: the same whole environment still compares equal"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// BR-A: an entry the walk cannot bind makes the closure **incomplete** rather than silently absent, and an
/// incomplete closure is never presented as reusable.
/// BR-A：遍历**绑不上**的条目让闭包**不完整**，而不是被静默漏掉；不完整的闭包绝不被当作可复用呈现。
#[cfg(unix)]
#[test]
fn an_unbindable_entry_makes_the_closure_incomplete() {
    let root = fixture("incomplete", &[("src/lib.rs", "a\n")]);
    let socket = root.join("run/socket");
    std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
    let _bound = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let inputs = Inputs::capture(&root, None).unwrap();
    let reason = inputs
        .incomplete_reason()
        .expect("a socket cannot be bound");
    assert!(reason.contains("run/socket"), "{reason}");
    let path = root.join("manifest.tsv");
    inputs.write(&path).unwrap();
    assert_eq!(
        Inputs::read(&path).unwrap().digest(),
        inputs.digest(),
        "incompleteness travels with the manifest, so a reader sees it too"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// BR01: a link the walk cannot resolve is **unknown**, not absent. A dangling link and a link to a directory both
/// make the closure incomplete, and the manifest carries the reason — so a reader of the manifest sees what the
/// closure does not cover instead of a shorter list that looks complete.
/// BR01：遍历解不开的链接是**未知**，而不是不存在。悬空链接与指向目录的链接都让闭包**不完整**，清单会带着理由——
/// 因此读清单的人看到的是闭包没覆盖什么，而不是一份看起来完整、其实更短的清单。
#[cfg(unix)]
#[test]
fn a_dangling_or_directory_link_makes_the_closure_incomplete() {
    let root = fixture("links", &[("src/lib.rs", "pub fn a() -> i64 { 1 }\n")]);
    std::fs::create_dir_all(root.join("elsewhere")).unwrap();
    std::fs::write(root.join("elsewhere/mod.rs"), "pub fn b() -> i64 { 2 }\n").unwrap();
    std::os::unix::fs::symlink(root.join("nowhere.txt"), root.join("src/dangling.rs")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), root.join("src/aliased")).unwrap();
    let inputs = Inputs::capture(&root, None).unwrap();
    let reason = inputs
        .incomplete_reason()
        .expect("a link the walk cannot resolve is not bound");
    assert!(reason.contains("src/dangling.rs"), "{reason}");
    assert!(reason.contains("src/aliased"), "{reason}");
    let path = root.join("manifest.tsv");
    inputs.write(&path).unwrap();
    assert_eq!(
        Inputs::read(&path).unwrap().digest(),
        inputs.digest(),
        "incompleteness travels with the manifest, so a reader sees it too"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// BR01: bytes reached through a symlink are inputs too, including when the link leaves the root. Binding
/// the link's target path alone let a root-external file change under a passing verdict: the link still
/// pointed at the same place, so the closure said "unchanged" after the input changed.
/// BR01：经符号链接到达的字节也是输入，包括链接指向根外时。只绑链接的目标路径会让根外文件在判定通过之后
/// 改变：链接还指向同一个地方，闭包却说"没变"。
#[cfg(unix)]
#[test]
fn bytes_reached_through_a_symlink_are_part_of_the_closure() {
    let root = fixture("symlink-bytes", &[("src/lib.rs", "a\n")]);
    let outside = root.parent().unwrap().join(format!(
        "{}-outside",
        root.file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&outside, "good\n").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("data.txt")).unwrap();
    let before = Inputs::capture(&root, None).unwrap();
    std::fs::write(&outside, "bad\n").unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    assert_ne!(
        before.digest(),
        after.digest(),
        "the bytes behind the link are an input: changing them changes the closure"
    );
    assert_ne!(
        before.verification_digest(),
        after.verification_digest(),
        "and therefore changes the identity a verdict would be bound to"
    );
    let changes = before.changes(&after);
    assert!(
        changes.iter().any(|line| line == "modified data.txt"),
        "{changes:?}"
    );
    // A link that resolves to a directory is not followed, so its contents cannot be bound: the closure
    // must say so rather than claim completeness.
    // 解析到目录的链接不被跟随，因此它的内容绑不上：闭包必须说出来，而不是声称完整。
    std::os::unix::fs::symlink(root.parent().unwrap(), root.join("linked")).unwrap();
    let with_directory_link = Inputs::capture(&root, None).unwrap();
    let reason = with_directory_link
        .incomplete_reason()
        .expect("a directory link cannot be bound");
    assert!(reason.contains("linked"), "{reason}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_file(outside);
}

/// A dependency's dependency is an input too: the closure follows `path` manifests outside the
/// root to a fixed depth, so a change to the transitive crate is not invisible to a reused pass
/// (BR01 external inputs; 2026-10-19 actual review, R2).
/// 依赖的依赖也是输入：闭包沿根外的 `path` 清单跟到固定深度，因此传递依赖被改动不会被复用通过漏掉
/// （BR01 根外输入；2026-10-19 实际复核 R2）。
#[test]
fn a_transitive_path_dependency_outside_the_root_is_inside_the_closure() {
    let base = std::env::temp_dir().join(format!(
        "xirang-dep-chain-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let outer = base.join("outer-dep");
    let inner = base.join("inner-dep");
    let root = base.join("root");
    std::fs::create_dir_all(outer.join("src")).unwrap();
    std::fs::create_dir_all(inner.join("src")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"root\"\nversion = \"0.0.0\"\n\n[dependencies]\nouter-dep = { path = \"../outer-dep\" }\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
    std::fs::write(
        outer.join("Cargo.toml"),
        "[package]\nname = \"outer-dep\"\nversion = \"0.0.0\"\n\n[dependencies]\ninner-dep = { path = \"../inner-dep\" }\n",
    )
    .unwrap();
    std::fs::write(outer.join("src/lib.rs"), "pub fn outer() {}\n").unwrap();
    std::fs::write(
        inner.join("Cargo.toml"),
        "[package]\nname = \"inner-dep\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    std::fs::write(inner.join("src/lib.rs"), "pub fn inner() -> i64 { 1 }\n").unwrap();

    let first = Inputs::capture(&root, None).unwrap();
    std::fs::write(inner.join("src/lib.rs"), "pub fn inner() -> i64 { 2 }\n").unwrap();
    let second = Inputs::capture(&root, None).unwrap();
    assert_ne!(
        first.digest(),
        second.digest(),
        "a change in the dependency's dependency must move the closure's digest"
    );
    let changed = second.changes(&first);
    assert!(
        changed.iter().any(|line| line.contains("inner-dep")),
        "the changed crate must be named, not silently folded: {changed:?}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// The review's R2 bypass had a second shape: TOML literal (single-quoted) paths and commented
/// lines. The closure must read `path = '../dep'` the same as `path = "../dep"`, and it must not
/// bind a `# path = …` comment as if it were a dependency.
/// 复核的 R2 绕行还有第二种形状：TOML 单引号路径与注释行。闭包必须把 `path = '../dep'` 读得与
/// `path = "../dep"` 一样，也不能把 `# path = …` 注释当成依赖绑定。
#[test]
fn a_single_quoted_or_commented_path_dependency_is_still_read() {
    let root = fixture(
        "quotes",
        &[
            (
                "Cargo.toml",
                "[package]\nname=\"quoted\"\n\n# a comment that mentions path = \"../ignored\"\n[dependencies]\nexternal = { path = '../external-dep' }\n",
            ),
            ("src/lib.rs", "pub fn a() {}\n"),
        ],
    );
    let external = root.parent().unwrap().join(format!(
        "xirang-task-external-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(external.join("src")).unwrap();
    std::fs::write(
        external.join("Cargo.toml"),
        "[package]\nname=\"external-dep\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(external.join("src/lib.rs"), "pub fn value() -> i64 { 1 }\n").unwrap();
    // Point the fixture at the directory just created, then capture twice around an edit.
    // 让夹具指向刚创建的目录，然后在一次编辑前后各捕获一次。
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname=\"quoted\"\n\n# a comment that mentions path = \"../ignored\"\n[dependencies]\nexternal = {{ path = '{}' }}\n",
            external.display()
        ),
    )
    .unwrap();
    // The comment names `../ignored`; if comments were bound as dependencies this would be two
    // spellings, and the closure would be keyed to a directory that does not exist.
    // 注释里写着 `../ignored`；若注释被当成依赖绑定，这里会是两条拼写，闭包还会被绑到一个不存在的目录上。
    let parsed = super::path_dependencies(&root.join("Cargo.toml"));
    assert_eq!(parsed.len(), 1, "a comment is not a dependency: {parsed:?}");
    assert_eq!(parsed[0].1, external, "the one dependency is the real path");
    let before = Inputs::capture(&root, None).unwrap();
    assert!(
        before
            .rows
            .iter()
            .any(|(path, _)| path.display().to_string().contains("dependency:")),
        "a bound dependency is a row of the closure: {:?}",
        before.rows
    );
    std::fs::write(external.join("src/lib.rs"), "pub fn value() -> i64 { 2 }\n").unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    assert_ne!(
        before.digest(),
        after.digest(),
        "a dependency's changed source is a changed input"
    );
}

/// A dependency can have dependencies of its own. Binding only the first hop would leave the second
/// hop's bytes outside the closure, so a change there would not invalidate the verdict.
/// 依赖自己也可以有依赖。只绑第一跳会把第二跳的字节留在闭包之外，于是那里的改动不会让判定失效。
#[test]
fn a_path_dependency_of_a_path_dependency_is_bound_too() {
    let stamp = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let base = std::env::temp_dir();
    let near = base.join(format!("xirang-task-near-{stamp}"));
    let far = base.join(format!("xirang-task-far-{stamp}"));
    let root = base.join(format!("xirang-task-root-{stamp}"));
    std::fs::create_dir_all(near.join("src")).unwrap();
    std::fs::create_dir_all(far.join("src")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        near.join("Cargo.toml"),
        format!(
            "[package]\nname=\"near\"\n[workspace]\n[dependencies]\nfar = {{ path = '{}' }}\n",
            far.display()
        ),
    )
    .unwrap();
    std::fs::write(near.join("src/lib.rs"), "pub fn near() -> i64 { 1 }\n").unwrap();
    std::fs::write(
        far.join("Cargo.toml"),
        "[package]\nname=\"far\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(far.join("src/lib.rs"), "pub fn far() -> i64 { 1 }\n").unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname=\"root\"\n[workspace]\n[dependencies]\nnear = {{ path = '{}' }}\n",
            near.display()
        ),
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn root() {}\n").unwrap();
    let before = Inputs::capture(&root, None).unwrap();
    std::fs::write(far.join("src/lib.rs"), "pub fn far() -> i64 { 2 }\n").unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    assert_ne!(
        before.digest(),
        after.digest(),
        "the second hop of a path dependency chain is an input"
    );
}

/// The fact store is shared with the build, so it must not turn a same-size, same-mtime rewrite
/// into a hit: a write updates the change time even when a program restores the modification time.
/// 事实存储与构建共享，因此它不得把“同 size、同 mtime 的改写”当成命中：写文件会更新变更时间，
/// 哪怕之后把修改时间设回原值。
#[test]
fn a_same_size_same_mtime_rewrite_changes_the_input_closure() {
    let root = fixture(
        "facts",
        &[("src/lib.rs", "pub fn a() { let value = 1; }\n")],
    );
    std::fs::create_dir_all(root.join("target/xirang")).unwrap();
    let before = Inputs::capture(&root, None).unwrap();
    assert!(
        root.join("target/xirang/facts.json").is_file(),
        "the capture writes the shared fact store"
    );
    let path = root.join("src/lib.rs");
    let metadata = std::fs::metadata(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let rewritten = text.replace("value = 1", "value = 2");
    assert_eq!(rewritten.len(), text.len());
    std::fs::write(&path, &rewritten).unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(metadata.modified().unwrap())
        .unwrap();
    let after = Inputs::capture(&root, None).unwrap();
    assert_ne!(
        before.digest(),
        after.digest(),
        "content, not the timestamps, decides the closure"
    );
    assert!(
        before
            .changes(&after)
            .iter()
            .any(|line| line == "modified src/lib.rs"),
        "and the diff names it"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A target's `path` is not a dependency's `path`.**
/// **target 的 `path` 不是依赖的 `path`。**
///
/// `[lib] path` and `[[bin]] path` name targets, not crates. Reading them as dependencies bound every
/// target of every dependency as "a path dependency that is not a directory", so a host depending on
/// a crate that ships binaries got an `incomplete` closure it could never repair — measured by P5 on
/// a generated host depending on this checkout (sixty-odd bogus entries, and the maintenance task's
/// `verified` stayed `unknown`). The closure must be complete when the only `path` keys outside a
/// dependency table are target paths.
/// `[lib] path` 与 `[[bin]] path` 点名的是 target 而不是 crate。把它们当成依赖，会把每个依赖的每个
/// target 绑成"不是目录的 path 依赖"，于是依赖一个带二进制的 crate 的宿主得到一份永远修不好的
/// `incomplete` 闭包——P5 在一个依赖本检出的生成宿主上量到（六十余条假条目，维护任务的 `verified` 一直
/// 是 `unknown`）。当依赖表之外的 `path` 键只是 target 路径时，闭包必须是完整的。
#[test]
fn a_target_path_is_not_a_path_dependency() {
    let root = fixture(
        "target-paths",
        &[
            (
                "Cargo.toml",
                "[package]\nname=\"target-paths\"\nbuild=\"build.rs\"\n\n[lib]\npath=\"lib/entry.rs\"\n\n\
                 [[bin]]\nname=\"tool\"\npath=\"bins/tool.rs\"\n",
            ),
            ("lib/entry.rs", "pub fn entry() {}\n"),
            ("bins/tool.rs", "fn main() {}\n"),
            ("build.rs", "fn main() {}\n"),
        ],
    );
    let external = root.parent().unwrap().join(format!(
        "xirang-task-target-dep-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(external.join("src")).unwrap();
    std::fs::write(
        external.join("Cargo.toml"),
        "[package]\nname=\"target-dep\"\n[workspace]\n\n[lib]\npath=\"src/lib.rs\"\n\n\
         [[bin]]\nname=\"dep-tool\"\npath=\"src/tool.rs\"\n",
    )
    .unwrap();
    std::fs::write(external.join("src/lib.rs"), "pub fn value() -> i64 { 1 }\n").unwrap();
    std::fs::write(external.join("src/tool.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname=\"target-paths\"\nbuild=\"build.rs\"\n\n[lib]\npath=\"lib/entry.rs\"\n\n\
             [[bin]]\nname=\"tool\"\npath=\"bins/tool.rs\"\n\n[dependencies]\nexternal = {{ path = '{}' }}\n",
            external.display()
        ),
    )
    .unwrap();
    let parsed = super::path_dependencies(&root.join("Cargo.toml"));
    assert_eq!(
        parsed.len(),
        1,
        "one dependency, not the four target paths: {parsed:?}"
    );
    assert_eq!(parsed[0].1, external, "the dependency is the real path");
    let inputs = Inputs::capture(&root, None).unwrap();
    assert!(
        inputs.incomplete().is_none(),
        "a target path is not an unbound dependency: {:?}",
        inputs.incomplete()
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(external);
}
