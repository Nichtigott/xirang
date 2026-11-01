//! Pins for reading a host's crate-shape declaration: it resolves, it publishes the lock, and every
//! way of writing something else is refused by name (audit `M7`, P3.1).
//! 读宿主 crate 形状声明的钉子：它解析、发布锁，而其它任何写法都被点名拒绝（审计 `M7`，P3.1）。

use std::fs;
use std::path::PathBuf;

use super::{read_shape_declaration, write_shape_lock};
// The declaration's writers ride the authoring surfaces, so these pins do too: a default build has no
// `declare`/`undeclare` to test, and dead code is a warning this workspace refuses.
// 声明的写入方随创作面走，因此这些钉子也是：默认构建里没有 `declare`/`undeclare` 可测，而死代码是本工作区
// 拒绝的告警。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
use super::{declare, undeclare};

/// Read the declaration the way a run does, then validate it — the two halves the pipeline keeps
/// apart (read once, use twice), put back together for a test.
/// 照一次运行的方式读声明、再校验它——管线拆开的那两半（读一次、用两次），在测试里合回来。
fn check_declaration(
    package_root: &std::path::Path,
    out_dir: &std::path::Path,
    rows: &[super::PruningRow],
) -> Result<(), String> {
    match super::read_shape_declaration(package_root)? {
        Some(declaration) => super::check_shape(&declaration, out_dir, rows),
        None => Ok(()),
    }
}

