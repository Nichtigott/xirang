//! Test half of the discovery cache: the generation writer's four judgements and the A33 scale
//! measurements (cold / warm / restart / after-edit), which used to live inside `discovery_cache.rs`
//! until the file crossed the source-line budget.
//! 发现缓存的测试半边：代次写入器的四条判定与 A33 的规模测量（冷/热/重启/编辑后）。在 `discovery_cache.rs`
//! 越过源码行预算之前，它们住在那份文件里。

mod write_generation_tests {
    use crate::build_method::discovery_cache::write_if_changed_at;

    /// W7/A30, the half that **is** there: one write is idempotent and atomic. Read to the `return`
    /// (`discovery_cache.rs:47-60`) the pair is: compare the current bytes first, and publish through
    /// a unique temporary sibling plus `rename`, so a reader never sees a half-written file.
    /// W7/A30 中**已经存在**的那一半：单次写入既幂等又原子。读到 `return`（`discovery_cache.rs:47-60`）
    /// 可以看到这一对：先比当前字节，再经唯一的同级临时文件 + `rename` 发布，因此读者绝不会看到半成品。
    #[test]
    fn one_write_is_idempotent_and_leaves_no_temporary() {
        let root = std::env::temp_dir().join(format!("xirang-wic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("plan.tsv");

        write_if_changed_at(&target, "first\n", 1).expect("the first write lands");
        // The generation is **inside** the artifact now (the maintainer's ruling, 2026-10-18, option A1), so this
        // is the whole published file: one `rename` carries both the bytes and the number that names them.
        // 代次现在**在产物里**（维护者 2026-10-18 裁定，选项 A1），因此这就是被发布的整个文件：一次 `rename`
        // 同时带着字节与命名它们的那个数字。
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 1\nfirst\n"
        );
        let written = std::fs::metadata(&target).unwrap().modified().unwrap();

        // Same content ⇒ nothing is published: the file keeps its identity.
        // 内容相同 ⇒ 什么都不发布：文件保持原样。
        write_if_changed_at(&target, "first\n", 1).expect("the second write is a no-op");
        assert_eq!(
            std::fs::metadata(&target).unwrap().modified().unwrap(),
            written,
            "an unchanged write does not touch the file"
        );

        // Different content ⇒ published, and no temporary survives the call.
        // 内容不同 ⇒ 发布，且调用之后不留临时文件。
        write_if_changed_at(&target, "second\n", 1).expect("a changed write lands");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 1\nsecond\n"
        );
        // A **temporary** might survive a call; the generation line is not one — it is part of the artifact.
        // 可能残留的是**临时文件**；代次行不是残留，它是产物的一部分。
        let names: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !names.iter().any(|name| name.contains(".tmp-")),
            "the temporary is a sibling this call cleans up: {names:?}"
        );
        assert!(
            !names.iter().any(|name| name == "plan.tsv.generation"),
            "and no generation sidecar is published beside it any more: {names:?}"
        );
        assert!(
            std::fs::read_to_string(&target)
                .unwrap()
                .starts_with("# xirang-generation: 1\n"),
            "because the artifact carries the number itself"
        );

        // **Within one generation** the last writer still wins, and that is deliberate rather than
        // left over: the rule orders *generations*, not writers. Two writers of the same generation
        // disagreeing about the bytes is a bug in one of them, and the generation cannot arbitrate it
        // (W7/A30, report §7e).
        // **同一代次之内**仍是最后写的人赢，而这是有意的、不是残留：这条规则排序的是**代次**而不是写入者。同一
        // 代次的两个写入者对字节有分歧，那是其中一个的缺陷，而代次无法替它们裁决（W7/A30，报告 §7e）。
        write_if_changed_at(&target, "same-generation\n", 1).expect("a peer writer lands");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 1\nsame-generation\n",
            "generations order writers; they do not arbitrate within one"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// W7/A30's four judgements, written before the code (report §7e) and pinned here in that order:
    /// a **newer** published generation is refused with both numbers named; an **older** one is
    /// published; identical bytes **at the same generation** publish nothing; and a repeat at the same
    /// generation is a no-op because the bytes already match.
    /// W7/A30 的四条判定，先于代码写成（报告 §7e），这里按那个顺序钉住：目标代次**更新** ⇒ 拒绝并报出两个
    /// 编号；目标代次**更旧** ⇒ 发布；**同一代次**下内容相同 ⇒ 什么都不发布；同代次重复 ⇒ 因字节已相同而空操作。
    ///
    /// One sentence of this changed on 2026-10-18, and the change is the ruling rather than a drift: the generation
    /// used to live in a sidecar, so "identical bytes publish nothing" could hold *across* generations. It lives in
    /// the artifact now (option A1), which makes the number part of the artifact's identity — a newer generation is a
    /// different file even when its payload is the same, and ③ says so instead of pretending otherwise.
    /// 这条里有一句在 2026-10-18 变了，而那是**裁定**而不是漂移：代次过去住在旁注里，所以"内容相同就什么都不发布"
    /// 能**跨代次**成立。现在它住在产物里（选项 A1），于是那个数字成了产物身份的一部分——更新的代次即使负载相同也是
    /// 另一个文件，③ 现在就这么说，而不是假装相反。
    #[test]
    fn a_stale_generation_is_refused_and_a_fresher_one_is_published() {
        let root = std::env::temp_dir().join(format!("xirang-gen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("plan.tsv");

        // ⓪ A tree with nothing published yet is generation 0: the first write always lands.
        // ⓪ 什么都还没发布的树是第 0 代：第一次写入总是落地。
        write_if_changed_at(&target, "gen-3\n", 3).expect("the first write lands");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 3\ngen-3\n",
            "the artifact records the generation that wrote it, in its first line"
        );

        // ① A writer from an older generation is refused, and the refusal names both numbers.
        // ① 来自更旧代次的写入者被拒绝，且拒绝里报出两个编号。
        let refusal = write_if_changed_at(&target, "gen-1\n", 1)
            .expect_err("a stale plan must not overwrite a fresher artifact");
        assert!(
            refusal.contains("generation 3") && refusal.contains("generation 1"),
            "the refusal names what is on disk and what was offered: {refusal}"
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 3\ngen-3\n",
            "and the fresher body is untouched"
        );

        // ② A newer generation publishes, and the number inside the artifact moves with it.
        // ② 更新的代次发布，产物里的那个数字跟着走。
        write_if_changed_at(&target, "gen-5\n", 5).expect("a fresher write lands");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 5\ngen-5\n"
        );

        // ③ The same payload at a **newer generation** publishes the generation line: the line is part of the
        // artifact's identity now, so leaving it behind would make the artifact claim a writer that did not write it.
        // ③ **更新的代次**下相同的负载会发布代次行：这一行现在属于产物身份，把它留在旧值上会让产物声称一个并没有
        // 写它的写入者。
        let written = std::fs::metadata(&target).unwrap().modified().unwrap();
        write_if_changed_at(&target, "gen-5\n", 9).expect("a newer generation publishes");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 9\ngen-5\n",
            "the payload is unchanged and the number is not"
        );
        assert!(
            std::fs::metadata(&target).unwrap().modified().unwrap() >= written,
            "the file was republished"
        );

