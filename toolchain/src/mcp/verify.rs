//! Re-run the kernel's validation and report the tree delta it just published.
//! 重新运行内核的校验，并报告它刚刚发布的树差异。
//!
//! The convergence loop's last rung: `xirang.apply` writes a face, and this says
//! whether the kernel still accepts the tree afterwards. It drives the *same* entry
//! the CLI's `check` drives (`check_for`), so a verdict here and a verdict there
//! cannot drift, and it publishes the build evidence as a side effect — which is what
//! makes the delta below describe the tree that was just verified rather than the one
//! from the last build. A failed verdict is the answer, not a tool failure: the reply
//! says `verdict failed` and carries the diagnostics, while the reply's `isError`
//! stays false because the verification itself succeeded.
//! 收敛闭环的最后一级：`xirang.apply` 写下一个面，而这里回答内核之后是否仍然接受这棵树。它驱动
//! CLI 的 `check` 所驱动的**同一个入口**（`check_for`），因此这里的判断与那里的判断不可能漂移；它还
//! 顺带发布构建证据——这正是下面那份差异描述的是"刚刚被校验的那棵树"、而不是"上次构建的那棵树"的原因。
//! 判断失败是答案而不是工具故障：回复写出 `verdict failed` 并带上诊断，而回复的 `isError` 仍为 false，
//! 因为校验本身成功了。

use std::path::Path;

use serde_json::Value;

use crate::mcp::build_evidence::out_dir;

/// The most diagnostic lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的诊断行数。
const MAX_DIAGNOSTIC_LINES: usize = 200;

/// Validate the package at `root`, then report the delta the run published.
/// 校验 `root` 处的包，然后报告那次运行发布的差异。
pub(crate) fn verify(root: &Path, arguments: &Value) -> Result<String, String> {
    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(format!(
            "verify needs a Cargo package: {} does not exist",
            manifest.display()
        ));
    }
    // The name this run publishes under is the *readers'* namespace, not Cargo's
    // package name: `diff` and `search` resolve identities through
    // `registry::namespace`, which honours `XIRANG_NAMESPACE`. Stamping the Cargo
    // name here made every face in a verified tree come back `re-identified` there —
    // a freshly checked tree reported as one whose every identity moved
    // (audit `LGC-LG-13`). One namespace for the writer and the readers.
    // 本次运行用于发布的那个名是**读取者**的命名空间，而不是 Cargo 的包名：`diff` 与 `search`
    // 经 `registry::namespace` 解析身份，而它会认可 `XIRANG_NAMESPACE`。在这里盖上 Cargo 名，
    // 会让刚校验过的树在那边把每个面都报成 `re-identified`（审计 `LGC-LG-13`）。写入方与读取方
    // 用同一个命名空间。
    let package = crate::mcp::registry::namespace(root)?;
    let out = out_dir(root);
    // `check_for` takes the package *directory* while `package_name` takes its manifest
    // *file*: pass the file here and the pipeline looks for `<Cargo.toml>/src` and reports
    // "is not a source directory". The CLI's local is named `manifest` and holds the
    // directory, which is how the confusion survives review.
    // `check_for` 收包**目录**，而 `package_name` 收它的清单**文件**：这里传文件，管线就会去找
    // `<Cargo.toml>/src` 并报 "is not a source directory"。CLI 的局部变量名叫 `manifest` 却装着
    // 目录——混淆就是这样通过审阅的。
    let verdict = match crate::build_method::check_for(root, &out, &package) {
        Ok(()) => "verdict ok (the kernel accepted the tree)\n".to_owned(),
        Err(diagnostics) => format!(
            "verdict failed ({} diagnostic(s))\n{}",
            diagnostics.len(),
            bounded(&diagnostics.render_build_diagnostics())
        ),
    };
    // The static verdict above is one of the two faces this package is judged on, and
    // the other one is the authoring connector: `apply`, `usages` and `converge` all
    // validate through `load_registry`, which reads every face file rather than the
    // build's active scope. A tree can therefore pass above and be refused there — the
    // build checks the requirements of the faces it ships, the connector checks the
    // ones on disk — and an agent that reads only the first line carries a green light
    // into a rejection (audit `F1`). Both lines are true of the same tree, so both are
    // printed, and the static wording above is left exactly as it was.
    // The label reports whether that surface accepts this tree, which is the
    // precondition of every connector-face tool; a tree it refuses is the answer rather
    // than a tool failure, so the rejection is rendered instead of returned.
    // 上面那条静态判断只是本包会被评判的两个面之一，另一个面是创作连接器：`apply`、`usages` 与
    // `converge` 都经 `load_registry` 校验，而它读的是每个面文件，而不是构建的活跃作用域。因此一棵树
    // 可以在上面通过、在那里被拒——构建检查的是它会发布的面，连接器检查的是磁盘上的面——而只读第一行的
    // 代理会把绿灯带进拒绝里（审计 `F1`）。两行对同一棵树都成立，所以两行都打印，而上面那条静态措辞
    // 保持原样。
    // 这个标签报告的是那个面是否接受这棵树——它是每个连接器面工具的前置条件；而它拒绝的树是答案而不是
    // 工具故障，所以拒绝被渲染出来，而不是当作错误返回。
    let connector = match crate::mcp::apply::load_registry(root, &package) {
        Ok(_) => "connector verdict: ok (the package's own faces were admitted)\n".to_owned(),
        Err(rejection) => format!(
            "connector verdict: rejected\n{}",
            bounded(&rejection).trim_end()
        ),
    };
    // The delta is the same report `xirang.diff` gives, and it is meaningful here
    // precisely because the call above just refreshed the build's side of it.
    // 这份差异就是 `xirang.diff` 给出的同一份报告，而它在这里有意义，正是因为上面那次调用刚刚刷新
    // 了它在构建一侧的数据。
    //
    // Only the arguments this tool declares reach it. Forwarding the whole request let an
    // undeclared `records: true` replace the promised tree delta with the records report, so
    // the same call silently answered a different question than the one its description
    // promises.
    // 只有本工具声明的参数会传进去。把整个请求透传，会让一个未声明的 `records: true` 把承诺的树差异
    // 换成记录报告——同一次调用因此静默回答了与它描述所承诺的另一个问题。
    let mut declared = serde_json::Map::new();
    for key in ["limit", "root"] {
        if let Some(value) = arguments.get(key) {
            declared.insert(key.to_owned(), value.clone());
        }
    }
    let delta = crate::mcp::diff::diff(root, &Value::Object(declared))?;
    Ok(format!(
        "{verdict}scope: static registration only; Rust compilation/tests NOT run\n{connector}\n{delta}\nnext   test behavior via xirang_tools {{\"tool\":\"check\",\"arguments\":{{\"workspace\":true,\"census\":false}}}}\n"
    ))
}

/// Keep one reply inside a size an agent can read, and say when it did not.
/// 把一条回复限制在代理读得下的规模里，并在截断时说出来。
///
/// The cap cannot be raised from here — it is this bridge's, not the kernel's — so the
/// way to the whole list is the command that prints it without one: the CLI.
/// 这道理上限不能从这里提高——它是本桥的，不是内核的——因此拿到完整清单的出路是那条不加限制就
/// 把它全打印出来的命令：CLI。
fn bounded(text: &str) -> String {
    crate::mcp::truncation::bounded(
        text,
        MAX_DIAGNOSTIC_LINES,
        "diagnostic lines",
        "run `xirang check` for the whole list",
    )
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod verify_tests;
