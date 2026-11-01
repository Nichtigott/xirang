//! Compiler failures must arrive without another execution, within a declared budget.
//! 编译失败无需再次执行即可到达，且遵守披露的预算。

#[test]
fn compiler_errors_keep_codes_locations_and_suggestions() {
    let log = "Compiling demo\nerror[E0308]: mismatched types\n --> src/lib.rs:2:4\n  |\n2 | \"wrong\"\n  | expected u32\nhelp: convert it\n\nwarning: unrelated\nnoise\n";
    for verbose in [false, true] {
        let reply = super::compiler_lines(log, verbose).join("\n");
        for expected in ["E0308", "src/lib.rs:2:4", "expected u32", "help: convert"] {
            assert!(reply.contains(expected), "{reply}");
        }
        assert!(!reply.contains("noise"));
        assert!(!reply.contains("Compiling"));
    }
}

#[test]
fn huge_unicode_diagnostics_are_bounded_and_disclose_loss() {
    let log = format!(
        "error[E0308]: {}\n --> src/lib.rs:2:4\n",
        "错误".repeat(20_000)
    );
    let normal = super::compiler_lines(&log, false).join("\n");
    let verbose = super::compiler_lines(&log, true).join("\n");
    assert!(normal.len() < 3_200, "{}", normal.len());
    assert!(verbose.len() < 12_200, "{}", verbose.len());
    assert!(verbose.len() > normal.len());
    assert!(normal.contains("withheld"));
    assert!(verbose.contains("withheld"));
}
