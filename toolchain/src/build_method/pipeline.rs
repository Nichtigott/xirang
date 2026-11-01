use super::diagnostics::append_error;
use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::partition_compat::under_a_claim;
use super::{
    BuildInput, SourceScope, aggregate_contract_errors, aggregate_parent_macro_errors,
    aggregate_registry_rule_errors, aggregate_requirements, aggregate_stable_name_errors,
    cache_directory, emit_rerun_paths, face_syntax_errors, graft_plan_check, prime_node_id_cache,
    render_lib_with_external, static_plan, unplaced_face_errors, update_discovery_cache,
    write_file_manifest, write_function_manifest, write_generation, write_graft_manifest,
    write_graph_manifest, write_if_changed, write_pruning_manifest, write_shape_manifest,
    write_source_scope_manifest,
};
use std::time::{Duration, Instant};

#[path = "pipeline_helpers.rs"]
mod helpers;
#[path = "pipeline_catalog.rs"]
mod pipeline_catalog;

use self::helpers::{build_output_is_verbose, cache_status_line, phase_line, retain_unsuperseded};

use xirang_kernel::lexicon;

pub(crate) fn run(
    input: &BuildInput,
    external: &[crate::build_method::external_face::ExternalFaceRecord],
    declared: &[crate::build_method::declared_native::DeclaredFace],
    native: &[(String, super::registry_identity::NodeId, bool)],
    catalog_declared: &[crate::build_method::declared_native::DeclaredFace],
    constants: Option<&super::record_constants::Request>,
) -> Option<BuildDiagnostics> {
    let manifest = &input.manifest;
    let (_metadata, _syntax) = (
        crate::cargo_metadata::scope(),
        crate::source_cache::scope(manifest),
    );
    let mut phases = PhaseLog::for_run();
    // Two bases, resolved once: `scan` is the tree the walk reads, and `src` is
    // what an identity path is taken relative to. They are the same directory for
    // every package that keeps its sources under `src/`, and differ for one whose
    // library target lives elsewhere (`source_layout` explains why, and why the
    // identity answer has to match the declaration macros).
    // 两个基准只解析一次：`scan` 是遍历读取的树，`src` 是身份路径所相对的基准。对每个把源码放在
    // `src/` 下的包，它们是同一个目录；而库目标住在别处的包会让它们分开（`source_layout` 解释了
    // 原因，以及为什么身份那一半必须与声明宏一致）。
    let layout = match &input.layout {
        Ok(layout) => layout,
        // A layout the resolver had to refuse — a `[lib] path` naming no file — is
        // reported exactly where a missing tree is, and with the same
        // consequences: `check --json` writes the document, the build script stops
        // with the rendered text.
        // 解析器不得不拒绝的布局——`[lib] path` 指不到文件——报到"源树缺失"所报的同一处，后果也
        // 相同：`check --json` 写出文档，构建脚本带着渲染后的文本停下。
        Err(message) => return layout_diagnostic(input, message.clone()),
    };
    let scan = &layout.scan_root;
    let src = &layout.identity_base;
    // A package with no source tree — or with something that is not a directory
    // where one is expected — is a layout diagnostic, and nothing below runs on a
    // tree that cannot be read. This used to reach
    // `expect("src directory must exist")` inside discovery: the build script died
    // with exit 101 and `check --json` printed nothing at all, which is the
    // contract that command was fixed to keep.
    // 没有源树（或该有目录的地方放着别的东西）的包得到一条布局诊断，下面任何步骤都不会在一个
    // 读不到的树上运行。这里过去会走到 discovery 里的 `expect("src directory must exist")`：
    // 构建脚本以退出 101 死掉，而 `check --json` 什么都不打印——而那正是那条命令被修好要守住的
    // 契约。
    if !scan.is_dir() {
        return layout_diagnostic(
            input,
            format!(
                "{} is not a source directory; the build reads registration faces from it, \
                 and a missing tree is not an empty one",
                scan.display()
            ),
        );
    }
    let mut unplaced = Vec::new();
    let nodes = super::source_walk::discover_package(layout, &mut unplaced);
    prime_node_id_cache(manifest, src, &nodes);
    let discovery_fingerprint = super::discovery_fingerprint(src, scan, &nodes);
    phases.mark("discover");
    // One entry, resolved once, for both readers below. Pruning and the generated
    // cut table must describe the same file, and they silently stopped doing so
    // when each resolved the entry on its own — `XIRANG_ENTRY` reached only
    // pruning, so the release could prune one file's slots while the runtime cut
    // table described another's. `host_entry_from_environment` also panics here
    // if the variable names something that is not a file, rather than letting one
    // reader fall back while the other follows it.
    // 入口只解析一次，供下面两个读取者共用。剪枝与生成的切口表必须描述同一个文件，
    // 而它们各自解析入口时就静默地不再一致——`XIRANG_ENTRY` 只作用于剪枝，发布态
    // 可能剪掉一个文件的槽位，运行期切口表却在描述另一个文件。若变量指的不是文件，
    // `host_entry_from_environment` 也在这里直接 panic，而不是让一个读取者回退、另一个
    // 跟随。
    // Resolving the entry and the scope can refuse a configured value, and those
    // refusals are diagnostics now: they used to panic, which took the build
    // script down and left `check --json` printing nothing at all.
    // 解析入口与范围可能拒绝一个被配置的取值，而这些拒绝现在是诊断：它们过去会 panic，
    // 那会打死构建脚本，并让 `check --json` 什么都不打印。
    let mut early_errors = BuildDiagnostics::default();
    let entry = super::host_entry_from_environment(layout, &nodes, &mut early_errors);
    // The claims are read **before** the scope, because they are forced-liveness roots for the
    // derivation below: a crate that claims a subtree exists to compile it, so a claim may not be
    // pruned by the narrowing that graft slots cause. Read once and used twice (scope and render),
    // like the shape itself.
    // 认领在作用域**之前**读取，因为它们是下面那次推导的强制存活根：认领一棵子树的 crate 存在的意义就是编译它，
    // 因此认领不得被嫁接槽位造成的收窄剪掉。读一次、用两次（作用域与渲染），与 crate 形状本身一样。
    let claims = input.only.as_deref().map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect::<Vec<String>>()
    });
    let scope = SourceScope::from_environment(
        src,
        &nodes,
        &entry,
        claims.as_deref().unwrap_or(&[]),
        &mut early_errors,
    );
    let cache_units = cache_directory(manifest).join("units");
    // One facts store for the **whole run**, not one per pass: the discovery pass learns a face's fields
    // and the declare/plan/render passes are the ones that ask for them. A store that died with the
    // discovery pass would leave every consumer falling back to a parse — which is exactly what the first
    // version of this switch did, and the phase line said so (declare rose instead of falling ✗).
    // 整个运行共用**一个**事实存储，而不是一趟一个：发现那一趟学到面的字段，而问它们的正是 declare/plan/render
    // 那几趟。若存储在发现那一趟结束时死掉，每个消费者都会退回解析——这正是这次切换的第一版发生的事，而相位行当场
    // 说了出来（`declare` 不降反升 ✗）。
    let _face_facts = super::discovery_cache::FaceFactsScope::open();
    let cache_state = update_discovery_cache(manifest, src, &nodes, &discovery_fingerprint);
    phases.mark("scope");

    // Faces the build cannot place or parse are reported first: the stages below
    // decode fields from a parsed face, and they skip a file they cannot parse, so
    // this diagnostic is the only thing a reader would otherwise never see. Before
    // this existed they panicked instead, which took the whole build script down
    // and left `check --json` with an empty stdout.
    // 构建安放不了或解析不了的面先被报告：下面的阶段从已解析的面里解码字段，它们会跳过
    // 自己解析不了的文件，因此这条诊断是读者本来永远看不到的东西。在这之前它们会 panic，
    // 那会打死整个构建脚本，并让 `check --json` 的 stdout 一片空白。
    // Whether this tree already published a record decides **which plan runs** (`stem 2`, step 3). It is
    // read before the declaration checks rather than beside the writes, because that is where its answer
    // is needed: a hit means this tree's bytes were already found clean once, so the eight error and demo
    // walks below cannot find anything — and the plan comes from the payload instead of from a parse.
    // 这棵树是否已发布过记录，决定**跑哪一份计划**（`stem 2` 第③步）。它在声明层检查**之前**读，而不是和写入
    // 放在一起，因为答案正是在那里被需要：命中意味着这棵树的字节曾被判定干净一次，因此下面那八趟错误与 demo
    // 遍历不可能发现任何东西——而计划来自载荷，不来自解析。
    let record = super::discovery_record::load(
        manifest,
        &discovery_fingerprint,
        &super::registry_identity::package_namespace(),
    );
    let mut compile_errors = early_errors;
    if record.is_none() {
        // A deliberately out-of-tree implementation is a **fact about the host**, not a defect: the boundary says so
        // ("it declares its own registry and this package's generated tree does not contain it; the host that grafts
        // it registers it explicitly"), and a check that goes red for following the documented workflow teaches the
        // reader to do the one thing the message no longer asks for — move the third-party file in.
        // 有意放在树外的实现是**宿主的一个事实**，不是缺陷：边界的说法就是"它自带注册机、本包的生成树不含它；嫁接
        // 它的宿主会显式登记它"，而一个对着**照文档做**的宿主变红的 check，会教会读者去做那条消息早就不再要求的事
        // ——把第三方文件搬进来。
        let external: Vec<_> = unplaced
            .iter()
            .filter(|face| face.phase == super::source_walk::EXTERNAL_FACE_PHASE)
            .cloned()
            .collect();
        for face in &external {
            eprintln!(
                "xirang: {}:{} — {} (an external face is a notice, not a defect: it is registered by the host that \
                 grafts it)",
                face.relative, face.line, face.message
            );
        }
        // If a later phase needs the full list, it starts from the defects now: the externals have been answered.
        // 若后续阶段需要完整清单，它从现在开始从缺陷出发：树外的那些已经答复过了。
        let unplaced = super::source_walk::notices_and_defects(unplaced);
        append_error(&mut compile_errors, unplaced_face_errors(&unplaced));
        append_error(&mut compile_errors, face_syntax_errors(src, &nodes));
        append_error(
            &mut compile_errors,
            aggregate_requirements(src, &nodes, false, &scope, Some(&cache_units)),
        );
        let contract_errors = aggregate_contract_errors(src, &nodes, false, &scope);
        append_error(&mut compile_errors, contract_errors);
        // A missing sibling rule is the build's own failure, so it belongs with the declaration-level
        // checks: it is what makes `check` and `cargo build` agree about a face neither can compile.
        // 缺同目录规则是构建自身的失败，因此它属于声明层检查：正是它让 `check` 与 `cargo build` 对"两边都
        // 编译不了的面"给出一致答案。
        append_error(
            &mut compile_errors,
            aggregate_registry_rule_errors(src, &nodes, false, &scope),
        );
        let stable_errors = aggregate_stable_name_errors(src, &nodes);
        append_error(&mut compile_errors, stable_errors);
        let parent_macro_errors = aggregate_parent_macro_errors(src, &nodes);
        append_error(&mut compile_errors, parent_macro_errors);
    }
    let demo_errors = if record.is_none() {
        aggregate_requirements(src, &nodes, true, &scope, Some(&cache_units))
    } else {
        // A hit: the tree was clean when the record was published, so this pass has nothing to say.
        // 命中：记录发布时这棵树是干净的，因此这一趟无话可说。
        BuildDiagnostics::default()
    };
    phases.mark("declare");
    // A hit means the tree's bytes were found clean once — but the **products** this build would read
    // them from (`out_dir`) are not necessarily that tree's: a partition narrows the plan per crate and
    // writes it there, and a later build of a **different** tree reuses the same directory. So the copy
    // is read only when its `discovery.fingerprint` is this build's; otherwise the work is done again.
    // Note the two failure modes this avoids at once: reading the copy unconditionally described the
    // **pre-promote** tree (the promote reuses `out_dir`), and reading the record's copy unconditionally
    // dropped the **per-crate narrowing** a partition applies (measured: leg 6's two crates each
    // reported the host's whole cut table).
    // 命中意味着这棵树的字节曾被判定干净一次——但本构建要读的那些**产物**（`out_dir`）不一定属于这棵树：
    // 分区会按 crate 收窄计划并写在那里，而之后**另一棵树**的构建会复用同一个目录。因此只有在它的
    // `discovery.fingerprint` 就是本次的指纹时才读那份副本，否则重新把活干一遍。注意这一条同时避开了两种
    // 失败模式：无条件读副本描述的是 **promote 之前**那棵树（promote 复用 `out_dir`），而无条件读记录的副本
    // 丢掉了分区所施加的**按 crate 收窄**（实测：leg 6 的两个 crate 各自报出了宿主的整张切口表）。
    let copy_is_this_tree = std::fs::read_to_string(input.out_dir.join("discovery.fingerprint"))
        .is_ok_and(|text| text.trim() == discovery_fingerprint);
    // 声明面的身份只由 (命名空间, 源码拼写, 名字) 决定，因此能在静态计划**之前**算好：一个 Rust 面
    // 把父写成 `crate::xirang_external::<ident>` 时，静态计划要能当场解析出那个身份。
    // A declared face's identity is decided by (namespace, source spelling, name) alone, so it can be
    // computed **before** the static plan: when a Rust face spells its parent as
    // `crate::xirang_external::<ident>`, the plan has to resolve that identity on the spot.
    let native_parents: std::collections::BTreeMap<String, super::registry_identity::NodeId> =
        native
            .iter()
            .map(|(ident, id, _)| (ident.clone(), *id))
            .collect();
    let declared_elsewhere: Vec<(super::registry_identity::NodeId, bool)> =
        native.iter().map(|(_, id, owns)| (*id, *owns)).collect();
    // A partition parses its faces instead of reading the row cache: the cache is written by the
    // package whose scope wrote it, and a row it never carried (a face the host cut away, a file
    // added after the row pass) used to be a **missing face** — the fragment silently planned one
    // fewer face than the tree it was handed (measured 2026-10-22: the second pass dropped
    // `shop::panel::area::reading`). Parsing is the authority; the rows are an optimization.
    // 碎片不读行缓存而是解析源码：缓存由写它的那个包的 scope 写，它没带的行（宿主切走的面、行扫描之后
    // 才加的文件）过去等于**面不存在**——碎片会悄悄少计划一个面（2026-10-22 实测：第二遍丢掉了
    // `shop::panel::area::reading`）。解析才是权威，行只是优化。
    let partition = input.only.is_some() || input.facade;
    // A package with declared foreign faces parses instead of reading rows: a row records the parent
    // this tree resolved when it was written, and a foreign parent is resolved from the declaration
    // text plus the native list — a record published by a surface that lacked that list carries the
    // root as the parent, and reusing it drops the Rust child silently (measured 2026-10-24: five
    // faces parsed, four planned, `sample` missing until a clean rebuild).
    // 带外语声明的包改为解析而不是读行：行记录的是写它时这棵树解析出的父级，而外语父级由声明文本加
    // 原生面清单解析——由一个缺少该清单的执行面发布的记录会把根记成父级，复用它就会静默丢掉 Rust 子面
    // （2026-10-24 实测：解析出 5 面、计划里 4 面，`sample` 直到干净重建才回来）。
    let use_rows = record.is_some() && copy_is_this_tree && !partition && native.is_empty();
    let (static_faces, static_errors) =
        super::discovery_cache::with_native_parents(&native_parents, || {
            if use_rows {
                match super::discovery_record::read_face_rows(&input.out_dir).and_then(|rows| {
                    super::static_plan::static_plan_from_rows(
                        src,
                        &nodes,
                        &scope,
                        &rows,
                        &declared_elsewhere,
                    )
                }) {
                    Ok((faces, errors)) => (faces, errors),
                    Err(_) => static_plan(src, &nodes, &scope, &declared_elsewhere),
                }
            } else {
                static_plan(src, &nodes, &scope, &declared_elsewhere)
            }
        });
    append_error(&mut compile_errors, static_errors);
    // A foreign face can be a **parent** only if the header names it before the source that hangs a
    // child under it is compiled. Its identity is the hash of (namespace, source spelling, name) — all
    // three are in the declaration text — so the identity, the module path and the identifier are
    // derived here, in the same pass that computes every Rust face's. What the compiler hands back
    // afterwards is the record: exports, flags, and the check that the name and the parent agree.
    // 一个外语面要成为**父**，头文件必须在"把子面挂到它下面"的源码被编译之前就点名它。它的身份是
    // (命名空间, 源码拼写, 名字) 的散列——三者都在声明文本里——因此身份、模块路径与标识符在这里推导，
    // 与每个 Rust 面的身份同一趟算出。编译器之后交回的是记录：导出、标志，以及"名字与父是否与这次推导一致"。
    let mut known_idents: std::collections::BTreeMap<
        String,
        (String, super::registry_identity::NodeId),
    > = static_faces
        .iter()
        .map(|face| {
            (
                super::face_header::identifier(&face.module, &face.kind),
                (face.module.clone(), face.id),
            )
        })
        .collect();
    // A C declaration names its parent by the header's flat constant (`XIRANG_FACES_PANEL`), whose
    // suffix is the module folded and uppercased. The resolver has to know that spelling too, or a
    // C face under a Rust parent is refused even though the header names the parent.
    // C 声明经头文件的扁平常量（`XIRANG_FACES_PANEL`）点名父级，其后缀是模块折成大写的样子。解析器也必须
    // 知道这个拼法，否则挂在 Rust 父级下的 C 面会被拒绝——尽管头文件点得到那个父。
    #[cfg(feature = "lang-c")]
    super::face_header::fill_c_flat_modules(&mut known_idents, &static_faces);
    // The retired module paths of this package's objects stay nameable: a third party's declaration
    // that names a moved parent by its old path must resolve here exactly as the header aliases it.
    // 本包对象退役的模块路径仍然可被点名：按旧路径点名被搬父级的第三方声明，必须在这里像头部起别名那样解析。
    let live = super::compatibility::live_faces(
        &static_faces,
        declared,
        &super::registry_identity::package_namespace(),
    );
    super::compatibility::extend_known(&mut known_idents, &layout.package_root, &live);
    // The generated Zig module names a parent by its **module** constant, so the same map resolves
    // `xirang_faces.shop__panel` as it resolves the C++ spelling. A module owns one face; a second
    // face in the same module is a duplicate the resolver refuses.
    // 生成的 Zig 模块按**模块**常量命名父级，因此同一张映射也能解析 `xirang_faces.shop__panel`，与解析 C++
    // 拼法一样。一个模块拥有一个面；同模块第二个面是解析器会拒绝的重复。
    #[cfg(feature = "lang-zig")]
    super::face_header::fill_static_zig_modules(&mut known_idents, &static_faces);
    // Seed the logical catalog paths before native declarations are resolved: a C++ declaration
    // generated by the authoring service names its Rust parent through `crate::catalog::…`.
    // 先预置逻辑目录路径，再解析外语声明：创作服务生成的 C++ 声明经 `crate::catalog::…` 点名 Rust 父。
    pipeline_catalog::seed_known(&layout.package_root, &static_faces, &mut known_idents);
    let native = match super::declared_native::resolve(
        &super::registry_identity::package_namespace(),
        declared,
        &known_idents,
    ) {
        Ok(native) => native,
        Err(refusal) => {
            compile_errors.push(BuildDiagnostic::new("native", refusal));
            Vec::new()
        }
    };
    #[cfg(feature = "lang-zig")]
    super::face_header::fill_native_zig_modules(&mut known_idents, &native);
    // A plan with cuts narrows the build to the subtrees those cuts name, and a face no cut names is not
    // published. That narrowing is the feature — it is what makes a slot replaceable without shipping the
    // rest — but it is also the one consequence a reader could not see: neither `check` nor the build said
    // anything, and the only place the answer existed was `explain --overlay`. Measured on a hand-built
    // project: one cut took the run from 6 faces to 3, the build reported success, and the only way to
    // learn it was to ask a different command (audit `M7`, §M7.58).
    // 带切口的计划会把构建收窄到那些切口命名的子树，而没有切口点名的面不会被发布。那次收窄是功能本身——它
    // 正是"让一个槽位可替换而不必发布其余"的东西——但它也是读者唯一看不见的后果：`check` 与构建都一个字不说，
    // 答案只存在于 `explain --overlay` 里。在一个手工项目上实测：一条切口让这次运行从 6 个面变成 3 个，构建
    // 报告成功，而要知道这件事只能去问另一条命令（审计 `M7`，§M7.58）。
    //
    // Spoken here because this is the one moment both lists are in hand, and spoken with a **paste-ready
    // clause** rather than advice: the shared `cut(…) graft(…)` form is what keeps a face in the tree
    // without an external implementation to link (the same self-graft `promote` lands, audit `M7`, §M7.42).
    // 在这里说，因为这是两份清单同时在手的唯一时刻；而且给的是**可粘贴的子句**而不是建议：共用的
    // `cut(…) graft(…)` 形式正是"既保住这个面、又不需要链接外部实现"的写法（与 `promote` 落地的是同一种
    // 自嫁接，审计 `M7`，§M7.42）。
    // A **generated** package carries its host's declarations as data, because the release shape has no
    // entry to read: the ghost compiles copied faces and nothing else. The entry wins when this tree has
    // one — the development shape reads the host's own tree, so a host that edits its plan must not keep
    // answering from a snapshot taken when the package was generated. Both routes end in the same
    // `enable_graft_entries`, so the `cfg` gate is evaluated once, by one rule.
    // **生成的**包把宿主的声明当作数据携带，因为发布形状没有入口可读：幽灵编译的是复制过来的面，仅此
    // 而已。本树自己有声明时**入口赢**——开发形状读的就是宿主自己的树，因此改了计划的宿主不该继续用生成
    // 那个时刻的快照作答。两条路都终于同一个 `enable_graft_entries`，因此 `cfg` 门控只由一条规则求值一次。
    let from_entry = super::host_graft_entries(&entry, &mut compile_errors);
    // Counted before the value is moved into the branch below: the note further down needs to know
    // whether this tree declares any slot at all (gated declarations included — a declaration the
    // `cfg` turned off is still a declaration the entry carries).
    // 在这个值被下面的分支移走之前先数出来：再往下那条提示需要知道这棵树到底有没有声明槽位（被门控关掉的
    // 也算——`cfg` 关掉的声明仍然是入口携带的声明）。
    let declared_slots = from_entry.declared.len();
    let mut graft_entries = if from_entry.declared.is_empty() && !input.host_cuts.is_empty() {
        super::enable_graft_entries(
            input.host_cuts.iter().map(super::HostCut::syntax).collect(),
            entry.path(),
            &mut compile_errors,
        )
    } else {
        from_entry
    };
    // Spoken **after** the declarations are read — not before, as it was: the sentence's reason clause
    // depends on how many slots this tree declares, and saying "the cuts in <entry> …" on a tree with
    // none is the lie the 2026-10-10 queue recorded (`S8-ours`: three-line entry, zero cut sites, the
    // same session's `check` said so — report §7h).
    // 在**读完声明之后**才说——不再是之前：这句话的原因子句取决于这棵树声明了几个槽位，而在一个槽位都没有的
    // 树上说"<入口> 里的切口……"，正是 2026-10-10 队列记下的那句假话（`S8-ours`：三行入口、零个 cut 站点，
    // 同一次会话的 `check` 自己这么说——报告 §7h）。
    if let Some(roots) = &scope.roots
        && let Some(lines) = super::scope::outside_slots_note(
            entry.path(),
            &super::collect_faces(src, &nodes),
            roots,
            declared_slots,
        )
    {
        for line in lines {
            // A build script's stderr is captured by cargo and printed only under `-vv`, so on the
            // ordinary `cargo build` this sentence reached nobody: measured on the control-button
            // example with one face outside its cuts — `cargo build` printed nothing, `cargo build -vv`
            // and `xirang check` both printed it. The warning channel is the one cargo prints on a
            // successful build, and this file already uses it for the cache status line.
            // 构建脚本的 stderr 会被 cargo 捕获、只在 `-vv` 下打印，因此在普通的 `cargo build` 里这句话
            // 到不了任何人：在 control-button 示例上实测（一个面落在切口之外）——`cargo build` 一个字不说，
            // `cargo build -vv` 与 `xirang check` 都会说。warning 通道正是 cargo 在成功构建时也会打印的那条，
            // 本文件已用它发缓存状态行。
            if input.emit_cargo_directives {
                println!("cargo:warning={line}");
            } else {
                eprintln!("{line}");
            }
        }
    }
    // Overlap is answered by **declaration order** (the later entry wins), and this is the one place
    // that has to apply it: the record below and the rendered table are both built from what survives.
    // The hint is spoken here, once, so a reader hears the decision instead of inferring it.
    // 重叠由**声明顺序**作答（后一条赢），而这里是必须应用它的那一个地方：下面的记录与渲染出的表都由幸存者
    // 构建。提示在这里说一次，好让读者听到裁定而不是自己去推断。
    {
        let pairs: Vec<(String, String)> = graft_entries
            .declared
            .iter()
            .filter_map(|entry| {
                super::scope::cut_subtree(&entry.cut).map(|subtree| (subtree, entry.cut.clone()))
            })
            .collect();
        let (superseded, overlaps) =
            xirang_kernel::registry_core::shape::superseded_by_later(&pairs);
        for (earlier, later) in &overlaps {
            eprintln!(
                "xirang: two graft cuts overlap: `{earlier}` (earlier, ignored) and `{later}` \
                 (later, wins); the later entry answers, so `{later}` replaces its subtree"
            );
        }
        // Dropped by **position**, not by subtree, and the difference is a defect rather than a nicety: two cuts
        // naming the same slot have the same subtree, so a subtree-keyed drop removes the winner together with the
        // entry it beat, and the plan is left with **no row at all** — the declaration says one cut answers and the
        // published plan says none does. Measured 2026-10-18 in `tools/xirang-graft-matrix` leg 6 (two identical
        // string cuts for one slot: `check` names the winner, the plan carries 0 rows).
        // 按**位置**丢弃，而不是按子树；这个差别是缺陷而不是讲究：两条切口点名同一个槽位时子树相同，因此按子树键丢弃
        // 会把赢家连同它击败的那条一起删掉，计划里**一行都不剩**——声明说有一条切口作数，而发布的计划说一条也没有。
        // 2026-10-18 在 `tools/xirang-graft-matrix` leg 6 实测（同一槽位的两条相同字符串切口：`check` 点名了赢家，
        // 计划却是 0 行）。
        retain_unsuperseded(&mut graft_entries.declared, &superseded);
        retain_unsuperseded(&mut graft_entries.enabled, &superseded);
    }
    // A plan file is an authoring record the build never reads, so a plan whose
    // slot no declaration names would otherwise be discovered at runtime, long
    // after the build pruned it. This is the one case that is unambiguous *and*
    // decidable here: the entry's full declaration list is in hand, so a slot
    // that only a gated declaration names is not reported (that configuration is
    // legitimate — the gate is the author's), while a slot nothing can name is a
    // build failure rather than a warning, because shipping a binary whose graft
    // silently never happens is exactly what must not leave the build.
    // 计划文件是构建从不读取的创作记录，因此没有任何声明命名其槽位的计划，只会在运行期
    // 才被发现——那时构建早已把它剪掉。这里是唯一既无歧义、又能在构建期判定的一类：入口
    // 的完整声明清单就在手里，因此只有门控声明命名的槽位不会被上报（那是合法配置——门控
    // 是作者的事），而没有任何声明可能命名的槽位是构建失败而不只是警告，因为"发布一个
    // 嫁接静默地从未发生的二进制"正是绝不能离开构建的东西。
    let plan_errors = graft_plan_check::undeclared_plan_errors(
        &graft_plan_check::planned_slots(manifest),
        &graft_entries
            .declared
            .iter()
            .map(super::declared_graft_view)
            .collect::<Vec<_>>(),
        |id| graft_plan_check::slot_module(src, &nodes, id),
    );
    append_error(&mut compile_errors, plan_errors);
    // A record this build cannot honor must not leave it: skipping an **unsupported version** would
    // ship a binary whose graft silently never happens. Malformed files stay the authoring surfaces'
    // business — they cannot even say what they promised.
    // 本构建兑现不了的记录不得离开它：跳过**不支持的版本**等于发布一个嫁接静默地从未发生的二进制。
    // 畸形文件仍归创作界面管——它们连自己承诺过什么都说不出来。
    append_error(
        &mut compile_errors,
        graft_plan_check::unsupported_version_errors(manifest),
    );
    phases.mark("plan");
    // The crate shape is an input to the **render** as well as to the release checks: a subtree the
    // declaration hands to another crate must not be compiled here a second time, so the host's
    // generated tree emits no module for it while the records below still carry every face (the tree
    // is unchanged; what changes is which crate compiles it). Read once, used twice — audit `M7`, P3.2.
    // crate 形状既是发布校验的输入，也是**渲染**的输入：被声明交给另一个 crate 的子树不能在这里再编译一遍，
    // 因此宿主的生成树不为它发射模块，而下面的记录仍带着每一个面（树没变，变的是"由哪个 crate 编译"）。
    // 读一次、用两次——审计 `M7`，P3.2。
    let shape = match super::shape_decl::read_shape_declaration(&layout.package_root) {
        Ok(shape) => shape,
        Err(refusal) => {
            compile_errors.push(BuildDiagnostic::new("add-crates", refusal));
            None
        }
    };
    // The render mode comes first, because it decides **which** subtrees this run must not render: a
    // ghost is the crate the declared subtrees were handed to, so it hands nothing away. The claims
    // themselves were read above, before the scope, because they are forced-liveness roots there.
    // 先定渲染模式，因为它决定本次运行**不得**渲染哪些子树：幽灵正是那些被交出去的子树所交给的 crate，因此它
    // 什么都不交出去。认领本身在上面、作用域之前就读过了，因为它们是那里的强制存活根。
    let only = claims.clone();
    let cut_out = super::crate_plan::cut_out_for(shape.as_ref(), only.is_some());
    // The plan is computed **here**, before the render, because the render needs what it produces: the
    // mount spelling per face. A ghost's files live in the host package, so the spelling has to walk
    // out of the ghost and into the host's `src` — and `file!()` reports that spelling, which makes it
    // part of the identity (audit `M7`, P3.2). A plan this build cannot make is refused by name here,
    // which is also where the generated tree can carry the refusal.
    // 规划在**这里**、渲染之前算，因为渲染需要它的产物：每个面的挂载拼写。幽灵的文件住在宿主包里，因此拼写
    // 必须走出幽灵、走进宿主的 `src`——而 `file!()` 报告的就是那个拼写，于是它是身份的一部分（审计 `M7`，
    // P3.2）。本次构建做不出的规划在这里被点名拒绝，而生成树也正好能承载那条拒绝。
    let shape_faces: Vec<(String, String, super::registry_identity::NodeId)> = static_faces
        .iter()
        .map(|face| (face.source.clone(), face.module.clone(), face.id))
        .collect();
    let planned = match &shape {
        Some(shape) => match super::crate_plan::plan(
            &layout.package_root,
            &super::registry_identity::package_namespace(),
            shape,
            &shape_faces,
            // This call plans the **mounts and shells** the render needs; it writes no build script, so
            // the cuts it carries are inert here. They are passed through rather than replaced with an
            // empty slice so there is one answer to "what does this package carry".
            // 本次调用规划的是渲染所需的**挂载与壳**；它不写任何构建脚本，因此这里携带的切口是惰性的。它们
            // 被原样传下去而不是换成空切片，这样"本包携带什么"只有一个答案。
            &input.host_cuts,
        ) {
            Ok(planned) => planned,
            Err(refusal) => {
                compile_errors.push(BuildDiagnostic::new("add-crates", refusal));
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    // One plan, two consumers: the mounts (the `#[path]` spelling per face) and the shells (the
    // ancestors' identities, so a fragment's `parent:` still resolves).
    // 一份规划、两个消费者：挂载（每个面的 `#[path]` 拼写）与壳（祖先的身份，好让碎片的 `parent:` 仍然解析）。
    let mounts: Vec<(String, String)> = planned
        .iter()
        .flat_map(|crate_plan| {
            crate_plan
                .mounts
                .iter()
                .map(|mount| (mount.module_path.clone(), mount.spelling.clone()))
        })
        .collect();
    let ancestors: Vec<(String, [u8; 16])> = planned
        .iter()
        .flat_map(|crate_plan| {
            crate_plan
                .ancestors
                .iter()
                .map(|(module, id)| (module.clone(), id.into_bytes()))
        })
        .collect();
    // The facade renders the **cross-crate** half: no modules, and every `crate::<module>` rewritten to
    // the crate that compiles that module. The owner of a module comes from the plan (a claim, its root
    // face and its ancestor shells all belong to that ghost), and every other module stays with the
    // host — including the host's own faces, because `crate::` inside the facade means the facade.
    // facade 渲染的是**跨 crate**那一半：不发模块，且每个 `crate::<模块>` 都改写成编译该模块的那个 crate。
    // 模块的属主来自规划：一条认领、它的根面与它的祖先壳都归那个幽灵，其余模块留在宿主——**包括宿主自己的
    // 面**，因为 facade 里的 `crate::` 指的是 facade 自己。
    let facade = input.facade;
    let host_crate = input
        .host_crate
        .clone()
        .or_else(|| super::package::library_name(manifest).ok())
        .unwrap_or_else(super::registry_identity::package_namespace);
    let owners: Vec<(String, String)> =
        super::crate_plan::owners(&host_crate, &shape_faces, &planned);
    phases.mark("shape");
    // A crate asserts only about the faces **it compiles**. The registration block and the identity
    // assertions both name modules, so after a partition the host must drop every face a ghost took and
    // a ghost must keep only its own — otherwise the host's generated tree names `panel::frame::…`, a
    // module it no longer has (`error[E0433]: cannot find 'frame' in 'panel'`, measured on the first
    // end-to-end build). The facade carries the union, which is why it keeps them all.
    // 一个 crate 只为**自己编译的面**做断言。注册块与身份断言都点名模块，因此分区之后宿主必须丢掉被幽灵拿走的
    // 每个面、幽灵必须只留自己的——否则宿主的生成树会点名 `panel::frame::…`，一个它已经没有的模块
    // （第一次端到端构建实测 `error[E0433]: cannot find 'frame' in 'panel'`）。facade 携带并集，因此它全都留。
    let all_claims: Vec<String> = shape
        .as_ref()
        .map(|shape| shape.cut_subtrees())
        .unwrap_or_default();
    let visible: Vec<super::static_plan::StaticFaceRecord> = static_faces
        .iter()
        .filter(|face| {
            if facade {
                return true;
            }
            match only.as_deref() {
                Some(claims) => under_a_claim(&face.module, claims.iter().map(String::as_str)),
                None => !under_a_claim(&face.module, all_claims.iter().map(String::as_str)),
            }
        })
        .cloned()
        .collect();
    // Both modes come from one shape, read once: a host renders all but what it hands away, a ghost
    // renders only its fragment (audit `M7`, P3.2).
    // 两种模式出自同一份形状，只读一次：宿主渲染除交出去之外的全部，幽灵只渲染自己的碎片（审计 `M7`，P3.2）。
    // The cuts this crate emits are attributed **once** and read twice: the renderer compiles them into
    // `BUILTIN_GRAFT_CUTS`, and `graft_plan.tsv` records the same list. Two filters was one filter too
    // many — the audit text used to carry the host's whole plan into every package (audit `M7`, §M7.55).
    // 本 crate 发射的切口只判**一次**、读两处：渲染器把它们编进 `BUILTIN_GRAFT_CUTS`，`graft_plan.tsv`
    // 记录同一份清单。两个过滤器就是多了一个——审计文本过去把宿主的整份计划带进了每一个包（审计 `M7`，§M7.55）。
    let emitted_grafts = super::renderer::emitted_grafts(&visible, &graft_entries.enabled, facade);
    // The walk writes each generated container into its own file as it goes (`stem 2`, step 3): the
    // render used to hand back one big string that `write_modules` then parsed again to split it, and
    // that round trip measured 3.710 s on a 1,000-face host.
    // 这次遍历在下行的同时把每个生成的容器写进它自己的文件（`stem 2` 第③步）：渲染过去交回一大份字符串，
    // 再由 `write_modules` 解析它来切分，而那趟往返在 1,000 面的宿主上实测 3.710 s。
    let module_directory = input.out_dir.join("xirang_modules");
    let mut module_files = Vec::new();
    let mut generated = render_lib_with_external(
        src,
        &nodes,
        &compile_errors,
        &demo_errors,
        &scope,
        &visible,
        &emitted_grafts,
        if only.is_none() && cut_out.is_empty() {
            // No declaration at all: this crate renders the whole tree, which is what every host did
            // before `add_crates.rs` existed.
            // 完全没有声明：本 crate 渲染整棵树，也就是 `add_crates.rs` 存在之前每个宿主的样子。
            super::renderer::ShapeRender::whole()
        } else {
            super::renderer::ShapeRender {
                cut_out: &cut_out,
                only: only.as_deref(),
                mounts: &mounts,
                ancestors: &ancestors,
                facade,
                owners: &owners,
            }
        },
        &module_directory,
        &mut module_files,
        external,
        &native,
    );
    #[rustfmt::skip]
    let cpp_catalog = pipeline_catalog::append(&mut generated, &layout.package_root, catalog_declared, &known_idents, &static_faces, &mut compile_errors);

    // A host whose declaration has already been written re-exports each claimed subtree from its
    // generated crate, so host faces outside the subtree keep naming the same types through
    // `crate::<subtree>::…`. A ghost is excluded (`only` is its own crate) and a declaration without
    // written packages contributes nothing (see `partition_compat::reexports`).
    // 声明已经写下的宿主，会把每棵被认领子树从生成 crate re-export，使子树之外的宿主面继续用
    // `crate::<子树>::…` 点名同一批类型。幽灵被排除（`only` 就是它自己），而只有声明、没有已写下包的
    // 情形不贡献任何东西（见 `partition_compat::reexports`）。
    super::partition_compat::append(
        &mut generated,
        shape.as_ref(),
        &layout.package_root,
        only.is_none(),
    );
    let out_dir = &input.out_dir;
    phases.mark("render");
    // Write failures are collected rather than fatal here. They are the one
    // failure the generated tree cannot carry, because the file that would carry
    // it is exactly what could not be written, so the two callers are told
    // differently below: a build script stops with the reason, and a structured
    // caller (`check --json`) reports it with the rest of the diagnostics.
    // 写失败在这里被收集而不是立即致命。它们是生成树唯一无法承载的失败——本该承载它的文件
    // 正是写不成的那个——因此下面按两种调用方分别处理：构建脚本带着原因停下，
    // 结构化调用方（`check --json`）把它与其余诊断一起报出。
    let mut write_errors = Vec::new();
    // The generated headers: the C++ one a translation unit includes to name a Rust face by
    // constant, and the Zig identity module a hand-written declaration names its parent through.
    // Both are written beside the generated tree rather than into the source tree, because they are
    // products of this build and a copy under `src/` would be edited by mistake.
    // 生成件：C++ 翻译单元按常量命名 Rust 面所用的头，与手写 Zig 声明经其点名父级的恒等模块。两者都写在
    // 生成树旁边而不是源码树里，因为它们是本次构建的产物，放在 `src/` 下的副本迟早会被误改。
    // The constants file a record-file language names its parents through: written from the same
    // face lists the header is written from, before the producer that reads it runs.
    // 记录文件语言点名父级所用的常量文件：从与头文件相同的那两份面清单写出，在读取它的生产者运行之前。
    if let Some(request) = constants
        && let Err(why) = super::record_constants::write_for_faces(
            request,
            &layout.package_root,
            &super::registry_identity::package_namespace(),
            &static_faces,
            &native,
        )
    {
        write_errors.push(why);
    }
    let (mut header_errors, refusal) = super::face_header::write_headers(
        out_dir,
        &static_faces,
        &native,
        &live,
        &layout.package_root,
        &super::registry_identity::package_namespace(),
        &cpp_catalog,
    );
    write_errors.append(&mut header_errors);
    if let Some(refusal) = refusal {
        compile_errors.push(BuildDiagnostic::new("native-header", refusal));
    }
    // The module paths this tree's objects have left behind stay nameable: a physical move that changes
    // a parent's module path would otherwise break every third party that names it through
    // `::xirang::faces::<module>::NODE_ID` — and that third party may live in a declared (read-only)
    // source, so "rewrite the reference" is not always a thing this package is allowed to do.
    // 这棵树里的对象离开过的模块路径仍然可以被点名：否则一次改变父模块路径的物理搬动会弄坏每一个经
    // `::xirang::faces::<模块>::NODE_ID` 点名它的第三方——而那个第三方可能住在已声明（只读）来源里，
    // 因此"改写那条引用"并不总是本包被允许做的事。
    if input.emit_cargo_directives
        && let Some(status) = cache_status_line(
            &cache_state,
            &discovery_fingerprint,
            build_output_is_verbose(),
        )
    {
        println!("cargo:warning={status}");
    }
    // The payloads are published first; the fingerprint is written below, once
    // every one of them has landed. The fingerprint is the token
    // `build_output_is_current` reads, so only a clean run may write it: a failed
    // run publishes no token at all, and a reader then asks the build instead of
    // trusting output that run left behind. Writing the token first — which is what
    // this did — leaves a new token beside a manifest that is old or missing when a
    // payload write fails, and the next reader calls that mixed generation
    // `current` and trusts its rows; the pruning column is what a maintainer reads
    // before a release prunes. The prose here always required "a clean run", but the
    // check looked at the diagnostics only, never at the writes.
    // 载荷先发布；指纹在下面、它们全部落地之后才写。指纹是 `build_output_is_current` 读取的那枚
    // 凭据，因此只有干净的一次运行才能写它：失败的一次运行不发布任何凭据，读取方于是去问构建，而不是
    // 相信那次运行留下的产物。先写凭据——也就是这里过去做的事——会在某份载荷写失败时留下"新凭据 +
    // 旧或缺的清单"，下一个读取方把这种混代产物称作 `current` 并相信它的行；而修剪列正是维护者在发布
    // 剪枝前读的东西。这段说明一直要求"干净的一次运行"，但检查只看诊断，从不看写入。
    // The record's rows are produced **once** and published twice: `pruning_manifest.tsv` is their
    // record form, and `graph_edges.tsv` is the same rows seen as the graph (audit `M7`, P1.1).
    // Reading the manifest back to build the graph would be a second parse of the same fifty
    // thousand lines, and it would also let the two files disagree about a face.
    // 记录的行**只产出一次**、发布两次：`pruning_manifest.tsv` 是它们的记录形态，而 `graph_edges.tsv`
    // 是同一批行按图来看的样子（审计 `M7`，P1.1）。把清单读回来建图等于把同一批五万行解析第二遍，而且会让
    // 两份文件对一个面产生分歧。
    // One writer per tree for the duration of the **payload set** (audit `M7`, P3.4-lock): each file
    // below is atomic on its own, but the set is not, and two processes publishing one tree would let
    // the later one win file by file. The lock is taken here rather than around the whole run because
    // everything above is computation (discovery, parsing, rendering) — idempotent, and the expensive
    // part — while what must not interleave is this write phase.
    // 在**载荷集合**的整个期间一棵树只有一个写者（审计 `M7`，P3.4-lock）：下面每一份文件各自原子，但这一组
    // 不是，而两个进程发布同一棵树会让后写者逐份取胜。锁在这里取、而不是包住整次运行，因为上面的全是计算
    // （发现、解析、渲染）——幂等，而且是昂贵的部分——不许交错的是这段写入。
    let publish_lock = match super::publish_lock::acquire(out_dir) {
        Ok(lock) => Some(lock),
        Err(refusal) => {
            // A run that cannot take the tree's lock publishes **nothing**: half a generation beside
            // another run's files is exactly the state the lock exists to prevent.
            // 拿不到这棵树的锁的运行**什么都不发布**：半代产物躺在另一次运行的文件旁边，正是这把锁存在的
            // 理由。
            compile_errors.push(BuildDiagnostic::new("publish-lock", refusal));
            None
        }
    };
    if let Some(lock) = &publish_lock
        && let Some(pid) = lock.stolen_from
        && input.emit_cargo_directives
    {
        println!(
            "cargo:warning=removed a stale publish lock left by pid {pid} (that process is gone)"
        );
    }
    if publish_lock.is_some() {
        // Stage 1 of the discovery record (`docs/design-discovery-record.md`): the tree-level products
        // are computed **once per tree** and copied by every package after that. What a hit skips is the
        // *derivation* — the pruning rows take three walks over every face, the file and function rows
        // read and scan every file — and what it costs is one copy plus reading the two row sets back
        // from the copies, which the steps below still need in hand.
        // 发现记录的阶段 1（`docs/design-discovery-record.md`）：树级产物**一棵树算一次**，此后各包复制。一次命中
        // 跳过的是**推导**——修剪行要对每个面走三趟，文件与函数行要读并扫过每个文件——代价是一次复制，以及把下面
        // 仍然需要的那两份行从复制里读回来。
        let mut pruning_rows = Vec::new();
        let mut file_rows = Vec::new();
        // Sub-phases of `write`: the coarse mark said 4–6 s on a 1,000-face tree and hid which of the
        // jobs behind it pays (`stem 2`, step 3). Same instrument, finer grain — and the numbers decide
        // what the projection has to replace first.
        // `write` 的子相位：粗标记在 1,000 面的树上报 4–6 s，却把"后面几件活里是哪一件在付钱"藏了起来
        // （`stem 2` 第③步）。同一套量器、更细的粒度——而这些数决定投影该先替换什么。
        if let Some(record) = &record {
            if let Err(error) = super::discovery_record::copy_products(record, out_dir) {
                write_errors.push(error);
            }
            match super::read_pruning_manifest(out_dir) {
                Ok(rows) => pruning_rows = rows,
                Err(error) => write_errors.push(error),
            }
            match super::discovery_record::read_file_rows(out_dir) {
                Ok(rows) => file_rows = rows,
                Err(error) => write_errors.push(error),
            }
            phases.mark("reuse");
        } else {
            match write_pruning_manifest(src, &nodes, out_dir) {
                Ok(rows) => pruning_rows = rows,
                Err(error) => write_errors.push(error),
            }
            phases.mark("manifest.pruning");
            // The declarations are the second half of the graph's raw material: a called name that exactly
            // one file declares is a dependency on that file, and the graph carries that edge rather than
            // making every reader re-derive it (audit `M7`, P1.1).
            // 声明是图的另一半原料：一个恰好被一份文件声明的被调用名字，就是对那份文件的一处依赖，而图直接带上
            // 那条边，而不是让每个读者重新推导（审计 `M7`，P1.1）。
            match write_file_manifest(src, &nodes, out_dir) {
                Ok(rows) => file_rows = rows,
                Err(error) => write_errors.push(error),
            }
            phases.mark("manifest.files");
        }
        let mut graph = None;
        // Unrolled so each job carries its own mark: the coarse `manifest.rest` said 4.7 s on a
        // 1,000-face tree and did not say which of these five paid for it (`stem 2`, step 3).
        // 拆开，让每件活带自己的标记：粗的 `manifest.rest` 在 1,000 面的树上报 4.7 s，却没说这五件里是哪一件付的钱
        // （`stem 2` 第③步）。
        let mut failures: Vec<String> = Vec::new();
        let mut run = |label: &'static str, result: Result<(), String>, phases: &mut PhaseLog| {
            if let Err(error) = result {
                failures.push(error);
            }
            phases.mark(label);
        };
        run(
            "manifest.functions",
            if record.is_none() {
                write_function_manifest(src, &nodes, out_dir)
            } else {
                Ok(())
            },
            &mut phases,
        );
        run(
            "manifest.scope",
            write_source_scope_manifest(src, &nodes, &scope, out_dir),
            &mut phases,
        );
        run(
            "manifest.shape",
            if record.is_none() {
                write_shape_manifest(src, &nodes, out_dir)
            } else {
                Ok(())
            },
            &mut phases,
        );
        // **Always**, hit or miss: this file is what the tree *publishes*, and it is the only one of the
        // three that the caller's `XIRANG_SHAPE_ONLY` narrows — `emitted_grafts` is the fragment's own
        // cuts. Skipping it on a hit left whatever the **last** build wrote into the same `target/`, so a
        // fragment read the host's whole cut table while its generated tree carried one cut (measured on
        // `tools/xirang-partition-rehearsal` leg 6: each crate reported both cuts, and the union came out
        // twice the baseline's). The record makes the **computation** skippable; a published artifact is
        // not the computation.
        // **总是**写，命中与否：这份文件是这棵树**发布**的东西，而它是三件里唯一会被调用方的
        // `XIRANG_SHAPE_ONLY` 收窄的——`emitted_grafts` 就是本碎片的切口。命中时跳过它，会让同一个
        // `target/` 里留着**上一次**构建写下的那一份，于是碎片的生成树只带一条切口、它读到的却是宿主的整张表
        // （在 `tools/xirang-partition-rehearsal` leg 6 上实测：每个 crate 都报出两条切口，并集是基线的两倍）。
        // 记录让**计算**可跳；**发布出去的产物**不是计算。
        run(
            "manifest.graft",
            write_graft_manifest(out_dir, &emitted_grafts),
            &mut phases,
        );
        run(
            "modules",
            super::renderer::write_modules(out_dir, &generated, &module_files),
            &mut phases,
        );
        write_errors.extend(failures);
        phases.mark("manifest.rest");
        match write_graph_manifest(out_dir, &pruning_rows, &file_rows, &graft_entries.enabled) {
            Ok(header) => graph = Some(header),
            Err(error) => write_errors.push(error),
        }
        // The tree's faces, straight out of the units (`stem 2`, step 3) — no face is parsed to write
        // this. Only on a **miss**: on a hit the file already arrived inside the record, and re-deriving
        // it would be exactly the parse the projection exists to avoid. It is a payload, so the generic
        // publish/copy path carries it to every package of this tree with the other five.
        // 树里的面，直接从 unit 里读出（`stem 2` 第③步）——写它不解析任何面。只在**未命中**时写：命中时这份
        // 文件已随记录抵达，重新推导它正是投影存在的意义所要避免的那次解析。它是载荷，因此通用的发布/复制
        // 路径会与另外五份一起把它带给这棵树的每一个包。
        if record.is_none()
            && let Err(error) =
                super::discovery_record::write_faces_manifest(layout, &cache_units, out_dir)
        {
            write_errors.push(error);
        }
        phases.mark("graph");
        // The crate-shape declaration is a build input like any face: a shape that does not hold
        // together fails the build here, before anything is published, and its lock is written the same
        // way. A host without a declaration is not an error — it is the one-crate package every host was
        // before this file existed.
        // crate 形状声明与任何注册面一样是构建输入：不成立的形状在这里、在发布任何东西之前让构建失败，而它的锁
        // 以同样的方式写下。没有声明的宿主不是错误——它就是这份文件存在之前每个宿主的样子：一个 crate 的包。
        if compile_errors.is_empty()
            && let Some(shape) = &shape
        {
            // The declaration is validated first (does every claim name a subtree?), then planned
            // (can every claimed subtree become a crate that compiles?). Refusing a declaration this
            // build cannot turn into a working crate is the difference between a named refusal now
            // and a subtree nobody compiles, which rustc reports later as `cannot find 'object' in
            // 'control'` (audit `M7`, P3.2).
            // 先校验声明（每条认领都点名了一棵子树吗？），再规划它（每条认领的子树都能变成一个编译得过的
            // crate 吗？）。拒绝一份本次构建变不成可用 crate 的声明，就是"现在得到一句点名拒绝"与"留下一棵
            // 没人编译的子树、以后由 rustc 报 `cannot find 'object' in 'control'`"之间的差别（审计 `M7`，P3.2）。
            // The plan was already made before the render (it produced the mount spellings); here the
            // declaration's claims are checked against the rows, which is the half that needs them.
            // 规划已在渲染之前做过（它产出了挂载拼写）；这里拿行对账声明里的认领，那是需要行的那一半。
            if let Err(refusal) = super::shape_decl::check_shape(shape, out_dir, &pruning_rows) {
                compile_errors.push(BuildDiagnostic::new("add-crates", refusal));
            }
        }
        if compile_errors.is_empty() && write_errors.is_empty() {
            // Publish this tree's record for the next package that builds it. Written from the products
            // that just landed, with its own fingerprint line **last** — the same discipline the output's
            // fingerprint follows, one level up. A failure here is reported like any other write failure:
            // the products are the answer, the record is only what the next run saves.
            // 为下一个构建这棵树的包发布记录。它由刚落地的产物写出，自己的指纹行**最后**写——与输出那份指纹同一
            // 条纪律，只是高一层。这里失败按任何写入失败上报：产物才是答案，记录只是下一次运行省下的东西。
            if let Err(error) = super::discovery_record::publish(
                manifest,
                &discovery_fingerprint,
                &super::registry_identity::package_namespace(),
                out_dir,
            ) {
                write_errors.push(error);
            }
            phases.mark("record");
            if let Err(error) = write_if_changed(
                &out_dir.join("discovery.fingerprint"),
                &discovery_fingerprint,
            ) {
                write_errors.push(error);
            }
            // The readiness record is the **last** thing a clean run writes (audit `M7`, P2.1): a reader
            // that finds it knows this run finished, and one that does not knows the previous run (or
            // none) is what is on disk. It is written after the fingerprint for the same reason the
            // fingerprint is written last among the payloads.
            // 就绪记录是干净的一次运行写的**最后**一样东西（审计 `M7`，P2.1）：找到它的读者知道这次运行完成了，
            // 而找不到的读者知道磁盘上是上一次（或者没有）。它排在指纹之后，理由与指纹排在载荷之后相同。
            if let Some(header) = &graph
                && let Err(error) = write_generation(out_dir, &layout.package_root, header)
            {
                write_errors.push(error);
            }
        } else {
            // A token that cannot be removed still claims this output describes the
            // current sources, so the failure is reported rather than discarded: the
            // previous `let _ =` left exactly that claim standing.
            // 删不掉的凭据仍然声称这份产物描述的是当前源码，因此这次失败被报出而不是被丢弃：过去的
            // `let _ =` 恰恰让那句话继续成立。
            match std::fs::remove_file(out_dir.join("discovery.fingerprint")) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => write_errors.push(format!(
                    "cannot remove {}: {error}",
                    out_dir.join("discovery.fingerprint").display()
                )),
            }
        }
    } else {
        // Nothing was published, and the diagnostic above says which lock to clear.
        // 什么都没发布，上面那条诊断说明了该清哪把锁。
    }
    phases.mark("write");
    if !write_errors.is_empty() {
        let message = format!(
            "xirang could not write its generated tree: {}",
            write_errors.join("; ")
        );
        if input.emit_cargo_directives {
            // The build-script path has nowhere to render this: the generated tree
            // is what would carry it. A build script fails by panicking, which is
            // how a missing `OUT_DIR` write has always failed here.
            // 构建脚本路径没有地方渲染它：生成树正是本该承载它的东西。构建脚本以 panic
            // 失败，这也是此处 `OUT_DIR` 写不了时一向的失败方式。
            panic!("{message}");
        }
        for error in write_errors {
            compile_errors.push(BuildDiagnostic::new("out-dir", error));
        }
    }
    if input.emit_cargo_directives {
        emit_rerun_paths(scan, &nodes);
        // The demo directory's diagnostics are gated behind this feature, so a host
        // that declares it would otherwise be told the cfg is unexpected. rustc's
        // `check-cfg` takes the value-list form for a feature name; the name comes
        // from the same constant the renderer writes into the generated tree.
        // 演示目录的诊断挂在这个特性之后，因此声明了它的宿主否则会被通知该 cfg 是意外的。对特性名，
        // rustc 的 `check-cfg` 采用值列表形式；这个名字取自渲染器写进生成树的同一个常量。
        println!(
            "cargo::rustc-check-cfg=cfg(feature, values({:?}))",
            crate::build_method::DEMO_ONLY_FEATURE
        );
        println!("cargo:rerun-if-env-changed={}", lexicon::SCOPE_ENV);
        println!("cargo:rerun-if-env-changed={}", lexicon::ENTRY_ENV);
        println!("cargo:rerun-if-env-changed={}", lexicon::BUILD_VERBOSE_ENV);
        println!(
            "cargo:rerun-if-changed={}",
            manifest.join("Cargo.toml").display()
        );
        // A plan file only feeds the cross-check above, so a new or edited plan
        // has to bring the build back or its diagnostic would never appear.
        // 计划文件只喂给上面那条交叉校验，因此新增或改动计划必须让构建重跑，否则它的
        // 诊断永远不会出现。
        println!(
            "cargo:rerun-if-changed={}",
            manifest
                .join(lexicon::XIRANG_DIR)
                .join(lexicon::EXTERNAL_GRAFT_DIR)
                .display()
        );
    }
    // The per-phase reading lands here, after every phase has run and whether or not the run
    // succeeded: a failing build is exactly when "where did the time go" gets asked.
    // 逐相位读数落在这里：所有相位都跑完之后，而且无论这次运行成功与否——失败的构建正是会问
    // "时间花在哪了"的时候。
    if input.emit_cargo_directives && build_output_is_verbose() {
        println!("cargo:warning={}", phases.line());
    }
    if compile_errors.is_empty() {
        None
    } else {
        Some(compile_errors)
    }
}

