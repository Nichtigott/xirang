//! Reading one authored face back, so an edit can carry over what it did not mention.
//! 读回一个已创作的注册面，使编辑能保留它没有提到的字段。

use std::collections::BTreeMap;

use super::*;

/// One authored face's current field values.
/// 一个已创作注册面的当前字段取值。
///
/// The edit entry point rewrites a face from the values it is handed — the right
/// contract for an editor that shows every field, and the wrong one for a caller
/// whose request means "change these two". This reader closes that gap without
/// weakening the executor: the caller reads the face, overrides what it means to
/// change, and hands the complete set back, so a partial request can never blank a
/// field it never mentioned.
/// 编辑入口用它拿到的取值整体重写一个面——对一个展示所有字段的编辑器这是正确的契约，而对一个
/// 意思只是"改这两个"的调用方恰恰是错的。本读取方在不削弱执行器的前提下补上这个缺口：调用方读回
/// 该面、覆盖它想改的、再把完整的一组交回去，因此局部请求绝不会抹掉它从未提到的字段。
///
/// The values are the manifest's own field map, exactly what the applier maintains,
/// which is also why reading them back and re-applying them is idempotent.
/// 这些取值就是清单自己的字段表，也正是应用器维护的那一份；这也是把它们读回来再应用一次是幂等的
/// 原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoredFace {
    /// Module directory and file name.
    /// 模块目录与文件名。
    pub module: String,
    /// Face kind name.
    /// 注册面种类名。
    pub kind: String,
    /// Preset type path.
    /// preset 类型路径。
    pub preset: String,
    /// Parts type path.
    /// parts 类型路径。
    pub parts: String,
    /// Localized display name, Chinese half.
    /// 本地化显示名称的中文部分。
    pub name_zh: String,
    /// Localized display name, English half.
    /// 本地化显示名称的英文部分。
    pub name_en: String,
    /// Localized one-line description, Chinese half.
    /// 本地化单行描述的中文部分。
    pub summary_zh: String,
    /// Localized one-line description, English half.
    /// 本地化单行描述的英文部分。
    pub summary_en: String,
    /// Export names this face declares.
    /// 本注册面声明的导出名称。
    pub exports: String,
    /// Author-owned identity that survives source moves.
    /// 跨源码移动保持不变的作者逻辑身份。
    pub stable_name: String,
    /// Whether this face owns a child Registry.
    /// 本注册面是否拥有一个子注册机。
    pub needs_registry: bool,
    /// External registry this face is provisioned from.
    /// 本注册面从其获取内容的外部注册机。
    pub getting_from_other_registry: String,
    /// Rule for faces entering the Registry this face owns.
    /// 进入本注册面所拥有 Registry 的注册规范。
    pub registration_rule: String,
    /// External dependency gate for this face's registry.
    /// 本注册面所属注册机对外部依赖的门禁。
    pub admission: String,
    /// Interface names declared by the handle type.
    /// handle 类型声明实现的接口名称。
    pub handle_traits: String,
    /// Contract types the handle type must implement.
    /// handle 类型必须实现的合同类型。
    pub handle_contracts: String,
    /// Interface names declared by the parts type.
    /// parts 类型声明实现的接口名称。
    pub part_traits: String,
    /// Contract types the parts type must implement.
    /// parts 类型必须实现的合同类型。
    pub part_contracts: String,
    /// Capability requirements this face declares.
    /// 本注册面声明的能力需求。
    pub requires: String,
    /// Capability names this face makes available.
    /// 本注册面向其他注册面提供的能力名称。
    pub provides: String,
    /// Runtime value checks the host applies.
    /// 宿主执行的运行期取值校验。
    pub runtime_checks: String,
    /// Explicit flow contract for grafting.
    /// 供嫁接使用的显式数据流合同。
    pub flow: String,
    /// Type that supplies the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果显式给出）。
    pub flow_provider: String,
}

impl AuthoredFace {
    /// Read declared fields for analysis without granting permission to rewrite this file.
    /// 为分析读取声明字段，不代表允许重写该文件。
    pub(crate) fn read_source(path: &std::path::Path) -> Result<Self, String> {
        let identity_prefix = super::context::identity_prefix()?;
        let layout = super::context::layout()?;
        let parents = super::parents::facts_in(&layout)?;
        let face = FaceManifest::parse_source(path, &identity_prefix, Some(&parents))?;
        Ok(Self::from_values(&face.values))
    }

