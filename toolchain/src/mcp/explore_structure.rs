//! Connect exploration targets to native object responsibility, without widening explicit scopes.
//! 将探索目标连接到原生对象责任，不扩大显式范围。
//! online: target ownership is checked against current declarations; a published build cannot certify unbuilt edits or later observations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{Selection, SourceFile};
use crate::build_method::{FaceView, face_views, parsed_face, source_layout};
use crate::mcp::{converge, registry::namespace, source_index::load_one};
use xirang_kernel::NodeId;

pub(super) struct Joined {
    packet: converge::Packet,
    paths: BTreeMap<String, bool>,
    /// Every registered face's source path mapped to its logical path, whether or not the packet
    /// selected it: a repair projection names the public route of a file it is already showing, and
    /// that file's face need not be one of the neighbours the packet pulled in.
    /// 每个注册面的源码路径映射到它的逻辑路径，无论 packet 是否选中它：修复投影要为**已经展示的**文件点名
    /// 公开路线，而该文件的面不一定是 packet 拉进来的邻居之一。
    faces: BTreeMap<String, String>,
    focus: String,
    notice: String,
}

impl Joined {
    /// Logical object paths can select a source neighbourhood before the general source walk.
    /// 逻辑对象路径可在通用源码遍历前选择源码邻域。
    pub fn explicit(root: &Path, query: &str, limit: usize) -> Result<Option<Self>, String> {
        let targets: Vec<_> = query.split_whitespace().collect();
        if !targets.iter().all(|target| target.starts_with("root/")) {
            return Ok(None);
        }
        let namespace = namespace(root)?;
        let faces = crate::source_cache::phase("face_views", || face_views(root, &namespace))?;
        let seeds = targets
            .iter()
            .map(|target| crate::mcp::resolve::resolve_in(&namespace, target, &faces))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if seeds.is_empty()
            || seeds
                .iter()
                .any(|id| !faces.iter().any(|face| face.id == *id))
        {
            return Err("no face in the derived tree has that identity".into());
        }
        let mut joined = Self::for_nodes(root, &namespace, &faces, &seeds, limit)?;
        joined.notice = "structural source scope: package declarations examined; source/caller/test search reads selected face files only; outside files not examined. Query a symbol without scope for whole-tree lexical callers/tests.\n".to_owned();
        Ok(Some(joined))
    }

    /// Only real declarations near selected targets trigger the package-wide join.
    /// 只有选定目标附近的真实声明才触发包级拼接。
    pub fn from_targets(
        root: &Path,
        sources: &[SourceFile],
        selections: &BTreeMap<usize, Selection>,
        limit: usize,
    ) -> Result<Option<Self>, String> {
        let by_path: BTreeMap<_, _> = sources
            .iter()
            .map(|file| (file.relative.as_str(), file))
            .collect();
        let mut owners = BTreeSet::new();
        let mut examined = 0;
        for (id, _) in selections
            .iter()
            .filter(|(_, selection)| selection.rank == 0)
            .take(limit)
        {
            let file = &sources[*id];
            examined += 1;
            if parsed_face(&file.source, &file.relative).is_some() {
                owners.insert(file.relative.clone());
                continue;
            }
            // Native face directories own their parts, but never invent a face from a directory name.
            // 原生面的目录容纳内部部件，但不能仅凭目录名编造注册面。
            let mut directory = Path::new(&file.relative).parent();
            while let Some(parent) = directory {
                if let Some(name) = parent.file_name().and_then(|name| name.to_str()) {
                    let candidate = parent
                        .join(format!("{name}.rs"))
                        .to_string_lossy()
                        .replace('\\', "/");
                    if let Some(owner) = by_path.get(candidate.as_str())
                        && parsed_face(&owner.source, &owner.relative).is_some()
                    {
                        owners.insert(candidate);
                        break;
                    }
                }
                directory = parent.parent();
            }
        }
        if owners.is_empty() {
            return Ok(None);
        }
        let namespace = namespace(root)?;
        let faces = crate::source_cache::phase("face_views", || face_views(root, &namespace))?;
        let paths = source_paths(root, &faces)?;
        let seeds: BTreeSet<_> = paths
            .iter()
            .filter(|(_, path)| owners.contains(*path))
            .map(|(id, _)| *id)
            .collect();
        if seeds.is_empty() {
            return Err("selected declarations are outside this package's native face tree (possibly external); source remains available".into());
        }
        let mut joined = Self::for_nodes(root, &namespace, &faces, &seeds, limit)?;
        let total = selections
            .values()
            .filter(|selection| selection.rank == 0)
            .count();
        joined.notice = format!(
            "responsibility ownership: {examined} of {total} target files checked; {} declared owners; source containment is not resolved dispatch. Remaining target owners unexamined.\n",
            seeds.len()
        );
        Ok(Some(joined))
    }

