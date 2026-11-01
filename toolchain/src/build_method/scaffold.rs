//! Project scaffolding for new XiRang host crates.
//! 新 XiRang 宿主 crate 的项目脚手架。
//!
//! Three parts live in sibling files: editor snippet generation (which renders the
//! face-field completion files every editor reads), editor snippet installation
//! (which probes each editor's user-level location and writes them there), and
//! project scaffolding (which renders and writes a new host). This file mounts
//! all three and re-exports the surface callers already use.
//! 三个部分位于同级文件：编辑器 snippet 生成（渲染每个编辑器读取的注册面字段补全
//! 文件）、编辑器 snippet 安装（探测各编辑器的用户级位置并写入其中）与项目脚手架
//! （渲染并写入新宿主）。本文件挂载三者，并重新导出调用方已经在用的表面。

#[path = "scaffold/install.rs"]
mod install;
#[path = "scaffold/project.rs"]
mod project;
#[path = "scaffold/snippets.rs"]
mod snippets;

pub use install::{
    EditorTarget, InstallReport, SnippetInstall, config_home, has_nvim_config, install_everywhere,
    nvim_config_dir, nvim_config_dir_in, snippet_targets,
};
pub use project::{
    DependencySource, ProjectKind, create_project, dependency_specs, detected_source,
    offline_source_warning, project_files, registry_release_present, requested_source,
};
pub use snippets::{
    BLINK_SNIPPET_FILE, Editor, NVIM_SNIPPET_FILE, SNIPPET_FILE, editor_snippets,
    write_editor_snippets, write_snippets_file,
};
