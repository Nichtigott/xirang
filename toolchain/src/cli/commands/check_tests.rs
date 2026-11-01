//! Tests for the validation pass: the duplicated-own-crates gate reads every package, not just the host.
//! 校验流程的测试：重复自有 crate 的门禁读每一个包，而不只是宿主。

/// 重复自有 crate 的门禁会读宿主**以及每一个生成包**。
///
/// It used to read only the host, and a ghost is an independent package: the host's `[patch.crates-io]` does
/// not reach it, so it can hold this workspace's crates twice while the host holds them once. Measured on a
/// partitioned tree — the host's own `check` was green while `cargo metadata --manifest-path
/// crates/dash-dash-board/Cargo.toml` reported `{'xirang-kernel': 2, 'xirang-macro': 2,
/// 'xirang-toolchain': 2}` (audit `M7`, §M7.66). This pin covers the half that is testable without a
/// registry: **which manifests the gate looks at**.
/// 它过去只读宿主，而幽灵是一个独立的包：宿主的 `[patch.crates-io]` 到不了它，因此宿主只有一份时它可能带着
/// 本工作区的 crate 两份。在一个分区树上实测——宿主自己的 `check` 是绿的，而
/// `cargo metadata --manifest-path crates/dash-dash-board/Cargo.toml` 报
/// `{'xirang-kernel': 2, 'xirang-macro': 2, 'xirang-toolchain': 2}`（审计 `M7`，§M7.66）。这条钉子
/// 覆盖的是"不需要 registry 就能测"的那一半：**门禁读哪些清单**。
#[test]
fn the_duplicated_gate_reads_every_generated_package() {
    let area = std::env::temp_dir().join(format!("xirang-gate-manifests-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&area);
    std::fs::create_dir_all(area.join("host")).expect("the host");
    std::fs::create_dir_all(area.join("host/crates/one")).expect("a generated package");
    std::fs::create_dir_all(area.join("host/crates/two")).expect("a second one");
    std::fs::write(area.join("host/Cargo.toml"), "[package]\nname = \"host\"\n")
        .expect("the host manifest");
    std::fs::write(
        area.join("host/crates/one/Cargo.toml"),
        "[package]\nname = \"one\"\n",
    )
    .expect("one");
    std::fs::write(
        area.join("host/crates/two/Cargo.toml"),
        "[package]\nname = \"two\"\n",
    )
    .expect("two");
    // A directory that is not a package must not be read as one.
    // 一个不是包的目录不许被当成包读。
    std::fs::create_dir_all(area.join("host/crates/not-a-package")).expect("noise");

    let host = area.join("host/Cargo.toml");
    let read = super::own_crate_manifests(&host);
    assert_eq!(
        read.first(),
        Some(&host),
        "the host's own graph comes first"
    );
    let mut rest: Vec<String> = read
        .iter()
        .skip(1)
        .map(|path| {
            path.parent()
                .and_then(|parent| parent.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .collect();
    rest.sort();
    assert_eq!(
        rest,
        vec!["one".to_owned(), "two".to_owned()],
        "every generated package is read, and nothing else is"
    );
    let _ = std::fs::remove_dir_all(&area);
}