/// A throwaway package with a declaration and a two-face tree under it.
/// 一个一次性包：一份声明，以及声明之下两个面的树。
fn package(label: &str, declaration: &str) -> (PathBuf, PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-shape-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("src");
    let out = root.join("out");
    fs::create_dir_all(src.join("control/object/button")).expect("subtree directory");
    fs::write(
        src.join("control/control.rs"),
        "crate::root_object! {\n    kind: Control,\n}\n",
    )
    .expect("the root face");
    fs::write(
        src.join("control/object/button/button.rs"),
        "crate::control_object! {\n    kind: Button,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("the leaf face");
    fs::create_dir_all(src.join("panel/gauge")).expect("a container face's subtree");
    fs::write(
        src.join("panel/panel.rs"),
        "crate::root_object! {\n    kind: Panel,\n}\n",
    )
    .expect("the container face");
    fs::write(
        src.join("panel/gauge/gauge.rs"),
        "crate::control_object! {\n    kind: Gauge,\n    parent: crate::panel::NODE_ID,\n}\n",
    )
    .expect("the gauge face");
    fs::create_dir_all(src.join("control/registry_rule")).expect("second subtree directory");
    fs::write(
        src.join("control/registry_rule/registry_rule.rs"),
        "crate::control_object! {\n    kind: RegistryRule,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("the rule face");
    fs::create_dir_all(&out).expect("out directory");
    // A cargo-namable package: `declare` fills the template's `package_prefix` in from the host
    // package's name, which is the same value every generated crate name is built from — so it has to
    // be readable (`cargo metadata` needs a manifest and a target).
    // 一个 cargo 说得出名字的包：`declare` 用宿主包名填模板里的 `package_prefix`，而那是每个生成包名所依据
    // 的同一个值——因此它必须读得出来（`cargo metadata` 需要清单与一个 target）。
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("the host manifest");
    fs::write(root.join("src/lib.rs"), "//! A host.\n").expect("the lib target");
    fs::write(root.join("add_crates.rs"), declaration).expect("the declaration");
    (root, src, out)
}

/// The declaration a host writes, in the shape the reader accepts.
/// 宿主写下的声明，读取器接受的那种形状。
const DECLARATION: &str = r#"//! The shape.
use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[
        Crate::named("widgets").at(&[crate::control::object::SUBTREE]),
        Crate::named("panel").at(&[crate::panel::SUBTREE]),
    ],
};
"#;

/// A package with no declaration is the one-crate package, not a missing input.
/// 没有声明的包就是一个 crate 的包，而不是缺了输入。
#[test]
fn a_package_without_a_declaration_answers_none() {
    let (root, _, _) = package("absent", DECLARATION);
    fs::remove_file(root.join("add_crates.rs")).expect("remove the declaration");
    assert!(
        read_shape_declaration(&root)
            .expect("no declaration")
            .is_none()
    );
}

/// The declaration resolves to the crates it names, and the digest follows the file's bytes.
/// 声明解析成它点名的那些 crate，而摘要跟着文件的字节走。
#[test]
fn a_declaration_resolves_to_the_crates_it_names() {
    let (root, _, _) = package("reads", DECLARATION);
    let declaration = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares a shape");
    assert_eq!(declaration.package_prefix, "myapp");
    assert_eq!(
        declaration.crates,
        vec![
            ("widgets".to_owned(), vec!["control::object".to_owned()]),
            ("panel".to_owned(), vec!["panel".to_owned()]),
        ]
    );
    // The digest is the identity `NodeId` derives from the file's bytes (128 bits, 32 hex digits),
    // not a second hash of our own: the shape records the same kind of value everything else does.
    // 摘要就是 `NodeId` 从文件字节推导出的身份（128 位、32 个十六进制位），不是我们自己另算的第二种散列：
    // 形状记录的是与其它一切同类的值。
    assert_eq!(declaration.digest.len(), 32);
}

/// The lock answers which faces each crate would own, and leaves the rest with the host.
/// 锁回答每个 crate 会拥有哪些面，其余的留在宿主。
#[test]
fn the_lock_says_which_faces_each_crate_would_own() {
    let (root, src, out) = package("lock", DECLARATION);
    let declaration = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares a shape");
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let rows = crate::build_method::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    write_shape_lock(&out, &declaration, &rows).expect("the lock writes");
    let lock = fs::read_to_string(out.join(xirang_kernel::lexicon::ADD_CRATES_LOCK_FILE))
        .expect("the lock reads");
    assert!(
        lock.starts_with("# add-crates\txirang-crate-shape\n"),
        "{lock}"
    );
    assert!(lock.contains("package_prefix\tmyapp"), "{lock}");
    assert!(
        lock.contains("crate\twidgets\tsubtrees=control::object\tfaces=1"),
        "{lock}"
    );
    // The claimed node is a **container face**: its own face goes with the crate, and so does the one
    // below it.
    // 被认领的节点是一个**容器面**：它自己的面跟着 crate 走，它下面那个面也是。
    assert!(
        lock.contains("crate\tpanel\tsubtrees=panel\tfaces=2"),
        "{lock}"
    );
    // The host keeps the two faces no crate claimed: the root face and the rule face.
    // 宿主留着没有 crate 认领的那两个面：根面与规则面。
    assert!(lock.contains("host\tfaces=2"), "{lock}");
}

/// A second spelling of the same declaration is refused, because guessing is how a shape is decided
/// by accident.
/// 同一份声明的第二种拼写会被拒绝，因为猜测正是形状被意外决定的方式。
#[test]
fn another_spelling_is_refused_by_name() {
    let (root, _, _) = package(
        "spelling",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::REGISTRATION]),\n] };\n",
    );
    let refusal = read_shape_declaration(&root).expect_err("refused");
    assert!(refusal.contains("::SUBTREE"), "{refusal}");
    assert!(refusal.contains("add_crates.rs"), "{refusal}");
}

/// A crate with no subtree, an overlapping pair and a missing prefix are three different refusals.
/// 没有子树的 crate、重叠的一对、以及缺失的前缀，是三种不同的拒绝。
#[test]
fn the_empty_and_overlapping_shapes_are_refused_where_they_are_wrong() {
    let (no_subtree, _, _) = package(
        "nosubtree",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\"),\n] };\n",
    );
    assert!(
        read_shape_declaration(&no_subtree)
            .expect_err("nothing claimed")
            .contains("claims nothing")
    );

    let (prefix, _, _) = package(
        "prefix",
        "pub const SHAPE: Shape = Shape { crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE]),\n] };\n",
    );
    assert!(
        read_shape_declaration(&prefix)
            .expect_err("no prefix")
            .contains("package_prefix")
    );

    let (overlap, _, _) = package(
        "overlap",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE]),\n\
         Crate::named(\"tiny\").at(&[crate::control::object::button::SUBTREE]),\n] };\n",
    );
    let refusal = read_shape_declaration(&overlap).expect_err("overlap");
    assert!(refusal.contains("`widgets`"), "{refusal}");
    assert!(refusal.contains("`tiny`"), "{refusal}");
    assert!(refusal.contains("exactly one crate"), "{refusal}");
}

