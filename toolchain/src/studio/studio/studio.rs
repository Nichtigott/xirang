//! XiRang Studio: an optional Ratatui adapter for the registration core.
//! XiRang Studio：注册核心的可选 Ratatui 适配器。

#[path = "app/app.rs"]
mod app;
mod terminal;
#[path = "ui/ui.rs"]
mod ui;

use std::io::{self, IsTerminal, stdout};

use crossterm::event;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;
use terminal::{install_panic_restore, open_editor_in_terminal};

/// Run Studio's terminal event loop until the user quits or the terminal fails.
/// 运行 Studio 终端事件循环，直到用户退出或终端出错。
///
/// Raw mode, the alternate screen, and mouse capture are installed here and
/// restored on return as well as on panic.
/// 原始模式、备用屏幕与鼠标捕获都在此安装，并在返回和 panic 时恢复。
pub fn launch() -> io::Result<()> {
    launch_with(None)
}

/// Run Studio on the project named on the command line.
/// 在命令行指定的项目上运行 Studio。
///
/// The path goes through the same resolution the environment variable does, so an
/// unusable one is refused with a message rather than opening the wrong tree.
/// 该路径走与环境变量相同的解析，因此不可用的路径会带着消息被拒绝，而不是打开错的树。
pub fn launch_with(project: Option<std::path::PathBuf>) -> io::Result<()> {
    // Refuse before taking over the terminal. An empty user interface that exits
    // zero is worse than a message here: the process would look healthy, and the
    // next authoring command would write into whatever directory it happened to
    // be in instead of the project the reader thought was open.
    // 在接管终端之前就拒绝。此处"空界面 + 退出 0"比一条消息更糟：进程看上去是健康的，而
    // 随后的创作命令会写进它恰好所在的目录，而不是读者以为打开的那个项目。
    // The caller reports it: `main` prints this one line, and the CLI maps it
    // into its own error channel, so the message is never rendered twice.
    // 由调用方报告：`main` 打印这一行，CLI 把它映射进自己的错误通道，因此这条消息不会
    // 被渲染两次。
    app::preflight(project.as_deref()).map_err(io::Error::other)?;
    // A TUI owes the caller a message, not an errno. Without a terminal the raw
    // -mode setup used to fail with a bare `os error 6` (ENXIO) from opening
    // `/dev/tty`; naming the stream and the cause is what a reader can act on.
    // TUI 欠调用方的是一条消息，而不是一个 errno。没有终端时，原始模式初始化会因打开
    // `/dev/tty` 失败而抛出裸的 `os error 6`（ENXIO）；点名是哪个流、什么原因，读者
    // 才能据此行动。
    if !stdout().is_terminal() {
        return Err(io::Error::other(
            "xirang-toolchain draws to a terminal, but stdout is not one (it is redirected or a \
             pipe); run it directly",
        ));
    }
    if !io::stdin().is_terminal() {
        return Err(io::Error::other(
            "xirang-toolchain reads keys from a terminal, but stdin is not one; run it directly",
        ));
    }
    install_panic_restore();
    enable_raw_mode()?;
    let _terminal_guard = TerminalGuard;
    let mut output = stdout();
    execute!(output, EnterAlternateScreen, event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_loop(&mut terminal);
    terminal.show_cursor()?;
    result
}

/// Restore terminal state even when setup or the event loop returns an error.
/// 即使初始化或事件循环返回错误，也要恢复终端状态。
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), event::DisableMouseCapture, LeaveAlternateScreen);
    }
}

/// Draw only after input or a terminal event; idle Studio has no redraw loop.
/// 仅在输入或终端事件后绘制；Studio 空闲时没有重绘循环。
fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::load_app();
    let mut redraw = true;
    loop {
        if redraw {
            ui::draw_once(terminal, &mut app)?;
            redraw = false;
        }
        if app.should_quit {
            return Ok(());
        }
        if event::poll(std::time::Duration::from_millis(250))? {
            app.handle_app(event::read()?);
            redraw = true;
        }
        // The method is internally throttled, so checking after input keeps
        // reload latency low without adding work to the hot path.
        // 方法内部自带节流；每次输入后检查即可降低刷新延迟，不增加热路径开销。
        let event_before_reload = app.event.clone();
        app.poll_hot_reload();
        redraw |= app.event != event_before_reload;
        if let Some((path, line)) = app.take_editor_request() {
            let result = open_editor_in_terminal(&path, line);
            app.event = match result {
                Ok(editor) => format!("{editor} opened in a new terminal."),
                Err(error) => format!("Editor failed: {error}"),
            };
            redraw = true;
        }
    }
}
