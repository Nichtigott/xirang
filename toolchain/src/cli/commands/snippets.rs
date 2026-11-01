//! `xirang snippets`: inject face-field editor snippets.
//! `xirang snippets`：注入注册面字段的编辑器 snippet。
//!
//! Split out of `lib.rs`: this is the largest per-editor execution surface, and
//! its rules (VS Code project file, Neovim/Blink user config, `auto` scanning)
//! are independent of argv dispatch and of every other command.
//! 从 `lib.rs` 拆出：这是最大的按编辑器划分的执行面，其规则（VS Code 项目文件、
//! Neovim/Blink 用户配置、`auto` 扫描）与 argv 分发及其他命令无关。

use std::io::Write;

use crate::build_method::scaffold;

/// Inject the face-field editor snippets into a project or an editor config.
/// 把注册面字段的编辑器 snippet 注入项目或编辑器配置。
///
/// An editor's field completion inserts the bare name, and a language-server
/// snippet does not fire inside a macro call's token tree, so the `: ` after a
/// field name has to come from the editor's own snippet layer — and that layer
/// speaks a different format per editor. VS Code reads a project-scoped file
/// next to the sources; Neovim's LuaSnip loader scans its config for
/// `luasnippets/<filetype>/`, so nothing has to be wired up there either.
/// 编辑器的字段补全插入的是裸名字，而语言服务器的 snippet 在宏调用的 token 树里不会
/// 触发，因此字段名后的 `: ` 只能由编辑器自己的 snippet 层提供——而每个编辑器的格式不同。
/// VS Code 读源码旁的项目级文件；Neovim 的 LuaSnip 加载器会扫描配置目录里的
/// `luasnippets/<filetype>/`，因此那边同样无需额外接线。
pub(crate) fn snippets(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut directory: Option<String> = None;
    let mut editor = scaffold::Editor::Vscode;
    let mut auto = false;
    let mut stdout = false;
    let args = args.by_ref();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return crate::cli::usage(out),
            "--stdout" => stdout = true,
            "--editor" => {
                let name = args.next().ok_or("--editor requires a name")?;
                if name == "auto" {
                    auto = true;
                    continue;
                }
                let accepted = scaffold::Editor::ALL
                    .iter()
                    .map(|editor| editor.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                editor = scaffold::Editor::parse_editor(&name).ok_or_else(|| {
                    format!("unknown editor '{name}' (accepted: {accepted}, auto)")
                })?;
            }
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err("snippets accepts at most one path".to_owned()),
        }
    }
    if auto {
        if stdout {
            return Err(
                "--stdout prints one editor's file; pick one with --editor <name>".to_owned(),
            );
        }
        if directory.is_some() {
            return Err(
                "auto writes each editor's user-level location; use --editor <name> for a \
                 project file"
                    .to_owned(),
            );
        }
        let report = scaffold::install_everywhere()?;
        for install in &report.installs {
            println!(
                "xirang snippets: {} {} ({})",
                if install.written { "wrote" } else { "kept" },
                install.target.path.display(),
                install.target.editor.name()
            );
        }
        if report.nvim_skipped {
            println!(
                "xirang snippets: skipped Neovim — its snippet engines match fuzzily, so these \
                 triggers would also appear at value positions; run `--editor blink` or \
                 `--editor nvim` if you want them anyway"
            );
        }
        return Ok(());
    }
    if stdout {
        print!("{}", scaffold::editor_snippets(editor));
        return Ok(());
    }
    let path = match editor {
        scaffold::Editor::Vscode => {
            let directory = directory.unwrap_or_else(|| ".".to_owned());
            std::fs::canonicalize(&directory)
                .map_err(|error| format!("cannot resolve {directory}: {error}"))?
                .join(scaffold::SNIPPET_FILE)
        }
        scaffold::Editor::Nvim | scaffold::Editor::Blink => {
            if directory.is_some() {
                return Err(
                    "nvim snippets live in the editor config; use --stdout to write them \
                     somewhere else"
                        .to_owned(),
                );
            }
            let file = match editor {
                scaffold::Editor::Blink => scaffold::BLINK_SNIPPET_FILE,
                _ => scaffold::NVIM_SNIPPET_FILE,
            };
            scaffold::nvim_config_dir().join(file)
        }
    };
    let written = scaffold::write_snippets_file(&path, &scaffold::editor_snippets(editor))?;
    println!(
        "xirang snippets: {} {}",
        if written { "wrote" } else { "kept" },
        path.display()
    );
    Ok(())
}
