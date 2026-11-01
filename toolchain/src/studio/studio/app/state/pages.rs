//! Top-level workspace and focus vocabulary.
//! 顶层工作区与焦点词汇。

/// Which pane currently owns keyboard focus.
/// 当前拥有键盘焦点的面板。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    /// Keyboard focus is on the registry-tree pane.
    /// 键盘焦点在注册树面板。
    Tree,
    /// Keyboard focus is on the selected face's detail pane.
    /// 键盘焦点在所选注册面的详情面板。
    Details,
}

/// Top-level Studio workspace selected by the user.
/// Studio 用户当前选择的顶层工作区。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StudioPage {
    /// Search workspace: find faces and functions by text.
    /// 搜索工作区：按文本查找注册面与函数。
    Search,
    /// Inspect workspace: browse one face's fields and details.
    /// 检视工作区：浏览单个注册面的字段与详情。
    Inspect,
    /// Data workspace: provenance graph over the selected face.
    /// 数据工作区：围绕所选注册面的溯源图。
    Data,
    /// Multilingual workspace: the read-only `xirang.toml` inventory of the project.
    /// 多语言工作区：当前项目的只读 `xirang.toml` 单元清单。
    Multilang,
}

impl StudioPage {
    /// Uppercase tab label drawn for this workspace.
    /// 该工作区绘制的大写标签。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Search => "SEARCH",
            Self::Inspect => "INSPECT",
            Self::Data => "DATA",
            Self::Multilang => "MULTILANG",
        }
    }
}
