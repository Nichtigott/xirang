//! Source indexing and registry snapshot helpers for Studio.
//! Studio 的源码索引与注册快照辅助逻辑。
//!
//! The pure Rust-source lexer lives in the kernel `source` module; this shim
//! keeps the historical paths and hosts the Studio-specific file-bound helpers.
//! 纯 Rust 源码词法器本体在 kernel 的 `source` 模块；本 shim 保留历史路径，
//! 并承载 Studio 特有的文件绑定辅助逻辑。

// Kernel lexer re-exports. Historical Studio paths stay valid through these.
// kernel 词法器重导出，Studio 历史路径经由它们保持可用。
pub use crate::run_method::source::{body_calls, function_source_range, function_symbols};

use super::project_context::{package_namespace, package_root, with_authoring_context};
use super::*;

/// Compose compiled, external, and newly authored faces into one snapshot.
/// 将已编译、外部和刚落盘的注册面装配成同一个快照。
pub(super) fn load_registry() -> Result<Registry, String> {
    // Studio owns an empty, namespace-isolated root and lets the authoring UI
    // add faces. A host can choose a stable namespace per library through the
    // environment when several libraries share one process.
    let namespace = package_namespace();
    let mut registry = Registry::root_for_namespace(
        crate::run_method::FrameworkId::new("xirang.studio"),
        namespace,
    );
    // Scan only the host package's `src/` tree. When Studio is launched from
    // the XiRang workspace itself there is no host `src/`; show an empty
    // registry instead of mistaking build/debug fixtures for faces.
    // 只扫描宿主包的 `src/`。直接从 XiRang workspace 启动时没有宿主 `src/`，
    // 此时显示空注册树，不要把 build/debug 测试夹具误认成注册面。
    let layout = crate::build_method::source_layout(&package_root())?;
    if !layout.scan_root.is_dir() {
        return Ok(registry);
    }
    let snapshots =
        with_authoring_context(|| crate::run_method::generated_snapshots_in(&layout))
            .map_err(|error| format!("cannot load generated registration faces: {error}"))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("generated registration faces were rejected: {error}"))?;
    Ok(registry)
}

/// Render one admission policy as the compact clause form the Edit form carries.
/// 将一条 admission 策略渲染成 Edit 表单所携带的紧凑子句形式。
///
/// This call site does not spell the grammar: the kernel's single renderer,
/// `crate::run_method::authoring::parse::compact_admission`, owns the spelling
/// (`FIXR-01`). Studio used to assemble the clauses here — one clause per non-empty
/// list, `;` between them, `ANY` for none — because the kernel kept its compact
/// renderer private, and that second copy was a drift risk by construction: it
/// happened to agree byte for byte, so nothing could report the day it stopped
/// agreeing. The widened deny list of `LGC-LG-02` is what such a copy costs when it
/// stops agreeing.
/// 本调用点不拼语法：内核唯一的渲染器
/// `crate::run_method::authoring::parse::compact_admission` 拥有拼法（`FIXR-01`）。Studio
/// 过去在这里装配子句——每张非空列表一个子句、之间用 `;`、都没有则 `ANY`——因为内核当时把紧凑
/// 渲染器设成私有；而那份副本本身就是漂移风险：它恰好逐字节相同，因此它哪天不再相同也没有任何
/// 东西会报告。`LGC-LG-02` 里 deny 列表被放宽，正是这种副本不再相同时的代价。
///
/// Delegating keeps the Edit form's prefill and the inspector row byte-identical to
/// the spelling `parse_admission_owned` reads back — by construction, not by two
/// copies happening to agree. Both surfaces go through this one call site
/// (`studio/src/studio/app/keyboard.rs`, `studio/src/studio/ui/panels.rs`).
/// 改为委派之后，Edit 表单预填与检视器那一行与 `parse_admission_owned` 读回的拼法逐字节一致——
/// 由构造保证，而不是靠两份副本恰好相符。两个面都经过这一个调用点
/// （`studio/src/studio/app/keyboard.rs`、`studio/src/studio/ui/panels.rs`）。
pub(crate) fn admission_text(admission: &crate::run_method::OwnedAdmission) -> String {
    crate::run_method::authoring::parse::compact_admission(admission)
}

