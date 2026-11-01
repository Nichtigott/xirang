//! What one Studio frame computes for the next frame, instead of for the session.
//! 一帧 Studio 为下一帧（而不是为会话状态）算出的东西。

use super::super::app::{App, HotZones, Overlay};

/// The click targets, the tree's scroll offset and the built call-graph widget — the
/// three things a draw produces.
/// 点击目标、树的滚动偏移，以及构建好的调用图控件——一次绘制产生的三样东西。
///
/// The render path used to write these straight into `App`, which made drawing an event
/// that changes the session: "the same state draws the same frame" was then not something a
/// reader — or a test — could rely on (audit `STU-S-04`). A frame now fills this cache, and
/// the caller applies it to `App` once the draw is over (`ui::draw_once`).
/// 渲染路径过去把这些直接写进 `App`，于是绘制成了一个会改变会话的事件：“同一状态画出同一帧”
/// 也就不是读者——或测试——能够依赖的东西（审计 `STU-S-04`）。现在一帧填充本缓存，绘制结束后由
/// 调用方（`ui::draw_once`）把它应用回 `App`。
#[derive(Clone, Debug, Default)]
pub struct RenderCache {
    /// Click targets this frame drew, seeded from the previous frame's.
    /// 本帧画出的点击目标，从上帧的那份起步。
    pub hot: HotZones,
    /// First visible row of the registration tree.
    /// 注册树的第一行可见行。
    pub tree_offset: usize,
    /// Where the search list's viewport ended up, when a search overlay was drawn. The list
    /// widget owns that viewport, so this is the one write the frame owes the session's
    /// overlay state.
    /// 搜索列表的视口最后停在哪里（画过搜索浮层时）。视口归列表控件所有，因此这是帧欠会话浮层
    /// 状态的那一笔写入。
    pub search_offset: Option<usize>,
    /// The built call-graph widget, keyed by the focus and the source stamp. It lives here
    /// for the duration of one frame: the pointer handler reads `App::graph_flow`, and
    /// `apply` puts it back before the next input event.
    /// 构建好的调用图控件，以焦点与源码戳为键。它只在一帧之内住在这里：指针处理器读的是
    /// `App::graph_flow`，而 `apply` 会在下一个输入事件之前把它放回去。
    #[cfg(feature = "node-graph")]
    pub graph_flow: Option<(String, usize, rataflow::Flow)>,
}

impl RenderCache {
    /// Start a frame from the state the previous frame left in `App`.
    /// 从上一帧留在 `App` 里的状态开始一帧。
    ///
    /// The call-graph widget is *taken* rather than copied: it is a widget tree, it belongs
    /// to exactly one frame at a time, and the pointer handler must not see two of them.
    /// 调用图控件是**取走**而不是复制：它是一棵控件树，同一时刻只属于一帧，指针处理器不得看到两份。
    pub(super) fn take(app: &mut App) -> Self {
        Self {
            hot: app.hot.clone(),
            tree_offset: app.tree_offset,
            search_offset: None,
            #[cfg(feature = "node-graph")]
            graph_flow: app.graph_flow.take(),
        }
    }

    /// Apply the frame's output to the session, once the draw is over.
    /// 绘制结束后，把这一帧的产出应用回会话。
    pub(super) fn apply(self, app: &mut App) {
        app.hot = self.hot;
        app.tree_offset = self.tree_offset;
        if let (Some(offset), Some(Overlay::Search(search))) =
            (self.search_offset, app.overlay.as_mut())
        {
            search.offset = offset;
        }
        #[cfg(feature = "node-graph")]
        {
            app.graph_flow = self.graph_flow;
        }
    }
}
