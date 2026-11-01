//! Tests for the face renderer: the field set it writes back, and the fields it
//! refuses to drop.
//! 注册面渲染器的测试：它写回的字段集合，以及它拒绝丢掉的字段。

#[cfg(test)]
mod field_truth_tests {
    use super::super::{OFF_DISK_KEYS, ON_DISK_FIELDS};
    use crate::run_method::authoring::manifest::parse;
    use std::path::PathBuf;

    /// The accepted set is the rendered set: every field an edit accepts comes
    /// back after render → parse, except where the round trip *canonicalizes* the
    /// spelling (named per row).
    /// 接受集就是渲染集：每个被 edit 接受的字段在 渲染 → 解析 之后都能读回，除了往返会**规范化**
    /// 拼写的那一个（逐行点名）。
    ///
    /// This is the test the audit found missing: the idempotency pin built its
    /// field table with `BTreeMap::from([...])` and went straight to
    /// `from_values → as_patch`, so it never parsed a file, applied an edit, or
    /// rendered — "the fields parse produced can survive a rewrite" was untested,
    /// and three accepted fields could not survive one at all (audit `LG-38`).
    /// 这正是审计指出缺失的那条测试：幂等钉子用 `BTreeMap::from([...])` 手搓字段表、直接走
    /// `from_values → as_patch`，因此它从未解析文件、应用编辑或渲染——"解析产生的字段能否活过
    /// 一次重写"根本没被验过，而有三个被接受的字段根本活不过（审计 `LG-38`）。
    #[test]
    fn the_round_trip_keeps_every_editable_field() {
        let root = unique_root("roundtrip");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            r#"// generated-by=XiRang
pub struct Card;

impl Card {
    pub const REGISTRATION: Registration = Registration {
        name: LocalizedText { zh: "卡片", en: "Card" },
        ..Registration::DEFAULT
    };
}
"#,
        )
        .expect("fixture face");

        // Every field `edit` accepts, with a value that survives the round trip.
        // Two rows keep the fixture's initial value (`registration_rule` and
        // `admission`, both `ANY`): a custom rule needs its sibling rule file, and a
        // non-`ANY` admission is a different spelling rather than a different
        // field, so their rows are the weak half of this pin — and `admission`'s
        // non-default rendering is covered by the refusal pin below plus
        // `the_keys_no_template_emits_are_refused_not_stored`'s counterpart in the
        // operations tests.
        // `runtime_checks` is the one canonicalization: the compact
        // `NON_EMPTY_TEXT` is written back as the expression
        // `crate::run_method::NON_EMPTY_TEXT`, and parsing that back yields the expression.
        // `edit` 接受的每个字段，取值都能活过往返。`runtime_checks` 是唯一的规范化：紧凑的
        // `NON_EMPTY_TEXT` 被写成表达式 `crate::run_method::NON_EMPTY_TEXT`，再解析回来得到的是表达式形式。
        // The count is deliberately not written here: the field *set* comes from
        // `ON_DISK_FIELDS` and the assertion below compares the two, so a number in
        // this line would be a third declaration of the set.
        // 这里有意不写数量：字段**集合**来自 `ON_DISK_FIELDS`，下面的断言会比对两者，因此在这一行
        // 写数字就等于对同一个集合的第三次声明。
        let table: &[(&str, &str, &str)] = &[
            ("kind", "Gadget", "Gadget"),
            ("preset", "crate::ProbePreset", "crate::ProbePreset"),
            ("parts", "crate::ProbeParts", "crate::ProbeParts"),
            ("name_zh", "组件", "组件"),
            ("name_en", "Widget", "Widget"),
            ("summary_zh", "摘要", "摘要"),
            ("summary_en", "summary", "summary"),
            ("stable_name", "widget-stable", "widget-stable"),
            ("exports", "widget", "widget"),
            ("provides", "canvas", "canvas"),
            ("handle_traits", "fast", "fast"),
            ("handle_contracts", "crate::Foo::Bar", "crate::Foo::Bar"),
            ("part_traits", "slow", "slow"),
            ("part_contracts", "crate::Foo::Baz", "crate::Foo::Baz"),
            ("requires", "canvas=>Canvas", "canvas=>Canvas"),
            ("runtime_checks", "NON_EMPTY_TEXT", "crate::NON_EMPTY_TEXT"),
            ("needs_registry", "true", "true"),
            ("registration_rule", "ANY", "ANY"),
            ("admission", "ANY", "ANY"),
            (
                "flow",
                "canvas|1|CanvasFrame|CanvasFrame",
                "canvas|1|CanvasFrame|CanvasFrame",
            ),
            (
                "flow_provider",
                "crate::flow::Render",
                "crate::flow::Render",
            ),
            (
                "getting_from_other_registry",
                "Some(\"engine\")",
                "Some(\"engine\")",
            ),
        ];