/// Render one registry rule as the compact clause form the Edit form carries.
/// 将一条注册规范渲染成 Edit 表单所携带的紧凑子句形式。
///
/// This call site does not spell the grammar: the kernel's single renderer,
/// `crate::run_method::authoring::parse::compact_registration_rule`, owns the
/// spelling (`FIXR-01`'s third member — the admission copies were the first two,
/// `LGC-LG-02`/t8). Studio used to assemble the `preset:/parts:/exports:/handle:/
/// part_trait:` clauses here because the kernel's owned-value renderer was private,
/// and that copy was a drift risk by construction: it happened to agree byte for
/// byte with the kernel, so nothing could report the day it stopped agreeing.
/// 本调用点不拼语法：内核唯一的渲染器
/// `crate::run_method::authoring::parse::compact_registration_rule` 拥有拼法（`FIXR-01`
/// 的第三例——前两例是 admission 的两份副本，`LGC-LG-02`/t8）。Studio 过去在这里装配
/// `preset:/parts:/exports:/handle:/part_trait:` 子句，因为内核“从拥有型取值渲染”的入口是私有
/// 的；而那份副本本身就是漂移风险：它恰好与内核逐字节相同，因此它哪天不再相同也没有任何东西会报告。
///
/// Delegating keeps the Edit form's prefill and the inspector row byte-identical to
/// the spelling `parse_registration_rule_owned` reads back — by construction, not by
/// two copies happening to agree. Both surfaces go through this one call site
/// (`studio/src/studio/app/keyboard.rs`, `studio/src/studio/ui/panels.rs`).
/// 改为委派之后，Edit 表单预填与检视器那一行与 `parse_registration_rule_owned` 读回的拼法逐字节
/// 一致——由构造保证，而不是靠两份副本恰好相符。两个面都经过这一个调用点
/// （`studio/src/studio/app/keyboard.rs`、`studio/src/studio/ui/panels.rs`）。
pub(crate) fn registration_rule_text(rule: &crate::run_method::OwnedRegistrationRule) -> String {
    crate::run_method::authoring::parse::compact_registration_rule(rule)
}

/// Locate the 1-based declaration line of a function in a registry source file.
/// 在注册面源码文件中定位函数声明的 1 起始行号。
pub(crate) fn function_line(file: &str, function: &str) -> Option<u32> {
    let text = std::fs::read_to_string(source_path_for(file)).ok()?;
    function_symbols(&text)
        .into_iter()
        .find(|item| item.name == function)
        .map(|item| item.line)
}

#[cfg(test)]
pub(super) fn function_bodies(source: &str) -> Vec<(String, String)> {
    function_symbols(source)
        .into_iter()
        .map(|function| (function.name, function.body))
        .collect()
}

#[cfg(test)]
mod admission_text_tests {
    //! Studio's compact admission rendering is the kernel's renderer, reached
    //! through one call site (audit `LGC-LG-02`, `FIXR-01`).
    //! Studio 的紧凑 admission 渲染就是内核的渲染器，只有一个调用点（审计 `LGC-LG-02`、
    //! `FIXR-01`）。

    use super::*;
    use crate::run_method::authoring::parse::{compact_admission, parse_admission_owned};

    /// One policy in the owned form both sides take, from path spellings.
    /// 一份策略的拥有型形式，由路径拼法构造——两边取用的都是它。
    fn admission(allowed: &[&str], denied: &[&str]) -> crate::run_method::OwnedAdmission {
        let owned = |paths: &[&str]| paths.iter().map(|path| (*path).to_owned()).collect();
        crate::run_method::OwnedAdmission {
            allowed_paths: owned(allowed),
            denied_paths: owned(denied),
        }
    }

