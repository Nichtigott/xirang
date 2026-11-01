//! Reading the host entry and exposing the graft declarations it carries.
//! 读取宿主入口并暴露它携带的 graft 声明。
//!
//! The build reads the entry once to capture the static plan; an authoring
//! surface reads the same entry to tell an author whether a slot is shipped.
//! Both read one conversion from the parsed syntax, defined here.
//! 构建读取入口一次以捕获静态计划；创作界面读取同一个入口，以告诉作者某个槽位是否
//! 被发布。两者都从已解析语法经同一份转换取得，该转换定义于此。

use std::fs;
use std::path::Path;

use super::declared::{DeclaredGraft, DeclaredGraftExpressions, DeclaredGrafts};
use crate::build_method::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use crate::build_method::registry_syntax::{GraftSyntax, graft_entries};
use crate::build_method::{HostEntry, host_entry_source};

/// The entry's graft declarations, and the subset this build enables.
/// 入口里的 graft 声明，以及本次构建启用的那一部分。
///
/// Two views of one parse, because the build needs both and they answer different
/// questions. `enabled` fills the cut table: a slot whose `#[cfg]` is off cannot be
/// cut in this build. `declared` answers "could any declaration ever name this
/// slot?", which is what the plan cross-check asks — and a gated declaration must
/// count there, or a record for a slot that this feature set compiles out would be
/// reported as an authoring mistake. Splitting the list here (rather than filtering
/// once and letting the checker guess) is what keeps those two answers from being
/// the same answer.
/// 一次解析的两个视图，因为构建需要两者、而它们回答不同的问题。`enabled` 填充切口表：
/// `#[cfg]` 关掉的槽位在本次构建里不可能被切开。`declared` 回答"是否存在任何声明可能
/// 命名这个槽位"，也就是计划交叉校验要问的问题——门控声明在那里必须算数，否则"本次
/// 特性组合把该槽位编译掉了"的记录会被报成作者失误。在这里把列表分开（而不是只过滤
/// 一次、让校验器去猜）正是让这两个答案不会变成同一个答案的原因。
pub(crate) struct HostGraftEntries {
    /// Every declaration in the entry, including the gated-off ones.
    /// 入口里的每一条声明，包括被门控关掉的。
    pub(crate) declared: Vec<GraftSyntax>,
    /// The declarations whose `#[cfg]` this build evaluates to true.
    /// 本次构建把 `#[cfg]` 求值为真的那些声明。
    pub(crate) enabled: Vec<GraftSyntax>,
}

/// Read the graft declarations carried by the entry the build resolved.
/// 读取构建解析出的入口所携带的 graft 声明。
///
/// The entry is a parameter, not something this function resolves for itself.
/// Resolving it here is what let the build disagree with itself:
/// `XIRANG_ENTRY` used to reach pruning while this reader silently took
/// `application!`/`main.rs`, so the generated `BUILTIN_GRAFT_CUTS` and the
/// `graft_plan.tsv` audit text described a different file than the one the
/// release pruned — a slot declared only in the configured file never reached
/// the runtime, and a cut kept from the other file could outlive a pruning
/// decision that never saw it. `pipeline` resolves the entry once and hands the
/// same value here and to `SourceScope`.
/// 入口是参数，而不是本函数自行解析的东西。在这里自行解析正是构建自相矛盾的原因：
/// `XIRANG_ENTRY` 过去只作用于剪枝，而这个读取者静默取 `application!`/`main.rs`，
/// 于是生成的 `BUILTIN_GRAFT_CUTS` 与 `graft_plan.tsv` 审计文本描述的文件，和发布态
/// 实际剪枝依据的文件不是同一个——只在被指定文件里声明的槽位永远到不了运行期，而从
/// 另一个文件保留下来的切口则可能活过一次根本没看到它的剪枝决策。现在 `pipeline`
/// 解析一次入口，把同一个值交给这里和 `SourceScope`。
pub(crate) fn host_graft_entries(
    entry: &HostEntry,
    errors: &mut BuildDiagnostics,
) -> HostGraftEntries {
    let declared = read_graft_entries(entry.path(), entry.is_required(), errors);
    enable_graft_entries(declared, entry.path(), errors)
}

