//! Editor snippet installation orchestration.
//! 编辑器 snippet 安装编排。
//!
//! Discovery and writing live next to the rendering in `snippets.rs`: this module
//! probes where each editor keeps its user-level snippet directory, decides which
//! engines are installed by default, and writes the rendered file. The CLI keeps
//! only the argv spellings and the human-readable report, so the install policy
//! has one home instead of two.
//! 发现与写入紧挨 `snippets.rs` 的渲染：本模块探测每个编辑器的用户级 snippet 目录、
//! 决定默认安装哪些引擎，并写入渲染结果。CLI 只保留 argv 写法与给人看的报告，因此安装
//! 策略只有一个归属，而不是两处。

use std::path::{Path, PathBuf};

use super::snippets::{
    BLINK_SNIPPET_FILE, Editor, NVIM_SNIPPET_FILE, editor_snippets, write_snippets_file,
};

/// One editor's user-level snippet destination found on this machine.
/// 本机上找到的某个编辑器的用户级 snippet 目标位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorTarget {
    /// Which editor the file is written for.
    /// 该文件写给哪个编辑器。
    pub editor: Editor,
    /// The snippet file's path.
    /// snippet 文件的路径。
    pub path: PathBuf,
}

/// What one write attempt did to one discovered target.
/// 一次写入尝试对某个已发现目标做了什么。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetInstall {
    /// The target the write was aimed at.
    /// 写入所针对的目标。
    pub target: EditorTarget,
    /// Whether the file changed (`true`) or already held the current content
    /// (`false`).
    /// 文件是否被改动（`true`），还是已经是最新内容（`false`）。
    pub written: bool,
}

/// The full result of one auto-installation pass.
/// 一次自动安装的完整结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallReport {
    /// One entry per discovered target, in discovery order.
    /// 每个已发现目标一项，按发现顺序排列。
    pub installs: Vec<SnippetInstall>,
    /// Whether a Neovim configuration exists and was skipped because its engines
    /// match fuzzily.
    /// 是否存在 Neovim 配置，并因其引擎模糊匹配而被跳过。
    pub nvim_skipped: bool,
}

/// Install the snippets for every editor this machine has, each in that editor's
/// **user-level** location, so one run covers every project from then on.
/// 给本机装有的每个编辑器安装 snippet，都写进各编辑器的**用户级**位置，因此跑一次就覆盖
/// 之后的所有项目。
///
/// The files are small, generated from one vocabulary and written only when their
/// content changes, so this is safe to run again (a post-checkout hook, a shell
/// alias, or by hand after `FACE_FIELD_ORDER` gains a field). The returned report
/// names every file, whether it changed, and whether Neovim was skipped, so the
/// caller keeps the human-readable output.
/// 这些文件很小、由同一份词表生成、内容不变时不写盘，因此可以反复运行（检出钩子、shell
/// 别名，或在 `FACE_FIELD_ORDER` 新增字段后手动跑一次）。返回的报告列出每个文件、是否
/// 改动、以及 Neovim 是否被跳过，因此给用户看的输出仍由调用方负责。
pub fn install_everywhere() -> Result<InstallReport, String> {
    let config_home = config_home();
    let targets = snippet_targets(&config_home, false);
    if targets.is_empty() && !has_nvim_config(&config_home) {
        return Err(
            "no VS Code, VSCodium, Cursor or Neovim configuration found; write the file \
             yourself with --editor <name> --stdout"
                .to_owned(),
        );
    }
    let mut installs = Vec::with_capacity(targets.len());
    for target in targets {
        let written = write_snippets_file(&target.path, &editor_snippets(target.editor))?;
        installs.push(SnippetInstall { target, written });
    }
    Ok(InstallReport {
        installs,
        nvim_skipped: has_nvim_config(&config_home),
    })
}

/// Every editor snippet location this machine has.
/// 本机具有的每个编辑器 snippet 位置。
///
/// Only editors that match a snippet **prefix by prefix** are installed by
/// default. An engine that matches fuzzily — blink.cmp, LuaSnip through
/// nvim-cmp — also offers a field trigger at a **value** position, so
/// `crc` there matches `parent: ` and the field shapes pollute the
/// value chain (measured; see `docs/audit-2026-09-21.md`). Those engines are
/// skipped unless the caller asks for them by name, because the sentence a field
/// shape really needs — `crate::build_method::…::NODE_ID`, the face's own marker type — is
/// value completion, which rust-analyzer already provides.
/// 默认只安装**按前缀**匹配 snippet 的编辑器。模糊匹配的引擎（blink.cmp、经 nvim-cmp 的
/// LuaSnip）在**值位**也会命中字段 trigger，于是值位敲 `crc` 会命中 `parent: `、
/// 字段定式污染值补全链（已实测，见 `docs/audit-2026-09-21.md`）。这类引擎只有在调用方显式
/// 指名时才安装——因为字段定式真正需要的语句（`crate::build_method::…::NODE_ID`、本面自己的标记类型）
/// 属于值补全，rust-analyzer 本来就提供。
///
/// VS Code and its forks merge every `*.code-snippets` in their user snippets
/// directory, so one file there is enough and covers all projects.
/// VS Code 及其衍生编辑器会合并用户 snippet 目录里所有 `*.code-snippets`，因此那边一个
/// 文件就够，而且覆盖所有项目。
pub fn snippet_targets(config_home: &Path, fuzzy_engines: bool) -> Vec<EditorTarget> {
    let mut targets = Vec::new();
    if fuzzy_engines {
        let nvim = config_home.join("nvim");
        if nvim.is_dir() {
            targets.push(EditorTarget {
                editor: Editor::Blink,
                path: nvim.join(BLINK_SNIPPET_FILE),
            });
            targets.push(EditorTarget {
                editor: Editor::Nvim,
                path: nvim.join(NVIM_SNIPPET_FILE),
            });
        }
    }
    for fork in ["Code", "VSCodium", "Cursor"] {
        let root = config_home.join(fork);
        if root.is_dir() {
            targets.push(EditorTarget {
                editor: Editor::Vscode,
                path: root
                    .join("User")
                    .join("snippets")
                    .join("xirang-face.code-snippets"),
            });
        }
    }
    targets
}

