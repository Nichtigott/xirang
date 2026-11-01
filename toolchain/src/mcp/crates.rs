//! `xirang.crates` — the crate partition a host declares, and the three writers that make it real.
//! `xirang.crates` —— 宿主声明的 crate 分区，以及把它变成现实的三个写入方。
//!
//! A host declares at its package root which subtrees become crates of their own (`add_crates.rs`),
//! and `xirang crates` plus Studio's partition screen already turn that declaration into packages.
//! The bridge is the third surface, and it answers the same question with the same reader
//! (`build_method::partition_view`) and writes with the same writer — so an agent can plan a split,
//! see what is on disk, judge whether it could be published, and only then write it.
//! 宿主在包根声明哪些子树各自成为一个 crate（`add_crates.rs`），而 `xirang crates` 与 Studio 的分区屏
//! 已经能把这份声明变成包。桥是第三个执行面，它用**同一个**读取器（`build_method::partition_view`）回答
//! 同一个问题、用**同一个**写入方写入——于是代理可以先规划一次拆分、看清磁盘上有什么、判断能不能发布，
//! 然后才写。
//!
//! As everywhere else in this bridge, **a request is a preview unless `apply` is true**: `plan` never
//! writes at all, and `write`/`release`/`revert` describe exactly what they would change until they
//! are told to change it.
//! 与本桥其它地方一样，**除非 `apply` 为真，请求只是预览**：`plan` 从不写入，而
//! `write`/`release`/`revert` 在被告知动手之前，只描述它们会改什么。

use std::path::Path;

use serde_json::Value;

use crate::build_method::partition_view::{OnDisk, PartitionView};
use crate::build_method::{DeclarationEdit, declare, undeclare};

/// The most package rows one reply prints before it says how many it left out.
/// 一次回复在说明"省掉多少"之前最多打印多少个包行。
///
/// The figure is the one every other listing tool in this bridge uses, so a reader who learned the
/// bound once does not have to learn a second one.
/// 这个数字是本桥其它列表面具共用的那个，因此记住过一次的读者不必再记第二个。
const ROW_BUDGET: usize = 40;

