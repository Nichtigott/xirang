//! Terminal lifecycle and external editor integration.
//! 终端生命周期与外部编辑器集成。

use std::io::{self, stdout};
use std::panic;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};

/// Open the editor in a separate terminal window and leave Studio untouched.
/// 在独立终端窗口打开编辑器，Studio 自身不离开当前 TUI。
pub(super) fn open_editor_in_terminal(path: &Path, line: u32) -> io::Result<String> {
    let editor = std::env::var_os("VISUAL")
        .or_else(|| std::env::var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into());
    let editor_name = editor.to_string_lossy().into_owned();
    let script = editor_script(&editor_name, path, line);
    let shell = PathBuf::from(std::env::var_os("SHELL").unwrap_or_else(|| "sh".into()));
    let terminal = terminal_launcher().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no supported terminal launcher found (set TERMINAL)",
        )
    })?;
    let shell_arg = shell.to_string_lossy().into_owned();
    let mut command = Command::new(&terminal.program);
    match terminal.kind.as_str() {
        "kitty" | "foot" | "gnome-terminal" | "konsole" | "xfce4-terminal" => {
            command.args(["--", &shell_arg, "-lc", &script]);
        }
        "alacritty" | "xterm" => {
            command.args(["-e", &shell_arg, "-lc", &script]);
        }
        "wezterm" => {
            command.args(["start", "--", &shell_arg, "-lc", &script]);
        }
        _ => {
            command.args(["--", &shell_arg, "-lc", &script]);
        }
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| editor_name)
}

#[derive(Debug)]
struct TerminalLauncher {
    program: PathBuf,
    kind: String,
}

/// Prefer the configured terminal, then common desktop terminals.
/// 优先使用 TERMINAL 配置，再尝试常见桌面终端。
fn terminal_launcher() -> Option<TerminalLauncher> {
    let candidates = std::env::var("TERMINAL").ok().into_iter().chain(
        [
            "kitty",
            "foot",
            "alacritty",
            "wezterm",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "xterm",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    candidates
        .filter_map(|spec| {
            let name = spec.split_whitespace().next()?;
            let program = find_in_path(name)?;
            let kind = program.file_name()?.to_string_lossy().to_ascii_lowercase();
            Some(TerminalLauncher { program, kind })
        })
        .next()
}

fn find_in_path(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_file() {
        return Some(path.to_owned());
    }
    std::env::var_os("PATH")?.to_str().and_then(|path| {
        path.split(':')
            .map(|entry| Path::new(entry).join(program))
            .find(|candidate| candidate.is_file())
    })
}

/// Build a shell command so `$EDITOR` may include flags or wrappers.
/// 通过 shell 组装命令，允许 `$EDITOR` 自带参数或包装器。
pub(super) fn editor_script(editor: &str, path: &std::path::Path, line: u32) -> String {
    let path = shell_quote(&path.to_string_lossy());
    let program = editor
        .split_whitespace()
        .next()
        .and_then(|value| std::path::Path::new(value).file_name())
        .and_then(|value| value.to_str())
        .unwrap_or(editor);
    let lower = program.to_ascii_lowercase();

    if matches!(
        lower.as_str(),
        "nvim" | "nvim-qt" | "vim" | "vi" | "nano" | "emacs" | "emacsclient"
    ) {
        format!(
            "exec {editor} +{line} {path}",
            editor = editor,
            line = line,
            path = path
        )
    } else if lower == "hx" || lower == "helix" {
        format!(
            "exec {editor} {path}:{line}",
            editor = editor,
            path = path,
            line = line
        )
    } else {
        format!("exec {editor} {path}", editor = editor, path = path)
    }
}

/// Quote one argument for POSIX shells without changing its contents.
/// 为 POSIX shell 安全转义单个参数，不改变参数内容。
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Restore the user's shell before printing a panic.
/// panic 输出前恢复用户终端。
pub(super) fn install_panic_restore() {
    let original = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen);
        original(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::editor_script;
    use std::path::Path;

    #[test]
    fn editor_script_keeps_nvim_command_plain() {
        let script = editor_script("nvim --clean", Path::new("src/thing's.rs"), 12);
        assert!(script.contains("nvim --clean +12 'src/thing'\\''s.rs'"));
        assert!(!script.contains("vim.treesitter"));
        assert!(!script.contains("syntax=rust"));
    }

    #[test]
    fn editor_script_passes_unknown_editors_a_single_safe_path() {
        let script = editor_script("my-editor --wait", Path::new("src/a b.rs"), 7);
        assert_eq!(script, "exec my-editor --wait 'src/a b.rs'");
    }

    #[test]
    fn vim_script_is_plain_too() {
        let script = editor_script("vim", Path::new("src/main.rs"), 3);
        assert!(script.contains("vim +3 'src/main.rs'"));
        assert!(!script.contains("vim.treesitter"));
    }
}