    /// A policy that names both lists must render both: the deny list is the veto
    /// `Admission::accepts` prioritises, so a rendering that keeps only the allow
    /// list widens the gate the reader sees and the next save writes back.
    /// 同时点名两张列表的策略必须把两张都渲染出来：deny 是 `Admission::accepts` 优先的否决权，
    /// 只保留 allow 列表的渲染会放宽读者看到的、以及下次保存写回的那道门禁。
    #[test]
    fn both_lists_survive_the_compact_rendering() {
        let policy = admission(&["ui"], &["ui/experimental"]);
        let text = admission_text(&policy);
        assert_eq!(text, "allow:ui;deny:ui/experimental", "{text}");
        let read = parse_admission_owned(&text).expect("the kernel reads what this renderer emits");
        assert_eq!(read.allowed_paths, ["ui"], "{read:?}");
        assert_eq!(read.denied_paths, ["ui/experimental"], "{read:?}");
    }

    /// The call site renders exactly what the kernel's renderer renders, and the
    /// kernel reads back the value it emits.
    /// 调用点渲染出的正是内核渲染器渲染的值，且内核读得回它吐出的那份。
    ///
    /// This replaces the old "two implementations are byte-identical" pin. A value
    /// can differ between the two sides only when the call site spells the grammar
    /// itself, so a reintroduced copy — or a kernel renderer that moved while a
    /// Studio copy stayed — fails here. The matrix names every shape the two could
    /// disagree on (no list, either list alone, both, several paths), and the round
    /// trip adds the half Studio actually depends on: the prefill it shows is a
    /// value `parse_admission_owned` reads back as the same policy.
    /// 这是取代"两份实现逐字节相同"的钉子。两边的值只会在调用点自己拼语法时不同，因此重新引入的
    /// 副本——或内核渲染器改了而 Studio 副本没跟着改——会在这里失败。矩阵点名了可能分歧的每一种
    /// 形状（两张列表都没有、只有其中一张、两张都有、多路径）；往返则补上 Studio 真正依赖的另一半：
    /// 它显示的预填值，`parse_admission_owned` 读回的就是同一份策略。
    #[test]
    fn the_call_site_renders_what_the_kernel_renders() {
        for policy in [
            admission(&[], &[]),
            admission(&["ui"], &[]),
            admission(&[], &["ui/experimental"]),
            admission(&["ui"], &["ui/experimental"]),
            admission(&["ui", "controls"], &["ui/experimental"]),
        ] {
            let through_studio = admission_text(&policy);
            assert_eq!(
                through_studio,
                compact_admission(&policy),
                "the call site stopped being the kernel's rendering"
            );
            assert_eq!(
                parse_admission_owned(&through_studio).expect("the kernel reads its own spelling"),
                policy,
                "the prefill the Edit form shows does not read back: {through_studio}"
            );
        }
    }

    /// The call site carries no second implementation: its body is one delegation
    /// to the kernel's renderer and assembles no clause of its own.
    /// 调用点不含第二份实现：它的函数体就是一次对内核渲染器的委派，自己不装配任何子句。
    ///
    /// Why this pin is structural rather than behavioural: Studio's copy happened to
    /// agree with the kernel byte for byte on every parseable policy (the widening
    /// had already been fixed there), so no behavioural test could see which of the
    /// two ran — that agreement is exactly what made the second copy silent drift
    /// waiting to happen. Only the code can answer which one runs, and the code is
    /// reachable from a test: `include_str!` is the same-file shape
    /// `run_method/src/macros/face.rs` uses for its field mirror.
    /// 为什么这条钉子看结构而不是行为：Studio 的副本在每一份可解析策略上都恰好与内核逐字节相同
    /// （放宽问题在那里已被修好），因此没有行为测试能看出跑的是哪一份——而这份"恰好相同"正是第二份
    /// 副本会变成静默漂移的原因。哪一份在跑只有代码能回答，而代码对测试是可达的：`include_str!` 正是
    /// `run_method/src/macros/face.rs` 为字段镜像使用的同文件形态。
    #[test]
    fn the_call_site_carries_no_second_compact_renderer() {
        let source = include_str!("source_index.rs");
        // Built at runtime so the needles cannot match this test's own source.
        // 在运行时拼接，避免 needle 匹配到本测试自身的源码。
        let head = ["pub(crate) ", "fn admission", "_text"].concat();
        let start = source.find(&head).expect("the admission_text definition");
        let body = &source[start..];
        let end = body
            .find("\n}")
            .expect("the definition closes at column zero")
            + start;
        let body = &source[start..end];

        assert!(
            body.contains(&["compact", "_admission"].concat()),
            "the call site no longer calls the kernel's renderer:\n{body}"
        );
        for needle in [
            ["format", "!"].concat(),
            [".join", "("].concat(),
            ["debug_", "assert"].concat(),
            ["\"allow", ":"].concat(),
            ["\"deny", ":"].concat(),
            ["\"ANY", "\""].concat(),
        ] {
            assert!(
                !body.contains(&needle),
                "the call site spells the compact grammar itself (`{needle}`), which is the \
                 second implementation `FIXR-01` removed:\n{body}"
            );
        }
    }
}

