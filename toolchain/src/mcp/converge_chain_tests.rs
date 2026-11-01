//! Responsibility joins must follow identity, ancestry and evidence provenance.
//! 责任链拼接必须遵循身份、祖先关系与证据来源。

use super::*;

#[test]
fn closeout_capability_labels_keep_punctuation_in_both_joined_entrances() {
    let (root, _) = package("capability-labels");
    let label = "cap.render-v1/path";
    apply(
        &root,
        &json!({"action":"add", "apply":true,
        "fields":{"module":"provider","kind":"Provider","needs_registry":true,"provides":label}}),
    )
    .unwrap();
    for module in ["consumer", "sibling"] {
        apply(
            &root,
            &json!({"action":"add", "apply":true,"parent":"root/provider",
            "fields":{"module":module,"kind":"Consumer","requires":format!("{label}=>Provider")}}),
        )
        .unwrap();
    }
    let focused = converge(&root, &json!({"node":"root/provider/consumer"})).unwrap();
    let explored =
        crate::mcp::explore::explore(&root, &json!({"query":"root/provider/consumer"})).unwrap();
    for answer in [focused, explored] {
        assert!(answer.contains(&format!("{label} => Provider")), "{answer}");
        assert!(!answer.contains("UNANSWERED"), "{answer}");
        assert!(answer.contains("verify root/provider/sibling"), "{answer}");
    }
    // Whitespace and non-ASCII labels must also survive the native requirement parser.
    // 含空白与非 ASCII 字符的标签也必须按原生需求解析器的结果保留。
    let native_label = "渲染 cap-version/path";
    for file in [
        "src/provider/provider.rs",
        "src/provider/object/consumer/consumer.rs",
        "src/provider/object/sibling/sibling.rs",
    ] {
        let path = root.join(file);
        let source = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, source.replace(label, native_label)).unwrap();
    }
    let answer = converge(&root, &json!({"node":"root/provider/consumer"})).unwrap();
    assert!(
        answer.contains(&format!(
            "{native_label} => Provider  answered by root/provider"
        )),
        "{answer}"
    );
    assert!(!answer.contains("UNANSWERED"), "{answer}");
    assert!(answer.contains("verify root/provider/sibling"), "{answer}");
    cleanup(&root);
}

fn chain_run(root: &Path, namespace: &str) {
    provider_and_consumer(root);
    apply(
        root,
        &json!({"action":"add", "parent":"root/label", "apply":true,
        "fields":{"module":"knob", "kind":"Knob", "requires":"cap.render=>Label"}}),
    )
    .unwrap();
    let faces = crate::build_method::face_views(root, namespace).unwrap();
    let id = |path| faces.iter().find(|face| face.path == path).unwrap().id;
    let mut trace = CallTrace::full();
    trace.with_at(
        id("root/label/slider"),
        "Slider::render",
        at("src/main.rs", 9, "main"),
        |trace| {
            let request = trace.local("request", "u8", 7, LocalKind::Binding);
            trace.with_at(
                id("root/label"),
                "Label::render",
                at("src/label/object/slider/slider.rs", 20, "Slider::render"),
                |trace| {
                    trace.transform(request, "answer", "u8", 8);
                },
            );
        },
    );
    write_trace_artifact(&trace, &trace_artifact_path(root), namespace).unwrap();
}

#[test]
fn focused_run_connects_provider_calls_values_and_unobserved_consumers() {
    let (root, name) = package("chain");
    chain_run(&root, &name);
    let answer = converge(&root, &json!({"node":"root/label/slider", "trace":true})).unwrap();
    for expected in [
        "focus root/label/slider",
        "root/label/slider --[cap.render => Label]--> root/label",
        "root/label/knob --[cap.render => Label]--> root/label",
        "root/label/slider::Slider::render -> root/label::Label::render",
        "callsite src/label/object/slider/slider.rs:20",
        "root/label::answer",
        "root/label/slider::request -> root/label::answer",
        "verify root/label/knob",
        "not observed; absence is not proof of bypass",
        "source/config/input freshness unknown",
    ] {
        assert!(answer.contains(expected), "missing {expected}: {answer}");
    }
    assert!(
        !answer.contains("root/label/slider::answer"),
        "callsite must not own the callee's values: {answer}"
    );
    cleanup(&root);
}