/// Wall time per pipeline phase, for the one verbose line that reports them.
/// 管线各相位的墙钟耗时，供那一条 verbose 行报告。
///
/// It exists because "the build script is 73% of the wall" is an aggregate, and the next change
/// would be a guess without a per-phase reading. Measured on a 1001-face host, these phases
/// separate `discover` + `declare` (the whole tree read and parsed by several passes) from
/// `render` and `write`, which are the only parts a partition turns per-package.
/// 它存在的理由是："构建脚本占墙 73%"是个聚合数，没有逐相位读数，下一步改动就是猜。在一个 1001 面的宿主上
/// 实测，这些相位把 `discover` ＋ `declare`（整棵树被若干遍读取与解析）与 `render`、`write` 分开——后者是
/// 分片唯一按包计的那两部分。
struct PhaseLog {
    total: Instant,
    last: Instant,
    marks: Vec<(&'static str, Duration)>,
}

impl PhaseLog {
    /// A log whose clock starts now — one per `run`, named for that run rather than for the verb
    /// (`start` is a bare verb, which this workspace's `verb_table` gate refuses outside entry
    /// positions).
    /// 时钟从此刻开始的一份记录——每次 `run` 一份；名字按"为这次运行"取，而不是按动词（`start`
    /// 是裸动词，本工作区的 `verb_table` 门禁不允许它出现在入口位以外）。
    fn for_run() -> Self {
        let now = Instant::now();
        Self {
            total: now,
            last: now,
            marks: Vec::new(),
        }
    }

