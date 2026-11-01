//! Tests for the audit manifests: which functions a file declares, and which
//! symbol the release prunes.
//! 审计清单的测试：一个文件声明了哪些函数，以及发布期修剪的是哪个符号。

use std::fs;
use std::path::PathBuf;

/// A throwaway package with one face file, so a manifest writer has a face to key
/// its rows on. `body` is appended after the face invocation, because one file may
/// hold only one registration face.
/// 一个含单个注册面文件的一次性包，因此清单写入方有面可以给行做键。`body` 追加在面调用之后，
/// 因为一个文件只能承载一个注册面。
fn package(label: &str, body: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-manifests-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("src");
    fs::create_dir_all(src.join("dial")).expect("face directory");
    fs::write(
        src.join("dial/dial.rs"),
        format!("crate::root_object! {{\n    kind: Dial,\n}}\n{body}"),
    )
    .expect("face file");
    (root, src)
}

/// The mask the kernel scanner applies is the point of this column: `fn ` inside a
/// comment or a doc example is not a declaration, and the line scanner this file
/// used to carry listed both.
/// 内核扫描器所做的屏蔽正是这一列的意义：注释或文档示例里的 `fn ` 不是声明，而本文件过去带的那套
/// 按行扫描器把两者都列了出来。
#[test]
fn a_commented_out_function_is_not_a_declaration() {
    let (root, src) = package(
        "masked",
        "pub fn real() {}\n\
         // fn ghost() {}\n\
         /* fn in_a_block_comment() {} */\n\
         /// A doc example: fn documented() {}\n\
         pub fn after_doc() {}\n\
         pub fn with_string() { let text = \"fn in_a_string() {}\"; let _ = text; }\n",
    );
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    super::write_function_manifest(&src, &nodes, &out).expect("the manifest writes");
    let text = fs::read_to_string(out.join("function_manifest.tsv")).expect("rows");
    assert!(text.contains("dial::real"), "{text}");
    assert!(text.contains("dial::after_doc"), "{text}");
    assert!(text.contains("dial::with_string"), "{text}");
    for ghost in ["ghost", "in_a_block_comment", "documented", "in_a_string"] {
        assert!(
            !text.contains(ghost),
            "`{ghost}` is not a declaration: the kernel scanner masks what is not Rust: {text}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_method_carries_no_impl_owner_the_scanner_cannot_know() {
    let (root, src) = package("owner", "impl Dial {\n    pub fn method(&self) {}\n}\n");
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    super::write_function_manifest(&src, &nodes, &out).expect("the manifest writes");
    let text = fs::read_to_string(out.join("function_manifest.tsv")).expect("rows");
    assert!(text.contains("dial::method"), "{text}");
    assert!(
        !text.contains("Dial::method"),
        "the impl owner was a guess, and it is gone: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A tracked symbol comes from the facts that name it: a module item keeps its
/// module path, and `optional_pruning_probe` — a method on the face's own type —
/// starts at that type's name, which is the face's `kind`. The row scanner this
/// replaces returned the literal `Button::optional_pruning_probe` for every face.
/// 被跟踪的符号来自命名它的事实：模块条目保留模块路径，而 `optional_pruning_probe`——面自己类型上
/// 的方法——从那个类型的名字开始，也就是面的 `kind`。它替换掉的按行扫描器对每个面都返回字面量
/// `Button::optional_pruning_probe`。
#[test]
fn a_pruning_probe_symbol_is_named_after_the_face_kind() {
    let (root, src) = package(
        "pruning",
        "impl Dial {\n    pub fn optional_pruning_probe(&self) {}\n}\n\n\
         pub static PRUNING_TABLE: &[&str] = &[];\n",
    );
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes");
    let text = fs::read_to_string(out.join("pruning_manifest.tsv")).expect("rows");
    assert!(text.contains("Dial::optional_pruning_probe"), "{text}");
    assert!(
        !text.contains("Button::optional_pruning_probe"),
        "the symbol must come from this face's kind, not from a literal: {text}"
    );
    assert!(
        text.contains("dial::PRUNING_TABLE"),
        "a module item keeps its module path: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The record carries the three facts audit `W3-1` added, and a derivation agrees with them.
/// 记录带着审计 `W3-1` 新增的三项事实，而一次推导与它们一致。
///
/// The acceptance is an **对账**: the columns are only worth publishing if a reader that derives
/// the same facts by reading the sources gets the same answers. So the pin recomputes each column
/// from the file with the same kernel functions a reader would use, and compares — a record that
/// disagreed with the derivation would be worse than no record, because the readers would trust it.
/// 验收是一次**对账**：这些列只在"自己读源码推导出同样事实的读者得到同样答案"时才值得发布。因此钉子用
/// 读者会用的同一批内核函数从文件重算每一列并比对——与推导不一致的记录比没有记录更糟，因为读者会信它。
#[test]
fn the_record_carries_the_source_hash_fields_and_calls_a_derivation_agrees_with() {
    let (_root, src) = package(
        "record-facts",
        "pub fn paint(&self) -> i32 { helper() }\nfn helper() -> i32 { 7 }\n",
    );
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let out = src.join("out");
    fs::create_dir_all(&out).expect("out dir");
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes");
    let record = fs::read_to_string(out.join("pruning_manifest.tsv")).expect("the record");
    let header = record.lines().next().unwrap_or_default();
    assert_eq!(
        header,
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls\tparent_node\towns_registry\tlogical_path",
        "the columns are the record's contract"
    );

    let text = fs::read_to_string(src.join("dial/dial.rs")).expect("the face source");
    let expected_hash = xirang_kernel::sha256_hex(text.as_bytes());
    let mut expected_calls: Vec<String> = xirang_kernel::source::function_symbols(&text)
        .into_iter()
        .flat_map(|function| xirang_kernel::source::direct_calls(&function.body, &function.name))
        .collect();
    expected_calls.sort();
    expected_calls.dedup();

    let row = record
        .lines()
        .find(|line| line.contains("dial/dial.rs"))
        .expect("a row for the face");
    let columns = row.split('\t').collect::<Vec<_>>();
    assert_eq!(columns.len(), 13, "thirteen columns: {row}");
    assert_eq!(
        columns[7], expected_hash,
        "the source hash is the file's own sha256: {row}"
    );
    assert!(
        columns[9].contains("helper"),
        "the calls column names what the file calls: {row}"
    );
    assert_eq!(
        columns[9],
        expected_calls.join(","),
        "and it is the kernel's own direct-call rule, sorted: {row}"
    );
    // The fingerprint is a hash, so the **check** that it is well-formed is its length and its
    // stability across two runs — a fingerprint that moved between identical builds would make
    // every reader fall back to deriving, silently.
    // 指纹是散列，因此对它的**检查**是长度与"两次运行之间稳定"——在两次相同构建之间会变的指纹，会让每个
    // 读者静默地回落到推导。
    assert_eq!(
        columns[8].len(),
        64,
        "the field fingerprint is a sha256: {row}"
    );
    // Audit `W3-1b`: the fixture's face declares no `parent`, which means **the package root** —
    // and the record publishes that identity rather than leaving the reader to derive it.
    // 审计 `W3-1b`：夹具的面没有声明 `parent`，那意味着**包根**——而记录发布的就是那个身份，不留给读者推导。
    // The identity itself is compared in `a_module_named_parent_resolves_to_its_identity`, where
    // the expected value is another row's own `id` — no namespace arithmetic on the test's side.
    // Here the claim is only that the column is **present and well-formed**: this face declares no
    // `parent`, which means the package root, and a `-` here would mean the writer could not resolve
    // something it demonstrably can.
    // 身份本身在 `a_module_named_parent_resolves_to_its_identity` 里比，那里的期望值是**另一行自己的
    // `id`**——测试这一侧不做命名空间算术。这里只主张这一列**在且形状对**：这个面没声明 `parent`，那意味着
    // 包根，而这里写 `-` 就意味着写入方解析不出一件它明明解析得了的事。
    assert_eq!(
        columns[10].len(),
        32,
        "the resolved parent is a node id: {row}"
    );
    assert_eq!(
        columns[11], "false",
        "and the fixture declares no registry of its own: {row}"
    );
    // Audit `W3-2`: the logical path is the **derived** answer (the walk over resolved parents), not
    // what the declaration spelled — and this fixture's face declares no `path` at all, which is the
    // case the column exists for.
    // 审计 `W3-2`：逻辑路径是**推导出来的**答案（沿解析后的父链走），不是声明拼出的东西——而这个夹具的
    // 面根本没声明 `path`，正是这一列为之存在的情形。
    assert_eq!(
        columns[12], "root/dial",
        "the logical path is the derived one: {row}"
    );
    let before = columns[8].to_owned();
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes again");
    let again = fs::read_to_string(out.join("pruning_manifest.tsv")).expect("the record");
    assert!(
        again.contains(&before),
        "and the same sources produce the same fingerprint"
    );
}

/// Every column the writer spells is one the reader reads back (audit `W3-1`).
/// 写入方拼出的每一列，读取方都读得回来（审计 `W3-1`）。
///
/// The rule this pins is the repo's own: a text contract has **one** renderer and **one** parser,
/// and a round trip is what proves they agree. Without it, three new columns would be write-only —
/// the writer would publish facts no reader could reach, which is the "default surface lags the
/// capability surface" defect in its other direction.
/// 这条钉子守的是本仓自己的规矩：一份文本契约**一份**渲染方与**一份**解析方，而回环是证明两者一致的
/// 东西。没有它，三个新列就是"只写不读"——写入方发布的事实没有任何读者够得着，正是"承诺面落后于能力面"
/// 那个缺陷的另一个方向。
#[test]
fn every_published_column_reads_back() {
    let (root, src) = package(
        "round-trip",
        "pub fn paint(&self) -> i32 { helper() }\nfn helper() -> i32 { 7 }\n",
    );
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let nodes = crate::build_method::source_walk::discover_root(&src);
    crate::build_method::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the manifest writes");

    let rows = crate::build_method::read_pruning_manifest(&out).expect("the record reads");
    let row = rows
        .iter()
        .find(|row| row.source == "dial/dial.rs")
        .expect("a row for the face");
    assert_eq!(
        row.kind.as_deref(),
        Some("Dial"),
        "the declared kind reads back: {row:?}"
    );
    let hash = row
        .source_hash
        .as_deref()
        .expect("the source hash reads back");
    assert_eq!(hash.len(), 64, "and it is a sha256: {hash}");
    let text = fs::read_to_string(src.join("dial/dial.rs")).expect("the source");
    assert_eq!(
        hash,
        xirang_kernel::sha256_hex(text.as_bytes()),
        "and it is the hash of the bytes the build was looking at"
    );
    assert_eq!(
        row.fields.as_deref().map(str::len),
        Some(64),
        "the field fingerprint reads back: {row:?}"
    );
    assert_eq!(
        row.calls.as_deref(),
        Some("helper"),
        "the direct calls read back: {row:?}"
    );
    assert_eq!(
        row.logical_path.as_deref(),
        Some("root/dial"),
        "the logical path reads back: {row:?}"
    );

    // A record from before these columns still parses: the older three-column form leaves every
    // optional column `None` rather than failing, because a reader that would rather derive than
    // trust a published value has to survive an old record.
    // 早于这些列的记录照样解析：较早的三列形式让每个可选列为 `None` 而不是失败，因为"宁可推导也不采信
    // 已发布值"的读者必须能活过一份旧记录。
    fs::write(
        out.join("pruning_manifest.tsv"),
        "# node\tsource\tsymbol\nbdb4427ce81c9bc51e56bee7667fd2be\tcontrol/control.rs\t-\n",
    )
    .expect("an old record");
    let old = crate::build_method::read_pruning_manifest(&out).expect("an old record reads");
    assert_eq!(old.len(), 1, "the old form still parses: {old:?}");
    assert!(
        old[0].source_hash.is_none() && old[0].calls.is_none(),
        "and its absent columns are `None`, not an error: {old:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A child whose parent is named by module path gets that parent's **identity**, not just its path.
/// 一个按模块路径点名父级的子面，拿到的是父级的**身份**，而不只是路径。
///
/// Audit `W3-1b`: `FaceView` needs the resolved `parent`, and the record published only the Rust path
/// the declaration spelled — so a reader that wanted whole faces still had to walk the sources. The
/// two-pass writer is what closes that: pass one indexes every face's module, pass two resolves. The
/// pin walks both shapes the resolver has — a module the tree owns, and a parent that names
/// something this tree does not have (`-`, which is the honest answer rather than a guess).
/// 审计 `W3-1b`：`FaceView` 需要解析后的 `parent`，而记录只发布声明拼出的 Rust 路径——因此想要整份面的
/// 读者仍然得走一遍源码。两遍写入正是关掉这一点：第一遍索引每个面的模块，第二遍解析。钉子走解析器的两种
/// 形状——树拥有的模块，与点名了树里没有的东西的父级（`-`，那是诚实的答案而不是猜）。
#[test]
fn a_module_named_parent_resolves_to_its_identity() {
    let (root, src) = package("parent-node", "");
    // A child whose `parent:` names the root face's module, and a second one whose parent names a
    // module nothing owns.
    fs::create_dir_all(src.join("child")).expect("child directory");
    fs::write(
        src.join("child/child.rs"),
        "crate::control_object! {\n    kind: Child,\n    parent: crate::dial::NODE_ID,\n}\n",
    )
    .expect("child face");
    fs::create_dir_all(src.join("orphan")).expect("orphan directory");
    fs::write(
        src.join("orphan/orphan.rs"),
        "crate::control_object! {\n    kind: Orphan,\n    parent: crate::nowhere::NODE_ID,\n}\n",
    )
    .expect("orphan face");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let nodes = crate::build_method::source_walk::discover_root(&src);
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes");
    let rows = crate::build_method::read_pruning_manifest(&out).expect("the record reads");

    let dial = rows
        .iter()
        .find(|row| row.source == "dial/dial.rs")
        .expect("the parent face");
    let child = rows
        .iter()
        .find(|row| row.source == "child/child.rs")
        .expect("the child face");
    assert_eq!(
        child.parent_node.as_deref(),
        Some(dial.id.to_string().as_str()),
        "the child's resolved parent is the parent face's own identity"
    );
    assert_eq!(
        child.parent.as_deref(),
        Some("crate::dial::NODE_ID"),
        "and the spelled path is still published beside it"
    );
    let orphan = rows
        .iter()
        .find(|row| row.source == "orphan/orphan.rs")
        .expect("the orphan face");
    assert!(
        orphan.parent_node.is_none(),
        "a parent this tree does not own stays unresolved rather than guessed: {orphan:?}"
    );
    assert_eq!(
        dial.owns_registry.as_deref(),
        Some("false"),
        "`owns_registry` is published for every face: {dial:?}"
    );
    assert_eq!(
        child.logical_path.as_deref(),
        Some("root/dial/child"),
        "the logical path nests under the parent's registry name: {child:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The shape record carries what each declaration spelled, and reads back whole (audit `W6-2`, step three).
/// 形状记录携带每条声明拼出的东西，并能整份读回（审计 `W6-2` 第③步）。
///
/// This is the record the family comparison will read instead of lexing every sibling: the bridge's
/// `SHAPE_FIELDS` vocabulary decides what the pairs **mean**, and this side only publishes the pairs —
/// which is why the file is keyed by source path and carries raw text rather than a verdict.
/// 这是家族比对将要读、以取代"词法每个兄弟"的那份记录：桥的 `SHAPE_FIELDS` 词汇决定这些配对**意味着什么**，
/// 而这一侧只发布配对——这正是文件按源码路径做键、携带原始文本而不是结论的原因。
#[test]
fn the_shape_record_carries_the_declared_fields_and_reads_back() {
    let (root, src) = package("shape-record", "pub fn paint(&self) -> i32 { 7 }\n");
    fs::write(
        src.join("dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n    exports: [\"a\", \"b\"],\n    \
         handle_traits: [\"DialHandle\"],\n}\n",
    )
    .expect("the face with two families of fields");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let nodes = crate::build_method::source_walk::discover_root(&src);
    crate::build_method::write_shape_manifest(&src, &nodes, &out).expect("the record writes");
    let record = fs::read_to_string(out.join("shape_manifest.tsv")).expect("the record");
    assert_eq!(
        record.lines().next().unwrap_or_default(),
        "# source\tfield\tvalue",
        "the columns are the record's contract"
    );

    let rows = crate::build_method::read_shape_manifest(&out).expect("the record reads");
    let fields: Vec<(&str, &str)> = rows
        .iter()
        .filter(|row| row.source == "dial/dial.rs")
        .map(|row| (row.field.as_str(), row.value.as_str()))
        .collect();
    // The record carries the **raw** text the parser re-joined (`["a" , "b"]`), not a normalized
    // spelling: what the declaration said is the build-time fact; what it *means* is the bridge's
    // vocabulary (`SHAPE_FIELDS`), and normalizing here would be that second implementation.
    // 记录携带的是解析器重新拼出的**原始**文本（`["a" , "b"]`），不是规范化的拼写：声明说了什么是构建期
    // 事实，"它意味着什么"是桥的词汇（`SHAPE_FIELDS`），在这里做规范化就是那第二份实现。
    assert!(
        fields.contains(&("exports", "[\"a\" , \"b\"]"))
            && fields.contains(&("handle_traits", "[\"DialHandle\"]")),
        "the declaration's own text reads back: {fields:?}"
    );
    assert!(
        fields.contains(&("kind", "Dial")),
        "and `kind`, which the family comparison also reads: {fields:?}"
    );

    // A file with no registration face is not in the record at all, and a face that spells none of the
    // covered fields still gets a row — a reader that counts must see the whole family.
    // 没有注册面的文件根本不在记录里，而一个没声明任何被覆盖字段的面照样有一行——要计数的读者必须看得见整个家族。
    assert!(
        !rows
            .iter()
            .any(|row| row.source == "src/lib.rs" || row.source == "lib.rs"),
        "a non-face file has no row: {rows:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
