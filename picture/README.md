# README visuals / 首页视觉素材

`xirang-operations.svg` shows before/after registration trees for move, graft and partition. Gold marks a changed parent edge or implementation binding; dashed outlines mark compilation ownership. `xirang-graft.svg` shows four independent base/effective tree pairs: G1 single-face, G2 multi-cut group, G3 intermediate layer retaining children, and G4 whole-subtree replacement. The README usage section follows the same G1–G4 labels. This figure uses the control-button example, where the whole-subtree replacement has no children; replacements with children bring their own subtree. The English README uses these English diagrams; the Chinese README uses `xirang-operations.zh-CN.svg` and `xirang-graft.zh-CN.svg`. Code identifiers stay unchanged across languages.

操作图用注册树的前后对照表示移动、嫁接与分区：金色标出父级连线或实现绑定的变化，虚线框表示编译归属。嫁接详图用 G1–G4 四组独立的基树／有效树对照，分别展示局部替换、多个切口群替换、中间层保留子级、整子树替换。使用章节沿用同一编号；图中 control-button 示例的整子树替换件无子级，若替换件自带子树则一起接入。英文首页使用英文 SVG，中文首页使用 `.zh-CN.svg` 中文版本；两种语言中的代码标识符保持原名。

These are editable vector diagrams, not UI screenshots or evidence of universal automatic dispatch. Their meaning is also stated in the READMEs and alternative text. They reuse the existing mark's dark green, warm gold and neutral palette.

这些是可维护矢量示意图，不是界面截图，也不代表通用自动分派已实现。要点同时写在 README 与替代文本中，配色沿用现有标识的深绿、暖金和中性色。

The core-concept section uses `xirang-overview.svg` in English and `xirang-overview.zh-CN.svg` in Chinese. The three cards separate logical relationships, team source placement and compilation ownership. Architecture, team collaboration and language relationships use Mermaid diagrams.

核心理念章节使用 overview：英文为 `xirang-overview.svg`，中文为 `xirang-overview.zh-CN.svg`。三个面板区分逻辑关系、团队源码落点与编译归属；架构、团队协作与混合语言关系使用 Mermaid 表达。

`xirang-mark.svg` is a proposed XiRang mark: a shared foundation, a parent node, and two extension branches. It accompanies the existing brand assets rather than replacing their history.

`xirang-mark.svg` 是本次提供的标识方案：共同基础、父节点与两个扩展分支。旧字标与截图保留，未覆盖历史素材。

`xirang-overview.svg` is an editable concept illustration. Registration relationships, source placement, and compilation ownership are deliberately separate. It is not a screenshot or a claim that arbitrary remote materialization is implemented.

`xirang-overview.svg` 是可编辑概念图，明确区分注册关系、源码落点与编译归属；不是界面截图，也不表示任意远程物化已经实现。

`xirang-studio-current.webp` comes from the actual Studio process on 2026-10-25, in a temporary `demo` library host created by the CLI. The CLI created a Rust `Panel` and a `Reading` face under it at `components/reading/reading.rs`. Terminal size: 118 columns × 34 rows. The first ten rows, containing the legacy decorative banner, were cropped; the tree, inspector, event area, and controls retain their terminal text and colors. An external caption identifies the product. No registry rows were invented or edited.

`xirang-studio-current.webp` 采自 2026-10-25 的真实 Studio 进程。临时 `demo` 库宿主由 CLI 新建，CLI 创作 Rust `Panel` 及其位于 `components/reading/reading.rs` 的 `Reading` 子面。终端为 118 列 × 34 行。裁掉包含旧装饰横幅的前十行，树、详情、事件与操作栏保留实际文字和颜色；图外加产品标题，未添加或改写注册行。

The corresponding SVG preserves the terminal cells and is the editable rendering source. It was rasterized with resvg and encoded as WebP with ImageMagick. The CLI used for capture was built with `cli,lang-cpp`; its SHA-256 was `52a250777fa0bb3ebceca952a968683f14b5657ae88bec5a819c6453141b1621`. This identifies the capture, not a release.

对应 SVG 保留终端单元格，是可编辑渲染源；经 resvg 栅格化，再由 ImageMagick 编码为 WebP。采集所用 CLI 带 `cli,lang-cpp`，上述 SHA-256 只标识画面来源，不是发布声明。

The new artwork follows this repository's MIT license. All overview text also appears in the READMEs; the architecture diagram remains Mermaid so contributors can change it with the implementation.

新增图形沿用本仓 MIT 许可证。概念图的要点在 README 正文中也有说明；架构图使用 Mermaid，便于随实现维护。
