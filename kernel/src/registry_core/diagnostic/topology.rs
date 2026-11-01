use super::*;

// ---------------------------------------------------------------------------
// Face topology validation. Pure graph checks over build-collected records;
// the build surface collects the records, the kernel owns the rules.
// 注册面拓扑校验。对构建期收集的记录做纯图检查；
// 记录由 build 面收集，规则归 kernel 所有。

/// One registration face participating in a topology check.
/// 参与拓扑校验的一个注册面。
#[derive(Clone, Debug)]
pub struct TopologyRecord {
    /// Identity of the face under check.
    /// 受检注册面的身份。
    pub id: NodeId,
    /// Identity of the face this one registers under; the package root for a
    /// top-level face.
    /// 本注册面所挂载的父面身份；顶层注册面则为包根。
    pub parent: NodeId,
    /// Whether this face provides a registry its children may register into.
    /// 该注册面是否提供可供子级注册的注册表。
    pub owns_registry: bool,
    /// Source file label the diagnostic points back to.
    /// 诊断指回的源文件标签。
    pub source: String,
}

/// Sort `records` by identity, then check duplicate identities, missing parents,
/// parents that do not own a registry, and parent cycles. Returns the collected
/// diagnostics.
/// 将 `records` 按身份排序，然后检查重复身份、缺失的父节点、父节点不持有注册表、
/// 以及父链成环；返回收集到的诊断。
pub fn validate_face_topology(
    records: &mut [TopologyRecord],
    package_root: NodeId,
    declared_elsewhere: &[(NodeId, bool)],
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    records.sort_by_key(|record| record.id);

    // Two records sharing one identity used to pass every check below: the sets and maps
    // are keyed by identity, so the later record simply replaced the earlier one. The
    // runtime's `plan_batch` refuses exactly this ("duplicate registration node identity
    // in batch"), and the static plan is the same tree one stage earlier — downstream
    // `StaticPlan::find` looks a node up by identity, so a duplicate makes which
    // declaration won a matter of insertion order (audit `LGC-LG-39`).
    // 两条记录共用一个身份过去能通过下面每一项检查：集合与映射都以身份为键，后一条只是替换了前一条。
    // 运行时的 `plan_batch` 明确拒绝这种情况（"duplicate registration node identity in batch"），
    // 而静态计划就是同一棵树早一个阶段的样子——下游 `StaticPlan::find` 按身份查节点，因此重复会让
    // "哪条声明赢了"变成插入顺序的问题（审计 `LGC-LG-39`）。
    for pair in records.windows(2) {
        if pair[0].id == pair[1].id {
            errors.push(
                BuildDiagnostic::new("static-plan", "duplicate registration node identity")
                    .at(pair[1].source.clone(), 0)
                    .field("id")
                    .expected("one declaration per node identity")
                    .actual(format!("`{}` is declared more than once", pair[1].id)),
            );
        }
    }

    // A parent may be declared in another language: it has no Rust record in this pass, but it is
    // still a node of this tree, and a Rust child that names it is not dangling. The caller hands
    // those identities over so the two checks below stay about "is the parent part of this tree",
    // not "was it written in Rust".
    // 父可能声明在别的语言里：这一趟它没有 Rust 记录，但它仍是这棵树的一个节点，点名它的 Rust 子面
    // 不是悬挂的。调用方把这些身份交进来，下面两条检查才是在问"父是不是这棵树的一部分"，而不是
    // "它是用 Rust 写的吗"。
    let mut ids = records
        .iter()
        .map(|record| record.id)
        .collect::<std::collections::BTreeSet<_>>();
    let mut owners = records
        .iter()
        .map(|record| (record.id, record.owns_registry))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (id, owns_registry) in declared_elsewhere {
        ids.insert(*id);
        owners.insert(*id, *owns_registry);
    }
    for record in records.iter() {
        if record.parent != package_root && !ids.contains(&record.parent) {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent node is missing")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registered parent")
                    .actual(record.parent.to_string()),
            );
        } else if record.parent != package_root && owners.get(&record.parent) == Some(&false) {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent does not own a registry")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registry owner")
                    .actual(record.parent.to_string()),
            );
        }
    }

    let parents = records
        .iter()
        .map(|record| (record.id, record.parent))
        .collect::<std::collections::BTreeMap<_, _>>();
    for record in records.iter() {
        let mut current = record.id;
        let mut seen = std::collections::BTreeSet::new();
        while current != package_root {
            if !seen.insert(current) {
                errors.push(
                    BuildDiagnostic::new("static-plan", "parent cycle detected")
                        .at(record.source.clone(), 0)
                        .field("parent")
                        .actual(current.to_string()),
                );
                break;
            }
            let Some(parent) = parents.get(&current).copied() else {
                break;
            };
            current = parent;
        }
    }
    errors
}