#[test]
fn providers_in_other_branches_do_not_answer_or_gain_consumers() {
    let (root, _) = package("chain-branches");
    provider_and_consumer(&root);
    apply(
        &root,
        &json!({"action":"add", "apply":true,
        "fields":{"module":"aaa", "kind":"Label", "needs_registry":true,"provides":"cap.render"}}),
    )
    .unwrap();
    let answer = converge(&root, &json!({"node":"root/label/slider"})).unwrap();
    assert!(
        answer.contains("cap.render => Label  answered by root/label"),
        "{answer}"
    );
    assert!(!answer.contains("answered by root/aaa"), "{answer}");
    let answer = converge(&root, &json!({"node":"root/aaa"})).unwrap();
    assert!(answer.contains("declared consumers 0"), "{answer}");
    assert!(!answer.contains("verify root/label/slider"), "{answer}");
    cleanup(&root);
}

#[test]
fn a_missing_or_foreign_run_preserves_the_focused_declarations() {
    let (root, name) = package("chain-missing");
    chain_run(&root, &name);
    std::fs::remove_file(trace_artifact_path(&root)).unwrap();
    let missing = converge(&root, &json!({"node":"root/label", "trace":true})).unwrap();
    assert!(
        missing.contains("trace absent") && missing.contains("verify root/label/slider"),
        "{missing}"
    );
    let mut trace = CallTrace::full();
    trace.with(xirang_kernel::root_node_id("foreign"), "foreign", |_| {});
    write_trace_artifact(&trace, &trace_artifact_path(&root), "foreign").unwrap();
    let foreign = converge(&root, &json!({"node":"root/label", "trace":true})).unwrap();
    assert!(
        foreign.contains("REFUSED") && foreign.contains("verify root/label/slider"),
        "{foreign}"
    );
    assert!(!foreign.contains("observed call paths"), "{foreign}");
    assert!(converge(&root, &json!({"node":"root/no-such-face", "trace":true})).is_err());
    cleanup(&root);
}

#[test]
fn graft_selection_is_joined_to_the_provider_without_claiming_runtime_replacement() {
    let (root, name) = package("chain-graft");
    chain_run(&root, &name);
    assert!(crate::mcp::index::wait_until_idle(
        &root,
        std::time::Duration::from_secs(30)
    ));
    let out = root.join("target/xirang/out");
    crate::build_method::check_for(&root, &out, &name).unwrap();
    std::fs::write(out.join("source_scope.tsv"), "# mode\tauto\n# selected\t1\n# node\tsource\tmodule\n00000000000000000000000000000000\telsewhere.rs\tother\n").unwrap();
    let entry = root.join("src/lib.rs");
    let mut source = std::fs::read_to_string(&entry).unwrap();
    source.push_str(
        "\ncrate::static_graft_plan!(FRAMEWORK, cut \"root/label\" graft \"label_fast\");\n",
    );
    std::fs::write(entry, source).unwrap();
    let provider = crate::build_method::face_views(&root, &name)
        .unwrap()
        .into_iter()
        .find(|face| face.path == "root/label")
        .unwrap();
    for selector in ["label_fast", "other_fast"] {
        let plan = xirang_kernel::plugin::graft_document::GraftPlanDocument::new(
            provider.id,
            "root/label",
            selector,
            false,
        );
        let path = root.join(".xirang/external-grafts").join(selector);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("graft.plan"), plan.render_graft_plan_document()).unwrap();
    }
    // The discovery fingerprint covers face bodies, not the entry's contents.
    // 发现指纹覆盖面的正文，不覆盖入口正文。
    let face_source = root.join("src/label/label.rs");
    let source = std::fs::read_to_string(&face_source).unwrap();
    std::fs::write(
        &face_source,
        format!("{source}\n// provider edited after build\n"),
    )
    .unwrap();
    let stale = converge(&root, &json!({"node":"root/label/slider", "trace":true})).unwrap();
    assert!(
        stale.contains("build stale") && stale.contains("scope/pruning are from that build"),
        "{stale}"
    );
    let response = crate::mcp::tools::tool_call(
        &root,
        json!(1),
        &json!({
            "name":"xirang_tools", "arguments":{"tool":"converge", "arguments":{"node":"root/label/slider", "trace":true}}
        }),
    );
    let answer = response.to_string();
    for expected in [
        "declared graft=label_fast",
        "not runtime confirmation",
        "build stale",
        "scope/pruning are from that build",
        "static projection",
        "graft plan other_fast: declared=Some(true) selector_matches=Some(false)",
        "graft plan label_fast: declared=Some(true) selector_matches=Some(true)",
        "verify root/label/knob",
    ] {
        assert!(answer.contains(expected), "missing {expected}: {answer}");
    }
    cleanup(&root);
}

