//! Tests for the feature and target manifest gate.
//! 特性与 target 清单门禁的测试。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// The manifest a well-shaped workspace has: the workspace-only binary carries the
/// feature that keeps it out of an installed copy, and the fixture feature is opt-in.
/// 形状正确的工作区的清单：只在检出内工作的二进制带着把它挡在安装副本之外的特性，而 fixture
/// 特性需要显式打开。
const WELL_SHAPED: &str = "\
[package]
name = \"xirang-toolchain\"
version = \"0.1.6\"

[features]
default = [\"node-graph\"]
node-graph = []
prototype-fixtures = []
dev-supervisor = []

[[bin]]
name = \"xirang-dev\"
path = \"src/bin/xirang-dev.rs\"
required-features = [\"dev-supervisor\"]
";

/// A throwaway workspace whose single member carries `manifest`.
/// 一个一次性工作区，其唯一成员带着 `manifest`。
fn workspace(manifest: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("xirang-features-{}-{sequence}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("toolchain/src")).expect("fixture directory");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"toolchain\"]\nresolver = \"2\"\n",
    )
    .expect("root manifest");
    fs::write(root.join("toolchain/Cargo.toml"), manifest).expect("member manifest");
    root
}

/// The manifests this checkout ships satisfy the rules.
/// 本检出出厂的清单满足这些规则。
#[test]
fn the_shipped_manifests_satisfy_the_feature_rules() {
    let found = findings(&crate::workspace_root());
    assert!(found.is_empty(), "a shipped manifest regressed: {found:#?}");
}

/// The shape this gate exists for: `required-features` deleted from a target that only
/// works inside this checkout. Every other gate stayed silent on exactly that edit.
/// 本门禁为之存在的形状：从一个只在本检出内工作的 target 上删掉 `required-features`。正是那次
/// 编辑让其余门禁全部沉默。
#[test]
fn a_missing_requirement_on_a_workspace_only_target_is_reported() {
    let without = WELL_SHAPED.replace("required-features = [\"dev-supervisor\"]\n", "");
    assert!(
        !without.contains("required-features"),
        "the fixture drops the line"
    );
    let root = workspace(&without);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("`xirang-dev` must carry")),
        "a deleted `required-features` is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A requirement that a typo, or a default feature, made decorative is reported too: the
/// first keeps nothing out because the feature does not exist, the second because Cargo
/// turns it on anyway.
/// 被拼写错误或"默认特性"变成装饰的要求同样被报出：前者因为特性不存在而什么都挡不住，后者因为
/// Cargo 无论如何都会打开它。
#[test]
fn a_decorative_requirement_is_reported() {
    let undeclared = WELL_SHAPED.replace("[\"dev-supervisor\"]", "[\"dev-superviser\"]");
    let root = workspace(&undeclared);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("which the manifest does not declare")),
        "a requirement naming no feature is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);

    let defaulted = WELL_SHAPED.replace("[\"dev-supervisor\"]", "[\"node-graph\"]");
    let root = workspace(&defaulted);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("requires default feature")),
        "a requirement on a default feature is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A feature that gates test-only fixtures must stay off by default: turning it on ships
/// the fixture host inside the published package.
/// 门控测试用 fixture 的特性必须保持默认关闭：打开它会把 fixture 宿主一起发进已发布包。
#[test]
fn a_fixture_feature_in_default_is_reported() {
    let turned_on = WELL_SHAPED.replace(
        "default = [\"node-graph\"]",
        "default = [\"node-graph\", \"prototype-fixtures\"]",
    );
    let root = workspace(&turned_on);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("must stay off by default")),
        "a fixture feature on by default is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A wrapped `default` array is read as one value, so the gate cannot be defeated by
/// reformatting the manifest.
/// 被折行的 `default` 数组作为一个取值读取，因此重新排版清单绕不过门禁。
#[test]
fn a_wrapped_array_is_read_whole() {
    let wrapped = WELL_SHAPED.replace(
        "default = [\"node-graph\"]",
        "default = [\n    \"node-graph\",\n    \"prototype-fixtures\",\n]",
    );
    let root = workspace(&wrapped);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("must stay off by default")),
        "a wrapped array is not a hiding place: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The manifest whose CLI target is gated: the shape a documented install has to respect.
/// 把 CLI target 门控起来的清单：文档里的安装命令必须尊重的形状。
const GATED_CLI: &str = "\
[package]
name = \"xirang-toolchain\"
version = \"0.1.6\"

[features]
default = [\"build\", \"run\"]
build = []
run = []
studio = [\"build\", \"run\"]
mcp = [\"build\", \"run\"]
cli = [\"build\", \"run\", \"studio\", \"mcp\"]
dev-supervisor = []

[[bin]]
name = \"xirang\"
path = \"src/bin/xirang.rs\"
required-features = [\"cli\"]

[[bin]]
name = \"xirang-dev\"
path = \"src/bin/xirang-dev.rs\"
required-features = [\"dev-supervisor\"]
";

/// A throwaway workspace that also documents an install command.
/// 一个同时写了安装命令的一次性工作区。
fn installed_workspace(manifest: &str, document: &str) -> PathBuf {
    let root = workspace(manifest);
    fs::write(root.join("README.md"), document).expect("readme");
    root
}

/// The failure this rule exists for: `cargo install` without `--features` prints a warning
/// about every gated binary, installs none of them, and still exits successfully.
/// 这条规则为之存在的失败：不带 `--features` 的 `cargo install` 为每个门控二进制打印一条警告，
/// 一个都不装，却仍然成功退出。
#[test]
fn a_documented_install_that_installs_nothing_is_reported() {
    let root = installed_workspace(
        GATED_CLI,
        "```sh\ncargo install xirang-toolchain\nxirang new my-app\n```\n",
    );
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("installs no `xirang` binary")),
        "a bare documented install is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The three spellings a working install command takes all reach the same feature set.
/// 一条能装上的安装命令的三种拼法都能到达同一组特性。
#[test]
fn a_documented_install_that_enables_the_cli_is_accepted() {
    for command in [
        "cargo install xirang-toolchain --features cli",
        "cargo install xirang-toolchain --features=cli",
        "cargo install xirang-toolchain --features \"cli,mcp\"",
    ] {
        let root = installed_workspace(GATED_CLI, &format!("```sh\n{command}\n```\n"));
        let found = findings(&root);
        assert!(
            found.is_empty(),
            "`{command}` installs `xirang`: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}

/// A command naming a feature no manifest declares is refused, rather than accepted as
/// "some feature, therefore some binary".
/// 点名了清单里不存在的特性的命令会被拒绝，而不是被当成"有特性，所以有二进制"接受。
#[test]
fn a_documented_install_naming_an_undeclared_feature_is_reported() {
    let root = installed_workspace(
        GATED_CLI,
        "```sh\ncargo install xirang-toolchain --features clii\n```\n",
    );
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("does not declare")),
        "a documented typo is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A command wrapped across two fenced lines is read as one command, because a long install
/// line is written that way.
/// 折在围栏两行里的命令按一条命令读取，因为长安装命令就是这么写的。
#[test]
fn a_wrapped_documented_command_is_read_whole() {
    let root = installed_workspace(
        GATED_CLI,
        "```sh\ncargo install --git\nhttps://example.test/xirang xirang-toolchain --features cli\n```\n",
    );
    let found = findings(&root);
    assert!(found.is_empty(), "the wrapped command is read: {found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The rule's stated boundary: a sentence may name the broken command to explain it, and
/// that sentence is not a command a reader copies. If this test starts failing, the rule was
/// widened to prose and the contract has to change with it.
/// 规则写明的边界：散文可以点名那条坏命令来解释它，而那句话不是读者会照抄的命令。这条测试哪天
/// 开始失败，就说明规则被扩到了散文，契约必须跟着改。
#[test]
fn a_sentence_about_the_command_is_not_a_documented_command() {
    let root = installed_workspace(
        GATED_CLI,
        "`--features cli` is what installs the command: a bare \
         `cargo install xirang-toolchain` would install nothing.\n",
    );
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "a sentence explaining the defect is not a documented command: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}
