//! Tests for the plugin-lock write: what a preview promises, and what an apply stores.
//! 插件锁写入的测试：预览承诺了什么，落盘存下了什么。
//!
//! The pins are the same three the write path keeps: a request without `apply` writes
//! nothing (asserted before the reply is read, so the mutation "preview means write" fails
//! naming the file that appeared), a write without `confirm` is refused, and the bytes an
//! apply stores are the bytes the preview showed — which is why the last assertion of the
//! happy path re-parses the lock with the kernel's own reader.
//! 钉子与写入路径守的是同样三条：不带 `apply` 的请求什么都不写（在读取回复之前断言，因此"预览即写入"的
//! 变异会失败并点名那个出现的文件）；不带 `confirm` 的写入被拒绝；落盘存下的字节就是预览显示的字节
//! ——这也是顺利路径的最后一条断言会用核内自己的读取器重新解析那份锁的原因。

use std::path::PathBuf;

use serde_json::json;
use xirang_kernel::plugin::catalog::PluginCatalog;

use super::plugin;

/// A throwaway package: `cargo metadata` can name it, which is what the write path asks
/// before it resolves an owner.
/// 一个一次性包：`cargo metadata` 能给它命名，而写入路径在解析拥有者之前问的正是这个。
struct Package {
    root: PathBuf,
}

impl Drop for Package {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Package {
    /// `.xirang/plugins`, the directory this write creates.
    /// `.xirang/plugins`，本次写入会创建的目录。
    fn plugins(&self) -> PathBuf {
        self.root.join(".xirang/plugins")
    }

    /// The lock file a source writes.
    /// 某个来源写入的那份锁文件。
    fn lock(&self, source: &str) -> PathBuf {
        self.plugins().join(if source == "official" {
            "official.lock"
        } else {
            "user.lock"
        })
    }