    fn for_nodes(
        root: &Path,
        namespace: &str,
        faces: &[FaceView],
        seeds: &BTreeSet<NodeId>,
        limit: usize,
    ) -> Result<Self, String> {
        let packet = converge::packet(root, namespace, faces, Some(seeds), None, limit);
        let all = source_paths(root, faces)?;
        let paths = all
            .iter()
            .filter(|(id, _)| packet.selected.contains(id))
            .map(|(id, path)| (path.clone(), seeds.contains(id)))
            .collect();
        let known: BTreeMap<NodeId, &FaceView> = faces.iter().map(|face| (face.id, face)).collect();
        let faces_by_path = all
            .iter()
            .filter_map(|(id, path)| known.get(id).map(|face| (path.clone(), face.path.clone())))
            .collect();
        let focus = faces
            .iter()
            .filter(|face| seeds.contains(&face.id))
            .take(limit)
            .map(|face| face.path.clone())
            .collect::<Vec<_>>()
            .join(", ");
        Ok(Self {
            packet,
            paths,
            faces: faces_by_path,
            focus,
            notice: String::new(),
        })
    }

    /// The logical path of every registered face, by source path, for the repair projection.
    /// 每个注册面的逻辑路径（按源码路径索引），供修复投影使用。
    pub fn face_paths(&self) -> &BTreeMap<String, String> {
        &self.faces
    }

    pub fn scopes(&self) -> Vec<String> {
        self.paths.keys().cloned().collect()
    }

    pub fn is_target(&self, path: &str) -> bool {
        self.paths.get(path) == Some(&true)
    }

    /// Add responsibility neighbours before incidental lexical neighbours, preserving verbatim source.
    /// 责任邻居优先于偶然词法邻居，同时保留源码原文。
    pub fn add_sources(
        &mut self,
        root: &Path,
        sources: &mut Vec<SourceFile>,
        selected: &mut BTreeMap<usize, Selection>,
    ) {
        let mut ids: BTreeMap<_, _> = sources
            .iter()
            .enumerate()
            .map(|(id, file)| (file.relative.clone(), id))
            .collect();
        for (path, focus) in &self.paths {
            let id = if let Some(id) = ids.get(path) {
                *id
            } else {
                match load_one(root, path) {
                    Ok(file) => {
                        let id = sources.len();
                        sources.push(file);
                        ids.insert(path.clone(), id);
                        id
                    }
                    Err(error) => {
                        self.notice.push_str(&format!(
                            "responsibility source unavailable {path}: {error}\n"
                        ));
                        continue;
                    }
                }
            };
            let rank = if *focus { 0 } else { 1 };
            let selection = selected.entry(id).or_insert_with(|| Selection {
                rank,
                ..Selection::default()
            });
            selection.rank = selection.rank.min(rank);
            selection.notes.insert(
                if *focus {
                    "registered target; source ownership checked"
                } else {
                    "responsibility neighbour; verify shared contracts and consumers"
                }
                .into(),
            );
            let file = &mut sources[id];
            if file.functions.is_empty() {
                file.functions = crate::mcp::source_index::parse_functions(&file.source);
            }
            if selection.ranges.is_empty() || !focus {
                selection.ranges.push((1, file.source.lines().count(), 2));
                for function in &file.functions {
                    selection
                        .ranges
                        .push((function.line, function.end_line, rank));
                }
            }
        }
    }

    /// Budget independent sections so a long contract list cannot erase all runtime evidence.
    /// 独立分配段落预算，避免长合同清单挤掉全部运行证据。
    pub fn report(&self, budget: usize) -> String {
        let heading = format!("responsibility focus: {}\n{}", self.focus, self.notice);
        if budget < 1200 {
            let scope = if self.notice.starts_with("structural source scope") {
                "structural source scope; outside files not examined\n"
            } else {
                ""
            };
            return format!(
                "{scope}responsibility evidence omitted at this budget; raise max_bytes or converge {{node}}\n"
            );
        }
        let mut output = section(
            &(heading + &self.packet.overview),
            650.min(budget / 5),
            "responsibility scope",
        );
        let remaining = budget - output.len();
        output.push_str(&section(
            &self.packet.responsibilities,
            remaining * 2 / 3,
            "responsibility chain",
        ));
        output.push_str(&section(
            &self.packet.runtime,
            budget - output.len(),
            "runtime evidence",
        ));
        output
    }
}

fn source_paths(root: &Path, faces: &[FaceView]) -> Result<BTreeMap<NodeId, String>, String> {
    let layout = source_layout(root)?;
    faces
        .iter()
        .map(|face| {
            // The shared fact already spells the source package-relative; joining it to the package
            // root only produces the same display spelling and never consults the filesystem.
            // 共享事实已经把源码拼成包内相对；拼到包根上只得到同一个显示拼写，从不查询文件系统。
            let fact = face.source_ref(&layout);
            let path = layout.package_root.join(&fact.source);
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "face source escaped package root".to_owned())?;
            Ok((face.id, relative.to_string_lossy().replace('\\', "/")))
        })
        .collect()
}

pub(super) fn section(text: &str, budget: usize, label: &str) -> String {
    if text.len() <= budget {
        return text.to_owned();
    }
    let suffix = format!("\n{label} omitted/truncated; raise max_bytes or converge {{node}}\n");
    if suffix.len() > budget {
        return "…".chars().take(usize::from(budget >= 3)).collect();
    }
    let mut end = budget.saturating_sub(suffix.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{suffix}", &text[..end])
}