/// Split one declaration list into what the author declared and what this feature set enables.
/// 把一份声明清单拆成"作者声明的"与"本次特性组合启用的"。
///
/// The gate rule is one implementation because two callers reach it: the build reads the host entry
/// itself, and a **generated package** receives its host's declarations as data — the release shape
/// carries only the sources it compiles, so it has no entry to read. Both must evaluate `#[cfg]` the
/// same way, or a slot that is gated off in the host would be live in the ghost that compiles it.
/// 门控规则只有一份实现，因为有两个调用方到达它：构建自己读宿主入口，而**生成包**把宿主的声明当作数据
/// 收下——发布形状只携带它编译的那些源码，因此没有入口可读。两者必须以同一方式求值 `#[cfg]`，否则在
/// 宿主里被门控关掉的槽位，会在编译它的幽灵里活着。
pub(crate) fn enable_graft_entries(
    declared: Vec<GraftSyntax>,
    entry_path: &Path,
    errors: &mut BuildDiagnostics,
) -> HostGraftEntries {
    // The same gate rule the static plan follows: a feature the build can see
    // decides, and a gate it cannot evaluate is refused rather than guessed.
    // Refused *with a diagnostic*: this used to panic, which took `check --json`
    // down with an exit code of 101 and an empty stdout instead of the document the
    // CLI promises — the same class C2 fixed for faces, in the one place that was
    // left. The message matches the one `scope` already pushes for a malformed
    // entry, so the two collapse into one line instead of repeating.
    // 与静态计划同一条规则：构建看得见的特性说了算，无法求值的门控一律拒绝而非猜测。拒绝方式是
    // **发诊断**：这里过去会 panic，使 `check --json` 以 101 退出、stdout 为空，而不是给出 CLI
    // 承诺的文档——与 C2 为注册面修掉的是同一类，只是这一处被漏了。消息与 `scope` 为畸形入口
    // 已推的那条一致，因此两者会折叠成一行而不是重复。
    let mut enabled = Vec::new();
    for declaration in declared.iter() {
        let Some(cfg) = declaration.cfg.as_deref() else {
            enabled.push(declaration.clone());
            continue;
        };
        match crate::build_method::static_plan::face_cfg_enabled(
            cfg,
            &crate::build_method::static_plan::feature_enabled,
        ) {
            Ok(true) => enabled.push(declaration.clone()),
            Ok(false) => {}
            Err(message) => errors.push(BuildDiagnostic::new(
                "graft-entry",
                format!("{message} in `{}`", entry_path.display()),
            )),
        }
    }
    HostGraftEntries { declared, enabled }
}

/// Read the declarations in one entry file, with the absence rule made explicit.
/// 读取一个入口文件里的声明，并把“可否缺失”显式化。
///
/// `required` is the whole difference between a host-named entry and Cargo's
/// convention: a host that named the file gets a build failure when it cannot be
/// read, while a package with neither `src/main.rs` nor `src/lib.rs` is a host
/// without an entry, which is not an error. The boundary is pinned by
/// `an_absent_convention_entry_is_no_entry_not_a_failure`.
/// `required` 正是“宿主指定的入口”与“Cargo 约定”之间的全部差别：自己指定了文件的
/// 宿主读不出来就是构建失败，而既无 `src/main.rs` 也无 `src/lib.rs` 的包只是没有入口
/// 的宿主，不算错误。该边界由 `an_absent_convention_entry_is_no_entry_not_a_failure`
/// 钉住。
fn read_graft_entries(
    entry_path: &Path,
    required: bool,
    errors: &mut BuildDiagnostics,
) -> Vec<GraftSyntax> {
    let Ok(source) = fs::read_to_string(entry_path) else {
        if required {
            errors.push(BuildDiagnostic::new(
                "graft-entry",
                format!(
                    "failed to read graft entry source `{}`",
                    entry_path.display()
                ),
            ));
        }
        return Vec::new();
    };
    match graft_entries(&source) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(BuildDiagnostic::new(
                "graft-entry",
                format!(
                    "invalid graft declaration in `{}`: {error}",
                    entry_path.display()
                ),
            ));
            Vec::new()
        }
    }
}

/// Read the host entry and return the graft declarations it carries.
/// 读取宿主入口并返回它携带的 graft 声明。
///
/// Feature gates are reported, not evaluated: the authoring surface shows the
/// gate so the author can see that a feature decides, and the build remains the
/// only place that follows it.
/// 特性门控只上报、不求值：创作界面显示门控让作者知道"由特性决定"，而跟随门控
/// 始终只发生在构建里。
pub fn declared_grafts(root: &Path) -> Result<DeclaredGrafts, String> {
    let entry = host_entry_source(root)?;
    let source = fs::read_to_string(&entry)
        .map_err(|error| format!("cannot read host entry {}: {error}", entry.display()))?;
    let entries = graft_entries(&source)
        .map_err(|error| format!("invalid graft declaration in {}: {error}", entry.display()))?;
    Ok(DeclaredGrafts {
        entry,
        cuts: entries.iter().map(declared_graft_view).collect(),
    })
}

