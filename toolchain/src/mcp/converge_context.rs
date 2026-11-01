//! Change obligations from the validated registry's own authoring read-back.
//! 从已校验注册机自己的创作读回取得修改约束。

use std::collections::BTreeMap;
use std::fmt::Write;

use xirang_kernel::NodeId;

use crate::build_method::FaceView;
use crate::run_method::AuthoredFace;

/// Keep contracts, family and declared consumers together, without inventing runtime call edges.
/// 将合同、同族与声明消费者放在一起，不编造运行期调用边。
pub(super) fn change_context(
    faces: &[FaceView],
    face: &FaceView,
    declarations: &super::chain::Declarations,
    limit: usize,
) -> String {
    let mut output = String::from(
        "change context: current declarations; capability consumers are not runtime call edges\n",
    );
    let consumer_ids: std::collections::BTreeSet<_> = declarations
        .edges
        .iter()
        .filter(|edge| edge.provider == Some(face.id))
        .map(|edge| edge.consumer)
        .collect();
    let consumers: Vec<_> = faces
        .iter()
        .filter(|candidate| consumer_ids.contains(&candidate.id))
        .collect();
    face_rows(&mut output, "declared consumers", &consumers, limit);
    let siblings: Vec<_> = faces
        .iter()
        .filter(|candidate| candidate.parent == face.parent && candidate.id != face.id)
        .collect();
    face_rows(&mut output, "siblings", &siblings, limit);
    output.push_str("  changes to contracts/providers need consumer checks; parts deepen within this face, grafts replace a face\n");
    output
}

/// Contracts attached to this identity and its registration parent.
/// 此身份与注册父级的合同。
pub(super) fn contracts(face: &FaceView, declarations: &BTreeMap<NodeId, AuthoredFace>) -> String {
    let mut output = String::new();
    if let Some(own) = declarations.get(&face.id) {
        for (label, value) in [
            ("parts", &own.parts),
            ("preset", &own.preset),
            ("exports", &own.exports),
            ("handle contracts", &own.handle_contracts),
            ("part contracts", &own.part_contracts),
            ("handle traits", &own.handle_traits),
            ("part traits", &own.part_traits),
            ("provides", &own.provides),
            ("runtime checks", &own.runtime_checks),
            ("flow", &own.flow),
            ("flow provider", &own.flow_provider),
            ("registration rule", &own.registration_rule),
            ("admission", &own.admission),
        ] {
            field_line(&mut output, label, value);
        }
        if let Some(parent) = declarations.get(&face.parent) {
            field_line(
                &mut output,
                "parent registration rule",
                &parent.registration_rule,
            );
            field_line(&mut output, "parent admission", &parent.admission);
        }
    } else {
        output.push_str("  contract fields unreadable; inspect the face declaration\n");
    }
    output
}

fn field_line(output: &mut String, label: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    let shown: String = value.chars().take(180).collect();
    let suffix = if shown.len() < value.len() {
        " … [truncated; read the declaration]"
    } else {
        ""
    };
    let _ = writeln!(output, "  {label}: {shown}{suffix}");
}

fn face_rows(output: &mut String, label: &str, faces: &[&FaceView], limit: usize) {
    let limit = limit.min(12);
    let _ = writeln!(output, "{label} {}", faces.len());
    for face in faces.iter().take(limit) {
        let _ = writeln!(
            output,
            "  {} — src/{}",
            face.path,
            face.source.trim_start_matches("./")
        );
    }
    if faces.len() > limit {
        let _ = writeln!(
            output,
            "  {} {label} withheld; usages {{node}} / registry {{full:true}} for details",
            faces.len() - limit
        );
    }
}
