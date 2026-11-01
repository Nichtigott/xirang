# README visuals / 首页视觉素材

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
