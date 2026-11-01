//! Atomic registration batches.
//! 原子注册批次。

use super::entry_pages::RegisteredEntry;
use std::sync::Arc;

use std::collections::{BTreeMap, BTreeSet};

use super::Registry;
use crate::registry_core::declaration::{RegistrationInfo, RegistrationSnapshot, SourceLocation};
use crate::registry_core::diagnostic::{DiagnosticSource, RegistryError, RegistryResult};
use crate::registry_core::identity::{NodeId, StableFaceId};

impl Registry {
    /// Submit one batch atomically. Failure leaves the receiver unchanged.
    /// 一批提交作为原子事务处理，失败时接收者完全不变。
    pub fn register_batch<I>(&mut self, submissions: I) -> RegistryResult<()>
    where
        I: IntoIterator<Item = RegistrationInfo>,
    {
        self.register_snapshot_batch(submissions.into_iter().map(RegistrationInfo::into_snapshot))
    }

    /// Register a borrowed slice of compiled declarations atomically.
    /// 原子注册一个借用的编译期声明切片。
    pub fn register_all(&mut self, registrations: &[RegistrationInfo]) -> RegistryResult<()> {
        self.register_batch(registrations.iter().copied())
    }

    /// Atomically attach file-backed snapshots without borrowing or leaking their metadata.
    /// 原子挂载文件快照，不借用也不泄漏快照中的元数据。
    pub fn register_snapshot_batch<I>(&mut self, submissions: I) -> RegistryResult<()>
    where
        I: IntoIterator<Item = RegistrationSnapshot>,
    {
        let submissions = submissions.into_iter().collect::<Vec<_>>();
        self.plan_batch(&submissions)?;
        let mut staged = self.clone();
        let mut pending = submissions;
        pending.sort_by_key(|snapshot| (snapshot.parent, snapshot.id));
        let mut failures = Vec::new();
        let mut progressed = true;
        while !pending.is_empty() && progressed {
            progressed = false;
            let mut waiting = Vec::new();
            for snapshot in pending {
                if staged.registry(snapshot.parent).is_some() {
                    match staged.submit_snapshot_at(snapshot.parent, snapshot) {
                        Ok(()) => progressed = true,
                        Err(error) => failures.push(*error),
                    }
                } else {
                    waiting.push(snapshot);
                }
            }
            pending = waiting;
        }
        // `plan_batch` above is the authoritative missing-parent check: it walks
        // every snapshot's parent chain against the *base* tree and rejects a
        // chain that never reaches a registered registry, all before any staging
        // happens. Because every snapshot in `pending` therefore has its parent
        // in the base tree or in an earlier-registered batch entry, the loop can
        // always make progress and cannot leave `pending` non-empty. The
        // historical trailing loop rebuilt the same `<missing-parent:…>` error
        // and was unreachable; keeping two copies risked the two drifting apart,
        // so it was removed.
        // 上面的 `plan_batch` 是缺父检查的权威：它在任何暂存之前，沿每个快照的父链对照
        // **基树**走一遍，拒绝走不到已注册注册机的链。因此 `pending` 里每个快照的父级要么
        // 在基树里、要么在更早注册的批次条目里，循环总能推进，不可能留下非空的 `pending`。
        // 历史上重建同一 `<missing-parent:…>` 错误的尾部循环因此不可达；留两份副本只会让
        // 它们彼此漂移，故删除。
        if failures.is_empty() {
            if let Some(error) = staged.connector_error() {
                return Err(error.into());
            }
            *self = staged;
            Ok(())
        } else {
            Err(Box::new(
                RegistryError::new(
                    self.header.id,
                    self.header.path.clone(),
                    DiagnosticSource::from(SourceLocation {
                        file: "<owned-snapshot-batch>",
                        line: 0,
                        column: 0,
                        function: "Registry::register_snapshot_batch",
                    }),
                    format!("snapshot batch rejected ({} error(s))", failures.len()),
                )
                .with_children(failures),
            ))
        }
    }

