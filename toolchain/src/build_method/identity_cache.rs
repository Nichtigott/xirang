//! Incremental identity cache helpers.
//! 增量身份缓存辅助函数。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::Node;
use super::registry_identity::NodeId;
use super::source_walk::collect_source_files;

/// Prime the process-wide identity cache from the per-unit cache files.
/// 从逐单元缓存文件预热进程级身份缓存。
///
/// Every entry is keyed by the namespace in force **while priming** — the
/// `check_for`/`run_for` package, because both run the pipeline inside
/// [`super::registry_identity::run_as_package`] — together with the relative
/// source path. The static itself stays first-write-wins: a second package in
/// the same process caches nothing here, so its lookups miss and
/// [`super::node_identity::node_id`] falls back to parsing the face and computing the
/// identity under its own namespace. That miss is the intended outcome — the
/// cache is an optimization, and a miss is right where a hit on another
/// package's entry was wrong.
/// 每条记录都以**预热当时生效的命名空间**（`check_for`/`run_for` 的包，因为两者都在
/// [`super::registry_identity::run_as_package`] 里跑管线）与相对源码路径共同为键。这个 static
/// 本身仍是先到先得：同进程的第二个包在这里什么都缓存不到，于是它的查询落空，
/// [`super::node_identity::node_id`] 退回解析注册面并在自己的命名空间下现算身份。落空正是预期结果——
/// 缓存只是优化，而"落空"在"命中别的包的条目"出错的地方是对的。
pub(crate) fn prime_node_id_cache(manifest: &Path, src: &Path, nodes: &[Node]) {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || manifest.join("target"),
            |path| {
                if path.is_absolute() {
                    path
                } else {
                    manifest.join(path)
                }
            },
        );
    let units = target.join("xirang/cache/units");
    let namespace = super::registry_identity::package_namespace();
    let mut values = super::node_identity::NodeIdCache::default();
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    for file in files {
        // The spelling is the discovery fact (see `source_spelling`), so a unit written for an
        // explicitly placed face is readable under the same key the plan asks for.
        // 拼写是发现事实（见 `source_spelling`），因此为显式落点的面写下的单元能在计划所用的同一个键下读到。
        let relative = super::discovery_node::source_spelling(src, &file);
        let fingerprint = source_unit_fingerprint(&file);
        let unit = units.join(format!("{fingerprint}.tsv"));
        let Ok(content) = fs::read_to_string(unit) else {
            continue;
        };
        if content
            .lines()
            .find_map(|line| line.strip_prefix("# schema\t"))
            != Some(super::CACHE_SCHEMA)
        {
            continue;
        }
        let path_matches = content
            .lines()
            .find_map(|line| line.strip_prefix("path\t"))
            .is_some_and(|path| path == relative);
        let id = content
            .lines()
            .find_map(|line| line.strip_prefix("node\t"))
            .and_then(|value| value.parse::<NodeId>().ok());
        if let (true, Some(id)) = (path_matches, id) {
            let kind = content
                .lines()
                .find_map(|line| line.strip_prefix("kind\t"))
                .unwrap_or_default()
                .to_owned();
            if !kind.is_empty() && id == super::registry_identity::package_node_id(&relative, &kind)
            {
                values.insert(&namespace, &relative, id, &kind);
            }
        }
    }
    let _ = super::CACHED_NODE_IDS.set(values);
}

pub(crate) fn valid_cached_unit(content: &str, path: &str, id: NodeId) -> bool {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# schema\t"))
        == Some(super::CACHE_SCHEMA)
        && content.lines().find_map(|line| line.strip_prefix("path\t")) == Some(path)
        && content
            .lines()
            .find_map(|line| line.strip_prefix("node\t"))
            .and_then(|value| value.parse::<NodeId>().ok())
            == Some(id)
}

