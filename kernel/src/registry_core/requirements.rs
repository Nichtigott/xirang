//! Capability requirements analysis over a registration declaration set.
//! Pure graph logic: the caller collects declarations and requirements from
//! wherever they live (build scans files, tools read snapshots) and the
//! kernel decides which requirements miss an ancestor provider.
//! 对注册声明集合做能力需求分析。纯图逻辑：由调用方从任意来源收集
//! 声明与需求（build 扫文件、工具读快照），kernel 判定哪些需求
//! 在祖先链上找不到提供者。

use std::collections::{BTreeMap, BTreeSet};

use crate::registry_core::diagnostic::BuildDiagnostic;
use crate::registry_core::identity::NodeId;

/// One registration declaration that may provide capabilities to its subtree.
/// 一条可能向子树提供能力的注册声明。
#[derive(Clone, Debug)]
pub struct CapabilityDeclaration {
    /// Identity of the declaring face.
    /// 作出声明的注册面身份。
    pub id: NodeId,
    /// Face this declaration registers under; `None` for a root declaration.
    /// 该声明所挂载的父面；根声明为 `None`。
    pub parent: Option<NodeId>,
    /// Registration kind a requirement's expected provider must match.
    /// 需求所期望的提供者必须匹配的注册类型。
    pub kind: String,
    /// Capability names this declaration offers to its subtree.
    /// 该声明向其子树提供的能力名称。
    pub provides: Vec<String>,
}

/// One `requires = ...` entry collected from a registration face.
/// 从注册面收集到的一条 `requires = ...` 需求。
#[derive(Clone, Debug)]
pub struct CapabilityRequirement {
    /// Identity of the face that declares the requirement.
    /// 声明该需求的注册面身份。
    pub node: NodeId,
    /// Registration kind of the requiring face.
    /// 需求方注册面的注册类型。
    pub kind: String,
    /// Logical function or handle the requirement was written in.
    /// 书写该需求所在的逻辑函数或 handle。
    pub function: String,
    /// Graft branch the requirement was collected under.
    /// 收集该需求时所在的 graft 分支。
    pub branch: String,
    /// Capability name the face needs from an ancestor.
    /// 该注册面需要祖先提供的能力名称。
    pub capability: String,
    /// Expected registration kind of the ancestor that must provide it.
    /// 必须提供该能力的祖先的期望注册类型。
    pub provider: String,
    /// Where the ancestor search starts; `None` means no ancestor can satisfy
    /// it, so the requirement is always reported.
    /// 祖先搜索的起点；`None` 表示没有祖先能满足它，因此该需求总会被报告。
    pub parent: Option<NodeId>,
    /// Source file the requirement was written in.
    /// 书写该需求的源文件。
    pub source: String,
    /// 1-based line inside `source`.
    /// `source` 内以 1 起始的行号。
    pub line: usize,
}

/// Every requirement whose ancestor chain offers no matching provider.
/// 祖先链上找不到匹配提供者的全部需求，逐条生成诊断。
pub fn missing_capabilities(
    requirements: Vec<CapabilityRequirement>,
    declarations: &[CapabilityDeclaration],
) -> Vec<BuildDiagnostic> {
    let index = CapabilityIndex::new(declarations);
    requirements
        .into_iter()
        .filter_map(|requirement| missing(requirement, &index))
        .collect()
}

fn missing(
    requirement: CapabilityRequirement,
    index: &CapabilityIndex<'_>,
) -> Option<BuildDiagnostic> {
    let provided = requirement
        .parent
        .and_then(|parent| index.provider(parent, &requirement.capability, &requirement.provider));
    if provided.is_some() {
        return None;
    }
    let detail = requirement
        .parent
        .and_then(|parent| index.ancestor(parent, &requirement.capability, None))
        .map(|declaration| declaration.kind.clone());
    let mut diagnostic = BuildDiagnostic::new(
        "requirements",
        format!("missing capability `{}`", requirement.capability),
    )
    .branch(requirement.branch)
    .node(requirement.node.to_string(), requirement.kind)
    .at(requirement.source, requirement.line)
    .function(requirement.function)
    .field(requirement.capability)
    .expected(requirement.provider.clone());
    if let Some(provider) = detail {
        diagnostic = diagnostic.provider(provider);
    }
    Some(diagnostic)
}

/// Request-local identity index shared by validation and responsibility reports.
/// 校验与责任报告共用的请求内身份索引。
pub struct CapabilityIndex<'a> {
    declarations: BTreeMap<NodeId, Vec<&'a CapabilityDeclaration>>,
}

impl<'a> CapabilityIndex<'a> {
    /// Index declarations once, preserving multiple declarations at one identity.
    /// 对声明一次建表，保留同一身份的多个声明。
    pub fn new(declarations: &'a [CapabilityDeclaration]) -> Self {
        let mut indexed: BTreeMap<NodeId, Vec<&CapabilityDeclaration>> = BTreeMap::new();
        for declaration in declarations {
            indexed.entry(declaration.id).or_default().push(declaration);
        }
        Self {
            declarations: indexed,
        }
    }