        // ④ The same generation twice is a no-op for the same reason — the bytes already match. The generation here
        // is the one ③ published (9), because a repeat at the *older* number is ① again: refused, not a no-op.
        // ④ 同一代次写两次同样是空操作，理由相同：字节已经一致。这里的代次是 ③ 发布的那个（9），因为在**更旧**的
        // 编号上重复就是 ① 那种情形：被拒绝，而不是空操作。
        let at_nine = std::fs::metadata(&target).unwrap().modified().unwrap();
        write_if_changed_at(&target, "gen-5\n", 9).expect("a repeat is a no-op");
        assert_eq!(
            std::fs::metadata(&target).unwrap().modified().unwrap(),
            at_nine
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

mod inventory_scale_tests {
    use crate::build_method::discovery_cache::update_discovery_cache;
    use crate::build_method::discovery_node::Node;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::Instant;

    /// What the acceptance calls a tree's two densities: the same declarations, with or without a
    /// body of ordinary Rust around them, so the parse cost can be attributed to one or the other.
    /// 验收所说的两种树密度：同一批声明，外加或不外加一大段普通 Rust——解析开销因此能归到其中一边。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Density {
        /// Declaration only: the parse cost *is* the declaration.
        /// 只有声明：解析开销**就是**那条声明。
        Declarations,
        /// Declaration buried in ordinary items: the parse cost is the file.
        /// 声明埋在一堆普通条目里：解析开销是整份文件。
        Ordinary,
    }

    /// Ordinary items per face in the ordinary-dense tree. A knob, not a promise.
    /// 普通源码稠密的树里每个面携带的普通条目数。是旋钮，不是承诺。
    const ORDINARY_ITEMS: usize = 80;

    /// Faces per parent: one group face plus its leaves. A real tree distributes its faces across
    /// many parents, so this is the shipped shape taken seriously rather than one parent with fifty
    /// thousand children. It does **not** make the parent lookup cheaper, and the legs were written
    /// expecting it to: the cold path resolves a declared parent **per new face**, so the number of
    /// distinct parents does not enter the cost at all — the grouped 10k leg costs what the
    /// single-parent one did (21 522 ms vs 20 227 ms), and the isolation leg below is what actually
    /// prices that lookup: 15 239 ms for the same tree with parentless leaves, i.e. the lookup is
    /// 6 283 ms of a 21 522 ms cold pass (29%), or 0.63 ms per face (delivery report §7g).
    /// 每组多少个面：一个组面加它的叶子。真实树把面分布在很多父级下，因此这是把出厂形状当真，而不是一个
    /// 父级挂五万个孩子。它**不会**让父级查找变便宜，而下面这些腿本来就是这么以为的：冷路径**每个新面**都要
    /// 解析一次被点名的父级，因此"有多少个不同的父级"根本不进代价——分组后的 10k 腿与先前单父级那次同价
    /// （21 522 ms 对 20 227 ms），真正给这次查找定价的是下面那条隔离腿：同一棵树、叶子不点名父级时 15 239 ms，
    /// 即这次查找占 21 522 ms 冷路径里的 6 283 ms（29%）、每面 0.63 ms（交付报告 §7g）。
    const GROUP_FACES: usize = 32;

    /// Whether a leaf names its parent **by path** or points at the package root. The difference is
    /// one extra file read plus one parse **per new face** on the cold path: resolving a declared
    /// parent is done per unit, not once per parent, and the summary's `misses=` does not count it.
    /// That is why this axis exists — it is the one parse source the numbers would otherwise hide.
    /// 叶子是**按路径**点名父级，还是指向包根。差别是冷路径上**每个新面**多一次文件读取加一次解析：
    /// 解析一个被点名的父级是按单元做的，不是按父级做一次，而摘要里的 `misses=` 不数它。这条轴就是
    /// 为此存在——它是那些数字本来会藏起来的唯一一处解析来源。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Parents {
        /// `.under(crate::control::NODE_ID)` — the shipped spelling.
        /// `.under(crate::control::NODE_ID)`——出厂拼法。
        Declared,
        /// `.under(crate::root_node_id(crate::XIRANG_NAMESPACE))` — nobody to look up.
        /// `.under(crate::root_node_id(crate::XIRANG_NAMESPACE))`——没有父级要查。
        Root,
    }

    /// What a fabricated tree is made of: declared faces, how dense their bodies are, whether they
    /// name a parent by path, and how many ordinary neighbours sit beside them. The neighbours are
    /// the part a real crate has and an all-faces tree cannot show: a file with no declaration gets
    /// **no row**, so it has nothing to reuse.
    /// 合成的树由什么构成：声明了多少个面、面体多稠密、是否按路径点名父级、旁边有多少普通邻居。
    /// 邻居是真实 crate 有、而"全是面"的树给不出的那部分：没有声明的文件**没有行**，因此没有东西可复用。
    #[derive(Clone, Copy)]
    struct Shape {
        faces: usize,
        density: Density,
        parents: Parents,
        plain: usize,
    }

    impl Shape {
        /// `faces` declared faces, declaration-dense, each naming its parent by path, no neighbours.
        /// `faces` 个声明面、声明稠密、各自按路径点名父级、没有邻居。
        fn faces(faces: usize) -> Shape {
            Shape {
                faces,
                density: Density::Declarations,
                parents: Parents::Declared,
                plain: 0,
            }
        }

        /// The same tree with each face buried in ordinary items.
        /// 同一棵树，但每个面埋在一段普通条目里。
        fn ordinary(faces: usize) -> Shape {
            Shape {
                density: Density::Ordinary,
                ..Shape::faces(faces)
            }
        }

        /// The same tree with leaves that name no parent by path.
        /// 同一棵树，但叶子不按路径点名父级。
        fn parentless(faces: usize) -> Shape {
            Shape {
                parents: Parents::Root,
                ..Shape::faces(faces)
            }
        }

