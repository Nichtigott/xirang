//! The authoring view of one graft declaration.
//! 一条 graft 声明的创作视图。
//!
//! The view types and the one cut/face matching rule live here. Reading the
//! host entry and mapping cut paths to modules live in the sibling modules, so
//! both the build and an authoring surface read one vocabulary.
//! 视图类型与唯一的切口/注册面匹配规则位于此处。读取宿主入口、把切口路径映射为
//! 模块位于同级模块，因此构建与创作界面读取同一套词汇。

use std::path::PathBuf;

/// One graft declaration the build step found in the host entry.
/// 构建步骤在宿主入口里发现的一条 graft 声明。
///
/// This is the authoring view of the same declaration the build captures into
/// the static plan: it exists so a tool can tell an author whether the slot
/// they are about to graft is shipped, without re-implementing the entry rule.
/// 这是构建会捕获进静态计划的那条声明的创作视图：它存在的意义是让工具无需重新
/// 实现入口规则，就能告诉作者准备嫁接的槽位会不会被发布。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGraft {
    /// Logical path, or the Rust expression text of a typed cut. For a range
    /// this is the start endpoint.
    /// 逻辑路径，或类型化切口的 Rust 表达式原文。区间切口这里是起点端点。
    pub cut: String,
    /// The far endpoint of a sibling range, as data.
    /// 兄弟区间的远端端点，以数据形式携带。
    ///
    /// The build's parser decides range-ness once and stores the far endpoint
    /// here; this view must carry it rather than re-deriving it from `cut`, or a
    /// logical path that literally contains `" to "` becomes indistinguishable
    /// from a range. Pinned by `names_face`'s range test in
    /// `build_method/src/graft_plan_check.rs`.
    /// 构建的解析器只判定一次区间性并把远端存进这里；本视图必须携带它，而不是从
    /// `cut` 重新推导，否则字面含有 `" to "` 的逻辑路径会与区间无法区分。由
    /// `build_method/src/graft_plan_check.rs` 中 `names_face` 的区间测试钉住。
    pub cut_end: Option<String>,
    /// The replacement side: an implementation selector, or the Rust expression
    /// text of a typed graft.
    /// 替换侧：实现选择器，或类型化 graft 的 Rust 表达式原文。
    pub graft: String,
    /// Whether the replacement covers the cut target's whole subtree rather than
    /// only its node.
    /// 替换是否覆盖切口目标的整棵子树，而不只是该节点自身。
    pub full: bool,
    /// The `cfg` gate the declaration carries, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
    pub cfg: Option<String>,
    /// Rust expressions, when the cut was written in the typed form.
    /// 切口写成类型化形式时的 Rust 表达式。
    pub expressions: Option<DeclaredGraftExpressions>,
    /// 1-based line of the declaration in the entry.
    /// 声明在入口中的 1 起始行号。
    pub line: usize,
}

/// Rust expressions used by one typed graft cut, in source order.
/// 一条类型化 graft 切口使用的 Rust 表达式，按源码顺序。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGraftExpressions {
    /// The cut target's Rust expression text; for a range, its start endpoint.
    /// 切口目标的 Rust 表达式原文；区间时是起点端点。
    pub cut: String,
    /// The far endpoint of a typed range, as data.
    /// 类型化区间的远端端点，以数据形式携带。
    pub cut_end: Option<String>,
    /// The replacement's Rust expression text.
    /// 替换件的 Rust 表达式原文。
    pub graft: String,
}

/// The graft declarations one package's host entry declares.
/// 一个包的宿主入口声明的 graft 切口。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGrafts {
    /// The entry the build step would read.
    /// 构建步骤会读取的入口。
    pub entry: PathBuf,
    /// Every declaration the entry carries, in source order.
    /// 入口携带的每一条声明，按源码顺序。
    pub cuts: Vec<DeclaredGraft>,
}

impl DeclaredGraft {
    /// The cut's endpoints rendered as one string, the way the author wrote it.
    /// 把切口的端点渲染成一个字符串，与作者写下的形式一致。
    ///
    /// Delegates to `graft_cut_label`, which owns the format; the build writes
    /// that text into `graft_plan.tsv` and the CLI prints it, so both read one
    /// rule instead of two.
    /// 委托给拥有该格式的 `graft_cut_label`；构建把这段文本写进 `graft_plan.tsv`，CLI
    /// 打印它，因此两者读的是同一条规则而不是两条。
    pub fn cut_label(&self) -> String {
        graft_cut_label(&self.cut, self.cut_end.as_deref())
    }

