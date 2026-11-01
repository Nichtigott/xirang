//! XiRang Studio: an optional Ratatui adapter for the registration core.
//! XiRang Studio：注册核心的可选 Ratatui 适配器。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

// The historical crate root (`studio/studio.rs`) keeps its file name, so the module nests:
// `xirang_toolchain::studio::studio`. That path is what the `conventions` shims ratchet pins,
// and the merge plan deliberately left the inner name alone (renaming it is a separate naming
// batch). `clippy::module_inception` is allowed here rather than renamed, so the exception sits
// next to the pin instead of being silent — and it only shows up under
// `--all-features --all-targets -- -D warnings`, the face CI's all-features job already runs
// (`ci.yml`) and the face this checkout's own landing set had never run until 2026-09-29.
// 历史的 crate 根（`studio/studio.rs`）保留文件名，因此模块同名嵌套：
// `xirang_toolchain::studio::studio`。这条路径正是 `conventions` 的 shims 棘轮钉住的，合并方案
// 也有意保留内层名（改它属于另一个命名批次）。这里对 `clippy::module_inception` 放行而不改名，
// 使例外就记在钉子旁边而不是静默存在——而且它只在
// `--all-features --all-targets -- -D warnings` 这一面出现：那一面 CI 的 all-features 作业本来就在跑
// （`ci.yml`），而本检出自己的落地门禁直到 2026-09-29 才第一次跑它。
#[allow(clippy::module_inception)]
#[path = "studio/studio.rs"]
mod studio;

pub use studio::{launch, launch_with};