        /// The same tree plus `plain` ordinary neighbour files, one per directory, none of which
        /// declares anything.
        /// 同一棵树外加 `plain` 个普通邻居文件（每个一个目录），没有一个声明任何东西。
        fn mixed(faces: usize, plain: usize) -> Shape {
            Shape {
                plain,
                ..Shape::faces(faces)
            }
        }

        fn label(&self) -> String {
            let mut label = format!("{} ({:?}/{:?})", self.faces, self.density, self.parents);
            if self.plain > 0 {
                label.push_str(&format!(" +{} plain", self.plain));
            }
            label
        }
    }

    /// The child-process protocol of the restart phase.
    /// 重启阶段所用的子进程协议。
    const CHILD_ROOT: &str = "XIRANG_SCALE_CHILD_ROOT";
    /// `0` means "this child rebuilds its nodes by walking a real tree" (the checkout leg);
    /// any other value is the fabricated face count.
    /// `0` 表示"这个子进程靠遍历一棵真树重建节点"（本检出那条腿）；其余值是合成面的数量。
    const CHILD_FACES: &str = "XIRANG_SCALE_CHILD_FACES";
    /// How many ordinary neighbours the child must rebuild alongside the faces.
    /// 子进程要跟着面一起重建多少个普通邻居。
    const CHILD_PLAIN: &str = "XIRANG_SCALE_CHILD_PLAIN";
    const CHILD_MARKER: &str = "restart: ";

    /// One tree to measure: where it lives and what the walk found.
    /// 待测的一棵树：它在哪里、遍历发现了什么。
    struct Fixture {
        /// The package root — the discovery cache lands under `<root>/target/xirang/cache`.
        /// 包根——发现缓存落在 `<root>/target/xirang/cache` 下。
        root: PathBuf,
        src: PathBuf,
        nodes: Vec<Node>,
    }

    /// The parent face, in the shape the shipped example uses, so it really parses.
    /// 父面，用出厂示例的形状写，因此它**真的**能被解析。
    fn parent_source(group: usize) -> String {
        format!(
            "//! Scale fixture group face {group}.\n\
             //! 规模夹具的组面 {group}。\n\n\
             #[xirang_toolchain::soil]\n\
             pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Group{group}>()\n\
             \x20\x20\x20\x20.under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\n\
             pub struct Group{group};\n"
        )
    }

    /// A leaf face at the path its declaration names — the shipped shape, one directory per face,
    /// one level below the parent it names (`src/control/object/node_7/node_7.rs`). The declaration
    /// is what the *cache* reads; the directory is what the *walk* reads, and the two must agree for
    /// the fixture to exercise both halves.
    /// 叶子面放在它声明里点名的路径上——出厂形状，每个面一个目录，位于它点名的父级下一层
    /// （`src/control/object/node_7/node_7.rs`）。**缓存**读的是声明，**遍历**读的是目录，夹具要同时
    /// 走到这两半，两者就必须一致。
    fn leaf_source(index: usize, density: Density, parents: Parents) -> String {
        let group = group_of(index);
        let declared = format!(".under(crate::group_{group}::NODE_ID);\n\n");
        let parent = match parents {
            Parents::Declared => declared.as_str(),
            Parents::Root => ".under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\n",
        };
        let mut source = format!(
            "//! Scale fixture face {index}.\n\
             //! 规模夹具面 {index}。\n\n\
             #[xirang_toolchain::soil]\n\
             pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Node{index}>()\n\
             \x20\x20\x20\x20{parent}\
             pub struct Node{index};\n"
        );
        if density == Density::Ordinary {
            for item in 0..ORDINARY_ITEMS {
                source.push_str(&format!(
                    "\npub fn helper_{index}_{item}(value: u32) -> u32 {{ value.wrapping_add({item}) }}\n"
                ));
            }
        }
        source
    }

    /// Which group a face belongs to.
    /// 一个面属于哪一组。
    fn group_of(index: usize) -> usize {
        index / GROUP_FACES
    }

    /// `src/group_<g>/group_<g>.rs` — the face that owns a registry for its group.
    /// `src/group_<g>/group_<g>.rs`——为它那一组拥有注册机的面。
    fn parent_path(src: &Path, group: usize) -> PathBuf {
        let name = format!("group_{group}");
        src.join(&name).join(format!("{name}.rs"))
    }

    /// `src/group_<g>/object/node_<i>/node_<i>.rs` — the shipped shape, one directory per face,
    /// one level below the parent it names.
    /// `src/group_<g>/object/node_<i>/node_<i>.rs`——出厂形状，每个面一个目录，位于它点名的父级下一层。
    fn leaf_path(src: &Path, index: usize) -> PathBuf {
        let name = format!("node_{index}");
        src.join(format!("group_{}", group_of(index)))
            .join("object")
            .join(&name)
            .join(format!("{name}.rs"))
    }

    /// The group faces a tree of `faces` needs: one per [`GROUP_FACES`], the last group included.
    /// 一棵 `faces` 个面的树需要多少个组面：每 [`GROUP_FACES`] 个一组，最后一组也算。
    fn group_count(faces: usize) -> usize {
        faces.div_ceil(GROUP_FACES).max(usize::from(faces > 0))
    }

    /// The leaf faces: everything that is not a group face.
    /// 叶子面：除了组面之外的全部。
    fn leaf_count(faces: usize) -> usize {
        faces.saturating_sub(group_count(faces))
    }

    /// The node list the walk would produce for the fabricated shape: one group face per
    /// [`GROUP_FACES`], a directory node for each group's `object/`, one leaf node per leaf face, and
    /// — when neighbours were asked for — a directory node per ordinary neighbour.
    /// 遍历对合成形状会产出的节点表：每 [`GROUP_FACES`] 个面一个组面、每组一个 `object/` 目录节点、
    /// 每个叶子面一个叶子节点；要到邻居时，每个普通邻居还有一个目录节点。
    fn fabricated_nodes(src: &Path, shape: &Shape) -> Vec<Node> {
        let mut groups = Vec::with_capacity(group_count(shape.faces));
        for group in 0..group_count(shape.faces) {
            let first = group * GROUP_FACES;
            let last = (first + GROUP_FACES).min(shape.faces);
            let leaves = (first..last)
                .map(|index| Node {
                    name: format!("node_{index}"),
                    file: Some(leaf_path(src, index)),
                    children: Vec::new(),
                })
                .collect();
            groups.push(Node {
                name: format!("group_{group}"),
                file: Some(parent_path(src, group)),
                children: vec![Node {
                    name: "object".to_owned(),
                    file: None,
                    children: leaves,
                }],
            });
        }
        let neighbours = (0..shape.plain)
            .map(|index| Node {
                name: format!("item_{index}"),
                file: Some(plain_path(src, index)),
                children: Vec::new(),
            })
            .collect();
        vec![Node {
            name: "src".to_owned(),
            file: None,
            children: vec![Node {
                name: "plain".to_owned(),
                file: None,
                children: neighbours,
            }]
            .into_iter()
            .chain(groups)
            .collect(),
        }]
    }

