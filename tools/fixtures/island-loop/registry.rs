//! The built host's **actual** registration tree: identities, parents and the object set.
//! 构建出来宿主的**实际**注册树：身份、父级与对象集合。
//!
//! The expectations live in `expect.rs`, which the gate writes from the requests it sent — never
//! from the product's reply. Everything here is recomputed from the namespace and the spelling the
//! record carries, so a face whose identity input drifted fails instead of merely being spelled
//! differently.
//! 期望住在 `expect.rs`，由门禁按它发出的请求写下——绝不按产品的回复填写。这里的一切都由命名空间与记录
//! 携带的拼写现算，因此身份输入漂移的面会失败，而不是只被拼成另一个样子。

#[path = "support/expect.rs"]
mod expect;

use e2e::{base_registry, builtin_static_plan, registrations};
use expect::{FACES, OBJECTS};
use xirang_toolchain::run_method::registry_core::NodeId;
use xirang_toolchain::run_method::registry_core::identity::StableFaceId;

/// The identity the declaration macro and the build must agree on.
/// 声明宏与构建必须一致的那个身份。
fn derived(source: &str, kind: &str) -> NodeId {
    NodeId::from_namespaced_path(env!("CARGO_PKG_NAME"), source, kind)
}

/// The root every parent chain starts at.
/// 每条父级链的起点。
fn root() -> NodeId {
    e2e::root_node_id(env!("CARGO_PKG_NAME"))
}

#[test]
fn the_built_tree_is_exactly_what_the_requests_named() {
    let registry = base_registry();
    let tree = registry.depth_first();
    let mut actual = tree
        .iter()
        .map(|info| {
            format!(
                "{}|{}|{}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.portable_file()
            )
        })
        .collect::<Vec<_>>();
    actual.sort();
    let mut expected = FACES
        .iter()
        .map(|face| format!("{}|{}|{}", face.path, face.kind, face.source))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(
        actual, expected,
        "the actual tree holds exactly the product-generated faces, under the spellings the \
         requests named — self-consistency is not enough"
    );

    for face in FACES {
        let id = derived(face.source, face.kind);
        let info = tree
            .iter()
            .find(|info| info.id == id)
            .unwrap_or_else(|| panic!("no face with the derived identity of `{}`", face.path));
        let parent = if face.parent.is_empty() {
            root()
        } else {
            let parent_spec = FACES
                .iter()
                .find(|candidate| candidate.path == face.parent)
                .unwrap_or_else(|| panic!("the expectation names an unknown parent `{}`", face.parent));
            derived(parent_spec.source, parent_spec.kind)
        };
        assert_eq!(
            info.parent, parent,
            "`{}` hangs under `{}`",
            face.path, face.parent
        );
        assert_eq!(
            registry.path_for(info.id).as_deref(),
            Some(face.path),
            "the logical path is the one the request named"
        );
    }

    let plan = builtin_static_plan();
    assert_eq!(
        plan.len(),
        FACES.len(),
        "the generated plan carries every face and no other"
    );
    for face in FACES {
        let id = derived(face.source, face.kind);
        assert!(
            plan.find_face(id).is_some(),
            "the plan holds `{}` under its derived identity",
            face.path
        );
    }
    let mut registered = registrations()
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    registered.sort();
    let mut expected_ids = FACES
        .iter()
        .map(|face| derived(face.source, face.kind))
        .collect::<Vec<_>>();
    expected_ids.sort();
    assert_eq!(
        registered, expected_ids,
        "the compiled registration table carries exactly these identities"
    );
}

#[test]
fn the_ledger_keeps_one_object_per_face_across_every_move() {
    let ledger = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/xirang.objects.tsv"
    ))
    .expect("the project ledger");
    for object in OBJECTS {
        let row = ledger
            .lines()
            .find(|line| line.split('\t').nth(1) == Some(object.path))
            .unwrap_or_else(|| panic!("no ledger row for `{}`:\n{ledger}", object.path));
        let columns = row.split('\t').collect::<Vec<_>>();
        assert_eq!(
            columns[0],
            StableFaceId::from_namespaced_name(env!("CARGO_PKG_NAME"), object.created_at)
                .to_string(),
            "`{}` is still the object it was created as; a move is a physical change, not a new \
             object",
            object.path
        );
        assert_eq!(columns[2], object.source, "the ledger names the file that exists");
        assert_eq!(
            columns[3], object.directory,
            "the explicit mapping follows the file"
        );
        let mut expected = object
            .instances
            .iter()
            .map(|(package, record)| {
                format!("{}@{}", derived(record, object.kind), package)
            })
            .collect::<Vec<_>>();
        // A dash is the ledger's spelling of "no instance yet": a face being created knows its
        // object before it knows the identity the file will be stamped with.
        // 破折号是台账对"还没有实例"的拼写：正在创建的面在知道文件将被盖上的身份之前，先知道它的对象。
        let mut actual = if columns[4] == "-" {
            Vec::new()
        } else {
            columns[4].split(',').map(str::to_owned).collect::<Vec<_>>()
        };
        expected.sort();
        actual.sort();
        assert_eq!(
            actual, expected,
            "every source instance the object lived through stays resolvable on one object"
        );
    }
}
