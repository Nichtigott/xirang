//! Pins for the execution identity: what it binds, and what it refuses to leave out.
//! 执行身份的钉子：它绑定了什么，以及它拒绝漏掉什么。

use super::{Environment, ExecutionIdentity, execution_identity};
use serde_json::json;

/// BR05: the fold is only worth anything if it is total. A dimension that is absent must be a loud
/// error, because "this run never recorded a profile" and "this run's profile matched" are the same
/// thing to a digest — and it is the second reading that would merge two different builds.
/// BR05：这个折叠只有**完备**才有价值。缺一个维度必须是响亮的错误，因为对摘要来说"这次运行从没记录过
/// profile"与"这次运行的 profile 相同"是同一件事——而正是后一种读法会合并两次不同的构建。
#[test]
fn a_missing_dimension_is_refused_rather_than_silently_unbound() {
    let complete: Vec<(&str, &str)> = xirang_kernel::maintenance::EXECUTION_DIMENSIONS
        .iter()
        .map(|dimension| (*dimension, "v"))
        .collect();
    assert!(ExecutionIdentity::of(complete.clone()).is_ok());
    for dropped in xirang_kernel::maintenance::EXECUTION_DIMENSIONS {
        let partial = complete
            .iter()
            .filter(|(name, _)| name != dropped)
            .cloned()
            .collect::<Vec<_>>();
        let error = ExecutionIdentity::of(partial).expect_err("a partial identity must be refused");
        assert!(error.contains(dropped), "{dropped}: {error}");
    }
    let unknown = ExecutionIdentity::of([("candidate", "v"), ("surprise", "v")])
        .expect_err("an unknown field is refused too");
    assert!(unknown.contains("not an execution dimension"), "{unknown}");
    let empty = complete
        .iter()
        .map(|(name, _)| (*name, if *name == "profile" { "" } else { "v" }))
        .collect::<Vec<_>>();
    let blank = ExecutionIdentity::of(empty).expect_err("an empty value is not a binding");
    assert!(blank.contains("has no value"), "{blank}");
}

/// BR05: equality is decided by the whole record, and the difference is nameable — that is what lets
/// a reader see *which* dimension separated two runs instead of being told only that they differ.
/// BR05：相等由整条记录决定，而且差异是**可点名**的——这正是读者能看出是哪一个维度分开了两次运行、而不是
/// 只被告知"它们不同"的原因。
#[test]
fn two_identities_differ_only_where_their_dimensions_do() {
    let environment = Environment::observe();
    let root = std::path::Path::new("/tmp/xirang-execution-pin");
    let debug = execution_identity(
        &json!({"workspace":true,"face":"default"}),
        "candidate-a",
        &environment,
        root,
    )
    .unwrap();
    let release = execution_identity(
        &json!({"workspace":true,"face":"default","profile":"release"}),
        "candidate-a",
        &environment,
        root,
    )
    .unwrap();
    let other_bytes = execution_identity(
        &json!({"workspace":true,"face":"default"}),
        "candidate-b",
        &environment,
        root,
    )
    .unwrap();
    assert!(debug.is_the_same_run_as(&debug.clone()));
    assert!(!debug.is_the_same_run_as(&release));
    assert!(!debug.is_the_same_run_as(&other_bytes));
    assert_ne!(debug.digest(), release.digest());
    assert!(
        debug.render_summary().contains("profile=debug"),
        "{}",
        debug.render_summary()
    );
    assert!(release.render_summary().contains("profile=release"));
    assert!(
        release.render_summary().contains("toolchain="),
        "{}",
        release.render_summary()
    );
    assert!(
        debug
            .render_summary()
            .contains("isolation=in-tree (cargo default)"),
        "{}",
        debug.render_summary()
    );
    // A named output directory is a different execution, not a cosmetic difference.
    // 点名了输出目录就是另一次执行，而不是外观上的差别。
    let elsewhere = execution_identity(
        &json!({"workspace":true,"face":"default","target_dir":"/tmp/xirang-execution-pin/target"}),
        "candidate-a",
        &environment,
        root,
    )
    .unwrap();
    assert!(!debug.is_the_same_run_as(&elsewhere));
    assert!(
        elsewhere.render_summary().contains("isolation=target"),
        "{}",
        elsewhere.render_summary()
    );
}
