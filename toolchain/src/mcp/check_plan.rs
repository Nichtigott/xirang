//! Executable regression obligations tied to current source/configuration identities.
//! 绑定当前源码与配置身份、可执行的回归验证义务。
use serde_json::{Value, json};
use std::path::Path;

use super::check::CheckOutcome;
use super::check_execution::{Environment, VERIFICATION_POLICY, execution_identity};
use xirang_kernel::maintenance::{Completion, VerificationOutcome};

/// The intended failure a negative control must actually produce.
/// 负控制必须实际产生的那一个预期失败。
///
/// A negative control certifies a check only when **the named test** ran and **its own assertion**
/// carried this violation: a compile error, a timeout, zero executed tests, a failure in another
/// test, an ambiguous same-name test in another target, or the phrase appearing elsewhere in the log
/// are all `unknown`, never a pass (A09).
/// 只有当**点名的测试**执行、且**它自己的断言**携带这条违约时，负控制才给检查背书：编译错误、超时、
/// 零测试、别的测试失败、另一个 target 里同名的测试、或那句话出现在日志别处，一律 `unknown`，绝不是
/// 通过（A09）。
pub(super) struct ExpectedFailure {
    /// The test's **exact full name**, as cargo prints it.
    /// 测试的**完整原名**，按 cargo 打印的样子。
    pub(super) test: String,
    /// The Cargo target (source path) to bind the name to, when the name is not unique.
    /// 名字不唯一时，要绑定到的 Cargo target（源路径）。
    pub(super) target: Option<String>,
    /// A phrase from that test's own assertion output.
    /// 那个测试自己断言输出里的一句话。
    pub(super) violation: String,
}

/// One planned verification obligation.
/// 一条被计划的验证义务。
pub(super) struct Obligation {
    /// The case's name, as the ledger and reply print it.
    /// 用例名，按台账与回复打印的样子。
    pub(super) name: String,
    /// `passed` or `failed`.
    /// `passed` 或 `failed`。
    pub(super) expected: String,
    /// The intended failure, for a negative control.
    /// 负控制的预期失败。
    pub(super) expect: Option<ExpectedFailure>,
    /// The check request to execute.
    /// 要执行的检查请求。
    pub(super) request: Value,
}

/// Judge one typed check outcome against a case's expectation, without re-reading rendered text.
/// 用一次类型化检查结果对照用例的预期，而不重读渲染出来的文本。
pub(super) fn judge(
    expected: &str,
    expect: Option<&ExpectedFailure>,
    outcome: &CheckOutcome,
) -> &'static str {
    match expected {
        "passed" => match outcome.verdict {
            VerificationOutcome::Passed => "passed",
            VerificationOutcome::Failed => "failed",
            VerificationOutcome::Unknown(_) => "unknown",
        },
        "failed" => match outcome.verdict {
            // The counterproof did not reproduce: the target now passes.
            // 反证没有复现：目标现在通过了。
            VerificationOutcome::Passed => "failed",
            // A compile error, zero tests, an ignore or a timeout is not a detected failure.
            // 编译错误、零测试、被忽略或超时都不是"检出了故障"。
            VerificationOutcome::Unknown(_) => "unknown",
            VerificationOutcome::Failed => {
                // No declared target means the failure is unqualified: unknown, never a pass.
                // 没有声明目标就是未限定失败：unknown，绝不是通过。
                let Some(selector) = expect else {
                    return "unknown";
                };
                let target = selector.target.as_deref();
                // An unsupported evidence shape is unknown before any name is compared: a
                // test row with no `Running` header cannot be attributed, so a negative
                // control built on it is not evidence (A09).
                // 不受支持的证据形状在任何名字比较之前就是 unknown：没有 `Running` 头的测试行无从归属，
                // 因此建在它上面的负控制不是证据（A09）。
                if outcome.target_evidence_incomplete()
                    || outcome.name_is_ambiguous(&selector.test, target)
                {
                    return "unknown";
                }
                if outcome.failed_on(&selector.test, target, &selector.violation) {
                    "passed"
                } else if outcome.named_test_failed(&selector.test, target) {
                    // The named test failed, but not on the declared violation: the intended failure
                    // was not identified, so the evidence is unsupported.
                    // 点名的测试失败了，但不是声明的违约：没有识别出预期失败，因此证据不受支持。
                    "unknown"
                } else {
                    // Some other test failed; the target check did not.
                    // 别的测试失败了；目标检查没有。
                    "failed"
                }
            }
        },
        _ => "unknown",
    }
}

