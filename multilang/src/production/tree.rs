//! The build-time registry tree a project's declaration documents produce. Every
//! `declarations/*.json` document is read with the same parser `xirang declaration check` uses, each
//! face gets the identity the build would give it (the namespace is the root package's name and the
//! relative path is the document's own path, so a checkout that moves keeps the same ids), and the
//! tree is published as `registry.json` beside the plan's generated files. `xirang declaration tree`
//! prints the same tree without building anything.
//! 项目的声明文档产出的构建期注册树。`declarations/*.json` 用与 `xirang declaration check` 相同的
//! 解析器读取，每个面得到构建会给它的身份（命名空间取根包名、相对路径取文档自己的路径，因此换了
//! 检出也不变），树随计划产物发布成 `registry.json`；`xirang declaration tree` 不构建、只打印同一棵树。
//!
//! What this tree is not: it carries the **identity subset** the interchange format defines. The parts
//! of a `Registration` that only Rust can express today — `registry_rule`, `admission`, `handle_*`,
//! `part_*`, `runtime_checks` — are absent here by construction, not by omission.
//! 这棵树**不是**什么：它只承载交换格式定义的**身份子集**。`Registration` 里目前只有 Rust 能表达的
//! 部分——`registry_rule`、`admission`、`handle_*`、`part_*`、`runtime_checks`——在构造上就不在这里，
//! 不是遗漏。

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value, json};
use xirang_kernel::{NodeId, root_node_id};

use super::declaration;
use super::project::Project;

/// The directory under the project root that holds declaration documents.
/// 项目根下存放声明文档的目录。
const DECLARATION_DIRECTORY: &str = "declarations";

/// One face a declaration document puts into the registry tree.
/// 一份声明文档放进注册树的一个面。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredFace {
    /// The node identity, hex-encoded: `NodeId::from_namespaced_path(namespace, document, name)`.
    /// 节点身份的十六进制形式：`NodeId::from_namespaced_path(namespace, document, name)`。
    pub(crate) id: String,
    /// The declared name; unique under the root, so two documents cannot both claim it.
    /// 声明的名字；在根之下唯一，因此两份文档不能都声明它。
    pub(crate) name: String,
    /// The kind the face is an instance of.
    /// 该面所属的种类。
    pub(crate) kind: String,
    /// The contracts the face fills, sorted by the parser.
    /// 该面填写的合同，由解析器排序。
    pub(crate) contracts: Vec<String>,
    /// The document the face came from, relative to the project root.
    /// 该面来自的文档，相对项目根。
    pub(crate) document: String,
    pub(crate) preset: Option<String>,
    pub(crate) parts: Vec<String>,
    pub(crate) requires: BTreeMap<String, String>,
    pub(crate) provides: Vec<String>,
    pub(crate) exports: Vec<String>,
    pub(crate) graft: Option<String>,
    pub(crate) flow: Option<String>,
    pub(crate) stable_name: Option<String>,
}

/// The tree a project's declaration documents describe.
/// 一个项目的声明文档所描述的树。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredTree {
    /// The package namespace the ids are hashed with.
    /// 计算身份所用的包命名空间。
    pub(crate) namespace: String,
    /// The root node every declared face hangs under, hex-encoded.
    /// 每个声明面所挂载的根节点，十六进制形式。
    pub(crate) root: String,
    /// The faces, in name order.
    /// 各面，按名字排序。
    pub(crate) faces: Vec<DeclaredFace>,
}

impl DeclaredTree {
    /// The tree as the `registry.json` artifact: sorted keys, faces in name order.
    /// 树的 `registry.json` 产物：键有序、面按名字排序。
    pub(crate) fn json(&self) -> String {
        let faces: Vec<Value> = self.faces.iter().map(face_json).collect();
        let document = json!({
            "protocol": 1,
            "namespace": self.namespace,
            "root": self.root,
            "faces": faces,
        });
        serde_json::to_string(&document).unwrap_or_default()
    }

    /// The tree as `xirang declaration tree` prints it: a header line, then one row per face.
    /// `xirang declaration tree` 打印的形状：一行表头，随后每个面一行。
    pub(crate) fn text(&self) -> String {
        let mut text = format!(
            "namespace {}\nroot {}\nid\tname\tkind\tcontracts\tdocument\n",
            self.namespace, self.root
        );
        for face in &self.faces {
            text.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                face.id,
                face.name,
                face.kind,
                face.contracts.join(","),
                face.document
            ));
        }
        text
    }

    /// A digest over [`DeclaredTree::json`]: the value a plan identity carries, so editing a
    /// declaration is a different plan.
    /// [`DeclaredTree::json`] 的摘要：计划身份携带的值，因此改动一份声明就是另一份计划。
    pub(crate) fn digest(&self) -> String {
        xirang_kernel::sha256_hex(self.json().as_bytes())
    }

    /// Whether any document declared a face.
    /// 是否有任何文档声明了面。
    pub(crate) fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