        // The rows are samples; the *set* of fields is not this table's to
        // declare. It comes from `ON_DISK_FIELDS`, and the two have to match, or
        // the renderer and its evidence are two half-lists again (audit
        // `LGC-LG-38`).
        // 这些行是样本；字段**集合**不是本表该声明的东西。它来自 `ON_DISK_FIELDS`，两者必须一致，
        // 否则渲染器与它的证据又是一份半的清单（审计 `LGC-LG-38`）。
        let mut sampled = table.iter().map(|(field, _, _)| *field).collect::<Vec<_>>();
        sampled.sort_unstable();
        let mut declared = ON_DISK_FIELDS.to_vec();
        declared.sort_unstable();
        assert_eq!(
            sampled, declared,
            "the sample table must cover exactly the rendered field set"
        );
        for key in OFF_DISK_KEYS {
            assert!(
                !ON_DISK_FIELDS.contains(key),
                "`{key}` is an input or metadata key, not an on-disk field"
            );
            assert!(
                !sampled.contains(key),
                "`{key}` must not be sampled as an editable field"
            );
        }

        // Two passes, because the trait labels and the contracts they name are
        // one rendered field each: with a contract present the declaration
        // *derives* the labels from it, so a manifest that sets both would read
        // the labels back from the contracts rather than from the row.
        // 分两遍，因为 trait 标签与它们点名的契约各自是同一处渲染：有契约在场时，声明会从它**派生**
        // 标签，因此同时设两者的清单会把标签读成契约派生的结果，而不是本行的取值。
        let without_contracts: Vec<_> = table
            .iter()
            .copied()
            .filter(|(field, _, _)| !field.ends_with("_contracts"))
            .collect();
        let without_traits: Vec<_> = table
            .iter()
            .copied()
            .filter(|(field, _, _)| !field.ends_with("_traits"))
            .collect();