/// Whether a Neovim configuration exists, so the auto installer can say why it
/// skipped it.
/// 是否存在 Neovim 配置，好让自动安装说明它为什么跳过。
pub fn has_nvim_config(config_home: &Path) -> bool {
    config_home.join("nvim").is_dir()
}

/// The user configuration directory: `$XDG_CONFIG_HOME`, or `~/.config`.
/// 用户配置目录：`$XDG_CONFIG_HOME`，否则 `~/.config`。
pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

/// The Neovim configuration directory: `$XDG_CONFIG_HOME` (or `~/.config`)
/// followed by `$NVIM_APPNAME` (or `nvim`), which is how Neovim itself resolves
/// it.
/// Neovim 配置目录：`$XDG_CONFIG_HOME`（或 `~/.config`）加上 `$NVIM_APPNAME`
/// （或 `nvim`），与 Neovim 自身的解析方式一致。
pub fn nvim_config_dir() -> PathBuf {
    let app_name = std::env::var("NVIM_APPNAME").unwrap_or_else(|_| "nvim".to_owned());
    nvim_config_dir_in(&config_home(), &app_name)
}

/// Resolve the Neovim config directory from explicit values, so the rule can be
/// pinned by a test.
/// 用显式值解析 Neovim 配置目录，使这条规则可以被测试钉住。
pub fn nvim_config_dir_in(config_home: &Path, app_name: &str) -> PathBuf {
    config_home.join(if app_name.is_empty() {
        "nvim"
    } else {
        app_name
    })
}

#[cfg(test)]
mod tests {
    use super::{has_nvim_config, nvim_config_dir_in, snippet_targets};
    use crate::build_method::scaffold::{BLINK_SNIPPET_FILE, NVIM_SNIPPET_FILE};
    use std::path::{Path, PathBuf};

    /// The Neovim config directory follows Neovim's own rule, and the nvim
    /// snippets land in the rust-only directory LuaSnip scans.
    /// Neovim 配置目录遵循 Neovim 自身的规则，nvim snippet 落在 LuaSnip 扫描的 rust
    /// 专属目录里。
    #[test]
    fn nvim_snippet_path_follows_the_editor_config() {
        assert_eq!(
            nvim_config_dir_in(Path::new("/home/x/.config"), "nvim"),
            PathBuf::from("/home/x/.config/nvim")
        );
        assert_eq!(
            nvim_config_dir_in(Path::new("/home/x/.config"), "nvchad"),
            PathBuf::from("/home/x/.config/nvchad")
        );
        assert_eq!(
            nvim_config_dir_in(Path::new("/home/x/.config"), ""),
            PathBuf::from("/home/x/.config/nvim")
        );
        let target = nvim_config_dir_in(Path::new("/tmp/cfg"), "nvim").join(NVIM_SNIPPET_FILE);
        assert_eq!(
            target,
            PathBuf::from("/tmp/cfg/nvim/luasnippets/rust/xirang-face.lua")
        );
        let blink = nvim_config_dir_in(Path::new("/tmp/cfg"), "nvim").join(BLINK_SNIPPET_FILE);
        assert_eq!(
            blink,
            PathBuf::from("/tmp/cfg/nvim/snippets/rust/xirang-face.json")
        );
    }

    /// The auto installer finds each editor's user-level snippet location, and
    /// writes both Neovim files because it cannot know which engine is loaded.
    /// 自动安装会找到每个编辑器的用户级 snippet 位置；Neovim 两个文件都写，因为无法得知
    /// 它加载的是哪个引擎。
    #[test]
    fn auto_targets_follow_the_installed_editors() {
        let root = std::env::temp_dir().join(format!(
            "xirang-auto-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        for dir in ["nvim", "Code", "VSCodium"] {
            std::fs::create_dir_all(root.join(dir)).expect("editor config");
        }
        let names = |fuzzy: bool| {
            snippet_targets(&root, fuzzy)
                .iter()
                .map(|target| {
                    format!(
                        "{} {}",
                        target.editor.name(),
                        xirang_kernel::declaration::portable_path(
                            &target
                                .path
                                .strip_prefix(&root)
                                .expect("under root")
                                .display()
                                .to_string()
                        )
                    )
                })
                .collect::<Vec<_>>()
        };
        // By default only the engines that match a snippet prefix as a prefix.
        assert_eq!(
            names(false),
            [
                "vscode Code/User/snippets/xirang-face.code-snippets",
                "vscode VSCodium/User/snippets/xirang-face.code-snippets",
            ]
        );
        assert!(has_nvim_config(&root));
        // Fuzzy-matching engines are installed only when asked for by name.
        assert_eq!(
            names(true),
            [
                "blink nvim/snippets/rust/xirang-face.json",
                "nvim nvim/luasnippets/rust/xirang-face.lua",
                "vscode Code/User/snippets/xirang-face.code-snippets",
                "vscode VSCodium/User/snippets/xirang-face.code-snippets",
            ]
        );
        // No editor at all is reported rather than guessed.
        assert!(snippet_targets(&root.join("missing"), true).is_empty());
        assert!(!has_nvim_config(&root.join("missing")));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