/// One face as the artifact spells it; absent optional fields are absent from the object, so
/// "not declared" and "declared empty" stay distinguishable.
/// 产物里一个面的拼法；没声明的可选字段就不出现，于是"没声明"与"声明为空"仍可区分。
fn face_json(face: &DeclaredFace) -> Value {
    let mut object = Map::new();
    object.insert("id".to_owned(), json!(face.id));
    object.insert("name".to_owned(), json!(face.name));
    object.insert("kind".to_owned(), json!(face.kind));
    object.insert("contracts".to_owned(), json!(face.contracts));
    object.insert("document".to_owned(), json!(face.document));
    if let Some(preset) = &face.preset {
        object.insert("preset".to_owned(), json!(preset));
    }
    if !face.parts.is_empty() {
        object.insert("parts".to_owned(), json!(face.parts));
    }
    if !face.requires.is_empty() {
        object.insert("requires".to_owned(), json!(face.requires));
    }
    if !face.provides.is_empty() {
        object.insert("provides".to_owned(), json!(face.provides));
    }
    if !face.exports.is_empty() {
        object.insert("exports".to_owned(), json!(face.exports));
    }
    if let Some(graft) = &face.graft {
        object.insert("graft".to_owned(), json!(graft));
    }
    if let Some(flow) = &face.flow {
        object.insert("flow".to_owned(), json!(flow));
    }
    if let Some(stable) = &face.stable_name {
        object.insert("stable_name".to_owned(), json!(stable));
    }
    Value::Object(object)
}

/// Read every `declarations/*.json` document under `directory` and build the tree.
/// 读取 `directory` 下每一份 `declarations/*.json` 并建树。
///
/// A missing `declarations/` directory is an empty tree, not an error: a project that declares no
/// non-Rust face is the normal case. Everything else is refused by name — a document that is not a
/// `.json` file, a directory, unreadable bytes, a parse error, or two documents claiming one name —
/// because the tree is what the build publishes and a half-read tree would be a lie.
/// 缺 `declarations/` 目录是空树而不是错误：不声明非 Rust 面是常态。其余一律按名拒绝——不是 `.json`
/// 的文件、目录、读不出的字节、解析错误、两份文档抢同一个名字——因为这棵树是构建要发布的东西，
/// 半读出来的树是假的。
pub(super) fn load(directory: &Path, namespace: &str) -> Result<DeclaredTree, String> {
    let declarations = directory.join(DECLARATION_DIRECTORY);
    let root = root_node_id(namespace).to_string();
    if !declarations.exists() {
        return Ok(DeclaredTree {
            namespace: namespace.to_owned(),
            root,
            faces: Vec::new(),
        });
    }
    if !declarations.is_dir() {
        return Err(format!(
            "`{}` is not a directory; declaration documents live in `{DECLARATION_DIRECTORY}/`",
            declarations.display()
        ));
    }
    let mut documents = Vec::new();
    let entries = std::fs::read_dir(&declarations)
        .map_err(|error| format!("cannot read {}: {error}", declarations.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", declarations.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            return Err(format!(
                "`{}` is a directory; a declaration document is a `.json` file directly under \
                 `{DECLARATION_DIRECTORY}/`",
                path.display()
            ));
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            return Err(format!(
                "`{}` is not a `.json` document; `{DECLARATION_DIRECTORY}/` holds declaration \
                 documents and nothing else",
                path.display()
            ));
        }
        documents.push(path);
    }
    documents.sort();
    let mut faces: Vec<DeclaredFace> = Vec::new();
    let mut claimed: Vec<(String, String)> = Vec::new();
    for path in documents {
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let document = path
            .strip_prefix(directory)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        let parsed =
            declaration::parse(&text).map_err(|error| format!("`{}`: {error}", path.display()))?;
        for face in parsed {
            if let Some((name, other)) = claimed.iter().find(|(name, _)| name == &face.name) {
                return Err(format!(
                    "`{other}` and `{document}` both declare a face named `{name}`; two faces \
                     under one parent cannot share a name"
                ));
            }
            claimed.push((face.name.clone(), document.clone()));
            faces.push(DeclaredFace {
                id: NodeId::from_namespaced_path(namespace, &document, &face.name).to_string(),
                name: face.name,
                kind: face.kind,
                contracts: face.contracts,
                document: document.clone(),
                preset: face.preset,
                parts: face.parts,
                requires: face.requires,
                provides: face.provides,
                exports: face.exports,
                graft: face.graft,
                flow: face.flow,
                stable_name: face.stable_name,
            });
        }
    }
    faces.sort_by(|left, right| {
        (left.name.as_str(), left.document.as_str())
            .cmp(&(right.name.as_str(), right.document.as_str()))
    });
    Ok(DeclaredTree {
        namespace: namespace.to_owned(),
        root,
        faces,
    })
}

impl Project {
    /// The registry tree this project's declaration documents describe.
    /// 本项目的声明文档所描述的注册树。
    ///
    /// The namespace is the root package's name and the ids are relative to the project root, so two
    /// checkouts of one project agree and a dependency's own documents are not merged in: they
    /// describe that dependency, not this project.
    /// 命名空间取根包名、身份相对项目根计算，因此同一项目的两个检出结果一致；依赖自己的文档不并入——
    /// 它们描述的是那个依赖，不是本项目。
    pub(crate) fn declared_tree(&self) -> Result<DeclaredTree, String> {
        let namespace = self.root_unit().manifest.package.name.clone();
        load(&self.root, &namespace)
    }
}