pub(crate) fn crates(root: &Path, arguments: &Value) -> Result<String, String> {
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("plan");
    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // The declaration layer comes first, because the case the view cannot describe — a host that
    // declares nothing — is exactly what `declare` is for: it creates the file.
    // 声明层放在最前面，因为视图描述不了的那种情形——一个什么都没声明的宿主——正是 `declare` 存在的理由：
    // 它把这个文件建起来。
    if matches!(action, "declare" | "undeclare") {
        return match action {
            "declare" => {
                let name = required_argument(arguments, "crate")?;
                let subtrees = subtrees_argument(arguments)?;
                let edit = declare(root, &name, &subtrees)?;
                finish_declaration_edit(root, &name, edit, apply)
            }
            _ => {
                let name = required_argument(arguments, "crate")?;
                // The packages on disk are named by the declaration this is about to change: once the
                // entry is gone, no action knows that directory any more, so `revert` has to run
                // **first** or the directory stays behind as an orphan this tool can no longer reach.
                // 磁盘上的包是由这份即将被改动的声明命名的：条目一旦消失，就没有动作还知道那个目录了，因此
                // `revert` 必须**先**跑，否则那个目录会作为孤儿留下、本工具再也够不到它。
                if let Some(directory) = crate::build_method::package_directory_of(root, &name)? {
                    return Err(format!(
                        "`{name}` still has its package at {}: `revert` first (it takes the generated \
                         packages back while the declaration still names them), then undeclare\n\
                         `{name}` 的包还在 {}：先 `revert`（它趁声明还点名它们时把生成的包收回去），再 \
                         undeclare\n",
                        directory.display(),
                        directory.display()
                    ));
                }
                let edit = undeclare(root, &name)?;
                // Nothing on disk is guaranteed here: the reader refuses by name when the crate is
                // unknown, and a known crate with a package still on disk is refused before the edit.
                finish_declaration_edit(root, &name, edit, apply)
            }
        };
    }
    let view = crate::build_method::partition_view::view(root)?;
    let Some(view) = view else {
        return Ok(format!(
            "{} declares no crates: it is one crate. A split is declared at the package root in \
             `{}` — `Shape {{ package_prefix, crates: &[Crate::named(\"x\").at(&[…::SUBTREE])] }}` — \
             and `plan` describes it once that file exists.\n\
             {} 没有声明任何 crate：它就是一个 crate。拆分在包根的 `{}` 里声明——\
             `Shape {{ package_prefix, crates: &[Crate::named(\"x\").at(&[…::SUBTREE])] }}`——\
             那个文件存在之后 `plan` 就能描述它。\n",
            root.display(),
            xirang_kernel::lexicon::ADD_CRATES_FILE,
            root.display(),
            xirang_kernel::lexicon::ADD_CRATES_FILE,
        ));
    };
    let report = describe(&view);
    match action {
        "plan" => Ok(format!(
            "{report}\n\
             no action taken: `plan` writes nothing. `write` (the development shape), `release` \
             (the publishable shape) and `revert` each need `apply: true`.\n\
             未采取动作：`plan` 不写任何东西。`write`（开发形状）、`release`（可发布形状）与 \
             `revert` 各自需要 `apply: true`。\n"
        )),
        "write" | "release" | "revert" => {
            if !apply {
                return Ok(format!(
                    "{report}\n\
                     preview: `{action}` would {} — call it again with `apply: true` to write it.\n\
                     预览：`{action}` 将{}——再次调用并带 `apply: true` 才会写入。\n",
                    plan_sentence(action),
                    plan_sentence_cn(action)
                ));
            }
            let plan =
                crate::build_method::partition_view::plan(root)?.expect("a declaration exists");
            let directories: Vec<&Path> = plan
                .planned
                .iter()
                .map(|planned| planned.directory.as_path())
                .chain(plan.facade.iter().map(|facade| facade.directory.as_path()))
                .collect();
            let outcome = match action {
                "write" => {
                    crate::build_method::guard_shape(&directories, false)?;
                    let written = crate::build_method::write_partition(
                        &plan.config_root,
                        plan.workspace.as_deref(),
                        &plan.planned,
                        plan.facade.as_ref(),
                    )?;
                    written_sentence(&written)
                }
                "release" => {
                    crate::build_method::guard_shape(&directories, true)?;
                    let written = crate::build_method::write_release(
                        &plan.config_root,
                        plan.workspace.as_deref(),
                        &plan.release,
                        &plan.host_root,
                    )?;
                    written_sentence(&written)
                }
                _ => {
                    let written = crate::build_method::revert_partition(
                        &plan.config_root,
                        plan.workspace.as_deref(),
                        &plan.planned,
                        plan.facade.as_ref(),
                    )?;
                    written_sentence(&written)
                }
            };
            // The answer carries the tree **after** the write, read from the disk: the next call can
            // be aimed with it instead of with what the caller hoped happened.
            // 答案携带写入**之后**的树、从磁盘读出：下一次调用可以据此瞄准，而不是据调用方希望发生的事。
            let after = crate::build_method::partition_view::view(root)?
                .map(|view| describe(&view))
                .unwrap_or_default();
            Ok(format!("{outcome}\n{after}"))
        }
        other => Err(format!(
            "action `{other}` is not implemented; this tool supports `plan` (the default, and the \
             only one that writes nothing), `write`, `release`, `revert`, `declare` and `undeclare`\n\
             动作 `{other}` 未实现；本工具支持 `plan`（默认，也是唯一不写入的）、`write`、`release`、\
             `revert`、`declare` 与 `undeclare`\n"
        )),
    }
}

/// One required string argument, refused by name when it is missing.
/// 一个必需的字符串参数，缺失时按名字拒绝。
fn required_argument(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "`{key}` is required for this action: `declare` needs the crate's name and the \
                 subtree it claims (`subtree`), `undeclare` needs the name\n\
                 本动作需要 `{key}`：`declare` 要 crate 的名字与它认领的子树（`subtree`），\
                 `undeclare` 要名字\n"
            )
        })
}

/// The subtrees a `declare` claims: one string or an array of them.
/// `declare` 认领的子树：一个字符串，或它们的数组。
fn subtrees_argument(arguments: &Value) -> Result<Vec<String>, String> {
    match arguments.get("subtree") {
        Some(Value::String(one)) if !one.is_empty() => Ok(vec![one.clone()]),
        Some(Value::Array(many)) => {
            let subtrees: Vec<String> = many
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect();
            if subtrees.is_empty() {
                return Err(subtrees_help());
            }
            Ok(subtrees)
        }
        _ => Err(subtrees_help()),
    }
}

fn subtrees_help() -> String {
    "`subtree` is required: the host's own path to a `SUBTREE` constant, e.g. \
     `crate::panel::frame::SUBTREE` (or an array of them)\n\
     需要 `subtree`：宿主自己指向某个 `SUBTREE` 常量的路径，例如 `crate::panel::frame::SUBTREE`（或其数组）\n"
        .to_owned()
}

