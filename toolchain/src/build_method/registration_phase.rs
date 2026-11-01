//! Build-time registration requirement analysis.
//! 构建期注册需求分析。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::diagnostics::BuildDiagnostics;
use super::registry_identity::NodeId;
use super::registry_syntax::parse_face;
use super::{CACHE_SCHEMA, cached_parent_id, relative_display};
use xirang_kernel::{CapabilityDeclaration, CapabilityRequirement, missing_capabilities};

#[derive(Clone, Debug)]
struct CachedFace {
    id: NodeId,
    parent: Option<NodeId>,
    kind: String,
    function: String,
    line: usize,
    requires: Vec<(String, String)>,
    provides: Vec<String>,
    contract: Vec<(String, String)>,
}

pub fn aggregate(
    src: &Path,
    include_demo: bool,
    active: Option<&BTreeSet<NodeId>>,
    cache_units: Option<&Path>,
) -> BuildDiagnostics {
    let mut requirements = Vec::new();
    let mut declarations = Vec::new();
    visit_registration_phase(
        src,
        src,
        include_demo,
        cache_units,
        &mut requirements,
        &mut declarations,
    );
    if let Some(active) = active {
        requirements.retain(|requirement| active.contains(&requirement.node));
        declarations.retain(|declaration| active.contains(&declaration.id));
    }

    let mut diagnostics = BuildDiagnostics::default();
    for diagnostic in missing_capabilities(requirements, &declarations) {
        diagnostics.push(diagnostic);
    }
    diagnostics
}

fn visit_registration_phase(
    root: &Path,
    dir: &Path,
    include_demo: bool,
    cache_units: Option<&Path>,
    requirements: &mut Vec<CapabilityRequirement>,
    declarations: &mut Vec<CapabilityDeclaration>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            let named = path.file_name().and_then(|name| name.to_str());
            if named != Some(xirang_kernel::lexicon::SCOPE_REGISTRATION_MODULE)
                && named.is_some_and(crate::build_method::source_walk::valid_name)
            {
                visit_registration_phase(
                    root,
                    &path,
                    include_demo,
                    cache_units,
                    requirements,
                    declarations,
                );
            }
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        if !include_demo
            && path.file_stem().and_then(|name| name.to_str())
                == Some(crate::build_method::DEMO_ONLY_DIRECTORY)
        {
            continue;
        }
        if let Some(face) = cache_units.and_then(|directory| cached_face(root, &path, directory)) {
            visit_cached_face(
                &relative_display(root, &path),
                face,
                requirements,
                declarations,
            );
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = relative_display(root, &path);
        // A face that does not parse is reported once by
        // `face_syntax_check::face_syntax_errors`, before this pass runs; skipping it
        // here is what keeps the diagnostic the only outcome instead of a panic.
        // 解析不了的注册面由 `face_syntax_check::face_syntax_errors` 在本轮之前报告一次；此处跳过
        // 它，正是让诊断成为唯一结果、而不是 panic 的原因。
        let Ok(Some(face)) = parse_face(&source) else {
            continue;
        };
        visit_face(root, &relative, face, requirements, declarations);
    }
}

fn visit_cached_face(
    relative: &str,
    face: CachedFace,
    requirements: &mut Vec<CapabilityRequirement>,
    declarations: &mut Vec<CapabilityDeclaration>,
) {
    let CachedFace {
        id,
        parent,
        kind,
        function,
        line,
        requires: face_requirements,
        provides,
        contract,
        ..
    } = face;
    // Keep contract fields available to the upcoming structural validator.
    let _contract_fields = contract;
    declarations.push(CapabilityDeclaration {
        id,
        parent,
        kind: kind.clone(),
        provides,
    });
    let branch = relative
        .rsplit_once('/')
        .map_or("<root>", |(branch, _)| branch)
        .to_owned();
    for (capability, provider) in face_requirements {
        requirements.push(CapabilityRequirement {
            node: id,
            kind: kind.clone(),
            function: function.clone(),
            branch: branch.clone(),
            capability,
            provider,
            parent,
            source: relative.to_owned(),
            line,
        });
    }
}

/// Read one cache unit only when its fingerprint still matches the source.
/// 只有 fingerprint 仍与源码匹配时才读取缓存单元。
fn cached_face(root: &Path, path: &Path, directory: &Path) -> Option<CachedFace> {
    let source = fs::read(path).ok()?;
    let mut input = path.to_string_lossy().as_bytes().to_vec();
    input.push(0);
    input.extend_from_slice(&source);
    let fingerprint = NodeId::from_bytes(&input).to_string();
    let content = fs::read_to_string(directory.join(format!("{fingerprint}.tsv"))).ok()?;
    if content
        .lines()
        .find_map(|line| line.strip_prefix("# schema\t"))
        != Some(CACHE_SCHEMA)
    {
        return None;
    }
    let facts = super::discovery_cache::read_unit(&content)?;
    let relative = relative_display(root, path);
    if facts.relative != relative {
        return None;
    }
    let id = facts.id.parse().ok()?;
    let kind = facts.kind.clone();
    if kind.is_empty() || id != super::registry_identity::package_node_id(&relative, &kind) {
        return None;
    }
    let parent = facts.parent.and_then(|value| value.parse().ok());
    let function = facts
        .handle
        .unwrap_or_else(|| "<registration-face>".to_owned());
    let line = usize::try_from(facts.line?).ok()?;
    Some(CachedFace {
        id,
        parent,
        kind,
        function,
        line,
        requires: facts.requires,
        provides: facts.provides,
        contract: facts.contract,
    })
}

fn visit_face(
    root: &Path,
    relative: &str,
    face: super::registry_syntax::FaceSyntax,
    requirements: &mut Vec<CapabilityRequirement>,
    declarations: &mut Vec<CapabilityDeclaration>,
) {
    let kind = face.path("kind").unwrap_or_else(|| "<unknown>".to_owned());
    let function = face
        .path("handle")
        .unwrap_or_else(|| "<registration-face>".to_owned());
    let node = super::registry_identity::package_node_id(relative, &kind);
    let parent = cached_parent_id(root, &face);
    declarations.push(CapabilityDeclaration {
        id: node,
        parent,
        kind: kind.clone(),
        provides: face.string_list("provides").unwrap_or_default(),
    });

    let line = face
        .field_location("requires")
        .map_or(face.location.line, |location| location.line);
    let branch = relative
        .rsplit_once('/')
        .map_or("<root>", |(branch, _)| branch)
        .to_owned();
    for (capability, provider) in face.requirements("requires").unwrap_or_default() {
        requirements.push(CapabilityRequirement {
            node,
            kind: kind.clone(),
            function: function.clone(),
            branch: branch.clone(),
            capability,
            provider,
            parent,
            source: relative.to_owned(),
            line,
        });
    }
}

#[cfg(test)]
#[path = "registration_phase_tests.rs"]
mod registration_phase_tests;