    /// An ordinary neighbour: real Rust, no declaration. It must not even spell the words that make
    /// the validation path look for one, or it stops being the thing this measures.
    /// 普通邻居：真 Rust、没有声明。它连"让校验路径去找声明"的那些词都不该拼出来，否则它就不是这条腿要量的东西了。
    fn plain_source(index: usize) -> String {
        let mut source = format!(
            "//! Ordinary neighbour {index}.\n\
             //! 普通邻居 {index}。\n\n\
             pub fn neighbour_{index}(value: u32) -> u32 {{ value.wrapping_add({index} as u32) }}\n"
        );
        for item in 0..ORDINARY_ITEMS {
            source.push_str(&format!(
                "\npub fn companion_{index}_{item}(value: u32) -> u32 {{ value.wrapping_sub({item}) }}\n"
            ));
        }
        source
    }

    fn plain_path(src: &Path, index: usize) -> PathBuf {
        let name = format!("item_{index}");
        src.join("plain").join(&name).join(format!("{name}.rs"))
    }

    /// Write a fabricated tree into a fresh temp root.
    /// 在一个新的临时根下写下一棵合成的树。
    /// The parent identity a fixture leaf resolves to, parsed the way every pass parses it.
    /// 夹具里某个叶子解析出的父级身份，按每一趟解析它的方式解析。
    ///
    /// Shared by the three memo pins: they differ in **what is open around the call**, not in how the
    /// leaf is read, and three copies of the read would be three places to drift.
    /// 三条备忘钉子共用它：它们的区别在于**调用时周围开着什么**，而不在于怎么读这个叶子；写三份读法就是三处会
    /// 各自漂移的地方。
    fn parent_of(fixture: &Fixture, index: usize) -> Option<xirang_kernel::identity::NodeId> {
        let source =
            std::fs::read_to_string(leaf_path(&fixture.src, index)).expect("the leaf reads");
        let relative = format!("group_0/object/node_{index}/node_{index}.rs");
        let face = crate::build_method::face_syntax_check::parsed_face(&source, &relative)
            .expect("the leaf parses");
        crate::build_method::discovery_cache::cached_parent_id(&fixture.src, &face)
    }