pub(crate) fn source_unit_fingerprint(path: &Path) -> String {
    let source = fs::read(path).unwrap_or_default();
    let mut input = Vec::new();
    // W7/A31: this key **is** the evidence-identity shape W4 landed — it is derived from the inputs
    // the record was produced from (this file's path and its bytes) rather than from a timestamp or
    // a scan order. A29 put the producing toolchain into the *discovery* fingerprint; leaving it out
    // here would mean two keys, two input groups, and only one of them knowing which toolchain
    // produced it — so the same group is spelled the same way in both places.
    // W7/A31：这个键**就是** W4 落地的那条"证据身份"形状——它由产生该记录的那组输入（这个文件的路径与它的
    // 字节）导出，而不是由时间戳或扫描顺序导出。A29 把产生它的工具链放进**发现**指纹；这里若漏掉，就会出现
    // 两个键、两组输入，而只有一组知道自己是哪个工具链产生的——因此同一组输入在两个地方拼法一致。
    input.extend_from_slice(b"xirang-unit\0");
    input.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
    input.push(0);
    // A record spelling is `/`-separated while a discovered path carries the native separator. On
    // Windows the same file then spells two ways, the payload's freshness filter stops matching,
    // and every row is dropped. Hash `/` as the separator in both cases; on Unix this is the path
    // unchanged.
    // 记录拼写用 `/` 分隔，而发现路径带本机分隔符。在 Windows 上同一个文件会有两种拼法，载荷的新鲜性过滤
    // 就对不上、每一行都会被丢掉。两种情况下都按 `/` 分隔来哈希；在 Unix 上这就是原路径。
    input.extend_from_slice(path.to_string_lossy().replace('\\', "/").as_bytes());
    input.push(0);
    input.extend_from_slice(&source);
    NodeId::from_bytes(&input).to_string()
}

pub(crate) fn cache_directory(manifest: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || manifest.join("target/xirang/cache"),
            |path| {
                if path.is_absolute() {
                    path.join("xirang/cache")
                } else {
                    manifest.join(path).join("xirang/cache")
                }
            },
        )
}

#[cfg(test)]
mod tests {
    use super::{cache_directory, prime_node_id_cache, source_unit_fingerprint};
    use crate::build_method::discovery_node::{Node, relative_display};
    use crate::build_method::registry_identity::{NodeId, package_node_id, run_as_package};
    use std::path::{Path, PathBuf};

    /// A throwaway tree root, unique per call.
    /// 一次性目录树根，每次调用都不同。
    fn scratch(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "xirang-identity-cache-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("scratch root");
        root
    }

    const FACE: &str = "crate::root_object! { kind: Button, parent: crate::ROOT_NODE_ID, }\n";
    /// The relative path both fixture packages use, so a cache keyed by it alone
    /// cannot tell them apart.
    /// 两个夹具包共用的相对路径：只用它作键的缓存无法把两者区分开。
    const RELATIVE: &str = "control/object/button/button.rs";