#[cfg(test)]
mod topology_tests {
    use super::{TopologyRecord, validate_face_topology};
    use crate::registry_core::identity::{NodeId, ROOT_NODE_ID};

    fn record(id: u8, parent: u8, owns_registry: bool) -> TopologyRecord {
        TopologyRecord {
            id: NodeId::from_raw([id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            parent: NodeId::from_raw([parent, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            owns_registry,
            source: format!("face-{id}.rs"),
        }
    }

    #[test]
    fn missing_parent_is_reported() {
        let mut records = vec![record(2, 9, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID, &[]);
        assert_eq!(
            errors
                .render_build_diagnostics()
                .matches("parent node is missing")
                .count(),
            1
        );
    }

    #[test]
    fn parent_without_registry_is_reported() {
        let mut records = vec![record(1, 0, false), record(2, 1, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID, &[]);
        assert_eq!(
            errors
                .render_build_diagnostics()
                .matches("parent does not own a registry")
                .count(),
            1
        );
    }

    #[test]
    fn parent_cycle_is_reported() {
        let mut records = vec![record(1, 2, true), record(2, 1, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID, &[]);
        assert_eq!(
            errors
                .render_build_diagnostics()
                .matches("parent cycle detected")
                .count(),
            2
        );
    }

    #[test]
    fn valid_chain_is_clean() {
        let root = ROOT_NODE_ID;
        let mut records = vec![
            TopologyRecord {
                parent: root,
                ..record(1, 0, true)
            },
            TopologyRecord {
                parent: NodeId::from_raw([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                ..record(2, 0, true)
            },
        ];
        let errors = validate_face_topology(&mut records, root, &[]);
        assert!(errors.is_empty());
    }

    /// Two records sharing one identity are reported, the way the runtime reports them.
    /// 两条记录共用一个身份会被报出，与运行时报的方式相同。
    ///
    /// Red before the fix: `ids`, `owners` and `parents` are keyed by identity, so the second
    /// record replaced the first and every check passed. The runtime's `plan_batch` refuses
    /// the same input ("duplicate registration node identity in batch"), and
    /// `StaticPlan::find` looks a node up by identity, so a duplicate made which declaration
    /// won a matter of insertion order (audit `LGC-LG-39`).
    /// 修前为红：`ids`、`owners`、`parents` 都以身份为键，因此第二条记录替换了第一条、每项检查都
    /// 通过。运行时 `plan_batch` 对同一输入明确拒绝（"duplicate registration node identity in
    /// batch"），而 `StaticPlan::find` 按身份查节点，因此重复会让"哪条声明赢了"变成插入顺序的问题
    /// （审计 `LGC-LG-39`）。
    #[test]
    fn duplicate_identity_is_reported() {
        let mut records = vec![record(1, 0, true), record(1, 0, false)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID, &[]);
        let rendered = errors.render_build_diagnostics();
        assert_eq!(
            rendered
                .matches("duplicate registration node identity")
                .count(),
            1,
            "the duplicate is named once, as the runtime names it: {rendered}"
        );
        assert!(
            rendered.contains(&record(1, 0, true).id.to_string()),
            "the diagnostic names the identity: {rendered}"
        );
    }
}