#[cfg(test)]
mod registration_rule_text_tests {
    //! Studio's compact registration-rule rendering is the kernel's renderer,
    //! reached through one call site (`FIXR-01`, the family's third member: the two
    //! admission copies were `LGC-LG-02`/t8).
    //! Studio 的紧凑注册规范渲染就是内核的渲染器，只有一个调用点（`FIXR-01`，该家族第三例：
    //! 前两例是两份 admission 副本，即 `LGC-LG-02`/t8）。

    use super::*;
    use crate::run_method::authoring::parse::{
        compact_registration_rule, parse_registration_rule_owned,
    };

    /// One rule in the owned form both sides take, from field spellings.
    /// 一份规则的拥有型形式，由各字段拼法构造——两边取用的都是它。
    fn rule(
        preset: Option<&str>,
        parts: &[&str],
        exports: &[&str],
        handle_traits: &[&str],
        part_traits: &[&str],
    ) -> crate::run_method::OwnedRegistrationRule {
        let owned = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        crate::run_method::OwnedRegistrationRule {
            required_preset: preset.map(str::to_owned),
            required_parts: owned(parts),
            required_exports: owned(exports),
            required_handle_traits: owned(handle_traits),
            required_part_traits: owned(part_traits),
        }
    }

    /// The clauses keep their historical order and spelling: this value is what the
    /// Edit form shows and what the next save writes back.
    /// 各子句保持历史顺序与拼法：这个值正是 Edit 表单展示、并由下次保存写回的那一份。
    #[test]
    fn the_rule_clauses_keep_their_historical_spelling() {
        for (policy, expected) in [
            (rule(None, &[], &[], &[], &[]), "ANY"),
            (
                rule(Some("ActionParts"), &[], &[], &[], &[]),
                "preset:ActionParts",
            ),
            (
                rule(None, &["paint", "shape"], &[], &[], &[]),
                "parts:paint,shape",
            ),
            (
                rule(None, &[], &["control.render"], &[], &[]),
                "exports:control.render",
            ),
            (
                rule(None, &[], &[], &["ControlHandle"], &[]),
                "handle:ControlHandle",
            ),
            (
                rule(None, &[], &[], &[], &["ActionParts"]),
                "part_trait:ActionParts",
            ),
            (
                rule(
                    Some("ActionParts"),
                    &["paint"],
                    &["control.render"],
                    &["ControlHandle"],
                    &["ActionParts"],
                ),
                "preset:ActionParts;parts:paint;exports:control.render;handle:ControlHandle;part_trait:ActionParts",
            ),
        ] {
            assert_eq!(registration_rule_text(&policy), expected);
        }
    }