/// The rule the reader applies is the kernel's, so what the build refuses is what the host's own
/// crate refuses at load: one rule, two readers.
/// 读取器适用的规则是内核的，因此构建拒绝的东西与宿主自己的 crate 在装载时拒绝的是同一样东西：一条规则，
/// 两个读者。
#[test]
fn the_build_and_the_host_apply_one_rule() {
    let (root, _, _) = package(
        "onerule",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE]),\n\
         Crate::named(\"widgets\").at(&[crate::control::registry_rule::SUBTREE]),\n] };\n",
    );
    assert!(
        read_shape_declaration(&root)
            .expect_err("declared twice")
            .contains("declared twice")
    );
}

/// A claim on a **face** is refused, and the refusal names the node that does have a subtree.
/// 认领一个**面**会被拒绝，而拒绝点名那个真正拥有子树的节点。
///
/// The compiler refuses this one too (a face has no `SUBTREE` marker); this is the reader saying the
/// same rule for a declaration no crate mounts — and, because it reads the tree, it can name the way
/// forward instead of leaving the author to guess.
/// 编译器也会拒它（面没有 `SUBTREE` 标记）；这里是读取器对"没有 crate 挂载的声明"说同一条规则——而且因为它
/// 读得到树，它能点名出路，而不是让作者去猜。
#[test]
fn a_claim_on_a_face_names_the_node_that_has_a_subtree() {
    let (root, src, out) = package(
        "leafclaim",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"button\").at(&[crate::control::object::button::SUBTREE]),\n] };\n",
    );
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let rows = crate::build_method::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    let refusal = check_declaration(&root, &out, &rows).expect_err("refused");
    assert!(refusal.contains("is a face and not a subtree"), "{refusal}");
    assert!(refusal.contains("control::object"), "{refusal}");
}

/// A claim on a node with no faces below it is refused as an empty crate.
/// 认领一个下面没有面的节点，被拒为空 crate。
#[test]
fn a_claim_with_no_faces_below_it_is_an_empty_crate() {
    let (root, src, out) = package(
        "emptyclaim",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"nothing\").at(&[crate::nowhere::SUBTREE]),\n] };\n",
    );
    let nodes = crate::build_method::source_walk::discover_root(&src);
    let rows = crate::build_method::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    let refusal = check_declaration(&root, &out, &rows).expect_err("refused");
    assert!(refusal.contains("no faces below it"), "{refusal}");
    assert!(refusal.contains("would be empty"), "{refusal}");
}