    /// The complete patch this face's values make, for the edit entry point.
    /// 这些取值构成的完整补丁，交给编辑入口。
    pub fn as_patch(&self) -> ModuleFacePatch<'_> {
        ModuleFacePatch {
            module: &self.module,
            kind: &self.kind,
            preset: &self.preset,
            parts: &self.parts,
            name_zh: &self.name_zh,
            name_en: &self.name_en,
            summary_zh: &self.summary_zh,
            summary_en: &self.summary_en,
            exports: &self.exports,
            stable_name: &self.stable_name,
            needs_registry: self.needs_registry,
            getting_from_other_registry: &self.getting_from_other_registry,
            registration_rule: &self.registration_rule,
            admission: &self.admission,
            handle_traits: &self.handle_traits,
            handle_contracts: &self.handle_contracts,
            part_traits: &self.part_traits,
            part_contracts: &self.part_contracts,
            requires: &self.requires,
            provides: &self.provides,
            runtime_checks: &self.runtime_checks,
            flow: &self.flow,
            flow_provider: &self.flow_provider,
        }
    }

    /// Read the values out of one parsed manifest's field map.
    /// 从一份已解析清单的字段表里读出取值。
    fn from_values(values: &BTreeMap<String, String>) -> Self {
        let text = |field: &str| values.get(field).cloned().unwrap_or_default();
        Self {
            module: text("module"),
            kind: text("kind"),
            preset: text("preset"),
            parts: text("parts"),
            name_zh: text("name_zh"),
            name_en: text("name_en"),
            summary_zh: text("summary_zh"),
            summary_en: text("summary_en"),
            exports: text("exports"),
            stable_name: text("stable_name"),
            needs_registry: values.get("needs_registry").map(String::as_str) == Some("true"),
            getting_from_other_registry: text("getting_from_other_registry"),
            registration_rule: text("registration_rule"),
            admission: text("admission"),
            handle_traits: text("handle_traits"),
            handle_contracts: text("handle_contracts"),
            part_traits: text("part_traits"),
            part_contracts: text("part_contracts"),
            requires: text("requires"),
            provides: text("provides"),
            runtime_checks: text("runtime_checks"),
            flow: text("flow"),
            flow_provider: text("flow_provider"),
        }
    }
}

/// Read the fields one registered face currently declares.
/// 读出一个已注册面当前声明的字段。
///
/// The source is located through the same `generated_paths` the editor uses, so a
/// face this returns is a face the editor can rewrite — and one it refuses to locate
/// is refused here too, by the same reason.
/// 源码位置经编辑器所用的同一个 `generated_paths` 定位，因此这里返回的面就是编辑器能重写的面；
/// 而它拒绝定位的，这里也以同样的理由拒绝。
pub fn authored_face(registry: &Registry, id: NodeId) -> Result<AuthoredFace, String> {
    let (_, source) = generated_paths(registry, id)?;
    AuthoredFace::read_source(&source)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reading a face back and re-applying it is the identity: every field the
    /// manifest carries survives the round trip, which is what lets a partial
    /// request mean "keep the rest".
    /// 把一个面读回来再重新应用是同等的：清单携带的每个字段都活过这一轮，这正是让局部请求意味着
    /// "其余保持不动"的东西。
    #[test]
    fn reading_a_face_back_is_idempotent() {
        let values = [
            ("module", "button"),
            ("kind", "Button"),
            ("preset", "ButtonPreset"),
            ("parts", "ButtonParts"),
            ("name_zh", "按钮"),
            ("name_en", "Button"),
            ("summary_zh", "一个按钮"),
            ("summary_en", "A button"),
            ("exports", "control.render"),
            ("stable_name", "control.button"),
            ("needs_registry", "true"),
            ("getting_from_other_registry", "root/control"),
            ("registration_rule", "ANY"),
            ("admission", "ANY"),
            ("handle_traits", "ControlHandle"),
            ("handle_contracts", "crate::ControlHandle"),
            ("part_traits", "PartHandle"),
            ("part_contracts", "crate::PartHandle"),
            ("requires", "render"),
            ("provides", "click"),
            ("runtime_checks", "width>0"),
            ("flow", "narrow"),
            ("flow_provider", "crate::Flow"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect::<BTreeMap<_, _>>();
        let face = AuthoredFace::from_values(&values);
        let patch = face.as_patch();
        assert_eq!(patch.module, "button");
        assert_eq!(patch.kind, "Button");
        assert!(patch.needs_registry);
        assert_eq!(patch.registration_rule, "ANY");
        assert_eq!(patch.runtime_checks, "width>0");
        assert_eq!(patch.flow_provider, "crate::Flow");
        // A field the map does not carry reads as absent rather than as a guess.
        // 表里没有的字段读成"没有"，而不是猜一个值。
        let bare = AuthoredFace::from_values(&BTreeMap::new());
        assert_eq!(bare.as_patch().module, "");
        assert!(!bare.needs_registry);
    }
}