        for (label, rows) in [
            ("without contracts", &without_contracts),
            ("without traits", &without_traits),
        ] {
            let root = unique_root(&format!("roundtrip-{}", label.replace(' ', "-")));
            let face_path = root.join("widget/widget.rs");
            std::fs::create_dir_all(face_path.parent().expect("fixture parent"))
                .expect("fixture dir");
            std::fs::write(
                &face_path,
                "// generated-by=XiRang\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
            )
            .expect("fixture face");

            let mut manifest = parse::source(&face_path, "", None).expect("the fixture parses");
            for (field, value, _) in rows.iter().copied() {
                manifest
                    .edit(field, value)
                    .unwrap_or_else(|error| panic!("{label}: {field} is not editable: {error}"));
            }
            let rendered = manifest.render_source().expect("the face renders");
            std::fs::write(&face_path, &rendered).expect("write the rendered face");
            let second =
                parse::source(&face_path, "", None).expect("the rendered face parses again");
            for (field, value, expected) in rows.iter().copied() {
                let back = second
                    .values
                    .get(field)
                    .map(String::as_str)
                    .unwrap_or("<absent>");
                assert_eq!(
                    back, expected,
                    "{label}: {field} was set to {value} and read back as {back}"
                );
            }
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    /// The declaration is the **first** thing a rendered face file says (after the module docs and the
    /// imports), with the marker type it names right below it: a reader — and an agent — should see what
    /// the face *is* before reading what it does. Items are order-independent in Rust, so the marker type
    /// does not have to precede the block that names it; measured on the example host, moving the
    /// declaration above `pub struct Slider;` compiles and leaves every identity byte identical.
    /// 渲染出来的面文件**第一件**说的就是声明（在模块文档与导入之后），它点名的标记类型紧随其后：读者——以及
    /// 代理——应当先看到这个面**是什么**，再读它做什么。Rust 的条目与顺序无关，因此标记类型不必写在点名它的
    /// 那块之前；在示例宿主上实测，把声明移到 `pub struct Slider;` 之前照样编译，且每个身份逐字节不变。
    #[test]
    fn the_declaration_is_the_first_thing_the_file_says() {
        let root = unique_root("declaration-first");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            "// generated-by=XiRang\npub struct Widget;\n\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
        )
        .expect("fixture face");

        let manifest = parse::source(&face_path, "", None).expect("the fixture parses");
        let rendered = manifest.render_source().expect("the face renders");
        let declaration = rendered
            .find("#[xirang_toolchain::soil]")
            .expect("the declaration renders");
        let marker = rendered
            .find("pub struct Widget;")
            .expect("the marker type renders");
        assert!(
            declaration < marker,
            "the declaration comes first, with the marker type below it:\n{rendered}"
        );
        assert!(
            !rendered[..declaration].contains("impl "),
            "no implementation may precede the declaration:\n{rendered}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The three keys that were accepted while nothing rendered them are now
    /// refused by name, so an edit fails loudly instead of being dropped.
    /// 三个"被接受却无人渲染"的键现在按名被拒，因此编辑会响亮地失败，而不是被丢掉。
    ///
    /// Red before the fix: each of these returned `Ok` from `edit` and changed
    /// nothing that a later `render_source` would write.
    /// 修前为红：这三个键的 `edit` 都返回 `Ok`，且不会改变任何 `render_source` 会写出的东西。
    #[test]
    fn the_keys_no_template_emits_are_refused_not_stored() {
        let root = unique_root("refused");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            "// generated-by=XiRang\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
        )
        .expect("fixture face");
        let mut manifest = parse::source(&face_path, "", None).expect("the fixture parses");
        for field in ["registry_name", "handle", "params"] {
            let error = manifest
                .edit(field, "anything")
                .expect_err("a key no template emits is not editable");
            assert!(
                error.contains("is not editable"),
                "{field} must be refused as not editable: {error}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A `requires` entry the declaration cannot reproduce refuses the rewrite
    /// instead of being dropped by it.
    /// 声明复现不了的 `requires` 条目会让这次重写失败，而不是被它丢掉。
    ///
    /// `edit` validates this field, so the value is set the way a hand-written or
    /// older declaration reaches the renderer: `parse` stores the list as text and
    /// only the caller above decides what to make of it. The published
    /// `render_requirements` is lossy (it drops the entry), which is exactly what a
    /// rewrite must not do: the result goes back into the author's file.
    /// `edit` 会校验这个字段，因此这里按"手写或旧版本声明到达渲染器"的方式设值：`parse` 把列表按文本
    /// 存下，只由上面那个调用方决定怎么处理它。已发布的 `render_requirements` 是有损的（它会丢掉该
    /// 条目），而这正是重写绝不能做的事：结果要写回作者的文件。
    #[test]
    fn a_malformed_requires_entry_refuses_the_rewrite() {
        let root = unique_root("malformed-requires");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            "// generated-by=XiRang\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
        )
        .expect("fixture face");
        let mut manifest = parse::source(&face_path, "", None).expect("the fixture parses");
        manifest
            .values
            .insert("requires".to_owned(), "canvas=>Canvas, broken".to_owned());

        let error = manifest
            .render_source()
            .expect_err("a malformed `requires` entry must refuse the rewrite");
        assert!(
            error.contains("capability=>provider"),
            "the refusal names the shape it wanted: {error}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A directory no other test shares.
    /// 一个没有别的测试共用的目录。
    fn unique_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-lg38-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        root
    }
}

#[cfg(test)]
mod plugin_preservation_tests {
    use crate::run_method::authoring::manifest::parse;

    /// Rebuilding a declaration must never delete a field it cannot reproduce:
    /// a face carrying `plugin:` preserves the opaque expression on rewrite.
    /// 重建声明绝不能删掉自己无法复现的字段：带 `plugin:` 的面保留不透明表达式，避免把它
    /// 弄丢。
    #[test]
    fn a_face_with_a_plugin_preserves_its_expression() {
        let root = std::env::temp_dir().join("xirang-plugin-face-fixture");
        let face = root.join("widget/widget.rs");
        std::fs::create_dir_all(face.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face,
            "// generated-by=XiRang\n\
             crate::root_object! {\n\
                 kind: Widget,\n\
                 parent: crate::ROOT_NODE_ID,\n\
                 plugin: crate::PluginSpec::new(\"widget\"),\n\
             }\n",
        )
        .expect("fixture face");

        let manifest = parse::source(&face, "", None).expect("face manifest");
        let output = manifest
            .render_source()
            .expect("plugin expression preserved");
        let parsed = xirang_kernel::syntax::parse_face(&output).unwrap().unwrap();
        assert_eq!(
            parsed.field("plugin").as_deref(),
            Some("crate::PluginSpec::new(\"widget\")")
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// The two key lists and their partition: on-disk fields against everything else.
/// 两份键清单及其划分：落盘字段与其余键。
#[cfg(test)]
mod key_partition_tests {
    use super::super::{OFF_DISK_KEYS, ON_DISK_FIELDS};

    /// Neither list carries a key twice. A duplicate is invisible to `contains`, which is
    /// exactly why it survived until the audit: the partition read as if it had one entry where
    /// it had two, and a reader comparing lengths would have been misled. `OFF_DISK_KEYS` held
    /// ten such pairs (27 entries, 17 distinct) before this pin existed (audit `LGC-LG-38`).
    /// 两份清单都不重复携带同一个键。重复项对 `contains` 不可见，这正是它能活到审计的原因：那份
    /// 划分读起来像只有一个条目、而实际有两个，比较长度的读者也会被误导。这条钉子出现之前，
    /// `OFF_DISK_KEYS` 里有十对这种成对的重复（27 个元素、17 个不同）。
    #[test]
    fn neither_key_list_repeats_a_key() {
        for (label, keys) in [("on-disk", ON_DISK_FIELDS), ("off-disk", OFF_DISK_KEYS)] {
            let mut sorted = keys.to_vec();
            sorted.sort_unstable();
            let mut deduped = sorted.clone();
            deduped.dedup();
            assert_eq!(
                sorted,
                deduped,
                "the {label} list must not repeat a key: {} entries, {} distinct",
                keys.len(),
                deduped.len()
            );
        }
    }
}

#[cfg(test)]
mod literal_spelling_tests {
    use crate::run_method::authoring::manifest::parse;
    use std::path::Path;

    /// The macro spelling of a face, parsed once and then re-read as if the file had used the literal
    /// spelling — the renderer's input is the same set of values either way, and what this test is about
    /// is what the **writer** emits for that input.
    /// 一个面的宏形文本，解析一次，然后当作"文件用的是字面量形"来读——两种拼法给渲染器的输入是同一批取值，
    /// 而这条测试要钉的是**写者**为这批输入发射什么。
    const MACRO_FORM: &str = r#"// generated-by=XiRang
crate::root_object! {
    kind: Widget,
    name: { zh: "组件", en: "Widget" },
    summary: { zh: "说明", en: "Summary" },
    needs_registry: true,
    parent: crate::ROOT_NODE_ID,
}
"#;

    fn literal_manifest() -> crate::run_method::authoring::manifest::FaceManifest {
        let mut manifest =
            parse::parse_face_macro_impl(Path::new("widget/widget.rs"), MACRO_FORM, "", None)
                .expect("the fixture parses");
        manifest
            .values
            .insert("written_as_literal".to_owned(), "true".to_owned());
        manifest
    }

    /// A face the author wrote as a `Registration` literal comes back as one: the rewriter writes the
    /// spelling it found, so an edit never silently converts a file to the macro form (which would also
    /// move the declaration, and with it everything a reader keys on the declaration's shape). The
    /// file-level leg is the probe host (`target/xirang-h2/host/`): a real package whose only face is
    /// written this way, built by the real build.
    /// 作者写成 `Registration` 字面量的面，回来时还是字面量：重写器写回它**读到**的那种拼法，因此一次编辑
    /// 绝不会把一个文件悄悄变成宏形（那同时会挪动声明位置，而读者依赖声明的形状）。文件级那一腿是探针宿主
    /// （`target/xirang-h2/host/`）：一个真包，里面唯一的面就是这么写的，由真构建编译。
    #[test]
    fn a_literal_form_face_renders_back_into_its_own_spelling() {
        let rendered = literal_manifest()
            .render_source()
            .expect("a literal face renders");

        assert!(
            rendered.contains("pub const REGISTRATION: Registration = Registration {"),
            "the spelling is kept: {rendered}"
        );
        assert!(
            rendered.contains("..Registration::DEFAULT"),
            "and unwritten fields keep spreading the default: {rendered}"
        );
        assert!(
            !rendered.contains("_object!"),
            "no macro block is written into a literal-form file: {rendered}"
        );
        assert!(
            rendered.contains("needs_registry: true,") && rendered.contains("组件"),
            "the fields the author wrote survive the rewrite: {rendered}"
        );
        // The output has to be readable by the reader that reads a face file, or the next edit would fail
        // on what this one wrote.
        // 输出必须能被读面文件的那个读者读回，否则下一次编辑会败在这一次写下的东西上。
        let parsed = xirang_kernel::syntax::parse_face(&rendered)
            .expect("the rendered declaration parses")
            .expect("and it is still a face");
        assert!(
            parsed.is_struct_literal(),
            "the round trip stays in the literal spelling"
        );
    }

    /// A literal-form face that declares contracts is written **with** them: `handle_traits` /
    /// `part_traits` become the associated constants of `impl FaceContract`, and a named contract is
    /// asserted by the author-facing `assert_contracts!` line. This used to be refused by name (the spelling
    /// had no home for it); the refusal is gone, so this pin now asserts the shape that replaced it — and it
    /// keeps asserting it, so a rewrite can never quietly drop what the author wrote.
    /// 声明了合同的字面量形面会**带着**它们落盘：`handle_traits` / `part_traits` 成为 `impl FaceContract`
    /// 的关联常量，而点名了合同就由那句给作者看的 `assert_contracts!` 断言。这里过去按名拒绝（那时拼法没有它
    /// 们的家）；拒绝已撤，因此这条钉子改为断言取代它的那个形状——并继续断言，使得重写永远不会悄悄丢掉作者写的东西。
    #[test]
    fn a_literal_form_face_with_contracts_writes_the_type_class_half() {
        let mut manifest = literal_manifest();
        manifest
            .values
            .insert("handle_traits".to_owned(), "ControlHandle".to_owned());
        manifest
            .values
            .insert("handle_contracts".to_owned(), "ControlHandle".to_owned());
        let rendered = manifest
            .render_source()
            .expect("a face with contracts is written in this spelling now");
        assert!(
            rendered.contains("impl FaceContract for"),
            "the type-class half has a home: {rendered}"
        );
        assert!(
            rendered
                .contains("const HANDLE_TRAITS: &'static [&'static str] = &[\"ControlHandle\"];"),
            "the label travels into the associated constant: {rendered}"
        );
        assert!(
            rendered.contains("assert_contracts!") && rendered.contains("ControlHandle"),
            "a named contract is asserted for the author: {rendered}"
        );
        assert!(
            rendered.contains("impl ") && rendered.contains("pub const REGISTRATION: Registration"),
            "the declaration is still written beside it: {rendered}"
        );
    }

    /// A face the caller gave an identity to is written **with** it, as a baked constant; without one,
    /// nothing is written. The identity cannot be computed in a const context (no hasher, no view of the
    /// face's own relative path), so the tool that writes the face writes the id it already has — and the
    /// build asserts that constant against the identity it computes, which is what catches a moved file.
    /// 调用方给了身份的面**带着**它落盘（一个烘焙常量）；没给就不写。身份在常量上下文里算不出来（没有哈希器、
    /// 也看不到面自己的相对路径），因此写下这个面的工具写下它已经握着的那个 id——而构建会拿这个常量与自己算出的
    /// 身份核对，正是这一步抓住了"文件被移动"。
    #[test]
    fn a_literal_form_face_is_written_with_the_identity_its_caller_supplied() {
        let mut manifest = literal_manifest();
        let without = manifest.render_source().expect("renders");
        assert!(
            !without.contains("NODE_ID"),
            "no identity, no constant: {without}"
        );

        manifest.set_identity(xirang_kernel::identity::NodeId::from_raw([
            161, 37, 69, 96, 191, 118, 200, 186, 41, 82, 135, 53, 71, 55, 170, 39,
        ]));
        let with = manifest.render_source().expect("renders");
        assert!(
            with.contains("pub const NODE_ID: NodeId = NodeId::from_raw([161, 37, 69, 96")
                && with.contains("41, 82, 135, 53, 71, 55, 170, 39]);"),
            "the identity is baked verbatim: {with}"
        );
        assert!(
            with.contains("pub const REGISTRATION: Registration = Registration {"),
            "and the declaration still follows it: {with}"
        );
    }
    /// With only `handle_traits` declared, the rendering must still carry an `assert_contracts!` line naming
    /// that label: the reader refuses a literal face whose labels have no matching assertion ("two statements
    /// of one fact"), so the writer has to state both.
    /// 只声明 `handle_traits` 时，渲染结果仍必须带一条点名该标签的 `assert_contracts!`：读者会拒绝"标签没有
    /// 对应断言"的字面量面（"同一件事的两句话"），因此写者必须把两句都写出来。
    #[test]
    fn the_assertion_names_the_labels_when_only_labels_are_declared() {
        let mut manifest = literal_manifest();
        manifest
            .values
            .insert("handle_traits".to_owned(), "ControlHandle".to_owned());
        let rendered = manifest.render_source().expect("renders");
        assert!(
            rendered.contains("assert_contracts!") && rendered.contains("ControlHandle"),
            "the assertion names the declared label: {rendered}"
        );
    }

    /// A requirement written in the literal spelling must be a `RequirementSpec`, not the macro's `a => b`:
    /// the reader normalizes the struct back and cannot parse the arrow inside a literal face.
    /// 字面量拼法里的需求必须写成 `RequirementSpec`，而不是宏的 `a => b`：读者会把该结构体归一化回去，而箭头
    /// 出现在字面量面里它解不开。
    #[test]
    fn the_literal_spelling_writes_a_requirement_spec() {
        let mut manifest = literal_manifest();
        manifest
            .values
            .insert("requires".to_owned(), "cap.render=>Provider".to_owned());
        let rendered = manifest.render_source().expect("renders");
        assert!(
            rendered
                .contains("RequirementSpec { capability: \"cap.render\", provider: \"Provider\" }"),
            "the requirement is a struct in this spelling: {rendered}"
        );
        assert!(
            !rendered.contains("=>"),
            "no arrow survives into the literal spelling: {rendered}"
        );
    }
}