/// Removing one crate leaves every other byte of the declaration alone.
/// 移除一个 crate 时，声明的其余每一个字节都保持原样。
///
/// This is the property a text edit exists for: `add_crates.rs` is hand-written source the author also
/// reads, so a writer that re-rendered it would reformat their code and drop their comments.
/// 这正是"文本编辑"存在的理由：`add_crates.rs` 是作者也会读的手写源码，一个重渲染的写入方会重排他们的代码、
/// 丢掉他们的注释。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn undeclaring_one_crate_touches_nothing_else() {
    let (root, _, _) = package("undeclare", DECLARATION);
    let edit = undeclare(&root, "panel").expect("panel is declared");
    assert!(
        edit.after.contains(r#"Crate::named("widgets")"#),
        "the other crate stays: {}",
        edit.after
    );
    assert!(
        !edit.after.contains(r#"Crate::named("panel")"#),
        "the named entry is gone: {}",
        edit.after
    );
    // Everything that is not the removed line is byte-identical.
    let kept: Vec<&str> = edit
        .before
        .lines()
        .filter(|line| !line.contains(r#"Crate::named("panel")"#))
        .collect();
    let now: Vec<&str> = edit.after.lines().collect();
    assert_eq!(kept, now, "no other line moved or changed shape");
    let diff = edit.diff();
    assert!(
        diff.contains(r#"-        Crate::named("panel")"#),
        "the preview shows the line that goes: {diff}"
    );
}

/// Removing the **last** crate takes the file with it: the host is one crate again, which is what a
/// declaration naming no crate would try (and fail) to mean.
/// 移除**最后一个** crate 会连文件一起带走：宿主回到"就是一个 crate"，而一份不点名任何 crate 的声明只会
/// 尝试表达那个意思（并且做不到）。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn undeclaring_the_last_crate_removes_the_declaration() {
    let (root, _, _) = package("last", DECLARATION);
    let first = undeclare(&root, "panel").expect("declared");
    assert!(!first.removes_file, "one crate still stands");
    first.apply().expect("the edit lands");
    let last = undeclare(&root, "widgets").expect("declared");
    assert!(last.removes_file, "the last one takes the file");
    last.apply().expect("the removal lands");
    assert!(
        !root.join("add_crates.rs").exists(),
        "the host declares nothing again"
    );
    assert!(
        read_shape_declaration(&root).expect("it reads").is_none(),
        "and the reader says this package is one crate"
    );
}

/// A name the declaration does not carry is refused **with the names it does**, so the caller can aim
/// the next call instead of guessing.
/// 声明里没有的名字会被拒绝，**并附上它确实带着的名字**，好让调用方据此瞄准下一次调用而不是去猜。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn undeclaring_an_unknown_crate_names_the_ones_that_are_there() {
    let (root, _, _) = package("unknown", DECLARATION);
    let refusal = undeclare(&root, "slider").expect_err("refused");
    assert!(refusal.contains("slider"), "{refusal}");
    assert!(
        refusal.contains("widgets") && refusal.contains("panel"),
        "the refusal lists what is declared: {refusal}"
    );
}

/// A host with no declaration has nothing to remove, and the refusal says so by name.
/// 没有声明的宿主没有东西可移除，拒绝会点名说清这一点。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn undeclaring_without_a_declaration_is_refused() {
    let (root, _, _) = package("none", DECLARATION);
    fs::remove_file(root.join("add_crates.rs")).expect("remove the declaration");
    let refusal = undeclare(&root, "widgets").expect_err("refused");
    assert!(refusal.contains("add_crates.rs"), "{refusal}");
    assert!(refusal.contains("no crates"), "{refusal}");
}

/// Declaring the first crate writes the canonical file, and declaring a second one appends a line
/// without disturbing the first.
/// 声明第一个 crate 会写下规范文件，声明第二个会在不动第一个的前提下追加一行。
/// The refusal must say **who** checks these paths, and name the fix.
/// 拒绝必须说清**谁**在检查这些路径，并点出改法。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn a_refusal_says_who_checks_the_paths_and_names_the_spelling_to_write() {
    let (root, _, _) = package("refusal-wording", DECLARATION);
    let path = root.join("add_crates.rs");
    let text = fs::read_to_string(&path).expect("the declaration");
    fs::write(
        &path,
        text.replace(
            "crate::control::object::SUBTREE",
            "crate::control::object::SUBTRE",
        ),
    )
    .expect("a typo");
    let refusal = read_shape_declaration(&root).expect_err("it is refused");
    assert!(
        refusal.contains("resolves these paths against the registration tree"),
        "it says XiRang resolves them: {refusal}"
    );
    assert!(
        !refusal.contains("the compiler checks the paths"),
        "and does not send the reader to a compiler that never sees the file: {refusal}"
    );
    assert!(
        refusal.contains("crate::control::object::SUBTREE"),
        "and names the spelling to write: {refusal}"
    );
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn declaring_appends_without_disturbing_what_is_there() {
    let (root, _, _) = package("declare", DECLARATION);
    fs::remove_file(root.join("add_crates.rs")).expect("start with no declaration");
    let first = declare(
        &root,
        "widgets",
        &["crate::control::object::SUBTREE".to_owned()],
    )
    .expect("the file is created");
    assert!(first.before.is_empty(), "there was no file");
    assert!(
        first.after.contains(r#"Shape::of("myapp""#),
        "the prefix comes from the host package: {}",
        first.after
    );
    first.apply().expect("it lands");
    let read = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares");
    assert_eq!(
        read.crates,
        vec![("widgets".to_owned(), vec!["control::object".to_owned()])],
        "the reader reads back what the writer wrote"
    );

    let second =
        declare(&root, "panel", &["crate::panel::SUBTREE".to_owned()]).expect("a second crate");
    assert!(
        second.after.contains(r#"Crate::named("widgets")"#),
        "the first entry is untouched: {}",
        second.after
    );
    second.apply().expect("it lands");
    let read = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares");
    assert_eq!(read.crates.len(), 2, "{:?}", read.crates);
}

/// Declaring the same name twice is refused rather than silently producing two crates with one name.
/// 同名声明两次会被拒绝，而不是静默产出两个同名的 crate。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn declaring_a_name_twice_is_refused() {
    let (root, _, _) = package("twice", DECLARATION);
    let refusal =
        declare(&root, "panel", &["crate::panel::gauge::SUBTREE".to_owned()]).expect_err("refused");
    assert!(refusal.contains("already declared"), "{refusal}");
}

/// A subtree that is not a path is refused before anything is written: the file is Rust, so a quote
/// or a newline in it would be a syntax error in somebody else's source.
/// 不是路径的子树会在写下任何东西之前被拒绝：这个文件是 Rust，里面出现引号或换行就是别人源码里的语法错误。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn declaring_a_non_path_is_refused() {
    let (root, _, _) = package("nonpath", DECLARATION);
    for bad in [r#"panel::frame"; evil()"#, "", "a\nb"] {
        let refusal = declare(&root, "widgets", &[bad.to_owned()]).expect_err("refused");
        assert!(
            refusal.contains("not a subtree path")
                || refusal.contains("claims no subtree")
                || refusal.contains("already declared"),
            "{bad:?} → {refusal}"
        );
    }
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// The last entry of a **single-line** list has no trailing comma, and it is still removable.
/// **单行**列表里的最后一条没有结尾逗号，而它同样可以被移除。
#[test]
fn undeclaring_works_on_a_single_line_declaration() {
    let inline = "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
                  pub const SHAPE: Shape = Shape {\n    package_prefix: \"myapp\",\n\
                  crates: &[Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE])],\n};\n";
    let (root, _, _) = package("inline", inline);
    let edit = undeclare(&root, "widgets").expect("the inline entry is removable");
    assert!(edit.removes_file, "it was the only entry");
    edit.apply().expect("it lands");
    assert!(
        read_shape_declaration(&root).expect("it reads").is_none(),
        "the host is one crate again"
    );
}

/// A declaration a writer broke in half is **not** read back.
/// 被写入方切成两半的声明**读不回来**。
///
/// This is the file `crates --declare --write` used to leave behind: the new `Crate::named(…)` landed
/// inside the previous entry, so an inner `&[` never closes. The reader answered from it anyway — it took
/// the text up to the first `}` — which is why `check` reported `ok` on a file that is not valid Rust
/// (audit `M7`, §M7.61). Nothing compiled that file either: it sits in the package root, and no crate
/// graph contains it.
/// 这就是 `crates --declare --write` 过去留下的那份文件：新的 `Crate::named(…)` 落进了上一条 entry 里，
/// 于是一个内层 `&[` 永不闭合。而读者照样从它作答——它取到第一个 `}` 为止——这正是 `check` 在一份不是合法
/// Rust 的文件上报 `ok` 的原因（审计 `M7`，§M7.61）。那份文件也没有任何东西编译它：它住在包根，没有任何
/// crate 图包含它。
#[test]
fn a_declaration_with_unbalanced_brackets_is_refused() {
    let root = declaration_root("unbalanced");
    std::fs::write(
        root.join("add_crates.rs"),
        // Verbatim from a real run of `crates --declare extra --subtree crate::input::SUBTREE --write`:
        // the new entry landed **inside** the previous one, so the array holds two adjacent expressions.
        // Every bracket still balances — which is exactly why counting them was not enough.
        // 逐字取自一次真实的 `crates --declare extra --subtree crate::input::SUBTREE --write`：新的 entry 落进了
        // 上一条**里面**，于是数组里有两个相邻的表达式。每一个括号仍然配平——这正是"只数括号"不够的原因。
        "pub fn add_crates() -> Shape {\n    Shape::of(\"dash\", &[\n        \
         Crate::named(\"dash-board\").at(&[crate::board::SUBTREE\n        \
         Crate::named(\"extra\").at(&[crate::input::SUBTREE]),\n]),\n    ])\n}\n",
    )
    .expect("the broken declaration");
    let refused =
        read_shape_declaration(&root).expect_err("a file that does not parse is not a declaration");
    assert!(
        refused.contains("does not read as one Rust expression"),
        "the refusal names what is wrong, in the reader's own words: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The legal spellings still read, including one whose string contains a brace.
/// 合法的拼法仍能读，包括一处字符串里含花括号的。
#[test]
fn a_quoted_brace_is_not_a_delimiter() {
    let root = declaration_root("quoted-brace");
    std::fs::write(
        root.join("add_crates.rs"),
        "pub fn add_crates() -> Shape {\n    Shape::of(\"a}b\", &[\n        \
         Crate::named(\"x\").at(&[crate::control::SUBTREE]),\n    ])\n}\n",
    )
    .expect("the declaration");
    let declaration = read_shape_declaration(&root)
        .expect("it balances once quoted text is skipped")
        .expect("it declares a shape");
    assert_eq!(declaration.package_prefix, "a}b");
    assert_eq!(declaration.crates.len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// A throwaway package root for one declaration file.
/// 一个只装一份声明文件的一次性包根。
fn declaration_root(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join("shape-decl")
        .join(format!(
            "xirang-declaration-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    root
}

/// Every **layout** a declaration is written in gains an entry that parses.
/// 声明被写下的**每一种版式**都要在加一条 entry 之后仍能解析。
///
/// The anchor defect had two shapes, and the first fix only covered one of them: a multi-line list found its
/// bracket wrong (the first `]` belongs to the first entry), and an **inline** list had no trailing comma to
/// inherit, so a whole entry inserted after it produced two adjacent expressions. Both were measured on real
/// runs, one after the other — which is why this pin is a table of layouts rather than a single case
/// (audit `M7`, §M7.61).
/// 锚点那个缺陷有两种版式，而第一次修复只覆盖了其中一种：跨行的列表把括号找错了（第一个 `]` 属于第一条
/// entry），而**单行**列表没有可继承的尾逗号，因此插进一整条 entry 之后会得到两个相邻的表达式。两者都是
/// 在真实运行里量到的，一个接一个——这就是这条钉子是一张版式表、而不是单个用例的原因（审计 `M7`，§M7.61）。
// `declare` rides the authoring surfaces, like the `declare`/`undeclare` imports at the top of this file: a
// default build has no writer to test.
// `declare` 随创作界面走，与本文件顶部的 `declare`/`undeclare` 导入相同：默认构建里没有写入方可测。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
#[test]
fn every_layout_gains_an_entry_that_parses() {
    let layouts = [
        // The template's constructor, one entry per line.
        // 模板的构造器，一条 entry 一行。
        "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
         pub fn add_crates() -> Shape {\n    Shape::of(\"app\", &[\n        \
         Crate::named(\"widgets\").at(&[crate::panel::frame::SUBTREE]),\n    ])\n}\n",
        // The same, all on one line: no trailing comma to inherit.
        // 同上，但全在一行里：没有可继承的尾逗号。
        "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
         pub fn add_crates() -> Shape {\n    \
         Shape::of(\"app\", &[Crate::named(\"widgets\").at(&[crate::panel::frame::SUBTREE])])\n}\n",
        // The struct literal, one entry per line.
        // 结构体字面量，一条 entry 一行。
        "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
         pub const SHAPE: Shape = Shape {\n    package_prefix: \"app\",\n    \
         crates: &[\n        Crate::named(\"widgets\").at(&[crate::panel::frame::SUBTREE]),\n    ],\n};\n",
        // The struct literal inline.
        // 结构体字面量，单行。
        "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
         pub const SHAPE: Shape = Shape {\n    package_prefix: \"app\",\n    \
         crates: &[Crate::named(\"widgets\").at(&[crate::panel::frame::SUBTREE])],\n};\n",
    ];
    for (index, layout) in layouts.iter().enumerate() {
        let root = declaration_root(&format!("layout-{index}"));
        fs::write(root.join("add_crates.rs"), layout).expect("the declaration");
        let edit = super::declare(&root, "gauge", &["crate::panel::gauge::SUBTREE".to_owned()])
            .unwrap_or_else(|error| panic!("layout {index} is declarable: {error}"));
        fs::write(root.join("add_crates.rs"), &edit.after).expect("the write");
        let read = read_shape_declaration(&root).unwrap_or_else(|error| {
            panic!(
                "layout {index} must read back after the write: {error}\n{}",
                edit.after
            )
        });
        let read = read.unwrap_or_else(|| panic!("layout {index} declares a shape"));
        assert_eq!(
            read.crates.len(),
            2,
            "layout {index} gained exactly one crate:\n{}",
            edit.after
        );
        assert!(
            edit.after.contains(r#"Crate::named("widgets")"#),
            "layout {index} kept what was there:\n{}",
            edit.after
        );
        let _ = fs::remove_dir_all(&root);
    }
}