#[test]
fn focusing_a_node_excludes_an_unrelated_recorded_branch_and_bounds_expansion() {
    let (root, name) = package("chain-focus");
    // Add the unrelated face before recording: a source change after the trace makes it stale, and
    // a stale trace must not be presented as current observation.
    // 先加无关面再记录：trace 之后改源码会让它过期，而过期 trace 不得当成当前观察。
    apply(
        &root,
        &json!({"action":"add", "apply":true, "fields":{"module":"unrelated", "kind":"Unrelated"}}),
    )
    .unwrap();
    chain_run(&root, &name);
    let answer = converge(&root, &json!({"node":"root/unrelated", "trace":true})).unwrap();
    assert!(answer.contains("focus root/unrelated"), "{answer}");
    assert!(answer.contains("observed call paths (0)"), "{answer}");
    assert!(!answer.contains("Label::render"), "{answer}");
    let answer = converge(
        &root,
        &json!({"node":"root/label/slider", "trace":true, "limit":1}),
    )
    .unwrap();
    assert!(answer.contains("1 queued objects withheld"), "{answer}");
    assert!(!answer.contains("verify root/label/knob"), "{answer}");
    assert!(
        answer.contains("deeper closure beyond the limit is not computed"),
        "{answer}"
    );
    cleanup(&root);
}

#[test]
fn malformed_trace_does_not_hide_current_responsibilities() {
    let (root, _) = package("chain-malformed");
    provider_and_consumer(&root);
    let path = trace_artifact_path(&root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "this is not a trace").unwrap();
    let answer = converge(&root, &json!({"node":"root/label"})).unwrap();
    assert!(
        answer.contains("trace unreadable") && answer.contains("verify root/label/slider"),
        "{answer}"
    );
    assert!(answer.contains("runtime unknown"), "{answer}");
    cleanup(&root);
}

#[test]
fn a_run_without_callsites_still_reaches_an_unobserved_declared_provider() {
    let (root, name) = package("chain-unobserved");
    provider_and_consumer(&root);
    let slider = crate::build_method::face_views(&root, &name)
        .unwrap()
        .into_iter()
        .find(|face| face.path == "root/label/slider")
        .unwrap();
    let mut trace = CallTrace::full();
    trace.with(slider.id, "Slider::render", |trace| {
        trace.local("symptom", "u8", 0, LocalKind::Binding);
    });
    write_trace_artifact(&trace, &trace_artifact_path(&root), &name).unwrap();
    let answer = converge(&root, &json!({"trace":true})).unwrap();
    assert!(answer.contains("root/label/slider::symptom"), "{answer}");
    assert!(
        answer.contains(
            "object root/label kind=Label [not observed; absence is not proof of bypass]"
        ),
        "{answer}"
    );
    assert!(answer.contains("verify root/label:"), "{answer}");
    cleanup(&root);
}