    fn package(root: &Path, name: &str) -> (PathBuf, PathBuf, Node) {
        let manifest = root.join(name);
        let src = manifest.join("src");
        let file = src.join(RELATIVE);
        std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&file, FACE).expect("write face");
        let node = Node {
            name: "button".to_owned(),
            file: Some(file),
            children: Vec::new(),
        };
        (manifest, src, node)
    }

    /// Leave behind the unit file a build of `namespace` would have written.
    /// 留下一次以 `namespace` 身份构建会写出的那份 unit 文件。
    fn warm_unit(manifest: &Path, src: &Path, id: NodeId) {
        let units = cache_directory(manifest).join("units");
        std::fs::create_dir_all(&units).expect("units dir");
        let fingerprint = source_unit_fingerprint(&src.join(RELATIVE));
        std::fs::write(
            units.join(format!("{fingerprint}.tsv")),
            format!(
                "# schema\t{}\n# fingerprint\t{fingerprint}\npath\t{RELATIVE}\nnode\t{id}\nkind\tButton\n",
                crate::build_method::CACHE_SCHEMA
            ),
        )
        .expect("unit file");
    }

    /// Prime the process-wide cache as a build of `namespace` would, with or
    /// without the warm unit file on disk.
    /// 像一次以 `namespace` 身份的构建那样预热进程级缓存，磁盘上可有或没有那份已预热的 unit 文件。
    fn build_identity_cache(manifest: &Path, src: &Path, node: &Node, warm: bool) {
        if warm {
            warm_unit(manifest, src, package_node_id(RELATIVE, "Button"));
        }
        prime_node_id_cache(manifest, src, std::slice::from_ref(node));
    }

    /// The probe for LGC-LG-01: one process, two packages, the same relative
    /// source path — each package must read back its own identity.
    /// LGC-LG-01 的探针：一个进程、两个包、同一条相对源码路径——每个包都必须读回自己的身份。
    ///
    /// Run it alone, because it depends on this fixture being the first thing in
    /// the process to prime the cache:
    /// `cargo test -p xirang-toolchain --offline --lib -- --exact
    /// identity_cache::tests::a_second_package_does_not_read_the_first_packages_identity
    /// --test-threads=1`.
    /// In a full-suite run another test may prime first, which turns this probe
    /// into a (correct) cache miss; the two tests below cover the cache's own
    /// behaviour without depending on process-wide state.
    /// 它必须单独跑：前提是本夹具是本进程里第一个预热缓存的东西。全量跑时别的测试可能先预热，
    /// 探针就退化成一次（正确的）未命中；下面两条测试不依赖进程级状态，覆盖缓存自身的行为。
    #[test]
    fn a_second_package_does_not_read_the_first_packages_identity() {
        let root = scratch("two-packages");
        let (manifest_a, src_a, node_a) = package(&root, "pkg-a");
        let (manifest_b, src_b, node_b) = package(&root, "pkg-b");
        assert_eq!(
            relative_display(&src_a, node_a.file.as_ref().expect("a")),
            relative_display(&src_b, node_b.file.as_ref().expect("b")),
        );
        run_as_package("pkg-a", || {
            build_identity_cache(&manifest_a, &src_a, &node_a, true)
        });
        run_as_package("pkg-b", || {
            build_identity_cache(&manifest_b, &src_b, &node_b, true)
        });
        run_as_package("pkg-a", || {
            assert_eq!(
                crate::build_method::node_identity::node_id(&src_a, &node_a),
                Some(package_node_id(RELATIVE, "Button")),
                "the first package must read back its own identity",
            );
        });
        run_as_package("pkg-b", || {
            assert_eq!(
                crate::build_method::node_identity::node_id(&src_b, &node_b),
                Some(package_node_id(RELATIVE, "Button")),
                "the second package must read back its own identity, not the first package's",
            );
        });
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Cold cache (nothing on disk to prime from), warm cache, and both package
    /// orders: the answer is the package's own identity in every combination.
    /// 冷缓存（磁盘上无可预热之物）、热缓存与两种包顺序：每种组合下的答案都是该包自己的身份。
    #[test]
    fn every_package_reads_its_own_identity_in_both_orders_and_temperatures() {
        for warm in [false, true] {
            for first_b in [false, true] {
                let root = scratch(&format!("matrix-{warm}-{first_b}"));
                let a = package(&root, &format!("cold-a-{warm}{first_b}"));
                let b = package(&root, &format!("cold-b-{warm}{first_b}"));
                let names = if first_b { ["b", "a"] } else { ["a", "b"] };
                let (manifest_a, src_a, node_a) = &a;
                let (manifest_b, src_b, node_b) = &b;
                for name in names {
                    let (manifest, src, node) = match name {
                        "a" => (manifest_a, src_a, node_a),
                        _ => (manifest_b, src_b, node_b),
                    };
                    run_as_package(name, || build_identity_cache(manifest, src, node, warm));
                }
                let packages: [(&PathBuf, &PathBuf, &Node, &str); 2] = [
                    (manifest_a, src_a, node_a, "a"),
                    (manifest_b, src_b, node_b, "b"),
                ];
                for (_, src, node, name) in packages {
                    run_as_package(name, || {
                        assert_eq!(
                            crate::build_method::node_identity::node_id(src, node),
                            Some(package_node_id(RELATIVE, "Button")),
                            "warm={warm} first_b={first_b} package={name}",
                        );
                    });
                }
                let _ = std::fs::remove_dir_all(&root);
            }
        }
    }

    /// The cache's own namespace axis: an entry primed for one package is not
    /// readable for another, even for the identical relative path.
    /// 缓存自身的命名空间轴：为某个包预热的条目对别的包不可读，即便相对路径完全相同。
    #[test]
    fn an_entry_is_readable_only_in_the_namespace_it_was_primed_for() {
        let mut cache = crate::build_method::node_identity::NodeIdCache::default();
        let id_a = NodeId::from_namespaced_path("pkg-a", RELATIVE, "Button");
        let id_b = NodeId::from_namespaced_path("pkg-b", RELATIVE, "Button");
        assert_ne!(id_a, id_b);
        cache.insert("pkg-a", RELATIVE, id_a, "Button");
        let in_b = run_as_package("pkg-b", || cache.get(RELATIVE).map(|(id, _)| *id));
        assert_eq!(in_b, None, "another package's entry is not reachable");
        let in_a = run_as_package("pkg-a", || cache.get(RELATIVE).map(|(id, _)| *id));
        assert_eq!(in_a, Some(id_a), "the owning package still reads it");
    }

    /// The read side rechecks: an entry stored under the namespace in force but
    /// carrying an identity that disagrees with its stored kind is refused.
    /// 读取侧复核：在当前命名空间下存着、但身份与其 kind 不符的条目会被拒绝。
    #[test]
    fn an_entry_that_does_not_match_its_own_identity_is_refused() {
        let mut cache = crate::build_method::node_identity::NodeIdCache::default();
        let namespace = crate::build_method::registry_identity::package_namespace();
        let foreign = NodeId::from_namespaced_path("some-other-package", RELATIVE, "Button");
        cache.insert(&namespace, RELATIVE, foreign, "Button");
        assert_eq!(
            cache.get(RELATIVE),
            None,
            "an identity that disagrees with its kind is not returned",
        );
        let own = package_node_id(RELATIVE, "Button");
        cache.insert(&namespace, RELATIVE, own, "Button");
        assert_eq!(
            cache.get(RELATIVE).map(|(id, _)| *id),
            Some(own),
            "a matching entry is returned",
        );
    }
}