    /// Check the batch graph before touching the transactional working copy.
    /// 在创建事务副本前检查批次图，避免无意义的重复扫描。
    fn plan_batch(&self, submissions: &[RegistrationSnapshot]) -> RegistryResult<()> {
        let mut parents = BTreeMap::new();
        // A parent that lives in this batch answers children only when it owns a registry; the map of
        // batch entries is what lets the walk below tell `the parent is missing` from `the parent is
        // here and cannot own anything`.
        // 批次里的父只有自持注册机时才能挂住子面；批次条目表让下面的遍历能区分"父不在"与"父在、但什么也拥有不了"。
        let mut batch_registries = BTreeSet::new();
        let mut batch_by_id = BTreeMap::new();
        let mut stable_names = BTreeMap::<StableFaceId, (NodeId, String)>::new();
        let mut existing_ids = BTreeSet::new();
        self.collect_stable_names(&mut stable_names);
        self.collect_node_ids(&mut existing_ids);
        for snapshot in submissions {
            if snapshot.namespace != self.header.namespace {
                return Err(Box::new(RegistryError::new(
                    snapshot.id,
                    snapshot.registry_name.clone(),
                    snapshot.source.clone(),
                    format!(
                        "registration namespace `{}` does not match registry namespace `{}`",
                        snapshot.namespace, self.header.namespace
                    ),
                )));
            }
            if existing_ids.contains(&snapshot.id)
                || parents.insert(snapshot.id, snapshot.parent).is_some()
            {
                return Err(Box::new(RegistryError::new(
                    snapshot.id,
                    snapshot.registry_name.clone(),
                    snapshot.source.clone(),
                    format!(
                        "duplicate registration node identity `{}` in batch",
                        snapshot.id
                    ),
                )));
            }
            if let Some(stable) = snapshot.explicit_stable_face_id() {
                if let Some((existing, existing_path)) = stable_names.get(&stable) {
                    if *existing != snapshot.id {
                        return Err(Box::new(RegistryError::new(
                            snapshot.id,
                            snapshot.registry_name.clone(),
                            snapshot.source.clone(),
                            format!(
                                "duplicate stable face name `{}` (already owned by {} at {})",
                                snapshot.stable_name.as_deref().unwrap_or_default(),
                                existing,
                                existing_path
                            ),
                        )));
                    }
                } else {
                    stable_names.insert(stable, (snapshot.id, snapshot.source.file.clone()));
                }
            }
        }
        for snapshot in submissions {
            if snapshot.needs_registry {
                batch_registries.insert(snapshot.id);
            }
            batch_by_id.insert(snapshot.id, snapshot);
        }
        for snapshot in submissions {
            let mut current = snapshot.parent;
            let mut seen = BTreeSet::new();
            while let Some(parent) = parents.get(&current).copied() {
                if !seen.insert(current) {
                    return Err(Box::new(RegistryError::new(
                        snapshot.id,
                        snapshot.registry_name.clone(),
                        snapshot.source.clone(),
                        format!("registration parent cycle reaches `{current}`"),
                    )));
                }
                // A parent in this batch attaches children only when it owns a registry. This walk
                // used to step straight past that fact, and the staging loop then had nowhere to put
                // the child: the batch committed with fewer faces than it was given and reported
                // success (measured 2026-10-22 — a face declared in C++, under a parent declared in
                // C++ that never asked to own a registry, was dropped in silence). A node the caller
                // handed over is never dropped in silence.
                // 批次里的父只有自持注册机时才能挂住子面。这个遍历过去直接绕过那个事实，暂存循环于是无处
                // 安放子面：批次带着比交给它更少的面提交，还报成功（2026-10-22 实测——一个在 C++ 里声明的
                // 面，挂在同样由 C++ 声明、却从未要求自持注册机的父面下，被静默丢弃）。调用方交来的节点
                // 绝不能被静默丢掉。
                if !batch_registries.contains(&current) {
                    let owner = batch_by_id.get(&current).copied();
                    return Err(Box::new(RegistryError::new(
                        snapshot.id,
                        snapshot.registry_name.clone(),
                        snapshot.source.clone(),
                        format!(
                            "`{}` owns no registry, so `{}` has no parent registry to register into; a face owns children only when it declares `needs_registry`",
                            owner
                                .map(|owner| owner.kind.as_str())
                                .unwrap_or("<unknown>"),
                            snapshot.kind
                        ),
                    )));
                }
                current = parent;
            }
            if self.registry(current).is_none() {
                return Err(Box::new(RegistryError::new(
                    snapshot.id,
                    format!(
                        "<missing-parent:{}>/{}",
                        snapshot.parent, snapshot.registry_name
                    ),
                    snapshot.source.clone(),
                    format!(
                        "parent registry `{}` was not found for `{}`",
                        snapshot.parent, snapshot.kind
                    ),
                )));
            }
        }
        // The graph's identity keys are checked here, and a node's *path* is one of
        // them: two siblings may not carry the same `registry_name`. A path is not a
        // display string in this workspace — connector admission classifies a provider as
        // inside or outside its owner with `path_is_strictly_under`, and the graft cut
        // selector resolves its target *by path* — so two faces sharing one path made
        // "which face does this path name" unanswerable while three consumers silently
        // picked one: the
        // by-path index (last insert wins), `path_for` (two identical strings), and the
        // path arm of the cut selector (first match wins). A duplicate `stable_name` was
        // already refused; a duplicate `registry_name` was not, and the two are the same
        // kind of collision. Refusing it is also what keeps that path arm single-valued
        // rather than an arbitrary pick, because this batch is the only place new nodes are
        // admitted (audit `LGC-LG-07`).
        // 批次图的身份键在这里检查，而节点的**路径**就是身份键之一：两个兄弟不得携带相同的
        // `registry_name`。在本工作区里路径不是显示串——连接器准入用 `path_is_strictly_under`
        // 判定提供者在所有者之内还是之外，graft 切口选择器**按路径**解析它的目标——因此两个面共用一条
        // 路径会让"这条路径命名哪个面"无法回答，而三个消费方各自静默地选了一个：按路径的索引
        // （后插入者赢）、`path_for`（两段完全相同的字符串）、以及切口选择器的路径那一支（取第一个
        // 匹配）。重复的 `stable_name` 本来就被拒绝，重复的 `registry_name` 却没有，而两者是同一种
        // 碰撞。
        // 拒绝它也正是让那一支成为单值、而不是随意挑选的原因：本批次是唯一接纳新节点的地方
        // （审计 `LGC-LG-07`）。
        //
        // This block sits after the missing-parent walk on purpose: `run_method`'s two
        // READMEs anchor `transaction.rs:167` on the `<missing-parent:…>` message, and
        // adding lines above it drifted that anchor (the documentation-anchor gate caught
        // it). A check placed below the arm keeps the shipped reference true.
        // 这段刻意放在缺父遍历之后：`run_method` 的两份 README 把 `transaction.rs:167` 锚定在
        // `<missing-parent:…>` 那条消息上，在它之前加行会让锚点漂移（文档锚点门禁抓到了这一点）。
        // 放在该分支之后，出厂的引用就仍然为真。
        let mut sibling_names = BTreeMap::<NodeId, BTreeMap<String, (NodeId, String)>>::new();
        let mut seeded = BTreeSet::new();
        for snapshot in submissions {
            if seeded.insert(snapshot.parent)
                && let Some(parent) = self.registry(snapshot.parent)
            {
                let names = sibling_names.entry(snapshot.parent).or_default();
                for entry in parent.entries.values() {
                    names.insert(
                        entry.info.registry_name.clone(),
                        (entry.info.id, entry.info.source.file.clone()),
                    );
                }
            }
            let names = sibling_names.entry(snapshot.parent).or_default();
            match names.get(&snapshot.registry_name) {
                Some((existing, existing_source)) if *existing != snapshot.id => {
                    return Err(Box::new(RegistryError::new(
                        snapshot.id,
                        snapshot.registry_name.clone(),
                        snapshot.source.clone(),
                        format!(
                            "duplicate sibling registry name `{}` (already owned by {} at {})",
                            snapshot.registry_name, existing, existing_source
                        ),
                    )));
                }
                Some(_) => {}
                None => {
                    names.insert(
                        snapshot.registry_name.clone(),
                        (snapshot.id, snapshot.source.file.clone()),
                    );
                }
            }
        }
        Ok(())
    }

