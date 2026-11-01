//! §7e point 4 against the **actual file state** — the pin the maintainer's ruling of 2026-10-18 turned around.
//! §7e 第 4 条对着**实际文件状态**——这条钉子被维护者 2026-10-18 的裁定掉了个方向。

mod generation_pair_tests {
    use crate::build_method::discovery_cache::write_if_changed_at;

    /// **Tombstone + positive control**: the two-publish window this file used to record no longer exists.
    /// **墓碑 + 正控制**：这份文件过去记录的那扇"两次发布"的窗口已经不存在了。
    ///
    /// What was here (measured 2026-10-18, before the ruling): the body and its `<path>.generation` sidecar were two
    /// renames, so a crash between them left a current body recorded at an older generation; the identical-bytes retry
    /// returned `Ok` without repairing the sidecar, and a writer carrying that older generation then passed the
    /// `published > generation` guard and **overwrote the fresher body**. That is a half change reported as complete,
    /// which the maintainer's fifth item forbids. The ruling chose option A1: write the generation into the artifact
    /// itself, so one `rename` publishes the bytes and the number that names them.
    /// 这里原本记着（2026-10-18 实测，在裁定之前）：正文与 `<path>.generation` 旁注是两次 rename，崩在中间会留下
    /// "正文是新的、记在旧代次上"；相同字节的重试返回 `Ok` 却不修旁注；随后带旧代次的写入方通过
    /// `published > generation` 那道闸，**覆盖了更新的正文**。那是"半套改动却报告完成"，正是裁定第 5 条禁止的。
    /// 裁定选了 A1：把代次写进产物本身，于是**一次** `rename` 同时发布字节与命名它们的那个数字。
    ///
    /// This pin now asserts the closed window and keeps a control for each half that used to be wrong:
    /// 这条钉子现在断言那扇窗已经关上，并为过去错的两半各留一个控制：
    ///
    /// 1. **One publish**: the write leaves exactly one artifact and no sidecar, so the state "body new, record old"
    ///    has no way to exist.
    /// 1. **一次发布**：写入只留下一个产物、没有旁注，因此"正文新、记录旧"这个状态无从存在。
    /// 2. **A retired sidecar cannot resurrect a stale writer**: an old tree's `.generation` file is read as a floor
    ///    and **deleted** by the write that replaces it — the exact failure of the recorded defect was a sidecar
    ///    number that kept guarding an artifact it no longer described.
    /// 2. **退役旁注不能复活旧写入者**：老树上的 `.generation` 文件被当作下界读出，并在替换它的那次写入里**删掉**
    ///    ——被记录的那个缺陷，其失败正是"一个不再描述产物的数字继续守着它"。
    /// 3. **The stale writer is still refused** — the guard did not go away with the sidecar.
    /// 3. **旧写入者仍被拒绝**——那道闸没有跟着旁注一起消失。
    #[test]
    fn the_generation_lives_in_the_artifact_so_the_two_publish_window_is_gone() {
        let root = std::env::temp_dir().join(format!("xirang-pair-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("plan.tsv");
        let retired = root.join("plan.tsv.generation");

        // 1. One publish: the artifact carries its own generation and nothing is published beside it.
        // 1. 一次发布：产物自己带着代次，旁边没有别的东西被发布。
        write_if_changed_at(&target, "first\n", 1).expect("the first write lands");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 1\nfirst\n",
            "the number and the bytes are one file"
        );
        assert!(
            !retired.exists(),
            "no sidecar is written, so no crash can split the pair"
        );

        // 2. A tree written by the **older scheme** (a sidecar, no line inside the artifact) keeps its guard on the
        // first write and loses the sidecar to that same write.
        // 2. 由**旧方案**写下的树（有旁注、产物里没有那一行）在第一次写入时仍保有守卫，而旁注被那次写入一并丢掉。
        std::fs::write(&target, "body from the older scheme\n").unwrap();
        std::fs::write(&retired, "7\n").unwrap();
        write_if_changed_at(&target, "first\n", 6)
            .expect_err("the retired sidecar is still a floor");
        write_if_changed_at(&target, "first\n", 7).expect("the same generation lands");
        assert!(
            !retired.exists(),
            "the retired sidecar is deleted by the write that replaces it"
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 7\nfirst\n"
        );

        // 3. The guard still refuses a stale plan, with both numbers named — measured on the artifact's own line.
        // 3. 闸门仍然拒绝过期计划、报出两个编号——依据是产物自己那一行。
        let refusal = write_if_changed_at(&target, "first\n", 1)
            .expect_err("a stale plan must not overwrite a fresher artifact");
        assert!(
            refusal.contains("generation 7") && refusal.contains("generation 1"),
            "the refusal names what is on disk and what was offered: {refusal}"
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# xirang-generation: 7\nfirst\n",
            "and the fresher artifact is untouched"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