#[cfg(test)]
mod unit_key_tests {
    use super::source_unit_fingerprint;

    /// W7/A31: the unit key is the **evidence-identity shape** — derived from the inputs the record
    /// was produced from — and it now spells the same input group as the discovery fingerprint does
    /// (path, bytes, and the producing toolchain). What the pin can show is the first two: the
    /// toolchain comes from `env!` at compile time, so a test cannot vary it here; that half is
    /// covered by `source_walk::toolchain_version_tests` on the discovery side.
    /// W7/A31：单元键就是**证据身份**那条形状——由产生记录的那组输入导出——并且现在与发现指纹**拼同一组
    /// 输入**（路径、字节、以及产生它的工具链）。钉子能展示的是前两项：工具链来自编译期 `env!`，测试在这里
    /// 换不了它；那一半由发现侧的 `source_walk::toolchain_version_tests` 覆盖。
    #[test]
    fn the_unit_key_follows_the_path_and_the_bytes() {
        let root = std::env::temp_dir().join(format!("xirang-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let one = root.join("one.rs");
        let two = root.join("two.rs");
        std::fs::write(&one, "pub fn a() {}\n").unwrap();
        std::fs::write(&two, "pub fn a() {}\n").unwrap();

        // Same bytes, different path ⇒ different key: the path is part of the input group.
        // 字节相同、路径不同 ⇒ 键不同：路径是那组输入的一部分。
        assert_ne!(source_unit_fingerprint(&one), source_unit_fingerprint(&two));
        // Same path and bytes ⇒ the same key, twice running: nothing time- or order-dependent.
        // 路径与字节都相同 ⇒ 两次得到同一个键：没有任何依赖时间或顺序的东西。
        assert_eq!(source_unit_fingerprint(&one), source_unit_fingerprint(&one));
        // An edit moves it.
        // 编辑一次就动。
        let before = source_unit_fingerprint(&one);
        std::fs::write(&one, "pub fn b() {}\n").unwrap();
        assert_ne!(before, source_unit_fingerprint(&one), "content is an input");
        let _ = std::fs::remove_dir_all(&root);
    }
}