    fn submit_snapshot_at(
        &mut self,
        target: NodeId,
        snapshot: RegistrationSnapshot,
    ) -> RegistryResult<()> {
        let target_path = self
            .registry(target)
            .expect("the caller resolved the parent registry")
            .header
            .path
            .clone();
        let framework = self.header.framework;
        let registry = self
            .registry_mut(target)
            .expect("the caller resolved the parent registry");
        let rule_failures = registry
            .header
            .registration_rule
            .validate_owned_registration_rule(&snapshot);
        if !rule_failures.is_empty() {
            return Err(RegistryError::new(
                snapshot.id,
                format!("{target_path}/{}", snapshot.registry_name),
                snapshot.source.clone(),
                format!(
                    "registration rule rejected `{}` for registry `{target}` (rule `{}`): {}",
                    snapshot.kind,
                    registry.header.registration_rule_path,
                    rule_failures.join("; ")
                ),
            )
            .into());
        }
        let contract_failures = snapshot
            .contract
            .validate_owned_object_contract(&snapshot.kind);
        if !contract_failures.is_empty() {
            let mut error = RegistryError::new(
                snapshot.id,
                format!("{target_path}/{}", snapshot.registry_name),
                snapshot.source.clone(),
                "construction contract rejected registration",
            );
            *error.children_mut() = contract_failures
                .into_iter()
                .map(|failure| {
                    RegistryError::new(
                        snapshot.id,
                        format!("{target_path}/{}", snapshot.registry_name),
                        snapshot.source.clone(),
                        failure,
                    )
                })
                .collect();
            return Err(Box::new(error));
        }
        if snapshot
            .plugin
            .is_some_and(|plugin| plugin.mode == crate::PluginMode::Replacement)
            && !snapshot.flow.is_declared()
        {
            return Err(RegistryError::new(
                snapshot.id,
                format!("{target_path}/{}", snapshot.registry_name),
                snapshot.source.clone(),
                "replacement plugin must declare a flow contract",
            )
            .into());
        }
        let segment = snapshot.registry_name.clone();
        let child = snapshot.needs_registry.then(|| {
            Arc::new(Registry::new(
                framework,
                snapshot.namespace.clone(),
                &format!("{target_path}/{segment}"),
                snapshot.id,
                snapshot.registry_rule.clone(),
                snapshot.registry_rule_path.clone(),
                snapshot.admission.clone(),
            ))
        });
        Arc::make_mut(&mut registry.entries).insert(
            snapshot.id,
            RegisteredEntry {
                info: Arc::new(snapshot),
                child,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::declaration::{
        Admission, FrameworkId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
        OwnedSourceLocation, RegistrationRule,
    };
    use crate::registry_core::identity::root_node_id;

    fn snapshot(namespace: &str, kind: &str) -> RegistrationSnapshot {
        RegistrationSnapshot {
            namespace: namespace.to_owned(),
            id: NodeId::from_namespaced_path(namespace, "src/item.rs", kind),
            parent: root_node_id(namespace),
            kind: kind.to_owned(),
            preset: "NoPreset".to_owned(),
            parts: "NoParts".to_owned(),
            params: kind.to_owned(),
            handle: kind.to_owned(),
            stable_name: None,
            name: OwnedLocalizedText {
                zh: kind.to_owned(),
                en: kind.to_owned(),
            },
            summary: OwnedLocalizedText {
                zh: String::new(),
                en: String::new(),
            },
            exports: Vec::new(),
            needs_registry: false,
            registry_name: kind.to_owned(),
            getting_from_other_registry: None,
            registry_rule_path: "<test>".to_owned(),
            registry_rule: RegistrationRule::ANY.into_owned(),
            admission: Admission::ANY.into_owned(),
            requires: Vec::new(),
            provides: Vec::new(),
            contract: OwnedObjectContract {
                required_parts: Vec::new(),
                provided_parts: Vec::new(),
            },
            flow: OwnedFlowContract::none(),
            flow_provider: None,
            handle_traits: Vec::new(),
            part_traits: Vec::new(),
            runtime_checks: Vec::new(),
            plugin: None,
            source: OwnedSourceLocation {
                file: "src/item.rs".to_owned(),
                line: 1,
                column: 1,
                function: kind.to_owned(),
            },
        }
    }

    #[test]
    fn rejects_snapshot_from_another_namespace_before_mutation() {
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), "library-a");
        let error = registry
            .register_snapshot_batch([snapshot("library-b", "Thing")])
            .expect_err("cross-namespace snapshots must be rejected");
        assert!(error.to_string().contains("namespace"));
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn same_names_are_independent_in_two_registries() {
        let mut left = Registry::root_for_namespace(FrameworkId::new("test"), "library-a");
        let mut right = Registry::root_for_namespace(FrameworkId::new("test"), "library-a");
        left.register_snapshot_batch([snapshot("library-a", "Thing")])
            .unwrap();
        right
            .register_snapshot_batch([snapshot("library-a", "Thing")])
            .unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(left.find_kind("Thing").len(), 1);
        assert_eq!(right.find_kind("Thing").len(), 1);
    }

    #[test]
    fn parent_rule_is_a_minimum_shape_not_a_kind_filter() {
        let namespace = "structural-rule";
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);
        Arc::make_mut(&mut registry.header).registration_rule = RegistrationRule::new()
            .require_preset("ActionParts")
            .require_parts(&["paint"])
            .require_exports(&["control.render"])
            .require_handle_traits(&["ControlHandle"])
            .require_part_traits(&["ActionParts"])
            .into_owned();
        let mut child = snapshot(namespace, "AnyChildKind");
        child.preset = "ActionParts".to_owned();
        child.contract.provided_parts = vec!["paint".to_owned(), "extra".to_owned()];
        child.exports = vec!["control.render".to_owned(), "control.inspect".to_owned()];
        child.handle_traits = vec!["ControlHandle".to_owned(), "Debug".to_owned()];
        child.part_traits = vec!["ActionParts".to_owned(), "Clone".to_owned()];

        registry
            .register_snapshot_batch([child])
            .expect("a child may use any kind and provide more than the minimum shape");
    }

    #[test]
    fn parent_rule_aggregates_every_missing_structural_requirement() {
        let namespace = "broken-structural-rule";
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);
        Arc::make_mut(&mut registry.header).registration_rule = RegistrationRule::new()
            .require_preset("ActionParts")
            .require_parts(&["paint"])
            .require_exports(&["control.render"])
            .require_handle_traits(&["ControlHandle"])
            .require_part_traits(&["ActionParts"])
            .into_owned();

        let error = registry
            .register_snapshot_batch([snapshot(namespace, "Button")])
            .expect_err("missing parent requirements must reject the child")
            .to_string();

        for missing in [
            "preset `ActionParts` is required",
            "structural part `paint`",
            "export `control.render`",
            "interface `ControlHandle`",
            "interface `ActionParts`",
        ] {
            assert!(
                error.contains(missing),
                "missing diagnostic: {missing}\n{error}"
            );
        }
    }
}