    /// The entry file a source writes.
    /// 某个来源写入的那个入口文件。
    fn entry(&self, source: &str) -> PathBuf {
        self.plugins().join(if source == "official" {
            "official.rs"
        } else {
            "user.rs"
        })
    }
}

/// A fresh package under a label that keeps concurrent tests apart.
/// 一个把并发测试彼此分开的新包，带标签。
fn package(label: &str) -> Package {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-plugin-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    Package { root }
}

/// One user-lane request, with the two flags a caller adds.
/// 一次用户通道请求，带调用方会加上的那两个标志。
fn user_request(checksum: &str, apply: bool, confirm: bool) -> serde_json::Value {
    json!({
        "source": "user",
        "framework": "xirang.test",
        "package": "demo-plugin",
        "version": "0.1.0",
        "crate": "demo_plugin",
        "checksum": checksum,
        "mode": "extension",
        "apply": apply,
        "confirm": confirm,
    })
}

/// A preview writes nothing — not the lock, not the entry, not even the directory — and
/// shows the bytes and the paths.
/// 预览什么都不写——锁、入口、乃至那个目录都不写——并显示字节与路径。
#[test]
fn a_preview_writes_nothing_and_shows_the_exact_bytes() {
    let fixture = package("preview");
    let plugins = fixture.plugins();
    // The request carries `apply` and `confirm` **nowhere**: this is the default path, so
    // the mutation that flips it to a write is what this test is aimed at. A request that
    // spells `apply: false` would pass a flipped default right by.
    // 请求里**根本没有** `apply` 与 `confirm`：这是默认路径，因此"把默认翻成写入"正是本测试瞄准的
    // 变异。若请求写明 `apply: false`，被翻掉的默认就能从旁边溜过去。
    let request = json!({
        "source": "user",
        "framework": "xirang.test",
        "package": "demo-plugin",
        "version": "0.1.0",
        "crate": "demo_plugin",
        "checksum": "sha256:00",
        "mode": "extension",
    });
    let reply = plugin(&fixture.root, &request).expect("the preview answers");

    // Asserted first, so the mutation that flips the default fails naming the directory a
    // preview must not create.
    // 先断言这一条，因此把默认翻过去的变异会失败并点名"预览不得创建"的那个目录。
    assert!(
        !plugins.exists(),
        "a request without `apply` must not create {} — the default is a preview, not a write. \
         reply was:\n{reply}",
        plugins.display()
    );
    assert!(
        !fixture.lock("user").exists() && !fixture.entry("user").exists(),
        "the preview wrote a file:\n{reply}"
    );
    let record = "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension";
    for expected in [
        "preview: xirang.plugin would write one user record",
        "nothing was written",
        &format!("+ {}", plugins.display()),
        &format!("~ {}", fixture.lock("user").display()),
        &format!("+{record}"),
        &format!("~ {}", fixture.entry("user").display()),
        "+#[allow(unused_imports)]",
        "+use demo_plugin as _;",
        &format!("record: {record}"),
    ] {
        assert!(
            reply.contains(expected),
            "`{expected}` is part of the preview's contract:\n{reply}"
        );
    }
    assert!(
        reply.contains(&format!("root {}", fixture.root.display())),
        "{reply}"
    );
}

/// A write needs the request to say `confirm`, and the refusal comes before the directory
/// exists.
/// 写入需要请求自己说出 `confirm`，而拒绝发生在目录出现之前。
#[test]
fn a_plugin_write_without_confirm_is_refused() {
    let fixture = package("confirm");
    let error = plugin(&fixture.root, &user_request("sha256:00", true, false))
        .expect_err("a write without `confirm` is refused");

    assert!(error.contains("confirm: true"), "{error}");
    assert!(error.contains("user.lock"), "{error}");
    assert!(
        !fixture.plugins().exists(),
        "the refusal created {}",
        fixture.plugins().display()
    );
}

/// An apply stores the lock and the entry, and the lock is what the kernel's own reader
/// parses back.
/// 落盘会存下锁与入口，而那把锁正是核内自己的读取器能解析回来的。
#[test]
fn an_apply_stores_a_lock_the_kernel_reads_back() {
    let fixture = package("apply");
    let record = "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension";
    let reply =
        plugin(&fixture.root, &user_request("sha256:00", true, true)).expect("the write answers");

    assert!(reply.contains("applied: wrote one user record"), "{reply}");
    for expected in [
        &format!("created {}", fixture.plugins().display()),
        &format!("wrote {}", fixture.lock("user").display()),
        &format!("wrote {}", fixture.entry("user").display()),
        &format!("record: {record}"),
    ] {
        assert!(reply.contains(expected), "{reply}");
    }
    let lock = std::fs::read_to_string(fixture.lock("user")).expect("the lock reads");
    assert_eq!(lock, format!("{record}\n"));
    let catalog = PluginCatalog::parse_plugin_catalog(&lock)
        .expect("the host's own reader accepts what this write stored");
    assert_eq!(catalog.records().len(), 1, "{lock:?}");
    let entry = std::fs::read_to_string(fixture.entry("user")).expect("the entry reads");
    assert!(
        entry.ends_with("\n#[allow(unused_imports)]\nuse demo_plugin as _;\n"),
        "{entry:?}"
    );
}

/// An official record the lock does not account for is refused by the trust rule, before
/// anything is written.
/// 锁没有覆盖的官方记录被信任规则拒绝，且在写下任何东西之前。
#[test]
fn an_official_record_the_lock_does_not_account_for_is_refused() {
    let fixture = package("official-gate");
    let error = plugin(
        &fixture.root,
        &json!({
            "source": "official",
            "framework": "xirang.test",
            "package": "official-plugin",
            "version": "1.0.0",
            "crate": "official_plugin",
            "checksum": "sha256:00",
            "mode": "extension",
            "apply": true,
            "confirm": true,
        }),
    )
    .expect_err("an unaccounted official record is refused");
    assert!(error.contains("not present in the trusted lock"), "{error}");
    assert!(
        error.contains("contains_record"),
        "the refusal names the rule it applied:\n{error}"
    );
    assert!(!fixture.plugins().exists());
}

/// The promotion an official write exists for (PH-7): the trusted lock carries the bare
/// seven-field record, the request names a signature, and the kernel *replaces* that record
/// instead of refusing a duplicate — one identity, one record, now carrying provenance.
/// 官方写入存在的理由（PH-7）：受信锁里是裸的七字段记录，请求点名了签名，于是核内**取代**那条记录，
/// 而不是以重复为由拒绝——一个身份一条记录，现在携带来源。
#[test]
fn an_official_write_promotes_the_trusted_record_with_its_provenance() {
    let fixture = package("official-promotion");
    let seeded = "official|xirang.test|official-plugin|1.0.0|official_plugin|sha256:00|extension\n";
    std::fs::create_dir_all(fixture.plugins()).expect("plugin directory");
    std::fs::write(fixture.lock("official"), seeded).expect("seed lock");

    let text = plugin(
        &fixture.root,
        &json!({
            "source": "official",
            "framework": "xirang.test",
            "package": "official-plugin",
            "version": "1.0.0",
            "crate": "official_plugin",
            "checksum": "sha256:00",
            "mode": "extension",
            "signature": "sig-v1",
            "fingerprint": "key-v1",
            "apply": true,
            "confirm": true,
        }),
    )
    .expect("the trusted record is promoted");

    let written = std::fs::read_to_string(fixture.lock("official")).expect("lock text");
    assert!(written.contains("|sig-v1|key-v1|"), "{written}");
    // The promotion replaces the trusted line rather than sitting beside it.
    // 这次升级**替换**掉那条受信记录，而不是与它并排。
    assert_eq!(
        written
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .count(),
        1,
        "{written}"
    );
    let catalog = PluginCatalog::parse_plugin_catalog(&written).expect("the lock parses");
    assert_eq!(catalog.records().len(), 1, "one identity keeps one record");
    assert_eq!(catalog.records()[0].signature.as_deref(), Some("sig-v1"));
    assert!(text.contains("sig-v1"), "{text}");
}

/// An official record the lock *does* account for still has to pass the lock's parser, and
/// a second record with the same identity is refused there — with the existing bytes left
/// exactly as they were.
/// 锁**确实**覆盖的官方记录仍要过锁自己的解析器，而第二条同身份记录会在那里被拒绝——既有字节原样留下。
#[test]
fn an_official_record_the_lock_accounts_for_is_still_judged_by_the_parser() {
    let fixture = package("official-parser");
    let seeded =
        "official|xirang.test|official-plugin|1.0.0|official_plugin|sha256:00|extension|||\n";
    std::fs::create_dir_all(fixture.plugins()).expect("plugin directory");
    std::fs::write(fixture.lock("official"), seeded).expect("seed lock");

    let error = plugin(
        &fixture.root,
        &json!({
            "source": "official",
            "framework": "xirang.test",
            "package": "official-plugin",
            "version": "1.0.0",
            "crate": "official_plugin",
            "checksum": "sha256:00",
            "mode": "extension",
            "apply": true,
            "confirm": true,
        }),
    )
    .expect_err("the parser refuses the duplicate identity");

    // The parser's refusal *is* the proof the gate let it through: the trust rule's own
    // message would have been "is not present in the trusted lock".
    // 解析器的拒绝**正是**闸门放行过的证明：信任规则自己的消息会是 "is not present in the trusted
    // lock"。
    assert!(
        !error.contains("not present in the trusted lock"),
        "{error}"
    );
    assert!(
        error.contains("would not parse with this record"),
        "{error}"
    );
    assert!(error.contains("duplicates package identity"), "{error}");
    assert_eq!(
        std::fs::read_to_string(fixture.lock("official")).expect("the seed survives"),
        seeded
    );
}

/// One package version can carry two digests, and `accounts_for` reads that pair as two
/// identities — so the append lands instead of being refused as a duplicate. The pair the
/// parser refuses is the one whose seven fields are equal, and the downgrade below is the
/// reachable refusal for this lane.
/// 同一个包版本可以承载两个摘要，而 `accounts_for` 把这一对读成两个身份——因此这次追加会落盘，而不是以
/// 重复为由被拒绝。解析器拒绝的是七个字段相等的那一对，而下面那条降级是这个通道上可达的拒绝。
#[test]
fn a_second_digest_for_one_package_version_is_a_second_identity() {
    let fixture = package("user-two-digests");
    let seeded = "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension\n";
    std::fs::create_dir_all(fixture.plugins()).expect("plugin directory");
    std::fs::write(fixture.lock("user"), seeded).expect("seed lock");

    plugin(&fixture.root, &user_request("sha256:01", true, true))
        .expect("a second digest is a second identity");
    let written = std::fs::read_to_string(fixture.lock("user")).expect("lock text");
    assert_eq!(
        PluginCatalog::parse_plugin_catalog(&written)
            .expect("the lock parses")
            .records()
            .len(),
        2,
        "{written}"
    );
}

/// A record that would drop a provenance column is a downgrade, and the parser refuses it:
/// the lock keeps its bytes and the reply carries the parser's own reason.
/// 会让某一来源列消失的记录是降级，解析器拒绝它：锁保留自己的字节，而回复携带解析器自己的理由。
#[test]
fn a_user_record_that_would_drop_provenance_is_not_written() {
    let fixture = package("user-downgrade");
    let seeded = "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension|sig-v1||\n";
    std::fs::create_dir_all(fixture.plugins()).expect("plugin directory");
    std::fs::write(fixture.lock("user"), seeded).expect("seed lock");

    let error = plugin(&fixture.root, &user_request("sha256:00", true, true))
        .expect_err("the parser refuses the downgrade");
    assert!(error.contains("duplicates package identity"), "{error}");
    assert_eq!(
        std::fs::read_to_string(fixture.lock("user")).expect("the seed survives"),
        seeded
    );
}

/// A record the lock already carries is reported as already selected, and no file changes
/// — with or without `apply`.
/// 锁已经载有的记录会以 "already selected" 报告，且没有任何文件改变——带不带 `apply` 都一样。
#[test]
fn a_record_the_lock_already_carries_is_not_rewritten() {
    let fixture = package("already");
    let seeded = "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension\n";
    std::fs::create_dir_all(fixture.plugins()).expect("plugin directory");
    std::fs::write(fixture.lock("user"), seeded).expect("seed lock");

    let preview = plugin(&fixture.root, &user_request("sha256:00", false, false))
        .expect("the preview answers");
    assert!(preview.starts_with("already selected:"), "{preview}");

    let applied =
        plugin(&fixture.root, &user_request("sha256:00", true, true)).expect("the apply answers");
    assert!(applied.starts_with("already selected:"), "{applied}");
    assert_eq!(
        std::fs::read_to_string(fixture.lock("user")).expect("the seed survives"),
        seeded
    );
    assert!(
        !fixture.entry("user").exists(),
        "nothing needed the entry file, and nothing wrote it"
    );
}

/// A lock whose last line has no terminator gains one, and the preview says so rather than
/// printing an empty added line.
/// 末行没有终止符的锁会补一个，而预览会说出来，而不是打印一个空的新增行。
#[test]
fn a_lock_without_a_trailing_newline_gains_one() {
    let fixture = package("terminator");
    std::fs::create_dir_all(fixture.plugins()).expect("plugin directory");
    std::fs::write(
        fixture.lock("user"),
        "user|xirang.test|first-plugin|0.1.0|first_plugin|sha256:00|extension",
    )
    .expect("seed lock");

    let request = json!({
        "source": "user",
        "framework": "xirang.test",
        "package": "second-plugin",
        "version": "0.2.0",
        "crate": "second_plugin",
        "checksum": "sha256:01",
        "mode": "extension",
        "apply": true,
        "confirm": true,
    });
    let preview = plugin(
        &fixture.root,
        &json!({ "source": "user", "framework": "xirang.test", "package": "second-plugin",
                 "version": "0.2.0", "crate": "second_plugin", "checksum": "sha256:01",
                 "mode": "extension" }),
    )
    .expect("the preview answers");
    assert!(
        preview.contains("+ (adds the missing final newline)"),
        "{preview}"
    );

    let applied = plugin(&fixture.root, &request).expect("the write answers");
    assert!(applied.contains("applied:"), "{applied}");
    let lock = std::fs::read_to_string(fixture.lock("user")).expect("the lock reads");
    let catalog =
        PluginCatalog::parse_plugin_catalog(&lock).expect("the pair of records parses as one lock");
    assert_eq!(catalog.records().len(), 2, "{lock:?}");
    assert!(
        lock.contains("extension\nuser|xirang.test|second-plugin"),
        "the completion separates the two records:\n{lock:?}"
    );
}

/// A plugin write on a virtual workspace root is refused with the candidate members, and
/// nothing is created in any of them.
/// 虚拟工作区根上的插件写入会带着候选成员被拒绝，且不会在其中任何一个里创建东西。
#[test]
fn a_virtual_workspace_root_refuses_a_plugin_write_with_candidates() {
    let fixture = package("virtual");
    std::fs::create_dir_all(fixture.root.join("host/src")).expect("member directory");
    std::fs::write(
        fixture.root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"host\"]\nresolver = \"2\"\n",
    )
    .expect("workspace manifest");
    std::fs::write(
        fixture.root.join("host/Cargo.toml"),
        "[package]\nname = \"mcp-plugin-virtual-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("member manifest");
    std::fs::write(fixture.root.join("host/src/lib.rs"), "// host entry\n").expect("member entry");
    // The root's own manifest is a virtual one, and the fixture package's is gone.
    // 根本身的清单是虚拟清单，而夹具包的清单已被替换。
    let reply = crate::mcp::tools::tool_call(
        &fixture.root,
        json!(1),
        &json!({"name": "xirang.plugin", "arguments": user_request("sha256:00", true, true)}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("a text reply");
    // A refusal this entrance renders is an answer rather than a failed call — the same
    // reading the existing `apply`-on-a-virtual-root pin records — so `isError` stays false
    // and the candidates are what the caller reads.
    // 本入口渲染出的拒绝是答案而不是失败的调用——与既有的"虚拟根上的 `apply`"钉子记下的口径相同
    // ——因此 `isError` 保持 false，调用方读到的是那些候选。
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert!(
        text.contains("REFUSED: xirang.plugin needs one package"),
        "{text}"
    );
    assert!(text.contains("candidates"), "{text}");
    assert!(text.contains("host"), "{text}");
    assert!(
        !fixture.root.join(".xirang").exists() && !fixture.root.join("host/.xirang").exists(),
        "the refusal wrote something:\n{text}"
    );
}

/// A request missing one of the seven fields is refused by name, and a crate that is not
/// an identifier is refused before anything is read.
/// 缺少七个字段之一的请求被点名拒绝，而不是标识符的 crate 会在读取任何东西之前被拒绝。
#[test]
fn a_request_missing_a_field_is_refused_by_name() {
    let fixture = package("fields");
    for key in ["framework", "package", "version", "crate", "checksum"] {
        let mut request = user_request("sha256:00", false, false);
        request.as_object_mut().expect("an object").remove(key);
        let error =
            plugin(&fixture.root, &request).expect_err("a request without every field is refused");
        assert!(error.contains(key), "{key} is not named in: {error}");
    }
    let bad_source = plugin(
        &fixture.root,
        &json!({"source": "third-party", "framework": "f", "package": "p", "version": "1",
                "crate": "k", "checksum": "c", "mode": "extension"}),
    )
    .expect_err("an unknown source is refused");
    assert!(bad_source.contains("`official` or `user`"), "{bad_source}");

    let bad_crate = plugin(
        &fixture.root,
        &json!({"source": "user", "framework": "f", "package": "p", "version": "1",
                "crate": "demo-plugin", "checksum": "c", "mode": "extension"}),
    )
    .expect_err("a crate that is not an identifier is refused");
    assert!(bad_crate.contains("Rust identifier"), "{bad_crate}");
}