    /// Resolve the nearest matching ancestor, starting at the consumer's parent.
    /// 从消费者的父级起，解析最近的匹配祖先。
    pub fn provider(
        &self,
        parent: NodeId,
        capability: &str,
        expected_kind: &str,
    ) -> Option<&'a CapabilityDeclaration> {
        self.ancestor(parent, capability, Some(expected_kind))
    }

    fn ancestor(
        &self,
        mut target: NodeId,
        capability: &str,
        expected_kind: Option<&str>,
    ) -> Option<&'a CapabilityDeclaration> {
        let mut visited = BTreeSet::new();
        while visited.insert(target) {
            let declarations = self.declarations.get(&target)?;
            if let Some(declaration) = declarations.iter().find(|declaration| {
                expected_kind.is_none_or(|kind| declaration.kind == kind)
                    && declaration.provides.iter().any(|item| item == capability)
            }) {
                return Some(declaration);
            }
            target = declarations.first()?.parent?;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityDeclaration, CapabilityRequirement, missing_capabilities};
    use crate::registry_core::identity::{NodeId, ROOT_NODE_ID};

    fn id(byte: u8) -> NodeId {
        NodeId::from_raw([byte, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    fn declaration(
        byte: u8,
        parent: Option<u8>,
        kind: &str,
        provides: &[&str],
    ) -> CapabilityDeclaration {
        CapabilityDeclaration {
            id: id(byte),
            parent: parent.map(id),
            kind: kind.to_owned(),
            provides: provides.iter().map(|item| item.to_string()).collect(),
        }
    }

    fn requirement(
        byte: u8,
        parent: Option<u8>,
        capability: &str,
        provider: &str,
    ) -> CapabilityRequirement {
        CapabilityRequirement {
            node: id(byte),
            kind: "Leaf".to_owned(),
            function: "register".to_owned(),
            branch: "a".to_owned(),
            capability: capability.to_owned(),
            provider: provider.to_owned(),
            parent: parent.map(id),
            source: "a/leaf.rs".to_owned(),
            line: 3,
        }
    }

    #[test]
    fn satisfied_requirement_produces_no_diagnostic() {
        let declarations = vec![
            declaration(1, None, "CanvasProvider", &["render"]),
            declaration(2, Some(1), "Leaf", &[]),
        ];
        let requirements = vec![requirement(2, Some(1), "render", "CanvasProvider")];
        assert!(missing_capabilities(requirements, &declarations).is_empty());
    }

    #[test]
    fn missing_capability_is_reported_with_ancestor_detail() {
        let declarations = vec![
            declaration(1, None, "WrongProvider", &["render"]),
            declaration(2, Some(1), "Leaf", &[]),
        ];
        let requirements = vec![requirement(2, Some(1), "render", "CanvasProvider")];
        let missing = missing_capabilities(requirements, &declarations);
        assert_eq!(missing.len(), 1);
        let rendered = missing[0].clone().message;
        assert!(rendered.contains("missing capability `render`"));
    }

    #[test]
    fn root_requirement_without_parent_is_reported() {
        let declarations = vec![declaration(1, None, "Root", &[])];
        let requirements = vec![requirement(1, None, "render", "CanvasProvider")];
        assert_eq!(missing_capabilities(requirements, &declarations).len(), 1);
    }

    #[test]
    fn parent_cycle_does_not_hang() {
        let mut root = declaration(1, Some(2), "A", &[]);
        root.parent = Some(id(2));
        let mut other = declaration(2, Some(1), "B", &[]);
        other.parent = Some(id(1));
        let declarations = vec![root, other];
        let requirements = vec![requirement(2, Some(1), "render", "CanvasProvider")];
        let missing = missing_capabilities(requirements, &declarations);
        assert_eq!(missing.len(), 1);
        let _ = ROOT_NODE_ID;
    }

    #[test]
    fn resolution_ignores_other_branches_and_uses_the_nearest_matching_kind() {
        let declarations = vec![
            declaration(9, None, "Provider", &["x"]),
            declaration(1, None, "Provider", &["x"]),
            declaration(2, Some(1), "Provider", &["x"]),
            declaration(3, Some(2), "Other", &["x"]),
            declaration(4, Some(3), "Leaf", &[]),
        ];
        let index = super::CapabilityIndex::new(&declarations);
        assert_eq!(index.provider(id(3), "x", "Provider").unwrap().id, id(2));
        assert_eq!(index.provider(id(1), "x", "Provider").unwrap().id, id(1));
        assert!(index.provider(id(3), "absent", "Provider").is_none());
        assert!(index.provider(id(99), "x", "Provider").is_none());
        assert!(
            missing_capabilities(
                vec![requirement(4, Some(3), "x", "Provider")],
                &declarations
            )
            .is_empty()
        );
    }
}