    fn fabricate(shape: &Shape) -> Fixture {
        // A **per-call sequence**, not just the process id and the shape: two tests in one process that
        // happen to use the same shape would otherwise share a root, and each one's opening
        // `remove_dir_all` would delete the other's cache mid-pass — measured here as a pass answering
        // `disabled` (`misses=0`) because the directory it had just written was gone.
        // **逐调用递增**的序号，而不只是进程 id 与形状：同一个进程里两个恰好用同一形状的测试会共用同一个根，
        // 而各自开头的 `remove_dir_all` 会删掉对方正在用的缓存——本轮实测到的形状是这一趟回 `disabled`
        // （`misses=0`），因为它刚写下的目录已经不在了。
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-scale-{}-{}-{}-{sequence}",
            std::process::id(),
            shape.faces,
            shape.plain
        ));
        let _ = std::fs::remove_dir_all(&root);
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        for group in 0..group_count(shape.faces) {
            let file = parent_path(&src, group);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, parent_source(group)).unwrap();
        }
        for index in 0..leaf_count(shape.faces) {
            let file = leaf_path(&src, index);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, leaf_source(index, shape.density, shape.parents)).unwrap();
        }
        for index in 0..shape.plain {
            let file = plain_path(&src, index);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, plain_source(index)).unwrap();
        }
        let nodes = fabricated_nodes(&src, shape);
        Fixture { root, src, nodes }
    }

    /// One pass over the cache, and what the process paid for it: wall time, the disk bytes it
    /// moved, and the resident set afterwards. The counters come from `/proc`, which is Linux-only;
    /// where they are unreadable the line says nothing rather than inventing a number.
    /// 一次缓存遍历，以及进程为此付出的东西：墙钟时间、移动的磁盘字节、之后的常驻内存。计数器来自
    /// `/proc`（仅 Linux）；读不到时那一行什么都不说，而不是编一个数字。
    struct Pass {
        ms: u128,
        summary: String,
        read_bytes: Option<u64>,
        write_bytes: Option<u64>,
        rss_kb: Option<u64>,
    }

    fn one_pass(fixture: &Fixture) -> Pass {
        let before = process_io();
        let started = Instant::now();
        let summary =
            update_discovery_cache(&fixture.root, &fixture.src, &fixture.nodes, "scale-probe");
        let ms = started.elapsed().as_millis();
        let after = process_io();
        Pass {
            ms,
            summary,
            read_bytes: pair_delta(before, after, 0),
            write_bytes: pair_delta(before, after, 1),
            rss_kb: status_field("VmRSS:"),
        }
    }

    fn process_io() -> Option<(u64, u64)> {
        let text = std::fs::read_to_string("/proc/self/io").ok()?;
        let field = |name: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(name))
                .and_then(|rest| rest.trim().parse::<u64>().ok())
        };
        Some((field("read_bytes:")?, field("write_bytes:")?))
    }

    fn pair_delta(
        before: Option<(u64, u64)>,
        after: Option<(u64, u64)>,
        half: usize,
    ) -> Option<u64> {
        let (before, after) = (before?, after?);
        let read = |pair: (u64, u64)| if half == 0 { pair.0 } else { pair.1 };
        Some(read(after).saturating_sub(read(before)))
    }

    fn status_field(name: &str) -> Option<u64> {
        let text = std::fs::read_to_string("/proc/self/status").ok()?;
        text.lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse().ok())
    }

    /// The number after `key` in a summary string (`misses=1000,` ⇒ `1000`).
    /// 摘要字符串里 `key` 后面的那个数（`misses=1000,` ⇒ `1000`）。
    fn count(summary: &str, key: &str) -> Option<usize> {
        let (_, rest) = summary.split_once(key)?;
        rest.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()
    }

    /// Files and bytes under `directory` — the disk the cache costs.
    /// `directory` 下的文件数与字节数——缓存占的磁盘。
    fn footprint(directory: &Path) -> (usize, u64) {
        let (mut files, mut bytes) = (0, 0);
        let mut stack = vec![directory.to_path_buf()];
        while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };
                if metadata.is_dir() {
                    stack.push(entry.path());
                } else {
                    files += 1;
                    bytes += metadata.len();
                }
            }
        }
        (files, bytes)
    }

    fn report(leg: &str, phase: &str, pass: &Pass) {
        let mut line = format!("scale {leg} {phase}: {:>7} ms · {}", pass.ms, pass.summary);
        if let Some(rss) = pass.rss_kb {
            line.push_str(&format!(" · rss {rss} kB"));
        }
        if let (Some(read), Some(write)) = (pass.read_bytes, pass.write_bytes) {
            line.push_str(&format!(" · io r{read} w{write} B"));
        }
        println!("{line}");
    }

    /// The child half of the restart phase: a **new process** on the same disk cache. It rebuilds
    /// only the node list — no files are written — runs one pass, and prints it for the parent.
    /// 重启阶段的子进程那半：同一个磁盘缓存上的**新进程**。它只重建节点表（不写任何文件），跑一次，
    /// 把结果打印给父进程。
    fn child_fixture() -> Option<Fixture> {
        let root = PathBuf::from(std::env::var_os(CHILD_ROOT)?);
        let src = root.join("src");
        let faces: usize = std::env::var(CHILD_FACES).ok()?.parse().ok()?;
        let plain: usize = std::env::var(CHILD_PLAIN)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        // Only the *paths* matter for rebuilding the nodes; the bodies live on disk already.
        // 重建节点表只与**路径**有关；文件内容已经在盘上了。
        let nodes = if faces == 0 {
            crate::build_method::source_walk::discover_root(&src)
        } else {
            fabricated_nodes(
                &src,
                &Shape {
                    faces,
                    density: Density::Declarations,
                    parents: Parents::Declared,
                    plain,
                },
            )
        };
        Some(Fixture { root, src, nodes })
    }

    /// The path libtest prints for a test in this module: `module_path!()` carries the crate name
    /// and libtest's own listing does not, so the leading segment is dropped rather than spelled out.
    /// libtest 为这个模块里的测试打印的路径：`module_path!()` 带 crate 名而 libtest 的清单不带，
    /// 因此去掉首段，而不是把它写死。
    fn test_path(name: &str) -> String {
        let module = module_path!();
        let path = module.split_once("::").map_or(module, |(_, rest)| rest);
        format!("{path}::{name}")
    }

    /// Pin the namespace this measurement runs under, and hand it to the child.
    /// 固定本次测量所用的命名空间，并把它交给子进程。
    ///
    /// `NodeId` is a function of `(namespace, path, kind)` and `package_namespace()` reads a
    /// **process-wide** override first-write-wins, then `CARGO_PKG_NAME`. This test binary pins the
    /// override (`identity::freeze_test_namespace`, the fixture helper that exists for exactly this);
    /// a freshly spawned child starts with no override and would resolve `CARGO_PKG_NAME` instead —
    /// different namespace, different ids, every unit invalid, and the restart would report a miss for
    /// a cache that is perfectly good. Measured: that is what the child did until the namespace was
    /// passed along, and the failure is *order-dependent* — the guard passes on its own and fails in
    /// the full suite, because which namespace is pinned depends on what ran before it.
    /// `NodeId` 是 `(命名空间, 路径, kind)` 的函数，而 `package_namespace()` 先读一个**进程级**、先到先得的
    /// 覆盖值，再读 `CARGO_PKG_NAME`。本测试二进制会固定那个覆盖值（`identity::freeze_test_namespace`，
    /// 这个夹具辅助函数正是为此存在）；而新起的子进程没有覆盖值，会改读 `CARGO_PKG_NAME`——命名空间不同、
    /// 身份不同、每个单元都失效，于是一次完好的缓存被报成 miss。实测：在把命名空间传过去之前，子进程正是
    /// 这么做的，而且这个失败**与顺序有关**——守卫单跑过、放进整套里失败，因为固定的是哪个命名空间取决于
    /// 谁先跑过。
    fn pinned_namespace() -> String {
        crate::build_method::registry_identity::freeze_test_namespace();
        crate::build_method::registry_identity::package_namespace()
    }

    /// The restart phase for a fabricated tree: the child rebuilds the same node list from the shape.
    /// 合成树的重启阶段：子进程按形状重建同一份节点表。
    fn restart(fixture: &Fixture, shape: &Shape, name: &str) -> String {
        child_restart(fixture, Some(shape), name)
    }

    /// The restart phase for a real tree: the child walks the copy again.
    /// 真树的重启阶段：子进程重新遍历那份副本。
    fn restart_real(fixture: &Fixture, name: &str) -> String {
        child_restart(fixture, None, name)
    }

    fn child_restart(fixture: &Fixture, shape: Option<&Shape>, name: &str) -> String {
        let namespace = pinned_namespace();
        let output = std::process::Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                &test_path(name),
                "--exact",
                "--include-ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_ROOT, &fixture.root)
            .env("CARGO_PKG_NAME", &namespace)
            .env(
                CHILD_FACES,
                shape.map_or(0, |shape| shape.faces).to_string(),
            )
            .env(
                CHILD_PLAIN,
                shape.map_or(0, |shape| shape.plain).to_string(),
            )
            .output()
            .expect("the restart child runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find_map(|line| {
                line.split_once(CHILD_MARKER)
                    .map(|(_, rest)| rest.trim().to_owned())
            })
            .unwrap_or_else(|| {
                panic!(
                    "the child prints its restart line: {stdout}\n{}",
                    String::from_utf8_lossy(&output.stderr)
                )
            })
    }

    /// Cold, warm, restart and after-edit at one scale and one density — the four states the
    /// acceptance names, each with its own number, plus the disk the cache costs and the rows it
    /// returns. `name` is this test's exact name, which the restart child re-runs.
    /// 一个规模、一种密度下的冷、热、重启、编辑后四态——验收点名的四种状态各带自己的数字，外加缓存
    /// 占的磁盘与它返回的行数。`name` 是本测试的准确名字，重启子进程会重跑它。
    fn four_states(shape: Shape, name: &str) {
        if let Some(fixture) = child_fixture() {
            let pass = one_pass(&fixture);
            println!("{CHILD_MARKER}{} ms · {}", pass.ms, pass.summary);
            assert!(
                pass.summary.starts_with("hit"),
                "a fresh process reuses the disk cache: {}",
                pass.summary
            );
            return;
        }
        let fixture = fabricate(&shape);
        let leg = shape.label();
        let cold = one_pass(&fixture);
        report(&leg, "cold", &cold);
        assert_eq!(
            count(&cold.summary, "misses="),
            Some(shape.faces),
            "every face is parsed on the cold pass, so the rows recall every face: {}",
            cold.summary
        );
        assert_eq!(
            count(&cold.summary, "hits="),
            Some(0),
            "cold: {}",
            cold.summary
        );
        let (files, bytes) = footprint(&fixture.root.join("target"));
        let discovery = fixture
            .root
            .join("target")
            .join("xirang")
            .join("cache")
            .join("discovery-scale-probe.tsv");
        let rows = std::fs::read_to_string(&discovery)
            .map(|text| {
                let rows = text.lines().filter(|line| !line.starts_with('#')).count();
                (rows, text.len())
            })
            .unwrap_or((0, 0));
        println!(
            "scale {leg} cache: {files} files · {bytes} B · discovery {} rows · {} B",
            rows.0, rows.1
        );
        assert_eq!(
            rows.0, shape.faces,
            "the discovery file has one row per face — and none for an ordinary neighbour"
        );

        let warm = one_pass(&fixture);
        report(&leg, "warm", &warm);
        assert!(
            warm.summary.starts_with("hit"),
            "with every unit file valid nothing is re-parsed: {}",
            warm.summary
        );
        assert_eq!(
            count(&warm.summary, "units="),
            Some(shape.faces),
            "warm: {}",
            warm.summary
        );

        println!("scale {leg} restart: {}", restart(&fixture, &shape, name));

        std::fs::write(
            leaf_path(&fixture.src, 0),
            edited_leaf(shape.density, shape.parents),
        )
        .unwrap();
        let edit = one_pass(&fixture);
        report(&leg, "edit", &edit);
        assert_eq!(
            count(&edit.summary, "misses="),
            Some(1),
            "one edited face is re-parsed: {}",
            edit.summary
        );
        assert_eq!(
            count(&edit.summary, "hits="),
            Some(shape.faces - 1),
            "and every untouched face is still reused: {}",
            edit.summary
        );
        let _ = std::fs::remove_dir_all(&fixture.root);
    }

    /// The edited face: same declaration, different bytes — the cache must follow the bytes.
    /// 被编辑的那个面：声明不变、字节不同——缓存必须跟着字节走。
    fn edited_leaf(density: Density, parents: Parents) -> String {
        format!(
            "{}\n// edited after the warm pass\n",
            leaf_source(0, density, parents)
        )
    }

    /// The leg that runs in the default set. It is small on purpose and it is the guard that would
    /// have caught the fixture rule this module first got wrong: a `Node` whose `file` cannot be read
    /// as it stands produces **no rows at all**, and every scale number would then be a fast run over
    /// nothing.
    /// 默认集里跑的那条腿。它刻意很小，而它正是当初能抓住本模块搞错的夹具规则的守卫：`Node` 的 `file`
    /// 原样读不出来时会**一行都不产生**，于是每个规模数字都只是"很快地跑过了空无一物"。
    #[test]
    fn a_small_tree_reuses_every_unit_and_reparses_only_the_edited_face() {
        four_states(
            Shape::faces(64),
            "a_small_tree_reuses_every_unit_and_reparses_only_the_edited_face",
        );
    }

    /// A33's 1k leg. `#[ignore]` because it writes a thousand face files; run the whole set with
    /// `cargo test -p xirang-toolchain --offline --features mcp --lib -- inventory_scale_tests
    /// --include-ignored --nocapture --test-threads=1`.
    /// A33 的 1k 那条腿。`#[ignore]` 是因为它要写一千个面文件；整组用上面那条命令跑。
    #[test]
    #[ignore = "writes a thousand face files; run explicitly (A33)"]
    fn a_thousand_faces_cold_then_warm() {
        four_states(Shape::faces(1_000), "a_thousand_faces_cold_then_warm");
    }

    /// A33's 10k leg.
    /// A33 的 10k 那条腿。
    #[test]
    #[ignore = "writes ten thousand face files; run explicitly (A33)"]
    fn ten_thousand_faces_cold_then_warm() {
        four_states(Shape::faces(10_000), "ten_thousand_faces_cold_then_warm");
    }

    /// The isolation leg for the one parse source the summary does not count: the same 10k tree with
    /// leaves that name **no** parent by path. The gap to the `Declared` leg is what resolving a
    /// declared parent costs per new face on the cold path — resolution is per unit, not per parent.
    /// 摘要不数的那一处解析来源的隔离腿：同一棵 10k 的树，但叶子**不按路径**点名父级。它与
    /// `Declared` 腿的差，就是冷路径上每个新面解析一个被点名父级的代价——解析是按单元做的，不是按父级做一次。
    #[test]
    #[ignore = "writes ten thousand parentless face files; run explicitly (A33)"]
    fn ten_thousand_parentless_faces_cold_then_warm() {
        four_states(
            Shape::parentless(10_000),
            "ten_thousand_parentless_faces_cold_then_warm",
        );
    }

    /// A33's 50k leg — the top of the range the acceptance names.
    /// A33 的 50k 那条腿——验收点名的量程上端。
    #[test]
    #[ignore = "writes fifty thousand face files; run explicitly (A33)"]
    fn fifty_thousand_faces_cold_then_warm() {
        four_states(Shape::faces(50_000), "fifty_thousand_faces_cold_then_warm");
    }

    /// The acceptance's other density at 1k: the same declarations inside a body of ordinary items.
    /// 验收里的另一种密度，跑 1k：同样的声明，外面包着一大段普通条目。
    #[test]
    #[ignore = "writes a thousand ordinary-dense face files; run explicitly (A33)"]
    fn a_thousand_ordinary_dense_faces_cold_then_warm() {
        four_states(
            Shape::ordinary(1_000),
            "a_thousand_ordinary_dense_faces_cold_then_warm",
        );
    }

    /// The shape a real crate actually has: one declared face **plus one ordinary neighbour** per
    /// directory. It is the leg that answers "who can be reused" — a file with no declaration gets no
    /// row, hence no unit, hence nothing the next pass can consume, and the in-process validator is
    /// the only thing that ever spares it a parse.
    /// 真实 crate 真正具有的形状：每个目录里一个声明面**加一个普通邻居**。它是回答"谁能被复用"的那条腿
    /// ——没有声明的文件没有行、因而没有单元、因而下一趟没有任何东西可消费，唯一让它免于解析的是进程内
    /// 的校验器。
    #[test]
    #[ignore = "writes two thousand files; run explicitly (A33)"]
    fn a_thousand_faces_with_a_thousand_ordinary_neighbours_cold_then_warm() {
        four_states(
            Shape::mixed(1_000, 1_000),
            "a_thousand_faces_with_a_thousand_ordinary_neighbours_cold_then_warm",
        );
    }

    /// A real tree out of this checkout, copied aside: `toolchain/src` as it stands. It declares
    /// **no** faces — this crate is not an XiRang host — so the measurement is the limit case the
    /// other real leg cannot show: 416 files, and not one row, because a row needs a declaration and
    /// not a file. Copied rather than measured in place because the measurement writes a cache
    /// directory, and a gate running in parallel with this one must not share it with the build it is
    /// auditing.
    /// 本检出里的一棵真树，复制出来测：现成的 `toolchain/src`。它**一个注册面都没有**——本 crate 不是
    /// XiRang 宿主——因此这次测量给出另一条真树腿给不出的极限情形：416 个文件、零行，因为一行需要的是
    /// 声明而不是文件。复制而不是就地测，是因为本测量会写一个缓存目录，而与它并行的门禁不该与它所审计
    /// 的构建共用那个目录。
    #[test]
    #[ignore = "copies this checkout's toolchain sources and walks them; run explicitly (A33)"]
    fn the_checkouts_own_sources_declare_nothing_to_cache() {
        let origin = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let files = count_files(&origin);
        let root = std::env::temp_dir().join(format!("xirang-scale-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy_tree(&origin, &root.join("src"));
        let src = root.join("src");
        let nodes = crate::build_method::source_walk::discover_root(&src);
        let fixture = Fixture { root, src, nodes };
        let cold = one_pass(&fixture);
        report(
            &format!("real toolchain/src ({files} files)"),
            "cold",
            &cold,
        );
        assert_eq!(
            count(&cold.summary, "misses="),
            Some(0),
            "no declaration, no row: {}",
            cold.summary
        );
        assert_eq!(
            count(&cold.summary, "hits="),
            Some(0),
            "and nothing to reuse either: {}",
            cold.summary
        );
        let _ = std::fs::remove_dir_all(&fixture.root);
    }

    /// The largest **real** host in this checkout: the shipped example's host package, copied aside.
    /// It is small — a handful of faces — and that is the honest limit of "real" here: no tree in this
    /// checkout carries the thousand faces the scale legs above fabricate. What this leg adds is that
    /// the fabricated shape *is* the shipped shape, walked by the same `discover_root` the build uses.
    /// 本检出里**最大**的真宿主：出厂示例的 host 包，复制出来测。它很小——只有几个面——而这就是"真实"
    /// 在本检出里的诚实上限：没有任何一棵树带着上面那些规模腿合成出来的上千个面。这条腿加的是：合成的
    /// 形状**就是**出厂形状，而且是由构建自己那趟 `discover_root` 走出来的。
    #[test]
    #[ignore = "copies the shipped example host and walks it; run explicitly (A33)"]
    fn the_shipped_example_host_cold_then_warm() {
        if let Some(fixture) = child_fixture() {
            let pass = one_pass(&fixture);
            println!("{CHILD_MARKER}{} ms · {}", pass.ms, pass.summary);
            assert!(pass.summary.starts_with("hit"), "restart: {}", pass.summary);
            return;
        }
        let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let origin = checkout.join("examples/control-suite/host");
        let root =
            std::env::temp_dir().join(format!("xirang-scale-example-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy_tree(&origin, &root);
        let src = root.join("src");
        let nodes = crate::build_method::source_walk::discover_root(&src);
        let fixture = Fixture { root, src, nodes };
        let cold = one_pass(&fixture);
        report("real example host", "cold", &cold);
        let faces = count(&cold.summary, "misses=").expect("the cold pass counts its misses");
        assert!(
            faces > 0,
            "the shipped host declares faces: {}",
            cold.summary
        );
        let warm = one_pass(&fixture);
        report("real example host", "warm", &warm);
        assert_eq!(
            count(&warm.summary, "units="),
            Some(faces),
            "warm: {}",
            warm.summary
        );
        println!(
            "scale real example host restart: {}",
            restart_real(&fixture, "the_shipped_example_host_cold_then_warm")
        );
        let _ = std::fs::remove_dir_all(&fixture.root);
    }

    /// How many files a copied real tree carries — part of the measurement, not decoration.
    /// 复制出来的真树带了多少个文件——这是测量的一部分，不是装饰。
    fn count_files(directory: &Path) -> usize {
        let mut files = 0;
        let mut stack = vec![directory.to_path_buf()];
        while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    stack.push(entry.path());
                } else {
                    files += 1;
                }
            }
        }
        files
    }

    /// Copy every file under `from` into `to`, preserving the tree and skipping build output — a
    /// shipped example carries a `target/` of its own, and copying it would cost more than the
    /// measurement it feeds.
    /// 把 `from` 下的每个文件按原树形复制进 `to`，并跳过构建产物——出厂示例自带一个 `target/`，
    /// 复制它的代价比它所喂养的那次测量还大。
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        let mut stack = vec![(from.to_path_buf(), to.to_path_buf())];
        while let Some((from, to)) = stack.pop() {
            for entry in std::fs::read_dir(&from)
                .expect("the source is readable")
                .flatten()
            {
                let target = to.join(entry.file_name());
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    if entry.file_name() == "target" {
                        continue;
                    }
                    std::fs::create_dir_all(&target).unwrap();
                    stack.push((entry.path(), target));
                } else {
                    std::fs::copy(entry.path(), target).unwrap();
                }
            }
        }
    }

    /// The memo is an answer cache, not a second rule — and it composes: an inner pass cannot empty an
    /// outer pass's memo.
    /// 备忘是答案缓存、不是第二条规则——而且它可以组合：内层那一趟不能清空外层那一趟的备忘。
    ///
    /// Two halves of one property. The first: under a scope, the answer is the identity the unmemoized
    /// lookup gives, so nothing about *what* a parent is changed by caching it. The second: passes call
    /// passes here (the contract pass and the registration pass both run inside one build), so the guard
    /// **restores** the memo it found instead of writing a constant — a guard that cleared on drop would
    /// silently throw the outer pass's work away, and the only symptom would be a slower build, which no
    /// test would notice. The parent's declaration is rewritten while the outer scope is still open —
    /// something a pass never does to its own tree — so an answer still equal to the first one can only
    /// have come from the outer memo.
    /// 同一条性质的两半。第一半：在作用域之下，答案与不做备忘时的身份相同，因此缓存没有改变"父级是什么"。
    /// 第二半：这里一趟会调用另一趟（契约那一趟与注册那一趟都跑在同一次构建里），因此守卫**恢复**它找到的
    /// 备忘而不是写死一个常量——一个在 drop 时清空的守卫会静默丢掉外层的成果，而唯一的症状是构建变慢，
    /// 没有任何测试会发现。外层作用域还开着时把父级的声明改掉——一趟不会对自己的树做这种事——因此答案若仍
    /// 等于第一个，只可能来自外层那份备忘。
    #[test]
    fn the_memo_is_an_answer_cache_not_a_second_rule_and_it_composes() {
        let fixture = fabricate(&Shape::faces(4));
        let plain = parent_of(&fixture, 1);
        assert!(plain.is_some(), "this leaf names its parent by path");
        let _outer = crate::build_method::discovery_cache::ParentLookupScope::open();
        let scoped = parent_of(&fixture, 1);
        assert_eq!(
            scoped, plain,
            "the memo answers what the plain lookup answers"
        );
        {
            let _inner = crate::build_method::discovery_cache::ParentLookupScope::open();
            assert_eq!(
                parent_of(&fixture, 1),
                plain,
                "the inner pass answers alike"
            );
        }
        std::fs::write(
            parent_path(&fixture.src, 0),
            parent_source(0).replace("Group0", "Renamed0"),
        )
        .expect("the parent is rewritten");
        assert_eq!(
            parent_of(&fixture, 1),
            plain,
            "the outer memo survived the inner scope instead of being cleared by it"
        );
        let _ = std::fs::remove_dir_all(&fixture.root);
    }

    /// The memo lives exactly one pass: a parent whose **kind** changed is read again by the next pass,
    /// never answered out of the previous one.
    /// 备忘只活一趟：**kind** 变了的父级会被下一趟重新读取，绝不会用上一趟的答案作答。
    ///
    /// This is the property that makes reuse sound here at all — a pass reads a tree that is not moving
    /// under it, and the guard is what turns that into a fact rather than an assumption. The child is
    /// edited too, because a unit whose own bytes are unchanged is a hit and never asks the question; the
    /// pin has to make the question get asked for the answer to mean anything.
    /// 这正是复用在这里能成立的唯一性质——一趟读的树在它脚下不动，而那个作用域把这件事变成事实而不是假设。
    /// 子级也要改：字节没变的单元是命中、根本不会问这个问题；钉子必须让问题真的被问出来，答案才有意义。
    #[test]
    fn a_parent_kind_change_is_read_again_by_the_next_pass() {
        let fixture = fabricate(&Shape::faces(4));
        let first = one_pass(&fixture);
        assert_eq!(
            count(&first.summary, "misses="),
            Some(4),
            "{}",
            first.summary
        );
        let child = "group_0/object/node_1/node_1.rs";
        let parent = "group_0/group_0.rs";
        // Select by the **fingerprint of the file as it is now**, not by `path` alone: a re-parsed unit
        // is written under a new fingerprint while the old one stays on disk, so a search by path finds
        // one of two files — this pin flapped about half its runs because of that, and the flake was in
        // the finder, not in the memo (measured before this line).
        // 按**该文件此刻的指纹**选，而不是只按 `path`：重解析的单元写在新指纹下，而旧的仍在盘上，因此只按
        // path 找会在两个文件之间任选其一——本钉子在加上这一行之前大约一半的运行会闪，而闪在**查找器**上，
        // 不在备忘上（实测）。
        let unit_parent_of = |path: &str| {
            let wanted = crate::build_method::identity_cache::source_unit_fingerprint(&leaf_path(
                &fixture.src,
                1,
            ));
            let content = std::fs::read_dir(fixture.root.join("target/xirang/cache/units"))
                .expect("the units directory")
                .flatten()
                .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
                .find(|content| {
                    content
                        .lines()
                        .any(|line| line.strip_prefix("path\t") == Some(path))
                        && content
                            .lines()
                            .any(|line| line.strip_prefix("# fingerprint\t") == Some(&wanted))
                })
                .unwrap_or_else(|| panic!("a unit for {path} under its current fingerprint"));
            content
                .lines()
                .find_map(|line| line.strip_prefix("parent\t").map(str::to_owned))
                .expect("the unit carries its parent")
        };
        let old_parent = unit_parent_of(child);

        // The parent's declaration changes kind, and the child's bytes change so its unit is re-parsed.
        // 父级的声明换了 kind，而子级的字节也变，好让它的单元被重新解析。
        std::fs::write(
            parent_path(&fixture.src, 0),
            parent_source(0).replace("Group0", "Renamed0"),
        )
        .expect("the parent is rewritten");
        let leaf = leaf_path(&fixture.src, 1);
        let mut edited = std::fs::read_to_string(&leaf).expect("the leaf reads");
        edited.push_str("\n// edited so this unit is parsed again\n");
        std::fs::write(&leaf, edited).expect("the child is rewritten");

        let second = one_pass(&fixture);
        // Two misses, not one: the parent is a face file too, so its own unit changes when its kind does.
        // 两处 miss 而不是一处：父级本身也是面文件，它的 kind 一变、它自己的单元也就变了。
        assert_eq!(
            count(&second.summary, "misses="),
            Some(2),
            "the rewritten parent and the edited child are re-parsed: {}",
            second.summary
        );
        assert_eq!(
            count(&second.summary, "hits="),
            Some(2),
            "the untouched siblings are still reused: {}",
            second.summary
        );
        let new_parent = unit_parent_of(child);
        assert_ne!(
            old_parent, new_parent,
            "the next pass reads the parent again instead of answering from the last one"
        );
        assert_eq!(
            new_parent,
            xirang_kernel::identity::NodeId::from_namespaced_path(
                &crate::build_method::registry_identity::package_namespace(),
                parent,
                "Renamed0",
            )
            .to_string(),
            "and it is the identity the new declaration carries"
        );
        let _ = std::fs::remove_dir_all(&fixture.root);
    }
}