/// The one entry under the `files`/`plan_id` branch: create a plan, or act on an existing one.
/// `files`/`plan_id` 分支下的唯一入口：创建一份计划，或对一份既有计划采取动作。
pub(super) fn check_or_plan(root: &Path, arguments: &Value) -> Result<String, String> {
    if arguments.get("plan_id").is_some() {
        return super::check_resume::plan_action(root, arguments);
    }
    run(root, arguments)
}

pub(super) fn run(root: &Path, args: &Value) -> Result<String, String> {
    let files = args.get("files").ok_or("files is required")?;
    let impact = super::affected::affected(root, &json!({"files":files,"limit":200}))?;
    let mut base = args.clone();
    base.as_object_mut()
        .ok_or("arguments must be an object")?
        .remove("files");
    let cases = base
        .as_object_mut()
        .and_then(|object| object.remove("cases"));
    super::check_request::validate(&base)?;
    super::check_request::validate_action(&base)?;
    // `reuse` is validated while it is still present and stripped only afterwards, because it decides
    // whether this call runs rather than what it runs: leaving it in would write a scheduling choice
    // into the ledger and into the argv of every later replay of that row.
    // `reuse` 在仍然存在时被校验，之后才剥掉——它决定这次调用**跑不跑**，不决定跑什么：留在里面会把一个
    // 调度选择写进台账，也会写进那一行之后每次重放所用的 argv。
    base.as_object_mut()
        .expect("an object was checked above")
        .remove("reuse");
    if ["package", "test", "filter"]
        .iter()
        .any(|key| base.get(key).is_some())
        || base
            .get("mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| mode != "test")
    {
        return Err("repair obligations require workspace tests without package/test/filter narrowing; use a separate check for narrower probes".into());
    }
    if base.get("workspace").and_then(Value::as_bool) == Some(false) {
        return Err("repair obligations require workspace:true".into());
    }
    base["workspace"] = json!(true);
    base["census"] = json!(false);
    // One slot per obligation, aligned with `requests`: a variant is a property of the obligation
    // that runs it, and keeping the two vectors the same length is what makes "which tree does row i
    // run in" answerable without a lookup.
    // 每条义务一个槽位，与 `requests` 对齐：变体是运行它的那条义务的性质，而让两个向量等长，正是"第 i 行
    // 在哪棵树里跑"不需要查表就能回答的原因。
    let mut variants: Vec<Option<super::check_variant::Variant>> = Vec::new();
    let mut requests = vec![Obligation {
        name: "regression".to_owned(),
        expected: "passed".to_owned(),
        expect: None,
        request: base.clone(),
    }];
    variants.push(None);
    if base.get("face").is_none() && base.get("no_default_features").is_none() {
        let mut all = base.clone();
        all["face"] = json!("all");
        requests.push(Obligation {
            name: "all-features".into(),
            expected: "passed".into(),
            expect: None,
            request: all,
        });
        variants.push(None);
        let mut minimal = base.clone();
        minimal["no_default_features"] = json!(true);
        requests.push(Obligation {
            name: "no-default-features".into(),
            expected: "passed".into(),
            expect: None,
            request: minimal,
        });
        variants.push(None);
    }
    if let Some(cases) = cases {
        let cases = cases
            .as_array()
            .filter(|rows| rows.len() <= 16)
            .ok_or("cases must be an array of at most 16 regression/counterproof configurations")?;
        for case in cases {
            if case.as_object().is_none_or(|object| {
                object.keys().any(|key| {
                    !["name", "expected", "request", "expect", "variant"].contains(&key.as_str())
                })
            }) {
                return Err("cases accept only name, expected, request, expect and variant".into());
            }
            let name = case["name"]
                .as_str()
                .filter(|name| !name.is_empty() && name.len() <= 120)
                .ok_or("each case needs a name")?;
            let expected = match case.get("expected") {
                None => "passed",
                Some(value) => value.as_str().ok_or("case expected must be a string")?,
            };
            if !matches!(expected, "passed" | "failed") {
                return Err("case expected must be passed or failed".into());
            }
            // The selector is per case and optional; an old request without it stays syntactically
            // accepted, but a `failed` case without it is judged `unknown` rather than certified.
            // 选择器逐用例、可选；不带它的旧请求仍然语法可接受，但不带它的 `failed` 用例如实判为
            // `unknown`，而不是被背书。
            let expect = match case.get("expect") {
                None => None,
                Some(value) => {
                    let selector = value.as_object().ok_or("case expect must be an object")?;
                    if selector
                        .keys()
                        .any(|key| !["test", "target", "violation"].contains(&key.as_str()))
                    {
                        return Err("case expect accepts only test, target and violation".into());
                    }
                    let test = selector
                        .get("test")
                        .and_then(Value::as_str)
                        .filter(|test| !test.is_empty() && test.len() <= 200)
                        .ok_or("case expect needs a non-empty `test`")?;
                    let target = match selector.get("target") {
                        None => None,
                        Some(value) => Some(
                            value
                                .as_str()
                                .filter(|target| !target.is_empty() && target.len() <= 300)
                                .ok_or("case expect `target` must be a non-empty string")?
                                .to_owned(),
                        ),
                    };
                    let violation = selector
                        .get("violation")
                        .and_then(Value::as_str)
                        .filter(|violation| !violation.is_empty() && violation.len() <= 500)
                        .ok_or("case expect needs a non-empty `violation`")?;
                    Some(ExpectedFailure {
                        test: test.to_owned(),
                        target,
                        violation: violation.to_owned(),
                    })
                }
            };
            let mut request = case["request"]
                .as_object()
                .cloned()
                .ok_or("each case needs a request object")?;
            if request.contains_key("root")
                || request.contains_key("files")
                || request.contains_key("cases")
            {
                return Err("cases cannot change root or recursively create plans".into());
            }
            if request.contains_key("target_dir") {
                return Err("cases share the plan's content-specific build directory; set target_dir on the outer plan instead".into());
            }
            // `reuse` is a property of the call, and a case is not a call. Accepting it here would
            // make it a parameter the reply never acts on — exactly the "documented but ineffective"
            // shape this validator exists to refuse.
            // `reuse` 是一次调用的性质，而用例不是调用。在这里接受它，会让它变成一个回复从不据以行动的
            // 参数——正是这个校验器存在就是为了拒绝的"写进文档却无效"的形状。
            if request.contains_key("reuse") {
                return Err(
                    "cases cannot ask for cross-plan reuse; set reuse on the outer plan".into(),
                );
            }
            request.entry("census").or_insert(json!(false));
            let request = Value::Object(request);
            super::check_request::validate(&request)?;
            if request
                .get("mode")
                .and_then(Value::as_str)
                .is_some_and(|mode| mode != "test")
            {
                return Err("counterproof cases must execute tests".into());
            }
            // The variant is parsed before the obligation is pushed, so a malformed one is refused
            // while nothing has been written and no process has been started.
            // 变体在义务被推入之前解析，因此畸形的变体会在什么都没写、也没有进程启动时就被拒绝。
            let declared = match case.get("variant") {
                None => None,
                Some(value) => Some(super::check_variant::Variant::parse(
                    root,
                    name,
                    value,
                    expect.as_ref().map(|expect| expect.test.as_str()),
                    expected,
                )?),
            };
            requests.push(Obligation {
                name: name.into(),
                expected: expected.into(),
                expect,
                request,
            });
            variants.push(declared);
        }
    }
    let excluded = args
        .get("target_dir")
        .and_then(Value::as_str)
        .map(|path| root.join(path));
    // BR06: a plan phase says what would be run, and starting Cargo is not part of saying that.
    // Everything below this line is the plan; `execute` further down is the only thing that runs a
    // test, and a plan returns before it.
    // BR06：计划阶段说的是**将要跑什么**，而启动 Cargo 不属于"说"这件事。这一行以下都是计划；真正跑测试的
    // 只有更下面的 `execute`，而计划会在它之前返回。
    let planning = super::check_plan_phase::planning(args);
    // Cargo creates its lockfile on the first run; resolve it before sealing obligations.
    // Cargo 首次运行会生成锁文件；先解析它，再封存验证义务身份。
    if !root.join("Cargo.lock").exists() && planning {
        return Ok(super::check_plan_phase::unresolved_lockfile(root));
    }
    if !root.join("Cargo.lock").exists() {
        let log = excluded
            .clone()
            .unwrap_or_else(|| root.join("target"))
            .join(format!("lock-{}.log", super::check_jobs::new_id()));
        let mut command = std::process::Command::new("cargo");
        command.arg("generate-lockfile").current_dir(root);
        let timeout = args
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(900_000)
            .clamp(1000, 3_600_000);
        let result = super::check::run_command(
            &mut command,
            std::time::Duration::from_millis(timeout),
            &log,
        )?;
        if result.code != Some(0) || result.timed_out {
            return Ok(format!(
                "obligations open; dependency resolution did not complete; log {}",
                log.display()
            ));
        }
    }
    if let Some(output) = &excluded {
        let absolute_root = root.canonicalize().map_err(|e| e.to_string())?;
        let absolute_output = output.canonicalize().unwrap_or_else(|_| output.clone());
        if absolute_root.starts_with(&absolute_output) {
            return Err("verification target_dir must not contain the source root".into());
        }
        let metadata = crate::cargo_metadata::read(&root.join("Cargo.toml"))?;
        for package in metadata["packages"].as_array().into_iter().flatten() {
            for target in package["targets"].as_array().into_iter().flatten() {
                if target["src_path"]
                    .as_str()
                    .is_some_and(|source| Path::new(source).starts_with(&absolute_output))
                {
                    return Err("verification target_dir contains Cargo source inputs".into());
                }
            }
        }
    }
    let identity = fingerprint(root, excluded.as_deref())?;
    // Content-specific build directories prevent restored mtimes from reusing another revision's tests.
    // 按内容分隔构建目录，避免恢复旧时间戳时复用另一版本的测试二进制。
    let build_directory = excluded
        .clone()
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/verification-build")
        .join(&identity);
    for obligation in &mut requests {
        obligation.request["target_dir"] = json!(build_directory);
    }
    // The execution identities are computed **after** the build directory is written into every
    // request, because that directory is one of the dimensions: computing them before would bind the
    // pre-substitution value and make two different output directories look like one execution.
    // 执行身份在构建目录被写进每个请求**之后**计算，因为那个目录本身是一个维度：提前算会绑上替换前的
    // 取值，让两个不同的输出目录看起来像同一次执行。
    let environment = Environment::observe();
    // A variant obligation runs in an immutable copy of the base tree, with a build directory beside
    // that copy, so both the `candidate` and the `isolation` dimension change with the declared fault
    // and no artifact of one variant can be read as evidence about another.
    // 变体义务在基树的一份不可变副本里运行，构建目录在那份副本**旁边**，因此 `candidate` 与 `isolation`
    // 两个维度都随声明的故障而变，而任何一个变体的产物都不能读成关于另一个变体的证据。
    let variants_directory = excluded
        .clone()
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/variants");
    let mut candidates = vec![identity.clone(); requests.len()];
    let mut targets = vec![root.to_path_buf(); requests.len()];
    let mut declared: Vec<Option<Value>> = vec![None; requests.len()];
    let mut broken_routes: Vec<(String, String, String)> = Vec::new();
    for (position, obligation) in requests.iter_mut().enumerate() {
        let Some(variant) = variants.get_mut(position).and_then(Option::take) else {
            continue;
        };
        let materialized = variant.materialize(root, excluded.as_deref(), &variants_directory)?;
        let slot = variants_directory.join(variant.digest());
        obligation.request["target_dir"] = json!(slot.join("build"));
        declared[position] = Some(json!({
            "digest": variant.digest(),
            "kind": variant.kind_label(),
            "edits": materialized.report,
            "route_holds": materialized.route_holds,
        }));
        // A broken route is a **finding about the variant**, and it is reported through the same duty
        // ledger the executable checks use — because the behaviour run may well be green, and a green
        // run is exactly what "the consumer copied the formula" produces.
        // 断掉的路线是一条**关于变体的发现**，它经可执行检查所用的同一本义务台账报出——因为那次行为运行很
        // 可能全绿，而全绿正是"消费者把公式抄了过去"会产生的样子。
        if materialized.route_holds == Some(false)
            && let Some(route) = variant.route()
        {
            broken_routes.push((
                obligation.name.clone(),
                route.caller.clone(),
                route.callee.clone(),
            ));
        }
        candidates[position] = materialized.identity;
        targets[position] = materialized.root;
    }
    let executions = requests
        .iter()
        .enumerate()
        .map(|(position, obligation)| {
            execution_identity(
                &obligation.request,
                &candidates[position],
                &environment,
                &targets[position],
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let directory = excluded
        .clone()
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/verification");
    // `reuse` is opt-in, and that is deliberate: a `files` call executes by default, so nothing that
    // used to run stops running because of this feature. What a caller opts into is not "skip work"
    // but "accept a plan that recorded the same closure and the same execution identities" — and the
    // reply names the plan, its age and which obligation covered each execution, so the reader can
    // tell a reuse from a run without reading the ledger.
    // `reuse` 是选择加入的，这是有意的：`files` 调用默认执行，因此没有任何过去会跑的东西因为本特性而不再跑。
    // 调用方选择加入的不是"跳过工作"，而是"接受一份记录了同一个闭包与同一批执行身份的计划"——而回复点名那份
    // 计划、它的年龄，以及每条执行由哪条义务覆盖，于是读者不必翻台账就能分辨复用与真跑。
    if args.get("reuse").and_then(Value::as_bool) == Some(true)
        && let Some(reused) = super::check_resume::reusable_plan(
            root,
            &directory,
            &super::check_task::task_key(args),
            &identity,
            &executions,
            &environment,
        )?
    {
        let mut output = format!(
            "obligations passed (reused; nothing was run)\nplan {}\nrecord {}\nsource_configuration {identity}\nreused: written {} second(s) ago, same input closure and identical execution identities\n",
            reused.id,
            reused.record.display(),
            reused.age_seconds
        );
        for (execution, who) in executions.iter().zip(&reused.covered_by) {
            output.push_str(&format!(
                "  {} — execution {} run by `{who}` in that plan\n",
                execution.render_summary(),
                &execution.digest()[..16]
            ));
        }
        output.push_str(
            "nothing was run and nothing was written; omit `reuse` (or edit any input) to execute again\n",
        );
        return Ok(output);
    }
    // Asked for reuse and did not get it: say which dimension refused, computed **now** (before this call writes its own
    // plan, which would become the newest one and hide the reason).
    // 要求了复用却没拿到：现在就说清是哪个维度拒绝的（在这次调用写下自己的计划之前——那之后它就成了最新的一份，会把理由藏起来）。
    let reuse_note = if args.get("reuse").and_then(Value::as_bool) == Some(true) {
        super::check_witness::reuse_note(root, &directory, args)
    } else {
        String::new()
    };
    let id = super::check_jobs::new_id();
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let plan_files = super::check_task::files(&directory, &id);
    let path = plan_files.ledger.clone();
    // The manifest is the durable half of the identity: the digest alone can say "something
    // changed", while this file can say which paths did (A15/A16).
    // 清单是身份中持久的那一半：只有摘要时说得出"有东西变了"，而这份文件说得出是哪些路径（A15/A16）。
    let inputs = super::check_task::Inputs::capture(root, excluded.as_deref())?;
    inputs.write(&plan_files.inputs)?;
    let total_inputs = inputs.count();
    super::check_task::note(
        &plan_files.journal,
        super::check_task::phase(&id, "planned"),
    )?;
    let mut ledger = json!({"id":id,"task":super::check_task::task_key(args),"source_configuration":identity,
        "inputs_file":plan_files.inputs.file_name().map(|name| name.to_string_lossy().into_owned()),
        "inputs":total_inputs,
        "journal_file":plan_files.journal.file_name().map(|name| name.to_string_lossy().into_owned()),
        "impact":impact,
        "policy":VERIFICATION_POLICY,
        "scope":"current in-root Rust/configuration/graft inputs; external dependencies, arbitrary build-script inputs and runtime inputs are not certified; workspace tests do not cover doctests, other profiles or configurations this plan did not name",
        "environment":{"toolchain":environment.toolchain,"host":environment.host,"checker":environment.checker},
        "obligations": requests.iter().zip(&executions).enumerate().map(|(position, (obligation, execution))| {
            let mut row = json!({"name":obligation.name,"expected":obligation.expected,"request":obligation.request,"state":"pending","effect":"read_only",
                "candidate":candidates[position],"execution":execution.render_summary(),"execution_digest":execution.digest()});
            if let Some(expect) = &obligation.expect {
                row["expect"] = json!({"test":expect.test,"target":expect.target,"violation":expect.violation});
            }
            if let Some(variant) = &declared[position] {
                row["variant"] = variant.clone();
                row["variant_root"] = json!(targets[position].display().to_string());
            }
            row
        }).collect::<Vec<_>>()});
    persist(&path, &ledger)?;
    // BR08: a variant that was supposed to keep delegating and no longer does leaves the same kind of
    // duty as an uncovered consumer. It is recorded separately from the behaviour verdict, so "the
    // tests passed" and "the consumers still route through the provider" never collapse into one.
    // BR08：一个本该继续委派、却不再委派的变体，留下与"未被覆盖的消费者"同一类义务。它被记在行为判定**之外**，
    // 因此"测试通过了"与"消费者仍然经过提供者"永远不会塌缩成一句话。
    for (case, caller, callee) in &broken_routes {
        ledger["obligations"]
            .as_array_mut()
            .expect("obligations array")
            .push(json!({
                "name": format!("variant route: {case}"),
                "expected": "passed",
                "state": "unknown",
                "evidence": format!(
                    "the variant for `{case}` no longer calls `{callee}` from {caller}, so the consumers were not shown to follow the provider — a behaviourally green run is what a locally re-implemented formula produces, and it does not close this duty"
                ),
            }));
    }
    let outside_workspace = super::workspace::enclosing_workspace(root);
    let mut all_passed = outside_workspace.is_none() && broken_routes.is_empty();
    if let Some(workspace) = outside_workspace {
        ledger["outside_root_obligations"] = json!(format!(
            "workspace inputs outside this root are unverified; run the plan from {}",
            workspace.display()
        ));
    }
    // BR07: a shared implementation whose consumer entry has no test calling it by name leaves a
    // duty in the **same** ledger the executable checks write into. It is recorded, not assumed: the
    // row names the consumer, the provider and the fact that the search was lexical, and a plan whose
    // cargo runs were all green still reads `open` while the duty stands.
    // BR07：共享实现身上"没有任何测试按名字调用"的消费者入口，会在可执行检查所写入的**同一本**台账里留下
    // 一条义务。它是被记录的，不是被假定的：那一行点名消费者、提供者，以及这次查找是词法的；而一份 cargo
    // 全绿的计划，在义务仍然站着的时候依然读作 `open`。
    match super::check_protection::census(root, files) {
        Ok(Some(census)) => {
            all_passed &= census.duties.iter().all(|duty| duty["state"] == "passed");
            ledger["obligations"]
                .as_array_mut()
                .expect("obligations array")
                .extend(census.duties);
            ledger["protection"] = census.summary;
        }
        Ok(None) => {}
        Err(error) => {
            // A census that could not run is **not** a census that found nothing: the row says so and
            // keeps the duty open rather than letting a failed scan read as coverage.
            // 跑不起来的普查**不是**什么都没找到的普查：这一行说明这一点，并让义务保持开着，而不是让一次
            // 失败的扫描读成覆盖。
            all_passed = false;
            ledger["obligations"]
                .as_array_mut()
                .expect("obligations array")
                .push(json!({"name":"consumer protection census","expected":"passed","state":"unknown","evidence":error}));
        }
    }
    // Recorded whether or not the census found anything: "no shared consumers" is an observation, not a silence.
    // 无论普查有没有找到东西都记录："没有共享消费者"是一次观测，不是沉默。
    super::check_witness::record(&mut ledger, root, files);
    match super::check_registry::obligations(root, files) {
        Ok((rows, evidence)) => {
            all_passed &= rows.iter().all(|row| row["state"] == "passed");
            ledger["obligations"].as_array_mut().unwrap().extend(rows);
            ledger["responsibilities"] = json!(evidence);
        }
        Err(error) => {
            all_passed = false;
            ledger["obligations"].as_array_mut().unwrap().push(json!({"name":"registration inventory","expected":"passed","state":"unknown","evidence":error}));
        }
    }
    persist(&path, &ledger)?;
    if planning {
        return Ok(super::check_plan_phase::render_plan(
            &id,
            &path,
            &ledger,
            &executions,
            &requests,
        ));
    }
    all_passed &= super::check_resume::execute(
        &mut ledger,
        &path,
        &plan_files.journal,
        root,
        &identity,
        &id,
        excluded.as_deref(),
        &(0..requests.len()).collect::<Vec<_>>(),
        &requests,
        &executions,
        &targets,
    )?;
    // BR09: every obligation above ran either in the base tree or in a copy, and the counterproof is
    // only worth anything if the thing it was made from still has the bytes it started with. The
    // verdict is recorded rather than assumed, because a variant that edited the base by accident
    // would otherwise be indistinguishable from one that did not.
    // BR09：上面每条义务要么在基树里跑、要么在副本里跑，而反证只有在"它据以做出的那棵树仍是开始时那些字节"
    // 时才值钱。这个判定是被记录的而不是被假定的，否则一个不小心改了基树的变体与一个没改的将无从分辨。
    ledger["base_tree_unchanged"] = json!(fingerprint(root, excluded.as_deref())? == identity);
    // W1/A02: the four delivery claims stay distinct. A check plan answers the query and verifies
    // obligations; it never applies an operation and never accepts its own result. The check-plan
    // policy does **not** require acceptance (authorization persists; adoption keeps its own explicit
    // confirmation), and `applied` is false because a check mutates nothing — so completion is false
    // by the pure rule, whatever the verification says. Task/input binding for stale receipts is W4.
    // W1/A02：四种交付声明保持分开。检查计划作答并验证义务，它从不落盘操作、也从不接受自己的结果。检查
    // 计划政策**不**要求接受（授权持续有效；采信保留自己的显式确认），且检查不改动任何东西，故 `applied`
    // 为 false——因此无论验证说什么，按纯规则完成都是 false。过期凭据的任务/输入绑定属于 W4。
    let plan_completion = Completion {
        answered: true,
        applied: false,
        verified: all_passed.then_some(VerificationOutcome::Passed),
        accepted: false,
    };
    super::check_task::note(
        &plan_files.journal,
        super::check_task::phase(&id, if all_passed { "completed" } else { "open" }),
    )?;
    ledger["state"] = json!(if all_passed { "passed" } else { "open" });
    ledger["completion"] = json!({
        "query_answered": plan_completion.answered,
        "operation_applied": plan_completion.applied,
        "verification_closes": plan_completion.verification_closes(),
        "accepted": plan_completion.accepted,
        "acceptance_required": false,
        "complete": plan_completion.is_complete(false),
    });
    persist(&path, &ledger)?;
    // The matrix is reported as **executions and duties**, not as a list of states: one execution can
    // settle several obligations, every obligation keeps its own row and its own expectation, and what
    // is still open is named separately so "passed" is never read as "everything is covered".
    // 矩阵按**执行与义务**报告，而不是一串状态：一次执行可以结清多条义务，每条义务保留自己的行与自己的预期，
    // 而仍然开着的东西单独点名，因此"passed"永远不会被读成"所有事都覆盖了"。
    let mut output = format!(
        "obligations {}\nplan {id}\nrecord {}\nsource_configuration {identity}\nenvironment {}\n",
        if all_passed {
            "passed (declared matrix only)"
        } else {
            "open"
        },
        path.display(),
        environment.toolchain,
    );
    let rows = ledger["obligations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut runs = 0usize;
    let mut reused = 0usize;
    for row in &rows {
        let state = row["state"].as_str().unwrap_or("unknown");
        if row["execution"].is_string() {
            if row["satisfied_by"].is_string() {
                reused += 1;
            } else {
                runs += 1;
            }
        }
        output.push_str(&format!(
            "{}: {} (expected {})",
            row["name"].as_str().unwrap_or("?"),
            state,
            row["expected"].as_str().unwrap_or("passed")
        ));
        match row["execution_digest"].as_str() {
            Some(digest) if digest.len() >= 16 => {
                output.push_str(&format!(" [execution {}", &digest[..16]));
                match row["satisfied_by"].as_str() {
                    Some(first) => output.push_str(&format!("; settled by `{first}`]")),
                    None => output.push_str("; ran]"),
                }
            }
            _ => output.push_str(" [no execution identity recorded]"),
        }
        output.push('\n');
        if state != "passed"
            && let Some(evidence) = row["evidence"].as_str()
        {
            let mut end = evidence.len().min(2500);
            while !evidence.is_char_boundary(end) {
                end -= 1;
            }
            output.push_str(&evidence[..end]);
            output.push('\n');
        }
    }
    output.push_str(&format!(
        "executions {runs} run, {reused} settled by an identical execution identity\n"
    ));
    for row in &rows {
        if let Some(variant) = row["variant"].as_object() {
            output.push_str(&format!(
                "variant {} [{}, digest {}]: {}; it runs in an immutable copy at {}, so the base tree is not the thing the fault was injected into\n",
                row["name"].as_str().unwrap_or("?"),
                variant.get("kind").and_then(Value::as_str).unwrap_or("fault"),
                variant.get("digest").and_then(Value::as_str).unwrap_or("?"),
                variant["edits"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
                row["variant_root"].as_str().unwrap_or("?")
            ));
        }
    }
    output.push_str(&format!(
        "base tree {}; a variant is never a candidate and is never published\n",
        if ledger["base_tree_unchanged"].as_bool() == Some(true) {
            "unchanged across every obligation"
        } else {
            "CHANGED while the plan ran, so the counterproofs above are not evidence"
        }
    ));
    super::check_plan_tail::render(&mut output, &rows, &ledger, &reuse_note);
    Ok(output)
}
pub(super) fn persist(path: &Path, ledger: &Value) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let encoded = serde_json::to_vec_pretty(ledger).map_err(|e| e.to_string())?;
    // Name the file that could not be written: a caller told only "Is a directory" has to go
    // looking for which of the three files a plan owns it meant (W4/A19).
    // 点名写不下去的那个文件：只被告知 "Is a directory" 的调用方还得自己去找这是计划拥有的三个文件里的哪
    // 一个（W4/A19）。
    std::fs::write(&temporary, encoded)
        .map_err(|e| format!("cannot write {}: {e}", temporary.display()))?;
    std::fs::rename(&temporary, path).map_err(|e| format!("cannot publish {}: {e}", path.display()))
}

pub(super) fn fingerprint(root: &Path, excluded: Option<&Path>) -> Result<String, String> {
    // `verification_digest`, not `digest`: this is the identity a **verdict** is bound to, and a verdict can
    // depend on any environment variable. The tree-only `digest` stays what a candidate's base identity
    // uses — a tree does not move because a shell variable did.
    // 这里用 `verification_digest` 而不是 `digest`：这是**判定**所绑定到的身份，而一个判定可能依赖任意一个环境
    // 变量。只绑树的 `digest` 仍是候选基树身份所用的那个——一棵树不会因为一个 shell 变量而移动。
    Ok(super::check_task::Inputs::capture(root, excluded)?.verification_digest())
}

#[cfg(test)]
#[path = "check_fixtures.rs"]
pub(super) mod check_fixtures;

#[cfg(test)]
#[path = "check_plan_tests.rs"]
mod check_plan_tests;