    fn mark(&mut self, name: &'static str) {
        let now = Instant::now();
        self.marks.push((name, now.duration_since(self.last)));
        self.last = now;
    }

    fn line(&self) -> String {
        phase_line(&self.marks, self.total.elapsed())
    }
}

/// Keep the generated diagnostic stream deterministic and compact.
/// 保持生成的诊断流稳定且紧凑。
/// Report a source layout the build cannot read, the way both callers need it.
/// 以两种调用方各自需要的方式报告构建读不了的源码布局。
///
/// A build script has no stdout contract to keep and no generated tree to carry
/// the message — the tree is what would have carried it — so it fails loudly with
/// the rendered diagnostic, the way the write path below fails. What it replaces is
/// a bare `expect("src directory must exist")` that named neither the tree nor the
/// reason.
/// 构建脚本没有 stdout 契约要守，也没有生成树可以承载这条消息——生成树正是本该承载它的东西——
/// 因此它带着渲染后的诊断响亮失败，与下面的写出路径一致。它取代的是一条既没说清哪棵树、也没说清
/// 原因的裸 `expect("src directory must exist")`。
fn layout_diagnostic(input: &BuildInput, message: String) -> Option<BuildDiagnostics> {
    let mut diagnostics = BuildDiagnostics::default();
    diagnostics.push(BuildDiagnostic::new("face-layout", message));
    if input.emit_cargo_directives {
        panic!("{}", diagnostics.render_build_diagnostics());
    }
    Some(diagnostics)
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;