    /// The call site renders exactly what the kernel's renderer renders, and the
    /// kernel reads back the value it emits.
    /// 调用点渲染出的正是内核渲染器渲染的值，且内核读得回它吐出的那份。
    ///
    /// This replaces "the two implementations are byte-identical". The two sides can
    /// differ only when the call site spells the grammar itself, so a reintroduced
    /// copy — or a kernel renderer that moved while a Studio copy stayed — fails
    /// here; the matrix names every shape they could disagree on, and the round trip
    /// adds the half Studio depends on: the text the form shows is a rule the kernel
    /// reads back unchanged.
    /// 这是取代"两份实现逐字节相同"的钉子。两边只会在调用点自己拼语法时分歧，因此重新引入的副本
    /// ——或内核渲染器改了而 Studio 副本没跟着改——会在这里失败；矩阵点名了可能分歧的每一种形状，
    /// 往返则补上 Studio 依赖的那一半：表单显示的文本，内核原样读回。
    #[test]
    fn the_call_site_renders_what_the_kernel_renders() {
        for policy in [
            rule(None, &[], &[], &[], &[]),
            rule(Some("ActionParts"), &[], &[], &[], &[]),
            rule(None, &["paint"], &[], &[], &[]),
            rule(None, &[], &["control.render"], &[], &[]),
            rule(None, &[], &[], &["ControlHandle"], &[]),
            rule(None, &[], &[], &[], &["ActionParts"]),
            rule(
                Some("ActionParts"),
                &["paint", "shape"],
                &["control.render"],
                &["ControlHandle"],
                &["ActionParts"],
            ),
        ] {
            let through_studio = registration_rule_text(&policy);
            assert_eq!(
                through_studio,
                compact_registration_rule(&policy),
                "the call site stopped being the kernel's rendering"
            );
            assert_eq!(
                parse_registration_rule_owned(&through_studio)
                    .expect("the kernel reads its own spelling"),
                policy,
                "the form's text does not read back: {through_studio}"
            );
        }
    }

    /// The call site carries no second implementation: its body is one delegation
    /// to the kernel's renderer and assembles no clause of its own.
    /// 调用点不含第二份实现：它的函数体就是一次对内核渲染器的委派，自己不装配任何子句。
    ///
    /// Why this pin is structural rather than behavioural: Studio's copy assembled
    /// exactly the kernel's bytes on every input (the same historical order and
    /// keys), so no behavioural test could see which of the two ran — that agreement
    /// is what made the second copy silent drift waiting to happen. Only the code can
    /// answer which one runs, and the code is reachable from a test: `include_str!`
    /// is the same-file shape `run_method/src/macros/face.rs` uses for its field
    /// mirror and `mod admission_text_tests` above uses for the admission copy.
    /// 为什么这条钉子看结构而不是行为：Studio 的副本在每一份输入上都恰好装配出内核的字节（历史
    /// 顺序与键名都相同），因此没有行为测试能看出跑的是哪一份——而这份"恰好相同"正是第二份副本会
    /// 变成静默漂移的原因。哪一份在跑只有代码能回答，而代码对测试是可达的：`include_str!` 正是
    /// `run_method/src/macros/face.rs` 为字段镜像、以及上面 `mod admission_text_tests` 为
    /// admission 副本使用的同文件形态。
    #[test]
    fn the_call_site_carries_no_second_rule_renderer() {
        let source = include_str!("source_index.rs");
        // Built at runtime so the needles cannot match this test's own source.
        // 在运行时拼接，避免 needle 匹配到本测试自身的源码。
        let head = ["pub(crate) ", "fn registration", "_rule_text"].concat();
        let start = source
            .find(&head)
            .expect("the registration_rule_text definition");
        let body = &source[start..];
        let end = body
            .find("\n}")
            .expect("the definition closes at column zero")
            + start;
        let body = &source[start..end];

        assert!(
            body.contains(&["compact", "_registration", "_rule"].concat()),
            "the call site no longer calls the kernel's renderer:\n{body}"
        );
        for needle in [
            ["format", "!"].concat(),
            [".join", "("].concat(),
            ["\"preset", ":"].concat(),
            ["\"parts", ":"].concat(),
            ["\"exports", ":"].concat(),
            ["\"handle", ":"].concat(),
            ["\"part_trait", ":"].concat(),
            ["\"ANY", "\""].concat(),
        ] {
            assert!(
                !body.contains(&needle),
                "the call site spells the compact rule grammar itself (`{needle}`), which is \
                 the second implementation `FIXR-01` removed:\n{body}"
            );
        }
    }
}
