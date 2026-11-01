//! The move's **record publish** against the actual file state: the record is the last step of a transactional
//! write, so a failure there has to undo the move rather than leave half of it.
//! move 的**记录发布**对着实际文件状态：记录是一次事务式写入的最后一步，因此在那里失败必须把整次搬动撤回去，
//! 而不是留下半套。

mod record_fault_tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    /// Every authored byte under `src/`, keyed by path below the root.
    /// `src/` 下的每一个作者字节，按相对根的路径为键。
    fn authored(root: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![root.join("src")];
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else if let Ok(bytes) = std::fs::read(&path) {
                    files.insert(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                        bytes,
                    );
                }
            }
        }
        files
    }

    /// The module path the tree itself reports for one face.
    /// 树自己报告的那个面的模块路径。
    fn module_of(root: &std::path::Path, node: &str) -> String {
        crate::mcp::tools::run_tool(root, "xirang.explain", &json!({"node": node}))
            .expect("explain answers")
            .lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("module ")
                    .map(str::trim)
                    .map(str::to_owned)
            })
            .expect("explain reports the module path")
    }

    fn fixture(label: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("mcp-move-record-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"move-record\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "// host entry\n").unwrap();
        for request in [
            json!({"action":"add","fields":{"module":"control","kind":"Control","needs_registry":true},"apply":true}),
            json!({"action":"add","parent":"root/control","fields":{"module":"button","kind":"Button","needs_registry":true},"apply":true}),
            json!({"action":"add","parent":"root/control","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
            json!({"action":"add","parent":"root/control/button","fields":{"module":"cap","kind":"Cap"},"apply":true}),
        ] {
            crate::mcp::apply::apply(&root, &request)
                .unwrap_or_else(|error| panic!("{request}: {error}"));
        }
        root
    }

    /// §7e: the record is published **last**, and a failure publishing it must undo the whole move. This pin holds the
    /// two halves together — the bytes really are restored, and the same request succeeds once the obstacle is gone.
    /// §7e：记录**最后**发布，发布失败必须撤回整次搬动。本钉子把两半钉在一起——字节真的被还原，而拿掉障碍之后同一请求
    /// 确实成功。
    ///
    /// Measured 2026-10-18: `.xirang/moves` is where the record directory is created, so occupying it with a **file**
    /// makes `create_dir_all` fail inside `publish()` — after the face file was renamed and every edit was written.
    /// 2026-10-18 实测：`.xirang/moves` 正是创建记录目录的地方，因此用一个**文件**占住它，会让 `publish()` 里的
    /// `create_dir_all` 失败——而此时面文件已经改名、每处编辑也已经写入。
    #[test]
    fn a_failed_record_publish_undoes_the_whole_move_and_a_retry_lands() {
        let root = fixture("undo");
        let before = authored(&root);
        let obstacle = root.join(".xirang/moves");
        std::fs::create_dir_all(root.join(".xirang")).unwrap();
        std::fs::write(&obstacle, "not a directory\n").unwrap();
        let refused = crate::mcp::apply::apply(
            &root,
            &json!({"action":"move","node":"root/control/button","to":"root/control/panel","apply":true}),
        )
        .expect_err("the record cannot be published into a file");
        assert!(
            refused.contains("moves"),
            "the failure names the record it could not write: {refused}"
        );
        // The actual file state, byte for byte: every authored file is where it was, with what it had.
        // 实际文件状态、逐字节：每个作者文件都在原位、内容原样。
        assert_eq!(
            authored(&root),
            before,
            "a failed record publish must undo the move's writes and renames"
        );
        assert!(
            root.join(".xirang/moves").is_file(),
            "and the obstacle is still the obstacle, not a record directory"
        );
        // Recovery: take the obstacle away and run the same request again.
        // 恢复：拿掉障碍、再跑同一个请求。
        std::fs::remove_file(&obstacle).unwrap();
        let applied = crate::mcp::apply::apply(
            &root,
            &json!({"action":"move","node":"root/control/button","to":"root/control/panel","apply":true}),
        )
        .expect("the same move lands once the record can be published");
        assert!(
            applied.contains("record "),
            "the reply names the record: {applied}"
        );
        let record = root.join(".xirang/moves/1/move.plan");
        let recorded = std::fs::read_to_string(&record).expect("the record is on disk");
        assert!(
            recorded.contains("from=root/control/button")
                && recorded.contains("to=root/control/panel"),
            "the record states the move it published: {recorded}"
        );
        let moved = root.join("src/control/object/panel/button/button.rs");
        assert!(
            moved.is_file(),
            "and the face really moved: {}",
            moved.display()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// The documented repair, walked end to end: a move that would change graft coverage is **refused before writing**,
    /// the declaration is detached, the same move then lands, and the graft is **rebound** through the product entry so
    /// it names the face where it now lives.
    /// 走一遍文档里的修复：会改变切口覆盖的搬动被**写入前拒绝**，把声明摘下来，同一个搬动就能落地，再用产品入口把
    /// 切口**重绑**到面现在所在的地方。
    ///
    /// The two claims this replaces "it looks fine" with: (1) a refused move writes **nothing** (every byte, and no
    /// record directory), and (2) the repair the refusal prints is executable — detach, move, rebind, and the resulting
    /// plan names the new spelling rather than the old one.
    /// 这条钉子把"看起来没问题"换成两句可判定的断言：(1) 被拒的搬动**什么都没写**（逐字节，而且没有记录目录）；
    /// (2) 拒绝回执里那份修复是**可执行的**——摘下、搬、重绑，最后计划里是新拼法而不是旧的。
    #[test]
    fn a_coverage_changing_move_writes_nothing_and_the_documented_repair_works() {
        let root = fixture("repair");
        // G: one cut over `cap`, whose replacement is the sibling `panel`.
        // G：一条切 `cap` 的切口，替换端是兄弟面 `panel`。
        let cap_module = module_of(&root, "root/control/button/cap");
        crate::mcp::apply::apply(
            &root,
            &json!({"action":"cut","cut":format!("crate::{cap_module}::NODE_ID"),
                    "graft":"crate::control::object::panel::NODE_ID","full":true,"apply":true}),
        )
        .expect("the cut applies");
        let before = authored(&root);
        // M: moving the cut face changes the coverage, so the move must refuse **before** writing.
        // M：搬被切的面会改变覆盖，因此搬动必须在**写入之前**拒绝。
        let refused = crate::mcp::apply::apply(
            &root,
            &json!({"action":"move","node":"root/control/button/cap","to":"root/control","apply":true}),
        )
        .expect_err("the move would change graft coverage");
        assert!(
            refused.contains("refused before writing") && refused.contains("repair map"),
            "the refusal names both the boundary and the repair: {refused}"
        );
        assert_eq!(
            authored(&root),
            before,
            "a refused move writes nothing at all"
        );
        assert!(
            !root.join(".xirang/moves").exists(),
            "and it publishes no move record"
        );
        // Detach: take the declaration out of the host entry, the way the repair sequence says.
        // 摘下：按修复序列把声明从宿主入口拿掉。
        let entry = root.join("src/lib.rs");
        let text = std::fs::read_to_string(&entry).unwrap();
        // Cut from the doc comment, not from the item: a trailing `///` with no item after it is a parse error, and
        // the move's graft verification reads the entry before it plans anything.
        // 从文档注释处切、而不是从条目处：留下一条没有条目的 `///` 是解析错误，而 move 的 graft 校验在规划之前
        // 就要读这个入口。
        let start = text
            .find("/// Graft declaration")
            .expect("the cut wrote a plan");
        std::fs::write(&entry, &text[..start]).unwrap();
        // Move: the same request, now that nothing names the face.
        // 搬：同一个请求，现在没有东西点名那个面了。
        crate::mcp::apply::apply(
            &root,
            &json!({"action":"move","node":"root/control/button/cap","to":"root/control","apply":true}),
        )
        .expect("after the detach the move lands");
        // The face is where the move put it, and its module path is what the rebind has to spell.
        // 面在搬动放它的地方，而它现在的模块路径正是重绑要写出的拼法。
        let moved = root.join("src/control/cap/cap.rs");
        assert!(moved.is_file(), "the face moved: {}", moved.display());
        // Rebind through the product entry, using the path the tree reports rather than a guess.
        // 用产品入口重绑，拼法取自树自己报告的路径，而不是猜。
        let module = module_of(&root, "root/control/cap");
        crate::mcp::apply::apply(
            &root,
            &json!({"action":"cut","cut":format!("crate::{module}::NODE_ID"),
                    "graft":"crate::control::object::panel::NODE_ID","full":true,"apply":true}),
        )
        .expect("the rebind applies");
        // Both surfaces spell the endpoint their own way (`apply cut` renders the catalog spelling, the graft view
        // prints the module path), so the assertion is on the part they share: the moved face's module path.
        // 两个面各自用各自的拼法（`apply cut` 渲染 catalog 拼法，graft 视图印模块路径），因此断言落在它们共有的
        // 那部分上：被搬的面的模块路径。
        let plan = std::fs::read_to_string(&entry).unwrap();
        assert!(
            plan.contains(&format!("{module}::NODE_ID")),
            "the rebound plan names the face where it now is ({module}): {plan}"
        );
        let grafts = crate::mcp::tools::run_tool(&root, "xirang.grafts", &json!({}))
            .expect("grafts answers");
        assert!(
            grafts.contains(&format!("{module}::NODE_ID")),
            "and the tree's graft view agrees: {grafts}"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