/// The authoring view of one declaration the build read.
/// 构建读到的某条声明的创作视图。
///
/// The build's static plan and every authoring surface read one declaration, so
/// this conversion lives here once instead of being spelled out at each call
/// site.
/// 构建的静态计划与每个创作界面读的是同一条声明，因此这份转换只在这里写一次，
/// 而不是在每个调用点各写一遍。
pub(crate) fn declared_graft_view(entry: &GraftSyntax) -> DeclaredGraft {
    DeclaredGraft {
        cut: entry.cut.clone(),
        cut_end: entry.cut_end.clone(),
        graft: entry.graft.clone(),
        full: entry.full,
        cfg: entry.cfg.clone(),
        expressions: entry
            .expressions
            .as_ref()
            .map(|expressions| DeclaredGraftExpressions {
                cut: expressions.cut.clone(),
                cut_end: expressions.cut_end.clone(),
                graft: expressions.graft.clone(),
            }),
        line: entry.location.line,
    }
}

#[cfg(test)]
mod tests {
    use super::{declared_grafts, host_graft_entries};
    use crate::build_method::diagnostics::BuildDiagnostics;
    use crate::build_method::host_entry_source;
    use crate::build_method::source_walk::discover_root;
    use std::fs;
    use std::path::PathBuf;

