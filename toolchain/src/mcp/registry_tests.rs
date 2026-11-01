//! Tests for the registry tool: the derivation it reports, and the namespace it
//! derives it under.
//! 注册树工具的测试：它报告的那份推导，以及推导所用的命名空间。

use std::path::PathBuf;

use xirang_kernel::NodeId;

use super::{namespace_from, registry};

/// The shape `face_views` recognises as a root face; the parent is omitted
/// deliberately, which means "the package root".
/// `face_views` 认作根面的形状；此处有意省略父级，含义就是"包根"。
const FACE: &str = "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n";

/// A throwaway package with exactly one registration face, laid out the way the
/// discovery walk reads: a face lives in `<name>/<name>.rs` under `src/`, and a
/// loose `.rs` file at the top of `src/` would be an unplaced face instead.
/// 只含一个注册面的一次性包，布局按发现遍历的读法：面住在 `src/` 下的 `<name>/<name>.rs`，
/// 而 `src/` 顶层的散装 `.rs` 文件会成为无法安放的面。
fn fixture(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-toolchain-registry-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let face = root.join("src/control/control.rs");
    std::fs::create_dir_all(face.parent().expect("face directory")).expect("fixture directories");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("entry");
    std::fs::write(&face, FACE).expect("face");
    root
}

