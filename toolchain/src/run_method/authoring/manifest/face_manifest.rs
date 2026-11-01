//! Stable registration-face metadata model.
//! 稳定的注册面元数据模型。

use std::collections::BTreeMap;

/// Parsed registration-face metadata shared by authoring and Studio.
/// authoring 与 Studio 共用的注册面元数据模型。
///
/// The struct used to carry `get()` and `fields()` accessors under
/// `#[allow(dead_code)]`. Neither had a caller — every consumer indexes
/// `values` directly, and Studio reads the same data through
/// `authoring::validation` — so they were the only reason the allow existed.
/// Both are gone: a zero-caller item returns when a caller appears, which is
/// this workspace's rule for deleting them.
/// 本结构曾在 `#[allow(dead_code)]` 下带着 `get()` 与 `fields()` 两个访问器。两者都没有
/// 调用者——每个消费者都直接索引 `values`，而 Studio 经 `authoring::validation` 读取同一份
/// 数据——它们正是那个 allow 存在的唯一原因。两者都已删除：零调用者的项在真实调用者出现时
/// 回来，这是本工作区删除它们的规则。
#[derive(Clone, Debug)]
pub(crate) struct FaceManifest {
    pub(crate) values: BTreeMap<String, String>,
}