/// Preview or apply one declaration edit, and say what the tree looks like afterwards.
/// 预览或落盘一次声明改动，并说明改完之后这棵树是什么样。
fn finish_declaration_edit(
    root: &Path,
    name: &str,
    edit: DeclarationEdit,
    apply: bool,
) -> Result<String, String> {
    let diff = edit.diff();
    let action = if edit.removes_file {
        format!(
            "remove {} (removing the last declared crate gives the host back to one crate)",
            edit.path.display()
        )
    } else if edit.before.is_empty() {
        format!(
            "create {} with `{name}` as its first crate",
            edit.path.display()
        )
    } else {
        format!(
            "edit {} in place, touching only `{name}`'s entry",
            edit.path.display()
        )
    };
    if !apply {
        return Ok(format!(
            "preview: {action}\n{diff}\
             the file is untouched — call this again with `apply: true` to write it\n\
             预览：{action}\n{diff}\
             文件未被动过——再次调用并带 `apply: true` 才会写入\n"
        ));
    }
    let after = crate::build_method::apply_declaration_edit(root, edit)?;
    Ok(format!(
        "{action}\n{diff}\
         the declaration changed, so the packages on disk describe the shape before it: `write` (or \
         `release`) writes the new one, `plan` shows it\n\
         声明变了，因此磁盘上的包描述的是改之前的形状：`write`（或 `release`）写下新的，`plan` 显示它\n{}",
        after
            .map(|view| describe(&view))
            .unwrap_or_else(|| format!("{name}: no crate is declared now\n")),
    ))
}

/// What the screen and the CLI both print: the declaration, then one row per package.
/// 本屏与 CLI 都打印的东西：声明，然后每个包一行。
fn describe(view: &PartitionView) -> String {
    let mut text = format!(
        "partition of `{}`: {} face(s) in the host's own tree, {} generated package(s)\n",
        view.package_prefix,
        view.host_faces,
        view.packages.len()
    );
    text.push_str(&format!(
        "  belongs to     {}\n",
        match &view.workspace {
            Some(workspace) => format!(
                "workspace {} (config and member list live there)",
                workspace.display()
            ),
            None => format!(
                "no enclosing workspace; the config goes to {}",
                view.config_root.display()
            ),
        }
    ));
    if !view.members_missing.is_empty() {
        text.push_str(&format!(
            "  members        not listed yet: {}\n",
            view.members_missing.join(", ")
        ));
    }
    for note in &view.notes {
        text.push_str(&format!("  note           {note}\n"));
    }
    for package in view.packages.iter().take(ROW_BUDGET) {
        let role = match &package.crate_name {
            Some(name) => format!("crate {name} ({})", package.subtrees.join(", ")),
            None => "facade (the cross-crate half)".to_owned(),
        };
        text.push_str(&format!("  {:<28} {role}\n", package.package,));
        text.push_str(&format!(
            "    on disk      {} · {} face(s) compiled · {} mounted (development) · {} copied \
             (release) · {} file(s), {:.1} KiB\n",
            match package.on_disk {
                OnDisk::Absent => "absent",
                OnDisk::Development => "development",
                OnDisk::Release => "release",
                OnDisk::Foreign => "NOT this action's package",
            },
            package.compiles,
            package.mounts,
            package.copies,
            package.files,
            package.bytes as f64 / 1024.0
        ));
        text.push_str(&format!(
            "    at           {}\n",
            package.directory.display()
        ));
        if !package.depends_on.is_empty() {
            text.push_str(&format!(
                "    depends on   {}\n",
                package.depends_on.join(", ")
            ));
        }
        for publish in &package.publish {
            text.push_str(&format!("    publish      {publish}\n"));
        }
    }
    if view.packages.len() > ROW_BUDGET {
        text.push_str(&format!(
            "  … {} more package(s) not shown\n",
            view.packages.len() - ROW_BUDGET
        ));
    }
    text
}

/// What an action would do, for the preview sentence.
/// 一个动作会做什么，供预览句使用。
fn plan_sentence(action: &str) -> &'static str {
    match action {
        "write" => {
            "write the development shape (mounts the host's files, and the workspace config gains \
             the remap)"
        }
        "release" => {
            "write the release shape (each package carries the sources its build reads, so it can \
             be published)"
        }
        _ => "remove the generated packages this action wrote, and the member list entries",
    }
}

/// The same sentence in Chinese, so the preview is one message rather than two lookups.
/// 同一句话的中文，让预览是一条消息而不是两次查阅。
fn plan_sentence_cn(action: &str) -> &'static str {
    match action {
        "write" => "写下开发形状（挂载宿主的文件，工作区配置随之得到 remap）",
        "release" => "写下发布形状（每个包携带自己构建要读的源码，因此可以发布）",
        _ => "移除本次动作写下的生成包，以及成员清单里它写下的条目",
    }
}

/// What a write or a revert did.
/// 一次写入或撤回做了什么。
fn written_sentence(written: &crate::build_method::crate_write::Written) -> String {
    format!(
        "wrote {} file(s); workspace config {}; member list {}\n",
        written.files.len(),
        if written.config_changed {
            "updated"
        } else {
            "untouched"
        },
        if written.members_changed {
            "updated"
        } else {
            "untouched"
        }
    )
}

#[cfg(test)]
#[path = "crates_tests.rs"]
mod crates_tests;