    /// Which spelling the declaration used: the string form, a range, or the
    /// typed Rust form.
    /// 该声明用的是哪种写法：字符串形式、区间，还是类型化 Rust 形式。
    ///
    /// The rule is a property of the declaration, so it lives on the declaration.
    /// It used to be a private helper in the CLI's `explain`, which meant the MCP
    /// bridge's overlay projection could only label a cut by re-deriving the same
    /// match — the drift this module exists to prevent. A typed cut is
    /// recognised by carrying expressions, and a range by carrying a far
    /// endpoint; neither is inferred from the text of `cut`, because a logical
    /// path may literally contain `" to "`.
    /// 这条规则是声明自身的属性，因此住在声明上。它过去是 CLI `explain` 的私有辅助
    /// 函数，意味着 MCP 桥的覆盖投影只能用同一份匹配再推导一次才能给切口贴标签——
    /// 而本模块存在的意义正是消除这种漂移。类型化切口靠携带表达式识别，区间靠携带
    /// 远端端点识别；两者都不从 `cut` 的文本推断，因为逻辑路径可能字面含有 `" to "`。
    pub fn form(&self) -> &'static str {
        match (&self.expressions, &self.cut_end) {
            (Some(_), Some(_)) => "typed-range",
            (Some(_), None) => "typed",
            (None, Some(_)) => "range",
            (None, None) => "string",
        }
    }

    /// Whether this declaration hands over exactly the face at the logical
    /// `path` whose source module is `module`.
    /// 这条声明是否正好交出逻辑路径为 `path`、源码模块为 `module` 的那个注册面。
    ///
    /// A typed cut names its target with a Rust path, so it is matched through
    /// the same module mapping the build uses; a string cut names the logical
    /// path, and a range names both of its endpoints. `module` is `None` when
    /// the target cannot be resolved in the current tree: nothing then proves
    /// which module a typed cut names, so only a string cut can match.
    /// 类型化切口用 Rust 路径命名目标，因此通过与构建相同的模块映射来匹配；字符串切口
    /// 命名逻辑路径；区间切口命名它的两个端点。目标在当前树里解析不出来时 `module` 为
    /// `None`：此时没有任何东西能证明类型化切口命名的是哪个模块，因此只有字符串切口
    /// 可能匹配。
    ///
    /// The build's static plan and the authoring surface both decide "this
    /// declaration is about that face" with this one rule.
    /// 构建的静态计划与创作界面都用这唯一一条规则判断"这条声明说的就是那个注册面"。
    pub fn names_face(&self, path: &str, module: Option<&str>) -> bool {
        match &self.expressions {
            Some(expressions) => {
                let Some(module) = module else {
                    return false;
                };
                let wanted = format!("{module}::NODE_ID");
                normalized_reference(&expressions.cut) == wanted
                    || expressions
                        .cut_end
                        .as_deref()
                        .is_some_and(|end| normalized_reference(end) == wanted)
            }
            None => {
                // The endpoints are already separate fields, so this compares
                // the two paths directly; splitting `cut` here would cut a path
                // that literally contains `" to "` in half.
                // 两个端点已经是独立字段，因此这里直接比较两条路径；在此拆分 `cut`
                // 会把字面含有 `" to "` 的路径拦腰截断。
                self.cut == path || self.cut_end.as_deref() == Some(path)
            }
        }
    }

    /// Whether a typed cut names exactly this Rust reference — the spelling the generated
    /// `xirang_external` module gives a face declared in another language.
    /// 类型化切口是否正好点名这条 Rust 引用——生成 `xirang_external` 模块给外语声明的面的拼写。
    ///
    /// `names_face` matches a foreign face by its **module**, which a typed cut can never spell
    /// (that module is a C++ header namespace, not a Rust path). The reference generated for it is
    /// the only Rust spelling that resolves, so it is matched here instead of widening `names_face`.
    /// `names_face` 用**模块**匹配外语面，而类型化切口根本拼不出那个模块（它是 C++ 头命名空间，不是
    /// Rust 路径）。为它生成的引用才是唯一能解析的 Rust 拼写，因此在这里匹配，而不是放宽 `names_face`。
    pub fn names_reference(&self, reference: &str) -> bool {
        let Some(expressions) = &self.expressions else {
            return false;
        };
        let wanted = normalized_reference(reference);
        normalized_reference(&expressions.cut) == wanted
            || expressions
                .cut_end
                .as_deref()
                .is_some_and(|end| normalized_reference(end) == wanted)
    }
}

/// One Rust reference without its `crate::`/`self::` prefix, for comparing a cut to a face.
/// 去掉 `crate::`/`self::` 前缀的一条 Rust 引用，用于把切口与被点名的面对上。
fn normalized_reference(value: &str) -> &str {
    value
        .strip_prefix("crate::")
        .or_else(|| value.strip_prefix("self::"))
        .unwrap_or(value)
}

/// Render a cut and its optional far endpoint as `"start"` or `"start to end"`.
/// 把切口与其可选远端端点渲染成 `"start"` 或 `"start to end"`。
///
/// The one implementation for every consumer that shows or persists the pair:
/// the build writes it as the human-readable `graft_plan.tsv` column and the
/// CLI's report renders the same text. It reads the two data fields and never
/// re-derives range-ness from `cut`, so a logical path that literally contains
/// `" to "` is written verbatim rather than split.
/// 每个需要展示或持久化这一对端点的消费方共用同一份实现：构建把它写成人类可读的
/// `graft_plan.tsv` 列，CLI 报告渲染同一段文本。它读取那两个数据字段，绝不从 `cut` 重新
/// 推导区间性，因此字面含有 `" to "` 的逻辑路径会原样写出而不是被拆分。
pub(crate) fn graft_cut_label(cut: &str, cut_end: Option<&str>) -> String {
    match cut_end {
        Some(end) => format!("{cut} to {end}"),
        None => cut.to_owned(),
    }
}