/// The rows carry the identity the host compiled, so the namespace has to be
/// the package's own name — Cargo's answer, not the documented default. The id
/// assertion is the load-bearing half: it is a hash over
/// `(namespace, "control/control.rs", "Control")`, so a tool that answered under
/// `xirang.default` would report an id nothing in the package holds.
/// 行携带的是宿主编译出的身份，因此命名空间必须是这个包自己的名字——Cargo 的答案，而不是
/// 文档化的默认值。id 断言是承重的那一半：它是对
/// `(namespace, "control/control.rs", "Control")` 的散列，因此若工具在 `xirang.default`
/// 之下作答，报告的 id 就是包里没有任何东西持有的。
#[test]
fn a_face_is_reported_under_the_namespace_the_package_compiled_with() {
    let root = fixture("fixture-host");
    let report = registry(&root).expect("the tree is derived");
    assert!(report.starts_with("namespace fixture-host\n"), "{report}");
    assert!(report.contains("faces 1\n"), "{report}");
    assert!(report.contains("root/control"), "{report}");
    assert!(report.contains("control/control.rs"), "{report}");
    let expected = NodeId::from_namespaced_path("fixture-host", "control/control.rs", "Control");
    assert!(
        report.contains(&expected.to_string()),
        "the row must carry the compiled identity {expected}: {report}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The configured override wins verbatim, and it does so *before* Cargo is
/// asked: a directory with no manifest still answers when the namespace is
/// given, which is what makes the override usable for a package Cargo cannot
/// name.
/// 配置的覆盖原样胜出，而且它**先于**询问 Cargo：只要给出了命名空间，连没有清单的目录也能
/// 作答——正是这一点让覆盖对 Cargo 说不出的包也可用。
#[test]
fn a_configured_namespace_wins_verbatim_before_cargo_is_asked() {
    let bare = std::env::temp_dir().join(format!(
        "xirang-toolchain-registry-bare-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    assert_eq!(
        namespace_from(Some("given-by-the-user"), &bare).expect("the override answers"),
        "given-by-the-user"
    );
    let _ = std::fs::remove_dir_all(&bare);
}

/// A package Cargo cannot name is refused, and the refusal names the way out.
/// This is the asymmetry with authoring: a query about an existing tree must not
/// invent the identity domain, because every id below it would be wrong.
/// Cargo 说不出的包会被拒绝，而且拒绝点名了出路。这正是与创作侧的不对称：针对已存在树的
/// 查询不能凭空造出身份域，因为其下每个 id 都会是错的。
#[test]
fn a_directory_without_a_package_is_refused_with_the_way_out() {
    let bare = std::env::temp_dir().join(format!(
        "xirang-toolchain-registry-nameless-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    let error = namespace_from(None, &bare).expect_err("no manifest means no namespace");
    assert!(error.contains("identity namespace"), "{error}");
    assert!(error.contains("XIRANG_NAMESPACE"), "{error}");
    let _ = std::fs::remove_dir_all(&bare);
}

/// A registration file the derivation cannot parse is counted in the reply instead of vanishing:
/// a read-only tree query used to report a smaller tree as if it were the whole one, which is what
/// `LGC-LG-11` recorded on the producer side and what the consumer half now says out loud.
/// 推导解析不了的注册面文件被计入回复而不是消失：只读的树查询过去把一棵更小的树当成完整的树报出去
/// ——这正是 `LGC-LG-11` 在生产端记录的事，而消费端现在把它说出来。
fn broken_face(root: &std::path::Path, label: &str) {
    let directory = root.join("src").join(label);
    std::fs::create_dir_all(&directory).expect("module directory");
    std::fs::write(
        directory.join(format!("{label}.rs")),
        "crate::root_object! {\n    kind: Broken,\n",
    )
    .expect("truncated face");
}

#[test]
fn an_unparsable_registration_file_is_counted_in_the_reply() {
    let root = fixture("unparsable");
    broken_face(&root, "broken");
    let reply = registry(&root).expect("the tree is derived");
    assert!(reply.contains("unparsable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The writer and the readers resolve identities through **one** entry, which is what keeps
/// `verify` (which stamps records) and `diff`/`search` (which read them) from reporting a fresh
/// tree as one whose every identity moved (audit `LGC-LG-13`). Spelling `package_name` in any of
/// them is how that drift starts, so it is refused here rather than reviewed for.
/// 写入方与读取方经**同一个**入口解析身份——正是它让 `verify`（盖记录）与 `diff`/`search`（读记录）
/// 不会把一棵刚校验过的树报成每个身份都动过（审计 `LGC-LG-13`）。在其中任何一个里另写 `package_name`
/// 正是漂移的起点，因此这里直接拒绝，而不是留给评审去看。
#[test]
fn the_writer_and_the_readers_share_one_namespace_entry() {
    for (name, source) in [
        ("verify.rs", include_str!("verify.rs")),
        ("diff.rs", include_str!("diff.rs")),
        ("search.rs", include_str!("search.rs")),
    ] {
        assert!(
            !source.contains("package_name("),
            "{name} must resolve identities through `registry::namespace`, not Cargo directly"
        );
    }
    assert!(
        include_str!("verify.rs").contains("registry::namespace"),
        "verify is the writer; it has to use the shared entry"
    );
}

/// A throwaway **virtual** workspace: two member packages, each with a `src/` and no published
/// records, which is the shape the workspace default has to summarize.
/// 一个一次性**虚拟**工作区：两个成员包，各自有 `src/`、什么都没发布——正是工作区默认档要概括的形状。
fn workspace(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-toolchain-registry-ws-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\nresolver = \"2\"\n",
    );
    for member in ["a", "b"] {
        write_fixture(
            &root.join(format!("crates/{member}/Cargo.toml")),
            &format!(
                "[package]\nname = \"mcp-{label}-{member}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        );
        write_fixture(
            &root.join(format!("crates/{member}/src/lib.rs")),
            "// member entry\n",
        );
    }
    root
}

/// Write one fixture file, creating its directory.
/// 写一个夹具文件并建好它的目录。
fn write_fixture(path: &std::path::Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    std::fs::write(path, text).expect("fixture file");
}

/// The default at a virtual root is the **member→faces** shape, and `full` is the one word that
/// buys the per-member trees back.
/// 虚拟根上的默认是**成员→面数**的形状，而 `full` 就是买回逐成员树的那一个词。
///
/// The measured reason: a workspace-root `registry` cost 1,558 characters, most of them the trees of
/// members the caller had not asked about yet. What must not change with the shape is *what the
/// answer says about the tree* — every member, its status and its face count — so those are asserted
/// on the short form too, and only the per-member sections are asserted **absent**.
/// 量到的理由：工作区根上一次 `registry` 花 1,558 个字符，其中大多是调用方还没问到的成员的树。不随形状
/// 改变的是**答案对被读到的那棵树说了什么**——每个成员、它的状态与它的面数——因此这些在短档上也要断言，
/// 只断言逐成员小节**不在**。
#[test]
fn a_virtual_root_answers_members_and_faces_until_full_is_asked_for() {
    let root = workspace("brief");
    let brief = super::registry_brief(&root).expect("the short default answers");
    assert!(brief.contains("members 2"), "{brief}");
    for member in ["mcp-brief-a", "mcp-brief-b"] {
        assert!(brief.contains(member), "every member is named: {brief}");
    }
    assert!(
        brief.contains("0 faces"),
        "each member's face count is the payload of the short form: {brief}"
    );
    assert!(
        brief.contains("full: true") && brief.contains("(no faces)"),
        "the way to the trees it leaves out, and each member's status, are in the answer: {brief}"
    );
    assert!(
        !brief.contains("== ") && !brief.contains("source_scope.tsv"),
        "the per-member sections and reasons are what the default leaves out: {brief}"
    );

    let full = registry(&root).expect("the expanded answer");
    assert!(full.contains("== mcp-brief-a ("), "{full}");
    assert!(
        full.contains("source_scope.tsv"),
        "the expanded answer carries the reasons the default folded: {full}"
    );
    assert!(
        full.len() > brief.len(),
        "`full` is the bigger answer by the sections it restores ({} vs {}): {full}",
        brief.len(),
        full.len()
    );
    // The member list is this answer's payload, so it must not collapse the way a preamble may:
    // the *second* call in a session says the same thing as the first.
    let again = super::registry_brief(&root).expect("the short default answers again");
    assert_eq!(
        again, brief,
        "the short form is a stable answer, not a preamble"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The switches this batch added are advertised, and the registry one is **accepted** rather than
/// silently ignored; `read`'s window is advertised as the 8 it now is.
/// 这一批新加的开关被声明，而注册树的那一个是被**接受**的、不是被静默忽略；`read` 的窗口按它现在的
/// 8 行声明。
///
/// The check half is pinned by advertisement here, because this batch's in-scope test file is this
/// one: `check_tests.rs` is out of it, and calling `check` runs `cargo test`. `--census` end to end
/// is in the measured evidence (`target/xirang-t1/granularity-compare.txt`).
/// `census` 那一半在这里按"被声明"钉住，因为本任务 in-scope 的测试文件就是这一个：`check_tests.rs`
/// 不在其中，而调用 `check` 会跑 `cargo test`。`--census` 的端到端证据在量到的记录里
/// （`target/xirang-t1/granularity-compare.txt`）。
#[test]
fn the_new_granularity_switches_are_advertised_and_the_registry_one_is_accepted() {
    let listed = crate::mcp::tools::tools();
    let schema = |name: &str| {
        listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is advertised"))["inputSchema"]["properties"]
            .clone()
    };
    assert!(
        schema("xirang.registry").get("full").is_some(),
        "xirang.registry advertises `full`"
    );
    assert!(
        schema("xirang.check").get("census").is_some(),
        "xirang.check advertises `census`"
    );
    let context = schema("xirang.read")["context"]["description"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(
        context.contains("default 8") && !context.contains("default 40"),
        "the read window's description must be the 8 it is: {context}"
    );

    let root = workspace("switches");
    let reply = crate::mcp::tools::tool_call(
        &root,
        serde_json::json!(1),
        &serde_json::json!({"name": "xirang.registry", "arguments": {"full": true}}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert!(
        text.contains("== mcp-switches-a ("),
        "`full: true` is accepted and expands the answer: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A wide tree's default answer is the **census**, and its size follows the levels, not the faces.
/// 一棵很宽的树默认回的是**普查**，而它的大小跟层级走，不跟面数走。
///
/// Audit `W2-1`: the default printed one row per face, so a 50,000-face package answered with a
/// multi-megabyte list whose first screen already decided the next call. The census is that first
/// screen — how many faces, and how many sit at each level — and it is bought back with
/// `full: true`. The load-bearing assertion is the **flatness**: doubling the faces must not grow
/// the default answer, because that is what makes it hold at 50,000.
/// 审计 `W2-1`：默认每个面印一行，因此五万面的包回的是几 MB 的清单，而它第一屏就已经决定了下一个调用。
/// 普查就是那一屏——多少个面、每一层各有多少——而它由 `full: true` 买回。承重的断言是**拍平**：面数翻倍
/// 不能让默认答案变大，正是这一条让它在五万面上成立。
#[test]
fn the_default_is_a_census_whose_size_follows_the_levels() {
    fn wide(label: &str, per_directory: usize) -> PathBuf {
        let root = fixture(label);
        for index in 0..per_directory {
            let directory = root.join(format!("src/control/object/child{index}"));
            std::fs::create_dir_all(&directory).expect("child directory");
            std::fs::write(
                directory.join(format!("child{index}.rs")),
                format!(
                    "crate::control_object! {{\n    kind: Child{index},\n    parent: \
                     crate::control::NODE_ID,\n}}\n"
                ),
            )
            .expect("child face");
        }
        root
    }
    let small = wide("census-small", 4);
    let large = wide("census-wide", 120);
    let (small, large) = (
        super::registry_brief(&small).expect("a census"),
        super::registry_brief(&large).expect("a census"),
    );
    for (label, answer) in [("small", &small), ("wide", &large)] {
        assert!(
            answer.contains("level (path prefix)") && answer.contains("face(s)"),
            "the {label} default is the census: {answer}"
        );
        assert!(
            answer.contains("`full: true` prints the rows"),
            "and it names the way to the rows: {answer}"
        );
        assert!(
            answer.len() <= 2048,
            "the {label} census is ≤2 KB: {} bytes",
            answer.len()
        );
        assert!(
            !answer.contains("control/object/child0/child0.rs"),
            "the {label} default prints no row: {answer}"
        );
    }
    // The level count is the same in both fixtures, so the answers are within a byte or two: the
    // size follows the shape of the tree, not how many faces hang off it.
    // 两个夹具的层级数相同，因此答案只差一两个字节：大小跟着树的形状走，而不是挂着多少个面。
    let growth = large.len() as i64 - small.len() as i64;
    assert!(
        growth.abs() < 40,
        "•120 faces must not grow the census (small {} vs wide {})",
        small.len(),
        large.len()
    );

    // And `full: true` is a page: at most 200 rows, with the cursor naming the next one.
    // 而 `full: true` 是一页：最多 200 行，并带点名下一页的游标。
    let page = super::registry_page(&wide("census-page", 260), 0, 200).expect("a page");
    assert!(
        page.contains("rows 1-200 of 261") && page.contains("withheld at the limit of 200"),
        "the page is bounded and says which slice it is: {page}"
    );
    assert!(
        page.contains("offset: 200"),
        "and the cursor names the next page: {page}"
    );
    let second = super::registry_page(&wide("census-page-2", 260), 200, 200).expect("a page");
    assert!(
        second.contains("rows 201-261 of 261") && !second.contains("withheld"),
        "the last page is short and withholds nothing: {second}"
    );
}

/// The 2026-10-10 defect queue, item ⑤: a caller who built with a `target_dir` can point the readers
/// at the directory that build published into.
/// 2026-10-10 缺陷队列第 ⑤ 条：用 `target_dir` 构建过的调用方可以把读取方指到那次构建发布的目录。
///
/// Measured before the fix through the real bridge, on the shipped example: `verify
/// {target_dir: "../out-evidence-probe"}` published the records under
/// `<package>/../out-evidence-probe/xirang/out`, and `registry` — told that same directory — still read
/// `<package>/target/xirang/out`, answered `no published records … cannot read …/source_scope.tsv` and
/// fell back to deriving (report §7h). After the fix the same two calls answer
/// `tree published from …/out-evidence-probe/xirang/out` ✓ — that end-to-end pair is the bridge
/// evidence in §7h, because publishing records needs a package the pipeline recognises as a host.
/// What a unit test can pin is the half that *was* broken and needs no host: **which directory the
/// reader looks in**, and that the request-level value is what decides it.
/// 修之前用真桥在出厂示例上实测：`verify {target_dir: "../out-evidence-probe"}` 把记录发布到
/// `<包>/../out-evidence-probe/xirang/out`，而被明确告知同一目录的 `registry` 仍然去读
/// `<包>/target/xirang/out`，答 `no published records … cannot read …/source_scope.tsv` 并回退到推导
/// （报告 §7h）。修后同两次调用答 `tree published from …/out-evidence-probe/xirang/out` ✓——那一对端到端
/// 证据在 §7h，因为发布记录需要一个被管线认作宿主的包。单元测试能钉的是**坏掉的那一半、且不需要宿主**：
/// **读取方到底去哪个目录找**，以及决定它的是不是那个请求级的值。
#[test]
fn a_reader_looks_in_the_directory_the_request_named() {
    let root = fixture("evidence-target");
    let target = "build-output";
    let named = root.join(target).join("xirang/out");
    let default = root.join("target/xirang/out");

    crate::mcp::build_evidence::set_target_dir(Some(target));
    assert_eq!(
        crate::mcp::build_evidence::out_dir(&root),
        named,
        "the evidence directory follows a relative target dir, resolved per package"
    );
    let read = registry(&root).expect("the tree is read");
    assert!(
        read.contains(&named.display().to_string()),
        "the reader looks where the request pointed it: {read}"
    );
    assert!(
        !read.contains(&default.display().to_string()),
        "and not in the default directory: {read}"
    );

    // An absolute target dir works the same way, and clearing the value restores the default.
    // 绝对 target 目录同理，而清除该值即恢复默认。
    crate::mcp::build_evidence::set_target_dir(Some("/tmp/xirang-evidence-absolute"));
    assert_eq!(
        crate::mcp::build_evidence::out_dir(&root),
        std::path::PathBuf::from("/tmp/xirang-evidence-absolute/xirang/out")
    );
    crate::mcp::build_evidence::set_target_dir(None);
    assert_eq!(
        crate::mcp::build_evidence::out_dir(&root),
        default,
        "no target dir named ⇒ the default is unchanged"
    );
    let read = registry(&root).expect("the tree is read");
    assert!(
        read.contains(&default.display().to_string()),
        "and the reader is back on the default: {read}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