    /// A throwaway package root for the entry-resolution fixtures.
    /// 入口解析夹具使用的临时包根。
    fn entry_fixture(name: &str) -> PathBuf {
        // A counter keeps two fixtures apart even when the clock is too coarse
        // to tell their two `now()` calls apart.
        // 计数器让两个夹具彼此分开，即使时钟分不清它们两次 `now()` 的先后。
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-{name}-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("src")).expect("fixture dir");
        root
    }

    /// The authoring query and the build capture must see the same cuts.
    /// 创作查询与构建捕获必须看到同一批切口。
    #[test]
    fn the_authoring_query_reads_the_same_declarations_the_build_captures() {
        let root = entry_fixture("declared-grafts");
        let src = root.join("src");
        std::fs::write(
            src.join("lib.rs"),
            "crate::host!();\n\
             crate::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut \"root/control/button\" graft \"button_fast\",\n\
                 cut(crate::control::object::slider::NODE_ID) full graft(crate::external::slider_fast::NODE_ID),\n\
             );\n",
        )
        .expect("host entry");

        assert_eq!(
            host_entry_source(&root).expect("entry resolves"),
            src.join("lib.rs")
        );
        let declared = declared_grafts(&root).expect("declarations parse");
        assert_eq!(declared.entry, src.join("lib.rs"));
        assert_eq!(declared.cuts.len(), 2);
        assert_eq!(declared.cuts[0].cut, "root/control/button");
        assert_eq!(declared.cuts[0].graft, "button_fast");
        assert!(!declared.cuts[0].full);
        assert_eq!(declared.cuts[0].expressions, None);
        assert_eq!(
            declared.cuts[1].expressions,
            Some(super::DeclaredGraftExpressions {
                cut: "crate::control::object::slider::NODE_ID".to_owned(),
                cut_end: None,
                graft: "crate::external::slider_fast::NODE_ID".to_owned(),
            })
        );
        assert!(declared.cuts[1].full);

        // The build capture is the same declaration set.
        let nodes = discover_root(&src);
        let entry = crate::build_method::entry::resolve_host_entry(&src, &nodes, None);
        let captured = host_graft_entries(&entry, &mut BuildDiagnostics::default());
        assert_eq!(captured.declared.len(), declared.cuts.len());
        assert_eq!(captured.enabled.len(), declared.cuts.len());
        for (captured, declared) in captured.enabled.iter().zip(&declared.cuts) {
            assert_eq!(captured.cut, declared.cut);
            assert_eq!(captured.cut_end, declared.cut_end);
            assert_eq!(captured.graft, declared.graft);
            assert_eq!(captured.full, declared.full);
            assert_eq!(captured.location.line, declared.line);
        }

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A package with no entry at all is a host without an entry, not a failure.
    /// 完全没有入口的包只是没有入口的宿主，不是失败。
    ///
    /// Cargo's convention is the only resolution allowed to name a file that does
    /// not exist, so whether an absent entry is tolerable is a property of the
    /// resolution, not of this reader. Every other variant reaches the reader with
    /// `is_required() == true` and fails loudly on an unreadable file.
    /// 只有 Cargo 约定可以指向不存在的文件，因此“入口缺失可否容忍”是解析结果的属性，
    /// 而不是本读取者的属性。其他变体都以 `is_required() == true` 到达读取者，读不出来
    /// 就响亮失败。
    #[test]
    fn an_absent_convention_entry_is_no_entry_not_a_failure() {
        let root = entry_fixture("declared-grafts-no-entry");
        let src = root.join("src");
        let nodes = discover_root(&src);
        let entry = crate::build_method::entry::resolve_host_entry(&src, &nodes, None);
        assert!(
            matches!(entry, crate::build_method::HostEntry::Convention(_)),
            "{entry:?}"
        );

        assert!(
            host_graft_entries(&entry, &mut BuildDiagnostics::default())
                .enabled
                .is_empty()
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A feature gate is reported for the author to see, not evaluated here.
    /// 特性门控只上报给作者看，不在这里求值。
    #[test]
    fn a_gated_declaration_reports_its_gate() {
        let root = entry_fixture("declared-grafts-gated");
        std::fs::write(
            root.join("src/lib.rs"),
            "crate::host!();\n\
             #[cfg(feature = \"extra\")]\n\
             crate::static_graft_plan!(FRAMEWORK, cut \"root/a\" graft \"a_fast\");\n",
        )
        .expect("host entry");

        let declared = declared_grafts(&root).expect("declarations parse");
        assert_eq!(declared.cuts.len(), 1);
        assert_eq!(declared.cuts[0].cfg.as_deref(), Some("feature = \"extra\""));
        assert_eq!(declared.cuts[0].line, 3);

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// `application!` names the entry, and a missing one is an error, not a panic.
    /// `application!` 指定入口；入口缺失是错误而非 panic。
    #[test]
    fn the_authoring_query_follows_application_and_reports_a_missing_entry() {
        let root = entry_fixture("declared-grafts-application");
        let src = root.join("src");
        std::fs::write(src.join("lib.rs"), "crate::host!();\n").expect("host entry");
        std::fs::write(
            src.join("app.rs"),
            "crate::application!(entry = crate::app::run);\n",
        )
        .expect("application declaration");
        assert_eq!(
            host_entry_source(&root).expect("entry resolves"),
            src.join("app.rs")
        );

        std::fs::remove_file(src.join("app.rs")).expect("drop declaration");
        std::fs::remove_file(src.join("lib.rs")).expect("drop entry");
        assert!(host_entry_source(&root).is_err());
        assert!(declared_grafts(&root).is_err());
        assert!(host_entry_source(&root.join("missing")).is_err());

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A malformed graft declaration is a diagnostic, not a panic.
    /// 畸形的 graft 声明是一条诊断，而不是 panic。
    ///
    /// This used to `panic!`, which took `check --json` down with an exit code of
    /// 101 and an empty stdout — while `scope` had already produced the same
    /// diagnostic that the panic then discarded.
    /// 这里过去会 `panic!`，让 `check --json` 以 101 退出、stdout 为空——而 `scope` 其实已经
    /// 产出了同一条诊断，随后被 panic 丢掉。
    #[test]
    fn a_malformed_graft_declaration_is_reported_not_fatal() {
        let root = entry_fixture("malformed-graft");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("src");
        fs::write(
            src.join("lib.rs"),
            "crate::host!();\n\
             crate::static_graft_plan!(FRAMEWORK, cut);\n",
        )
        .expect("host entry");
        let nodes = discover_root(&src);
        let entry = crate::build_method::entry::resolve_host_entry(&src, &nodes, None);

        let mut errors = BuildDiagnostics::default();
        let captured = host_graft_entries(&entry, &mut errors);
        assert!(captured.declared.is_empty(), "{:?}", captured.declared);
        assert!(captured.enabled.is_empty());
        assert_eq!(errors.len(), 1, "{}", errors.render_build_diagnostics());
        let rendered = errors.render_build_diagnostics();
        assert!(rendered.contains("phase=graft-entry"), "{rendered}");
        assert!(rendered.contains("invalid graft declaration"), "{rendered}");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A gate this build cannot evaluate is a diagnostic too, not a panic.
    /// 构建无法求值的门控同样是一条诊断，而不是 panic。
    #[test]
    fn an_unevaluable_graft_gate_is_reported_not_fatal() {
        let root = entry_fixture("unevaluable-gate");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("src");
        fs::write(
            src.join("lib.rs"),
            "crate::host!();\n\
             #[cfg(unix)]\n\
             crate::static_graft_plan!(FRAMEWORK, \
             cut(crate::control::NODE_ID) graft(canvas_fast));\n",
        )
        .expect("host entry");
        let nodes = discover_root(&src);
        let entry = crate::build_method::entry::resolve_host_entry(&src, &nodes, None);

        let mut errors = BuildDiagnostics::default();
        let captured = host_graft_entries(&entry, &mut errors);
        // A refused gate keeps nothing enabled and reports why.
        // 被拒的门控不启用任何东西，并说明原因。
        assert!(captured.enabled.is_empty());
        assert_eq!(errors.len(), 1, "{}", errors.render_build_diagnostics());
        let rendered = errors.render_build_diagnostics();
        assert!(rendered.contains("phase=graft-entry"), "{rendered}");
        assert!(rendered.contains("cfg"), "{rendered}");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
