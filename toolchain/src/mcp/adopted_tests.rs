//! Pins for the adoption surface: a lease reads as provisional, a moved byte reads as lapsed
//! with the file named, and a preview writes nothing.
//! 采信面的钉子：租约读作 provisional、动过的字节读作 lapsed 并点名文件、预览什么都不写。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

use super::adopted;

fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-adopted-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(&root.join("src/lib.rs"), "// entry\n");
    root
}

fn write_fixture(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

/// Write a ledger whose single entry was taken on the bytes in `src/lib.rs` right now.
/// 写一份台账，其中唯一一条条目是针对 `src/lib.rs` **此刻**的字节做出的。
fn adopt_current(root: &Path) -> String {
    let contents = std::fs::read_to_string(root.join("src/lib.rs")).expect("fixture source");
    let fingerprint =
        xirang_kernel::adoption::adoption_fingerprint(&[("src/lib.rs".to_owned(), contents)]);
    let line = format!(
        "root/button|chain+impl|converge: 6 live edges|maintainer|2026-09-29T14:00:00Z|src/lib.rs|{fingerprint}|first adoption\n"
    );
    write_fixture(&root.join(".xirang/adopted/entries"), &line);
    line
}

#[test]
fn a_lease_whose_bytes_are_unchanged_reads_as_provisional() {
    let root = package("provisional");
    adopt_current(&root);
    let text = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(text.contains("(provisional)"), "{text}");
    assert!(text.contains("provisional 1  lapsed 0"), "{text}");
    assert!(
        !text.contains("verified") && !text.contains("guaranteed"),
        "an adoption is a lease, never a certificate: {text}"
    );
    // The state is not the whole answer: a route the ledger does not name needs the action spelled
    // out, because the round measured an arm that read the rules and concluded it should do nothing.
    // 状态不是全部答案：台账没点名的路线需要把**动作**说出来——那轮量到的正是一臂读懂了规矩、
    // 于是推出"什么都不该做"。
    assert!(
        text.contains("new anchor") && text.contains("apply: true"),
        "the read names the action for a route the ledger does not carry: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_moved_byte_lapses_the_lease_and_the_file_is_named() {
    let root = package("lapsed");
    adopt_current(&root);
    write_fixture(&root.join("src/lib.rs"), "// entry changed\n");
    let text = adopted(&root, &json!({})).expect("the ledger answers");
    // The file that moved is named, and the verdict rides with it. The two prints used to be asserted
    // here ("beside both fingerprints") — that contract was **replaced**, not relaxed: the answer now
    // says which file moved instead of quoting two hashes at the reader (see
    // `an_entry_reads_as_a_verdict_rather_than_a_print` for the other half of the same contract).
    // 动了的那条被点名，判定跟着它。那两个指纹以前钉在这里（"beside both fingerprints"）——那条契约是**被替换**
    // 而不是被放宽：答案现在说"哪个文件动了"，而不是把两串散列摆给读者看（同一契约的另一半见
    // `an_entry_reads_as_a_verdict_rather_than_a_print`）。
    assert!(
        text.contains("adoption lapsed at src/lib.rs (changed since the confirmation)")
            && text.contains("); needs confirmation"),
        "the file that moved is named, with its verdict: {text}"
    );
    assert!(text.contains("provisional 0  lapsed 1"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Both fingerprints ride on the entry, and the byte that moves is the byte that changes them.
/// 两串指纹都挂在条目上，而"动的那个字节就是改变它的那个字节"。
///
/// Measured need (W8, h2): the client wanted to know whether the entry's *recorded* fingerprint
/// described some other file's current bytes. The reply carried neither value, so it read this
/// tree's sources and rebuilt the kernel's composition rule by hand — **18,177 characters of
/// reasoning, 11.5% of that arm's whole chain**. Two twelve-character strings remove the reason for
/// that work to exist.
/// 量出来的需求（W8 的 h2）：客户端想知道条目的**记录**指纹是否描述了别的文件的当前字节。回复里两个值
/// 都没有，于是它去读本树的源码、手工把内核的组合规则重拼出来——**18,177 字符的推理，占该臂整条链的
/// 11.5%**。两串十二个字符，就把那段工作存在的理由去掉了。
#[test]
fn an_entry_reads_as_a_verdict_rather_than_a_print() {
    let root = package("verdicts");
    adopt_current(&root);
    let unchanged = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        unchanged.contains("(unchanged since the confirmation)"),
        "an untouched lease says the bytes are the ones it was taken on: {unchanged}"
    );
    // The ledger still holds a full sha — that is the record, and the comparison key — but the
    // **answer** must not lead with it: a twelve-character hex string is a comparison key, not
    // information, and a reader can neither find a file with it nor change a line with it
    // (measured in T-21: an arm spent ~15,000 characters deriving what one of these prints meant).
    // 台账里仍然存着完整的 sha——那是记录、也是比较键——但**答案**不许拿它当主语：十二位十六进制是比较键、
    // 不是信息，读者既不能拿它找文件、也不能拿它改一行（T-21 实测：某一臂花了约 15,000 字符去推它是什么意思）。
    let ledger = std::fs::read_to_string(root.join(".xirang/adopted/entries")).expect("the ledger");
    let recorded = ledger
        .lines()
        .find(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .and_then(|line| line.split('|').nth(6))
        .expect("a recorded fingerprint");
    assert_eq!(recorded.len(), 64, "the ledger holds a full fingerprint");
    assert!(
        !unchanged.contains(&recorded[..12]),
        "no twelve-character print reaches the reader: {unchanged}"
    );
    // 反证：动一个字节 ⇒ 同一个条目必须说"变了"，并点名那个文件（单文件条目说得出来）。
    write_fixture(&root.join("src/lib.rs"), "// a byte moved\n");
    let lapsed = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        lapsed.contains("changed since the confirmation")
            && lapsed.contains("lapsed at src/lib.rs"),
        "a moved byte reads as changed, at a named path: {lapsed}"
    );
    assert!(
        !lapsed.contains(&recorded[..12]),
        "and the print still does not reach the reader: {lapsed}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_multi_file_lease_says_that_the_record_cannot_name_the_file_that_moved() {
    let root = package("multi-file");
    let contents = std::fs::read_to_string(root.join("src/lib.rs")).expect("fixture source");
    let other = "// a second covered file\n".to_owned();
    write_fixture(&root.join("src/other.rs"), &other);
    let print = xirang_kernel::adoption::adoption_fingerprint(&[
        ("src/lib.rs".to_owned(), contents),
        ("src/other.rs".to_owned(), other),
    ]);
    write_fixture(
        &root.join(".xirang/adopted/entries"),
        &format!(
            "root/button|chain+impl|converge|maintainer|2026-09-29T14:00:00Z|src/lib.rs,src/other.rs|{print}|first adoption\n"
        ),
    );
    write_fixture(&root.join("src/other.rs"), "// this one moved\n");
    let answer = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        answer.contains("which one is **not in the record**"),
        "the record's own limit is stated rather than hidden: {answer}"
    );
    assert!(
        !answer.contains(&print[..12]),
        "and the limit is stated without a print: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The twelve characters that follow one of the two labels, when the reply carries them.
/// 回复带着它们时，两个标签之一后面的那十二个字符。

#[test]
fn a_renewal_is_a_preview_until_it_is_applied_and_confirmed() {
    let root = package("renew");
    let request = json!({
        "anchor": "root/button",
        "certifies": "chain+impl",
        "evidence": "trace run",
        "verifier": "maintainer",
        "reason": "re-confirmed after the chain fix",
        "files": ["src/lib.rs"],
    });
    let preview = adopted(&root, &request).expect("a preview");
    assert!(preview.contains("nothing was written"), "{preview}");
    assert!(!root.join(".xirang/adopted/entries").exists());
    let applied = adopted(
        &root,
        &json!({
            "anchor": "root/button",
            "certifies": "chain+impl",
            "evidence": "trace run",
            "verifier": "maintainer",
            "reason": "re-confirmed after the chain fix",
            "files": ["src/lib.rs"],
            "apply": true,
            "confirm": true,
        }),
    )
    .expect("an append");
    assert!(applied.contains("append-only"), "{applied}");
    let written = std::fs::read_to_string(root.join(".xirang/adopted/entries")).expect("ledger");
    assert!(
        written.contains("re-confirmed after the chain fix"),
        "{written}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The anchor's claim is answered in three states: unknown, lapsed at a named file, and the newest
/// revision in force when an anchor has a history.
/// 一个 anchor 的声明有三种状态：没有条目、在点名文件上失效、以及有历史时**最后一条**生效。
#[test]
fn conformance_answers_unknown_lapsed_and_in_force() {
    let root = std::env::temp_dir().join(format!("mcp-conformance-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".xirang/adopted")).expect("ledger directory");
    std::fs::write(root.join("src.rs"), "pub fn one() {}\n").expect("a file to cover");
    let ledger = root.join(".xirang/adopted/entries");
    std::fs::write(
        &ledger,
        "root/control/button|first reading|traced once|nich|2026-10-01T10:00:00+08:00|src.rs|cafe|first\n\
         root/control/button|second reading|traced twice|nich|2026-10-01T11:00:00+08:00|src.rs|cafe|second\n",
    )
    .expect("ledger");

    let unknown = super::conformance(&root, &json!({"anchor": "root/nope"})).expect("an answer");
    assert!(
        unknown.starts_with("no ledger entry names `root/nope`"),
        "{unknown}"
    );

    let answer =
        super::conformance(&root, &json!({"anchor": "root/control/button"})).expect("an answer");
    assert!(
        answer.contains("2 revision(s)") && answer.contains("2026-10-01T11:00:00+08:00"),
        "the newest revision is the one in force (its `at` is printed): {answer}"
    );
    assert!(
        answer.contains("lapsed at") && answer.contains("needs a **person**"),
        "a fingerprint that does not match the bytes is lapsed, and the file is named: {answer}"
    );
    assert!(
        answer.contains("covers     src.rs") && answer.contains("not covered here"),
        "the covered files and the bounds ride along: {answer}"
    );

    // The contract here was **replaced**, not relaxed: asking without an anchor used to be an error
    // ("an anchor is required"), and it now lists every anchor, one line each. Measured (T-21): two
    // entries cost two calls, and what a caller wants first is "which of these still hold".
    // 这条契约是**被替换**而不是被放宽：不带 anchor 以前是错误（"an anchor is required"），现在逐行列出每个
    // 锚点。量到的（T-21）：两条条目要两次调用，而调用方首先想知道的是"这些里面哪些还成立"。
    let all = super::conformance(&root, &json!({})).expect("no anchor lists every anchor");
    assert!(
        all.contains("  lapsed      root/control/button")
            && all.contains("provisional 0  lapsed 1"),
        "every anchor is listed with its verdict: {all}"
    );
    assert!(
        all.contains("for one anchor\'s history"),
        "and the way to one anchor's history is named: {all}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// One missing covered file is that entry's fact, never the whole query's error.
/// 一个缺失的被覆盖文件是那条条目的事实，绝不该是整次查询的错误。
///
/// A01: two adopted entries, one missing and one valid. Aggregate, named and family queries must
/// report the missing entry individually while still returning the valid one; a missing file is
/// never silently renewed into `provisional`.
/// A01：两条采信条目，一条文件缺失、一条有效。全量、具名与同族查询都必须逐条报出缺失项，
/// 同时仍然返回有效项；缺失文件绝不静默续期成 `provisional`。
#[test]
fn one_missing_entry_does_not_erase_the_valid_one() {
    let root = package("missing-entry");
    adopt_current(&root);
    let ledger = root.join(".xirang/adopted/entries");
    let mut text = std::fs::read_to_string(&ledger).expect("ledger");
    text.push_str(
        "root/gone|chain+impl|converge: 1 live edge|maintainer|2026-09-30T14:00:00Z|src/gone.rs|\
         0000000000000000000000000000000000000000000000000000000000000000|second adoption\n",
    );
    std::fs::write(&ledger, text).expect("ledger");

    let aggregate =
        adopted(&root, &json!({})).expect("aggregate reports each entry, not one error");
    assert!(
        aggregate.contains("root/button"),
        "the valid entry is still returned: {aggregate}"
    );
    assert!(
        aggregate.contains("root/gone"),
        "the missing entry is named: {aggregate}"
    );
    assert!(
        aggregate.contains("unreadable") || aggregate.contains("missing"),
        "the missing entry reads as missing/unknown rather than provisional: {aggregate}"
    );
    assert!(
        aggregate.contains("provisional 1"),
        "the missing entry is not counted as a current lease: {aggregate}"
    );

    let named =
        super::conformance(&root, &json!({"anchor": "root/gone"})).expect("named anchor answers");
    assert!(
        named.contains("missing") || named.contains("unreadable"),
        "a named query for the missing entry answers with its state: {named}"
    );

    let entries = super::entries(&root).expect("entries parse");
    let family = super::family_leases(&root, &entries, "root")
        .expect("family answers")
        .join("\n");
    assert!(
        family.contains("root/button") && family.contains("unreadable"),
        "the family query keeps the valid lease and marks the missing one: {family}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The ledger's next hint names **this** ledger's lapsed anchor, not a placeholder.
/// 台账的 next 提示点名**这份**台账里失效的那个 anchor，而不是占位符。
///
/// Measured (W8): `adopted`'s hint was prose with no call in it at all, and the round's arm spent a
/// call working out what to ask next. An instantiated `conformance {anchor: "…"}` is copyable.
/// 实测（W8）：`adopted` 的提示是**一段没有任何调用的散文**，那一轮的臂花了一次调用去弄清下一步该问什么。
/// 实例化后的 `conformance {anchor: "…"}` 是可粘贴的。
#[test]
fn the_ledgers_next_hint_names_the_anchor_that_lapsed() {
    let root = package("next-anchor");
    adopt_current(&root);
    write_fixture(&root.join("src/lib.rs"), "// a byte moved\n");
    let answer = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        answer.contains("next   `conformance {anchor: \"root/button\"}` says whether that lease"),
        "the call names the lapsed anchor: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A lapsed line says **whose** print its recorded one is, when it is another entry's.
/// 失效的那一行说出它记录的那串**是谁的**——当那串属于另一个条目时。
///
/// Measured (T-21, h2): the arm spent roughly 15,000 characters working out that the `edc72845…`
/// recorded in a line naming `panel.rs` is `button.rs`'s print. The ledger printed `recorded … ·
/// now …` and never said whose, so the reader had to recompute the other file's print by hand —
/// the same family as the round that made `why` hand out both fingerprint strings: a value the
/// bridge already holds, computed and thrown away.
/// 量到的（T-21 的 h2）：那一臂花了约 15,000 字符才弄清"点名 `panel.rs` 的那一行里记录的 `edc72845…` 是
/// `button.rs` 的"。台账印了 `recorded … · now …`，从不说"是谁的"，于是读者得自己把另一个文件的指纹算一遍——
/// 与"让 `why` 交出两串指纹"那一轮同族：桥本来握着的值，算出来又丢掉。
#[test]
fn a_lapsed_line_names_whose_print_it_carries() {
    let root = package("borrowed-print");
    // Two entries over two files: the second carries the **first file's** print, which is exactly the
    // shape that names one file and prints another.
    // 两条条目覆盖两个文件：第二条带着**第一个文件的**指纹，这正是"点名一个文件、印着另一个文件的指纹"的形状。
    let borrowed = adopt_current(&root);
    let fingerprint = borrowed
        .split('|')
        .nth(6)
        .expect("the helper writes the fingerprint in field 7")
        .to_owned();
    write_fixture(&root.join("src/other.rs"), "// other\n");
    let ledger = format!(
        "{borrowed}root/other|chain+impl|converge: 1 live edge|maintainer|2026-09-30T14:00:00Z|src/other.rs|{fingerprint}|a second adoption\n"
    );
    write_fixture(&root.join(".xirang/adopted/entries"), &ledger);
    let answer = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        answer.contains("the recorded print is `root/button`'s current print, not this file's"),
        "the lapsed line names whose print it carries: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A populated ledger names its own file, exactly as an empty one does.
/// 有内容的台账与空台账一样，点名它自己那个文件。
///
/// Measured (T-21): the arm spent shell calls on `find`/`ls` to locate `.xirang/adopted/entries`
/// before it could read the two lines it was asking about — the path was printed only in the empty
/// case, so the reader that had something to read was the one that had to go looking.
/// 量到的（T-21）：那一臂先花了若干 shell 调用 `find`/`ls` 去找 `.xirang/adopted/entries`，才能读它在问的
/// 那两行——路径只在空台账时印，于是**有东西可读的读者**恰恰是得自己去找的那一个。
#[test]
fn a_populated_ledger_names_its_own_file() {
    let root = package("names-its-file");
    adopt_current(&root);
    let answer = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        answer.contains(".xirang/adopted/entries (provisional by construction)"),
        "the first line carries the path a reader would open: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A lease over two files can say **which** one moved — and it does, end to end through the writer.
/// 覆盖两个文件的租约能说出**哪一个**动了——而且从写者到读者是端到端的。
///
/// Measured (T-21, h2): the arm spent ~15,000 characters on a ledger whose recorded print belonged to
/// another file, including `sha256sum` of the covered files and a `sed` over this module's own source
/// to learn how a fingerprint is composed. A record that keeps **one** print per entry cannot answer
/// "which file moved" — not because the tool is lazy, but because the bytes are gone — so the print
/// per file rides beside the entry as a comment line (which an older parser skips, so the record stays
/// readable in both directions).
/// 量到的（T-21 的 h2）：那一臂在一份"记录的指纹属于另一个文件"的台账上花了约 15,000 字符，其中包含对被覆盖
/// 文件跑 `sha256sum`、以及 `sed` 本模块自己的源码去搞清指纹怎么合成。一条条目只留**一个**指纹的记录答不了
/// "哪个文件动了"——不是工具懒，是那些字节已经不在了——因此逐文件指纹以**注释行**形式跟在条目旁边（更旧的解析器
/// 会跳过它，于是记录在两个方向上都可读）。
#[test]
fn a_two_file_lease_can_say_which_file_moved() {
    let root = package("per-file");
    write_fixture(&root.join("src/other.rs"), "// other\n");
    super::renew(
        &root,
        &json!({
            "anchor": "root/x",
            "certifies": "two files",
            "evidence": "e",
            "verifier": "v",
            "reason": "r",
            "files": ["src/lib.rs", "src/other.rs"],
            "apply": true,
            "confirm": true,
        }),
        &super::ledger_path(&root),
    )
    .expect("the lease is written");
    let ledger = std::fs::read_to_string(root.join(".xirang/adopted/entries")).expect("a ledger");
    assert!(
        ledger.contains("# prints root/x |"),
        "the per-file line is written beside the entry: {ledger}"
    );
    assert!(
        ledger.lines().filter(|line| !line.starts_with('#')).count() == 1,
        "and it is not an entry of its own: {ledger}"
    );
    write_fixture(&root.join("src/other.rs"), "// other moved\n");
    let answer = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        answer.contains("lapsed at src/other.rs"),
        "the file that moved is the one named: {answer}"
    );
    assert!(
        answer.contains("1 of 2 covered file(s)"),
        "and the line still says how much of the entry moved: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **ME04: a contract version change inside the specimen lapses the lease.**
/// **ME04：标本里的合同版本一变，租约就失效。**
///
/// The contract is declared in the specimen's own bytes, so "the contract moved" and "the bytes
/// moved" must be the same fact — and this fixture makes it hard to fake: the edit is the **same
/// length** as the original, so a lease that compared size (or that silently reused a record
/// because nothing looked newer) would still read `(provisional)`. It reads `lapsed` instead, and
/// names the file the reader has to look at.
/// 合同声明在标本自己的字节里，因此"合同变了"与"字节变了"必须是同一个事实——这个夹具还让造假变难：
/// 改动与原文件**等长**，于是若租约比的是大小（或因为"没有东西看起来更新"而悄悄复用记录），它会照样
/// 读成 `(provisional)`。它读成 `lapsed`，并点名读者要去看的那个文件。
#[test]
fn a_contract_version_change_inside_the_specimen_lapses_the_lease() {
    let root = package("contract");
    let before = "// entry\n// flow: control.render.v1|1|ControlInput|ControlFrame\n";
    write_fixture(&root.join("src/lib.rs"), before);
    adopt_current(&root);
    let after = before.replace("control.render.v1", "control.render.v2");
    assert_eq!(
        after.len(),
        before.len(),
        "the edit must not change the size"
    );
    write_fixture(&root.join("src/lib.rs"), &after);

    let text = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        text.contains("lapsed at src/lib.rs"),
        "the contract version moved, so the lease is not in force: {text}"
    );
    assert!(
        text.contains("changed since the confirmation"),
        "the bytes clause names the reason, not only the verdict: {text}"
    );
    assert!(
        text.contains("provisional 0  lapsed 1"),
        "the counter agrees with the row: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
